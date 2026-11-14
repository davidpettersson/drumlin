// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! A state machine drawing as the quality test measures it: the routed lines, its size,
//! a `core::metrics::Scene` for the label measure (built by `hybrid::scene`)
//! and a `core::ends::Figure` for the end measures, drawn with a `chosen`
//! engine or the source's.

use super::{drawing, hybrid, model, render, routing};
use crate::diagrams::engine::Engine;
use crate::syntax::Res;

/// The routed lines, for layout quality measurements.
pub fn lines(source: &str, chosen: Option<Engine>) -> Res<Vec<Vec<crate::core::geometry::Point>>> {
    let (_, drawn) = drawing(source, chosen)?;
    Ok(drawn.routes.edges)
}

/// The size measures: the routed lines' length and the drawn document's frame.
pub fn size(source: &str, chosen: Option<Engine>) -> Res<crate::core::metrics::Size> {
    let (d, drawn) = drawing(source, chosen)?;
    let frame = crate::core::svg::frame(&render::render(&d, &drawn.geometry, &drawn.routes, &drawn.placed)).expect("a document has a frame");
    Ok(crate::core::metrics::size(&drawn.routes.edges, frame))
}

/// The drawing as a scene for the label measure: each transition label belongs
/// to its line, and a self-transition's label to its loop.
pub fn label_scene(source: &str, chosen: Option<Engine>) -> Res<crate::core::metrics::Scene> {
    let (diagram, drawn) = drawing(source, chosen)?;
    Ok(hybrid::scene(&diagram, &drawn))
}

/// The drawing as a figure for the end measures (`core::ends`). Note links and
/// self-transitions are left out. Regions take no ends, so they get an empty box.
pub fn figure(source: &str, chosen: Option<Engine>) -> Res<crate::core::ends::Figure> {
    use model::{Kind, Line};
    let (d, drawn) = drawing(source, chosen)?;
    let kept: Vec<usize> = (0..d.edges.len()).filter(|&e| d.edges[e].line == Line::Transition && d.edges[e].from != d.edges[e].to).collect();
    let rect = |n: usize| match d.nodes[n].kind {
        Kind::Root | Kind::Region => Default::default(),
        _ => routing::visual(&d, &drawn.geometry, n),
    };
    Ok(crate::core::ends::Figure {
        direction: d.direction,
        lines: kept.iter().map(|&e| drawn.routes.edges[e].clone()).collect(),
        marks: kept.iter().map(|_| (0.0, 10.0)).collect(),
        boxes: (0..d.nodes.len()).map(|n| (rect(n), d.nodes[n].parent)).collect(),
        fixtures: Vec::new(),
        links: kept.iter().map(|&e| (d.edges[e].from, d.edges[e].to)).collect(),
        pivots: (0..d.nodes.len()).filter(|&n| matches!(d.nodes[n].kind, Kind::Choice | Kind::Junction)).collect(),
    })
}
