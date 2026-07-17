use std::collections::HashMap;

use bigdecimal::{BigDecimal, RoundingMode};
use either::Either;
use knitout_ast::{
    BaseOperation, Bed, Carrier, Direction, ExtensionOperation, MyBigDecimal, MyString, MyU32,
    Needle, Never, Operation, PresserMode, Program, Validated, XferStyle,
};

use crate::backend::Backend;
use crate::effect::{Diagnostic, KnitEffect, Setting};

/// `Needle<Validated>` is not `Copy`, so neither is `NeedleSide`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct NeedleSide {
    needle: Needle<Validated>,
    side: Direction<Validated>,
}

impl NeedleSide {
    /// Create a `NeedleSide` from a needle and the direction a carrier followed through it.
    pub fn new(needle: Needle<Validated>, side: Direction<Validated>) -> Self {
        Self { needle, side }
    }

    pub fn needle(&self) -> &Needle<Validated> {
        &self.needle
    }

    pub fn side(&self) -> &Direction<Validated> {
        &self.side
    }
}

/// The state of a carrier that has been introduced via `in` or `inhook`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CarrierState {
    /// Whether the carrier is hooked by the yarn inserting hook
    hooked: bool,
    /// The logical position of the carrier (last needle it stitched on, with follow through
    /// direction, follows `xfer` and `split`)
    logical_position: Option<NeedleSide>,
}

impl CarrierState {
    pub fn hooked(&self) -> bool {
        self.hooked
    }

    pub fn logical_position(&self) -> Option<&NeedleSide> {
        self.logical_position.as_ref()
    }
}

/// State of the virtual knitting machine
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MachineState {
    /// Smallest index of any needle mentioned so far (across both beds). Used to determine whether
    /// the width is too large for the machine.
    min_index: Option<i32>,
    /// Largest index of any needle mentioned so far (across both beds).
    max_index: Option<i32>,
    /// The state of each carrier. Carriers that aren't `in`/`inhook` are not in the map.
    carrier_states: HashMap<Carrier<Validated>, CarrierState>,
    /// The number of loops on each needle. Needles with no loops are not in the map.
    needle_loop_counts: HashMap<Needle<Validated>, u32>,
    /// Positive values shift the back bed to the right. Back bed needles at index `b` are aligned
    /// with front bed needles at index `b + rack`. Front bed needles at `f` are aligned with back
    /// bed needles at index `f - rack`.
    rack: BigDecimal,
    /// The carriers that are currently hooked by the yarn inserting hook. None represents that the
    /// hook is currently not in use; Some(vec![]) comes from an empty `inhook`, which shouldn't be
    /// valid but we'll accept it while warning (while expecting a corresponding empty
    /// `releasehook`!)
    hook: Option<Vec<Carrier<Validated>>>,
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
    presser_mode: Option<PresserMode<Validated>>,
    /// Colors set by x-vis-color
    vis_colors: HashMap<Carrier<Validated>, String>,
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
    xfer_style: Option<XferStyle<Validated>>,
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

    pub fn presser_mode(&self) -> Option<PresserMode<Validated>> {
        self.presser_mode
    }

    pub fn vis_color(&self, carrier: &Carrier<Validated>) -> Option<&str> {
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

    pub fn xfer_style(&self) -> Option<XferStyle<Validated>> {
        self.xfer_style
    }
}

/// Unwrap the left side of an `Either<T, Never>`; the right side is uninhabited in `Validated`.
fn only<T>(e: &Either<T, Never>) -> &T {
    match e {
        Either::Left(t) => t,
        Either::Right(never) => match *never {},
    }
}

/// The integer index of a validated needle.
fn needle_index(needle: &Needle<Validated>) -> i32 {
    *only(needle.index().value())
}

/// Whether a validated needle is on the front bed.
fn is_front(needle: &Needle<Validated>) -> bool {
    match needle.bed() {
        Bed::Front(_) => true,
        Bed::Back(_) => false,
        Bed::Other(_, never) => match *never {},
    }
}

/// The integer value of a validated `MyU32`.
fn u32_value(v: &MyU32<Validated>) -> u32 {
    *only(v.value())
}

/// The `BigDecimal` value of a validated `MyBigDecimal`.
fn bigdecimal_value(v: &MyBigDecimal<Validated>) -> BigDecimal {
    only(v.value()).clone()
}

/// The `String` value of a validated `MyString`.
fn string_value(v: &MyString<Validated>) -> String {
    only(v.value()).clone()
}

/// Collect a validated carrier list into owned carriers.
fn carriers_of(carriers: &[Either<Carrier<Validated>, Never>]) -> Vec<Carrier<Validated>> {
    carriers.iter().map(|c| *only(c)).collect()
}

impl MachineState {
    /// Create a new state
    fn new() -> Self {
        Default::default()
    }

    pub fn min_index(&self) -> Option<i32> {
        self.min_index
    }

    pub fn max_index(&self) -> Option<i32> {
        self.max_index
    }

    pub fn carrier_state(&self, carrier: &Carrier<Validated>) -> Option<&CarrierState> {
        self.carrier_states.get(carrier)
    }

    pub fn needle_loop_counts(&self, needle: &Needle<Validated>) -> u32 {
        *self.needle_loop_counts.get(needle).unwrap_or(&0)
    }

    pub fn rack(&self) -> &BigDecimal {
        &self.rack
    }

    pub fn hook(&self) -> Option<&Vec<Carrier<Validated>>> {
        self.hook.as_ref()
    }

    pub fn settings(&self) -> &MachineSettings {
        &self.settings
    }

    /// The slot a needle occupies at the current racking: front needles sit at their index, back
    /// needles are shifted right by `floor(rack)`.
    fn slot(&self, needle: &Needle<Validated>) -> BigDecimal {
        let index = BigDecimal::from(needle_index(needle));
        if is_front(needle) {
            index
        } else {
            index + self.rack.with_scale_round(0, RoundingMode::Floor)
        }
    }

    /// Whether `source` and `destination` can transfer to each other: they must be on opposite beds
    /// and occupy the same slot at the current racking.
    fn aligned(&self, source: &Needle<Validated>, destination: &Needle<Validated>) -> bool {
        is_front(source) != is_front(destination) && self.slot(source) == self.slot(destination)
    }

    /// Record that a needle at `index` was mentioned, widening the tracked bounds.
    fn observe_index(&mut self, index: i32) {
        self.min_index = Some(self.min_index.map_or(index, |m| m.min(index)));
        self.max_index = Some(self.max_index.map_or(index, |m| m.max(index)));
    }

    /// Set the loop count on a needle, removing it from the map when the count reaches zero.
    fn set_loops(&mut self, needle: &Needle<Validated>, count: u32) {
        if count == 0 {
            self.needle_loop_counts.remove(needle);
        } else {
            self.needle_loop_counts.insert(needle.clone(), count);
        }
    }

    /// Point every active carrier whose logical position sits on `from` at `to` instead, keeping
    /// its follow-through direction (the spec's transfer "motion rule").
    fn follow_transfer(&mut self, from: &Needle<Validated>, to: &Needle<Validated>) {
        for state in self.carrier_states.values_mut() {
            if let Some(pos) = &mut state.logical_position {
                if pos.needle == *from {
                    pos.needle = to.clone();
                }
            }
        }
    }

    /// Set the logical position of each named carrier to `(needle, direction)`. Carriers that are
    /// not in action are reported as [`Diagnostic::InactiveCarrier`] and left alone.
    fn set_carrier_positions(
        &mut self,
        carriers: &[Carrier<Validated>],
        needle: &Needle<Validated>,
        direction: &Direction<Validated>,
        diagnostics: &mut Vec<KnitEffect>,
    ) {
        for carrier in carriers {
            match self.carrier_states.get_mut(carrier) {
                Some(state) => {
                    state.logical_position = Some(NeedleSide::new(needle.clone(), *direction));
                }
                None => diagnostics.push(KnitEffect::Diagnostic(Diagnostic::InactiveCarrier {
                    carrier: *carrier,
                })),
            }
        }
    }

    /// Execute a program, passing effects to a backend as it is executed.
    pub fn execute<B: Backend>(
        program: &Program<Validated>,
    ) -> Result<<B as Backend>::Output, <B as Backend>::Error> {
        let mut backend = B::register(program)?;
        let mut state = MachineState::new();
        for i in 0..program.operations().len() {
            if let Some(err) = state.apply_operation(i, program, &mut backend) {
                return Err(err);
            }
        }
        if let Some(err) = state.finish_program(program, &mut backend) {
            return Err(err);
        }
        backend.finish()
    }

    /// Hand one effect to the backend, turning its optional error into a `Result` for `?`.
    fn emit<B: Backend>(
        &self,
        backend: &mut B,
        effect: KnitEffect,
        i: usize,
        program: &Program<Validated>,
    ) -> Result<(), <B as Backend>::Error> {
        match backend.observe(effect, self, i, program) {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    /// Apply the operation at index `i` to the machine state, passing effects to the backend as
    /// they are generated. Returns `Some` if the backend asked to stop.
    fn apply_operation<B: Backend>(
        &mut self,
        i: usize,
        program: &Program<Validated>,
        backend: &mut B,
    ) -> Option<<B as Backend>::Error> {
        self.apply_operation_inner(i, program, backend).err()
    }

    fn apply_operation_inner<B: Backend>(
        &mut self,
        i: usize,
        program: &Program<Validated>,
        backend: &mut B,
    ) -> Result<(), <B as Backend>::Error> {
        match &program.operations()[i].value {
            Operation::BaseOperation(op) => self.apply_base(op, i, program, backend),
            Operation::ExtensionOperation(op) => self.apply_extension(op, i, program, backend),
            Operation::Other(never) => match *never {},
        }
    }

    fn apply_base<B: Backend>(
        &mut self,
        op: &BaseOperation<Validated>,
        i: usize,
        program: &Program<Validated>,
        backend: &mut B,
    ) -> Result<(), <B as Backend>::Error> {
        // Each handler mutates the state and returns the effects (including diagnostics) it
        // produced; we then hand those to the backend against the post-operation state.
        let effects = match op {
            BaseOperation::In { carriers } => self.op_in(&carriers_of(carriers), false),
            BaseOperation::Inhook { carriers } => self.op_in(&carriers_of(carriers), true),
            BaseOperation::Releasehook { carriers } => self.op_releasehook(&carriers_of(carriers)),
            BaseOperation::Out { carriers } => self.op_out(&carriers_of(carriers), false),
            BaseOperation::Outhook { carriers } => self.op_out(&carriers_of(carriers), true),
            BaseOperation::Stitch {
                before_pull,
                after_pull,
            } => vec![KnitEffect::StitchSet {
                before: bigdecimal_value(before_pull),
                after: bigdecimal_value(after_pull),
            }],
            BaseOperation::Rack { offset } => self.op_rack(bigdecimal_value(offset)),
            BaseOperation::Knit {
                direction,
                needle,
                carriers,
            } => self.op_knit(direction, only(needle), &carriers_of(carriers)),
            BaseOperation::Tuck {
                direction,
                needle,
                carriers,
            } => self.op_tuck(direction, only(needle), &carriers_of(carriers)),
            BaseOperation::Miss {
                direction,
                needle,
                carriers,
            } => self.op_miss(direction, only(needle), &carriers_of(carriers)),
            BaseOperation::Split {
                direction,
                source,
                destination,
                carriers,
            } => self.op_split(
                direction,
                only(source),
                only(destination),
                &carriers_of(carriers),
            ),
            BaseOperation::Drop { needle } => self.op_drop(only(needle)),
            BaseOperation::Amiss { needle } => self.op_amiss(only(needle)),
            BaseOperation::Xfer {
                source,
                destination,
            } => self.op_xfer(only(source), only(destination)),
            BaseOperation::Pause => vec![KnitEffect::Paused],
        };
        for effect in effects {
            self.emit(backend, effect, i, program)?;
        }
        Ok(())
    }

    /// `in` / `inhook`: bring carriers into action.
    fn op_in(&mut self, carriers: &[Carrier<Validated>], hooked: bool) -> Vec<KnitEffect> {
        let mut effects = Vec::new();
        if hooked && self.hook.is_some() {
            effects.push(KnitEffect::Diagnostic(Diagnostic::HookInUse));
        } else if hooked {
            self.hook = Some(carriers.to_vec());
        }
        for &carrier in carriers {
            match self.carrier_states.entry(carrier) {
                std::collections::hash_map::Entry::Occupied(_) => {
                    effects.push(KnitEffect::Diagnostic(Diagnostic::CarrierAlreadyIn {
                        carrier,
                    }));
                }
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert(CarrierState {
                        hooked,
                        logical_position: None,
                    });
                    effects.push(KnitEffect::CarrierIn { carrier, hooked });
                }
            }
        }
        effects
    }

    /// `releasehook`: release the carriers held by the yarn inserting hook.
    fn op_releasehook(&mut self, carriers: &[Carrier<Validated>]) -> Vec<KnitEffect> {
        match &self.hook {
            None => vec![KnitEffect::Diagnostic(Diagnostic::ReleaseEmptyHook)],
            Some(held) if held != carriers => {
                vec![KnitEffect::Diagnostic(Diagnostic::ReleaseWrongHook)]
            }
            Some(_) => {
                for carrier in carriers {
                    if let Some(state) = self.carrier_states.get_mut(carrier) {
                        state.hooked = false;
                    }
                }
                self.hook = None;
                vec![KnitEffect::HookReleased {
                    carriers: carriers.to_vec(),
                }]
            }
        }
    }

    /// `out` / `outhook`: take carriers out of action.
    fn op_out(&mut self, carriers: &[Carrier<Validated>], hooked: bool) -> Vec<KnitEffect> {
        let mut effects = Vec::new();
        if hooked && self.hook.is_some() {
            effects.push(KnitEffect::Diagnostic(Diagnostic::HookInUse));
        }
        for &carrier in carriers {
            match self.carrier_states.get(&carrier) {
                None => effects.push(KnitEffect::Diagnostic(Diagnostic::InactiveCarrier {
                    carrier,
                })),
                Some(state) => {
                    if !hooked && state.hooked {
                        effects.push(KnitEffect::Diagnostic(Diagnostic::OutWhileHooked {
                            carrier,
                        }));
                    }
                    if state.logical_position.is_none() {
                        effects.push(KnitEffect::Diagnostic(Diagnostic::UnstitchedCarrier {
                            carrier,
                        }));
                    }
                    self.carrier_states.remove(&carrier);
                    effects.push(KnitEffect::CarrierOut { carrier, hooked });
                }
            }
        }
        effects
    }

    /// `rack`: set the racking offset.
    fn op_rack(&mut self, offset: BigDecimal) -> Vec<KnitEffect> {
        let from = std::mem::replace(&mut self.rack, offset.clone());
        vec![KnitEffect::Racked { from, to: offset }]
    }

    /// `knit`: drop the loops on the needle, then form a new loop from the carriers (empty carriers
    /// means `drop`).
    fn op_knit(
        &mut self,
        direction: &Direction<Validated>,
        needle: &Needle<Validated>,
        carriers: &[Carrier<Validated>],
    ) -> Vec<KnitEffect> {
        self.observe_index(needle_index(needle));
        let mut effects = Vec::new();
        let existing = self.needle_loop_counts(needle);
        if existing == 0 {
            effects.push(KnitEffect::Diagnostic(Diagnostic::EmptyNeedle {
                needle: needle.clone(),
            }));
        } else {
            effects.push(KnitEffect::LoopsDropped {
                needle: needle.clone(),
                count: existing,
            });
        }
        if carriers.is_empty() {
            self.set_loops(needle, 0);
        } else {
            self.set_loops(needle, 1);
            effects.push(KnitEffect::LoopFormed {
                needle: needle.clone(),
                direction: *direction,
                carriers: carriers.to_vec(),
            });
        }
        self.set_carrier_positions(carriers, needle, direction, &mut effects);
        effects
    }

    /// `tuck`: add a loop from the carriers on top of the ones already there (empty carriers means
    /// `amiss`, which just selects the needle).
    fn op_tuck(
        &mut self,
        direction: &Direction<Validated>,
        needle: &Needle<Validated>,
        carriers: &[Carrier<Validated>],
    ) -> Vec<KnitEffect> {
        self.observe_index(needle_index(needle));
        if carriers.is_empty() {
            return Vec::new();
        }
        let count = self.needle_loop_counts(needle) + 1;
        self.set_loops(needle, count);
        let mut effects = vec![KnitEffect::LoopFormed {
            needle: needle.clone(),
            direction: *direction,
            carriers: carriers.to_vec(),
        }];
        self.set_carrier_positions(carriers, needle, direction, &mut effects);
        effects
    }

    /// `miss`: move carriers past a needle without forming a loop.
    fn op_miss(
        &mut self,
        direction: &Direction<Validated>,
        needle: &Needle<Validated>,
        carriers: &[Carrier<Validated>],
    ) -> Vec<KnitEffect> {
        self.observe_index(needle_index(needle));
        let mut effects = vec![KnitEffect::CarrierMissed {
            needle: needle.clone(),
            direction: *direction,
            carriers: carriers.to_vec(),
        }];
        self.set_carrier_positions(carriers, needle, direction, &mut effects);
        effects
    }

    /// `split`: transfer the loops on `source` to `destination`, then form a new loop on `source`
    /// from the carriers (empty carriers means `xfer`).
    fn op_split(
        &mut self,
        direction: &Direction<Validated>,
        source: &Needle<Validated>,
        destination: &Needle<Validated>,
        carriers: &[Carrier<Validated>],
    ) -> Vec<KnitEffect> {
        self.observe_index(needle_index(source));
        self.observe_index(needle_index(destination));
        if !self.aligned(source, destination) {
            return vec![KnitEffect::Diagnostic(Diagnostic::MisalignedTransfer {
                source: source.clone(),
                destination: destination.clone(),
            })];
        }
        let mut effects = self.transfer_loops(source, destination);
        if !carriers.is_empty() {
            self.set_loops(source, 1);
            effects.push(KnitEffect::LoopFormed {
                needle: source.clone(),
                direction: *direction,
                carriers: carriers.to_vec(),
            });
        }
        self.set_carrier_positions(carriers, source, direction, &mut effects);
        effects
    }

    /// `xfer`: move all loops from `source` to `destination`.
    fn op_xfer(
        &mut self,
        source: &Needle<Validated>,
        destination: &Needle<Validated>,
    ) -> Vec<KnitEffect> {
        self.observe_index(needle_index(source));
        self.observe_index(needle_index(destination));
        if !self.aligned(source, destination) {
            return vec![KnitEffect::Diagnostic(Diagnostic::MisalignedTransfer {
                source: source.clone(),
                destination: destination.clone(),
            })];
        }
        self.transfer_loops(source, destination)
    }

    /// Move every loop from `source` onto `destination` and slide carrier positions along, emitting
    /// the transfer effect (or an empty-needle warning when there was nothing to move).
    fn transfer_loops(
        &mut self,
        source: &Needle<Validated>,
        destination: &Needle<Validated>,
    ) -> Vec<KnitEffect> {
        let moved = self.needle_loop_counts(source);
        if moved == 0 {
            return vec![KnitEffect::Diagnostic(Diagnostic::EmptyNeedle {
                needle: source.clone(),
            })];
        }
        let combined = self.needle_loop_counts(destination) + moved;
        self.set_loops(destination, combined);
        self.set_loops(source, 0);
        self.follow_transfer(source, destination);
        vec![KnitEffect::LoopsTransferred {
            source: source.clone(),
            destination: destination.clone(),
            count: moved,
        }]
    }

    /// `drop`: drop the loops on a needle (a `knit` with no carriers).
    fn op_drop(&mut self, needle: &Needle<Validated>) -> Vec<KnitEffect> {
        self.op_knit(&Direction::Right(()), needle, &[])
    }

    /// `amiss`: select a needle without stitching (a `tuck` with no carriers).
    fn op_amiss(&mut self, needle: &Needle<Validated>) -> Vec<KnitEffect> {
        self.op_tuck(&Direction::Right(()), needle, &[])
    }

    fn apply_extension<B: Backend>(
        &mut self,
        op: &ExtensionOperation<Validated>,
        i: usize,
        program: &Program<Validated>,
        backend: &mut B,
    ) -> Result<(), <B as Backend>::Error> {
        let effect = match op {
            ExtensionOperation::XStitchNumber { value } => {
                self.settings.stitch_number = Some(u32_value(value));
                KnitEffect::SettingChanged {
                    setting: Setting::StitchNumber,
                }
            }
            ExtensionOperation::XXferStitchNumber { value } => {
                self.settings.xfer_stitch_number = Some(u32_value(value));
                KnitEffect::SettingChanged {
                    setting: Setting::XferStitchNumber,
                }
            }
            ExtensionOperation::XSpeedNumber { value } => {
                self.settings.speed = Some(u32_value(value));
                KnitEffect::SettingChanged {
                    setting: Setting::Speed,
                }
            }
            ExtensionOperation::XSubRollerNumber { value } => {
                self.settings.sub_roller = Some(u32_value(value));
                KnitEffect::SettingChanged {
                    setting: Setting::SubRoller,
                }
            }
            ExtensionOperation::XPresserMode { mode } => {
                self.settings.presser_mode = Some(*mode);
                KnitEffect::SettingChanged {
                    setting: Setting::PresserMode,
                }
            }
            ExtensionOperation::XVisColor { color, carriers } => {
                let color = string_value(color);
                for carrier in carriers_of(carriers) {
                    self.settings.vis_colors.insert(carrier, color.clone());
                }
                KnitEffect::SettingChanged {
                    setting: Setting::VisColor,
                }
            }
            ExtensionOperation::XRollerAdvance { value } => {
                self.settings.take_down_roller_advance_base = Some(bigdecimal_value(value));
                KnitEffect::SettingChanged {
                    setting: Setting::RollerAdvance,
                }
            }
            ExtensionOperation::XCarrierSpacing { value } => {
                self.settings.carrier_spacing = Some(u32_value(value) as usize);
                KnitEffect::SettingChanged {
                    setting: Setting::CarrierSpacing,
                }
            }
            ExtensionOperation::XCarrierStoppingDistance { value } => {
                self.settings.carrier_stopping_distance = Some(u32_value(value) as usize);
                KnitEffect::SettingChanged {
                    setting: Setting::CarrierStoppingDistance,
                }
            }
            ExtensionOperation::XXferStyle { style } => {
                self.settings.xfer_style = Some(*style);
                KnitEffect::SettingChanged {
                    setting: Setting::XferStyle,
                }
            }
            ExtensionOperation::XEndPass
            | ExtensionOperation::XLoopIn { .. }
            | ExtensionOperation::XYarnIn { .. }
            | ExtensionOperation::XYarnOut { .. }
            | ExtensionOperation::XEnd
            | ExtensionOperation::XAddRollerAdvance { .. }
            | ExtensionOperation::XLoopStart { .. }
            | ExtensionOperation::XLoopEnd { .. }
            | ExtensionOperation::UnknownExtension { .. } => KnitEffect::UnhandledExtension,
        };
        self.emit(backend, effect, i, program)
    }

    /// After the last operation, warn about carriers left in action and a hook left engaged.
    fn finish_program<B: Backend>(
        &mut self,
        program: &Program<Validated>,
        backend: &mut B,
    ) -> Option<<B as Backend>::Error> {
        self.finish_program_inner(program, backend).err()
    }

    fn finish_program_inner<B: Backend>(
        &mut self,
        program: &Program<Validated>,
        backend: &mut B,
    ) -> Result<(), <B as Backend>::Error> {
        let i = program.operations().len();
        if self.hook.is_some() {
            self.emit(
                backend,
                KnitEffect::Diagnostic(Diagnostic::HookNeverReleased),
                i,
                program,
            )?;
        }
        // Sort by carrier name for an order independent of the hash map's iteration order.
        let mut leftover: Vec<Carrier<Validated>> = self.carrier_states.keys().copied().collect();
        leftover.sort_by(|a, b| {
            program
                .carrier_table()
                .carrier_name(a)
                .cmp(program.carrier_table().carrier_name(b))
        });
        for carrier in leftover {
            self.emit(
                backend,
                KnitEffect::Diagnostic(Diagnostic::CarrierNeverOut { carrier }),
                i,
                program,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_knitout;
    use knitout_ast::{
        Annotated, CarrierTable, CarriersHeaderValue, Headers, IsSlider, MyI32, PresserMode,
        Provenance, XferStyle,
    };

    // --- program builders -------------------------------------------------

    fn front(index: i32) -> Needle<Validated> {
        Needle::new(
            Bed::Front(()),
            IsSlider::NotSlider(()),
            MyI32::new(Either::Left(index), ()),
            (),
        )
    }

    fn back(index: i32) -> Needle<Validated> {
        Needle::new(
            Bed::Back(()),
            IsSlider::NotSlider(()),
            MyI32::new(Either::Left(index), ()),
            (),
        )
    }

    fn right() -> Direction<Validated> {
        Direction::Right(())
    }

    fn left() -> Direction<Validated> {
        Direction::Left(())
    }

    fn cs(carriers: &[Carrier<Validated>]) -> Vec<Either<Carrier<Validated>, Never>> {
        carriers.iter().map(|c| Either::Left(*c)).collect()
    }

    fn bigdec(n: i64) -> MyBigDecimal<Validated> {
        MyBigDecimal::new(Either::Left(BigDecimal::from(n)), ())
    }

    fn myu32(n: u32) -> MyU32<Validated> {
        MyU32::new(Either::Left(n), ())
    }

    fn mystr(s: &str) -> MyString<Validated> {
        MyString::new(Either::Left(s.to_string()), ())
    }

    fn base(op: BaseOperation<Validated>) -> Operation<Validated> {
        Operation::BaseOperation(op)
    }

    fn ext(op: ExtensionOperation<Validated>) -> Operation<Validated> {
        Operation::ExtensionOperation(op)
    }

    fn program(table: CarrierTable, ops: Vec<Operation<Validated>>) -> Program<Validated> {
        let headers = Headers::new(Either::Left(Annotated::unit(CarriersHeaderValue::new(
            Either::Left(vec![]),
            (),
        ))));
        let mut program = Program::new(
            Either::Left(Annotated::unit(2u32)),
            Either::Left(headers),
            Vec::new(),
            table,
        );
        for op in ops {
            program.push_operation(Annotated::new(op, Provenance::default()));
        }
        program
    }

    /// A backend that records every `(effect, post-op state)` pair.
    struct Rec {
        log: Vec<(KnitEffect, MachineState)>,
    }

    impl Backend for Rec {
        type Output = Vec<(KnitEffect, MachineState)>;
        type Error = ();

        fn register(_program: &Program<Validated>) -> Result<Self, Self::Error> {
            Ok(Rec { log: Vec::new() })
        }

        fn observe(
            &mut self,
            effect: KnitEffect,
            state: &MachineState,
            _operation_index: usize,
            _program: &Program<Validated>,
        ) -> Option<Self::Error> {
            self.log.push((effect, state.clone()));
            None
        }

        fn finish(self) -> Result<Self::Output, Self::Error> {
            Ok(self.log)
        }
    }

    /// A backend that stops (returns an error) the first time it observes anything.
    struct AlwaysStop;

    impl Backend for AlwaysStop {
        type Output = ();
        type Error = ();

        fn register(_program: &Program<Validated>) -> Result<Self, Self::Error> {
            Ok(AlwaysStop)
        }

        fn observe(
            &mut self,
            _effect: KnitEffect,
            _state: &MachineState,
            _operation_index: usize,
            _program: &Program<Validated>,
        ) -> Option<Self::Error> {
            Some(())
        }

        fn finish(self) -> Result<Self::Output, Self::Error> {
            Ok(())
        }
    }

    fn run(program: &Program<Validated>) -> Vec<(KnitEffect, MachineState)> {
        run_knitout::<Rec>(program).unwrap()
    }

    fn effects(program: &Program<Validated>) -> Vec<KnitEffect> {
        run(program).into_iter().map(|(e, _)| e).collect()
    }

    /// The state recorded alongside the last effect, i.e. the final machine state.
    fn final_state(program: &Program<Validated>) -> MachineState {
        run(program).pop().expect("expected at least one effect").1
    }

    // --- pure helper unit tests ------------------------------------------

    #[test]
    fn only_returns_left() {
        let e: Either<i32, Never> = Either::Left(7);
        assert_eq!(*only(&e), 7);
    }

    #[test]
    fn needle_index_and_bed_extraction() {
        assert_eq!(needle_index(&front(4)), 4);
        assert_eq!(needle_index(&back(-3)), -3);
        assert!(is_front(&front(0)));
        assert!(!is_front(&back(0)));
    }

    #[test]
    fn slot_front_is_index_back_is_shifted() {
        let mut state = MachineState::new();
        state.rack = BigDecimal::from(2);
        assert_eq!(state.slot(&front(5)), BigDecimal::from(5));
        // back index 3 at rack 2 sits at slot 5
        assert_eq!(state.slot(&back(3)), BigDecimal::from(5));
    }

    #[test]
    fn slot_uses_floor_of_rack() {
        let mut state = MachineState::new();
        state.rack = BigDecimal::from(-1) / BigDecimal::from(2); // -0.5, floors to -1
        assert_eq!(state.slot(&back(0)), BigDecimal::from(-1));
        // front is never shifted regardless of rack
        assert_eq!(state.slot(&front(0)), BigDecimal::from(0));
    }

    #[test]
    fn aligned_requires_opposite_beds_and_equal_slot() {
        let mut state = MachineState::new();
        state.rack = BigDecimal::from(2);
        // front 5 and back 3 share slot 5 at rack 2
        assert!(state.aligned(&front(5), &back(3)));
        // same bed never aligns
        assert!(!state.aligned(&front(5), &front(5)));
        // opposite beds but different slots
        assert!(!state.aligned(&front(5), &back(4)));
    }

    #[test]
    fn aligned_with_fractional_negative_rack() {
        let mut state = MachineState::new();
        state.rack = BigDecimal::from(-1) / BigDecimal::from(2); // -0.5
        // back 0 -> slot -1, so it aligns with front -1
        assert!(state.aligned(&front(-1), &back(0)));
        assert!(!state.aligned(&front(0), &back(0)));
    }

    #[test]
    fn observe_index_widens_bounds() {
        let mut state = MachineState::new();
        assert_eq!(state.min_index(), None);
        state.observe_index(5);
        state.observe_index(2);
        state.observe_index(8);
        assert_eq!(state.min_index(), Some(2));
        assert_eq!(state.max_index(), Some(8));
    }

    #[test]
    fn set_loops_removes_at_zero() {
        let mut state = MachineState::new();
        state.set_loops(&front(1), 3);
        assert_eq!(state.needle_loop_counts(&front(1)), 3);
        state.set_loops(&front(1), 0);
        assert_eq!(state.needle_loop_counts(&front(1)), 0);
        assert!(!state.needle_loop_counts.contains_key(&front(1)));
    }

    // --- carrier lifecycle ------------------------------------------------

    #[test]
    fn in_brings_carrier_into_action() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(table, vec![base(BaseOperation::In { carriers: cs(&[c]) })]);
        let log = run(&p);
        assert_eq!(
            log[0].0,
            KnitEffect::CarrierIn {
                carrier: c,
                hooked: false
            }
        );
        assert!(log[0].1.carrier_state(&c).is_some());
        assert!(!log[0].1.carrier_state(&c).unwrap().hooked());
        // finish warns the carrier was never taken out
        assert_eq!(
            log[1].0,
            KnitEffect::Diagnostic(Diagnostic::CarrierNeverOut { carrier: c })
        );
    }

    #[test]
    fn inhook_engages_hook() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![base(BaseOperation::Inhook { carriers: cs(&[c]) })],
        );
        let log = run(&p);
        assert_eq!(
            log[0].0,
            KnitEffect::CarrierIn {
                carrier: c,
                hooked: true
            }
        );
        assert!(log[0].1.carrier_state(&c).unwrap().hooked());
        assert_eq!(log[0].1.hook(), Some(&vec![c]));
        // finish warns both about the hook and the carrier, hook first
        assert_eq!(
            log[1].0,
            KnitEffect::Diagnostic(Diagnostic::HookNeverReleased)
        );
        assert_eq!(
            log[2].0,
            KnitEffect::Diagnostic(Diagnostic::CarrierNeverOut { carrier: c })
        );
    }

    #[test]
    fn inhook_while_hooked_is_fatal() {
        let mut table = CarrierTable::new();
        let c1 = table.make_carrier("1", ());
        let c2 = table.make_carrier("2", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::Inhook {
                    carriers: cs(&[c1]),
                }),
                base(BaseOperation::Inhook {
                    carriers: cs(&[c2]),
                }),
            ],
        );
        let es = effects(&p);
        assert_eq!(es[1], KnitEffect::Diagnostic(Diagnostic::HookInUse));
        // c2 is still brought in despite the hook complaint
        assert_eq!(
            es[2],
            KnitEffect::CarrierIn {
                carrier: c2,
                hooked: true
            }
        );
    }

    #[test]
    fn bringing_in_twice_is_fatal() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::In { carriers: cs(&[c]) }),
            ],
        );
        let es = effects(&p);
        assert_eq!(
            es[0],
            KnitEffect::CarrierIn {
                carrier: c,
                hooked: false
            }
        );
        assert_eq!(
            es[1],
            KnitEffect::Diagnostic(Diagnostic::CarrierAlreadyIn { carrier: c })
        );
    }

    #[test]
    fn releasehook_releases_matching_set() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::Inhook { carriers: cs(&[c]) }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(1)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Releasehook { carriers: cs(&[c]) }),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let log = run(&p);
        let es: Vec<_> = log.iter().map(|(e, _)| e.clone()).collect();
        assert_eq!(es[2], KnitEffect::HookReleased { carriers: vec![c] });
        // after release, the hook is empty and the carrier is no longer hooked
        assert_eq!(log[2].1.hook(), None);
        assert!(!log[2].1.carrier_state(&c).unwrap().hooked());
        // then it goes out cleanly (no diagnostics)
        assert_eq!(
            es[3],
            KnitEffect::CarrierOut {
                carrier: c,
                hooked: false
            }
        );
        assert_eq!(es.len(), 4);
    }

    #[test]
    fn releasehook_without_hook_is_fatal() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![base(BaseOperation::Releasehook { carriers: cs(&[c]) })],
        );
        assert_eq!(
            effects(&p)[0],
            KnitEffect::Diagnostic(Diagnostic::ReleaseEmptyHook)
        );
    }

    #[test]
    fn releasehook_wrong_set_is_fatal() {
        let mut table = CarrierTable::new();
        let c1 = table.make_carrier("1", ());
        let c2 = table.make_carrier("2", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::Inhook {
                    carriers: cs(&[c1]),
                }),
                base(BaseOperation::Releasehook {
                    carriers: cs(&[c2]),
                }),
            ],
        );
        assert_eq!(
            effects(&p)[1],
            KnitEffect::Diagnostic(Diagnostic::ReleaseWrongHook)
        );
    }

    #[test]
    fn out_while_hooked_is_fatal() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::Inhook { carriers: cs(&[c]) }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(1)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let es = effects(&p);
        assert_eq!(
            es[2],
            KnitEffect::Diagnostic(Diagnostic::OutWhileHooked { carrier: c })
        );
        assert_eq!(
            es[3],
            KnitEffect::CarrierOut {
                carrier: c,
                hooked: false
            }
        );
    }

    #[test]
    fn out_inactive_carrier_is_fatal() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(table, vec![base(BaseOperation::Out { carriers: cs(&[c]) })]);
        assert_eq!(
            effects(&p)[0],
            KnitEffect::Diagnostic(Diagnostic::InactiveCarrier { carrier: c })
        );
    }

    #[test]
    fn out_unstitched_carrier_warns() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let es = effects(&p);
        assert_eq!(
            es[1],
            KnitEffect::Diagnostic(Diagnostic::UnstitchedCarrier { carrier: c })
        );
        assert_eq!(
            es[2],
            KnitEffect::CarrierOut {
                carrier: c,
                hooked: false
            }
        );
    }

    #[test]
    fn outhook_while_hooked_is_fatal() {
        let mut table = CarrierTable::new();
        let c1 = table.make_carrier("1", ());
        let c2 = table.make_carrier("2", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In {
                    carriers: cs(&[c1, c2]),
                }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(1)),
                    carriers: cs(&[c1]),
                }),
                base(BaseOperation::Inhook {
                    carriers: cs(&[c2]),
                }),
                base(BaseOperation::Outhook {
                    carriers: cs(&[c1]),
                }),
            ],
        );
        let es = effects(&p);
        // outhook fires HookInUse (hook holds c2) but still takes c1 out
        assert!(es.contains(&KnitEffect::Diagnostic(Diagnostic::HookInUse)));
        assert!(es.contains(&KnitEffect::CarrierOut {
            carrier: c1,
            hooked: true
        }));
    }

    // --- loop-forming operations -----------------------------------------

    #[test]
    fn tuck_adds_a_loop() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(3)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(3)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let log = run(&p);
        assert_eq!(
            log[1].0,
            KnitEffect::LoopFormed {
                needle: front(3),
                direction: right(),
                carriers: vec![c],
            }
        );
        // two tucks stack up to two loops
        assert_eq!(log[2].1.needle_loop_counts(&front(3)), 2);
        // the carrier's logical position tracks the needle and direction it followed
        let pos = log[2]
            .1
            .carrier_state(&c)
            .unwrap()
            .logical_position()
            .unwrap();
        assert_eq!(pos.needle(), &front(3));
        assert_eq!(pos.side(), &right());
    }

    #[test]
    fn knit_on_empty_needle_warns_then_forms() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Knit {
                    direction: right(),
                    needle: Either::Left(front(1)),
                    carriers: cs(&[c]),
                }),
            ],
        );
        let log = run(&p);
        assert_eq!(
            log[1].0,
            KnitEffect::Diagnostic(Diagnostic::EmptyNeedle { needle: front(1) })
        );
        assert_eq!(
            log[2].0,
            KnitEffect::LoopFormed {
                needle: front(1),
                direction: right(),
                carriers: vec![c],
            }
        );
        assert_eq!(log[2].1.needle_loop_counts(&front(1)), 1);
    }

    #[test]
    fn knit_on_occupied_needle_drops_then_forms() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(1)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(1)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Knit {
                    direction: left(),
                    needle: Either::Left(front(1)),
                    carriers: cs(&[c]),
                }),
            ],
        );
        let log = run(&p);
        // knit drops the two existing loops...
        assert_eq!(
            log[3].0,
            KnitEffect::LoopsDropped {
                needle: front(1),
                count: 2
            }
        );
        // ...and forms one new loop
        assert_eq!(
            log[4].0,
            KnitEffect::LoopFormed {
                needle: front(1),
                direction: left(),
                carriers: vec![c],
            }
        );
        assert_eq!(log[4].1.needle_loop_counts(&front(1)), 1);
    }

    #[test]
    fn drop_clears_loops() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(1)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Drop {
                    needle: Either::Left(front(1)),
                }),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let log = run(&p);
        assert_eq!(
            log[2].0,
            KnitEffect::LoopsDropped {
                needle: front(1),
                count: 1
            }
        );
        assert_eq!(log[2].1.needle_loop_counts(&front(1)), 0);
    }

    #[test]
    fn amiss_selects_needle_without_stitching() {
        let table = CarrierTable::new();
        let p = program(
            table,
            vec![base(BaseOperation::Amiss {
                needle: Either::Left(front(4)),
            })],
        );
        let log = run(&p);
        // amiss produces no physical or diagnostic effect...
        assert!(log.is_empty());
        // ...but width still needs to be reachable, so run a follow-up we can inspect
        let table2 = CarrierTable::new();
        let p2 = program(
            table2,
            vec![
                base(BaseOperation::Amiss {
                    needle: Either::Left(front(4)),
                }),
                base(BaseOperation::Pause),
            ],
        );
        let st = final_state(&p2);
        assert_eq!(st.min_index(), Some(4));
        assert_eq!(st.max_index(), Some(4));
    }

    #[test]
    fn miss_moves_carrier_without_loops() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Miss {
                    direction: left(),
                    needle: Either::Left(front(7)),
                    carriers: cs(&[c]),
                }),
            ],
        );
        let log = run(&p);
        assert_eq!(
            log[1].0,
            KnitEffect::CarrierMissed {
                needle: front(7),
                direction: left(),
                carriers: vec![c],
            }
        );
        assert_eq!(log[1].1.needle_loop_counts(&front(7)), 0);
        let pos = log[1]
            .1
            .carrier_state(&c)
            .unwrap()
            .logical_position()
            .unwrap();
        assert_eq!(pos.needle(), &front(7));
    }

    #[test]
    fn operations_on_inactive_carriers_warn() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        // tuck without bringing the carrier in
        let p = program(
            table,
            vec![base(BaseOperation::Tuck {
                direction: right(),
                needle: Either::Left(front(1)),
                carriers: cs(&[c]),
            })],
        );
        let es = effects(&p);
        assert!(
            es.contains(&KnitEffect::Diagnostic(Diagnostic::InactiveCarrier {
                carrier: c
            }))
        );
    }

    // --- transfers --------------------------------------------------------

    #[test]
    fn xfer_moves_loops_and_follows_carriers() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(2)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Xfer {
                    source: Either::Left(front(2)),
                    destination: Either::Left(back(2)),
                }),
            ],
        );
        let log = run(&p);
        assert_eq!(
            log[2].0,
            KnitEffect::LoopsTransferred {
                source: front(2),
                destination: back(2),
                count: 1,
            }
        );
        assert_eq!(log[2].1.needle_loop_counts(&front(2)), 0);
        assert_eq!(log[2].1.needle_loop_counts(&back(2)), 1);
        // the carrier followed its loop from front(2) to back(2)
        let pos = log[2]
            .1
            .carrier_state(&c)
            .unwrap()
            .logical_position()
            .unwrap();
        assert_eq!(pos.needle(), &back(2));
    }

    #[test]
    fn xfer_stacks_onto_existing_destination_loops() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(back(2)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(2)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Xfer {
                    source: Either::Left(front(2)),
                    destination: Either::Left(back(2)),
                }),
            ],
        );
        let st = final_state(&p);
        assert_eq!(st.needle_loop_counts(&back(2)), 2);
        assert_eq!(st.needle_loop_counts(&front(2)), 0);
    }

    #[test]
    fn xfer_empty_needle_warns() {
        let table = CarrierTable::new();
        let p = program(
            table,
            vec![base(BaseOperation::Xfer {
                source: Either::Left(front(2)),
                destination: Either::Left(back(2)),
            })],
        );
        assert_eq!(
            effects(&p)[0],
            KnitEffect::Diagnostic(Diagnostic::EmptyNeedle { needle: front(2) })
        );
    }

    #[test]
    fn xfer_misaligned_is_fatal() {
        let table = CarrierTable::new();
        // same-bed transfer never aligns
        let p = program(
            table,
            vec![base(BaseOperation::Xfer {
                source: Either::Left(front(2)),
                destination: Either::Left(front(3)),
            })],
        );
        assert_eq!(
            effects(&p)[0],
            KnitEffect::Diagnostic(Diagnostic::MisalignedTransfer {
                source: front(2),
                destination: front(3),
            })
        );
    }

    #[test]
    fn split_transfers_then_forms_new_loop() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(2)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Split {
                    direction: right(),
                    source: Either::Left(front(2)),
                    destination: Either::Left(back(2)),
                    carriers: cs(&[c]),
                }),
            ],
        );
        let log = run(&p);
        assert_eq!(
            log[2].0,
            KnitEffect::LoopsTransferred {
                source: front(2),
                destination: back(2),
                count: 1,
            }
        );
        assert_eq!(
            log[3].0,
            KnitEffect::LoopFormed {
                needle: front(2),
                direction: right(),
                carriers: vec![c],
            }
        );
        // destination has the moved loop, source has the fresh one
        assert_eq!(log[3].1.needle_loop_counts(&back(2)), 1);
        assert_eq!(log[3].1.needle_loop_counts(&front(2)), 1);
    }

    #[test]
    fn split_misaligned_is_fatal() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Split {
                    direction: right(),
                    source: Either::Left(front(2)),
                    destination: Either::Left(back(5)),
                    carriers: cs(&[c]),
                }),
            ],
        );
        assert_eq!(
            effects(&p)[1],
            KnitEffect::Diagnostic(Diagnostic::MisalignedTransfer {
                source: front(2),
                destination: back(5),
            })
        );
    }

    #[test]
    fn split_aligns_under_racking() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(5)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Rack { offset: bigdec(2) }),
                // at rack 2, front 5 aligns with back 3
                base(BaseOperation::Xfer {
                    source: Either::Left(front(5)),
                    destination: Either::Left(back(3)),
                }),
            ],
        );
        let st = final_state(&p);
        assert_eq!(st.needle_loop_counts(&back(3)), 1);
        assert_eq!(st.needle_loop_counts(&front(5)), 0);
    }

    // --- settings and misc ------------------------------------------------

    #[test]
    fn rack_reports_from_and_to() {
        let table = CarrierTable::new();
        let p = program(
            table,
            vec![
                base(BaseOperation::Rack { offset: bigdec(3) }),
                base(BaseOperation::Rack { offset: bigdec(-1) }),
            ],
        );
        let log = run(&p);
        assert_eq!(
            log[0].0,
            KnitEffect::Racked {
                from: BigDecimal::from(0),
                to: BigDecimal::from(3),
            }
        );
        assert_eq!(
            log[1].0,
            KnitEffect::Racked {
                from: BigDecimal::from(3),
                to: BigDecimal::from(-1),
            }
        );
        assert_eq!(log[1].1.rack(), &BigDecimal::from(-1));
    }

    #[test]
    fn stitch_sets_pull_values() {
        let table = CarrierTable::new();
        let p = program(
            table,
            vec![base(BaseOperation::Stitch {
                before_pull: bigdec(10),
                after_pull: bigdec(5),
            })],
        );
        assert_eq!(
            effects(&p)[0],
            KnitEffect::StitchSet {
                before: BigDecimal::from(10),
                after: BigDecimal::from(5),
            }
        );
    }

    #[test]
    fn pause_emits_paused() {
        let table = CarrierTable::new();
        let p = program(table, vec![base(BaseOperation::Pause)]);
        assert_eq!(effects(&p)[0], KnitEffect::Paused);
    }

    #[test]
    fn extension_ops_update_settings() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                ext(ExtensionOperation::XStitchNumber { value: myu32(11) }),
                ext(ExtensionOperation::XXferStitchNumber { value: myu32(3) }),
                ext(ExtensionOperation::XSpeedNumber { value: myu32(200) }),
                ext(ExtensionOperation::XSubRollerNumber { value: myu32(7) }),
                ext(ExtensionOperation::XPresserMode {
                    mode: PresserMode::On(()),
                }),
                ext(ExtensionOperation::XRollerAdvance { value: bigdec(42) }),
                ext(ExtensionOperation::XCarrierSpacing { value: myu32(4) }),
                ext(ExtensionOperation::XCarrierStoppingDistance { value: myu32(2) }),
                ext(ExtensionOperation::XXferStyle {
                    style: XferStyle::TwoPass(()),
                }),
                ext(ExtensionOperation::XVisColor {
                    color: mystr("#ff0000"),
                    carriers: cs(&[c]),
                }),
            ],
        );
        let log = run(&p);
        assert_eq!(
            log[0].0,
            KnitEffect::SettingChanged {
                setting: Setting::StitchNumber
            }
        );
        let st = &log[9].1;
        let s = st.settings();
        assert_eq!(s.stitch_number(), Some(11));
        assert_eq!(s.xfer_stitch_number(), Some(3));
        assert_eq!(s.speed(), Some(200));
        assert_eq!(s.sub_roller(), Some(7));
        assert_eq!(s.presser_mode(), Some(PresserMode::On(())));
        assert_eq!(
            s.take_down_roller_advance_base(),
            Some(&BigDecimal::from(42))
        );
        assert_eq!(s.carrier_spacing(), Some(4));
        assert_eq!(s.carrier_stopping_distance(), Some(2));
        assert_eq!(s.xfer_style(), Some(XferStyle::TwoPass(())));
        assert_eq!(s.vis_color(&c), Some("#ff0000"));
        // the vis-color effect tag is distinct from the stitch-number one
        assert_eq!(
            log[9].0,
            KnitEffect::SettingChanged {
                setting: Setting::VisColor
            }
        );
    }

    #[test]
    fn unhandled_extension_is_reported() {
        let table = CarrierTable::new();
        let p = program(
            table,
            vec![
                ext(ExtensionOperation::XEnd),
                ext(ExtensionOperation::XLoopStart { value: myu32(2) }),
            ],
        );
        let es = effects(&p);
        assert_eq!(es[0], KnitEffect::UnhandledExtension);
        assert_eq!(es[1], KnitEffect::UnhandledExtension);
    }

    // --- end of program ---------------------------------------------------

    #[test]
    fn leftover_carriers_reported_in_name_order() {
        let mut table = CarrierTable::new();
        // insert out of alphabetical order to prove the sort runs
        let c2 = table.make_carrier("2", ());
        let c1 = table.make_carrier("1", ());
        let p = program(
            table,
            vec![base(BaseOperation::In {
                carriers: cs(&[c2, c1]),
            })],
        );
        let es = effects(&p);
        assert_eq!(
            es[0],
            KnitEffect::CarrierIn {
                carrier: c2,
                hooked: false
            }
        );
        assert_eq!(
            es[1],
            KnitEffect::CarrierIn {
                carrier: c1,
                hooked: false
            }
        );
        // end-of-program diagnostics come out sorted by carrier name: "1" before "2"
        assert_eq!(
            es[2],
            KnitEffect::Diagnostic(Diagnostic::CarrierNeverOut { carrier: c1 })
        );
        assert_eq!(
            es[3],
            KnitEffect::Diagnostic(Diagnostic::CarrierNeverOut { carrier: c2 })
        );
    }

    #[test]
    fn clean_program_has_no_trailing_diagnostics() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Tuck {
                    direction: right(),
                    needle: Either::Left(front(1)),
                    carriers: cs(&[c]),
                }),
                base(BaseOperation::Out { carriers: cs(&[c]) }),
            ],
        );
        let es = effects(&p);
        assert_eq!(
            es,
            vec![
                KnitEffect::CarrierIn {
                    carrier: c,
                    hooked: false
                },
                KnitEffect::LoopFormed {
                    needle: front(1),
                    direction: right(),
                    carriers: vec![c],
                },
                KnitEffect::CarrierOut {
                    carrier: c,
                    hooked: false
                },
            ]
        );
    }

    // --- backend control flow --------------------------------------------

    #[test]
    fn backend_error_stops_execution() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let p = program(
            table,
            vec![
                base(BaseOperation::In { carriers: cs(&[c]) }),
                base(BaseOperation::Pause),
            ],
        );
        assert!(run_knitout::<AlwaysStop>(&p).is_err());
    }

    #[test]
    fn empty_program_runs_cleanly() {
        let table = CarrierTable::new();
        let p = program(table, vec![]);
        assert!(effects(&p).is_empty());
    }
}
