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
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Operation {
    BaseOperation(BaseOperation),
    ExtensionOperation(ExtensionOperation),
}

