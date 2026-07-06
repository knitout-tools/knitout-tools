//! Hover support.
//!
//! Given a cursor position, decide what knitout construct is under it and produce hover text from
//! the [`crate::docs`] module. Operations, needles, and directions are located from the parsed AST
//! (whose nodes carry byte spans); headers and the magic version line are located from the
//! tree-sitter CST (whose coarse node kinds distinguish those line types directly).

use either::Either;

use knitout_ast::{BaseOperation, Direction, ExtensionOperation, Needle, Operation, Program, Span};
use knitout_weak_ast::Parsed;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};
use tree_sitter::{Node, Tree};

use crate::docs;
use crate::positions::{position_to_offset, span_to_range};

/// Compute hover content for `position` in `text`, using the parse `tree` and parsed `program`.
pub fn compute_hover(
    text: &str,
    tree: &Tree,
    program: &Program<Parsed>,
    position: Position,
) -> Option<Hover> {
    let offset = position_to_offset(text, position);

    // Operations, needles, and directions come from the AST.
    if let Some((markdown, range)) = hover_in_operation(text, program, offset) {
        return Some(make_hover(markdown, range, text));
    }

    // Headers and the magic line come from the CST.
    if let Some((markdown, range)) = hover_from_cst(text, tree, offset) {
        return Some(make_hover(markdown, range, text));
    }

    None
}

/// If `offset` is inside an operation, produce hover for the needle/direction under it, or the
/// operation itself otherwise. Returns the Markdown and the byte span to highlight.
fn hover_in_operation(
    text: &str,
    program: &Program<Parsed>,
    offset: usize,
) -> Option<(String, Span)> {
    for annotated in program.operations() {
        let op_span = annotated.annotation.0;
        if !span_contains(op_span, offset) {
            continue;
        }
        let op = &annotated.value;

        // A needle under the cursor wins over the operation.
        for needle_span in needle_spans(op) {
            if span_contains(needle_span, offset) {
                return Some((docs::render(&docs::needle_doc()), needle_span));
            }
        }
        // Then a direction.
        for (dir_span, direction) in direction_spans(op) {
            if span_contains(dir_span, offset) {
                return Some((docs::render(&docs::direction_doc(direction)), dir_span));
            }
        }

        // Otherwise (opcode, a number, a carrier, whitespace…) show the operation's docs. Fall
        // back to the opcode text from the source when the operation could not be fully parsed
        // (e.g. it collapsed to `Other` because of too many arguments).
        let doc = docs::operation_doc(op)
            .or_else(|| docs::operation_doc_by_opcode(leading_token(text, op_span)))?;
        return Some((docs::render(&doc), op_span));
    }
    None
}

/// If `offset` is inside a header line or the magic version line, produce the corresponding hover.
fn hover_from_cst(text: &str, tree: &Tree, offset: usize) -> Option<(String, Span)> {
    let node = tree.root_node().descendant_for_byte_range(offset, offset)?;
    let target = ancestor_of_kind(node, &["header", "tag_comment"])?;
    let span = Span::new(target.start_byte(), target.end_byte());
    let node_text = target.utf8_text(text.as_bytes()).ok()?;

    match target.kind() {
        "header" => {
            // Strip the leading ";;" and take the name up to the first ':'.
            let body = node_text.strip_prefix(";;").unwrap_or(node_text);
            let name = body.split(':').next().unwrap_or("");
            let kind = docs::header_kind_from_name(name);
            Some((docs::render(&docs::header_doc(kind)), span))
        }
        "tag_comment" => {
            // Only the ";!knitout-N" magic line has hover docs; other ";!" tags do not.
            let body = node_text.strip_prefix(";!").unwrap_or(node_text);
            if body.trim_start().starts_with("knitout-") {
                Some((docs::render(&docs::magic_doc()), span))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn make_hover(markdown: String, span: Span, text: &str) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(span_to_range(text, span)),
    }
}

fn span_contains(span: Span, offset: usize) -> bool {
    span.start() <= offset && offset < span.end()
}

/// The first whitespace-delimited token of the operation's source (its opcode).
fn leading_token(text: &str, op_span: Span) -> &str {
    text.get(op_span.start()..op_span.end())
        .unwrap_or("")
        .split_whitespace()
        .next()
        .unwrap_or("")
}

/// Walk up from `node` to the nearest ancestor whose kind is in `kinds`.
fn ancestor_of_kind<'t>(node: Node<'t>, kinds: &[&str]) -> Option<Node<'t>> {
    let mut current = Some(node);
    while let Some(n) = current {
        if kinds.contains(&n.kind()) {
            return Some(n);
        }
        current = n.parent();
    }
    None
}

/// The byte spans of every needle argument in an operation.
fn needle_spans(op: &Operation<Parsed>) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut push = |needle: &Either<Needle<Parsed>, knitout_weak_ast::ParseError>| {
        if let Either::Left(needle) = needle {
            spans.push(*needle.annotation());
        }
    };
    match op {
        Operation::BaseOperation(base) => match base {
            BaseOperation::Knit { needle, .. }
            | BaseOperation::Tuck { needle, .. }
            | BaseOperation::Miss { needle, .. }
            | BaseOperation::Drop { needle }
            | BaseOperation::Amiss { needle } => push(needle),
            BaseOperation::Split {
                source,
                destination,
                ..
            }
            | BaseOperation::Xfer {
                source,
                destination,
            } => {
                push(source);
                push(destination);
            }
            _ => {}
        },
        Operation::ExtensionOperation(ext) => match ext {
            ExtensionOperation::XLoopIn { needle, .. }
            | ExtensionOperation::XYarnIn { needle, .. }
            | ExtensionOperation::XYarnOut { needle, .. } => push(needle),
            _ => {}
        },
        Operation::Other(_) => {}
    }
    spans
}

/// The byte spans of every direction argument in an operation, paired with the direction itself.
fn direction_spans(op: &Operation<Parsed>) -> Vec<(Span, &Direction<Parsed>)> {
    let dir = |d: &Direction<Parsed>| direction_span(d);
    match op {
        Operation::BaseOperation(base) => match base {
            BaseOperation::Knit { direction, .. }
            | BaseOperation::Tuck { direction, .. }
            | BaseOperation::Miss { direction, .. }
            | BaseOperation::Split { direction, .. } => vec![(dir(direction), direction)],
            _ => Vec::new(),
        },
        Operation::ExtensionOperation(ext) => match ext {
            ExtensionOperation::XLoopIn { direction, .. }
            | ExtensionOperation::XYarnIn { direction, .. }
            | ExtensionOperation::XYarnOut { direction, .. } => vec![(dir(direction), direction)],
            _ => Vec::new(),
        },
        Operation::Other(_) => Vec::new(),
    }
}

fn direction_span(direction: &Direction<Parsed>) -> Span {
    match direction {
        Direction::Left(span) | Direction::Right(span) | Direction::Other(span, _) => *span,
    }
}
