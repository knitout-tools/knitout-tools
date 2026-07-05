//! Converters from weak AST nodes to their strong knitout-ast equivalents.

use knitout_ast::{
    BaseOperation, Bed, CarrierOrRail, CarriersHeaderValue, Direction, ExtensionOperation, Headers,
    Needle, Operation, PositionHeaderValue, PresserMode, Program, Provenance, Rail, RailSide, Span,
    XferStyle,
};

use crate::diagnostics::ParseError;
use crate::types::*;

// ---------------------------------------------------------------------------
// Leaf converters
// ---------------------------------------------------------------------------

impl WeakBed {
    pub fn try_into_strong(self) -> Result<Bed, ParseError> {
        match self {
            WeakBed::Front => Ok(Bed::Front),
            WeakBed::Back => Ok(Bed::Back),
            WeakBed::Unknown(span) => Err(ParseError::UnrecognizedBed(span)),
        }
    }
}

impl WeakDirection {
    pub fn try_into_strong(self) -> Result<Direction, ParseError> {
        match self {
            WeakDirection::Left => Ok(Direction::Left),
            WeakDirection::Right => Ok(Direction::Right),
            WeakDirection::Unknown(span) => Err(ParseError::UnrecognizedDirection(span)),
        }
    }
}

impl WeakPresserMode {
    pub fn try_into_strong(self) -> Result<PresserMode, ParseError> {
        match self {
            WeakPresserMode::Off => Ok(PresserMode::Off),
            WeakPresserMode::Auto => Ok(PresserMode::Auto),
            WeakPresserMode::On => Ok(PresserMode::On),
            WeakPresserMode::Unknown(span) => Err(ParseError::UnrecognizedPresserMode(span)),
        }
    }
}

impl WeakXferStyle {
    pub fn try_into_strong(self) -> Result<XferStyle, ParseError> {
        match self {
            WeakXferStyle::FourPass => Ok(XferStyle::FourPass),
            WeakXferStyle::TwoPass => Ok(XferStyle::TwoPass),
            WeakXferStyle::Unknown(span) => Err(ParseError::UnrecognizedXferStyle(span)),
        }
    }
}

impl WeakPositionValue {
    pub fn try_into_strong(self) -> Result<PositionHeaderValue, ParseError> {
        match self {
            WeakPositionValue::Left => Ok(PositionHeaderValue::Left),
            WeakPositionValue::Center => Ok(PositionHeaderValue::Center),
            WeakPositionValue::Right => Ok(PositionHeaderValue::Right),
            WeakPositionValue::Keep => Ok(PositionHeaderValue::Keep),
            WeakPositionValue::Unknown(span) => Err(ParseError::UnrecognizedPositionValue(span)),
        }
    }
}

// ---------------------------------------------------------------------------
// Needle
// ---------------------------------------------------------------------------

impl WeakNeedle {
    pub fn try_into_strong(self) -> Result<Needle, ParseError> {
        let span = self.span;
        let bed = self.bed.try_into_strong()?;
        let is_slider = self.is_slider;
        let index = self.index.ok_or(ParseError::MissingField {
            span,
            field: "index",
        })?;
        Ok(Needle::new(bed, is_slider, index))
    }
}

// ---------------------------------------------------------------------------
// Program
// ---------------------------------------------------------------------------

impl WeakProgram {
    pub fn try_into_strong(self) -> Result<Program, ParseError> {
        let version = self
            .magic
            .version
            .ok_or(ParseError::MissingVersion(self.magic.span))?;

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

fn convert_headers(
    weak_headers: Vec<WeakHeader>,
    mut carrier_table: knitout_ast::CarrierTable,
) -> Result<(Headers, knitout_ast::CarrierTable), ParseError> {
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
    let mut header = Headers::new(carriers);

    for h in deferred {
        let span = h.span;
        match h.kind {
            WeakHeaderKind::Carriers(_) => unreachable!(),
            WeakHeaderKind::Gauge(gh) => {
                let v = gh.value.ok_or(ParseError::MissingField {
                    span,
                    field: "value",
                })?;
                header.set_gauge(v);
            }
            WeakHeaderKind::Width(wh) => {
                let v = wh.value.ok_or(ParseError::MissingField {
                    span,
                    field: "value",
                })?;
                header.set_width(v);
            }
            WeakHeaderKind::Position(ph) => {
                let v = ph.value.try_into_strong()?;
                header.set_position(v);
            }
            WeakHeaderKind::Machine(mh) => {
                let v = mh.value.ok_or(ParseError::MissingField {
                    span,
                    field: "value",
                })?;
                header.set_machine(v);
            }
            WeakHeaderKind::Yarn(yh) => {
                let carrier = yh.carrier.ok_or(ParseError::MissingField {
                    span,
                    field: "carrier",
                })?;
                let value = yh.value.ok_or(ParseError::MissingField {
                    span,
                    field: "value",
                })?;
                header.add_yarn(carrier, value);
            }
            WeakHeaderKind::Extension(eh) => {
                let name = eh.name.ok_or(ParseError::MissingField {
                    span,
                    field: "name",
                })?;
                let value = eh.value.ok_or(ParseError::MissingField {
                    span,
                    field: "value",
                })?;
                header.add_unknown(name, value);
            }
            WeakHeaderKind::Unknown => {
                return Err(ParseError::UnrecognizedHeader(span));
            }
        }
    }

    Ok((header, carrier_table))
}

fn convert_carriers_value(
    weak: WeakCarriersHeader,
    carrier_table: &mut knitout_ast::CarrierTable,
) -> Result<CarriersHeaderValue, ParseError> {
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
                return Err(ParseError::InvalidCarrierOrRail(span));
            }
        }
    }
    Ok(CarriersHeaderValue::new(result))
}

fn convert_rail(
    weak: WeakRail,
    _carrier_table: &mut knitout_ast::CarrierTable,
) -> Result<Rail, ParseError> {
    let span = weak.span;
    let left = convert_rail_side(weak.left, span)?;
    let right = convert_rail_side(weak.right, span)?;
    Ok(Rail::new(left, right))
}

fn convert_rail_side(weak: WeakRailSide, _parent_span: Span) -> Result<RailSide, ParseError> {
    Ok(RailSide::new(weak.carriers))
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

impl WeakOperation {
    pub fn try_into_strong(self) -> Result<Operation, ParseError> {
        match self {
            WeakOperation::Base(b) => Ok(Operation::BaseOperation(b.try_into_strong()?)),
            WeakOperation::Extension(e) => Ok(Operation::ExtensionOperation(e.try_into_strong()?)),
            WeakOperation::Unknown(span) => Err(ParseError::InvalidOperation(span)),
        }
    }
}

// ---------------------------------------------------------------------------
// Base operations
// ---------------------------------------------------------------------------

impl WeakBaseOperation {
    pub fn try_into_strong(self) -> Result<BaseOperation, ParseError> {
        let span = self.span;
        match self.kind {
            WeakBaseOperationKind::In { carriers } => Ok(BaseOperation::In {
                carriers: carriers.ok_or(ParseError::MissingField {
                    span,
                    field: "carriers",
                })?,
            }),
            WeakBaseOperationKind::Inhook { carriers } => Ok(BaseOperation::Inhook {
                carriers: carriers.ok_or(ParseError::MissingField {
                    span,
                    field: "carriers",
                })?,
            }),
            WeakBaseOperationKind::Releasehook { carriers } => Ok(BaseOperation::Releasehook {
                carriers: carriers.ok_or(ParseError::MissingField {
                    span,
                    field: "carriers",
                })?,
            }),
            WeakBaseOperationKind::Out { carriers } => Ok(BaseOperation::Out {
                carriers: carriers.ok_or(ParseError::MissingField {
                    span,
                    field: "carriers",
                })?,
            }),
            WeakBaseOperationKind::Outhook { carriers } => Ok(BaseOperation::Outhook {
                carriers: carriers.ok_or(ParseError::MissingField {
                    span,
                    field: "carriers",
                })?,
            }),
            WeakBaseOperationKind::Stitch {
                before_pull,
                after_pull,
            } => Ok(BaseOperation::Stitch {
                before_pull: before_pull.ok_or(ParseError::MissingField {
                    span,
                    field: "before_pull",
                })?,
                after_pull: after_pull.ok_or(ParseError::MissingField {
                    span,
                    field: "after_pull",
                })?,
            }),
            WeakBaseOperationKind::Rack { offset } => Ok(BaseOperation::Rack {
                offset: offset.ok_or(ParseError::MissingField {
                    span,
                    field: "offset",
                })?,
            }),
            WeakBaseOperationKind::Knit {
                direction,
                needle,
                carriers,
            } => Ok(BaseOperation::Knit {
                direction: direction.try_into_strong()?,
                needle: needle
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "needle",
                    })?
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
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "needle",
                    })?
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
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "needle",
                    })?
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
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "source",
                    })?
                    .try_into_strong()?,
                destination: destination
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "destination",
                    })?
                    .try_into_strong()?,
                carriers,
            }),
            WeakBaseOperationKind::Pause => Ok(BaseOperation::Pause),
            WeakBaseOperationKind::Drop { needle } => Ok(BaseOperation::Drop {
                needle: needle
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "needle",
                    })?
                    .try_into_strong()?,
            }),
            WeakBaseOperationKind::Amiss { needle } => Ok(BaseOperation::Amiss {
                needle: needle
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "needle",
                    })?
                    .try_into_strong()?,
            }),
            WeakBaseOperationKind::Xfer {
                source,
                destination,
            } => Ok(BaseOperation::Xfer {
                source: source
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "source",
                    })?
                    .try_into_strong()?,
                destination: destination
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "destination",
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
    pub fn try_into_strong(self) -> Result<ExtensionOperation, ParseError> {
        let span = self.span;
        match self.kind {
            WeakExtensionOperationKind::XStitchNumber { value } => {
                Ok(ExtensionOperation::XStitchNumber {
                    value: value.ok_or(ParseError::MissingField {
                        span,
                        field: "value",
                    })?,
                })
            }
            WeakExtensionOperationKind::XSpeedNumber { value } => {
                Ok(ExtensionOperation::XSpeedNumber {
                    value: value.ok_or(ParseError::MissingField {
                        span,
                        field: "value",
                    })?,
                })
            }
            WeakExtensionOperationKind::XSubRollerNumber { value } => {
                Ok(ExtensionOperation::XSubRollerNumber {
                    value: value.ok_or(ParseError::MissingField {
                        span,
                        field: "value",
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
                    value: value.ok_or(ParseError::MissingField {
                        span,
                        field: "value",
                    })?,
                })
            }
            WeakExtensionOperationKind::XEndPass => Ok(ExtensionOperation::XEndPass),
            WeakExtensionOperationKind::XVisColor { color, carriers } => {
                Ok(ExtensionOperation::XVisColor {
                    color: color.ok_or(ParseError::MissingField {
                        span,
                        field: "color",
                    })?,
                    carriers: carriers.ok_or(ParseError::MissingField {
                        span,
                        field: "carriers",
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
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "needle",
                    })?
                    .try_into_strong()?,
                carriers: carriers.ok_or(ParseError::MissingField {
                    span,
                    field: "carriers",
                })?,
            }),
            WeakExtensionOperationKind::XYarnIn {
                direction,
                needle,
                carriers,
            } => Ok(ExtensionOperation::XYarnIn {
                direction: direction.try_into_strong()?,
                needle: needle
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "needle",
                    })?
                    .try_into_strong()?,
                carriers: carriers.ok_or(ParseError::MissingField {
                    span,
                    field: "carriers",
                })?,
            }),
            WeakExtensionOperationKind::XYarnOut {
                direction,
                needle,
                carriers,
            } => Ok(ExtensionOperation::XYarnOut {
                direction: direction.try_into_strong()?,
                needle: needle
                    .ok_or(ParseError::MissingField {
                        span,
                        field: "needle",
                    })?
                    .try_into_strong()?,
                carriers: carriers.ok_or(ParseError::MissingField {
                    span,
                    field: "carriers",
                })?,
            }),
            WeakExtensionOperationKind::XEnd => Ok(ExtensionOperation::XEnd),
            WeakExtensionOperationKind::XRollerAdvance { value } => {
                Ok(ExtensionOperation::XRollerAdvance {
                    value: value.ok_or(ParseError::MissingField {
                        span,
                        field: "value",
                    })?,
                })
            }
            WeakExtensionOperationKind::XAddRollerAdvance { value } => {
                Ok(ExtensionOperation::XAddRollerAdvance {
                    value: value.ok_or(ParseError::MissingField {
                        span,
                        field: "value",
                    })?,
                })
            }
            WeakExtensionOperationKind::XCarrierSpacing { value } => {
                Ok(ExtensionOperation::XCarrierSpacing {
                    value: value.ok_or(ParseError::MissingField {
                        span,
                        field: "value",
                    })?,
                })
            }
            WeakExtensionOperationKind::XCarrierStoppingDistance { value } => {
                Ok(ExtensionOperation::XCarrierStoppingDistance {
                    value: value.ok_or(ParseError::MissingField {
                        span,
                        field: "value",
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
                    name: name.ok_or(ParseError::MissingField {
                        span,
                        field: "name",
                    })?,
                    arguments,
                })
            }
        }
    }
}
