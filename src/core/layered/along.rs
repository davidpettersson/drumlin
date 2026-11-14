// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Placement along the flow and composition: ranks spaced for margins, edge
//! gaps and reserved labels, then item positions turned into child offsets
//! and the container's size.

use super::{Arranged, Graph, LabelSide, LabelSpot, Layers, Local, Size, LABEL_OFFSET};
use crate::core::geometry::{from_flow, snap, snap_up, Direction, Rect, GRID};

/// The outcome of main-axis placement.
pub(super) struct Along {
    /// Each item's start.
    main: Vec<f64>,
    /// The far end of the last rank.
    extent: f64,
    /// The centre of each rank that holds labels, indexed by rank.
    label_centre: Vec<f64>,
}

/// Main-axis placement, leaving room for margins and edge gaps.
pub(super) fn place_along(local: &Local, layers: &Layers, order: &[Vec<usize>]) -> Along {
    let spacing = &local.spacing;
    let largest = |values: &mut dyn Iterator<Item = f64>| values.fold(0.0, f64::max);
    let mut main = vec![0.0f64; local.count];
    let mut at = 0.0;
    let mut pending_after = 0.0;
    for (r, row) in order.iter().enumerate() {
        let items: Vec<usize> = row.iter().copied().filter(|&x| x < local.count).collect();
        let before = largest(&mut items.iter().map(|&x| local.margin[x].before));
        let extent = largest(&mut items.iter().map(|&x| local.main_size[x]));
        if r == 0 {
            at = before;
        } else {
            let gap = (pending_after + before + spacing.margin_clearance).max(spacing.rank_gap).max(layers.gap_need[r - 1]);
            at += snap_up(gap, GRID);
        }
        for &x in &items {
            main[x] = at + (extent - local.main_size[x]) / 2.0;
        }
        at += extent;
        pending_after = largest(&mut items.iter().map(|&x| local.margin[x].after));
    }
    Along { main, extent: at + pending_after, label_centre: Vec::new() }
}

/// Main-axis placement when every other rank holds labels: the gap between two
/// ranks of items must also fit the labels, which are centred in it.
pub(super) fn place_along_labelled(local: &Local, layers: &Layers, order: &[Vec<usize>]) -> Along {
    let spacing = &local.spacing;
    let largest = |values: &mut dyn Iterator<Item = f64>| values.fold(0.0, f64::max);
    let mut main = vec![0.0f64; local.count];
    let mut label_centre = vec![0.0f64; order.len()];
    let mut at = 0.0;
    let mut pending_after = 0.0;
    for r in (0..order.len()).step_by(2) {
        let items: Vec<usize> = order[r].iter().copied().filter(|&x| x < local.count).collect();
        let before = largest(&mut items.iter().map(|&x| local.margin[x].before));
        let extent = largest(&mut items.iter().map(|&x| local.main_size[x]));
        if r == 0 {
            at = before;
        } else {
            let label = largest(&mut layers.labels.iter().filter(|label| label.rank == r - 1).map(|label| label.main));
            let room = if label > 0.0 { label + 2.0 * spacing.label_gap } else { 0.0 };
            let gap = snap_up((pending_after + before + spacing.margin_clearance).max(spacing.rank_gap).max(layers.gap_need[r - 2]).max(room), GRID);
            label_centre[r - 1] = snap(at + pending_after + (gap - pending_after - before) / 2.0);
            at += gap;
        }
        for &x in &items {
            main[x] = at + (extent - local.main_size[x]) / 2.0;
        }
        at += extent;
        pending_after = largest(&mut items.iter().map(|&x| local.margin[x].after));
    }
    Along { main, extent: at + pending_after, label_centre }
}

/// Reserved labels as `(edge, spot)`, in the coordinates of the item positions.
pub(super) fn label_spots(direction: Direction, layers: &Layers, along: &Along, cross: &[f64]) -> Vec<(usize, LabelSpot)> {
    layers
        .labels
        .iter()
        .map(|label| {
            let (centre, line) = (along.label_centre[label.rank], cross[label.item]);
            let (main_len, cross_len) = (label.main, label.before + label.after - LABEL_OFFSET);
            let cross_start = match label.side {
                LabelSide::Before => line - LABEL_OFFSET - cross_len,
                LabelSide::After => line + LABEL_OFFSET,
            };
            let (x, y) = from_flow(direction, centre - main_len / 2.0, cross_start);
            let (w, h) = from_flow(direction, main_len, cross_len);
            (label.edge, LabelSpot { anchor: from_flow(direction, centre, line), rect: Rect { x, y, w, h } })
        })
        .collect()
}

/// Converts item positions into child offsets and the container's size.
pub(super) fn compose(graph: &Graph, node: usize, arranged: Vec<Arranged>, local: &Local, along: &Along, cross: &[f64], spots: Vec<(usize, LabelSpot)>) -> Arranged {
    let (main, extent) = (&along.main[..], along.extent);
    let spec = &graph.nodes[node];
    let insets = spec.insets;
    let centres = || cross[..local.count].iter().zip(&local.cross_size);
    let label_span = |spot: &LabelSpot| match graph.direction {
        Direction::Down => (spot.rect.x, spot.rect.right()),
        Direction::Right => (spot.rect.y, spot.rect.bottom()),
    };
    let lo = centres().map(|(c, size)| c - size / 2.0).chain(spots.iter().map(|(_, spot)| label_span(spot).0)).fold(f64::MAX, f64::min);
    let hi = centres().map(|(c, size)| c + size / 2.0).chain(spots.iter().map(|(_, spot)| label_span(spot).1)).fold(f64::MIN, f64::max);
    // A label's edge need not lie on the grid, but item positions must stay on it.
    let lo = if spots.is_empty() { lo } else { (lo / GRID).floor() * GRID };
    let start = main.iter().cloned().fold(f64::MAX, f64::min).min(0.0);
    let (w, h) = from_flow(graph.direction, snap_up(extent - start, GRID), snap_up(hi - lo, GRID));
    let step = graph.spacing.size_step;
    let size = Size {
        w: snap_up((w + insets.left + insets.right).max(spec.size.w), step),
        h: snap_up((h + insets.top + insets.bottom).max(spec.size.h), step),
    };
    let children = arranged
        .into_iter()
        .enumerate()
        .map(|(i, child)| {
            let (px, py) = from_flow(graph.direction, main[i] - start, cross[i] - local.cross_size[i] / 2.0 - lo);
            Arranged { at: (insets.left + px, insets.top + py), ..child }
        })
        .collect();
    let shift = |(x, y): (f64, f64)| {
        let (dx, dy) = from_flow(graph.direction, -start, -lo);
        (insets.left + x + dx, insets.top + y + dy)
    };
    let labels = spots
        .into_iter()
        .map(|(edge, spot)| {
            let (x, y) = shift((spot.rect.x, spot.rect.y));
            (edge, LabelSpot { anchor: shift(spot.anchor), rect: Rect { x, y, ..spot.rect } })
        })
        .collect();
    Arranged { node, size, at: (0.0, 0.0), children, labels }
}
