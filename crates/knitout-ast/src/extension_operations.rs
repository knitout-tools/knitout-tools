use std::fmt::{Display, Formatter, Result};

use either::Either;

use crate::{
    base_operations::fmt_carriers,
    knitting_types::{Carrier, Direction, MyBigDecimal, MyString, MyU32, Needle},
    phase::{Phase, Validated},
    program::DisplayCarrierTable,
};

/// An knitout extension operation `x-...`. Known extensions are parsed into their own variants,
/// while unknown extensions are stored in the `UnknownExtension` variant.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ExtensionOperation<P: Phase> {
    XStitchNumber {
        value: MyU32<P>,
    },
    XSpeedNumber {
        value: MyU32<P>,
    },
    XSubRollerNumber {
        value: MyU32<P>,
    },
    XPresserMode {
        mode: PresserMode<P>,
    },
    XXferStitchNumber {
        value: MyU32<P>,
    },
    XEndPass,
    XVisColor {
        color: MyString<P>,
        carriers: Vec<Carrier<P>>,
    },
    XLoopIn {
        direction: Direction<P>,
        needle: Either<Needle<P>, P::Other>,
        carriers: Vec<Carrier<P>>,
    },
    XYarnIn {
        direction: Direction<P>,
        needle: Either<Needle<P>, P::Other>,
        carriers: Vec<Carrier<P>>,
    },
    XYarnOut {
        direction: Direction<P>,
        needle: Either<Needle<P>, P::Other>,
        carriers: Vec<Carrier<P>>,
    },
    XEnd,
    XRollerAdvance {
        value: MyBigDecimal<P>,
    },
    XAddRollerAdvance {
        value: MyBigDecimal<P>,
    },
    XCarrierSpacing {
        value: MyU32<P>,
    },
    XCarrierStoppingDistance {
        /// A parsed spacing of "0.5" is represented using 0 here. That is, carriers move `value` +
        /// 0.5 needles past the needle they just operated on.
        value: MyU32<P>,
    },
    XXferStyle {
        style: XferStyle<P>,
    },
    UnknownExtension {
        /// Includes the leading "x-"
        name: MyString<P>,
        arguments: Vec<MyString<P>>,
    },
}

/// Valid presser modes for the `x-presser-mode` extension operation.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PresserMode<P: Phase> {
    Off(P::PresserMode),
    Auto(P::PresserMode),
    On(P::PresserMode),
    Other(P::Other),
}

impl Display for PresserMode<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            PresserMode::Off(_) => write!(f, "Off"),
            PresserMode::Auto(_) => write!(f, "Auto"),
            PresserMode::On(_) => write!(f, "On"),
            PresserMode::Other(x) => match *x {},
        }
    }
}

/// Valid transfer styles for the `x-xfer-style` extension operation.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum XferStyle<P: Phase> {
    FourPass(P::XferStyle),
    TwoPass(P::XferStyle),
    Other(P::Other),
}

impl Display for XferStyle<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            XferStyle::FourPass(_) => write!(f, "FourPass"),
            XferStyle::TwoPass(_) => write!(f, "TwoPass"),
            XferStyle::Other(x) => match *x {},
        }
    }
}

impl DisplayCarrierTable for ExtensionOperation<Validated> {
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

    use crate::phase::Validated;

    #[test]
    fn test_presser_mode_display() {
        assert_eq!(PresserMode::<Validated>::Off(()).to_string(), "Off");
        assert_eq!(PresserMode::<Validated>::Auto(()).to_string(), "Auto");
        assert_eq!(PresserMode::<Validated>::On(()).to_string(), "On");
    }

    #[test]
    fn test_xfer_style_display() {
        assert_eq!(XferStyle::<Validated>::FourPass(()).to_string(), "FourPass");
        assert_eq!(XferStyle::<Validated>::TwoPass(()).to_string(), "TwoPass");
    }
}
