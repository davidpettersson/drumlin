// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Collision-aware label placement. A `Placer` is passed by value: each operation
//! consumes it and returns the placer that also knows about the new label.

use super::geometry::{Point, Rect};
use super::svg::Anchor;

#[derive(Clone, Debug, Default)]
pub struct Placer {
    occupied: Vec<Rect>,
    /// Names already placed, which later glyphs must stay off.
    names: Vec<Rect>,
}

/// Where to draw a label.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub anchor: Anchor,
}

struct Candidate {
    score: f64,
    placement: Placement,
    rect: Rect,
}

/// Which sides of a vertical segment a label may use.
#[derive(Clone, Copy)]
enum Beside {
    Right,
    Either,
}

fn candidates_on(a: Point, b: Point, w: f64, occupied: &[Rect], beside: Beside) -> impl Iterator<Item = Candidate> + '_ {
    let h = 14.0;
    let length = (a.0 - b.0).abs() + (a.1 - b.1).abs();
    let horizontal = a.1 == b.1;
    let sides: &[bool] = match (horizontal, beside) {
        (false, Beside::Either) => &[false, true],
        _ => &[false],
    };
    [0.5, 0.35, 0.65, 0.2, 0.8].into_iter().flat_map(move |fraction: f64| sides.iter().map(move |&left| (fraction, left))).map(move |(fraction, left)| {
        let c = (a.0 + (b.0 - a.0) * fraction, a.1 + (b.1 - a.1) * fraction);
        let (placement, rect) = if horizontal {
            (Placement { x: c.0, y: c.1 - 7.0, anchor: Anchor::Middle }, Rect { x: c.0 - w / 2.0, y: c.1 - 5.0 - h, w, h })
        } else if left {
            (Placement { x: c.0 - 8.0, y: c.1 + 4.0, anchor: Anchor::End }, Rect { x: c.0 - 6.0 - w, y: c.1 - h / 2.0, w, h })
        } else {
            (Placement { x: c.0 + 8.0, y: c.1 + 4.0, anchor: Anchor::Start }, Rect { x: c.0 + 6.0, y: c.1 - h / 2.0, w, h })
        };
        let hits = occupied.iter().filter(|o| o.intersects(&rect)).count() as f64;
        let tight = if length < w + 16.0 && horizontal { 200.0 } else { 0.0 };
        let centred = (fraction - 0.5).abs() * 40.0;
        let vertical = if horizontal { 0.0 } else { 30.0 };
        Candidate { score: hits * 1000.0 + tight + centred - length.min(240.0) * 0.5 + vertical, placement, rect }
    })
}

impl Placer {
    pub fn occupy(mut self, rect: Rect) -> Placer {
        self.occupied.push(rect);
        self
    }

    pub fn occupy_all(mut self, rects: impl IntoIterator<Item = Rect>) -> Placer {
        self.occupied.extend(rects);
        self
    }

    /// Records a name that later glyphs must stay off.
    pub fn name(mut self, rect: Rect) -> Placer {
        self.names.push(rect);
        self
    }

    /// Reserves a thin box around each segment of a polyline.
    pub fn avoid_polyline(self, points: &[Point]) -> Placer {
        self.occupy_all(points.windows(2).map(|pair| {
            let (a, b) = (pair[0], pair[1]);
            Rect { x: a.0.min(b.0) - 1.0, y: a.1.min(b.1) - 1.0, w: (a.0 - b.0).abs() + 2.0, h: (a.1 - b.1).abs() + 2.0 }
        }))
    }

    pub fn is_occupied(&self, rect: &Rect) -> bool {
        self.occupied.iter().any(|o| o.intersects(rect))
    }

    pub fn is_on_name(&self, rect: &Rect) -> bool {
        self.names.iter().any(|o| o.intersects(rect))
    }

    /// Places a label of width `w` beside the polyline where it collides least.
    /// Earlier candidates win ties, which keeps the output deterministic.
    pub fn place(self, points: &[Point], w: f64) -> (Placer, Placement) {
        self.place_at(points, w, Beside::Right)
    }

    /// Like `place`, but a label may also sit left of a vertical segment.
    pub fn place_around(self, points: &[Point], w: f64) -> (Placer, Placement) {
        self.place_at(points, w, Beside::Either)
    }

    fn place_at(self, points: &[Point], w: f64, beside: Beside) -> (Placer, Placement) {
        let best = points
            .windows(2)
            .flat_map(|pair| candidates_on(pair[0], pair[1], w, &self.occupied, beside))
            .fold(None, |best: Option<Candidate>, candidate| match best {
                Some(best) if best.score <= candidate.score => Some(best),
                _ => Some(candidate),
            })
            .expect("a label needs a polyline with at least one segment");
        (self.occupy(best.rect), best.placement)
    }
}
