// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! L over Z: after nudging, a Z-shaped line whose end sits on a movable box
//! becomes an L that meets the side of the box facing it, when that stands
//! better (Drawing principles, Lines: an L over a Z).

use super::grid::{blocked, simplify};
use super::nudge::{is_orthogonal, standing};
use super::{manhattan, Body, Connection, Problem, Terminal, SLIDE_GAP, STEM};
use crate::core::ends;
use crate::core::geometry::{Point, Rect, Side, GRID};
use crate::core::metrics;

/// The shortest run into a side, so the arrowhead shows.
const ARRIVAL: f64 = 16.0;

/// The point on `side` of `r` at `along`.
fn on_side(r: &Rect, side: Side, along: f64) -> Point {
    match side {
        Side::Left => (r.x, along),
        Side::Right => (r.right(), along),
        Side::Top => (along, r.y),
        Side::Bottom => (along, r.bottom()),
    }
}

/// L-shaped lines that keep a Z's start and first direction, and end on
/// another side of `body`, the box at the Z's end: the first run continues past
/// the box's corner and turns into the side facing it. `from` is the start.
fn l_shapes(path: &[Point], from: &Terminal, body: &Body, arrive: f64) -> Vec<Vec<Point>> {
    let (p0, p1) = (path[0], path[1]);
    let vertical = p0.0 == p1.0;
    let r = body.rect;
    let (dx, dy) = from.outward.out();
    // Across the first run: where it passes the box, and the side it meets.
    let (at, lo, hi, near, far) = if vertical { (p0.0, r.x, r.right(), Side::Left, Side::Right) } else { (p0.1, r.y, r.bottom(), Side::Top, Side::Bottom) };
    let side = match () {
        _ if at <= lo - arrive => near,
        _ if at >= hi + arrive => far,
        _ => return Vec::new(),
    };
    if !body.allows(side) {
        return Vec::new();
    }
    let (span_lo, span_hi) = if vertical { ((r.y + body.inset).max(r.y + body.head + body.inset), r.bottom() - body.inset) } else { (r.x + body.inset, r.right() - body.inset) };
    let (start, out) = if vertical { (p0.1, dy) } else { (p0.0, dx) };
    let steps = ((span_lo / GRID).ceil() as i64..=(span_hi / GRID).floor() as i64).map(|g| g as f64 * GRID);
    steps
        .filter(|&along| (along - start) * out >= from.reach().max(STEM))
        .map(|along| {
            let corner = if vertical { (at, along) } else { (along, at) };
            vec![p0, corner, on_side(&r, side, along)]
        })
        .collect()
}

/// Whether `rect` holds `p`, edges included.
fn holds(rect: &Rect, p: Point) -> bool {
    rect.x <= p.0 && p.0 <= rect.right() && rect.y <= p.1 && p.1 <= rect.bottom()
}

/// Segments of `path` through obstacles of `connection`, and segments along
/// its strips. The obstacle around each end, which every line starts in, does
/// not count: for an end on a body, the one that also holds the body's centre.
fn intrusions_of(connection: &Connection, path: &[Point], bodies: [Option<&Body>; 2]) -> (usize, usize) {
    let ends = [path[0], path[path.len() - 1]];
    let own = |o: &Rect| ends.iter().zip(bodies).any(|(&p, body)| holds(o, p) && body.is_none_or(|b| holds(o, b.rect.center())));
    let walls: Vec<Rect> = connection.obstacles.iter().filter(|o| !own(o)).copied().collect();
    let pieces = path.windows(2);
    let hits = pieces.clone().filter(|w| blocked(&walls, w[0], w[1])).count();
    let strips = pieces.filter(|w| blocked(if w[0].1 == w[1].1 { &connection.strips.horizontal } else { &connection.strips.vertical }, w[0], w[1])).count();
    (hits, strips)
}

/// How many strips `path` crosses: (horizontal runs across strips beside
/// left and right boundaries, vertical runs across strips beside top and
/// bottom boundaries). A line that left a container through its side should
/// not come to leave it through its top instead.
fn crossings_of(connection: &Connection, path: &[Point]) -> (usize, usize) {
    let across = |horizontal: bool, strips: &[Rect]| path.windows(2).filter(|w| (w[0].1 == w[1].1) == horizontal).map(|w| strips.iter().filter(|s| blocked(std::slice::from_ref(s), w[0], w[1])).count()).sum();
    (across(true, &connection.strips.vertical), across(false, &connection.strips.horizontal))
}

/// Whether an end at `p` on a side of `body` keeps `SLIDE_GAP` from every
/// other line's end on that side.
fn spaced(paths: &[Vec<Point>], i: usize, body: &Body, p: Point) -> bool {
    let r = body.rect;
    let on_left_right = p.0 == r.x || p.0 == r.right();
    let same_side = |q: Point| if on_left_right { q.0 == p.0 && q.1 >= r.y && q.1 <= r.bottom() } else { q.1 == p.1 && q.0 >= r.x && q.0 <= r.right() };
    let gap = |q: Point| if on_left_right { (q.1 - p.1).abs() } else { (q.0 - p.0).abs() };
    let ends = paths.iter().enumerate().filter(|&(j, l)| j != i && l.len() >= 2).flat_map(|(_, l)| [l[0], l[l.len() - 1]]);
    ends.filter(|&q| same_side(q)).all(|q| gap(q) >= SLIDE_GAP)
}

/// Whether no other line ends on the side of `body` that holds `p`.
fn alone(paths: &[Vec<Point>], i: usize, body: &Body, p: Point) -> bool {
    let r = body.rect;
    let on_left_right = p.0 == r.x || p.0 == r.right();
    let same_side = |q: Point| if on_left_right { q.0 == p.0 && q.1 >= r.y && q.1 <= r.bottom() } else { q.1 == p.1 && q.0 >= r.x && q.0 <= r.right() };
    let ends = paths.iter().enumerate().filter(|&(j, l)| j != i && l.len() >= 2).flat_map(|(_, l)| [l[0], l[l.len() - 1]]);
    !ends.into_iter().any(same_side)
}

/// Whether `p`, on a side of `body`, lies at the middle of that side.
fn centred(body: &Body, p: Point) -> bool {
    let (cx, cy) = body.rect.center();
    let on_left_right = p.0 == body.rect.x || p.0 == body.rect.right();
    if on_left_right { (p.1 - cy).abs() <= ends::CENTRED } else { (p.0 - cx).abs() <= ends::CENTRED }
}

/// Turns Z-shaped lines into L-shaped ones where an end may move to the side
/// of its box that faces the line (Drawing principles, Lines: an L over a Z).
/// A line changes only when it then meets no obstacle or strip more than
/// before, its new end keeps clear of other ends, and it stands better: no more
/// overlaps, crossings or close runs, one bend fewer, and no two lines come to
/// leave one point together.
pub(super) fn prefer_l(problem: &Problem, paths: Vec<Vec<Point>>, fixed: &[bool]) -> Vec<Vec<Point>> {
    // Each change takes a bend or two off one line, so a few passes settle it.
    (0..paths.len()).fold(paths, |paths, i| (0..4).fold(paths, |paths, _| prefer_l_once(problem, paths, fixed, i)))
}

/// The side whose outward direction runs from `a` to `b`.
fn heading(a: Point, b: Point) -> Side {
    match () {
        _ if a.0 == b.0 && b.1 > a.1 => Side::Bottom,
        _ if a.0 == b.0 => Side::Top,
        _ if b.0 > a.0 => Side::Right,
        _ => Side::Left,
    }
}

/// The L shapes for the end at the close of `path`: the last three runs, when
/// the first of them is parallel to the last (a Z or a U at the end), are
/// replaced by two whose corner lies beside the box, so the line meets the side
/// it arrives from (Drawing principles, Lines: meet the side the line comes
/// from). On a three-run line the start is the line's own terminal; on a longer
/// line it is the bend where the last three runs begin.
fn tail_shapes(path: &[Point], from: &Terminal, body: &Body, arrive: f64) -> Vec<Vec<Point>> {
    let n = path.len();
    if n < 4 {
        return Vec::new();
    }
    let tail = &path[n - 4..];
    let parallel = (tail[0].0 == tail[1].0) == (tail[2].0 == tail[3].0);
    if metrics::bends(tail) != 2 || !parallel {
        return Vec::new();
    }
    let start = if n == 4 { *from } else { Terminal { point: tail[0], stem: tail[0], outward: heading(tail[0], tail[1]), slide: None, body: None } };
    let head = &path[..n - 4];
    l_shapes(tail, &start, body, arrive).into_iter().map(|shape| simplify(head.iter().copied().chain(shape).collect())).collect()
}

/// One L over Z step for line `i`, at either end, or the paths unchanged.
fn prefer_l_once(problem: &Problem, paths: Vec<Vec<Point>>, fixed: &[bool], i: usize) -> Vec<Vec<Point>> {
    {
        let path = &paths[i];
        let connection = &problem.connections[i];
        let (from, to) = (&connection.from, &connection.to);
        if fixed[i] || path.len() < 4 || (from.body.is_none() && to.body.is_none()) {
            return paths;
        }
        let before = intrusions_of(connection, path, [from.body.as_ref(), to.body.as_ref()]);
        let crossed = crossings_of(connection, path);
        let current = standing(&paths, i, path);
        let merged = metrics::merged_ends(&paths);
        // Move the end at `to`, or reverse the line and move the end at `from`.
        let reversed: Vec<Point> = path.iter().rev().copied().collect();
        let moved_to = to.body.map(|b| (tail_shapes(path, from, &b, to.reach().max(ARRIVAL)), b, [from.body, Some(b)], false));
        let moved_from = from.body.map(|b| (tail_shapes(&reversed, to, &b, from.reach().max(ARRIVAL)).into_iter().map(|l| l.into_iter().rev().collect()).collect(), b, [Some(b), to.body], true));
        let options = moved_to.into_iter().chain(moved_from).flat_map(|(shapes, body, bodies, at_start): (Vec<Vec<Point>>, Body, [Option<Body>; 2], bool)| {
            shapes.into_iter().filter_map(move |shape| {
                let moved_end = if at_start { shape[0] } else { shape[shape.len() - 1] };
                let (hits, strips) = intrusions_of(connection, &shape, [bodies[0].as_ref(), bodies[1].as_ref()]);
                let middle = body.rect.center();
                let off_centre = (moved_end.0 - middle.0).abs() + (moved_end.1 - middle.1).abs();
                let across = crossings_of(connection, &shape);
                (hits <= before.0 && strips <= before.1 && (!at_start || (across.0 <= crossed.0 && across.1 <= crossed.1))).then_some((shape, body, moved_end, off_centre))
            })
        });
        let accepted = options.filter_map(|(shape, body, moved_end, off_centre)| {
            if !spaced(&paths, i, &body, moved_end) || (alone(&paths, i, &body, moved_end) && !centred(&body, moved_end)) {
                return None;
            }
            let standing_now = standing(&paths, i, &shape);
            let mut next = paths.clone();
            next[i] = shape;
            let better = standing_now.0 <= current.0 && standing_now.1 <= current.1 && standing_now.2 <= current.2 && standing_now.3 < current.3;
            if !(better && metrics::merged_ends(&next) <= merged) {
                return None;
            }
            // The end left behind returns to its middle, or the L is not worth it.
            let next = if problem.centred { recentre(problem, next, fixed, path) } else { next };
            (!problem.centred || !stranded(problem, &next, path)).then_some((standing_now, off_centre, next))
        });
        let best = accepted.min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        best.map_or(paths.clone(), |(_, _, next)| next)
    }
}

/// After a line's end moved off a side, the one end left on that side moves to
/// its middle when its box is known, its line stays as clear and stands no
/// worse (Drawing principles, Lines: meet the middle). `old` is the moved
/// line as it was.
fn recentre(problem: &Problem, paths: Vec<Vec<Point>>, fixed: &[bool], old: &[Point]) -> Vec<Vec<Point>> {
    let left = [old[0], old[old.len() - 1]];
    (0..paths.len()).fold(paths, |paths, j| {
        let connection = &problem.connections[j];
        let path = &paths[j];
        if fixed[j] || path.len() < 3 {
            return paths;
        }
        let ends = [(0, &connection.from), (path.len() - 1, &connection.to)];
        let found = ends.into_iter().find_map(|(k, t)| {
            let body = t.body?;
            let p = path[k];
            let shares = left.iter().any(|&q| q != p && side_of(&body.rect, q).is_some() && side_of(&body.rect, q) == side_of(&body.rect, p));
            (shares && alone(&paths, j, &body, p) && !centred(&body, p)).then_some((k, body, p))
        });
        let Some((k, body, p)) = found else { return paths };
        let (cx, cy) = body.rect.center();
        let vertical_side = p.0 == body.rect.x || p.0 == body.rect.right();
        let near = if k == 0 { 1 } else { path.len() - 2 };
        let mut next_path = path.clone();
        if vertical_side {
            next_path[k].1 = cy;
            next_path[near].1 = cy;
        } else {
            next_path[k].0 = cx;
            next_path[near].0 = cx;
        }
        let next_path = simplify(next_path);
        let orthogonal = is_orthogonal(&next_path) && next_path.len() >= 2;
        let before = intrusions_of(connection, path, [connection.from.body.as_ref(), connection.to.body.as_ref()]);
        let after = intrusions_of(connection, &next_path, [connection.from.body.as_ref(), connection.to.body.as_ref()]);
        let run = |l: &[Point], at_start: bool| if at_start { manhattan(l[0], l[1]) } else { manhattan(l[l.len() - 2], l[l.len() - 1]) };
        let (own, other) = if k == 0 { (&connection.from, &connection.to) } else { (&connection.to, &connection.from) };
        let keeps_stem = next_path.len() >= 2 && run(&next_path, k == 0) >= own.reach() && run(&next_path, k != 0) >= other.reach();
        if !orthogonal || !keeps_stem || after.0 > before.0 || after.1 > before.1 || !spaced(&paths, j, &body, next_path[if k == 0 { 0 } else { next_path.len() - 1 }]) {
            return paths;
        }
        let (was, now) = (standing(&paths, j, path), standing(&paths, j, &next_path));
        if now.0 <= was.0 && now.1 <= was.1 && now.2 <= was.2 && now.3 <= was.3 {
            let mut next = paths;
            next[j] = next_path;
            next
        } else {
            paths
        }
    })
}

/// Whether an end that shared a side with an end of `old`, now moved away, is
/// left alone there off the middle.
fn stranded(problem: &Problem, paths: &[Vec<Point>], old: &[Point]) -> bool {
    let left = [old[0], old[old.len() - 1]];
    (0..paths.len()).any(|j| {
        let path = &paths[j];
        let connection = &problem.connections[j];
        path.len() >= 2
            && [(path[0], &connection.from), (path[path.len() - 1], &connection.to)].into_iter().any(|(p, t)| {
                t.body.is_some_and(|body| {
                    let shared = left.iter().any(|&q| q != p && side_of(&body.rect, q).is_some() && side_of(&body.rect, q) == side_of(&body.rect, p));
                    shared && !left.contains(&p) && alone(paths, j, &body, p) && !centred(&body, p)
                })
            })
    })
}

/// Which side of `r` holds `p`, if any.
fn side_of(r: &Rect, p: Point) -> Option<Side> {
    let within_y = p.1 > r.y && p.1 < r.bottom();
    let within_x = p.0 > r.x && p.0 < r.right();
    match () {
        _ if p.0 == r.x && within_y => Some(Side::Left),
        _ if p.0 == r.right() && within_y => Some(Side::Right),
        _ if p.1 == r.y && within_x => Some(Side::Top),
        _ if p.1 == r.bottom() && within_x => Some(Side::Bottom),
        _ => None,
    }
}
