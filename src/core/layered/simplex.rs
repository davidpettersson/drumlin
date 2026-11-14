// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Network simplex ranking (Gansner, Koutsofios, North and Vo, "A Technique for
//! Drawing Directed Graphs", 1993, section 2): ranks that minimise the sum over
//! edges of weight times length, where each edge spans at least one rank.
//!
//! The problem is the one `rank::assign_ranks` solves heuristically, on the same
//! `rank::oriented` pairs: same-rank groups are one node, constraints order
//! their ends with weight 0, and cycles are broken the same way, so both
//! rankings agree on which edges point back. Longest path is the feasible
//! start. Each connected part is solved on its own and starts at rank 0, as
//! longest path leaves it. Where longest path is already optimal its ranks are
//! kept, so the two engines differ only where this one shortens edges.

use super::rank::{assign_ranks, oriented};
use super::Local;

/// One pair to rank: `(lower, higher, weight)`, in a part's own numbering.
type Pair = (usize, usize, i64);

/// Ranks of `local`'s children by network simplex. Ranks start at 0, and
/// children in one same-rank group share a rank.
pub(super) fn network_simplex(local: &Local) -> Vec<usize> {
    let start = assign_ranks(local);
    let pairs = oriented(local);
    let solved = parts(local, &pairs).into_iter().fold(start.clone(), |mut rank, part| {
        let index = |child: usize| part.binary_search(&child).unwrap();
        let own: Vec<Pair> = pairs.iter().filter(|p| part.binary_search(&p.0).is_ok()).map(|&(a, b, w)| (index(a), index(b), i64::from(w))).collect();
        let begun: Vec<i64> = part.iter().map(|&child| start[child] as i64).collect();
        let ranked = solve(&begun, &own, pivot_cap(own.len()));
        let lowest = ranked.iter().copied().min().unwrap_or(0);
        for (&child, r) in part.iter().zip(ranked) {
            rank[child] = (r - lowest) as usize;
        }
        rank
    });
    let ranked = if cost(&pairs, &solved) < cost(&pairs, &start) { solved } else { start };
    local.group.iter().map(|&g| ranked[g]).collect()
}

/// At most this many pivots before the search stops with the ranking it has.
/// Network simplex can cycle through degenerate pivots, and this bound is far
/// above the handful of pivots the layouts here take.
fn pivot_cap(pairs: usize) -> usize {
    64 + 16 * pairs
}

/// The weighted length of the pairs under `rank` (indexed by child).
fn cost(pairs: &[(usize, usize, u32)], rank: &[usize]) -> i64 {
    pairs.iter().map(|&(a, b, w)| i64::from(w) * (rank[b] as i64 - rank[a] as i64)).sum()
}

/// The connected parts of the group representatives under `pairs`, each sorted,
/// in order of their first child.
fn parts(local: &Local, pairs: &[(usize, usize, u32)]) -> Vec<Vec<usize>> {
    let root = |link: &[usize], x: usize| std::iter::successors(Some(x), |&y| (link[y] != y).then_some(link[y])).last().unwrap();
    let link = pairs.iter().fold((0..local.count).collect::<Vec<usize>>(), |mut link, &(a, b, _)| {
        let (ra, rb) = (root(&link, a), root(&link, b));
        link[ra.max(rb)] = ra.min(rb);
        link
    });
    let reps = (0..local.count).filter(|&x| local.group[x] == x);
    reps.fold(Vec::new(), |mut parts: Vec<Vec<usize>>, x| {
        let r = root(&link, x);
        match parts.iter_mut().find(|part| root(&link, part[0]) == r) {
            Some(part) => part.push(x),
            None => parts.push(vec![x]),
        }
        parts
    })
}

fn slack(rank: &[i64], &(a, b, _): &Pair) -> i64 {
    rank[b] - rank[a] - 1
}

/// Network simplex on one connected part, from the feasible ranking `start`.
/// Returns optimal ranks, or, after `cap` pivots, the ranking reached then:
/// every pivot keeps the ranking feasible and never raises its cost, so that is
/// the best one found.
fn solve(start: &[i64], pairs: &[Pair], cap: usize) -> Vec<i64> {
    let (mut rank, mut tree) = feasible_tree(start, pairs);
    let mut from = 0;
    for _ in 0..cap {
        // Leave: the next tree pair with a negative cut value, searching on
        // from the last one, as the paper does.
        let cuts: Vec<Option<i64>> = (0..pairs.len()).map(|e| tree[e].then(|| cut_value(pairs, &tree, e, rank.len()))).collect();
        let Some(leave) = (0..pairs.len()).map(|k| (from + k) % pairs.len()).find(|&e| cuts[e].is_some_and(|c| c < 0)) else { break };
        from = leave + 1;
        // Enter: the pair of least slack from the leaving pair's head side to
        // its tail side; moving the tail side by that slack makes it tight.
        let tail_side = side(pairs, &tree, leave, rank.len());
        let crossing = (0..pairs.len()).filter(|&f| !tree[f] && !tail_side[pairs[f].0] && tail_side[pairs[f].1]);
        let Some(enter) = crossing.min_by_key(|&f| (slack(&rank, &pairs[f]), f)) else { break };
        let delta = slack(&rank, &pairs[enter]);
        for (r, _) in rank.iter_mut().zip(&tail_side).filter(|(_, &on)| on) {
            *r -= delta;
        }
        tree[leave] = false;
        tree[enter] = true;
    }
    rank
}

/// A spanning tree of tight pairs (each exactly one rank long), and the
/// ranking that makes them tight: grow a tree from the first node over tight
/// pairs; while it does not span, shift it along the pair of least slack that
/// leaves it, which keeps every pair feasible and makes that one tight.
fn feasible_tree(start: &[i64], pairs: &[Pair]) -> (Vec<i64>, Vec<bool>) {
    let count = start.len();
    let mut rank = start.to_vec();
    let mut tree = vec![false; pairs.len()];
    let mut inside = vec![false; count];
    inside[0] = true;
    loop {
        while let Some(e) = (0..pairs.len()).find(|&e| !tree[e] && inside[pairs[e].0] != inside[pairs[e].1] && slack(&rank, &pairs[e]) == 0) {
            tree[e] = true;
            inside[pairs[e].0] = true;
            inside[pairs[e].1] = true;
        }
        if inside.iter().all(|&x| x) {
            return (rank, tree);
        }
        let leaving = (0..pairs.len()).filter(|&e| inside[pairs[e].0] != inside[pairs[e].1]);
        let Some(e) = leaving.min_by_key(|&e| (slack(&rank, &pairs[e]), e)) else { return (rank, tree) };
        let delta = if inside[pairs[e].0] { slack(&rank, &pairs[e]) } else { -slack(&rank, &pairs[e]) };
        for (r, _) in rank.iter_mut().zip(&inside).filter(|(_, &on)| on) {
            *r += delta;
        }
    }
}

/// The nodes on the tail side of tree pair `cut` once it is removed.
fn side(pairs: &[Pair], tree: &[bool], cut: usize, count: usize) -> Vec<bool> {
    let mut on = vec![false; count];
    let mut stack = vec![pairs[cut].0];
    while let Some(x) = stack.pop() {
        if !std::mem::replace(&mut on[x], true) {
            let next = (0..pairs.len()).filter(|&e| tree[e] && e != cut).filter_map(|e| match pairs[e] {
                (a, b, _) if a == x => Some(b),
                (a, b, _) if b == x => Some(a),
                _ => None,
            });
            stack.extend(next);
        }
    }
    on
}

/// The cut value of tree pair `cut`: the weight of the pairs from its tail side
/// to its head side, less the weight of those back. Negative means that
/// lengthening `cut` shortens the rest by more.
fn cut_value(pairs: &[Pair], tree: &[bool], cut: usize, count: usize) -> i64 {
    let tail_side = side(pairs, tree, cut, count);
    pairs
        .iter()
        .map(|&(a, b, w)| match (tail_side[a], tail_side[b]) {
            (true, false) => w,
            (false, true) => -w,
            _ => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::super::{Direction, Lifted, Gap, Spacing};
    use super::*;

    /// A container of `count` children with weighted edges, `Before` hints
    /// and same-rank pairs.
    fn local(count: usize, edges: &[(usize, usize, u32)], before: &[(usize, usize)], same: &[(usize, usize)]) -> Local {
        let root = |group: &[usize], x: usize| std::iter::successors(Some(x), |&y| (group[y] != y).then_some(group[y])).last().unwrap();
        let unions = same.iter().fold((0..count).collect::<Vec<usize>>(), |mut group, &(u, v)| {
            let (ru, rv) = (root(&group, u), root(&group, v));
            group[ru] = rv;
            group
        });
        Local {
            direction: Direction::Down,
            count,
            main_size: vec![48.0; count],
            cross_size: vec![96.0; count],
            margin: vec![Default::default(); count],
            edges: edges.iter().map(|&(from, to, weight)| Lifted { from, to, weight, gap: Gap::Fixed(0.0), label: None }).collect(),
            before: before.to_vec(),
            across: Vec::new(),
            adjacent: Vec::new(),
            beside: Vec::new(),
            group: (0..count).map(|x| root(&unions, x)).collect(),
            notes: Vec::new(),
            pivots: vec![false; count],
            spread: vec![false; count],
            spacing: Spacing::default(),
        }
    }

    fn total(local: &Local, rank: &[usize]) -> i64 {
        cost(&oriented(local), rank)
    }

    /// Every pair spans at least one rank, groups share a rank, and ranks
    /// start at 0 in every connected part.
    fn assert_feasible(local: &Local, rank: &[usize]) {
        for (a, b, _) in oriented(local) {
            assert!(rank[b] > rank[a], "{a} -> {b} spans {:?}", rank);
        }
        for x in 0..local.count {
            assert_eq!(rank[x], rank[local.group[x]], "{x} shares its group's rank");
        }
        for part in parts(local, &oriented(local)) {
            assert_eq!(part.iter().map(|&x| rank[x]).min(), Some(0), "part {part:?} starts at 0: {rank:?}");
        }
    }

    /// The least cost over every ranking with ranks below the child count.
    fn brute_force(local: &Local) -> i64 {
        let pairs = oriented(local);
        let reps: Vec<usize> = (0..local.count).filter(|&x| local.group[x] == x).collect();
        let n = local.count;
        let mut best = i64::MAX;
        let mut choice = vec![0usize; reps.len()];
        loop {
            let mut rank = vec![0usize; n];
            for (&r, &c) in reps.iter().zip(&choice) {
                rank[r] = c;
            }
            if pairs.iter().all(|&(a, b, _)| rank[b] > rank[a]) {
                best = best.min(cost(&pairs, &rank));
            }
            let Some(k) = (0..choice.len()).find(|&k| choice[k] + 1 < n) else { return best };
            choice[k] += 1;
            choice[..k].iter_mut().for_each(|c| *c = 0);
        }
    }

    fn check(local: &Local) -> Vec<usize> {
        let rank = network_simplex(local);
        assert_feasible(local, &rank);
        assert_eq!(total(local, &rank), brute_force(local), "optimal for {:?}", local.edges.iter().map(|e| (e.from, e.to, e.weight)).collect::<Vec<_>>());
        assert!(total(local, &rank) <= total(local, &assign_ranks(local)));
        rank
    }

    #[test]
    fn a_node_moves_down_next_to_its_children() {
        // 0 heads a long chain 0 -> 1 -> 2 -> 3 -> {5, 6, 7}; 4 hangs from 0 and
        // feeds 5, 6 and 7. Longest path puts 4 just below 0, three ranks
        // above its children; the optimum puts it right above them.
        let l = local(8, &[(0, 1, 1), (1, 2, 1), (2, 3, 1), (3, 5, 1), (3, 6, 1), (3, 7, 1), (0, 4, 1), (4, 5, 1), (4, 6, 1), (4, 7, 1)], &[], &[]);
        assert_eq!(assign_ranks(&l)[4], 1);
        let rank = network_simplex(&l);
        assert_eq!(rank[4], 3);
        assert_eq!((total(&l, &rank), total(&l, &assign_ranks(&l))), (12, 16));
        assert_feasible(&l, &rank);
    }

    #[test]
    fn small_graphs_are_ranked_optimally() {
        check(&local(1, &[], &[], &[]));
        check(&local(3, &[], &[], &[]));
        check(&local(3, &[(0, 1, 1), (1, 2, 1), (0, 2, 1)], &[], &[]));
        check(&local(4, &[(0, 1, 1), (0, 2, 1), (1, 3, 1), (2, 3, 1)], &[], &[]));
        check(&local(5, &[(0, 1, 1), (1, 2, 1), (2, 3, 1), (0, 4, 3), (4, 3, 1)], &[], &[]));
        check(&local(5, &[(0, 4, 1), (1, 4, 1), (2, 4, 1), (3, 4, 5), (0, 1, 1)], &[], &[]));
    }

    #[test]
    fn pseudo_random_graphs_are_ranked_optimally() {
        // A tiny linear congruential generator: deterministic, no dependency.
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = |bound: u64| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % bound
        };
        let mut improved = 0;
        for _ in 0..300 {
            let n = 2 + next(5) as usize;
            // Forward edges, some of them turned back so that cycles occur.
            let mut edges = Vec::new();
            for (a, b) in (0..n).flat_map(|a| (a + 1..n).map(move |b| (a, b))) {
                if next(100) < 45 {
                    let weight = 1 + next(3) as u32;
                    edges.push(if next(100) < 15 { (b, a, weight) } else { (a, b, weight) });
                }
            }
            let before: Vec<(usize, usize)> = (0..next(2)).map(|_| (next(n as u64) as usize, next(n as u64) as usize)).filter(|(a, b)| a != b).collect();
            let same: Vec<(usize, usize)> = (0..next(2)).map(|_| (next(n as u64) as usize, next(n as u64) as usize)).filter(|(a, b)| a != b).collect();
            let l = local(n, &edges, &before, &same);
            improved += usize::from(total(&l, &check(&l)) < total(&l, &assign_ranks(&l)));
        }
        assert!(improved >= 5, "only {improved} graphs improve on longest path");
    }

    #[test]
    fn larger_pseudo_random_graphs_are_feasible_and_no_longer() {
        // Too large for brute force: feasible, never longer than longest
        // path, and often shorter. Optimality holds when no cut value is
        // negative, which is where `solve` stops before its cap.
        let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = |bound: u64| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % bound
        };
        let mut improved = 0;
        for _ in 0..200 {
            let n = 6 + next(15) as usize;
            let mut edges = Vec::new();
            for (a, b) in (0..n).flat_map(|a| (a + 1..n).map(move |b| (a, b))) {
                if next(100) < 20 {
                    let weight = if next(100) < 20 { 4 } else { 1 };
                    edges.push(if next(100) < 10 { (b, a, weight) } else { (a, b, weight) });
                }
            }
            let l = local(n, &edges, &[], &[]);
            let rank = network_simplex(&l);
            assert_feasible(&l, &rank);
            assert!(total(&l, &rank) <= total(&l, &assign_ranks(&l)));
            improved += usize::from(total(&l, &rank) < total(&l, &assign_ranks(&l)));
        }
        assert!(improved >= 40, "only {improved} graphs improve on longest path");
    }

    #[test]
    fn cycles_are_broken_as_longest_path_breaks_them() {
        let l = local(4, &[(0, 1, 1), (1, 2, 1), (2, 0, 1), (2, 3, 1), (3, 1, 2)], &[], &[]);
        assert_eq!(oriented(&l), [(0, 1, 1), (1, 2, 1), (0, 2, 1), (2, 3, 1), (1, 3, 2)]);
        check(&l);
    }

    #[test]
    fn hints_order_their_ends_without_pulling() {
        // 0 before 2 although the edge says 2 -> 0; 1 before 3 with no edge.
        let l = local(4, &[(2, 0, 1), (0, 1, 1)], &[(0, 2), (1, 3)], &[]);
        let rank = check(&l);
        assert!(rank[0] < rank[2] && rank[1] < rank[3]);
        assert_eq!(oriented(&l)[1], (1, 3, 0), "a hint weighs nothing");
    }

    #[test]
    fn a_same_rank_group_is_one_node() {
        // 1 and 4 share a rank and are ranked as one node.
        let l = local(6, &[(0, 1, 1), (1, 2, 1), (2, 3, 1), (3, 5, 1), (0, 4, 1), (4, 5, 3)], &[], &[(1, 4)]);
        let rank = check(&l);
        assert_eq!(rank[1], rank[4]);
    }

    #[test]
    fn disconnected_parts_each_start_at_the_top() {
        // Two parts and a lone node; the second part's middle node moves down.
        let l = local(7, &[(0, 1, 1), (2, 3, 1), (3, 4, 1), (4, 5, 1), (2, 6, 1), (6, 5, 2)], &[], &[]);
        let rank = check(&l);
        assert_eq!((rank[0], rank[1], rank[2]), (0, 1, 0));
        assert_eq!(rank[6], 2);
        let lone = local(3, &[(0, 1, 1)], &[], &[]);
        assert_eq!(check(&lone), [0, 1, 0]);
    }

    #[test]
    fn where_longest_path_is_optimal_its_ranks_are_kept() {
        let l = local(5, &[(0, 1, 1), (0, 2, 1), (1, 3, 1), (2, 3, 1), (3, 4, 1)], &[], &[]);
        assert_eq!(network_simplex(&l), assign_ranks(&l));
    }

    #[test]
    fn the_pivot_cap_stops_with_a_feasible_ranking() {
        let pairs: Vec<Pair> = vec![(0, 1, 1), (1, 2, 1), (2, 3, 1), (3, 5, 1), (0, 4, 1), (4, 5, 3)];
        let start = [0, 1, 2, 3, 1, 4];
        for cap in [0, 1, 1000] {
            let rank = solve(&start, &pairs, cap);
            assert!(pairs.iter().all(|p| slack(&rank, p) >= 0), "cap {cap}: {rank:?}");
        }
        let optimal = solve(&start, &pairs, 1000);
        assert_eq!(optimal[4] - optimal[0], 3);
    }
}
