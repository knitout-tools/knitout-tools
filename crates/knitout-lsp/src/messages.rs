//! Centralized mapping from `knitout-weak-ast` diagnostic variants to human-readable messages.
//!
//! This is the single place to edit the wording of any diagnostic surfaced by the language
//! server. Every [`ParseWarning`] and [`ParseError`] variant maps to exactly one message here;
//! the [`crate::backend`] module turns those messages into LSP diagnostics.

use knitout_runtime::{RuntimeError, RuntimeWarning};
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

/// Human-readable message for a runtime warning (the program still runs).
pub fn runtime_warning_message(warning: &RuntimeWarning) -> String {
    match warning {
        RuntimeWarning::NoYarnCarrier { .. } => {
            "This knit/tuck/split has no carriers; use `drop`, `amiss`, or `xfer` instead."
                .to_string()
        }
        RuntimeWarning::MissWithNoYarn { .. } => {
            "A `miss` with no carriers does nothing.".to_string()
        }
        RuntimeWarning::KnittingThroughNothing { .. } => {
            "This knit acts on a needle that holds no loops (a `tuck` is usually meant)."
                .to_string()
        }
        RuntimeWarning::TransferringNothing { .. } => {
            "The source needle holds no loops, so this transfer moves nothing.".to_string()
        }
        RuntimeWarning::LongFloat { distance, .. } => {
            format!("This carrier makes a long float of {distance} needles.")
        }
        RuntimeWarning::MaxHoldLoops { count, .. } => {
            format!("{count} loops are stacked on one needle, which can break the needle.")
        }
        RuntimeWarning::TransferWithOverlappedLoops { count, .. } => {
            format!("A stack of {count} loops is being transferred at once.")
        }
        RuntimeWarning::ContinuousPickUpStitch { .. } => {
            "The same carrier makes a new loop on two adjacent needles, laddering the piece."
                .to_string()
        }
        RuntimeWarning::DropAfterPickUp { .. } => {
            "This knits off a loop the same carrier just tucked, in the opposite direction — the \
             same as a `drop` (a common cast-on mistake)."
                .to_string()
        }
        RuntimeWarning::UnnecessaryLoopRemain { .. } => {
            "This needle is still holding a loop at the end of the program.".to_string()
        }
        RuntimeWarning::OverstretchedFloat {
            birth_length,
            current_length,
            ..
        } => format!(
            "This float has stretched from {birth_length} to {current_length} (racking, transfers, \
             or age)."
        ),
        RuntimeWarning::ContinuousTransferWithoutKnitting { transfers, .. } => {
            format!("This loop has been transferred {transfers} times without being knit.")
        }
        RuntimeWarning::UnstableSplitKnit { split_throughs, .. } => {
            format!("This loop has been split-through {split_throughs} times, which is unstable.")
        }
        RuntimeWarning::TransferWithPickUpStitch { .. } => {
            "Transferring a loop that never pulled through another loop gives inconsistent results."
                .to_string()
        }
        RuntimeWarning::PulldownTakedownOverload { distance, .. } => {
            format!(
                "This loop is moved {distance} needles laterally, which can overload the take-down."
            )
        }
        RuntimeWarning::UnstableYarnFeeding { .. } => {
            "This carrier's first stitch feeds onto the bed opposite its Shima-Seiki home bed, \
             which is unstable."
                .to_string()
        }
        RuntimeWarning::UnstitchedCarrier { .. } => {
            "This carrier is taken out before it ever knitted, so its position is unknown."
                .to_string()
        }
        RuntimeWarning::CarrierLeftInAction { .. } => {
            "This carrier is never taken out (`out`/`outhook`).".to_string()
        }
        RuntimeWarning::HookLeftEngaged { .. } => {
            "The yarn inserting hook is never released (`releasehook`).".to_string()
        }
    }
}

/// Human-readable message for a runtime error (the program will not run as written).
pub fn runtime_error_message(error: &RuntimeError) -> String {
    match error {
        RuntimeError::UseOfInactiveCarrier { .. } => {
            "This carrier is used before it is brought in (`in`/`inhook`).".to_string()
        }
        RuntimeError::CarrierBroughtInTwice { .. } => {
            "This carrier is already in action.".to_string()
        }
        RuntimeError::HookAlreadyInUse { .. } => {
            "The yarn inserting hook is already holding carriers.".to_string()
        }
        RuntimeError::ReleaseWithoutHook { .. } => {
            "`releasehook` with no carriers held by the hook.".to_string()
        }
        RuntimeError::ReleaseOfWrongCarriers { .. } => {
            "`releasehook` names a different carrier set than the hook is holding.".to_string()
        }
        RuntimeError::TakeOutWhileHooked { .. } => {
            "This carrier is still held by the hook; release it first (`releasehook`/`outhook`)."
                .to_string()
        }
        RuntimeError::TransferAtWrongRacking { .. } => {
            "These needles do not line up at the current racking.".to_string()
        }
        RuntimeError::TransferWithWrongBeds { .. } => {
            "A transfer must be between opposite beds (`f`↔`b`, `f`↔`bs`, or `fs`↔`b`).".to_string()
        }
        RuntimeError::MachineTooWide {
            needles_used,
            machine_width,
            ..
        } => format!(
            "This program spans {needles_used} needles, but the machine is only {machine_width} wide."
        ),
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

    #[test]
    fn all_runtime_messages_are_nonempty_and_distinct() {
        let warnings = [
            RuntimeWarning::NoYarnCarrier { span: None },
            RuntimeWarning::MissWithNoYarn { span: None },
            RuntimeWarning::KnittingThroughNothing { span: None },
            RuntimeWarning::TransferringNothing { span: None },
            RuntimeWarning::LongFloat {
                from_span: None,
                to_span: None,
                distance: 8,
            },
            RuntimeWarning::MaxHoldLoops {
                span: None,
                count: 4,
            },
            RuntimeWarning::TransferWithOverlappedLoops {
                span: None,
                count: 3,
            },
            RuntimeWarning::ContinuousPickUpStitch {
                first_span: None,
                second_span: None,
            },
            RuntimeWarning::DropAfterPickUp {
                span: None,
                pickup_span: None,
            },
            RuntimeWarning::UnnecessaryLoopRemain { span: None },
            RuntimeWarning::OverstretchedFloat {
                span: None,
                birth_length: 1,
                current_length: 9,
            },
            RuntimeWarning::ContinuousTransferWithoutKnitting {
                span: None,
                transfers: 4,
            },
            RuntimeWarning::UnstableSplitKnit {
                span: None,
                split_throughs: 2,
            },
            RuntimeWarning::TransferWithPickUpStitch { span: None },
            RuntimeWarning::PulldownTakedownOverload {
                span: None,
                distance: 13,
            },
            RuntimeWarning::UnstableYarnFeeding { span: None },
            RuntimeWarning::UnstitchedCarrier { span: None },
            RuntimeWarning::CarrierLeftInAction { span: None },
            RuntimeWarning::HookLeftEngaged { span: None },
        ];
        let errors = [
            RuntimeError::UseOfInactiveCarrier { span: None },
            RuntimeError::CarrierBroughtInTwice { span: None },
            RuntimeError::HookAlreadyInUse { span: None },
            RuntimeError::ReleaseWithoutHook { span: None },
            RuntimeError::ReleaseOfWrongCarriers { span: None },
            RuntimeError::TakeOutWhileHooked { span: None },
            RuntimeError::TransferAtWrongRacking { span: None },
            RuntimeError::TransferWithWrongBeds { span: None },
            RuntimeError::MachineTooWide {
                span: None,
                needles_used: 6,
                machine_width: 1,
            },
        ];

        let mut messages: Vec<String> = warnings.iter().map(runtime_warning_message).collect();
        messages.extend(errors.iter().map(runtime_error_message));

        assert!(messages.iter().all(|m| !m.trim().is_empty()));
        let unique: std::collections::HashSet<&String> = messages.iter().collect();
        assert_eq!(
            unique.len(),
            messages.len(),
            "runtime messages must be distinct"
        );
    }

    #[test]
    fn machine_too_wide_message_includes_the_numbers() {
        let message = runtime_error_message(&RuntimeError::MachineTooWide {
            span: None,
            needles_used: 6,
            machine_width: 1,
        });
        assert!(message.contains('6') && message.contains('1'), "{message}");
    }
}
