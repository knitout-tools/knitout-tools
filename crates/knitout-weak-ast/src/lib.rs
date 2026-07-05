mod convert;
mod diagnostics;
mod lower;

pub use convert::{ParsedToValidated, parsed_to_valid};
pub use diagnostics::{ParseError, ParseWarning};
pub use knitout_ast::{PhaseTransform, transform_program};
pub use lower::cst_to_program;

pub use knitout_ast::{Phase, Provenance, Span};

// ─── CommentInfo ─────────────────────────────────────────────────────────────

/// Information about a trailing comment on an operation line.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum CommentInfo {
    /// No trailing comment.
    NoComment,
    /// An ordinary `; text` comment. The span covers the full comment including `;`.
    Comment(Span),
    /// A `;!source: …` tag. `span` covers the whole tag; `source` is the text after `source: `,
    /// which becomes the operation's provenance when the program is validated.
    SourceTag { span: Span, source: String },
    /// A `;!foo` tag that is not a recognised knitout tag. The span covers the whole tag.
    UnknownTag(Span),
}

// ─── Parsed phase ─────────────────────────────────────────────────────────────

/// The `Parsed` phase: every AST node is annotated with a `Span` pointing to the
/// corresponding bytes in the source buffer. Unrecognised constructs are represented
/// as `Other(ParseError)` rather than aborting parsing.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Parsed {}

impl Phase for Parsed {
    /// Unrecognised constructs, carrying the `ParseError` that explains why the node could not be
    /// resolved.
    type Other = ParseError;

    /// `(prefix_span, version_span)`: e.g. `";!knitout-"` and `"2"`.
    type MagicVersion = (Span, Span);

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

    /// Span of the signed-integer token.
    type MyI32 = Span;

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
    use either::Either;
    use knitout_ast::{
        Annotated, BaseOperation, CarrierOrRail, Direction, ExtensionOperation, Operation,
        PositionHeaderValue, PresserMode, Program, XferStyle,
    };

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
        (span.end()) <= source.len()
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
        match prog.version() {
            Either::Left(ann) => assert_eq!(ann.value, 2),
            Either::Right(_) => panic!("expected version, got Other"),
        }
    }

    #[test]
    fn magic_version_18() {
        let prog = parse_ok(";!knitout-18\n;;Carriers: A\n");
        assert_eq!(prog.version().as_ref().left().unwrap().value, 18);
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
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadMagicVersion(_))),
            "expected BadMagicVersion, got {errors:?}"
        );
    }

    #[test]
    fn magic_not_first_emits_warning() {
        let (_, warnings, _) = parse(";;Carriers: A\n;!knitout-2\n");
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::MagicNotFirst(_))),
            "expected MagicNotFirst, got {warnings:?}"
        );
    }

    #[test]
    fn magic_version_span_within_source() {
        let source = ";!knitout-2\n;;Carriers: A\n";
        let (prog, _, _) = parse(source);
        if let Either::Left(ann) = prog.version() {
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
            errors
                .iter()
                .any(|e| matches!(e, ParseError::MissingCarriersHeader)),
            "expected MissingCarriersHeader, got {errors:?}"
        );
    }

    fn carriers_items(prog: &Program<Parsed>) -> Vec<CarrierOrRail<Parsed>> {
        let ann = prog
            .headers()
            .as_ref()
            .left()
            .unwrap()
            .carriers()
            .as_ref()
            .left()
            .expect("expected carriers");
        ann.value
            .carriers_and_rails()
            .left()
            .expect("expected items list")
            .to_vec()
    }

    #[test]
    fn single_carrier_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n");
        assert_eq!(carriers_items(&prog).len(), 1);
    }

    #[test]
    fn multiple_carriers_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: 1 2 3\n");
        assert_eq!(carriers_items(&prog).len(), 3);
    }

    #[test]
    fn rail_carrier_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A;B\n");
        let items = carriers_items(&prog);
        assert_eq!(items.len(), 1);
        match &items[0] {
            CarrierOrRail::Rail(rail) => {
                let left_carriers = rail.left().as_ref().left().unwrap().carriers();
                let right_carriers = rail.right().as_ref().left().unwrap().carriers();
                assert_eq!(left_carriers.len(), 1);
                assert_eq!(right_carriers.len(), 1);
            }
            other => panic!("expected Rail, got {other:?}"),
        }
    }

    #[test]
    fn multi_carrier_rail_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A,C;B,D\n");
        let items = carriers_items(&prog);
        assert_eq!(items.len(), 1);
        match &items[0] {
            CarrierOrRail::Rail(rail) => {
                let lc = rail.left().as_ref().left().unwrap().carriers();
                let rc = rail.right().as_ref().left().unwrap().carriers();
                assert_eq!(lc.len(), 2);
                assert_eq!(rc.len(), 2);
            }
            other => panic!("expected Rail, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_carriers_header_emits_warning() {
        // Any second occurrence of a header → DuplicateHeader warning (regardless of value).
        let (_, warnings, _) = parse(";!knitout-2\n;;Carriers: A\n;;Carriers: B\n");
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::DuplicateHeader { .. })),
            "expected DuplicateHeader warning, got {warnings:?}"
        );
    }

    #[test]
    fn conflicting_header_values_emit_error() {
        // Re-declaring a header with a *different* value → ConflictingHeader error (in addition to
        // the DuplicateHeader warning).
        let (_, warnings, errors) = parse(";!knitout-2\n;;Carriers: A\n;;Gauge: 10\n;;Gauge: 20\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::ConflictingHeader { .. })),
            "expected ConflictingHeader error, got {errors:?}"
        );
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::DuplicateHeader { .. })),
            "expected DuplicateHeader warning, got {warnings:?}"
        );
    }

    #[test]
    fn duplicate_header_same_value_no_conflict_error() {
        // Re-declaring a header with the *same* value → DuplicateHeader warning but no error.
        let (_, warnings, errors) = parse(";!knitout-2\n;;Carriers: A\n;;Gauge: 10\n;;Gauge: 10\n");
        assert!(
            !errors
                .iter()
                .any(|e| matches!(e, ParseError::ConflictingHeader { .. })),
            "unexpected ConflictingHeader error, got {errors:?}"
        );
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::DuplicateHeader { .. })),
            "expected DuplicateHeader warning, got {warnings:?}"
        );
    }

    // ─── other header tests ───────────────────────────────────────────────────

    #[test]
    fn gauge_header_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Gauge: 15\n");
        let gauge_ann = prog
            .headers()
            .as_ref()
            .left()
            .unwrap()
            .gauge()
            .as_ref()
            .expect("expected gauge");
        let val = gauge_ann
            .value
            .value()
            .clone()
            .left()
            .expect("expected u32");
        assert_eq!(val, 15);
    }

    #[test]
    fn width_header_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Width: 250\n");
        let val = prog
            .headers()
            .as_ref()
            .left()
            .unwrap()
            .width()
            .as_ref()
            .unwrap()
            .value
            .value()
            .clone()
            .left()
            .expect("expected u32");
        assert_eq!(val, 250);
    }

    #[test]
    fn position_left_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Position: Left\n");
        assert!(matches!(
            prog.headers()
                .as_ref()
                .left()
                .unwrap()
                .position()
                .as_ref()
                .unwrap()
                .value,
            PositionHeaderValue::Left(_)
        ));
    }

    #[test]
    fn position_center_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Position: Center\n");
        assert!(matches!(
            prog.headers()
                .as_ref()
                .left()
                .unwrap()
                .position()
                .as_ref()
                .unwrap()
                .value,
            PositionHeaderValue::Center(_)
        ));
    }

    #[test]
    fn position_right_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Position: Right\n");
        assert!(matches!(
            prog.headers()
                .as_ref()
                .left()
                .unwrap()
                .position()
                .as_ref()
                .unwrap()
                .value,
            PositionHeaderValue::Right(_)
        ));
    }

    #[test]
    fn position_keep_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Position: Keep\n");
        assert!(matches!(
            prog.headers()
                .as_ref()
                .left()
                .unwrap()
                .position()
                .as_ref()
                .unwrap()
                .value,
            PositionHeaderValue::Keep(_)
        ));
    }

    #[test]
    fn bad_position_value_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\n;;Position: Bogus\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadPositionValue(_))),
            "expected BadPositionValue, got {errors:?}"
        );
    }

    #[test]
    fn machine_header_parsed() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Machine: SWG091N2\n");
        let machine = prog
            .headers()
            .as_ref()
            .left()
            .unwrap()
            .machine()
            .as_ref()
            .unwrap();
        let s = machine
            .value
            .value()
            .as_ref()
            .left()
            .expect("expected string");
        assert_eq!(s, "SWG091N2");
    }

    #[test]
    fn unrecognized_extension_header_emits_warning() {
        let (_, warnings, _) = parse(";!knitout-2\n;;Carriers: A\n;;X-foo: bar\n");
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::UnrecognizedExtensionHeader(_))),
            "expected UnrecognizedExtensionHeader, got {warnings:?}"
        );
    }

    #[test]
    fn header_name_with_multibyte_char_does_not_panic() {
        // A header name whose byte 2 falls inside a multi-byte UTF-8 char must not panic
        // when we test for the "X-" extension prefix. It should be treated as an unknown header.
        let (prog, _, errors) = parse(";!knitout-2\n;;Carriers: A\n;;a\u{20ac}: x\n");
        assert!(
            !errors.iter().any(|e| matches!(e, ParseError::BadUtf8(_))),
            "unexpected BadUtf8, got {errors:?}"
        );
        assert!(
            prog.headers()
                .as_ref()
                .left()
                .unwrap()
                .unknown()
                .iter()
                .any(|(k, _)| k == "a\u{20ac}"),
            "expected unknown header keyed by the full name"
        );
    }

    #[test]
    fn header_after_operations_emits_warning() {
        let (_, warnings, _) = parse(";!knitout-2\n;;Carriers: A\npause\n;;Gauge: 15\n");
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::HeaderAfterOperations(_))),
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
        prog.operations().to_vec()
    }

    #[test]
    fn pause_op_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\npause\n");
        assert_eq!(ops.len(), 1);
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::Pause)
        ));
    }

    #[test]
    fn knit_plus_front_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nknit + f5 A\n");
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Knit {
                direction,
                needle,
                carriers,
            }) => {
                assert!(matches!(direction, Direction::Right(_)));
                let needle = needle.as_ref().left().expect("expected needle");
                assert!(matches!(needle.bed(), knitout_ast::Bed::Front(_)));
                assert!(!matches!(
                    needle.is_slider(),
                    knitout_ast::IsSlider::Slider(_)
                ));
                assert_eq!(needle.index().value().clone().left().unwrap(), 5);
                assert_eq!(carriers.len(), 1);
            }
            other => panic!("expected Knit, got {other:?}"),
        }
    }

    #[test]
    fn knit_minus_back_slider_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nknit - bs-3 A\n");
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Knit {
                direction, needle, ..
            }) => {
                assert!(matches!(direction, Direction::Left(_)));
                let n = needle.as_ref().left().unwrap();
                assert!(matches!(n.bed(), knitout_ast::Bed::Back(_)));
                assert!(matches!(n.is_slider(), knitout_ast::IsSlider::Slider(_)));
                assert_eq!(n.index().value().clone().left().unwrap(), -3);
            }
            other => panic!("expected Knit, got {other:?}"),
        }
    }

    #[test]
    fn tuck_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\ntuck + f1 A\n");
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::Tuck { .. })
        ));
    }

    #[test]
    fn miss_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nmiss + f1 A\n");
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::Miss { .. })
        ));
    }

    #[test]
    fn split_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nsplit + f1 b1 A\n");
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::Split { .. })
        ));
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
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::Amiss { .. })
        ));
    }

    #[test]
    fn xfer_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nxfer f5 b5\n");
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Xfer {
                source,
                destination,
            }) => {
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
            Operation::BaseOperation(BaseOperation::Stitch {
                before_pull,
                after_pull,
            }) => {
                let bp = before_pull.value().as_ref().left().unwrap();
                let ap = after_pull.value().as_ref().left().unwrap();
                assert_eq!(bp, &BigDecimal::from_str("-2").unwrap());
                assert_eq!(ap, &BigDecimal::from_str("3").unwrap());
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
                let v = offset.value().as_ref().left().unwrap();
                assert_eq!(v, &BigDecimal::from_str("0.5").unwrap());
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
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::Inhook { .. })
        ));
    }

    #[test]
    fn releasehook_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nreleasehook A\n");
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::Releasehook { .. })
        ));
    }

    #[test]
    fn out_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nout A\n");
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::Out { .. })
        ));
    }

    #[test]
    fn outhook_parsed() {
        let ops = get_ops(";!knitout-2\n;;Carriers: A\nouthook A\n");
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::Outhook { .. })
        ));
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
            ExtensionOperation::XPresserMode { mode } => {
                assert!(matches!(mode, PresserMode::Off(_)))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_presser_mode_auto() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-presser-mode auto\n") {
            ExtensionOperation::XPresserMode { mode } => {
                assert!(matches!(mode, PresserMode::Auto(_)))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_presser_mode_on() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-presser-mode on\n") {
            ExtensionOperation::XPresserMode { mode } => {
                assert!(matches!(mode, PresserMode::On(_)))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bad_presser_mode_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nx-presser-mode bogus\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadPresserMode(_)))
        );
    }

    #[test]
    fn x_xfer_style_four_pass() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-xfer-style four-pass\n") {
            ExtensionOperation::XXferStyle { style } => {
                assert!(matches!(style, XferStyle::FourPass(_)))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_xfer_style_two_pass() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-xfer-style two-pass\n") {
            ExtensionOperation::XXferStyle { style } => {
                assert!(matches!(style, XferStyle::TwoPass(_)))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bad_xfer_style_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nx-xfer-style bogus\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadXferStyle(_)))
        );
    }

    #[test]
    fn x_stitch_number() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-stitch-number 5\n") {
            ExtensionOperation::XStitchNumber { value } => {
                assert_eq!(value.value().clone().left().unwrap(), 5);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_speed_number() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-speed-number 3\n") {
            ExtensionOperation::XSpeedNumber { value } => {
                assert_eq!(value.value().clone().left().unwrap(), 3);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_sub_roller_number() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-sub-roller-number 2\n") {
            ExtensionOperation::XSubRollerNumber { value } => {
                assert_eq!(value.value().clone().left().unwrap(), 2);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_xfer_stitch_number() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-xfer-stitch-number 4\n") {
            ExtensionOperation::XXferStitchNumber { value } => {
                assert_eq!(value.value().clone().left().unwrap(), 4);
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
                let s = color.value().as_ref().left().unwrap();
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
                assert!(value.value().as_ref().left().is_some());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_add_roller_advance() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-add-roller-advance 0.5\n") {
            ExtensionOperation::XAddRollerAdvance { value } => {
                assert!(value.value().as_ref().left().is_some());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_carrier_spacing() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-carrier-spacing 5\n") {
            ExtensionOperation::XCarrierSpacing { value } => {
                assert_eq!(value.value().clone().left().unwrap(), 5);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unknown_extension_op() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-my-custom arg1 arg2\n") {
            ExtensionOperation::UnknownExtension { name, arguments } => {
                let n = name.value().as_ref().left().unwrap();
                assert_eq!(n, "x-my-custom");
                assert_eq!(arguments.len(), 2);
                let a0 = arguments[0].value().as_ref().left().unwrap();
                let a1 = arguments[1].value().as_ref().left().unwrap();
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
            errors
                .iter()
                .any(|e| matches!(e, ParseError::UnrecognizedOpcode(_))),
            "expected UnrecognizedOpcode, got {errors:?}"
        );
    }

    #[test]
    fn unrecognized_opcode_produces_other_node() {
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\nfoobar\n");
        let ops = prog.operations();
        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0].value, Operation::Other(_)));
    }

    // ─── missing arguments ────────────────────────────────────────────────────

    #[test]
    fn missing_direction_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nknit\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::MissingArgument(_))),
            "expected MissingArgument, got {errors:?}"
        );
    }

    #[test]
    fn bad_direction_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nknit x f5\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadDirection(_))),
            "expected BadDirection, got {errors:?}"
        );
    }

    #[test]
    fn bad_needle_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\ndrop x5\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadBed(_) | ParseError::BadNeedle(_))),
            "expected BadBed or BadNeedle, got {errors:?}"
        );
    }

    #[test]
    fn bad_needle_index_does_not_make_whole_op_other() {
        // A bad needle index should produce a Needle with Other index, but the
        // enclosing operation should NOT become Other.
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\ndrop fxyz\n");
        let ops = prog.operations();
        assert_eq!(ops.len(), 1);
        // The op is Drop (not Other), even though the needle index is bad.
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::Drop { .. })
        ));
    }

    #[test]
    fn bad_stitch_value_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\nstitch abc def\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadBigDecimal(_))),
            "expected BadBigDecimal, got {errors:?}"
        );
    }

    #[test]
    fn too_many_arguments_emits_error() {
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\npause extra\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::TooManyArguments(_))),
            "expected TooManyArguments, got {errors:?}"
        );
    }

    // ─── comment info tests ───────────────────────────────────────────────────

    #[test]
    fn no_comment_info() {
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\npause\n");
        let ops = prog.operations();
        let (_, comment) = &ops[0].annotation;
        assert_eq!(*comment, CommentInfo::NoComment);
    }

    #[test]
    fn plain_comment_info() {
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\npause ; hello\n");
        let ops = prog.operations();
        let (_, comment) = &ops[0].annotation;
        assert!(matches!(comment, CommentInfo::Comment(_)));
    }

    #[test]
    fn source_tag_info() {
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\npause ;!source: myfile.js:5\n");
        let ops = prog.operations();
        let (_, comment) = &ops[0].annotation;
        assert!(
            matches!(comment, CommentInfo::SourceTag { source, .. } if source == "myfile.js:5"),
            "expected SourceTag carrying the source text, got {comment:?}"
        );
    }

    #[test]
    fn unknown_tag_info() {
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\npause ;!foo\n");
        let ops = prog.operations();
        let (_, comment) = &ops[0].annotation;
        assert!(matches!(comment, CommentInfo::UnknownTag(_)));
    }

    // ─── op span within source ────────────────────────────────────────────────

    #[test]
    fn op_span_within_source() {
        let source = ";!knitout-2\n;;Carriers: A\nknit + f5 A\n";
        let (prog, _, _) = parse(source);
        let ops = prog.operations();
        let (op_span, _) = ops[0].annotation;
        assert!(span_within(op_span, source));
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
    fn bad_carrier_name_becomes_other_and_blocks_validation() {
        // An empty carrier name in a Yarn header is illegal; it must surface as an `Other`
        // node (not a bogus Carrier) and therefore block conversion to Validated.
        let (prog, _, errors) = parse(";!knitout-2\n;;Carriers: A\n;;Yarn-: red\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadCarrierName(_))),
            "expected BadCarrierName, got {errors:?}"
        );
        assert!(
            prog.headers()
                .as_ref()
                .left()
                .unwrap()
                .yarn()
                .keys()
                .any(|k| k.is_right()),
            "expected the yarn key to be an Other node"
        );
        assert!(
            parsed_to_valid(prog).is_none(),
            "a bad carrier name must block validation"
        );
    }

    #[test]
    fn conflicting_header_escalates_headers_to_other_and_blocks_validation() {
        let (prog, _, errors) = parse(";!knitout-2\n;;Carriers: A\n;;Gauge: 10\n;;Gauge: 20\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::ConflictingHeader { .. })),
            "expected ConflictingHeader, got {errors:?}"
        );
        assert!(
            prog.headers().is_right(),
            "a conflicting header should escalate the whole header block to Other"
        );
        assert!(parsed_to_valid(prog).is_none());
    }

    #[test]
    fn conflicting_magic_version_forces_other_and_blocks_validation() {
        let (prog, _, errors) = parse(";!knitout-2\n;;Carriers: A\n;!knitout-3\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::ConflictingMagicVersion { .. })),
            "expected ConflictingMagicVersion, got {errors:?}"
        );
        assert!(
            prog.version().is_right(),
            "a magic conflict should make the version Other"
        );
        assert!(parsed_to_valid(prog).is_none());
    }

    #[test]
    fn too_many_arguments_makes_operation_other_and_blocks_validation() {
        // `pause` takes no arguments; a trailing argument makes the whole op Other.
        let (prog, _, errors) = parse(";!knitout-2\n;;Carriers: A\npause 5\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::TooManyArguments(_))),
            "expected TooManyArguments, got {errors:?}"
        );
        assert!(
            matches!(
                prog.operations()[0].value,
                Operation::Other(ParseError::TooManyArguments(_))
            ),
            "the operation should be Other"
        );
        assert!(parsed_to_valid(prog).is_none());
    }

    #[test]
    fn bad_utf8_blocks_validation() {
        // 0xFF is never valid UTF-8; it appears as an operation token here.
        let mut src = b";!knitout-2\n;;Carriers: A\n".to_vec();
        src.extend_from_slice(&[b'k', b'n', b'i', b't', b' ', 0xFF, b'\n']);
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_knitout::LANGUAGE.into())
            .expect("failed to load grammar");
        let tree = parser.parse(&src, None).expect("parse returned None");
        let (prog, _, errors) = cst_to_program(&src, &tree);
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::BadUtf8(_))),
            "expected BadUtf8, got {errors:?}"
        );
        assert!(
            prog.version().is_right(),
            "bad UTF-8 should force version Other"
        );
        assert!(parsed_to_valid(prog).is_none());
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

    // ─── needle annotation ────────────────────────────────────────────────────

    #[test]
    fn needle_annotation_span_covers_whole_needle() {
        let source = ";!knitout-2\n;;Carriers: A\ndrop f5\n";
        let source_bytes = source.as_bytes();
        let (prog, _, _) = parse(source);
        let ops = prog.operations();
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Drop { needle }) => {
                let n = needle.as_ref().left().unwrap();
                let ann = n.annotation();
                // The needle span should point to "f5" in the source
                let text = &source_bytes[ann.start()..ann.end()];
                assert_eq!(text, b"f5");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bad_needle_index_keeps_needle_with_other_index() {
        // "fxyz" — bed is 'f', no slider, index bytes are "xyz" which fail to parse as i32
        let (prog, _, errors) = parse(";!knitout-2\n;;Carriers: A\ndrop fxyz\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadNeedleIndex(_)))
        );
        let ops = prog.operations();
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Drop { needle }) => {
                let n = needle.as_ref().left().unwrap();
                // index should be Other
                assert!(
                    n.index().value().clone().right().is_some(),
                    "expected Other index"
                );
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
        let ops = prog.operations();
        assert_eq!(ops.len(), 3);
        assert!(matches!(
            ops[0].value,
            Operation::BaseOperation(BaseOperation::In { .. })
        ));
        assert!(matches!(
            ops[1].value,
            Operation::BaseOperation(BaseOperation::Knit { .. })
        ));
        assert!(matches!(
            ops[2].value,
            Operation::BaseOperation(BaseOperation::Out { .. })
        ));
    }

    #[test]
    fn empty_program_with_only_headers() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Gauge: 7\n");
        let ops = prog.operations();
        assert!(ops.is_empty());
    }

    // ─── yarn header ─────────────────────────────────────────────────────────

    #[test]
    fn yarn_header_stored() {
        let prog = parse_ok(";!knitout-2\n;;Carriers: A\n;;Yarn-A: 50/50 Wool/Nylon Natural\n");
        assert_eq!(prog.headers().as_ref().left().unwrap().yarn().len(), 1);
    }

    // ─── exact span byte-offset tests ────────────────────────────────────────

    #[test]
    fn magic_version_span_exact_bytes() {
        // ";!knitout-2\n..." — ";!" is 2 bytes, "knitout-" is 8 bytes, "2" is 1 byte
        // tag_comment node covers ";!knitout-2" (bytes 0-10)
        // value node "knitout-2" starts at byte 2 (vstart=2), ends at byte 11 (vend=11)
        // prefix_span = Span::new(0, vstart + 8) = Span::new(0, 10) → ";!knitout-"
        // ver_span    = Span::new(10, 11)                            → "2"
        let source = ";!knitout-2\n;;Carriers: A\n";
        let (prog, _, _) = parse(source);
        let ann = prog.version().as_ref().left().unwrap().annotation;
        let (prefix_span, ver_span) = ann;
        let src = source.as_bytes();
        assert_eq!(&src[prefix_span.start()..prefix_span.end()], b";!knitout-");
        assert_eq!(&src[ver_span.start()..ver_span.end()], b"2");
        assert_eq!(prefix_span.start(), 0);
        assert_eq!(prefix_span.end(), 10);
        assert_eq!(ver_span.start(), 10);
        assert_eq!(ver_span.end(), 11);
    }

    #[test]
    fn magic_version_span_two_digit_version() {
        // ";!knitout-18\n..." — ver is "18" at bytes 10-11
        let source = ";!knitout-18\n;;Carriers: A\n";
        let (prog, _, _) = parse(source);
        let (prefix_span, ver_span) = prog.version().as_ref().left().unwrap().annotation;
        let src = source.as_bytes();
        assert_eq!(&src[prefix_span.start()..prefix_span.end()], b";!knitout-");
        assert_eq!(&src[ver_span.start()..ver_span.end()], b"18");
        assert_eq!(ver_span.start(), 10);
        assert_eq!(ver_span.end(), 12);
    }

    // ─── comment-line before magic ────────────────────────────────────────────

    #[test]
    fn comment_line_before_magic_triggers_magic_not_first() {
        // A plain comment on line 0 should increment line_index.
        // Then magic on line 1 (line_index=1 > 0) triggers MagicNotFirst.
        let (_, warnings, _) = parse("; just a comment\n;!knitout-2\n;;Carriers: A\n");
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::MagicNotFirst(_))),
            "expected MagicNotFirst when magic follows a comment, got {warnings:?}"
        );
    }

    #[test]
    fn comment_only_line_does_not_produce_operations() {
        // A bare comment line must not produce any operation nodes.
        let (prog, _, _) = parse(";!knitout-2\n;;Carriers: A\n; just a comment\n");
        let ops = prog.operations();
        assert!(
            ops.is_empty(),
            "comment line should produce no operations, got {ops:?}"
        );
    }

    // ─── needle span byte-offset tests ───────────────────────────────────────

    #[test]
    fn needle_bed_and_index_span_exact_bytes() {
        // Source: ";!knitout-2\n;;Carriers: A\ndrop f5\n"
        // Byte layout:
        //   0-11:  ";!knitout-2\n"
        //   12-25: ";;Carriers: A\n"  (14 bytes)
        //   26-33: "drop f5\n"
        // "f" is at byte 31, "5" is at byte 32.
        let source = ";!knitout-2\n;;Carriers: A\ndrop f5\n";
        let src = source.as_bytes();
        let (prog, _, _) = parse(source);
        let ops = prog.operations();
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Drop { needle }) => {
                let n = needle.as_ref().left().unwrap();

                // bed span covers just "f"
                let bed_span = match n.bed() {
                    knitout_ast::Bed::Front(s) => *s,
                    other => panic!("expected Front, got {other:?}"),
                };
                assert_eq!(&src[bed_span.start()..bed_span.end()], b"f");
                assert_eq!(bed_span.start(), 31);
                assert_eq!(bed_span.end(), 32);

                // NotSlider: zero-length span immediately after bed byte
                let slider_span = match n.is_slider() {
                    knitout_ast::IsSlider::NotSlider(s) => *s,
                    other => panic!("expected NotSlider, got {other:?}"),
                };
                assert_eq!(slider_span.start(), 32);
                assert_eq!(slider_span.end(), 32); // zero length

                // index span covers "5"
                let index_span = n.index().annotation();
                assert_eq!(&src[index_span.start()..index_span.end()], b"5");
                assert_eq!(index_span.start(), 32);
                assert_eq!(index_span.end(), 33);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn needle_slider_span_exact_bytes() {
        // Source: ";!knitout-2\n;;Carriers: A\ndrop fs5\n"
        //   "f" at byte 31, "s" at byte 32, "5" at byte 33
        let source = ";!knitout-2\n;;Carriers: A\ndrop fs5\n";
        let src = source.as_bytes();
        let (prog, _, _) = parse(source);
        let ops = prog.operations();
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Drop { needle }) => {
                let n = needle.as_ref().left().unwrap();

                let slider_span = match n.is_slider() {
                    knitout_ast::IsSlider::Slider(s) => *s,
                    other => panic!("expected Slider, got {other:?}"),
                };
                assert_eq!(&src[slider_span.start()..slider_span.end()], b"s");
                assert_eq!(slider_span.start(), 32);
                assert_eq!(slider_span.end(), 33);

                let index_span = n.index().annotation();
                assert_eq!(&src[index_span.start()..index_span.end()], b"5");
                assert_eq!(index_span.start(), 33);
                assert_eq!(index_span.end(), 34);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn needle_negative_index_span_exact_bytes() {
        // Source: ";!knitout-2\n;;Carriers: A\nknit - bs-3 A\n"
        //   Byte layout:
        //   0-11:  ";!knitout-2\n"
        //   12-25: ";;Carriers: A\n"
        //   26-: "knit - bs-3 A\n"
        //      "knit" = 26-29, " " = 30, "-" = 31, " " = 32,
        //      "bs-3" = 33-36: "b"=33, "s"=34, "-"=35, "3"=36, " "=37
        let source = ";!knitout-2\n;;Carriers: A\nknit - bs-3 A\n";
        let src = source.as_bytes();
        let (prog, _, _) = parse(source);
        let ops = prog.operations();
        match &ops[0].value {
            Operation::BaseOperation(BaseOperation::Knit { needle, .. }) => {
                let n = needle.as_ref().left().unwrap();

                let bed_span = match n.bed() {
                    knitout_ast::Bed::Back(s) => *s,
                    other => panic!("expected Back, got {other:?}"),
                };
                assert_eq!(&src[bed_span.start()..bed_span.end()], b"b");
                assert_eq!(bed_span.start(), 33);

                let slider_span = match n.is_slider() {
                    knitout_ast::IsSlider::Slider(s) => *s,
                    other => panic!("expected Slider, got {other:?}"),
                };
                assert_eq!(&src[slider_span.start()..slider_span.end()], b"s");
                assert_eq!(slider_span.start(), 34);
                assert_eq!(slider_span.end(), 35);

                let index_ann = n.index().value().clone().left().unwrap();
                assert_eq!(index_ann, -3);
                let index_span = n.index().annotation();
                assert_eq!(&src[index_span.start()..index_span.end()], b"-3");
                assert_eq!(index_span.start(), 35);
                assert_eq!(index_span.end(), 37);
            }
            other => panic!("{other:?}"),
        }
    }

    // ─── header span byte-offset tests ───────────────────────────────────────

    #[test]
    fn gauge_header_name_and_value_span_exact_bytes() {
        // Source: ";!knitout-2\n;;Carriers: A\n;;Gauge: 15\n"
        // Byte layout:
        //   0-11:  ";!knitout-2\n"
        //   12-25: ";;Carriers: A\n"
        //   26-36: ";;Gauge: 15"  (header node, excludes the trailing \n)
        //     26=;, 27=;, 28=G, 29=a, 30=u, 31=g, 32=e, 33=:, 34= , 35=1, 36=5, 37=\n
        // header_span = Span::new(26, 37) → ";;Gauge: 15"
        // value span: trim_start=35, end=37 → "15"
        let source = ";!knitout-2\n;;Carriers: A\n;;Gauge: 15\n";
        let src = source.as_bytes();
        let (prog, _, _) = parse(source);
        let gauge_ann = prog
            .headers()
            .as_ref()
            .left()
            .unwrap()
            .gauge()
            .as_ref()
            .expect("expected gauge header");
        // P::Header annotation is now the whole header span (starting at ";;")
        let header_span = gauge_ann.annotation;
        assert_eq!(&src[header_span.start()..header_span.end()], b";;Gauge: 15");
        assert_eq!(header_span.start(), 26);
        assert_eq!(header_span.end(), 37);
        // value span still covers exactly "15"
        let value_span = gauge_ann.value.annotation();
        assert_eq!(&src[value_span.start()..value_span.end()], b"15");
        assert_eq!(value_span.start(), 35);
        assert_eq!(value_span.end(), 37);
    }

    #[test]
    fn gauge_header_value_with_leading_whitespace_span() {
        // ";;Gauge:   15\n" — three leading spaces before "15"
        // Trims to "15"; value_trim_start should point past all three spaces.
        let source = ";!knitout-2\n;;Carriers: A\n;;Gauge:   15\n";
        let src = source.as_bytes();
        // Count bytes: 0-11 magic, 12-25 carriers (14 bytes), 26- gauge
        // ";;Gauge:   15\n": ;;=26-27, content "Gauge:   15" starts at 28
        // colon at pos 5 in content (abs 33), value=" " " " " " "1""5", value_start=34
        // three spaces → trim_offset=3, trim_start=37
        // value_end = 28 + len("Gauge:   15") = 28 + 11 = 39
        let (prog, _, _) = parse(source);
        let gauge_ann = prog
            .headers()
            .as_ref()
            .left()
            .unwrap()
            .gauge()
            .as_ref()
            .expect("expected gauge header");
        let value_span = gauge_ann.value.annotation();
        assert_eq!(&src[value_span.start()..value_span.end()], b"15");
        // value must start AFTER the three leading spaces
        assert!(
            value_span.start() > 34,
            "value_trim_start should skip leading spaces"
        );
    }

    // ─── carrier header span byte-offset tests ────────────────────────────────

    #[test]
    fn carrier_token_span_exact_bytes() {
        // Source: ";!knitout-2\n;;Carriers: A B\n"
        // Byte layout:
        //   0-11:  ";!knitout-2\n"
        //   12-27: ";;Carriers: A B\n"  (16 bytes)
        //     12=;, 13=;, 14=C..21=s, 22=:, 23= , 24=A, 25= , 26=B, 27=\n
        // trimmed value "A B" starts at byte 24
        // token "A": abs_start=24, abs_end=25
        // token "B": abs_start=26, abs_end=27
        let source = ";!knitout-2\n;;Carriers: A B\n";
        let src = source.as_bytes();
        let (prog, _, _) = parse(source);
        let ann = prog
            .headers()
            .as_ref()
            .left()
            .unwrap()
            .carriers()
            .as_ref()
            .left()
            .unwrap();
        let items = ann.value.carriers_and_rails().left().unwrap();
        assert_eq!(items.len(), 2);
        match &items[0] {
            CarrierOrRail::Carrier(c) => {
                let span = *c.annotation();
                println!("Carrier A span: {:?}", span);
                assert_eq!(&src[span.start()..span.end()], b"A");
                assert_eq!(span.start(), 24);
                assert_eq!(span.end(), 25);
            }
            other => panic!("expected Carrier, got {other:?}"),
        }
        match &items[1] {
            CarrierOrRail::Carrier(c) => {
                let span = *c.annotation();
                assert_eq!(&src[span.start()..span.end()], b"B");
                assert_eq!(span.start(), 26);
                assert_eq!(span.end(), 27);
            }
            other => panic!("expected Carrier, got {other:?}"),
        }
    }

    #[test]
    fn rail_carrier_spans_exact_bytes() {
        // Source: ";!knitout-2\n;;Carriers: A,C;B,D\n"
        // Byte layout:
        //   0-11:  ";!knitout-2\n"
        //   12-31: ";;Carriers: A,C;B,D\n"  (20 bytes)
        //     12=;, 13=;, 14=C..21=s, 22=:, 23= , 24=A, 25=,,26=C, 27=;, 28=B, 29=,, 30=D, 31=\n
        // trimmed value "A,C;B,D" starts at byte 24, ends at byte 31
        // rail token: abs_start=24, abs_end=31 (semicolon at byte 27)
        // left side "A,C": left_start=24, left_end=27
        //   carrier "A": part_start=24, part_end=25
        //   carrier "C": part_start=26, part_end=27
        // right side "B,D": right_start=28, right_end=31
        //   carrier "B": part_start=28, part_end=29
        //   carrier "D": part_start=30, part_end=31
        let source = ";!knitout-2\n;;Carriers: A,C;B,D\n";
        let src = source.as_bytes();
        let (prog, _, _) = parse(source);
        let ann = prog
            .headers()
            .as_ref()
            .left()
            .unwrap()
            .carriers()
            .as_ref()
            .left()
            .unwrap();
        let items = ann.value.carriers_and_rails().left().unwrap();
        assert_eq!(items.len(), 1);
        match &items[0] {
            CarrierOrRail::Rail(rail) => {
                let rail_span = *rail.annotation();
                assert_eq!(&src[rail_span.start()..rail_span.end()], b"A,C;B,D");
                assert_eq!(rail_span.start(), 24);
                assert_eq!(rail_span.end(), 31);

                let left_side = rail.left().as_ref().left().unwrap();
                let left_span = *left_side.annotation();
                let left_carriers = left_side.carriers();
                assert_eq!(&src[left_span.start()..left_span.end()], b"A,C");
                assert_eq!(left_span.start(), 24);
                assert_eq!(left_span.end(), 27);
                assert_eq!(
                    &src[left_carriers[0]
                        .as_ref()
                        .left()
                        .unwrap()
                        .annotation()
                        .start()
                        ..left_carriers[0].as_ref().left().unwrap().annotation().end()],
                    b"A"
                );
                assert_eq!(
                    left_carriers[0]
                        .as_ref()
                        .left()
                        .unwrap()
                        .annotation()
                        .start(),
                    24
                );
                assert_eq!(
                    left_carriers[0].as_ref().left().unwrap().annotation().end(),
                    25
                );
                assert_eq!(
                    &src[left_carriers[1]
                        .as_ref()
                        .left()
                        .unwrap()
                        .annotation()
                        .start()
                        ..left_carriers[1].as_ref().left().unwrap().annotation().end()],
                    b"C"
                );
                assert_eq!(
                    left_carriers[1]
                        .as_ref()
                        .left()
                        .unwrap()
                        .annotation()
                        .start(),
                    26
                );
                assert_eq!(
                    left_carriers[1].as_ref().left().unwrap().annotation().end(),
                    27
                );

                let right_side = rail.right().as_ref().left().unwrap();
                let right_span = *right_side.annotation();
                let right_carriers = right_side.carriers();
                assert_eq!(&src[right_span.start()..right_span.end()], b"B,D");
                assert_eq!(right_span.start(), 28);
                assert_eq!(right_span.end(), 31);
                assert_eq!(
                    &src[right_carriers[0]
                        .as_ref()
                        .left()
                        .unwrap()
                        .annotation()
                        .start()
                        ..right_carriers[0]
                            .as_ref()
                            .left()
                            .unwrap()
                            .annotation()
                            .end()],
                    b"B"
                );
                assert_eq!(
                    right_carriers[0]
                        .as_ref()
                        .left()
                        .unwrap()
                        .annotation()
                        .start(),
                    28
                );
                assert_eq!(
                    right_carriers[0]
                        .as_ref()
                        .left()
                        .unwrap()
                        .annotation()
                        .end(),
                    29
                );
                assert_eq!(
                    &src[right_carriers[1]
                        .as_ref()
                        .left()
                        .unwrap()
                        .annotation()
                        .start()
                        ..right_carriers[1]
                            .as_ref()
                            .left()
                            .unwrap()
                            .annotation()
                            .end()],
                    b"D"
                );
                assert_eq!(
                    right_carriers[1]
                        .as_ref()
                        .left()
                        .unwrap()
                        .annotation()
                        .start(),
                    30
                );
                assert_eq!(
                    right_carriers[1]
                        .as_ref()
                        .left()
                        .unwrap()
                        .annotation()
                        .end(),
                    31
                );
            }
            other => panic!("expected Rail, got {other:?}"),
        }
    }

    // ─── lowercase x-header ───────────────────────────────────────────────────

    #[test]
    fn lowercase_x_header_triggers_unrecognized_warning() {
        // ";;x-foo: bar" uses lowercase 'x'. The guard does not ignore case,
        // so this should not emit UnrecognizedExtensionHeader.
        let (_, warnings, _) = parse(";!knitout-2\n;;Carriers: A\n;;x-foo: bar\n");
        assert!(
            !warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::UnrecognizedExtensionHeader(_))),
            "expected no UnrecognizedExtensionHeader for lowercase x-header, got {warnings:?}"
        );
    }

    #[test]
    fn non_x_unknown_header_does_not_emit_extension_warning() {
        // ";;FooBar: baz" is not an X- header. The extension-header arm guard
        // (s.len() >= 2 && s[..2].eq_ignore_ascii_case("X-")) must NOT match it.
        let (_, warnings, _) = parse(";!knitout-2\n;;Carriers: A\n;;FooBar: baz\n");
        assert!(
            !warnings
                .iter()
                .any(|w| matches!(w, ParseWarning::UnrecognizedExtensionHeader(_))),
            "non-X- header should not emit UnrecognizedExtensionHeader: {warnings:?}"
        );
    }

    // ─── carriers trailing-space loop-bound test ──────────────────────────────

    #[test]
    fn carriers_with_trailing_space_parses_correctly() {
        // ";;Carriers: A B \n" — trailing space in value.
        // Only leading whitespace is trimmed, so parse_carriers_header_value
        // receives b"A B " (trailing space). The loop must correctly handle
        // ending on a space without accessing text[text.len()].
        let prog = parse_ok(";!knitout-2\n;;Carriers: A B \n");
        let items = carriers_items(&prog);
        assert_eq!(items.len(), 2, "expected 2 carriers, got {}", items.len());
    }

    #[test]
    fn carriers_with_multiple_trailing_spaces() {
        // Extra trailing whitespace must not cause any panic or extra carriers.
        let prog = parse_ok(";!knitout-2\n;;Carriers: 1 2 3   \n");
        let items = carriers_items(&prog);
        assert_eq!(items.len(), 3, "expected 3 carriers, got {}", items.len());
    }

    // ─── bare bed-letter needle ───────────────────────────────────────────────

    #[test]
    fn bare_bed_letter_needle_emits_error() {
        // "drop f" — needle text is b"f", len=1. After consuming the bed byte,
        // pos=1 == text.len(). The is_slider check must NOT access text[1].
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\ndrop f\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadNeedleIndex(_) | ParseError::BadNeedle(_))),
            "expected error for bare bed-letter needle, got {errors:?}"
        );
    }

    #[test]
    fn bare_back_bed_letter_needle_emits_error() {
        // Same as above but for 'b' to ensure both bed chars are covered.
        let (_, _, errors) = parse(";!knitout-2\n;;Carriers: A\ndrop b\n");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ParseError::BadNeedleIndex(_) | ParseError::BadNeedle(_))),
            "expected error for bare back-bed-letter needle, got {errors:?}"
        );
    }

    // ─── missing x-op tests ───────────────────────────────────────────────────

    #[test]
    fn x_loop_in() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-loop-in + f5 A\n") {
            ExtensionOperation::XLoopIn {
                direction,
                needle,
                carriers,
            } => {
                assert!(matches!(direction, Direction::Right(_)));
                assert!(needle.as_ref().left().is_some());
                assert_eq!(carriers.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_yarn_in() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-yarn-in - b3 A\n") {
            ExtensionOperation::XYarnIn {
                direction,
                needle,
                carriers,
            } => {
                assert!(matches!(direction, Direction::Left(_)));
                assert!(needle.as_ref().left().is_some());
                assert_eq!(carriers.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_yarn_out() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-yarn-out + f1 A\n") {
            ExtensionOperation::XYarnOut {
                direction,
                needle,
                carriers,
            } => {
                assert!(matches!(direction, Direction::Right(_)));
                assert!(needle.as_ref().left().is_some());
                assert_eq!(carriers.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_carrier_stopping_distance() {
        match get_ext(";!knitout-2\n;;Carriers: A\nx-carrier-stopping-distance 10\n") {
            ExtensionOperation::XCarrierStoppingDistance { value } => {
                assert_eq!(value.value().clone().left().unwrap(), 10);
            }
            other => panic!("{other:?}"),
        }
    }

    // ─── header without colon ────────────────────────────────────────────────

    #[test]
    fn gauge_header_without_colon_is_silently_ignored() {
        // ";;Gauge" has no colon, so there is no name:value split possible.
        // lower_header returns early without emitting any error.
        // The program is still valid (Gauge header simply absent).
        let source = ";!knitout-2\n;;Carriers: A\n;;Gauge\n";
        let (prog, _, errors) = parse(source);
        assert!(
            !errors.iter().any(|e| matches!(e, ParseError::BadU32(_))),
            "should not emit BadU32 for a header with no colon"
        );
        assert!(
            prog.headers().as_ref().left().unwrap().gauge().is_none(),
            "Gauge header should be absent when content has no colon"
        );
    }

    // ─── validated operation provenance ──────────────────────────────────────

    #[test]
    fn validated_operation_provenance_carries_span() {
        // transform_operation carries the operation's source span into its Provenance.
        let source = ";!knitout-2\n;;Carriers: A\npause\n";
        let (prog, _, errors) = parse(source);
        assert!(errors.is_empty(), "{errors:?}");
        let v = parsed_to_valid(prog).unwrap();
        let ops = v.operations();
        assert_eq!(ops.len(), 1);
        // "pause" starts at byte 26 in the source above.
        let span = ops[0]
            .annotation
            .span()
            .expect("expected a provenance span");
        assert_eq!(&source[span.start()..span.end()], "pause");
        assert_eq!(ops[0].annotation.source(), None);
    }

    #[test]
    fn validated_operation_provenance_carries_source_tag() {
        // A `;!source:` tag becomes the operation's provenance source string.
        let source = ";!knitout-2\n;;Carriers: A\npause ;!source: myfile.js:5\n";
        let (prog, _, errors) = parse(source);
        assert!(errors.is_empty(), "{errors:?}");
        let v = parsed_to_valid(prog).unwrap();
        let ops = v.operations();
        assert_eq!(ops[0].annotation.source(), Some("myfile.js:5"));
        assert!(ops[0].annotation.span().is_some());
    }

    #[test]
    fn validated_operation_provenance_multi_op() {
        let source = ";!knitout-2\n;;Carriers: A\nknit + f1 A\ntuck - b2 A\n";
        let (prog, _, errors) = parse(source);
        assert!(errors.is_empty(), "{errors:?}");
        let v = parsed_to_valid(prog).unwrap();
        for op in v.operations() {
            assert!(op.annotation.span().is_some());
            assert_eq!(op.annotation.source(), None);
        }
    }
}
