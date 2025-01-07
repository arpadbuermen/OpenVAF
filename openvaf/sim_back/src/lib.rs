use hir::{BranchWrite, CompilationDB, Node};
use hir_lower::{CurrentKind, HirInterner, ImplicitEquation, ParamKind};
use lasso::Rodeo;
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

fn print_intern(pfx: &str, db: &CompilationDB, intern: &HirInterner) {
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
        dump_mir: bool, 
    ) -> CompiledModule<'a> {
        let mut cx = Context::new(db, literals, module);
        cx.compute_outputs(true);
        cx.compute_cfg();
        cx.optimize(OptimiziationStage::Initial);
        debug_assert!(cx.func.validate());

        let topology = Topology::new(&mut cx);
        debug_assert!(cx.func.validate());
        let mut dae_system = DaeSystem::new(&mut cx, topology);
        debug_assert!(cx.func.validate());
        cx.compute_cfg();
        let gvn = cx.optimize(OptimiziationStage::PostDerivative);
        dae_system.sparsify(&mut cx);

        // For debugging purposes - print parameters
        let debugging = dump_mir; //  && cfg!(debug_assertions);
        if debugging {
            
            let cu = db.compilation_unit();
            println!("Compilation unit: {}", cu.name(db));
                        
            let m = module.module;
            println!("Module: {:?}", m.name(db));
            println!("Ports: {:?}", m.ports(db));
            println!("Internal nodes: {:?}", m.internal_nodes(db));
                        
            let str = format!("{dae_system:#?}");
            println!("{}", str);
            println!("");
        }
        
        debug_assert!(cx.func.validate());

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

        let cm = CompiledModule {
            eval: cx.func,
            intern: cx.intern,
            info: module,
            dae_system,
            init,
            model_param_intern,
            model_param_setup,
            node_collapse,
        };

        if debugging {
            println!("Model param intern");
            print_intern("  ", db, &cm.model_param_intern);
            println!("Model param setup");
            println!("{}", cm.model_param_setup.print(literals));
            println!("");

            println!("Init intern");
            print_intern("  ", db, &cm.init.intern);
            println!("Init cached values");
            cm.init.cached_vals.iter().for_each(|(val, slot)| {
                println!("  {:?} -> {:?}", val, slot);
            });
            cm.init.cache_slots.iter_enumerated().for_each(|(slot, (cls, ty))| {
                println!("  {:?} -> {:?} {:?}", slot, cls, ty);
            });
            println!("Init");
            println!("{}", cm.init.func.print(literals));
            println!("");

            println!("Evaluation intern");
            print_intern("  ", db, &cm.intern);
            println!("Evaluation - trailing arguments are cache slots?");
            println!("{}", cm.eval.print(literals));
            println!("");
        }
        cm
    }
}
