// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Slots: connection points that slide along box sides, spread apart and
//! settle toward what they connect to, so straight lines emerge.

use crate::core::geometry::{along_side, snap, Direction, Rect, Side};

const SLOT_ROUNDS: usize = 6;

/// Picks the facing sides for a connection between two separated boxes. Sides
/// along the flow win when the boxes are separated in both axes.
pub fn facing(direction: Direction, a: &Rect, b: &Rect) -> (Side, Side) {
    let horizontal = match () {
        _ if b.x >= a.right() + 1.0 => Some((Side::Right, Side::Left)),
        _ if b.right() + 1.0 <= a.x => Some((Side::Left, Side::Right)),
        _ => None,
    };
    let vertical = match () {
        _ if b.y >= a.bottom() + 1.0 => Some((Side::Bottom, Side::Top)),
        _ if b.bottom() + 1.0 <= a.y => Some((Side::Top, Side::Bottom)),
        _ => None,
    };
    let (first, second) = match direction {
        Direction::Right => (horizontal, vertical),
        Direction::Down => (vertical, horizontal),
    };
    first.or(second).unwrap_or((Side::Right, Side::Left))
}

/// A connection point sliding along one side of a box.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub item: usize,
    pub side: Side,
    /// The position along the side: y for left and right, x for top and bottom.
    pub coord: f64,
}

impl Slot {
    /// A slot at the middle of `side` of `rect`, the box of `item`.
    pub fn centred(item: usize, side: Side, rect: Rect) -> Slot {
        Slot { item, side, coord: along_side(rect.center(), side) }
    }
}

/// Whether slot `i` is the only one on its side of its box.
pub fn lone(slots: &[Slot], i: usize) -> bool {
    !slots.iter().enumerate().any(|(j, s)| j != i && s.item == slots[i].item && s.side == slots[i].side)
}

/// Spreads wanted positions within `[lo, hi]`, at least `spacing` apart, keeping
/// their order. Returns (position, key) pairs.
fn distribute(mut items: Vec<(f64, usize)>, lo: f64, hi: f64, spacing: f64) -> Vec<(f64, usize)> {
    items.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let top = hi.max(lo);
    let forward = items.into_iter().scan(f64::MIN, |previous, (wanted, key)| {
        let coord = snap(wanted.clamp(lo, top)).max(*previous + spacing);
        *previous = coord;
        Some((coord, key))
    });
    let mut backward: Vec<(f64, usize)> = forward
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .scan(top, |limit, (coord, key)| {
            let clamped = coord.min(*limit);
            *limit = clamped - spacing;
            Some((clamped, key))
        })
        .collect();
    backward.reverse();
    backward
}

/// Lets slots settle toward what they connect to, so straight lines emerge.
/// `range` gives the usable span of a side, and `wanted` a slot's preferred
/// position given all current slots.
pub fn settle_slots(slots: Vec<Slot>, spacing: f64, range: impl Fn(usize, Side) -> (f64, f64), wanted: impl Fn(&[Slot], usize) -> f64) -> Vec<Slot> {
    let mut groups: Vec<(usize, Side)> = slots.iter().map(|s| (s.item, s.side)).collect();
    groups.sort();
    groups.dedup();
    (0..SLOT_ROUNDS).flat_map(|_| groups.iter()).fold(slots, |mut slots, &(item, side)| {
        let (lo, hi) = range(item, side);
        let members = slots.iter().enumerate().filter(|(_, s)| s.item == item && s.side == side);
        let items = members.map(|(i, _)| (wanted(&slots, i), i)).collect();
        for (coord, i) in distribute(items, lo, hi, spacing) {
            slots[i].coord = coord;
        }
        slots
    })
}

/// The span each slot may slide along its side during nudging: within `range`,
/// and no nearer than `spacing` to the other slots on that side as they stand.
pub fn slide_ranges(slots: &[Slot], spacing: f64, range: impl Fn(usize, Side) -> (f64, f64)) -> Vec<(f64, f64)> {
    slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let peers = slots.iter().enumerate().filter(|&(j, s)| j != i && s.item == slot.item && s.side == slot.side).map(|(_, s)| s.coord);
            peers.fold(range(slot.item, slot.side), |(lo, hi), c| if c <= slot.coord { (lo.max(c + spacing), hi) } else { (lo, hi.min(c - spacing)) })
        })
        .collect()
}

/// Removes a small jog between two slots: a straight line near a corner beats
/// an 8px kink. `bounds` is indexed by `Slot::item`.
pub fn straighten(mut slots: Vec<Slot>, i: usize, j: usize, bounds: &[Rect], spacing: f64) -> Vec<Slot> {
    let (a, b) = (slots[i].clone(), slots[j].clone());
    let gap = (a.coord - b.coord).abs();
    if a.side.is_vertical_edge() != b.side.is_vertical_edge() || gap == 0.0 || gap > 16.0 {
        return slots;
    }
    let loose = |slot: &Slot| {
        let r = bounds[slot.item];
        if slot.side.is_vertical_edge() { (r.y + 8.0, r.bottom() - 8.0) } else { (r.x + 8.0, r.right() - 8.0) }
    };
    let (ra, rb) = (loose(&a), loose(&b));
    let (lo, hi) = (ra.0.max(rb.0), ra.1.min(rb.1));
    if lo > hi {
        return slots;
    }
    let value = snap((lo + hi) / 2.0).clamp(lo, hi);
    let free = |own: usize| {
        slots.iter().enumerate().all(|(k, s)| k == own || s.item != slots[own].item || s.side != slots[own].side || (s.coord - value).abs() >= spacing)
    };
    if free(i) && free(j) {
        slots[i].coord = value;
        slots[j].coord = value;
    }
    slots
}
