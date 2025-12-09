use bitset::BitSet;

use crate::{Block, ControlFlowGraph};
use smallvec::SmallVec;

/// Postorder traversal of a graph.
///
/// Postorder traversal is when each node is visited after all of its
/// successors, except when the successor is only reachable by a back-edge
///
///
/// ```text
///
///         A
///        / \
///       /   \
///      B     C
///       \   /
///        \ /
///         D
/// ```
///
/// A Postorder traversal of this graph is `D B C A` or `D C B A`
// TODO: rewrite this without using recursion
pub struct Postorder<'a> {
    cfg: &'a ControlFlowGraph,
    visited: BitSet<Block>,
    result: SmallVec<[Block; 32]>,
    index: usize,
}

impl<'a> Postorder<'a> {
    pub fn new(cfg: &'a ControlFlowGraph, root: Block) -> Self {
        let mut po = Postorder {
            cfg,
            visited: BitSet::new_empty(cfg.data.len()),
            result: SmallVec::new(),
            index: 0,
        };

        po.dfs(root);
        po
    }

    fn dfs(&mut self, bb: Block) {
        if !self.visited.insert(bb) {
            return;
        }
        for succ in self.cfg.successors(bb).iter() {
            self.dfs(succ);
        }

        self.result.push(bb);
    }
}

impl<'a> Iterator for Postorder<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.result.len() {
            let block = self.result[self.index];
            self.index += 1;
            Some(block)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.result.len() - self.index;
        (len, Some(len))
    }
}

/// Reverse postorder traversal of a graph
///
/// Reverse postorder is the reverse order of a postorder traversal.
/// This is different to a preorder traversal and represents a natural
/// linearization of control-flow.
///
/// ```text
///
///         A
///        / \
///       /   \
///      B     C
///       \   /
///        \ /
///         D
/// ```
///
/// A reverse postorder traversal of this graph is either `A B C D` or `A C B D`
/// Note that for a graph containing no loops (i.e., A DAG), this is equivalent to
/// a topological sort.
///
/// Construction of a `ReversePostorder` traversal requires doing a full
/// postorder traversal of the graph, therefore this traversal should be
/// constructed as few times as possible. Use the `reset` method to be able
/// to re-use the traversal
#[derive(Clone, Debug)]
pub struct ReversePostorder {
    blocks: Vec<Block>,
    idx: usize,
}

impl ReversePostorder {
    pub fn new(cfg: &ControlFlowGraph, root: Block) -> Self {
        let blocks: Vec<_> = Postorder::new(cfg, root).collect();

        let len = blocks.len();

        ReversePostorder { blocks, idx: len }
    }

    pub fn reset(&mut self) {
        self.idx = self.blocks.len();
    }
}

impl Iterator for ReversePostorder {
    type Item = Block;

    fn next(&mut self) -> Option<Block> {
        if self.idx == 0 {
            return None;
        }
        self.idx -= 1;

        self.blocks.get(self.idx).copied()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.idx, Some(self.idx))
    }
}

impl ExactSizeIterator for ReversePostorder {
    fn len(&self) -> usize {
        self.idx
    }
}
