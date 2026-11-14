// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! A laid-out sequence diagram as a `core::metrics::Scene`, for the quality test.
//!
//! Only what the notation leaves to layout counts. Messages cross lifelines and
//! execution bars by definition, and a self message always has two corners, so
//! neither is a flaw. Text crossing a lifeline, a frame edge or an arrowhead is.

use super::layout::*;
use super::model::*;
use crate::core::geometry::{Point, Rect};
use crate::core::metrics::{ink, tip_box, Scene, Solid};
use crate::core::svg::Anchor;
use crate::core::text::{keyword_width, width, wrap, Weight, FONT, SMALL};

const NOTE_LINE: f64 = 15.0;

fn left_of(x: f64, w: f64, anchor: Anchor) -> f64 {
    match anchor {
        Anchor::Start => x,
        Anchor::Middle => x - w / 2.0,
        Anchor::End => x - w,
    }
}

fn widest(lines: &[String], size: f64, weight: Weight) -> f64 {
    lines.iter().map(|l| width(l, size, weight)).fold(0.0, f64::max)
}

/// Stacked small text whose last baseline is `baseline`, as `render` draws it.
fn small_block(lines: &[String], x: f64, baseline: f64, anchor: Anchor) -> Rect {
    let w = widest(lines, SMALL, Weight::Regular);
    let first = baseline - (lines.len() as f64 - 1.0) * LINE;
    ink(left_of(x, w, anchor), w, first, baseline, SMALL)
}

fn solid(rect: Rect) -> Solid {
    Solid { rect, across: false }
}

fn outline(r: Rect) -> Vec<Vec<Point>> {
    let (x0, y0, x1, y1) = (r.x, r.y, r.right(), r.bottom());
    vec![vec![(x0, y0), (x1, y0)], vec![(x1, y0), (x1, y1)], vec![(x1, y1), (x0, y1)], vec![(x0, y1), (x0, y0)]]
}

/// Head text: names in a box are contained text; an actor's name stands free.
fn head_text(p: &Participant, line: &Lifeline) -> (Vec<(Rect, Rect)>, Vec<Rect>, Solid) {
    let names = wrap(&p.name, FONT, 132.0);
    let keyword = p.stereotype.as_ref().map(|s| format!("\u{ab}{s}\u{bb}"));
    let name_w = widest(&names, FONT, Weight::Bold);
    let cx = line.x;
    let name_box = |first: f64| ink(cx - name_w / 2.0, name_w, first, first + (names.len() as f64 - 1.0) * 16.0, FONT);
    let keyword_box = |baseline: f64| keyword.as_ref().map(|k| small_block(std::slice::from_ref(k), cx, baseline, Anchor::Middle));
    match p.figure {
        Figure::Box => {
            let block = names.len() as f64 * 16.0 + keyword.as_ref().map_or(0.0, |_| 14.0);
            let top = line.head.y + (line.head.h - block) / 2.0;
            let first = top + keyword.as_ref().map_or(0.0, |_| 14.0) + 12.0;
            let texts = std::iter::once(name_box(first)).chain(keyword_box(top + 10.0)).map(|t| (t, line.head)).collect();
            (texts, Vec::new(), solid(line.head))
        }
        Figure::Actor => {
            let bottom = line.head.bottom();
            let text_block = names.len() as f64 * 16.0 + keyword.as_ref().map_or(0.0, |_| 14.0);
            let figure_top = bottom - text_block - 38.0;
            let names_top = bottom - names.len() as f64 * 16.0 + 12.0;
            let labels = std::iter::once(name_box(names_top)).chain(keyword_box(names_top - 18.0)).collect();
            (Vec::new(), labels, solid(Rect { x: cx - 12.0, y: figure_top, w: 24.0, h: 36.0 }))
        }
    }
}

fn note_text(rect: Rect, text: &str, wrap_at: f64) -> (Rect, Rect) {
    let lines = wrap(text, SMALL, wrap_at);
    let first = rect.y + (rect.h - lines.len() as f64 * NOTE_LINE) / 2.0 + 11.0;
    let w = widest(&lines, SMALL, Weight::Regular);
    (ink(rect.x + 12.0, w, first, first + (lines.len() as f64 - 1.0) * NOTE_LINE, SMALL), rect)
}

fn ref_text(r: &DrawnRef) -> (Rect, Rect) {
    let centre = r.rect.x + r.rect.w / 2.0;
    let total = r.lines.len() as f64 * LINE;
    let baseline = r.rect.y + (r.rect.h + total) / 2.0 - 5.0;
    let room = Rect { x: r.tab.right(), w: r.rect.right() - r.tab.right(), ..r.rect };
    (small_block(&r.lines, centre, baseline, Anchor::Middle), room)
}

fn tab_text(tab: Rect, label: &str) -> (Rect, Rect) {
    let w = keyword_width(label);
    let baseline = tab.y + tab.h - 7.0;
    (ink(tab.x + 8.0, w, baseline, baseline, FONT), Rect { w: tab.w - tab_slant(), ..tab })
}

/// Arrowheads and boundary dots.
fn marks(a: &DrawnArrow) -> Vec<Rect> {
    let head = tip_box(&a.points, 10.0, 4.5);
    let dot = a.dot.map(|(x, y)| Rect { x: x - 4.0, y: y - 4.0, w: 8.0, h: 8.0 });
    std::iter::once(head).chain(dot).collect()
}

/// The row of a plain message: one horizontal run with at most one label line,
/// not creating a lifeline (whose head needs room above the row).
fn plain_row(a: &DrawnArrow) -> Option<f64> {
    (a.points.len() == 2 && a.sort != Sort::Create && a.label.as_ref().is_none_or(|l| l.lines.len() == 1)).then_some(a.points[0].1)
}

/// The space between consecutive plain messages with nothing drawn between them.
fn row_gaps(g: &Layout) -> Vec<f64> {
    let events: Vec<f64> = g
        .bars
        .iter()
        .flat_map(|b| [b.rect.y, b.rect.bottom()])
        .chain(g.frames.iter().flat_map(|f| [f.rect.y, f.rect.bottom()].into_iter().chain(f.separators.iter().copied())))
        .chain(g.refs.iter().flat_map(|r| [r.rect.y, r.rect.bottom()]))
        .chain(g.notes.iter().flat_map(|n| [n.rect.y, n.rect.bottom()]))
        .chain(g.crosses.iter().map(|c| c.1))
        .chain(g.lifelines.iter().map(|l| l.head.bottom()))
        .chain(g.arrows.iter().filter(|a| plain_row(a).is_none()).flat_map(|a| a.points.iter().map(|p| p.1)))
        .collect();
    let mut rows: Vec<f64> = g.arrows.iter().filter_map(plain_row).collect();
    rows.sort_by(f64::total_cmp);
    rows.windows(2).filter(|w| !events.iter().any(|&y| y > w[0] + 0.5 && y < w[1] - 0.5)).map(|w| w[1] - w[0]).collect()
}

pub fn scene(d: &Diagram, g: &Layout) -> Scene {
    let heads: Vec<_> = d.participants.iter().zip(&g.lifelines).map(|(p, l)| head_text(p, l)).collect();
    let solids: Vec<Solid> = heads
        .iter()
        .map(|h| h.2)
        .chain(g.notes.iter().map(|n| solid(n.rect)))
        .chain(g.refs.iter().map(|r| solid(r.rect)))
        .chain(g.frames.iter().map(|f| solid(f.tab)))
        .collect();
    let message_labels = g.arrows.iter().filter_map(|a| a.label.as_ref()).map(|l| small_block(&l.lines, l.x, l.baseline, l.anchor));
    let guards = g.frames.iter().flat_map(|f| &f.guards).map(|gd| small_block(&gd.lines, gd.rect.x, gd.rect.bottom() - 5.0, Anchor::Start));
    let labels = message_labels.chain(guards).chain(heads.iter().flat_map(|h| h.1.clone())).collect();
    let texts = heads
        .iter()
        .flat_map(|h| h.0.clone())
        .chain(g.notes.iter().map(|n| note_text(n.rect, &n.text, n.wrap)))
        .chain(g.refs.iter().map(ref_text))
        .chain(g.frames.iter().map(|f| tab_text(f.tab, &f.operator)))
        .collect();
    let borders = g
        .frames
        .iter()
        .flat_map(|f| outline(f.rect).into_iter().chain(f.separators.iter().map(|&y| vec![(f.rect.x, y), (f.rect.right(), y)])))
        .collect();
    let crosses = g.crosses.iter().map(|&(x, y)| Rect { x: x - 7.0, y: y - 7.0, w: 14.0, h: 14.0 });
    Scene {
        lines: g.arrows.iter().map(|a| a.points.clone()).collect(),
        count_bends: false,
        solids,
        backs: g.bars.iter().map(|b| b.rect).collect(),
        // From just below the head, so an actor's name may sit on top of its own lifeline.
        rules: g.lifelines.iter().map(|l| vec![(l.x, l.top + 3.0), (l.x, l.bottom)]).collect(),
        borders,
        labels,
        marks: g.arrows.iter().flat_map(marks).chain(crosses).collect(),
        texts,
        gaps: row_gaps(g),
        owners: Vec::new(),
    }
}
