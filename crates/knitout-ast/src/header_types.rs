use std::fmt::{Display, Formatter, Result};

use either::Either;
use indexmap::IndexMap;

use crate::{
    knitting_types::{Carrier, MyString, MyU32},
    phase::{Annotated, Phase, Validated, valid},
    program::{CarrierTable, DisplayCarrierTable},
};

/// The value of the `Carriers:` header, which is a list of carriers and rails.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CarriersHeaderValue<P: Phase> {
    carriers_and_rails: Either<Vec<CarrierOrRail<P>>, P::Other>,
    annotation: P::CarriersHeaderValue,
}

impl<P: Phase> CarriersHeaderValue<P> {
    pub fn new(
        carriers_and_rails: Either<Vec<CarrierOrRail<P>>, P::Other>,
        annotation: P::CarriersHeaderValue,
    ) -> Self {
        Self {
            carriers_and_rails,
            annotation,
        }
    }

    pub fn carriers_and_rails(&self) -> Either<&[CarrierOrRail<P>], &P::Other> {
        match &self.carriers_and_rails {
            Either::Left(v) => Either::Left(&v[..]),
            Either::Right(x) => Either::Right(x),
        }
    }
}

/// Either a carrier or a rail.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum CarrierOrRail<P: Phase> {
    Carrier(Carrier<P>),
    Rail(Rail<P>),
    Other(P::Other),
}

/// A rail, which has a left and right side, each of which has a list of carriers.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Rail<P: Phase> {
    left: Either<RailSide<P>, P::Other>,
    right: Either<RailSide<P>, P::Other>,
    annotation: P::Rail,
}

impl<P: Phase> Rail<P> {
    pub fn new(
        left: Either<RailSide<P>, P::Other>,
        right: Either<RailSide<P>, P::Other>,
        annotation: P::Rail,
    ) -> Self {
        Self {
            left,
            right,
            annotation,
        }
    }

    pub fn left(&self) -> &Either<RailSide<P>, P::Other> {
        &self.left
    }

    pub fn right(&self) -> &Either<RailSide<P>, P::Other> {
        &self.right
    }

    pub fn annotation(&self) -> &P::Rail {
        &self.annotation
    }
}

/// The left or right side of a rail, which has an ordered list of carriers separated by commas.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RailSide<P: Phase> {
    carriers: Vec<Carrier<P>>,
    annotation: P::RailSide,
}

impl<P: Phase> RailSide<P> {
    pub fn new(carriers: Vec<Carrier<P>>, annotation: P::RailSide) -> Self {
        Self {
            carriers,
            annotation,
        }
    }

    pub fn carriers(&self) -> &[Carrier<P>] {
        &self.carriers
    }

    pub fn annotation(&self) -> &P::RailSide {
        &self.annotation
    }
}

/// Acceptable values for the `Position:` header.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PositionHeaderValue<P: Phase> {
    Left(P::PositionHeaderValue),
    Center(P::PositionHeaderValue),
    Right(P::PositionHeaderValue),
    Keep(P::PositionHeaderValue),
    Other(P::Other),
}

impl Display for PositionHeaderValue<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            PositionHeaderValue::Left(_) => write!(f, "Left"),
            PositionHeaderValue::Center(_) => write!(f, "Center"),
            PositionHeaderValue::Right(_) => write!(f, "Right"),
            PositionHeaderValue::Keep(_) => write!(f, "Keep"),
            PositionHeaderValue::Other(x) => match *x {},
        }
    }
}

/// The header section of a knitout program.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Headers<P: Phase> {
    /// Carriers: value
    carriers: Either<Annotated<CarriersHeaderValue<P>, P::Header>, P::Other>,
    /// Yarn-C: header values, stored as a key-value vector
    yarn: IndexMap<Carrier<P>, Annotated<MyString<P>, P::Header>>,
    /// Machine: value
    machine: Option<Annotated<MyString<P>, P::Header>>,
    /// Gauge: value
    gauge: Option<Annotated<MyU32<P>, P::Header>>,
    /// Width: value
    width: Option<Annotated<MyU32<P>, P::Header>>,
    /// Position: value
    position: Option<Annotated<PositionHeaderValue<P>, P::Header>>,
    /// Unknown extension headers. The leading "X-" is included in the key.
    unknown: IndexMap<String, Annotated<String, P::UnknownHeader>>,
}

impl<P: Phase> Headers<P> {
    pub fn new(carriers: Annotated<CarriersHeaderValue<P>, P::Header>) -> Self {
        Self {
            carriers: Either::Left(carriers),
            yarn: IndexMap::new(),
            machine: None,
            gauge: None,
            width: None,
            position: None,
            unknown: IndexMap::new(),
        }
    }

    pub fn carriers(&self) -> &Either<Annotated<CarriersHeaderValue<P>, P::Header>, P::Other> {
        &self.carriers
    }

    pub fn yarn(&self) -> &IndexMap<Carrier<P>, Annotated<MyString<P>, P::Header>> {
        &self.yarn
    }

    pub fn machine(&self) -> &Option<Annotated<MyString<P>, P::Header>> {
        &self.machine
    }

    pub fn gauge(&self) -> &Option<Annotated<MyU32<P>, P::Header>> {
        &self.gauge
    }

    pub fn width(&self) -> &Option<Annotated<MyU32<P>, P::Header>> {
        &self.width
    }

    pub fn unknown(&self) -> &IndexMap<String, Annotated<String, P::UnknownHeader>> {
        &self.unknown
    }

    pub fn position(&self) -> &Option<Annotated<PositionHeaderValue<P>, P::Header>> {
        &self.position
    }

    pub fn add_yarn(
        &mut self,
        carrier: Carrier<P>,
        color: MyString<P>,
        annotation: P::Header,
    ) -> Option<Annotated<MyString<P>, P::Header>> {
        self.yarn.insert(carrier, Annotated::new(color, annotation))
    }

    pub fn set_machine(
        &mut self,
        machine: Annotated<MyString<P>, P::Header>,
    ) -> Option<Annotated<MyString<P>, P::Header>> {
        self.machine.replace(machine)
    }

    pub fn set_gauge(
        &mut self,
        gauge: Annotated<MyU32<P>, P::Header>,
    ) -> Option<Annotated<MyU32<P>, P::Header>> {
        self.gauge.replace(gauge)
    }

    pub fn set_width(
        &mut self,
        width: Annotated<MyU32<P>, P::Header>,
    ) -> Option<Annotated<MyU32<P>, P::Header>> {
        self.width.replace(width)
    }

    pub fn set_position(
        &mut self,
        position: Annotated<PositionHeaderValue<P>, P::Header>,
    ) -> Option<Annotated<PositionHeaderValue<P>, P::Header>> {
        self.position.replace(position)
    }

    pub fn add_unknown(
        &mut self,
        key: String,
        value: String,
        annotation: P::UnknownHeader,
    ) -> Option<Annotated<String, P::UnknownHeader>> {
        self.unknown.insert(key, Annotated::new(value, annotation))
    }
}

impl DisplayCarrierTable for Headers<Validated> {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        write!(f, ";;Carriers:")?;
        valid(&self.carriers).value.fmt_table(f, table)?;
        writeln!(f)?;
        for (carrier, color) in &self.yarn {
            write!(f, ";;Yarn-")?;
            carrier.fmt_table(f, table)?;
            writeln!(f, ": {}", color.value)?;
        }
        if let Some(machine) = &self.machine {
            writeln!(f, ";;Machine: {}", machine.value)?;
        }
        if let Some(gauge) = &self.gauge {
            writeln!(f, ";;Gauge: {}", gauge.value)?;
        }
        if let Some(width) = &self.width {
            writeln!(f, ";;Width: {}", width.value)?;
        }
        if let Some(position) = &self.position {
            writeln!(f, ";;Position: {}", position.value)?;
        }
        for (key, value) in &self.unknown {
            writeln!(f, ";;{}: {}", key, value.value)?;
        }
        Ok(())
    }
}

/// Writes a leading space for all carriers and rails, including the first one!
impl DisplayCarrierTable for CarriersHeaderValue<Validated> {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        for carrier_or_rail in valid(&self.carriers_and_rails).iter() {
            write!(f, " ")?;
            carrier_or_rail.fmt_table(f, table)?;
        }
        Ok(())
    }
}

impl DisplayCarrierTable for CarrierOrRail<Validated> {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        match self {
            CarrierOrRail::Carrier(carrier) => carrier.fmt_table(f, table),
            CarrierOrRail::Rail(rail) => rail.fmt_table(f, table),
            CarrierOrRail::Other(x) => match *x {},
        }
    }
}

impl DisplayCarrierTable for Rail<Validated> {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        valid(&self.left).fmt_table(f, table)?;
        write!(f, ";")?;
        valid(&self.right).fmt_table(f, table)
    }
}

impl DisplayCarrierTable for RailSide<Validated> {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        for (i, carrier) in self.carriers.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            carrier.fmt_table(f, table)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    use crate::knitting_types::{MyString, MyU32};
    use crate::phase::{Annotated, Validated};

    #[test]
    fn test_position_header_value_display() {
        assert_eq!(
            PositionHeaderValue::<Validated>::Left(()).to_string(),
            "Left"
        );
        assert_eq!(
            PositionHeaderValue::<Validated>::Center(()).to_string(),
            "Center"
        );
        assert_eq!(
            PositionHeaderValue::<Validated>::Right(()).to_string(),
            "Right"
        );
        assert_eq!(
            PositionHeaderValue::<Validated>::Keep(()).to_string(),
            "Keep"
        );
    }

    #[test]
    fn test_header_get_set() {
        let mut table = CarrierTable::new();
        let carrier_a = table.make_carrier("A", ());
        let carrier_b = table.make_carrier("B", ());
        let carriers = Annotated::unit(CarriersHeaderValue::new(
            Either::Left(vec![
                CarrierOrRail::Carrier(carrier_a),
                CarrierOrRail::Carrier(carrier_b),
            ]),
            (),
        ));
        let mut header = Headers::<Validated>::new(carriers.clone());
        assert_eq!(header.carriers().as_ref(), Either::Left(&carriers));

        let color = MyString::new(Either::Left(Annotated::unit("Red".to_string())));
        header.add_yarn(carrier_a, color.clone(), ());
        assert_eq!(
            header.yarn().get(&carrier_a),
            Some(&Annotated::new(color, ()))
        );

        header.set_machine(Annotated::new(
            MyString::new(Either::Left(Annotated::unit("MyMachine".to_string()))),
            (),
        ));
        assert!(header.machine().is_some());

        header.set_gauge(Annotated::new(
            MyU32::new(Either::Left(Annotated::unit(10u32))),
            (),
        ));
        assert!(header.gauge().is_some());

        header.set_width(Annotated::new(
            MyU32::new(Either::Left(Annotated::unit(20u32))),
            (),
        ));
        assert!(header.width().is_some());

        header.set_position(Annotated::unit(PositionHeaderValue::Center(())));
        assert_eq!(
            header.position().as_ref().map(|a| a.value),
            Some(PositionHeaderValue::Center(()))
        );

        header.add_unknown("X-Custom".to_string(), "Value".to_string(), ());
        assert_eq!(
            header.unknown().get("X-Custom"),
            Some(&Annotated::new("Value".to_string(), ()))
        );
    }

    #[test]
    fn test_rail_side_getters() {
        let mut table = CarrierTable::new();
        let carrier1: Carrier<Validated> = table.make_carrier("A", ());
        let carrier2: Carrier<Validated> = table.make_carrier("B", ());
        let rail_side = RailSide {
            carriers: vec![carrier1, carrier2],
            annotation: (),
        };
        assert_eq!(rail_side.carriers(), &[carrier1, carrier2]);
    }

    // A throwaway wrapper so we can drive a real Formatter through the standard machinery.
    struct RailSideHarness<'a> {
        rail_side: &'a RailSide<Validated>,
        table: &'a CarrierTable,
    }

    impl<'a> fmt::Display for RailSideHarness<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            self.rail_side.fmt_table(f, self.table)
        }
    }

    fn render_rail_side(rail_side: RailSide<Validated>, table: &CarrierTable) -> String {
        RailSideHarness {
            rail_side: &rail_side,
            table,
        }
        .to_string()
    }

    #[test]
    fn test_rail_side_display() {
        let mut table = CarrierTable::new();
        let carrier1 = table.make_carrier("A", ());
        let carrier2 = table.make_carrier("B", ());
        let rail_side = RailSide {
            carriers: vec![carrier1, carrier2],
            annotation: (),
        };
        assert_eq!(render_rail_side(rail_side, &table), "A,B");
    }

    // A throwaway wrapper so we can drive a real Formatter through the standard machinery.
    struct CarriersHeaderValueHarness<'a> {
        carriers_header_value: &'a CarriersHeaderValue<Validated>,
        table: &'a CarrierTable,
    }

    impl<'a> fmt::Display for CarriersHeaderValueHarness<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            self.carriers_header_value.fmt_table(f, self.table)
        }
    }

    fn render_carriers_header_value(
        carriers_header_value: CarriersHeaderValue<Validated>,
        table: &CarrierTable,
    ) -> String {
        CarriersHeaderValueHarness {
            carriers_header_value: &carriers_header_value,
            table,
        }
        .to_string()
    }

    #[test]
    fn test_carriers_header_value_display() {
        let mut table = CarrierTable::new();
        let carrier1 = table.make_carrier("A", ());
        let carrier2 = table.make_carrier("B", ());
        let rail_side_left = RailSide {
            carriers: vec![carrier1],
            annotation: (),
        };
        let rail_side_right = RailSide {
            carriers: vec![carrier2],
            annotation: (),
        };
        let rail = Rail {
            left: Either::Left(rail_side_left),
            right: Either::Left(rail_side_right),
            annotation: (),
        };
        let carriers_header_value = CarriersHeaderValue::new(
            Either::Left(vec![
                CarrierOrRail::Carrier(carrier1),
                CarrierOrRail::Rail(rail),
                CarrierOrRail::Carrier(carrier2),
            ]),
            (),
        );
        assert_eq!(
            render_carriers_header_value(carriers_header_value, &table),
            " A A;B B"
        );
    }
}
