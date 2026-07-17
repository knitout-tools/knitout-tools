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
    /// Warn if a loop is still on the machine after its carrier has advanced at least this many
    /// rows (deepest-wale generations) past the loop's birth ("Loop hold error").
    pub loop_hold_rows: u32,
    /// Warn once two yarn-adjacent loops that were born close together end up more than this many
    /// slots apart ("Racking wearing out yarn"). Strict `>` bound.
    pub racking_wear_distance: i64,
    /// Warn once a single loop is moved more than this many slots laterally from where it was made
    /// ("Pulldown/take-down overload", SWG-N). Strict `>` bound.
    pub takedown_overload_distance: i64,
}

impl Thresholds {
    /// The default thresholds, usable in `const` context.
    pub const DEFAULT: Thresholds = Thresholds {
        long_float: 6,
        max_hold_loops: 4,
        continuous_transfer: 4,
        unstable_split_through: 2,
        loop_hold_rows: 3,
        racking_wear_distance: 6,
        takedown_overload_distance: 12,
    };
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds::DEFAULT
    }
}
