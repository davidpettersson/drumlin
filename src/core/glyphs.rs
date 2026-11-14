// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Line work and notation shapes shared by the diagram kinds.

use super::geometry::{snap_up, Point, Rect};
use super::style::{Role, Style};
use super::svg::{self, n, Anchor, Mark, TextStyle};
use super::text::{width, wrap, Weight, SMALL};
use std::fmt::Write;

pub const NOTE_WRAP: f64 = 168.0;
const NOTE_LINE: f64 = 15.0;

fn manhattan(u: Point, v: Point) -> f64 {
    (u.0 - v.0).abs() + (u.1 - v.1).abs()
}

/// Path data for an orthogonal polyline with softened corners.
pub fn rounded(points: &[Point]) -> String {
    let start = format!("M{} {}", n(points[0].0), n(points[0].1));
    let corners = points.windows(3).fold(start, |mut out, w| {
        let (a, p, c) = (w[0], w[1], w[2]);
        let radius = 6.0f64.min(manhattan(a, p) / 2.0).min(manhattan(p, c) / 2.0);
        let toward = |to: Point| {
            let l = manhattan(p, to).max(0.001);
            (p.0 + (to.0 - p.0) / l * radius, p.1 + (to.1 - p.1) / l * radius)
        };
        let (enter, leave) = (toward(a), toward(c));
        write!(out, " L{} {} Q{} {} {} {}", n(enter.0), n(enter.1), n(p.0), n(p.1), n(leave.0), n(leave.1)).unwrap();
        out
    });
    let last = points[points.len() - 1];
    format!("{corners} L{} {}", n(last.0), n(last.1))
}

/// The length of an open arrowhead, and of a filled triangle, along its line.
pub const ARROWHEAD: f64 = 10.0;
/// The length of a hollow triangle along its line.
pub const TRIANGLE: f64 = 12.0;

/// An open arrowhead at the end of a polyline, with the given class.
pub fn arrowhead(points: &[Point], class: &str) -> Mark {
    let (tip, before) = (points[points.len() - 1], points[points.len() - 2]);
    let l = ((tip.0 - before.0).powi(2) + (tip.1 - before.1).powi(2)).sqrt().max(0.001);
    let (dx, dy) = ((tip.0 - before.0) / l, (tip.1 - before.1) / l);
    let (back, half) = (ARROWHEAD, 4.5);
    let a = (tip.0 - dx * back - dy * half, tip.1 - dy * back + dx * half);
    let b = (tip.0 - dx * back + dy * half, tip.1 - dy * back - dx * half);
    svg::path(&format!("M{} {} L{} {} L{} {}", n(a.0), n(a.1), n(tip.0), n(tip.1), n(b.0), n(b.1)), class)
}

/// A closed hollow triangle at the end of a polyline, drawn over the line's end.
pub fn hollow_triangle(points: &[Point], class: &str) -> Vec<Mark> {
    let (tip, before) = (points[points.len() - 1], points[points.len() - 2]);
    let l = ((tip.0 - before.0).powi(2) + (tip.1 - before.1).powi(2)).sqrt().max(0.001);
    let (dx, dy) = ((tip.0 - before.0) / l, (tip.1 - before.1) / l);
    let (back, half) = (TRIANGLE, 7.0);
    let a = (tip.0 - dx * back - dy * half, tip.1 - dy * back + dx * half);
    let b = (tip.0 - dx * back + dy * half, tip.1 - dy * back - dx * half);
    vec![svg::bound(vec![a, b]), svg::path(&format!("M{} {} L{} {} L{} {} Z", n(a.0), n(a.1), n(tip.0), n(tip.1), n(b.0), n(b.1)), class)]
}

/// A circle of radius `r` with a plus inside, centred at `c`.
pub fn circled_plus(c: Point, r: f64, style: Style) -> Vec<Mark> {
    vec![
        svg::bound(vec![(c.0 - r, c.1 - r), (c.0 + r, c.1 + r)]),
        svg::circle(c, r, style.class(Role::Box)),
        svg::path(&format!("M{} {} L{} {} M{} {} L{} {}", n(c.0 - r), n(c.1), n(c.0 + r), n(c.1), n(c.0), n(c.1 - r), n(c.0), n(c.1 + r)), style.class(Role::Line)),
    ]
}

/// A half circle of radius 8 centred at `c`, opening along the unit vector `open`.
pub fn socket(c: Point, open: Point, class: &str) -> Vec<Mark> {
    let r = 8.0;
    let (px, py) = (-open.1, open.0);
    let a = (c.0 + px * r, c.1 + py * r);
    let b = (c.0 - px * r, c.1 - py * r);
    vec![svg::bound(vec![a, b]), svg::path(&format!("M{} {} A{r} {r} 0 0 1 {} {}", n(a.0), n(a.1), n(b.0), n(b.1)), class)]
}

/// A folder outline: a tab above a body, both inside `r`, with the given class.
pub fn folder(r: Rect, tab_width: f64, tab_height: f64, class: &str) -> Vec<Mark> {
    let attributes = format!("class=\"{class}\"");
    vec![
        svg::rect(Rect { x: r.x, y: r.y, w: tab_width, h: tab_height }, &attributes),
        svg::rect(Rect { x: r.x, y: r.y + tab_height, w: r.w, h: r.h - tab_height }, &attributes),
    ]
}

/// The size of the note drawn by `note` for this text.
pub fn note_size(content: &str) -> (f64, f64) {
    note_size_wrapped(content, NOTE_WRAP)
}

/// A sheet with a folded corner holding wrapped, vertically centred text.
pub fn note(r: Rect, content: &str, style: Style) -> Vec<Mark> {
    note_wrapped(r, content, NOTE_WRAP, style)
}

/// A folded sheet whose text wraps at `wrap_at`, for notes wider than the default.
pub fn note_wrapped(r: Rect, content: &str, wrap_at: f64, style: Style) -> Vec<Mark> {
    let fold = 10.0;
    let (x, y, right, bottom) = (r.x, r.y, r.right(), r.bottom());
    let lines = wrap(content, SMALL, wrap_at);
    let start = r.y + (r.h - lines.len() as f64 * NOTE_LINE) / 2.0 + 11.0;
    let outline = [
        svg::bound(vec![(x, y), (right, bottom)]),
        svg::path(&format!("M{} {} L{} {} L{} {} L{} {} L{} {} Z", n(x), n(y), n(right - fold), n(y), n(right), n(y + fold), n(right), n(bottom), n(x), n(bottom)), style.class(Role::Note)),
        svg::path(&format!("M{} {} L{} {} L{} {}", n(right - fold), n(y), n(right - fold), n(y + fold), n(right), n(y + fold)), style.class(Role::NoteFold)),
    ];
    let text = lines.iter().enumerate().map(|(index, line)| svg::text(r.x + 12.0, start + index as f64 * NOTE_LINE, line, TextStyle::Small, Anchor::Start));
    outline.into_iter().chain(text).collect()
}

/// A filled triangle at the end of a polyline.
pub fn filled_triangle(points: &[Point], class: &str) -> Vec<Mark> {
    let (tip, before) = (points[points.len() - 1], points[points.len() - 2]);
    let l = ((tip.0 - before.0).powi(2) + (tip.1 - before.1).powi(2)).sqrt().max(0.001);
    let (dx, dy) = ((tip.0 - before.0) / l, (tip.1 - before.1) / l);
    let (back, half) = (ARROWHEAD, 4.5);
    let a = (tip.0 - dx * back - dy * half, tip.1 - dy * back + dx * half);
    let b = (tip.0 - dx * back + dy * half, tip.1 - dy * back - dx * half);
    vec![svg::bound(vec![a, b]), svg::path(&format!("M{} {} L{} {} L{} {} Z", n(a.0), n(a.1), n(tip.0), n(tip.1), n(b.0), n(b.1)), class)]
}

/// The size of a note whose text wraps at `wrap_at`.
pub fn note_size_wrapped(content: &str, wrap_at: f64) -> (f64, f64) {
    let lines = wrap(content, SMALL, wrap_at);
    let max = lines.iter().map(|line| width(line, SMALL, Weight::Regular)).fold(0.0, f64::max);
    (snap_up(max + 32.0, 16.0), snap_up(lines.len() as f64 * NOTE_LINE + 16.0, 16.0))
}
