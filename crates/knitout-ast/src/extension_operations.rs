use std::fmt::{Display, Formatter, Result};

use bigdecimal::BigDecimal;

use crate::{
    base_operations::fmt_carriers,
    knitting_types::{Carrier, Direction, Needle},
    program::DisplayCarrierTable,
};

/// An knitout extension operation `x-...`. Known extensions are parsed into their own variants,
/// while unknown extensions are stored in the `UnknownExtension` variant.
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
        value: BigDecimal,
    },
    XAddRollerAdvance {
        value: BigDecimal,
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

/// Valid presser modes for the `x-presser-mode` extension operation.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PresserMode {
    Off,
    Auto,
    On,
}

impl Display for PresserMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            PresserMode::Off => write!(f, "Off"),
            PresserMode::Auto => write!(f, "Auto"),
            PresserMode::On => write!(f, "On"),
        }
    }
}

/// Valid transfer styles for the `x-xfer-style` extension operation.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum XferStyle {
    FourPass,
    TwoPass,
}

impl Display for XferStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            XferStyle::FourPass => write!(f, "FourPass"),
            XferStyle::TwoPass => write!(f, "TwoPass"),
        }
    }
}

impl DisplayCarrierTable for ExtensionOperation {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &crate::program::CarrierTable) -> Result {
        match self {
            ExtensionOperation::XStitchNumber { value } => write!(f, "x-stitch-number {}", value),
            ExtensionOperation::XSpeedNumber { value } => write!(f, "x-speed-number {}", value),
            ExtensionOperation::XSubRollerNumber { value } => {
                write!(f, "x-sub-roller-number {}", value)
            }
            ExtensionOperation::XPresserMode { mode } => write!(f, "x-presser-mode {}", mode),
            ExtensionOperation::XXferStitchNumber { value } => {
                write!(f, "x-xfer-stitch-number {}", value)
            }
            ExtensionOperation::XEndPass => write!(f, "x-end-pass"),
            ExtensionOperation::XVisColor { color, carriers } => {
                write!(f, "x-vis-color {}", color)?;
                fmt_carriers(f, carriers, table)
            }
            ExtensionOperation::XLoopIn {
                direction,
                needle,
                carriers,
            } => {
                write!(f, "x-loop-in {} {}", direction, needle)?;
                fmt_carriers(f, carriers, table)
            }
            ExtensionOperation::XYarnIn {
                direction,
                needle,
                carriers,
            } => {
                write!(f, "x-yarn-in {} {}", direction, needle)?;
                fmt_carriers(f, carriers, table)
            }
            ExtensionOperation::XYarnOut {
                direction,
                needle,
                carriers,
            } => {
                write!(f, "x-yarn-out {} {}", direction, needle)?;
                fmt_carriers(f, carriers, table)
            }
            ExtensionOperation::XEnd => write!(f, "x-end"),
            ExtensionOperation::XRollerAdvance { value } => write!(f, "x-roller-advance {}", value),
            ExtensionOperation::XAddRollerAdvance { value } => {
                write!(f, "x-add-roller-advance {}", value)
            }
            ExtensionOperation::XCarrierSpacing { value } => {
                write!(f, "x-carrier-spacing {}", value)
            }
            ExtensionOperation::XCarrierStoppingDistance { value } => {
                write!(f, "x-carrier-stopping-distance {}", value)
            }
            ExtensionOperation::XXferStyle { style } => write!(f, "x-xfer-style {}", style),
            ExtensionOperation::UnknownExtension { name, arguments } => {
                write!(f, "{}", name)?;
                for arg in arguments {
                    write!(f, " {}", arg)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presser_mode_display() {
        assert_eq!(PresserMode::Off.to_string(), "Off");
        assert_eq!(PresserMode::Auto.to_string(), "Auto");
        assert_eq!(PresserMode::On.to_string(), "On");
    }

    #[test]
    fn test_xfer_style_display() {
        assert_eq!(XferStyle::FourPass.to_string(), "FourPass");
        assert_eq!(XferStyle::TwoPass.to_string(), "TwoPass");
    }
}
