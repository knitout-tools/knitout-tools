use knitout_ast::Span;

/// A non-fatal observation about a knitout program's *runtime* behaviour: the program is still
/// executable on the virtual machine, but a human likely wants to know about this. Analogous to
/// `knitout-weak-ast`'s `ParseWarning`, but for execution rather than parsing.
///
/// Each variant carries the [`Span`] of the operation that triggered it, when one is known. It can
/// be absent for operations with no source span (e.g. machine-generated knitout) and for the
/// end-of-program checks, whose span points at the originating operation when that is recoverable.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum RuntimeWarning {
    /// A knit, split, xfer, or drop acted on a needle that held no loops. Knitting an empty needle
    /// usually means a `tuck` was intended.
    OperationOnEmptyNeedle { span: Option<Span> },
    /// A carrier was taken out or released before it ever stitched, so the machine never knew where
    /// it was.
    UnstitchedCarrier { span: Option<Span> },
    /// The program ended with a carrier still in action (never `out`/`outhook`). The span, when
    /// present, points at the `in`/`inhook` that brought it in.
    CarrierLeftInAction { span: Option<Span> },
    /// The program ended with the yarn inserting hook still engaged (missing a `releasehook`). The
    /// span, when present, points at the `inhook` that engaged it.
    HookLeftEngaged { span: Option<Span> },
}

/// A fatal runtime problem: the operation cannot be meaningfully executed on the machine. The
/// virtual machine keeps going so that later diagnostics still surface, but the program as written
/// will not run. Analogous to `knitout-weak-ast`'s `ParseError`, but for execution.
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
    /// A transfer/split named two needles that are not aligned (same bed, or slots that do not line
    /// up at the current racking).
    MisalignedTransfer { span: Option<Span> },
    /// The program uses a wider span of needles than the machine has. `needles_used` is the number
    /// of needle positions the program spans; `machine_width` is the declared `Width:` header.
    MachineTooWide {
        span: Option<Span>,
        needles_used: u32,
        machine_width: u32,
    },
}
