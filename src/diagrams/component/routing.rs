// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Adapts a component diagram to the generic router: slot allocation for ports,
//! stubs and edge ends, then terminals, obstacles and soft strips per connector.

use super::layout::{end_pitch, insets, stub_port_label, stub_side, PACKAGE_TAB, SLOT, STUB};
use super::model::*;
use super::render::{glyph_run, glyph_segment, CLEARANCE};
use crate::core::align::{self, Align, Reach};
use crate::core::ends::HEAD_ROOM;
use crate::core::geometry::{self, along_side, note_link, side_middle, snap, Direction, Point, Rect, Side, GRID};
use crate::core::glyphs::ARROWHEAD;
use crate::core::grow::{self, content, Room};
use crate::core::layered::Layout;
use crate::core::router::{self, boundary_strips, facing, lone, room_strips, settle_slots, slide_ranges, straighten, Body, Connection, Problem, Slot, Strips, Terminal};
use crate::core::text::{name_width, small_width};
use crate::diagrams::boxes;

/// How far slots are first spread from a box's corners.
const SLOT_INSET: f64 = 16.0;
/// How near a corner nudging may slide an end on a square box: an arrow still
/// meets the straight side, so it reads as attached to that side.
const CORNER_INSET: f64 = 8.0;
const PORT_LIFT: f64 = 6.0;

pub struct StubGeom {
    pub stub: usize,
    pub base: Point,
    pub side: Side,
    /// On a top or bottom side, the interface name goes beyond the ball,
    /// centred on the line, instead of right of it, where the right would
    /// cross the side of the container around the owner.
    pub name_beyond: bool,
}

pub struct PortGeom {
    pub port: usize,
    pub center: Point,
    pub side: Side,
}

pub struct Routes {
    pub edges: Vec<Vec<Point>>,
    pub stubs: Vec<StubGeom>,
    pub ports: Vec<PortGeom>,
    /// Connectors with no clear route, drawn directly.
    pub failed: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Holder {
    /// `end` is 0 for the edge's `from` and 1 for its `to`.
    Edge { edge: usize, end: usize },
    Stub,
    Port(usize),
}

/// All connection points, with lookups from the things that use them.
struct Slots {
    slots: Vec<Slot>,
    holders: Vec<Holder>,
    port_slot: Vec<usize>,
    stub_slot: Vec<Option<usize>>,
    ends: Vec<[usize; 2]>,
}

fn is_package(d: &Diagram, n: usize) -> bool {
    d.nodes[n].kind == Kind::Package
}

fn body_top(d: &Diagram, g: &Layout, n: usize) -> f64 {
    let r = g.rects[n];
    if is_package(d, n) { r.y + PACKAGE_TAB } else { r.y }
}

/// The point of a slot on its box. A package's top side is below its tab.
fn side_point(g: &Layout, d: &Diagram, slot: &Slot) -> Point {
    match slot.side {
        Side::Top => (slot.coord, body_top(d, g, slot.item)),
        side => geometry::side_point(g.rects[slot.item], side, slot.coord),
    }
}

/// The usable span of a side, `inset` from its corners.
fn side_range(g: &Layout, d: &Diagram, node: usize, side: Side, inset: f64) -> (f64, f64) {
    let r = g.rects[node];
    match side {
        Side::Left | Side::Right => (body_top(d, g, node) + inset, r.bottom() - inset),
        Side::Top | Side::Bottom => {
            let left = if is_package(d, node) && side == Side::Top { r.x + r.w * 0.5 } else { r.x };
            let hi = r.right() - inset;
            (tab_clear(g, d, node, side, hi, left + inset), hi)
        }
    }
}

/// In the top row of a package, enter clear of the name tab.
fn tab_clear(g: &Layout, d: &Diagram, node: usize, side: Side, hi: f64, lo: f64) -> f64 {
    let package_parent = d.nodes[node].parent.filter(|&p| side == Side::Top && is_package(d, p));
    package_parent.map_or(lo, |parent| {
        let p = g.rects[parent];
        let tab_right = p.x + name_width(&d.nodes[parent].name) + 24.0;
        if g.rects[node].y - (p.y + PACKAGE_TAB) <= 32.0 && tab_right + 16.0 < hi { lo.max(tab_right + 16.0) } else { lo }
    })
}

/// The boundary sides of an edge's ends. Ports fix their own side.
fn edge_sides(d: &Diagram, g: &Layout, edge: &Edge) -> (Side, Side) {
    let (from, to) = (d.owner(edge.from), d.owner(edge.to));
    match (edge.from, edge.to) {
        (End::Port(p), End::Port(q)) => (d.port_side(p), d.port_side(q)),
        (End::Port(p), _) => {
            let side = d.port_side(p);
            (side, if d.contains(from, to) { side } else { side.opposite() })
        }
        (_, End::Port(q)) => {
            let side = d.port_side(q);
            (if d.contains(to, from) { side } else { side.opposite() }, side)
        }
        _ => facing(d.direction, &g.rects[from], &g.rects[to]),
    }
}

/// The sides of every edge's ends: the facing sides (`edge_sides`), except that
/// a dashed dependency is secondary to the other lines (Drawing principles,
/// Lines: meet the side the line comes from). Where its end shares a side of a
/// leaf component with another line or a stub and its straight way to the far
/// box, which it overlaps across the flow, runs through another box, so it must
/// go round anyway, that end takes a free side across the flow: the right
/// (bottom) one, or the other when that is taken. The solid lines keep the
/// facing side.
fn secondary_sides(d: &Diagram, g: &Layout) -> Vec<(Side, Side)> {
    let facing: Vec<(Side, Side)> = d.edges.iter().map(|edge| edge_sides(d, g, edge)).collect();
    let mut taken: Vec<(usize, Side)> = d.stubs.iter().filter_map(|stub| match stub.at {
        End::Node(n) => Some((n, stub_side(d, stub))),
        End::Port(_) => None,
    }).chain(d.ports.iter().enumerate().map(|(p, port)| (port.owner, d.port_side(p)))).collect();
    for (edge, &(a, b)) in d.edges.iter().zip(&facing) {
        if let End::Node(n) = edge.from { taken.push((n, a)); }
        if let End::Node(n) = edge.to { taken.push((n, b)); }
    }
    let leaf = |n: usize| d.nodes[n].kind == Kind::Component && d.nodes[n].children.is_empty();
    (0..d.edges.len()).map(|e| {
        let edge = &d.edges[e];
        let (End::Node(from), End::Node(to)) = (edge.from, edge.to) else { return facing[e] };
        if edge.kind != EdgeKind::Dashed || from == to {
            return facing[e];
        }
        let (ca, cb) = (g.rects[from].center(), g.rects[to].center());
        let between = Rect { x: ca.0.min(cb.0), y: ca.1.min(cb.1), w: (ca.0 - cb.0).abs().max(1.0), h: (ca.1 - cb.1).abs().max(1.0) };
        // A box linked to either end belongs to its fan or chain; a line past
        // a box it has nothing to do with is the one that must go round.
        let linked = |n: usize| d.edges.iter().any(|other| [(other.from, other.to), (other.to, other.from)].iter().any(|&(x, y)| d.owner(x) == n && (d.owner(y) == from || d.owner(y) == to)));
        let blocked = (1..d.nodes.len()).any(|n| n != from && n != to && leaf(n) && !linked(n) && g.rects[n].intersects(&between));
        // Only boxes that overlap across the flow: their line would run
        // straight if nothing stood between, so it must go round one side.
        let (ra, rb) = (g.rects[from], g.rects[to]);
        let overlap = match d.direction {
            Direction::Down => ra.x < rb.right() && rb.x < ra.right(),
            Direction::Right => ra.y < rb.bottom() && rb.y < ra.bottom(),
        };
        if !blocked || !overlap {
            return facing[e];
        }
        let count = |taken: &[(usize, Side)], n: usize, side: Side| taken.iter().filter(|&&t| t == (n, side)).count();
        let moved = |n: usize, side: Side, far: usize, taken: &[(usize, Side)]| -> Option<Side> {
            if !leaf(n) || count(taken, n, side) < 2 {
                return None;
            }
            let (own, other) = (g.rects[n], g.rects[far]);
            let (low, high, before) = if side.is_vertical_edge() { (Side::Top, Side::Bottom, other.bottom() <= own.y) } else { (Side::Left, Side::Right, other.right() <= own.x) };
            let order = if before { [low, high] } else { [high, low] };
            order.into_iter().find(|&s| count(taken, n, s) == 0)
        };
        let (a, b) = facing[e];
        if let Some(s) = moved(from, a, to, &taken) {
            let k = taken.iter().position(|&t| t == (from, a)).expect("taken");
            taken[k] = (from, s);
            (s, b)
        } else if let Some(s) = moved(to, b, from, &taken) {
            let k = taken.iter().position(|&t| t == (to, b)).expect("taken");
            taken[k] = (to, s);
            (a, s)
        } else {
            facing[e]
        }
    }).collect()
}

/// Creates one slot per port, node stub and node-attached edge end.
fn allocate(d: &Diagram, g: &Layout) -> Slots {
    let port_slots = d.ports.iter().enumerate().map(|(p, port)| (Slot::centred(port.owner, d.port_side(p), g.rects[port.owner]), Holder::Port(p)));
    let (mut slots, mut holders): (Vec<Slot>, Vec<Holder>) = port_slots.unzip();
    let port_slot: Vec<usize> = (0..d.ports.len()).collect();
    let stub_slot: Vec<Option<usize>> = d
        .stubs
        .iter()
        .map(|stub| match stub.at {
            End::Node(n) => {
                slots.push(Slot::centred(n, stub_side(d, stub), g.rects[n]));
                holders.push(Holder::Stub);
                Some(slots.len() - 1)
            }
            End::Port(_) => None,
        })
        .collect();
    let sides = secondary_sides(d, g);
    let ends = d
        .edges
        .iter()
        .enumerate()
        .map(|(e, edge)| {
            let (side_a, side_b) = sides[e];
            let mut slot_for = |end: End, side: Side, index: usize| match end {
                End::Port(p) => port_slot[p],
                End::Node(n) => {
                    slots.push(Slot::centred(n, side, g.rects[n]));
                    holders.push(Holder::Edge { edge: e, end: index });
                    slots.len() - 1
                }
            };
            let first = slot_for(edge.from, side_a, 0);
            let second = slot_for(edge.to, side_b, 1);
            [first, second]
        })
        .collect();
    Slots { slots, holders, port_slot, stub_slot, ends }
}

/// Where a port's slot wants to be. Inner gaps are narrow, so delegation
/// connectors get the straight line and outer connectors absorb any jog.
fn port_target(d: &Diagram, g: &Layout, table: &Slots, slots: &[Slot], i: usize, p: usize) -> f64 {
    let node = slots[i].item;
    let peers: Vec<(End, f64)> = d
        .edges
        .iter()
        .enumerate()
        .filter_map(|(e, edge)| match () {
            _ if edge.from == End::Port(p) => Some((edge.to, e, 1)),
            _ if edge.to == End::Port(p) => Some((edge.from, e, 0)),
            _ => None,
        })
        .map(|(other, e, end)| (other, along_side(side_point(g, d, &slots[table.ends[e][end]]), slots[i].side)))
        .collect();
    let (inner, outer): (Vec<_>, Vec<_>) = peers.into_iter().partition(|&(other, _)| d.contains(node, d.owner(other)));
    let chosen = if inner.is_empty() { outer } else { inner };
    if chosen.is_empty() {
        slots[i].coord
    } else {
        chosen.iter().map(|peer| peer.1).sum::<f64>() / chosen.len() as f64
    }
}

/// Whether port `p` has connectors to the inside of its owner. Such a port
/// lines up with them; any other port sits at the middle of its side.
fn delegates(d: &Diagram, p: usize) -> bool {
    let owner = d.ports[p].owner;
    let inner = |other: End| d.owner(other) != owner && d.contains(owner, d.owner(other));
    d.edges.iter().any(|edge| (edge.from == End::Port(p) && inner(edge.to)) || (edge.to == End::Port(p) && inner(edge.from)))
}

/// Whether slot `i` is alone on its side, counting lines that leave a
/// container's contents through that side (`leaving`), and so held at the
/// middle. A note link's end, an annotation, is never held.
fn alone_on_side(table: &Slots, d: &Diagram, slots: &[Slot], leaving: &[(usize, Side)], i: usize) -> bool {
    let note = matches!(table.holders[i], Holder::Edge { edge, .. } if d.edges[edge].kind == EdgeKind::NoteLink);
    !note && lone(slots, i) && !leaving.contains(&(slots[i].item, slots[i].side))
}

/// For each container side, whether a line from its contents to outside leaves
/// through it: the inner end faces that way.
fn leaving_sides(d: &Diagram, table: &Slots) -> Vec<(usize, Side)> {
    let crossing = d.edges.iter().zip(&table.ends).flat_map(|(edge, &[a, b])| {
        let ends = [(edge.from, a, edge.to), (edge.to, b, edge.from)];
        ends.into_iter().flat_map(move |(end, slot, other)| {
            let side = table.slots[slot].side;
            std::iter::successors(d.nodes[d.owner(end)].parent, move |&p| d.nodes[p].parent).filter(move |&p| p != 0 && !d.contains(p, d.owner(other))).map(move |p| (p, side))
        })
    });
    let mut found: Vec<(usize, Side)> = crossing.collect();
    found.sort();
    found.dedup();
    found
}

/// The middle of a side, or the nearest place to it that the side offers.
fn middle(g: &Layout, d: &Diagram, node: usize, side: Side) -> f64 {
    side_middle(g.rects[node], body_top(d, g, node), side, side_range(g, d, node, side, SLOT_INSET))
}

/// Ends that share a side, with no port among them, spread evenly in their
/// order and centred on the side as a group (Drawing principles, Lines: meet
/// the middle): `SLOT` apart (`NAMED_SLOT` where a line carries a name), or
/// closer where the side is short, but never closer than nudging lets ends
/// come. A group stays as settled when centring would take an end off a
/// straight line to another box. Groups are judged together, so two boxes
/// joined by several lines can spread their ends alike and keep every line
/// straight.
fn centre_groups(d: &Diagram, g: &Layout, table: &Slots, slots: Vec<Slot>, wanted: &dyn Fn(&[Slot], usize) -> f64) -> Vec<Slot> {
    let mut sides: Vec<(usize, Side)> = slots.iter().map(|s| (s.item, s.side)).collect();
    sides.sort();
    sides.dedup();
    let groups: Vec<Vec<(usize, f64)>> = sides
        .into_iter()
        .filter_map(|(item, side)| {
            let mut members: Vec<usize> = (0..slots.len()).filter(|&i| slots[i].item == item && slots[i].side == side).collect();
            if members.len() < 2 || members.iter().any(|&i| matches!(table.holders[i], Holder::Port(_))) {
                return None;
            }
            members.sort_by(|&a, &b| slots[a].coord.total_cmp(&slots[b].coord).then(a.cmp(&b)));
            let (lo, hi) = side_range(g, d, item, side, CORNER_INSET);
            let gaps = (members.len() - 1) as f64;
            let named = members.iter().any(|&i| matches!(table.holders[i], Holder::Edge { edge, .. } if d.edges[edge].label.is_some()));
            let step = (((hi - lo) / gaps / GRID).floor() * GRID).min(end_pitch(side, named));
            let first = snap(middle(g, d, item, side) - step * gaps / 2.0);
            if step < router::SLIDE_GAP || first < lo || first + step * gaps > hi {
                return None;
            }
            Some(members.iter().enumerate().map(|(k, &i)| (i, first + k as f64 * step)).collect())
        })
        .collect();
    let with = |base: &[Slot], chosen: &[bool]| {
        let mut out = base.to_vec();
        for (group, _) in groups.iter().zip(chosen).filter(|(_, &c)| c) {
            for &(i, coord) in group {
                out[i].coord = coord;
            }
        }
        out
    };
    // A port follows its inner ends, so only lines between boxes count.
    let to_box = |i: usize| matches!(table.holders[i], Holder::Edge { edge, end } if matches!(if end == 0 { d.edges[edge].to } else { d.edges[edge].from }, End::Node(_)));
    let straight = |group: &[(usize, f64)], slots: &[Slot]| group.iter().filter(|&&(i, _)| to_box(i) && (slots[i].coord - wanted(slots, i)).abs() < 0.5).count();
    let all = with(&slots, &vec![true; groups.len()]);
    let mut chosen: Vec<bool> = groups.iter().map(|group| straight(group, &all) >= straight(group, &slots)).collect();
    // A group that counted on a neighbour that stays as settled is judged again.
    let first = with(&slots, &chosen);
    for (k, group) in groups.iter().enumerate() {
        if chosen[k] && straight(group, &first) < straight(group, &slots) {
            chosen[k] = false;
        }
    }
    with(&slots, &chosen)
}

/// Lets slots settle toward their peers. An end alone on its side sits at its
/// middle (Drawing principles, Lines: meet the middle), other ends face the
/// far end of their line, groups centre on their side, ports that delegate
/// follow their inner ends, and a connector between boxes whose ends both
/// share their sides straightens where it can.
fn settle(d: &Diagram, g: &Layout, table: Slots) -> Slots {
    let leaving = leaving_sides(d, &table);
    let wanted = |slots: &[Slot], i: usize| match table.holders[i] {
        _ if alone_on_side(&table, d, slots, &leaving, i) && !matches!(table.holders[i], Holder::Port(p) if delegates(d, p)) => middle(g, d, slots[i].item, slots[i].side),
        Holder::Edge { edge, end } => along_side(side_point(g, d, &slots[table.ends[edge][1 - end]]), slots[i].side),
        // A stub yields the middle of a side to the lines that share it, going
        // to the end its name faces (above the ball on a left or right side,
        // right of it on a top or bottom side), so the name keeps out of their way.
        Holder::Stub if slots.iter().enumerate().any(|(j, s)| j != i && s.item == slots[i].item && s.side == slots[i].side) => {
            let (lo, hi) = side_range(g, d, slots[i].item, slots[i].side, SLOT_INSET);
            if slots[i].side.is_vertical_edge() { lo } else { hi }
        }
        Holder::Stub => slots[i].coord,
        Holder::Port(p) => port_target(d, g, &table, slots, i, p),
    };
    let settled = settle_slots(table.slots.clone(), SLOT, |n, s| side_range(g, d, n, s, SLOT_INSET), wanted);
    let grouped = centre_groups(d, g, &table, settled, &wanted);
    // Ports that line up with the inside follow their inner ends to where they now are.
    let followed = (0..d.ports.len()).filter(|&p| delegates(d, p)).fold(grouped, |mut slots, p| {
        let i = table.port_slot[p];
        let (lo, hi) = side_range(g, d, slots[i].item, slots[i].side, SLOT_INSET);
        let target = snap(port_target(d, g, &table, &slots, i, p)).clamp(lo, hi.max(lo));
        let free = slots.iter().enumerate().all(|(j, s)| j == i || s.item != slots[i].item || s.side != slots[i].side || (s.coord - target).abs() >= SLOT);
        if free {
            slots[i].coord = target;
        }
        slots
    });
    let is_plain = |edge: &Edge| matches!((edge.from, edge.to), (End::Node(_), End::Node(_)));
    let slots = d
        .edges
        .iter()
        .zip(&table.ends)
        .filter(|(edge, &[i, j])| is_plain(edge) && !alone_on_side(&table, d, &table.slots, &leaving, i) && !alone_on_side(&table, d, &table.slots, &leaving, j))
        .fold(followed, |slots, (_, &[i, j])| straighten(slots, i, j, &g.rects, SLOT));
    Slots { slots, ..table }
}

/// The slots for a layout: allocated, then settled.
fn settled(d: &Diagram, g: &Layout) -> Slots {
    settle(d, g, allocate(d, g))
}

/// The direction an edge end leaves its boundary. A delegation connector leaves
/// a port toward the inside of the owner.
fn outward(d: &Diagram, table: &Slots, e: usize, index: usize) -> Side {
    let edge = &d.edges[e];
    let (end, other) = if index == 0 { (edge.from, edge.to) } else { (edge.to, edge.from) };
    let side = table.slots[table.ends[e][index]].side;
    match end {
        End::Port(_) if d.contains(d.owner(end), d.owner(other)) => side.opposite(),
        _ => side,
    }
}

/// Which interface ends of a route get more than the clearance of a node end:
/// per edge, the end whose first run holds its glyph's whole `glyph_run`, and
/// whether ends at ports keep `CLEARANCE` from the port too.
#[derive(Clone)]
struct Stems {
    long: Vec<Option<usize>>,
    ports: bool,
}

/// The two terminals of an edge. An interface connector (ball and socket, or a
/// wired dependency) leaves a box on a straight run of at least `CLEARANCE`, and
/// a port too when `stems.ports` says so. The end `stems.long` names, when its
/// run fits before the far end's stem, reaches out by the whole `glyph_run`,
/// so the glyph gets a segment with a stub on each side (Drawing principles,
/// Component diagrams: give an interface room).
fn terminals(d: &Diagram, g: &Layout, table: &Slots, slides: &[(f64, f64)], e: usize, stems: &Stems) -> (Terminal, Terminal) {
    let run = glyph_run(d.edges[e].kind);
    let at_port = |index: usize| matches!(if index == 0 { d.edges[e].from } else { d.edges[e].to }, End::Port(_));
    let clearance = |index: usize| if at_port(index) && !stems.ports { 0.0 } else { run.min(CLEARANCE) };
    let plain = [0, 1].map(|index| terminal(d, g, table, slides, e, index, clearance(index)));
    let Some(index) = stems.long[e].filter(|_| run > 0.0) else {
        return (plain[0], plain[1]);
    };
    let near = terminal(d, g, table, slides, e, index, run);
    let far = plain[1 - index];
    // A run fits when it ends before the far end's own stem begins.
    let (dx, dy) = near.outward.out();
    if (far.stem.0 - near.stem.0) * dx + (far.stem.1 - near.stem.1) * dy < 0.0 {
        return (plain[0], plain[1]);
    }
    if index == 0 { (near, far) } else { (far, near) }
}

/// A terminal for an edge end, whose first run reaches at least `run`. An end on
/// a node's side may slide along it when nudging straightens its line; ports
/// stay where they are.
fn terminal(d: &Diagram, g: &Layout, table: &Slots, slides: &[(f64, f64)], e: usize, index: usize, run: f64) -> Terminal {
    let slot = table.ends[e][index];
    let base = side_point(g, d, &table.slots[slot]);
    let end = if index == 0 { d.edges[e].from } else { d.edges[e].to };
    // An arrowhead needs a straight run longer than itself.
    let headed = index == 1 && matches!(d.edges[e].kind, EdgeKind::Solid | EdgeKind::Dashed) && d.edges[e].head == Head::Open;
    let run = if headed { run.max(ARROWHEAD + HEAD_ROOM) } else { run };
    let room = |t: Terminal| if run > 0.0 { t.reaching(run) } else { t };
    match end {
        End::Port(_) => room(Terminal::at(base, outward(d, table, e, index), PORT_LIFT)),
        End::Node(n) => {
            let terminal = room(Terminal::at(base, outward(d, table, e, index), 0.0)).sliding(slides[slot].0, slides[slot].1);
            // An end alone on its side stays there when the other end shares
            // its side: that line is a fan's branch, and moving its far end
            // would pull it out of the fan. Moving an end off a crowded side
            // leaves that side tidier.
            let other = table.ends[e][1 - index];
            let free = d.nodes[n].children.is_empty() && !(lone(&table.slots, slot) && !lone(&table.slots, other)) && d.edges[e].kind != EdgeKind::NoteLink;
            // Every end knows its box, so an end left alone can return to the
            // middle. It may move only to a side that holds no stub or port.
            let sides = [Side::Left, Side::Right, Side::Top, Side::Bottom].map(|side| free && !fixtures(d, n).contains(&side));
            terminal.on(Body { rect: g.rects[n], inset: SLOT_INSET, sides, head: 0.0 })
        }
    }
}

/// Whether an end may move to another side of its box to turn a Z into an L:
/// plain leaf components, whose sides hold no stub or port.
fn movable(d: &Diagram, n: usize) -> bool {
    d.nodes[n].kind == Kind::Component && d.nodes[n].children.is_empty() && d.nodes[n].ports.is_empty() && !d.stubs.iter().any(|stub| d.owner(stub.at) == n)
}

/// The sides of node `n` that hold a stub or a port.
fn fixtures(d: &Diagram, n: usize) -> Vec<Side> {
    let stubs = d.stubs.iter().filter(|stub| stub.at == End::Node(n)).map(|stub| stub_side(d, stub));
    let ports = d.nodes[n].ports.iter().map(|&p| d.port_side(p));
    stubs.chain(ports).collect()
}

fn stub_geometry(d: &Diagram, g: &Layout, table: &Slots, i: usize) -> StubGeom {
    let (base, side) = match (d.stubs[i].at, table.stub_slot[i]) {
        (End::Port(p), _) => {
            let slot = &table.slots[table.port_slot[p]];
            let point = side_point(g, d, slot);
            let (dx, dy) = slot.side.out();
            ((point.0 + dx * PORT_LIFT, point.1 + dy * PORT_LIFT), slot.side)
        }
        (End::Node(_), slot) => {
            let slot = &table.slots[slot.expect("node stubs have slots")];
            (side_point(g, d, slot), slot.side)
        }
    };
    let stub = &d.stubs[i];
    let name_right = base.0 + 6.0 + 12.0 + small_width(&stub.interface);
    let container = d.nodes[d.owner(stub.at)].parent.map(|parent| g.rects[parent]);
    let name_beyond = !side.is_vertical_edge() && container.is_some_and(|c| name_right > c.right() - 8.0);
    StubGeom { stub: i, base, side, name_beyond }
}

/// Boxes that keep connectors off a stub's line, glyph and interface name.
fn stub_obstacles(d: &Diagram, geometry: &StubGeom) -> [Rect; 2] {
    let stub = &d.stubs[geometry.stub];
    let (base, side) = (geometry.base, geometry.side);
    let (dx, dy) = side.out();
    let reach = STUB + stub_port_label(d, stub);
    let tip = (base.0 + dx * reach, base.1 + dy * reach);
    let far = (tip.0 + dx * 14.0, tip.1 + dy * 14.0);
    let line = Rect { x: base.0.min(far.0) - 8.0, y: base.1.min(far.1) - 8.0, w: (base.0 - far.0).abs() + 16.0, h: (base.1 - far.1).abs() + 16.0 };
    let label = small_width(&stub.interface);
    let ball = (tip.0 + dx * 6.0, tip.1 + dy * 6.0);
    // The name's box overlaps the line's, so no connector squeezes between a
    // ball and its name.
    let name = if side.is_vertical_edge() {
        Rect { x: ball.0 - label / 2.0 - 4.0, y: ball.1 - 26.0, w: label + 8.0, h: 22.0 }
    } else if geometry.name_beyond {
        Rect { x: ball.0 - label / 2.0 - 4.0, y: if dy < 0.0 { ball.1 - 26.0 } else { ball.1 + 4.0 }, w: label + 8.0, h: 22.0 }
    } else {
        Rect { x: ball.0 + 4.0, y: ball.1 - 9.0, w: label + 12.0, h: 18.0 }
    };
    [line, name]
}

/// A node's outline. A package keeps its name tab free as a separate box.
fn node_obstacles(d: &Diagram, g: &Layout, n: usize) -> Vec<Rect> {
    boxes::outline(g.rects[n], is_package(d, n).then(|| (name_width(&d.nodes[n].name) + 24.0, PACKAGE_TAB)))
}

/// The space a composite's centred name takes, kept free of connectors.
fn name_box(d: &Diagram, g: &Layout, n: usize) -> Rect {
    let node = &d.nodes[n];
    let r = g.rects[n];
    let w = name_width(&node.name) + 16.0;
    let bottom = if node.stereotype.is_some() { 44.0 } else { 28.0 };
    Rect { x: r.x + (r.w - w) / 2.0, y: r.y + 8.0, w, h: bottom - 8.0 }
}

fn connection(d: &Diagram, g: &Layout, table: &Slots, slides: &[(f64, f64)], stubs: &[Rect], e: usize, stems: &Stems) -> Connection {
    let (a, b) = (d.owner(d.edges[e].from), d.owner(d.edges[e].to));
    let encloses = |n: usize| (n != a && d.contains(n, a)) || (n != b && d.contains(n, b));
    let (containers, solid): (Vec<usize>, Vec<usize>) = (1..d.nodes.len()).partition(|&n| encloses(n));
    let tabs = containers.iter().filter(|&&n| is_package(d, n)).map(|&n| node_obstacles(d, g, n)[1]);
    let names = containers.iter().filter(|&&n| d.nodes[n].kind == Kind::Component).map(|&n| name_box(d, g, n));
    let walls = solid.iter().flat_map(|&n| node_obstacles(d, g, n));
    let (from, to) = terminals(d, g, table, slides, e, stems);
    // Containers the line does not end on keep it at their clearance, as do
    // containers it reaches at a port, which it should not run along on the
    // way (Drawing principles, Lines: give long lines room).
    let at_side = |end: End| matches!(end, End::Node(_));
    let ends_on = |n: usize| (n == a && at_side(d.edges[e].from)) || (n == b && at_side(d.edges[e].to));
    let beside = solid.iter().copied().filter(|&n| !d.nodes[n].children.is_empty() && !ends_on(n)).map(|n| (g.rects[n], crate::core::flow::CONTAINER_CLEARANCE));
    let room = room_strips(beside.collect());
    let strips = boundary_strips(containers.iter().map(|&n| (g.rects[n], body_top(d, g, n))));
    Connection {
        from,
        to,
        obstacles: stubs.iter().copied().chain(walls).chain(tabs).chain(names).collect(),
        strips: Strips { vertical: strips.vertical.into_iter().chain(room.vertical).collect(), horizontal: strips.horizontal.into_iter().chain(room.horizontal).collect() },
    }
}

fn port_geometry(d: &Diagram, g: &Layout, table: &Slots) -> Vec<PortGeom> {
    let at = |p: usize| &table.slots[table.port_slot[p]];
    (0..d.ports.len()).map(|p| PortGeom { port: p, center: side_point(g, d, at(p)), side: at(p).side }).collect()
}

/// Moves boxes across the flow so that lone connectors meet the middles of
/// their sides on one line and parents sit centred over their children
/// (`core::align`). Only lone connectors pull: both ends alone on their sides,
/// or at ports.
pub fn aligned(d: &Diagram, g: Layout) -> Layout {
    let room = alignment(d, &g, false);
    // Alignment cannot see routes: keep it only when no route fails or crosses
    // more. Families are lined up with their leaders, or else centred on all
    // their members as balanced fans, whichever then routes with fewer bends
    // (the leaders on a tie, and only when they add no bends).
    let links: Vec<(usize, usize)> = d.edges.iter().filter(|e| e.kind != EdgeKind::NoteLink).map(|e| (d.owner(e.from), d.owner(e.to))).collect();
    let fallback = Align { families: align::balanced(d.direction, &g.rects, &links), trains: false, ..room.clone() };
    let standing = |l: &Layout| {
        let r = routed(d, l, false);
        let m = crate::core::metrics::measure(&r.edges);
        ((r.failed.len(), m.crossings), m.bends, crowding(d, l, &r.edges).0)
    };
    let start = standing(&g);
    let tried = [room, fallback].map(|room| {
        let moved = Layout { rects: align::with_root(align::align(g.rects.clone(), &room), &room.parents), ..g.clone() };
        // Then the lines that carry the spine on from a port line up with a
        // shared side beyond, where that breaks nothing lined up already.
        let onward = Align { families: room.families.clone(), trains: room.trains, ..alignment(d, &moved, true) };
        let moved = if onward.pulls.len() > alignment(d, &moved, false).pulls.len() {
            Layout { rects: align::with_root(align::align(moved.rects.clone(), &onward), &onward.parents), ..moved }
        } else {
            moved
        };
        let s = standing(&moved);
        (moved, s)
    });
    let [(led, a), (fan, b)] = tried;
    // A tie in bends is broken after box growth, which may still straighten a
    // line that one of them left with a jog (the leaders on a further tie).
    let grown_bends = |l: &Layout| crate::core::metrics::measure(&routed(d, &grown(d, l.clone()), false).edges).bends;
    // An end crowded at a corner reads worse than a bend
    // (Drawing principles, When rules conflict: unambiguous reading first).
    let fan_first = |a: usize, b: usize| b < a || (a == b && led.rects != fan.rects && grown_bends(&fan) < grown_bends(&led));
    let (crowd_led, crowd_fan) = (a.2, b.2);
    let fan_first = |a: usize, b: usize| crowd_fan < crowd_led || (crowd_fan == crowd_led && fan_first(a, b));
    // Lining up leaders must also not add bends overall; a balanced fan may
    // add bends, to meet the middles.
    // Nor may it bring ends nearer the corners (Drawing principles, Lines:
    // keep ends off corners).
    match (a.0 <= start.0 && a.1 <= start.1 && a.2 <= start.2, b.0 <= start.0) {
        (true, true) if fan_first(a.1, b.1) => fan,
        (true, _) => led,
        (false, true) => fan,
        (false, false) => g,
    }
}

/// The boxes a delegating port joins: the one box inside its owner that its
/// connectors reach (`inside`), or the one box outside.
fn port_partner(d: &Diagram, p: usize, inside: bool) -> Option<usize> {
    let owner = d.ports[p].owner;
    let mut found: Vec<usize> = d
        .edges
        .iter()
        .filter(|edge| edge.kind != EdgeKind::NoteLink)
        .filter_map(|edge| match () {
            _ if edge.from == End::Port(p) => Some(edge.to),
            _ if edge.to == End::Port(p) => Some(edge.from),
            _ => None,
        })
        .map(|end| d.owner(end))
        .filter(|&n| n != owner && d.contains(owner, n) == inside)
        .collect();
    found.sort_unstable();
    found.dedup();
    if let [only] = found[..] { Some(only) } else { None }
}

/// The links between boxes that alignment follows, with every port that
/// delegates seen through (Drawing principles, Flow: keep the spine on one
/// line): a line from outside to a port joins the box the port leads to
/// inside, and a line from inside to a port joins the box beyond it outside.
/// So a chain that runs through a container by its ports is one chain, and
/// its length and its members are counted across the container's border.
fn bridged_links(d: &Diagram) -> Vec<(usize, usize)> {
    let through = |end: End, other: End| match end {
        End::Port(p) if delegates(d, p) => {
            let owner = d.ports[p].owner;
            let inside = d.contains(owner, d.owner(other)) && d.owner(other) != owner;
            port_partner(d, p, !inside).unwrap_or(owner)
        }
        _ => d.owner(end),
    };
    let mut links: Vec<(usize, usize)> = d
        .edges
        .iter()
        .filter(|edge| edge.kind != EdgeKind::NoteLink)
        .map(|edge| (through(edge.from, edge.to), through(edge.to, edge.from)))
        .filter(|&(a, b)| a != b)
        .collect();
    links.dedup();
    links
}

/// What alignment needs for a layout: the lone connectors that pull, the
/// families and whom each centres on, containers, margins and notes.
fn alignment(d: &Diagram, g: &Layout, onward: bool) -> Align {
    let table = settled(d, g);
    let across = |side: Side| side.is_vertical_edge() == (d.direction == Direction::Right);
    let kept: Vec<usize> = (0..d.edges.len()).filter(|&e| d.edges[e].kind != EdgeKind::NoteLink).collect();
    let leaving = leaving_sides(d, &table);
    let reach = |e: usize, index: usize| {
        let i = table.ends[e][index];
        let slot = &table.slots[i];
        let end = if index == 0 { d.edges[e].from } else { d.edges[e].to };
        // An end that shares its side belongs to a fan, which stays even
        // around its centred parent rather than lining up one branch.
        let alone = matches!(end, End::Port(_)) || alone_on_side(&table, d, &table.slots, &leaving, i);
        // A port that follows one connector inside its owner moves with the
        // box that connector reaches, not with the owner.
        let node = match end {
            End::Port(p) if delegates(d, p) => {
                let inner: Vec<usize> = d
                    .edges
                    .iter()
                    .filter_map(|edge| match () {
                        _ if edge.from == End::Port(p) => Some(d.owner(edge.to)),
                        _ if edge.to == End::Port(p) => Some(d.owner(edge.from)),
                        _ => None,
                    })
                    .filter(|&n| n != slot.item && d.contains(slot.item, n))
                    .collect();
                if let [only] = inner[..] { only } else { slot.item }
            }
            _ => slot.item,
        };
        (alone && across(slot.side)).then_some(Reach { node, lo: slot.coord, hi: slot.coord })
    };
    let apart = |a: usize, b: usize| !d.contains(a, b) && !d.contains(b, a);
    let mut pulls: Vec<(Reach, Reach)> = kept.iter().filter_map(|&e| Some((reach(e, 0)?, reach(e, 1)?))).filter(|(a, b)| apart(a.node, b.node)).collect();
    let lone: Vec<(usize, usize)> = pulls.iter().map(|(a, b)| (a.node, b.node)).collect();
    // A line that leaves a delegating port carries the spine on beyond its
    // container (Drawing principles, Flow: keep the spine on one line). Where
    // its far end shares a side of a leaf box, the line lines up when the
    // port meets that side anywhere its ends may sit.
    let shared = |e: usize, index: usize| {
        let i = table.ends[e][index];
        let slot = &table.slots[i];
        let end = if index == 0 { d.edges[e].from } else { d.edges[e].to };
        let leaf = matches!(end, End::Node(n) if d.nodes[n].children.is_empty());
        (leaf && across(slot.side) && !alone_on_side(&table, d, &table.slots, &leaving, i)).then(|| {
            let (lo, hi) = side_range(g, d, slot.item, slot.side, SLOT_INSET);
            Reach { node: slot.item, lo, hi }
        })
    };
    let spine_port = |e: usize, index: usize| matches!(if index == 0 { d.edges[e].from } else { d.edges[e].to }, End::Port(p) if delegates(d, p));
    for &e in kept.iter().filter(|_| onward) {
        for index in [0, 1] {
            if !spine_port(e, index) {
                continue;
            }
            if let (Some(port), Some(far)) = (reach(e, index), shared(e, 1 - index)) {
                if apart(port.node, far.node) {
                    pulls.push(if index == 0 { (port, far) } else { (far, port) });
                }
            }
        }
    }
    let parents: Vec<Option<usize>> = d.nodes.iter().map(|node| node.parent).collect();
    let links = bridged_links(d);
    let direct: Vec<(usize, usize)> = kept.iter().map(|&e| (d.owner(d.edges[e].from), d.owner(d.edges[e].to))).collect();
    let bridged: Vec<(usize, usize)> = links.iter().copied().filter(|l| !direct.contains(l)).collect();
    let tied = d.edges.iter().filter(|edge| edge.kind == EdgeKind::NoteLink).map(|edge| (d.owner(edge.from), d.owner(edge.to))).collect();
    Align {
        direction: d.direction,
        families: align::families_through(d.direction, &g.rects, &links, &lone, &bridged),
        parents,
        insets: (0..d.nodes.len()).map(|n| insets(d, n)).collect(),
        margins: (0..d.nodes.len()).map(|n| super::layout::margin_of(d, n)).collect(),
        gap: crate::core::layered::Spacing::default().item_gap,
        pulls,
        tied,
        trains: true,
    }
}

/// The aligned layout with chains in columns (Drawing principles, Flow: keep a
/// column) when it routes with fewer bends and no more failures, crossings,
/// overlaps, uncentred parents or cramped heads than the layout without; otherwise the layout without. Long lines
/// back past a column can need more room than the column leaves, and a column
/// that straightens nothing is not worth moving boxes that were placed well.
pub fn arranged(d: &Diagram) -> Layout {
    let tight = arranged_with(d, false);
    if !super::layout::ends_need_room(d) {
        return tight;
    }
    // Boxes tall enough for the ends on their sides (Drawing principles,
    // Shapes: leave room for what attaches) are kept where the drawing without
    // them crowds ends at corners or against each other, and they route with
    // no more failures, crossings, overlaps or bends.
    let standing = |l: &Layout| {
        let r = routed(d, l, false);
        let m = crate::core::metrics::measure(&r.edges);
        ([r.failed.len(), m.crossings, m.overlaps, m.bends], crowded(d, l, &r.edges))
    };
    let (b, crowd) = standing(&tight);
    if crowd == 0 {
        return tight;
    }
    let roomy = arranged_with(d, true);
    let (a, fewer) = standing(&roomy);
    if fewer < crowd && a.iter().zip(&b).all(|(x, y)| x <= y) { roomy } else { tight }
}

/// Connector ends at a box's side that sit nearer a corner than `SLOT_INSET`,
/// and pairs of ends on one side nearer each other than `SLOT`.
fn crowded(d: &Diagram, g: &Layout, paths: &[Vec<Point>]) -> usize {
    let (corners, close) = crowding(d, g, paths);
    corners + close
}

/// Connector ends nearer a corner of their box than `SLOT_INSET`, and pairs of
/// ends on one side nearer each other than `SLOT`.
fn crowding(d: &Diagram, g: &Layout, paths: &[Vec<Point>]) -> (usize, usize) {
    let mut ends: Vec<(usize, Side, f64, f64, f64)> = Vec::new();
    for (edge, path) in d.edges.iter().zip(paths) {
        if edge.kind == EdgeKind::NoteLink || path.len() < 2 {
            continue;
        }
        for (end, p) in [(edge.from, path[0]), (edge.to, path[path.len() - 1])] {
            let End::Node(n) = end else { continue };
            if !d.nodes[n].children.is_empty() {
                continue;
            }
            let r = g.rects[n];
            let side = [Side::Left, Side::Right, Side::Top, Side::Bottom].into_iter().find(|&side| match side {
                Side::Left => (p.0 - r.x).abs() < 0.5,
                Side::Right => (p.0 - r.right()).abs() < 0.5,
                Side::Top => (p.1 - body_top(d, g, n)).abs() < 0.5,
                Side::Bottom => (p.1 - r.bottom()).abs() < 0.5,
            });
            let Some(side) = side else { continue };
            let (lo, hi) = if side.is_vertical_edge() { (body_top(d, g, n), r.bottom()) } else { (r.x, r.right()) };
            ends.push((n, side, along_side(p, side), lo, hi));
        }
    }
    let corners = ends.iter().filter(|&&(_, _, at, lo, hi)| at - lo < SLOT_INSET - 0.5 || hi - at < SLOT_INSET - 0.5).count();
    let close = ends.iter().enumerate().flat_map(|(i, a)| ends[i + 1..].iter().map(move |b| (a, b))).filter(|(a, b)| a.0 == b.0 && a.1 == b.1 && (a.2 - b.2).abs() > 0.5 && (a.2 - b.2).abs() < SLOT - 0.5).count();
    (corners, close)
}

fn arranged_with(d: &Diagram, roomy: bool) -> Layout {
    let heads: Vec<(f64, f64)> = d.edges.iter().filter(|e| e.kind != EdgeKind::NoteLink).map(|e| (0.0, if matches!(e.kind, EdgeKind::Solid | EdgeKind::Dashed) && e.head == Head::Open { ARROWHEAD } else { 0.0 })).collect();
    let score = |l: &Layout| {
        let r = routed(d, l, false);
        let m = crate::core::metrics::measure(&r.edges);
        let kept: Vec<Vec<Point>> = d.edges.iter().zip(&r.edges).filter(|(e, _)| e.kind != EdgeKind::NoteLink).map(|(_, p)| p.clone()).collect();
        let families = alignment(d, l, false).families;
        let uncentred = families.iter().filter(|(p, c)| !crate::core::ends::centred_over(d.direction, &l.rects, *p, c)).count();
        ((r.failed.len(), m.crossings, m.overlaps, uncentred, crate::core::ends::cramped(&kept, &heads).len()), m.bends)
    };
    let [columned, plain] = super::layout::layouts(d, roomy).map(|l| aligned(d, l));
    let (c, p) = (score(&columned), score(&plain));
    let no_worse = c.0 .0 <= p.0 .0 && c.0 .1 <= p.0 .1 && c.0 .2 <= p.0 .2 && c.0 .3 <= p.0 .3 && c.0 .4 <= p.0 .4;
    if no_worse && c.1 < p.1 { columned } else { plain }
}

/// Grows end boxes where that lets a connector run straight (`core::grow`).
/// Boxes with stubs or ports keep their size, and nothing grows into a stub,
/// a port or a port's name.
pub fn grown(d: &Diagram, g: Layout) -> Layout {
    let table = settled(d, &g);
    let stubs = (0..d.stubs.len()).flat_map(|i| stub_obstacles(d, &stub_geometry(d, &g, &table, i)));
    let ports = port_geometry(d, &g, &table).into_iter().flat_map(|port| [Rect { x: port.center.0 - 8.0, y: port.center.1 - 8.0, w: 16.0, h: 16.0 }, super::render::port_label(d, &port).3]);
    let room = Room {
        direction: d.direction,
        growable: (0..d.nodes.len()).map(|n| movable(d, n)).collect(),
        parents: d.nodes.iter().map(|node| node.parent).collect(),
        within: d.nodes.iter().map(|node| node.parent.map_or(g.rects[0], |p| content(g.rects[p], insets(d, p), 8.0))).collect(),
        keep_out: stubs.chain(ports).collect(),
        ends: d.edges.iter().map(|edge| (d.owner(edge.from), d.owner(edge.to))).collect(),
        via: vec![None; d.edges.len()],
    };
    let rects = grow::grow(g.rects.clone(), &room, |rects| routed(d, &Layout { rects: rects.to_vec(), labels: g.labels.clone() }, false).edges);
    Layout { rects, ..g }
}

/// The end of edge `e` whose first run holds its interface glyph when that run
/// is shorter than the glyph needs, or `None`: the glyph has room, sits on a
/// middle segment that no stem can lengthen, or the edge has no glyph.
fn cramped(d: &Diagram, solution: &router::Solution, e: usize) -> Option<usize> {
    let path = &solution.paths[e];
    let run = glyph_run(d.edges[e].kind);
    if run == 0.0 || path.len() < 3 || solution.failed.contains(&e) {
        return None;
    }
    let k = glyph_segment(path, d.direction);
    let length = (path[k].0 - path[k + 1].0).abs() + (path[k].1 - path[k + 1].1).abs();
    match k {
        _ if length >= run => None,
        0 => Some(0),
        k if k + 2 == path.len() => Some(1),
        _ => None,
    }
}

/// The drawn routes, with interfaces given their room where a route allows it.
pub fn route(d: &Diagram, g: &Layout) -> Routes {
    routed(d, g, true)
}

/// The routes for a layout. Alignment, columns and box growth compare layouts
/// by the plain route (`roomy` false); the drawing then tries the routes that
/// give interfaces more room, which never route worse than the plain one.
fn routed(d: &Diagram, g: &Layout, roomy: bool) -> Routes {
    let table = settled(d, g);
    let stubs: Vec<StubGeom> = (0..d.stubs.len()).map(|i| stub_geometry(d, g, &table, i)).collect();
    let stub_boxes: Vec<Rect> = stubs.iter().flat_map(|stub| stub_obstacles(d, stub)).collect();
    // An end alone on its side stays at its middle.
    let ranges = slide_ranges(&table.slots, router::SLIDE_GAP, |n, side| side_range(g, d, n, side, CORNER_INSET));
    let leaving = leaving_sides(d, &table);
    let slides: Vec<(f64, f64)> = ranges.into_iter().enumerate().map(|(i, range)| if alone_on_side(&table, d, &table.slots, &leaving, i) { (table.slots[i].coord, table.slots[i].coord) } else { range }).collect();
    let problem = |stems: &Stems| Problem {
        bounds: g.rects[1..].to_vec(),
        walls: (1..d.nodes.len()).flat_map(|n| node_obstacles(d, g, n)).collect(),
        connections: (0..d.edges.len()).map(|e| connection(d, g, &table, &slides, &stub_boxes, e, stems)).collect(),
        centred: true,
    };
    let base = Stems { long: vec![None; d.edges.len()], ports: false };
    let first = router::route(&problem(&base));
    // More room for interfaces is tried in a further route, first everything
    // (each cramped glyph's long run and the clearance at ports), then the long
    // runs alone, and the first that is no worse than the plain route in
    // failures, crossings, overlaps and bends is kept. The router settles which
    // line passes which by its order and costs, so a longer stem can tip two
    // lines into crossing; such a stem is left out rather than the drawing
    // changed.
    let long: Vec<Option<usize>> = (0..d.edges.len()).map(|e| cramped(d, &first, e)).collect();
    let ported = (0..d.edges.len()).any(|e| glyph_run(d.edges[e].kind) > 0.0 && (matches!(d.edges[e].from, End::Port(_)) || matches!(d.edges[e].to, End::Port(_))));
    let stretched = long.iter().any(Option::is_some);
    let (ported, stretched) = (roomy && ported, roomy && stretched);
    let trials = [(ported, Stems { long: long.clone(), ports: true }), (stretched, Stems { long, ports: false })];
    let standing = |s: &router::Solution| {
        let m = crate::core::metrics::measure(&s.paths);
        [s.failed.len(), m.crossings, m.overlaps, m.bends]
    };
    let plain = standing(&first);
    let solution = trials
        .into_iter()
        .filter(|(useful, _)| *useful)
        .map(|(_, stems)| router::route(&problem(&stems)))
        .find(|s| standing(s).iter().zip(&plain).all(|(b, a)| b <= a))
        .unwrap_or(first);
    // Note links are routed, so other lines keep clear of them, but drawn
    // straight (`geometry::note_link`).
    let edges = solution.paths.into_iter().enumerate().map(|(e, path)| match (d.edges[e].kind, d.edges[e].from, d.edges[e].to) {
        (EdgeKind::NoteLink, End::Node(note), End::Node(target)) => note_link(g.rects[note], g.rects[target]).to_vec(),
        _ => path,
    });
    Routes { edges: edges.collect(), stubs, ports: port_geometry(d, g, &table), failed: solution.failed }
}
