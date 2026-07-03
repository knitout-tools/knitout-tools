use crate::knitting_types::{Carrier, Direction, Needle};
use num_rational::Ratio;

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
        before_pull: Ratio<i32>,
        after_pull: Ratio<i32>,
    },
    Rack {
        offset: Ratio<i32>,
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
        soruce: Needle,
        destination: Needle,
        carriers: Vec<Carrier>,
    },
    Pause,
}

