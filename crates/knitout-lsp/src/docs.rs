//! Editable documentation for knitout constructs.
//!
//! This module is the single place to edit the human-readable text shown on hover and in
//! argument-count diagnostics. Each construct maps to a [`Doc`], whose fields are the separate
//! pieces of documentation (summary, syntax/signature, example, longer description); [`render`]
//! combines them into the Markdown shown in an editor.
//!
//! - Operations: [`operation_doc`] (from a parsed `Operation`) and [`operation_doc_by_opcode`]
//!   (from the opcode text, used when the operation could not be fully parsed).
//! - Needles: [`needle_doc`].
//! - Directions: [`direction_doc`].
//! - Headers: [`header_doc`] keyed by [`HeaderKind`].
//! - The magic version line: [`magic_doc`].

use knitout_ast::{BaseOperation, Direction, ExtensionOperation, Operation};
use knitout_weak_ast::Parsed;

/// A single construct's documentation. The fields are the separately-editable pieces that
/// [`render`] combines into the Markdown shown on hover.
#[derive(Clone, Copy, Debug)]
pub struct Doc {
    /// Short bold heading, e.g. `"knit"` or `"Needle"`.
    pub title: &'static str,
    /// One-line summary of what the construct does.
    pub summary: &'static str,
    /// The signature / representative syntax, e.g. `"knit <direction> <needle> [carriers…]"`.
    pub syntax: &'static str,
    /// A representative concrete example, e.g. `"knit + f10 3"`.
    pub example: Option<&'static str>,
    /// A longer description.
    pub description: Option<&'static str>,
}

/// Combine a [`Doc`]'s pieces into the Markdown string shown on hover.
pub fn render(doc: &Doc) -> String {
    let mut out = format!("**{}** — {}\n\n", doc.title, doc.summary);
    out.push_str("Signature:\n```knitout\n");
    out.push_str(doc.syntax);
    out.push_str("\n```\n");
    if let Some(example) = doc.example {
        out.push_str("Example:\n```knitout\n");
        out.push_str(example);
        out.push_str("\n```\n");
    }
    if let Some(description) = doc.description {
        out.push_str("\n\n");
        out.push_str(description);
    }
    out
}

/// A one-line hint (signature + example) for argument-count diagnostics. Returns `None` for an
/// unrecognized opcode.
pub fn operation_arity_hint(opcode: &str) -> Option<String> {
    let doc = operation_doc_by_opcode(opcode)?;
    Some(match doc.example {
        Some(example) => format!("Signature: `{}`\nExample: `{}`", doc.syntax, example),
        None => format!("Signature: `{}`", doc.syntax),
    })
}

/// Documentation for a parsed operation. Returns `None` for `Operation::Other` and for unknown
/// extensions (which have no fixed signature).
pub fn operation_doc(op: &Operation<Parsed>) -> Option<Doc> {
    opcode_of(op).and_then(operation_doc_by_opcode)
}

/// The opcode string for a parsed operation, or `None` if it has none (an unresolved `Other` node,
/// or an unknown `x-…` extension).
fn opcode_of(op: &Operation<Parsed>) -> Option<&'static str> {
    match op {
        Operation::BaseOperation(base) => Some(match base {
            BaseOperation::In { .. } => "in",
            BaseOperation::Inhook { .. } => "inhook",
            BaseOperation::Releasehook { .. } => "releasehook",
            BaseOperation::Out { .. } => "out",
            BaseOperation::Outhook { .. } => "outhook",
            BaseOperation::Stitch { .. } => "stitch",
            BaseOperation::Rack { .. } => "rack",
            BaseOperation::Knit { .. } => "knit",
            BaseOperation::Tuck { .. } => "tuck",
            BaseOperation::Miss { .. } => "miss",
            BaseOperation::Split { .. } => "split",
            BaseOperation::Pause => "pause",
            BaseOperation::Drop { .. } => "drop",
            BaseOperation::Amiss { .. } => "amiss",
            BaseOperation::Xfer { .. } => "xfer",
        }),
        Operation::ExtensionOperation(ext) => match ext {
            ExtensionOperation::XStitchNumber { .. } => Some("x-stitch-number"),
            ExtensionOperation::XSpeedNumber { .. } => Some("x-speed-number"),
            ExtensionOperation::XSubRollerNumber { .. } => Some("x-sub-roller-number"),
            ExtensionOperation::XPresserMode { .. } => Some("x-presser-mode"),
            ExtensionOperation::XXferStitchNumber { .. } => Some("x-xfer-stitch-number"),
            ExtensionOperation::XEndPass => Some("x-end-pass"),
            ExtensionOperation::XVisColor { .. } => Some("x-vis-color"),
            ExtensionOperation::XLoopIn { .. } => Some("x-loop-in"),
            ExtensionOperation::XYarnIn { .. } => Some("x-yarn-in"),
            ExtensionOperation::XYarnOut { .. } => Some("x-yarn-out"),
            ExtensionOperation::XEnd => Some("x-end"),
            ExtensionOperation::XRollerAdvance { .. } => Some("x-roller-advance"),
            ExtensionOperation::XAddRollerAdvance { .. } => Some("x-add-roller-advance"),
            ExtensionOperation::XCarrierSpacing { .. } => Some("x-carrier-spacing"),
            ExtensionOperation::XCarrierStoppingDistance { .. } => {
                Some("x-carrier-stopping-distance")
            }
            ExtensionOperation::XXferStyle { .. } => Some("x-xfer-style"),
            ExtensionOperation::XLoopStart { .. } => Some("x-loop-start"),
            ExtensionOperation::XLoopEnd { .. } => Some("x-loop-end"),
            ExtensionOperation::UnknownExtension { .. } => None,
        },
        Operation::Other(_) => None,
    }
}

/// Documentation for an operation keyed by its opcode text. This is the single place to edit the
/// per-operation documentation. Returns `None` for unrecognized opcodes.
pub fn operation_doc_by_opcode(opcode: &str) -> Option<Doc> {
    let doc = match opcode {
        // ── carrier-list operations ───────────────────────────────────────────
        "in" => Doc {
            title: "in",
            summary: "Bring carriers into action using the yarn grippers.",
            syntax: "in <carriers…>",
            example: Some("in 3 2"),
            description: Some(
                "Marks the given carriers as in use without using the yarn inserting hook. Pair with `out` or `outhook` to remove them.",
            ),
        },
        "inhook" => Doc {
            title: "inhook",
            summary: "Bring carriers into action using the yarn inserting hook.",
            syntax: "inhook <carriers…>",
            example: Some("inhook 3 2"),
            description: Some(
                "Like `in`, but holds the yarn(s) with the inserting hook until a later `releasehook`. The inserting hook will be parked just before the first stitch made with the carriers.",
            ),
        },
        "releasehook" => Doc {
            title: "releasehook",
            summary: "Release the yarn inserting hook.",
            syntax: "releasehook <carriers…>",
            example: Some("releasehook 3 2"),
            description: Some(
                "Releases the carriers previously brought in with `inhook`, freeing the inserting
                hook. Must be after an `inhook` for the same carriers.",
            ),
        },
        "out" => Doc {
            title: "out",
            summary: "Take carriers out of action by moving them to the yarn grippers.",
            syntax: "out <carriers…>",
            example: Some("out 3 2"),
            description: Some(
                "Marks the given carriers as no longer in use, without the yarn inserting hook. Can undo an `in` or an `inhook` (as long as the hook has been released).",
            ),
        },
        "outhook" => Doc {
            title: "outhook",
            summary: "Take carriers out using the yarn inserting hook.",
            syntax: "outhook <carriers…>",
            example: Some("outhook 3"),
            description: Some(
                "Like `out`, but uses the yarn inserting hook to hold and cut the yarn. The inserting hook must not be in use when this is called.",
            ),
        },

        // ── stitch / rack ─────────────────────────────────────────────────────
        "stitch" => Doc {
            title: "stitch",
            summary: "Set the stitch (loop-length) values.",
            syntax: "stitch <before> <after>",
            example: Some("stitch 5 10"),
            description: Some(
                "Before forming a loop, pull needle by `before` units; after forming the loop, pull by `after` units. The units are machine-specific.",
            ),
        },
        "rack" => Doc {
            title: "rack",
            summary: "Set the racking offset between the beds.",
            syntax: "rack <offset>",
            example: Some("rack -2"),
            description: Some(
                "Aligns the back bed relative to the front bed by the given number of needles. Positive racking values move the back bed right. Fractional offset, if supported by the machine, enable all-needle knitting.",
            ),
        },

        // ── direction + needle + carriers ─────────────────────────────────────
        "knit" => Doc {
            title: "knit",
            summary: "Knit through the loops on the needle, dropping any existing loops on the needle.",
            syntax: "knit <direction> <needle> [carriers…]",
            example: Some("knit + f-10 3 9"),
            description: Some(
                "Pulls the carried yarns through all loops on the needle, forming a new loop and dropping the old ones. With no carriers, behaves like `drop`. The needle bed controls whether this is a knit or a purl. The direction controls the direction the carrier moves past the old loops.",
            ),
        },
        "tuck" => Doc {
            title: "tuck",
            summary: "Drape loops without clearing existing loops.",
            syntax: "tuck <direction> <needle> [carriers…]",
            example: Some("tuck + f4 3 2"),
            description: Some(
                "Flows the carried yarn over the needle, preserving any loops already held there. Used at the start of a program to form loops in a cast-on.",
            ),
        },
        "miss" => Doc {
            title: "miss",
            summary: "Move carriers past a needle without knitting.",
            syntax: "miss <direction> <needle> [carriers…]",
            example: Some("miss - f10 3 2"),
            description: Some(
                "Repositions the given carriers to the needle's location without forming a loop. The knit structure is unchanged.",
            ),
        },
        "split" => Doc {
            title: "split",
            summary: "Knit through the first loop, transferring old loops to the opposite bed.",
            syntax: "split <direction> <source> <destination> [carriers…]",
            example: Some("split + f10 b10 3 2"),
            description: Some(
                "Forms a new loop on the source needle and transfers its previous loops to the destination needle. The source and destination needles must be on opposite beds and they must be aligned via racking.",
            ),
        },

        // ── no-arg / single-needle ─────────────────────────────────────────────
        "pause" => Doc {
            title: "pause",
            summary: "Pause the machine.",
            syntax: "pause",
            example: None,
            description: Some(
                "Requests that the machine pauses at this point for operator intervention. Can be useful for changing yarns or for debugging.",
            ),
        },
        "drop" => Doc {
            title: "drop",
            summary: "Drop the loops on a needle.",
            syntax: "drop <needle>",
            example: Some("drop f10"),
            description: Some(
                "Releases all loops held on the needle. Equivalent to a `knit` with no carriers.",
            ),
        },
        "amiss" => Doc {
            title: "amiss",
            summary: "Equivalent to `miss` without carriers.",
            syntax: "amiss <needle>",
            example: Some("amiss b-10"),
            description: None, // TODO: what is amiss useful for?
        },
        "xfer" => Doc {
            title: "xfer",
            summary: "Transfer loops across the beds.",
            syntax: "xfer <source> <destination>",
            example: Some("xfer f10 b10"),
            description: Some(
                "Moves all loops on the source needle to the destination needle. No carriers are involved. The source and destination needles must be on opposite beds and they must be aligned via racking.",
            ),
        },

        // ── extension operations ───────────────────────────────────────────────
        "x-stitch-number" => Doc {
            title: "x-stitch-number",
            summary: "Select a machine stitch-number setting.",
            syntax: "x-stitch-number <n>",
            example: Some("x-stitch-number 5"),
            description: Some(
                "Selects a stitch-number entry from the machine's stitch table. An alternative to `stitch`.",
            ),
        },
        "x-speed-number" => Doc {
            title: "x-speed-number",
            summary: "Select a machine speed setting.",
            syntax: "x-speed-number <n>",
            example: Some("x-speed-number 3"),
            description: Some("Selects a speed entry from the machine's table."),
        },
        "x-sub-roller-number" => Doc {
            title: "x-sub-roller-number",
            summary: "Select a machine sub-roller setting.",
            syntax: "x-sub-roller-number <n>",
            example: Some("x-sub-roller-number 2"),
            description: Some("Selects a sub-roller entry from the machine's table."),
        },
        "x-presser-mode" => Doc {
            title: "x-presser-mode",
            summary: "Set the fabric presser mode.",
            syntax: "x-presser-mode <off|auto|on>",
            example: Some("x-presser-mode auto"),
            description: Some(
                "Controls the fabric presser. `auto` runs the presser whenever possible, whereas `on` fails to compile if the presser cannot be used at any point.",
            ),
        },
        "x-xfer-stitch-number" => Doc {
            title: "x-xfer-stitch-number",
            summary: "Select a stitch-number setting for transfers.",
            syntax: "x-xfer-stitch-number <n>",
            example: Some("x-xfer-stitch-number 5"),
            description: Some(
                "Selects the stitch-number entry used for transfer passes from the machine's table.",
            ),
        },
        "x-end-pass" => Doc {
            title: "x-end-pass",
            summary: "Mark the end of a knitting pass.",
            syntax: "x-end-pass",
            example: Some("x-end-pass"),
            description: Some(
                "Signals to a compiler that the current knitting pass should be forced to end here. Useful for debugging.",
            ),
        },
        "x-vis-color" => Doc {
            title: "x-vis-color",
            summary: "Set a visualization color for carriers.",
            syntax: "x-vis-color <color> [carriers…]",
            example: Some("x-vis-color #ff0000 3"),
            description: Some(
                "Sets a display color for the given carriers in a visualizer; has no effect on the knit fabric. Colors are any CSS color string.",
            ),
        },
        "x-loop-in" => Doc {
            title: "x-loop-in",
            summary: "Visualize an existing starter loop.",
            syntax: "x-loop-in <direction> <needle> [carriers…]",
            example: Some("x-loop-in + f10 3"),
            description: Some(
                "Creates a loop out of nowhere for visualization purposes. Should have no effect on compiled knitout.",
            ),
        },
        "x-yarn-in" => Doc {
            title: "x-yarn-in",
            summary: "Control the visualizer to bring a yarn in",
            syntax: "x-yarn-in <direction> <needle> [carriers…]",
            example: Some("x-yarn-in + f10 3"),
            description: None, // TODO: why isn't this just in?
        },
        "x-yarn-out" => Doc {
            title: "x-yarn-out",
            summary: "Control the visualizer to take a yarn out.",
            syntax: "x-yarn-out <direction> <needle> [carriers…]",
            example: Some("x-yarn-out + f10 3"),
            description: None, // TODO: why isn't this just out?
        },
        "x-end" => Doc {
            title: "x-end",
            summary: "Tell the visualizer to stop drawing the file here.",
            syntax: "x-end",
            example: None,
            description: Some("Should have no effect on compiled knitout."),
        },
        "x-roller-advance" => Doc {
            title: "x-roller-advance",
            summary: "Set the roller advance amount.",
            syntax: "x-roller-advance <value>",
            example: Some("x-roller-advance 100"),
            description: Some("Sets the amount the take-down roller advances between passes."),
        },
        "x-add-roller-advance" => Doc {
            title: "x-add-roller-advance",
            summary: "Add to the roller advance amount for this pass only.",
            syntax: "x-add-roller-advance <value>",
            example: Some("x-add-roller-advance 50"),
            description: Some(
                "Adds to the base roller advance amount for the current pass only. Multiple calls are cumulative, but reset after a pass completes.",
            ),
        },
        "x-carrier-spacing" => Doc {
            title: "x-carrier-spacing",
            summary: "Set the carrier spacing.",
            syntax: "x-carrier-spacing <n>",
            example: Some("x-carrier-spacing 4"),
            description: Some(
                "Sets the minimum spacing (in needle units) that is forcibly maintained between carriers' stopping points.",
            ),
        },
        "x-carrier-stopping-distance" => Doc {
            title: "x-carrier-stopping-distance",
            summary: "Set how far carriers stop past a needle.",
            syntax: "x-carrier-stopping-distance <n>", // TODO: ends with 0.5?
            example: Some("x-carrier-stopping-distance 2"), // TODO: is this wrong?
            description: Some(
                "Make carriers stop `n` (+0.5 with follow through) needles past the needle they last operated on.",
            ),
        },
        "x-xfer-style" => Doc {
            title: "x-xfer-style",
            summary: "Select the transfer style.",
            syntax: "x-xfer-style <four-pass|two-pass>",
            example: Some("x-xfer-style four-pass"),
            description: Some(
                "Sets how transfers are performed. `four-pass` organizes even and odd front-to-back and back-to-front transfers into their own passes (4 classes), and `two-pass` just separates front-to-back and back-to-front. `four-pass` is slower but might be more reliable.",
            ),
        },
        "x-loop-start" => Doc {
            title: "x-loop-start",
            summary: "Sets up the start of an iterated loop that repeats instructions.",
            syntax: "x-loop-start <n>",
            example: Some("x-loop-start 23"),
            description: Some(
                "Marks the beginning of a repeated region. The number is a machine-specific constant, not the number of repetitions. Each `x-loop-start` must be paired with a matching `x-loop-end` with the same number. Machines may have limits on how deeply nested loops can be.",
            ),
        },
        "x-loop-end" => Doc {
            title: "x-loop-end",
            summary: "Ends an iterated loop that repeats instructions.",
            syntax: "x-loop-end <n>",
            example: Some("x-loop-end 23"),
            description: Some(
                "Marks the end of a repeated region. The number is a machine-specific constant, not the number of repetitions. Each `x-loop-end` must be paired with a matching `x-loop-start` with the same number. Machines may have limits on how deeply nested loops can be.",
            ),
        },
        // TODO doc for unrecognized extensions? Probably not... but we have doc for extension headers...
        _ => return None,
    };
    Some(doc)
}

/// Documentation for a needle argument.
pub fn needle_doc() -> Doc {
    Doc {
        title: "Needle",
        summary: "A position on a bed where loops are held.",
        syntax: "[f|b] [s] <index>",
        example: Some("fs10"),
        description: Some(
            "A bed (`f` front or `b` back), an optional slider marker `s`, and an integer index (which may be negative). Examples: `f10`, `b-3`, `fs5` (front slider 5).",
        ),
    }
}

/// Documentation for a direction argument. The title reflects which direction, if known.
pub fn direction_doc(direction: &Direction<Parsed>) -> Doc {
    let (title, summary) = match direction {
        Direction::Right(_) => (
            "Direction +",
            "Rightward carrier movement (increasing needle index).",
        ),
        Direction::Left(_) => (
            "Direction -",
            "Leftward carrier movement (decreasing needle index).",
        ),
        Direction::Other(_, _) => ("Direction", "A carrier movement direction."),
    };
    Doc {
        title,
        summary,
        syntax: "+ | -",
        example: None,
        description: Some(
            "The direction a carrier moves across the bed: `+` is rightward (toward larger needle indices) and `-` is leftward.",
        ),
    }
}

/// Documentation for the `;!knitout-N` magic version line.
pub fn magic_doc() -> Doc {
    Doc {
        title: ";!knitout-N",
        summary: "The knitout magic string.",
        syntax: ";!knitout-<version>",
        example: Some(";!knitout-2"),
        description: Some(
            "Must be the first non-blank line of the file. `N` is the knitout format version (a non-negative integer).",
        ),
    }
}

/// The kinds of header a knitout file may contain.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeaderKind {
    Carriers,
    Yarn,
    Machine,
    Gauge,
    Width,
    Position,
    /// An `X-...` extension header or any unrecognized header.
    Extension,
}

/// Classify a header by its name (the text before the `:`), e.g. `"Carriers"`, `"Yarn-A"`.
pub fn header_kind_from_name(name: &str) -> HeaderKind {
    let name = name.trim();
    if name.eq_ignore_ascii_case("Carriers") {
        HeaderKind::Carriers
    } else if name.eq_ignore_ascii_case("Machine") {
        HeaderKind::Machine
    } else if name.eq_ignore_ascii_case("Gauge") {
        HeaderKind::Gauge
    } else if name.eq_ignore_ascii_case("Width") {
        HeaderKind::Width
    } else if name.eq_ignore_ascii_case("Position") {
        HeaderKind::Position
    } else if name.len() >= 5 && name[..5].eq_ignore_ascii_case("Yarn-") {
        HeaderKind::Yarn
    } else {
        HeaderKind::Extension
    }
}

/// Documentation for a header, keyed by its kind.
pub fn header_doc(kind: HeaderKind) -> Doc {
    match kind {
        HeaderKind::Carriers => Doc {
            title: "Carriers header",
            summary: "Declares the available yarn carriers, in order, separated by spaces.",
            syntax: ";;Carriers: <name…>",
            example: Some(";;Carriers: 1 2 3 4 5 6"),
            description: Some(
                "Required. Lists the carrier names the program may use, front-to-back on the machine. Machines with multiple carriers on the same rail can specify a rail like \"1,2,3;4,5,6\" to indicate that 1,2,3 are on the left side of the rail and 4,5,6 are on the right.",
            ),
        },
        HeaderKind::Yarn => Doc {
            title: "Yarn header",
            summary: "Describes the yarn loaded on a carrier.",
            syntax: ";;Yarn-<carrier>: <description>",
            example: Some(";;Yarn-1: 50-50 Wool-Nylon"),
            description: Some(
                "Associates a human-readable yarn description with a carrier named in `Carriers`.",
            ),
        },
        HeaderKind::Machine => Doc {
            title: "Machine header",
            summary: "Names the target knitting machine.",
            syntax: ";;Machine: <name>",
            example: Some(";;Machine: SWG091N2"),
            description: Some("Identifies the machine the program is written for."),
        },
        HeaderKind::Gauge => Doc {
            title: "Gauge header",
            summary: "The machine gauge (needles per inch).",
            syntax: ";;Gauge: <n>",
            example: Some(";;Gauge: 15"),
            description: None,
        },
        HeaderKind::Width => Doc {
            title: "Width header",
            summary: "The number of needles on a bed of the machine.",
            syntax: ";;Width: <n>",
            example: Some(";;Width: 540"),
            description: Some("A non-negative integer counting the needles on each bed."),
        },
        HeaderKind::Position => Doc {
            title: "Position header",
            summary: "How to place the piece on the bed.",
            syntax: ";;Position: <Left|Center|Right|Keep>",
            example: Some(";;Position: Center"),
            description: Some(
                "Controls horizontal translation of all operations on the needle bed. `Keep` preserves the given indices, `Center` centers, and `Left`/`Right` move the piece as far as it can go.",
            ),
        },
        HeaderKind::Extension => Doc {
            title: "Extension header",
            summary: "A non-standard extension header.",
            syntax: ";;X-<name>: <value>",
            example: Some(";;X-My-Header: 5 6 7"),
            description: Some(
                "Extension headers begin with `X-`. They carry machine- or tool-specific metadata.",
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_opcodes_have_docs() {
        for opcode in [
            "in",
            "inhook",
            "releasehook",
            "out",
            "outhook",
            "stitch",
            "rack",
            "knit",
            "tuck",
            "miss",
            "split",
            "pause",
            "drop",
            "amiss",
            "xfer",
            "x-stitch-number",
            "x-speed-number",
            "x-sub-roller-number",
            "x-presser-mode",
            "x-xfer-stitch-number",
            "x-end-pass",
            "x-vis-color",
            "x-loop-in",
            "x-yarn-in",
            "x-yarn-out",
            "x-end",
            "x-roller-advance",
            "x-add-roller-advance",
            "x-carrier-spacing",
            "x-carrier-stopping-distance",
            "x-xfer-style",
            "x-loop-start",
            "x-loop-end",
        ] {
            let doc = operation_doc_by_opcode(opcode)
                .unwrap_or_else(|| panic!("no doc for opcode {opcode}"));
            assert!(!doc.summary.is_empty());
            assert!(doc.syntax.starts_with(opcode), "syntax for {opcode}");
            assert!(doc.example.is_none_or(|s| !s.is_empty()),);
            assert!(operation_arity_hint(opcode).is_some());
        }
    }

    #[test]
    fn unknown_opcode_has_no_doc() {
        assert!(operation_doc_by_opcode("frobnicate").is_none());
        assert!(operation_arity_hint("frobnicate").is_none());
    }

    #[test]
    fn render_includes_all_pieces() {
        let doc = operation_doc_by_opcode("knit").unwrap();
        let md = render(&doc);
        assert!(md.contains("knit"));
        assert!(md.contains(doc.syntax));
        assert!(doc.example.is_none_or(|s| md.contains(s)));
        assert!(md.contains("Example:"));
    }

    #[test]
    fn header_names_classify() {
        assert_eq!(header_kind_from_name("Carriers"), HeaderKind::Carriers);
        assert_eq!(header_kind_from_name("Yarn-A"), HeaderKind::Yarn);
        assert_eq!(header_kind_from_name("Gauge"), HeaderKind::Gauge);
        assert_eq!(header_kind_from_name("X-Custom"), HeaderKind::Extension);
    }
}
