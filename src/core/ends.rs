// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Measures of how lines meet their boxes and each other: ends off the middle
//! of their side, merging lines that join a few pixels apart instead of at one
//! point, parents not centred over their children, and arrowheads or start
//! decorations without a straight run to sit on. Pure functions over routed
//! output, used by the quality test and, for junctions, by the router.

// The figure and counts serve the quality test; the router uses `near_misses`.
#![cfg_attr(not(test), allow(dead_code))]

use super::geometry::{Direction, Point, Rect};

/// How far an end may sit from the middle of its side and still read as centred:
/// half a grid step, which snapping alone can cause.
pub const CENTRED: f64 = 4.0;
/// Two merging lines that turn off their shared run closer than this read as a
/// near miss rather than as two deliberate junctions.
pub const NEAR_MISS: f64 = 24.0;
/// The straight run a head or start decoration needs beyond its own length.
pub const HEAD_ROOM: f64 = 8.0;

const EPS: f64 = 0.5;

/// A routed drawing reduced to what the end measures need. Each component,
/// package and state adapter builds one.
#[derive(Clone, Debug)]
pub struct Figure {
    pub direction: Direction,
    pub lines: Vec<Vec<Point>>,
    /// Per line: the length of the decoration at its start (a circle-plus) and
    /// of the head at its end (an arrowhead or triangle), 0 for none.
    pub marks: Vec<(f64, f64)>,
    /// Boxes lines attach to, with the index of the box that contains each, or
    /// `None` at the top level.
    pub boxes: Vec<(Rect, Option<usize>)>,
    /// Points on box sides that hold no line end but take room there: ports and
    /// interface stubs.
    pub fixtures: Vec<Point>,
    /// Connections between boxes, as indexes into `boxes`, in flow order.
    pub links: Vec<(usize, usize)>,
    /// Decisions, as indexes into `boxes`: choices and junctions, which the flow
    /// should pass straight through (`core::flow`).
    pub pivots: Vec<usize>,
}

/// What the end measures count over one figure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Ends and fixtures alone on their side but off its middle.
    pub off_centre: usize,
    /// Pairs of merging lines that turn off their shared run a few pixels apart.
    pub near_misses: usize,
    /// Boxes with several children along the flow that are not centred over them.
    pub uncentred: usize,
    /// Heads and start decorations on a run shorter than their length plus `HEAD_ROOM`.
    pub cramped: usize,
}

impl std::ops::Add for Counts {
    type Output = Counts;
    fn add(self, o: Counts) -> Counts {
        Counts { off_centre: self.off_centre + o.off_centre, near_misses: self.near_misses + o.near_misses, uncentred: self.uncentred + o.uncentred, cramped: self.cramped + o.cramped }
    }
}

impl std::iter::Sum for Counts {
    fn sum<I: Iterator<Item = Counts>>(iter: I) -> Counts {
        iter.fold(Counts::default(), |a, b| a + b)
    }
}

/// The four sides of `r` as (fixed coordinate, span low, span high, vertical).
fn sides(r: &Rect) -> [(f64, f64, f64, bool); 4] {
    [(r.x, r.y, r.bottom(), true), (r.right(), r.y, r.bottom(), true), (r.y, r.x, r.right(), false), (r.bottom(), r.x, r.right(), false)]
}

/// Where `p` lies along a side, if it lies on it, corners excluded.
fn on_side(p: Point, (at, lo, hi, vertical): (f64, f64, f64, bool)) -> Option<f64> {
    let (fixed, along) = if vertical { (p.0, p.1) } else { (p.1, p.0) };
    ((fixed - at).abs() < EPS && along > lo + EPS && along < hi - EPS).then_some(along)
}

/// Ends and fixtures alone on a side of a box but more than `CENTRED` from its
/// middle, as (box, point). Several lines meeting at one point count once.
pub fn off_centre(boxes: &[Rect], lines: &[Vec<Point>], fixtures: &[Point]) -> Vec<(usize, Point)> {
    let mut points: Vec<Point> = lines.iter().filter(|l| l.len() >= 2).flat_map(|l| [l[0], l[l.len() - 1]]).chain(fixtures.iter().copied()).collect();
    points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    points.dedup_by(|a, b| (a.0 - b.0).abs() < EPS && (a.1 - b.1).abs() < EPS);
    boxes
        .iter()
        .enumerate()
        .flat_map(|(i, r)| {
            let points = &points;
            sides(r).into_iter().filter_map(move |side| {
                let held: Vec<(Point, f64)> = points.iter().filter_map(|&p| on_side(p, side).map(|along| (p, along))).collect();
                let middle = (side.1 + side.2) / 2.0;
                match held[..] {
                    [(p, along)] if (along - middle).abs() > CENTRED => Some((i, p)),
                    _ => None,
                }
            })
        })
        .collect()
}

fn unit(a: Point, b: Point) -> (i8, i8) {
    ((b.0 - a.0).signum() as i8 * ((b.0 - a.0).abs() > EPS) as i8, (b.1 - a.1).signum() as i8 * ((b.1 - a.1).abs() > EPS) as i8)
}

fn distance(a: Point, b: Point) -> f64 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

fn same(a: Point, b: Point) -> bool {
    distance(a, b) < EPS
}

/// Where two lines that leave a shared end together part, when one of them
/// runs on along the shared direction and turns within `NEAR_MISS`: the parting
/// point, where the other turns, and which line runs on (0 for `a`). `a` and `b`
/// both begin at the shared end. Lines that part at once, or where the line
/// running on ends straight, meet cleanly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Miss {
    pub at: Point,
    pub turn: Point,
    pub runs_on: usize,
}

pub fn near_miss(a: &[Point], b: &[Point]) -> Option<Miss> {
    let (mut ia, mut ib, mut ca, mut cb) = (0, 0, *a.first()?, *b.first()?);
    let mut trunk = None;
    loop {
        let (na, nb) = (*a.get(ia + 1)?, *b.get(ib + 1)?);
        let (da, db) = (unit(ca, na), unit(cb, nb));
        if da != db {
            let trunk = trunk?;
            let runs_on = |d: (i8, i8), n: Point, i: usize, line: &[Point]| (d == trunk && i + 2 < line.len()).then_some(n);
            let (turn, which) = runs_on(da, na, ia, a).map(|n| (n, 0)).or_else(|| runs_on(db, nb, ib, b).map(|n| (n, 1)))?;
            let gap = distance(ca, turn);
            return (gap > EPS && gap < NEAR_MISS).then_some(Miss { at: ca, turn, runs_on: which });
        }
        trunk = Some(da);
        let step = distance(ca, na).min(distance(cb, nb));
        let advance = |c: Point, d: (i8, i8)| (c.0 + d.0 as f64 * step, c.1 + d.1 as f64 * step);
        (ca, cb) = (advance(ca, da), advance(cb, db));
        if same(ca, na) {
            ia += 1;
            ca = na;
        }
        if same(cb, nb) {
            ib += 1;
            cb = nb;
        }
    }
}

/// Both lines oriented from an end they share, for each end they share.
fn shared_starts(a: &[Point], b: &[Point]) -> Vec<(Vec<Point>, Vec<Point>)> {
    let forms = |l: &[Point]| [l.to_vec(), l.iter().rev().copied().collect::<Vec<Point>>()];
    forms(a).into_iter().flat_map(|x| forms(b).into_iter().filter(|y| same(x[0], y[0])).map(|y| (x.clone(), y)).collect::<Vec<_>>()).collect()
}

/// Pairs of lines that share an end and part a few pixels apart, with where;
/// `Miss::runs_on` names the line that runs on, `i` or `j`.
pub fn near_misses(lines: &[Vec<Point>]) -> Vec<(usize, usize, Miss)> {
    let pairs = (0..lines.len()).flat_map(|i| (i + 1..lines.len()).map(move |j| (i, j)));
    pairs
        .filter(|&(i, j)| lines[i].len() >= 2 && lines[j].len() >= 2)
        .filter_map(|(i, j)| shared_starts(&lines[i], &lines[j]).into_iter().find_map(|(a, b)| near_miss(&a, &b)).map(|m| (i, j, Miss { runs_on: if m.runs_on == 0 { i } else { j }, ..m })))
        .collect()
}

fn median(sorted: &[f64]) -> f64 {
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 { sorted[middle] } else { (sorted[middle - 1] + sorted[middle]) / 2.0 }
}

/// Each box with two or more children: boxes in the same container, linked
/// from it, wholly further along the flow. Boxes are indexed like `rects`, and
/// `containers` names each box's container.
pub fn families(direction: Direction, rects: &[Rect], containers: &[Option<usize>], links: &[(usize, usize)]) -> Vec<(usize, Vec<usize>)> {
    let beyond = |p: &Rect, c: &Rect| if direction == Direction::Right { c.x >= p.right() } else { c.y >= p.bottom() };
    (0..rects.len())
        .filter_map(|p| {
            let mut children: Vec<usize> = links.iter().filter(|&&(a, b)| a == p && b != p && containers[b] == containers[p] && beyond(&rects[p], &rects[b])).map(|&(_, b)| b).collect();
            children.sort_unstable();
            children.dedup();
            (children.len() >= 2).then_some((p, children))
        })
        .collect()
}

/// Where a parent's centre across the flow could go to sit centred over its
/// children: the median and the mean of their centres.
pub fn centres_over(direction: Direction, rects: &[Rect], children: &[usize]) -> [f64; 2] {
    let across = |r: &Rect| if direction == Direction::Right { r.center().1 } else { r.center().0 };
    let mut centres: Vec<f64> = children.iter().map(|&c| across(&rects[c])).collect();
    centres.sort_by(f64::total_cmp);
    [median(&centres), centres.iter().sum::<f64>() / centres.len() as f64]
}

/// Whether box `p` sits within `CENTRED` of the median or the mean of its
/// children's centres across the flow.
pub fn centred_over(direction: Direction, rects: &[Rect], p: usize, children: &[usize]) -> bool {
    let own = if direction == Direction::Right { rects[p].center().1 } else { rects[p].center().0 };
    centres_over(direction, rects, children).iter().any(|t| (own - t).abs() <= CENTRED)
}

/// Boxes with several children (see `families`) not centred over them. A
/// decision (`pivots`), and a box in line with one of its children, keep a
/// column instead, which ranks higher (Drawing principles, Flow: keep a
/// column), so they do not count.
pub fn uncentred_parents(direction: Direction, boxes: &[(Rect, Option<usize>)], links: &[(usize, usize)], pivots: &[usize]) -> Vec<usize> {
    let rects: Vec<Rect> = boxes.iter().map(|b| b.0).collect();
    let containers: Vec<Option<usize>> = boxes.iter().map(|b| b.1).collect();
    let across = |r: &Rect| if direction == Direction::Right { r.center().1 } else { r.center().0 };
    let in_column = |p: usize, children: &[usize]| children.iter().any(|&c| (across(&rects[c]) - across(&rects[p])).abs() <= CENTRED);
    families(direction, &rects, &containers, links)
        .into_iter()
        .filter(|(p, children)| !pivots.contains(p) && !in_column(*p, children) && !centred_over(direction, &rects, *p, children))
        .map(|(p, _)| p)
        .collect()
}

/// Heads and start decorations whose run is shorter than the decoration plus
/// `HEAD_ROOM`, as (line, true for the end).
pub fn cramped(lines: &[Vec<Point>], marks: &[(f64, f64)]) -> Vec<(usize, bool)> {
    lines
        .iter()
        .zip(marks)
        .enumerate()
        .filter(|(_, (l, _))| l.len() >= 2)
        .flat_map(|(i, (l, &(start, end)))| {
            let first = distance(l[0], l[1]);
            let last = distance(l[l.len() - 2], l[l.len() - 1]);
            [(start > 0.0 && first < start + HEAD_ROOM - EPS).then_some((i, false)), (end > 0.0 && last < end + HEAD_ROOM - EPS).then_some((i, true))].into_iter().flatten()
        })
        .collect()
}

/// Line ends that double back into their box (Drawing principles, Lines: never
/// double back into a box): the last three runs into the end make a Z (the
/// first and last run the same way), and the run across before the last one
/// starts beyond the box's side, at least 16px out, so the line came down
/// beside the box and turned back into another side. A U, which climbs over
/// something beside the box, is a detour, not counted. Returns (line, end
/// point). Used by the side tests.
#[cfg(test)]
pub fn doubled_back(boxes: &[Rect], lines: &[Vec<Point>]) -> Vec<(usize, Point)> {
    let back = |tail: &[Point]| {
        let [q0, q1, q2, q3] = [tail[0], tail[1], tail[2], tail[3]];
        let vertical = |a: Point, b: Point| a.0 == b.0;
        let same_way = (q1.0 - q0.0) * (q3.0 - q2.0) + (q1.1 - q0.1) * (q3.1 - q2.1) > 0.0;
        if vertical(q0, q1) != vertical(q2, q3) || vertical(q1, q2) == vertical(q2, q3) || !same_way {
            return false;
        }
        boxes.iter().any(|r| {
            let on_top_bottom = (q3.1 == r.y || q3.1 == r.bottom()) && q3.0 > r.x && q3.0 < r.right();
            let on_left_right = (q3.0 == r.x || q3.0 == r.right()) && q3.1 > r.y && q3.1 < r.bottom();
            (on_top_bottom && vertical(q2, q3) && (q1.0 >= r.right() + 16.0 || q1.0 <= r.x - 16.0)) || (on_left_right && !vertical(q2, q3) && (q1.1 >= r.bottom() + 16.0 || q1.1 <= r.y - 16.0))
        })
    };
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.len() >= 5)
        .flat_map(|(i, l)| {
            let reversed: Vec<Point> = l.iter().rev().copied().collect();
            let at_end = back(&l[l.len() - 4..]).then_some((i, l[l.len() - 1]));
            let at_start = back(&reversed[reversed.len() - 4..]).then_some((i, l[0]));
            at_end.into_iter().chain(at_start)
        })
        .collect()
}

/// All four counts for one figure.
pub fn count(f: &Figure) -> Counts {
    let rects: Vec<Rect> = f.boxes.iter().map(|b| b.0).collect();
    Counts {
        off_centre: off_centre(&rects, &f.lines, &f.fixtures).len(),
        near_misses: near_misses(&f.lines).len(),
        uncentred: uncentred_parents(f.direction, &f.boxes, &f.links, &f.pivots).len(),
        cramped: cramped(&f.lines, &f.marks).len(),
    }
}

/// One line per flaw counted in `f`, for finding them in a drawing.
pub fn explain(f: &Figure) -> Vec<String> {
    let rects: Vec<Rect> = f.boxes.iter().map(|b| b.0).collect();
    let off = off_centre(&rects, &f.lines, &f.fixtures).into_iter().map(|(b, p)| format!("off centre: ({}, {}) on box {b} {:?}", p.0, p.1, rects[b]));
    let near = near_misses(&f.lines).into_iter().map(|(i, j, m)| format!("near miss: lines {i} and {j} part at ({}, {}); line {} turns at ({}, {})", m.at.0, m.at.1, m.runs_on, m.turn.0, m.turn.1));
    let parents = uncentred_parents(f.direction, &f.boxes, &f.links, &f.pivots).into_iter().map(|b| format!("uncentred parent: box {b} {:?}", rects[b]));
    let heads = cramped(&f.lines, &f.marks).into_iter().map(|(i, end)| format!("cramped {}: line {i} {:?}", if end { "head" } else { "start" }, f.lines[i]));
    off.chain(near).chain(parents).chain(heads).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn a_lone_end_off_the_middle_counts_but_a_shared_side_does_not() {
        let b = boxed(0.0, 0.0, 100.0, 40.0);
        assert_eq!(off_centre(&[b], &[vec![(100.0, 20.0), (200.0, 20.0)]], &[]).len(), 0);
        assert_eq!(off_centre(&[b], &[vec![(100.0, 12.0), (200.0, 12.0)]], &[]).len(), 1);
        assert_eq!(off_centre(&[b], &[vec![(100.0, 12.0), (200.0, 12.0)]], &[(100.0, 30.0)]).len(), 0);
    }

    #[test]
    fn lines_that_part_a_step_apart_are_a_near_miss() {
        let bus = (300.0, 100.0);
        let upper = vec![(100.0, 50.0), (200.0, 50.0), (200.0, 100.0), bus];
        let lower = vec![(100.0, 150.0), (192.0, 150.0), (192.0, 100.0), bus];
        assert_eq!(near_misses(&[upper.clone(), lower]).len(), 1);
        let joined = vec![(100.0, 150.0), (200.0, 150.0), (200.0, 100.0), bus];
        assert_eq!(near_misses(&[upper.clone(), joined]).len(), 0, "one junction");
        let straight = vec![(100.0, 100.0), bus];
        assert_eq!(near_misses(&[upper, straight]).len(), 0, "a branch onto a straight line");
    }

    #[test]
    fn a_parent_off_its_children_counts() {
        let parent = |x: f64| (boxed(x, 0.0, 100.0, 40.0), None);
        let children = [(boxed(0.0, 100.0, 100.0, 40.0), None), (boxed(200.0, 100.0, 100.0, 40.0), None)];
        let links = [(0, 1), (0, 2)];
        assert_eq!(uncentred_parents(Direction::Down, &[parent(100.0), children[0], children[1]], &links, &[]).len(), 0);
        assert_eq!(uncentred_parents(Direction::Down, &[parent(40.0), children[0], children[1]], &links, &[]).len(), 1);
        assert_eq!(uncentred_parents(Direction::Down, &[parent(0.0), children[0], children[1]], &links, &[]).len(), 0, "in a column with one child");
    }

    #[test]
    fn a_head_needs_a_run_longer_than_itself() {
        let line = vec![(0.0, 0.0), (0.0, 50.0), (40.0, 50.0), (40.0, 60.0)];
        assert_eq!(cramped(&[line.clone()], &[(0.0, 10.0)]), vec![(0, true)]);
        assert_eq!(cramped(&[line], &[(12.0, 0.0)]).len(), 0);
    }
}
