use bitset::{BitSet, HybridBitSet, SparseBitMatrix};
use mir::{Block, ControlFlowGraph, DominatorTree, Function, Inst, InstructionData, Value};
use std::collections::{HashMap, HashSet};

// Constructs
// - a map from block to corresponding top level loop header block
// - a map from top level loop header block to all blocks
//   within that loop (including the header block).
pub fn loop_block_map(
    func: &Function,
    cfg: &ControlFlowGraph,
) -> (HashMap<Block, Vec<Block>>, HashMap<Block, Block>) {
    let mut loop_blocks_map: HashMap<Block, Vec<Block>> = HashMap::new();
    let mut header_map: HashMap<Block, Block> = HashMap::new();

    // Reverse postordering
    let mut rpo = cfg.reverse_postorder(func);
    rpo.reset();

    // Scan function's blocks in reverse postorder
    for bb in rpo {
        // Go through instructions of a block
        let mut bb_cursor = func.layout.block_inst_cursor(bb);
        let mut head: Option<Block> = None;
        let mut tail: Option<Block> = None;
        while let Some(inst) = bb_cursor.next(&func.layout) {
            // Is it a branch
            if let InstructionData::Branch { else_dst, loop_entry, .. } = func.dfg.insts[inst] {
                if loop_entry {
                    head = bb.into();
                    tail = else_dst.into();
                    loop_blocks_map.insert(bb, Vec::new());
                }
            }
            // Did we reach end of the loop
            if Some(bb) == tail {
                // No head, no tail
                head = None;
                tail = None;
            }

            // Are we within a loop
            if let Some(bb) = head {
                // Add to header_map
                header_map.insert(bb, head.unwrap());
                // Add to vector of loop blocks
                loop_blocks_map
                    .get_mut(&head.unwrap())
                    .expect("Blocks vector not in map.")
                    .push(bb);
            }
        }
    }
    (loop_blocks_map, header_map)
}

pub fn propagate_taint(
    func: &Function,
    dom_tree: &DominatorTree,
    cfg: &ControlFlowGraph,
    tainted: impl Iterator<Item = Value>,
    tainted_insts: &mut BitSet<Inst>,
) {
    tainted_insts.ensure(func.dfg.num_insts());

    let mut solver = TaintSolver {
        dom_tree,
        func,
        inst_queue: Vec::new(),
        tainted_blocks: BitSet::new_empty(func.layout.num_blocks()),
        tainted_insts,
        cfg,
        bb_queue: Vec::new(),
    };

    // Taint all instructions that use the values in tainted list
    for val in tainted {
        for use_ in func.dfg.uses(val) {
            let inst = func.dfg.use_to_operand(use_).0;
            solver.taint_inst(inst)
        }
    }

    solver.solve();
}

struct TaintSolver<'a> {
    dom_tree: &'a DominatorTree,
    cfg: &'a ControlFlowGraph,
    func: &'a Function,
    tainted_insts: &'a mut BitSet<Inst>,

    inst_queue: Vec<Inst>,
    bb_queue: Vec<Block>,
    tainted_blocks: BitSet<Block>,
}

impl TaintSolver<'_> {
    fn taint_inst(&mut self, inst: Inst) {
        if self.tainted_insts.insert(inst) {
            self.inst_queue.push(inst);
        }
    }

    fn taint_block(&mut self, mut bb: Block, end: Option<Block>) {
        // TODO: benchmark whether permanent hashmap is faster?
        let mut visited = HybridBitSet::new_empty();
        // While there are blocks in bb_queue, repeat this loop
        loop {
            // Loop through blocks until end is reached
            loop {
                if Some(bb) == end {
                    break;
                }
                // If block is not alredy tainted, taint all instructions in the block
                if self.tainted_blocks.insert(bb) {
                    for inst in self.func.layout.block_insts(bb) {
                        self.taint_inst(inst);
                    }
                }
                // Get iterator for block successors
                let mut successors = self.cfg.succ_iter(bb);
                // enumlate tail recursion
                if let Some(succ) =
                    successors.find(|&bb| visited.insert(bb, self.func.layout.num_blocks()))
                {
                    // We have a successor that has not been visited yet, next for tainting
                    bb = succ;
                } else {
                    // No successor found, inner loop is done
                    break;
                }
                // Add all not visited successors to bb_queue
                for succ in successors {
                    if visited.insert(succ, self.func.layout.num_blocks()) {
                        self.bb_queue.push(succ);
                    }
                }
            }
            // Pop a block from bb_queue, process it
            if let Some(next) = self.bb_queue.pop() {
                bb = next
            } else {
                break;
            }
        }
    }

    fn solve(&mut self) {
        // Construct a map from block to the header block of the outermost loop.
        // If a block is not in that map it does not belong to any loop.
        let (loop_blocks_map, header_map) = loop_block_map(self.func, self.cfg);

        // Tainted loops
        let mut tainted_loops: HashSet<Block> = HashSet::new();

        // While there are instructions in the queue
        while !self.inst_queue.is_empty() {
            // Propagate taint from tainted instructions onward
            while let Some(inst) = self.inst_queue.pop() {
                match self.func.dfg.insts[inst] {
                    InstructionData::Branch { then_dst, else_dst, .. } => {
                        // For branch instructions taint the then and the else destination blocks
                        let bb = self.func.layout.inst_block(inst).unwrap();
                        let end = self.dom_tree.ipdom(bb);
                        self.taint_block(then_dst, end);
                        self.taint_block(else_dst, end);
                        continue;
                    }
                    InstructionData::Jump { destination } => {
                        // For jump instructions, taint all phi nodes in destination
                        for inst in self.func.layout.block_insts(destination) {
                            if self.func.dfg.insts[inst].is_phi() {
                                self.taint_inst(inst)
                            } else {
                                break;
                            }
                        }
                        continue;
                    }
                    _ => (),
                }

                // TODO: If instruction is in a loop and that loop is not tainted yet,
                // taint all blocks of the outermost loop that contains it.
                // Need a map block -> header block of the outermost loop containing that block
                // A loop is identified by its header block.

                // Find all uses of an instruction, taint instructions that correspond to uses
                for use_ in self.func.dfg.inst_uses(inst) {
                    let user = self.func.dfg.use_to_operand(use_).0;
                    self.taint_inst(user);
                }
            }

            // Propagate taint from tainted instructions to loops holding those instructions
            // Collect all loop header blocks
            let hdrs: HashSet<Block> = self
                .tainted_insts
                .iter()
                .filter_map(|inst| {
                    if let Some(bb) = self.func.layout.inst_block(inst) {
                        return header_map.get(&bb).cloned();
                    }
                    None
                })
                .collect();
            // Taint all loops
            // This adds new instructions to inst_queue
            for hdr in hdrs {
                // Taint loop only once
                if !tainted_loops.contains(&hdr) {
                    if let Some(bb_vec) = loop_blocks_map.get(&hdr) {
                        // Taint all instructions in all blocks of a loop
                        for bb in bb_vec.iter() {
                            // Taint all instructions in a block
                            let mut bb_cursor = self.func.layout.block_inst_cursor(*bb);
                            while let Some(inst) = bb_cursor.next(&self.func.layout) {
                                self.taint_inst(inst);
                            }
                        }
                    }
                    // Mark loop as tainted
                    tainted_loops.insert(hdr);
                }
            }
            // Repeat until inst_queue is empty
        }
    }
}

pub fn propagate_direct_taint(
    func: &Function,
    dom_frontiers: &SparseBitMatrix<Block, Block>,
    tainted: impl Iterator<Item = Value>,
    tainted_insts: &mut BitSet<Inst>,
) {
    tainted_insts.ensure(func.dfg.num_insts());
    let mut solver =
        DirectTaintSolver { func, inst_queue: Vec::new(), tainted_insts, dom_frontiers };

    // Go through all tainted values
    for val in tainted {
        // Go through all uses of a value
        for use_ in func.dfg.uses(val) {
            // Get the instruction where the use appears
            let inst = func.dfg.use_to_operand(use_).0;
            // Taint instruction, add to instructuon queue
            solver.taint_inst(inst)
        }
    }

    // Solve taint
    solver.solve();
}

struct DirectTaintSolver<'a> {
    dom_frontiers: &'a SparseBitMatrix<Block, Block>,
    func: &'a Function,
    tainted_insts: &'a mut BitSet<Inst>,
    inst_queue: Vec<Inst>,
}

impl DirectTaintSolver<'_> {
    fn taint_inst(&mut self, inst: Inst) {
        if self.tainted_insts.insert(inst) {
            self.inst_queue.push(inst);
        }
    }
    fn taint_dom_frontier_phis(&mut self, frontiers: impl Iterator<Item = Block>) {
        for dom_frontier in frontiers {
            for inst in self.func.layout.block_insts(dom_frontier) {
                if self.func.dfg.insts[inst].is_phi() {
                    self.taint_inst(inst)
                } else {
                    break;
                }
            }
        }
    }

    fn solve(&mut self) {
        while let Some(inst) = self.inst_queue.pop() {
            if let InstructionData::Branch { then_dst, else_dst, .. } = self.func.dfg.insts[inst] {
                if let Some(frontiers) = self.dom_frontiers.row(then_dst) {
                    self.taint_dom_frontier_phis(frontiers.iter());
                }
                if let Some(frontiers) = self.dom_frontiers.row(else_dst) {
                    self.taint_dom_frontier_phis(frontiers.iter());
                }
            } else {
                for use_ in self.func.dfg.inst_uses(inst) {
                    let user = self.func.dfg.use_to_operand(use_).0;
                    self.taint_inst(user);
                }
            }
        }
    }
}
