// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Measures of how a drawing flows: chains that leave their column, lines that
//! run too close beside a shape, and decisions the flow does not pass straight
//! through. Pure functions over the same `Figure` the end measures read, used by
//! the quality test (Drawing principles, Flow: keep a column; Lines: give long
//! lines room, flow passes straight through a decision).

#![cfg_attr(not(test), allow(dead_code))]

use super::ends::{Figure, CENTRED};
use super::geometry::{Direction, Point, Rect, Side};

/// The clear gap a line keeps beside a shape: two grid steps.
pub const CLEARANCE: f64 = 16.0;
/// The clear gap beside a container, such as a composite state: three steps.
pub const CONTAINER_CLEARANCE: f64 = 24.0;
/// A line runs beside a shape when it is parallel to a side for longer than this.
const BESIDE: f64 = 8.0;
const EPS: f64 = 0.5;

/// What the flow measures count over one figure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flow {
    /// Boxes whose flow continues into chain children, none of them in line.
    pub off_column: usize,
    /// The summed centre offsets of those boxes from their nearest chain child,
    /// in pixels.
    pub drift: usize,
    /// Pairs of a line and a shape it runs beside closer than the clearance.
    pub crowded: usize,
    /// Decisions where no outgoing line leaves opposite an incoming one.
    pub turned: usize,
}

impl std::ops::Add for Flow {
    type Output = Flow;
    fn add(self, o: Flow) -> Flow {
        Flow { off_column: self.off_column + o.off_column, drift: self.drift + o.drift, crowded: self.crowded + o.crowded, turned: self.turned + o.turned }
    }
}

impl std::iter::Sum for Flow {
    fn sum<I: Iterator<Item = Flow>>(iter: I) -> Flow {
        iter.fold(Flow::default(), |a, b| a + b)
    }
}

fn across(direction: Direction, r: &Rect) -> f64 {
    if direction == Direction::Right { r.center().1 } else { r.center().0 }
}

fn beyond(direction: Direction, p: &Rect, c: &Rect) -> bool {
    if direction == Direction::Right { c.x >= p.right() - EPS } else { c.y >= p.bottom() - EPS }
}

/// Forward links within one container: `(parent, child)` pairs, child wholly
/// further along the flow, without repeats.
fn forward(f: &Figure) -> Vec<(usize, usize)> {
    let mut found: Vec<(usize, usize)> = f
        .links
        .iter()
        .copied()
        .filter(|&(a, b)| a != b && f.boxes[a].1 == f.boxes[b].1 && f.boxes[a].0.w > 0.0 && f.boxes[b].0.w > 0.0 && beyond(f.direction, &f.boxes[a].0, &f.boxes[b].0))
        .collect();
    found.sort_unstable();
    found.dedup();
    found
}

/// Boxes whose flow should continue straight into one of their chain children
/// but does not, with the offset to the nearest. A link is a chain link when
/// its parent has one forward child or its child one forward parent; every
/// forward link of a decision is.
pub fn off_column(f: &Figure) -> Vec<(usize, f64)> {
    let links = forward(f);
    let children = |p: usize| links.iter().filter(move |&&(a, _)| a == p).map(|&(_, b)| b);
    let parents = |c: usize| links.iter().filter(move |&&(_, b)| b == c).count();
    let mut sources: Vec<usize> = links.iter().map(|&(a, _)| a).collect();
    sources.dedup();
    sources
        .into_iter()
        .filter_map(|p| {
            let pivot = f.pivots.contains(&p);
            let single = children(p).count() == 1;
            let chain: Vec<usize> = children(p).filter(|&c| pivot || single || parents(c) == 1).collect();
            let own = across(f.direction, &f.boxes[p].0);
            let nearest = chain.iter().map(|&c| (across(f.direction, &f.boxes[c].0) - own).abs()).fold(f64::MAX, f64::min);
            (!chain.is_empty() && nearest > CENTRED).then_some((p, nearest))
        })
        .collect()
}

/// Pairs `(line, box)` where a line runs beside a side of a box that is not one
/// of its ends, inside or outside it, closer than the clearance.
pub fn crowded(f: &Figure) -> Vec<(usize, usize)> {
    let containers: Vec<bool> = (0..f.boxes.len()).map(|k| f.boxes.iter().any(|b| b.1 == Some(k))).collect();
    f.lines
        .iter()
        .enumerate()
        .flat_map(|(i, line)| {
            let (a, b) = f.links[i];
            let containers = &containers;
            (0..f.boxes.len()).filter(move |&k| k != a && k != b && f.boxes[k].0.w > 0.0).filter_map(move |k| {
                let r = f.boxes[k].0;
                let limit = if containers[k] { CONTAINER_CLEARANCE } else { CLEARANCE };
                let close = line.windows(2).any(|w| {
                    let vertical = (w[0].0 - w[1].0).abs() < EPS;
                    let horizontal = (w[0].1 - w[1].1).abs() < EPS;
                    let (at, lo, hi, sides, span) = match (vertical, horizontal) {
                        (true, false) => (w[0].0, w[0].1.min(w[1].1), w[0].1.max(w[1].1), [r.x, r.right()], (r.y, r.bottom())),
                        (false, true) => (w[0].1, w[0].0.min(w[1].0), w[0].0.max(w[1].0), [r.y, r.bottom()], (r.x, r.right())),
                        _ => return false,
                    };
                    let overlap = hi.min(span.1) - lo.max(span.0);
                    overlap > BESIDE && sides.iter().any(|&s| (at - s).abs() < limit - EPS)
                });
                close.then_some((i, k))
            })
        })
        .collect()
}

/// The side of `r` that `p` lies on, the nearest one.
fn side_of(r: &Rect, p: Point) -> Side {
    let distances = [((p.0 - r.x).abs(), Side::Left), ((p.0 - r.right()).abs(), Side::Right), ((p.1 - r.y).abs(), Side::Top), ((p.1 - r.bottom()).abs(), Side::Bottom)];
    distances.into_iter().min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, s)| s).unwrap()
}

/// Whether the flow passes straight through a decision: some line leaves on the
/// side opposite where some line arrives. `None` without both.
pub fn passes(r: &Rect, arriving: &[Point], leaving: &[Point]) -> Option<bool> {
    if arriving.is_empty() || leaving.is_empty() {
        return None;
    }
    Some(arriving.iter().any(|&a| leaving.iter().any(|&l| side_of(r, l) == side_of(r, a).opposite())))
}

/// Decisions (`Figure::pivots`) the flow turns in instead of passing through.
pub fn turned(f: &Figure) -> Vec<usize> {
    f.pivots
        .iter()
        .copied()
        .filter(|&p| {
            let ends = |incoming: bool| -> Vec<Point> {
                f.links.iter().zip(&f.lines).filter(|((a, b), l)| l.len() >= 2 && if incoming { *b == p } else { *a == p }).map(|(_, l)| if incoming { l[l.len() - 1] } else { l[0] }).collect()
            };
            passes(&f.boxes[p].0, &ends(true), &ends(false)) == Some(false)
        })
        .collect()
}

/// All flow counts for one figure.
pub fn flow(f: &Figure) -> Flow {
    let off = off_column(f);
    Flow { off_column: off.len(), drift: off.iter().map(|(_, d)| d.round() as usize).sum(), crowded: crowded(f).len(), turned: turned(f).len() }
}

/// One line per flaw counted in `f`.
pub fn explain(f: &Figure) -> Vec<String> {
    let off = off_column(f).into_iter().map(|(b, d)| format!("off column: box {b} {:?}, {d}px from its nearest chain child", f.boxes[b].0));
    let tight = crowded(f).into_iter().map(|(i, k)| format!("crowded: line {i} {:?} beside box {k} {:?}", f.lines[i], f.boxes[k].0));
    let turns = turned(f).into_iter().map(|p| format!("turned: decision {p} {:?}", f.boxes[p].0));
    off.chain(tight).chain(turns).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(x: f64, y: f64, w: f64, h: f64) -> (Rect, Option<usize>) {
        (Rect { x, y, w, h }, None)
    }

    fn figure(boxes: Vec<(Rect, Option<usize>)>, lines: Vec<Vec<Point>>, links: Vec<(usize, usize)>, pivots: Vec<usize>) -> Figure {
        Figure { direction: Direction::Down, marks: vec![(0.0, 0.0); lines.len()], lines, boxes, fixtures: Vec::new(), links, pivots }
    }

    #[test]
    fn a_chain_off_its_column_counts() {
        let lined = figure(vec![boxed(0.0, 0.0, 100.0, 40.0), boxed(0.0, 100.0, 100.0, 40.0)], vec![], vec![(0, 1)], vec![]);
        assert_eq!(flow(&lined).off_column, 0);
        let off = figure(vec![boxed(0.0, 0.0, 100.0, 40.0), boxed(40.0, 100.0, 100.0, 40.0)], vec![], vec![(0, 1)], vec![]);
        assert_eq!(off_column(&off), vec![(0, 40.0)]);
    }

    #[test]
    fn a_line_close_beside_a_shape_counts() {
        let boxes = vec![boxed(0.0, 0.0, 40.0, 40.0), boxed(0.0, 200.0, 40.0, 40.0), boxed(100.0, 80.0, 100.0, 40.0)];
        let near = figure(boxes.clone(), vec![vec![(20.0, 40.0), (20.0, 60.0), (90.0, 60.0), (90.0, 180.0), (20.0, 180.0), (20.0, 200.0)]], vec![(0, 1)], vec![]);
        assert_eq!(crowded(&near), vec![(0, 2)]);
        let clear = figure(boxes, vec![vec![(20.0, 40.0), (20.0, 200.0)]], vec![(0, 1)], vec![]);
        assert!(crowded(&clear).is_empty());
    }

    #[test]
    fn a_decision_passes_when_a_line_leaves_opposite() {
        let r = Rect { x: 0.0, y: 0.0, w: 24.0, h: 24.0 };
        assert_eq!(passes(&r, &[(12.0, 0.0)], &[(12.0, 24.0), (24.0, 12.0)]), Some(true));
        assert_eq!(passes(&r, &[(12.0, 0.0)], &[(0.0, 12.0), (24.0, 12.0)]), Some(false));
    }
}
