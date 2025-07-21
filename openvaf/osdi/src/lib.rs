use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};

use base_n::CASE_INSENSITIVE;
use camino::{Utf8Path, Utf8PathBuf};
use hir::{CompilationDB, ParamSysFun, Type};
use hir_lower::{CallBackKind, HirInterner, ParamKind};
use lasso::Rodeo;
use llvm::{LLVMABISizeOfType, LLVMDisposeTargetData, LLVMPrintModuleToString, OptLevel};
use mir_llvm::{CodegenCx, LLVMBackend};
use salsa::ParallelDatabase;
use sim_back::{CompiledModule, ModuleInfo};
use stdx::{impl_debug_display, impl_idx_from};
use target::spec::Target;
use typed_index_collections::TiVec;
use typed_indexmap::TiSet;

use crate::compilation_unit::{new_codegen, OsdiCompilationUnit, OsdiModule};
use crate::metadata::osdi_0_4::OsdiTys;
use crate::metadata::OsdiLimFunction;

mod access;
mod bitfield;
mod compilation_unit;
mod inst_data;
mod metadata;
mod model_data;

mod eval;
mod load;
mod noise;
mod setup;

const OSDI_VERSION: (u32, u32) = (0, 4);

pub fn compile<'a>(
    db: &'a CompilationDB,
    modules: &'a [ModuleInfo],
    dst: &'a Utf8Path,
    target: &'a Target,
    back: &'a LLVMBackend,
    emit: bool,
    opt_lvl: OptLevel,
    dump_mir: bool,
    dump_unopt_mir: bool,
    dump_ir: bool,
    dump_unopt_ir: bool,
) -> (Vec<Utf8PathBuf>, Vec<CompiledModule<'a>>, Rodeo) {
    let mut literals = Rodeo::new();
    let mut lim_table = TiSet::default();
    let mnames: Vec<_> = modules.iter().map(|m| m.module.name(db)).collect();
    let modules: Vec<_> = modules
        .iter()
        .map(|module| {
            let mir = CompiledModule::new(db, module, &mut literals, dump_unopt_mir, dump_mir);
            for cb in mir.intern.callbacks.iter() {
                if let CallBackKind::BuiltinLimit { name, num_args } = *cb {
                    lim_table.ensure(OsdiLimFunction { name, num_args: num_args - 2 });
                }
            }
            mir
        })
        .collect();

    let name = dst.file_stem().expect("destination is a file").to_owned();

    let mut paths: Vec<Utf8PathBuf> = (0..modules.len() * 4)
        .map(|i| {
            let num = base_n::encode((i + 1) as u128, CASE_INSENSITIVE);
            let extension = format!("o{num}");
            dst.with_extension(extension)
        })
        .collect();

    let target_data = unsafe {
        let src = CString::new(target.data_layout.clone()).unwrap();
        llvm::LLVMCreateTargetData(src.as_ptr())
    };

    let compiled_modules = modules;

    let osdi_modules: Vec<_> = compiled_modules
        .iter()
        .map(|module| {
            let unit = OsdiModule::new(db, module, &lim_table);
            unit.intern_names(&mut literals, db);
            unit
        })
        .collect();

    let db = db.snapshot();

    let main_file = dst.with_extension("o");

    let unoptirs = Arc::new(Mutex::new(HashMap::new()));
    let irs = Arc::new(Mutex::new(HashMap::new()));

    rayon_core::scope(|scope| {
        let db = db;
        let literals_ = &literals;
        let target_data_ = &target_data;
        let paths = &paths;

        for (i, module) in osdi_modules.iter().enumerate() {
            let _db = db.snapshot();
            let unoptirs_clone = Arc::clone(&unoptirs);
            let irs_clone = Arc::clone(&irs);
            scope.spawn(move |_| {
                let access = format!("access_{}", &module.sym);
                let name1 = access.clone();
                let llmod = unsafe { back.new_module(&access, opt_lvl).unwrap() };
                let cx = new_codegen(back, &llmod, literals_);
                let tys = OsdiTys::new(&cx, target_data_);
                let cguint = OsdiCompilationUnit::new(&_db, module, &cx, &tys, false);

                cguint.access_function();
                if dump_unopt_ir {
                    let mut unoptirs = unoptirs_clone.lock().unwrap();
                    unoptirs.insert((i, access), cx.to_str().to_string());
                }
                debug_assert!(llmod.verify_and_print());

                if emit {
                    let path = &paths[i * 4];
                    llmod.optimize();
                    assert_eq!(llmod.emit_object(path.as_ref()), Ok(()))
                }

                if dump_ir {
                    let mut irs = irs_clone.lock().unwrap();
                    irs.insert((i, name1), llmod.to_str().to_string());
                }
            });

            let unoptirs_clone = Arc::clone(&unoptirs);
            let irs_clone = Arc::clone(&irs);
            let _db = db.snapshot();
            scope.spawn(move |_| {
                let name = format!("setup_model_{}", &module.sym);
                let name1 = name.clone();
                let llmod = unsafe { back.new_module(&name, opt_lvl).unwrap() };
                let cx = new_codegen(back, &llmod, literals_);
                let tys = OsdiTys::new(&cx, target_data_);
                let cguint = OsdiCompilationUnit::new(&_db, module, &cx, &tys, false);

                cguint.setup_model();
                if dump_unopt_ir {
                    let mut unoptirs = unoptirs_clone.lock().unwrap();
                    unoptirs.insert((i, name), cx.to_str().to_string());
                }
                debug_assert!(llmod.verify_and_print());

                if emit {
                    let path = &paths[i * 4 + 1];
                    llmod.optimize();
                    assert_eq!(llmod.emit_object(path.as_ref()), Ok(()))
                }

                if dump_ir {
                    let mut irs = irs_clone.lock().unwrap();
                    irs.insert((i, name1), llmod.to_str().to_string());
                }
            });

            let unoptirs_clone = Arc::clone(&unoptirs);
            let irs_clone = Arc::clone(&irs);
            let _db = db.snapshot();
            scope.spawn(move |_| {
                let name = format!("setup_instance_{}", &module.sym);
                let name1 = name.clone();
                let llmod = unsafe { back.new_module(&name, opt_lvl).unwrap() };
                let cx = new_codegen(back, &llmod, literals_);
                let tys = OsdiTys::new(&cx, target_data_);
                let mut cguint = OsdiCompilationUnit::new(&_db, module, &cx, &tys, false);

                cguint.setup_instance();
                if dump_unopt_ir {
                    let mut unoptirs = unoptirs_clone.lock().unwrap();
                    unoptirs.insert((i, name), cx.to_str().to_string());
                }
                debug_assert!(llmod.verify_and_print());

                if emit {
                    let path = &paths[i * 4 + 2];
                    llmod.optimize();
                    assert_eq!(llmod.emit_object(path.as_ref()), Ok(()))
                }

                if dump_ir {
                    let mut irs = irs_clone.lock().unwrap();
                    irs.insert((i, name1), llmod.to_str().to_string());
                }
            });

            let unoptirs_clone = Arc::clone(&unoptirs);
            let irs_clone = Arc::clone(&irs);
            let _db = db.snapshot();
            scope.spawn(move |_| {
                let access = format!("eval_{}", &module.sym);
                let name1 = access.clone();
                let llmod = unsafe { back.new_module(&access, opt_lvl).unwrap() };
                let cx = new_codegen(back, &llmod, literals_);
                let tys = OsdiTys::new(&cx, target_data_);
                let cguint = OsdiCompilationUnit::new(&_db, module, &cx, &tys, true);

                cguint.eval();
                if dump_unopt_ir {
                    let mut unoptirs = unoptirs_clone.lock().unwrap();
                    unoptirs.insert((i, access), llmod.to_str().to_string());
                }
                debug_assert!(llmod.verify_and_print());

                if emit {
                    let path = &paths[i * 4 + 3];
                    llmod.optimize();
                    assert_eq!(llmod.emit_object(path.as_ref()), Ok(()))
                }

                if dump_ir {
                    let mut irs = irs_clone.lock().unwrap();
                    irs.insert((i, name1), llmod.to_str().to_string());
                }
            });
        }

        let llmod = unsafe { back.new_module(&name, opt_lvl).unwrap() };
        let cx = new_codegen(back, &llmod, &literals);
        let tys = OsdiTys::new(&cx, target_data);

        let descriptors: Vec<_> = osdi_modules
            .iter()
            .map(|module| {
                let cguint = OsdiCompilationUnit::new(&db, module, &cx, &tys, false);
                let descriptor = cguint.descriptor(target_data, &db);
                descriptor.to_ll_val(&cx, &tys)
            })
            .collect();

        cx.export_array("OSDI_DESCRIPTORS", tys.osdi_descriptor, &descriptors, true, false);
        cx.export_val(
            "OSDI_NUM_DESCRIPTORS",
            cx.ty_int(),
            cx.const_unsigned_int(descriptors.len() as u32),
            true,
        );
        cx.export_val(
            "OSDI_VERSION_MAJOR",
            cx.ty_int(),
            cx.const_unsigned_int(OSDI_VERSION.0),
            true,
        );
        cx.export_val(
            "OSDI_VERSION_MINOR",
            cx.ty_int(),
            cx.const_unsigned_int(OSDI_VERSION.1),
            true,
        );

        let descr_size: u32;
        unsafe {
            descr_size = LLVMABISizeOfType(target_data, tys.osdi_descriptor) as u32;
        }

        cx.export_val("OSDI_DESCRIPTOR_SIZE", cx.ty_int(), cx.const_unsigned_int(descr_size), true);

        if !lim_table.is_empty() {
            let lim: Vec<_> = lim_table.iter().map(|entry| entry.to_ll_val(&cx, &tys)).collect();
            cx.export_array("OSDI_LIM_TABLE", tys.osdi_lim_function, &lim, false, false);
            cx.export_val(
                "OSDI_LIM_TABLE_LEN",
                cx.ty_int(),
                cx.const_unsigned_int(lim.len() as u32),
                true,
            );
        }

        let osdi_log =
            cx.get_declared_value("osdi_log").expect("symbol osdi_log missing from std lib");
        let val = cx.const_null_ptr();
        unsafe {
            llvm::LLVMSetInitializer(osdi_log, val);
            llvm::LLVMSetLinkage(osdi_log, llvm::Linkage::ExternalLinkage);
            llvm::LLVMSetUnnamedAddress(osdi_log, llvm::UnnamedAddr::No);
            llvm::LLVMSetDLLStorageClass(osdi_log, llvm::DLLStorageClass::Export);
        }

        debug_assert!(llmod.verify_and_print());

        if emit {
            // println!("{}", llmod.to_str());
            llmod.optimize();
            // println!("{}", llmod.to_str());
            assert_eq!(llmod.emit_object(main_file.as_ref()), Ok(()))
        }
    });

    if dump_unopt_ir {
        let unoptirs_clone = Arc::clone(&unoptirs);
        let unoptirs = unoptirs_clone.lock().unwrap();
        for ((i, fname), v) in unoptirs.iter() {
            println!("LLVM IR for {} in {}", fname, mnames[*i]);
            println!("{}", v);
            println!();
        }
    }

    if dump_ir {
        let irs_clone = Arc::clone(&irs);
        let irs = irs_clone.lock().unwrap();
        for ((i, fname), v) in irs.iter() {
            println!("Optimized LLVM IR for {} in {}", fname, mnames[*i]);
            println!("{}", v);
            println!();
        }
    }

    paths.push(main_file);
    unsafe { LLVMDisposeTargetData(target_data) };

    (paths, compiled_modules, literals)
}

impl OsdiModule<'_> {
    fn intern_names(&self, literals: &mut Rodeo, db: &CompilationDB) {
        literals.get_or_intern(&*self.info.module.name(db));
        self.intern_node_strs(literals, db);
        literals.get_or_intern_static("Multiplier (Verilog-A $mfactor)");
        literals.get_or_intern_static("deg");
        literals.get_or_intern_static("m");
        literals.get_or_intern_static("");

        for param in self.info.params.values() {
            for alias in &param.alias {
                literals.get_or_intern(&**alias);
            }

            literals.get_or_intern(&*param.name);
            literals.get_or_intern(&param.unit);
            literals.get_or_intern(&param.description);
            literals.get_or_intern(&param.group);
        }

        for (var, opvar_info) in self.info.op_vars.iter() {
            literals.get_or_intern(&*var.name(db));
            literals.get_or_intern(&opvar_info.unit);
            literals.get_or_intern(&opvar_info.description);
        }

        for alias_list in self.info.sys_fun_alias.values() {
            for alias in alias_list {
                literals.get_or_intern(&**alias);
            }
        }

        for param in ParamSysFun::iter() {
            let is_live = |intern: &HirInterner, func| {
                intern.is_param_live(func, &ParamKind::ParamSysFun(param))
            };
            if is_live(self.intern, self.eval)
                || is_live(&self.init.intern, &self.init.func)
                || is_live(self.model_param_intern, self.model_param_setup)
            {
                literals.get_or_intern(format!("${param:?}"));
            }
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct OsdiLimId(u32);
impl_idx_from!(OsdiLimId(u32));
impl_debug_display! {match OsdiLimId{OsdiLimId(id) => "lim{id}";}}

fn ty_len(ty: &Type) -> Option<u32> {
    match ty {
        Type::Array { ty, len } => Some(len * ty_len(ty).unwrap_or(1)),
        Type::EmptyArray => Some(0),
        _ => None,
    }
}

fn lltype<'ll>(ty: &Type, cx: &CodegenCx<'_, 'll>) -> &'ll llvm::Type {
    let llty = match ty.base_type() {
        Type::Real => cx.ty_double(),
        Type::Integer => cx.ty_int(),
        Type::String => cx.ty_ptr(),
        Type::EmptyArray => cx.ty_array(cx.ty_int(), 0),
        Type::Bool => cx.ty_c_bool(),
        Type::Void => cx.ty_void(),
        Type::Err | Type::Array { .. } => unreachable!(),
    };

    if let Some(len) = ty_len(ty) {
        cx.ty_array(llty, len)
    } else {
        llty
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Offset(u32);
impl_idx_from!(Offset(u32));
impl_debug_display! {match Offset{Offset(id) => "lim{id}";}}
