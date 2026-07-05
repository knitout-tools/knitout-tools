use std::fmt::{Display, Error, Formatter, Result};
use std::hash::{Hash, Hasher};

use bigdecimal::BigDecimal;
use either::Either;

use crate::{
    phase::{Phase, Validated},
    program::{CarrierTable, DisplayCarrierTable},
};

/// A carrier, which is essentially a variable in knitout. Use a `CarrierTable` to construct
/// carriers. Don't mix carriers from different tables; you will get undefined behavior or panic! if
/// so. One table, one program, many carriers.
#[derive(Copy, Clone, Debug)]
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

    /// Create a carrier with the same id and table_id but a different annotation, for use when
    /// mapping between phases.
    pub fn map_annotation<Q: Phase>(&self, annotation: Q::Carrier) -> Carrier<Q> {
        Carrier {
            id: self.id,
            annotation,
            table_id: self.table_id,
        }
    }
}

// Manual impls so the bounds are `P: Phase` (not the spurious `P: Hash + Eq` that
// `#[derive]` would generate, since it conservatively adds bounds on the type parameter
// rather than on the associated type `P::Carrier`).
impl<P: Phase> PartialEq for Carrier<P> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.table_id == other.table_id
    }
}

impl<P: Phase> Eq for Carrier<P> {}

impl<P: Phase> Hash for Carrier<P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.table_id.hash(state);
    }
}

impl DisplayCarrierTable for Carrier<Validated> {
    fn fmt_table(&self, f: &mut Formatter<'_>, table: &CarrierTable) -> Result {
        write!(f, "{}", table.carrier_name_option(self).ok_or(Error)?)
    }
}

/// A direction of movement: Left (-) or Right (+). Oriented with respect to the front bed
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Direction<P: Phase> {
    Left(P::Direction),
    Right(P::Direction),
    Other(P::Direction, P::Other),
}

impl Display for Direction<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Direction::Left(_) => write!(f, "-"),
            Direction::Right(_) => write!(f, "+"),
            Direction::Other(_, x) => match *x {},
        }
    }
}

/// A machine's bed: Front (f) or Back (b).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Bed<P: Phase> {
    Front(P::Bed),
    Back(P::Bed),
    Other(P::Bed, P::Other),
}

impl Display for Bed<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Bed::Front(_) => write!(f, "f"),
            Bed::Back(_) => write!(f, "b"),
            Bed::Other(_, x) => match *x {},
        }
    }
}

/// Whether a needle is a slider (s) or not.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum IsSlider<P: Phase> {
    Slider(P::IsSlider),
    NotSlider(P::IsSlider),
    Other(P::IsSlider, P::Other),
}

impl Display for IsSlider<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            IsSlider::Slider(_) => write!(f, "s"),
            IsSlider::NotSlider(_) => write!(f, ""),
            IsSlider::Other(_, x) => match *x {},
        }
    }
}

/// A needle struct: a triple of a bed, whether it's a slider, and an index into the bed.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Needle<P: Phase> {
    pub(crate) bed: Bed<P>,
    pub(crate) is_slider: IsSlider<P>,
    pub(crate) index: MyI32<P>,
    pub(crate) annotation: P::Needle,
}

impl<P: Phase> Needle<P> {
    pub fn new(
        bed: Bed<P>,
        is_slider: IsSlider<P>,
        index: MyI32<P>,
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

    pub fn index(&self) -> &MyI32<P> {
        &self.index
    }

    pub fn annotation(&self) -> &P::Needle {
        &self.annotation
    }
}

impl Display for Needle<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}{}{}", self.bed, self.is_slider, self.index)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MyBigDecimal<P: Phase> {
    value: Either<BigDecimal, P::Other>,
    annotation: P::MyBigDecimal,
}

impl<P: Phase> MyBigDecimal<P> {
    pub fn new(value: Either<BigDecimal, P::Other>, annotation: P::MyBigDecimal) -> Self {
        Self { value, annotation }
    }

    pub fn value(&self) -> &Either<BigDecimal, P::Other> {
        &self.value
    }

    pub fn annotation(&self) -> &P::MyBigDecimal {
        &self.annotation
    }
}
impl Display for MyBigDecimal<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.value {
            Either::Left(v) => write!(f, "{}", v),
            Either::Right(x) => match *x {},
        }
    }
}
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MyU32<P: Phase> {
    value: Either<u32, P::Other>,
    annotation: P::MyU32,
}
impl<P: Phase> MyU32<P> {
    pub fn new(value: Either<u32, P::Other>, annotation: P::MyU32) -> Self {
        Self { value, annotation }
    }

    pub fn value(&self) -> &Either<u32, P::Other> {
        &self.value
    }

    pub fn annotation(&self) -> &P::MyU32 {
        &self.annotation
    }
}
impl Display for MyU32<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.value {
            Either::Left(v) => write!(f, "{}", v),
            Either::Right(x) => match *x {},
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MyI32<P: Phase> {
    value: Either<i32, P::Other>,
    annotation: P::MyI32,
}
impl<P: Phase> MyI32<P> {
    pub fn new(value: Either<i32, P::Other>, annotation: P::MyI32) -> Self {
        Self { value, annotation }
    }

    pub fn value(&self) -> &Either<i32, P::Other> {
        &self.value
    }

    pub fn annotation(&self) -> &P::MyI32 {
        &self.annotation
    }
}
impl Display for MyI32<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.value {
            Either::Left(v) => write!(f, "{}", v),
            Either::Right(x) => match *x {},
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MyString<P: Phase> {
    value: Either<String, P::Other>,
    annotation: P::MyString,
}
impl<P: Phase> MyString<P> {
    pub fn new(value: Either<String, P::Other>, annotation: P::MyString) -> Self {
        Self { value, annotation }
    }

    pub fn value(&self) -> &Either<String, P::Other> {
        &self.value
    }

    pub fn annotation(&self) -> &P::MyString {
        &self.annotation
    }
}
impl Display for MyString<Validated> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.value {
            Either::Left(v) => write!(f, "{}", v),
            Either::Right(x) => match *x {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::phase::{Annotated, Validated};

    #[test]
    fn test_needle_getters() {
        let needle = Needle::<Validated>::new(
            Bed::Front(()),
            IsSlider::Slider(()),
            MyI32::new(Either::Left(5i32), ()),
            (),
        );
        assert_eq!(*needle.bed(), Bed::Front(()));
        assert_eq!(*needle.is_slider(), IsSlider::Slider(()));
        assert_eq!(*needle.index(), MyI32::new(Either::Left(5i32), ()));
    }

    #[test]
    fn test_carrier_equality() {
        let c1 = Carrier::<Validated>::new(1, (), 42);
        let c2 = Carrier::<Validated>::new(1, (), 42);
        let c3 = Carrier::<Validated>::new(2, (), 42);
        assert_eq!(c1, c2);
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_carier_hash() {
        use std::collections::hash_map::DefaultHasher;

        let c1 = Carrier::<Validated>::new(1, (), 42);
        let c2 = Carrier::<Validated>::new(1, (), 42);
        let c3 = Carrier::<Validated>::new(2, (), 42);

        let mut hasher1 = DefaultHasher::new();
        c1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        c2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        let mut hasher3 = DefaultHasher::new();
        c3.hash(&mut hasher3);
        let hash3 = hasher3.finish();

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_my_big_decimal_display() {
        let value = MyBigDecimal::<Validated>::new(Either::Left(BigDecimal::from(42)), ());
        assert_eq!(value.to_string(), "42");
    }

    #[test]
    fn test_needle_display() {
        let needle = Needle::<Validated>::new(
            Bed::Front(()),
            IsSlider::Slider(()),
            MyI32::new(Either::Left(5i32), ()),
            (),
        );
        assert_eq!(needle.to_string(), "fs5");
    }
}
