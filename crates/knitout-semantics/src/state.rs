use std::collections::HashMap;

use bigdecimal::BigDecimal;
use knitout_ast::{Carrier, Direction, Needle, PresserMode, XferStyle};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct NeedleSide {
    needle: Needle,
    side: Direction,
}

/// The state of a carrier that has been introduced via `in` or `inhook`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct CarrierState {
    /// Whether the carrier is hooked by the yarn inserting hook
    hooked: bool,
    /// The logical position of the carrier (last needle it stitched on, with follow through
    /// direction, follows `xfer` and `split`)
    logical_position: Option<NeedleSide>,
}

/// State of the virtual knitting machine
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MachineState {
    /// Smallest index of the needles mentioned on the front bed so far. Used to determine whether
    /// the width is too large for the machine.
    min_index: Option<i32>,
    /// Largest index of the needles mentioned on the front bed so far
    max_index: Option<i32>,
    /// The state of each carrier. Carriers that aren't `in`/`inhook` are not in the map.
    carrier_states: HashMap<Carrier, CarrierState>,
    /// The number of loops on each needle. Needles with no loops are not in the map.
    needle_loop_counts: HashMap<Needle, u32>,
    /// Positive values shift the back bed to the right. Back bed needles at index `b` are aligned
    /// with front bed needles at index `b + rack`. Front bed needles at `f` are aligned with back
    /// bed needles at index `f - rack`.
    rack: BigDecimal,
    /// The carriers that are currently hooked by the yarn inserting hook. None represents that the
    /// hook is currently not in use; Some(vec![]) comes from an empty `inhook`, which shouldn't be
    /// valid but we'll accept it while warning (while expecting a corresponding empty
    /// `releasehook`!)
    hook: Option<Vec<Carrier>>,
    /// The settings of the machine, changed by extension operations
    settings: MachineSettings,
}

/// The settings of various machine parameters across multiple manufacturers. These are all changed
/// by extension operations; some of them have defaults, but as they aren't part of the knitout
/// spec, these are initialized to None and backends are expected to come with their own defaults.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MachineSettings {
    /// Current knit/tuck stitch-tension number; None if not set yet
    stitch_number: Option<u32>,
    /// Current xfer stitch-tension number; None if not set yet
    xfer_stitch_number: Option<u32>,
    /// Current machine speed; None if not set yet
    speed: Option<u32>,
    /// Sub-roller tension value; None if not set yet
    sub_roller: Option<u32>,
    /// Presser mode; None if not set yet
    presser_mode: Option<PresserMode>,
    /// Colors set by x-vis-color
    vis_colors: HashMap<Carrier, String>,
    /// How much the take-down roller will advance after each pass. Does NOT take into account
    /// x-add-roller-advance, as that's impossible to calculate without knowing when passes happen.
    /// It is the backend's responsibility to keep track of the added value and reset it per pass.
    take_down_roller_advance_base: Option<BigDecimal>,
    /// How much spacing between carriers is forced
    carrier_spacing: Option<usize>,
    /// When carriers stop after passing by the needle they stitch on. A value of 0 means they move
    /// 0.5 needles "past" the needle they just stitched on to be between needles, 1 means 1.5, and
    /// so on.
    carrier_stopping_distance: Option<usize>,
    /// How to put transfers into passes
    xfer_style: Option<XferStyle>,
}

impl MachineSettings {
    pub fn stitch_number(&self) -> Option<u32> {
        self.stitch_number
    }

    pub fn xfer_stitch_number(&self) -> Option<u32> {
        self.xfer_stitch_number
    }

    pub fn speed(&self) -> Option<u32> {
        self.speed
    }

    pub fn sub_roller(&self) -> Option<u32> {
        self.sub_roller
    }

    pub fn presser_mode(&self) -> Option<PresserMode> {
        self.presser_mode
    }

    pub fn vis_color(&self, carrier: &Carrier) -> Option<&str> {
        self.vis_colors.get(carrier).map(String::as_str)
    }

    pub fn take_down_roller_advance_base(&self) -> Option<&BigDecimal> {
        self.take_down_roller_advance_base.as_ref()
    }

    pub fn carrier_spacing(&self) -> Option<usize> {
        self.carrier_spacing
    }

    pub fn carrier_stopping_distance(&self) -> Option<usize> {
        self.carrier_stopping_distance
    }

    pub fn xfer_style(&self) -> Option<XferStyle> {
        self.xfer_style
    }
}

impl MachineState {
    /// Create a new state
    pub fn new() -> Self {
        Default::default()
    }

    pub fn min_index(&self) -> Option<i32> {
        self.min_index
    }

    pub fn max_index(&self) -> Option<i32> {
        self.max_index
    }

    pub fn carrier_state(&self, carrier: &Carrier) -> Option<&CarrierState> {
        self.carrier_states.get(carrier)
    }

    pub fn needle_loop_counts(&self, needle: &Needle) -> u32 {
        *self.needle_loop_counts.get(needle).unwrap_or(&0)
    }

    pub fn rack(&self) -> &BigDecimal {
        &self.rack
    }

    pub fn hook(&self) -> Option<&Vec<Carrier>> {
        self.hook.as_ref()
    }

    pub fn settings(&self) -> &MachineSettings {
        &self.settings
    }
}
