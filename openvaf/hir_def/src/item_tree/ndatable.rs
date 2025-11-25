// Natues, disciplines, and attributes table

use arena::Idx;
use std::collections::HashMap;
use std::sync::Arc;

use super::{Discipline, ItemTree, Nature};
use crate::db::HirDefDB;
use basedb::FileId;

#[derive(PartialEq, Eq, Debug)]
pub struct NDATable {
    pub nature_name_map: std::collections::HashMap<String, Idx<Nature>>,
    pub discipline_name_map: std::collections::HashMap<String, Idx<Discipline>>,
}

impl NDATable {
    pub fn new() -> Self {
        Self { nature_name_map: HashMap::new(), discipline_name_map: HashMap::new() }
    }
    pub fn nda_table_query(db: &dyn HirDefDB, file: FileId) -> Arc<NDATable> {
        let ctx = NDACtx::new(db, file);
        let mut table = NDATable::new();
        ctx.fill_table(&mut table);
        Arc::new(table)
    }
}

pub(super) struct NDACtx {
    tree: Arc<ItemTree>,
}

impl NDACtx {
    pub fn new(db: &dyn HirDefDB, file: FileId) -> Self {
        Self { tree: db.item_tree(file) }
    }

    // Fill it with natures and disciplines
    pub fn fill_table(&self, table: &mut NDATable) {
        // Add natures
        for (idx, nature) in self.tree.data.natures.iter_enumerated() {
            table.nature_name_map.insert(nature.name.to_string(), idx);
        }

        // Add disciplines.
        for (idx, discipline) in self.tree.data.disciplines.iter_enumerated() {
            table.discipline_name_map.insert(discipline.name.to_string(), idx);
        }
    }
}
