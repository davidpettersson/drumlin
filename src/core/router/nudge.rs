// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Nudging after routing: interior segments move along their own axis to
//! separate, straighten and spread lines, without crossing an obstacle or a
//! soft strip, and merging lines meet at one junction.

use super::grid::{blocked, simplify};
use super::{Problem, Terminal};
use crate::core::ends;
use crate::core::geometry::{Point, Rect, GRID};
use crate::core::metrics;

/// The closest two ends on one side may come when nudging slides them, which
/// is closer than slots are first spread: two arrowheads still read apart.
pub const SLIDE_GAP: f64 = 16.0;

/// Parallel lines of different connections closer than this read as one.
pub(super) const SEPARATION: f64 = 8.0;
/// Improvement rounds in `nudge`.
const NUDGE_ROUNDS: usize = 4;

/// An interior segment of a path: never the first or last, which stay on their
/// terminals' lines.
#[derive(Clone, Copy, Debug)]
struct Segment {
    path: usize,
    /// The segment runs from point `k` to point `k + 1`.
    k: usize,
    horizontal: bool,
    /// Its fixed coordinate: y for a horizontal segment, x for a vertical one.
    at: f64,
    /// Its extent along the other axis.
    span: (f64, f64),
    /// How far it may move without passing an obstacle or shortening a stem.
    range: (f64, f64),
    /// Its neighbours leave in opposite directions, as in a Z. Otherwise it is
    /// the bottom of a U.
    stair: bool,
}

fn coord(horizontal: bool, p: Point) -> f64 {
    if horizontal { p.1 } else { p.0 }
}

fn along(horizontal: bool, p: Point) -> f64 {
    if horizontal { p.0 } else { p.1 }
}

pub(super) fn is_orthogonal(path: &[Point]) -> bool {
    path.windows(2).all(|w| w[0].0 == w[1].0 || w[0].1 == w[1].1)
}

/// The box around all items, with room for lines that run outside them.
fn frame(problem: &Problem) -> Rect {
    let (x0, y0, x1, y1) = problem.bounds.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |(x0, y0, x1, y1), r| (x0.min(r.x), y0.min(r.y), x1.max(r.right()), y1.max(r.bottom())));
    if x0 > x1 {
        return Rect::default();
    }
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }.inflate(32.0)
}

/// What a nudge needs to know about the problem.
struct Room<'a> {
    problem: &'a Problem,
    frame: Rect,
    /// Connections whose paths must not change: direct lines and failures.
    fixed: Vec<bool>,
}

impl Room<'_> {
    /// Segment `k` of path `i`, with the room it has to move in, or `None` when
    /// it cannot move. A neighbour may shrink to nothing, merging this segment
    /// into the one beyond it, but a terminal's own segment keeps at least its
    /// stem. A terminal's own segment moves only if the terminal may slide.
    fn segment(&self, paths: &[Vec<Point>], i: usize, k: usize) -> Option<Segment> {
        let path = &paths[i];
        let last = path.len().checked_sub(2)?;
        if self.fixed[i] || last == 0 || k > last {
            return None;
        }
        let (a, b) = (path[k], path[k + 1]);
        let horizontal = a.1 == b.1;
        let at = coord(horizontal, a);
        let span = (along(horizontal, a).min(along(horizontal, b)), along(horizontal, a).max(along(horizontal, b)));
        let (from, to) = (&self.problem.connections[i].from, &self.problem.connections[i].to);
        // An end segment slides with its terminal, which keeps clear of its own box.
        let end = match () {
            _ if k == 0 => Some(from),
            _ if k == last => Some(to),
            _ => None,
        };
        let slide = match end {
            Some(t) => Some(t.slide?),
            None => None,
        };
        // A straight line's only segment slides with both of its terminals.
        let slide = match slide {
            Some((lo, hi)) if k == 0 && k == last => {
                let (to_lo, to_hi) = to.slide?;
                Some((lo.max(to_lo), hi.min(to_hi)))
            }
            other => other,
        };
        let neighbours = [(k > 0).then(|| (path[k - 1], (k == 1).then_some(from))), (k < last).then(|| (path[k + 2], (k + 1 == last).then_some(to)))];
        let open = slide.unwrap_or(if horizontal { (self.frame.y.min(at), self.frame.bottom().max(at)) } else { (self.frame.x.min(at), self.frame.right().max(at)) });
        let (lo, hi) = neighbours.iter().flatten().fold(open, |(lo, hi), &(far, terminal)| {
            let limit = terminal.map_or(coord(horizontal, far), |t| coord(horizontal, t.stem));
            if coord(horizontal, far) < at { (lo.max(limit.min(at)), hi) } else { (lo, hi.min(limit.max(at))) }
        });
        let connection = &self.problem.connections[i];
        let (own, across) = if horizontal { (&connection.strips.horizontal, &connection.strips.vertical) } else { (&connection.strips.vertical, &connection.strips.horizontal) };
        // The segment sweeps the area between its old and new lines, and its
        // neighbours grow or shrink along the ends of its span. An obstacle or a
        // strip along the segment bounds the move where it overlaps the span; a
        // strip along the neighbours, where it holds one of them. A segment
        // already inside one, which only a forced route has, stays where it is.
        let overlaps_span = |o_lo: f64, o_hi: f64| o_lo < span.1 && span.0 < o_hi;
        let holds_an_end = |o_lo: f64, o_hi: f64| [span.0, span.1].iter().any(|&e| o_lo < e && e < o_hi);
        let around_terminal = |o: &Rect| end.is_some_and(|t| o.x <= t.point.0 && t.point.0 <= o.right() && o.y <= t.point.1 && t.point.1 <= o.bottom());
        let bounds = connection.obstacles.iter().chain(own).map(|o| (o, &overlaps_span as &dyn Fn(f64, f64) -> bool)).chain(across.iter().map(|o| (o, &holds_an_end as &dyn Fn(f64, f64) -> bool)));
        let range = bounds.filter(|(o, _)| !around_terminal(o)).try_fold((lo, hi), |(lo, hi), (o, relevant)| {
            let (o_lo, o_hi, c_lo, c_hi) = if horizontal { (o.x, o.right(), o.y, o.bottom()) } else { (o.y, o.bottom(), o.x, o.right()) };
            match () {
                _ if !relevant(o_lo, o_hi) => Some((lo, hi)),
                _ if c_hi <= at => Some((lo.max(c_hi), hi)),
                _ if c_lo >= at => Some((lo, hi.min(c_lo))),
                _ => None,
            }
        });
        let stair = match neighbours {
            [Some((before, _)), Some((after, _))] => (coord(horizontal, before) < at) == (coord(horizontal, after) > at),
            _ => false,
        };
        Some(Segment { path: i, k, horizontal, at, span, range: range.unwrap_or((at, at)), stair })
    }

    /// Path `i` with segment `k` moved to `to` on its own axis, or `None` if that
    /// would not leave an orthogonal path that still leaves and enters along its
    /// terminals' stems. A neighbour shrinking to nothing merges the segments
    /// around it, and a run doubling back on itself folds away.
    fn moved(&self, path: &[Point], i: usize, k: usize, horizontal: bool, to: f64) -> Option<Vec<Point>> {
        let shifted = path.iter().enumerate().map(|(n, &p)| match (n == k || n == k + 1, horizontal) {
            (true, true) => (p.0, to),
            (true, false) => (to, p.1),
            (false, _) => p,
        });
        self.checked(i, simplify(shifted.collect()))
    }

    /// `next` as a path for connection `i`, if it is orthogonal and still leaves
    /// and enters along its terminals' stems.
    fn checked(&self, i: usize, next: Vec<Point>) -> Option<Vec<Point>> {
        let keeps_stem = |point: Point, next_point: Point, t: &Terminal| {
            let (dx, dy) = t.outward.out();
            let reach = (t.stem.0 - t.point.0) * dx + (t.stem.1 - t.point.1) * dy;
            let along = (next_point.0 - point.0) * dx + (next_point.1 - point.1) * dy;
            let straight = (next_point.0 - point.0) * dy - (next_point.1 - point.1) * dx;
            straight == 0.0 && along >= reach
        };
        let (from, to) = (&self.problem.connections[i].from, &self.problem.connections[i].to);
        let ends_kept = match next[..] {
            [a, b, .., c, d] if next.len() > 2 => keeps_stem(a, b, from) && keeps_stem(d, c, to),
            _ => true,
        };
        (is_orthogonal(&next) && ends_kept).then_some(next)
    }

    /// Segments of `path` that pass through an obstacle, and segments that run
    /// along a strip, for connection `i`. Obstacles around a terminal, which a
    /// line always starts in, do not count.
    fn intrusions(&self, i: usize, path: &[Point]) -> (usize, usize) {
        let connection = &self.problem.connections[i];
        let (start, end) = (path[0], path[path.len() - 1]);
        let holds = |o: &Rect, p: Point| o.x <= p.0 && p.0 <= o.right() && o.y <= p.1 && p.1 <= o.bottom();
        let walls: Vec<Rect> = connection.obstacles.iter().filter(|o| !holds(o, start) && !holds(o, end)).copied().collect();
        let pieces = path.windows(2);
        let hits = pieces.clone().filter(|w| blocked(&walls, w[0], w[1])).count();
        let strips = pieces.filter(|w| blocked(if w[0].1 == w[1].1 { &connection.strips.horizontal } else { &connection.strips.vertical }, w[0], w[1])).count();
        (hits, strips)
    }

    fn segments(&self, paths: &[Vec<Point>], i: usize) -> Vec<Segment> {
        (0..paths[i].len().saturating_sub(1)).filter_map(|k| self.segment(paths, i, k)).collect()
    }
}

/// How a path fares against the others, worst first: overlaps, crossings, lines
/// running too close, bends. Lower is better. Lines lying on top of each other
/// cannot be told apart, which is worse than a crossing.
type Standing = (usize, usize, usize, usize);

pub(super) fn standing(paths: &[Vec<Point>], i: usize, candidate: &[Point]) -> Standing {
    let others = paths.iter().enumerate().filter(|&(j, p)| j != i && p.len() >= 2).map(|(_, p)| p);
    let (crossings, overlaps, close) = others.fold((0, 0, 0), |(c, o, n), p| (c + metrics::crossings_between(candidate, p), o + metrics::overlaps_between(candidate, p), n + metrics::close_runs(candidate, p, SEPARATION)));
    (overlaps, crossings, close, metrics::bends(candidate))
}

fn add(a: Standing, b: Standing) -> Standing {
    (a.0 + b.0, a.1 + b.1, a.2 + b.2, a.3 + b.3)
}

fn total(paths: &[Vec<Point>], which: &[usize]) -> Standing {
    which.iter().fold((0, 0, 0, 0), |sum, &i| add(sum, standing(paths, i, &paths[i])))
}

/// Grid positions in `range`, plus the lines of the segments two away, where a
/// move removes a jog.
fn candidates(path: &[Point], s: &Segment) -> Vec<f64> {
    let (lo, hi) = s.range;
    let grid = ((lo / GRID).ceil() as i64..=(hi / GRID).floor() as i64).map(|g| g as f64 * GRID);
    let merges = [s.k.checked_sub(1).map(|j| path[j]), path.get(s.k + 2).copied()].into_iter().flatten().map(|p| coord(s.horizontal, p));
    grid.chain(merges).filter(|&c| c >= lo && c <= hi && c != s.at).collect()
}

/// Moves each interior segment to the place in its range where its path stands
/// best, preferring small moves. Returns the paths and whether any moved.
fn improve(room: &Room, mut paths: Vec<Vec<Point>>) -> (Vec<Vec<Point>>, bool) {
    let mut changed = false;
    for i in 0..paths.len() {
        let mut k = 0;
        while k + 1 < paths[i].len() {
            let Some(s) = room.segment(&paths, i, k) else {
                k += 1;
                continue;
            };
            let current = standing(&paths, i, &paths[i]);
            let options = candidates(&paths[i], &s).into_iter().filter_map(|c| room.moved(&paths[i], i, k, s.horizontal, c).map(|p| (standing(&paths, i, &p), (c - s.at).abs(), p)));
            let best = options.min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
            if let Some((_, _, path)) = best.filter(|best| best.0 < current) {
                paths[i] = path;
                changed = true;
            }
            k += 1;
        }
        let (path, collapsed) = collapse_jogs(room, &paths, i);
        paths[i] = path;
        changed |= collapsed;
    }
    (paths, changed)
}

/// Removes jogs that single moves cannot: two parallel segments with one
/// between them move onto a common line, which may lie beyond both, when the
/// new path meets no more obstacles or strips than the old one and stands better.
fn collapse_jogs(room: &Room, paths: &[Vec<Point>], i: usize) -> (Vec<Point>, bool) {
    let mut all = paths.to_vec();
    let mut changed = false;
    let mut k = 0;
    while k + 4 <= all[i].len() {
        let (Some(a), Some(b)) = (room.segment(&all, i, k), room.segment(&all, i, k + 2)) else {
            k += 1;
            continue;
        };
        let path = &all[i];
        let mut lines: Vec<f64> = candidates(path, &a).into_iter().chain(candidates(path, &b)).chain([a.at, b.at]).collect();
        lines.sort_by(f64::total_cmp);
        lines.dedup();
        let before = room.intrusions(i, path);
        let current = standing(&all, i, path);
        // A terminal's own segment stays within its slide.
        let last = path.len() - 2;
        let slides = [(k == 0).then_some(room.problem.connections[i].from.slide), (k + 2 == last).then_some(room.problem.connections[i].to.slide)];
        let within = |c: f64| slides.iter().flatten().all(|slide| slide.is_some_and(|(lo, hi)| lo <= c && c <= hi));
        let options = lines.into_iter().filter(|&c| within(c)).filter_map(|c| {
            let shifted = path.iter().enumerate().map(|(n, &p)| match (n >= k && n <= k + 3, a.horizontal) {
                (true, true) => (p.0, c),
                (true, false) => (c, p.1),
                (false, _) => p,
            });
            let next = room.checked(i, simplify(shifted.collect()))?;
            let intrusions = room.intrusions(i, &next);
            (intrusions.0 <= before.0 && intrusions.1 <= before.1).then(|| (standing(&all, i, &next), (c - a.at).abs() + (c - b.at).abs(), next))
        });
        let best = options.min_by(|p, q| p.0.cmp(&q.0).then(p.1.total_cmp(&q.1)));
        if let Some((_, _, next)) = best.filter(|best| best.0 < current) {
            all[i] = next;
            changed = true;
        }
        k += 1;
    }
    (all.swap_remove(i), changed)
}

/// How far a joint move in `move_pairs` looks from each segment's line.
const PAIR_REACH: f64 = 48.0;

/// Moves two parallel segments of different paths together, when their lines
/// cross or overlap and no single move helps: two lines change their order in a
/// channel together, since moving either alone puts them on one line first.
fn move_pairs(room: &Room, mut paths: Vec<Vec<Point>>) -> (Vec<Vec<Point>>, bool) {
    let mut any = false;
    for i in 0..paths.len() {
        for j in i + 1..paths.len() {
            let tangled = metrics::crossings_between(&paths[i], &paths[j]) + metrics::overlaps_between(&paths[i], &paths[j]) > 0;
            if !tangled {
                continue;
            }
            let pairs: Vec<(Segment, Segment)> = room
                .segments(&paths, i)
                .into_iter()
                .flat_map(|a| room.segments(&paths, j).into_iter().map(move |b| (a, b)))
                .filter(|(a, b)| a.horizontal == b.horizontal && a.span.0.max(b.span.0) < a.span.1.min(b.span.1))
                .collect();
            let near = |path: &[Point], s: &Segment| candidates(path, s).into_iter().chain(std::iter::once(s.at)).filter(|c| (c - s.at).abs() <= PAIR_REACH).collect::<Vec<f64>>();
            let before = total(&paths, &[i, j]);
            let options = pairs.iter().flat_map(|(a, b)| {
                let (ca, cb) = (near(&paths[i], a), near(&paths[j], b));
                let paths = &paths;
                ca.into_iter().flat_map(move |x| cb.clone().into_iter().map(move |y| (x, y))).filter_map(move |(x, y)| {
                    let mut next = paths.clone();
                    next[i] = room.moved(&paths[i], i, a.k, a.horizontal, x)?;
                    next[j] = room.moved(&paths[j], j, b.k, b.horizontal, y)?;
                    Some((total(&next, &[i, j]), (x - a.at).abs() + (y - b.at).abs(), next))
                })
            });
            let best = options.min_by(|p, q| p.0.cmp(&q.0).then(p.1.total_cmp(&q.1)));
            if let Some((_, _, next)) = best.filter(|best| best.0 < before) {
                paths = next;
                any = true;
            }
        }
    }
    (paths, any)
}

fn find(parent: &[usize], x: usize) -> usize {
    std::iter::successors(Some(x), |&y| (parent[y] != y).then_some(parent[y])).last().unwrap()
}

/// Spreads the middle segments of Z shapes that share a channel evenly across
/// it, which centres a lone one in its gap. A group moves only if none of its
/// paths gains a crossing, an overlap, a close run or a bend.
fn spread(room: &Room, paths: Vec<Vec<Point>>) -> Vec<Vec<Point>> {
    let segments: Vec<Segment> = (0..paths.len()).flat_map(|i| room.segments(&paths, i)).filter(|s| s.stair && s.range.0 < s.range.1).collect();
    let shares = |a: &Segment, b: &Segment| a.horizontal == b.horizontal && a.span.0.max(b.span.0) < a.span.1.min(b.span.1) && a.range.0.max(b.range.0) < a.range.1.min(b.range.1);
    let pairs = (0..segments.len()).flat_map(|i| (i + 1..segments.len()).map(move |j| (i, j)));
    let parent = pairs.filter(|&(i, j)| shares(&segments[i], &segments[j])).fold((0..segments.len()).collect::<Vec<usize>>(), |mut parent, (i, j)| {
        let (a, b) = (find(&parent, i), find(&parent, j));
        parent[a.max(b)] = a.min(b);
        parent
    });
    let mut roots: Vec<usize> = (0..segments.len()).map(|i| find(&parent, i)).collect();
    roots.sort_unstable();
    roots.dedup();
    roots.into_iter().fold(paths, |paths, root| {
        let mut members: Vec<Segment> = (0..segments.len()).filter(|&i| find(&parent, i) == root).map(|i| segments[i]).collect();
        members.sort_by(|a, b| a.at.total_cmp(&b.at).then(a.path.cmp(&b.path)));
        let mut owners: Vec<usize> = members.iter().map(|s| s.path).collect();
        owners.sort_unstable();
        owners.dedup();
        // One segment per path, still where it was found: earlier groups may have
        // changed the paths.
        let current = |s: &Segment| paths[s.path].get(s.k + 1).is_some_and(|&p| coord(s.horizontal, p) == s.at && coord(s.horizontal, paths[s.path][s.k]) == s.at);
        if owners.len() != members.len() || !members.iter().all(current) {
            return paths;
        }
        let (lo, hi) = members.iter().fold((f64::MIN, f64::MAX), |(lo, hi), s| (lo.max(s.range.0), hi.min(s.range.1)));
        let count = members.len() as f64;
        if hi - lo < (count + 1.0) * SEPARATION {
            return paths;
        }
        let even = |j: usize, step: f64| {
            let exact = lo + (hi - lo) * (j as f64 + 1.0) / (count + 1.0);
            let snapped = (exact / step).round() * step;
            if snapped > lo && snapped < hi { snapped } else { exact }
        };
        let targets = [GRID, GRID / 2.0, 1.0].iter().map(|&step| (0..members.len()).map(|j| even(j, step)).collect::<Vec<f64>>()).find(|t| t.windows(2).all(|w| w[1] - w[0] >= SEPARATION));
        let proposal = targets.and_then(|targets| {
            members.iter().zip(&targets).try_fold(paths.clone(), |mut next, (s, &to)| {
                let path = room.moved(&next[s.path], s.path, s.k, s.horizontal, to)?;
                (path.len() == next[s.path].len()).then_some(())?;
                next[s.path] = path;
                Some(next)
            })
        });
        match proposal {
            Some(next) => {
                let (before, after) = (owners.iter().map(|&i| standing(&paths, i, &paths[i])), owners.iter().map(|&i| standing(&next, i, &next[i])));
                let worse = before.zip(after).any(|(b, a)| a.0 > b.0 || a.1 > b.1 || a.2 > b.2 || a.3 > b.3);
                if worse { paths } else { next }
            }
            None => paths,
        }
    })
}

/// The nudging stage after routing (Wybrow et al. 2009, section 5). Segments
/// move along their own axis only, so terminals and stems stay put, and never
/// across an obstacle or a soft strip. First each segment finds its best place
/// and tangled pairs move together, then shared channels are spread evenly.
pub(super) fn nudge(problem: &Problem, paths: Vec<Vec<Point>>, fixed: Vec<bool>) -> Vec<Vec<Point>> {
    let room = Room { problem, frame: frame(problem), fixed };
    let settled_paths = (0..NUDGE_ROUNDS).try_fold(paths, |paths, _| {
        let (paths, moved) = improve(&room, paths);
        let (paths, paired) = move_pairs(&room, paths);
        if moved || paired { Ok(paths) } else { Err(paths) }
    });
    let paths = settled_paths.unwrap_or_else(|paths| paths);
    join(&room, spread(&room, paths))
}

/// Merging lines meet at one junction (Drawing principles, Lines: one junction
/// per merge). When two lines that share an end part a few pixels apart, the
/// segment where the line running on turns moves back to where the other turns,
/// if its line then stands no worse.
fn join(room: &Room, paths: Vec<Vec<Point>>) -> Vec<Vec<Point>> {
    let rounds = paths.len();
    let joined = (0..rounds).try_fold(paths, |paths, _| {
        let found = ends::near_misses(&paths).into_iter().find_map(|(_, _, miss)| {
            let path = &paths[miss.runs_on];
            let k = path.iter().position(|&p| p == miss.turn)?;
            // The trunk runs from `at` to `turn`; the turning segment crosses it.
            let along_x = miss.at.1 == miss.turn.1;
            let crossing = |a: Point, b: Point| if along_x { a.0 == b.0 } else { a.1 == b.1 };
            let s = [k.checked_sub(1), (k + 1 < path.len()).then_some(k)].into_iter().flatten().find(|&s| crossing(path[s], path[s + 1]))?;
            let l = miss.runs_on;
            let before = standing(&paths, l, path);
            let to = if along_x { miss.at.0 } else { miss.at.1 };
            let moved = room.moved(path, l, s, !along_x, to).filter(|p| room.intrusions(l, p) <= room.intrusions(l, path) && standing(&paths, l, p) <= before)?;
            Some((l, moved))
        });
        match found {
            Some((l, path)) => {
                let mut next = paths;
                next[l] = path;
                Ok(next)
            }
            None => Err(paths),
        }
    });
    joined.unwrap_or_else(|paths| paths)
}
