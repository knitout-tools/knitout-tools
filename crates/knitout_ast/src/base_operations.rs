use std::fmt::{Formatter, Result};

use bigdecimal::BigDecimal;

use crate::{
    knitting_types::{Carrier, Direction, Needle},
    program::{CarrierTable, DisplayCarrierTable},
};

/// A basic knitout operation. Excludes extension operations, which are a different enum. Also
/// excludes syntactic sugar operations; faux constructors are provided.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum BaseOperation {
    In {
        carriers: Vec<Carrier>,
    },
    Inhook {
        carriers: Vec<Carrier>,
    },
    Releasehook {
        carriers: Vec<Carrier>,
    },
    Out {
        carriers: Vec<Carrier>,
    },
    Outhook {
        carriers: Vec<Carrier>,
    },
    Stitch {
        before_pull: BigDecimal,
        after_pull: BigDecimal,
    },
    Rack {
        offset: BigDecimal,
    },
    Knit {
        direction: Direction,
        needle: Needle,
        carriers: Vec<Carrier>,
    },
    Tuck {
        direction: Direction,
        needle: Needle,
        carriers: Vec<Carrier>,
    },
    Miss {
        direction: Direction,
        needle: Needle,
        carriers: Vec<Carrier>,
    },
    Split {
        direction: Direction,
        source: Needle,
        destination: Needle,
        carriers: Vec<Carrier>,
    },
    Pause,
    Drop {
        needle: Needle,
    },
    Amiss {
        needle: Needle,
    },
    Xfer {
        source: Needle,
        destination: Needle,
    },
}

pub(crate) fn fmt_carriers(
    f: &mut Formatter<'_>,
    carriers: &[Carrier],
    table: &CarrierTable,
) -> Result {
    for (i, carrier) in carriers.iter().enumerate() {
        if i > 0 {
            write!(f, " ")?;
        }
        carrier.fmt_table(f, table)?;
    }
    Ok(())
}

impl DisplayCarrierTable for BaseOperation {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        match self {
            BaseOperation::In { carriers } => {
                write!(f, "in ")?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Inhook { carriers } => {
                write!(f, "inhook ")?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Releasehook { carriers } => {
                write!(f, "releasehook ")?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Out { carriers } => {
                write!(f, "out ")?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Outhook { carriers } => {
                write!(f, "outhook ")?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Stitch {
                before_pull,
                after_pull,
            } => {
                write!(f, "stitch {} {}", before_pull, after_pull)
            }
            BaseOperation::Rack { offset } => {
                write!(f, "rack {}", offset)
            }
            BaseOperation::Knit {
                direction,
                needle,
                carriers,
            } => {
                write!(f, "knit {} {}", direction, needle)?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Tuck {
                direction,
                needle,
                carriers,
            } => {
                write!(f, "tuck {} {}", direction, needle)?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Miss {
                direction,
                needle,
                carriers,
            } => {
                write!(f, "miss {} {}", direction, needle)?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Split {
                direction,
                source: soruce,
                destination,
                carriers,
            } => {
                write!(f, "split {} {} {} ", direction, soruce, destination)?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Pause => {
                write!(f, "pause")
            }
            BaseOperation::Drop { needle } => {
                write!(f, "drop {}", needle)
            }
            BaseOperation::Amiss { needle } => {
                write!(f, "amiss {}", needle)
            }
            BaseOperation::Xfer {
                source,
                destination,
            } => {
                write!(f, "xfer {} {}", source, destination)
            }
        }
    }
}
