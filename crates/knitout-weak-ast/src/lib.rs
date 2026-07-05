mod convert;
mod diagnostics;
mod lower;

pub use convert::{parsed_to_valid, transform_program, ParsedToValidated, PhaseTransform};
pub use diagnostics::{ParseError, ParseWarning};
pub use lower::cst_to_program;

pub use knitout_ast::{Phase, Provenance, Span};

// ─── CommentInfo ─────────────────────────────────────────────────────────────

/// Information about a trailing comment on an operation line.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum CommentInfo {
    /// No trailing comment.
    NoComment,
    /// An ordinary `; text` comment. The span covers the full comment including `;`.
    Comment(Span),
    /// A `;!source: …` tag. The span covers the whole tag.
    SourceTag(Span),
    /// A `;!foo` tag that is not a recognised knitout tag. The span covers the whole tag.
    UnknownTag(Span),
}

// ─── Parsed phase ─────────────────────────────────────────────────────────────

/// The `Parsed` phase: every AST node is annotated with a `Span` pointing to the
/// corresponding bytes in the source buffer. Unrecognised constructs are represented
/// as `Other(Span)` rather than aborting parsing.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Parsed {}

impl Phase for Parsed {
    /// Unrecognised constructs.
    type Other = Span;

    /// `(prefix_span, version_span)`: e.g. `";!knitout-"` and `"2"`.
    type MagicVersion = (Span, Span);

    /// Not used at the `Headers` level for `Parsed`; headers are stored field-by-field.
    type Headers = ();

    /// `(op_span, CommentInfo)`: the span of the whole operation (excluding any comment)
    /// plus information about any trailing comment.
    type Operation = (Span, CommentInfo);

    /// Span of the `"Name:"` portion of a header line.
    type Header = Span;

    /// Span of the `"Name:"` portion of an unknown header.
    type UnknownHeader = Span;

    /// Span of the position-value token (e.g. `"Left"`).
    type PositionHeaderValue = Span;

    /// Span of the whole carriers value (e.g. `"1 2,3;4,5"`).
    type CarriersHeaderValue = Span;

    /// Not used individually; carriers/rails share a common annotation at the item level.
    type CarrierOrRail = ();

    /// Span of a `"left;right"` rail token.
    type Rail = Span;

    /// Span of one rail side (`"2,3"`).
    type RailSide = Span;

    /// Span of a single carrier name token.
    type Carrier = Span;

    /// Span of the direction token (`"+"` or `"-"`).
    type Direction = Span;

    /// Span of the bed character (`"f"` or `"b"`).
    type Bed = Span;

    /// Span of the optional slider character (`"s"`), or a zero-length span if absent.
    type IsSlider = Span;

    /// Span of the integer index portion of a needle.
    type Index = Span;

    /// Span of the entire needle token (e.g. `"fs-3"`).
    type Needle = Span;

    /// Span of the decimal-number token.
    type MyBigDecimal = Span;

    /// Span of the unsigned-integer token.
    type MyU32 = Span;

    /// Span of the string token.
    type MyString = Span;

    /// Span of the presser-mode token.
    type PresserMode = Span;

    /// Span of the transfer-style token.
    type XferStyle = Span;
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use knitout_ast::{
        Annotated, BaseOperation, CarrierOrRail, Direction, ExtensionOperation, Operation,
        PositionHeaderValue, PresserMode, Program, XferStyle,
    };
    use either::Either;

    use super::*;

    // ── parser helper ─────────────────────────────────────────────────────────

    fn parse(source: &str) -> (Program<Parsed>, Vec<ParseWarning>, Vec<ParseError>) {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_knitout::LANGUAGE.into())
            .expect("failed to load grammar");
        let tree = parser.parse(source, None).expect("parse returned None");
        cst_to_program(source.as_bytes(), &tree)
    }

    fn parse_ok(source: &str) -> Program<Parsed> {
        let (prog, warnings, errors) = parse(source);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        prog
    }

    // ── span helpers ──────────────────────────────────────────────────────────

    fn span_within(span: Span, source: &str) -> bool {
        (span.end() as usize) <= source.len()
    }

    // ─── magic / version tests ────────────────────────────────────────────────

    #[test]
    fn valid_minimal_program_no_diagnostics() {
        let (_, warnings, errors) = parse(";!knitout-2\n;;Carriers: A\n");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn magic_version_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n");
        let (version, _headers, _ops, _ct) = prog.into_components();
        match version {
            Either::Left(ann) => assert_eq!(ann.value, 2),
            Either::Right(_) => panic!("expected version, got Other"),
        }
    }

    #[test]
    fn magic_version_18() {
        let prog = parse_ok(";!knitout-18\n;;Carriers: A\n");
        let (version, _, _, _) = prog.into_components();
        assert_eq!(version.left().unwrap().value, 18);
    }

    #[test]
    fn missing_magic_emits_error() {
        let (_, _, errors) = parse(";;Carriers: A\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MagicMissing)),
            "expected MagicMissing, got {errors:?}"
        );
    }

    #[test]
    fn bad_magic_version_emits_error() {
        let (_, _, errors) = parse(";!knitout-xyz\n;;Carriers: A\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::BadMagicVersion(_))),
            "expected BadMagicVersion, got {errors:?}"
        );
    }

    #[test]
    fn magic_not_first_emits_warning() {
        let (_, warnings, _) = parse(";;Carriers: A\n;!knitout-2\n");
        assert!(
            warnings.iter().any(|w| matches!(w, ParseWarning::MagicNotFirst(_))),
            "expected MagicNotFirst, got {warnings:?}"
        );
    }

    #[test]
    fn magic_version_span_within_source() {
        let source = ";!knitout-2\n;;Carriers: A\n";
        let (prog, _, _) = parse(source);
        let (version, _, _, _) = prog.into_components();
        if let Either::Left(ann) = version {
            let (prefix_span, ver_span) = ann.annotation;
            assert!(span_within(prefix_span, source));
            assert!(span_within(ver_span, source));
        }
    }

    // ─── carriers header tests ────────────────────────────────────────────────

    #[test]
    fn missing_carriers_header_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MissingCarriersHeader)),
            "expected MissingCarriersHeader, got {errors:?}"
        );
    }

    fn carriers_items(prog: Program<Parsed>) -> Vec<CarrierOrRail<Parsed>> {
        let (_, headers, _, _) = prog.into_components();
        let (carriers_field, _, _, _, _, _, _) = headers.into_components();
        let ann = carriers_field.left().expect("expected carriers");
        let (items_either, _) = ann.value.into_inner();
        items_either.left().expect("expected items list")
    }

    #[test]
    fn single_carrier_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n");
        assert_eq!(carriers_items(prog).len(), 1);
    }

    #[test]
    fn multiple_carriers_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: 1 2 3\n");
        assert_eq!(carriers_items(prog).len(), 3);
    }

    #[test]
    fn rail_carrier_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A;B\n");
        let items = carriers_items(prog);
        assert_eq!(items.len(), 1);
        match items.into_iter().next().unwrap() {
            CarrierOrRail::Rail(rail) => {
                let (left, right, _) = rail.into_parts();
                let (left_carriers, _) = left.left().unwrap().into_parts();
                let (right_carriers, _) = right.left().unwrap().into_parts();
                assert_eq!(left_carriers.len(), 1);
                assert_eq!(right_carriers.len(), 1);
            }
            other => panic!("expected Rail, got {other:?}"),
        }
    }

    #[test]
    fn multi_carrier_rail_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A,C;B,D\n");
        let items = carriers_items(prog);
        assert_eq!(items.len(), 1);
        match items.into_iter().next().unwrap() {
            CarrierOrRail::Rail(rail) => {
                let (left, right, _) = rail.into_parts();
                let (lc, _) = left.left().unwrap().into_parts();
                let (rc, _) = right.left().unwrap().into_parts();
                assert_eq!(lc.len(), 2);
                assert_eq!(rc.len(), 2);
            }
            other => panic!("expected Rail, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_carriers_header_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\n;;Carriers: B\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::ConflictingHeader { .. })),
            "expected ConflictingHeader, got {errors:?}"
        );
    }

    // ─── other header tests ───────────────────────────────────────────────────

    #[test]
    fn gauge_header_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Gauge: 15\n");
        let (_, headers, _, _) = prog.into_components();
        let (_, _, _, gauge, _, _, _) = headers.into_components();
        let gauge_ann = gauge.expect("expected gauge");
        let val = gauge_ann.value.value().clone().left().expect("expected u32").value;
        assert_eq!(val, 15);
    }

    #[test]
    fn width_header_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Width: 250\n");
        let (_, headers, _, _) = prog.into_components();
        let (_, _, _, _, width, _, _) = headers.into_components();
        let val = width.unwrap().value.value().clone().left().unwrap().value;
        assert_eq!(val, 250);
    }

    #[test]
    fn position_left_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Position: Left\n");
        let (_, headers, _, _) = prog.into_components();
        let (_, _, _, _, _, position, _) = headers.into_components();
        assert!(matches!(position.unwrap().value, PositionHeaderValue::Left(_)));
    }

    #[test]
    fn position_center_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Position: Center\n");
        let (_, headers, _, _) = prog.into_components();
        let (_, _, _, _, _, position, _) = headers.into_components();
        assert!(matches!(position.unwrap().value, PositionHeaderValue::Center(_)));
    }

    #[test]
    fn position_right_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Position: Right\n");
        let (_, headers, _, _) = prog.into_components();
        let (_, _, _, _, _, position, _) = headers.into_components();
        assert!(matches!(position.unwrap().value, PositionHeaderValue::Right(_)));
    }

    #[test]
    fn position_keep_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Position: Keep\n");
        let (_, headers, _, _) = prog.into_components();
        let (_, _, _, _, _, position, _) = headers.into_components();
        assert!(matches!(position.unwrap().value, PositionHeaderValue::Keep(_)));
    }

    #[test]
    fn bad_position_value_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\n;;Position: Bogus\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::BadPositionValue(_))),
            "expected BadPositionValue, got {errors:?}"
        );
    }

    #[test]
    fn machine_header_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Machine: SWG091N2\n");
        let (_, headers, _, _) = prog.into_components();
        let (_, _, machine, _, _, _, _) = headers.into_components();
        let s = machine.unwrap().value.value().clone().left().unwrap().value;
        assert_eq!(s, "SWG091N2");
    }

    #[test]
    fn unrecognized_extension_header_emits_warning() {
        let (_, warnings, _) = parse(";!knitout-2\n;;Carriers: A\n;;X-foo: bar\n");
        assert!(
            warnings.iter().any(|w| matches!(w, ParseWarning::UnrecognizedExtensionHeader(_))),
            "expected UnrecognizedExtensionHeader, got {warnings:?}"
        );
    }

    #[test]
    fn header_after_operations_emits_warning() {
        let (_, warnings, _) = parse(";!knitout-2\n;;Carriers: A\npause\n;;Gauge: 15\n");
        assert!(
            warnings.iter().any(|w| matches!(w, ParseWarning::HeaderAfterOperations(_))),
            "expected HeaderAfterOperations, got {warnings:?}"
        );
    }

    #[test]
    fn bad_gauge_value_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\n;;Gauge: notanumber\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::BadU32(_))),
            "expected BadU32, got {errors:?}"
        );
    }

    // ─── operation tests ──────────────────────────────────────────────────────

    fn get_ops(source: &str) -> Vec<Annotated<Operation<Parsed>, (Span, CommentInfo)>> {
        let prog = parse_ok(source);
        let (_, _, ops, _) = prog.into_components();
        ops
    }

    #[test]
    fn pause_op_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\npause\n");
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::Pause)));
    }

    #[test]
    fn knit_plus_front_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nknit + f5 A\n");
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Knit { direction, needle, carriers }) => {
                assert!(matches!(direction, Direction::Right(_)));
                let needle = needle.as_ref().left().expect("expected needle");
                assert!(matches!(needle.bed(), knitout_ast::Bed::Front(_)));
                assert!(!matches!(needle.is_slider(), knitout_ast::IsSlider::Slider(_)));
                assert_eq!(needle.index().as_ref().left().unwrap().value, 5);
                assert_eq!(carriers.len(), 1);
            }
            other => panic!("expected Knit, got {other:?}"),
        }
    }

    #[test]
    fn knit_minus_back_slider_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nknit - bs-3 A\n");
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Knit { direction, needle, .. }) => {
                assert!(matches!(direction, Direction::Left(_)));
                let n = needle.as_ref().left().unwrap();
                assert!(matches!(n.bed(), knitout_ast::Bed::Back(_)));
                assert!(matches!(n.is_slider(), knitout_ast::IsSlider::Slider(_)));
                assert_eq!(n.index().as_ref().left().unwrap().value, -3);
            }
            other => panic!("expected Knit, got {other:?}"),
        }
    }

    #[test]
    fn tuck_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\ntuck + f1 A\n");
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::Tuck { .. })));
    }

    #[test]
    fn miss_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nmiss + f1 A\n");
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::Miss { .. })));
    }

    #[test]
    fn split_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nsplit + f1 b1 A\n");
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::Split { .. })));
    }

    #[test]
    fn drop_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\ndrop f1\n");
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Drop { needle }) => {
                assert!(needle.as_ref().left().is_some());
            }
            other => panic!("expected Drop, got {other:?}"),
        }
    }

    #[test]
    fn amiss_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\namiss b5\n");
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::Amiss { .. })));
    }

    #[test]
    fn xfer_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nxfer f5 b5\n");
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Xfer { source, destination }) => {
                assert!(source.as_ref().left().is_some());
                assert!(destination.as_ref().left().is_some());
            }
            other => panic!("expected Xfer, got {other:?}"),
        }
    }

    #[test]
    fn stitch_parsed() {
        use bigdecimal::BigDecimal;
        use std::str::FromStr;
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nstitch -2 3\n");
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Stitch { before_pull, after_pull }) => {
                let bp = before_pull.value().clone().left().unwrap().value;
                let ap = after_pull.value().clone().left().unwrap().value;
                assert_eq!(bp, BigDecimal::from_str("-2").unwrap());
                assert_eq!(ap, BigDecimal::from_str("3").unwrap());
            }
            other => panic!("expected Stitch, got {other:?}"),
        }
    }

    #[test]
    fn rack_parsed() {
        use bigdecimal::BigDecimal;
        use std::str::FromStr;
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nrack 0.5\n");
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Rack { offset }) => {
                let v = offset.value().clone().left().unwrap().value;
                assert_eq!(v, BigDecimal::from_str("0.5").unwrap());
            }
            other => panic!("expected Rack, got {other:?}"),
        }
    }

    #[test]
    fn in_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A B\nin A B\n");
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::In { carriers }) => {
                assert_eq!(carriers.len(), 2);
            }
            other => panic!("expected In, got {other:?}"),
        }
    }

    #[test]
    fn inhook_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\ninhook A\n");
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::Inhook { .. })));
    }

    #[test]
    fn releasehook_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nreleasehook A\n");
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::Releasehook { .. })));
    }

    #[test]
    fn out_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nout A\n");
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::Out { .. })));
    }

    #[test]
    fn outhook_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nouthook A\n");
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::Outhook { .. })));
    }

    // ─── extension operations ─────────────────────────────────────────────────

    fn get_ext(source: &str) -> ExtensionOperation<Parsed> {
        let ops = get_ops(source);
        match ops.into_iter().next().unwrap().value {
            Operation::ExtensionOperation(ext) => ext,
            other => panic!("expected ExtensionOperation, got {other:?}"),
        }
    }

    #[test]
    fn x_presser_mode_off() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-presser-mode off\n") {
            ExtensionOperation::XPresserMode { mode } => assert!(matches!(mode, PresserMode::Off(_))),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_presser_mode_auto() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-presser-mode auto\n") {
            ExtensionOperation::XPresserMode { mode } => assert!(matches!(mode, PresserMode::Auto(_))),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_presser_mode_on() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-presser-mode on\n") {
            ExtensionOperation::XPresserMode { mode } => assert!(matches!(mode, PresserMode::On(_))),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bad_presser_mode_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nx-presser-mode bogus\n");
        assert!(errors.iter().any(|e| matches!(e, ParseError::BadPresserMode(_))));
    }

    #[test]
    fn x_xfer_style_four_pass() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-xfer-style four-pass\n") {
            ExtensionOperation::XXferStyle { style } => assert!(matches!(style, XferStyle::FourPass(_))),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_xfer_style_two_pass() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-xfer-style two-pass\n") {
            ExtensionOperation::XXferStyle { style } => assert!(matches!(style, XferStyle::TwoPass(_))),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bad_xfer_style_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nx-xfer-style bogus\n");
        assert!(errors.iter().any(|e| matches!(e, ParseError::BadXferStyle(_))));
    }

    #[test]
    fn x_stitch_number() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-stitch-number 5\n") {
            ExtensionOperation::XStitchNumber { value } => {
                assert_eq!(value.value().clone().left().unwrap().value, 5);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_speed_number() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-speed-number 3\n") {
            ExtensionOperation::XSpeedNumber { value } => {
                assert_eq!(value.value().clone().left().unwrap().value, 3);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_sub_roller_number() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-sub-roller-number 2\n") {
            ExtensionOperation::XSubRollerNumber { value } => {
                assert_eq!(value.value().clone().left().unwrap().value, 2);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_xfer_stitch_number() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-xfer-stitch-number 4\n") {
            ExtensionOperation::XXferStitchNumber { value } => {
                assert_eq!(value.value().clone().left().unwrap().value, 4);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_end_pass() {
        assert!(matches!(
            get_ext(";!knitout-2\n;;Carriers: A\nx-end-pass\n"),
            ExtensionOperation::XEndPass
        ));
    }

    #[test]
    fn x_end() {
        assert!(matches!(
            get_ext(";!knitout-2\n;;Carriers: A\nx-end\n"),
            ExtensionOperation::XEnd
        ));
    }

    #[test]
    fn x_vis_color() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-vis-color 0xff0000 A\n") {
            ExtensionOperation::XVisColor { color, carriers } => {
                let s = color.value().clone().left().unwrap().value;
                assert_eq!(s, "0xff0000");
                assert_eq!(carriers.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_roller_advance() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-roller-advance 1.5\n") {
            ExtensionOperation::XRollerAdvance { value } => {
                assert!(value.value().clone().left().is_some());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_add_roller_advance() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-add-roller-advance 0.5\n") {
            ExtensionOperation::XAddRollerAdvance { value } => {
                assert!(value.value().clone().left().is_some());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_carrier_spacing() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-carrier-spacing 5\n") {
            ExtensionOperation::XCarrierSpacing { value } => {
                assert_eq!(value.value().clone().left().unwrap().value, 5);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unknown_extension_op() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-my-custom arg1 arg2\n") {
            ExtensionOperation::UnknownExtension { name, arguments } => {
                let n = name.value().clone().left().unwrap().value;
                assert_eq!(n, "x-my-custom");
                assert_eq!(arguments.len(), 2);
                let a0 = arguments[0].value().clone().left().unwrap().value;
                let a1 = arguments[1].value().clone().left().unwrap().value;
                assert_eq!(a0, "arg1");
                assert_eq!(a1, "arg2");
            }
            other => panic!("{other:?}"),
        }
    }

    // ─── unrecognized opcode ──────────────────────────────────────────────────

    #[test]
    fn unrecognized_opcode_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nfoobar\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::UnrecognizedOpcode(_))),
            "expected UnrecognizedOpcode, got {errors:?}"
        );
    }

    #[test]
    fn unrecognized_opcode_produces_other_node() {
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\nfoobar\n");
        let (_, _, ops, _) = prog.into_components();
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0].value, Operation::Other(_)));
    }

    // ─── missing arguments ────────────────────────────────────────────────────

    #[test]
    fn missing_direction_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nknit\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MissingArgument(_))),
            "expected MissingArgument, got {errors:?}"
        );
    }

    #[test]
    fn bad_direction_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nknit x f5\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::BadDirection(_))),
            "expected BadDirection, got {errors:?}"
        );
    }

    #[test]
    fn bad_needle_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\ndrop x5\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::BadBed(_) | ParseError::BadNeedle(_))),
            "expected BadBed or BadNeedle, got {errors:?}"
        );
    }

    #[test]
    fn bad_needle_index_does_not_make_whole_op_other() {
        // A bad needle index should produce a Needle with Other index, but the
        // enclosing operation should NOT become Other.
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\ndrop fxyz\n");
        let (_, _, ops, _) = prog.into_components();
        assert_eq!(ops.len(), 1);
        // The op is Drop (not Other), even though the needle index is bad.
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::Drop { .. })));
    }

    #[test]
    fn bad_stitch_value_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nstitch abc def\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::BadBigDecimal(_))),
            "expected BadBigDecimal, got {errors:?}"
        );
    }

    #[test]
    fn too_many_arguments_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\npause extra\n");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::TooManyArguments(_))),
            "expected TooManyArguments, got {errors:?}"
        );
    }

    // ─── comment info tests ───────────────────────────────────────────────────

    #[test]
    fn no_comment_info() {
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\npause\n");
        let (_, _, ops, _) = prog.into_components();
        let (_, comment) = ops[0].annotation;
        assert_eq!(comment, CommentInfo::NoComment);
    }

    #[test]
    fn plain_comment_info() {
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\npause ; hello\n");
        let (_, _, ops, _) = prog.into_components();
        let (_, comment) = ops[0].annotation;
        assert!(matches!(comment, CommentInfo::Comment(_)));
    }

    #[test]
    fn source_tag_info() {
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\npause ;!source: myfile.js:5\n");
        let (_, _, ops, _) = prog.into_components();
        let (_, comment) = ops[0].annotation;
        assert!(matches!(comment, CommentInfo::SourceTag(_)));
    }

    #[test]
    fn unknown_tag_info() {
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\npause ;!foo\n");
        let (_, _, ops, _) = prog.into_components();
        let (_, comment) = ops[0].annotation;
        assert!(matches!(comment, CommentInfo::UnknownTag(_)));
    }

    // ─── op span within source ────────────────────────────────────────────────

    #[test]
    fn op_span_within_source() {
        let source = ";!knitout-2\n;;Carriers: A\nknit + f5 A\n";
        let (prog, _, _) = parse(source);
        let (_, _, ops, _) = prog.into_components();
        let (op_span, _) = ops[0].annotation;
        assert!(span_within(op_span, source));
    }

    #[test]
    fn all_error_spans_within_source() {
        let source = ";!knitout-2\n;;Carriers: A\nknit x badneedle\n";
        let (_, _, errors) = parse(source);
        for e in &errors {
            if let Some(span) = e.span() {
                assert!(
                    span_within(span, source),
                    "span {span:?} out of bounds for source len {}",
                    source.len()
                );
            }
        }
    }

    // ─── parsed_to_valid ──────────────────────────────────────────────────────

    #[test]
    fn valid_program_converts_to_validated() {
        let source = ";!knitout-2\n;;Carriers: A\nknit + f5 A\n";
        let (prog, warnings, errors) = parse(source);
        assert!(warnings.is_empty());
        assert!(errors.is_empty());
        let validated = parsed_to_valid(prog);
        assert!(validated.is_some(), "expected Some(Program<Validated>)");
        let v = validated.unwrap();
        let ver = v.version().as_ref().left().unwrap().value;
        assert_eq!(ver, 2);
        assert_eq!(v.operations().len(), 1);
    }

    #[test]
    fn program_with_errors_does_not_convert() {
        let (prog, _, errors) = parse(";!knitout-2\n;;Carriers: A\nfoobar\n");
        assert!(!errors.is_empty());
        let validated = parsed_to_valid(prog);
        assert!(validated.is_none(), "expected None due to errors");
    }

    #[test]
    fn missing_magic_program_does_not_convert() {
        let (prog, _, _) = parse(";;Carriers: A\n");
        let validated = parsed_to_valid(prog);
        assert!(validated.is_none());
    }

    #[test]
    fn missing_carriers_program_does_not_convert() {
        let (prog, _, _) = parse(";!knitout-2\n");
        let validated = parsed_to_valid(prog);
        assert!(validated.is_none());
    }

    #[test]
    fn validated_version_correct() {
        let source = ";!knitout-18\n;;Carriers: A B\n";
        let (prog, _, _) = parse(source);
        let v = parsed_to_valid(prog).unwrap();
        let ver = v.version().as_ref().left().unwrap().value;
        assert_eq!(ver, 18);
    }

    #[test]
    fn validated_has_all_operations() {
        let source = ";!knitout-2\n;;Carriers: A\nknit + f1 A\ntuck - b2 A\ndrop f3\n";
        let (prog, _, errors) = parse(source);
        assert!(errors.is_empty(), "{errors:?}");
        let v = parsed_to_valid(prog).unwrap();
        assert_eq!(v.operations().len(), 3);
    }

    // ─── transform_program ───────────────────────────────────────────────────

    #[test]
    fn transform_program_identity_via_parsed_to_validated() {
        let source = ";!knitout-2\n;;Carriers: 1 2\nknit + f5 1\n";
        let (prog, _, _) = parse(source);
        let result = transform_program::<Parsed, knitout_ast::Validated, ParsedToValidated>(prog);
        assert!(result.is_some());
    }

    // ─── ParseError::span ─────────────────────────────────────────────────────

    #[test]
    fn parse_error_span_missing_magic_is_none() {
        assert_eq!(ParseError::MagicMissing.span(), None);
    }

    #[test]
    fn parse_error_span_missing_carriers_is_none() {
        assert_eq!(ParseError::MissingCarriersHeader.span(), None);
    }

    #[test]
    fn parse_error_span_bad_direction_is_some() {
        let s = Span::new(0, 1);
        assert_eq!(ParseError::BadDirection(s).span(), Some(s));
    }

    // ─── ParseWarning::span ───────────────────────────────────────────────────

    #[test]
    fn parse_warning_span_magic_not_first() {
        let s = Span::new(5, 15);
        assert_eq!(ParseWarning::MagicNotFirst(s).span(), s);
    }

    #[test]
    fn parse_warning_span_header_after_ops() {
        let s = Span::new(3, 9);
        assert_eq!(ParseWarning::HeaderAfterOperations(s).span(), s);
    }

    #[test]
    fn parse_warning_span_unrecognized_extension() {
        let s = Span::new(0, 5);
        assert_eq!(ParseWarning::UnrecognizedExtensionHeader(s).span(), s);
    }

    #[test]
    fn parse_warning_span_duplicate_same_value() {
        let s = Span::new(10, 15);
        let p = Span::new(0, 5);
        assert_eq!(ParseWarning::DuplicateHeaderSameValue { span: s, previous: p }.span(), s);
    }

    // ─── needle annotation ────────────────────────────────────────────────────

    #[test]
    fn needle_annotation_span_covers_whole_needle() {
        let source = ";!knitout-2\n;;Carriers: A\ndrop f5\n";
        let source_bytes = source.as_bytes();
        let (prog, _, _) = parse(source);
        let (_, _, ops, _) = prog.into_components();
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Drop { needle }) => {
                let n = needle.as_ref().left().unwrap();
                let ann = n.annotation();
                // The needle span should point to "f5" in the source
                let text = &source_bytes[ann.start() as usize..ann.end() as usize];
                assert_eq!(text, b"f5");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bad_needle_index_keeps_needle_with_other_index() {
        // "fxyz" — bed is 'f', no slider, index bytes are "xyz" which fail to parse as i32
        let (prog, _, errors) = parse(";!knitout-2\n;;Carriers: A\ndrop fxyz\n");
        assert!(errors.iter().any(|e| matches!(e, ParseError::BadNeedleIndex(_))));
        let (_, _, ops, _) = prog.into_components();
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Drop { needle }) => {
                let n = needle.as_ref().left().unwrap();
                // index should be Other
                assert!(n.index().as_ref().right().is_some(), "expected Other index");
                // bed should still be Front
                assert!(matches!(n.bed(), knitout_ast::Bed::Front(_)));
            }
            other => panic!("{other:?}"),
        }
    }

    // ─── multi-op programs ────────────────────────────────────────────────────

    #[test]
    fn multiple_ops_in_order() {
        let source = ";!knitout-2\n;;Carriers: A\nin A\nknit + f1 A\nout A\n";
        let prog = parse_ok(source);
        let (_, _, ops, _) = prog.into_components();
        assert_eq!(ops.len(), 3);
        assert!(matches!(ops[0].value, Operation::BaseOperation(BaseOperation::In { .. })));
        assert!(matches!(ops[1].value, Operation::BaseOperation(BaseOperation::Knit { .. })));
        assert!(matches!(ops[2].value, Operation::BaseOperation(BaseOperation::Out { .. })));
    }

    #[test]
    fn empty_program_with_only_headers() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Gauge: 7\n");
        let (_, _, ops, _) = prog.into_components();
        assert!(ops.is_empty());
    }

    // ─── yarn header ─────────────────────────────────────────────────────────

    #[test]
    fn yarn_header_stored() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Yarn-A: 50/50 Wool/Nylon Natural\n");
        let (_, headers, _, _) = prog.into_components();
        assert_eq!(headers.yarn().len(), 1);
    }
}
