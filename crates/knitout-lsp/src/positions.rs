//! Conversions between byte offsets (used by knitout spans and tree-sitter) and LSP
//! [`Position`]s (zero-based line + UTF-16 character offset).

use tower_lsp::lsp_types::{Position, Range};

use knitout_weak_ast::Span;

/// Convert a byte offset into an LSP [`Position`] (zero-based line, UTF-16 character offset).
pub fn offset_to_position(text: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    let mut idx = 0usize;
    for ch in text.chars() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += ch.len_utf16() as u32;
        }
        idx += ch.len_utf8();
    }
    Position { line, character }
}

/// Convert an LSP [`Position`] into a byte offset into `text`. A position past the end of a line
/// clamps to the line's newline; a position past the end of the document clamps to its length.
pub fn position_to_offset(text: &str, position: Position) -> usize {
    let mut line = 0u32;
    let mut character = 0u32;
    for (byte, ch) in text.char_indices() {
        if line == position.line && character >= position.character {
            return byte;
        }
        if line > position.line {
            return byte;
        }
        if ch == '\n' {
            if line == position.line {
                return byte;
            }
            line += 1;
            character = 0;
        } else {
            character += ch.len_utf16() as u32;
        }
    }
    text.len()
}

/// Convert a byte span into an LSP range.
pub fn span_to_range(text: &str, span: Span) -> Range {
    Range {
        start: offset_to_position(text, span.start()),
        end: offset_to_position(text, span.end()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_to_position_handles_lines_and_utf16() {
        let text = "ab\ncd\n";
        assert_eq!(offset_to_position(text, 0), Position::new(0, 0));
        assert_eq!(offset_to_position(text, 1), Position::new(0, 1));
        assert_eq!(offset_to_position(text, 3), Position::new(1, 0));
        assert_eq!(offset_to_position(text, 4), Position::new(1, 1));

        // A non-BMP scalar (👍, 4 UTF-8 bytes, 2 UTF-16 code units) advances the character column
        // by 2, and the following byte offset maps past it.
        let emoji = "👍x";
        assert_eq!(offset_to_position(emoji, 0), Position::new(0, 0));
        assert_eq!(offset_to_position(emoji, 4), Position::new(0, 2));
    }

    #[test]
    fn position_to_offset_is_inverse_on_ascii() {
        let text = "knit + f10\n;;Carriers: 1\n";
        for offset in 0..=text.len() {
            // Only round-trip offsets that fall on char boundaries.
            if text.is_char_boundary(offset) {
                let pos = offset_to_position(text, offset);
                assert_eq!(position_to_offset(text, pos), offset, "offset {offset}");
            }
        }
    }

    #[test]
    fn position_to_offset_handles_utf16() {
        let text = "👍x\n";
        // The `x` is at UTF-16 character 2 on line 0, byte offset 4.
        assert_eq!(position_to_offset(text, Position::new(0, 2)), 4);
    }

    #[test]
    fn position_past_line_end_clamps_to_newline() {
        let text = "ab\ncd\n";
        assert_eq!(position_to_offset(text, Position::new(0, 99)), 2);
    }
}
