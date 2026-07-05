use std::fmt::{Formatter, Result};

use either::Either;
use serde::{Deserialize, Serialize};

use crate::phase::Never;
use crate::{
    knitting_types::{Carrier, Direction, MyBigDecimal, Needle},
    phase::{Phase, SerdePhase, Validated, valid},
    program::{CarrierTable, DisplayCarrierTable},
};

/// A basic knitout operation. Excludes extension operations, which are a different enum. Also
/// excludes syntactic sugar operations; faux constructors are provided.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(bound(serialize = "P: SerdePhase", deserialize = "P: SerdePhase"))]
pub enum BaseOperation<P: Phase> {
    In {
        carriers: Vec<Either<Carrier<P>, P::Other>>,
    },
    Inhook {
        carriers: Vec<Either<Carrier<P>, P::Other>>,
    },
    Releasehook {
        carriers: Vec<Either<Carrier<P>, P::Other>>,
    },
    Out {
        carriers: Vec<Either<Carrier<P>, P::Other>>,
    },
    Outhook {
        carriers: Vec<Either<Carrier<P>, P::Other>>,
    },
    Stitch {
        before_pull: MyBigDecimal<P>,
        after_pull: MyBigDecimal<P>,
    },
    Rack {
        offset: MyBigDecimal<P>,
    },
    Knit {
        direction: Direction<P>,
        needle: Either<Needle<P>, P::Other>,
        carriers: Vec<Either<Carrier<P>, P::Other>>,
    },
    Tuck {
        direction: Direction<P>,
        needle: Either<Needle<P>, P::Other>,
        carriers: Vec<Either<Carrier<P>, P::Other>>,
    },
    Miss {
        direction: Direction<P>,
        needle: Either<Needle<P>, P::Other>,
        carriers: Vec<Either<Carrier<P>, P::Other>>,
    },
    Split {
        direction: Direction<P>,
        source: Either<Needle<P>, P::Other>,
        destination: Either<Needle<P>, P::Other>,
        carriers: Vec<Either<Carrier<P>, P::Other>>,
    },
    Pause,
    Drop {
        needle: Either<Needle<P>, P::Other>,
    },
    Amiss {
        needle: Either<Needle<P>, P::Other>,
    },
    Xfer {
        source: Either<Needle<P>, P::Other>,
        destination: Either<Needle<P>, P::Other>,
    },
}

/// Writes a leading space in front of every carrier, including the first one
pub(crate) fn fmt_carriers(
    f: &mut Formatter<'_>,
    carriers: &[Either<Carrier<Validated>, Never>],
    table: &CarrierTable,
) -> Result {
    for carrier in carriers.iter() {
        write!(f, " ")?;
        valid(carrier).fmt_table(f, table)?;
    }
    Ok(())
}

impl DisplayCarrierTable for BaseOperation<Validated> {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        match self {
            BaseOperation::In { carriers } => {
                write!(f, "in")?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Inhook { carriers } => {
                write!(f, "inhook")?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Releasehook { carriers } => {
                write!(f, "releasehook")?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Out { carriers } => {
                write!(f, "out")?;
                fmt_carriers(f, carriers, table)
            }
            BaseOperation::Outhook { carriers } => {
                write!(f, "outhook")?;
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
                source,
                destination,
                carriers,
            } => {
                write!(f, "split {} {} {}", direction, source, destination)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    use crate::phase::Validated;

    // A throwaway wrapper so we can drive a real Formatter through the standard machinery.
    struct FmtCarriersHarness<'a> {
        carriers: &'a [Either<Carrier<Validated>, Never>],
        table: &'a CarrierTable,
    }

    impl<'a> fmt::Display for FmtCarriersHarness<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            fmt_carriers(f, self.carriers, self.table)
        }
    }

    fn render(carriers: &[Either<Carrier<Validated>, Never>], table: &CarrierTable) -> String {
        FmtCarriersHarness { carriers, table }.to_string()
    }

    #[test]
    fn empty_carriers_produces_empty_string() {
        let table = CarrierTable::new();
        assert_eq!(render(&[], &table), "");
    }

    #[test]
    fn single_carrier_has_leading_space() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        assert_eq!(render(&[Either::Left(c)], &table), " 1");
    }

    #[test]
    fn multiple_carriers_are_space_separated() {
        let mut table = CarrierTable::new();
        let c1 = table.make_carrier("1", ());
        let c2 = table.make_carrier("2", ());
        let c3 = table.make_carrier("3", ());
        assert_eq!(
            render(
                &[Either::Left(c1), Either::Left(c2), Either::Left(c3)],
                &table
            ),
            " 1 2 3"
        );
    }
}
