use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result},
};

use crate::{
    knitting_types::Carrier,
    program::{CarrierTable, DisplayCarrierTable},
};

/// The value of the `Carriers:` header, which is a list of carriers and rails.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CarriersHeaderValue {
    carriers_and_rails: Vec<CarrierOrRail>,
}

impl CarriersHeaderValue {
    pub fn new(carriers_and_rails: Vec<CarrierOrRail>) -> Self {
        Self { carriers_and_rails }
    }

    pub fn carriers_and_rails(&self) -> &[CarrierOrRail] {
        &self.carriers_and_rails
    }
}

/// Either a carrier or a rail.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum CarrierOrRail {
    Carrier(Carrier),
    Rail(Rail),
}

/// A rail, which has a left and right side, each of which has a list of carriers.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Rail {
    left: RailSide,
    right: RailSide,
}

impl Rail {
    pub fn new(left: RailSide, right: RailSide) -> Self {
        Self { left, right }
    }

    pub fn left(&self) -> &RailSide {
        &self.left
    }

    pub fn right(&self) -> &RailSide {
        &self.right
    }
}

/// The left or right side of a rail, which has an ordered list of carriers.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RailSide {
    carriers: Vec<Carrier>,
}

impl RailSide {
    pub fn carriers(&self) -> &[Carrier] {
        &self.carriers
    }
}

/// Acceptable values for the `Position:` header.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PositionHeaderValue {
    Left,
    Center,
    Right,
    Keep,
}

impl Display for PositionHeaderValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            PositionHeaderValue::Left => write!(f, "Left"),
            PositionHeaderValue::Center => write!(f, "Center"),
            PositionHeaderValue::Right => write!(f, "Right"),
            PositionHeaderValue::Keep => write!(f, "Keep"),
        }
    }
}

/// The header section of a knitout program.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Header {
    /// Carriers: value
    carriers: CarriersHeaderValue,
    /// Yarn-C: header values
    yarn: HashMap<Carrier, String>,
    /// Machine: value
    machine: Option<String>,
    /// Gauge: value
    gauge: Option<u32>,
    /// Width: value
    width: Option<u32>,
    /// Position: value
    position: Option<PositionHeaderValue>,
    /// Unknown extension headers. The leading "X-" is included in the key.
    unknown: HashMap<String, String>,
}

impl Header {
    pub fn new(carriers: CarriersHeaderValue) -> Self {
        Self {
            carriers,
            yarn: HashMap::new(),
            machine: None,
            gauge: None,
            width: None,
            position: None,
            unknown: HashMap::new(),
        }
    }

    pub fn carriers(&self) -> &CarriersHeaderValue {
        &self.carriers
    }

    pub fn yarn(&self) -> &HashMap<Carrier, String> {
        &self.yarn
    }

    pub fn machine(&self) -> Option<&str> {
        self.machine.as_deref()
    }

    pub fn gauge(&self) -> Option<u32> {
        self.gauge
    }

    pub fn width(&self) -> Option<u32> {
        self.width
    }

    pub fn position(&self) -> Option<PositionHeaderValue> {
        self.position
    }

    pub fn unknown(&self) -> &HashMap<String, String> {
        &self.unknown
    }

    pub fn add_yarn(&mut self, carrier: Carrier, yarn: String) -> Option<String> {
        self.yarn.insert(carrier, yarn)
    }

    pub fn set_machine(&mut self, machine: String) -> Option<String> {
        self.machine.replace(machine)
    }

    pub fn set_gauge(&mut self, gauge: u32) -> Option<u32> {
        self.gauge.replace(gauge)
    }

    pub fn set_width(&mut self, width: u32) -> Option<u32> {
        self.width.replace(width)
    }

    pub fn set_position(&mut self, position: PositionHeaderValue) -> Option<PositionHeaderValue> {
        self.position.replace(position)
    }

    pub fn add_unknown(&mut self, key: String, value: String) -> Option<String> {
        self.unknown.insert(key, value)
    }
}

impl DisplayCarrierTable for Header {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        write!(f, ";;Carriers:")?;
        self.carriers.fmt_table(f, table)?;
        writeln!(f)?;
        for (carrier, color) in &self.yarn {
            write!(f, ";;Yarn-")?;
            carrier.fmt_table(f, table)?;
            writeln!(f, ": {}", color)?;
        }
        if let Some(machine) = &self.machine {
            writeln!(f, ";;Machine: {}", machine)?;
        }
        if let Some(gauge) = &self.gauge {
            writeln!(f, ";;Gauge: {}", gauge)?;
        }
        if let Some(width) = &self.width {
            writeln!(f, ";;Width: {}", width)?;
        }
        if let Some(position) = &self.position {
            writeln!(f, ";;Position: {}", position)?;
        }
        for (key, value) in &self.unknown {
            writeln!(f, ";;{}: {}", key, value)?;
        }
        Ok(())
    }
}

/// Writes a leading space for all carriers and rails, including the first one!
impl DisplayCarrierTable for CarriersHeaderValue {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        for carrier_or_rail in self.carriers_and_rails.iter() {
            write!(f, " ")?;
            carrier_or_rail.fmt_table(f, table)?;
        }
        Ok(())
    }
}

impl DisplayCarrierTable for CarrierOrRail {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        match self {
            CarrierOrRail::Carrier(carrier) => carrier.fmt_table(f, table),
            CarrierOrRail::Rail(rail) => rail.fmt_table(f, table),
        }
    }
}

impl DisplayCarrierTable for Rail {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        self.left.fmt_table(f, table)?;
        write!(f, ";")?;
        self.right.fmt_table(f, table)
    }
}

impl DisplayCarrierTable for RailSide {
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

    #[test]
    fn test_position_header_value_display() {
        assert_eq!(PositionHeaderValue::Left.to_string(), "Left");
        assert_eq!(PositionHeaderValue::Center.to_string(), "Center");
        assert_eq!(PositionHeaderValue::Right.to_string(), "Right");
        assert_eq!(PositionHeaderValue::Keep.to_string(), "Keep");
    }

    #[test]
    fn test_header_get_set() {
        let mut table = CarrierTable::new();
        let carriers = CarriersHeaderValue {
            carriers_and_rails: vec![
                CarrierOrRail::Carrier(table.make_carrier("A")),
                CarrierOrRail::Carrier(table.make_carrier("B")),
            ],
        };
        let mut header = Header::new(carriers.clone());
        assert_eq!(header.carriers(), &carriers);

        header.add_yarn(table.make_carrier("A"), "Red".to_string());
        assert_eq!(
            header.yarn().get(&table.make_carrier("A")),
            Some(&"Red".to_string())
        );

        header.set_machine("MyMachine".to_string());
        assert_eq!(header.machine(), Some("MyMachine"));

        header.set_gauge(10);
        assert_eq!(header.gauge(), Some(10));

        header.set_width(20);
        assert_eq!(header.width(), Some(20));

        header.set_position(PositionHeaderValue::Center);
        assert_eq!(header.position(), Some(PositionHeaderValue::Center));

        header.add_unknown("X-Custom".to_string(), "Value".to_string());
        assert_eq!(header.unknown().get("X-Custom"), Some(&"Value".to_string()));
    }

    #[test]
    fn test_rail_side_getters() {
        let mut table = CarrierTable::new();
        let carrier1 = table.make_carrier("A");
        let carrier2 = table.make_carrier("B");
        let rail_side = RailSide {
            carriers: vec![carrier1, carrier2],
        };
        assert_eq!(rail_side.carriers(), &[carrier1, carrier2]);
    }

    // A throwaway wrapper so we can drive a real Formatter through the standard machinery.
    struct RailSideHarness<'a> {
        rail_side: &'a RailSide,
        table: &'a CarrierTable,
    }

    impl<'a> fmt::Display for RailSideHarness<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            self.rail_side.fmt_table(f, self.table)
        }
    }

    fn render_rail_side(rail_side: RailSide, table: &CarrierTable) -> String {
        RailSideHarness {
            rail_side: &rail_side,
            table,
        }
        .to_string()
    }

    #[test]
    fn test_rail_side_display() {
        let mut table = CarrierTable::new();
        let carrier1 = table.make_carrier("A");
        let carrier2 = table.make_carrier("B");
        let rail_side = RailSide {
            carriers: vec![carrier1, carrier2],
        };
        assert_eq!(render_rail_side(rail_side, &table), "A,B");
    }

    // A throwaway wrapper so we can drive a real Formatter through the standard machinery.
    struct CarriersHeaderValueHarness<'a> {
        carriers_header_value: &'a CarriersHeaderValue,
        table: &'a CarrierTable,
    }

    impl<'a> fmt::Display for CarriersHeaderValueHarness<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            self.carriers_header_value.fmt_table(f, self.table)
        }
    }

    fn render_carriers_header_value(
        carriers_header_value: CarriersHeaderValue,
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
        let carrier1 = table.make_carrier("A");
        let carrier2 = table.make_carrier("B");
        let rail_side_left = RailSide {
            carriers: vec![carrier1],
        };
        let rail_side_right = RailSide {
            carriers: vec![carrier2],
        };
        let rail = Rail {
            left: rail_side_left,
            right: rail_side_right,
        };
        let carriers_header_value = CarriersHeaderValue {
            carriers_and_rails: vec![
                CarrierOrRail::Carrier(carrier1),
                CarrierOrRail::Rail(rail),
                CarrierOrRail::Carrier(carrier2),
            ],
        };
        assert_eq!(
            render_carriers_header_value(carriers_header_value, &table),
            " A A;B B"
        );
    }
}
