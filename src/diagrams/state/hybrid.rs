// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Transition labels placed after routing: the label hybrid of the Layout
//! levers study (Drawing principles, Layout levers). The layout reserves a spot for every
//! label, and a line with a reserved label is routed through the spot's
//! anchor. That keeps the label beside its line, but a short line then jogs
//! only to pass its anchor. Here each such jogging line is routed again
//! without its waypoint, and its label is placed beside the new line: on the
//! segment and side that read best, clear of all ink, nearer its own line than
//! any other. A label that finds no such place, or whose line gains no
//! straightness or gains a crossing, keeps its reserved spot, and the rest are
//! routed again. The result is kept only when the whole diagram is better:
//! fewer bends, and no count of crossings, overlaps, merged ends or label flaws
//! higher than with every label reserved.

use super::layout::{header_height, Geometry, LABEL_LINE};
use super::model::*;
use super::render::{all_labels, connection_points, PlacedLabel};
use super::routing::{route, Routes};
use crate::core::geometry::{Direction, Point, Rect};
use crate::core::metrics::{self, bends, crossings_between, mark_touches, measure, merged_ends, path_touches, read_labels, rect_to_line, rects_meet, Scene, FAR};
use crate::core::svg::{Anchor, Mark};
use crate::core::text::{small_width, SMALL};

/// Rounds of routing with labels freed, each pinning the labels that failed.
const ROUNDS: usize = 4;
/// How much nearer its own line a label must be than any other line: with
/// the label 8px from its own line, others keep 20px.
const CLEARER: f64 = 12.0;
/// The gap between a label's box and the line beside it, which puts the ink
/// 8px from the line, as for a reserved label: right or left of a vertical
/// segment, above or below a horizontal one.
const BESIDE: f64 = 8.0;
const ABOVE: f64 = 6.0;
const BELOW: f64 = 5.5;
/// How far a segment must reach past each end of a label beside it.
const OVERHANG: f64 = 8.0;
/// Space kept between a placed label's ink and shapes.
const AIR: f64 = 4.0;
/// Space kept between a placed label's ink and other labels: closer, two
/// labels read as one.
const LABEL_AIR: f64 = 8.0;
/// Space kept between a placed label's ink and a container's outline or a
/// region divider, beyond the halo: a label hugging a border reads as a name.
const BORDER_AIR: f64 = 4.0;

/// The routed drawing with its labels settled.
pub struct Drawn {
    /// Reserved labels that stayed. Freed labels are no longer reserved.
    pub geometry: Geometry,
    pub routes: Routes,
    /// The freed labels, placed beside their lines.
    pub placed: Vec<PlacedLabel>,
}

/// The ink box of a label as `render::label_marks` draws it.
pub fn ink(label: &PlacedLabel) -> Rect {
    let w = label.lines.iter().map(|l| small_width(l)).fold(0.0, f64::max);
    let r = label.rect;
    let left = match label.anchor {
        Anchor::Start => r.x,
        Anchor::Middle => r.x + r.w / 2.0 - w / 2.0,
        Anchor::End => r.right() - w,
    };
    let first = r.y + 10.5;
    metrics::ink(left, w, first, first + (label.lines.len().max(1) - 1) as f64 * LABEL_LINE, SMALL)
}

/// The dashed lines between orthogonal regions.
fn dividers(d: &Diagram, g: &Geometry) -> Vec<Vec<Point>> {
    (1..d.nodes.len())
        .flat_map(|n| {
            let r = g.rects[n];
            let top = r.y + header_height(&d.nodes[n]);
            d.regions(n)
                .windows(2)
                .map(|pair| {
                    let (a, b) = (g.rects[pair[0]], g.rects[pair[1]]);
                    match d.direction {
                        Direction::Down => vec![((a.right() + b.x) / 2.0, top), ((a.right() + b.x) / 2.0, r.bottom())],
                        Direction::Right => vec![(r.x, (a.bottom() + b.y) / 2.0), (r.right(), (a.bottom() + b.y) / 2.0)],
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The drawing as a `Scene` with the given labels. Lines are the routed edges,
/// indexed like them, then the self-transition loops. Each label belongs to its
/// line, and a self-transition's label to its loop.
fn scene_with(d: &Diagram, g: &Geometry, r: &Routes, labels: &[PlacedLabel]) -> Scene {
    let loop_line = |e: usize| g.loops.iter().position(|l| l.edge == e).map(|i| d.edges.len() + i);
    let composite = |n: usize| !d.nodes[n].children.is_empty() && d.nodes[n].kind == Kind::State;
    let shown = |n: usize| !matches!(d.nodes[n].kind, Kind::Root | Kind::Region) && !composite(n);
    let headers = (1..d.nodes.len()).filter(|&n| composite(n)).map(|n| Rect { h: header_height(&d.nodes[n]), ..g.rects[n] });
    let heads = (0..d.edges.len()).filter(|&e| d.edges[e].line != Line::NoteLink && d.edges[e].from != d.edges[e].to).map(|e| (e, 10.0, 4.5)).chain((0..g.loops.len()).map(|i| (d.edges.len() + i, 10.0, 4.5)));
    let mut scene = crate::diagrams::connectors::scene(crate::diagrams::connectors::Drawn {
        heads: heads.collect(),
        lines: r.edges.iter().cloned().chain(g.loops.iter().map(|l| l.points.clone())).collect(),
        solids: (1..d.nodes.len()).filter(|&n| shown(n)).map(|n| g.rects[n]).chain(headers).collect(),
        outlines: (1..d.nodes.len()).filter(|&n| composite(n)).map(|n| g.rects[n]).collect(),
        owned: Vec::new(),
        free: connection_points(d, g).into_iter().filter(|m: &Mark| m.ink().is_some()).collect(),
    });
    scene.borders.extend(dividers(d, g));
    let owned = labels.iter().map(|l| (ink(l), Some(loop_line(l.edge).unwrap_or(l.edge))));
    let (inks, owners): (Vec<Rect>, Vec<Option<usize>>) = owned.chain(scene.labels.iter().map(|&r| (r, None))).unzip();
    Scene { labels: inks, owners, ..scene }
}

/// The finished drawing as a `Scene`, for the label measure.
pub fn scene(d: &Diagram, drawn: &Drawn) -> Scene {
    scene_with(d, &drawn.geometry, &drawn.routes, &all_labels(d, &drawn.geometry, &drawn.routes, &drawn.placed))
}

/// The geometry with the labels of `freed` no longer reserved.
fn without(g: &Geometry, freed: &[usize]) -> Geometry {
    Geometry { labels: g.labels.iter().enumerate().map(|(e, spot)| spot.filter(|_| !freed.contains(&e))).collect(), ..g.clone() }
}

/// A label of `size` beside the point `c` of a segment, on one side of it.
fn beside(c: Point, w: f64, h: f64, vertical: bool, second: bool) -> (Rect, Anchor) {
    match (vertical, second) {
        (true, false) => (Rect { x: c.0 + BESIDE, y: c.1 - h / 2.0, w, h }, Anchor::Start),
        (true, true) => (Rect { x: c.0 - BESIDE - w, y: c.1 - h / 2.0, w, h }, Anchor::End),
        (false, false) => (Rect { x: c.0 - w / 2.0, y: c.1 - ABOVE - h, w, h }, Anchor::Middle),
        (false, true) => (Rect { x: c.0 - w / 2.0, y: c.1 + BELOW, w, h }, Anchor::Middle),
    }
}

/// Whether a label's ink touches nothing in the scene: no line, outline,
/// arrowhead, shape or other label.
fn clear(s: &Scene, ink: &Rect) -> bool {
    let strokes = s.lines.iter().all(|p| !path_touches(p, ink)) && s.borders.iter().all(|p| !path_touches(p, &ink.inflate(BORDER_AIR)));
    let marks = s.marks.iter().all(|m| !mark_touches(m, ink));
    let room = ink.inflate(AIR);
    let shapes = s.solids.iter().all(|o| !rects_meet(&room, &o.rect));
    let labels = s.labels.iter().all(|l| !rects_meet(&ink.inflate(LABEL_AIR), l));
    strokes && marks && shapes && labels
}

/// The best place for edge `e`'s label beside its line, or `None` when every
/// place touches ink, sits far from the line or as near another line.
fn place_one(d: &Diagram, g: &Geometry, s: &Scene, e: usize) -> Option<PlacedLabel> {
    let lines = super::layout::label_lines(&d.edges[e]);
    let size = super::layout::label_size(&lines);
    let own = &s.lines[e];
    let flow_down = d.direction == Direction::Down;
    let bounds = g.rects[0];
    let options = own.windows(2).enumerate().flat_map(|(k, w)| {
        let (a, b) = (w[0], w[1]);
        let length = (a.0 - b.0).abs() + (a.1 - b.1).abs();
        let vertical = a.0 == b.0;
        [0.5, 0.35, 0.65, 0.2, 0.8].into_iter().flat_map(move |f: f64| [false, true].into_iter().map(move |second| (k, a, b, length, vertical, f, second)))
    });
    let scored = options.filter_map(|(k, a, b, length, vertical, f, second)| {
        // The label stays beside its segment: no longer than the segment along it.
        let extent = if vertical { size.h } else { size.w };
        if length < extent + 2.0 * OVERHANG {
            return None;
        }
        let c = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
        let (rect, anchor) = beside(c, size.w, size.h, vertical, second);
        let label = PlacedLabel { edge: e, lines: lines.clone(), rect, anchor };
        let box_ = ink(&label);
        let near = rect_to_line(&box_, own);
        let other = (0..s.lines.len()).filter(|&j| j != e).map(|j| rect_to_line(&box_, &s.lines[j])).fold(f64::INFINITY, f64::min);
        // The rest of its own line keeps its distance too, so the label does
        // not sit tight in a corner.
        let rest = own.windows(2).enumerate().filter(|&(j, _)| j != k).map(|(_, w)| rect_to_line(&box_, w)).fold(f64::INFINITY, f64::min);
        let inside = bounds.x <= rect.x && rect.right() <= bounds.right() && bounds.y <= rect.y && rect.bottom() <= bounds.bottom();
        if !(inside && near <= FAR && rest >= near && other >= near + CLEARER && clear(s, &box_)) {
            return None;
        }
        // A label in a downward flow reads best right of a vertical run, and
        // in a rightward flow above a horizontal one (Labels: consistent side).
        let along_flow = vertical == flow_down;
        let side = match (along_flow, second) {
            (true, false) => 0.0,
            (true, true) => 20.0,
            (false, false) => 10.0,
            (false, true) => 15.0,
        };
        let score = side + (f - 0.5).abs() * 40.0 - length.min(240.0) * 0.5 - (other - near).min(40.0) * 0.5;
        Some((score, label))
    });
    scored.fold(None, |best: Option<(f64, PlacedLabel)>, (score, label)| match best {
        Some(b) if b.0 <= score => Some(b),
        _ => Some((score, label)),
    }).map(|(_, label)| label)
}

/// Places the labels of `freed` one by one, each clear of those before it.
/// Returns the placed labels and the edges whose label found no place.
fn place(d: &Diagram, g: &Geometry, r: &Routes, freed: &[usize]) -> (Vec<PlacedLabel>, Vec<usize>) {
    let fixed: Vec<PlacedLabel> = all_labels(d, g, r, &[]).into_iter().filter(|l| !freed.contains(&l.edge) && (g.labels[l.edge].is_some() || d.edges[l.edge].from == d.edges[l.edge].to)).collect();
    let start = scene_with(d, g, r, &fixed);
    let (_, placed, homeless) = freed.iter().fold((start, Vec::new(), Vec::new()), |(mut s, mut placed, mut homeless), &e| {
        match place_one(d, g, &s, e) {
            Some(label) => {
                s.labels.push(ink(&label));
                placed.push(label);
            }
            None => homeless.push(e),
        }
        (s, placed, homeless)
    });
    (placed, homeless)
}

/// Crossings of line `e` with every other line.
fn crossings_of(paths: &[Vec<Point>], e: usize) -> usize {
    (0..paths.len()).filter(|&j| j != e).map(|j| crossings_between(&paths[e], &paths[j])).sum()
}

/// Every count a reader would call a flaw, for the per-diagram check.
fn flaws(d: &Diagram, drawn: &Drawn) -> (metrics::Metrics, usize, usize, metrics::Reading) {
    let paths = &drawn.routes.edges;
    (measure(paths), merged_ends(paths), drawn.routes.failed.len(), read_labels(&scene(d, drawn)))
}

/// Whether `trial` has fewer bends than `base` and no flaw counted more often.
fn better(d: &Diagram, trial: &Drawn, base: &Drawn) -> bool {
    let (tm, tmerged, tfailed, tl) = flaws(d, trial);
    let (bm, bmerged, bfailed, bl) = flaws(d, base);
    let lines = tm.crossings <= bm.crossings && tm.overlaps <= bm.overlaps && tm.bends < bm.bends && tmerged <= bmerged && tfailed <= bfailed;
    let labels = tl.on_lines <= bl.on_lines && tl.on_shapes <= bl.on_shapes && tl.on_labels <= bl.on_labels && tl.ambiguous <= bl.ambiguous && tl.far <= bl.far;
    lines && labels && cramped(d, trial) <= cramped(d, base)
}

/// Arrowheads on a run too short for them (`ends::cramped`).
fn cramped(d: &Diagram, drawn: &Drawn) -> usize {
    let kept: Vec<Vec<Point>> = (0..d.edges.len()).filter(|&e| d.edges[e].line == Line::Transition && d.edges[e].from != d.edges[e].to).map(|e| drawn.routes.edges[e].clone()).collect();
    crate::core::ends::cramped(&kept, &vec![(0.0, 10.0); kept.len()]).len()
}

/// Lines with a reserved label that bend.
pub fn jogging(d: &Diagram, g: &Geometry, routes: &Routes) -> Vec<usize> {
    (0..d.edges.len()).filter(|&e| g.labels[e].is_some() && !d.is_local(&d.edges[e]) && d.edges[e].from != d.edges[e].to && bends(&routes.edges[e]) > 0).collect()
}

/// Routes the diagram, freeing the labels of jogging lines where that is
/// better (see the module comment).
pub fn settle(d: &Diagram, g: Geometry) -> Drawn {
    let routes = route(d, &g);
    let jogging = jogging(d, &g, &routes);
    let base = Drawn { geometry: g, routes, placed: Vec::new() };
    let mut freed = jogging;
    for _ in 0..ROUNDS {
        if freed.is_empty() {
            break;
        }
        let geometry = without(&base.geometry, &freed);
        let routes = route(d, &geometry);
        let (placed, homeless) = place(d, &geometry, &routes, &freed);
        let base_paths = &base.routes.edges;
        let no_gain = |e: usize| bends(&routes.edges[e]) >= bends(&base_paths[e]) || crossings_of(&routes.edges, e) > crossings_of(base_paths, e);
        let pinned: Vec<usize> = freed.iter().copied().filter(|&e| homeless.contains(&e) || no_gain(e)).collect();
        if pinned.is_empty() {
            let trial = Drawn { geometry, routes, placed };
            return if better(d, &trial, &base) { trial } else { base };
        }
        freed.retain(|e| !pinned.contains(e));
    }
    base
}
