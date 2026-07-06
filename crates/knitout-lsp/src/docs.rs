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
    pub example: &'static str,
    /// A longer description. May be empty.
    pub description: &'static str,
}

/// Combine a [`Doc`]'s pieces into the Markdown string shown on hover.
pub fn render(doc: &Doc) -> String {
    let mut out = format!("**{}** — {}\n\n", doc.title, doc.summary);
    out.push_str("Signature:\n```knitout\n");
    out.push_str(doc.syntax);
    out.push_str("\n```\n\nExample:\n```knitout\n");
    out.push_str(doc.example);
    out.push_str("\n```");
    if !doc.description.is_empty() {
        out.push_str("\n\n");
        out.push_str(doc.description);
    }
    out
}

/// A one-line hint (signature + example) for argument-count diagnostics. Returns `None` for an
/// unrecognized opcode.
pub fn operation_arity_hint(opcode: &str) -> Option<String> {
    let doc = operation_doc_by_opcode(opcode)?;
    Some(format!(
        "Signature: `{}`\nExample: `{}`",
        doc.syntax, doc.example
    ))
}

/// Documentation for a parsed operation. Returns `None` for `Operation::Other` and for unknown
/// extensions (which have no fixed signature).
pub fn operation_doc(op: &Operation<Parsed>) -> Option<Doc> {
    opcode_of(op).and_then(operation_doc_by_opcode)
}

/// The canonical opcode string for a parsed operation, or `None` if it has none (an unresolved
/// `Other` node, or an unknown `x-…` extension).
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
            summary: "Bring carriers into action.",
            syntax: "in <carriers…>",
            example: "in 3",
            description: "Marks the given carriers as in use without using the yarn inserting hook. Pair with `out` to remove them.",
        },
        "inhook" => Doc {
            title: "inhook",
            summary: "Bring carriers in using the yarn inserting hook.",
            syntax: "inhook <carriers…>",
            example: "inhook 3",
            description: "Like `in`, but holds the yarn with the inserting hook until a later `releasehook`.",
        },
        "releasehook" => Doc {
            title: "releasehook",
            summary: "Release the yarn inserting hook.",
            syntax: "releasehook <carriers…>",
            example: "releasehook 3",
            description: "Releases carriers previously brought in with `inhook`, freeing the inserting hook.",
        },
        "out" => Doc {
            title: "out",
            summary: "Take carriers out of action.",
            syntax: "out <carriers…>",
            example: "out 3",
            description: "Marks the given carriers as no longer in use, without the yarn inserting hook.",
        },
        "outhook" => Doc {
            title: "outhook",
            summary: "Take carriers out using the yarn inserting hook.",
            syntax: "outhook <carriers…>",
            example: "outhook 3",
            description: "Like `out`, but uses the yarn inserting hook to hold and cut the yarn.",
        },

        // ── stitch / rack ─────────────────────────────────────────────────────
        "stitch" => Doc {
            title: "stitch",
            summary: "Set the stitch (loop-length) values.",
            syntax: "stitch <before> <after>",
            example: "stitch 5 10",
            description: "Sets the leading/stitch values applied before and after a needle is operated on.",
        },
        "rack" => Doc {
            title: "rack",
            summary: "Set the racking offset between the beds.",
            syntax: "rack <offset>",
            example: "rack 2",
            description: "Aligns the back bed relative to the front bed by the given number of needles. A fractional offset (e.g. `0.25`) enables all-needle knitting.",
        },

        // ── direction + needle + carriers ─────────────────────────────────────
        "knit" => Doc {
            title: "knit",
            summary: "Knit a loop, dropping any existing loops.",
            syntax: "knit <direction> <needle> [carriers…]",
            example: "knit + f10 3",
            description: "Pulls the carried yarn through all loops on the needle, forming a new loop and dropping the old ones. With no carriers, behaves like `drop`.",
        },
        "tuck" => Doc {
            title: "tuck",
            summary: "Add a loop without clearing existing loops.",
            syntax: "tuck <direction> <needle> [carriers…]",
            example: "tuck + f10 3",
            description: "Adds the carried yarn to the needle, keeping any loops already held there.",
        },
        "miss" => Doc {
            title: "miss",
            summary: "Move carriers past a needle without knitting.",
            syntax: "miss <direction> <needle> [carriers…]",
            example: "miss + f10 3",
            description: "Moves the given carriers to the needle's location without forming a loop.",
        },
        "split" => Doc {
            title: "split",
            summary: "Split loops to another needle while knitting.",
            syntax: "split <direction> <source> <destination> [carriers…]",
            example: "split + f10 b10 3",
            description: "Forms a new loop on the source needle and transfers its previous loops to the destination needle.",
        },

        // ── no-arg / single-needle ─────────────────────────────────────────────
        "pause" => Doc {
            title: "pause",
            summary: "Pause the machine.",
            syntax: "pause",
            example: "pause",
            description: "Requests that the machine pause for operator intervention.",
        },
        "drop" => Doc {
            title: "drop",
            summary: "Drop the loops on a needle.",
            syntax: "drop <needle>",
            example: "drop f10",
            description: "Releases all loops held on the needle. Equivalent to a `knit` with no carriers.",
        },
        "amiss" => Doc {
            title: "amiss",
            summary: "Activate a needle without knitting.",
            syntax: "amiss <needle>",
            example: "amiss f10",
            description: "Marks activity on the needle without forming or dropping loops.",
        },
        "xfer" => Doc {
            title: "xfer",
            summary: "Transfer loops between needles.",
            syntax: "xfer <source> <destination>",
            example: "xfer f10 b10",
            description: "Moves the loops on the source needle to the destination needle. No yarn or carriers are involved.",
        },

        // ── extension operations ───────────────────────────────────────────────
        "x-stitch-number" => Doc {
            title: "x-stitch-number",
            summary: "Select a machine stitch-number setting.",
            syntax: "x-stitch-number <n>",
            example: "x-stitch-number 5",
            description: "Extension: selects a stitch-number entry from the machine's stitch table.",
        },
        "x-speed-number" => Doc {
            title: "x-speed-number",
            summary: "Select a machine speed-number setting.",
            syntax: "x-speed-number <n>",
            example: "x-speed-number 3",
            description: "Extension: selects a speed-number entry from the machine's speed table.",
        },
        "x-sub-roller-number" => Doc {
            title: "x-sub-roller-number",
            summary: "Select a machine sub-roller setting.",
            syntax: "x-sub-roller-number <n>",
            example: "x-sub-roller-number 2",
            description: "Extension: selects a sub-roller entry from the machine's table.",
        },
        "x-presser-mode" => Doc {
            title: "x-presser-mode",
            summary: "Set the fabric presser mode.",
            syntax: "x-presser-mode <off|auto|on>",
            example: "x-presser-mode auto",
            description: "Extension: controls the presser foot. `auto` lets the machine decide when it is safe.",
        },
        "x-xfer-stitch-number" => Doc {
            title: "x-xfer-stitch-number",
            summary: "Select a stitch-number setting for transfers.",
            syntax: "x-xfer-stitch-number <n>",
            example: "x-xfer-stitch-number 5",
            description: "Extension: selects the stitch-number entry used for transfer passes.",
        },
        "x-end-pass" => Doc {
            title: "x-end-pass",
            summary: "Mark the end of a knitting pass.",
            syntax: "x-end-pass",
            example: "x-end-pass",
            description: "Extension: signals a pass boundary to compilers that group operations into passes.",
        },
        "x-vis-color" => Doc {
            title: "x-vis-color",
            summary: "Set a visualization color for carriers.",
            syntax: "x-vis-color <color> [carriers…]",
            example: "x-vis-color #ff0000 3",
            description: "Extension: hints a display color for the given carriers; has no effect on the knit fabric.",
        },
        "x-loop-in" => Doc {
            title: "x-loop-in",
            summary: "Bring a carrier in, forming a starting loop.",
            syntax: "x-loop-in <direction> <needle> [carriers…]",
            example: "x-loop-in + f10 3",
            description: "Extension: introduces a carrier while forming an anchoring loop on the needle.",
        },
        "x-yarn-in" => Doc {
            title: "x-yarn-in",
            summary: "Bring a yarn in at a needle.",
            syntax: "x-yarn-in <direction> <needle> [carriers…]",
            example: "x-yarn-in + f10 3",
            description: "Extension: introduces a carrier's yarn at the given needle and direction.",
        },
        "x-yarn-out" => Doc {
            title: "x-yarn-out",
            summary: "Take a yarn out at a needle.",
            syntax: "x-yarn-out <direction> <needle> [carriers…]",
            example: "x-yarn-out + f10 3",
            description: "Extension: removes a carrier's yarn at the given needle and direction.",
        },
        "x-end" => Doc {
            title: "x-end",
            summary: "Mark the end of the program.",
            syntax: "x-end",
            example: "x-end",
            description: "Extension: signals the end of the knitout program.",
        },
        "x-roller-advance" => Doc {
            title: "x-roller-advance",
            summary: "Set the roller advance amount.",
            syntax: "x-roller-advance <value>",
            example: "x-roller-advance 100",
            description: "Extension: sets the take-down roller advance used on subsequent passes.",
        },
        "x-add-roller-advance" => Doc {
            title: "x-add-roller-advance",
            summary: "Add to the roller advance amount.",
            syntax: "x-add-roller-advance <value>",
            example: "x-add-roller-advance 50",
            description: "Extension: adds to the current take-down roller advance for the next pass.",
        },
        "x-carrier-spacing" => Doc {
            title: "x-carrier-spacing",
            summary: "Set the carrier spacing.",
            syntax: "x-carrier-spacing <n>",
            example: "x-carrier-spacing 4",
            description: "Extension: sets the spacing (in needle units) maintained between carriers.",
        },
        "x-carrier-stopping-distance" => Doc {
            title: "x-carrier-stopping-distance",
            summary: "Set how far carriers stop past a needle.",
            syntax: "x-carrier-stopping-distance <n>",
            example: "x-carrier-stopping-distance 2",
            description: "Extension: carriers stop `n` (+0.5) needles past the needle they last operated on.",
        },
        "x-xfer-style" => Doc {
            title: "x-xfer-style",
            summary: "Select the transfer style.",
            syntax: "x-xfer-style <four-pass|two-pass>",
            example: "x-xfer-style four-pass",
            description: "Extension: chooses how the compiler realizes transfers on the machine.",
        },
        "x-loop-start" => Doc {
            title: "x-loop-start",
            summary: "Mark the start of a loop region.",
            syntax: "x-loop-start <n>",
            example: "x-loop-start 1",
            description: "Extension: marks the beginning of a numbered loop/repeat region.",
        },
        "x-loop-end" => Doc {
            title: "x-loop-end",
            summary: "Mark the end of a loop region.",
            syntax: "x-loop-end <n>",
            example: "x-loop-end 1",
            description: "Extension: marks the end of a numbered loop/repeat region.",
        },

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
        example: "f10",
        description: "A bed (`f` front or `b` back), an optional slider marker `s`, and an integer index (which may be negative). Examples: `f10`, `b-3`, `fs5` (front slider 5).",
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
        example: "+",
        description: "The direction a carrier moves across the bed: `+` is rightward (toward larger needle indices) and `-` is leftward.",
    }
}

/// Documentation for the `;!knitout-N` magic version line.
pub fn magic_doc() -> Doc {
    Doc {
        title: ";!knitout-N",
        summary: "The knitout version line.",
        syntax: ";!knitout-<version>",
        example: ";!knitout-2",
        description: "Must be the first non-blank line of the file. `N` is the knitout format version (a non-negative integer).",
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
    /// An `X-…` extension header or any unrecognized header.
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
            summary: "Declares the available yarn carriers, in order.",
            syntax: ";;Carriers: <name…>",
            example: ";;Carriers: 1 2 3 4 5 6",
            description: "Required. Lists the carrier names the program may use, front-to-back on the machine.",
        },
        HeaderKind::Yarn => Doc {
            title: "Yarn header",
            summary: "Describes the yarn loaded on a carrier.",
            syntax: ";;Yarn-<carrier>: <description>",
            example: ";;Yarn-1: 50-50 Wool-Nylon",
            description: "Optional. Associates a human-readable yarn description with a carrier named in `Carriers`.",
        },
        HeaderKind::Machine => Doc {
            title: "Machine header",
            summary: "Names the target knitting machine.",
            syntax: ";;Machine: <name>",
            example: ";;Machine: SWGXL",
            description: "Optional. Identifies the machine the program is written for.",
        },
        HeaderKind::Gauge => Doc {
            title: "Gauge header",
            summary: "The machine gauge (needles per inch).",
            syntax: ";;Gauge: <n>",
            example: ";;Gauge: 15",
            description: "Optional. A non-negative integer giving the needle gauge.",
        },
        HeaderKind::Width => Doc {
            title: "Width header",
            summary: "The number of needles on the machine.",
            syntax: ";;Width: <n>",
            example: ";;Width: 540",
            description: "Optional. A non-negative integer giving the needle-bed width.",
        },
        HeaderKind::Position => Doc {
            title: "Position header",
            summary: "How to place the piece on the bed.",
            syntax: ";;Position: <Left|Center|Right|Keep>",
            example: ";;Position: Center",
            description: "Optional. Controls horizontal placement of the operations on the needle bed.",
        },
        HeaderKind::Extension => Doc {
            title: "Extension header",
            summary: "A non-standard extension header.",
            syntax: ";;X-<name>: <value>",
            example: ";;X-Stitch-Numbers: 5 6 7",
            description: "Extension headers begin with `X-`. They carry machine- or tool-specific metadata.",
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
            assert!(!doc.example.is_empty());
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
        assert!(md.contains(doc.example));
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
