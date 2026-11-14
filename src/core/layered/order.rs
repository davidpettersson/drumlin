// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Ordering within ranks: weighted-median sweeps with transposition to reduce
//! crossings, order constraints applied after every sweep, and before-side
//! labels put first between parallel edges.

use super::{LabelSide, Layers, Local};

pub(super) fn positions(ranks: &[Vec<usize>], total: usize) -> Vec<usize> {
    ranks.iter().flat_map(|row| row.iter().enumerate()).fold(vec![0usize; total], |mut p, (i, &x)| {
        p[x] = i;
        p
    })
}

fn crossings(ranks: &[Vec<usize>], links: &[(usize, usize)], total: usize) -> usize {
    let p = positions(ranks, total);
    let rank_of = rank_index(ranks, total);
    links
        .iter()
        .enumerate()
        .map(|(i, &(a, b))| {
            links[i + 1..]
                .iter()
                .filter(|&&(e, f)| rank_of[a] == rank_of[e] && (p[a] as i64 - p[e] as i64) * (p[b] as i64 - p[f] as i64) < 0)
                .count()
        })
        .sum()
}

/// The weighted median of sorted neighbour positions (Gansner et al. 1993): with
/// an even count above two, the middle pair is weighted toward the side whose
/// positions are packed more tightly. `None` when there are no neighbours.
fn weighted_median(p: &[f64]) -> Option<f64> {
    let m = p.len() / 2;
    match p.len() {
        0 => None,
        n if n % 2 == 1 => Some(p[m]),
        2 => Some((p[0] + p[1]) / 2.0),
        n => {
            let (left, right) = (p[m - 1] - p[0], p[n - 1] - p[m]);
            if left + right == 0.0 {
                Some((p[m - 1] + p[m]) / 2.0)
            } else {
                Some((p[m - 1] * right + p[m] * left) / (left + right))
            }
        }
    }
}

/// One weighted-median pass over rank `r`, using `fixed` as the reference rank.
/// Items without neighbours there keep their place. With `flip`, items with
/// equal medians swap, which lets alternate sweeps escape ties.
fn reorder_rank(ranks: &[Vec<usize>], links: &[(usize, usize)], total: usize, r: usize, fixed: usize, flip: bool) -> Vec<usize> {
    let p = positions(ranks, total);
    let in_fixed: Vec<bool> = ranks[fixed].iter().fold(vec![false; total], |mut seen, &x| {
        seen[x] = true;
        seen
    });
    let medians: Vec<Option<f64>> = ranks[r]
        .iter()
        .map(|&x| {
            let mut neighbours: Vec<f64> = links
                .iter()
                .filter_map(|&(a, b)| match () {
                    _ if a == x && in_fixed[b] => Some(p[b] as f64),
                    _ if b == x && in_fixed[a] => Some(p[a] as f64),
                    _ => None,
                })
                .collect();
            neighbours.sort_by(f64::total_cmp);
            weighted_median(&neighbours)
        })
        .collect();
    // Items with a median are sorted among the places such items hold; the
    // others stay where they are.
    let mut movable: Vec<(f64, usize, usize)> = ranks[r].iter().enumerate().filter_map(|(i, &x)| medians[i].map(|m| (m, i, x))).collect();
    movable.sort_by(|a, b| a.0.total_cmp(&b.0).then(if flip { b.1.cmp(&a.1) } else { a.1.cmp(&b.1) }));
    let mut sorted = movable.into_iter().map(|t| t.2);
    ranks[r].iter().enumerate().map(|(i, &x)| if medians[i].is_some() { sorted.next().unwrap() } else { x }).collect()
}

/// Crossings among the links joining rank `r` to its neighbouring ranks.
fn rank_crossings(links: &[(usize, usize)], rank_of: &[usize], p: &[usize], r: usize) -> usize {
    let touching: Vec<(usize, usize)> = links
        .iter()
        .filter(|&&(a, b)| rank_of[a] == r || rank_of[b] == r)
        .map(|&(a, b)| if rank_of[a] < rank_of[b] { (a, b) } else { (b, a) })
        .collect();
    touching
        .iter()
        .enumerate()
        .map(|(i, &(a, b))| touching[i + 1..].iter().filter(|&&(e, f)| rank_of[a] == rank_of[e] && (p[a] as i64 - p[e] as i64) * (p[b] as i64 - p[f] as i64) < 0).count())
        .sum()
}

pub(super) fn rank_index(ranks: &[Vec<usize>], total: usize) -> Vec<usize> {
    ranks.iter().enumerate().flat_map(|(r, row)| row.iter().map(move |&x| (r, x))).fold(vec![0usize; total], |mut acc, (r, x)| {
        acc[x] = r;
        acc
    })
}

/// Swaps neighbours within each rank while that lowers crossings (Gansner et
/// al.'s `transpose`). With `flip`, equal counts swap too. Pairs held by an
/// order constraint never swap.
fn transpose(mut ranks: Vec<Vec<usize>>, links: &[(usize, usize)], total: usize, pinned: &dyn Fn(usize, usize) -> bool, flip: bool) -> Vec<Vec<usize>> {
    let rank_of = rank_index(&ranks, total);
    let mut p = positions(&ranks, total);
    // Equal-count swaps happen once per pair at most, so the loop ends.
    let mut flipped: Vec<(usize, usize)> = Vec::new();
    for _ in 0..total.max(4) {
        let mut improved = false;
        for r in 0..ranks.len() {
            for i in 1..ranks[r].len() {
                let (v, w) = (ranks[r][i - 1], ranks[r][i]);
                if pinned(v, w) {
                    continue;
                }
                let before = rank_crossings(links, &rank_of, &p, r);
                p.swap(v, w);
                let after = rank_crossings(links, &rank_of, &p, r);
                let tie = flip && after == before && before > 0 && !flipped.contains(&(v.min(w), v.max(w)));
                if after < before || tie {
                    ranks[r].swap(i - 1, i);
                    improved |= after < before;
                    if tie {
                        flipped.push((v.min(w), v.max(w)));
                    }
                } else {
                    p.swap(v, w);
                }
            }
        }
        if !improved {
            break;
        }
    }
    ranks
}

/// An initial order from a depth-first walk along the links, so a tree starts
/// with no crossings. Roots are taken in input order.
fn depth_first_order(layers: &Layers) -> Vec<Vec<usize>> {
    let total = layers.total;
    let rank_of = rank_index(&layers.ranks, total);
    let mut seen = vec![false; total];
    let mut visit = Vec::with_capacity(total);
    let starts: Vec<usize> = layers.ranks.iter().flatten().copied().collect();
    for &start in &starts {
        let mut stack = vec![start];
        while let Some(x) = stack.pop() {
            if std::mem::replace(&mut seen[x], true) {
                continue;
            }
            visit.push(x);
            let mut next: Vec<usize> = layers.links.iter().filter(|&&(a, b)| a == x && rank_of[b] > rank_of[a] && !seen[b]).map(|&(_, b)| b).collect();
            next.dedup();
            stack.extend(next.into_iter().rev());
        }
    }
    let order = positions(&[visit], total);
    layers
        .ranks
        .iter()
        .map(|row| {
            let mut row = row.clone();
            row.sort_by_key(|&x| order[x]);
            row
        })
        .collect()
}

/// Weighted-median sweeps with transposition, from two initial orders, keeping
/// the best ordering seen. Order constraints are applied after every sweep, so
/// the kept ordering already satisfies them and is scored as it will be drawn.
pub(super) fn reduce_crossings(layers: &Layers, local: &Local) -> Vec<Vec<usize>> {
    let (links, total) = (&layers.links, layers.total);
    let rank_count = layers.ranks.len();
    let pinned = |a: usize, b: usize| {
        let held = |&(x, y): &(usize, usize)| (x == a && y == b) || (x == b && y == a);
        local.adjacent.iter().any(held) || local.across.iter().any(held)
    };
    let constrain = |ranks: Vec<Vec<usize>>| apply_order_constraints(ranks, local, total);
    let starts = [layers.ranks.clone(), depth_first_order(layers)];
    let mut best = constrain(layers.ranks.clone());
    let mut best_score = crossings(&best, links, total);
    for start in starts {
        let mut ranks = constrain(start);
        for sweep in 0..24 {
            if best_score == 0 {
                return best;
            }
            let flip = sweep % 4 >= 2;
            let visits: Vec<(usize, usize)> = match sweep % 2 {
                0 => (1..rank_count).map(|r| (r, r - 1)).collect(),
                _ => (0..rank_count.saturating_sub(1)).rev().map(|r| (r, r + 1)).collect(),
            };
            for (r, fixed) in visits {
                ranks[r] = reorder_rank(&ranks, links, total, r, fixed, flip);
            }
            ranks = constrain(transpose(constrain(ranks), links, total, &pinned, flip));
            let score = crossings(&ranks, links, total);
            if score < best_score {
                best_score = score;
                best = ranks.clone();
            }
        }
    }
    best
}

fn move_next_to(ranks: Vec<Vec<usize>>, moved: usize, anchor: usize, slot: Place) -> Vec<Vec<usize>> {
    ranks
        .into_iter()
        .map(|mut row| {
            if row.contains(&moved) && row.contains(&anchor) {
                row.retain(|&x| x != moved);
                let at = row.iter().position(|&x| x == anchor).unwrap();
                row.insert(match slot { Place::Before => at, Place::After => at + 1 }, moved);
            }
            row
        })
        .collect()
}

#[derive(Clone, Copy)]
enum Place {
    Before,
    After,
}

/// Explicit ordering overrides crossing reduction.
fn apply_order_constraints(order: Vec<Vec<usize>>, local: &Local, total: usize) -> Vec<Vec<usize>> {
    let order = local.adjacent.iter().fold(order, |order, &(moved, anchor)| move_next_to(order, moved, anchor, Place::After));
    local.across.iter().fold(order, |order, &(a, b)| {
        let p = positions(&order, total);
        if p[a] > p[b] {
            move_next_to(order, a, b, Place::Before)
        } else {
            order
        }
    })
}

/// Parallel edges between the same two items put a before-side label first, so
/// the two lines run close together with their labels on the outside.
pub(super) fn order_label_sides(order: Vec<Vec<usize>>, layers: &Layers) -> Vec<Vec<usize>> {
    let swaps = layers.labels.iter().flat_map(|a| layers.labels.iter().map(move |b| (a, b))).filter(|(a, b)| {
        a.rank == b.rank && a.side == LabelSide::Before && b.side == LabelSide::After && a.ends == b.ends
    });
    swaps.fold(order, |mut order, (a, b)| {
        let row = &mut order[a.rank];
        let (i, j) = (row.iter().position(|&x| x == a.item).unwrap(), row.iter().position(|&x| x == b.item).unwrap());
        if i > j {
            row.swap(i, j);
        }
        order
    })
}
