use hir::{BranchWrite, CompilationDB, Node};
use hir_lower::{CurrentKind, HirInterner, ImplicitEquation, ParamKind};
use lasso::Rodeo;
use mir::cursor::FuncCursor;
use mir::Function;
use mir_opt::{simplify_cfg, sparse_conditional_constant_propagation};
use stdx::impl_debug_display;

pub use module_info::{collect_modules, ModuleInfo};

use crate::context::{Context, OptimiziationStage};
use crate::dae::DaeSystem;
use crate::init::Initialization;
use crate::node_collapse::NodeCollapse;
use crate::topology::Topology;

mod context;
pub mod dae;
pub mod init;
mod module_info;
pub mod node_collapse;
mod noise;
mod topology;

mod util;

// #[cfg(test)]
// mod tests;

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum SimUnknownKind {
    KirchoffLaw(Node),
    Current(CurrentKind),
    Implicit(ImplicitEquation),
}

impl_debug_display! {
    match SimUnknownKind{
        SimUnknownKind::KirchoffLaw(node) => "{node:?}";
        SimUnknownKind::Current(curr) => "br[{curr:?}]";
        SimUnknownKind::Implicit(node) => "{node}";
    }
}

pub struct CompiledModule<'a> {
    pub info: &'a ModuleInfo,
    pub dae_system: DaeSystem,
    pub eval: Function,
    pub intern: HirInterner,
    pub init: Initialization,
    pub model_param_setup: Function,
    pub model_param_intern: HirInterner,
    pub node_collapse: NodeCollapse,
}

pub fn print_intern(pfx: &str, db: &CompilationDB, intern: &HirInterner) {
    println!("{pfx}Parameters:");
    intern.params.iter().for_each(|(p, val)| { 
        print!("{pfx}  {:?}", p);
        match p {
            ParamKind::Param(param) => {
                println!("{pfx} .. {:?} -> {:?}", param.name(db), val);
            }, 
            ParamKind::ParamGiven { param } => {
                println!("{pfx} .. {:?} -> {:?}", param.name(db), val);
            }, 
            ParamKind::Voltage{ hi, lo} => {
                if lo.is_some() {
                    print!("{pfx} .. V({:?},{:?})", hi.name(db), lo.unwrap().name(db));
                } else {
                    print!("{pfx} .. V({:?})", hi.name(db));
                }
                println!(" -> {:?}", val);
            }, 
            ParamKind::Current(ck) => {
                match ck {
                    CurrentKind::Branch(br) => {
                        println!("{pfx} .. {:?} -> {:?}", br.name(db), val);        
                    }, 
                    CurrentKind::Unnamed{hi, lo} => {
                        if lo.is_some() {
                            print!("{pfx} .. I({:?},{:?})", hi.name(db), lo.unwrap().name(db));        
                        } else {
                            print!("{pfx} .. I({:?})", hi.name(db));        
                        }
                        println!(" -> {:?}", val);        
                    }, 
                    CurrentKind::Port(n) => {
                        println!("{pfx} .. {:?} -> {:?}", n.name(db), val);
                    }
                }
            },
            ParamKind::HiddenState (var) => {
                println!("{pfx} .. {:?} -> {:?}", var.name(db), val);
            }, 
            // ParamKind::ImplicitUnknown
            ParamKind::PortConnected { port } => {
                println!("{pfx} .. {:?} -> {:?}", port.name(db), val);
            }
            _ => {
                println!("{pfx} -> {:?}", val);
            }, 
        }
    });
    println!("");

    println!("{pfx}Outputs:");
    intern.outputs.iter().for_each(|(p, val)| { 
        if val.is_some() {
            println!("{pfx}  {:?} -> {:?}", p, val.unwrap());
        } else {
            println!("{pfx}  {:?} -> None", p);
        }
    });
    println!("");

    println!("{pfx}Tagged reads:");
    intern.tagged_reads.iter().for_each(|(val, var)| { 
        println!("{pfx}  {:?} -> {:?}", val, var);
    });
    println!("");

    println!("{pfx}Implicit equations:");
    for (i, &iek) in intern.implicit_equations.iter().enumerate() {
        println!("{pfx}  {:?} : {:?}", i, iek);
    }
    println!("");
}

impl<'a> CompiledModule<'a> {
    pub fn new(
        db: &CompilationDB,
        module: &'a ModuleInfo,
        literals: &mut Rodeo, 
        dump_unoptimized_mir: bool
    ) -> CompiledModule<'a> {
        let mut cx = Context::new(db, literals, module);

        // At this point the unoptimized MIR for the module implementation is ready
        if dump_unoptimized_mir {
            println!("Interner for unoptimized MIR");
            print_intern("  ", db, &(cx.intern));
            println!("{}", cx.func.print(&literals));
        }

        // Compute outputs
        cx.compute_outputs(true);
        // Compute control flow graph
        cx.compute_cfg();
        cx.optimize(OptimiziationStage::Initial);
        debug_assert!(cx.func.validate());

        if dump_unoptimized_mir {
            println!("Interner before topology");
            print_intern("  ", db, &(cx.intern));
            println!("{}", cx.func.print(&literals));
        }


        let topology = Topology::new(&mut cx);
        debug_assert!(cx.func.validate());

        if dump_unoptimized_mir {
            println!("Interner for initially optimized MIR");
            print_intern("  ", db, &(cx.intern));
            println!("{}", cx.func.print(&literals));
        }


        // Build DAE system in last_block, end it with jump to ret_block (in builder.finish())
        let mut dae_system = DaeSystem::new(&mut cx, topology);

        // Append new block after last block (it will become the new last block)
        let ret_block = cx.func.layout.ret_block().unwrap();
        let entry_block = cx.func.layout.entry_block().unwrap();

        // Switch all jumps to ret_block to exit_block
        // cx.func.dfg.retarget_jumps(ret_block, exit_block);

        if dump_unoptimized_mir {
            println!("Interner for unoptimized MIR after DAE added");
            print_intern("  ", db, &(cx.intern));
            println!("{}", cx.func.print(&literals));
        }
        println!("entry block {:?}", cx.func.layout.entry_block().unwrap());
        println!("last block {:?}", cx.func.layout.last_block().unwrap());
        println!("ret block {:?}", cx.func.layout.ret_block());

        debug_assert!(cx.func.validate());
        cx.compute_cfg();
        let gvn = cx.optimize(OptimiziationStage::PostDerivative);
        dae_system.sparsify(&mut cx);

        debug_assert!(cx.func.validate());

        if dump_unoptimized_mir {
            println!("Interner post derivative optimization");
            print_intern("  ", db, &(cx.intern));
            println!("{}", cx.func.print(&literals));
        }
        

        cx.refresh_op_dependent_insts();
        let mut init = Initialization::new(&mut cx, gvn);
        let node_collapse = NodeCollapse::new(&init, &dae_system, &cx);
        debug_assert!(cx.func.validate());

        debug_assert!(init.func.validate());
        
        // TODO: refactor param intilization to use tables
        let inst_params: Vec<_> = module
            .params
            .iter()
            .filter_map(|(param, info)| info.is_instance.then_some(*param))
            .collect();
        init.intern.insert_param_init(db, &mut init.func, literals, false, true, &inst_params);

        let mut model_param_setup = Function::default();
        let model_params: Vec<_> = module.params.keys().copied().collect();
        let mut model_param_intern = HirInterner::default();
        model_param_intern.insert_param_init(
            db,
            &mut model_param_setup,
            literals,
            false,
            true,
            &model_params,
        );
        cx.cfg.compute(&model_param_setup);
        simplify_cfg(&mut model_param_setup, &mut cx.cfg);
        sparse_conditional_constant_propagation(&mut model_param_setup, &cx.cfg);
        simplify_cfg(&mut model_param_setup, &mut cx.cfg);

        println!("{:?}", model_param_setup);

        println!("{:?}", init.func);

        CompiledModule {
            eval: cx.func,
            intern: cx.intern,
            info: module,
            dae_system,
            init,
            model_param_intern,
            model_param_setup,
            node_collapse,
        }
    }
}
