// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Draws a laid-out package diagram as a list of marks.

use super::layout::tab_size;
use super::model::*;
use super::routing::Routes;
use crate::core::geometry::{Point, Rect};
use crate::core::glyphs::{arrowhead, circled_plus, folder, hollow_triangle, note};
use crate::core::labels::Placer;
use crate::core::layered::Layout;
use crate::core::style::{Role, Style};
use crate::core::svg::{bound, document, path, rect, text, Anchor, Mark, TextStyle};
use crate::core::text::{name_width, small_width};

const ANCHOR_RADIUS: f64 = 6.0;

/// Centres a keyword line and a name in `r`, or just the name.
fn centred_labels(r: Rect, keyword: Option<String>, name: &str, style: TextStyle) -> Vec<Mark> {
    let middle = r.x + r.w / 2.0;
    let baseline = r.y + r.h / 2.0 + 4.5;
    match keyword {
        Some(k) => vec![
            text(middle, baseline - 9.0, &k, TextStyle::Stereotype, Anchor::Middle),
            text(middle, baseline + 8.0, name, style, Anchor::Middle),
        ],
        None => vec![text(middle, baseline, name, style, Anchor::Middle)],
    }
}

fn package_shape(node: &Node, r: Rect, style: Style) -> Vec<Mark> {
    let (tab_w, tab_h) = tab_size(node);
    let role = if node.children.is_empty() { Role::Box } else { Role::Container };
    let outline = folder(r, tab_w, tab_h, style.class(role));
    let keyword = keywords(node);
    let labels = if node.children.is_empty() {
        centred_labels(Rect { y: r.y + tab_h, h: r.h - tab_h, ..r }, keyword, &node.name, TextStyle::Name)
    } else {
        // With its contents shown, a package carries its name in the tab.
        let (x, y) = (r.x + 12.0, r.y + tab_h - 7.5);
        keyword
            .iter()
            .map(|k| text(x, y - 17.0, k, TextStyle::Stereotype, Anchor::Start))
            .chain([text(x, y, &node.name, TextStyle::Name, Anchor::Start)])
            .collect()
    };
    [outline, labels].concat()
}

fn node_shape(node: &Node, r: Rect, style: Style) -> Vec<Mark> {
    match node.kind {
        Kind::Root => Vec::new(),
        Kind::Package => package_shape(node, r, style),
        Kind::Note => note(r, &node.name, style),
        Kind::Class | Kind::Interface | Kind::Enum => {
            let lettering = if is_abstract(node) { TextStyle::NameItalic } else { TextStyle::Name };
            [vec![rect(r, &format!("class=\"{}\"", style.class(Role::Box)))], centred_labels(r, keywords(node), &node.name, lettering)].concat()
        }
    }
}

/// A node's marks, then its children's, so containers paint behind contents.
fn draw_node(d: &Diagram, g: &Layout, i: usize) -> Vec<Mark> {
    let own = node_shape(&d.nodes[i], g.rects[i], d.style);
    own.into_iter().chain(d.nodes[i].children.iter().flat_map(|&child| draw_node(d, g, child))).collect()
}

fn draw_edge(edge: &Edge, points: &[Point], style: Style) -> Vec<Mark> {
    if points.len() < 2 {
        return Vec::new();
    }
    let dashed = matches!(edge.line, Line::Dashed | Line::NoteLink);
    let (line_role, head_role, triangle_role) =
        if dashed { (Role::DashedLine, Role::DashedHead, Role::DashedHollowHead) } else { (Role::Line, Role::Head, Role::HollowHead) };
    let line = path(&style.line(points), style.class(line_role));
    let head = match edge.head {
        Head::None => Vec::new(),
        Head::Open => vec![arrowhead(points, style.class(head_role))],
        Head::Triangle => hollow_triangle(points, style.class(triangle_role)),
    };
    [vec![bound(points.to_vec()), line], head].concat()
}

/// Circle-plus anchors, one per owner, where containment lines leave it.
fn draw_anchors(d: &Diagram, r: &Routes) -> Vec<Mark> {
    let mut centres: Vec<(i64, i64)> = d
        .edges
        .iter()
        .zip(&r.edges)
        .filter(|(edge, points)| edge.line == Line::Containment && points.len() >= 2)
        .map(|(_, points)| {
            let (a, b) = (points[0], points[1]);
            let l = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt().max(0.001);
            let centre = (a.0 + (b.0 - a.0) / l * ANCHOR_RADIUS, a.1 + (b.1 - a.1) / l * ANCHOR_RADIUS);
            ((centre.0 * 100.0).round() as i64, (centre.1 * 100.0).round() as i64)
        })
        .collect();
    centres.sort_unstable();
    centres.dedup();
    centres.iter().flat_map(|&(x, y)| circled_plus((x as f64 / 100.0, y as f64 / 100.0), ANCHOR_RADIUS, d.style)).collect()
}

fn draw_edge_label(placer: Placer, edge: &Edge, points: &[Point]) -> (Placer, Vec<Mark>) {
    let Some(label) = edge.label.as_ref().filter(|_| points.len() >= 2) else {
        return (placer, Vec::new());
    };
    let (placer, at) = placer.place_around(points, small_width(label) + 4.0);
    (placer, vec![text(at.x, at.y, label, TextStyle::SmallHalo, at.anchor)])
}

/// Thin strips along a container's outline, its tab and its name.
fn container_marks(d: &Diagram, g: &Layout, n: usize) -> Vec<Rect> {
    let r = g.rects[n];
    let (tab_w, tab_h) = tab_size(&d.nodes[n]);
    let top = r.y + tab_h;
    vec![
        Rect { x: r.x, y: r.y, w: tab_w.max(name_width(&d.nodes[n].name)), h: tab_h },
        Rect { x: r.x - 1.0, y: top, w: 2.0, h: r.h - tab_h },
        Rect { x: r.right() - 1.0, y: top, w: 2.0, h: r.h - tab_h },
        Rect { x: r.x, y: top - 1.0, w: r.w, h: 2.0 },
        Rect { x: r.x, y: r.bottom() - 1.0, w: r.w, h: 2.0 },
    ]
}

/// Everything fixed before labels: leaf boxes, container outlines and lines.
fn fixed_occupancy(d: &Diagram, g: &Layout, r: &Routes) -> Placer {
    let leaves = d.nodes.iter().zip(&g.rects).filter(|(node, _)| node.children.is_empty() && node.kind != Kind::Root).map(|(_, &rect)| rect);
    let outlines = (1..d.nodes.len()).filter(|&n| !d.nodes[n].children.is_empty()).flat_map(|n| container_marks(d, g, n));
    let placer = Placer::default().occupy_all(leaves).occupy_all(outlines);
    r.edges.iter().filter(|points| points.len() >= 2).fold(placer, |placer, points| placer.avoid_polyline(points))
}

/// Connector labels, one list of marks per edge.
pub fn edge_labels(d: &Diagram, g: &Layout, r: &Routes) -> Vec<Vec<Mark>> {
    let placer = fixed_occupancy(d, g, r);
    let step = |placer: Placer, (edge, points): (&Edge, &Vec<Point>)| draw_edge_label(placer, edge, points);
    d.edges.iter().zip(&r.edges).fold((placer, Vec::new()), |(placer, mut all), item| {
        let (placer, marks) = step(placer, item);
        all.push(marks);
        (placer, all)
    }).1
}

pub fn render(d: &Diagram, g: &Layout, r: &Routes) -> String {
    let nodes = draw_node(d, g, 0);
    let edges: Vec<Mark> = d.edges.iter().zip(&r.edges).flat_map(|(edge, points)| draw_edge(edge, points, d.style)).collect();
    let anchors = draw_anchors(d, r);
    let labels = edge_labels(d, g, r).concat();
    document(d.style, &[nodes, edges, anchors, labels].concat(), d.title.as_deref(), &[])
}
