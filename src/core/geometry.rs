// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Points, rectangles, sides and grid snapping.

pub type Point = (f64, f64);

/// All layout coordinates sit on this grid.
pub const GRID: f64 = 8.0;

pub fn snap(v: f64) -> f64 {
    (v / GRID).round() * GRID
}

pub fn snap_up(v: f64, step: f64) -> f64 {
    (v / step).ceil() * step
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn right(&self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
    pub fn center(&self) -> Point {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
    pub fn inflate(&self, by: f64) -> Rect {
        Rect { x: self.x - by, y: self.y - by, w: self.w + 2.0 * by, h: self.h + 2.0 * by }
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
}

/// A note's anchor line: one straight line between the nearest points of the
/// note and the element it annotates, diagonal when they share no span. It is
/// an annotation, not a flow, so it need not be orthogonal (Drawing
/// principles, Lines: note links may be diagonal). Facing sides that share a
/// span are joined across the middle of what they share.
pub fn note_link(note: Rect, target: Rect) -> [Point; 2] {
    let shared = |a0: f64, a1: f64, b0: f64, b1: f64| (a0.max(b0) < a1.min(b1)).then(|| (a0.max(b0) + a1.min(b1)) / 2.0);
    let facing = |a0: f64, a1: f64, b0: f64, b1: f64| if a1 <= b0 { (a1, b0) } else if b1 <= a0 { (a0, b1) } else { ((a0 + a1) / 2.0, (b0 + b1) / 2.0) };
    match (shared(note.y, note.bottom(), target.y, target.bottom()), shared(note.x, note.right(), target.x, target.right())) {
        (Some(y), _) => {
            let (a, b) = facing(note.x, note.right(), target.x, target.right());
            [(a, y), (b, y)]
        }
        (None, Some(x)) => {
            let (a, b) = facing(note.y, note.bottom(), target.y, target.bottom());
            [(x, a), (x, b)]
        }
        (None, None) => {
            let corner = (if note.right() <= target.x { note.right() } else { note.x }, if note.bottom() <= target.y { note.bottom() } else { note.y });
            [corner, (corner.0.clamp(target.x, target.right()), corner.1.clamp(target.y, target.bottom()))]
        }
    }
}

/// The flow of a layered layout.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Right,
    Down,
}

/// (main axis, cross axis) values of a point, or of a (w, h) size.
pub fn to_flow(direction: Direction, p: Point) -> (f64, f64) {
    match direction {
        Direction::Down => (p.1, p.0),
        Direction::Right => (p.0, p.1),
    }
}

/// A point, or a (w, h) size, from (main axis, cross axis) values.
pub fn from_flow(direction: Direction, main: f64, cross: f64) -> Point {
    match direction {
        Direction::Down => (cross, main),
        Direction::Right => (main, cross),
    }
}

/// The point on `side` of `r` at `coord` along it: y for left and right, x for
/// top and bottom.
pub fn side_point(r: Rect, side: Side, coord: f64) -> Point {
    match side {
        Side::Left => (r.x, coord),
        Side::Right => (r.right(), coord),
        Side::Top => (coord, r.y),
        Side::Bottom => (coord, r.bottom()),
    }
}

/// The position of `point` along `side`: y for left and right, x for top and
/// bottom.
pub fn along_side(point: Point, side: Side) -> f64 {
    if side.is_vertical_edge() { point.1 } else { point.0 }
}

/// The middle of `side` of `r`, whose top boundary lies at `top` (below a tab,
/// say), or the nearest place to it within `(lo, hi)`, the span the side offers.
pub fn side_middle(r: Rect, top: f64, side: Side, (lo, hi): (f64, f64)) -> f64 {
    let geometric = if side.is_vertical_edge() { (top + r.bottom()) / 2.0 } else { r.center().0 };
    geometric.clamp(lo, hi.max(lo))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    pub fn out(self) -> Point {
        match self {
            Side::Left => (-1.0, 0.0),
            Side::Right => (1.0, 0.0),
            Side::Top => (0.0, -1.0),
            Side::Bottom => (0.0, 1.0),
        }
    }
    pub fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
        }
    }
    /// True for the left and right sides, whose boundaries run vertically.
    pub fn is_vertical_edge(self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_axes_round_trip() {
        for direction in [Direction::Right, Direction::Down] {
            let (main, cross) = to_flow(direction, (3.0, 7.0));
            assert_eq!(from_flow(direction, main, cross), (3.0, 7.0));
        }
        assert_eq!(to_flow(Direction::Down, (3.0, 7.0)), (7.0, 3.0));
    }

    #[test]
    fn side_middle_starts_below_a_lowered_top() {
        let r = Rect { x: 0.0, y: 0.0, w: 100.0, h: 80.0 };
        assert_eq!(side_middle(r, 0.0, Side::Left, (16.0, 64.0)), 40.0);
        assert_eq!(side_middle(r, 24.0, Side::Right, (40.0, 64.0)), 52.0);
        assert_eq!(side_middle(r, 24.0, Side::Top, (60.0, 84.0)), 60.0);
        assert_eq!(side_point(r, Side::Bottom, 30.0), (30.0, 80.0));
        assert_eq!(along_side((30.0, 80.0), Side::Bottom), 30.0);
    }
}
