// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Draws a laid-out state machine as a list of marks.

use super::layout::{header_height, keyword, label_lines, label_size, name_place, Geometry, ACTIVITY_LINE, LABEL_LINE};
use super::model::*;
use super::routing::Routes;
use crate::core::geometry::{Direction, Point, Rect};
use crate::core::glyphs::{arrowhead, note};
use crate::core::labels::Placer;
use crate::core::layered::LabelSpot;
use crate::core::style::{Extra, Role, Style};
use crate::core::svg::{bound, circle, document, draw_with, path, rect, text, Anchor, Mark, TextStyle};
use crate::core::text::small_width;

/// A state's outline: a rectangle with rounded corners (14.2.4).
fn rounded_box(r: Rect, role: Role, style: Style) -> Mark {
    rect(r, &format!("class=\"{}\" rx=\"10\"", style.class(role)))
}

/// A transition label placed clear of everything, ready to draw.
#[derive(Clone, Debug)]
pub struct PlacedLabel {
    pub edge: usize,
    pub lines: Vec<String>,
    pub rect: Rect,
    pub anchor: Anchor,
}

fn activity_room(node: &Node) -> f64 {
    if node.activities.is_empty() { 0.0 } else { node.activities.len() as f64 * ACTIVITY_LINE + 8.0 }
}

fn name_marks(node: &Node, r: Rect, (middle, anchor): (f64, Anchor)) -> Vec<Mark> {
    let row = if node.activities.is_empty() && node.children.is_empty() { r.h } else { header_height(node) - activity_room(node) };
    let baseline = r.y + row / 2.0 + 4.5 - if keyword(node).is_some() { 6.0 } else { 0.0 };
    match keyword(node) {
        Some(k) => vec![
            text(middle, baseline, &k, TextStyle::Small, anchor),
            text(middle, baseline + 16.0, &node.name, TextStyle::Name, anchor),
        ],
        None => vec![text(middle, baseline, &node.name, TextStyle::Name, anchor)],
    }
}

fn activity_marks(node: &Node, r: Rect) -> Vec<Mark> {
    let start = r.y + header_height(node) - node.activities.len() as f64 * ACTIVITY_LINE - 8.0;
    node.activities.iter().enumerate().map(|(i, line)| text(r.x + 12.0, start + 12.0 + i as f64 * ACTIVITY_LINE, line, TextStyle::Small, Anchor::Start)).collect()
}

fn separator(r: Rect, y: f64, style: Style) -> Mark {
    path(&format!("M{} {y} L{} {y}", r.x, r.right()), style.class(Role::Rule))
}

fn state_marks(d: &Diagram, g: &Geometry, n: usize, r: Rect) -> Vec<Mark> {
    let node = &d.nodes[n];
    let regions = d.regions(n);
    let decomposed = node.children.iter().any(|&c| !d.nodes[c].kind.is_connection_point());
    let style = d.style;
    let body = if decomposed && (!node.activities.is_empty() || !regions.is_empty()) { vec![separator(r, r.y + header_height(node), style)] } else { Vec::new() };
    let name_rule = if node.activities.is_empty() { Vec::new() } else { vec![separator(r, r.y + header_height(node) - activity_room(node), style)] };
    let tiles: Vec<Mark> = regions
        .windows(2)
        .map(|pair| {
            let (a, b) = (g.rects[pair[0]], g.rects[pair[1]]);
            let top = r.y + header_height(node);
            match d.direction {
                Direction::Down => {
                    let x = (a.right() + b.x) / 2.0;
                    path(&format!("M{x} {top} L{x} {}", r.bottom()), style.class(Role::DashedRule))
                }
                Direction::Right => {
                    let y = (a.bottom() + b.y) / 2.0;
                    path(&format!("M{} {y} L{} {y}", r.x, r.right()), style.class(Role::DashedRule))
                }
            }
        })
        .collect();
    let outline = if decomposed { Role::Container } else { Role::Box };
    [vec![rounded_box(r, outline, style)], name_marks(node, r, name_place(d, &g.rects, n)), activity_marks(node, r), name_rule, body, tiles].concat()
}

fn connection_point_marks(node: &Node, r: Rect, direction: Direction, parent: Rect, style: Style) -> Vec<Mark> {
    let c = r.center();
    let radius = r.w / 2.0;
    let cross = if node.kind == Kind::ExitPoint {
        let d = radius * 0.71;
        vec![path(&format!("M{} {} L{} {} M{} {} L{} {}", c.0 - d, c.1 - d, c.0 + d, c.1 + d, c.0 - d, c.1 + d, c.0 + d, c.1 - d), style.class(Role::Line))]
    } else {
        Vec::new()
    };
    let (x, y, anchor) = match direction {
        Direction::Down => (c.0 + radius + 4.0, if r.center().1 <= parent.y + 1.0 { c.1 - 6.0 } else { c.1 + 14.0 }, Anchor::Start),
        Direction::Right => (c.0, if r.center().0 <= parent.x + 1.0 { c.1 - radius - 4.0 } else { c.1 - radius - 4.0 }, Anchor::Middle),
    };
    [vec![bound(vec![(r.x, r.y), (r.right(), r.bottom())]), circle(c, radius, style.class(Role::Box))], cross, vec![text(x, y, &node.name, TextStyle::Small, anchor)]].concat()
}

fn symbol_marks(node: &Node, r: Rect, style: Style) -> Vec<Mark> {
    let c = r.center();
    let (outline, solid, line) = (style.class(Role::Box), style.class(Role::Solid), style.class(Role::Line));
    let extent = bound(vec![(r.x, r.y), (r.right(), r.bottom())]);
    match node.kind {
        Kind::Initial => vec![extent, circle(c, r.w / 2.0, solid)],
        Kind::Junction => vec![extent, circle(c, 5.0, solid)],
        Kind::Final => vec![extent, circle(c, r.w / 2.0 - 1.0, outline), circle(c, r.w / 2.0 - 5.5, solid)],
        Kind::Shallow | Kind::Deep => {
            let label = if node.kind == Kind::Deep { "H*" } else { "H" };
            vec![extent, circle(c, r.w / 2.0 - 1.0, outline), text(c.0, c.1 + 4.5, label, TextStyle::Name, Anchor::Middle)]
        }
        Kind::Choice => vec![extent, path(&format!("M{} {} L{} {} L{} {} L{} {} Z", c.0, r.y, r.right(), c.1, c.0, r.bottom(), r.x, c.1), outline)],
        Kind::Terminate => vec![extent, path(&format!("M{} {} L{} {} M{} {} L{} {}", r.x, r.y, r.right(), r.bottom(), r.right(), r.y, r.x, r.bottom()), line)],
        Kind::Fork | Kind::Join => vec![extent, rect(r, &format!("class=\"{solid}\""))],
        Kind::Note => note(r, &node.name, style),
        _ => Vec::new(),
    }
}

/// A node's marks, then its children's, so containers paint behind contents.
fn draw_node(d: &Diagram, g: &Geometry, n: usize) -> Vec<Mark> {
    let node = &d.nodes[n];
    let r = g.rects[n];
    let own = match node.kind {
        Kind::Root => Vec::new(),
        Kind::State => state_marks(d, g, n, r),
        Kind::Region => {
            if node.name.is_empty() { Vec::new() } else { vec![text(r.x + 4.0, r.y + 14.0, &node.name, TextStyle::Small, Anchor::Start)] }
        }
        Kind::EntryPoint | Kind::ExitPoint => Vec::new(),
        _ => symbol_marks(node, r, d.style),
    };
    let children = node.children.iter().flat_map(|&child| draw_node(d, g, child));
    own.into_iter().chain(children).collect()
}

pub fn connection_points(d: &Diagram, g: &Geometry) -> Vec<Mark> {
    (1..d.nodes.len()).filter(|&n| d.nodes[n].kind.is_connection_point()).flat_map(|n| connection_point_marks(&d.nodes[n], g.rects[n], d.direction, g.rects[d.nodes[n].parent.unwrap()], d.style)).collect()
}

fn draw_edge(edge: &Edge, points: &[Point], style: Style) -> Vec<Mark> {
    if points.len() < 2 {
        return Vec::new();
    }
    let role = if edge.line == Line::NoteLink { Role::DashedLine } else { Role::Line };
    let head = if edge.line == Line::NoteLink { Vec::new() } else { vec![arrowhead(points, style.class(Role::Head))] };
    [vec![bound(points.to_vec()), path(&style.line(points), style.class(role))], head].concat()
}

fn loop_marks(g: &Geometry, style: Style) -> Vec<Mark> {
    g.loops.iter().flat_map(|l| [bound(l.points.clone()), path(&style.line(&l.points), style.class(Role::Line)), arrowhead(&l.points, style.class(Role::Head))]).collect()
}

/// Lines of a label stacked inside its box, using `anchor` for their alignment.
pub fn label_marks(label: &PlacedLabel) -> Vec<Mark> {
    let x = match label.anchor {
        Anchor::Start => label.rect.x,
        Anchor::Middle => label.rect.x + label.rect.w / 2.0,
        Anchor::End => label.rect.right(),
    };
    label.lines.iter().enumerate().map(|(i, line)| text(x, label.rect.y + 10.5 + i as f64 * LABEL_LINE, line, TextStyle::Small, label.anchor)).collect()
}

/// Labels placed by the layout, beside their reserved anchors.
fn reserved_labels(d: &Diagram, g: &Geometry) -> Vec<PlacedLabel> {
    d.edges
        .iter()
        .enumerate()
        .filter_map(|(e, edge)| {
            let spot: &LabelSpot = g.labels[e].as_ref()?;
            let lines = label_lines(edge);
            let anchor = if d.direction == Direction::Right {
                Anchor::Middle
            } else if spot.rect.x > spot.anchor.0 {
                Anchor::Start
            } else {
                Anchor::End
            };
            Some(PlacedLabel { edge: e, lines, rect: spot.rect, anchor })
        })
        .collect()
}

fn loop_labels(g: &Geometry) -> Vec<PlacedLabel> {
    g.loops.iter().map(|l| PlacedLabel { edge: l.edge, lines: l.lines.clone(), rect: l.label, anchor: Anchor::Start }).collect()
}

/// Everything fixed before the leftover labels are placed.
fn occupancy(d: &Diagram, g: &Geometry, r: &Routes, placed: &[PlacedLabel]) -> Placer {
    let leaves = (1..d.nodes.len()).filter(|&n| d.nodes[n].children.is_empty() && d.nodes[n].kind != Kind::Region).map(|n| g.rects[n]);
    let outlines = (1..d.nodes.len()).filter(|&n| !d.nodes[n].children.is_empty() && d.nodes[n].kind != Kind::Region).flat_map(|n| {
        let b = g.rects[n];
        [Rect { x: b.x - 1.0, y: b.y, w: 2.0, h: b.h }, Rect { x: b.right() - 1.0, y: b.y, w: 2.0, h: b.h }, Rect { x: b.x, y: b.y - 1.0, w: b.w, h: 2.0 }, Rect { x: b.x, y: b.bottom() - 1.0, w: b.w, h: 2.0 }, Rect { x: b.x, y: b.y, w: b.w, h: header_height(&d.nodes[n]) }]
    });
    let base = Placer::default().occupy_all(leaves).occupy_all(outlines).occupy_all(placed.iter().map(|p| p.rect));
    let withloops = g.loops.iter().fold(base, |p, l| p.avoid_polyline(&l.points));
    r.edges.iter().filter(|points| points.len() >= 2).fold(withloops, |p, points| p.avoid_polyline(points))
}

/// A label with no reserved room, such as on a local transition, goes where it
/// collides least.
fn fallback_label(placer: Placer, d: &Diagram, r: &Routes, e: usize) -> (Placer, PlacedLabel) {
    let text_line = d.edges[e].label.as_ref().map_or_else(String::new, |label| label.text());
    let w = small_width(&text_line) + 4.0;
    let (placer, at) = placer.place_around(&r.edges[e], w);
    let left = match at.anchor {
        Anchor::Start => at.x,
        Anchor::Middle => at.x - w / 2.0,
        Anchor::End => at.x - w,
    };
    let lines = vec![text_line];
    let size = label_size(&lines);
    (placer, PlacedLabel { edge: e, lines, rect: Rect { x: left, y: at.y - 10.5, w: size.w, h: size.h }, anchor: Anchor::Start })
}

/// Every label with its final box, for drawing and for invariant checks:
/// reserved labels, self-transition labels, the labels `freed` placed after
/// routing (`hybrid`), then any left over.
pub fn all_labels(d: &Diagram, g: &Geometry, r: &Routes, freed: &[PlacedLabel]) -> Vec<PlacedLabel> {
    let placed: Vec<PlacedLabel> = reserved_labels(d, g).into_iter().chain(loop_labels(g)).chain(freed.iter().cloned()).collect();
    let leftover: Vec<usize> = (0..d.edges.len()).filter(|&e| d.edges[e].label.is_some() && !placed.iter().any(|p| p.edge == e) && d.edges[e].from != d.edges[e].to && r.edges[e].len() >= 2).collect();
    let (_, rest) = leftover.into_iter().fold((occupancy(d, g, r, &placed), Vec::new()), |(placer, mut rest), e| {
        let (placer, label) = fallback_label(placer, d, r, e);
        rest.push(label);
        (placer, rest)
    });
    placed.into_iter().chain(rest).collect()
}

pub fn render(d: &Diagram, g: &Geometry, r: &Routes, freed: &[PlacedLabel]) -> String {
    let nodes = draw_node(d, g, 0);
    let (_, lines) = draw_with((), d.edges.iter().zip(&r.edges), |(), (edge, points)| ((), draw_edge(edge, points, d.style)));
    let loops = loop_marks(g, d.style);
    let points = connection_points(d, g);
    let labels: Vec<Mark> = all_labels(d, g, r, freed).iter().flat_map(label_marks).collect();
    document(d.style, &[nodes, lines, loops, points, labels].concat(), d.title.as_deref(), &[Extra::Solid])
}
