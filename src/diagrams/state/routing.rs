// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Adapts a state machine to the generic router. A transition with a reserved
//! label is routed in two halves through the label's anchor, so the label is
//! always beside its own line, and all labels are obstacles for every other line.

use super::layout::{insets, self_loops, Geometry};
use super::model::*;
use crate::core::align::{self, Align, Reach};
use crate::core::ends::HEAD_ROOM;
use crate::core::geometry::{self, along_side, note_link, to_flow, Direction, Point, Rect, Side};
use crate::core::glyphs::ARROWHEAD;
use crate::core::grow::{self, content, Room};
use crate::core::layered::LabelSpot;
use crate::core::router::{self, boundary_strips, facing, room_strips, settle_slots, slide_ranges, straighten, Body, Connection, Problem, Slot, Strips, Terminal};
use crate::core::text::small_width;
use crate::diagrams::boxes::CLEAR;

/// The clear gap a line keeps beside a composite state.
const CONTAINER_CLEARANCE: f64 = crate::core::flow::CONTAINER_CLEARANCE;
const SLOT: f64 = 24.0;
const SLOT_INSET: f64 = 16.0;
const LABEL_CLEAR: f64 = 4.0;
const STEM: f64 = 8.0;
/// The shortest last run of a line that meets a symbol's side: its arrowhead
/// and a straight run before it.
const ARRIVAL_RUN: f64 = ARROWHEAD + HEAD_ROOM;

pub struct Routes {
    /// One polyline per edge. Self-transitions are empty here.
    pub edges: Vec<Vec<Point>>,
    pub failed: Vec<usize>,
}

/// One end of a transition on a side of a vertex.
#[derive(Clone, Debug)]
struct End {
    edge: usize,
    slot: Slot,
    /// Fixed at the middle of its side: points and bar-free symbols.
    fixed: bool,
    /// Set when the line ends inside its enclosing state: local transitions.
    inward: bool,
}

/// The rectangle a line touches. Some symbols are drawn smaller than their box.
pub fn visual(d: &Diagram, g: &Geometry, n: usize) -> Rect {
    match d.nodes[n].kind {
        Kind::Junction => g.rects[n].inflate(-3.0),
        Kind::Terminate => g.rects[n].inflate(-4.0),
        _ => g.rects[n],
    }
}

/// The point of a slot on the outline its line touches.
fn side_point(d: &Diagram, g: &Geometry, slot: &Slot) -> Point {
    geometry::side_point(visual(d, g, slot.item), slot.side, slot.coord)
}

fn point_rect(p: Point) -> Rect {
    Rect { x: p.0, y: p.1, w: 0.0, h: 0.0 }
}

fn side_range(d: &Diagram, g: &Geometry, n: usize, side: Side) -> (f64, f64) {
    let r = visual(d, g, n);
    match side {
        Side::Left | Side::Right => (r.y + SLOT_INSET.min(r.h / 2.0), r.bottom() - SLOT_INSET.min(r.h / 2.0)),
        Side::Top | Side::Bottom => (r.x + SLOT_INSET.min(r.w / 2.0), r.right() - SLOT_INSET.min(r.w / 2.0)),
    }
}

/// The side a choice or junction uses for a given outgoing line: spokes leave the
/// symbol's corners, so a fan-out does not funnel through one point.
fn spoke_sides(d: &Diagram, g: &Geometry, n: usize) -> Vec<(usize, Side)> {
    let target_cross = |e: usize| g.labels[e].map_or_else(|| to_flow(d.direction, g.rects[d.edges[e].to].center()).1, |s| to_flow(d.direction, s.anchor).1);
    let own = to_flow(d.direction, g.rects[n].center()).1;
    let mut out: Vec<usize> = (0..d.edges.len()).filter(|&e| d.edges[e].from == n && d.edges[e].line == Line::Transition && to_flow(d.direction, g.rects[d.edges[e].to].center()).0 > to_flow(d.direction, g.rects[n].center()).0).collect();
    out.sort_by(|&a, &b| target_cross(a).total_cmp(&target_cross(b)));
    let (low, forward, high) = match d.direction {
        Direction::Down => (Side::Left, Side::Bottom, Side::Right),
        Direction::Right => (Side::Top, Side::Right, Side::Bottom),
    };
    let last = out.len().saturating_sub(1);
    // A line going back against the flow leaves by the corner on its own side.
    let back = (0..d.edges.len()).filter(|&e| d.edges[e].from == n && d.edges[e].line == Line::Transition && !out.contains(&e)).map(|e| (e, if target_cross(e) >= own { high } else { low }));
    let back: Vec<(usize, Side)> = back.collect();
    // The branch whose target lies straight ahead continues the flow through
    // the symbol, out of the side opposite the incoming line (Drawing
    // principles, Lines: flow passes straight through a decision).
    let ahead = |e: usize| {
        let r = g.rects[d.edges[e].to];
        let (lo, hi) = match d.direction {
            Direction::Down => (r.x, r.right()),
            Direction::Right => (r.y, r.bottom()),
        };
        lo + SLOT_INSET.min((hi - lo) / 2.0) <= own && own <= hi - SLOT_INSET.min((hi - lo) / 2.0)
    };
    let main = out.iter().copied().find(|&e| ahead(e));
    out.iter()
        .enumerate()
        .map(|(i, &e)| match () {
            _ if main == Some(e) => (e, forward),
            _ if out.len() > 1 && i == 0 && target_cross(e) < own - 1.0 => (e, low),
            _ if out.len() > 1 && i == last && target_cross(e) > own + 1.0 => (e, high),
            _ if main.is_some() => (e, if target_cross(e) < own { low } else { high }),
            _ => (e, forward),
        })
        .chain(back)
        .collect()
}

/// The side a final state, choice, junction or terminate takes for an incoming
/// line along the flow. A line from a source that cannot line up with the symbol
/// meets the symbol's side facing it, as an L, instead of jogging onto its top;
/// so lines from both sides also stay apart instead of meeting at one point.
/// A side already used by an outgoing spoke is not taken.
fn arrival_sides(d: &Diagram, g: &Geometry, spokes: &[(usize, Side)], n: usize) -> Vec<(usize, Side)> {
    if !matches!(d.nodes[n].kind, Kind::Final | Kind::Choice | Kind::Junction | Kind::Terminate) {
        return Vec::new();
    }
    let (low, high) = match d.direction {
        Direction::Down => (Side::Left, Side::Right),
        Direction::Right => (Side::Top, Side::Bottom),
    };
    let taken = |side: Side| spokes.iter().any(|&(_, s)| s == side);
    // Where the line comes from across the flow: its label's anchor, or the slots
    // its source can offer.
    let source = |e: usize| match g.labels[e] {
        Some(spot) => (to_flow(d.direction, spot.anchor).1, to_flow(d.direction, spot.anchor).1),
        None => {
            let from = d.edges[e].from;
            let side = facing(d.direction, &g.rects[from], &g.rects[n]).0;
            let (lo, hi) = side_range(d, g, from, side);
            if side.is_vertical_edge() == (d.direction == Direction::Down) { (to_flow(d.direction, g.rects[from].center()).1, to_flow(d.direction, g.rects[from].center()).1) } else { (lo, hi) }
        }
    };
    let ahead = |e: usize| {
        let before = g.labels[e].map_or_else(|| to_flow(d.direction, g.rects[d.edges[e].from].center()).0, |spot| to_flow(d.direction, spot.anchor).0);
        before < to_flow(d.direction, g.rects[n].center()).0
    };
    (0..d.edges.len())
        .filter(|&e| d.edges[e].to == n && d.edges[e].from != n && d.edges[e].line == Line::Transition && !d.is_local(&d.edges[e]) && ahead(e))
        .filter_map(|e| {
            // The last run must be long enough to show its arrowhead.
            let (lo, hi) = source(e);
            let r = visual(d, g, n);
            let (near, far) = match d.direction {
                Direction::Down => (r.x, r.right()),
                Direction::Right => (r.y, r.bottom()),
            };
            match () {
                _ if near - hi >= ARRIVAL_RUN && !taken(low) => Some((e, low)),
                _ if lo - far >= ARRIVAL_RUN && !taken(high) => Some((e, high)),
                _ => None,
            }
        })
        .collect()
}

/// An entry or exit point sits on a border: a line from outside the state meets
/// its outer side, and a line from inside meets its inner side.
fn connection_side(d: &Diagram, n: usize, other: usize) -> Option<Side> {
    let kind = d.nodes[n].kind;
    if !kind.is_connection_point() {
        return None;
    }
    let parent = d.nodes[n].parent?;
    let outer = match (d.direction, kind) {
        (Direction::Down, Kind::EntryPoint) => Side::Top,
        (Direction::Down, _) => Side::Bottom,
        (Direction::Right, Kind::EntryPoint) => Side::Left,
        (Direction::Right, _) => Side::Right,
    };
    Some(if d.contains(parent, other) { outer.opposite() } else { outer })
}

fn ends_for(d: &Diagram, g: &Geometry, spokes: &[Vec<(usize, Side)>], arrivals: &[Vec<(usize, Side)>], e: usize) -> [End; 2] {
    let edge = &d.edges[e];
    let (a, b) = (g.rects[edge.from], g.rects[edge.to]);
    let local = d.is_local(edge);
    let (side_a, side_b, inward_a, inward_b) = if local {
        // The line runs from the inner vertex to the border, on the composite's right.
        let inner_first = d.contains(edge.to, edge.from);
        (Side::Right, Side::Right, !inner_first, inner_first)
    } else if let Some(spot) = g.labels[e] {
        let pin = point_rect(spot.anchor);
        (facing(d.direction, &a, &pin).0, facing(d.direction, &b, &pin).0, false, false)
    } else {
        let (x, y) = facing(d.direction, &a, &b);
        (x, y, false, false)
    };
    let bordered = |n: usize, other: usize, side: Side| connection_side(d, n, other).unwrap_or(side);
    let (side_a, side_b) = if local { (side_a, side_b) } else { (bordered(edge.from, edge.to, side_a), bordered(edge.to, edge.from, side_b)) };
    let spoke = spokes[edge.from].iter().find(|(i, _)| *i == e).map(|(_, s)| *s);
    let arrival = arrivals[edge.to].iter().find(|(i, _)| *i == e).map(|(_, s)| *s);
    let end = |n: usize, side: Side, inward: bool| End {
        edge: e,
        slot: Slot::centred(n, side, visual(d, g, n)),
        // A self-transition is drawn as a loop by the layout, so its ends take no slot.
        fixed: d.nodes[n].kind.is_point() || d.nodes[n].kind == Kind::Note || edge.from == edge.to,
        inward,
    };
    [end(edge.from, spoke.unwrap_or(side_a), inward_a), end(edge.to, arrival.unwrap_or(side_b), inward_b)]
}

/// Slides ends along their sides toward their lines. Fixed ends stay centred.
fn settle(d: &Diagram, g: &Geometry, ends: Vec<End>) -> Vec<End> {
    let movable: Vec<usize> = (0..ends.len()).filter(|&i| !ends[i].fixed).collect();
    let sub_of = |full: usize| movable.iter().position(|&i| i == full);
    let goal = |slots: &[Slot], end: usize| {
        let e = ends[end].edge;
        let side = ends[end].slot.side;
        match g.labels[e] {
            Some(spot) if !d.is_local(&d.edges[e]) => along_side(spot.anchor, side),
            _ => {
                let peer = if ends[e * 2].edge == e && end == e * 2 { e * 2 + 1 } else { e * 2 };
                let slot = sub_of(peer).map_or(&ends[peer].slot, |k| &slots[k]);
                along_side(side_point(d, g, slot), side)
            }
        }
    };
    let slots: Vec<Slot> = movable.iter().map(|&i| ends[i].slot.clone()).collect();
    let settled = settle_slots(slots, SLOT, |n, side| side_range(d, g, n, side), |slots, k| goal(slots, movable[k]));
    let pairs: Vec<(usize, usize)> = (0..d.edges.len()).filter_map(|e| Some((sub_of(e * 2)?, sub_of(e * 2 + 1)?))).collect();
    let straight = pairs.into_iter().fold(settled, |slots, (i, j)| straighten(slots, i, j, &g.rects, SLOT));
    ends.iter()
        .enumerate()
        .map(|(i, end)| match sub_of(i) {
            Some(k) => End { slot: straight[k].clone(), ..end.clone() },
            None => end.clone(),
        })
        .collect()
}

fn own_obstacles(loops: &[Rect], labels: &[LabelSpot]) -> Vec<Rect> {
    labels.iter().map(|spot| spot.rect.inflate(LABEL_CLEAR)).chain(loops.iter().copied()).collect()
}

/// The text block of a composite state, which lines must not cross.
fn header_text(d: &Diagram, g: &Geometry, n: usize) -> Rect {
    let r = g.rects[n];
    let node = &d.nodes[n];
    let name = small_width(&node.name) * 1.3 + 16.0;
    let wide = node.activities.iter().map(|l| small_width(l) + 24.0).fold(name, f64::max);
    // A name beside an entry point keeps its room on its own side of it.
    let x = match super::layout::name_place(d, &g.rects, n) {
        (at, crate::core::svg::Anchor::Start) => at - 8.0,
        (at, crate::core::svg::Anchor::End) => at + 8.0 - wide,
        _ => r.center().0 - wide / 2.0,
    };
    Rect { x, y: r.y, w: wide, h: super::layout::header_height(node) }
}

/// A terminal for an end. An end on a state's side may slide along it when
/// nudging straightens its line.
fn terminal(d: &Diagram, g: &Geometry, end: &End, slide: (f64, f64), headed: bool) -> Terminal {
    let point = side_point(d, g, &end.slot);
    let placed = Terminal::at(point, if end.inward { end.slot.side.opposite() } else { end.slot.side }, 0.0);
    // An arrowhead needs a straight run longer than itself.
    let fixed = if headed { placed.reaching(ARROWHEAD + HEAD_ROOM) } else { placed };
    if end.fixed || end.inward || d.is_local(&d.edges[end.edge]) {
        return fixed;
    }
    // An end on a simple state may move to another side to turn a Z into an L.
    let node = &d.nodes[end.slot.item];
    let sliding = fixed.sliding(slide.0, slide.1);
    // A state with internal behaviours takes new ends on its left and right
    // sides only below its compartment line, which meets those sides, so a
    // line never seems to continue that line.
    let head = if node.activities.is_empty() { 0.0 } else { super::layout::header_height(node) - node.activities.len() as f64 * super::layout::ACTIVITY_LINE - 8.0 };
    let sides = [true; 4];
    if node.kind == Kind::State && node.children.is_empty() && d.edges[end.edge].line == Line::Transition { sliding.on(Body { rect: g.rects[end.slot.item], inset: SLOT_INSET, sides, head }) } else { sliding }
}

/// The two terminals of the waypoint at a label anchor, for a line running
/// from `from_main` along the flow.
fn pass_terminals(d: &Diagram, anchor: Point, from_main: f64) -> (Terminal, Terminal) {
    let forward = to_flow(d.direction, anchor).0 >= from_main;
    let (back, ahead) = match (d.direction, forward) {
        (Direction::Down, true) => (Side::Top, Side::Bottom),
        (Direction::Down, false) => (Side::Bottom, Side::Top),
        (Direction::Right, true) => (Side::Left, Side::Right),
        (Direction::Right, false) => (Side::Right, Side::Left),
    };
    let stem = |side: Side| {
        let (dx, dy) = side.out();
        (anchor.0 + dx * STEM, anchor.1 + dy * STEM)
    };
    (Terminal { point: anchor, stem: stem(back), outward: back, slide: None, body: None }, Terminal { point: anchor, stem: stem(ahead), outward: ahead, slide: None, body: None })
}

fn connection(d: &Diagram, g: &Geometry, shared: &[Rect], e: usize, from: Terminal, to: Terminal) -> Connection {
    let edge = &d.edges[e];
    let (a, b) = (edge.from, edge.to);
    // A line from a connection point to outside its state stays outside that state.
    let inner = |end: usize, other: usize, n: usize| n != end && d.contains(n, end) && !(d.nodes[end].kind.is_connection_point() && d.nodes[end].parent == Some(n) && !d.contains(n, other));
    let encloses = |n: usize| inner(a, b, n) || inner(b, a, n);
    let (containers, solid): (Vec<usize>, Vec<usize>) = (1..d.nodes.len()).partition(|&n| encloses(n));
    let headers = containers.iter().filter(|&&n| d.nodes[n].kind == Kind::State).map(|&n| header_text(d, g, n).inflate(CLEAR));
    let walls = solid.iter().map(|&n| g.rects[n].inflate(CLEAR));
    let strips = boundary_strips(containers.iter().filter(|&&n| d.nodes[n].kind == Kind::State).map(|&n| (g.rects[n], g.rects[n].y)));
    // Composite states keep long lines at their clearance; simple states keep
    // them off by their wall, which their spacing already holds near 16px.
    let beside = solid.iter().copied().filter(|&n| n != a && n != b && d.nodes[n].kind == Kind::State && !d.nodes[n].children.is_empty()).map(|n| (g.rects[n], CONTAINER_CLEARANCE));
    let room = room_strips(beside.collect());
    Connection {
        from,
        to,
        obstacles: walls.chain(headers).chain(shared.iter().copied()).collect(),
        strips: Strips { vertical: strips.vertical.into_iter().chain(room.vertical).collect(), horizontal: strips.horizontal.into_iter().chain(room.horizontal).collect() },
    }
}

/// Joins two halves of a line, dropping the shared point and straight joints.
fn join_paths(first: Vec<Point>, second: Vec<Point>) -> Vec<Point> {
    let combined = first.into_iter().chain(second);
    combined.fold(Vec::new(), |mut out: Vec<Point>, p| {
        if out.last() != Some(&p) {
            if let [.., a, b] = out[..] {
                if (a.0 == b.0 && b.0 == p.0) || (a.1 == b.1 && b.1 == p.1) {
                    out.pop();
                }
            }
            out.push(p);
        }
        out
    })
}

/// The box around a self-transition's loop.
fn loop_box(points: &[Point]) -> Rect {
    let (min, max) = points.iter().fold(((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)), |(lo, hi), p| ((lo.0.min(p.0), lo.1.min(p.1)), (hi.0.max(p.0), hi.1.max(p.1))));
    Rect { x: min.0, y: min.1, w: max.0 - min.0, h: max.1 - min.1 }
}

/// The layout with chains of states in columns (Drawing principles, Flow:
/// keep a column), unless it draws with more failures, crossings or overlaps
/// than the layout without: a long line back up past a column can find no
/// clear way round a state beside it. When columns route worse with every
/// label reserved, both layouts are drawn with their labels settled
/// (`hybrid::settle`), which may straighten what the reserved labels tangled.
pub fn arranged(d: &Diagram) -> Geometry {
    let columned = super::layout::layout(d, true);
    if !d.edges.iter().any(|e| e.from != e.to) {
        return columned;
    }
    let plain = super::layout::layout(d, false);
    let reserved = |g: &Geometry| {
        let r = route(d, g);
        let m = crate::core::metrics::measure(&r.edges);
        (r.failed.len(), m.crossings, m.overlaps)
    };
    if reserved(&columned) <= reserved(&plain) {
        return columned;
    }
    let settled = |g: &Geometry| {
        let r = super::hybrid::settle(d, g.clone()).routes;
        let m = crate::core::metrics::measure(&r.edges);
        (r.failed.len(), m.crossings, m.overlaps)
    };
    if settled(&plain) < settled(&columned) { plain } else { columned }
}

/// Moves states across the flow so that a transition to or from an entry or
/// exit point meets the state's middle (Drawing principles, Flow: keep the
/// spine on one line; Lines: meet the middle): the box moves under the line,
/// with the states already lined up with it, rather than the line sliding off
/// the middle of its side. Lines that already run between two middles hold
/// (`core::align`). Kept only when it routes with no more failures, crossings,
/// overlaps or bends.
pub fn points_lined_up(d: &Diagram, g: Geometry) -> Geometry {
    let across = |r: &Rect| if d.direction == Direction::Down { r.center().0 } else { r.center().1 };
    let point = |n: usize| Reach { node: n, lo: across(&g.rects[n]), hi: across(&g.rects[n]) };
    let inside = |outer: usize, n: usize| std::iter::successors(d.nodes[n].parent, |&p| d.nodes[p].parent).any(|p| p == outer);
    let plain = |e: &Edge| e.from != e.to && e.line != Line::NoteLink;
    let connection = |n: usize| d.nodes[n].kind.is_connection_point();
    let mut pulls = Vec::new();
    for e in d.edges.iter().filter(|e| plain(e)) {
        let (a, b) = (e.from, e.to);
        let lined = (across(&g.rects[a]) - across(&g.rects[b])).abs() < 0.5;
        // A transition between a connection point and a state outside it.
        let outer = |p: usize, other: usize| connection(p) && !inside(d.nodes[p].parent.unwrap_or(0), other) && d.nodes[other].children.is_empty() && !connection(other);
        if outer(a, b) || outer(b, a) || (lined && d.nodes[a].children.is_empty() && d.nodes[b].children.is_empty()) {
            pulls.push((point(a), point(b)));
        }
    }
    // A bar spans its branches, so a line that meets it anywhere along it
    // stays lined up.
    let bar = |n: usize| matches!(d.nodes[n].kind, Kind::Fork | Kind::Join);
    let span = |n: usize| {
        let r = g.rects[n];
        let (lo, hi) = if d.direction == Direction::Down { (r.x, r.right()) } else { (r.y, r.bottom()) };
        Reach { node: n, lo: lo + SLOT_INSET, hi: hi - SLOT_INSET }
    };
    let meets = |x: &Reach, y: &Reach| x.lo <= y.hi + 0.5 && y.lo <= x.hi + 0.5;
    for e in d.edges.iter().filter(|e| plain(e)) {
        for (b, other) in [(e.from, e.to), (e.to, e.from)] {
            if bar(b) && !bar(other) && d.nodes[other].children.is_empty() && meets(&span(b), &point(other)) {
                pulls.push((span(b), point(other)));
            }
        }
    }
    // A fork whose branches all meet one join passes the column through: the
    // state before the fork and the state after the join line up (Drawing
    // principles, Flow: keep the spine on one line).
    let onward = |n: usize| d.edges.iter().filter(|e| plain(e) && e.from == n).map(|e| e.to).collect::<Vec<usize>>();
    let before = |n: usize| d.edges.iter().filter(|e| plain(e) && e.to == n).map(|e| e.from).collect::<Vec<usize>>();
    let mut goals = pulls.iter().filter(|(x, y)| connection(x.node) || connection(y.node)).count();
    for f in (0..d.nodes.len()).filter(|&f| d.nodes[f].kind == Kind::Fork) {
        let branches = onward(f);
        let joins: Vec<usize> = branches.iter().flat_map(|&b| onward(b)).collect();
        let Some(&j) = joins.first() else { continue };
        let closed = d.nodes[j].kind == Kind::Join && branches.len() >= 2 && branches.iter().all(|&b| onward(b) == vec![j]) && before(j).len() == branches.len();
        if let (true, [p], [q]) = (closed, &before(f)[..], &onward(j)[..]) {
            let fits = meets(&span(f), &point(*p)) && meets(&span(j), &point(*p));
            if fits && !bar(*p) && !bar(*q) && d.nodes[*q].children.is_empty() && d.nodes[*q].parent == d.nodes[*p].parent {
                pulls.push((point(*p), point(*q)));
                goals += 1;
            }
        }
    }
    if goals == 0 {
        return g;
    }
    let room = Align {
        direction: d.direction,
        parents: d.nodes.iter().map(|node| node.parent).collect(),
        insets: (0..d.nodes.len()).map(|n| insets(d, n)).collect(),
        margins: vec![(0.0, 0.0); d.nodes.len()],
        gap: 40.0,
        pulls,
        families: Vec::new(),
        tied: d.edges.iter().filter(|e| e.line == Line::NoteLink).map(|e| (e.from, e.to)).collect(),
        trains: true,
    };
    let rects = align::align(g.rects.clone(), &room);
    if rects == g.rects {
        return g;
    }
    let moved = Geometry { loops: self_loops(d, &rects), rects, labels: g.labels.clone() };
    let standing = |g: &Geometry| {
        let r = route(d, g);
        let m = crate::core::metrics::measure(&r.edges);
        [r.failed.len(), m.crossings, m.overlaps, m.bends]
    };
    let (a, b) = (standing(&moved), standing(&g));
    if a.iter().zip(&b).all(|(x, y)| x <= y) { moved } else { g }
}

/// Grows end states where that lets a transition run straight (`core::grow`).
/// Only simple states without self-transitions grow, and never into a reserved
/// label or a loop.
pub fn grown(d: &Diagram, g: Geometry) -> Geometry {
    let looped = |n: usize| d.edges.iter().any(|edge| edge.from == n && edge.to == n);
    let reserved = g.labels.iter().flatten().map(|spot| spot.rect);
    let loops = g.loops.iter().flat_map(|l| [loop_box(&l.points), l.label]);
    let room = Room {
        direction: d.direction,
        growable: (0..d.nodes.len()).map(|n| d.nodes[n].kind == Kind::State && d.nodes[n].children.is_empty() && !looped(n)).collect(),
        parents: d.nodes.iter().map(|node| node.parent).collect(),
        within: d.nodes.iter().map(|node| node.parent.map_or(g.rects[0], |p| content(g.rects[p], insets(d, p), 8.0))).collect(),
        keep_out: reserved.chain(loops).collect(),
        ends: d.edges.iter().map(|edge| (edge.from, edge.to)).collect(),
        via: (0..d.edges.len()).map(|e| g.labels[e].filter(|_| !d.is_local(&d.edges[e])).map(|spot| spot.anchor)).collect(),
    };
    let geometry = |rects: Vec<Rect>| Geometry { loops: self_loops(d, &rects), rects, labels: g.labels.clone() };
    let rects = grow::grow(g.rects.clone(), &room, |rects| route(d, &geometry(rects.to_vec())).edges);
    geometry(rects)
}

pub fn route(d: &Diagram, g: &Geometry) -> Routes {
    let spokes: Vec<Vec<(usize, Side)>> = (0..d.nodes.len()).map(|n| if matches!(d.nodes[n].kind, Kind::Choice | Kind::Junction) { spoke_sides(d, g, n) } else { Vec::new() }).collect();
    let arrivals: Vec<Vec<(usize, Side)>> = (0..d.nodes.len()).map(|n| arrival_sides(d, g, &spokes[n], n)).collect();
    let flat: Vec<End> = (0..d.edges.len()).flat_map(|e| ends_for(d, g, &spokes, &arrivals, e)).collect();
    let ends = settle(d, g, flat);
    let slots: Vec<Slot> = ends.iter().map(|end| end.slot.clone()).collect();
    let slides = slide_ranges(&slots, router::SLIDE_GAP, |n, side| side_range(d, g, n, side));
    let is_loop = |e: usize| d.edges[e].from == d.edges[e].to;
    let loop_rects: Vec<Rect> = g.loops.iter().map(|l| loop_box(&l.points).inflate(4.0)).chain(g.loops.iter().map(|l| l.label.inflate(LABEL_CLEAR))).collect();
    let spots: Vec<LabelSpot> = g.labels.iter().flatten().copied().collect();
    let shared = own_obstacles(&loop_rects, &spots);
    let parts = |e: usize| -> Vec<(Terminal, Terminal)> {
        // A line through a reserved label runs where its label is; in a tight
        // gap a longer last run would make two such lines cross.
        let headed = d.edges[e].line != Line::NoteLink && g.labels[e].is_none();
        let from = terminal(d, g, &ends[e * 2], slides[e * 2], false);
        let to = terminal(d, g, &ends[e * 2 + 1], slides[e * 2 + 1], headed);
        match g.labels[e] {
            Some(spot) if !d.is_local(&d.edges[e]) => {
                let (arrive, leave) = pass_terminals(d, spot.anchor, to_flow(d.direction, from.point).0);
                vec![(from, arrive), (leave, to)]
            }
            _ => vec![(from, to)],
        }
    };
    let pieces: Vec<(usize, Terminal, Terminal)> = (0..d.edges.len()).filter(|&e| !is_loop(e)).flat_map(|e| parts(e).into_iter().map(move |(f, t)| (e, f, t))).collect();
    let problem = Problem {
        bounds: g.rects[1..].iter().copied().chain(spots.iter().map(|s| s.rect)).collect(),
        walls: (1..d.nodes.len()).map(|n| g.rects[n].inflate(CLEAR)).collect(),
        connections: pieces.iter().map(|&(e, f, t)| connection(d, g, &shared, e, f, t)).collect(),
        centred: false,
    };
    let solution = router::route(&problem);
    // Note links are routed, so other lines keep clear of them, but drawn
    // straight (`geometry::note_link`).
    let edges = (0..d.edges.len())
        .map(|e| match d.edges[e].line {
            Line::NoteLink => note_link(g.rects[d.edges[e].from], visual(d, g, d.edges[e].to)).to_vec(),
            _ => pieces.iter().zip(&solution.paths).filter(|((owner, _, _), _)| *owner == e).fold(Vec::new(), |path: Vec<Point>, (_, part)| join_paths(path, part.clone())),
        })
        .collect();
    let failed = solution.failed.iter().map(|&i| pieces[i].0).collect();
    Routes { edges, failed }
}
