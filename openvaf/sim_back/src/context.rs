use bitset::{BitSet, SparseBitMatrix};
use hir::CompilationDB;
use hir_lower::{HirInterner, MirBuilder, PlaceKind};
use lasso::Rodeo;
use mir::{InstructionData, Block, ControlFlowGraph, DominatorTree, Function, Inst, Value};
use mir_opt::{
    aggressive_dead_code_elimination, dead_code_elimination, inst_combine, propagate_direct_taint,
    propagate_taint, simplify_cfg, simplify_cfg_no_phi_merge,
    sparse_conditional_constant_propagation, GVN,
};
use stdx::packed_option::PackedOption;

use crate::ModuleInfo;

pub(crate) struct Context<'a> {
    pub(crate) func: Function,
    pub(crate) cfg: ControlFlowGraph,
    pub(crate) dom_tree: DominatorTree,
    pub(crate) intern: HirInterner,
    pub(crate) db: &'a CompilationDB,
    pub(crate) module: &'a ModuleInfo,
    pub(crate) output_values: BitSet<Value>,
    pub(crate) op_dependent_insts: BitSet<Inst>,
    pub(crate) op_dependent_vals: Vec<Value>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum OptimiziationStage {
    Initial,
    PostDerivative,
    Final,
}

impl<'a> Context<'a> {
    pub fn new(db: &'a CompilationDB, literals: &mut Rodeo, module: &'a ModuleInfo) -> Self {
        let (mut func, mut intern) = MirBuilder::new(
            db,
            module.module,
            &|kind| match kind {
                PlaceKind::Contribute { .. }
                | PlaceKind::ImplicitResidual { .. }
                | PlaceKind::CollapseImplicitEquation(_)
                | PlaceKind::IsVoltageSrc(_) 
                | PlaceKind::BoundStep => true,
                PlaceKind::Var(var) => module.op_vars.contains_key(&var),
                _ => false,
            },
            &mut module.op_vars.keys().copied(),
        )
        .with_equations()
        .with_tagged_writes()
        .build(literals);
        // TODO hidden state
        intern.insert_var_init(db, &mut func, literals);

        Context {
            output_values: BitSet::new_empty(func.dfg.num_values()),
            func,
            cfg: ControlFlowGraph::new(),
            dom_tree: DominatorTree::default(),
            intern,
            db,
            module,
            op_dependent_insts: BitSet::new_empty(0),
            op_dependent_vals: Vec::new(),
        }
    }

    pub fn optimize(&mut self, stage: OptimiziationStage) -> GVN {
        if stage == OptimiziationStage::Initial {
            dead_code_elimination(&mut self.func, &self.output_values);
        }
        sparse_conditional_constant_propagation(&mut self.func, &self.cfg);
        inst_combine(&mut self.func);
        if stage == OptimiziationStage::Final {
            simplify_cfg(&mut self.func, &mut self.cfg);
        } else {
            simplify_cfg_no_phi_merge(&mut self.func, &mut self.cfg);
        }
        self.compute_domtree(true, true, false);

        let mut gvn = GVN::default();
        gvn.init(&self.func, &self.dom_tree, self.intern.params.len() as u32);
        gvn.solve(&mut self.func);
        gvn.remove_unnecessary_insts(&mut self.func, &self.dom_tree);

        if stage == OptimiziationStage::Final {
            let mut control_dep = SparseBitMatrix::new_square(0);
            self.dom_tree.compute_postdom_frontiers(&self.cfg, &mut control_dep);
            aggressive_dead_code_elimination(
                &mut self.func,
                &mut self.cfg,
                &|val, _| self.output_values.contains(val),
                &control_dep,
            );
            simplify_cfg(&mut self.func, &mut self.cfg);
        }

        gvn
    }

    pub fn compute_cfg(&mut self) {
        self.cfg.compute(&self.func);
    }

    pub fn compute_domtree(&mut self, dom: bool, pdom: bool, postorder: bool) {
        self.dom_tree.compute(&self.func, &self.cfg, dom, pdom, postorder);
    }

    pub fn compute_outputs(&mut self, contributes: bool) {
        self.output_values.clear();
        self.output_values.ensure(self.func.dfg.num_values() + 1);
        if contributes {
            self.output_values
                .extend(self.intern.outputs.values().copied().filter_map(PackedOption::expand));
        } else {
            for (kind, val) in self.intern.outputs.iter() {
                if matches!(kind, PlaceKind::Var(var) if self.module.op_vars.contains_key(var))
                    || matches!(kind, PlaceKind::CollapseImplicitEquation(_) | PlaceKind::BoundStep)
                {
                    self.output_values.insert(val.unwrap_unchecked());
                }
            }
        }
    }

    pub fn init_op_dependent_insts(&mut self, dom_frontiers: &mut SparseBitMatrix<Block, Block>) {
        self.dom_tree.compute_dom_frontiers(&self.cfg, dom_frontiers);
        let dfg = &mut self.func.dfg;
        self.op_dependent_insts.ensure(dfg.num_insts());

        for (cb, uses) in self.intern.callback_uses.iter_mut_enumerated() {
            if self.intern.callbacks[cb].is_noise() {
                uses.retain(|&inst| {
                    if self.func.layout.inst_block(inst).is_none() {
                        return false;
                    }
                    self.op_dependent_insts.insert(inst);
                    for &result in dfg.inst_results(inst) {
                        self.op_dependent_vals.push(result);
                    }
                    true
                })
            }
        }
        for (param, &val) in self.intern.params.iter() {
            if !dfg.value_dead(val) && param.op_dependent() {
                self.op_dependent_vals.push(val)
            }
        }
        loop {
            // Repeat taint propagation until no new command in a loop body is marked as op dependent
            propagate_direct_taint(
                &self.func,
                dom_frontiers,
                self.op_dependent_vals.iter().copied(),
                &mut self.op_dependent_insts,
            );
            if !self.loop_op_dependence() {
                break;
            }
        }
    }

    pub fn refresh_op_dependent_insts(&mut self) {
        let dfg = &mut self.func.dfg;
        self.op_dependent_vals.clear();
        self.op_dependent_insts.clear();
        self.op_dependent_insts.ensure(dfg.num_insts());
        for (cb, uses) in self.intern.callback_uses.iter_mut_enumerated() {
            if self.intern.callbacks[cb].op_dependent() {
                uses.retain(|&inst| {
                    if self.func.layout.inst_block(inst).is_none() {
                        return false;
                    }
                    self.op_dependent_insts.insert(inst);
                    for &result in dfg.inst_results(inst) {
                        self.op_dependent_vals.push(result);
                    }
                    true
                })
            }
        }
        for (param, &val) in self.intern.params.iter() {
            if !dfg.value_dead(val) && param.op_dependent() {
                self.op_dependent_vals.push(val)
            }
        }
        loop {
            // Repeat taint propagation until no new command in a loop body is marked as op dependent
            propagate_taint(
                &self.func,
                &self.dom_tree,
                &self.cfg,
                self.op_dependent_vals.iter().copied(),
                &mut self.op_dependent_insts,
            );
            if !self.loop_op_dependence() {
                break;
            }
        }
    }

    pub fn is_loop_op_dependent(&self, blk1: Option<Block>, blk2: Option<Block>) -> bool {
        // Traverse blocks
        let mut blk = blk1;
        while blk.is_some() && blk!=blk2 {
            // Traverse block instructions
            let mut bb_cursor = self.func.layout.block_inst_cursor(blk.unwrap());
            while let Some(inst) = bb_cursor.next(&self.func.layout) {
                // Is it op dependent
                if self.op_dependent_insts.contains(inst) {
                    return true;
                }
            }
            blk = self.func.layout.next_block(blk.unwrap());
        }
        return false;
    }

    pub fn make_loop_op_dependent(&mut self, blk1: Option<Block>, blk2: Option<Block>) -> bool {
        let func = &self.func;
        // Traverse blocks
        let mut blk = blk1;
        let mut changed = false;
        while blk.is_some() && blk!=blk2 {
            // Traverse block instructions
            let mut bb_cursor = self.func.layout.block_inst_cursor(blk.unwrap());
            while let Some(inst) = bb_cursor.next(&self.func.layout) {
                // Mark instruction op dependent
                changed = changed || self.op_dependent_insts.insert(inst);
            }
            blk = func.layout.next_block(blk.unwrap());
        }
        return changed;
    }

    pub fn loop_op_dependence(&mut self) -> bool {
        // Traverse blocks in the module MIR
        // Look for a block that contains a branch instruction that is labelled as loop
        let mut blocks = self.func.layout.blocks_cursor();
        let mut changed = false;
        while let Some(bb) = blocks.next(&self.func.layout) {
            // Traverse instructions
            let mut bb_cursor = self.func.layout.block_inst_cursor(bb);
            while let Some(inst) = bb_cursor.next(&self.func.layout) {
                match self.func.dfg.insts[inst] {
                    InstructionData::Branch { else_dst: else_block, loop_entry: is_loop, .. } => {
                        if is_loop && self.is_loop_op_dependent(bb.into(), else_block.into()) {
                            changed = changed || self.make_loop_op_dependent(bb.into(), else_block.into());
                        }
                    }, 
                    _ => ()
                }
            }
        }
        return changed;
    }
}
