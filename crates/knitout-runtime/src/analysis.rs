use std::collections::{HashMap, HashSet};
use std::convert::Infallible;

use bigdecimal::Zero;
use either::Either;
use knitout_ast::{
    BaseOperation, Bed, Carrier, IsSlider, MyI32, Needle, Operation, Program, Span, Validated,
};
use knitout_semantics::{Backend, Diagnostic, KnitEffect, MachineState, run_knitout};

use crate::config::Thresholds;
use crate::diagnostics::{RuntimeError, RuntimeWarning};
use crate::model::{
    LoopId, LoopInfo, LoopMachine, is_front, is_rightward, is_slider, needle_index, only,
};

/// The runtime diagnostics produced by analyzing a program: warnings (the program still runs) and
/// errors (it does not).
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct RuntimeDiagnostics {
    /// Non-fatal observations.
    pub warnings: Vec<RuntimeWarning>,
    /// Fatal problems that stop the program from running as written.
    pub errors: Vec<RuntimeError>,
}

impl RuntimeDiagnostics {
    /// Whether there are neither warnings nor errors.
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty() && self.errors.is_empty()
    }
}

/// Run the semantics virtual machine over a validated program and collect every runtime warning and
/// error it produces, using the default [`Thresholds`]. Unlike a compiler, this never stops early:
/// it reports everything it can so an editor can surface all of a file's problems at once.
pub fn analyze(program: &Program<Validated>) -> RuntimeDiagnostics {
    analyze_with(program, &Thresholds::default())
}

/// Like [`analyze`], but with caller-supplied [`Thresholds`].
pub fn analyze_with(program: &Program<Validated>, thresholds: &Thresholds) -> RuntimeDiagnostics {
    // `run_knitout` only hands `Backend::register` the program, so the thresholds are passed to it
    // through a scoped thread-local rather than through the call.
    PENDING_THRESHOLDS.with(|slot| slot.set(*thresholds));
    match run_knitout::<Analyzer>(program) {
        Ok(diagnostics) => diagnostics,
        Err(never) => match never {},
    }
}

thread_local! {
    /// The thresholds the next [`Analyzer::register`] should pick up (set by [`analyze_with`]).
    static PENDING_THRESHOLDS: std::cell::Cell<Thresholds> =
        const { std::cell::Cell::new(Thresholds::DEFAULT) };
}

/// The span of the operation at index `i`, if it has one.
fn op_span(program: &Program<Validated>, i: usize) -> Option<Span> {
    program
        .operations()
        .get(i)
        .and_then(|op| op.annotation.span())
}

/// The base operation at index `i`, if it is one.
fn base_op(program: &Program<Validated>, i: usize) -> Option<&BaseOperation<Validated>> {
    match program.operations().get(i).map(|op| &op.value) {
        Some(Operation::BaseOperation(op)) => Some(op),
        _ => None,
    }
}

/// The machine width declared by the `Width:` header, if present.
fn machine_width(program: &Program<Validated>) -> Option<u32> {
    match program.headers() {
        Either::Left(headers) => headers.width().as_ref().map(|w| match w.value.value() {
            Either::Left(width) => *width,
            Either::Right(never) => match *never {},
        }),
        Either::Right(never) => match *never {},
    }
}

/// The generation of the deepest loop among `generations`, for a new loop to inherit.
fn max_parent(generations: impl IntoIterator<Item = u32>) -> Option<u32> {
    generations.into_iter().max()
}

const ADJACENT: i64 = 1;
/// Two loops count as "made close together" if their birth slots are within this many columns.
const BORN_CLOSE: i64 = 2;

/// A [`Backend`] that layers the loop-lineage [`LoopMachine`] over the semantics VM and turns the
/// combined picture into runtime diagnostics.
struct Analyzer {
    thresholds: Thresholds,
    machine: LoopMachine,
    warnings: Vec<RuntimeWarning>,
    errors: Vec<RuntimeError>,
    machine_width: Option<u32>,
    width_reported: bool,
    /// The operation index whose op-start checks have already run.
    current_index: Option<usize>,
    /// The generation a loop formed later in the current operation should inherit.
    pending_parent: Option<u32>,
    in_spans: HashMap<Carrier<Validated>, Option<Span>>,
    hook_span: Option<Span>,
    hooked: HashSet<Carrier<Validated>>,
    gripper_in: HashSet<Carrier<Validated>>,
    stitched: HashSet<Carrier<Validated>>,
    /// Each carrier's last stitch: `(slot, span)`, for long-float detection.
    last_stitch: HashMap<Carrier<Validated>, (i64, Option<Span>)>,
    /// Each carrier's last pick-up stitch: `(needle, span)`, for the laddering check.
    last_pickup: HashMap<Carrier<Validated>, (Needle<Validated>, Option<Span>)>,
    warned_pulldown: HashSet<LoopId>,
    warned_wear: HashSet<(LoopId, LoopId)>,
}

impl Analyzer {
    // ── the once-per-program syntactic pass ──────────────────────────────

    /// Emit the purely-syntactic warnings that depend only on the operation text (not on machine
    /// state or order): a knit/tuck/split written with no carriers, or a miss with no carriers.
    /// These are done up front because an empty `tuck` (an `amiss`) produces no effect, so the
    /// per-effect path would never see it.
    fn syntactic_pass(&mut self, program: &Program<Validated>) {
        for op in program.operations() {
            let Operation::BaseOperation(base) = &op.value else {
                continue;
            };
            let span = op.annotation.span();
            match base {
                BaseOperation::Knit { carriers, .. }
                | BaseOperation::Tuck { carriers, .. }
                | BaseOperation::Split { carriers, .. }
                    if carriers.is_empty() =>
                {
                    self.warnings.push(RuntimeWarning::NoYarnCarrier { span });
                }
                BaseOperation::Miss { carriers, .. } if carriers.is_empty() => {
                    self.warnings.push(RuntimeWarning::MissWithNoYarn { span });
                }
                _ => {}
            }
        }
    }

    // ── op-start (stateful, pre-operation) checks ────────────────────────

    fn on_operation_start(&mut self, program: &Program<Validated>, i: usize) {
        let span = op_span(program, i);
        let Some(op) = base_op(program, i) else {
            return;
        };
        match op {
            BaseOperation::Knit {
                direction,
                needle,
                carriers,
            } if !carriers.is_empty() => {
                let needle = only(needle).clone();
                let carriers = owned_carriers(carriers);
                if self.machine.count(&needle) == 0 {
                    self.warnings
                        .push(RuntimeWarning::KnittingThroughNothing { span });
                }
                self.check_drop_after_pickup(&needle, direction, &carriers, span);
                for &c in &carriers {
                    self.last_pickup.remove(&c);
                    self.long_float(c, &needle, span);
                    self.unstable_feeding(c, &needle, span);
                }
            }
            BaseOperation::Tuck {
                needle, carriers, ..
            } => {
                // With no carriers this is an `amiss`; the loop below simply does nothing.
                let needle = only(needle).clone();
                let carriers = owned_carriers(carriers);
                for &c in &carriers {
                    self.continuous_pickup(c, &needle, span);
                    self.long_float(c, &needle, span);
                    self.unstable_feeding(c, &needle, span);
                }
            }
            BaseOperation::Miss {
                needle, carriers, ..
            } => {
                // A miss moves the carrier but does not itself open a float; just relocate it.
                let needle = only(needle).clone();
                for c in owned_carriers(carriers) {
                    let slot = self.machine.slot(&needle);
                    self.last_stitch.insert(c, (slot, span));
                }
            }
            BaseOperation::Split {
                source,
                destination,
                carriers,
                ..
            } => {
                let source = only(source).clone();
                let destination = only(destination).clone();
                self.transfer_checks(&source, &destination, span);
                if !carriers.is_empty() {
                    let carriers = owned_carriers(carriers);
                    for &c in &carriers {
                        self.last_pickup.remove(&c);
                        self.long_float(c, &source, span);
                        self.unstable_feeding(c, &source, span);
                    }
                }
            }
            BaseOperation::Xfer {
                source,
                destination,
            } => {
                self.transfer_checks(&only(source).clone(), &only(destination).clone(), span);
            }
            _ => {}
        }
    }

    /// A knit that undoes a same-carrier, opposite-direction pick-up is really a drop.
    fn check_drop_after_pickup(
        &mut self,
        needle: &Needle<Validated>,
        direction: &knitout_ast::Direction<Validated>,
        carriers: &[Carrier<Validated>],
        span: Option<Span>,
    ) {
        let Some(top) = self.machine.top(needle) else {
            return;
        };
        let pulled_through = top.pulled_through;
        let top_direction_right = is_rightward(&top.birth_direction);
        let top_carrier = top.carrier;
        let pickup_span = top.birth_span;
        if pulled_through
            || top_direction_right == is_rightward(direction)
            || !carriers.contains(&top_carrier)
        {
            return;
        }
        self.warnings
            .push(RuntimeWarning::DropAfterPickUp { span, pickup_span });
    }

    /// A carrier moving far from its previous stitch opens a long float.
    fn long_float(
        &mut self,
        carrier: Carrier<Validated>,
        needle: &Needle<Validated>,
        span: Option<Span>,
    ) {
        let slot = self.machine.slot(needle);
        if let Some((prev_slot, prev_span)) = self.last_stitch.get(&carrier).copied() {
            let distance = (slot - prev_slot).abs();
            if distance > self.thresholds.long_float {
                self.warnings.push(RuntimeWarning::LongFloat {
                    from_span: prev_span,
                    to_span: span,
                    distance,
                });
            }
        }
        self.last_stitch.insert(carrier, (slot, span));
    }

    /// Two adjacent same-bed pick-up stitches by one carrier ladder the piece.
    fn continuous_pickup(
        &mut self,
        carrier: Carrier<Validated>,
        needle: &Needle<Validated>,
        span: Option<Span>,
    ) {
        if let Some((prev_needle, prev_span)) = self.last_pickup.get(&carrier) {
            if is_front(prev_needle) == is_front(needle)
                && (needle_index(prev_needle) - needle_index(needle)).abs() == ADJACENT as i32
            {
                self.warnings.push(RuntimeWarning::ContinuousPickUpStitch {
                    first_span: *prev_span,
                    second_span: span,
                });
            }
        }
        self.last_pickup.insert(carrier, (needle.clone(), span));
    }

    /// A carrier still on the hook, or freshly off the grippers, knitting onto the back bed feeds
    /// unstably on SWG-N machines.
    fn unstable_feeding(
        &mut self,
        carrier: Carrier<Validated>,
        needle: &Needle<Validated>,
        span: Option<Span>,
    ) {
        let onto_back = !is_front(needle);
        let hooked = self.hooked.contains(&carrier);
        let fresh_gripper = self.gripper_in.contains(&carrier) && !self.stitched.contains(&carrier);
        if onto_back && (hooked || fresh_gripper) {
            self.warnings
                .push(RuntimeWarning::UnstableYarnFeeding { span });
        }
        self.stitched.insert(carrier);
    }

    /// The bed/racking legality and content checks for a split or xfer.
    fn transfer_checks(
        &mut self,
        source: &Needle<Validated>,
        destination: &Needle<Validated>,
        span: Option<Span>,
    ) {
        let opposite_beds = is_front(source) != is_front(destination);
        let both_sliders = is_slider(source) && is_slider(destination);
        if !opposite_beds || both_sliders {
            self.errors
                .push(RuntimeError::TransferWithWrongBeds { span });
            return;
        }
        if self.machine.slot(source) != self.machine.slot(destination) {
            self.errors
                .push(RuntimeError::TransferAtWrongRacking { span });
            return;
        }
        let count = self.machine.count(source);
        if count == 0 {
            self.warnings
                .push(RuntimeWarning::TransferringNothing { span });
            return;
        }
        if count > 1 {
            self.warnings
                .push(RuntimeWarning::TransferWithOverlappedLoops { span, count });
        }
        // Transferring a loop that never pulled through another loop is a pick-up-stitch transfer.
        let any_pickup = self
            .stack_infos(source)
            .into_iter()
            .any(|info| !info.pulled_through);
        if any_pickup {
            self.warnings
                .push(RuntimeWarning::TransferWithPickUpStitch { span });
        }
    }

    /// The loop infos currently on a needle, bottom to top.
    fn stack_infos(&self, needle: &Needle<Validated>) -> Vec<LoopInfo> {
        // Small helper: read the stack without exposing the machine's internals to the checks.
        let count = self.machine.count(needle);
        let mut out = Vec::with_capacity(count as usize);
        // The machine exposes `top`; walk by temporarily... instead, reconstruct via public API.
        // We only need `pulled_through` here, so query the top-most repeatedly is not possible;
        // fall back to the model's iterator.
        for info in self.machine.remaining() {
            if info.current_needle == *needle {
                out.push(info.clone());
            }
        }
        out
    }

    // ── effect-driven model updates + effect-level checks ────────────────

    fn on_loops_dropped(&mut self, needle: &Needle<Validated>, count: u32) {
        let removed = self.machine.drop_from(needle, count);
        // Hold check: any dropped loop whose carrier advanced many rows since its birth.
        for info in &removed {
            let held = self
                .machine
                .frontier_of(&info.carrier)
                .saturating_sub(info.born_frontier);
            if held >= self.thresholds.loop_hold_rows {
                self.warnings.push(RuntimeWarning::LoopHeldTooLong {
                    span: info.birth_span,
                    rows: held,
                });
            }
        }
        self.pending_parent = max_parent(removed.iter().map(|i| i.generation));
    }

    fn on_loops_transferred(
        &mut self,
        program: &Program<Validated>,
        i: usize,
        source: &Needle<Validated>,
        destination: &Needle<Validated>,
    ) {
        let span = op_span(program, i);
        let is_split = matches!(
            base_op(program, i),
            Some(BaseOperation::Split { carriers, .. }) if !carriers.is_empty()
        );
        let dest_before = self.machine.count(destination);
        let moved = self.machine.transfer(source, destination);
        let mut parents = Vec::new();
        for id in &moved {
            let transfers = self.machine.record_transfer(*id);
            if transfers == self.thresholds.continuous_transfer {
                self.warnings
                    .push(RuntimeWarning::ContinuousTransferWithoutKnitting { span, transfers });
            }
            if is_split {
                let split_throughs = self.machine.record_split_through(*id);
                if split_throughs == self.thresholds.unstable_split_through {
                    self.warnings.push(RuntimeWarning::UnstableSplitKnit {
                        span,
                        split_throughs,
                    });
                }
            }
            self.check_lateral_movement(*id, span);
            if let Some(info) = self.machine.get(*id) {
                parents.push(info.generation);
            }
        }
        self.pending_parent = max_parent(parents);
        let new_dest = self.machine.count(destination);
        self.check_max_hold(dest_before, new_dest, span);
    }

    /// Pulldown/take-down overload (a loop moved far laterally) and racking-wear (two yarn-adjacent
    /// loops pulled apart), evaluated after a loop has moved.
    fn check_lateral_movement(&mut self, id: LoopId, span: Option<Span>) {
        let Some(info) = self.machine.get(id) else {
            return;
        };
        let birth_slot = info.birth_slot;
        let current = self.machine.slot(&info.current_needle);
        let neighbours = [info.yarn_prev, info.yarn_next];

        let moved_by = (current - birth_slot).abs();
        if moved_by > self.thresholds.takedown_overload_distance && self.warned_pulldown.insert(id)
        {
            self.warnings
                .push(RuntimeWarning::PulldownTakedownOverload {
                    span,
                    distance: moved_by,
                });
        }
        for neighbour in neighbours.into_iter().flatten() {
            let Some(other) = self.machine.get(neighbour) else {
                continue;
            };
            let other_birth = other.birth_slot;
            let other_current = self.machine.slot(&other.current_needle);
            if (birth_slot - other_birth).abs() > BORN_CLOSE {
                continue;
            }
            let distance = (current - other_current).abs();
            if distance > self.thresholds.racking_wear_distance {
                let key = (id.min(neighbour), id.max(neighbour));
                if self.warned_wear.insert(key) {
                    self.warnings
                        .push(RuntimeWarning::RackingWearingOutYarn { span, distance });
                }
            }
        }
    }

    fn on_loop_formed(
        &mut self,
        needle: &Needle<Validated>,
        direction: &knitout_ast::Direction<Validated>,
        carriers: &[Carrier<Validated>],
        span: Option<Span>,
    ) {
        let Some(&carrier) = carriers.first() else {
            return;
        };
        let parent = self.pending_parent.take();
        self.machine.form(needle, carrier, *direction, span, parent);
        let new_count = self.machine.count(needle);
        self.check_max_hold(new_count.saturating_sub(1), new_count, span);
        self.check_slack_of_edge(needle, span);
    }

    /// Knitting the edge of a tube (a column with loops on both beds) while the beds are racked
    /// stretches the yarn joining the two edge stitches (SWG heuristic).
    fn check_slack_of_edge(&mut self, needle: &Needle<Validated>, span: Option<Span>) {
        if self.machine.rack().is_zero() {
            return;
        }
        let index = needle_index(needle);
        let opposite = Needle::new(
            if is_front(needle) {
                Bed::Back(())
            } else {
                Bed::Front(())
            },
            IsSlider::NotSlider(()),
            MyI32::new(Either::Left(index), ()),
            (),
        );
        if self.machine.count(&opposite) == 0 {
            return;
        }
        if let Some((min, max)) = self.machine.occupied_index_range() {
            if index == min || index == max {
                self.warnings
                    .push(RuntimeWarning::SlackOfEdgeStitch { span });
            }
        }
    }

    fn check_max_hold(&mut self, old: u32, new: u32, span: Option<Span>) {
        let limit = self.thresholds.max_hold_loops;
        if new >= limit && old < limit {
            self.warnings
                .push(RuntimeWarning::MaxHoldLoops { span, count: new });
        }
    }

    // ── semantic diagnostics + carrier bookkeeping ───────────────────────

    fn map_semantic_diagnostic(&mut self, diagnostic: Diagnostic, span: Option<Span>) {
        match diagnostic {
            Diagnostic::InactiveCarrier { .. } => {
                self.errors
                    .push(RuntimeError::UseOfInactiveCarrier { span });
            }
            Diagnostic::CarrierAlreadyIn { .. } => {
                self.errors
                    .push(RuntimeError::CarrierBroughtInTwice { span });
            }
            Diagnostic::HookInUse => {
                self.errors.push(RuntimeError::HookAlreadyInUse { span });
            }
            Diagnostic::ReleaseEmptyHook => {
                self.errors.push(RuntimeError::ReleaseWithoutHook { span });
            }
            Diagnostic::ReleaseWrongHook => {
                self.errors
                    .push(RuntimeError::ReleaseOfWrongCarriers { span });
            }
            Diagnostic::OutWhileHooked { .. } => {
                self.errors.push(RuntimeError::TakeOutWhileHooked { span });
            }
            Diagnostic::UnstitchedCarrier { .. } => {
                self.warnings
                    .push(RuntimeWarning::UnstitchedCarrier { span });
            }
            Diagnostic::CarrierNeverOut { carrier } => {
                let span = self.in_spans.get(&carrier).copied().flatten();
                self.warnings
                    .push(RuntimeWarning::CarrierLeftInAction { span });
            }
            Diagnostic::HookNeverReleased => {
                self.warnings.push(RuntimeWarning::HookLeftEngaged {
                    span: self.hook_span,
                });
            }
            // These are covered more precisely by our own transfer / empty-needle checks.
            Diagnostic::MisalignedTransfer { .. } | Diagnostic::EmptyNeedle { .. } => {}
        }
    }

    fn check_width(&mut self, state: &MachineState, span: Option<Span>) {
        if self.width_reported {
            return;
        }
        let Some(width) = self.machine_width else {
            return;
        };
        let (Some(min), Some(max)) = (state.min_index(), state.max_index()) else {
            return;
        };
        let used = (i64::from(max) - i64::from(min) + 1).max(0) as u32;
        if used > width {
            self.errors.push(RuntimeError::MachineTooWide {
                span,
                needles_used: used,
                machine_width: width,
            });
            self.width_reported = true;
        }
    }

    /// End-of-program: needles still holding loops, plus loops held too long that were never
    /// dropped.
    fn finish_program(&mut self) {
        for info in self.machine.remaining() {
            self.warnings.push(RuntimeWarning::UnnecessaryLoopRemain {
                span: info.birth_span,
            });
            let held = self
                .machine
                .frontier_of(&info.carrier)
                .saturating_sub(info.born_frontier);
            if held >= self.thresholds.loop_hold_rows {
                self.warnings.push(RuntimeWarning::LoopHeldTooLong {
                    span: info.birth_span,
                    rows: held,
                });
            }
        }
    }
}

/// Collect a validated carrier list into owned carriers.
fn owned_carriers(
    carriers: &[Either<Carrier<Validated>, knitout_ast::Never>],
) -> Vec<Carrier<Validated>> {
    carriers.iter().map(|c| *only(c)).collect()
}

impl Backend for Analyzer {
    type Output = RuntimeDiagnostics;
    type Error = Infallible;

    fn register(program: &Program<Validated>) -> Result<Self, Self::Error> {
        let mut analyzer = Analyzer {
            thresholds: PENDING_THRESHOLDS.with(|slot| slot.get()),
            machine: LoopMachine::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            machine_width: machine_width(program),
            width_reported: false,
            current_index: None,
            pending_parent: None,
            in_spans: HashMap::new(),
            hook_span: None,
            hooked: HashSet::new(),
            gripper_in: HashSet::new(),
            stitched: HashSet::new(),
            last_stitch: HashMap::new(),
            last_pickup: HashMap::new(),
            warned_pulldown: HashSet::new(),
            warned_wear: HashSet::new(),
        };
        analyzer.syntactic_pass(program);
        Ok(analyzer)
    }

    fn observe(
        &mut self,
        effect: KnitEffect,
        state: &MachineState,
        operation_index: usize,
        program: &Program<Validated>,
    ) -> Option<Self::Error> {
        let span = op_span(program, operation_index);
        // A new operation is starting (the end-of-program index has no base op, so
        // `on_operation_start` is a no-op there — no explicit bounds check needed).
        if self.current_index != Some(operation_index) {
            self.current_index = Some(operation_index);
            self.pending_parent = None;
            self.on_operation_start(program, operation_index);
        }
        match effect {
            KnitEffect::CarrierIn { carrier, hooked } => {
                self.in_spans.insert(carrier, span);
                if hooked {
                    self.hook_span = span;
                    self.hooked.insert(carrier);
                } else {
                    self.gripper_in.insert(carrier);
                }
            }
            KnitEffect::HookReleased { carriers } => {
                for carrier in carriers {
                    self.hooked.remove(&carrier);
                }
            }
            KnitEffect::CarrierOut { carrier, .. } => {
                self.hooked.remove(&carrier);
                self.gripper_in.remove(&carrier);
            }
            KnitEffect::Racked { to, .. } => {
                self.machine.set_rack(to);
            }
            KnitEffect::LoopsDropped { needle, count } => {
                self.on_loops_dropped(&needle, count);
            }
            KnitEffect::LoopsTransferred {
                source,
                destination,
                ..
            } => {
                self.on_loops_transferred(program, operation_index, &source, &destination);
            }
            KnitEffect::LoopFormed {
                needle,
                direction,
                carriers,
            } => {
                self.on_loop_formed(&needle, &direction, &carriers, span);
            }
            KnitEffect::Diagnostic(diagnostic) => self.map_semantic_diagnostic(diagnostic, span),
            KnitEffect::CarrierMissed { .. }
            | KnitEffect::StitchSet { .. }
            | KnitEffect::SettingChanged { .. }
            | KnitEffect::Paused
            | KnitEffect::UnhandledExtension => {}
        }
        self.check_width(state, span);
        None
    }

    fn finish(mut self) -> Result<Self::Output, Self::Error> {
        self.finish_program();
        Ok(RuntimeDiagnostics {
            warnings: self.warnings,
            errors: self.errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
    use knitout_ast::{
        Annotated, Bed, CarrierTable, CarriersHeaderValue, Direction, Headers, IsSlider,
        MyBigDecimal, MyI32, MyU32, Needle, Operation, Provenance,
    };

    // ── builders ─────────────────────────────────────────────────────────

    fn needle(bed: Bed<Validated>, slider: bool, index: i32) -> Needle<Validated> {
        Needle::new(
            bed,
            if slider {
                IsSlider::Slider(())
            } else {
                IsSlider::NotSlider(())
            },
            MyI32::new(Either::Left(index), ()),
            (),
        )
    }
    fn f(index: i32) -> Needle<Validated> {
        needle(Bed::Front(()), false, index)
    }
    fn b(index: i32) -> Needle<Validated> {
        needle(Bed::Back(()), false, index)
    }
    fn fs(index: i32) -> Needle<Validated> {
        needle(Bed::Front(()), true, index)
    }
    fn bs(index: i32) -> Needle<Validated> {
        needle(Bed::Back(()), true, index)
    }
    fn pos() -> Direction<Validated> {
        Direction::Right(())
    }
    fn neg() -> Direction<Validated> {
        Direction::Left(())
    }
    fn cs(carriers: &[Carrier<Validated>]) -> Vec<Either<Carrier<Validated>, knitout_ast::Never>> {
        carriers.iter().map(|c| Either::Left(*c)).collect()
    }
    fn op(base: BaseOperation<Validated>) -> Operation<Validated> {
        Operation::BaseOperation(base)
    }
    fn in_(c: &[Carrier<Validated>]) -> Operation<Validated> {
        op(BaseOperation::In { carriers: cs(c) })
    }
    fn inhook(c: &[Carrier<Validated>]) -> Operation<Validated> {
        op(BaseOperation::Inhook { carriers: cs(c) })
    }
    fn out(c: &[Carrier<Validated>]) -> Operation<Validated> {
        op(BaseOperation::Out { carriers: cs(c) })
    }
    fn knit(
        d: Direction<Validated>,
        n: Needle<Validated>,
        c: &[Carrier<Validated>],
    ) -> Operation<Validated> {
        op(BaseOperation::Knit {
            direction: d,
            needle: Either::Left(n),
            carriers: cs(c),
        })
    }
    fn tuck(
        d: Direction<Validated>,
        n: Needle<Validated>,
        c: &[Carrier<Validated>],
    ) -> Operation<Validated> {
        op(BaseOperation::Tuck {
            direction: d,
            needle: Either::Left(n),
            carriers: cs(c),
        })
    }
    fn miss(
        d: Direction<Validated>,
        n: Needle<Validated>,
        c: &[Carrier<Validated>],
    ) -> Operation<Validated> {
        op(BaseOperation::Miss {
            direction: d,
            needle: Either::Left(n),
            carriers: cs(c),
        })
    }
    fn split(
        d: Direction<Validated>,
        src: Needle<Validated>,
        dst: Needle<Validated>,
        c: &[Carrier<Validated>],
    ) -> Operation<Validated> {
        op(BaseOperation::Split {
            direction: d,
            source: Either::Left(src),
            destination: Either::Left(dst),
            carriers: cs(c),
        })
    }
    fn xfer(src: Needle<Validated>, dst: Needle<Validated>) -> Operation<Validated> {
        op(BaseOperation::Xfer {
            source: Either::Left(src),
            destination: Either::Left(dst),
        })
    }
    fn drop(n: Needle<Validated>) -> Operation<Validated> {
        op(BaseOperation::Drop {
            needle: Either::Left(n),
        })
    }
    fn rack(offset: i64) -> Operation<Validated> {
        op(BaseOperation::Rack {
            offset: MyBigDecimal::new(Either::Left(BigDecimal::from(offset)), ()),
        })
    }

    fn program(
        table: CarrierTable,
        width: Option<u32>,
        ops: Vec<Operation<Validated>>,
    ) -> Program<Validated> {
        let mut headers = Headers::new(Either::Left(Annotated::unit(CarriersHeaderValue::new(
            Either::Left(vec![]),
            (),
        ))));
        if let Some(w) = width {
            headers.set_width(Annotated::new(MyU32::new(Either::Left(w), ()), ()));
        }
        let mut program = Program::new(
            Either::Left(Annotated::unit(2u32)),
            Either::Left(headers),
            Vec::new(),
            table,
        );
        for (i, o) in ops.into_iter().enumerate() {
            program.push_operation(Annotated::new(
                o,
                Provenance::new(Some(Span::new(i, i + 1)), None),
            ));
        }
        program
    }

    /// Build a program (with one carrier "1" plus any extra) and analyze it with default thresholds.
    fn run(
        width: Option<u32>,
        build: impl FnOnce(&mut CarrierTable) -> Vec<Operation<Validated>>,
    ) -> RuntimeDiagnostics {
        let mut table = CarrierTable::new();
        let ops = build(&mut table);
        analyze(&program(table, width, ops))
    }

    fn warns(d: &RuntimeDiagnostics, f: impl Fn(&RuntimeWarning) -> bool) -> bool {
        d.warnings.iter().any(f)
    }
    fn errs(d: &RuntimeDiagnostics, f: impl Fn(&RuntimeError) -> bool) -> bool {
        d.errors.iter().any(f)
    }

    // ── syntactic warnings ───────────────────────────────────────────────

    #[test]
    fn no_yarn_carrier_on_empty_knit_tuck_split() {
        // Empty tuck emits no VM effect, so this exercises the syntactic pass specifically.
        let d = run(None, |t| {
            let _ = t;
            vec![tuck(pos(), f(0), &[]), knit(pos(), f(1), &[])]
        });
        let count = d
            .warnings
            .iter()
            .filter(|w| matches!(w, RuntimeWarning::NoYarnCarrier { .. }))
            .count();
        assert_eq!(count, 2, "{d:?}");
    }

    #[test]
    fn miss_with_no_yarn() {
        let d = run(None, |_t| vec![miss(pos(), f(0), &[])]);
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::MissWithNoYarn { .. }
        )));
    }

    // ── empty-needle operations ──────────────────────────────────────────

    #[test]
    fn knitting_through_nothing() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), knit(pos(), f(0), &[c])]
        });
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::KnittingThroughNothing { .. }
        )));
    }

    #[test]
    fn transferring_nothing() {
        let d = run(None, |_t| vec![xfer(f(0), b(0))]);
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::TransferringNothing { .. }
        )));
    }

    // ── floats / pickups ─────────────────────────────────────────────────

    #[test]
    fn long_float_reports_distance_and_both_ends() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), tuck(pos(), f(10), &[c])]
        });
        assert!(
            d.warnings.iter().any(|w| matches!(
                w,
                RuntimeWarning::LongFloat {
                    distance: 10,
                    from_span: Some(_),
                    to_span: Some(_)
                }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn long_float_boundary() {
        // Exactly 6 apart is not a long float; 7 is.
        let ok = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), tuck(pos(), f(6), &[c])]
        });
        assert!(!warns(&ok, |w| matches!(
            w,
            RuntimeWarning::LongFloat { .. }
        )));
        let bad = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), tuck(pos(), f(7), &[c])]
        });
        assert!(warns(&bad, |w| matches!(
            w,
            RuntimeWarning::LongFloat { distance: 7, .. }
        )));
    }

    #[test]
    fn miss_relocates_carrier_so_no_false_float() {
        // A miss between two stitches moves the carrier, so the float is measured from the miss.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]),
                miss(pos(), f(10), &[c]),
                tuck(pos(), f(11), &[c]),
            ]
        });
        assert!(
            !warns(&d, |w| matches!(w, RuntimeWarning::LongFloat { .. })),
            "{d:?}"
        );
        // The miss itself carried a yarn, so it is not a "miss with no yarn".
        assert!(!warns(&d, |w| matches!(
            w,
            RuntimeWarning::MissWithNoYarn { .. }
        )));
    }

    #[test]
    fn continuous_pickup_stitch() {
        // Adjacent non-zero indices (f1, f2) so the |prev - cur| == 1 test is exercised.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(1), &[c]), tuck(pos(), f(2), &[c])]
        });
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::ContinuousPickUpStitch { .. }
        )));
    }

    #[test]
    fn non_adjacent_pickups_are_fine() {
        // Same bed but 5 apart is not a continuous pick-up.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), tuck(pos(), f(5), &[c])]
        });
        assert!(!warns(&d, |w| matches!(
            w,
            RuntimeWarning::ContinuousPickUpStitch { .. }
        )));
    }

    #[test]
    fn drop_after_pickup() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), knit(neg(), f(0), &[c])]
        });
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::DropAfterPickUp { .. }
        )));
    }

    #[test]
    fn drop_after_pickup_needs_opposite_direction() {
        // Knitting the pick-up in the SAME direction is a normal knit, not a drop.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), knit(pos(), f(0), &[c])]
        });
        assert!(!warns(&d, |w| matches!(
            w,
            RuntimeWarning::DropAfterPickUp { .. }
        )));
    }

    #[test]
    fn drop_after_pickup_needs_a_pickup_loop() {
        // The loop on the needle was pulled through (a real knit), so knitting it off is normal.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]),
                knit(pos(), f(0), &[c]), // now f0 holds a pulled-through loop
                knit(neg(), f(0), &[c]),
            ]
        });
        assert!(!warns(&d, |w| matches!(
            w,
            RuntimeWarning::DropAfterPickUp { .. }
        )));
    }

    #[test]
    fn empty_knit_on_empty_needle_is_not_knitting_through_nothing() {
        // `knit` with no carriers is a drop; on an empty needle it must not warn about knitting
        // through nothing (only NoYarnCarrier applies).
        let d = run(None, |_t| vec![knit(pos(), f(0), &[])]);
        assert!(
            !warns(&d, |w| matches!(
                w,
                RuntimeWarning::KnittingThroughNothing { .. }
            )),
            "{d:?}"
        );
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::NoYarnCarrier { .. }
        )));
    }

    // ── loop stacking / remains ──────────────────────────────────────────

    #[test]
    fn max_hold_loops_warns_once_at_four() {
        // Five tucks on one needle: the warning fires exactly once, when the count reaches 4.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]),
                tuck(pos(), f(0), &[c]),
                tuck(pos(), f(0), &[c]),
                tuck(pos(), f(0), &[c]),
                tuck(pos(), f(0), &[c]),
            ]
        });
        let hits: Vec<_> = d
            .warnings
            .iter()
            .filter(|w| matches!(w, RuntimeWarning::MaxHoldLoops { .. }))
            .collect();
        assert_eq!(hits.len(), 1, "{d:?}");
        assert!(matches!(
            hits[0],
            RuntimeWarning::MaxHoldLoops { count: 4, .. }
        ));
    }

    #[test]
    fn three_loops_is_not_max_hold() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]),
                tuck(pos(), f(0), &[c]),
                tuck(pos(), f(0), &[c]),
            ]
        });
        assert!(!warns(&d, |w| matches!(
            w,
            RuntimeWarning::MaxHoldLoops { .. }
        )));
    }

    #[test]
    fn unnecessary_loop_remain() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), out(&[c])]
        });
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::UnnecessaryLoopRemain { .. }
        )));
    }

    #[test]
    fn transfer_with_overlapped_loops() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(2), &[c]),
                tuck(pos(), f(2), &[c]),
                xfer(f(2), b(2)),
            ]
        });
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::TransferWithOverlappedLoops { count: 2, .. }
        )));
    }

    #[test]
    fn transfer_with_pickup_stitch() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), xfer(f(0), b(0))]
        });
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::TransferWithPickUpStitch { .. }
        )));
        // A single loop is not "overlapped".
        assert!(!warns(&d, |w| matches!(
            w,
            RuntimeWarning::TransferWithOverlappedLoops { .. }
        )));
    }

    #[test]
    fn transfer_aligned_under_racking_has_no_error() {
        // At rack 2, f5 lines up with b3; this is a legal, aligned transfer.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(5), &[c]),
                rack(2),
                xfer(f(5), b(3)),
            ]
        });
        assert!(
            !errs(&d, |e| matches!(
                e,
                RuntimeError::TransferAtWrongRacking { .. }
                    | RuntimeError::TransferWithWrongBeds { .. }
            )),
            "{d:?}"
        );
    }

    // ── lineage: hold / continuous transfer / unstable split ─────────────

    #[test]
    fn loop_held_too_long() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]), // loop we will hold
                tuck(pos(), f(1), &[c]), // build a wale on f1: gen 0
                knit(neg(), f(1), &[c]), // gen 1
                knit(pos(), f(1), &[c]), // gen 2
                knit(neg(), f(1), &[c]), // gen 3 -> carrier frontier 3
                drop(f(0)),              // the held loop finally comes off after 3 rows
            ]
        });
        assert!(
            warns(&d, |w| matches!(
                w,
                RuntimeWarning::LoopHeldTooLong { rows: 3, .. }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn loop_held_too_long_at_end_of_program() {
        // Same as above, but the held loop is never taken off — the check fires at program end.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]), // held loop, never dropped
                tuck(pos(), f(1), &[c]),
                knit(neg(), f(1), &[c]),
                knit(pos(), f(1), &[c]),
                knit(neg(), f(1), &[c]), // carrier frontier reaches 3
            ]
        });
        assert!(
            warns(&d, |w| matches!(
                w,
                RuntimeWarning::LoopHeldTooLong { rows: 3, .. }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn short_lived_loop_is_not_held_too_long() {
        // A loop knit off after one row is not "held too long".
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]),
                knit(neg(), f(0), &[c]),
                drop(f(0)),
                out(&[c]),
            ]
        });
        assert!(
            !warns(&d, |w| matches!(w, RuntimeWarning::LoopHeldTooLong { .. })),
            "{d:?}"
        );
    }

    #[test]
    fn continuous_transfer_without_knitting() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]),
                xfer(f(0), b(0)),
                xfer(b(0), f(0)),
                xfer(f(0), b(0)),
                xfer(b(0), f(0)),
            ]
        });
        assert!(
            warns(&d, |w| matches!(
                w,
                RuntimeWarning::ContinuousTransferWithoutKnitting { transfers: 4, .. }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn unstable_split_knit() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]),
                split(pos(), f(0), b(0), &[c]), // old loop -> b0, split-through 1
                split(pos(), b(0), f(0), &[c]), // that loop -> f0, split-through 2
            ]
        });
        assert!(
            warns(&d, |w| matches!(
                w,
                RuntimeWarning::UnstableSplitKnit {
                    split_throughs: 2,
                    ..
                }
            )),
            "{d:?}"
        );
    }

    // ── lineage: racking / pulldown ──────────────────────────────────────

    #[test]
    fn racking_wearing_out_yarn() {
        // A at slot 3 and B at slot 5 are yarn-adjacent and born 2 apart; B is then moved to slot
        // 15, so they end up 12 apart.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(3), &[c]),
                tuck(pos(), f(5), &[c]),
                xfer(f(5), b(5)), // B -> b5 (slot 5 at rack 0)
                rack(10),
                xfer(b(5), f(15)), // B -> f15 (slot 15)
            ]
        });
        assert!(
            warns(&d, |w| matches!(
                w,
                RuntimeWarning::RackingWearingOutYarn { distance: 12, .. }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn racking_wear_boundary() {
        // Same setup but B only reaches slot 9, exactly 6 from A — not a wear warning.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(3), &[c]),
                tuck(pos(), f(5), &[c]),
                xfer(f(5), b(5)),
                rack(4),
                xfer(b(5), f(9)), // slot 9, exactly 6 from A at slot 3
            ]
        });
        assert!(
            !warns(&d, |w| matches!(
                w,
                RuntimeWarning::RackingWearingOutYarn { .. }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn pulldown_takedown_overload() {
        // A single loop born at slot 2, moved to slot 15: a lateral move of 13 (> 12).
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(2), &[c]),
                xfer(f(2), b(2)),
                rack(13),
                xfer(b(2), f(15)),
            ]
        });
        assert!(
            warns(&d, |w| matches!(
                w,
                RuntimeWarning::PulldownTakedownOverload { distance: 13, .. }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn pulldown_boundary() {
        // Moving to slot 14 is a lateral move of exactly 12 — not an overload.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(2), &[c]),
                xfer(f(2), b(2)),
                rack(12),
                xfer(b(2), f(14)),
            ]
        });
        assert!(
            !warns(&d, |w| matches!(
                w,
                RuntimeWarning::PulldownTakedownOverload { .. }
            )),
            "{d:?}"
        );
    }

    // ── SWG heuristics ───────────────────────────────────────────────────

    #[test]
    fn unstable_yarn_feeding_off_hook_onto_back() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![inhook(&[c]), tuck(pos(), b(0), &[c])]
        });
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::UnstableYarnFeeding { .. }
        )));
    }

    #[test]
    fn unstable_yarn_feeding_onto_front_is_fine() {
        // Hooked but knitting onto the near (front) bed feeds fine.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![inhook(&[c]), tuck(pos(), f(0), &[c])]
        });
        assert!(!warns(&d, |w| matches!(
            w,
            RuntimeWarning::UnstableYarnFeeding { .. }
        )));
    }

    #[test]
    fn unstable_yarn_feeding_off_grippers_first_stitch_only() {
        // The first stitch off the grippers onto the back bed is unstable; a later one is not.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), b(0), &[c]), tuck(pos(), b(2), &[c])]
        });
        let count = d
            .warnings
            .iter()
            .filter(|w| matches!(w, RuntimeWarning::UnstableYarnFeeding { .. }))
            .count();
        assert_eq!(count, 1, "{d:?}");
    }

    #[test]
    fn slack_of_edge_stitch_at_both_edges() {
        // A tube spanning columns 0 and 5; at nonzero racking, knitting either edge is slack.
        let cast_tube = |t: &mut CarrierTable| -> (Carrier<Validated>, Vec<Operation<Validated>>) {
            let c = t.make_carrier("1", ());
            (
                c,
                vec![
                    in_(&[c]),
                    tuck(pos(), f(0), &[c]),
                    tuck(pos(), b(0), &[c]),
                    tuck(pos(), f(5), &[c]),
                    tuck(pos(), b(5), &[c]),
                    rack(2),
                ],
            )
        };
        let at_min = run(None, |t| {
            let (c, mut ops) = cast_tube(t);
            ops.push(tuck(pos(), f(0), &[c])); // min edge (index 0)
            ops
        });
        assert!(
            warns(&at_min, |w| matches!(
                w,
                RuntimeWarning::SlackOfEdgeStitch { .. }
            )),
            "{at_min:?}"
        );

        let at_max = run(None, |t| {
            let (c, mut ops) = cast_tube(t);
            ops.push(tuck(pos(), f(5), &[c])); // max edge (index 5)
            ops
        });
        assert!(
            warns(&at_max, |w| matches!(
                w,
                RuntimeWarning::SlackOfEdgeStitch { .. }
            )),
            "{at_max:?}"
        );
    }

    // ── transfer errors ──────────────────────────────────────────────────

    #[test]
    fn transfer_wrong_beds_same_bed() {
        let d = run(None, |_t| vec![xfer(f(0), f(1))]);
        assert!(errs(&d, |e| matches!(
            e,
            RuntimeError::TransferWithWrongBeds { .. }
        )));
    }

    #[test]
    fn transfer_wrong_beds_slider_to_slider() {
        let d = run(None, |_t| vec![xfer(fs(0), bs(0))]);
        assert!(errs(&d, |e| matches!(
            e,
            RuntimeError::TransferWithWrongBeds { .. }
        )));
    }

    #[test]
    fn transfer_wrong_racking() {
        let d = run(None, |_t| vec![xfer(f(0), b(1))]);
        assert!(errs(&d, |e| matches!(
            e,
            RuntimeError::TransferAtWrongRacking { .. }
        )));
        assert!(!errs(&d, |e| matches!(
            e,
            RuntimeError::TransferWithWrongBeds { .. }
        )));
    }

    #[test]
    fn slider_transfer_is_allowed() {
        // f <-> bs is a legal bed pair and aligned at rack 0, so no transfer error.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), xfer(f(0), bs(0))]
        });
        assert!(
            !errs(&d, |e| matches!(
                e,
                RuntimeError::TransferWithWrongBeds { .. }
                    | RuntimeError::TransferAtWrongRacking { .. }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn use_of_inactive_carrier() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![knit(pos(), f(0), &[c])]
        });
        assert!(errs(&d, |e| matches!(
            e,
            RuntimeError::UseOfInactiveCarrier { .. }
        )));
    }

    #[test]
    fn machine_too_wide() {
        // Needles 2..=7 span 6 columns, which exceeds a width of 3.
        let d = run(Some(3), |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(2), &[c]),
                tuck(pos(), f(7), &[c]),
                out(&[c]),
            ]
        });
        assert!(errs(&d, |e| matches!(
            e,
            RuntimeError::MachineTooWide {
                needles_used: 6,
                machine_width: 3,
                ..
            }
        )));
    }

    #[test]
    fn width_boundary_exact_fit_is_ok() {
        // Needles 2..=7 span exactly 6; a width of 6 fits.
        let d = run(Some(6), |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(2), &[c]),
                tuck(pos(), f(7), &[c]),
                out(&[c]),
            ]
        });
        assert!(
            !errs(&d, |e| matches!(e, RuntimeError::MachineTooWide { .. })),
            "{d:?}"
        );
    }

    // ── carrier lifecycle (kept from the base version) ───────────────────

    #[test]
    fn unstitched_carrier_and_left_in_action() {
        let unstitched = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), out(&[c])]
        });
        assert!(warns(&unstitched, |w| matches!(
            w,
            RuntimeWarning::UnstitchedCarrier { .. }
        )));

        let left_in = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c])]
        });
        assert!(warns(&left_in, |w| matches!(
            w,
            RuntimeWarning::CarrierLeftInAction { .. }
        )));
    }

    #[test]
    fn hook_left_engaged() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![inhook(&[c]), tuck(pos(), f(0), &[c])]
        });
        assert!(warns(&d, |w| matches!(
            w,
            RuntimeWarning::HookLeftEngaged { .. }
        )));
    }

    // ── thresholds are adjustable ────────────────────────────────────────

    #[test]
    fn custom_threshold_suppresses_long_float() {
        let mut table = CarrierTable::new();
        let c = table.make_carrier("1", ());
        let prog = program(
            table,
            None,
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), tuck(pos(), f(10), &[c])],
        );
        let relaxed = Thresholds {
            long_float: 100,
            ..Thresholds::default()
        };
        let d = analyze_with(&prog, &relaxed);
        assert!(!warns(&d, |w| matches!(
            w,
            RuntimeWarning::LongFloat { .. }
        )));
    }

    #[test]
    fn clean_program_produces_nothing() {
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), f(0), &[c]), drop(f(0)), out(&[c])]
        });
        assert!(d.is_empty(), "{d:?}");
    }

    #[test]
    fn diagnostics_are_not_empty_when_present() {
        let d = run(None, |_t| vec![miss(pos(), f(0), &[])]);
        assert!(!d.warnings.is_empty());
        assert!(!d.is_empty());
    }

    #[test]
    fn split_runs_transfer_checks() {
        // A split (not just an xfer) must still be checked for bed/racking legality.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]),
                split(pos(), f(0), b(1), &[c]),
            ]
        });
        assert!(
            errs(&d, |e| matches!(
                e,
                RuntimeError::TransferAtWrongRacking { .. }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn split_with_carriers_opens_a_long_float() {
        // The new loop lands on the source, so a split moves the carrier there — far from f0 here.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]),
                split(pos(), f(10), b(10), &[c]),
            ]
        });
        assert!(
            warns(&d, |w| matches!(
                w,
                RuntimeWarning::LongFloat { distance: 10, .. }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn unstable_feeding_first_gripper_stitch_warns() {
        // Isolates the "fresh gripper, first stitch onto the back bed" case.
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![in_(&[c]), tuck(pos(), b(0), &[c])]
        });
        assert!(
            warns(&d, |w| matches!(
                w,
                RuntimeWarning::UnstableYarnFeeding { .. }
            )),
            "{d:?}"
        );
    }

    #[test]
    fn racking_wear_ignores_loops_born_far_apart() {
        // A and B are yarn-adjacent but born 5 apart; moving them is not "close together then far".
        let d = run(None, |t| {
            let c = t.make_carrier("1", ());
            vec![
                in_(&[c]),
                tuck(pos(), f(0), &[c]),
                tuck(pos(), f(5), &[c]), // born 5 from A (> BORN_CLOSE)
                xfer(f(5), b(5)),
                rack(15),
                xfer(b(5), f(20)),
            ]
        });
        assert!(
            !warns(&d, |w| matches!(
                w,
                RuntimeWarning::RackingWearingOutYarn { .. }
            )),
            "{d:?}"
        );
    }
}
