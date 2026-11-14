// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Generic orthogonal routing: A* over a sparse grid with costs for length,
//! bends, crossings and shared lanes, plus helpers that distribute connection
//! points along box boundaries. Callers describe everything as rectangles and
//! terminals, and pass per-connection obstacles, so they decide what may be crossed.
//! The stages live in submodules: `grid` (the search and the `route` pipeline),
//! `nudge`, `l_over_z` and `slots`. This module holds the problem types.

mod grid;
mod l_over_z;
mod nudge;
mod slots;
#[cfg(test)]
mod tests;

use super::geometry::{snap, Point, Rect, Side, GRID};

pub use grid::route;
pub use nudge::SLIDE_GAP;
pub use slots::{facing, lone, settle_slots, slide_ranges, straighten, Slot};

const STEM: f64 = 16.0;

/// One end of a connection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Terminal {
    /// Where the drawn line ends.
    pub point: Point,
    /// Where the search starts or ends, a short way out from `point`.
    pub stem: Point,
    /// The direction the line leaves `point`.
    pub outward: Side,
    /// The span along its side that the end may slide within when nudging
    /// straightens a line, or `None` to keep it where it is.
    pub slide: Option<(f64, f64)>,
    /// The box the end sits on, when the end may move to another side of it
    /// to turn a Z-shaped line into an L (`prefer_l`).
    pub body: Option<Body>,
}

/// A plain box an end may move around: its outline, how far from a corner an
/// end may attach, and which sides may take an end, indexed like `Side`
/// (left, right, top, bottom).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub rect: Rect,
    pub inset: f64,
    pub sides: [bool; 4],
    /// How far down from the top the left and right sides take no end: a
    /// header whose divider meets those sides.
    pub head: f64,
}

impl Body {
    /// A box whose every side may take an end.
    #[cfg(test)]
    pub fn all(rect: Rect, inset: f64) -> Body {
        Body { rect, inset, sides: [true; 4], head: 0.0 }
    }

    fn allows(&self, side: Side) -> bool {
        let index = match side {
            Side::Left => 0,
            Side::Right => 1,
            Side::Top => 2,
            Side::Bottom => 3,
        };
        self.sides[index]
    }
}

impl Terminal {
    /// A terminal on a boundary at `base`, drawn `lift` beyond it.
    pub fn at(base: Point, outward: Side, lift: f64) -> Terminal {
        let (dx, dy) = outward.out();
        let stem = if outward.is_vertical_edge() { (snap(base.0 + dx * STEM), base.1) } else { (base.0, snap(base.1 + dy * STEM)) };
        Terminal { point: (base.0 + dx * lift, base.1 + dy * lift), stem, outward, slide: None, body: None }
    }

    /// The same terminal, free to slide between `lo` and `hi` along its side.
    pub fn sliding(self, lo: f64, hi: f64) -> Terminal {
        Terminal { slide: (lo <= hi).then_some((lo, hi)), ..self }
    }

    /// The same terminal, with its stem at least `run` from its point, on the
    /// grid: a head or start decoration then sits on a straight run (Drawing
    /// principles, Lines: leave room for the head). Nudging keeps every stem.
    pub fn reaching(self, run: f64) -> Terminal {
        let (dx, dy) = self.outward.out();
        let out = |p: f64, s: f64, d: f64| if d > 0.0 { s.max(((p + run) / GRID).ceil() * GRID) } else if d < 0.0 { s.min(((p - run) / GRID).floor() * GRID) } else { s };
        Terminal { stem: (out(self.point.0, self.stem.0, dx), out(self.point.1, self.stem.1, dy)), ..self }
    }

    /// How far the stem reaches from the point: the shortest first or last run.
    fn reach(&self) -> f64 {
        manhattan(self.point, self.stem)
    }

    /// The same terminal, on `body`, which it may move around (`prefer_l`).
    pub fn on(self, body: Body) -> Terminal {
        Terminal { body: Some(body), ..self }
    }
}

/// Soft regions: travel along one is allowed but expensive.
#[derive(Clone, Debug, Default)]
pub struct Strips {
    /// Penalises segments running in x.
    pub horizontal: Vec<Rect>,
    /// Penalises segments running in y.
    pub vertical: Vec<Rect>,
}

/// Soft bands beside each shape, from its outline out to its clearance: a line
/// may run there only at a cost, so long lines keep their distance (Drawing
/// principles, Lines: give long lines room). A band only penalises running
/// along it, so lines still cross it to reach a shape.
pub fn room_strips(shapes: Vec<(Rect, f64)>) -> Strips {
    // From the shape's outline, under its wall, so a line along the wall's
    // edge counts as running in the band.
    let bands = |(r, clear): (Rect, f64)| {
        let (tall, wide) = (r.h + 2.0 * clear, r.w + 2.0 * clear);
        let vertical = [Rect { x: r.x - clear, y: r.y - clear, w: clear, h: tall }, Rect { x: r.right(), y: r.y - clear, w: clear, h: tall }];
        let horizontal = [Rect { x: r.x - clear, y: r.y - clear, w: wide, h: clear }, Rect { x: r.x - clear, y: r.bottom(), w: wide, h: clear }];
        (vertical, horizontal)
    };
    let all: Vec<([Rect; 2], [Rect; 2])> = shapes.into_iter().map(bands).collect();
    Strips { vertical: all.iter().flat_map(|b| b.0).collect(), horizontal: all.iter().flat_map(|b| b.1).collect() }
}

/// Strips along the boundaries of containers a line passes through, each given
/// as its box and the y of its top boundary (below a tab, say): the line may
/// cross them, but running along one looks like a drawing error.
pub fn boundary_strips(containers: impl IntoIterator<Item = (Rect, f64)>) -> Strips {
    let boxes: Vec<(Rect, f64)> = containers.into_iter().collect();
    let vertical = boxes.iter().flat_map(|&(r, _)| [r.x, r.right()].map(|x| Rect { x: x - 10.0, y: r.y - 10.0, w: 20.0, h: r.h + 20.0 }));
    let horizontal = boxes.iter().flat_map(|&(r, top)| [top, r.bottom()].map(|y| Rect { x: r.x - 10.0, y: y - 10.0, w: r.w + 20.0, h: 20.0 }));
    Strips { horizontal: horizontal.collect(), vertical: vertical.collect() }
}

#[derive(Clone, Debug)]
pub struct Connection {
    pub from: Terminal,
    pub to: Terminal,
    pub obstacles: Vec<Rect>,
    pub strips: Strips,
}

#[derive(Clone, Debug, Default)]
pub struct Problem {
    /// Item boxes. Search lines run just outside each one.
    pub bounds: Vec<Rect>,
    /// Blocking outlines. Channel lanes run between their edges.
    pub walls: Vec<Rect>,
    pub connections: Vec<Connection>,
    /// Ends alone on their side sit at its middle: after an L moves an end
    /// away, the end left behind returns to the middle, or the L is dropped.
    pub centred: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    /// One polyline per connection, in input order.
    pub paths: Vec<Vec<Point>>,
    /// Connections with no clear route, which have a direct line instead.
    pub failed: Vec<usize>,
}

fn manhattan(a: Point, b: Point) -> f64 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}
