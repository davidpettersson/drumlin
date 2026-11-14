// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! State machine diagrams: parse, then lay out, route and draw as pure stages.

mod hybrid;
mod layout;
mod model;
mod parse;
mod render;
mod routing;
#[cfg(test)]
mod scene;
#[cfg(test)]
mod tests;

use crate::core::style::Style;
use crate::core::text::with_face;
use crate::diagrams::engine::Engine;
use crate::syntax::Res;

#[cfg(test)]
pub use scene::{figure, label_scene, lines, size};

pub fn check(source: &str) -> Res<()> {
    parse::parse(source).map(|_| ())
}

/// Parses, then lays out, grows boxes, routes and settles the labels: the
/// drawing that the quality measures start from, measured in the face of the
/// caller's scope (`text::with_face`). A `chosen` engine wins over the source's.
#[cfg(test)]
fn drawing(source: &str, chosen: Option<Engine>) -> Res<(model::Diagram, hybrid::Drawn)> {
    Ok(laid_out(parse::parse(source)?, chosen))
}

/// Lays out a parsed diagram: what `render` draws, inside its style's face.
fn laid_out(parsed: model::Diagram, chosen: Option<Engine>) -> (model::Diagram, hybrid::Drawn) {
    let diagram = model::Diagram { engine: chosen.unwrap_or(parsed.engine), ..parsed };
    let drawn = hybrid::settle(&diagram, routing::grown(&diagram, routing::points_lined_up(&diagram, routing::arranged(&diagram))));
    (diagram, drawn)
}

/// Renders in the `style` chosen, or the source's.
pub fn render(source: &str, chosen: Option<Engine>, style: Option<Style>) -> Res<String> {
    let parsed = parse::parse(source)?;
    let parsed = model::Diagram { style: style.unwrap_or(parsed.style), ..parsed };
    Ok(with_face(parsed.style.face(), || {
        let (diagram, drawn) = laid_out(parsed, chosen);
        super::warn_unrouted("transition", &drawn.routes.failed);
        render::render(&diagram, &drawn.geometry, &drawn.routes, &drawn.placed)
    }))
}
