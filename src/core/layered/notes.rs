// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Notes: taken out of their ranks before ordering and placement, and placed
//! beside their anchors last (Drawing principles, Flow: notes come last).

use super::{Layers, Local};
use crate::core::geometry::{snap_up, GRID};

/// The layers with notes taken out of their ranks: they are placed last.
pub(super) fn without_notes(layers: Layers, local: &Local) -> Layers {
    let ranks = layers.ranks.into_iter().map(|row| row.into_iter().filter(|&x| !local.notes.iter().any(|&(n, _)| n == x)).collect()).collect();
    Layers { ranks, ..layers }
}

/// The order with each note in its anchor's rank, for main-axis placement,
/// which must make the rank deep enough for it.
pub(super) fn with_notes(order: &[Vec<usize>], local: &Local, item_rank: &[usize]) -> Vec<Vec<usize>> {
    order.iter().enumerate().map(|(r, row)| row.iter().copied().chain(local.notes.iter().filter(|&&(n, _)| item_rank[n] == r).map(|&(n, _)| n)).collect()).collect()
}

/// The gap between a note and its anchor.
const NOTE_GAP: f64 = 32.0;

/// Places each note beside its anchor, in the anchor's rank, after everything
/// else (Drawing principles, Flow: notes come last). A side is free when the
/// note fits there at its gap from the anchor, keeping the item gap from every
/// other item and the dummy gap from every long edge in the rank. Of two free
/// sides the roomier wins, the far side on a tie. With neither free, the note
/// takes the side it misses by less, and what lies beyond it in the rank moves
/// out by that much; the anchor never moves.
pub(super) fn place_notes(local: &Local, layers: &Layers, order: &[Vec<usize>], item_rank: &[usize], cross: Vec<f64>) -> Vec<f64> {
    let spacing = &local.spacing;
    local.notes.iter().fold((cross, Vec::<usize>::new()), |(mut cross, mut placed), &(note, anchor)| {
        let r = item_rank[anchor];
        let half = local.cross_size[note] / 2.0;
        let (alo, ahi) = (cross[anchor] - local.cross_size[anchor] / 2.0, cross[anchor] + local.cross_size[anchor] / 2.0);
        // Everything else in the rank, as (item, low, high, gap it keeps).
        let others: Vec<(usize, f64, f64, f64)> = order[r]
            .iter()
            .copied()
            .chain(placed.iter().copied().filter(|&n| item_rank[n] == r))
            .filter(|&x| x != anchor)
            .map(|x| match (x < local.count, layers.label_at(x)) {
                (true, _) => (x, cross[x] - local.cross_size[x] / 2.0, cross[x] + local.cross_size[x] / 2.0, spacing.item_gap),
                (false, Some(label)) => (x, cross[x] - label.before, cross[x] + label.after, spacing.dummy_gap),
                (false, None) => (x, cross[x], cross[x], spacing.dummy_gap),
            })
            .collect();
        let left_edge = ((alo - NOTE_GAP - 2.0 * half) / GRID).floor() * GRID;
        let right_edge = ahi + NOTE_GAP;
        // Per side: the note's centre, how far it misses room, and the room left.
        let side = |after: bool| {
            let (lo, hi) = if after { (right_edge, right_edge + 2.0 * half) } else { (left_edge, left_edge + 2.0 * half) };
            let beyond = others.iter().filter(|o| if after { o.1 >= ahi } else { o.2 <= alo });
            let miss = beyond.clone().map(|o| if after { hi + o.3 - o.1 } else { o.2 + o.3 - lo }).fold(0.0, f64::max);
            let room = beyond.map(|o| if after { o.1 - o.3 - hi } else { lo - o.2 - o.3 }).fold(f64::MAX, f64::min);
            ((lo + hi) / 2.0, miss, room)
        };
        let (left, right) = (side(false), side(true));
        let after = match (left.1 > 0.0, right.1 > 0.0) {
            (false, false) => right.2 >= left.2,
            (true, false) => true,
            (false, true) => false,
            (true, true) => right.1 <= left.1,
        };
        let (centre, miss, _) = if after { right } else { left };
        cross[note] = centre;
        let push = snap_up(miss, GRID);
        if push > 0.0 {
            for o in &others {
                match after {
                    true if o.1 >= ahi => cross[o.0] += push,
                    false if o.2 <= alo => cross[o.0] -= push,
                    _ => {}
                }
            }
        }
        placed.push(note);
        (cross, placed)
    }).0
}
