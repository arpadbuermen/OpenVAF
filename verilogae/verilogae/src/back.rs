use core::ptr::NonNull;
use std::borrow::Borrow;

use camino::Utf8Path;
use hir::Type;
use hir_lower::{CallBackKind, CurrentKind, HirInterner, ParamInfoKind, ParamKind, PlaceKind};
use lasso::Rodeo;
use llvm_sys::target_machine::LLVMCodeGenOptLevel;
use mir::{ControlFlowGraph, FuncRef, Function};
use mir_llvm::{Builder, BuilderVal, CallbackFun, CodegenCx, LLVMBackend, UNNAMED};
use stdx::iter::multiunzip;
use typed_index_collections::TiVec;
use typed_indexmap::TiSet;

use crate::compiler_db::{
    current_name, voltage_name, CompilationDB, FuncSpec, InternedModel, ModelInfo,
};

pub fn sim_param_stub<'ll>(cx: &CodegenCx<'_, 'll>) -> CallbackFun<'ll> {
    cx.const_callback(&[cx.ty_ptr()], cx.const_real(0.0))
}

pub fn sim_param_opt_stub<'ll>(cx: &CodegenCx<'_, 'll>) -> CallbackFun<'ll> {
    cx.const_return(&[cx.ty_ptr(), cx.ty_double()], 1)
}

pub fn sim_param_str_stub<'ll>(cx: &CodegenCx<'_, 'll>) -> CallbackFun<'ll> {
    let empty_str = cx.literals.get("").unwrap();
    let empty_str = cx.const_str(empty_str);
    let ty_str = cx.ty_ptr();
    cx.const_callback(&[ty_str], empty_str)
}

pub fn lltype<'ll>(ty: &Type, cx: &CodegenCx<'_, 'll>) -> &'ll llvm_sys::LLVMType {
    match ty {
        Type::Real => cx.ty_double(),
        Type::Integer => cx.ty_int(),
        Type::String => cx.ty_ptr(),
        Type::Array { ty, len } => cx.ty_array(lltype(ty, cx), *len),
        Type::EmptyArray => cx.ty_array(cx.ty_int(), 0),
        Type::Bool => cx.ty_bool(),
        Type::Void => cx.ty_void(),
        Type::Err => unreachable!(),
    }
}

pub fn stub_callbacks<'ll>(
    cb: &TiSet<FuncRef, CallBackKind>,
    cx: &CodegenCx<'_, 'll>,
    // invalid_param_dst: &AHashMap<ParamId, &'ll Value>,
) -> TiVec<FuncRef, Option<CallbackFun<'ll>>> {
    cb.raw
        .iter()
        .map(|kind| {
            let res = match kind {
                CallBackKind::SimParam => sim_param_stub(cx),
                CallBackKind::SimParamOpt => sim_param_opt_stub(cx),
                CallBackKind::SimParamStr => sim_param_str_stub(cx),
                CallBackKind::Derivative(_)
                | CallBackKind::NodeDerivative(_)
                | CallBackKind::TimeDerivative
                | CallBackKind::FlickerNoise { .. }
                | CallBackKind::WhiteNoise { .. }
                | CallBackKind::NoiseTable(_) => {
                    cx.const_callback(&[cx.ty_double()], cx.const_real(0.0))
                }
                CallBackKind::Print { .. }
                | CallBackKind::ParamInfo(_, _)
                | CallBackKind::BuiltinLimit { .. }
                | CallBackKind::StoreLimit(_)
                | CallBackKind::LimDiscontinuity
                | CallBackKind::CollapseHint(_, _) => return None,
                CallBackKind::Analysis => cx.const_callback(&[cx.ty_ptr()], cx.const_int(1)),
            };

            Some(res)
        })
        .collect()
}

pub struct CodegenCtx<'a, 't> {
    pub model_info: &'a ModelInfo,
    pub llbackend: &'a LLVMBackend<'t>,
    pub literals: &'a mut Rodeo,
    pub opt_lvl: LLVMCodeGenOptLevel,
}

struct Codegen<'a, 'b, 'll> {
    db: &'a CompilationDB,
    model_info: &'a ModelInfo,
    intern: &'a HirInterner,
    builder: &'b mut Builder<'a, 'a, 'll>,
    func: &'a Function,
    spec: &'a FuncSpec,
}

impl<'ll> Codegen<'_, '_, 'll> {
    unsafe fn read_depbreak(
        &mut self,
        offset: &'ll llvm_sys::LLVMValue,
        ptr: &'ll llvm_sys::LLVMValue,
        ty: Type,
    ) {
        let vars =
            self.spec.dependency_breaking.iter().copied().filter(|var| var.ty(self.db) == ty);
        let llty = lltype(&ty, self.builder.cx);
        for (i, var) in vars.clone().enumerate() {
            if let Some(id) = self.intern.params.index(&ParamKind::HiddenState(var)) {
                self.builder.params[id] = self.read_fat_ptr_at(i, offset, ptr, llty).into();
            }
        }

        let global_name = format!("{}.depbreak.{}", self.spec.prefix, ty);
        let names = vars.clone().map(|var| &*self.model_info.var_names[&var]);
        self.export_names(names, &global_name);
    }

    unsafe fn read_str_params(&mut self, ptr: &'ll llvm_sys::LLVMValue) {
        let params = self.intern.live_params(&self.func.dfg).filter_map(|(id, kind, _)| {
            if let ParamKind::Param(param) = *kind {
                (param.ty(self.db) == Type::String).then_some((id, param))
            } else {
                None
            }
        });

        for (i, (id, _)) in params.clone().enumerate() {
            let ptr = self.builder.cx.const_gep(
                self.builder.cx.ty_ptr(),
                ptr,
                &[self.builder.cx.const_usize(i)],
            );
            self.builder.params[id] = self.builder.load(self.builder.cx.ty_ptr(), ptr).into();
        }

        let global_name = format!("{}.params.{}", self.spec.prefix, Type::String);
        let names = params.map(|(_, param)| &*self.model_info.params[&param].name);
        self.export_names(names, &global_name);
    }

    unsafe fn read_params(
        &mut self,
        offset: &'ll llvm_sys::LLVMValue,
        ptr: &'ll llvm_sys::LLVMValue,
        ty: Type,
    ) {
        let params = self.intern.live_params(&self.func.dfg).filter_map(|(id, kind, _)| {
            if let ParamKind::Param(param) = kind {
                (param.ty(self.db) == ty).then_some((id, *param))
            } else {
                None
            }
        });

        let llty = lltype(&ty, self.builder.cx);
        for (i, (id, _)) in params.clone().enumerate() {
            self.builder.params[id] = self.read_fat_ptr_at(i, offset, ptr, llty).into();
        }

        let global_name = format!("{}.params.{}", self.spec.prefix, ty);
        let names = params.clone().map(|(_, param)| &*self.model_info.params[&param].name);
        self.export_names(names, &global_name);
    }

    unsafe fn read_voltages(
        &mut self,
        offset: &'ll llvm_sys::LLVMValue,
        ptr: &'ll llvm_sys::LLVMValue,
    ) {
        let voltages = self.intern.live_params(&self.func.dfg).filter_map(|(id, kind, _)| {
            if let ParamKind::Voltage { hi, lo } = kind {
                Some((id, (*hi, *lo)))
            } else {
                None
            }
        });

        let default_val = |(id, volt)| {
            self.model_info
                .optional_voltages
                .get(&volt)
                .map(|val| ((id, volt), self.builder.cx.const_real(*val)))
        };

        let (optional_voltages, default_vals): (Vec<_>, Vec<_>) =
            voltages.clone().filter_map(default_val).unzip();

        let non_optional_voltages: Vec<_> =
            voltages.filter(|kind| default_val(*kind).is_none()).collect();

        let voltages = optional_voltages.into_iter().chain(non_optional_voltages);

        for (i, (id, _)) in voltages.clone().enumerate() {
            self.builder.params[id] =
                self.read_fat_ptr_at(i, offset, ptr, self.builder.cx.ty_double()).into();
        }

        let global_name = format!("{}.voltages.default", self.spec.prefix);
        self.builder.cx.export_array(
            &global_name,
            self.builder.cx.ty_double(),
            &default_vals,
            true,
            true,
        );

        let global_name = format!("{}.voltages", self.spec.prefix);
        let names = voltages.map(|(_, (hi, lo))| voltage_name(self.db, hi, lo));
        self.export_names(names, &global_name);
    }

    unsafe fn read_currents(
        &mut self,
        offset: &'ll llvm_sys::LLVMValue,
        ptr: &'ll llvm_sys::LLVMValue,
    ) {
        let voltages = self.intern.live_params(&self.func.dfg).filter_map(|(id, kind, _)| {
            if let ParamKind::Current(kind) = kind {
                Some((id, *kind))
            } else {
                None
            }
        });

        let default_val = |(id, kind)| {
            if let CurrentKind::Branch(branch) = kind {
                if let Some(val) = self.model_info.optional_currents.get(&branch) {
                    return Some(((id, kind), self.builder.cx.const_real(*val)));
                }
            }
            None
        };

        let (optional_voltages, default_vals): (Vec<_>, Vec<_>) =
            voltages.clone().filter_map(default_val).unzip();

        let non_optional_voltages: Vec<_> =
            voltages.filter(|kind| default_val(*kind).is_none()).collect();

        let voltages = optional_voltages.into_iter().chain(non_optional_voltages);

        for (i, (id, _)) in voltages.clone().enumerate() {
            self.builder.params[id] =
                self.read_fat_ptr_at(i, offset, ptr, self.builder.cx.ty_double()).into();
        }

        let global_name = format!("{}.currents.default", self.spec.prefix);
        self.builder.cx.export_array(
            &global_name,
            self.builder.cx.ty_double(),
            &default_vals,
            true,
            true,
        );

        let global_name = format!("{}.currents", self.spec.prefix);
        let names = voltages.map(|(_, kind)| current_name(self.db, kind));
        self.export_names(names, &global_name);
    }

    fn export_names<T: Borrow<str>>(&mut self, names: impl Iterator<Item = T>, global_name: &str) {
        let cx = &mut self.builder.cx;
        let names: Vec<_> = names
            .map(|name| {
                let name = name.borrow();
                let name = cx.literals.get(name).unwrap();
                cx.const_str(name)
            })
            .collect();
        cx.export_array(global_name, cx.ty_ptr(), &names, true, true);
    }

    unsafe fn read_fat_ptr_at(
        &mut self,
        pos: usize,
        offset: &'ll llvm_sys::LLVMValue,
        ptr: &'ll llvm_sys::LLVMValue,
        ptr_ty: &'ll llvm_sys::LLVMType,
    ) -> &'ll llvm_sys::LLVMValue {
        let builder = &mut self.builder;

        // get correct ptrs from array
        let fat_ptr = builder.gep(builder.cx.ty_fat_ptr(), ptr, &[builder.cx.const_usize(pos)]);

        let (ptr, meta) = builder.fat_ptr_to_parts(fat_ptr);

        // get stride or scalar ptr by bitcasting
        let stride = builder.load(builder.cx.ty_size(), meta);

        // load the array ptr and check if its a null ptr
        let arr_ptr = builder.load(builder.cx.ty_ptr(), ptr);
        let is_arr_null = builder.is_null_ptr(arr_ptr);

        // offset the array _ptr
        let offset = builder.imul(stride, offset);
        let arr_ptr = builder.gep(ptr_ty, arr_ptr, &[offset]);

        // if the array_ptr is null select the scalar_ptr otherwise select the arr_ptr
        let ptr = builder.select(is_arr_null, meta, arr_ptr);

        //final load
        builder.load(ptr_ty, ptr)
    }
}

impl CodegenCtx<'_, '_> {
    pub(crate) fn gen_func_obj(
        &self,
        db: &CompilationDB,
        spec: &FuncSpec,
        func: &Function,
        cfg: &ControlFlowGraph,
        intern: &HirInterner,
        dst: &Utf8Path,
    ) {
        let module =
            unsafe { self.llbackend.new_module(&spec.var.name(db), self.opt_lvl).unwrap() };
        let cx = unsafe { self.llbackend.new_ctx(self.literals, &module) };

        let ret_ty = lltype(&spec.var.ty(db), &cx);

        let fun_ty = cx.ty_func(
            &[
                cx.ty_size(), // offset
                cx.ty_ptr(),  // voltages
                cx.ty_ptr(),  // curents
                cx.ty_ptr(),  // real paras
                cx.ty_ptr(),  // int paras
                cx.ty_ptr(),  // str paras
                cx.ty_ptr(),  // real dependency_breaking
                cx.ty_ptr(),  // int dependency_breaking
                cx.ty_ptr(),  // temperature
                cx.ty_ptr(),  // ret
            ],
            cx.ty_void(),
        );
        let llfun = cx.declare_ext_fn(&spec.prefix, fun_ty);

        // setup builder
        let mut builder = Builder::new(&cx, func, llfun);

        let mut codegen =
            Codegen { db, model_info: self.model_info, intern, builder: &mut builder, func, spec };

        // read parameters

        let offset = unsafe { llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 0) };

        let true_val = cx.const_bool(true);
        codegen.builder.params = intern
            .params
            .raw
            .iter()
            .map(|(kind, val)| {
                if func.dfg.value_dead(*val) {
                    return BuilderVal::Undef;
                }

                let val = match kind {
                    ParamKind::Param(_)
                    | ParamKind::Voltage { .. }
                    | ParamKind::Current(_)
                    | ParamKind::HiddenState(_) => return BuilderVal::Undef,
                    ParamKind::Temperature => unsafe {
                        let temperature =
                            llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 8);
                        codegen.read_fat_ptr_at(0, &*offset, &*temperature, cx.ty_double())
                    },
                    ParamKind::ParamGiven { .. } | ParamKind::PortConnected { .. } => true_val,
                    ParamKind::ParamSysFun(param) => {
                        codegen.builder.cx.const_real(param.default_value())
                    }
                    ParamKind::ImplicitUnknown(_)
                    | ParamKind::Abstime
                    | ParamKind::PrevState(_)
                    | ParamKind::NewState(_) => codegen.builder.cx.const_real(0.0),
                    ParamKind::EnableIntegration | ParamKind::EnableLim => {
                        codegen.builder.cx.const_bool(false)
                    }
                };

                val.into()
            })
            .collect();

        let voltages = unsafe { llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 1) };
        unsafe { codegen.read_voltages(&*offset, &*voltages) };

        let currents = unsafe { llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 2) };
        unsafe { codegen.read_currents(&*offset, &*currents) };

        let real_params = unsafe { llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 3) };
        unsafe { codegen.read_params(&*offset, &*real_params, Type::Real) };

        let int_params = unsafe { llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 4) };
        unsafe { codegen.read_params(&*offset, &*int_params, Type::Integer) };

        let str_params = unsafe { llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 5) };
        unsafe { codegen.read_str_params(&*str_params) };

        let real_dep_break =
            unsafe { llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 6) };
        unsafe { codegen.read_depbreak(&*offset, &*real_dep_break, Type::Real) };

        let int_dep_break =
            unsafe { llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 7) };
        unsafe { codegen.read_depbreak(&*offset, &*int_dep_break, Type::Integer) };

        // setup callbacks

        codegen.builder.callbacks = stub_callbacks(&intern.callbacks, codegen.builder.cx);
        let postorder: Vec<_> = cfg.postorder(func).collect();

        let exit_bb = *postorder
            .iter()
            .find(|bb| {
                func.layout
                    .last_inst(**bb)
                    .map_or(true, |term| !func.dfg.insts[term].is_terminator())
            })
            .unwrap();

        unsafe {
            // the actual compiled function
            builder.build_consts();
            builder.build_func();

            // write the return value
            builder.select_bb(exit_bb);

            let out = llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 9);
            let out = builder.gep(ret_ty, &*out, &[&*offset]);

            let ret_val = intern.outputs[&PlaceKind::Var(spec.var)].unwrap();
            let ret_val = builder.values[ret_val].get(&builder);

            builder.store(out, ret_val);

            builder.ret_void();
        }

        // build object file
        drop(builder);
        debug_assert!(module.verify_and_print(), "Invalid code generated");
        module.optimize();

        module.emit_object(dst.as_ref()).expect("code generation failed!")
    }

    pub(crate) fn ensure_names(&mut self, db: &CompilationDB, intern: &HirInterner) {
        for param in &intern.params.raw {
            match *param.0 {
                ParamKind::Voltage { hi, lo } => {
                    self.literals.get_or_intern(&voltage_name(db, hi, lo));
                }
                ParamKind::Current(kind) => {
                    self.literals.get_or_intern(&current_name(db, kind));
                }
                _ => (),
            }
        }

        for func in &self.model_info.functions {
            for dep in &*func.dependency_breaking {
                self.literals.get_or_intern(&*self.model_info.var_names[dep]);
            }
        }
    }

    fn read_params<'ll>(
        &self,
        intern: &HirInterner,
        ty: Type,
        builder: &mut Builder<'_, '_, 'll>,
        val_ptr: &'ll llvm_sys::LLVMValue,
        param_given_ptr: &'ll llvm_sys::LLVMValue,
        param_given_offset: usize,
    ) -> usize {
        let llty = lltype(&ty, builder.cx);
        let mut offset = 0;
        for (param, info) in self.model_info.params.iter() {
            if info.ty != ty {
                continue;
            }

            let given_id = intern.params.unwrap_index(&ParamKind::ParamGiven { param: *param });
            let given = unsafe {
                let off = builder.cx.const_usize(param_given_offset + offset);
                let ptr = builder.gep(builder.cx.ty_c_bool(), param_given_ptr, &[off]);
                let cbool = builder.load(builder.cx.ty_c_bool(), ptr);
                builder.int_cmp(
                    cbool,
                    builder.cx.const_c_bool(false),
                    llvm_sys::LLVMIntPredicate::LLVMIntNE,
                )
            };

            let val_id = intern.params.unwrap_index(&ParamKind::Param(*param));
            let val = unsafe {
                let off = builder.cx.const_usize(offset);
                let ptr = builder.gep(llty, val_ptr, &[off]);
                builder.load(llty, ptr)
            };

            builder.params[given_id] = given.into();
            builder.params[val_id] = val.into();
            offset += 1;
        }
        offset
    }

    fn write_params<'ll>(
        &self,
        intern: &HirInterner,
        ty: Type,
        builder: &mut Builder<'_, '_, 'll>,
        val_ptr: &'ll llvm_sys::LLVMValue,
        bounds_ptrs: Option<(&'ll llvm_sys::LLVMValue, &'ll llvm_sys::LLVMValue)>,
    ) {
        let llty = lltype(&ty, builder.cx);
        for (i, (param, _)) in
            self.model_info.params.iter().filter(|(_, info)| info.ty == ty).enumerate()
        {
            let param_val = intern.outputs[&PlaceKind::Param(*param)].unwrap();
            let param_val = unsafe { builder.values[param_val].get(builder) };

            unsafe {
                let off = builder.cx.const_usize(i);
                let ptr = builder.gep(llty, val_ptr, &[off]);
                builder.store(ptr, param_val)
            };

            if let Some((min_ptr, max_ptr)) = bounds_ptrs {
                unsafe {
                    let param_min = intern.outputs[&PlaceKind::ParamMin(*param)].unwrap();
                    let param_min = builder.values[param_min].get(builder);
                    let off = builder.cx.const_usize(i);
                    let ptr = builder.gep(llty, min_ptr, &[off]);
                    builder.store(ptr, param_min)
                }

                unsafe {
                    let param_max = intern.outputs[&PlaceKind::ParamMax(*param)].unwrap();
                    let param_max = builder.values[param_max].get(builder);
                    let off = builder.cx.const_usize(i);
                    let ptr = builder.gep(llty, max_ptr, &[off]);
                    builder.store(ptr, param_max)
                }
            }
        }
    }

    fn param_flag_cb<'ll>(
        &self,
        cx: &CodegenCx<'_, 'll>,
        set: bool,
    ) -> (&'ll llvm_sys::LLVMValue, &'ll llvm_sys::LLVMType) {
        let name = cx.local_callback_name();
        let fun_ty = cx.ty_func(&[cx.ty_ptr(), cx.ty_c_bool()], cx.ty_void());
        let fun = cx.declare_int_fn(&name, fun_ty);
        unsafe {
            let bb = llvm_sys::core::LLVMAppendBasicBlockInContext(
                NonNull::from(cx.llcx).as_ptr(),
                NonNull::from(fun).as_ptr(),
                UNNAMED,
            );
            let builder =
                llvm_sys::core::LLVMCreateBuilderInContext(NonNull::from(cx.llcx).as_ptr());
            llvm_sys::core::LLVMPositionBuilderAtEnd(builder, bb);
            let ptr = llvm_sys::core::LLVMGetParam(NonNull::from(fun).as_ptr(), 0);
            let flag = llvm_sys::core::LLVMGetParam(NonNull::from(fun).as_ptr(), 1);
            let val = llvm_sys::core::LLVMBuildLoad2(
                builder,
                NonNull::from(cx.ty_c_bool()).as_ptr(),
                ptr,
                UNNAMED,
            );
            let val = if set {
                llvm_sys::core::LLVMBuildOr(builder, val, flag, UNNAMED)
            } else {
                llvm_sys::core::LLVMBuildAnd(builder, val, flag, UNNAMED)
            };
            llvm_sys::core::LLVMBuildStore(builder, val, ptr);
            llvm_sys::core::LLVMBuildRetVoid(builder);
            llvm_sys::core::LLVMDisposeBuilder(builder);
        }

        (fun, fun_ty)
    }

    fn insert_param_info_callbacks<'ll>(
        &self,
        intern: &HirInterner,
        builder: &mut Builder<'_, '_, 'll>,
        param_flags: &'ll llvm_sys::LLVMValue,
        real_cnt: usize,
        int_cnt: usize,
    ) {
        let mut real_off = 0;
        let mut int_off = real_cnt;
        let mut str_off = int_off + int_cnt;

        let param_info_set_cb = self.param_flag_cb(builder.cx, true);
        let param_info_unset_cb = self.param_flag_cb(builder.cx, false);
        for (param, info) in self.model_info.params.iter() {
            let off = match info.ty {
                Type::Real => &mut real_off,
                Type::Integer => &mut int_off,
                Type::String => &mut str_off,
                _ => unreachable!(),
            };

            let dst = unsafe {
                let off = builder.cx.const_usize(*off);
                builder.gep(builder.cx.ty_c_bool(), param_flags, &[off])
            };

            *off += 1;

            for (kind, set, mut bits) in [
                (ParamInfoKind::Invalid, true, 0b100),
                (ParamInfoKind::MinInclusive, true, 0b001),
                (ParamInfoKind::MaxInclusive, true, 0b010),
                (ParamInfoKind::MinExclusive, false, 0b001),
                (ParamInfoKind::MaxExclusive, false, 0b010),
            ] {
                let (fun, fun_ty) = if set {
                    param_info_set_cb
                } else {
                    bits = !bits;
                    param_info_unset_cb
                };

                let res = CallbackFun {
                    fun_ty,
                    fun,
                    state: vec![dst, builder.cx.const_u8(bits)].into_boxed_slice(),
                    num_state: 0,
                };

                let cb = intern.callbacks.unwrap_index(&CallBackKind::ParamInfo(kind, *param));
                builder.callbacks[cb] = Some(res)
            }
        }

        assert_eq!(real_off, real_cnt);
        assert_eq!(int_off, real_cnt + int_cnt);
    }

    pub(crate) fn compile_model_info(
        &self,
        dst: &Utf8Path,
        interned_model: InternedModel,
        param_init_func: Function,
        param_init_intern: HirInterner,
    ) {
        let module = unsafe {
            self.llbackend
                .new_module("model_info", LLVMCodeGenOptLevel::LLVMCodeGenLevelNone)
                .unwrap()
        };
        let cx = unsafe { self.llbackend.new_ctx(self.literals, &module) };

        let (fun_names, fun_symbols) = interned_model.functions(&cx);
        cx.export_array("functions", cx.ty_ptr(), &fun_names, true, true);
        cx.export_array("functions.sym", cx.ty_ptr(), &fun_symbols, true, false);

        let op_vars = interned_model.opvars(&cx);
        cx.export_array("opvars", cx.ty_ptr(), &op_vars, true, true);

        let nodes = interned_model.nodes(&cx);
        cx.export_array("nodes", cx.ty_ptr(), &nodes, true, true);

        let module_name = cx.const_str(interned_model.module_name);
        cx.export_val("module_name", cx.ty_ptr(), module_name, true);

        interned_model.export_param_info(&cx, Type::Real);
        interned_model.export_param_info(&cx, Type::Integer);
        interned_model.export_param_info(&cx, Type::String);

        let fun_ty = cx.ty_func(
            &[
                cx.ty_ptr(), // real param values
                cx.ty_ptr(), // int param values
                cx.ty_ptr(), // str param values
                cx.ty_ptr(), // real param min
                cx.ty_ptr(), // int param min
                cx.ty_ptr(), // real param max
                cx.ty_ptr(), // int param max
                cx.ty_ptr(), // param_given/error
            ],
            cx.ty_void(),
        );

        let llfun = cx.declare_ext_fn("init_modelcard", fun_ty);

        let mut builder = Builder::new(&cx, &param_init_func, llfun);

        // read parameters

        builder.params = param_init_intern
            .params
            .raw
            .iter()
            .map(|(kind, val)| {
                if param_init_func.dfg.value_dead(*val) {
                    return BuilderVal::Undef;
                }
                let val = match kind {
                    ParamKind::Voltage { .. }
                    | ParamKind::Current(_)
                    | ParamKind::HiddenState(_) => {
                        unreachable!()
                    }
                    ParamKind::Param(_) | ParamKind::ParamGiven { .. } => return BuilderVal::Undef,
                    ParamKind::Temperature => builder.cx.const_real(293f64),
                    ParamKind::PortConnected { .. } => builder.cx.const_bool(true),
                    ParamKind::ParamSysFun(param) => builder.cx.const_real(param.default_value()),
                    ParamKind::ImplicitUnknown(_)
                    | ParamKind::Abstime
                    | ParamKind::PrevState(_)
                    | ParamKind::NewState(_) => builder.cx.const_real(0.0),
                    ParamKind::EnableIntegration | ParamKind::EnableLim => {
                        builder.cx.const_bool(false)
                    }
                };

                val.into()
            })
            .collect();

        let param_flags =
            unsafe { &*llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 7) };

        let param_val_real =
            unsafe { &*llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 0) };
        let real_cnt = self.read_params(
            &param_init_intern,
            Type::Real,
            &mut builder,
            param_val_real,
            param_flags,
            0,
        );

        let param_val_int =
            unsafe { &*llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 1) };
        let int_cnt = self.read_params(
            &param_init_intern,
            Type::Integer,
            &mut builder,
            param_val_int,
            param_flags,
            real_cnt,
        );

        let param_val_str =
            unsafe { &*llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 2) };
        self.read_params(
            &param_init_intern,
            Type::String,
            &mut builder,
            param_val_str,
            param_flags,
            int_cnt + real_cnt,
        );

        // insert callbacks

        builder.callbacks = stub_callbacks(&param_init_intern.callbacks, builder.cx);
        self.insert_param_info_callbacks(
            &param_init_intern,
            &mut builder,
            param_flags,
            real_cnt,
            int_cnt,
        );

        let postorder: Vec<_> = {
            let mut cfg = ControlFlowGraph::new();
            cfg.compute(&param_init_func);
            cfg.postorder(&param_init_func).collect()
        };

        unsafe {
            // the actual compiled function
            builder.build_consts();
            builder.build_func();

            // write the return values
            builder.select_bb(postorder[0]);

            let param_min_real = &*llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 3);
            let param_max_real = &*llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 5);
            self.write_params(
                &param_init_intern,
                Type::Real,
                &mut builder,
                param_val_real,
                Some((param_min_real, param_max_real)),
            );

            let param_min_int = &*llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 4);
            let param_max_int = &*llvm_sys::core::LLVMGetParam(NonNull::from(llfun).as_ptr(), 6);
            self.write_params(
                &param_init_intern,
                Type::Integer,
                &mut builder,
                param_val_int,
                Some((param_min_int, param_max_int)),
            );

            self.write_params(&param_init_intern, Type::String, &mut builder, param_val_str, None);

            builder.ret_void();
        }

        debug_assert!(module.verify_and_print(), "generated invalid code");
        module.optimize();
        // println!("{}", module.to_str());

        module.emit_object(dst.as_ref()).expect("code generation failed!");
    }
}

impl InternedModel<'_> {
    fn functions<'ll>(
        &self,
        cx: &CodegenCx<'_, 'll>,
    ) -> (Vec<&'ll llvm_sys::LLVMValue>, Vec<&'ll llvm_sys::LLVMValue>) {
        self.functions
            .iter()
            .map(|func| (cx.const_str(func.name), cx.const_str(func.prefix)))
            .unzip()
    }

    fn opvars<'ll>(&self, cx: &CodegenCx<'_, 'll>) -> Vec<&'ll llvm_sys::LLVMValue> {
        self.opvars.iter().map(|name| cx.const_str(*name)).collect()
    }

    fn nodes<'ll>(&self, cx: &CodegenCx<'_, 'll>) -> Vec<&'ll llvm_sys::LLVMValue> {
        self.nodes.iter().map(|name| cx.const_str(*name)).collect()
    }

    fn param_info<'ll>(&self, cx: &CodegenCx<'_, 'll>, ty: &Type) -> ParamInfo<'ll> {
        let iter = self.params.iter().filter_map(|param| {
            if ty == param.ty {
                Some((
                    cx.const_str(param.name),
                    cx.const_str(param.unit),
                    cx.const_str(param.description),
                    cx.const_str(param.group),
                ))
            } else {
                None
            }
        });
        let (names, units, descriptions, groups) = multiunzip(iter);
        ParamInfo { units, groups, names, descriptions }
    }

    fn export_param_info(&self, cx: &CodegenCx<'_, '_>, ty: Type) {
        let params = self.param_info(cx, &ty);

        let sym = format!("params.{}", ty);
        cx.export_array(&sym, cx.ty_ptr(), &params.names, true, true);

        let sym = format!("params.unit.{}", ty);
        cx.export_array(&sym, cx.ty_ptr(), &params.units, true, false);

        let sym = format!("params.desc.{}", ty);
        cx.export_array(&sym, cx.ty_ptr(), &params.descriptions, true, false);

        let sym = format!("params.group.{}", ty);
        cx.export_array(&sym, cx.ty_ptr(), &params.groups, true, false);
    }
}

struct ParamInfo<'ll> {
    names: Vec<&'ll llvm_sys::LLVMValue>,
    units: Vec<&'ll llvm_sys::LLVMValue>,
    descriptions: Vec<&'ll llvm_sys::LLVMValue>,
    groups: Vec<&'ll llvm_sys::LLVMValue>,
}
