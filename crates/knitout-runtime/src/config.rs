//! Tunable thresholds for the runtime diagnostics.
//!
//! All the "how far / how many / how long" limits live here so they are easy to adjust in one
//! place. [`analyze`](crate::analyze) uses [`Thresholds::default`]; use
//! [`analyze_with`](crate::analyze_with) to supply your own.
//!
//! Distances are measured in *slots* (racking-adjusted needle columns): a front needle sits at its
//! index, a back needle at its index plus `floor(rack)`.

/// Thresholds controlling when the runtime warnings fire.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Thresholds {
    /// A float longer than this many slots between two consecutive uses of a carrier warns
    /// ("Long float"). Given as a strict `>` bound.
    pub long_float: i64,
    /// Warn once a needle holds at least this many loops ("Max. no. of hold loops").
    pub max_hold_loops: u32,
    /// Warn once a single loop has been transferred at least this many times
    /// ("Continuous transfer without knitting").
    pub continuous_transfer: u32,
    /// Warn once a single loop has been split-through at least this many times
    /// ("Unstable split knit").
    pub unstable_split_through: u32,
    /// Warn once a single loop is moved more than this many slots laterally from where it was made
    /// ("Pulldown/take-down overload", SWG-N). Strict `>` bound.
    pub takedown_overload_distance: i64,

    // ── overstretched-float metric ──────────────────────────────────────────
    // A float (the yarn between two consecutive stitches of one carrier) is a vector in three
    // axes: lateral (slot), bed (front vs back), and age (how many rows the float has survived).
    // Its birth length is the length when it was made; its current length re-measures all three as
    // the loops move (racking/transfers) and age. A float that grows too much covers "slack of
    // edge stitch", "racking wearing out yarn", and "loop hold error" at once.
    /// The bed axis: how many slot-units apart the front and back beds are.
    pub bed_gap: i64,
    /// The age axis: how many slot-units one row (generation) of survival counts for.
    pub age_per_generation: i64,
    /// Warn once a float is longer than its birth length by more than this many slot-units. Strict
    /// `>` bound.
    pub max_float_stretch: i64,
}

impl Thresholds {
    /// The default thresholds, usable in `const` context.
    pub const DEFAULT: Thresholds = Thresholds {
        long_float: 6,
        max_hold_loops: 4,
        continuous_transfer: 4,
        unstable_split_through: 2,
        takedown_overload_distance: 12,
        bed_gap: 1,
        age_per_generation: 1,
        max_float_stretch: 4,
    };
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds::DEFAULT
    }
}
