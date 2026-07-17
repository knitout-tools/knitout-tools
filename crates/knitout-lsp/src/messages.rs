//! Centralized mapping from `knitout-weak-ast` diagnostic variants to human-readable messages.
//!
//! This is the single place to edit the wording of any diagnostic surfaced by the language
//! server. Every [`ParseWarning`] and [`ParseError`] variant maps to exactly one message here;
//! the [`crate::backend`] module turns those messages into LSP diagnostics.

use knitout_weak_ast::{ParseError, ParseWarning};

/// Human-readable message for a parse warning.
pub fn warning_message(warning: &ParseWarning) -> String {
    match warning {
        ParseWarning::MagicNotFirst(_) => {
            "The `;!knitout-N` version line should be the first non-blank line in the file."
                .to_string()
        }
        ParseWarning::DuplicateHeader { .. } => {
            "This header is defined more than once.".to_string()
        }
        ParseWarning::HeaderAfterOperations(_) => {
            "Header lines should appear before any operation lines.".to_string()
        }
        ParseWarning::UnrecognizedExtensionHeader(_) => {
            "Unrecognized extension header.".to_string()
        }
        ParseWarning::UnrecognizedExtension(_) => "Unrecognized extension operation.".to_string(),
    }
}

/// Human-readable message for a parse error.
pub fn error_message(error: &ParseError) -> String {
    match error {
        ParseError::MagicMissing => {
            "Missing `;!knitout-N` version line at the start of the file.".to_string()
        }
        ParseError::BadMagicVersion(_) => {
            "The knitout version must be a non-negative integer.".to_string()
        }
        ParseError::ConflictingMagicVersion { .. } => {
            "Conflicting `;!knitout-N` version lines.".to_string()
        }
        ParseError::ConflictingHeader { .. } => {
            "This header is defined more than once with different values.".to_string()
        }
        ParseError::MissingCarriersHeader => "Missing required `Carriers:` header.".to_string(),
        ParseError::UnrecognizedOpcode(_) => "Unrecognized operation.".to_string(),
        ParseError::MissingArgument(_) => "Missing a required argument.".to_string(),
        ParseError::BadDirection(_) => "Direction must be `+` or `-`.".to_string(),
        ParseError::BadNeedle(_) => {
            "Could not parse this needle (expected e.g. `f10`, `bs-3`).".to_string()
        }
        ParseError::BadNeedleIndex(_) => "The needle index must be an integer.".to_string(),
        ParseError::BadBed(_) => "The needle bed must be `f` or `b`.".to_string(),
        ParseError::BadU32(_) => "Expected a non-negative integer.".to_string(),
        ParseError::BadBigDecimal(_) => "Expected a decimal number.".to_string(),
        ParseError::BadPresserMode(_) => "Presser mode must be `off`, `auto`, or `on`.".to_string(),
        ParseError::BadXferStyle(_) => {
            "Transfer style must be `four-pass` or `two-pass`.".to_string()
        }
        ParseError::BadPositionValue(_) => {
            "Position must be `Left`, `Center`, `Right`, or `Keep`.".to_string()
        }
        ParseError::TooManyArguments(_) => "Too many arguments for this operation.".to_string(),
        ParseError::BadUtf8(_) => "The source could not be read as UTF-8.".to_string(),
        ParseError::BadCarrierName(_) => {
            "Illegal carrier name (must not be empty or contain whitespace, `;`, or `,`)."
                .to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knitout_weak_ast::Span;

    fn sp() -> Span {
        Span::new(0, 1)
    }

    /// Every variant maps to a distinct, non-empty message. This guards against a mapping that
    /// collapses to an empty string or returns the same text for every variant.
    #[test]
    fn all_messages_are_nonempty_and_distinct() {
        let warnings = [
            ParseWarning::MagicNotFirst(sp()),
            ParseWarning::DuplicateHeader {
                span: sp(),
                previous: sp(),
            },
            ParseWarning::HeaderAfterOperations(sp()),
            ParseWarning::UnrecognizedExtensionHeader(sp()),
            ParseWarning::UnrecognizedExtension(sp()),
        ];
        let errors = [
            ParseError::MagicMissing,
            ParseError::BadMagicVersion(sp()),
            ParseError::ConflictingMagicVersion {
                span: sp(),
                previous: sp(),
            },
            ParseError::ConflictingHeader {
                span: sp(),
                previous: sp(),
            },
            ParseError::MissingCarriersHeader,
            ParseError::UnrecognizedOpcode(sp()),
            ParseError::MissingArgument(sp()),
            ParseError::BadDirection(sp()),
            ParseError::BadNeedle(sp()),
            ParseError::BadNeedleIndex(sp()),
            ParseError::BadBed(sp()),
            ParseError::BadU32(sp()),
            ParseError::BadBigDecimal(sp()),
            ParseError::BadPresserMode(sp()),
            ParseError::BadXferStyle(sp()),
            ParseError::BadPositionValue(sp()),
            ParseError::TooManyArguments(sp()),
            ParseError::BadUtf8(sp()),
            ParseError::BadCarrierName(sp()),
        ];

        let mut messages: Vec<String> = warnings.iter().map(warning_message).collect();
        messages.extend(errors.iter().map(error_message));

        assert!(
            messages.iter().all(|m| !m.trim().is_empty()),
            "every diagnostic message must be non-empty"
        );

        let unique: std::collections::HashSet<&String> = messages.iter().collect();
        assert_eq!(
            unique.len(),
            messages.len(),
            "every diagnostic message must be distinct"
        );
    }
}
