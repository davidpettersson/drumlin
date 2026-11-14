// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Adapts a package diagram to the generic layered layout: node sizes, tab
//! geometry, edge gaps, layout hints and wrapping of unconnected siblings.

use super::model::*;
use crate::core::geometry::{snap_up, Direction};
use crate::core::glyphs::note_size;
use crate::core::layered::{self, Constraint, Gap, Graph, Insets, Layout, Size, Spacing};
use crate::core::text::{name_width, small_width};
use crate::diagrams::boxes;

const PADDING: f64 = 24.0;
/// Spacing of connection slots along a side, and their clearance from corners.
pub const SLOT: f64 = 24.0;
pub const SLOT_INSET: f64 = 16.0;
const TAB_HEIGHT: f64 = 24.0;
/// A tab holding a keyword line above the name.
const TAB_HEIGHT_KEYWORD: f64 = 40.0;
/// A package without visible contents has a blank tab and its name in the body.
const LEAF_TAB: (f64, f64) = (40.0, 16.0);

/// The width and height of a package's tab.
pub fn tab_size(node: &Node) -> (f64, f64) {
    if node.children.is_empty() {
        return LEAF_TAB;
    }
    let name = name_width(&node.name);
    match keywords(node) {
        Some(k) => (name.max(small_width(&k)) + 24.0, TAB_HEIGHT_KEYWORD),
        None => (name + 24.0, TAB_HEIGHT),
    }
}

/// The smallest (cross-flow) side length that fits this node's connection slots,
/// as a minimum size. The incoming side also has to clear a package's tab.
fn slot_room(d: &Diagram, n: usize) -> Size {
    let node = &d.nodes[n];
    if !matches!(node.kind, Kind::Package | Kind::Class | Kind::Interface | Kind::Enum) {
        return Size::default();
    }
    let relations = || d.edges.iter().filter(|edge| edge.line != Line::NoteLink);
    let incoming = relations().filter(|edge| edge.to == n).count();
    let outgoing = {
        let plain = relations().filter(|edge| edge.from == n && edge.line != Line::Containment).count();
        plain + usize::from(relations().any(|edge| edge.from == n && edge.line == Line::Containment))
    };
    let span = |count: usize| if count == 0 { 0.0 } else { (count - 1) as f64 * SLOT + 2.0 * SLOT_INSET };
    let tab = if node.kind == Kind::Package { tab_size(node) } else { (0.0, 0.0) };
    match d.direction {
        Direction::Down => Size { w: span(outgoing).max(span(incoming) + tab.0), h: 0.0 },
        Direction::Right => Size { w: 0.0, h: span(outgoing).max(span(incoming)) + tab.1 },
    }
}

fn leaf_size(node: &Node) -> Size {
    if node.kind == Kind::Note {
        let (w, h) = note_size(&node.name);
        return Size { w, h };
    }
    let keyword = keywords(node);
    let keyword_width = keyword.as_ref().map_or(0.0, |k| small_width(k));
    let text = name_width(&node.name).max(keyword_width);
    let lines = if keyword.is_some() { 56.0 } else { 40.0 };
    match node.kind {
        Kind::Package => Size { w: snap_up((text + 40.0).max(112.0), 16.0), h: snap_up(lines + LEAF_TAB.1 + 8.0, 16.0) },
        _ => Size { w: snap_up((text + 32.0).max(96.0), 16.0), h: snap_up(lines, 16.0) },
    }
}

/// Extra room on a container's far side for relations drawn to its inside, so
/// their keyword labels fit between the nested element and the boundary.
fn inward_room(d: &Diagram, n: usize) -> f64 {
    let labels = d.edges.iter().filter(|edge| d.nests(edge) && (edge.from == n || edge.to == n)).map(|edge| edge.label.as_ref().map_or(16.0, |l| small_width(l) + 16.0));
    match d.direction {
        Direction::Right => snap_up(labels.fold(0.0, f64::max), 8.0),
        Direction::Down => if labels.count() > 0 { 16.0 } else { 0.0 },
    }
}

pub fn insets(d: &Diagram, n: usize) -> Insets {
    let node = &d.nodes[n];
    let top = if node.kind == Kind::Package { tab_size(node).1 + PADDING } else { 0.0 };
    let pad = if node.kind == Kind::Root { 0.0 } else { PADDING };
    let far = inward_room(d, n);
    match d.direction {
        Direction::Down => Insets { top, right: pad, bottom: pad + far, left: pad },
        Direction::Right => Insets { top, right: pad + far, bottom: pad, left: pad },
    }
}

fn node_spec(d: &Diagram, n: usize) -> layered::Node {
    let node = &d.nodes[n];
    let base = if node.children.is_empty() { leaf_size(node) } else { Size { w: tab_size(node).0 + 32.0, h: 0.0 } };
    let room = slot_room(d, n);
    let size = Size { w: snap_up(base.w.max(room.w), 16.0), h: snap_up(base.h.max(room.h), 16.0) };
    layered::Node { parent: node.parent, size, insets: insets(d, n), margin: layered::Margin::default() }
}

fn edge_spec(d: &Diagram, edge: &Edge) -> layered::Edge {
    let label = edge.label.as_ref().map_or(0.0, |l| small_width(l) + 32.0);
    let gap = match d.direction {
        Direction::Right => Gap::Fixed(label),
        Direction::Down => Gap::Fixed(0.0),
    };
    let weight = if edge.line == Line::Containment { 3 } else { 1 };
    layered::Edge { from: edge.from, to: edge.to, weight, gap, label: None }
}

fn hinted(d: &Diagram, n: usize) -> bool {
    d.hints.iter().any(|hint| match *hint {
        Hint::Main(a, b) | Hint::Cross(a, b) | Hint::SameRank(a, b) => a == n || b == n,
    })
}

/// True when an edge joins `child`'s subtree to a sibling's within `container`.
fn linked_within(d: &Diagram, container: usize, child: usize) -> bool {
    d.edges.iter().filter(|edge| edge.line != Line::NoteLink && !d.nests(edge)).any(|edge| {
        let (from_here, to_here) = (d.contains(child, edge.from), d.contains(child, edge.to));
        from_here != to_here && d.contains(container, edge.from) && d.contains(container, edge.to)
    })
}

/// Unconnected siblings would otherwise share one very long rank. This folds
/// them into a compact block of ranks, leaving connected siblings alone.
fn wrap_constraints(d: &Diagram, container: usize) -> Vec<Constraint> {
    let free: Vec<usize> = d.nodes[container]
        .children
        .iter()
        .copied()
        .filter(|&c| d.nodes[c].kind != Kind::Note && !hinted(d, c) && !linked_within(d, container, c))
        .collect();
    if free.len() < 4 {
        return Vec::new();
    }
    let columns = (free.len() as f64).sqrt().ceil() as usize;
    let rows: Vec<&[usize]> = free.chunks(columns).collect();
    let same = rows.iter().flat_map(|row| row.windows(2).map(|pair| Constraint::SameRank(pair[0], pair[1])));
    let order = rows.windows(2).map(|pair| Constraint::Before(pair[0][0], pair[1][0]));
    same.chain(order).collect()
}

fn graph(d: &Diagram) -> Graph {
    let note_links = d.edges.iter().filter(|edge| edge.line == Line::NoteLink).map(|edge| Constraint::Note { note: edge.from, anchor: edge.to });
    let wraps = (0..d.nodes.len()).flat_map(|n| wrap_constraints(d, n));
    Graph {
        direction: d.direction,
        nodes: (0..d.nodes.len()).map(|n| node_spec(d, n)).collect(),
        edges: d.edges.iter().filter(|edge| edge.line != Line::NoteLink && !d.nests(edge)).map(|edge| edge_spec(d, edge)).collect(),
        constraints: d.hints.iter().map(boxes::hint_constraint).chain(note_links).chain(wraps).collect(),
        spacing: Spacing::default(),
        strategy: d.engine.strategy,
    }
}

/// The layout with chains in columns, and the layout without.
pub fn layouts(d: &Diagram) -> [Layout; 2] {
    boxes::layouts(&graph(d))
}
