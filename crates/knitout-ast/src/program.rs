use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result},
};

use crate::{
    Carrier, Provenance, base_operations::BaseOperation, extension_operations::ExtensionOperation,
    header_types::Header,
};

/// A knitout program, complete with headers and magic string
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Program {
    /// The ;!knitout-`version` at the top
    version: u32,
    /// The headers
    headers: Header,
    /// The operations, with attached provenance
    operations: Vec<(Operation, Provenance)>,

    /// The carrier naming table
    carrier_table: CarrierTable,
}

impl Program {
    pub fn new(version: u32, headers: Header, carriers_table: CarrierTable) -> Self {
        Self {
            version,
            headers,
            operations: Vec::new(),
            carrier_table: carriers_table,
        }
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn headers(&self) -> &Header {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut Header {
        &mut self.headers
    }

    pub fn operations(&self) -> &[(Operation, Provenance)] {
        &self.operations
    }

    pub fn push_operation(&mut self, op: Operation, provenance: Provenance) {
        self.operations.push((op, provenance));
    }

    pub fn carrier_table(&self) -> &CarrierTable {
        &self.carrier_table
    }

    pub fn carrier_table_mut(&mut self) -> &mut CarrierTable {
        &mut self.carrier_table
    }
}

/// A table that stores carrier names once. Use `CarrierTable::make_carrier` to create `Carrier`s.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CarrierTable {
    /// The map from carrier u32 IDs to their names in the source
    id_to_name: Vec<String>,
    /// The reverse map, taking a name and returning the carrier ID if it exists
    name_to_id: HashMap<String, u32>,
}

impl CarrierTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the name of a carrier, panicking if the carrier ID is invalid. This should be
    /// impossible, as as all carriers in a program must be created through the carrier table.
    pub fn carrier_name(&self, carrier: Carrier) -> &str {
        self.carrier_name_option(carrier)
            .expect("Invalid carrier ID: no name found in carrier table. Did you use multiple carrier tables when you should have used one?.")
    }

    pub(crate) fn carrier_name_option(&self, carrier: Carrier) -> Option<&str> {
        self.id_to_name
            .get(carrier.id() as usize)
            .map(String::as_str)
    }

    pub fn make_carrier(&mut self, name: &str) -> Carrier {
        if let Some(&id) = self.name_to_id.get(name) {
            Carrier::new(id)
        } else {
            let id = self.id_to_name.len() as u32;
            self.id_to_name.push(name.to_owned());
            self.name_to_id.insert(name.to_owned(), id);
            Carrier::new(id)
        }
    }
}

/// An operation in a knitout program: either a base operation or an extension operation
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Operation {
    BaseOperation(BaseOperation),
    ExtensionOperation(ExtensionOperation),
}

/// Errors out when the program's carrier table is missing entries
impl Display for Program {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, ";!knitout-{}", self.version)?;
        // writes a newline at the end of headers:
        self.headers.fmt_table(f, &self.carrier_table)?;
        for (op, provenance) in &self.operations {
            // makes an empty line between operations and headers
            writeln!(f)?;
            op.fmt_table(f, &self.carrier_table)?;
            if let Some(source) = provenance.source() {
                write!(f, " ;!source: {}", source)?;
            }
        }
        // no newline at end of program
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

#[cfg(test)]
mod tests {
    use crate::{
        Bed, CarrierOrRail, Direction, IsSlider, Needle, PositionHeaderValue, Span,
        header_types::CarriersHeaderValue,
    };

    use super::*;

    #[test]
    fn test_program_getters() {
        let mut carrier_table = CarrierTable::new();
        let carrier_a = carrier_table.make_carrier("A");
        let carrier_b = carrier_table.make_carrier("B");

        let headers = Header::new(CarriersHeaderValue::new(vec![
            CarrierOrRail::Carrier(carrier_a),
            CarrierOrRail::Carrier(carrier_b),
        ]));
        let mut program = Program::new(18, headers, carrier_table);

        let carrier_c = program.carrier_table_mut().make_carrier("C");
        assert_eq!(program.carrier_table().carrier_name(carrier_c), "C");

        assert_eq!(program.version(), 18);
        assert_eq!(program.headers().carriers().carriers_and_rails().len(), 2);
        assert_eq!(program.operations().len(), 0);

        program.push_operation(
            Operation::BaseOperation(BaseOperation::Tuck {
                direction: Direction::Right,
                needle: Needle::new(Bed::Front, IsSlider::NotSlider, 5),
                carriers: vec![carrier_a],
            }),
            Provenance::default(),
        );

        assert_eq!(program.operations().len(), 1);
    }

    #[test]
    fn test_program_display() {
        let mut carrier_table = CarrierTable::new();
        let carrier_a = carrier_table.make_carrier("A");
        let carrier_b = carrier_table.make_carrier("B");

        let headers = Header::new(CarriersHeaderValue::new(vec![
            CarrierOrRail::Carrier(carrier_a),
            CarrierOrRail::Carrier(carrier_b),
        ]));
        let mut program = Program::new(18, headers, carrier_table);
        program
            .headers_mut()
            .set_position(PositionHeaderValue::Left);
        program.headers_mut().set_gauge(1500);

        program.push_operation(
            Operation::BaseOperation(BaseOperation::Tuck {
                direction: Direction::Right,
                needle: Needle::new(Bed::Front, IsSlider::NotSlider, 5),
                carriers: vec![carrier_a],
            }),
            Provenance::new(Some(Span::new(1, 1)), None),
        );
        program.push_operation(
            Operation::ExtensionOperation(ExtensionOperation::UnknownExtension {
                name: "x-custom".to_string(),
                arguments: vec!["arg1".to_string(), "arg2".to_string()],
            }),
            Provenance::new(None, Some("test_source".to_string())),
        );

        let expected_output = ";!knitout-18
;;Carriers: A B
;;Gauge: 1500
;;Position: Left

tuck + f5 A
x-custom arg1 arg2 ;!source: test_source";
        assert_eq!(program.to_string(), expected_output);
    }
}
