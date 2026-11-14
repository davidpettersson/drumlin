// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Lifting and ranking: a container's edges and constraints lifted to its
//! direct children, ranks by the chosen `Ranking` (longest path here, network
//! simplex in `simplex`), and dummies that carry long edges and hold reserved
//! labels through the ranks between their ends.

use super::simplex::network_simplex;
use super::{along_flow, Arranged, Constraint, EdgeLabel, Gap, Graph, LabelDummy, LabelSide, Layers, Lifted, Local, Ranking, LABEL_OFFSET};
use crate::core::geometry::{snap_up, Direction, GRID};

/// The position in `children` of the child of `container` that holds `node`.
fn local_child(graph: &Graph, container: usize, children: &[usize], node: usize) -> Option<usize> {
    let child = std::iter::successors(Some(node), |&x| graph.nodes[x].parent).find(|&x| graph.nodes[x].parent == Some(container))?;
    children.iter().position(|&x| x == child)
}

/// Maps a pair of nodes to distinct local children, if it is relevant here.
fn lift_pair(graph: &Graph, container: usize, children: &[usize], a: usize, b: usize) -> Option<(usize, usize)> {
    let u = local_child(graph, container, children, a)?;
    let v = local_child(graph, container, children, b)?;
    (u != v).then_some((u, v))
}

fn find_root(group: &[usize], item: usize) -> usize {
    std::iter::successors(Some(item), |&x| (group[x] != x).then_some(group[x])).last().unwrap()
}

pub(super) fn lift(graph: &Graph, container: usize, children: &[usize], arranged: &[Arranged]) -> Local {
    let count = children.len();
    let pair = |a: usize, b: usize| lift_pair(graph, container, children, a, b);
    // A note is one of these children itself, anchored to one of them or to
    // something inside one.
    let notes: Vec<(usize, usize)> = graph
        .constraints
        .iter()
        .filter_map(|c| match *c {
            Constraint::Note { note, anchor } => Some((children.iter().position(|&x| x == note)?, local_child(graph, container, children, anchor)?)),
            _ => None,
        })
        .filter(|&(n, a)| n != a)
        .collect();
    let is_note = |x: usize| notes.iter().any(|&(n, _)| n == x);
    let free = |(a, b): (usize, usize)| (!is_note(a) && !is_note(b)).then_some((a, b));
    let pair = |a: usize, b: usize| pair(a, b).and_then(free);
    let edges = graph
        .edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| pair(edge.from, edge.to).map(|(from, to)| Lifted { from, to, weight: edge.weight, gap: edge.gap, label: edge.label.map(|label| (label, index)) }))
        .collect();
    let select = |pick: fn(&Constraint) -> Option<(usize, usize)>| -> Vec<(usize, usize)> {
        graph.constraints.iter().filter_map(pick).filter_map(|(a, b)| pair(a, b)).collect()
    };
    let pivots = (0..count).map(|i| graph.constraints.iter().any(|c| *c == Constraint::Pivot(children[i]))).collect();
    let spread = (0..count).map(|i| graph.constraints.iter().any(|c| *c == Constraint::Spread(children[i]))).collect();
    let before = select(|c| match *c {
        Constraint::Before(a, b) => Some((a, b)),
        _ => None,
    });
    let across = select(|c| match *c {
        Constraint::AcrossBefore(a, b) => Some((a, b)),
        _ => None,
    });
    let adjacent = select(|c| match *c {
        Constraint::Adjacent { moved, anchor } | Constraint::Beside { moved, anchor } => Some((moved, anchor)),
        _ => None,
    });
    let beside = select(|c| match *c {
        Constraint::Beside { moved, anchor } => Some((moved, anchor)),
        _ => None,
    });
    // Union in constraint order: the representative depends on it.
    let same = select(|c| match *c {
        Constraint::AcrossBefore(a, b) | Constraint::SameRank(a, b) => Some((a, b)),
        _ => None,
    });
    let unions = same.into_iter().fold((0..count).collect::<Vec<usize>>(), |mut group, (u, v)| {
        let (ru, rv) = (find_root(&group, u), find_root(&group, v));
        group[ru] = rv;
        group
    });
    let group = (0..count).map(|i| find_root(&unions, i)).collect();
    let axes: Vec<(f64, f64)> = arranged.iter().map(|a| along_flow(graph.direction, a.size)).collect();
    Local {
        direction: graph.direction,
        count,
        main_size: axes.iter().map(|a| a.0).collect(),
        cross_size: axes.iter().map(|a| a.1).collect(),
        margin: children.iter().map(|&c| graph.nodes[c].margin).collect(),
        edges,
        before,
        across,
        adjacent,
        beside,
        group,
        notes,
        pivots,
        spread,
        spacing: graph.spacing,
    }
}

fn reaches(count: usize, edges: &[(usize, usize)], from: usize, to: usize) -> bool {
    let mut seen = vec![false; count];
    let mut stack = vec![from];
    while let Some(x) = stack.pop() {
        if x == to {
            return true;
        }
        if !std::mem::replace(&mut seen[x], true) {
            stack.extend(edges.iter().filter(|e| e.0 == x).map(|e| e.1));
        }
    }
    false
}

/// Each child's rank by the chosen method. Ranks start at 0, and children in
/// one same-rank group share a rank.
pub(super) fn ranks(ranking: Ranking, local: &Local) -> Vec<usize> {
    match ranking {
        Ranking::LongestPath => assign_ranks(local),
        Ranking::NetworkSimplex => network_simplex(local),
        #[cfg(test)]
        Ranking::Flat => vec![0; local.count],
    }
}

/// The pairs every ranking must respect, between same-rank group
/// representatives, each `(lower, higher, weight)` with a minimum length of one
/// rank. Constraints come first, then edges, and any pair that would close a
/// cycle with the pairs accepted before it is reversed, so every ranking agrees
/// on which edges point back. A repeated pair is listed once, where it was
/// first accepted, with the weights summed. A `Before` constraint weighs 0: it
/// orders its ends without pulling them together.
pub(super) fn oriented(local: &Local) -> Vec<(usize, usize, u32)> {
    let count = local.count;
    let group = &local.group;
    let hints = local.before.iter().map(|&(u, v)| (u, v, 0));
    let wanted = hints.chain(local.edges.iter().map(|e| (e.from, e.to, e.weight)));
    wanted.fold(Vec::new(), |mut accepted: Vec<(usize, usize, u32)>, (u, v, weight)| {
        let (a, b) = (group[u], group[v]);
        if a != b {
            let pairs: Vec<(usize, usize)> = accepted.iter().map(|&(x, y, _)| (x, y)).collect();
            let pair = if reaches(count, &pairs, b, a) { (b, a) } else { (a, b) };
            match accepted.iter_mut().find(|e| (e.0, e.1) == pair) {
                Some(known) => known.2 += weight,
                None => accepted.push((pair.0, pair.1, weight)),
            }
        }
        accepted
    })
}

/// Longest-path ranking over the `oriented` pairs, then sources pulled next to
/// their targets.
pub(super) fn assign_ranks(local: &Local) -> Vec<usize> {
    let count = local.count;
    let group = &local.group;
    let accepted: Vec<(usize, usize)> = oriented(local).into_iter().map(|(a, b, _)| (a, b)).collect();
    let mut rank = vec![0i32; count];
    for _ in 0..=count {
        for &(a, b) in &accepted {
            rank[b] = rank[b].max(rank[a] + 1);
        }
    }
    // Pull sources next to their targets to shorten edges.
    for _ in 0..count {
        for g in 0..count {
            if group[g] != g || accepted.iter().any(|e| e.1 == g) {
                continue;
            }
            if let Some(m) = accepted.iter().filter(|e| e.0 == g).map(|e| rank[e.1]).min() {
                rank[g] = m - 1;
            }
        }
    }
    let lowest = group.iter().map(|&g| rank[g]).min().unwrap_or(0);
    group.iter().map(|&g| (rank[g] - lowest) as usize).collect()
}

/// The dummy for `label` in a layout flowing along `direction`.
fn label_dummy(direction: Direction, label: EdgeLabel, item: usize, rank: usize, ends: (usize, usize), edge: usize) -> LabelDummy {
    let (main, across) = along_flow(direction, label.size);
    let room = snap_up(across + LABEL_OFFSET, GRID);
    let (before, after) = match label.side {
        LabelSide::Before => (room, 0.0),
        LabelSide::After => (0.0, room),
    };
    LabelDummy { item, rank, ends, edge, side: label.side, main, before, after }
}

/// The odd rank nearest the middle of a labelled edge's span.
fn label_rank(r0: usize, r1: usize) -> usize {
    r0 + 1 + 2 * ((r1 - r0 - 2) / 4)
}

pub(super) fn insert_dummies(local: &Local, item_rank: &[usize]) -> Layers {
    let rank_count = item_rank.iter().max().unwrap() + 1;
    let mut ranks: Vec<Vec<usize>> = vec![Vec::new(); rank_count];
    for (item, &rank) in item_rank.iter().enumerate() {
        ranks[rank].push(item);
    }
    let mut total = local.count;
    let mut links = Vec::new();
    let mut gap_need = vec![0.0f64; rank_count + 1];
    let mut stagger = vec![0.0f64; rank_count + 1];
    let mut labels = Vec::new();
    let mut chains = Vec::new();
    for edge in &local.edges {
        let (lo, hi) = if item_rank[edge.from] <= item_rank[edge.to] { (edge.from, edge.to) } else { (edge.to, edge.from) };
        let (r0, r1) = (item_rank[lo], item_rank[hi]);
        if r0 == r1 {
            continue;
        }
        gap_need[r0] = gap_need[r0].max(edge.gap.amount());
        if let Gap::Staggered(amount) = edge.gap {
            stagger[r0] += amount - local.spacing.stagger_overlap;
            gap_need[r0] = gap_need[r0].max(stagger[r0].min(local.spacing.stagger_limit));
        }
        // Repeating a link is what gives an edge its weight in both sweeps.
        let mut add = |a: usize, b: usize| links.extend(std::iter::repeat((a, b)).take(edge.weight as usize));
        let mut previous = lo;
        let first = total;
        for (r, rank) in ranks.iter_mut().enumerate().take(r1).skip(r0 + 1) {
            rank.push(total);
            if let Some((label, index)) = edge.label.filter(|_| r == label_rank(r0, r1)) {
                labels.push(label_dummy(local.direction, label, total, r, (lo, hi), index));
            }
            add(previous, total);
            previous = total;
            total += 1;
        }
        add(previous, hi);
        if total > first {
            chains.push((lo, (first..total).collect(), hi));
        }
    }
    Layers { ranks, links, total, gap_need, labels, chains }
}
