mod convert;
mod diagnostics;
mod lower;
mod types;

pub use diagnostics::ParseError;
pub use lower::lower_cst;
pub use types::{
    WeakBaseOperation, WeakBaseOperationKind, WeakBed, WeakCarrierOrRail, WeakCarriersHeader,
    WeakCarriersValue, WeakComment, WeakDirection, WeakExtensionHeader, WeakExtensionOperation,
    WeakExtensionOperationKind, WeakGaugeHeader, WeakHeader, WeakHeaderKind, WeakLine,
    WeakMachineHeader, WeakMagic, WeakNeedle, WeakOperation, WeakPositionHeader, WeakPositionValue,
    WeakPresserMode, WeakProgram, WeakRail, WeakRailSide, WeakWidthHeader, WeakXferStyle,
    WeakYarnHeader,
};

// Re-export knitout-ast types used in the public surface
pub use knitout_ast::{Carrier, CarrierTable, IsSlider, Span};

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> (WeakProgram, Vec<ParseError>) {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_knitout::LANGUAGE.into())
            .expect("failed to load grammar");
        let tree = parser.parse(source, None).expect("parse returned None");
        lower_cst(source.as_bytes(), &tree)
    }

    /// Build a minimal WeakProgram with the given lines, for conversion tests.
    fn make_prog(lines: Vec<WeakLine>) -> WeakProgram {
        let span = Span::new(0, 0);
        WeakProgram {
            span,
            magic: WeakMagic { span, version: Some(2) },
            headers: vec![],
            lines,
            carrier_table: CarrierTable::new(),
        }
    }

    fn make_line(op: WeakOperation) -> WeakLine {
        WeakLine {
            span: Span::new(0, 0),
            operation: Some(op),
            comment: None,
            source: None,
        }
    }

    fn make_base(kind: WeakBaseOperationKind) -> WeakOperation {
        WeakOperation::Base(WeakBaseOperation {
            span: Span::new(0, 0),
            kind,
        })
    }

    fn make_needle(bed: WeakBed, index: i32) -> WeakNeedle {
        WeakNeedle {
            span: Span::new(0, 0),
            bed,
            is_slider: IsSlider::NotSlider,
            index: Some(index),
        }
    }

    // -----------------------------------------------------------------------
    // Happy-path lowering tests
    // -----------------------------------------------------------------------

    #[test]
    fn valid_minimal_program_has_no_diagnostics() {
        let (prog, diags) = parse(";!knitout-2\n");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        assert_eq!(prog.magic.version, Some(2));
    }

    #[test]
    fn valid_program_converts_to_strong() {
        let source = ";!knitout-2\n;;Carriers: A B\nknit + f5 A\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let strong = prog.try_into_strong().expect("conversion should succeed");
        assert_eq!(strong.version(), 2);
        assert_eq!(strong.operations().len(), 1);
    }

    #[test]
    fn knit_operation_fully_parsed() {
        let source = ";!knitout-2\n;;Carriers: A\nknit + fs3 A\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        assert_eq!(prog.lines.len(), 1);
        let line = &prog.lines[0];
        match line.operation.as_ref().unwrap() {
            WeakOperation::Base(b) => match &b.kind {
                WeakBaseOperationKind::Knit {
                    direction,
                    needle,
                    carriers,
                } => {
                    assert_eq!(*direction, WeakDirection::Right);
                    let n = needle.as_ref().unwrap();
                    assert_eq!(n.bed, WeakBed::Front);
                    assert_eq!(n.is_slider, IsSlider::Slider);
                    assert_eq!(n.index, Some(3));
                    assert_eq!(carriers.len(), 1);
                }
                other => panic!("expected Knit, got {other:?}"),
            },
            other => panic!("expected Base, got {other:?}"),
        }
    }

    #[test]
    fn presser_mode_parsed() {
        let source = ";!knitout-2\nx-presser-mode auto\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        match prog.lines[0].operation.as_ref().unwrap() {
            WeakOperation::Extension(e) => match &e.kind {
                WeakExtensionOperationKind::XPresserMode { mode } => {
                    assert_eq!(*mode, WeakPresserMode::Auto);
                }
                other => panic!("expected XPresserMode, got {other:?}"),
            },
            other => panic!("expected Extension, got {other:?}"),
        }
    }

    #[test]
    fn source_comment_extracted() {
        let source = ";!knitout-2\nknit + f1 ;!source: myfile.js:5\n";
        let (prog, diags) = parse(source);
        let line = &prog.lines[0];
        assert_eq!(line.source.as_deref(), Some("myfile.js:5"));
        let _ = diags;
    }

    #[test]
    fn program_with_all_headers_converts() {
        let source = concat!(
            ";!knitout-18\n",
            ";;Carriers: A B\n",
            ";;Gauge: 15\n",
            ";;Width: 250\n",
            ";;Position: Left\n",
            ";;Machine: MyMachine\n",
        );
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let strong = prog.try_into_strong().expect("conversion failed");
        assert_eq!(strong.version(), 18);
        assert_eq!(strong.headers().gauge(), Some(15));
        assert_eq!(strong.headers().width(), Some(250));
    }

    #[test]
    fn stitch_and_rack_parsed() {
        let source = ";!knitout-2\nstitch -2 3\nrack 0.5\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        assert_eq!(prog.lines.len(), 2);
    }

    #[test]
    fn xfer_parsed() {
        let source = ";!knitout-2\nxfer f5 b5\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        match prog.lines[0].operation.as_ref().unwrap() {
            WeakOperation::Base(b) => match &b.kind {
                WeakBaseOperationKind::Xfer { source, destination } => {
                    assert!(source.is_some());
                    assert!(destination.is_some());
                }
                other => panic!("expected Xfer, got {other:?}"),
            },
            other => panic!("expected Base, got {other:?}"),
        }
    }

    #[test]
    fn in_out_carriers_parsed() {
        let source = ";!knitout-2\n;;Carriers: 1 2\nin 1 2\nout 1 2\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        assert_eq!(prog.lines.len(), 2);
        let strong = prog.try_into_strong().expect("conversion failed");
        assert_eq!(strong.operations().len(), 2);
    }

    // -----------------------------------------------------------------------
    // Direction and bed variant tests
    // -----------------------------------------------------------------------

    #[test]
    fn knit_left_direction_and_back_bed() {
        let source = ";!knitout-2\nknit - b5\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        match prog.lines[0].operation.as_ref().unwrap() {
            WeakOperation::Base(b) => match &b.kind {
                WeakBaseOperationKind::Knit {
                    direction, needle, ..
                } => {
                    assert_eq!(*direction, WeakDirection::Left);
                    assert_eq!(needle.as_ref().unwrap().bed, WeakBed::Back);
                }
                other => panic!("expected Knit, got {other:?}"),
            },
            other => panic!("expected Base, got {other:?}"),
        }
    }

    #[test]
    fn knit_no_carriers_gives_empty_vec() {
        let source = ";!knitout-2\nknit + f5\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        match prog.lines[0].operation.as_ref().unwrap() {
            WeakOperation::Base(b) => match &b.kind {
                WeakBaseOperationKind::Knit { carriers, .. } => {
                    assert!(carriers.is_empty(), "expected no carriers, got {carriers:?}");
                }
                other => panic!("expected Knit, got {other:?}"),
            },
            other => panic!("expected Base, got {other:?}"),
        }
    }

    #[test]
    fn all_presser_modes_parsed() {
        for (mode_str, expected) in [
            ("off", WeakPresserMode::Off),
            ("auto", WeakPresserMode::Auto),
            ("on", WeakPresserMode::On),
        ] {
            let source = format!(";!knitout-2\nx-presser-mode {mode_str}\n");
            let (prog, diags) = parse(&source);
            assert!(
                diags.is_empty(),
                "mode={mode_str}: unexpected diags: {diags:?}"
            );
            match prog.lines[0].operation.as_ref().unwrap() {
                WeakOperation::Extension(e) => match &e.kind {
                    WeakExtensionOperationKind::XPresserMode { mode } => {
                        assert_eq!(*mode, expected, "mode={mode_str}");
                    }
                    other => panic!("expected XPresserMode, got {other:?}"),
                },
                other => panic!("expected Extension, got {other:?}"),
            }
        }
    }

    #[test]
    fn stitch_values_parsed_correctly() {
        use bigdecimal::BigDecimal;
        use std::str::FromStr;
        let source = ";!knitout-2\nstitch -2 3\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        match prog.lines[0].operation.as_ref().unwrap() {
            WeakOperation::Base(b) => match &b.kind {
                WeakBaseOperationKind::Stitch {
                    before_pull,
                    after_pull,
                } => {
                    assert_eq!(
                        *before_pull.as_ref().unwrap(),
                        BigDecimal::from_str("-2").unwrap()
                    );
                    assert_eq!(
                        *after_pull.as_ref().unwrap(),
                        BigDecimal::from_str("3").unwrap()
                    );
                }
                other => panic!("expected Stitch, got {other:?}"),
            },
            other => panic!("expected Base, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // MISSING-node lowering tests (tree-sitter partially parses these)
    // -----------------------------------------------------------------------

    /// `stitch -2` has one argument; tree-sitter produces a MISSING after_pull.
    #[test]
    fn stitch_missing_after_pull_produces_missing_field_diagnostic() {
        let source = ";!knitout-2\nstitch -2\n";
        let (prog, diags) = parse(source);
        match prog.lines[0].operation.as_ref().unwrap() {
            WeakOperation::Base(b) => match &b.kind {
                WeakBaseOperationKind::Stitch { after_pull, .. } => {
                    assert!(after_pull.is_none(), "expected after_pull=None");
                }
                other => panic!("expected Stitch, got {other:?}"),
            },
            other => panic!("expected Base, got {other:?}"),
        }
        assert!(
            diags
                .iter()
                .any(|d| matches!(d, ParseError::MissingField { field: "after_pull", .. })),
            "expected MissingField{{after_pull}}, got: {diags:?}"
        );
    }

    /// When the input has no valid magic line, lower_cst emits a file-level diagnostic
    /// (UnparsedFile when the root is an ERROR node, or MissingMagic when the root
    /// is otherwise valid but lacks a magic field).
    #[test]
    fn no_magic_line_produces_file_level_diagnostic() {
        let source = ";\n";
        let (prog, diags) = parse(source);
        assert!(prog.magic.version.is_none(), "version should be None");
        assert!(
            diags.iter().any(|d| matches!(
                d,
                ParseError::MissingMagic(_) | ParseError::UnparsedFile(_)
            )),
            "expected a file-level diagnostic, got: {diags:?}"
        );
    }

    /// Completely invalid syntax produces some diagnostic — the exact kind
    /// depends on tree-sitter's error recovery.
    #[test]
    fn completely_invalid_syntax_produces_diagnostics() {
        let source = ";!knitout-2\nthis_is_not_valid!!!\n";
        let (prog, diags) = parse(source);
        assert!(!diags.is_empty(), "expected at least one diagnostic for garbage input");
        let _ = prog;
    }

    /// For inputs that produce tree-sitter ERROR nodes, an InvalidOperation
    /// diagnostic is emitted.
    #[test]
    fn error_node_emits_invalid_operation_diagnostic() {
        let source = ";!knitout-2\nthis_is_not_valid!!!\n";
        let (_prog, diags) = parse(source);
        assert!(
            diags.iter().any(|d| matches!(d, ParseError::InvalidOperation(_))),
            "expected InvalidOperation diagnostic, got: {diags:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Span sanity checks
    // -----------------------------------------------------------------------

    #[test]
    fn parse_error_spans_are_within_source() {
        let source = ";!knitout-2\nknit +\n";
        let (_prog, diags) = parse(source);
        for d in &diags {
            let s = d.span();
            assert!(
                (s.end() as usize) <= source.len(),
                "span end {} > source len {}",
                s.end(),
                source.len()
            );
        }
    }

    // -----------------------------------------------------------------------
    // Leaf converter tests
    // -----------------------------------------------------------------------

    #[test]
    fn convert_unknown_bed_fails_with_unrecognized_bed() {
        let span = Span::new(0, 1);
        let result = WeakBed::Unknown(span).try_into_strong();
        assert!(
            matches!(result, Err(ParseError::UnrecognizedBed(_))),
            "expected UnrecognizedBed, got {result:?}"
        );
    }

    #[test]
    fn convert_front_bed_succeeds() {
        assert!(matches!(WeakBed::Front.try_into_strong(), Ok(knitout_ast::Bed::Front)));
    }

    #[test]
    fn convert_back_bed_succeeds() {
        assert!(matches!(WeakBed::Back.try_into_strong(), Ok(knitout_ast::Bed::Back)));
    }

    #[test]
    fn convert_unknown_direction_fails_with_unrecognized_direction() {
        let span = Span::new(0, 1);
        let result = WeakDirection::Unknown(span).try_into_strong();
        assert!(
            matches!(result, Err(ParseError::UnrecognizedDirection(_))),
            "expected UnrecognizedDirection, got {result:?}"
        );
    }

    #[test]
    fn convert_left_direction_succeeds() {
        assert!(matches!(
            WeakDirection::Left.try_into_strong(),
            Ok(knitout_ast::Direction::Left)
        ));
    }

    #[test]
    fn convert_right_direction_succeeds() {
        assert!(matches!(
            WeakDirection::Right.try_into_strong(),
            Ok(knitout_ast::Direction::Right)
        ));
    }

    #[test]
    fn convert_unknown_presser_mode_fails() {
        let span = Span::new(0, 1);
        let result = WeakPresserMode::Unknown(span).try_into_strong();
        assert!(
            matches!(result, Err(ParseError::UnrecognizedPresserMode(_))),
            "expected UnrecognizedPresserMode, got {result:?}"
        );
    }

    #[test]
    fn convert_presser_mode_off_succeeds() {
        assert!(matches!(
            WeakPresserMode::Off.try_into_strong(),
            Ok(knitout_ast::PresserMode::Off)
        ));
    }

    #[test]
    fn convert_presser_mode_auto_succeeds() {
        assert!(matches!(
            WeakPresserMode::Auto.try_into_strong(),
            Ok(knitout_ast::PresserMode::Auto)
        ));
    }

    #[test]
    fn convert_presser_mode_on_succeeds() {
        assert!(matches!(
            WeakPresserMode::On.try_into_strong(),
            Ok(knitout_ast::PresserMode::On)
        ));
    }

    #[test]
    fn convert_unknown_xfer_style_fails() {
        let span = Span::new(0, 1);
        let result = WeakXferStyle::Unknown(span).try_into_strong();
        assert!(
            matches!(result, Err(ParseError::UnrecognizedXferStyle(_))),
            "expected UnrecognizedXferStyle, got {result:?}"
        );
    }

    #[test]
    fn convert_four_pass_succeeds() {
        assert!(matches!(
            WeakXferStyle::FourPass.try_into_strong(),
            Ok(knitout_ast::XferStyle::FourPass)
        ));
    }

    #[test]
    fn convert_two_pass_succeeds() {
        assert!(matches!(
            WeakXferStyle::TwoPass.try_into_strong(),
            Ok(knitout_ast::XferStyle::TwoPass)
        ));
    }

    #[test]
    fn convert_unknown_position_value_fails() {
        let span = Span::new(0, 1);
        let result = WeakPositionValue::Unknown(span).try_into_strong();
        assert!(
            matches!(result, Err(ParseError::UnrecognizedPositionValue(_))),
            "expected UnrecognizedPositionValue, got {result:?}"
        );
    }

    #[test]
    fn convert_position_values_succeed() {
        use knitout_ast::PositionHeaderValue;
        assert!(matches!(
            WeakPositionValue::Left.try_into_strong(),
            Ok(PositionHeaderValue::Left)
        ));
        assert!(matches!(
            WeakPositionValue::Center.try_into_strong(),
            Ok(PositionHeaderValue::Center)
        ));
        assert!(matches!(
            WeakPositionValue::Right.try_into_strong(),
            Ok(PositionHeaderValue::Right)
        ));
        assert!(matches!(
            WeakPositionValue::Keep.try_into_strong(),
            Ok(PositionHeaderValue::Keep)
        ));
    }

    // -----------------------------------------------------------------------
    // WeakProgram conversion-path tests (constructed directly, no parser)
    // -----------------------------------------------------------------------

    #[test]
    fn convert_missing_version_fails() {
        let span = Span::new(0, 0);
        let prog = WeakProgram {
            span,
            magic: WeakMagic { span, version: None },
            headers: vec![],
            lines: vec![],
            carrier_table: CarrierTable::new(),
        };
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingVersion(_))),
            "expected MissingVersion, got {result:?}"
        );
    }

    #[test]
    fn convert_unknown_operation_fails_with_invalid_operation() {
        let span = Span::new(0, 4);
        let prog = make_prog(vec![make_line(WeakOperation::Unknown(span))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::InvalidOperation(_))),
            "expected InvalidOperation, got {result:?}"
        );
    }

    #[test]
    fn convert_rack_missing_offset_fails() {
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Rack {
            offset: None,
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "offset", .. })),
            "expected MissingField{{offset}}, got {result:?}"
        );
    }

    #[test]
    fn convert_stitch_missing_before_pull_fails() {
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Stitch {
            before_pull: None,
            after_pull: None,
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "before_pull", .. })),
            "expected MissingField{{before_pull}}, got {result:?}"
        );
    }

    #[test]
    fn convert_stitch_missing_after_pull_fails() {
        use bigdecimal::BigDecimal;
        use std::str::FromStr;
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Stitch {
            before_pull: Some(BigDecimal::from_str("-2").unwrap()),
            after_pull: None,
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "after_pull", .. })),
            "expected MissingField{{after_pull}}, got {result:?}"
        );
    }

    #[test]
    fn convert_in_missing_carriers_fails() {
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::In {
            carriers: None,
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "carriers", .. })),
            "expected MissingField{{carriers}}, got {result:?}"
        );
    }

    #[test]
    fn convert_out_missing_carriers_fails() {
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Out {
            carriers: None,
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "carriers", .. })),
            "expected MissingField{{carriers}}, got {result:?}"
        );
    }

    #[test]
    fn convert_knit_missing_needle_fails() {
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Knit {
            direction: WeakDirection::Right,
            needle: None,
            carriers: vec![],
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "needle", .. })),
            "expected MissingField{{needle}}, got {result:?}"
        );
    }

    #[test]
    fn convert_tuck_missing_needle_fails() {
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Tuck {
            direction: WeakDirection::Left,
            needle: None,
            carriers: vec![],
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "needle", .. })),
            "expected MissingField{{needle}}, got {result:?}"
        );
    }

    #[test]
    fn convert_xfer_missing_source_fails() {
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Xfer {
            source: None,
            destination: Some(make_needle(WeakBed::Back, 5)),
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "source", .. })),
            "expected MissingField{{source}}, got {result:?}"
        );
    }

    #[test]
    fn convert_xfer_missing_destination_fails() {
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Xfer {
            source: Some(make_needle(WeakBed::Front, 5)),
            destination: None,
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "destination", .. })),
            "expected MissingField{{destination}}, got {result:?}"
        );
    }

    #[test]
    fn convert_drop_missing_needle_fails() {
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Drop {
            needle: None,
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "needle", .. })),
            "expected MissingField{{needle}}, got {result:?}"
        );
    }

    #[test]
    fn convert_knit_unknown_direction_fails() {
        let span = Span::new(0, 1);
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Knit {
            direction: WeakDirection::Unknown(span),
            needle: Some(make_needle(WeakBed::Front, 3)),
            carriers: vec![],
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::UnrecognizedDirection(_))),
            "expected UnrecognizedDirection, got {result:?}"
        );
    }

    #[test]
    fn convert_knit_unknown_bed_fails() {
        let span = Span::new(0, 1);
        let prog = make_prog(vec![make_line(make_base(WeakBaseOperationKind::Knit {
            direction: WeakDirection::Right,
            needle: Some(WeakNeedle {
                span,
                bed: WeakBed::Unknown(span),
                is_slider: IsSlider::NotSlider,
                index: Some(3),
            }),
            carriers: vec![],
        }))]);
        let result = prog.try_into_strong();
        assert!(
            matches!(result, Err(ParseError::UnrecognizedBed(_))),
            "expected UnrecognizedBed, got {result:?}"
        );
    }

    #[test]
    fn convert_needle_missing_index_fails() {
        let span = Span::new(0, 2);
        let result = WeakNeedle {
            span,
            bed: WeakBed::Front,
            is_slider: IsSlider::NotSlider,
            index: None,
        }
        .try_into_strong();
        assert!(
            matches!(result, Err(ParseError::MissingField { field: "index", .. })),
            "expected MissingField{{index}}, got {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Lowering: xfer_style (catches delete-arm mutants 144-145)
    // -----------------------------------------------------------------------

    #[test]
    fn lower_xfer_style_four_pass() {
        let source = ";!knitout-2\nx-xfer-style four-pass\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
        match prog.lines[0].operation.as_ref().unwrap() {
            WeakOperation::Extension(e) => match &e.kind {
                WeakExtensionOperationKind::XXferStyle { style } => {
                    assert_eq!(*style, WeakXferStyle::FourPass);
                }
                other => panic!("expected XXferStyle, got {other:?}"),
            },
            other => panic!("expected Extension, got {other:?}"),
        }
    }

    #[test]
    fn lower_xfer_style_two_pass() {
        let source = ";!knitout-2\nx-xfer-style two-pass\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
        match prog.lines[0].operation.as_ref().unwrap() {
            WeakOperation::Extension(e) => match &e.kind {
                WeakExtensionOperationKind::XXferStyle { style } => {
                    assert_eq!(*style, WeakXferStyle::TwoPass);
                }
                other => panic!("expected XXferStyle, got {other:?}"),
            },
            other => panic!("expected Extension, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Lowering: position header values (catches delete-arm mutants 157-159)
    // -----------------------------------------------------------------------

    #[test]
    fn lower_position_center() {
        let source = ";!knitout-2\n;;Position: Center\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
        match &prog.headers[0].kind {
            WeakHeaderKind::Position(ph) => assert_eq!(ph.value, WeakPositionValue::Center),
            other => panic!("expected Position header, got {other:?}"),
        }
    }

    #[test]
    fn lower_position_right() {
        let source = ";!knitout-2\n;;Position: Right\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
        match &prog.headers[0].kind {
            WeakHeaderKind::Position(ph) => assert_eq!(ph.value, WeakPositionValue::Right),
            other => panic!("expected Position header, got {other:?}"),
        }
    }

    #[test]
    fn lower_position_keep() {
        let source = ";!knitout-2\n;;Position: Keep\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
        match &prog.headers[0].kind {
            WeakHeaderKind::Position(ph) => assert_eq!(ph.value, WeakPositionValue::Keep),
            other => panic!("expected Position header, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Lowering: comment text (catches ! deletion mutants 624:25 and 624:44)
    // -----------------------------------------------------------------------

    #[test]
    fn lower_comment_text_extracted() {
        // The comment text is " hello" — with guards deleted, valid nodes would
        // be filtered out and text would become None.
        let source = ";!knitout-2\nknit + f5 ; hello\n";
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
        let line = &prog.lines[0];
        let comment = line.comment.as_ref().expect("expected comment");
        assert!(
            comment.text.is_some(),
            "expected comment text to be Some, got None"
        );
    }

    // -----------------------------------------------------------------------
    // Lowering: magic with MISSING version (catches is_missing guard 377)
    // -----------------------------------------------------------------------

    #[test]
    fn lower_magic_missing_version_produces_missing_version_diagnostic() {
        // ";!knitout-" followed immediately by newline — version token is MISSING.
        // With is_missing→false mutation, MISSING version flows to the parse arm,
        // text_of(MISSING) = "" → fails → InvalidVersion pushed instead of MissingVersion.
        let source = ";!knitout-\n";
        let (prog, diags) = parse(source);
        assert!(prog.magic.version.is_none(), "version should be None");
        assert!(
            diags.iter().any(|d| matches!(d, ParseError::MissingVersion(_))),
            "expected MissingVersion diagnostic, got: {diags:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Lowering: extension operations (catches delete-arm mutants 788-881)
    // -----------------------------------------------------------------------

    fn get_extension_kind(source: &str) -> WeakExtensionOperationKind {
        let (prog, diags) = parse(source);
        assert!(diags.is_empty(), "unexpected diags for {source:?}: {diags:?}");
        match prog.lines[0].operation.as_ref().expect("expected operation") {
            WeakOperation::Extension(e) => e.kind.clone(),
            other => panic!("expected Extension, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_stitch_number() {
        match get_extension_kind(";!knitout-2\nx-stitch-number 5\n") {
            WeakExtensionOperationKind::XStitchNumber { value } => {
                assert_eq!(value, Some(5));
            }
            other => panic!("expected XStitchNumber, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_speed_number() {
        match get_extension_kind(";!knitout-2\nx-speed-number 3\n") {
            WeakExtensionOperationKind::XSpeedNumber { value } => {
                assert_eq!(value, Some(3));
            }
            other => panic!("expected XSpeedNumber, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_sub_roller_number() {
        match get_extension_kind(";!knitout-2\nx-sub-roller-number 2\n") {
            WeakExtensionOperationKind::XSubRollerNumber { value } => {
                assert_eq!(value, Some(2));
            }
            other => panic!("expected XSubRollerNumber, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_xfer_stitch_number() {
        match get_extension_kind(";!knitout-2\nx-xfer-stitch-number 4\n") {
            WeakExtensionOperationKind::XXferStitchNumber { value } => {
                assert_eq!(value, Some(4));
            }
            other => panic!("expected XXferStitchNumber, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_end_pass() {
        match get_extension_kind(";!knitout-2\nx-end-pass\n") {
            WeakExtensionOperationKind::XEndPass => {}
            other => panic!("expected XEndPass, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_vis_color() {
        match get_extension_kind(";!knitout-2\nx-vis-color 0xff0000 1\n") {
            WeakExtensionOperationKind::XVisColor { color, carriers } => {
                assert_eq!(color.as_deref(), Some("0xff0000"));
                assert!(carriers.is_some());
            }
            other => panic!("expected XVisColor, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_loop_in() {
        match get_extension_kind(";!knitout-2\nx-loop-in + f3 1\n") {
            WeakExtensionOperationKind::XLoopIn {
                direction,
                needle,
                carriers,
            } => {
                assert_eq!(direction, WeakDirection::Right);
                assert!(needle.is_some());
                assert!(carriers.is_some());
            }
            other => panic!("expected XLoopIn, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_yarn_in() {
        match get_extension_kind(";!knitout-2\nx-yarn-in + f3 1\n") {
            WeakExtensionOperationKind::XYarnIn {
                direction,
                needle,
                carriers,
            } => {
                assert_eq!(direction, WeakDirection::Right);
                assert!(needle.is_some());
                assert!(carriers.is_some());
            }
            other => panic!("expected XYarnIn, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_yarn_out() {
        match get_extension_kind(";!knitout-2\nx-yarn-out + f3 1\n") {
            WeakExtensionOperationKind::XYarnOut {
                direction,
                needle,
                carriers,
            } => {
                assert_eq!(direction, WeakDirection::Right);
                assert!(needle.is_some());
                assert!(carriers.is_some());
            }
            other => panic!("expected XYarnOut, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_end() {
        match get_extension_kind(";!knitout-2\nx-end\n") {
            WeakExtensionOperationKind::XEnd => {}
            other => panic!("expected XEnd, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_roller_advance() {
        match get_extension_kind(";!knitout-2\nx-roller-advance 1.5\n") {
            WeakExtensionOperationKind::XRollerAdvance { value } => {
                assert!(value.is_some());
            }
            other => panic!("expected XRollerAdvance, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_add_roller_advance() {
        match get_extension_kind(";!knitout-2\nx-add-roller-advance 0.5\n") {
            WeakExtensionOperationKind::XAddRollerAdvance { value } => {
                assert!(value.is_some());
            }
            other => panic!("expected XAddRollerAdvance, got {other:?}"),
        }
    }

    #[test]
    fn lower_x_carrier_spacing() {
        match get_extension_kind(";!knitout-2\nx-carrier-spacing 5\n") {
            WeakExtensionOperationKind::XCarrierSpacing { value } => {
                assert_eq!(value, Some(5));
            }
            other => panic!("expected XCarrierSpacing, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Lowering: unknown_extension name and arguments (catches 888-890)
    // -----------------------------------------------------------------------

    #[test]
    fn lower_unknown_extension_name_extracted() {
        // Name must be distinct from all known x- extensions.
        match get_extension_kind(";!knitout-2\nx-my-custom arg1\n") {
            WeakExtensionOperationKind::UnknownExtension { name, .. } => {
                assert_eq!(name.as_deref(), Some("x-my-custom"));
            }
            other => panic!("expected UnknownExtension, got {other:?}"),
        }
    }

    #[test]
    fn lower_unknown_extension_arguments_extracted() {
        // With kind()=="extension_argument" → kind()!="extension_argument" mutation:
        // the name node (kind "extension_name") would be included as an argument instead.
        match get_extension_kind(";!knitout-2\nx-my-custom arg1 arg2\n") {
            WeakExtensionOperationKind::UnknownExtension { arguments, .. } => {
                assert_eq!(
                    arguments.as_slice(),
                    &["arg1", "arg2"],
                    "arguments should be [arg1, arg2], got {arguments:?}"
                );
            }
            other => panic!("expected UnknownExtension, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Regression / smoke tests
    // -----------------------------------------------------------------------

    #[test]
    fn conversion_fails_on_incomplete_weak_ast() {
        let source = ";!knitout-2\nknit\n";
        let (prog, _diags) = parse(source);
        let _ = prog.try_into_strong();
    }
}
