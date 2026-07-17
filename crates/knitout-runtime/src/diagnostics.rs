use knitout_ast::Span;

/// A non-fatal observation about a knitout program's *runtime* behaviour: the program still runs on
/// the machine, but a human likely wants to know about it (this covers everything from gentle style
/// suggestions to "this will probably damage the yarn or the machine"). Analogous to
/// `knitout-weak-ast`'s `ParseWarning`, but for execution rather than parsing.
///
/// Each variant carries the [`Span`] of the operation(s) it concerns, when known. Spans are absent
/// for operations with no source location (e.g. machine-generated knitout) and for some
/// end-of-program checks.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum RuntimeWarning {
    /// A knit, tuck, or split was written with an empty carrier set; `drop`, `amiss`, or `xfer`
    /// says the same thing more clearly.
    NoYarnCarrier { span: Option<Span> },
    /// A `miss` with no carriers does nothing (a miss only moves carriers).
    MissWithNoYarn { span: Option<Span> },
    /// A `knit` acted on a needle holding no loops; it pulls a loop through nothing. A `tuck` is
    /// usually meant.
    KnittingThroughNothing { span: Option<Span> },
    /// A `split`/`xfer` had a source needle holding no loops, so nothing is transferred.
    TransferringNothing { span: Option<Span> },
    /// A carrier stitched at one location and then a long way away, leaving a long float. Both ends
    /// of the float are reported.
    LongFloat {
        /// The earlier stitch.
        from_span: Option<Span>,
        /// The later stitch that opened the long float.
        to_span: Option<Span>,
        /// The float length in slots.
        distance: i64,
    },
    /// Many loops are stacked on one needle (tucked on / transferred to). Stacking too many can
    /// break the needle.
    MaxHoldLoops { span: Option<Span>, count: u32 },
    /// A stack of more than one loop was transferred at once.
    TransferWithOverlappedLoops { span: Option<Span>, count: u32 },
    /// The same carrier made a new loop on two adjacent needles of the same bed, which ladders the
    /// piece.
    ContinuousPickUpStitch {
        /// The first of the two adjacent pick-up stitches.
        first_span: Option<Span>,
        /// The second (adjacent) pick-up stitch.
        second_span: Option<Span>,
    },
    /// A loop was tucked onto a needle and then knit off in the opposite direction with the same
    /// carrier, which is equivalent to a `drop` — a common cast-on mistake.
    DropAfterPickUp {
        /// The knit that undid the pick-up.
        span: Option<Span>,
        /// The tuck that first placed the loop.
        pickup_span: Option<Span>,
    },
    /// A needle was still holding loops at the end of the program. The span points at where the
    /// remaining loop was made.
    UnnecessaryLoopRemain { span: Option<Span> },
    /// A loop stayed on the machine while its carrier knit many further rows ("Loop hold error").
    LoopHeldTooLong {
        /// Where the held loop was made.
        span: Option<Span>,
        /// How many rows the carrier advanced while the loop was still held.
        rows: u32,
    },
    /// One loop was transferred many times without ever being knit through.
    ContinuousTransferWithoutKnitting {
        /// The transfer that crossed the threshold.
        span: Option<Span>,
        /// How many times the loop has now been transferred.
        transfers: u32,
    },
    /// The same loop was split-through more than once, which is unstable.
    UnstableSplitKnit {
        /// The split that crossed the threshold.
        span: Option<Span>,
        /// How many times the loop has now been split-through.
        split_throughs: u32,
    },
    /// A loop that never passed through another loop (e.g. a plain tuck) was transferred, which
    /// gives inconsistent transfer performance.
    TransferWithPickUpStitch { span: Option<Span> },
    /// Two yarn-adjacent loops that were made close together were moved far apart, stretching the
    /// yarn (and possibly bending needles).
    RackingWearingOutYarn {
        /// The transfer that pulled them apart.
        span: Option<Span>,
        /// How far apart the two loops now sit, in slots.
        distance: i64,
    },
    /// A loop was moved a long way laterally from where it was made, stressing the take-down
    /// (should not happen on SWG-N).
    PulldownTakedownOverload {
        /// The transfer that moved the loop.
        span: Option<Span>,
        /// How far the loop moved, in slots.
        distance: i64,
    },
    /// A carrier still on the yarn inserting hook, or freshly brought in on the grippers, knit onto
    /// the far bed, which feeds unstably (SWG-N).
    UnstableYarnFeeding { span: Option<Span> },
    /// The edge of a tube was knit with the beds racked, opening a slack gap at the edge.
    SlackOfEdgeStitch { span: Option<Span> },
    /// A carrier was taken out or released before it ever knit, so its position was never known.
    UnstitchedCarrier { span: Option<Span> },
    /// The program ended with a carrier still in action (never `out`/`outhook`).
    CarrierLeftInAction { span: Option<Span> },
    /// The program ended with the yarn inserting hook still engaged (missing a `releasehook`).
    HookLeftEngaged { span: Option<Span> },
}

/// A fatal runtime problem: the operation cannot be executed on the machine, so the program will
/// not compile. Analogous to `knitout-weak-ast`'s `ParseError`, but for execution.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum RuntimeError {
    /// A knit/tuck/miss/split used a carrier that was not brought into action first.
    UseOfInactiveCarrier { span: Option<Span> },
    /// `in`/`inhook` named a carrier that is already in action.
    CarrierBroughtInTwice { span: Option<Span> },
    /// `inhook`/`outhook` was issued while the yarn inserting hook already holds carriers.
    HookAlreadyInUse { span: Option<Span> },
    /// `releasehook` was issued while the hook holds nothing.
    ReleaseWithoutHook { span: Option<Span> },
    /// `releasehook` named a carrier set different from the one the hook is holding.
    ReleaseOfWrongCarriers { span: Option<Span> },
    /// `out` was issued for a carrier still held by the hook (needs `releasehook` or `outhook`).
    TakeOutWhileHooked { span: Option<Span> },
    /// A `split`/`xfer` named needles that are the right kinds of beds but do not line up at the
    /// current racking.
    TransferAtWrongRacking { span: Option<Span> },
    /// A `split`/`xfer` named needles that are not a legal bed pair (must be `f`↔`b`,
    /// `f`↔`bs`, or `fs`↔`b`).
    TransferWithWrongBeds { span: Option<Span> },
    /// The program uses a wider span of needles than the machine has. `needles_used` is the number
    /// of needle positions the program spans; `machine_width` is the declared `Width:` header.
    MachineTooWide {
        span: Option<Span>,
        needles_used: u32,
        machine_width: u32,
    },
}
