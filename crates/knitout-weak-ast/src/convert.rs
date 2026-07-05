//! Phase-transformation functor for mapping between `Program<A>` and `Program<B>`.
//!
//! The core abstraction is `PhaseTransform<A, B>`: a zero-sized type that provides one
//! conversion function per associated type of the `Phase` trait.  Given such a type, the
//! free function `transform_program` walks the entire `Program<A>` and produces an
//! `Option<Program<B>>`, returning `None` if it encounters any `A::Other` node (which
//! can never be mapped to `B`).
//!
//! `ParsedToValidated` is the canonical implementation that drops all `Parsed` span
//! annotations and produces a `Program<Validated>`, which is what `parsed_to_valid` uses.

use either::Either;
use knitout_ast::{
    Annotated, BaseOperation, Bed, CarrierOrRail, CarriersHeaderValue, Direction,
    ExtensionOperation, Headers, IsSlider, Needle, Operation, Phase, PositionHeaderValue,
    PresserMode, Program, Rail, RailSide, Validated, XferStyle,
};
use knitout_ast::{MyBigDecimal, MyString, MyU32};

use crate::Parsed;

/// A mapping between two `Phase` implementations.
///
/// Each method converts one associated type from phase `A` to the corresponding type in phase `B`.
/// The `Other` case is *not* mapped here — whenever an `A::Other` is encountered, the
/// enclosing `transform_*` function short-circuits to `None`.
///
/// Implement this trait on a zero-sized marker type, then pass it as the type parameter `T` to
/// `transform_program`.
pub trait PhaseTransform<A: Phase, B: Phase> {
    fn transform_magic_version(v: A::MagicVersion) -> B::MagicVersion;
    fn transform_operation(v: A::Operation) -> B::Operation;
    fn transform_headers(v: A::Headers) -> B::Headers;
    fn transform_header(v: A::Header) -> B::Header;
    fn transform_unknown_header(v: A::UnknownHeader) -> B::UnknownHeader;
    fn transform_position_header_value(v: A::PositionHeaderValue) -> B::PositionHeaderValue;
    fn transform_carriers_header_value(v: A::CarriersHeaderValue) -> B::CarriersHeaderValue;
    fn transform_carrier_or_rail(v: A::CarrierOrRail) -> B::CarrierOrRail;
    fn transform_rail(v: A::Rail) -> B::Rail;
    fn transform_rail_side(v: A::RailSide) -> B::RailSide;
    fn transform_carrier(v: A::Carrier) -> B::Carrier;
    fn transform_direction(v: A::Direction) -> B::Direction;
    fn transform_bed(v: A::Bed) -> B::Bed;
    fn transform_is_slider(v: A::IsSlider) -> B::IsSlider;
    fn transform_index(v: A::Index) -> B::Index;
    fn transform_needle(v: A::Needle) -> B::Needle;
    fn transform_my_big_decimal(v: A::MyBigDecimal) -> B::MyBigDecimal;
    fn transform_my_u32(v: A::MyU32) -> B::MyU32;
    fn transform_my_string(v: A::MyString) -> B::MyString;
    fn transform_presser_mode(v: A::PresserMode) -> B::PresserMode;
    fn transform_xfer_style(v: A::XferStyle) -> B::XferStyle;
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn t_carrier<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    c: &knitout_ast::Carrier<A>,
) -> knitout_ast::Carrier<B> {
    c.map_annotation(T::transform_carrier(c.annotation().clone()))
}

fn t_direction<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    d: Direction<A>,
) -> Option<Direction<B>> {
    Some(match d {
        Direction::Left(ann) => Direction::Left(T::transform_direction(ann)),
        Direction::Right(ann) => Direction::Right(T::transform_direction(ann)),
        Direction::Other(_) => return None,
    })
}

fn t_bed<A: Phase, B: Phase, T: PhaseTransform<A, B>>(b: Bed<A>) -> Option<Bed<B>> {
    Some(match b {
        Bed::Front(ann) => Bed::Front(T::transform_bed(ann)),
        Bed::Back(ann) => Bed::Back(T::transform_bed(ann)),
        Bed::Other(_) => return None,
    })
}

fn t_is_slider<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    s: IsSlider<A>,
) -> Option<IsSlider<B>> {
    Some(match s {
        IsSlider::Slider(ann) => IsSlider::Slider(T::transform_is_slider(ann)),
        IsSlider::NotSlider(ann) => IsSlider::NotSlider(T::transform_is_slider(ann)),
        IsSlider::Other(_) => return None,
    })
}

fn t_needle<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    n: Needle<A>,
) -> Option<Needle<B>> {
    let (bed, is_slider, index, annotation) = n.into_parts();
    let bed = t_bed::<A, B, T>(bed)?;
    let is_slider = t_is_slider::<A, B, T>(is_slider)?;
    let index = match index {
        Either::Left(ann) => Either::Left(Annotated::new(ann.value, T::transform_index(ann.annotation))),
        Either::Right(_) => return None,
    };
    let annotation = T::transform_needle(annotation);
    Some(Needle::new(bed, is_slider, index, annotation))
}

fn t_needle_either<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    n: Either<Needle<A>, A::Other>,
) -> Option<Either<Needle<B>, B::Other>> {
    match n {
        Either::Left(needle) => Some(Either::Left(t_needle::<A, B, T>(needle)?)),
        Either::Right(_) => None,
    }
}

fn t_my_big_decimal<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    v: MyBigDecimal<A>,
) -> Option<MyBigDecimal<B>> {
    Some(MyBigDecimal::new(match v.value().clone() {
        Either::Left(ann) => Either::Left(Annotated::new(ann.value, T::transform_my_big_decimal(ann.annotation))),
        Either::Right(_) => return None,
    }))
}

fn t_my_u32<A: Phase, B: Phase, T: PhaseTransform<A, B>>(v: MyU32<A>) -> Option<MyU32<B>> {
    Some(MyU32::new(match v.value().clone() {
        Either::Left(ann) => Either::Left(Annotated::new(ann.value, T::transform_my_u32(ann.annotation))),
        Either::Right(_) => return None,
    }))
}

fn t_my_string<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    v: MyString<A>,
) -> Option<MyString<B>> {
    Some(MyString::new(match v.value().clone() {
        Either::Left(ann) => {
            Either::Left(Annotated::new(ann.value.clone(), T::transform_my_string(ann.annotation)))
        }
        Either::Right(_) => return None,
    }))
}

fn t_presser_mode<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    m: PresserMode<A>,
) -> Option<PresserMode<B>> {
    Some(match m {
        PresserMode::Off(ann) => PresserMode::Off(T::transform_presser_mode(ann)),
        PresserMode::Auto(ann) => PresserMode::Auto(T::transform_presser_mode(ann)),
        PresserMode::On(ann) => PresserMode::On(T::transform_presser_mode(ann)),
        PresserMode::Other(_) => return None,
    })
}

fn t_xfer_style<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    s: XferStyle<A>,
) -> Option<XferStyle<B>> {
    Some(match s {
        XferStyle::FourPass(ann) => XferStyle::FourPass(T::transform_xfer_style(ann)),
        XferStyle::TwoPass(ann) => XferStyle::TwoPass(T::transform_xfer_style(ann)),
        XferStyle::Other(_) => return None,
    })
}

fn t_position_header_value<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    v: PositionHeaderValue<A>,
) -> Option<PositionHeaderValue<B>> {
    Some(match v {
        PositionHeaderValue::Left(ann) => {
            PositionHeaderValue::Left(T::transform_position_header_value(ann))
        }
        PositionHeaderValue::Center(ann) => {
            PositionHeaderValue::Center(T::transform_position_header_value(ann))
        }
        PositionHeaderValue::Right(ann) => {
            PositionHeaderValue::Right(T::transform_position_header_value(ann))
        }
        PositionHeaderValue::Keep(ann) => {
            PositionHeaderValue::Keep(T::transform_position_header_value(ann))
        }
        PositionHeaderValue::Other(_) => return None,
    })
}

fn t_rail_side<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    rs: RailSide<A>,
) -> Option<RailSide<B>> {
    let (carriers, annotation) = rs.into_parts();
    let carriers_b: Option<Vec<_>> = carriers.into_iter().map(|c| {
        Some(t_carrier::<A, B, T>(&c))
    }).collect();
    Some(RailSide::new(carriers_b?, T::transform_rail_side(annotation)))
}

fn t_rail<A: Phase, B: Phase, T: PhaseTransform<A, B>>(r: Rail<A>) -> Option<Rail<B>> {
    let (left, right, annotation) = r.into_parts();
    let left_b = match left {
        Either::Left(rs) => Either::Left(t_rail_side::<A, B, T>(rs)?),
        Either::Right(_) => return None,
    };
    let right_b = match right {
        Either::Left(rs) => Either::Left(t_rail_side::<A, B, T>(rs)?),
        Either::Right(_) => return None,
    };
    Some(Rail::new(left_b, right_b, T::transform_rail(annotation)))
}

fn t_carrier_or_rail<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    cor: CarrierOrRail<A>,
) -> Option<CarrierOrRail<B>> {
    Some(match cor {
        CarrierOrRail::Carrier(c) => CarrierOrRail::Carrier(t_carrier::<A, B, T>(&c)),
        CarrierOrRail::Rail(r) => CarrierOrRail::Rail(t_rail::<A, B, T>(r)?),
        CarrierOrRail::Other(_) => return None,
    })
}

fn t_carriers_header_value<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    v: CarriersHeaderValue<A>,
) -> Option<CarriersHeaderValue<B>> {
    let (inner, annotation) = v.into_inner();
    let items_b = match inner {
        Either::Left(items) => {
            let v: Option<Vec<_>> = items
                .into_iter()
                .map(|cor| t_carrier_or_rail::<A, B, T>(cor))
                .collect();
            Either::Left(v?)
        }
        Either::Right(_) => return None,
    };
    Some(CarriersHeaderValue::new(
        items_b,
        T::transform_carriers_header_value(annotation),
    ))
}

fn t_carriers<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    carriers: Vec<knitout_ast::Carrier<A>>,
) -> Vec<knitout_ast::Carrier<B>> {
    carriers.iter().map(|c| t_carrier::<A, B, T>(c)).collect()
}

fn t_base_op<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    op: BaseOperation<A>,
) -> Option<BaseOperation<B>> {
    Some(match op {
        BaseOperation::In { carriers } => BaseOperation::In {
            carriers: t_carriers::<A, B, T>(carriers),
        },
        BaseOperation::Inhook { carriers } => BaseOperation::Inhook {
            carriers: t_carriers::<A, B, T>(carriers),
        },
        BaseOperation::Releasehook { carriers } => BaseOperation::Releasehook {
            carriers: t_carriers::<A, B, T>(carriers),
        },
        BaseOperation::Out { carriers } => BaseOperation::Out {
            carriers: t_carriers::<A, B, T>(carriers),
        },
        BaseOperation::Outhook { carriers } => BaseOperation::Outhook {
            carriers: t_carriers::<A, B, T>(carriers),
        },
        BaseOperation::Stitch {
            before_pull,
            after_pull,
        } => BaseOperation::Stitch {
            before_pull: t_my_big_decimal::<A, B, T>(before_pull)?,
            after_pull: t_my_big_decimal::<A, B, T>(after_pull)?,
        },
        BaseOperation::Rack { offset } => BaseOperation::Rack {
            offset: t_my_big_decimal::<A, B, T>(offset)?,
        },
        BaseOperation::Knit {
            direction,
            needle,
            carriers,
        } => BaseOperation::Knit {
            direction: t_direction::<A, B, T>(direction)?,
            needle: t_needle_either::<A, B, T>(needle)?,
            carriers: t_carriers::<A, B, T>(carriers),
        },
        BaseOperation::Tuck {
            direction,
            needle,
            carriers,
        } => BaseOperation::Tuck {
            direction: t_direction::<A, B, T>(direction)?,
            needle: t_needle_either::<A, B, T>(needle)?,
            carriers: t_carriers::<A, B, T>(carriers),
        },
        BaseOperation::Miss {
            direction,
            needle,
            carriers,
        } => BaseOperation::Miss {
            direction: t_direction::<A, B, T>(direction)?,
            needle: t_needle_either::<A, B, T>(needle)?,
            carriers: t_carriers::<A, B, T>(carriers),
        },
        BaseOperation::Split {
            direction,
            source,
            destination,
            carriers,
        } => BaseOperation::Split {
            direction: t_direction::<A, B, T>(direction)?,
            source: t_needle_either::<A, B, T>(source)?,
            destination: t_needle_either::<A, B, T>(destination)?,
            carriers: t_carriers::<A, B, T>(carriers),
        },
        BaseOperation::Pause => BaseOperation::Pause,
        BaseOperation::Drop { needle } => BaseOperation::Drop {
            needle: t_needle_either::<A, B, T>(needle)?,
        },
        BaseOperation::Amiss { needle } => BaseOperation::Amiss {
            needle: t_needle_either::<A, B, T>(needle)?,
        },
        BaseOperation::Xfer { source, destination } => BaseOperation::Xfer {
            source: t_needle_either::<A, B, T>(source)?,
            destination: t_needle_either::<A, B, T>(destination)?,
        },
    })
}

fn t_extension_op<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    op: ExtensionOperation<A>,
) -> Option<ExtensionOperation<B>> {
    Some(match op {
        ExtensionOperation::XStitchNumber { value } => ExtensionOperation::XStitchNumber {
            value: t_my_u32::<A, B, T>(value)?,
        },
        ExtensionOperation::XSpeedNumber { value } => ExtensionOperation::XSpeedNumber {
            value: t_my_u32::<A, B, T>(value)?,
        },
        ExtensionOperation::XSubRollerNumber { value } => ExtensionOperation::XSubRollerNumber {
            value: t_my_u32::<A, B, T>(value)?,
        },
        ExtensionOperation::XPresserMode { mode } => ExtensionOperation::XPresserMode {
            mode: t_presser_mode::<A, B, T>(mode)?,
        },
        ExtensionOperation::XXferStitchNumber { value } => ExtensionOperation::XXferStitchNumber {
            value: t_my_u32::<A, B, T>(value)?,
        },
        ExtensionOperation::XEndPass => ExtensionOperation::XEndPass,
        ExtensionOperation::XVisColor { color, carriers } => ExtensionOperation::XVisColor {
            color: t_my_string::<A, B, T>(color)?,
            carriers: t_carriers::<A, B, T>(carriers),
        },
        ExtensionOperation::XLoopIn {
            direction,
            needle,
            carriers,
        } => ExtensionOperation::XLoopIn {
            direction: t_direction::<A, B, T>(direction)?,
            needle: t_needle_either::<A, B, T>(needle)?,
            carriers: t_carriers::<A, B, T>(carriers),
        },
        ExtensionOperation::XYarnIn {
            direction,
            needle,
            carriers,
        } => ExtensionOperation::XYarnIn {
            direction: t_direction::<A, B, T>(direction)?,
            needle: t_needle_either::<A, B, T>(needle)?,
            carriers: t_carriers::<A, B, T>(carriers),
        },
        ExtensionOperation::XYarnOut {
            direction,
            needle,
            carriers,
        } => ExtensionOperation::XYarnOut {
            direction: t_direction::<A, B, T>(direction)?,
            needle: t_needle_either::<A, B, T>(needle)?,
            carriers: t_carriers::<A, B, T>(carriers),
        },
        ExtensionOperation::XEnd => ExtensionOperation::XEnd,
        ExtensionOperation::XRollerAdvance { value } => ExtensionOperation::XRollerAdvance {
            value: t_my_big_decimal::<A, B, T>(value)?,
        },
        ExtensionOperation::XAddRollerAdvance { value } => ExtensionOperation::XAddRollerAdvance {
            value: t_my_big_decimal::<A, B, T>(value)?,
        },
        ExtensionOperation::XCarrierSpacing { value } => ExtensionOperation::XCarrierSpacing {
            value: t_my_u32::<A, B, T>(value)?,
        },
        ExtensionOperation::XCarrierStoppingDistance { value } => {
            ExtensionOperation::XCarrierStoppingDistance {
                value: t_my_u32::<A, B, T>(value)?,
            }
        }
        ExtensionOperation::XXferStyle { style } => ExtensionOperation::XXferStyle {
            style: t_xfer_style::<A, B, T>(style)?,
        },
        ExtensionOperation::UnknownExtension { name, arguments } => {
            ExtensionOperation::UnknownExtension {
                name: t_my_string::<A, B, T>(name)?,
                arguments: arguments
                    .into_iter()
                    .map(|a| t_my_string::<A, B, T>(a))
                    .collect::<Option<Vec<_>>>()?,
            }
        }
    })
}

fn t_operation<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    op: Operation<A>,
) -> Option<Operation<B>> {
    Some(match op {
        Operation::BaseOperation(base) => Operation::BaseOperation(t_base_op::<A, B, T>(base)?),
        Operation::ExtensionOperation(ext) => {
            Operation::ExtensionOperation(t_extension_op::<A, B, T>(ext)?)
        }
        Operation::Other(_) => return None,
    })
}

fn t_headers<A: Phase, B: Phase, T: PhaseTransform<A, B>>(
    headers: Headers<A>,
) -> Option<Headers<B>> {
    let (carriers, yarn, machine, gauge, width, position, unknown) = headers.into_components();

    let carriers_b = match carriers {
        Either::Left(ann) => Either::Left(Annotated::new(
            t_carriers_header_value::<A, B, T>(ann.value)?,
            T::transform_header(ann.annotation),
        )),
        Either::Right(_) => return None,
    };

    let mut headers_b = Headers::new(carriers_b);

    for (carrier, color_ann) in yarn {
        let carrier_b = t_carrier::<A, B, T>(&carrier);
        let color_b = t_my_string::<A, B, T>(color_ann.value)?;
        headers_b.add_yarn(carrier_b, color_b, T::transform_header(color_ann.annotation));
    }

    if let Some(machine_ann) = machine {
        let machine_b = t_my_string::<A, B, T>(machine_ann.value)?;
        headers_b.set_machine(Annotated::new(machine_b, T::transform_header(machine_ann.annotation)));
    }
    if let Some(gauge_ann) = gauge {
        let gauge_b = t_my_u32::<A, B, T>(gauge_ann.value)?;
        headers_b.set_gauge(Annotated::new(gauge_b, T::transform_header(gauge_ann.annotation)));
    }
    if let Some(width_ann) = width {
        let width_b = t_my_u32::<A, B, T>(width_ann.value)?;
        headers_b.set_width(Annotated::new(width_b, T::transform_header(width_ann.annotation)));
    }
    if let Some(pos_ann) = position {
        let pos_b = t_position_header_value::<A, B, T>(pos_ann.value)?;
        headers_b
            .set_position(Annotated::new(pos_b, T::transform_header(pos_ann.annotation)));
    }
    for (key, value_ann) in unknown {
        headers_b.add_unknown(
            key,
            value_ann.value,
            T::transform_unknown_header(value_ann.annotation),
        );
    }

    Some(headers_b)
}

/// Transform a `Program<A>` into an `Option<Program<B>>`, returning `None` if any `A::Other`
/// node is encountered. This lets any pair of phases be connected by a `PhaseTransform`
/// implementation without repeating the structural traversal.
pub fn transform_program<A, B, T>(prog: Program<A>) -> Option<Program<B>>
where
    A: Phase,
    B: Phase,
    T: PhaseTransform<A, B>,
{
    let (version, headers, operations, carrier_table) = prog.into_components();

    let version_b = match version {
        Either::Left(ann) => {
            Either::Left(Annotated::new(ann.value, T::transform_magic_version(ann.annotation)))
        }
        Either::Right(_) => return None,
    };

    let headers_b = t_headers::<A, B, T>(headers)?;

    let mut prog_b = Program::new(version_b, headers_b, carrier_table);

    for op_ann in operations {
        let op_b = t_operation::<A, B, T>(op_ann.value)?;
        prog_b.push_operation(Annotated::new(op_b, T::transform_operation(op_ann.annotation)));
    }

    Some(prog_b)
}

// ─── Parsed → Validated ──────────────────────────────────────────────────────

/// The canonical `PhaseTransform` from `Parsed` to `Validated`.  All span annotations are
/// dropped; all `Other(Span)` nodes cause the transformation to return `None`.
pub struct ParsedToValidated;

impl PhaseTransform<Parsed, Validated> for ParsedToValidated {
    fn transform_magic_version(_: (knitout_ast::Span, knitout_ast::Span)) -> () {}
    fn transform_operation(_: (knitout_ast::Span, crate::CommentInfo)) -> knitout_ast::Provenance {
        knitout_ast::Provenance::default()
    }
    fn transform_headers(_: ()) -> () {}
    fn transform_header(_: knitout_ast::Span) -> () {}
    fn transform_unknown_header(_: knitout_ast::Span) -> () {}
    fn transform_position_header_value(_: knitout_ast::Span) -> () {}
    fn transform_carriers_header_value(_: knitout_ast::Span) -> () {}
    fn transform_carrier_or_rail(_: ()) -> () {}
    fn transform_rail(_: knitout_ast::Span) -> () {}
    fn transform_rail_side(_: knitout_ast::Span) -> () {}
    fn transform_carrier(_: knitout_ast::Span) -> () {}
    fn transform_direction(_: knitout_ast::Span) -> () {}
    fn transform_bed(_: knitout_ast::Span) -> () {}
    fn transform_is_slider(_: knitout_ast::Span) -> () {}
    fn transform_index(_: knitout_ast::Span) -> () {}
    fn transform_needle(_: knitout_ast::Span) -> () {}
    fn transform_my_big_decimal(_: knitout_ast::Span) -> () {}
    fn transform_my_u32(_: knitout_ast::Span) -> () {}
    fn transform_my_string(_: knitout_ast::Span) -> () {}
    fn transform_presser_mode(_: knitout_ast::Span) -> () {}
    fn transform_xfer_style(_: knitout_ast::Span) -> () {}
}

/// Convert a `Program<Parsed>` to `Option<Program<Validated>>`.
///
/// Returns `None` if any `Other(Span)` node is present in the parsed tree (indicating that
/// some portion of the source could not be understood).
pub fn parsed_to_valid(prog: Program<Parsed>) -> Option<Program<Validated>> {
    transform_program::<Parsed, Validated, ParsedToValidated>(prog)
}
