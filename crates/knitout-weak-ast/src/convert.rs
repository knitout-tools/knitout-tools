//! Phase-transformation utilities for knitout programs.
//!
//! `ParsedToValidated` is the canonical `PhaseTransform` implementation that drops all `Parsed`
//! span annotations and produces a `Program<Validated>`, which is what `parsed_to_valid` uses.
//!
//! The `PhaseTransform` trait and the `transform_program` function live in `knitout_ast`.

use knitout_ast::{PhaseTransform, Program, Provenance, Span, Validated, transform_program};

use crate::{CommentInfo, Parsed};

// ─── Parsed → Validated ──────────────────────────────────────────────────────

/// The canonical `PhaseTransform` from `Parsed` to `Validated`.  All span annotations are
/// dropped; all `Other(ParseError)` nodes cause the transformation to return `None`.
pub struct ParsedToValidated;

impl PhaseTransform<Parsed, Validated> for ParsedToValidated {
    fn transform_magic_version(_: (Span, Span)) {}
    fn transform_operation(_: (Span, CommentInfo)) -> Provenance {
        // TODO this is wrong! Should use both the span AND the ;!source comment
        Provenance::default()
    }
    fn transform_header(_: Span) {}
    fn transform_unknown_header(_: Span) {}
    fn transform_position_header_value(_: Span) {}
    fn transform_carriers_header_value(_: Span) {}
    fn transform_rail(_: Span) {}
    fn transform_rail_side(_: Span) {}
    fn transform_carrier(_: Span) {}
    fn transform_direction(_: Span) {}
    fn transform_bed(_: Span) {}
    fn transform_is_slider(_: Span) {}
    fn transform_index(_: Span) {}
    fn transform_needle(_: Span) {}
    fn transform_my_big_decimal(_: Span) {}
    fn transform_my_u32(_: Span) {}
    fn transform_my_i32(_: Span) {}
    fn transform_my_string(_: Span) {}
    fn transform_presser_mode(_: Span) {}
    fn transform_xfer_style(_: Span) {}
}

/// Convert a `Program<Parsed>` to `Option<Program<Validated>>`.
///
/// Returns `None` if any `Other(ParseError)` node is present in the parsed tree (indicating that
/// some portion of the source could not be understood).
pub fn parsed_to_valid(prog: Program<Parsed>) -> Option<Program<Validated>> {
    transform_program::<Parsed, Validated, ParsedToValidated>(prog)
}
