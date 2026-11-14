// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! A laid-out activity as a `core::metrics::Scene`, for the quality test.
//!
//! Diamonds and circles are entered at their outline, which lies inside their
//! bounding box, so they stand in as the largest square inside the symbol. A
//! fork bar may span partitions. Guards are free text; names, notes and lane
//! titles must stay inside their shapes.

use super::layout::*;
use crate::core::geometry::{Point, Rect};
use crate::core::metrics::{ink, tip_box, Scene, Solid};
use crate::core::text::{width, wrap, Weight, FONT, SMALL};

const NOTE_LINE: f64 = 15.0;

fn shrink_to(r: Rect, side: f64) -> Rect {
    let (cx, cy) = r.center();
    Rect { x: cx - side / 2.0, y: cy - side / 2.0, w: side, h: side }
}

fn body(node: &Node) -> Solid {
    let r = node.rect;
    match node.kind {
        NodeKind::Decision | NodeKind::Merge => Solid { rect: shrink_to(r, r.w / 2.0), across: false },
        NodeKind::Start | NodeKind::Stop | NodeKind::End => Solid { rect: shrink_to(r, r.w * std::f64::consts::FRAC_1_SQRT_2), across: false },
        NodeKind::Bar => Solid { rect: r, across: true },
        _ => Solid { rect: r, across: false },
    }
}

/// A node's name, and the part of the node it must stay inside.
fn name_text(node: &Node) -> Option<(Rect, Rect)> {
    if node.lines.is_empty() {
        return None;
    }
    let r = node.rect;
    let n = node.lines.len() as f64;
    let first = r.y + (r.h - n * 16.0) / 2.0 + 12.0;
    let w = node.lines.iter().map(|l| width(l, FONT, Weight::Bold)).fold(0.0, f64::max);
    let room = match node.kind {
        NodeKind::Send => Rect { w: r.w - 12.0, ..r },
        NodeKind::Accept => Rect { x: r.x + 12.0, w: r.w - 12.0, ..r },
        _ => r,
    };
    Some((ink(r.x + r.w / 2.0 - w / 2.0, w, first, first + (n - 1.0) * 16.0, FONT), room))
}

fn note_text(note: &NoteBox) -> (Rect, Rect) {
    let lines = wrap(&note.text, SMALL, note.wrap);
    let r = note.rect;
    let first = r.y + (r.h - lines.len() as f64 * NOTE_LINE) / 2.0 + 11.0;
    let w = lines.iter().map(|l| width(l, SMALL, Weight::Regular)).fold(0.0, f64::max);
    (ink(r.x + 12.0, w, first, first + (lines.len() as f64 - 1.0) * NOTE_LINE, SMALL), r)
}

fn guard_ink(label: &Label) -> Rect {
    let first = label.rect.y + 11.0;
    let w = label.lines.iter().map(|l| width(l, SMALL, Weight::Regular)).fold(0.0, f64::max);
    ink(label.rect.x, w, first, first + (label.lines.len() as f64 - 1.0) * 14.0, SMALL)
}

fn band_text(band: &Band) -> (Rect, Rect) {
    let w = width(&band.name, FONT, Weight::Bold);
    let baseline = band.head.y + 21.0;
    (ink(band.head.x + band.head.w / 2.0 - w / 2.0, w, baseline, baseline, FONT), band.head)
}

/// A straight drop from one node to the next, with no guard beside it: in a
/// straight flow these should all be one length. Drops into a merge or a bar
/// are left out, because a shorter branch must stretch to reach them.
fn drop_length(e: &Edge, g: &Layout) -> Option<f64> {
    let [a, b] = e.points[..] else { return None };
    let guarded = g.labels.iter().any(|l| l.rect.x > a.0 && l.rect.x - a.0 <= 8.0 && l.rect.y < a.1.max(b.1) && l.rect.bottom() > a.1.min(b.1));
    let target = g.nodes.iter().find(|n| (n.rect.y - b.1).abs() < 0.5 && b.0 > n.rect.x && b.0 < n.rect.right());
    let stretches = target.is_none_or(|n| matches!(n.kind, NodeKind::Merge | NodeKind::Bar));
    (a.0 == b.0 && b.1 > a.1 && !guarded && !stretches).then_some(b.1 - a.1)
}

pub fn scene(g: &Layout) -> Scene {
    let lines: Vec<Vec<Point>> = g.edges.iter().map(|e| e.points.clone()).chain(g.links.iter().map(|l| l.to_vec())).collect();
    let solids = g
        .nodes
        .iter()
        .map(body)
        .chain(g.notes.iter().map(|n| Solid { rect: n.rect, across: false }))
        .chain(g.bands.iter().map(|b| Solid { rect: b.head, across: false }))
        .collect();
    let borders = g.bands.iter().skip(1).map(|b| vec![(b.rect.x, b.head.bottom()), (b.rect.x, b.rect.bottom())]).collect();
    let texts = g.nodes.iter().filter_map(name_text).chain(g.notes.iter().map(note_text)).chain(g.bands.iter().map(band_text)).collect();
    Scene {
        lines,
        count_bends: true,
        solids,
        backs: Vec::new(),
        rules: Vec::new(),
        borders,
        labels: g.labels.iter().map(guard_ink).collect(),
        marks: g.edges.iter().map(|e| tip_box(&e.points, 10.0, 4.5)).collect(),
        texts,
        gaps: g.edges.iter().filter_map(|e| drop_length(e, g)).collect(),
        owners: Vec::new(),
    }
}
