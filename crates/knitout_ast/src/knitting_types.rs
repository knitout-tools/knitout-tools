use std::fmt::{Display, Error, Formatter, Result};

use crate::program::{CarrierTable, DisplayCarrierTable};

/// A carrier, which is essentially a variable in knitout. Use `CarrierTable` to construct carriers.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Carrier {
    id: u32,
}

impl Carrier {
    pub(crate) fn new(id: u32) -> Self {
        Self { id }
    }

    pub(crate) fn id(&self) -> u32 {
        self.id
    }
}

impl DisplayCarrierTable for Carrier {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        write!(f, "{}", table.carrier_name_option(*self).ok_or(Error)?)
    }
}

/// A direction of movement: Left (-) or Right (+). Oriented with respect to the front bed
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Direction {
    Left,
    Right,
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Direction::Left => write!(f, "-"),
            Direction::Right => write!(f, "+"),
        }
    }
}

/// A machine's bed: Front (f) or Back (b).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Bed {
    Front,
    Back,
}

impl Display for Bed {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Bed::Front => write!(f, "f"),
            Bed::Back => write!(f, "b"),
        }
    }
}

/// Whether a needle is a slider (s) or not.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum IsSlider {
    Slider,
    NotSlider,
}

impl Display for IsSlider {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            IsSlider::Slider => write!(f, "s"),
            IsSlider::NotSlider => write!(f, ""),
        }
    }
}

/// A needle struct: a triple of a bed, whether it's a slider, and an index into the bed.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Needle {
    bed: Bed,
    is_slider: IsSlider,
    index: i32,
}

impl Needle {
    pub fn new(bed: Bed, is_slider: IsSlider, index: i32) -> Self {
        Self {
            bed,
            is_slider,
            index,
        }
    }

    pub fn bed(&self) -> Bed {
        self.bed
    }

    pub fn is_slider(&self) -> IsSlider {
        self.is_slider
    }

    pub fn index(&self) -> i32 {
        self.index
    }
}

impl Display for Needle {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}{}{}", self.bed, self.is_slider, self.index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needele_getters() {
        let needle = Needle::new(Bed::Front, IsSlider::Slider, 5);
        assert_eq!(needle.bed(), Bed::Front);
        assert_eq!(needle.is_slider(), IsSlider::Slider);
        assert_eq!(needle.index(), 5);
    }
}
