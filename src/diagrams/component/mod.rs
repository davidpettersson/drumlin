// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Component diagrams: parse, then lay out, route and draw as pure stages.

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

#[cfg(test)]
mod tests {
    use super::model::EdgeKind;
    use super::render::{glyph_run, glyph_segment};

    use crate::core::geometry::Side::{self, Bottom, Left, Right, Top};

    fn side_of(r: &crate::core::geometry::Rect, q: crate::core::geometry::Point) -> Option<Side> {
        let (within_x, within_y) = (q.0 > r.x && q.0 < r.right(), q.1 > r.y && q.1 < r.bottom());
        match () {
            _ if q.0 == r.x && within_y => Some(Left),
            _ if q.0 == r.right() && within_y => Some(Right),
            _ if q.1 == r.y && within_x => Some(Top),
            _ if q.1 == r.bottom() && within_x => Some(Bottom),
            _ => None,
        }
    }

    /// A wired interface whose line bends gets a segment long enough
    /// for its glyph and a `CLEARANCE` stub on each side, not a few pixels
    /// between the component and its socket. The agent feedback loop was the
    /// bending case until alignment lined its chain up; it must keep the room
    /// straight, and the debugger's fan bends one line instead.
    #[test]
    fn interface_glyphs_keep_their_clearance() {
        let agents = "diagram component\ndirection right\ncomponent Agent { provides IModel }\ncomponent \"drumlin check\" as check {\n  requires IModel\n  provides IDiagnostics\n}\ncomponent \"drumlin render\" as render { requires IDiagnostics }\ncomponent Reviewer\nrender --> Reviewer : SVG\n";
        let debugger = "diagram component\ndirection right\ncomponent UI { requires IDebug }\ncomponent Engine {\n  provides IDebug\n  requires ISymbols\n  requires ITarget\n}\ncomponent DWARF { provides ISymbols }\ncomponent ptrace { provides ITarget }\n";
        for (source, count) in [(agents, 2), (debugger, 3)] {
            let (d, _, r) = super::drawing(source, None).unwrap();
            let wired: Vec<usize> = (0..d.edges.len()).filter(|&e| d.edges[e].kind == EdgeKind::Wired).collect();
            assert_eq!(wired.len(), count);
            for &e in &wired {
                let path = &r.edges[e];
                let k = glyph_segment(path, d.direction);
                let length = (path[k].0 - path[k + 1].0).abs() + (path[k].1 - path[k + 1].1).abs();
                assert!(length >= glyph_run(EdgeKind::Wired), "edge {e}: glyph segment {length} in {path:?}");
            }
            if source == debugger {
                assert!(wired.iter().any(|&e| r.edges[e].len() > 2), "one interface line bends");
            }
        }
    }

    /// Connectors along the main chain run straight between their
    /// connection points (one segment), with the other member of a fan beside.
    #[test]
    fn chains_run_straight_between_connection_points() {
        let golden = |name: &str| std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/component").join(format!("{name}.uml"))).unwrap();
        let (boot, server, kernel) = (golden("gallery-embedded-04"), golden("gallery-tools-02"), golden("gallery-systems-01"));
        for (source, straight) in [(boot, vec!["ROM", "First Stage", "Second Stage", "Verifier"]), (server, vec!["Editor", "lsp", "Dispatcher", "Incremental Parser", "Symbol Index"]), (kernel, vec!["System Call Interface", "VFS", "Block Layer", "Device Drivers"])] {
            let (d, _, r) = super::drawing(&source, None).unwrap();
            let named = |e: &super::model::End| match *e {
                super::model::End::Node(n) => d.nodes[n].name.clone(),
                super::model::End::Port(p) => d.ports[p].name.clone(),
            };
            for pair in straight.windows(2) {
                let e = d.edges.iter().position(|edge| (named(&edge.from), named(&edge.to)) == (pair[0].to_string(), pair[1].to_string()) || (named(&edge.from), named(&edge.to)) == (pair[1].to_string(), pair[0].to_string()));
                let e = e.unwrap_or_else(|| panic!("no connector {} to {}", pair[0], pair[1]));
                assert_eq!(r.edges[e].len(), 2, "{} to {} jogs: {:?}", pair[0], pair[1], r.edges[e]);
            }
        }
    }

    /// A line routed past a container it does not end on keeps the
    /// container clearance (24px) from its border (Stream processor).
    #[test]
    fn lines_keep_their_room_beside_a_container() {
        let source = "diagram component\ndirection right\ncomponent Producers { requires ILog }\ncomponent \"Partitioned Log\" as log {\n  provides ILog\n  provides ISubscribe\n}\ncomponent Job as job {\n  port source requires ISubscribe\n  component Windowing\n  component Aggregator\n  component Sink\n  source --> Windowing\n  Windowing --> Aggregator\n  Aggregator --> Sink\n}\ncomponent State as state\njob ..> state : checkpoints\n";
        let (d, g, r) = super::drawing(source, None).unwrap();
        let job = d.nodes.iter().position(|n| n.name == "Job").unwrap();
        let e = d.edges.iter().position(|edge| edge.label.as_deref() == Some("ISubscribe")).unwrap();
        let box_ = g.rects[job];
        let above = r.edges[e].windows(2).filter(|w| w[0].1 == w[1].1 && w[0].1 < box_.y).map(|w| box_.y - w[0].1).fold(f64::MAX, f64::min);
        assert!(above >= 24.0, "the line runs {above}px above Job: {:?}", r.edges[e]);
    }


    /// No line doubles back into a box, and a box spreads its ends
    /// over free sides before it crowds one (Drawing principles, Lines: never
    /// double back into a box; use a free side before you share one). In
    /// Compiler pipeline the dashed `invoke` leaves Driver and enters Code
    /// Generator on the right, outside the solid chain, which stays straight.
    #[test]
    fn lines_meet_the_side_they_come_from() {
        let golden = |name: &str| std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/component").join(format!("{name}.uml"))).unwrap();
        for name in ["example-simple", "gallery-tools-01", "gallery-systems-04", "language-interfaces-04", "gallery-services-01", "gallery-services-04"] {
            let (d, g, r) = super::drawing(&golden(name), None).unwrap();
            let leaves: Vec<crate::core::geometry::Rect> = (1..d.nodes.len()).filter(|&n| d.nodes[n].children.is_empty()).map(|n| g.rects[n]).collect();
            let back = crate::core::ends::doubled_back(&leaves, &r.edges);
            assert!(back.is_empty(), "{name}: lines double back at {back:?}");
            // At most one line end per side of a box while a side of it is free.
            for (n, rect) in g.rects.iter().enumerate().skip(1).filter(|&(n, _)| d.nodes[n].children.is_empty() && d.nodes[n].kind == super::model::Kind::Component) {
                let ends: Vec<crate::core::geometry::Side> = r.edges.iter().zip(&d.edges).filter(|(_, e)| e.kind != EdgeKind::NoteLink).flat_map(|(p, _)| [p[0], p[p.len() - 1]]).filter_map(|q| side_of(rect, q)).collect();
                let crowded = [Left, Right, Top, Bottom].iter().any(|s| ends.iter().filter(|e| *e == s).count() > 1);
                let free = [Left, Right, Top, Bottom].iter().any(|s| !ends.contains(s));
                if ["gallery-systems-04", "language-interfaces-04"].contains(&name) {
                    assert!(!(crowded && free), "{name}: {} shares a side with a side free: {ends:?}", d.nodes[n].name);
                }
            }
        }
        let (d, _, r) = super::drawing(&golden("example-simple"), None).unwrap();
        let invoke = d.edges.iter().position(|e| e.label.as_deref() == Some("invoke")).unwrap();
        let path = &r.edges[invoke];
        let (first, last) = (path[1].0 - path[0].0, path[path.len() - 2].0 - path[path.len() - 1].0);
        assert!(first > 0.0 && last > 0.0, "invoke leaves Driver and enters Code Generator on the right: {path:?}");
    }

    /// The spine runs on one line through a container's ports (Drawing
    /// principles, Flow: keep the spine on one line). Order Platform: Web Gateway,
    /// port `api`, API Handler, Outbox, port `events` and Event Bus are joined by
    /// straight connectors, with Pricing Engine beside.
    #[test]
    fn the_spine_runs_straight_through_ports() {
        let golden = |name: &str| std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/component").join(format!("{name}.uml"))).unwrap();
        let (d, _, r) = super::drawing(&golden("example-orders"), None).unwrap();
        let named = |e: &super::model::End| match *e {
            super::model::End::Node(n) => d.nodes[n].name.clone(),
            super::model::End::Port(p) => d.ports[p].name.clone(),
        };
        let chain = ["Web Gateway", "api", "API Handler", "Outbox", "events", "Event Bus"];
        let mut level = None;
        for pair in chain.windows(2) {
            let e = d.edges.iter().position(|edge| [named(&edge.from), named(&edge.to)] == [pair[0], pair[1]] || [named(&edge.from), named(&edge.to)] == [pair[1], pair[0]]);
            let e = e.unwrap_or_else(|| panic!("no connector {} to {}", pair[0], pair[1]));
            let path = &r.edges[e];
            assert_eq!(path.len(), 2, "{} to {} jogs: {path:?}", pair[0], pair[1]);
            assert_eq!(*level.get_or_insert(path[0].1), path[0].1, "{} to {} leaves the spine's line", pair[0], pair[1]);
        }
    }

    /// A box grows across the flow so that the ends on one side keep
    /// their distance from each other and from the corners, wider where the
    /// lines carry names (Drawing principles, Shapes: leave room for what
    /// attaches). Three interfaces between App and Runtime.
    #[test]
    fn ends_keep_off_corners_and_apart() {
        let source = "diagram component\ncomponent Runtime { provides ITimer, IThreads, IFiles }\ncomponent App { requires ITimer, IThreads, IFiles }\n";
        let (d, g, r) = super::drawing(source, None).unwrap();
        for n in 1..d.nodes.len() {
            let rect = g.rects[n];
            let mut at: Vec<f64> = r.edges.iter().flat_map(|p| [p[0], p[p.len() - 1]]).filter(|q| side_of(&rect, *q).is_some()).map(|q| q.1).collect();
            at.sort_by(f64::total_cmp);
            assert_eq!(at.len(), 3);
            assert!(at[0] - rect.y >= 16.0 && rect.bottom() - at[2] >= 16.0, "{}: ends near a corner: {at:?} in {rect:?}", d.nodes[n].name);
            assert!(at.windows(2).all(|w| w[1] - w[0] >= super::layout::NAMED_SLOT), "{}: named lines closer than {}: {at:?}", d.nodes[n].name, super::layout::NAMED_SLOT);
        }
    }

    /// Of two lines into one side, the one between boxes in one row
    /// runs straight and the other yields (LSM storage engine: `probe` from Read
    /// Path to Memtable is straight, `insert` enters above it).
    #[test]
    fn the_straight_line_keeps_its_side() {
        let golden = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/component/gallery-systems-02.uml")).unwrap();
        let (d, _, r) = super::drawing(&golden, None).unwrap();
        let probe = d.edges.iter().position(|e| e.label.as_deref() == Some("probe")).unwrap();
        assert_eq!(r.edges[probe].len(), 2, "probe jogs: {:?}", r.edges[probe]);
    }
}
