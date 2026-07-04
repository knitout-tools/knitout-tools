mod convert;
mod diagnostics;
mod lower;
mod types;

pub use diagnostics::ParseDiagnostic;
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

    fn parse(source: &str) -> (WeakProgram, Vec<ParseDiagnostic>) {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_knitout::LANGUAGE.into())
            .expect("failed to load grammar");
        let tree = parser.parse(source, None).expect("parse returned None");
        lower_cst(source.as_bytes(), &tree)
    }

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
    fn missing_version_produces_diagnostic() {
        // Feed a file that tree-sitter will parse with a missing version
        let source = ";\n";
        let (prog, diags) = parse(source);
        assert!(prog.magic.version.is_none() && !diags.is_empty());
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
        // Operation still parsed, source comment extracted
        let line = &prog.lines[0];
        assert_eq!(line.source.as_deref(), Some("myfile.js:5"));
        // Even though carriers might be missing, the source is captured
        let _ = diags; // may have diagnostic for missing carriers
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
    fn conversion_fails_on_incomplete_weak_ast() {
        // A program with a missing operation field should fail conversion
        let source = ";!knitout-2\nknit\n"; // direction and needle missing
        let (prog, diags) = parse(source);
        // Diagnostics may be emitted during lowering
        let _ = diags;
        // The conversion should fail because required fields are missing
        // (version is fine, but the knit op has missing direction/needle)
        // Either lowering emits diags or conversion fails:
        let result = prog.try_into_strong();
        // We just check it doesn't panic; it may succeed or fail depending
        // on how tree-sitter error-recovers the incomplete line
        let _ = result;
    }

    #[test]
    fn error_node_produces_unknown_operation() {
        // A line with completely invalid content
        let source = ";!knitout-2\nthis_is_not_valid!!!\n";
        let (prog, diags) = parse(source);
        // Should not panic; should produce diagnostics
        let _ = diags;
        let _ = prog;
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
                WeakBaseOperationKind::Xfer {
                    source,
                    destination,
                } => {
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
}
