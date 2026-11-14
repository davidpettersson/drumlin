// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Placement across the flow: ranks pulled toward their neighbours at minimum
//! separations, chains held in columns, long edges straightened, and pendant
//! items lined up with what they hang from.

use super::order::{positions, rank_index};
use super::{Layers, Local};
use crate::core::geometry::snap;

fn separation(local: &Local, layers: &Layers, a: usize, b: usize) -> f64 {
    let size = |x: usize| if x < local.count { local.cross_size[x] } else { 0.0 };
    let gap = if a < local.count && b < local.count { local.spacing.item_gap } else { local.spacing.dummy_gap };
    match (layers.label_at(a), layers.label_at(b)) {
        (None, None) => (size(a) + size(b)) / 2.0 + gap,
        (first, second) => first.map_or(size(a) / 2.0, |label| label.after) + second.map_or(size(b) / 2.0, |label| label.before) + gap,
    }
}

fn neighbour_positions(links: &[(usize, usize)], x: usize, up: bool, down: bool, cross: &[f64]) -> Vec<f64> {
    let mut found: Vec<f64> = links
        .iter()
        .flat_map(|&(a, b)| [(a == x && down).then_some(b), (b == x && up).then_some(a)])
        .flatten()
        .map(|other| cross[other])
        .collect();
    found.sort_by(f64::total_cmp);
    found
}

fn median(sorted: &[f64]) -> f64 {
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[middle]
    } else {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    }
}

/// Offsets of a row's items when packed at their minimum separations.
fn row_offsets(local: &Local, layers: &Layers, row: &[usize]) -> Vec<f64> {
    row.iter()
        .enumerate()
        .scan(0.0, |offset, (i, &x)| {
            if i > 0 {
                *offset += separation(local, layers, row[i - 1], x);
            }
            Some(*offset)
        })
        .collect()
}

/// Cross-axis placement: alternately pull each rank toward its neighbours,
/// merging items into blocks so the order and minimum separations survive.
pub(super) fn place_across(local: &Local, layers: &Layers, order: &[Vec<usize>], partners: &Partners) -> Vec<f64> {
    let rank_count = order.len();
    let mut cross = vec![0.0f64; layers.total];
    for row in order {
        for (&x, offset) in row.iter().zip(row_offsets(local, layers, row)) {
            cross[x] = offset;
        }
    }
    for pass in 0..24 {
        let forward = pass % 2 == 0;
        let both = pass >= 16;
        let visits: Vec<usize> = if forward { (0..rank_count).collect() } else { (0..rank_count).rev().collect() };
        for r in visits {
            let row = &order[r];
            let offsets = row_offsets(local, layers, row);
            let mut blocks: Vec<(f64, f64, usize)> = Vec::new(); // (mean, weight, members)
            for (&x, &offset) in row.iter().zip(&offsets) {
                let v = neighbour_positions(&layers.links, x, forward || both, !forward || both, &cross);
                // An item in a column follows its column partners, ahead of
                // anything else it connects to.
                let column: Vec<f64> = partners.of(x, forward || both, !forward || both).map(|y| cross[y]).collect();
                let target = match (column.is_empty(), v.is_empty()) {
                    (false, _) => column.iter().sum::<f64>() / column.len() as f64,
                    (true, true) => cross[x],
                    (true, false) => median(&v),
                };
                let weight = match (x >= local.count, column.is_empty(), v.is_empty()) {
                    (_, false, _) => COLUMN_WEIGHT,
                    (true, true, _) => 8.0,
                    (false, true, true) => 0.1,
                    (false, true, false) => 1.0,
                };
                let mut block = (target - offset, weight, 1usize);
                while let Some(last) = blocks.last().copied() {
                    if last.0 < block.0 {
                        break;
                    }
                    blocks.pop();
                    let w = last.1 + block.1;
                    block = ((last.0 * last.1 + block.0 * block.1) / w, w, last.2 + block.2);
                }
                blocks.push(block);
            }
            let block_means = blocks.iter().flat_map(|&(mean, _, members)| std::iter::repeat(mean).take(members));
            for ((&x, mean), offset) in row.iter().zip(block_means).zip(&offsets) {
                cross[x] = mean + offset;
            }
        }
    }
    for row in order {
        for (i, &x) in row.iter().enumerate() {
            let snapped = snap(cross[x]);
            let tight = i > 0 && local.beside.iter().any(|&(moved, anchor)| moved == x && anchor == row[i - 1]);
            cross[x] = match (i > 0, tight) {
                (true, true) => cross[row[i - 1]] + separation(local, layers, row[i - 1], x),
                (true, false) => snapped.max(cross[row[i - 1]] + separation(local, layers, row[i - 1], x)),
                (false, _) => snapped,
            };
        }
    }
    cross
}

/// The pull of a column partner in `place_across`: stronger than the dummies of
/// a long edge, so a column stays straight when a side branch pushes it.
const COLUMN_WEIGHT: f64 = 100.0;

/// Column partners: for each item, the item just before it in its column and
/// the item just after it, along the flow. Items in one column sit centred on
/// one line, so the edges between them run straight (Drawing principles, Flow:
/// keep a column).
pub(super) struct Partners {
    above: Vec<Option<usize>>,
    below: Vec<Option<usize>>,
    /// For each dummy of an edge between two partners, the upper partner: the
    /// edge's line runs down the column too.
    lane: Vec<Option<usize>>,
}

impl Partners {
    fn of(&self, x: usize, up: bool, down: bool) -> impl Iterator<Item = usize> + '_ {
        let get = |list: &Vec<Option<usize>>| list.get(x).copied().flatten();
        [up.then(|| get(&self.above)).flatten(), down.then(|| get(&self.below)).flatten(), get(&self.lane)].into_iter().flatten()
    }
}

/// Chooses the columns. Each edge between neighbouring ranks of items may join
/// its two ends in a column. The edge on the longest path through the graph
/// wins, so the main chain keeps its column and side branches move out of it.
/// An item with several edges onward joins one of them only when that edge's
/// path is strictly the longest, or when the item is a decision (`Pivot`),
/// which then takes the first declared; a parent over equal branches stays
/// centred over them instead. An item fed by several joins the one on the
/// longest path, the first declared on a tie. Two columns between the same ranks keep the
/// order of their ends. Notes and boxes held beside another item take no part,
/// and a bar (`Spread`) only ends the column of a decision above it.
pub(super) fn columns(local: &Local, layers: &Layers, item_rank: &[usize], stretch: usize, order: &[Vec<usize>]) -> Partners {
    let count = local.count;
    if !local.spacing.columns {
        return Partners { above: vec![None; count], below: vec![None; count], lane: vec![None; layers.total] };
    }
    let held = |x: usize| local.notes.iter().any(|&(n, _)| n == x) || local.beside.iter().any(|&(m, _)| m == x);
    let links: Vec<(usize, usize)> = local
        .edges
        .iter()
        .filter(|e| !held(e.from) && !held(e.to) && item_rank[e.from] != item_rank[e.to])
        .map(|e| if item_rank[e.from] < item_rank[e.to] { (e.from, e.to) } else { (e.to, e.from) })
        .collect();
    let mut by_rank: Vec<usize> = (0..count).collect();
    by_rank.sort_by_key(|&x| item_rank[x]);
    // Items on the longest path down from and up to each item, itself included.
    let down = by_rank.iter().rev().fold(vec![1usize; count], |mut down, &x| {
        down[x] = 1 + links.iter().filter(|l| l.0 == x).map(|l| down[l.1]).max().unwrap_or(0);
        down
    });
    let up = by_rank.iter().fold(vec![1usize; count], |mut up, &x| {
        up[x] = 1 + links.iter().filter(|l| l.1 == x).map(|l| up[l.0]).max().unwrap_or(0);
        up
    });
    let mut near: Vec<(usize, usize)> = links.iter().copied().filter(|&(a, b)| item_rank[b] - item_rank[a] == stretch).collect();
    let mut seen = Vec::new();
    near.retain(|pair| if seen.contains(pair) { false } else { seen.push(*pair); true });
    let score = |&(a, b): &(usize, usize)| up[a] + down[b];
    // A bar spans its branches, so it starts no column, and ends one only
    // under a decision, whose main branch it is.
    // A join, a bar with one edge onward, passes the flow on into a column.
    let onward = |a: usize| near.iter().filter(|o| o.0 == a).count();
    let barred = |&(a, b): &(usize, usize)| (local.spread[a] && onward(a) > 1) || (local.spread[b] && !local.pivots[a]);
    let eligible = |pair: &(usize, usize)| {
        let others = near.iter().filter(|o| o.0 == pair.0 && o.1 != pair.1);
        local.pivots[pair.0] || others.clone().count() == 0 || others.map(score).all(|s| s < score(pair))
    };
    let mut wanted: Vec<(usize, (usize, usize))> = near.iter().enumerate().filter(|(_, p)| !barred(p) && eligible(p)).map(|(i, &p)| (i, p)).collect();
    wanted.sort_by(|x, y| score(&y.1).cmp(&score(&x.1)).then(x.0.cmp(&y.0)));
    let p = positions(order, local.count.max(order.iter().flatten().copied().max().map_or(0, |m| m + 1)));
    let (mut above, mut below) = (vec![None; count], vec![None; count]);
    let mut taken: Vec<(usize, usize)> = Vec::new();
    for (_, (a, b)) in wanted {
        let clash = taken.iter().any(|&(c, d)| item_rank[c] == item_rank[a] && item_rank[d] == item_rank[b] && (p[a] < p[c]) != (p[b] < p[d]));
        if below[a].is_none() && above[b].is_none() && !clash {
            below[a] = Some(b);
            above[b] = Some(a);
            taken.push((a, b));
        }
    }
    let lane = layers.chains.iter().filter(|(lo, _, hi)| below[*lo] == Some(*hi)).fold(vec![None; layers.total], |mut lane, (lo, dummies, _)| {
        dummies.iter().for_each(|&d| lane[d] = Some(*lo));
        lane
    });
    Partners { above, below, lane }
}

/// Puts each column on one line. Placement leaves partners close, but rows
/// may hold them a little apart, and snapping may part them by a step. Each
/// column moves as a whole to the line of one of its members, the widest
/// first, where every member's row has room; otherwise each partner moves onto
/// the line of the one before it where its row allows.
pub(super) fn hold_columns(local: &Local, layers: &Layers, order: &[Vec<usize>], partners: &Partners, cross: Vec<f64>) -> Vec<f64> {
    let p = positions(order, layers.total);
    let rank_of = rank_index(order, layers.total);
    let tops = (0..local.count).filter(|&x| partners.above[x].is_none() && partners.below[x].is_some());
    let chains: Vec<Vec<usize>> = tops.map(|top| std::iter::successors(Some(top), |&x| partners.below[x]).take(local.count).collect()).collect();
    let fits = |cross: &[f64], x: usize, at: f64| {
        let (lo, hi) = row_room(local, layers, order, &p, &rank_of, cross, x);
        lo <= at && at <= hi
    };
    chains.iter().fold(cross, |mut cross, chain| {
        let mut lines: Vec<(f64, f64)> = chain.iter().map(|&x| (local.cross_size[x], cross[x])).collect();
        lines.sort_by(|a, b| b.0.total_cmp(&a.0));
        match lines.iter().map(|l| l.1).find(|&at| chain.iter().all(|&x| fits(&cross, x, at))) {
            Some(at) => chain.iter().for_each(|&x| cross[x] = at),
            None => {
                for w in chain.windows(2) {
                    if cross[w[1]] != cross[w[0]] && fits(&cross, w[1], cross[w[0]]) {
                        cross[w[1]] = cross[w[0]];
                    }
                }
            }
        }
        cross
    })
}

/// Where a line may meet an item along the cross axis: its extent less a
/// margin, so that the line meets a flat side and leaves room beside it.
fn attach_range(local: &Local, cross: &[f64], item: usize) -> (f64, f64) {
    let half = local.cross_size[item] / 2.0;
    let reach = half - ATTACH_INSET.min(half);
    (cross[item] - reach, cross[item] + reach)
}

const ATTACH_INSET: f64 = 16.0;

/// Lines up the dummies of each long edge after placement, so the edge runs
/// straight: at a position both ends can take when there is one, otherwise one
/// that either end can take, so the line bends near the other end only. Edges
/// joining the same two items move together as a block, side by side, so that
/// they straighten at the same end. Each dummy stays between its neighbours in
/// its rank, and items do not move. A block with no common position keeps the
/// placement's positions.
pub(super) fn straighten_chains(local: &Local, layers: &Layers, order: &[Vec<usize>], cross: Vec<f64>) -> Vec<f64> {
    let p = positions(order, layers.total);
    let rank_of = rank_index(order, layers.total);
    let key = |&(lo, _, hi): &(usize, Vec<usize>, usize)| (lo.min(hi), lo.max(hi));
    let mut keys: Vec<(usize, usize)> = layers.chains.iter().map(key).collect();
    keys.dedup();
    let keys = keys.iter().enumerate().filter(|(i, k)| !keys[..*i].contains(k)).map(|(_, &k)| k);
    // Members of a block, in row order, must sit next to each other in every rank.
    let blocks = keys.flat_map(|k| {
        let mut members: Vec<&(usize, Vec<usize>, usize)> = layers.chains.iter().filter(|c| key(c) == k).collect();
        members.sort_by_key(|(_, dummies, _)| p[dummies[0]]);
        let adjacent = members.windows(2).all(|w| w[0].1.iter().zip(&w[1].1).all(|(&x, &y)| rank_of[x] == rank_of[y] && p[y] == p[x] + 1));
        if adjacent { vec![members] } else { members.into_iter().map(|m| vec![m]).collect() }
    });
    blocks.fold(cross, |mut cross, members| {
        let (lo, hi) = (members[0].0, members[0].2);
        let ranks = members[0].1.len();
        // Each member's offset from the first, at the widest separation any rank needs.
        let offsets = members.iter().scan((0.0, None::<&Vec<usize>>), |(offset, previous), (_, dummies, _)| {
            if let Some(prev) = previous {
                *offset += prev.iter().zip(dummies).map(|(&x, &y)| separation(local, layers, x, y)).fold(0.0, f64::max);
            }
            *previous = Some(dummies);
            Some(*offset)
        });
        let offsets: Vec<f64> = offsets.collect();
        let width = *offsets.last().unwrap();
        let free = (0..ranks).fold((f64::MIN, f64::MAX), |(a, b), r| {
            let (first, last) = (members[0].1[r], members[members.len() - 1].1[r]);
            let row = &order[rank_of[first]];
            let left = (p[first] > 0).then(|| cross[row[p[first] - 1]] + separation(local, layers, row[p[first] - 1], first));
            let right = row.get(p[last] + 1).map(|&n| cross[n] - separation(local, layers, last, n) - width);
            (left.map_or(a, |l| a.max(l)), right.map_or(b, |r| b.min(r)))
        });
        let meet = |x: (f64, f64), y: (f64, f64)| (x.0.max(y.0), x.1.min(y.1));
        let fit = |item: usize| {
            let (from, to) = attach_range(local, &cross, item);
            (from, to - width)
        };
        let (a, b) = (fit(lo), fit(hi));
        let options = [meet(free, meet(a, b)), meet(free, a), meet(free, b)];
        let Some(&(from, to)) = options.iter().find(|r| r.0 <= r.1) else { return cross };
        // A straight block goes to the middle of the ends' common range, which
        // leaves the most room beside it. Otherwise it moves as little as it can.
        let both = meet(a, b);
        let starts = members.iter().zip(&offsets).flat_map(|((_, dummies, _), offset)| dummies.iter().map(move |&d| (d, offset)));
        let mean = starts.clone().map(|(d, offset)| cross[d] - offset).sum::<f64>() / (ranks * members.len()) as f64;
        let wanted = if both.0 <= both.1 { (both.0 + both.1) / 2.0 } else { mean }.clamp(from, to);
        let snapped = snap(wanted);
        let base = if (from..=to).contains(&snapped) { snapped } else { wanted };
        for (d, offset) in starts {
            cross[d] = base + offset;
        }
        cross
    })
}

/// The free span for item or dummy `x` in its row: between its neighbours, at
/// their separations.
fn row_room(local: &Local, layers: &Layers, order: &[Vec<usize>], p: &[usize], rank_of: &[usize], cross: &[f64], x: usize) -> (f64, f64) {
    let row = &order[rank_of[x]];
    let i = p[x];
    let left = (i > 0).then(|| cross[row[i - 1]] + separation(local, layers, row[i - 1], x));
    let right = row.get(i + 1).map(|&n| cross[n] - separation(local, layers, x, n));
    (left.unwrap_or(f64::MIN), right.unwrap_or(f64::MAX))
}

/// Moves pendant items in line with what they hang from: an item joined to the
/// rest by a path of items with only two edges each, ending in an item with one,
/// such as an initial state above its first state or a final state below its
/// last. From the attachment outward, each item and the dummies of its edge move
/// onto the line through the centre of the item before it, as far as their rows
/// allow. Items under order constraints, or with same-rank edges, stay put.
pub(super) fn align_pendants(local: &Local, layers: &Layers, order: &[Vec<usize>], cross: Vec<f64>) -> Vec<f64> {
    let count = local.count;
    let p = positions(order, layers.total);
    let rank_of = rank_index(order, layers.total);
    let mut pairs: Vec<(usize, usize)> = local.edges.iter().filter(|e| e.from != e.to).map(|e| (e.from.min(e.to), e.from.max(e.to))).collect();
    pairs.sort_unstable();
    pairs.dedup();
    let neighbours = |x: usize| pairs.iter().filter_map(move |&(a, b)| match () {
        _ if a == x => Some(b),
        _ if b == x => Some(a),
        _ => None,
    });
    let constrained = |x: usize| local.adjacent.iter().chain(&local.across).any(|&(a, b)| a == x || b == x) || local.group[x] != x || local.group.iter().enumerate().any(|(i, &g)| g == x && i != x);
    let level = |x: usize| pairs.iter().any(|&(a, b)| (a == x || b == x) && rank_of[a] == rank_of[b]);
    let movable = |x: usize| !constrained(x) && !level(x) && neighbours(x).count() <= 2;
    // Each pendant path, from the item next to its attachment out to its leaf.
    let paths = (0..count).filter(|&x| neighbours(x).count() == 1 && movable(x)).map(|leaf| {
        let walk = std::iter::successors(Some((leaf, usize::MAX)), |&(x, came)| {
            let next = neighbours(x).find(|&n| n != came)?;
            movable(next).then_some((next, x))
        });
        let mut items: Vec<usize> = walk.map(|(x, _)| x).take(count).collect();
        let anchor = neighbours(*items.last().unwrap()).find(|&n| items.len() < 2 || n != items[items.len() - 2]);
        items.reverse();
        (anchor, items)
    });
    let chain_of = |a: usize, b: usize| layers.chains.iter().find(|(lo, _, hi)| (*lo == a && *hi == b) || (*lo == b && *hi == a)).map(|(_, dummies, _)| dummies);
    paths.fold(cross, |cross, (anchor, items)| {
        let Some(anchor) = anchor else { return cross };
        items.iter().fold((cross, anchor), |(mut cross, before), &x| {
            let dummies = chain_of(before, x).cloned().unwrap_or_default();
            let (ours, theirs) = (attach_range(local, &cross, x), attach_range(local, &cross, before));
            let fits = |cross: &[f64], y: usize, at: f64| {
                let (lo, hi) = row_room(local, layers, order, &p, &rank_of, cross, y);
                lo <= at && at <= hi
            };
            // The line the edge takes: the nearest place the item before can take,
            // moving the edge's dummies there if their rows allow.
            let start = dummies.first().map_or(cross[x], |&d| cross[d]);
            let line = snap(start.clamp(theirs.0, theirs.1)).clamp(theirs.0, theirs.1);
            if dummies.iter().all(|&d| fits(&cross, d, line)) {
                for &d in &dummies {
                    cross[d] = line;
                }
            }
            let line = dummies.first().map_or(line, |&d| cross[d]);
            if line < theirs.0 || line > theirs.1 || (ours.0 <= line && line <= ours.1) {
                return (cross, x);
            }
            // The smallest move that brings the line within this item's range.
            let shift = if line < ours.0 { line - ours.0 } else { line - ours.1 };
            let reach = (ours.1 - ours.0) / 2.0;
            let exact = cross[x] + shift;
            let snapped = snap(exact);
            let target = if (snapped - line).abs() <= reach { snapped } else { exact };
            if fits(&cross, x, target) {
                cross[x] = target;
            }
            (cross, x)
        }).0
    })
}
