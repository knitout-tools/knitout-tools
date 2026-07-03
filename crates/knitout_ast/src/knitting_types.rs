use std::fmt::{Display, Error, Formatter, Result};

use crate::program::{CarrierTable, DisplayCarrierTable};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Carrier {
    id: u32,
}

impl DisplayCarrierTable for Carrier {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        write!(f, "{}", table.lookup(self.id).ok_or(Error)?)
    }
}

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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Needle {
    bed: Bed,
    is_slider: IsSlider,
    index: i32,
}

impl Display for Needle {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}{}{}", self.bed, self.is_slider, self.index)
    }
}
