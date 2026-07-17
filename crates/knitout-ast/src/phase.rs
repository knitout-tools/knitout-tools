use std::fmt::Debug;
use std::hash::Hash;

use either::Either;

use crate::Provenance;

pub trait PhaseType: Debug + Hash + PartialEq + Eq + Clone {}

impl<T> PhaseType for T where T: Debug + Hash + PartialEq + Eq + Clone {}

/// An uninhabited stand-in for `Infallible` that also derives serde, used as the `Other` type of a
/// fully-validated program (where no `Other` node can exist).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Never {}

impl std::fmt::Display for Never {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

/// A `Phase` whose associated types are all serde-(de)serializable. Deriving `Serialize` /
/// `Deserialize` on the phase-generic AST types only requires `P: crate::SerdePhase` thanks to this alias.
#[cfg(feature = "serde")]
pub trait SerdePhase:
    Phase<
        Other: serde::Serialize + serde::de::DeserializeOwned,
        MagicVersion: serde::Serialize + serde::de::DeserializeOwned,
        Operation: serde::Serialize + serde::de::DeserializeOwned,
        Header: serde::Serialize + serde::de::DeserializeOwned,
        UnknownHeader: serde::Serialize + serde::de::DeserializeOwned,
        PositionHeaderValue: serde::Serialize + serde::de::DeserializeOwned,
        CarriersHeaderValue: serde::Serialize + serde::de::DeserializeOwned,
        Rail: serde::Serialize + serde::de::DeserializeOwned,
        RailSide: serde::Serialize + serde::de::DeserializeOwned,
        Carrier: serde::Serialize + serde::de::DeserializeOwned,
        Direction: serde::Serialize + serde::de::DeserializeOwned,
        Bed: serde::Serialize + serde::de::DeserializeOwned,
        IsSlider: serde::Serialize + serde::de::DeserializeOwned,
        Index: serde::Serialize + serde::de::DeserializeOwned,
        Needle: serde::Serialize + serde::de::DeserializeOwned,
        MyBigDecimal: serde::Serialize + serde::de::DeserializeOwned,
        MyU32: serde::Serialize + serde::de::DeserializeOwned,
        MyI32: serde::Serialize + serde::de::DeserializeOwned,
        MyString: serde::Serialize + serde::de::DeserializeOwned,
        PresserMode: serde::Serialize + serde::de::DeserializeOwned,
        XferStyle: serde::Serialize + serde::de::DeserializeOwned,
    >
{
}

#[cfg(feature = "serde")]
impl<P> SerdePhase for P where
    P: Phase<
            Other: serde::Serialize + serde::de::DeserializeOwned,
            MagicVersion: serde::Serialize + serde::de::DeserializeOwned,
            Operation: serde::Serialize + serde::de::DeserializeOwned,
            Header: serde::Serialize + serde::de::DeserializeOwned,
            UnknownHeader: serde::Serialize + serde::de::DeserializeOwned,
            PositionHeaderValue: serde::Serialize + serde::de::DeserializeOwned,
            CarriersHeaderValue: serde::Serialize + serde::de::DeserializeOwned,
            Rail: serde::Serialize + serde::de::DeserializeOwned,
            RailSide: serde::Serialize + serde::de::DeserializeOwned,
            Carrier: serde::Serialize + serde::de::DeserializeOwned,
            Direction: serde::Serialize + serde::de::DeserializeOwned,
            Bed: serde::Serialize + serde::de::DeserializeOwned,
            IsSlider: serde::Serialize + serde::de::DeserializeOwned,
            Index: serde::Serialize + serde::de::DeserializeOwned,
            Needle: serde::Serialize + serde::de::DeserializeOwned,
            MyBigDecimal: serde::Serialize + serde::de::DeserializeOwned,
            MyU32: serde::Serialize + serde::de::DeserializeOwned,
            MyI32: serde::Serialize + serde::de::DeserializeOwned,
            MyString: serde::Serialize + serde::de::DeserializeOwned,
            PresserMode: serde::Serialize + serde::de::DeserializeOwned,
            XferStyle: serde::Serialize + serde::de::DeserializeOwned,
        >
{
}

/// Trees that grow
pub trait Phase {
    type Other: PhaseType;

    type MagicVersion: PhaseType;
    type Operation: PhaseType;

    type Header: PhaseType;
    type UnknownHeader: PhaseType;
    type PositionHeaderValue: PhaseType;
    type CarriersHeaderValue: PhaseType;

    type Rail: PhaseType;
    type RailSide: PhaseType;

    type Carrier: PhaseType;
    type Direction: PhaseType;
    type Bed: PhaseType;
    type IsSlider: PhaseType;
    type Index: PhaseType;
    type Needle: PhaseType;

    type MyBigDecimal: PhaseType;
    type MyU32: PhaseType;
    type MyI32: PhaseType;
    type MyString: PhaseType;

    type PresserMode: PhaseType;
    type XferStyle: PhaseType;
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Validated {}

impl Phase for Validated {
    type Other = Never;

    type MagicVersion = ();
    type Operation = Provenance;

    type Header = ();
    type UnknownHeader = ();
    type PositionHeaderValue = ();
    type CarriersHeaderValue = ();

    type Rail = ();
    type RailSide = ();

    type Carrier = ();
    type Direction = ();
    type Bed = ();
    type IsSlider = ();
    type Index = ();
    type Needle = ();

    type MyBigDecimal = ();
    type MyU32 = ();
    type MyI32 = ();
    type MyString = ();

    type PresserMode = ();
    type XferStyle = ();
}

/// Quick helper function to say "nope, it's been validated; gimme!"
pub fn valid<T>(val: &Either<T, Never>) -> &T {
    match val {
        Either::Left(v) => v,
        Either::Right(i) => match *i {},
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Annotated<T, A> {
    pub value: T,
    pub annotation: A,
}

impl<T, A> Annotated<T, A> {
    pub fn new(value: T, annotation: A) -> Self {
        Self { value, annotation }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Annotated<U, A> {
        Annotated {
            value: f(self.value),
            annotation: self.annotation,
        }
    }

    pub fn as_ref(&self) -> Annotated<&T, &A> {
        Annotated {
            value: &self.value,
            annotation: &self.annotation,
        }
    }
}

impl<T> Annotated<T, ()> {
    pub fn unit(value: T) -> Self {
        Self {
            value,
            annotation: (),
        }
    }
}
