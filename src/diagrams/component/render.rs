// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Draws a laid-out component diagram as a list of marks.

use super::layout::{stub_port_label, PACKAGE_TAB, STUB};
use super::model::*;
use super::routing::{PortGeom, Routes, StubGeom};
use crate::core::geometry::{Direction, Point, Rect, Side};
use crate::core::glyphs::{arrowhead, folder, note, socket};
use crate::core::labels::Placer;
use crate::core::layered::Layout;
use crate::core::style::{Role, Style};
use crate::core::svg::{bound, circle, document, draw_with, n, path, rect, text, Anchor, Mark, TextStyle};
use crate::core::text::{name_width, small_width};

/// Plain line an interface glyph keeps on each side, up to the box edge or the
/// bend where its segment ends, so the glyph reads as a thing of its own between
/// two visible stubs (Drawing principles, Component diagrams: give an interface room).
pub const CLEARANCE: f64 = 16.0;
/// The longest half-length of a wired dependency, from its middle to the socket
/// or to the ball.
const WIRE_HALF: f64 = 24.0;
/// The socket's and the ball's reach along the line from their centres.
const SOCKET_R: f64 = 8.0;
const BALL_R: f64 = 5.0;

/// The straight run an interface connector's glyph needs on one segment: its
/// clearance, the glyph at full size, and its clearance again. Other
/// connectors need none.
pub fn glyph_run(kind: EdgeKind) -> f64 {
    match kind {
        EdgeKind::Wired => 2.0 * CLEARANCE + 2.0 * WIRE_HALF + SOCKET_R + BALL_R,
        EdgeKind::Assembly => 2.0 * CLEARANCE + SOCKET_R + BALL_R,
        _ => 0.0,
    }
}

fn guillemets(stereotype: &str) -> String {
    format!("\u{ab}{stereotype}\u{bb}")
}

fn component_icon(right: f64, top: f64, style: Style) -> Vec<Mark> {
    let attributes = format!("class=\"{}\"", style.class(Role::Icon));
    let (x, y) = (right - 24.0, top + 8.0);
    [Rect { x: x + 3.0, y, w: 13.0, h: 16.0 }, Rect { x, y: y + 3.0, w: 6.0, h: 3.5 }, Rect { x, y: y + 9.5, w: 6.0, h: 3.5 }]
        .into_iter()
        .map(|part| rect(part, &attributes))
        .collect()
}

fn component_labels(node: &Node, r: Rect) -> Vec<Mark> {
    let stereo = node.stereotype.as_deref().map(guillemets);
    let middle = r.x + r.w / 2.0;
    if !node.children.is_empty() {
        // 9.2.4: name and keywords are centred, keywords above the name.
        let name_y = r.y + if stereo.is_some() { 40.0 } else { 24.0 };
        let stereo_mark = stereo.iter().map(|s| text(middle, r.y + 22.0, s, TextStyle::Small, Anchor::Middle));
        return stereo_mark.chain([text(middle, name_y, &node.name, TextStyle::Name, Anchor::Middle)]).collect();
    }
    let baseline = r.y + r.h / 2.0 + 4.5;
    match stereo {
        Some(s) => vec![
            text(middle, baseline - 9.0, &s, TextStyle::Small, Anchor::Middle),
            text(middle, baseline + 8.0, &node.name, TextStyle::Name, Anchor::Middle),
        ],
        None => vec![text(middle, baseline, &node.name, TextStyle::Name, Anchor::Middle)],
    }
}

/// The outline role of a node: a box that holds others is a container.
fn outline(node: &Node) -> Role {
    if node.children.is_empty() { Role::Box } else { Role::Container }
}

fn node_shape(node: &Node, r: Rect, style: Style) -> Vec<Mark> {
    match node.kind {
        Kind::Root => Vec::new(),
        Kind::Package => {
            let tab = name_width(&node.name) + 24.0;
            let label = text(r.x + 12.0, r.y + 16.5, &node.name, TextStyle::Name, Anchor::Start);
            folder(r, tab, PACKAGE_TAB, style.class(outline(node))).into_iter().chain([label]).collect()
        }
        Kind::Note => note(r, &node.name, style),
        Kind::Component => {
            let body = rect(r, &format!("class=\"{}\"", style.class(outline(node))));
            [vec![body], component_icon(r.right(), r.y, style), component_labels(node, r)].concat()
        }
    }
}

/// A node's marks, then its children's, so containers paint behind contents.
fn draw_node(d: &Diagram, g: &Layout, i: usize) -> Vec<Mark> {
    let own = node_shape(&d.nodes[i], g.rects[i], d.style);
    own.into_iter().chain(d.nodes[i].children.iter().flat_map(|&child| draw_node(d, g, child))).collect()
}

fn draw_stub(placer: Placer, d: &Diagram, geometry: &StubGeom) -> (Placer, Vec<Mark>) {
    let stub = &d.stubs[geometry.stub];
    let (dx, dy) = geometry.side.out();
    let length = STUB + stub_port_label(d, stub);
    let base = geometry.base;
    let tip = (base.0 + dx * length, base.1 + dy * length);
    let style = d.style;
    let line = path(&format!("M{} {} L{} {}", n(base.0), n(base.1), n(tip.0), n(tip.1)), style.class(Role::Line));
    let glyph = match stub.role {
        super::model::Role::Provided => vec![circle((tip.0 + dx * 6.0, tip.1 + dy * 6.0), 6.0, style.class(Role::Box))],
        super::model::Role::Required => socket((tip.0 + dx * 8.0, tip.1 + dy * 8.0), (dx, dy), style.class(Role::Line)),
    };
    let w = small_width(&stub.interface);
    let middle = (tip.0 + dx * 6.0, tip.1 + dy * 6.0);
    let (label, occupied) = if geometry.side.is_vertical_edge() {
        (text(middle.0, middle.1 - 12.0, &stub.interface, TextStyle::Small, Anchor::Middle), Rect { x: middle.0 - w / 2.0, y: middle.1 - 24.0, w, h: 14.0 })
    } else if geometry.name_beyond {
        let baseline = if dy < 0.0 { middle.1 - 12.0 } else { middle.1 + 20.0 };
        (text(middle.0, baseline, &stub.interface, TextStyle::Small, Anchor::Middle), Rect { x: middle.0 - w / 2.0, y: baseline - 12.0, w, h: 14.0 })
    } else {
        (text(middle.0 + 12.0, middle.1 + 4.0, &stub.interface, TextStyle::Small, Anchor::Start), Rect { x: middle.0 + 10.0, y: middle.1 - 7.0, w, h: 14.0 })
    };
    let marks = [vec![bound(vec![(tip.0 + dx * 12.0, tip.1 + dy * 12.0)]), line], glyph, vec![label]].concat();
    (placer.occupy(occupied), marks)
}

fn segment_length(a: Point, b: Point) -> f64 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

/// Index of the longest segment. Earlier segments win ties.
fn longest(points: &[Point]) -> usize {
    (0..points.len() - 1).fold((0, -1.0), |(best, length), i| {
        let l = segment_length(points[i], points[i + 1]);
        if l > length { (i, l) } else { (best, length) }
    }).0
}

/// Index of the segment for a ball-and-socket glyph. Segments along the flow
/// keep the glyph and its name clear of container boundaries.
pub fn glyph_segment(points: &[Point], direction: Direction) -> usize {
    let flow_horizontal = direction == Direction::Right;
    let along_flow = (0..points.len() - 1).filter(|&i| {
        let (a, b) = (points[i], points[i + 1]);
        (a.1 == b.1) == flow_horizontal && segment_length(a, b) >= 56.0
    });
    along_flow
        .fold(None, |best: Option<(usize, f64)>, i| {
            let l = segment_length(points[i], points[i + 1]);
            match best {
                Some((_, length)) if l <= length => best,
                _ => Some((i, l)),
            }
        })
        .map_or_else(|| longest(points), |(i, _)| i)
}

fn draw_assembly(placer: Placer, points: &[Point], label: &str, direction: Direction, style: Style) -> (Placer, Vec<Mark>) {
    let i = glyph_segment(points, direction);
    let (a, b) = (points[i], points[i + 1]);
    let l = segment_length(a, b);
    let (dx, dy) = ((b.0 - a.0) / l.max(0.001), (b.1 - a.1) / l.max(0.001));
    let w = small_width(label);
    let horizontal = a.1 == b.1;
    let label_box = |c: Point| {
        if horizontal {
            Rect { x: c.0 - w / 2.0, y: c.1 - 22.0, w, h: 12.0 }
        } else {
            Rect { x: c.0 + 11.0, y: c.1 - 7.0, w, h: 14.0 }
        }
    };
    let glyph_box = |c: Point| Rect { x: c.0 - 9.0, y: c.1 - 9.0, w: 18.0, h: 18.0 };
    // Slide the glyph along its segment until the interface name fits, keeping
    // its clearance from both ends of the segment where the segment allows.
    let (lo, hi) = (CLEARANCE + SOCKET_R, l - CLEARANCE - BALL_R);
    let at = |f: f64| {
        let along = if lo <= hi { (l * f).clamp(lo, hi) } else { l / 2.0 };
        (a.0 + dx * along, a.1 + dy * along)
    };
    let c = [0.5, 0.6, 0.4, 0.7, 0.3, 0.8]
        .map(at)
        .into_iter()
        .find(|&c| !placer.is_occupied(&label_box(c)) && !placer.is_on_name(&glyph_box(c)))
        .unwrap_or_else(|| at(0.5));
    let (ball_r, socket_r) = (BALL_R, SOCKET_R);
    // The arc bulges toward the requirer. It opens toward the provider.
    let toward_requirer = (c.0 - dx * socket_r, c.1 - dy * socket_r);
    let toward_provider = (c.0 + dx * ball_r, c.1 + dy * ball_r);
    let first: Vec<Point> = points[..=i].iter().copied().chain([toward_requirer]).collect();
    let second: Vec<Point> = std::iter::once(toward_provider).chain(points[i + 1..].iter().copied()).collect();
    let (x, y, anchor) = if horizontal { (c.0, c.1 - 13.0, Anchor::Middle) } else { (c.0 + 13.0, c.1 + 4.0, Anchor::Start) };
    let marks = [
        vec![path(&style.line(&first), style.class(Role::Line)), path(&style.line(&second), style.class(Role::Line))],
        socket(c, (dx, dy), style.class(Role::Line)),
        vec![circle(c, ball_r, style.class(Role::Box)), text(x, y, label, TextStyle::Small, anchor)],
    ]
    .concat();
    (placer.occupy(label_box(c)).occupy(glyph_box(c)).name(label_box(c)), marks)
}

/// A dependency from a socket to a lollipop drawn apart on one segment, for ends
/// that may not use ball-and-socket (10.4.4, Figure 11.46).
fn draw_wired(placer: Placer, points: &[Point], label: &str, direction: Direction, style: Style) -> (Placer, Vec<Mark>) {
    let i = glyph_segment(points, direction);
    let (a, b) = (points[i], points[i + 1]);
    let l = segment_length(a, b);
    let (dx, dy) = ((b.0 - a.0) / l.max(0.001), (b.1 - a.1) / l.max(0.001));
    // On a short segment the dependency shortens before the stubs do.
    let (socket_r, ball_r) = (SOCKET_R, BALL_R);
    let half = ((l - 2.0 * CLEARANCE - socket_r - ball_r) / 2.0).clamp(12.0, WIRE_HALF);
    let w = small_width(label);
    let horizontal = a.1 == b.1;
    // A run may have another connector beside it, so a name can take either
    // side (above first on a horizontal run), and on a vertical run step along
    // it to clear a neighbouring name.
    let label_box = |c: Point, far: bool, step: f64| match (horizontal, far) {
        (true, false) => Rect { x: c.0 - w / 2.0, y: c.1 - 22.0, w, h: 12.0 },
        (true, true) => Rect { x: c.0 - w / 2.0, y: c.1 + 10.0, w, h: 12.0 },
        (false, false) => Rect { x: c.0 + 11.0, y: c.1 - 7.0 + step, w, h: 14.0 },
        (false, true) => Rect { x: c.0 - 11.0 - w, y: c.1 - 7.0 + step, w, h: 14.0 },
    };
    let glyph_box = |c: Point| Rect { x: c.0 - half - 9.0, y: c.1 - 9.0, w: 2.0 * half + 18.0, h: 18.0 };
    let (lo, hi) = (CLEARANCE + socket_r + half, l - CLEARANCE - ball_r - half);
    let at = |f: f64| {
        let along = if lo < hi { (l * f).clamp(lo, hi) } else { l / 2.0 };
        (a.0 + dx * along, a.1 + dy * along)
    };
    let fits = |c: Point, far: bool, step: f64| !placer.is_occupied(&label_box(c, far, step).inflate(6.0)) && !placer.is_on_name(&glyph_box(c));
    let spots = [0.5, 0.6, 0.4, 0.7, 0.3].map(at);
    let tried = [(false, 0.0), (true, 0.0), (false, 16.0), (false, -16.0), (true, 16.0), (true, -16.0)].into_iter().flat_map(|(far, step)| spots.iter().map(move |&c| (c, far, step)));
    let (c, far, step) = tried.into_iter().find(|&(c, far, step)| fits(c, far, step)).unwrap_or((at(0.5), false, 0.0));
    let socket_at = (c.0 - dx * half, c.1 - dy * half);
    let ball_at = (c.0 + dx * half, c.1 + dy * half);
    let bulge = (socket_at.0 - dx * socket_r, socket_at.1 - dy * socket_r);
    let ball_edge = (ball_at.0 + dx * ball_r, ball_at.1 + dy * ball_r);
    let tip = (ball_at.0 - dx * ball_r, ball_at.1 - dy * ball_r);
    let first: Vec<Point> = points[..=i].iter().copied().chain([bulge]).collect();
    let second: Vec<Point> = std::iter::once(ball_edge).chain(points[i + 1..].iter().copied()).collect();
    let arrow = [socket_at, tip];
    let (x, y, anchor) = match (horizontal, far) {
        (true, false) => (c.0, c.1 - 13.0, Anchor::Middle),
        (true, true) => (c.0, c.1 + 20.0, Anchor::Middle),
        (false, false) => (c.0 + 13.0, c.1 + 4.0 + step, Anchor::Start),
        (false, true) => (c.0 - 13.0, c.1 + 4.0 + step, Anchor::End),
    };
    let marks = [
        vec![
            path(&style.line(&first), style.class(Role::Line)),
            path(&style.line(&second), style.class(Role::Line)),
            path(&style.line(&arrow), style.class(Role::DashedLine)),
            arrowhead(&arrow, style.class(Role::DashedHead)),
        ],
        socket(socket_at, (dx, dy), style.class(Role::Line)),
        vec![circle(ball_at, ball_r, style.class(Role::Box)), text(x, y, label, TextStyle::Small, anchor)],
    ]
    .concat();
    (placer.occupy(label_box(c, far, step)).occupy(glyph_box(c)).name(label_box(c, far, step)), marks)
}

/// A port name's text position, anchor and occupied box.
pub fn port_label(d: &Diagram, port: &PortGeom) -> (f64, f64, Anchor, Rect) {
    let (x, y) = port.center;
    let w = small_width(&d.ports[port.port].name);
    match port.side {
        Side::Left => (x - 10.0, y - 9.0, Anchor::End, Rect { x: x - 10.0 - w, y: y - 20.0, w, h: 14.0 }),
        Side::Right => (x + 10.0, y - 9.0, Anchor::Start, Rect { x: x + 10.0, y: y - 20.0, w, h: 14.0 }),
        Side::Top => (x + 10.0, y - 9.0, Anchor::Start, Rect { x: x + 10.0, y: y - 20.0, w, h: 14.0 }),
        Side::Bottom => (x + 10.0, y + 18.0, Anchor::Start, Rect { x: x + 10.0, y: y + 7.0, w, h: 14.0 }),
    }
}

fn draw_edge(placer: Placer, d: &Diagram, edge: &Edge, points: &[Point]) -> (Placer, Vec<Mark>) {
    if points.len() < 2 {
        return (placer, Vec::new());
    }
    let extent = bound(points.to_vec());
    let style = d.style;
    let (line_role, head_role) = match edge.kind {
        EdgeKind::Assembly => {
            let (placer, marks) = draw_assembly(placer, points, edge.label.as_deref().unwrap_or(""), d.direction, d.style);
            return (placer, std::iter::once(extent).chain(marks).collect());
        }
        EdgeKind::Wired => {
            let (placer, marks) = draw_wired(placer, points, edge.label.as_deref().unwrap_or(""), d.direction, d.style);
            return (placer, std::iter::once(extent).chain(marks).collect());
        }
        EdgeKind::Dashed | EdgeKind::NoteLink => (Role::DashedLine, Role::DashedHead),
        EdgeKind::Solid => (Role::Line, Role::Head),
    };
    let line = path(&style.line(points), style.class(line_role));
    let head = match edge.head {
        Head::Open => Some(arrowhead(points, style.class(head_role))),
        Head::None => None,
    };
    (placer, [extent, line].into_iter().chain(head).collect())
}

fn draw_port(d: &Diagram, port: &PortGeom) -> Vec<Mark> {
    let (x, y) = port.center;
    let (tx, ty, anchor, _) = port_label(d, port);
    vec![rect(Rect { x: x - 6.0, y: y - 6.0, w: 12.0, h: 12.0 }, &format!("class=\"{}\"", d.style.class(Role::Box))), text(tx, ty, &d.ports[port.port].name, TextStyle::Small, anchor)]
}

fn draw_edge_label(placer: Placer, edge: &Edge, points: &[Point]) -> (Placer, Vec<Mark>) {
    let label = match (edge.kind, points.len() < 2, &edge.label) {
        (EdgeKind::Assembly | EdgeKind::Wired, _, _) | (_, true, _) | (_, _, None) => return (placer, Vec::new()),
        (_, false, Some(label)) => label,
    };
    let (placer, at) = placer.place(points, small_width(label) + 4.0);
    (placer, vec![text(at.x, at.y, label, TextStyle::SmallHalo, at.anchor)])
}

/// Everything fixed before movable labels: leaf boxes, lines, container
/// outlines and port names.
fn fixed_occupancy(d: &Diagram, g: &Layout, r: &Routes) -> Placer {
    let leaves = d.nodes.iter().zip(&g.rects).filter(|(node, _)| node.children.is_empty() && node.kind != Kind::Root).map(|(_, &rect)| rect);
    let port_names = r.ports.iter().map(|port| port_label(d, port).3);
    // A package's name tab is drawn like a box, so labels keep off it too.
    let tabs = (1..d.nodes.len()).filter(|&n| d.nodes[n].kind == Kind::Package).map(|n| Rect { h: PACKAGE_TAB, w: name_width(&d.nodes[n].name) + 24.0, ..g.rects[n] });
    let placer = Placer::default().occupy_all(leaves).occupy_all(port_names).occupy_all(tabs);
    let outlines = (1..d.nodes.len()).filter(|&n| !d.nodes[n].children.is_empty()).map(|n| {
        let top = if d.nodes[n].kind == Kind::Package { PACKAGE_TAB } else { 0.0 };
        let b = g.rects[n];
        vec![(b.x, b.y + top), (b.right(), b.y + top), (b.right(), b.bottom()), (b.x, b.bottom()), (b.x, b.y + top)]
    });
    r.edges.iter().cloned().chain(outlines).fold(placer, |placer, points| placer.avoid_polyline(&points))
}

/// The drawing in its parts: connector marks and connector labels are kept per
/// edge, so the quality test can tell which line a label belongs to.
pub struct Parts {
    pub nodes: Vec<Mark>,
    pub stubs: Vec<Mark>,
    pub edges: Vec<Vec<Mark>>,
    pub ports: Vec<Mark>,
    pub labels: Vec<Vec<Mark>>,
}

/// Runs `step` over `items`, threading a state through, one list of marks per item.
fn draw_each<S, I>(state: S, items: impl IntoIterator<Item = I>, mut step: impl FnMut(S, I) -> (S, Vec<Mark>)) -> (S, Vec<Vec<Mark>>) {
    items.into_iter().fold((state, Vec::new()), |(state, mut all), item| {
        let (state, marks) = step(state, item);
        all.push(marks);
        (state, all)
    })
}

pub fn parts(d: &Diagram, g: &Layout, r: &Routes) -> Parts {
    let nodes = draw_node(d, g, 0);
    let placer = fixed_occupancy(d, g, r);
    let (placer, stubs) = draw_with(placer, &r.stubs, |placer, stub| draw_stub(placer, d, stub));
    let (placer, edges) = draw_each(placer, d.edges.iter().zip(&r.edges), |placer, (edge, points)| draw_edge(placer, d, edge, points));
    let ports: Vec<Mark> = r.ports.iter().flat_map(|port| draw_port(d, port)).collect();
    let (_, labels) = draw_each(placer, d.edges.iter().zip(&r.edges), |placer, (edge, points)| draw_edge_label(placer, edge, points));
    Parts { nodes, stubs, edges, ports, labels }
}

pub fn render(d: &Diagram, g: &Layout, r: &Routes) -> String {
    let Parts { nodes, stubs, edges, ports, labels } = parts(d, g, r);
    document(d.style, &[nodes, stubs, edges.concat(), ports, labels.concat()].concat(), d.title.as_deref(), &[])
}
