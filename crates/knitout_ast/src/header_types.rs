use std::collections::HashMap;

use crate::knitting_types::Carrier;

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
    carrier: Vec<Carrier>,
}

/// Acceptable values for the `Position:` header
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PositionHeaderValue {
    Left,
    Center,
    Right,
    Keep,
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
