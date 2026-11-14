// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Package diagrams: parse, then lay out, route and draw as pure stages.

mod layout;
mod model;
mod parse;
mod render;
mod routing;
#[cfg(test)]
mod scene;

use crate::core::layered::Layout;
use crate::core::style::Style;
use crate::core::text::with_face;
use crate::diagrams::engine::Engine;
use crate::syntax::Res;

#[cfg(test)]
pub use scene::{figure, label_scene, lines, size};

pub fn check(source: &str) -> Res<()> {
    parse::parse(source).map(|_| ())
}

/// Parses, lays out, grows boxes and routes: the drawing that the quality
/// measures start from, measured in the face of the caller's scope
/// (`text::with_face`). A `chosen` engine wins over the source's.
#[cfg(test)]
fn drawing(source: &str, chosen: Option<Engine>) -> Res<(model::Diagram, Layout, routing::Routes)> {
    Ok(laid_out(parse::parse(source)?, chosen))
}

/// Lays out a parsed diagram: what `render` draws, inside its style's face.
fn laid_out(parsed: model::Diagram, chosen: Option<Engine>) -> (model::Diagram, Layout, routing::Routes) {
    let diagram = model::Diagram { engine: chosen.unwrap_or(parsed.engine), ..parsed };
    let geometry = routing::grown(&diagram, routing::arranged(&diagram));
    let routes = routing::route(&diagram, &geometry);
    (diagram, geometry, routes)
}

/// Renders in the `style` chosen, or the source's.
pub fn render(source: &str, chosen: Option<Engine>, style: Option<Style>) -> Res<String> {
    let parsed = parse::parse(source)?;
    let parsed = model::Diagram { style: style.unwrap_or(parsed.style), ..parsed };
    Ok(with_face(parsed.style.face(), || {
        let (diagram, geometry, routes) = laid_out(parsed, chosen);
        super::warn_unrouted("connector", &routes.failed);
        render::render(&diagram, &geometry, &routes)
    }))
}
