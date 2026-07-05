use std::fmt::{Display, Error, Formatter, Result};

use bigdecimal::BigDecimal;
use either::Either;

use crate::{
    phase::{Annotated, Phase, Validated},
    program::{CarrierTable, DisplayCarrierTable},
};

/// A carrier, which is essentially a variable in knitout. Use a `CarrierTable` to construct
/// carriers. Don't mix carriers from different tables; you will get undefined behavior or panic! if
/// so. One table, one program, many carriers.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Carrier<P: Phase> {
    id: u32,
    annotation: P::Carrier,
    table_id: u32,
}

impl<P: Phase> Carrier<P> {
    pub(crate) fn new(id: u32, annotation: P::Carrier, table_id: u32) -> Self {
        Self {
            id,
            annotation,
            table_id,
        }
    }

    pub(crate) fn id(&self) -> u32 {
        self.id
    }

    pub fn annotation(&self) -> &P::Carrier {
        &self.annotation
    }

    pub(crate) fn table_id(&self) -> u32 {
        self.table_id
    }
}

impl DisplayCarrierTable for Carrier<Validated> {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        write!(f, "{}", table.carrier_name_option(&self).ok_or(Error)?)
    }
}

/// A direction of movement: Left (-) or Right (+). Oriented with respect to the front bed
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Direction<P: Phase> {
    Left(P::Direction),
    Right(P::Direction),
    Other(P::Other),
}

impl Display for Direction<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Direction::Left(_) => write!(f, "-"),
            Direction::Right(_) => write!(f, "+"),
            Direction::Other(x) => match *x {},
        }
    }
}

/// A machine's bed: Front (f) or Back (b).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Bed<P: Phase> {
    Front(P::Bed),
    Back(P::Bed),
    Other(P::Other),
}

impl Display for Bed<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Bed::Front(_) => write!(f, "f"),
            Bed::Back(_) => write!(f, "b"),
            Bed::Other(x) => match *x {},
        }
    }
}

/// Whether a needle is a slider (s) or not.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum IsSlider<P: Phase> {
    Slider(P::IsSlider),
    NotSlider(P::IsSlider),
    Other(P::Other),
}

impl Display for IsSlider<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            IsSlider::Slider(_) => write!(f, "s"),
            IsSlider::NotSlider(_) => write!(f, ""),
            IsSlider::Other(x) => match *x {},
        }
    }
}

/// A needle struct: a triple of a bed, whether it's a slider, and an index into the bed.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Needle<P: Phase> {
    bed: Bed<P>,
    is_slider: IsSlider<P>,
    index: Annotated<i32, P::Index>,
    annotation: P::Needle,
}

impl<P: Phase> Needle<P> {
    pub fn new(
        bed: Bed<P>,
        is_slider: IsSlider<P>,
        index: Annotated<i32, P::Index>,
        annotation: P::Needle,
    ) -> Self {
        Self {
            bed,
            is_slider,
            index,
            annotation,
        }
    }

    pub fn bed(&self) -> &Bed<P> {
        &self.bed
    }

    pub fn is_slider(&self) -> &IsSlider<P> {
        &self.is_slider
    }

    pub fn index(&self) -> &Annotated<i32, P::Index> {
        &self.index
    }
}

impl Display for Needle<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}{}{}", self.bed, self.is_slider, self.index.value)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MyBigDecimal<P: Phase> {
    value: Either<Annotated<BigDecimal, P::MyBigDecimal>, P::Other>,
}

impl Display for MyBigDecimal<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.value {
            Either::Left(v) => write!(f, "{}", v.value),
            Either::Right(x) => match *x {},
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MyU32<P: Phase> {
    value: Either<Annotated<u32, P::MyU32>, P::Other>,
}

impl Display for MyU32<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.value {
            Either::Left(v) => write!(f, "{}", v.value),
            Either::Right(x) => match *x {},
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MyString<P: Phase> {
    value: Either<Annotated<String, P::MyString>, P::Other>,
}

impl Display for MyString<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.value {
            Either::Left(v) => write!(f, "{}", v.value),
            Either::Right(x) => match *x {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needle_getters() {
        let needle = Needle::new(Bed::Front, IsSlider::Slider, 5);
        assert_eq!(needle.bed(), Bed::Front);
        assert_eq!(needle.is_slider(), IsSlider::Slider);
        assert_eq!(needle.index(), 5);
    }
}
