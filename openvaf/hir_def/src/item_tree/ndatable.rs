// Natues, disciplines, and attributes table

// Option<PackedOption<u32>>
//   None .. not resolved (error)
//   Some(None) .. not specified (null)

use arena::{Arena, Idx, IdxRange};
use std::collections::HashMap;
use std::sync::Arc;

use super::{
    Discipline, DisciplineAttrKind, 
    ItemTree, Nature, NatureRef, NatureRefKind, 
};
use basedb::{AstIdMap, FileId};
use crate::db::HirDefDB;
use syntax::ConstExprValue;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NDANatureIdResolution {
    NDAId(Idx<NatureEntry>), 
    NotGiven, 
    Unresolved
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct NatureEntry {
    pub name: String, 
    pub parent: NDANatureIdResolution, 
    pub idt_nature: NDANatureIdResolution,
    pub ddt_nature: NDANatureIdResolution,
    pub attr_range: IdxRange<AttrEntry>, 
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct DisciplineEntry {
    pub name: String,
    pub flow_nature: NDANatureIdResolution, 
    pub potential_nature: NDANatureIdResolution, 
    pub attr_range: IdxRange<AttrEntry>,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct AttrEntry {
    pub name: String,
    pub value: ConstExprValue,
}

#[derive(PartialEq, Eq, Debug)]
pub struct NDATable {
    pub natures: Arena<NatureEntry>,
    pub disciplines: Arena<DisciplineEntry>,
    pub attributes: Arena<AttrEntry>,
    nature_name_map: std::collections::HashMap<String, Idx<NatureEntry>>,
    discipline_name_map: std::collections::HashMap<String, Idx<DisciplineEntry>>,
}

impl NDATable {
    pub fn new() -> Self {
        Self {
            natures: Arena::new(),
            disciplines: Arena::new(),
            attributes: Arena::new(),
            nature_name_map: HashMap::new(), 
            discipline_name_map: HashMap::new(), 
        }
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
        Self { 
            tree: db.item_tree(file), 
        }
    }

    // Create a nature entry, but do not fill in parent, idt_nature, ddt_nature yet    
    fn create_nature(&self, nature: &Nature, table: &mut NDATable) -> NatureEntry {
        let i1 = table.attributes.next_key();
        for ndx in nature.attrs.clone() {
            let attr = &self.tree.data.nature_attrs[ndx];
            if let Some(value) = &attr.value {
                table.attributes.push(AttrEntry { name: attr.name.to_string(), value: value.clone() });
            }
        }
        let i2 = table.attributes.next_key();
        NatureEntry {
            name: nature.name.to_string(),
            parent: NDANatureIdResolution::Unresolved,
            idt_nature: NDANatureIdResolution::Unresolved,
            ddt_nature: NDANatureIdResolution::Unresolved,
            attr_range: IdxRange::new(i1..i2),
        }
    }

    fn create_derived_nature(&self, discipline: &Discipline, is_flow: bool, name: String, parent_ndx: &Idx<NatureEntry>, table: &mut NDATable) -> NatureEntry {
        let i1 = table.attributes.next_key();
        // Add parent attribute
        table.attributes.push( AttrEntry {
            name: "parent".into(), 
            value: if is_flow {
                ConstExprValue::String(discipline.flow.as_ref().unwrap().0.name.to_string())
            } else {
                ConstExprValue::String(discipline.potential.as_ref().unwrap().0.name.to_string())
            }
        });
        // Collect attribute overrides from discipline
        for ndx in discipline.extra_attrs.clone() {
            let attr = &self.tree.data.discipline_attrs[ndx];
            if let Some(value) = &attr.value {
                if (is_flow && attr.kind==DisciplineAttrKind::FlowOverwrite) ||
                    (!is_flow && attr.kind==DisciplineAttrKind::PotentialOverwrite) {
                    table.attributes.push(AttrEntry { name: attr.name.to_string(), value: value.clone() });
                }
            }
        }
        let i2 = table.attributes.next_key();
        NatureEntry {
            name: name.clone(),
            parent: NDANatureIdResolution::NDAId(*parent_ndx),
            idt_nature: NDANatureIdResolution::Unresolved,
            ddt_nature: NDANatureIdResolution::Unresolved,
            attr_range: IdxRange::new(i1..i2),
        }
    }

    pub fn create_discipline(&self, discipline: &Discipline, flow_index: Option<Idx<NatureEntry>>, potential_index: Option<Idx<NatureEntry>>, table: &mut NDATable) -> DisciplineEntry {
        let i1 = table.attributes.next_key();
        for ndx in discipline.extra_attrs.clone() {
            let attr = &self.tree.data.discipline_attrs[ndx];
            // Flow and potential overrides are renamed to (potential|flow).<name>
            let mut name: String = attr.name.to_string();
            if attr.kind==DisciplineAttrKind::FlowOverwrite  {
                name = "flow.".to_string() + &name;
            } else if attr.kind==DisciplineAttrKind::PotentialOverwrite {
                name = "potential.".to_string() + &name;
            }
            if let Some(value) = &attr.value {
                table.attributes.push(AttrEntry { name: name, value: value.clone() });
            }
        }
        let i2 = table.attributes.next_key();
        DisciplineEntry {
            name: discipline.name.to_string(), 
            flow_nature: if let Some(flow_index) = flow_index { NDANatureIdResolution::NDAId(flow_index) }
                else { NDANatureIdResolution::Unresolved }, 
            potential_nature: if let Some(potential_index) = potential_index { NDANatureIdResolution::NDAId(potential_index) } 
                else { NDANatureIdResolution::Unresolved }, 
            attr_range: IdxRange::new(i1..i2), 
        }
    }

    pub fn resolve_nature_reference(&self, table: &NDATable, natref: &NatureRef) -> NDANatureIdResolution {
        if natref.kind == NatureRefKind::DisciplineFlow || natref.kind == NatureRefKind::DisciplinePotential {
            // Lookup discipline
            if let Some(idx) = table.discipline_name_map.get(&natref.name.to_string()) {
                // Found discipline
                if natref.kind == NatureRefKind::DisciplineFlow {
                    return table.disciplines[*idx].flow_nature.clone();
                } else {
                    return table.disciplines[*idx].potential_nature.clone();
                }
            } else {
                // Doscipline not found
                return NDANatureIdResolution::Unresolved;
            }
        } else {
            // Lookup nature
            if let Some(idx) = table.nature_name_map.get(&natref.name.to_string()) {
                return NDANatureIdResolution::NDAId(*idx);
            } else {
                return NDANatureIdResolution::Unresolved;
            }
        }
    }

    pub fn resolve(&self, table: &mut NDATable) {
        // Loop through natures, try to resolve parent, idt_nature, ddt_nature
        // After one pass without change we stop
        let mut changed = true;
        while changed {
            changed = false;
            for tree_nature in self.tree.data.natures.iter() {
                // Find nature in NDATable
                if let Some(idx) = table.nature_name_map.get(&tree_nature.name.to_string()) {
                    // Resolve parent, ddt, idt
                    if table.natures[*idx].parent==NDANatureIdResolution::Unresolved {
                        table.natures[*idx].parent = if let Some(natref) = tree_nature.parent.as_ref() {
                            let natres = self.resolve_nature_reference(table, natref);
                            if natres!=NDANatureIdResolution::Unresolved {
                                changed = true;
                            }
                            natres
                        } else {
                            changed = true;
                            NDANatureIdResolution::NotGiven
                        }
                    }
                    if table.natures[*idx].ddt_nature==NDANatureIdResolution::Unresolved {
                        table.natures[*idx].ddt_nature = if let Some((natref, _)) = tree_nature.ddt_nature.as_ref() {
                            let natres = self.resolve_nature_reference(table, natref);
                            if natres!=NDANatureIdResolution::Unresolved {
                                changed = true;
                            }
                            natres
                        } else {
                            changed = true;
                            NDANatureIdResolution::NotGiven
                        }
                    }
                    if table.natures[*idx].idt_nature==NDANatureIdResolution::Unresolved {
                        table.natures[*idx].idt_nature = if let Some((natref, _)) = tree_nature.idt_nature.as_ref() {
                            let natres = self.resolve_nature_reference(table, natref);
                            if natres!=NDANatureIdResolution::Unresolved {
                                changed = true;
                            }
                            natres
                        } else {
                            changed = true;
                            NDANatureIdResolution::NotGiven
                        }
                    }
                }
            }
        
            // Loop through disciplines, resolve flow and potential
            for tree_discipline in self.tree.data.disciplines.iter() {
                // Find discipline in NDATable
                if let Some(idx) = table.discipline_name_map.get(&tree_discipline.name.to_string()) {
                    // Resolve flow, potential
                    if table.disciplines[*idx].flow_nature==NDANatureIdResolution::Unresolved {
                        table.disciplines[*idx].flow_nature = if let Some((natref, _)) = tree_discipline.flow.as_ref() {
                            let natres = self.resolve_nature_reference(table, &natref);
                            if natres!=NDANatureIdResolution::Unresolved {
                                changed = true;
                            }
                            natres
                        } else {
                            NDANatureIdResolution::NotGiven
                        }
                    }
                    if table.disciplines[*idx].potential_nature==NDANatureIdResolution::Unresolved {
                        table.disciplines[*idx].potential_nature = if let Some((natref, _)) = tree_discipline.potential.as_ref() {
                            let natres = self.resolve_nature_reference(table, &natref);
                            if natres!=NDANatureIdResolution::Unresolved {
                                changed = true;
                            }
                            natres
                        } else {
                            NDANatureIdResolution::NotGiven
                        }
                    }
                }
            }
        } // while
    }

    // Now fill it with natures and disciplines
    pub fn fill_table(&self, table: &mut NDATable) {
        // Add natures
        for nature in self.tree.data.natures.iter() {
            let nat = self.create_nature(nature, table);
            table.natures.push(nat);
            let nature_ndx = (table.natures.len() - 1).into();
            table.nature_name_map.insert(nature.name.to_string(), nature_ndx);
        }

        // Add disciplines. If a discipline overrides in flow/potential attribute
        // create a nature named <discipline>.<flow|potential> with the flow/potential 
        // nature as parent. If the flow/potential nature is not given, create a 
        // nature named <discipline>.<flow|potential> with no parent.
        for discipline in self.tree.data.disciplines.iter() {
            // Check if a flow/potential attribute is overridden
            let mut flow_attr_set = false;
            let mut potential_attr_set = false;
            for ndx in discipline.extra_attrs.clone() {
                let attr = &self.tree.data.discipline_attrs[ndx];
                match attr.kind {
                    DisciplineAttrKind::FlowOverwrite => {
                        flow_attr_set = true;
                    }, 
                    DisciplineAttrKind::PotentialOverwrite => {
                        potential_attr_set = true;
                    },
                    _ => {}
                }
            }
            let mut flow_index = None;
            let mut potential_index = None;
            // Get flow nature name, reference is always to a nature, never discipline.(flow|potential)
            if let Some((nature_ref, _)) = &discipline.flow {
                let name = nature_ref.name.to_string();
                // Lookup nature
                let maybe_parent = table.nature_name_map.get(&name).cloned();
                if let Some(parent_ndx) = maybe_parent {
                    // Found it, are any of its attributes overridden
                    if flow_attr_set {
                        // Set its parent to be nature with index nature_ndx 
                        // Name the new nature <discipline>.flow
                        let flow_name = discipline.name.to_string() + ".flow";
                        let nat = self.create_derived_nature(discipline, true, flow_name.clone(), &parent_ndx, table);
                        table.natures.push(nat);
                        let nature_ndx = (table.natures.len() - 1).into();
                        table.nature_name_map.insert(flow_name, nature_ndx);
                        flow_index = nature_ndx.into();
                    }
                } else {
                    // Not found, panic
                }
            }
            
            // Get potential nature name, reference is always to a nature, never discipline.(flow|potential)
            if let Some((nature_ref, _)) = &discipline.potential {
                let name = nature_ref.name.to_string();
                // Lookup nature
                let maybe_parent = table.nature_name_map.get(&name).cloned();
                if let Some(parent_ndx) = maybe_parent {
                    if potential_attr_set {
                        // Set its parent to be nature with index nature_ndx
                        // Name the new nature <discipline>.potential
                        let potential_name = discipline.name.to_string() + ".potential";
                        let nat = self.create_derived_nature(discipline, false, potential_name.clone(), &parent_ndx, table);
                        table.natures.push(nat);
                        let nature_ndx = (table.natures.len() - 1).into();
                        table.nature_name_map.insert(potential_name, nature_ndx);
                        potential_index = nature_ndx.into();
                    }
                } else {
                    // Not found, panic
                }
            }

            // Add discipline
            let disc = self.create_discipline(discipline, flow_index, potential_index, table);
            table.disciplines.push(disc);
            let disc_ndx = (table.disciplines.len() - 1).into();
            table.discipline_name_map.insert(discipline.name.to_string(), disc_ndx);
        }
        
        // Resolve nature references
        self.resolve(table);
    }

}