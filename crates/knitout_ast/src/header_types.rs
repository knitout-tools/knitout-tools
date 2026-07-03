use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result},
};

use crate::{
    knitting_types::Carrier,
    program::{CarrierTable, DisplayCarrierTable},
};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CarriersHeaderValue {
    carriers_and_rails: Vec<CarrierOrRail>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum CarrierOrRail {
    Carrier(Carrier),
    Rail(Rail),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Rail {
    left: RailSide,
    right: RailSide,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RailSide {
    carriers: Vec<Carrier>,
}

/// Acceptable values for the `Position:` header
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

impl DisplayCarrierTable for Header {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        write!(f, ";;Carriers: ")?;
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

impl DisplayCarrierTable for CarriersHeaderValue {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        for (i, carrier_or_rail) in self.carriers_and_rails.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
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
