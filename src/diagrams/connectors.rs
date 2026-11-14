// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Component, package and state drawings as a `core::metrics::Scene` for the
//! label measure (`core::metrics::read_labels`): routed lines, the shapes and
//! outlines labels must keep off, and the text drawn, with the line each
//! connector label belongs to.

use crate::core::geometry::{Point, Rect};
use crate::core::metrics::{tip_box, Scene, Solid};
use crate::core::svg::Mark;

/// What a kind hands over to build its scene.
#[derive(Default)]
pub struct Drawn {
    /// Routed lines, then any drawn separately such as self-transition loops.
    pub lines: Vec<Vec<Point>>,
    /// Indexes into `lines` of lines that end in an arrowhead or a triangle,
    /// with its length and half width.
    pub heads: Vec<(usize, f64, f64)>,
    /// Leaf boxes and other opaque glyphs.
    pub solids: Vec<Rect>,
    /// Container outlines, which labels must not touch.
    pub outlines: Vec<Rect>,
    /// Text marks of one label and the line it belongs to. The lines of a
    /// wrapped label make one ink box.
    pub owned: Vec<(usize, Vec<Mark>)>,
    /// Text belonging to no line, such as port and container names.
    pub free: Vec<Mark>,
}

fn outline(r: Rect) -> Vec<Point> {
    vec![(r.x, r.y), (r.right(), r.y), (r.right(), r.bottom()), (r.x, r.bottom()), (r.x, r.y)]
}

pub fn scene(d: Drawn) -> Scene {
    let union = |a: Rect, b: Rect| {
        let (x, y) = (a.x.min(b.x), a.y.min(b.y));
        Rect { x, y, w: a.right().max(b.right()) - x, h: a.bottom().max(b.bottom()) - y }
    };
    let owned = d.owned.iter().filter_map(|(line, marks)| marks.iter().filter_map(Mark::ink).reduce(union).map(|ink| (ink, Some(*line))));
    let free = d.free.iter().filter_map(Mark::ink).map(|ink| (ink, None));
    let (labels, owners): (Vec<Rect>, Vec<Option<usize>>) = owned.chain(free).unzip();
    let marks = d.heads.iter().filter(|&&(i, _, _)| d.lines[i].len() >= 2).map(|&(i, back, half)| tip_box(&d.lines[i], back, half)).collect();
    Scene {
        solids: d.solids.into_iter().map(|rect| Solid { rect, across: false }).collect(),
        borders: d.outlines.into_iter().map(outline).collect(),
        lines: d.lines,
        labels,
        owners,
        marks,
        ..Scene::default()
    }
}
