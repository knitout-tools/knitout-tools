//! A loop-lineage model layered on top of the (frozen) `knitout-semantics` virtual machine.
//!
//! `knitout-semantics` tracks loop *counts* per needle. Several runtime diagnostics need loop
//! *identity* and history: how old a loop is, how many times it has been transferred or
//! split-through, whether it ever pulled through another loop, and where it was made. This module
//! reconstructs that by watching the semantics effect stream — it never re-derives the operation
//! semantics itself, it only applies the count-level movements the effects describe at loop
//! granularity, tagging each loop with lineage as it goes.

use std::collections::{HashMap, HashSet};

use bigdecimal::{BigDecimal, RoundingMode, ToPrimitive};
use either::Either;
use knitout_ast::{Bed, Carrier, Direction, IsSlider, Needle, Never, Span, Validated};

/// A unique identifier for a loop within one analysis run.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct LoopId(u64);

/// Everything we know about one loop currently on (or just removed from) the machine.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct LoopInfo {
    /// The carrier that formed this loop.
    pub carrier: Carrier<Validated>,
    /// Source span of the operation that made it.
    pub birth_span: Option<Span>,
    /// The direction the forming operation ran in.
    pub birth_direction: Direction<Validated>,
    /// Whether it was pulled through at least one existing loop when made (a knit/split on an
    /// occupied needle). `false` for a tuck or a knit/split on an empty needle (a "pick-up stitch").
    pub pulled_through: bool,
    /// How many generations deep in its wale this loop is (a proxy for "row").
    pub generation: u32,
    /// How many times this loop has been transferred.
    pub transfers: u32,
    /// How many times this loop has been split-through (been the "old" loop in a split).
    pub split_throughs: u32,
    /// The slot the loop was made at.
    pub birth_slot: i64,
    /// The carrier's frontier (deepest generation) at this loop's birth, for the hold check.
    pub born_frontier: u32,
    /// The needle the loop currently sits on.
    pub current_needle: Needle<Validated>,
    /// The previous loop the same carrier made (the yarn runs from it to here). May reference a loop
    /// that has since been knit off — its frozen death position keeps the float measurable.
    pub yarn_prev: Option<LoopId>,
    /// The next loop the same carrier made after this one. May reference a loop that has since been
    /// knit off — its frozen death position keeps the float measurable.
    pub yarn_next: Option<LoopId>,
    /// The length of the float to `yarn_prev` when this loop was made, cached for the overstretch
    /// check. Travels with the loop into the dead registry so the float stays measurable after death.
    pub prev_float_birth: Option<i64>,
}

/// The loop-lineage model. Field access is crate-internal; the analyzer drives it.
pub(crate) struct LoopMachine {
    /// `floor(rack)` cached as an integer: it is added to every back-bed slot lookup, so keeping
    /// it here avoids redoing `BigDecimal` arithmetic on the hot path.
    rack_floor: i64,
    loops: HashMap<LoopId, LoopInfo>,
    stacks: HashMap<Needle<Validated>, Vec<LoopId>>,
    frontier: HashMap<Carrier<Validated>, u32>,
    carrier_last_loop: HashMap<Carrier<Validated>, LoopId>,
    /// Loops that have been knit off, kept as `(info, frozen_slot)`. Their loops are gone from the
    /// machine, but the yarn still runs to where they were, so a float between a live loop and a dead
    /// one stays measurable from either end. An entry is dropped as soon as it has no live
    /// yarn-neighbour (see [`LoopMachine::prune_if_unreachable`]), which bounds this map to the
    /// loops adjacent to the live work rather than every loop ever knit off.
    dead: HashMap<LoopId, (LoopInfo, i64)>,
    /// The live loops currently on the back bed. Racking only shifts back-bed slots, so only these
    /// loops' floats can change on a rack — the set keeps that re-check cheap.
    back_bed: HashSet<LoopId>,
    next_id: u64,
}

impl LoopMachine {
    pub fn new() -> Self {
        LoopMachine {
            rack_floor: 0,
            loops: HashMap::new(),
            stacks: HashMap::new(),
            frontier: HashMap::new(),
            carrier_last_loop: HashMap::new(),
            dead: HashMap::new(),
            back_bed: HashSet::new(),
            next_id: 0,
        }
    }

    pub fn set_rack(&mut self, rack: BigDecimal) {
        self.rack_floor = floor_rack(&rack);
    }

    /// The racking-adjusted column a needle sits at: front needles at their index, back needles
    /// shifted by `floor(rack)`.
    pub fn slot(&self, needle: &Needle<Validated>) -> i64 {
        let index = i64::from(needle_index(needle));
        if is_front(needle) {
            index
        } else {
            index + self.rack_floor
        }
    }

    /// How many loops sit on a needle.
    pub fn count(&self, needle: &Needle<Validated>) -> u32 {
        self.stacks.get(needle).map_or(0, |s| s.len() as u32)
    }

    /// The top (most recently added) loop on a needle.
    pub fn top(&self, needle: &Needle<Validated>) -> Option<&LoopInfo> {
        let id = self.stacks.get(needle)?.last()?;
        self.loops.get(id)
    }

    /// The ids of the top `count` loops on a needle (topmost first), without removing them.
    pub fn top_n(&self, needle: &Needle<Validated>, count: u32) -> Vec<LoopId> {
        match self.stacks.get(needle) {
            Some(stack) => stack.iter().rev().take(count as usize).copied().collect(),
            None => Vec::new(),
        }
    }

    /// The ids of every loop on a needle, bottom to top.
    pub fn ids_on(&self, needle: &Needle<Validated>) -> &[LoopId] {
        self.stacks.get(needle).map_or(&[], |s| s.as_slice())
    }

    /// The ids of every loop currently on the machine.
    pub fn live_ids(&self) -> Vec<LoopId> {
        self.loops.keys().copied().collect()
    }

    /// The current frontier (deepest generation reached) for a carrier.
    pub fn frontier_of(&self, carrier: &Carrier<Validated>) -> u32 {
        *self.frontier.get(carrier).unwrap_or(&0)
    }

    /// Look up a *live* loop by id.
    pub fn get(&self, id: LoopId) -> Option<&LoopInfo> {
        self.loops.get(&id)
    }

    /// Look up a loop by id whether it is still live or has been knit off (dead loops keep their
    /// lineage so a float to a knit-off loop can be measured from either end).
    pub fn info(&self, id: LoopId) -> Option<&LoopInfo> {
        self.loops
            .get(&id)
            .or_else(|| self.dead.get(&id).map(|(info, _)| info))
    }

    /// Record the birth length of a live loop's float to its predecessor.
    pub fn set_prev_float_birth(&mut self, id: LoopId, birth: i64) {
        if let Some(info) = self.loops.get_mut(&id) {
            info.prev_float_birth = Some(birth);
        }
    }

    /// The current `(slot, is_front)` of a loop, whether it is still on the machine (its live
    /// position, which moves with racking) or has been knit off (its frozen death position).
    pub fn position(&self, id: LoopId) -> Option<(i64, bool)> {
        if let Some(info) = self.loops.get(&id) {
            Some((
                self.slot(&info.current_needle),
                is_front(&info.current_needle),
            ))
        } else {
            let (info, slot) = self.dead.get(&id)?;
            Some((*slot, is_front(&info.current_needle)))
        }
    }

    /// Form a new loop on `needle`. `parent` is the generation of the loop(s) it pulled through
    /// (`None` for a pick-up stitch, which starts a fresh wale at generation 0). Returns the new
    /// loop and updates the carrier's frontier and yarn chain.
    pub fn form(
        &mut self,
        needle: &Needle<Validated>,
        carrier: Carrier<Validated>,
        direction: Direction<Validated>,
        span: Option<Span>,
        parent: Option<u32>,
    ) -> LoopId {
        let generation = match parent {
            Some(g) => g + 1,
            None => 0,
        };
        let frontier = self.frontier.entry(carrier).or_insert(0);
        *frontier = (*frontier).max(generation);
        let born_frontier = *frontier;
        let birth_slot = self.slot(needle);
        // The yarn runs from the carrier's previous stitch — which may already have been knit off;
        // that is fine, its frozen position keeps the float measurable.
        let yarn_prev = self.carrier_last_loop.get(&carrier).copied();
        let id = LoopId(self.next_id);
        self.next_id += 1;
        self.loops.insert(
            id,
            LoopInfo {
                carrier,
                birth_span: span,
                birth_direction: direction,
                pulled_through: parent.is_some(),
                generation,
                transfers: 0,
                split_throughs: 0,
                birth_slot,
                born_frontier,
                current_needle: needle.clone(),
                yarn_prev,
                yarn_next: None,
                prev_float_birth: None,
            },
        );
        if let Some(prev) = yarn_prev {
            if let Some(prev_info) = self.loops.get_mut(&prev) {
                prev_info.yarn_next = Some(id);
            }
        }
        self.carrier_last_loop.insert(carrier, id);
        self.stacks.entry(needle.clone()).or_default().push(id);
        if !is_front(needle) {
            self.back_bed.insert(id);
        }
        id
    }

    /// The live loops currently on the back bed (whose floats a rack can change).
    pub fn back_bed_ids(&self) -> Vec<LoopId> {
        self.back_bed.iter().copied().collect()
    }

    /// Remove the top `count` loops from `needle` and return their info (for knit-through parents
    /// and end-of-life checks). The loops are gone from the machine afterwards.
    pub fn drop_from(&mut self, needle: &Needle<Validated>, count: u32) -> Vec<LoopInfo> {
        let mut ids = Vec::new();
        if let Some(stack) = self.stacks.get_mut(needle) {
            for _ in 0..count {
                match stack.pop() {
                    Some(id) => ids.push(id),
                    None => break,
                }
            }
        }
        let mut removed = Vec::new();
        for id in ids {
            if let Some(info) = self.loops.remove(&id) {
                // Freeze where the loop was so a float to it (now a dead endpoint) stays measurable.
                let frozen_slot = self.slot(&info.current_needle);
                self.back_bed.remove(&id);
                let (prev, next) = (info.yarn_prev, info.yarn_next);
                removed.push(info.clone());
                self.dead.insert(id, (info, frozen_slot));
                // This loop leaving the live machine can strand dead loops that only it kept
                // reachable — itself included if both its neighbours are already gone.
                self.prune_if_unreachable(id);
                if let Some(prev) = prev {
                    self.prune_if_unreachable(prev);
                }
                if let Some(next) = next {
                    self.prune_if_unreachable(next);
                }
            }
        }
        removed
    }

    /// Drop a dead loop from the registry once no float re-check can ever reach it again — i.e. no
    /// live loop names it as a yarn-neighbour *and* it is no longer any carrier's last loop (which
    /// would make the carrier's next stitch link back to it). Otherwise keeping it only leaks memory.
    fn prune_if_unreachable(&mut self, id: LoopId) {
        let Some((info, _)) = self.dead.get(&id) else {
            return;
        };
        let (carrier, prev, next) = (info.carrier, info.yarn_prev, info.yarn_next);
        let prev_live = prev.is_some_and(|p| self.loops.contains_key(&p));
        let next_live = next.is_some_and(|n| self.loops.contains_key(&n));
        let is_carrier_last = self.carrier_last_loop.get(&carrier) == Some(&id);
        if !prev_live && !next_live && !is_carrier_last {
            self.dead.remove(&id);
        }
    }

    /// Move all loops from `source` to `destination`, returning their ids in bottom-to-top order.
    /// Their `current_needle` is updated; birth information is left intact.
    pub fn transfer(
        &mut self,
        source: &Needle<Validated>,
        destination: &Needle<Validated>,
    ) -> Vec<LoopId> {
        let moved = self.stacks.remove(source).unwrap_or_default();
        let destination_is_front = is_front(destination);
        for id in &moved {
            if let Some(info) = self.loops.get_mut(id) {
                info.current_needle = destination.clone();
            }
            if destination_is_front {
                self.back_bed.remove(id);
            } else {
                self.back_bed.insert(*id);
            }
        }
        if !moved.is_empty() {
            self.stacks
                .entry(destination.clone())
                .or_default()
                .extend(moved.iter().copied());
        }
        moved
    }

    /// Bump a loop's transfer count and return the new value.
    pub fn record_transfer(&mut self, id: LoopId) -> u32 {
        let info = self.loops.get_mut(&id).expect("transferred a live loop");
        info.transfers += 1;
        info.transfers
    }

    /// Bump a loop's split-through count and return the new value.
    pub fn record_split_through(&mut self, id: LoopId) -> u32 {
        let info = self.loops.get_mut(&id).expect("split a live loop");
        info.split_throughs += 1;
        info.split_throughs
    }

    /// Every loop still on the machine.
    pub fn remaining(&self) -> impl Iterator<Item = &LoopInfo> {
        self.loops.values()
    }
}

// ─── validated-AST extraction helpers ────────────────────────────────────────

/// Unwrap the left side of an `Either<T, Never>`; the right is uninhabited in `Validated`.
pub(crate) fn only<T>(e: &Either<T, Never>) -> &T {
    match e {
        Either::Left(t) => t,
        Either::Right(never) => match *never {},
    }
}

/// The integer index of a validated needle.
pub(crate) fn needle_index(needle: &Needle<Validated>) -> i32 {
    *only(needle.index().value())
}

/// Whether a validated needle is on the front bed.
pub(crate) fn is_front(needle: &Needle<Validated>) -> bool {
    match needle.bed() {
        Bed::Front(_) => true,
        Bed::Back(_) => false,
        Bed::Other(_, never) => match *never {},
    }
}

/// Whether a validated needle is a slider.
pub(crate) fn is_slider(needle: &Needle<Validated>) -> bool {
    match needle.is_slider() {
        IsSlider::Slider(_) => true,
        IsSlider::NotSlider(_) => false,
        IsSlider::Other(_, never) => match *never {},
    }
}

/// Whether a validated direction is rightward (`+`).
pub(crate) fn is_rightward(direction: &Direction<Validated>) -> bool {
    match direction {
        Direction::Right(_) => true,
        Direction::Left(_) => false,
        Direction::Other(_, never) => match *never {},
    }
}

/// `floor(rack)` as an `i64`.
pub(crate) fn floor_rack(rack: &BigDecimal) -> i64 {
    rack.with_scale_round(0, RoundingMode::Floor)
        .to_i64()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use knitout_ast::{CarrierTable, MyI32};

    fn front(index: i32) -> Needle<Validated> {
        Needle::new(
            Bed::Front(()),
            IsSlider::NotSlider(()),
            MyI32::new(Either::Left(index), ()),
            (),
        )
    }

    #[test]
    fn dead_registry_is_pruned_to_the_live_frontier() {
        // Knitting a tall single-needle column knits off one loop per row. Each knit-off loop's only
        // yarn-neighbour is the loop knit off just before it, which is itself already dead — so no
        // live loop ever references them and the dead registry must stay bounded rather than growing
        // one entry per row (the memory leak this guards against).
        let mut table = CarrierTable::default();
        let c = table.make_carrier::<Validated>("1", ());
        let n = front(0);
        let mut machine = LoopMachine::new();
        machine.form(&n, c, Direction::Right(()), None, None);
        for _ in 0..1000 {
            let parent = machine.top(&n).map(|info| info.generation);
            machine.drop_from(&n, 1);
            machine.form(&n, c, Direction::Right(()), None, parent);
        }
        assert!(
            machine.dead.len() <= 2,
            "dead registry grew unbounded: {} entries after 1000 knit-offs",
            machine.dead.len()
        );
    }
}
