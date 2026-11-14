// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! A package drawing as the quality test measures it: the routed lines, its
//! size, a `core::metrics::Scene` for the label measure and a `core::ends::Figure`
//! for the end measures, drawn with a `chosen` engine or the source's.

use super::{drawing, layout, model, render};
use crate::diagrams::engine::Engine;
use crate::syntax::Res;

/// The routed lines, for layout quality measurements.
pub fn lines(source: &str, chosen: Option<Engine>) -> Res<Vec<Vec<crate::core::geometry::Point>>> {
    let (_, _, routes) = drawing(source, chosen)?;
    Ok(routes.edges)
}

/// The size measures: the routed lines' length and the drawn document's frame.
pub fn size(source: &str, chosen: Option<Engine>) -> Res<crate::core::metrics::Size> {
    let (d, g, r) = drawing(source, chosen)?;
    let frame = crate::core::svg::frame(&render::render(&d, &g, &r)).expect("a document has a frame");
    Ok(crate::core::metrics::size(&r.edges, frame))
}

/// The drawing as a scene for the label measure.
pub fn label_scene(source: &str, chosen: Option<Engine>) -> Res<crate::core::metrics::Scene> {
    use crate::core::geometry::Rect;
    let (d, g, r) = drawing(source, chosen)?;
    let labels = render::edge_labels(&d, &g, &r);
    let leaf = |n: usize| n > 0 && d.nodes[n].children.is_empty();
    let containers: Vec<usize> = (1..d.nodes.len()).filter(|&n| !leaf(n)).collect();
    let tab = |n: usize| layout::tab_size(&d.nodes[n]);
    let tabs = containers.iter().map(|&n| Rect { x: g.rects[n].x, y: g.rects[n].y, w: tab(n).0.max(crate::core::text::name_width(&d.nodes[n].name)), h: tab(n).1 });
    let bodies = containers.iter().map(|&n| Rect { y: g.rects[n].y + tab(n).1, h: g.rects[n].h - tab(n).1, ..g.rects[n] });
    let heads = (0..d.edges.len()).filter_map(|e| match d.edges[e].head {
        model::Head::Open => Some((e, 10.0, 4.5)),
        model::Head::Triangle => Some((e, 12.0, 7.0)),
        model::Head::None => None,
    });
    Ok(crate::diagrams::connectors::scene(crate::diagrams::connectors::Drawn {
        heads: heads.collect(),
        lines: r.edges.clone(),
        solids: (0..d.nodes.len()).filter(|&n| leaf(n)).map(|n| g.rects[n]).chain(tabs).collect(),
        outlines: bodies.collect(),
        owned: labels.into_iter().enumerate().collect(),
        free: Vec::new(),
    }))
}

/// The drawing as a figure for the end measures (`core::ends`). Note links are
/// left out: they are annotations, not connectors.
pub fn figure(source: &str, chosen: Option<Engine>) -> Res<crate::core::ends::Figure> {
    use model::{Head, Line};
    let (d, g, r) = drawing(source, chosen)?;
    let kept: Vec<usize> = (0..d.edges.len()).filter(|&e| d.edges[e].line != Line::NoteLink).collect();
    let marks = |e: usize| {
        let start = if d.edges[e].line == Line::Containment { 12.0 } else { 0.0 };
        let end = match d.edges[e].head {
            Head::None => 0.0,
            Head::Open => 10.0,
            Head::Triangle => 12.0,
        };
        (start, end)
    };
    let body = |n: usize| {
        let tab = layout::tab_size(&d.nodes[n]).1;
        let top = if d.is_package(n) { tab } else { 0.0 };
        crate::core::geometry::Rect { y: g.rects[n].y + top, h: g.rects[n].h - top, ..g.rects[n] }
    };
    Ok(crate::core::ends::Figure {
        direction: d.direction,
        lines: kept.iter().map(|&e| r.edges[e].clone()).collect(),
        marks: kept.iter().map(|&e| marks(e)).collect(),
        boxes: (0..d.nodes.len()).map(|n| (if n == 0 { Default::default() } else { body(n) }, d.nodes[n].parent)).collect(),
        fixtures: Vec::new(),
        links: kept.iter().map(|&e| (d.edges[e].from, d.edges[e].to)).collect(),
        pivots: Vec::new(),
    })
}
