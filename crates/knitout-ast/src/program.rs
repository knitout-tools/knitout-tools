use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result},
};

use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_CARRIER_TABLE_ID: AtomicU32 = AtomicU32::new(0);

use either::Either;

use crate::{
    Carrier,
    base_operations::BaseOperation,
    extension_operations::ExtensionOperation,
    header_types::Headers,
    phase::{Annotated, Phase, Validated, valid},
};

/// A knitout program, complete with headers and magic string
#[derive(Debug, PartialEq, Eq)]
pub struct Program<P: Phase> {
    /// The ;!knitout-`version` at the top
    version: Either<Annotated<u32, P::MagicVersion>, P::Other>,
    /// The headers
    headers: Headers<P>,
    /// The operations, with attached provenance
    operations: Vec<Annotated<Operation<P>, P::Operation>>,

    /// The carrier naming table
    carrier_table: CarrierTable,
}

impl<P: Phase> Program<P> {
    pub fn new(
        version: Either<Annotated<u32, P::MagicVersion>, P::Other>,
        headers: Headers<P>,
        carriers_table: CarrierTable,
    ) -> Self {
        Self {
            version,
            headers,
            operations: Vec::new(),
            carrier_table: carriers_table,
        }
    }

    pub fn version(&self) -> &Either<Annotated<u32, P::MagicVersion>, P::Other> {
        &self.version
    }

    pub fn headers(&self) -> &Headers<P> {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut Headers<P> {
        &mut self.headers
    }

    pub fn operations(&self) -> &[Annotated<Operation<P>, P::Operation>] {
        &self.operations
    }

    pub fn push_operation(&mut self, op: Annotated<Operation<P>, P::Operation>) {
        self.operations.push(op);
    }

    pub fn carrier_table(&self) -> &CarrierTable {
        &self.carrier_table
    }

    pub fn carrier_table_mut(&mut self) -> &mut CarrierTable {
        &mut self.carrier_table
    }

    /// Destructure into all components, for use when mapping between phases.
    pub fn into_components(
        self,
    ) -> (
        Either<Annotated<u32, P::MagicVersion>, P::Other>,
        Headers<P>,
        Vec<Annotated<Operation<P>, P::Operation>>,
        CarrierTable,
    ) {
        (
            self.version,
            self.headers,
            self.operations,
            self.carrier_table,
        )
    }
}

/// A table that stores carrier names once. Use `CarrierTable::make_carrier` to create `Carrier`s.
#[derive(Eq, Debug)]
pub struct CarrierTable {
    /// The map from carrier u32 IDs to their names in the source
    id_to_name: Vec<String>,
    /// The reverse map, taking a name and returning the carrier ID if it exists
    name_to_id: HashMap<String, u32>,
    /// A unique ID for this carrier table, passed along to its carriers
    id: u32,
}

/// We don't care about the ID; the name_to_id field is derived from id_to_name
impl PartialEq for CarrierTable {
    fn eq(&self, other: &Self) -> bool {
        self.id_to_name == other.id_to_name
    }
}

impl Default for CarrierTable {
    fn default() -> Self {
        Self::new()
    }
}

impl CarrierTable {
    pub fn new() -> Self {
        Self {
            id_to_name: Vec::new(),
            name_to_id: HashMap::new(),
            id: NEXT_CARRIER_TABLE_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Gets the name of a carrier, panicking if the carrier ID is invalid. This should be
    /// impossible, as as all carriers in a program must be created through the carrier table.
    pub fn carrier_name<P: Phase>(&self, carrier: &Carrier<P>) -> &str {
        self.carrier_name_option(carrier)
            .expect("Invalid carrier ID: no name found in carrier table. Did you use multiple carrier tables when you should have used only one?")
    }

    pub(crate) fn carrier_name_option<P: Phase>(&self, carrier: &Carrier<P>) -> Option<&str> {
        debug_assert_eq!(
            self.id,
            carrier.table_id(),
            "Carrier table ID mismatch: carrier was created from a different carrier table"
        );
        self.id_to_name
            .get(carrier.id() as usize)
            .map(String::as_str)
    }

    pub fn make_carrier<P: Phase>(&mut self, name: &str, annotation: P::Carrier) -> Carrier<P> {
        if let Some(&id) = self.name_to_id.get(name) {
            Carrier::new(id, annotation, self.id)
        } else {
            let id = u32::try_from(self.id_to_name.len())
                .expect("Too many carriers: carrier table overflowed u32");
            self.id_to_name.push(name.to_owned());
            self.name_to_id.insert(name.to_owned(), id);
            Carrier::new(id, annotation, self.id)
        }
    }
}

/// An operation in a knitout program: either a base operation or an extension operation
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Operation<P: Phase> {
    BaseOperation(BaseOperation<P>),
    ExtensionOperation(ExtensionOperation<P>),
    Other(P::Other),
}

/// Errors out when the program's carrier table is missing entries
impl Display for Program<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, ";!knitout-{}", valid(&self.version).value)?;
        // writes a newline at the end of headers:
        self.headers.fmt_table(f, &self.carrier_table)?;
        for op in &self.operations {
            // makes an empty line between operations and headers
            writeln!(f)?;
            op.value.fmt_table(f, &self.carrier_table)?;
            if let Some(source) = op.annotation.source() {
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

impl DisplayCarrierTable for Operation<Validated> {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        match self {
            Operation::BaseOperation(op) => op.fmt_table(f, table),
            Operation::ExtensionOperation(op) => op.fmt_table(f, table),
            Operation::Other(x) => match *x {},
        }
    }
}

#[cfg(test)]
mod tests {
    use either::Either;

    use crate::{
        BaseOperation, Bed, CarrierOrRail, Direction, ExtensionOperation, IsSlider, Needle,
        PositionHeaderValue, Provenance, Span, Validated,
        header_types::CarriersHeaderValue,
        knitting_types::{MyString, MyU32},
        phase::{Annotated, valid},
    };

    use super::*;

    #[test]
    fn test_carrier_table_eq() {
        let mut table1 = CarrierTable::new();
        let mut table2 = CarrierTable::new();

        table1.make_carrier::<Validated>("A", ());
        table1.make_carrier::<Validated>("B", ());

        table2.make_carrier::<Validated>("A", ());
        table2.make_carrier::<Validated>("B", ());

        assert_eq!(table1, table2);

        // Add a new carrier to table1
        table1.make_carrier::<Validated>("C", ());

        // Now the tables should not be equal
        assert_ne!(table1, table2);
    }

    #[test]
    fn test_program_getters() {
        let mut carrier_table = CarrierTable::new();
        let carrier_a = carrier_table.make_carrier("A", ());
        let carrier_b = carrier_table.make_carrier("B", ());

        let headers = Headers::new(Either::Left(Annotated::unit(CarriersHeaderValue::new(
            Either::Left(vec![
                CarrierOrRail::Carrier(carrier_a),
                CarrierOrRail::Carrier(carrier_b),
            ]),
            (),
        ))));
        let mut program =
            Program::<Validated>::new(Either::Left(Annotated::unit(18u32)), headers, carrier_table);

        let carrier_c: Carrier<Validated> = program.carrier_table_mut().make_carrier("C", ());
        assert_eq!(program.carrier_table().carrier_name(&carrier_c), "C");

        assert_eq!(valid(program.version()).value, 18u32);
        let carriers_val = valid(program.headers().carriers());
        assert_eq!(
            carriers_val
                .value
                .carriers_and_rails()
                .left()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(program.operations().len(), 0);

        program.push_operation(Annotated::new(
            Operation::BaseOperation(BaseOperation::Tuck {
                direction: Direction::Right(()),
                needle: Either::Left(Needle::new(
                    Bed::Front(()),
                    IsSlider::NotSlider(()),
                    Either::Left(Annotated::unit(5i32)),
                    (),
                )),
                carriers: vec![carrier_a],
            }),
            Provenance::default(),
        ));

        assert_eq!(program.operations().len(), 1);
    }

    #[test]
    fn test_program_display() {
        let mut carrier_table = CarrierTable::new();
        let carrier_a = carrier_table.make_carrier("A", ());
        let carrier_b = carrier_table.make_carrier("B", ());

        let headers = Headers::new(Either::Left(Annotated::unit(CarriersHeaderValue::new(
            Either::Left(vec![
                CarrierOrRail::Carrier(carrier_a),
                CarrierOrRail::Carrier(carrier_b),
            ]),
            (),
        ))));
        let mut program =
            Program::<Validated>::new(Either::Left(Annotated::unit(18u32)), headers, carrier_table);
        program
            .headers_mut()
            .set_position(Annotated::unit(PositionHeaderValue::Left(())));
        program.headers_mut().set_gauge(Annotated::new(
            MyU32::new(Either::Left(1500u32), ()),
            (),
        ));

        program.push_operation(Annotated::new(
            Operation::BaseOperation(BaseOperation::Tuck {
                direction: Direction::Right(()),
                needle: Either::Left(Needle::new(
                    Bed::Front(()),
                    IsSlider::NotSlider(()),
                    Either::Left(Annotated::unit(5i32)),
                    (),
                )),
                carriers: vec![carrier_a],
            }),
            Provenance::new(Some(Span::new(1, 1)), None),
        ));
        program.push_operation(Annotated::new(
            Operation::ExtensionOperation(ExtensionOperation::UnknownExtension {
                name: MyString::new(Either::Left("x-custom".to_string()), ()),
                arguments: vec![
                    MyString::new(Either::Left("arg1".to_string()), ()),
                    MyString::new(Either::Left("arg2".to_string()), ()),
                ],
            }),
            Provenance::new(None, Some("test_source".to_string())),
        ));

        let expected_output = ";!knitout-18
;;Carriers: A B
;;Gauge: 1500
;;Position: Left

tuck + f5 A
x-custom arg1 arg2 ;!source: test_source";
        assert_eq!(program.to_string(), expected_output);
    }
}
