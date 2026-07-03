use crate::knitting_types::{Carrier, Direction, Needle};

use num_rational::Ratio;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ExtensionOperation {
    XStitchNumber {
        value: u32,
    },
    XSpeedNumber {
        value: u32,
    },
    XSubRollerNumber {
        value: u32,
    },
    XPresserMode {
        mode: PresserMode,
    },
    XXferStitchNumber {
        value: u32,
    },
    XEndPass,
    XVisColor {
        color: String,
        carriers: Vec<Carrier>,
    },
    XLoopIn {
        direction: Direction,
        needle: Needle,
        carriers: Vec<Carrier>,
    },
    XYarnIn {
        direction: Direction,
        needle: Needle,
        carriers: Vec<Carrier>,
    },
    XYarnOut {
        direction: Direction,
        needle: Needle,
        carriers: Vec<Carrier>,
    },
    XEnd,
    XRollerAdvance {
        value: Ratio<i32>,
    },
    XAddRollerAdvance {
        value: Ratio<i32>,
    },
    XCarrierSpacing {
        value: u32,
    },
    XCarrierStoppingDistance {
        /// A parsed spacing of "0.5" is represented at "0" here. That is, carriers move `value` +
        /// 0.5 needles past the needle they just operated on.
        value: u32,
    },
    XXferStyle {
        style: XferStyle,
    },
    UnknownExtension {
        /// Includes the leading "x-"
        name: String,
        arguments: Vec<String>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PresserMode {
    Off,
    Auto,
    On,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum XferStyle {
    FourPass,
    TwoPass,
}
