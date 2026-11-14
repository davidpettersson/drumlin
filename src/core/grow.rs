// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Box growth after layout. A connector jogs when the sides of its two end boxes
//! offer no common span for a straight line. Growing one end box by a grid step
//! or two toward the other end often lets it run straight. Each growth is tried
//! by routing again, and kept only when the diagram gets strictly better: the
//! connector loses a bend, no total rises, and no two connectors come to leave
//! one point together. Boxes grow only into free space, so the diagram's bounds
//! do not change, and names centre in the final box as before.

use super::geometry::{Direction, Point, Rect, GRID};
use super::layered::Insets;
use super::metrics::{bends, measure, merged_ends, Metrics};

/// The most a box grows on one side: two grid steps, too little to read as a
/// different size beside its neighbours.
pub const MAX_GROWTH: f64 = 2.0 * GRID;
/// The space a grown box keeps from every box other than its own containers.
const CLEARANCE: f64 = 16.0;
/// The least distance from a corner where a line may attach to a side.
const ATTACH_INSET: f64 = 8.0;
/// The space a grown box keeps from reserved areas such as labels.
const KEEP_OUT_CLEARANCE: f64 = 4.0;

/// Where boxes may grow. Every vector is indexed like the rects.
pub struct Room {
    pub direction: Direction,
    /// Whether a box may grow: plain leaf boxes whose drawing does not hang on
    /// their size, such as loops or interface stubs.
    pub growable: Vec<bool>,
    pub parents: Vec<Option<usize>>,
    /// The area a box must stay inside: its container's content area.
    pub within: Vec<Rect>,
    /// Areas a grown box must not come near: reserved labels, loops, stubs.
    pub keep_out: Vec<Rect>,
    /// The two end boxes of each routed line, indexed like the lines.
    pub ends: Vec<(usize, usize)>,
    /// A fixed point a line passes through on its way, such as a reserved
    /// label's anchor, indexed like the lines. Each end then lines up with it
    /// rather than with the other end.
    pub via: Vec<Option<Point>>,
}

/// The content area of a container: its rect less its insets, with `slack` of
/// the padding given back for growth.
pub fn content(r: Rect, insets: Insets, slack: f64) -> Rect {
    let [top, right, bottom, left] = [insets.top, insets.right, insets.bottom, insets.left].map(|v| (v - slack).max(0.0));
    Rect { x: r.x + left, y: r.y + top, w: r.w - left - right, h: r.h - top - bottom }
}

/// One side of a box, as the growth stage names it.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// `r` with `edge` moved out to `to`.
fn with_edge(r: Rect, edge: Edge, to: f64) -> Rect {
    match edge {
        Edge::Left => Rect { x: to, w: r.right() - to, ..r },
        Edge::Right => Rect { w: to - r.x, ..r },
        Edge::Top => Rect { y: to, h: r.bottom() - to, ..r },
        Edge::Bottom => Rect { h: to - r.y, ..r },
    }
}

/// Where `edge` of `r` lies, and which way is outward.
fn edge_at(r: Rect, edge: Edge) -> (f64, f64) {
    match edge {
        Edge::Left => (r.x, -1.0),
        Edge::Right => (r.right(), 1.0),
        Edge::Top => (r.y, -1.0),
        Edge::Bottom => (r.bottom(), 1.0),
    }
}

/// The strip a box gains when `edge` moves from `old` out to `new`.
fn gained(old: Rect, new: Rect, edge: Edge) -> Rect {
    match edge {
        Edge::Left => Rect { x: new.x, w: old.x - new.x, ..old },
        Edge::Right => Rect { x: old.right(), w: new.right() - old.right(), ..old },
        Edge::Top => Rect { y: new.y, h: old.y - new.y, ..old },
        Edge::Bottom => Rect { y: old.bottom(), h: new.bottom() - old.bottom(), ..old },
    }
}

fn ancestors(room: &Room, n: usize) -> Vec<usize> {
    std::iter::successors(room.parents[n], |&p| room.parents[p]).collect()
}

fn inside(r: &Rect, area: &Rect) -> bool {
    r.x >= area.x && r.y >= area.y && r.right() <= area.right() && r.bottom() <= area.bottom()
}

/// Whether box `n` may grow from `old` to `new` on `edge` among `rects`.
fn fits(room: &Room, rects: &[Rect], n: usize, old: Rect, new: Rect, edge: Edge) -> bool {
    let strip = gained(old, new, edge);
    let up = ancestors(room, n);
    let others = (1..rects.len()).filter(|&m| m != n && !up.contains(&m) && !ancestors(room, m).contains(&n));
    let clear_of_boxes = others.into_iter().all(|m| !rects[m].intersects(&strip.inflate(CLEARANCE)));
    let clear_of_reserved = room.keep_out.iter().all(|k| !k.intersects(&strip.inflate(KEEP_OUT_CLEARANCE)));
    room.growable[n] && inside(&strip, &room.within[n]) && clear_of_boxes && clear_of_reserved
}

/// A strict improvement: fewer crossings, overlaps or bends in that order, and
/// none of them more.
fn better(a: Metrics, b: Metrics) -> bool {
    a < b && a.crossings <= b.crossings && a.overlaps <= b.overlaps
}

/// The sides of `node` that face `other`, cross-flow axis first: growing a box
/// across the flow keeps the rank's extent along the flow even.
fn toward(direction: Direction, node: Rect, other: Rect) -> Vec<Edge> {
    let (n, o) = (node.center(), other.center());
    let horizontal = (o.0 - n.0).abs() >= 1.0;
    let vertical = (o.1 - n.1).abs() >= 1.0;
    let x = horizontal.then_some(if o.0 > n.0 { Edge::Right } else { Edge::Left });
    let y = vertical.then_some(if o.1 > n.1 { Edge::Bottom } else { Edge::Top });
    match direction {
        Direction::Down => [x, y],
        Direction::Right => [y, x],
    }
    .into_iter()
    .flatten()
    .collect()
}

/// Whether a box grown on `edge` and its peer lie apart across that edge's axis
/// and share a span along it, so that a straight line could join them. Growth
/// that cannot achieve this is not worth routing.
fn lines_up(grown: Rect, peer: Rect, edge: Edge) -> bool {
    let along_x = matches!(edge, Edge::Left | Edge::Right);
    let (lo, hi, apart) = if along_x {
        (grown.x.max(peer.x), grown.right().min(peer.right()), grown.bottom() <= peer.y || peer.bottom() <= grown.y)
    } else {
        (grown.y.max(peer.y), grown.bottom().min(peer.bottom()), grown.right() <= peer.x || peer.right() <= grown.x)
    };
    apart && lo + ATTACH_INSET <= hi - ATTACH_INSET
}

struct State {
    rects: Vec<Rect>,
    paths: Vec<Vec<Point>>,
    score: Metrics,
    merged: usize,
}

fn assess(rects: Vec<Rect>, route: &impl Fn(&[Rect]) -> Vec<Vec<Point>>) -> State {
    let paths = route(&rects);
    State { score: measure(&paths), merged: merged_ends(&paths), rects, paths }
}

/// The first growth that straightens line `e` some, or `None`.
fn straighten(room: &Room, original: &[Rect], current: &State, e: usize, route: &impl Fn(&[Rect]) -> Vec<Vec<Point>>) -> Option<State> {
    let (a, b) = room.ends[e];
    let related = |n: usize, m: usize| n == m || ancestors(room, n).contains(&m) || ancestors(room, m).contains(&n);
    if related(a, b) {
        return None;
    }
    let steps = (1..=(MAX_GROWTH / GRID) as usize).map(|k| k as f64 * GRID);
    let tries = steps.flat_map(|amount| [(a, b), (b, a)].into_iter().map(move |pair| (amount, pair)));
    tries
        .flat_map(|(amount, (node, other))| {
            let peers: Vec<Rect> = [Some(current.rects[other]), room.via[e].map(|(x, y)| Rect { x, y, w: 0.0, h: 0.0 })].into_iter().flatten().collect();
            let edges: Vec<Edge> = peers.iter().flat_map(|&peer| toward(room.direction, current.rects[node], peer)).fold(Vec::new(), |mut out, edge| {
                if !out.contains(&edge) {
                    out.push(edge);
                }
                out
            });
            edges.into_iter().map(move |edge| (amount, node, peers.clone(), edge))
        })
        .filter_map(|(amount, node, peers, edge)| {
            let (start, out) = edge_at(original[node], edge);
            let old = current.rects[node];
            let to = start + out * amount;
            // Only further out than it has grown already.
            let further = (to - edge_at(old, edge).0) * out > 0.0;
            let new = with_edge(old, edge, to);
            (further && peers.iter().any(|&peer| lines_up(new, peer, edge)) && fits(room, &current.rects, node, old, new, edge)).then(|| {
                let mut rects = current.rects.clone();
                rects[node] = new;
                rects
            })
        })
        .map(|rects| assess(rects, route))
        .find(|trial| bends(&trial.paths[e]) < bends(&current.paths[e]) && better(trial.score, current.score) && trial.merged <= current.merged)
}

/// Grows end boxes, one connector at a time in order, until no growth helps.
/// `route` routes the diagram for a set of rects.
pub fn grow(rects: Vec<Rect>, room: &Room, route: impl Fn(&[Rect]) -> Vec<Vec<Point>>) -> Vec<Rect> {
    let original = rects.clone();
    let first = assess(rects, &route);
    let bent: Vec<usize> = (0..first.paths.len()).filter(|&e| bends(&first.paths[e]) > 0).collect();
    bent.into_iter().fold(first, |current, e| straighten(room, &original, &current, e, &route).unwrap_or(current)).rects
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(rects: &[Rect], ends: Vec<(usize, usize)>) -> Room {
        Room {
            direction: Direction::Down,
            growable: vec![false, true, true],
            parents: vec![None, Some(0), Some(0)],
            within: vec![rects[0]; rects.len()],
            keep_out: Vec::new(),
            via: vec![None; ends.len()],
            ends,
        }
    }

    /// A stand-in router: a straight line when the boxes' spans overlap across
    /// the gap, else a Z with two bends.
    fn route(rects: &[Rect]) -> Vec<Vec<Point>> {
        let (a, b) = (rects[1], rects[2]);
        let (lo, hi) = (a.x.max(b.x) + 8.0, a.right().min(b.right()) - 8.0);
        if lo <= hi {
            vec![vec![(lo, a.bottom()), (lo, b.y)]]
        } else {
            let mid = (a.bottom() + b.y) / 2.0;
            vec![vec![(a.center().0, a.bottom()), (a.center().0, mid), (b.center().0, mid), (b.center().0, b.y)]]
        }
    }

    #[test]
    fn a_box_grows_toward_its_peer_when_that_straightens_the_line() {
        let rects = vec![Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 }, Rect { x: 0.0, y: 0.0, w: 100.0, h: 50.0 }, Rect { x: 96.0, y: 150.0, w: 100.0, h: 50.0 }];
        let grown = grow(rects.clone(), &room(&rects, vec![(1, 2)]), route);
        assert_eq!(grown[1], Rect { x: 0.0, y: 0.0, w: 116.0, h: 50.0 });
        assert_eq!(grown[2], rects[2]);
    }

    #[test]
    fn a_box_never_grows_more_than_two_steps() {
        let rects = vec![Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 }, Rect { x: 0.0, y: 0.0, w: 100.0, h: 50.0 }, Rect { x: 180.0, y: 150.0, w: 100.0, h: 50.0 }];
        let grown = grow(rects.clone(), &room(&rects, vec![(1, 2)]), route);
        assert_eq!(grown, rects);
    }

    #[test]
    fn a_box_does_not_grow_into_a_reserved_area() {
        let rects = vec![Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 }, Rect { x: 0.0, y: 0.0, w: 100.0, h: 50.0 }, Rect { x: 96.0, y: 150.0, w: 100.0, h: 50.0 }];
        let blocked = Room { keep_out: vec![Rect { x: 104.0, y: 0.0, w: 40.0, h: 20.0 }], growable: vec![false, true, false], ..room(&rects, vec![(1, 2)]) };
        assert_eq!(grow(rects.clone(), &blocked, route), rects);
    }
}
