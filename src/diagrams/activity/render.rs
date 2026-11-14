// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Draws a laid-out activity as a list of marks.

use super::layout::*;
use super::model::Diagram;
use crate::core::geometry::{Point, Rect};
use crate::core::glyphs::{arrowhead, note_wrapped};
use crate::core::style::{Extra, Role, Style};
use crate::core::svg::{bound, circle, document, n, path, rect, text, Anchor, Mark, TextStyle};

const NAME_LINE: f64 = 16.0;

/// The `class` attribute of a rectangle with this role.
fn class(style: Style, role: Role) -> String {
    format!("class=\"{}\"", style.class(role))
}

fn outline(points: &[Point]) -> String {
    let body: Vec<String> = points.iter().enumerate().map(|(i, p)| format!("{}{} {}", if i == 0 { "M" } else { "L" }, n(p.0), n(p.1))).collect();
    format!("{} Z", body.join(" "))
}

fn centre_text(lines: &[String], r: Rect) -> Vec<Mark> {
    let block = lines.len() as f64 * NAME_LINE;
    let first = r.y + (r.h - block) / 2.0 + 12.0;
    lines.iter().enumerate().map(|(i, line)| text(r.x + r.w / 2.0, first + i as f64 * NAME_LINE, line, TextStyle::Name, Anchor::Middle)).collect()
}

/// A rectangle with a point on its right: a send signal action (Figure 16.18).
fn send(r: Rect, style: Style) -> Mark {
    let (x, y, right, bottom) = (r.x, r.y, r.right(), r.bottom());
    let tip = (right, y + r.h / 2.0);
    path(&outline(&[(x, y), (right - 12.0, y), tip, (right - 12.0, bottom), (x, bottom)]), style.class(Role::Box))
}

/// A rectangle with a notch in its left: an accept event action (Figure 16.40).
fn accept(r: Rect, style: Style) -> Mark {
    let (x, y, right, bottom) = (r.x, r.y, r.right(), r.bottom());
    let notch = (x + 12.0, y + r.h / 2.0);
    path(&outline(&[(x, y), (right, y), (right, bottom), (x, bottom), notch]), style.class(Role::Box))
}

fn draw_node(node: &Node, style: Style) -> Vec<Mark> {
    let (shape, solid, line) = (style.class(Role::Box), style.class(Role::Solid), style.class(Role::Line));
    let r = node.rect;
    let (cx, cy) = r.center();
    let outer = r.w / 2.0;
    let edge = bound(vec![(r.x, r.y), (r.right(), r.bottom())]);
    let body: Vec<Mark> = match node.kind {
        NodeKind::Action => vec![rect(r, &format!("class=\"{shape}\" rx=\"10\""))],
        NodeKind::Object => vec![rect(r, &class(style, Role::Box))],
        NodeKind::Send => vec![send(r, style)],
        NodeKind::Accept => vec![accept(r, style)],
        NodeKind::Start => vec![circle((cx, cy), outer, solid)],
        NodeKind::Stop => vec![circle((cx, cy), outer, shape), circle((cx, cy), outer - 3.5, solid)],
        NodeKind::End => {
            let d = outer * std::f64::consts::FRAC_1_SQRT_2;
            let cross = format!("M{} {} L{} {} M{} {} L{} {}", n(cx - d), n(cy - d), n(cx + d), n(cy + d), n(cx - d), n(cy + d), n(cx + d), n(cy - d));
            vec![circle((cx, cy), outer, shape), path(&cross, line)]
        }
        NodeKind::Decision | NodeKind::Merge => {
            vec![path(&outline(&[(cx, r.y), (r.right(), cy), (cx, r.bottom()), (r.x, cy)]), shape)]
        }
        NodeKind::Bar => vec![rect(r, &class(style, Role::Solid))],
    };
    [vec![edge], body, centre_text(&node.lines, r)].concat()
}

fn draw_edge(e: &Edge, style: Style) -> Vec<Mark> {
    vec![bound(e.points.clone()), path(&style.line(&e.points), style.class(Role::Line)), arrowhead(&e.points, style.class(Role::Head))]
}

fn draw_label(label: &Label) -> Vec<Mark> {
    label
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| text(label.rect.x, label.rect.y + 11.0 + i as f64 * 14.0, line, TextStyle::Small, Anchor::Start))
        .collect()
}

/// A partition: its frame holds the actions in it, and its name sits in a
/// head drawn with the same pen.
fn draw_band(band: &Band, style: Style) -> Vec<Mark> {
    vec![
        rect(band.rect, &class(style, Role::Container)),
        rect(band.head, &class(style, Role::Container)),
        text(band.head.x + band.head.w / 2.0, band.head.y + 21.0, &band.name, TextStyle::Name, Anchor::Middle),
    ]
}

/// Lane bands first, then lines under nodes so a node's outline stays whole.
pub fn render(d: &Diagram, g: &Layout) -> String {
    let style = d.style;
    let bands: Vec<Mark> = g.bands.iter().flat_map(|band| draw_band(band, style)).collect();
    let edges: Vec<Mark> = g.edges.iter().flat_map(|edge| draw_edge(edge, style)).collect();
    let links: Vec<Mark> = g.links.iter().map(|&[a, b]| path(&format!("M{} {} L{} {}", n(a.0), n(a.1), n(b.0), n(b.1)), style.class(Role::DashedLine))).collect();
    let nodes: Vec<Mark> = g.nodes.iter().flat_map(|node| draw_node(node, style)).collect();
    let notes: Vec<Mark> = g.notes.iter().flat_map(|note| note_wrapped(note.rect, &note.text, note.wrap, style)).collect();
    let labels: Vec<Mark> = g.labels.iter().flat_map(draw_label).collect();
    document(style, &[bands, edges, links, nodes, notes, labels].concat(), d.title.as_deref(), &[Extra::Solid])
}
