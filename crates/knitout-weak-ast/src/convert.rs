//! Converters from weak AST nodes to their strong knitout-ast equivalents.
//!
//! Each converter returns `Result<Strong, ParseDiagnostic>` — the first
//! encountered deficiency wins; nothing beyond that is checked.

use knitout_ast::{
    BaseOperation, Bed, CarrierOrRail, CarriersHeaderValue, Direction, ExtensionOperation, Header,
    Needle, Operation, PositionHeaderValue, PresserMode, Program, Provenance, Rail, RailSide, Span,
    XferStyle,
};

use crate::diagnostics::ParseDiagnostic;
use crate::types::*;

// ---------------------------------------------------------------------------
// Leaf converters
// ---------------------------------------------------------------------------

impl WeakBed {
    pub fn try_into_strong(self) -> Result<Bed, ParseDiagnostic> {
        match self {
            WeakBed::Front => Ok(Bed::Front),
            WeakBed::Back => Ok(Bed::Back),
            WeakBed::Unknown(span) => Err(ParseDiagnostic::new(span, "unrecognized bed")),
        }
    }
}

impl WeakDirection {
    pub fn try_into_strong(self) -> Result<Direction, ParseDiagnostic> {
        match self {
            WeakDirection::Left => Ok(Direction::Left),
            WeakDirection::Right => Ok(Direction::Right),
            WeakDirection::Unknown(span) => {
                Err(ParseDiagnostic::new(span, "unrecognized direction"))
            }
        }
    }
}

impl WeakPresserMode {
    pub fn try_into_strong(self) -> Result<PresserMode, ParseDiagnostic> {
        match self {
            WeakPresserMode::Off => Ok(PresserMode::Off),
            WeakPresserMode::Auto => Ok(PresserMode::Auto),
            WeakPresserMode::On => Ok(PresserMode::On),
            WeakPresserMode::Unknown(span) => {
                Err(ParseDiagnostic::new(span, "unrecognized presser mode"))
            }
        }
    }
}

impl WeakXferStyle {
    pub fn try_into_strong(self) -> Result<XferStyle, ParseDiagnostic> {
        match self {
            WeakXferStyle::FourPass => Ok(XferStyle::FourPass),
            WeakXferStyle::TwoPass => Ok(XferStyle::TwoPass),
            WeakXferStyle::Unknown(span) => {
                Err(ParseDiagnostic::new(span, "unrecognized xfer style"))
            }
        }
    }
}

impl WeakPositionValue {
    pub fn try_into_strong(self) -> Result<PositionHeaderValue, ParseDiagnostic> {
        match self {
            WeakPositionValue::Left => Ok(PositionHeaderValue::Left),
            WeakPositionValue::Center => Ok(PositionHeaderValue::Center),
            WeakPositionValue::Right => Ok(PositionHeaderValue::Right),
            WeakPositionValue::Keep => Ok(PositionHeaderValue::Keep),
            WeakPositionValue::Unknown(span) => {
                Err(ParseDiagnostic::new(span, "unrecognized position value"))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Needle
// ---------------------------------------------------------------------------

impl WeakNeedle {
    pub fn try_into_strong(self) -> Result<Needle, ParseDiagnostic> {
        let span = self.span;
        let bed = self.bed.try_into_strong()?;
        let is_slider = self.is_slider;
        let index = self
            .index
            .ok_or_else(|| ParseDiagnostic::new(span, "missing needle index"))?;
        Ok(Needle::new(bed, is_slider, index))
    }
}

// ---------------------------------------------------------------------------
// Program
// ---------------------------------------------------------------------------

impl WeakProgram {
    pub fn try_into_strong(self) -> Result<Program, ParseDiagnostic> {
        let version = self
            .magic
            .version
            .ok_or_else(|| ParseDiagnostic::new(self.magic.span, "missing version number"))?;

        let (strong_header, carrier_table) = convert_headers(self.headers, self.carrier_table)?;

        let mut program = Program::new(version, strong_header, carrier_table);

        for weak_line in self.lines {
            if let Some(op) = weak_line.operation {
                let strong_op = op.try_into_strong()?;
                let source = weak_line.source;
                let span_opt = Some(weak_line.span);
                let provenance = Provenance::new(span_opt, source);
                program.push_operation(strong_op, provenance);
            }
        }

        Ok(program)
    }
}

// ---------------------------------------------------------------------------
// Headers
// ---------------------------------------------------------------------------

/// Convert a Vec of weak headers into a strong `Header`, using the provided
/// carrier table (which may already contain carriers from previous scans).
fn convert_headers(
    weak_headers: Vec<WeakHeader>,
    mut carrier_table: knitout_ast::CarrierTable,
) -> Result<(Header, knitout_ast::CarrierTable), ParseDiagnostic> {
    // We need to find the Carriers header first to build the strong Header.
    // Walk the headers and collect them.
    let mut carriers_value: Option<CarriersHeaderValue> = None;
    let mut deferred: Vec<WeakHeader> = Vec::new();

    for h in weak_headers {
        match h.kind {
            WeakHeaderKind::Carriers(ch) => {
                let value = convert_carriers_value(ch, &mut carrier_table)?;
                carriers_value = Some(value);
            }
            other => deferred.push(WeakHeader {
                span: h.span,
                kind: other,
            }),
        }
    }

    let carriers = carriers_value.unwrap_or_else(|| CarriersHeaderValue::new(vec![]));
    let mut header = Header::new(carriers);

    for h in deferred {
        let span = h.span;
        match h.kind {
            WeakHeaderKind::Carriers(_) => unreachable!(),
            WeakHeaderKind::Gauge(gh) => {
                let v = gh
                    .value
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing gauge value"))?;
                header.set_gauge(v);
            }
            WeakHeaderKind::Width(wh) => {
                let v = wh
                    .value
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing width value"))?;
                header.set_width(v);
            }
            WeakHeaderKind::Position(ph) => {
                let v = ph.value.try_into_strong()?;
                header.set_position(v);
            }
            WeakHeaderKind::Machine(mh) => {
                let v = mh
                    .value
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing machine value"))?;
                header.set_machine(v);
            }
            WeakHeaderKind::Yarn(yh) => {
                let carrier = yh
                    .carrier
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing carrier in Yarn header"))?;
                let value = yh
                    .value
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing value in Yarn header"))?;
                header.add_yarn(carrier, value);
            }
            WeakHeaderKind::Extension(eh) => {
                let name = eh
                    .name
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing extension header name"))?;
                let value = eh
                    .value
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing extension header value"))?;
                header.add_unknown(name, value);
            }
            WeakHeaderKind::Unknown => {
                return Err(ParseDiagnostic::new(span, "unrecognized header"));
            }
        }
    }

    Ok((header, carrier_table))
}

fn convert_carriers_value(
    weak: WeakCarriersHeader,
    carrier_table: &mut knitout_ast::CarrierTable,
) -> Result<CarriersHeaderValue, ParseDiagnostic> {
    let items = match weak.value {
        None => return Ok(CarriersHeaderValue::new(vec![])),
        Some(v) => v,
    };

    let mut result = Vec::new();
    for item in items.items {
        match item {
            WeakCarrierOrRail::Carrier(c) => result.push(CarrierOrRail::Carrier(c)),
            WeakCarrierOrRail::Rail(wr) => {
                result.push(CarrierOrRail::Rail(convert_rail(wr, carrier_table)?));
            }
            WeakCarrierOrRail::Unknown(span) => {
                return Err(ParseDiagnostic::new(
                    span,
                    "invalid carrier or rail in Carriers header",
                ));
            }
        }
    }
    Ok(CarriersHeaderValue::new(result))
}

fn convert_rail(
    weak: WeakRail,
    _carrier_table: &mut knitout_ast::CarrierTable,
) -> Result<Rail, ParseDiagnostic> {
    let span = weak.span;
    let left = convert_rail_side(weak.left, span)?;
    let right = convert_rail_side(weak.right, span)?;
    Ok(Rail::new(left, right))
}

fn convert_rail_side(weak: WeakRailSide, _parent_span: Span) -> Result<RailSide, ParseDiagnostic> {
    Ok(RailSide::new(weak.carriers))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

impl WeakOperation {
    pub fn try_into_strong(self) -> Result<Operation, ParseDiagnostic> {
        match self {
            WeakOperation::Base(b) => Ok(Operation::BaseOperation(b.try_into_strong()?)),
            WeakOperation::Extension(e) => Ok(Operation::ExtensionOperation(e.try_into_strong()?)),
            WeakOperation::Unknown(span) => Err(ParseDiagnostic::new(span, "invalid operation")),
        }
    }
}

// ---------------------------------------------------------------------------
// Base operations
// ---------------------------------------------------------------------------

impl WeakBaseOperation {
    pub fn try_into_strong(self) -> Result<BaseOperation, ParseDiagnostic> {
        let span = self.span;
        match self.kind {
            WeakBaseOperationKind::In { carriers } => Ok(BaseOperation::In {
                carriers: carriers
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing carriers for `in`"))?,
            }),
            WeakBaseOperationKind::Inhook { carriers } => Ok(BaseOperation::Inhook {
                carriers: carriers
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing carriers for `inhook`"))?,
            }),
            WeakBaseOperationKind::Releasehook { carriers } => Ok(BaseOperation::Releasehook {
                carriers: carriers.ok_or_else(|| {
                    ParseDiagnostic::new(span, "missing carriers for `releasehook`")
                })?,
            }),
            WeakBaseOperationKind::Out { carriers } => Ok(BaseOperation::Out {
                carriers: carriers
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing carriers for `out`"))?,
            }),
            WeakBaseOperationKind::Outhook { carriers } => Ok(BaseOperation::Outhook {
                carriers: carriers
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing carriers for `outhook`"))?,
            }),
            WeakBaseOperationKind::Stitch {
                before_pull,
                after_pull,
            } => Ok(BaseOperation::Stitch {
                before_pull: before_pull.ok_or_else(|| {
                    ParseDiagnostic::new(span, "missing before_pull for `stitch`")
                })?,
                after_pull: after_pull
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing after_pull for `stitch`"))?,
            }),
            WeakBaseOperationKind::Rack { offset } => Ok(BaseOperation::Rack {
                offset: offset
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing offset for `rack`"))?,
            }),
            WeakBaseOperationKind::Knit {
                direction,
                needle,
                carriers,
            } => Ok(BaseOperation::Knit {
                direction: direction.try_into_strong()?,
                needle: needle
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing needle for `knit`"))?
                    .try_into_strong()?,
                carriers,
            }),
            WeakBaseOperationKind::Tuck {
                direction,
                needle,
                carriers,
            } => Ok(BaseOperation::Tuck {
                direction: direction.try_into_strong()?,
                needle: needle
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing needle for `tuck`"))?
                    .try_into_strong()?,
                carriers,
            }),
            WeakBaseOperationKind::Miss {
                direction,
                needle,
                carriers,
            } => Ok(BaseOperation::Miss {
                direction: direction.try_into_strong()?,
                needle: needle
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing needle for `miss`"))?
                    .try_into_strong()?,
                carriers,
            }),
            WeakBaseOperationKind::Split {
                direction,
                source,
                destination,
                carriers,
            } => Ok(BaseOperation::Split {
                direction: direction.try_into_strong()?,
                source: source
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing source needle for `split`"))?
                    .try_into_strong()?,
                destination: destination
                    .ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing destination needle for `split`")
                    })?
                    .try_into_strong()?,
                carriers,
            }),
            WeakBaseOperationKind::Pause => Ok(BaseOperation::Pause),
            WeakBaseOperationKind::Drop { needle } => Ok(BaseOperation::Drop {
                needle: needle
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing needle for `drop`"))?
                    .try_into_strong()?,
            }),
            WeakBaseOperationKind::Amiss { needle } => Ok(BaseOperation::Amiss {
                needle: needle
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing needle for `amiss`"))?
                    .try_into_strong()?,
            }),
            WeakBaseOperationKind::Xfer {
                source,
                destination,
            } => Ok(BaseOperation::Xfer {
                source: source
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing source needle for `xfer`"))?
                    .try_into_strong()?,
                destination: destination
                    .ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing destination needle for `xfer`")
                    })?
                    .try_into_strong()?,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Extension operations
// ---------------------------------------------------------------------------

impl WeakExtensionOperation {
    pub fn try_into_strong(self) -> Result<ExtensionOperation, ParseDiagnostic> {
        let span = self.span;
        match self.kind {
            WeakExtensionOperationKind::XStitchNumber { value } => {
                Ok(ExtensionOperation::XStitchNumber {
                    value: value.ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing value for `x-stitch-number`")
                    })?,
                })
            }
            WeakExtensionOperationKind::XSpeedNumber { value } => {
                Ok(ExtensionOperation::XSpeedNumber {
                    value: value.ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing value for `x-speed-number`")
                    })?,
                })
            }
            WeakExtensionOperationKind::XSubRollerNumber { value } => {
                Ok(ExtensionOperation::XSubRollerNumber {
                    value: value.ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing value for `x-sub-roller-number`")
                    })?,
                })
            }
            WeakExtensionOperationKind::XPresserMode { mode } => {
                Ok(ExtensionOperation::XPresserMode {
                    mode: mode.try_into_strong()?,
                })
            }
            WeakExtensionOperationKind::XXferStitchNumber { value } => {
                Ok(ExtensionOperation::XXferStitchNumber {
                    value: value.ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing value for `x-xfer-stitch-number`")
                    })?,
                })
            }
            WeakExtensionOperationKind::XEndPass => Ok(ExtensionOperation::XEndPass),
            WeakExtensionOperationKind::XVisColor { color, carriers } => {
                Ok(ExtensionOperation::XVisColor {
                    color: color.ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing color for `x-vis-color`")
                    })?,
                    carriers: carriers.ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing carriers for `x-vis-color`")
                    })?,
                })
            }
            WeakExtensionOperationKind::XLoopIn {
                direction,
                needle,
                carriers,
            } => Ok(ExtensionOperation::XLoopIn {
                direction: direction.try_into_strong()?,
                needle: needle
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing needle for `x-loop-in`"))?
                    .try_into_strong()?,
                carriers: carriers.ok_or_else(|| {
                    ParseDiagnostic::new(span, "missing carriers for `x-loop-in`")
                })?,
            }),
            WeakExtensionOperationKind::XYarnIn {
                direction,
                needle,
                carriers,
            } => Ok(ExtensionOperation::XYarnIn {
                direction: direction.try_into_strong()?,
                needle: needle
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing needle for `x-yarn-in`"))?
                    .try_into_strong()?,
                carriers: carriers.ok_or_else(|| {
                    ParseDiagnostic::new(span, "missing carriers for `x-yarn-in`")
                })?,
            }),
            WeakExtensionOperationKind::XYarnOut {
                direction,
                needle,
                carriers,
            } => Ok(ExtensionOperation::XYarnOut {
                direction: direction.try_into_strong()?,
                needle: needle
                    .ok_or_else(|| ParseDiagnostic::new(span, "missing needle for `x-yarn-out`"))?
                    .try_into_strong()?,
                carriers: carriers.ok_or_else(|| {
                    ParseDiagnostic::new(span, "missing carriers for `x-yarn-out`")
                })?,
            }),
            WeakExtensionOperationKind::XEnd => Ok(ExtensionOperation::XEnd),
            WeakExtensionOperationKind::XRollerAdvance { value } => {
                Ok(ExtensionOperation::XRollerAdvance {
                    value: value.ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing value for `x-roller-advance`")
                    })?,
                })
            }
            WeakExtensionOperationKind::XAddRollerAdvance { value } => {
                Ok(ExtensionOperation::XAddRollerAdvance {
                    value: value.ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing value for `x-add-roller-advance`")
                    })?,
                })
            }
            WeakExtensionOperationKind::XCarrierSpacing { value } => {
                Ok(ExtensionOperation::XCarrierSpacing {
                    value: value.ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing value for `x-carrier-spacing`")
                    })?,
                })
            }
            WeakExtensionOperationKind::XCarrierStoppingDistance { value } => {
                Ok(ExtensionOperation::XCarrierStoppingDistance {
                    value: value.ok_or_else(|| {
                        ParseDiagnostic::new(
                            span,
                            "missing value for `x-carrier-stopping-distance`",
                        )
                    })?,
                })
            }
            WeakExtensionOperationKind::XXferStyle { style } => {
                Ok(ExtensionOperation::XXferStyle {
                    style: style.try_into_strong()?,
                })
            }
            WeakExtensionOperationKind::UnknownExtension { name, arguments } => {
                Ok(ExtensionOperation::UnknownExtension {
                    name: name.ok_or_else(|| {
                        ParseDiagnostic::new(span, "missing name for unknown extension")
                    })?,
                    arguments,
                })
            }
        }
    }
}
