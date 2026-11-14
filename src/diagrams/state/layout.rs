// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Adapts a state machine to the generic layered layout: symbol sizes, compartments,
//! reserved transition labels, boxes reserved for self-transitions, tiled regions,
//! connection points moved onto borders and fork bars stretched over their targets.

use super::model::*;
use crate::core::geometry::{from_flow, snap_up, to_flow, Direction, Point, Rect};
use crate::core::glyphs::note_size;
use crate::core::layered::{self, Constraint, EdgeLabel, Gap, Graph, Insets, LabelSide, LabelSpot, Node as LayoutNode, Size, Spacing};
use crate::core::svg::Anchor;
use crate::core::text::{name_width, small_width};
use crate::diagrams::boxes;

pub const NAME_ROW: f64 = 32.0;
pub const ACTIVITY_LINE: f64 = 16.0;
pub const LABEL_LINE: f64 = 13.0;
pub const LOOP_DEPTH: f64 = 24.0;
/// The distance between the two legs of a self-transition.
pub const LOOP_SPAN: f64 = 16.0;
const LOOP_PITCH: f64 = 32.0;
const LABEL_WRAP: f64 = 168.0;
const ITEM_GAP: f64 = 40.0;
const BAR_THICKNESS: f64 = 8.0;
const BAR_SLOT: f64 = 24.0;
const REGION_INSET: f64 = 8.0;
const REGION_NAME: f64 = 24.0;
const PADDING: f64 = 24.0;

pub fn label_lines(edge: &Edge) -> Vec<String> {
    edge.label.as_ref().map_or_else(Vec::new, |label| label.lines(LABEL_WRAP))
}

pub fn label_size(lines: &[String]) -> Size {
    let w = lines.iter().map(|line| small_width(line)).fold(0.0, f64::max);
    Size { w: w.ceil() + 2.0, h: lines.len() as f64 * LABEL_LINE + 2.0 }
}

/// The name shown for a state, with its stereotype keyword above it if any.
pub fn keyword(node: &Node) -> Option<String> {
    node.stereotype.as_ref().map(|s| format!("\u{ab}{s}\u{bb}"))
}

/// The height of a state's name and internal behaviours.
pub fn header_height(node: &Node) -> f64 {
    let name = if keyword(node).is_some() { NAME_ROW + 16.0 } else { NAME_ROW };
    let activities = if node.activities.is_empty() { 0.0 } else { node.activities.len() as f64 * ACTIVITY_LINE + 8.0 };
    name + activities
}

fn text_width(node: &Node) -> f64 {
    let name = name_width(&node.name).max(keyword(node).map_or(0.0, |k| small_width(&k)));
    let activities = node.activities.iter().map(|line| small_width(line)).fold(0.0, f64::max);
    (name + 32.0).max(activities + 24.0)
}

fn point_size(kind: Kind) -> f64 {
    match kind {
        Kind::Final | Kind::Shallow | Kind::Deep | Kind::Choice => 24.0,
        _ => 16.0,
    }
}

/// How many edges meet a bar on its busier side, which sets its length.
fn bar_length(d: &Diagram, n: usize) -> f64 {
    let incoming = d.edges.iter().filter(|e| e.to == n).count();
    let outgoing = d.edges.iter().filter(|e| e.from == n).count();
    snap_up((incoming.max(outgoing) as f64 * BAR_SLOT + 16.0).max(64.0), 16.0)
}

fn along_cross(direction: Direction, main: f64, cross: f64) -> Size {
    let (w, h) = from_flow(direction, main, cross);
    Size { w, h }
}

fn self_edges(d: &Diagram, n: usize) -> Vec<usize> {
    (0..d.edges.len()).filter(|&e| d.edges[e].from == n && d.edges[e].to == n).collect()
}

/// The length of the state side that its self-transitions need.
fn loops_span(d: &Diagram, n: usize) -> f64 {
    let loops = self_edges(d, n);
    if loops.is_empty() {
        return 0.0;
    }
    loops.iter().map(|&e| loop_pitch(&d.edges[e])).sum::<f64>() + 16.0
}

fn loop_pitch(edge: &Edge) -> f64 {
    (label_lines(edge).len() as f64 * LABEL_LINE + 8.0).max(LOOP_PITCH)
}

fn state_size(d: &Diagram, n: usize) -> Size {
    let node = &d.nodes[n];
    let span = loops_span(d, n);
    let along_side = |h: f64| h.max(span);
    let incident = d.edges.iter().filter(|e| e.from != e.to && (e.from == n || e.to == n)).count() as f64;
    let room = (incident * 24.0 + 16.0).min(160.0);
    match d.direction {
        Direction::Down => Size { w: snap_up(text_width(node).max(96.0).max(room), 16.0), h: snap_up(along_side(if node.activities.is_empty() { 40.0 } else { header_height(node) + 8.0 }), 16.0) },
        Direction::Right => Size { w: snap_up(text_width(node).max(96.0).max(span), 16.0), h: snap_up(if node.activities.is_empty() { 40.0 } else { header_height(node) + 8.0 }, 16.0).max(snap_up(room, 16.0)) },
    }
}

fn leaf_size(d: &Diagram, n: usize) -> Size {
    let node = &d.nodes[n];
    match node.kind {
        Kind::State => state_size(d, n),
        Kind::Fork | Kind::Join => along_cross(d.direction, BAR_THICKNESS, bar_length(d, n)),
        Kind::Note => {
            let (w, h) = note_size(&node.name);
            Size { w, h }
        }
        Kind::Region | Kind::Root => Size { w: 96.0, h: 40.0 },
        kind => Size { w: point_size(kind), h: point_size(kind) },
    }
}

pub fn insets(d: &Diagram, n: usize) -> Insets {
    let node = &d.nodes[n];
    match node.kind {
        Kind::Root => Insets::default(),
        Kind::Region => Insets { top: if node.name.is_empty() { REGION_INSET } else { REGION_NAME + REGION_INSET }, right: REGION_INSET, bottom: REGION_INSET, left: REGION_INSET },
        _ if d.regions(n).is_empty() => Insets { top: header_height(node) + 8.0, right: PADDING + local_label_room(d, n), bottom: PADDING, left: PADDING },
        _ => Insets { top: header_height(node) + 8.0, right: REGION_INSET, bottom: REGION_INSET, left: REGION_INSET },
    }
}

/// Local transitions run along the inside of the right border, so their labels
/// need room between the contents and that border.
fn local_label_room(d: &Diagram, n: usize) -> f64 {
    let local = d.edges.iter().filter(|e| d.is_local(e) && (e.from == n || e.to == n));
    local.map(|e| label_size(&label_lines(e)).w + 24.0).fold(0.0, f64::max)
}

fn node_spec(d: &Diagram, n: usize, minimum: Size) -> LayoutNode {
    let node = &d.nodes[n];
    let base = if node.children.is_empty() { leaf_size(d, n) } else { Size { w: if node.kind == Kind::State { text_width(node) + 32.0 } else { 0.0 }, h: 0.0 } };
    let size = Size { w: base.w.max(minimum.w), h: base.h.max(minimum.h) };
    LayoutNode { parent: node.parent, size, insets: insets(d, n), margin: layered::Margin::default() }
}

/// A label goes on the far side of a line that runs opposite to another between
/// the same two vertices, so that the pair of lines stays close together.
fn label_side(d: &Diagram, e: usize) -> LabelSide {
    let edge = &d.edges[e];
    let flowing = match d.direction {
        Direction::Down => LabelSide::After,
        Direction::Right => LabelSide::Before,
    };
    let reverse_pair = d.edges.iter().enumerate().any(|(i, other)| i < e && other.from == edge.to && other.to == edge.from);
    match (reverse_pair, flowing) {
        (true, LabelSide::After) => LabelSide::Before,
        (true, LabelSide::Before) => LabelSide::After,
        (false, side) => side,
    }
}

fn edge_spec(d: &Diagram, e: usize) -> layered::Edge {
    let edge = &d.edges[e];
    let lines = label_lines(edge);
    let label = (!lines.is_empty()).then(|| EdgeLabel { size: label_size(&lines), side: label_side(d, e) });
    let keep_straight = d.nodes[edge.from].kind == Kind::Initial || d.nodes[edge.to].kind.is_bar() || d.nodes[edge.from].kind.is_bar();
    layered::Edge { from: edge.from, to: edge.to, weight: if keep_straight { 4 } else { 1 }, gap: Gap::Fixed(0.0), label }
}

/// Initial pseudostates and entry points start their container's flow, and exit
/// points end it. Orthogonal regions share a rank, so they tile across the flow.
fn flow_constraints(d: &Diagram, n: usize) -> Vec<Constraint> {
    let children = &d.nodes[n].children;
    let kind = |c: usize| d.nodes[c].kind;
    let first = children.iter().copied().filter(|&c| matches!(kind(c), Kind::Initial | Kind::EntryPoint));
    let last = children.iter().copied().filter(|&c| kind(c) == Kind::ExitPoint);
    let others = |c: usize| children.iter().copied().filter(move |&o| o != c && !matches!(kind(o), Kind::Initial | Kind::EntryPoint | Kind::ExitPoint | Kind::Note));
    let starts = first.flat_map(move |c| others(c).map(move |o| Constraint::Before(c, o)));
    let ends = last.flat_map(move |c| others(c).map(move |o| Constraint::Before(o, c)));
    let regions = d.regions(n);
    let tiles: Vec<Constraint> = regions.windows(2).flat_map(|pair| [Constraint::SameRank(pair[0], pair[1]), Constraint::AcrossBefore(pair[0], pair[1])]).collect();
    starts.chain(ends).chain(tiles).collect()
}

/// A self-transition's reserved box, one per state with loops, indexed after the nodes.
struct LoopBox {
    state: usize,
    size: Size,
}

/// The size of one state's box of self-transition labels.
fn loop_box(d: &Diagram, n: usize) -> LoopBox {
    let labels: Vec<Size> = self_edges(d, n).iter().map(|&e| label_size(&label_lines(&d.edges[e]))).collect();
    let wide = labels.iter().map(|s| s.w).fold(0.0, f64::max);
    let size = match d.direction {
        Direction::Down => Size { w: snap_up(wide.max(8.0), 8.0), h: snap_up(loops_span(d, n), 8.0) },
        // Below the state; the labels stand in a staircase under the loops.
        Direction::Right => {
            let span = (self_edges(d, n).len() as f64 - 1.0) * LOOP_PITCH + wide;
            Size { w: snap_up(span.max(8.0), 8.0), h: snap_up(labels.iter().map(|s| s.h).sum::<f64>().max(8.0), 8.0) }
        }
    };
    LoopBox { state: n, size }
}

fn graph(d: &Diagram, minimums: &[Size], columns: bool) -> Graph {
    let count = d.nodes.len();
    let boxes: Vec<LoopBox> = (1..count).filter(|&n| d.nodes[n].kind == Kind::State && !self_edges(d, n).is_empty()).map(|n| loop_box(d, n)).collect();
    let box_nodes = boxes.iter().map(|b| LayoutNode { parent: d.nodes[b.state].parent, size: b.size, ..LayoutNode::default() });
    let box_links = boxes.iter().enumerate().flat_map(|(i, b)| [Constraint::SameRank(count + i, b.state), Constraint::Beside { moved: count + i, anchor: b.state }]);
    let note_links = d.edges.iter().filter(|edge| edge.line == Line::NoteLink).map(|edge| Constraint::Note { note: edge.from, anchor: edge.to });
    let flows = (0..count).flat_map(|n| flow_constraints(d, n));
    let pivots = (0..count).filter(|&n| matches!(d.nodes[n].kind, Kind::Choice | Kind::Junction)).map(Constraint::Pivot);
    let bars = (0..count).filter(|&n| d.nodes[n].kind.is_bar()).map(Constraint::Spread);
    Graph {
        direction: d.direction,
        nodes: (0..count).map(|n| node_spec(d, n, minimums[n])).chain(box_nodes).collect(),
        edges: (0..d.edges.len()).map(|e| edge_spec(d, e)).collect(),
        constraints: d.hints.iter().map(boxes::hint_constraint).chain(flows).chain(note_links).chain(box_links).chain(pivots).chain(bars).collect(),
        spacing: Spacing { item_gap: ITEM_GAP, dummy_gap: 24.0, label_gap: 32.0, columns, ..Spacing::default() },
        strategy: d.engine.strategy,
    }
}

/// Everything fixed before routing.
#[derive(Clone, Debug)]
pub struct Geometry {
    /// Indexed like `Diagram::nodes`.
    pub rects: Vec<Rect>,
    /// Reserved labels, indexed like `Diagram::edges`.
    pub labels: Vec<Option<LabelSpot>>,
    pub loops: Vec<SelfLoop>,
}

/// One self-transition: its path outside the state and its label.
#[derive(Clone, Debug)]
pub struct SelfLoop {
    pub edge: usize,
    pub points: Vec<Point>,
    pub lines: Vec<String>,
    pub label: Rect,
}

/// Regions of one state get the same extent along the flow, so they tile.
fn equalised(d: &Diagram, rects: &[Rect]) -> Vec<Size> {
    let main = |r: &Rect| match d.direction {
        Direction::Down => r.h,
        Direction::Right => r.w,
    };
    (0..d.nodes.len())
        .map(|n| {
            let Some(parent) = d.nodes[n].parent.filter(|&p| d.nodes[n].kind == Kind::Region && d.regions(p).len() > 1) else {
                return Size::default();
            };
            let longest = d.regions(parent).iter().map(|&r| main(&rects[r])).fold(0.0, f64::max);
            along_cross(d.direction, longest, 0.0)
        })
        .collect()
}

/// Lays out, then lets regions grow to tile until nothing changes.
fn settle_regions(d: &Diagram, columns: bool) -> layered::Layout {
    let first = layered::layout(&graph(d, &vec![Size::default(); d.nodes.len()], columns));
    (0..4).fold(first, |current, _| layered::layout(&graph(d, &equalised(d, &current.rects), columns)))
}

/// Puts entry and exit points on their state's border, halfway across it.
fn move_connection_points(d: &Diagram, rects: Vec<Rect>) -> Vec<Rect> {
    (0..d.nodes.len()).fold(rects, |mut rects, n| {
        let kind = d.nodes[n].kind;
        if kind.is_connection_point() {
            let parent = rects[d.nodes[n].parent.unwrap()];
            let r = rects[n];
            rects[n] = match (d.direction, kind) {
                (Direction::Down, Kind::EntryPoint) => clear_of_name(d, n, parent, Rect { y: parent.y - r.h / 2.0, ..r }),
                (Direction::Down, _) => Rect { y: parent.bottom() - r.h / 2.0, ..r },
                (Direction::Right, Kind::EntryPoint) => Rect { x: parent.x - r.w / 2.0, ..r },
                (Direction::Right, _) => Rect { x: parent.right() - r.w / 2.0, ..r },
            };
        }
        rects
    })
}

/// Slides an entry point along the top border, off the state's centred name,
/// when the name cannot give way to it (`name_aside`).
fn clear_of_name(d: &Diagram, n: usize, parent: Rect, r: Rect) -> Rect {
    let owner_index = d.nodes[n].parent.unwrap();
    let owner = &d.nodes[owner_index];
    let half = name_width(&owner.name) / 2.0 + 16.0;
    let middle = parent.center().0;
    let centre = r.center().0;
    if (centre - middle).abs() >= half || name_aside(d, owner_index, parent, r).is_some() {
        return r;
    }
    let right = snap_up(middle + half, 8.0);
    Rect { x: right.min(parent.right() - 16.0 - r.w / 2.0) - r.w / 2.0, ..r }
}

/// The widest line of a state's name compartment: its name or its keyword.
fn name_extent(node: &Node) -> f64 {
    name_width(&node.name).max(keyword(node).map_or(0.0, |k| small_width(&k)))
}

/// Where state `n`'s name goes beside an entry point `circle` on its top
/// border that its centred name would meet (Drawing principles, Names: let a
/// name give way to a line): the name ends 8px before the circle, or starts
/// 8px after it, keeping 12px from the state's sides; the side away from the
/// middle first. `None` when neither side has room.
fn name_aside(d: &Diagram, n: usize, parent: Rect, circle: Rect) -> Option<(f64, Anchor)> {
    let w = name_extent(&d.nodes[n]);
    let before = (circle.x - 8.0, Anchor::End, circle.x - 8.0 - w >= parent.x + 12.0);
    let after = (circle.right() + 8.0, Anchor::Start, circle.right() + 8.0 + w <= parent.right() - 12.0);
    let order = if circle.center().0 >= parent.center().0 { [before, after] } else { [after, before] };
    order.into_iter().find(|o| o.2).map(|o| (o.0, o.1))
}

/// Where state `n`'s name sits across its compartment, as the text's x and
/// anchor: centred, or beside an entry point that wants the middle.
pub fn name_place(d: &Diagram, rects: &[Rect], n: usize) -> (f64, Anchor) {
    let r = rects[n];
    let middle = r.center().0;
    let half = name_width(&d.nodes[n].name) / 2.0 + 16.0;
    let clash = |c: usize| d.nodes[c].kind == Kind::EntryPoint && rects[c].center().1 <= r.y + 1.0 && (rects[c].center().0 - middle).abs() < half;
    let aside = (d.direction == Direction::Down).then(|| d.nodes[n].children.iter().copied().find(|&c| clash(c))).flatten().and_then(|c| name_aside(d, n, r, rects[c]));
    aside.unwrap_or((middle, Anchor::Middle))
}

/// Widens fork and join bars over the states they connect, when the rank has room.
fn stretch_bars(d: &Diagram, spots: &[Option<LabelSpot>], rects: Vec<Rect>) -> Vec<Rect> {
    (0..d.nodes.len()).filter(|&n| d.nodes[n].kind.is_bar()).fold(rects, |mut rects, bar| {
        let cross = |p: Point| to_flow(d.direction, p).1;
        let peers: Vec<f64> = d
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.from == bar || edge.to == bar)
            .map(|(e, edge)| spots[e].map_or_else(|| cross(rects[if edge.from == bar { edge.to } else { edge.from }].center()), |spot| cross(spot.anchor)))
            .collect();
        let r = rects[bar];
        let (lo, hi) = peers.iter().fold(match d.direction {
            Direction::Down => (r.x + 16.0, r.right() - 16.0),
            Direction::Right => (r.y + 16.0, r.bottom() - 16.0),
        }, |(lo, hi), &p| (lo.min(p), hi.max(p)));
        let stretched = match d.direction {
            Direction::Down => Rect { x: lo - 16.0, w: hi - lo + 32.0, ..r },
            Direction::Right => Rect { y: lo - 16.0, h: hi - lo + 32.0, ..r },
        };
        let clear = (1..d.nodes.len()).filter(|&n| n != bar && !d.contains(n, bar)).all(|n| !rects[n].intersects(&stretched));
        if clear {
            rects[bar] = stretched;
        }
        rects
    })
}

/// Self-transitions along the side of a state away from the flow, each with its
/// label beside it, in the room held by the state's box.
pub fn self_loops(d: &Diagram, rects: &[Rect]) -> Vec<SelfLoop> {
    (1..d.nodes.len()).filter(|&n| d.nodes[n].kind == Kind::State).flat_map(|n| state_loops(d, n, rects[n])).collect()
}

fn state_loops(d: &Diagram, n: usize, r: Rect) -> Vec<SelfLoop> {
    let edges = self_edges(d, n);
    let pitches: Vec<f64> = edges.iter().map(|&e| loop_pitch(&d.edges[e])).collect();
    let total: f64 = pitches.iter().sum();
    let starts: Vec<f64> = pitches.iter().scan(0.0, |at, p| { let start = *at; *at += p; Some(start) }).collect();
    let count = edges.len();
    edges
        .iter()
        .enumerate()
        .map(|(i, &e)| {
            let lines = label_lines(&d.edges[e]);
            let size = label_size(&lines);
            let centre = starts[i] + pitches[i] / 2.0 - total / 2.0;
            match d.direction {
                Direction::Down => {
                    let (right, y) = (r.right(), r.center().1 + centre);
                    let half = LOOP_SPAN / 2.0;
                    let points = vec![(right, y - half), (right + LOOP_DEPTH, y - half), (right + LOOP_DEPTH, y + half), (right, y + half)];
                    let label = Rect { x: right + LOOP_DEPTH + 8.0, y: y - size.h / 2.0, w: size.w, h: size.h };
                    SelfLoop { edge: e, points, lines, label }
                }
                Direction::Right => {
                    let (bottom, x) = (r.bottom(), r.center().0 + centre);
                    let half = LOOP_SPAN / 2.0;
                    let points = vec![(x - half, bottom), (x - half, bottom + LOOP_DEPTH), (x + half, bottom + LOOP_DEPTH), (x + half, bottom)];
                    let below = bottom + LOOP_DEPTH + 6.0 + (count - 1 - i) as f64 * (LABEL_LINE + 2.0);
                    let label = Rect { x: x - half, y: below, w: size.w, h: size.h };
                    SelfLoop { edge: e, points, lines, label }
                }
            }
        })
        .collect()
}

/// The layout, with chains of states in columns or without.
pub fn layout(d: &Diagram, columns: bool) -> Geometry {
    let solved = settle_regions(d, columns);
    let labels = solved.labels.clone();
    let rects: Vec<Rect> = solved.rects[..d.nodes.len()].to_vec();
    let rects = move_connection_points(d, rects);
    let rects = stretch_bars(d, &labels, rects);
    let loops = self_loops(d, &rects);
    Geometry { rects, labels, loops }
}
