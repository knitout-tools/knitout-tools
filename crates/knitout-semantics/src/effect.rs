use bigdecimal::BigDecimal;
use knitout_ast::{Carrier, Direction, Needle, Validated};

/// How serious a [`Diagnostic`] is.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Severity {
    /// The program can still be executed; this is advisory (e.g. knitting on an empty needle).
    Warning,
    /// The offending operation cannot be meaningfully executed. The virtual machine keeps going so
    /// that later diagnostics can still be produced, but backends usually stop after the first one.
    Fatal,
}

/// A semantic problem the virtual machine noticed while executing a program. Diagnostics are handed
/// to the backend as [`KnitEffect::Diagnostic`]s so they arrive in program order interleaved with
/// the physical effects.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Diagnostic {
    /// A knit/tuck/miss/split used a carrier that is not currently in action.
    InactiveCarrier { carrier: Carrier<Validated> },
    /// `in`/`inhook` named a carrier that is already in action.
    CarrierAlreadyIn { carrier: Carrier<Validated> },
    /// `inhook`/`outhook` was issued while the yarn inserting hook already holds carriers.
    HookInUse,
    /// `releasehook` was issued while the hook holds nothing.
    ReleaseEmptyHook,
    /// `releasehook` named a carrier set different from the one the hook is holding.
    ReleaseWrongHook,
    /// `out` was issued for a carrier still held by the yarn inserting hook (needs `releasehook`
    /// or `outhook`).
    OutWhileHooked { carrier: Carrier<Validated> },
    /// A carrier was brought out or released before it ever stitched, so its position is unknown.
    UnstitchedCarrier { carrier: Carrier<Validated> },
    /// A transfer/split named two needles that are not aligned: either they are on the same bed, or
    /// their slots do not line up at the current racking.
    MisalignedTransfer {
        source: Needle<Validated>,
        destination: Needle<Validated>,
    },
    /// A knit/split/drop/xfer acted on a needle that holds no loops.
    EmptyNeedle { needle: Needle<Validated> },
    /// The program ended with a carrier still in action.
    CarrierNeverOut { carrier: Carrier<Validated> },
    /// The program ended with the yarn inserting hook still engaged.
    HookNeverReleased,
}

impl Diagnostic {
    /// How serious this diagnostic is.
    pub fn severity(&self) -> Severity {
        match self {
            Diagnostic::InactiveCarrier { .. }
            | Diagnostic::CarrierAlreadyIn { .. }
            | Diagnostic::HookInUse
            | Diagnostic::ReleaseEmptyHook
            | Diagnostic::ReleaseWrongHook
            | Diagnostic::OutWhileHooked { .. }
            | Diagnostic::MisalignedTransfer { .. } => Severity::Fatal,
            Diagnostic::UnstitchedCarrier { .. }
            | Diagnostic::EmptyNeedle { .. }
            | Diagnostic::CarrierNeverOut { .. }
            | Diagnostic::HookNeverReleased => Severity::Warning,
        }
    }

    /// Whether this diagnostic is [`Severity::Fatal`].
    pub fn is_fatal(&self) -> bool {
        self.severity() == Severity::Fatal
    }
}

/// Which machine setting a [`KnitEffect::SettingChanged`] refers to. Backends that care about a
/// specific setting can read its new value off the [`crate::MachineState`] passed to `observe`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Setting {
    /// `x-stitch-number`
    StitchNumber,
    /// `x-xfer-stitch-number`
    XferStitchNumber,
    /// `x-speed-number`
    Speed,
    /// `x-sub-roller-number`
    SubRoller,
    /// `x-presser-mode`
    PresserMode,
    /// `x-vis-color`
    VisColor,
    /// `x-roller-advance`
    RollerAdvance,
    /// `x-carrier-spacing`
    CarrierSpacing,
    /// `x-carrier-stopping-distance`
    CarrierStoppingDistance,
    /// `x-xfer-style`
    XferStyle,
}

/// An observable effect of executing one knitout operation. A single operation may emit several
/// effects (e.g. a knit on an occupied needle drops loops *and* forms a new one). Diagnostics are
/// delivered through the [`KnitEffect::Diagnostic`] variant so they stay ordered with the physical
/// effects around them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum KnitEffect {
    /// A carrier was brought into action. `hooked` is true for `inhook`, false for `in`.
    CarrierIn {
        carrier: Carrier<Validated>,
        hooked: bool,
    },
    /// A carrier was taken out of action. `hooked` is true for `outhook`, false for `out`.
    CarrierOut {
        carrier: Carrier<Validated>,
        hooked: bool,
    },
    /// The yarn inserting hook released the given carriers (`releasehook`).
    HookReleased { carriers: Vec<Carrier<Validated>> },
    /// The racking changed from `from` to `to` (`rack`).
    Racked { from: BigDecimal, to: BigDecimal },
    /// A new loop was formed on a needle by `carriers` (tuck adds, knit/split replace).
    LoopFormed {
        needle: Needle<Validated>,
        direction: Direction<Validated>,
        carriers: Vec<Carrier<Validated>>,
    },
    /// `count` loops were dropped from a needle (`drop`, or the old loops consumed by a `knit`).
    LoopsDropped {
        needle: Needle<Validated>,
        count: u32,
    },
    /// `count` loops were transferred from `source` to `destination` (`xfer`/`split`).
    LoopsTransferred {
        source: Needle<Validated>,
        destination: Needle<Validated>,
        count: u32,
    },
    /// Carriers were moved past a needle without forming a loop (`miss`).
    CarrierMissed {
        needle: Needle<Validated>,
        direction: Direction<Validated>,
        carriers: Vec<Carrier<Validated>>,
    },
    /// The stitch pull tensions were set (`stitch L T`).
    StitchSet {
        before: BigDecimal,
        after: BigDecimal,
    },
    /// A machine setting changed via an extension operation. The new value lives on the state.
    SettingChanged { setting: Setting },
    /// A `pause` was requested.
    Paused,
    /// An extension operation the virtual machine does not model (e.g. `x-end`, `x-loop-start`, an
    /// unknown `x-...`). State is untouched; backends that understand it can react.
    UnhandledExtension,
    /// A semantic diagnostic was raised (see [`Diagnostic`]).
    Diagnostic(Diagnostic),
}

#[cfg(test)]
mod tests {
    use super::*;
    use knitout_ast::{CarrierTable, Validated};

    fn a_carrier() -> knitout_ast::Carrier<Validated> {
        CarrierTable::new().make_carrier("1", ())
    }

    #[test]
    fn fatal_diagnostics_are_fatal() {
        let d = Diagnostic::InactiveCarrier {
            carrier: a_carrier(),
        };
        assert_eq!(d.severity(), Severity::Fatal);
        assert!(d.is_fatal());
        assert!(Diagnostic::HookInUse.is_fatal());
        assert!(Diagnostic::ReleaseEmptyHook.is_fatal());
        assert!(Diagnostic::ReleaseWrongHook.is_fatal());
    }

    #[test]
    fn warning_diagnostics_are_not_fatal() {
        assert_eq!(Diagnostic::HookNeverReleased.severity(), Severity::Warning);
        assert!(!Diagnostic::HookNeverReleased.is_fatal());
        assert!(
            !Diagnostic::CarrierNeverOut {
                carrier: a_carrier(),
            }
            .is_fatal()
        );
        assert!(
            !Diagnostic::UnstitchedCarrier {
                carrier: a_carrier(),
            }
            .is_fatal()
        );
    }
}
