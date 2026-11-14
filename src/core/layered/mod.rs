// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Generic compound layered layout. Each container is laid out bottom-up as a
//! layered graph of its direct children, in explicit stages: lift the edges and
//! constraints to the children, assign ranks, insert dummies for long edges,
//! reduce crossings, then place items along both axes on the grid.
//!
//! Node 0 is the root. Every other node names a parent, and `Layout::rects` is
//! indexed like `Graph::nodes`. Nodes are boxes: a caller drawing a smaller or
//! rounder shape gives its bounding box. An edge whose ends lie under one child
//! of a container affects no layout at that level, and self-loops are ignored.
//!
//! The stages live in submodules: `rank` (lifting, ranking and dummies),
//! `simplex` (network simplex ranking), `order` (crossing reduction), `across`
//! (cross-axis placement and columns), `notes`, and `along` (main-axis
//! placement and composition). This module
//! holds the graph and result types, the data the stages share, and `arrange`,
//! which runs them for one container.
//!
//! `Graph::strategy` picks the method of a stage where more than one exists
//! (today only ranking has a choice: longest path or network simplex). It knows no engine names: callers map
//! their own names to a `Strategy`.

mod across;
mod along;
mod notes;
mod order;
mod rank;
mod simplex;
#[cfg(test)]
mod tests;

use super::geometry::{to_flow, Direction, Point, Rect};
use across::{align_pendants, columns, hold_columns, place_across, straighten_chains};
use along::{compose, label_spots, place_along, place_along_labelled};
use notes::{place_notes, with_notes, without_notes};
use order::{order_label_sides, reduce_crossings};
use rank::{insert_dummies, lift, ranks};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

/// Space between a container's boundary and its contents.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

/// Extra room a node needs in the gaps before and after it on the main axis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Margin {
    pub before: f64,
    pub after: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub parent: Option<usize>,
    /// The exact size of a leaf, or the minimum size of a container.
    pub size: Size,
    /// Only used when the node has children.
    pub insets: Insets,
    pub margin: Margin,
}

/// The main-axis gap an edge wants between the ranks it joins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gap {
    Fixed(f64),
    /// Like `Fixed`, and parallel edges add up so that decorations along them
    /// can stagger instead of stacking.
    Staggered(f64),
}

impl Gap {
    fn amount(self) -> f64 {
        match self {
            Gap::Fixed(amount) | Gap::Staggered(amount) => amount,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    /// How strongly the edge pulls its ends together. Use 1 by default.
    pub weight: u32,
    pub gap: Gap,
    /// A label to reserve room for. The edge then runs through `LabelSpot::anchor`.
    pub label: Option<EdgeLabel>,
}

/// Which side of its line, along the cross axis, a reserved label takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelSide {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeLabel {
    pub size: Size,
    pub side: LabelSide,
}

/// Where a reserved label went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelSpot {
    /// The point on the edge's line next to the label.
    pub anchor: Point,
    pub rect: Rect,
}

/// Ordering requests between nodes, applied at the level where they are siblings
/// or, for nested nodes, between the siblings that contain them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Constraint {
    /// The first node gets a lower rank than the second.
    Before(usize, usize),
    /// Same rank, with the first node earlier on the cross axis.
    AcrossBefore(usize, usize),
    SameRank(usize, usize),
    /// `moved` goes immediately after `anchor` if they share a rank.
    Adjacent { moved: usize, anchor: usize },
    /// Like `Adjacent`, and `moved` also sits at the minimum distance from
    /// `anchor`, instead of wherever median placement would leave it.
    Beside { moved: usize, anchor: usize },
    /// `note` annotates `anchor`: it takes no part in ranking, ordering or
    /// placement, and goes beside its anchor afterwards, in its rank, on the
    /// side that is free (Drawing principles, Flow: notes come last). Any other
    /// constraint on `note` is ignored.
    Note { note: usize, anchor: usize },
    /// The node is a decision: one of its outgoing edges always continues its
    /// column, even when no branch is longer (Flow: keep a column).
    Pivot(usize),
    /// The node spans what it connects to, as a fork or join bar does: it
    /// takes no column, and sits centred on its neighbours.
    Spread(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spacing {
    /// Smallest gap between ranks.
    pub rank_gap: f64,
    /// Gap between neighbouring items of a rank.
    pub item_gap: f64,
    /// Gap beside the invisible items carrying long edges.
    pub dummy_gap: f64,
    /// Added to the margins when they determine a rank gap.
    pub margin_clearance: f64,
    /// The part of a staggered gap that parallel edges may share.
    pub stagger_overlap: f64,
    /// Upper limit for the combined stagger in one rank gap.
    pub stagger_limit: f64,
    /// Container sizes are rounded up to multiples of this.
    pub size_step: f64,
    /// Free space on each main-axis side of a reserved label.
    pub label_gap: f64,
    /// Whether chains of items keep a column (`columns`).
    pub columns: bool,
}

impl Default for Spacing {
    fn default() -> Spacing {
        Spacing { rank_gap: 64.0, item_gap: 40.0, dummy_gap: 16.0, margin_clearance: 24.0, stagger_overlap: 24.0, stagger_limit: 320.0, size_step: 16.0, label_gap: 24.0, columns: true }
    }
}

/// How each container's children get their ranks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ranking {
    /// Longest path from the sources, then sources pulled next to their
    /// targets (`rank::assign_ranks`).
    #[default]
    LongestPath,
    /// Ranks that minimise the total weighted edge length, by network simplex
    /// (Gansner et al. 1993; `simplex::network_simplex`). A node then sits
    /// next to the larger weight of its neighbours, not as high as it can.
    /// Same cycle handling, constraints and groups as `LongestPath`.
    NetworkSimplex,
    /// Every child in rank 0. Only for tests that show the choice reaches the
    /// ranking stage.
    #[cfg(test)]
    Flat,
}

/// The method chosen for each stage that has a choice, as opposed to the
/// distances in `Spacing`. A new method for a stage is a new variant of that
/// stage's enum; a stage that gains a choice gets a field here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Strategy {
    pub ranking: Ranking,
}

#[derive(Clone, Debug)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Applied in order. An earlier constraint wins when they conflict, and all
    /// constraints win over edges.
    pub constraints: Vec<Constraint>,
    pub spacing: Spacing,
    /// The method of each stage that has a choice.
    pub strategy: Strategy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub rects: Vec<Rect>,
    /// Indexed like `Graph::edges`. `None` for an edge without a label, and for a
    /// label whose edge has no rank gap to sit in: same-rank edges and edges that
    /// do not join two children of one container.
    pub labels: Vec<Option<LabelSpot>>,
}

/// Lays out the graph with its root at the origin.
pub fn layout(graph: &Graph) -> Layout {
    let children = child_lists(graph);
    let tree = arrange(graph, &children, 0);
    let mut placed = flatten(&tree, (0.0, 0.0));
    placed.sort_by_key(|&(node, _)| node);
    let spots = flatten_labels(&tree, (0.0, 0.0));
    let labels = (0..graph.edges.len()).map(|e| spots.iter().find(|(edge, _)| *edge == e).map(|(_, spot)| *spot)).collect();
    Layout { rects: placed.into_iter().map(|(_, rect)| rect).collect(), labels }
}

/// A node with its size, its offset in its parent, and its arranged children.
struct Arranged {
    node: usize,
    size: Size,
    at: (f64, f64),
    children: Vec<Arranged>,
    /// Reserved labels of edges joining this node's children, relative to it.
    labels: Vec<(usize, LabelSpot)>,
}

fn child_lists(graph: &Graph) -> Vec<Vec<usize>> {
    let empty = vec![Vec::new(); graph.nodes.len()];
    graph.nodes.iter().enumerate().fold(empty, |mut lists, (index, node)| {
        if let Some(parent) = node.parent {
            lists[parent].push(index);
        }
        lists
    })
}

fn flatten(tree: &Arranged, origin: (f64, f64)) -> Vec<(usize, Rect)> {
    let (x, y) = (origin.0 + tree.at.0, origin.1 + tree.at.1);
    let own = (tree.node, Rect { x, y, w: tree.size.w, h: tree.size.h });
    std::iter::once(own).chain(tree.children.iter().flat_map(|child| flatten(child, (x, y)))).collect()
}

fn flatten_labels(tree: &Arranged, origin: (f64, f64)) -> Vec<(usize, LabelSpot)> {
    let (x, y) = (origin.0 + tree.at.0, origin.1 + tree.at.1);
    let shift = |spot: &LabelSpot| LabelSpot { anchor: (spot.anchor.0 + x, spot.anchor.1 + y), rect: Rect { x: spot.rect.x + x, y: spot.rect.y + y, ..spot.rect } };
    let own = tree.labels.iter().map(|(edge, spot)| (*edge, shift(spot)));
    own.chain(tree.children.iter().flat_map(|child| flatten_labels(child, (x, y)))).collect()
}

/// Splits a size into (main axis, cross axis).
fn along_flow(direction: Direction, size: Size) -> (f64, f64) {
    to_flow(direction, (size.w, size.h))
}

fn arrange(graph: &Graph, children: &[Vec<usize>], node: usize) -> Arranged {
    if children[node].is_empty() {
        return Arranged { node, size: graph.nodes[node].size, at: (0.0, 0.0), children: Vec::new(), labels: Vec::new() };
    }
    let arranged: Vec<Arranged> = children[node].iter().map(|&child| arrange(graph, children, child)).collect();
    let local = lift(graph, node, &children[node], &arranged);
    // Labelled containers get a rank between every two ranks of items, for the labels.
    let stretch = if local.edges.iter().any(|edge| edge.label.is_some()) { 2 } else { 1 };
    let ranked: Vec<usize> = ranks(graph.strategy.ranking, &local).into_iter().map(|rank| rank * stretch).collect();
    let item_rank: Vec<usize> = (0..local.count).map(|x| local.notes.iter().find(|&&(n, _)| n == x).map_or(ranked[x], |&(_, anchor)| ranked[anchor])).collect();
    let layers = without_notes(insert_dummies(&local, &item_rank), &local);
    let order = reduce_crossings(&layers, &local);
    let order = order_label_sides(order, &layers);
    let partners = columns(&local, &layers, &item_rank, stretch, &order);
    let placed = place_across(&local, &layers, &order, &partners);
    let placed = hold_columns(&local, &layers, &order, &partners, placed);
    let cross = align_pendants(&local, &layers, &order, straighten_chains(&local, &layers, &order, placed));
    let cross = place_notes(&local, &layers, &order, &item_rank, cross);
    let rows = with_notes(&order, &local, &item_rank);
    let along = if stretch == 2 { place_along_labelled(&local, &layers, &rows) } else { place_along(&local, &layers, &rows) };
    let spots = label_spots(graph.direction, &layers, &along, &cross);
    compose(graph, node, arranged, &local, &along, &cross, spots)
}

struct Lifted {
    from: usize,
    to: usize,
    weight: u32,
    gap: Gap,
    /// The label and the index of the edge in `Graph::edges`.
    label: Option<(EdgeLabel, usize)>,
}

/// One container's problem, in terms of local child positions.
struct Local {
    direction: Direction,
    count: usize,
    main_size: Vec<f64>,
    cross_size: Vec<f64>,
    margin: Vec<Margin>,
    edges: Vec<Lifted>,
    before: Vec<(usize, usize)>,
    across: Vec<(usize, usize)>,
    adjacent: Vec<(usize, usize)>,
    /// `(moved, anchor)` pairs from `Constraint::Beside`, also listed in `adjacent`.
    beside: Vec<(usize, usize)>,
    /// Each child's representative in its same-rank group.
    group: Vec<usize>,
    /// `(note, anchor)` pairs from `Constraint::Note`.
    notes: Vec<(usize, usize)>,
    /// Decisions, from `Constraint::Pivot`.
    pivots: Vec<bool>,
    /// Bars, from `Constraint::Spread`.
    spread: Vec<bool>,
    spacing: Spacing,
}

/// The layered graph. Items at or beyond `Local::count` are dummies.
struct Layers {
    ranks: Vec<Vec<usize>>,
    links: Vec<(usize, usize)>,
    total: usize,
    gap_need: Vec<f64>,
    labels: Vec<LabelDummy>,
    /// Each long edge as its lower-ranked end, its dummies in rank order, and
    /// its higher-ranked end.
    chains: Vec<(usize, Vec<usize>, usize)>,
}

/// The dummy that holds a label's place in its rank. Its cross position is the
/// line's, and the label extends to one side of it.
struct LabelDummy {
    item: usize,
    rank: usize,
    /// The edge's lower-ranked and higher-ranked ends.
    ends: (usize, usize),
    edge: usize,
    side: LabelSide,
    /// The label's length along the main axis.
    main: f64,
    /// The room taken on the cross axis before and after the line.
    before: f64,
    after: f64,
}

impl Layers {
    fn label_at(&self, item: usize) -> Option<&LabelDummy> {
        self.labels.iter().find(|label| label.item == item)
    }
}

/// Distance between a line and the label beside it.
const LABEL_OFFSET: f64 = 8.0;
