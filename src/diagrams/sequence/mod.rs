// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagrams: parse, then a timeline layout, then draw, as pure stages.
//! They do not use the layered engine: columns come from label widths and rows
//! from a walk over the message list.

mod layout;
mod model;
mod parse;
mod render;
#[cfg(test)]
mod scene;

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
    Ok(scene::scene(&diagram, &layout::layout(&diagram)))
}

/// The size measures: the messages' length (the scene's lines) and the drawn
/// document's frame.
#[cfg(test)]
pub fn size(source: &str) -> Res<crate::core::metrics::Size> {
    let diagram = parse::parse(source)?;
    let laid = layout::layout(&diagram);
    let frame = crate::core::svg::frame(&render::render(&diagram, &laid)).expect("a document has a frame");
    Ok(crate::core::metrics::size(&scene::scene(&diagram, &laid).lines, frame))
}
