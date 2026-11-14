// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Activity diagrams: parse, then a recursive structured-flow layout, then draw.
//! They do not use the layered engine: the block tree already says what is beside
//! what, so sizes are measured from the leaves up and placed from the root down.

mod layout;
mod model;
mod parse;
mod render;
#[cfg(test)]
mod scene;
#[cfg(test)]
mod tests;

use crate::syntax::Res;

pub fn check(source: &str) -> Res<()> {
    parse::parse(source).map(|_| ())
}

/// Renders in the `style` chosen, or the source's.
pub fn render(source: &str, style: Option<crate::core::style::Style>) -> Res<String> {
    let parsed = parse::parse(source)?;
    let diagram = model::Diagram { style: style.unwrap_or(parsed.style), ..parsed };
    Ok(crate::core::text::with_face(diagram.style.face(), || render::render(&diagram, &layout::layout(&diagram))))
}

/// The laid-out drawing, for layout quality measurements.
#[cfg(test)]
pub fn scene(source: &str) -> Res<crate::core::metrics::Scene> {
    let diagram = parse::parse(source)?;
    Ok(scene::scene(&layout::layout(&diagram)))
}

/// The size measures: the flows' and note links' length (the scene's lines)
/// and the drawn document's frame.
#[cfg(test)]
pub fn size(source: &str) -> Res<crate::core::metrics::Size> {
    let diagram = parse::parse(source)?;
    let laid = layout::layout(&diagram);
    let frame = crate::core::svg::frame(&render::render(&diagram, &laid)).expect("a document has a frame");
    Ok(crate::core::metrics::size(&scene::scene(&laid).lines, frame))
}

/// The laid-out nodes and flows, for the flow measures: every node's box with
/// whether it is a decision, and every flow's polyline.
#[cfg(test)]
pub fn flow_parts(source: &str) -> Res<(Vec<(crate::core::geometry::Rect, bool)>, Vec<Vec<crate::core::geometry::Point>>)> {
    let g = layout::layout(&parse::parse(source)?);
    let nodes = g.nodes.iter().map(|n| (n.rect, n.kind == layout::NodeKind::Decision)).collect();
    Ok((nodes, g.edges.iter().map(|e| e.points.clone()).collect()))
}
