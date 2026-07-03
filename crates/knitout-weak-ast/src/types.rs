use bigdecimal::BigDecimal;
use knitout_ast::{Carrier, CarrierTable, IsSlider, Span};

// ---------------------------------------------------------------------------
// Leaf-level enums — each has an Unknown(Span) variant for unrecognized text
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum WeakBed {
    Front,
    Back,
    Unknown(Span),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum WeakDirection {
    Left,
    Right,
    Unknown(Span),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum WeakPositionValue {
    Left,
    Center,
    Right,
    Keep,
    Unknown(Span),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum WeakPresserMode {
    Off,
    Auto,
    On,
    Unknown(Span),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum WeakXferStyle {
    FourPass,
    TwoPass,
    Unknown(Span),
}

// ---------------------------------------------------------------------------
// Composite types
// ---------------------------------------------------------------------------

/// A needle with a span covering the whole needle text. `bed` may be Unknown,
/// `index` is None when missing or unparseable.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct WeakNeedle {
    pub span: Span,
    pub bed: WeakBed,
    pub is_slider: IsSlider,
    pub index: Option<i32>,
}

// ---------------------------------------------------------------------------
// Top-level program
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct WeakProgram {
    pub span: Span,
    pub magic: WeakMagic,
    /// Headers in source order
    pub headers: Vec<WeakHeader>,
    /// Operation lines in source order
    pub lines: Vec<WeakLine>,
    /// All carrier names encountered during the walk
    pub carrier_table: CarrierTable,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WeakMagic {
    pub span: Span,
    /// None when the version number is absent or unparseable
    pub version: Option<u32>,
}

// ---------------------------------------------------------------------------
// Headers
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct WeakHeader {
    pub span: Span,
    pub kind: WeakHeaderKind,
}

#[derive(Clone, Debug)]
pub enum WeakHeaderKind {
    Carriers(WeakCarriersHeader),
    Gauge(WeakGaugeHeader),
    Width(WeakWidthHeader),
    Position(WeakPositionHeader),
    Machine(WeakMachineHeader),
    Yarn(WeakYarnHeader),
    Extension(WeakExtensionHeader),
    /// An ERROR node: the `span` on `WeakHeader` covers the bad text
    Unknown,
}

#[derive(Clone, Debug)]
pub struct WeakCarriersHeader {
    /// None when the value is absent (the grammar allows `;;Carriers:` with no value)
    pub value: Option<WeakCarriersValue>,
}

#[derive(Clone, Debug)]
pub struct WeakCarriersValue {
    pub span: Span,
    pub items: Vec<WeakCarrierOrRail>,
}

#[derive(Clone, Debug)]
pub enum WeakCarrierOrRail {
    Carrier(Carrier),
    Rail(WeakRail),
    /// An ERROR node inside the carriers value
    Unknown(Span),
}

#[derive(Clone, Debug)]
pub struct WeakRail {
    pub span: Span,
    pub left: WeakRailSide,
    pub right: WeakRailSide,
}

#[derive(Clone, Debug)]
pub struct WeakRailSide {
    pub span: Span,
    pub carriers: Vec<Carrier>,
}

#[derive(Clone, Debug)]
pub struct WeakGaugeHeader {
    /// None when the integer value is absent or unparseable
    pub value: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct WeakWidthHeader {
    pub value: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct WeakPositionHeader {
    /// Always present (WeakPositionValue::Unknown when unrecognized)
    pub value: WeakPositionValue,
}

#[derive(Clone, Debug)]
pub struct WeakMachineHeader {
    pub value: Option<String>,
}

#[derive(Clone, Debug)]
pub struct WeakYarnHeader {
    /// None when the carrier field is missing or errored
    pub carrier: Option<Carrier>,
    pub value: Option<String>,
}

#[derive(Clone, Debug)]
pub struct WeakExtensionHeader {
    pub name: Option<String>,
    pub value: Option<String>,
}

// ---------------------------------------------------------------------------
// Lines
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct WeakLine {
    pub span: Span,
    /// The operation on this line, if any
    pub operation: Option<WeakOperation>,
    /// A plain `;...` comment, if any
    pub comment: Option<WeakComment>,
    /// The source provenance string from `;!source: ...`, if present
    pub source: Option<String>,
}

#[derive(Clone, Debug)]
pub struct WeakComment {
    pub span: Span,
    pub text: Option<String>,
}

#[derive(Clone, Debug)]
pub enum WeakOperation {
    Base(WeakBaseOperation),
    Extension(WeakExtensionOperation),
    /// An ERROR node at the operation level; the span covers the bad text
    Unknown(Span),
}

// ---------------------------------------------------------------------------
// Base operations
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct WeakBaseOperation {
    pub span: Span,
    pub kind: WeakBaseOperationKind,
}

#[derive(Clone, Debug)]
pub enum WeakBaseOperationKind {
    /// Required carriers: None when the carrier set is missing or errored
    In {
        carriers: Option<Vec<Carrier>>,
    },
    Inhook {
        carriers: Option<Vec<Carrier>>,
    },
    Releasehook {
        carriers: Option<Vec<Carrier>>,
    },
    Out {
        carriers: Option<Vec<Carrier>>,
    },
    Outhook {
        carriers: Option<Vec<Carrier>>,
    },
    Stitch {
        before_pull: Option<BigDecimal>,
        after_pull: Option<BigDecimal>,
    },
    Rack {
        offset: Option<BigDecimal>,
    },
    Knit {
        direction: WeakDirection,
        needle: Option<WeakNeedle>,
        /// Optional in the grammar; empty vec means absent (valid)
        carriers: Vec<Carrier>,
    },
    Tuck {
        direction: WeakDirection,
        needle: Option<WeakNeedle>,
        carriers: Vec<Carrier>,
    },
    Miss {
        direction: WeakDirection,
        needle: Option<WeakNeedle>,
        carriers: Vec<Carrier>,
    },
    Split {
        direction: WeakDirection,
        source: Option<WeakNeedle>,
        destination: Option<WeakNeedle>,
        carriers: Vec<Carrier>,
    },
    Pause,
    Drop {
        needle: Option<WeakNeedle>,
    },
    Amiss {
        needle: Option<WeakNeedle>,
    },
    Xfer {
        source: Option<WeakNeedle>,
        destination: Option<WeakNeedle>,
    },
}

// ---------------------------------------------------------------------------
// Extension operations
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct WeakExtensionOperation {
    pub span: Span,
    pub kind: WeakExtensionOperationKind,
}

#[derive(Clone, Debug)]
pub enum WeakExtensionOperationKind {
    XStitchNumber {
        value: Option<u32>,
    },
    XSpeedNumber {
        value: Option<u32>,
    },
    XSubRollerNumber {
        value: Option<u32>,
    },
    XPresserMode {
        mode: WeakPresserMode,
    },
    XXferStitchNumber {
        value: Option<u32>,
    },
    XEndPass,
    XVisColor {
        color: Option<String>,
        /// Required in the grammar; None when the carrier set is missing/errored
        carriers: Option<Vec<Carrier>>,
    },
    XLoopIn {
        direction: WeakDirection,
        needle: Option<WeakNeedle>,
        carriers: Option<Vec<Carrier>>,
    },
    XYarnIn {
        direction: WeakDirection,
        needle: Option<WeakNeedle>,
        carriers: Option<Vec<Carrier>>,
    },
    XYarnOut {
        direction: WeakDirection,
        needle: Option<WeakNeedle>,
        carriers: Option<Vec<Carrier>>,
    },
    XEnd,
    XRollerAdvance {
        value: Option<BigDecimal>,
    },
    XAddRollerAdvance {
        value: Option<BigDecimal>,
    },
    XCarrierSpacing {
        value: Option<u32>,
    },
    XCarrierStoppingDistance {
        value: Option<u32>,
    },
    XXferStyle {
        style: WeakXferStyle,
    },
    UnknownExtension {
        name: Option<String>,
        arguments: Vec<String>,
    },
}
