use crate::GeneralOsdiCall;
use openvaf_data_structures::index_vec::{define_index_type, index_vec, IndexVec};
use openvaf_data_structures::{BitSet, HashMap, HashSet};
use openvaf_hir::{BranchId, DisciplineAccess};
use openvaf_ir::ids::NetId;
use openvaf_ir::Unknown;
use openvaf_middle::cfg::ControlFlowGraph;
use openvaf_middle::{Local, LocalKind, Mir, VariableLocalKind};
use std::mem::swap;
use std::ops::{Index, IndexMut};

define_index_type! {
    pub struct Connection = u8;
    DISPLAY_FORMAT = "connection_{}";
    DISABLE_MAX_INDEX_CHECK = cfg!(not(debug_assertions));
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionInfo {
    pub hi: NetId,
    pub lo: NetId,
    pub limited: bool,
    pub derivatives: IndexVec<BranchId, Option<Local>>,
}

impl ConnectionInfo {
    pub fn new(
        hi: NetId,
        lo: NetId,
        mir: &Mir<GeneralOsdiCall>,
        cfg: &mut ControlFlowGraph<GeneralOsdiCall>,
    ) -> Self {
        let mut derivatives = index_vec![None; mir.branches.len()];

        for (local, declaration) in cfg.locals.clone().iter_enumerated() {
            if let LocalKind::Branch(DisciplineAccess::Flow, branch, VariableLocalKind::User) =
                declaration.kind
            {
                let derivative =
                    cfg.demand_derivative_unchecked(local, Unknown::BranchPotential(hi, lo));
                derivatives[branch] = Some(derivative)
            }
        }

        Self {
            hi,
            lo,
            limited: false,
            derivatives,
        }
    }
}

define_index_type! {
    pub struct MatrixEntry = u16;
    DISPLAY_FORMAT = "connection_{}";
    DISABLE_MAX_INDEX_CHECK = cfg!(not(debug_assertions));
}

pub struct Stamp {
    matrix_entry: MatrixEntry,
    neg: bool,
    val: Local,
}

pub struct MatrixEntryInfo {
    node: NetId,
    derive_by: NetId,
}

pub struct CircuitTopology {
    pub connections: IndexVec<Connection, ConnectionInfo>,
    pub matrix_entries: IndexVec<MatrixEntry, MatrixEntryInfo>,
    pub matrix_stamps: Vec<Stamp>,
    pub matrix_stamp_locals: BitSet<Local>,
}

impl CircuitTopology {
    pub fn new(mir: &Mir<GeneralOsdiCall>, cfg: &mut ControlFlowGraph<GeneralOsdiCall>) -> Self {
        let mut res = HashSet::with_capacity(mir.branches.len());
        let connections: IndexVec<_, _> = mir
            .branches
            .iter()
            .filter_map(|branch| {
                let (lo, hi) = if branch.lo <= branch.hi {
                    (branch.lo, branch.hi)
                } else {
                    (branch.hi, branch.lo)
                };

                if res.insert((lo, hi)) {
                    Some(ConnectionInfo::new(lo, hi, mir, cfg))
                } else {
                    None
                }
            })
            .collect();

        let mut res = Self {
            matrix_entries: IndexVec::with_capacity(4 * connections.len()),
            matrix_stamps: Vec::with_capacity(16 * connections.len()),
            connections: IndexVec::default(), // Placeholder so we can appease the borrow checker
            matrix_stamp_locals: BitSet::new_empty(cfg.locals.len_idx()),
        };

        for connection in &connections {
            for (branch, derivative) in connection.derivatives.iter_enumerated() {
                if let Some(derivative) = derivative {
                    let hi_by_hi = res.create_matrix_entry(connection.hi, mir[branch].hi);
                    let lo_by_lo = res.create_matrix_entry(connection.lo, mir[branch].lo);
                    let hi_by_lo = res.create_matrix_entry(connection.hi, mir[branch].lo);
                    let lo_by_hi = res.create_matrix_entry(connection.lo, mir[branch].hi);

                    res.matrix_stamp_locals.insert(*derivative);

                    res.matrix_stamps.push(Stamp {
                        matrix_entry: hi_by_hi,
                        neg: false,
                        val: *derivative,
                    });

                    res.matrix_stamps.push(Stamp {
                        matrix_entry: lo_by_lo,
                        neg: false,
                        val: *derivative,
                    });

                    res.matrix_stamps.push(Stamp {
                        matrix_entry: hi_by_lo,
                        neg: false,
                        val: *derivative,
                    });

                    res.matrix_stamps.push(Stamp {
                        matrix_entry: lo_by_hi,
                        neg: false,
                        val: *derivative,
                    });
                }
            }
        }

        res.connections = connections;
        res
    }

    fn create_matrix_entry(&mut self, node: NetId, derive_by: NetId) -> MatrixEntry {
        if let Some(entry) = self.find_matrix_entry(node, derive_by) {
            entry
        } else {
            self.matrix_entries
                .push(MatrixEntryInfo { node, derive_by })
        }
    }

    pub fn find_matrix_entry(&mut self, node: NetId, derive_by: NetId) -> Option<MatrixEntry> {
        self.matrix_entries
            .position(|x| (x.node == node) & (x.derive_by == derive_by))
    }

    pub fn is_connected(&self, mut node_a: NetId, mut node_b: NetId) -> bool {
        self.find_connection(node_a, node_b).is_some()
    }

    pub fn find_connection_info_mut(
        &mut self,
        mut hi: NetId,
        mut lo: NetId,
    ) -> Option<&mut ConnectionInfo> {
        if hi < lo {
            swap(&mut hi, &mut lo)
        }
        self.connections
            .iter_mut()
            .find(|conn| (conn.hi == hi) & (conn.lo == lo))
    }

    pub fn find_connection(&self, mut node_a: NetId, mut node_b: NetId) -> Option<Connection> {
        if node_b < node_a {
            swap(&mut node_a, &mut node_b)
        }
        self.connections
            .position(|conn| (conn.hi == node_a) & (conn.lo == node_b))
    }

    pub fn find_connection_info(&self, mut hi: NetId, mut lo: NetId) -> Option<&ConnectionInfo> {
        if hi < lo {
            swap(&mut hi, &mut lo)
        }
        self.connections
            .iter()
            .find(|conn| (conn.hi == hi) & (conn.lo == lo))
    }

    pub fn connections_to<'a, const N: usize>(
        &'a self,
        nodes: &'a [NetId; N],
    ) -> impl Iterator<Item = &ConnectionInfo> + 'a {
        self.connections
            .iter()
            .filter(move |x| nodes.contains(&x.hi) | nodes.contains(&x.lo))
    }
}

impl Index<Connection> for CircuitTopology {
    type Output = ConnectionInfo;

    fn index(&self, index: Connection) -> &Self::Output {
        self.connections.index(index)
    }
}
impl IndexMut<Connection> for CircuitTopology {
    fn index_mut(&mut self, index: Connection) -> &mut Self::Output {
        self.connections.index_mut(index)
    }
}