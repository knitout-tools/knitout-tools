use std::fmt::Debug;
use std::hash::Hash;

use either::Either;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::Provenance;

pub trait PhaseType: Debug + Hash + PartialEq + Eq + Clone {}

impl<T> PhaseType for T where T: Debug + Hash + PartialEq + Eq + Clone {}

/// An uninhabited stand-in for `Infallible` that also derives serde, used as the `Other` type of a
/// fully-validated program (where no `Other` node can exist).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Never {}

impl std::fmt::Display for Never {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

/// A `Phase` whose associated types are all serde-(de)serializable. Deriving `Serialize` /
/// `Deserialize` on the phase-generic AST types only requires `P: SerdePhase` thanks to this alias.
pub trait SerdePhase:
    Phase<
        Other: Serialize + DeserializeOwned,
        MagicVersion: Serialize + DeserializeOwned,
        Operation: Serialize + DeserializeOwned,
        Header: Serialize + DeserializeOwned,
        UnknownHeader: Serialize + DeserializeOwned,
        PositionHeaderValue: Serialize + DeserializeOwned,
        CarriersHeaderValue: Serialize + DeserializeOwned,
        Rail: Serialize + DeserializeOwned,
        RailSide: Serialize + DeserializeOwned,
        Carrier: Serialize + DeserializeOwned,
        Direction: Serialize + DeserializeOwned,
        Bed: Serialize + DeserializeOwned,
        IsSlider: Serialize + DeserializeOwned,
        Index: Serialize + DeserializeOwned,
        Needle: Serialize + DeserializeOwned,
        MyBigDecimal: Serialize + DeserializeOwned,
        MyU32: Serialize + DeserializeOwned,
        MyI32: Serialize + DeserializeOwned,
        MyString: Serialize + DeserializeOwned,
        PresserMode: Serialize + DeserializeOwned,
        XferStyle: Serialize + DeserializeOwned,
    >
{
}

impl<P> SerdePhase for P where
    P: Phase<
            Other: Serialize + DeserializeOwned,
            MagicVersion: Serialize + DeserializeOwned,
            Operation: Serialize + DeserializeOwned,
            Header: Serialize + DeserializeOwned,
            UnknownHeader: Serialize + DeserializeOwned,
            PositionHeaderValue: Serialize + DeserializeOwned,
            CarriersHeaderValue: Serialize + DeserializeOwned,
            Rail: Serialize + DeserializeOwned,
            RailSide: Serialize + DeserializeOwned,
            Carrier: Serialize + DeserializeOwned,
            Direction: Serialize + DeserializeOwned,
            Bed: Serialize + DeserializeOwned,
            IsSlider: Serialize + DeserializeOwned,
            Index: Serialize + DeserializeOwned,
            Needle: Serialize + DeserializeOwned,
            MyBigDecimal: Serialize + DeserializeOwned,
            MyU32: Serialize + DeserializeOwned,
            MyI32: Serialize + DeserializeOwned,
            MyString: Serialize + DeserializeOwned,
            PresserMode: Serialize + DeserializeOwned,
            XferStyle: Serialize + DeserializeOwned,
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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
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
