// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Draws a laid-out sequence diagram as a list of marks.

use super::layout::*;
use super::model::*;
use crate::core::geometry::{Point, Rect};
use crate::core::glyphs::{arrowhead, filled_triangle, note_wrapped};
use crate::core::style::{Extra, Role, Style};
use crate::core::svg::{bound, circle, document, path, rect, text, Anchor, Mark, TextStyle};
use crate::core::text::{wrap, FONT};

/// The classes beyond the shared ones that a sequence diagram uses.
const EXTRAS: &[Extra] = &[Extra::Lifeline, Extra::Frame, Extra::Solid];

/// The `class` attribute of a rectangle with this role.
fn class(style: Style, role: Role) -> String {
    format!("class=\"{}\"", style.class(role))
}

fn stacked_lines(lines: &[String], x: f64, baseline: f64, style: TextStyle, anchor: Anchor) -> Vec<Mark> {
    let n = lines.len();
    lines.iter().enumerate().map(|(i, line)| text(x, baseline - (n - 1 - i) as f64 * LINE, line, style, anchor)).collect()
}

fn draw_arrow(a: &DrawnArrow, style: Style) -> Vec<Mark> {
    let (line_role, head_role) = match a.sort {
        Sort::Reply | Sort::Create => (Role::DashedLine, Role::DashedHead),
        Sort::Sync | Sort::Async => (Role::Line, Role::Head),
    };
    let line = path(&style.line(&a.points), style.class(line_role));
    let head = match a.sort {
        Sort::Sync => filled_triangle(&a.points, style.class(Role::Solid)),
        Sort::Async | Sort::Reply | Sort::Create => vec![arrowhead(&a.points, style.class(head_role))],
    };
    let dot: Vec<Mark> = a.dot.iter().map(|&c| circle(c, 4.0, style.class(Role::Solid))).collect();
    let bounds = vec![bound(a.points.clone()), bound(a.dot.iter().flat_map(|&(x, y)| [(x - 4.0, y - 4.0), (x + 4.0, y + 4.0)]).collect())];
    [bounds, vec![line], head, dot].concat()
}

fn draw_label(label: &Label) -> Vec<Mark> {
    stacked_lines(&label.lines, label.x, label.baseline, TextStyle::SmallHalo, label.anchor)
}

fn pentagon(tab: Rect, style: Style) -> Mark {
    let (x, y, right, bottom) = (tab.x, tab.y, tab.right(), tab.bottom());
    let slant = tab_slant();
    path(&format!("M{x} {y} L{right} {y} L{right} {} L{} {bottom} L{x} {bottom} Z", bottom - slant, right - slant), style.class(Role::FrameTab))
}

fn tab_label(tab: Rect, label: &str) -> Mark {
    text(tab.x + 8.0, tab.y + tab.h - 7.0, label, TextStyle::Keyword, Anchor::Start)
}

/// The frame outline and operand separators, drawn under the bars.
fn frame_back(frame: &DrawnFrame, style: Style) -> Vec<Mark> {
    let outline = rect(frame.rect, &class(style, Role::Frame));
    let separators: Vec<Mark> = frame
        .separators
        .iter()
        .map(|&y| path(&format!("M{} {y} L{} {y}", frame.rect.x, frame.rect.right()), style.class(Role::FrameRule)))
        .collect();
    [vec![outline], separators].concat()
}

/// The operator tab and guards, drawn over the bars so they stay legible.
fn frame_front(frame: &DrawnFrame, style: Style) -> Vec<Mark> {
    let guards: Vec<Mark> = frame
        .guards
        .iter()
        .flat_map(|g| stacked_lines(&g.lines, g.rect.x, g.rect.bottom() - 5.0, TextStyle::SmallHalo, Anchor::Start))
        .collect();
    [vec![pentagon(frame.tab, style), tab_label(frame.tab, &frame.operator)], guards].concat()
}

fn draw_ref(r: &DrawnRef, style: Style) -> Vec<Mark> {
    let body = rect(r.rect, &class(style, Role::RefFrame));
    let centre = r.rect.x + r.rect.w / 2.0;
    let total = r.lines.len() as f64 * LINE;
    let baseline = r.rect.y + (r.rect.h + total) / 2.0 - 5.0;
    let text_lines = stacked_lines(&r.lines, centre, baseline, TextStyle::Small, Anchor::Middle);
    [vec![body, pentagon(r.tab, style), tab_label(r.tab, "ref")], text_lines].concat()
}

fn stick_figure(cx: f64, top: f64, style: Style) -> Vec<Mark> {
    let (head, neck, hip, foot) = (top + 6.0, top + 12.0, top + 24.0, top + 36.0);
    vec![
        bound(vec![(cx - 12.0, top), (cx + 12.0, foot)]),
        circle((cx, head), 6.0, style.class(Role::Box)),
        path(&format!("M{cx} {neck} L{cx} {hip} M{} {} L{} {} M{cx} {hip} L{} {foot} M{cx} {hip} L{} {foot}", cx - 12.0, neck + 4.0, cx + 12.0, neck + 4.0, cx - 8.0, cx + 8.0), style.class(Role::Line)),
    ]
}

fn draw_head(p: &Participant, line: &Lifeline, style: Style) -> Vec<Mark> {
    let keyword = p.stereotype.as_ref().map(|s| format!("\u{ab}{s}\u{bb}"));
    let names = wrap(&p.name, FONT, 132.0);
    let cx = line.x;
    match p.figure {
        Figure::Box => {
            let body = rect(line.head, &class(style, Role::Box));
            let block = names.len() as f64 * 16.0 + keyword.as_ref().map_or(0.0, |_| 14.0);
            let top = line.head.y + (line.head.h - block) / 2.0;
            let kw_lines: Vec<Mark> = keyword.iter().map(|k| text(cx, top + 10.0, k, TextStyle::Small, Anchor::Middle)).collect();
            let first = top + keyword.as_ref().map_or(0.0, |_| 14.0) + 12.0;
            let name_lines = names.iter().enumerate().map(|(i, n)| text(cx, first + i as f64 * 16.0, n, TextStyle::Name, Anchor::Middle)).collect();
            [vec![body], kw_lines, name_lines].concat()
        }
        Figure::Actor => {
            let bottom = line.head.bottom();
            let text_block = names.len() as f64 * 16.0 + keyword.as_ref().map_or(0.0, |_| 14.0);
            let figure_top = bottom - text_block - 38.0;
            let names_top = bottom - names.len() as f64 * 16.0 + 12.0;
            let kw = keyword.iter().map(|k| text(cx, names_top - 18.0, k, TextStyle::Small, Anchor::Middle)).collect();
            let name_lines = names.iter().enumerate().map(|(i, n)| text(cx, names_top + i as f64 * 16.0, n, TextStyle::Name, Anchor::Middle)).collect();
            [stick_figure(cx, figure_top, style), kw, name_lines].concat()
        }
    }
}

fn draw_cross(&(x, y): &Point, style: Style) -> Vec<Mark> {
    let r = 7.0;
    vec![bound(vec![(x - r, y - r), (x + r, y + r)]), path(&format!("M{} {} L{} {} M{} {} L{} {}", x - r, y - r, x + r, y + r, x - r, y + r, x + r, y - r), style.class(Role::Line))]
}

/// Lifelines first, then frames, bars, heads and boxes, with arrows and labels on top.
pub fn render(d: &Diagram, g: &Layout) -> String {
    let style = d.style;
    let lines: Vec<Mark> = g.lifelines.iter().map(|l| path(&format!("M{} {} L{} {}", l.x, l.top, l.x, l.bottom), style.class(Role::Lifeline))).collect();
    let frames: Vec<Mark> = g.frames.iter().flat_map(|f| frame_back(f, style)).collect();
    let tabs: Vec<Mark> = g.frames.iter().flat_map(|f| frame_front(f, style)).collect();
    let bars: Vec<Mark> = g.bars.iter().map(|b| rect(b.rect, &class(style, Role::Box))).collect();
    let heads: Vec<Mark> = d.participants.iter().zip(&g.lifelines).flat_map(|(p, l)| draw_head(p, l, style)).collect();
    let refs: Vec<Mark> = g.refs.iter().flat_map(|r| draw_ref(r, style)).collect();
    let notes: Vec<Mark> = g.notes.iter().flat_map(|n| note_wrapped(n.rect, &n.text, n.wrap, style)).collect();
    let arrows: Vec<Mark> = g.arrows.iter().flat_map(|a| draw_arrow(a, style)).collect();
    let labels: Vec<Mark> = g.arrows.iter().filter_map(|a| a.label.as_ref()).flat_map(draw_label).collect();
    let crosses: Vec<Mark> = g.crosses.iter().flat_map(|c| draw_cross(c, style)).collect();
    let bottom: Vec<Mark> = g.lifelines.iter().map(|l| bound(vec![(l.x, l.bottom)])).collect();
    document(style, &[lines, frames, bars, tabs, heads, refs, notes, arrows, crosses, labels, bottom].concat(), d.title.as_deref(), EXTRAS)
}
