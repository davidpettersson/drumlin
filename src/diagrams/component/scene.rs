// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! A component drawing as the quality test measures it: the routed lines, its
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

/// The drawing as a scene for the label measure: connector labels belong to
/// their line. Interface names on assembly connectors read with their ball,
/// which sits on the line, so they are only text to keep clear of.
pub fn label_scene(source: &str, chosen: Option<Engine>) -> Res<crate::core::metrics::Scene> {
    use crate::core::geometry::Rect;
    let (d, g, r) = drawing(source, chosen)?;
    let parts = render::parts(&d, &g, &r);
    let leaf = |n: usize| n > 0 && d.nodes[n].children.is_empty();
    let leaves: Vec<Rect> = (0..d.nodes.len()).filter(|&n| leaf(n)).map(|n| g.rects[n]).collect();
    let in_leaf = |m: &crate::core::svg::Mark| m.ink().is_some_and(|i| leaves.iter().any(|l| l.x <= i.x && i.right() <= l.right() && l.y <= i.y && i.bottom() <= l.bottom()));
    let names = parts.nodes.iter().filter(|m| m.ink().is_some() && !in_leaf(m)).cloned();
    let ports = r.ports.iter().map(|p| Rect { x: p.center.0 - 6.0, y: p.center.1 - 6.0, w: 12.0, h: 12.0 });
    let containers = (1..d.nodes.len()).filter(|&n| !leaf(n)).map(|n| {
        let top = if d.nodes[n].kind == model::Kind::Package { layout::PACKAGE_TAB } else { 0.0 };
        Rect { y: g.rects[n].y + top, h: g.rects[n].h - top, ..g.rects[n] }
    });
    let plain = |e: usize| matches!(d.edges[e].kind, model::EdgeKind::Solid | model::EdgeKind::Dashed) && d.edges[e].head == model::Head::Open;
    Ok(crate::diagrams::connectors::scene(crate::diagrams::connectors::Drawn {
        heads: (0..d.edges.len()).filter(|&e| plain(e)).map(|e| (e, 10.0, 4.5)).collect(),
        lines: r.edges.clone(),
        solids: leaves.iter().copied().chain(ports).collect(),
        outlines: containers.collect(),
        owned: parts.labels.into_iter().enumerate().collect(),
        free: names.chain(parts.stubs).chain(parts.ports).chain(parts.edges.concat()).collect(),
    }))
}

/// The drawing as a figure for the end measures (`core::ends`). Note links are
/// left out: they are annotations, not connectors.
pub fn figure(source: &str, chosen: Option<Engine>) -> Res<crate::core::ends::Figure> {
    use model::{EdgeKind, End, Head, Kind};
    let (d, g, r) = drawing(source, chosen)?;
    let kept: Vec<usize> = (0..d.edges.len()).filter(|&e| d.edges[e].kind != EdgeKind::NoteLink).collect();
    let head = |e: usize| if matches!(d.edges[e].kind, EdgeKind::Solid | EdgeKind::Dashed) && d.edges[e].head == Head::Open { 10.0 } else { 0.0 };
    let body = |n: usize| {
        let top = if d.nodes[n].kind == Kind::Package { layout::PACKAGE_TAB } else { 0.0 };
        crate::core::geometry::Rect { y: g.rects[n].y + top, h: g.rects[n].h - top, ..g.rects[n] }
    };
    let stubs = r.stubs.iter().filter(|s| matches!(d.stubs[s.stub].at, End::Node(_))).map(|s| s.base);
    Ok(crate::core::ends::Figure {
        direction: d.direction,
        lines: kept.iter().map(|&e| r.edges[e].clone()).collect(),
        marks: kept.iter().map(|&e| (0.0, head(e))).collect(),
        boxes: (0..d.nodes.len()).map(|n| (if n == 0 { Default::default() } else { body(n) }, d.nodes[n].parent)).collect(),
        fixtures: r.ports.iter().map(|p| p.center).chain(stubs).collect(),
        links: kept.iter().map(|&e| (d.owner(d.edges[e].from), d.owner(d.edges[e].to))).collect(),
        pivots: Vec::new(),
    })
}
