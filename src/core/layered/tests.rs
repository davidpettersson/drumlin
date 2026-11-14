// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for the layered layout.

use super::*;

fn leaf(parent: usize) -> Node {
    Node { parent: Some(parent), size: Size { w: 96.0, h: 48.0 }, ..Node::default() }
}

fn edge(from: usize, to: usize) -> Edge {
    Edge { from, to, weight: 1, gap: Gap::Fixed(0.0), label: None }
}

fn graph(nodes: Vec<Node>, edges: Vec<Edge>, constraints: Vec<Constraint>) -> Graph {
    Graph { direction: Direction::Right, nodes, edges, constraints, spacing: Spacing::default(), strategy: Strategy::default() }
}

fn root_with(leaves: usize) -> Vec<Node> {
    std::iter::once(Node::default()).chain((0..leaves).map(|_| leaf(0))).collect()
}

fn assert_no_sibling_overlap(g: &Graph, layout: &Layout) {
    for (a, na) in g.nodes.iter().enumerate() {
        for (b, nb) in g.nodes.iter().enumerate().skip(a + 1) {
            if na.parent.is_some() && na.parent == nb.parent {
                assert!(!layout.rects[a].intersects(&layout.rects[b]), "nodes {a} and {b} overlap");
            }
        }
    }
}

#[test]
fn layout_is_deterministic() {
    let g = graph(root_with(6), vec![edge(1, 2), edge(1, 3), edge(2, 4), edge(3, 4), edge(5, 6), edge(1, 6)], vec![]);
    assert_eq!(layout(&g), layout(&g.clone()));
}

#[test]
fn siblings_do_not_overlap_even_in_a_cycle() {
    let g = graph(root_with(5), vec![edge(1, 2), edge(2, 3), edge(3, 1), edge(1, 4), edge(1, 5)], vec![]);
    assert_no_sibling_overlap(&g, &layout(&g));
}

#[test]
fn edges_advance_along_the_flow() {
    let result = layout(&graph(root_with(2), vec![edge(1, 2)], vec![]));
    assert!(result.rects[1].right() + 64.0 <= result.rects[2].x);
    let down = Graph { direction: Direction::Down, ..graph(root_with(2), vec![edge(1, 2)], vec![]) };
    let result = layout(&down);
    assert!(result.rects[1].bottom() + 64.0 <= result.rects[2].y);
}

#[test]
fn containers_enclose_their_children_and_keep_minimum_size() {
    let insets = Insets { top: 40.0, right: 24.0, bottom: 24.0, left: 24.0 };
    let nodes = vec![
        Node::default(),
        Node { parent: Some(0), size: Size { w: 640.0, h: 0.0 }, insets, ..Node::default() },
        leaf(1),
        leaf(1),
    ];
    let g = graph(nodes, vec![edge(2, 3)], vec![]);
    let result = layout(&g);
    let outer = result.rects[1];
    assert_eq!(outer.w, 640.0);
    for inner in &result.rects[2..] {
        assert!(inner.x >= outer.x + 24.0 && inner.y >= outer.y + 40.0);
        assert!(inner.right() <= outer.right() - 24.0 && inner.bottom() <= outer.bottom() - 24.0);
    }
}

#[test]
fn constraints_override_edge_direction_and_ordering() {
    let result = layout(&graph(root_with(2), vec![edge(1, 2)], vec![Constraint::Before(2, 1)]));
    assert!(result.rects[2].right() <= result.rects[1].x);
    let result = layout(&graph(root_with(2), vec![], vec![Constraint::AcrossBefore(2, 1)]));
    assert_eq!(result.rects[1].x, result.rects[2].x);
    assert!(result.rects[2].bottom() <= result.rects[1].y);
}

#[test]
fn a_labelled_edge_reserves_room_beside_its_line() {
    let label = EdgeLabel { size: Size { w: 120.0, h: 14.0 }, side: LabelSide::After };
    let edges = vec![Edge { label: Some(label), ..edge(1, 2) }];
    let g = Graph { direction: Direction::Down, ..graph(root_with(2), edges, vec![]) };
    let result = layout(&g);
    let spot = result.labels[0].expect("the label has a spot");
    assert!(spot.rect.x >= spot.anchor.0 + 8.0);
    assert!(result.rects[1].bottom() < spot.rect.y && spot.rect.bottom() < result.rects[2].y);
    assert!(result.rects[0].right() >= spot.rect.right(), "the root grows around the label");
}

#[test]
fn opposite_edges_keep_their_labels_on_the_outside() {
    let (before, after) = (EdgeLabel { size: Size { w: 120.0, h: 14.0 }, side: LabelSide::Before }, EdgeLabel { size: Size { w: 120.0, h: 14.0 }, side: LabelSide::After });
    let edges = vec![Edge { label: Some(after), ..edge(1, 2) }, Edge { label: Some(before), ..edge(2, 1) }];
    let g = Graph { direction: Direction::Down, ..graph(root_with(2), edges, vec![]) };
    let result = layout(&g);
    let (a, b) = (result.labels[0].unwrap(), result.labels[1].unwrap());
    assert!(b.anchor.0 < a.anchor.0 && a.rect.x > b.rect.right());
}

#[test]
fn long_edges_and_pendants_line_up_with_their_ends() {
    let small = Node { parent: Some(0), size: Size { w: 16.0, h: 16.0 }, ..Node::default() };
    let wide = Node { parent: Some(0), size: Size { w: 160.0, h: 48.0 }, ..Node::default() };
    let label = EdgeLabel { size: Size { w: 80.0, h: 14.0 }, side: LabelSide::After };
    // A small start above a wide box that fans out to two boxes, one of them far.
    let nodes = vec![Node::default(), small, wide, leaf(0), leaf(0)];
    let edges = vec![edge(1, 2), Edge { label: Some(label), ..edge(2, 3) }, Edge { label: Some(label), ..edge(2, 4) }, edge(3, 4)];
    // Without columns: with them, the three boxes stack and the long edge
    // must pass beside the middle one.
    let plain = graph(nodes, edges, vec![]);
    let g = Graph { direction: Direction::Down, spacing: Spacing { columns: false, ..plain.spacing }, ..plain };
    let result = layout(&g);
    let (start, wide) = (result.rects[1], result.rects[2]);
    assert!(start.center().0 >= wide.x + 16.0 && start.center().0 <= wide.right() - 16.0, "the start sits over the box");
    let spot = result.labels[2].expect("a reserved label");
    let far = result.rects[4];
    assert!((far.x + 16.0..=far.right() - 16.0).contains(&spot.anchor.0) || (wide.x + 16.0..=wide.right() - 16.0).contains(&spot.anchor.0), "the long edge meets one end straight");
}

#[test]
fn a_chain_keeps_its_column_and_a_note_takes_the_free_side() {
    let wide = Node { parent: Some(0), size: Size { w: 160.0, h: 48.0 }, ..Node::default() };
    // 1 -> 2 -> 3 -> 4 is the main chain; 2 -> 5 is a side branch beside 3;
    // 6 is a note on 3.
    let nodes = vec![Node::default(), leaf(0), wide, leaf(0), leaf(0), leaf(0), leaf(0)];
    let edges = vec![edge(1, 2), edge(2, 3), edge(3, 4), edge(2, 5)];
    let g = Graph { direction: Direction::Down, ..graph(nodes, edges, vec![Constraint::Note { note: 6, anchor: 3 }]) };
    let result = layout(&g);
    let centre = |n: usize| result.rects[n].center().0;
    assert!([2, 3, 4].iter().all(|&n| centre(n) == centre(1)), "the chain is one column: {:?}", result.rects);
    assert_eq!(result.rects[6].center().1, result.rects[3].center().1, "the note sits in its anchor's rank");
    let side = (centre(6) - centre(3)).signum();
    assert_ne!(side, (centre(5) - centre(3)).signum(), "the note goes on the side away from the branch");
    assert_no_sibling_overlap(&g, &result);
}

#[test]
fn varied_node_sizes_share_a_rank_centre() {
    let small = Node { parent: Some(0), size: Size { w: 16.0, h: 16.0 }, ..Node::default() };
    let nodes = vec![Node::default(), small.clone(), leaf(0), small];
    let g = graph(nodes, vec![edge(1, 2), edge(2, 3)], vec![]);
    let result = layout(&g);
    assert_no_sibling_overlap(&g, &result);
    assert_eq!(result.rects[1].center().1, result.rects[2].center().1);
    assert_eq!(result.rects[2].center().1, result.rects[3].center().1);
}

#[test]
fn the_strategy_chooses_the_ranking() {
    let g = Graph { direction: Direction::Down, ..graph(root_with(2), vec![edge(1, 2)], vec![]) };
    assert_eq!(g.strategy.ranking, Ranking::LongestPath);
    let ranked = layout(&g);
    assert!(ranked.rects[1].bottom() < ranked.rects[2].y, "longest path puts the target a rank below");
    let flat = layout(&Graph { strategy: Strategy { ranking: Ranking::Flat }, ..g });
    assert_eq!(flat.rects[1].y, flat.rects[2].y, "flat ranking keeps both in one rank");
    assert!(flat.rects[1].right() < flat.rects[2].x || flat.rects[2].right() < flat.rects[1].x);
}

#[test]
fn network_simplex_ranks_a_node_next_to_its_children() {
    // 1 -> 2 -> 3 -> {5, 6}, and 4 hangs from 1 and feeds 5 and 6.
    let edges = vec![edge(1, 2), edge(2, 3), edge(3, 5), edge(3, 6), edge(1, 4), edge(4, 5), edge(4, 6)];
    let g = Graph { direction: Direction::Down, ..graph(root_with(6), edges, vec![]) };
    let longest = layout(&g);
    assert_eq!(longest.rects[4].y, longest.rects[2].y, "longest path puts 4 right under 1");
    let simplex = layout(&Graph { strategy: Strategy { ranking: Ranking::NetworkSimplex }, ..g.clone() });
    assert_eq!(simplex.rects[4].y, simplex.rects[3].y, "network simplex puts 4 right over its children");
    assert_no_sibling_overlap(&g, &simplex);
    let chain = graph(root_with(3), vec![edge(1, 2), edge(2, 3)], vec![]);
    assert_eq!(layout(&Graph { strategy: Strategy { ranking: Ranking::NetworkSimplex }, ..chain.clone() }), layout(&chain), "an optimal longest-path ranking is kept");
}
