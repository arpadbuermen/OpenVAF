use hir_def::ndatable::{NDANatureIdResolution, NDATable, NatureEntry, DisciplineEntry, AttrEntry};
use crate::metadata::osdi_0_4::{
    OsdiAttribute, OsdiAttributeValue, OsdiDiscipline, OsdiNature, 
    ATTR_TYPE_INT, ATTR_TYPE_REAL, ATTR_TYPE_STR
};
use syntax::ConstExprValue;
use std::vec::Vec;
use lasso::Rodeo;

fn osdi_nature(nature: &NatureEntry, literals: &mut Rodeo) -> OsdiNature {
    let i1: u32 = nature.attr_range.start().into();
    let i2: u32 = nature.attr_range.end().into();
    literals.get_or_intern(&*nature.name);
    OsdiNature { 
        name: nature.name.clone(), 
        parent: match nature.parent {
            NDANatureIdResolution::NDAId(id) => u32::from(id), 
            _ => u32::MAX
        }, 
        ddt: match nature.ddt_nature {
            NDANatureIdResolution::NDAId(id) => u32::from(id), 
            _ => u32::MAX
        }, 
        idt: match nature.idt_nature {
            NDANatureIdResolution::NDAId(id) => u32::from(id), 
            _ => u32::MAX
        },  
        attr_start: if nature.attr_range.is_empty() { u32::MAX }
            else { i1 }, 
        num_attr: if nature.attr_range.is_empty() { 0 }
            else { i2-i1 }, 
    }
}

fn osdi_discipline(discipline: &DisciplineEntry, literals: &mut Rodeo) -> OsdiDiscipline {
    let i1: u32 = discipline.attr_range.start().into();
    let i2: u32 = discipline.attr_range.end().into();
    literals.get_or_intern(&*discipline.name);
    OsdiDiscipline { 
        name: discipline.name.clone(), 
        flow: match discipline.flow_nature {
            NDANatureIdResolution::NDAId(id) => u32::from(id), 
            _ => u32::MAX
        }, 
        potential: match discipline.potential_nature {
            NDANatureIdResolution::NDAId(id) => u32::from(id), 
            _ => u32::MAX
        },  
        attr_start: if discipline.attr_range.is_empty() { u32::MAX }
            else { i1 }, 
        num_attr: if discipline.attr_range.is_empty() { 0 }
            else { i2-i1 }, 
    }
}

impl OsdiAttributeValue {
    pub fn new(v: &ConstExprValue) -> OsdiAttributeValue {
        match v {
            ConstExprValue::Float(f) => OsdiAttributeValue::Real(f.into_inner()), 
            ConstExprValue::Int(i) => OsdiAttributeValue::Integer(*i), 
            ConstExprValue::String(s) => OsdiAttributeValue::String(s.clone()), 
        }
    }
}

fn osdi_attribute(attr: &AttrEntry, literals: &mut Rodeo) -> OsdiAttribute {
    literals.get_or_intern(&*attr.name);
    OsdiAttribute { 
        value: OsdiAttributeValue::new(&attr.value), 
        value_type: match &attr.value {
            ConstExprValue::Int(_) => ATTR_TYPE_INT, 
            ConstExprValue::Float(_) => ATTR_TYPE_REAL, 
            ConstExprValue::String(s) => { literals.get_or_intern(s); ATTR_TYPE_STR }, 
        }, 
        name: attr.name.clone(), 
    }
}

// Convert NDATable to a vector of OsdiNature structures
pub fn natures_vector(table: &NDATable, literals: &mut Rodeo) -> Vec<OsdiNature> {
    table.natures.iter().map(|nat| osdi_nature(nat, literals)).collect()
}

// Convert NDATable to a vector of OsdiDiscipline structures
pub fn disciplines_vector(table: &NDATable, literals: &mut Rodeo) -> Vec<OsdiDiscipline> {
    table.disciplines.iter().map(|disc| osdi_discipline(disc, literals)).collect()
}

// Convert NDATable to a vector of OsdiAttribute structures
pub fn attributes_vector(table: &NDATable, literals: &mut Rodeo) -> Vec<OsdiAttribute> {
    table.attributes.iter().map(|disc| osdi_attribute(disc, literals)).collect()
}

