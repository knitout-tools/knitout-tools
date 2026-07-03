use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result},
};

use crate::{
    base_operations::BaseOperation, extension_operations::ExtensionOperation, header_types::Header,
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Program {
    /// The ;!knitout-`version` at the top
    version: u32,
    /// The headers
    headers: Header,
    /// The operations, with attached provenance
    operations: Vec<Operation>,

    /// The carrier table
    carrier_table: CarrierTable,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CarrierTable {
    /// The map from carrier u32 IDs to their names in the source
    id_to_name: Vec<String>,
    /// The reverse map, taking a name and returning the carrier ID if it exists
    name_to_id: HashMap<String, u32>,
}

impl CarrierTable {
    pub fn lookup(&self, id: u32) -> Option<&String> {
        self.id_to_name.get(id as usize)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Operation {
    BaseOperation(BaseOperation),
    ExtensionOperation(ExtensionOperation),
}

/// Errors out when the program's carrier table is missing entries
impl Display for Program {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, ";!knitout-{}", self.version)?;
        self.headers.fmt_table(f, &self.carrier_table)?;
        if !self.operations.is_empty() {
            writeln!(f)?; // newline separating headers and body
        }
        for op in &self.operations {
            op.fmt_table(f, &self.carrier_table)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

/// A trait for things that know how to display themselves when provided with a carrier table.
pub trait DisplayCarrierTable {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result;
}

impl DisplayCarrierTable for Operation {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        match self {
            Operation::BaseOperation(op) => op.fmt_table(f, table),
            Operation::ExtensionOperation(op) => op.fmt_table(f, table),
        }
    }
}

