// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Adapts a component diagram to the generic layered layout: node sizes, margins
//! for stubs and ports, edge gaps and layout hints.

use super::model::*;
use crate::core::geometry::{snap_up, Direction, Side};
use crate::core::glyphs::note_size;
use crate::core::layered::{self, Constraint, Gap, Graph, Insets, Layout, Margin, Size, Spacing};
use crate::core::text::{name_width, small_width};
use crate::diagrams::boxes;

pub const PACKAGE_TAB: f64 = 24.0;
pub const STUB: f64 = 32.0;
pub const SLOT: f64 = 24.0;
/// How far the first and last thing on a side keep from its corners.
pub const SLOT_INSET: f64 = 16.0;
/// The distance between neighbouring ends on a left or right side when one of
/// their lines carries a name: the name sits above its line, and needs its
/// height and air between the two lines.
pub const NAMED_SLOT: f64 = 32.0;

/// The distance ends on `side` keep from each other: `NAMED_SLOT` on a left or
/// right side where a line carries a name, else `SLOT`.
pub fn end_pitch(side: Side, named: bool) -> f64 {
    if named && side.is_vertical_edge() { NAMED_SLOT } else { SLOT }
}
const HEADER: f64 = 48.0;
/// Room for a socket, a dependency arrow and a lollipop, beyond the usual gap.
const WIRE: f64 = 72.0;
const PADDING: f64 = 24.0;
/// Padding on a side whose port takes an arrow from inside: the port's half
/// width, the arrowhead and its straight run, and the clearance from boxes.
const HEADED_PORT_INSET: f64 = 32.0;
/// A composite's keyword line, which sits above its centred name.
const KEYWORD_ROW: f64 = 16.0;

pub fn stub_port_label(d: &Diagram, stub: &Stub) -> f64 {
    match stub.at {
        End::Port(p) => small_width(&d.ports[p].name) + 8.0,
        End::Node(_) => 0.0,
    }
}

/// Space a stub needs beyond the boundary of its node.
fn stub_reach(d: &Diagram, stub: &Stub) -> f64 {
    let name = stub_port_label(d, stub);
    let label = small_width(&stub.interface);
    match d.direction {
        Direction::Right => STUB + name + (label / 2.0).max(8.0) + 8.0,
        Direction::Down => STUB + name + 16.0,
    }
}

pub fn stub_side(d: &Diagram, stub: &Stub) -> Side {
    match (stub.at, stub.role) {
        (End::Port(p), _) => d.port_side(p),
        (End::Node(_), Role::Provided) => d.start_side(),
        (End::Node(_), Role::Required) => d.end_side(),
    }
}

fn owner_of(d: &Diagram, stub: &Stub) -> usize {
    d.owner(stub.at)
}

/// Space a node needs on (start, end) of the main axis for stubs and ports.
fn margin(d: &Diagram, n: usize) -> Margin {
    let stubs = d.stubs.iter().filter(|stub| owner_of(d, stub) == n).map(|stub| (stub_side(d, stub), stub_reach(d, stub)));
    let ports = d.nodes[n].ports.iter().map(|&p| (d.port_side(p), small_width(&d.ports[p].name) + 16.0));
    let demands: Vec<(Side, f64)> = stubs.chain(ports).collect();
    let on = |side: Side| demands.iter().filter(|demand| demand.0 == side).map(|demand| demand.1).fold(0.0, f64::max);
    Margin { before: on(d.start_side()), after: on(d.end_side()) }
}

/// The side of node `n` that the end of `edge` at `n` is expected to meet, as
/// the flow places it: the end side for a line that leaves, the start side for
/// one that arrives, and a port's own side (or the side facing it) for a line
/// to a port. `None` when `n` is not a plain end of the edge.
fn expected_side(d: &Diagram, n: usize, edge: &Edge) -> Option<Side> {
    if edge.kind == EdgeKind::NoteLink || edge.from == edge.to {
        return None;
    }
    let toward_port = |other: End, plain: Side| match other {
        End::Port(q) if d.contains(d.ports[q].owner, n) => d.port_side(q),
        End::Port(q) => d.port_side(q).opposite(),
        End::Node(_) => plain,
    };
    if edge.from == End::Node(n) && edge.to != End::Node(n) {
        Some(toward_port(edge.to, d.end_side()))
    } else if edge.to == End::Node(n) {
        Some(toward_port(edge.from, d.start_side()))
    } else {
        None
    }
}

/// Minimum boundary length needed for the stubs, ports and connector ends on
/// one side (Drawing principles, Shapes: leave room for what attaches): stubs
/// and ports take a slot each plus 16px at both corners, and all the things on
/// the side together keep `SLOT` apart and `SLOT_INSET` from the corners, so
/// three connectors on one side make the box taller rather than meet it at
/// its corners.
fn side_needs(d: &Diagram, n: usize, side: Side, roomy: bool) -> f64 {
    let room = |label: &str| if side.is_vertical_edge() { SLOT } else { small_width(label) + 16.0 };
    let stubs: Vec<f64> = d.stubs.iter().filter(|stub| stub.at == End::Node(n) && stub_side(d, stub) == side).map(|stub| room(&stub.interface)).collect();
    let ports: Vec<f64> = d.nodes[n].ports.iter().filter(|&&p| d.port_side(p) == side).map(|&p| room(&d.ports[p].name)).collect();
    let total: f64 = stubs.iter().chain(&ports).sum();
    let fixed = if total > 0.0 { total + 2.0 * SLOT_INSET } else { 0.0 };
    let ends: Vec<&Edge> = d.edges.iter().filter(|edge| roomy && expected_side(d, n, edge) == Some(side)).collect();
    let pitch = end_pitch(side, ends.len() >= 2 && ends.iter().any(|edge| edge.label.is_some()));
    let items = stubs.len() + ports.len() + ends.len();
    let spread = if items >= 2 { (items - 1) as f64 * pitch + 2.0 * SLOT_INSET } else { 0.0 };
    fixed.max(spread)
}

fn leaf_size(d: &Diagram, n: usize, roomy: bool) -> Size {
    let node = &d.nodes[n];
    if node.kind == Kind::Note {
        let (w, h) = note_size(&node.name);
        return Size { w, h };
    }
    let name = name_width(&node.name);
    let stereo = node.stereotype.as_ref().map_or(0.0, |s| small_width(&format!("\u{ab}{s}\u{bb}")));
    let base = if node.stereotype.is_some() { 64.0 } else { 56.0 };
    let tab = if node.kind == Kind::Package { PACKAGE_TAB } else { 0.0 };
    let w = (name.max(stereo) + 2.0 * 28.0).max(128.0).max(side_needs(d, n, Side::Top, roomy)).max(side_needs(d, n, Side::Bottom, roomy));
    let h = (base + tab).max(side_needs(d, n, Side::Left, roomy)).max(side_needs(d, n, Side::Right, roomy));
    Size { w: snap_up(w, 16.0), h: snap_up(h, 16.0) }
}

fn container_minimum(d: &Diagram, n: usize, roomy: bool) -> Size {
    let node = &d.nodes[n];
    let keyword = node.stereotype.as_ref().map_or(0.0, |s| small_width(&format!("\u{ab}{s}\u{bb}")));
    Size { w: name_width(&node.name).max(keyword) + 64.0, h: side_needs(d, n, Side::Left, roomy).max(side_needs(d, n, Side::Right, roomy)) }
}

pub fn insets(d: &Diagram, n: usize) -> Insets {
    let node = &d.nodes[n];
    let pad = |top: f64, side: f64| Insets { top, right: side, bottom: side, left: side };
    let base = match node.kind {
        Kind::Root => pad(0.0, 0.0),
        Kind::Package => pad(PACKAGE_TAB + PADDING, PADDING),
        Kind::Component | Kind::Note => pad(HEADER + if node.stereotype.is_some() { KEYWORD_ROW } else { 0.0 }, PADDING),
    };
    // An arrow from inside that ends on a port needs room for its head and a
    // straight run before it, beyond the port's half width and the clearance.
    let headed = |side: Side| {
        node.ports.iter().any(|&p| d.port_side(p) == side && d.edges.iter().any(|edge| edge.to == End::Port(p) && edge.head == Head::Open && d.owner(edge.from) != n && d.contains(n, d.owner(edge.from))))
    };
    let room = |side: Side, inset: f64| if headed(side) { inset.max(HEADED_PORT_INSET) } else { inset };
    Insets { top: room(Side::Top, base.top), right: room(Side::Right, base.right), bottom: room(Side::Bottom, base.bottom), left: room(Side::Left, base.left) }
}

/// Room a node takes before and after itself along the flow, for its stubs
/// and ports.
pub fn margin_of(d: &Diagram, n: usize) -> (f64, f64) {
    let m = margin(d, n);
    (m.before, m.after)
}

fn node_spec(d: &Diagram, n: usize, roomy: bool) -> layered::Node {
    let node = &d.nodes[n];
    let size = if node.children.is_empty() { leaf_size(d, n, roomy) } else { container_minimum(d, n, roomy) };
    layered::Node { parent: node.parent, size, insets: insets(d, n), margin: margin(d, n) }
}

/// Room a port's name takes on a connector's run.
fn port_name_room(d: &Diagram, end: End) -> f64 {
    match end {
        End::Port(p) => small_width(&d.ports[p].name) + 8.0,
        End::Node(_) => 0.0,
    }
}

fn edge_spec(d: &Diagram, edge: &Edge) -> layered::Edge {
    let label = edge.label.as_ref().map_or(0.0, |l| small_width(l) + 32.0);
    let names = port_name_room(d, edge.from) + port_name_room(d, edge.to);
    let gap = match (edge.kind, d.direction) {
        (EdgeKind::Assembly, Direction::Right) => Gap::Staggered(label + 40.0),
        (EdgeKind::Assembly, Direction::Down) => Gap::Fixed(40.0),
        (EdgeKind::Wired, Direction::Right) => Gap::Staggered(label.max(WIRE) + names + 40.0),
        (EdgeKind::Wired, Direction::Down) => Gap::Fixed(WIRE + 40.0),
        (_, Direction::Right) => Gap::Fixed(label),
        (_, Direction::Down) => Gap::Fixed(0.0),
    };
    layered::Edge { from: d.owner(edge.from), to: d.owner(edge.to), weight: 1, gap, label: None }
}

fn graph(d: &Diagram, roomy: bool) -> Graph {
    let is_note_link = |edge: &&Edge| edge.kind == EdgeKind::NoteLink;
    let note_links = d.edges.iter().filter(is_note_link).map(|edge| Constraint::Adjacent { moved: d.owner(edge.from), anchor: d.owner(edge.to) });
    Graph {
        direction: d.direction,
        nodes: (0..d.nodes.len()).map(|n| node_spec(d, n, roomy)).collect(),
        edges: d.edges.iter().filter(|edge| edge.kind != EdgeKind::NoteLink).map(|edge| edge_spec(d, edge)).collect(),
        constraints: d.hints.iter().map(boxes::hint_constraint).chain(note_links).collect(),
        spacing: Spacing::default(),
        strategy: d.engine.strategy,
    }
}

/// The layout with chains in columns, and the layout without. With `roomy`,
/// a side also makes room for the connector ends the flow will put on it.
pub fn layouts(d: &Diagram, roomy: bool) -> [Layout; 2] {
    boxes::layouts(&graph(d, roomy))
}

/// Whether making room for connector ends changes any box's size.
pub fn ends_need_room(d: &Diagram) -> bool {
    (0..d.nodes.len()).any(|n| node_spec(d, n, true).size != node_spec(d, n, false).size)
}
