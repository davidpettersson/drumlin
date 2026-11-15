// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! SVG output as data. Drawing functions return `Mark`s, and `document` assembles
//! them with the stylesheet of the chosen `Style`: ink lines, white fills, no
//! shadows.

use super::geometry::{Point, Rect};
use super::style::{Extra, Style};
use super::lettering::{self, Hand};
use super::text::{escape, face, keyword_width, width, Face, Weight, FONT, SMALL, TITLE};

const MARGIN: f64 = 24.0;

/// Formats a coordinate with at most two decimals.
pub fn n(v: f64) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    format!("{rounded}")
}

/// A piece of SVG and the points it adds to the drawing's bounds.
#[derive(Clone, Debug, Default)]
pub struct Mark {
    svg: String,
    extents: Vec<Point>,
    /// The ink box of a text mark, for quality measures.
    ink: Option<Rect>,
}

impl Mark {
    /// The ink box of a text mark: cap height to descender over its width.
    pub fn ink(&self) -> Option<Rect> {
        self.ink
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

impl Anchor {
    fn as_str(self) -> &'static str {
        match self {
            Anchor::Start => "start",
            Anchor::Middle => "middle",
            Anchor::End => "end",
        }
    }
}

/// Text styles of the shared stylesheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextStyle {
    Name,
    /// A name in italics, for an abstract classifier (9.2.4).
    NameItalic,
    /// A keyword set like a name, such as a fragment's operator: bold in
    /// Helvetica, lower case in lettering.
    Keyword,
    Small,
    /// Small text with a white halo, for labels drawn over lines.
    SmallHalo,
    /// A keyword or stereotype, such as `«component»`: small text that
    /// classifies. `pens` sets it a step smaller and softer.
    Stereotype,
    /// Small text that comments rather than states: note text, region and
    /// pseudostate names. `pens` sets it softer.
    Aside,
}

impl TextStyle {
    fn class(self) -> &'static str {
        match self {
            TextStyle::Name | TextStyle::Keyword => "name",
            TextStyle::NameItalic => "name italic",
            TextStyle::Small => "small",
            TextStyle::SmallHalo => "small halo",
            TextStyle::Stereotype => "small tag",
            TextStyle::Aside => "small aside",
        }
    }
    fn metrics(self) -> (f64, Weight) {
        match self {
            TextStyle::Name | TextStyle::NameItalic | TextStyle::Keyword => (FONT, Weight::Bold),
            TextStyle::Small | TextStyle::SmallHalo | TextStyle::Stereotype | TextStyle::Aside => (SMALL, Weight::Regular),
        }
    }
}

/// A mark that draws nothing and only grows the drawing's bounds.
pub fn bound(points: Vec<Point>) -> Mark {
    Mark { svg: String::new(), extents: points, ink: None }
}

/// `attributes` is written verbatim, for example `class="shape" rx="2"`.
pub fn rect(r: Rect, attributes: &str) -> Mark {
    Mark {
        svg: format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {attributes}/>\n", n(r.x), n(r.y), n(r.w), n(r.h)),
        extents: vec![(r.x, r.y), (r.right(), r.bottom())],
        ink: None,
    }
}

/// A path with the given data. Paths do not grow the bounds: add a `bound`.
pub fn path(data: &str, class: &str) -> Mark {
    Mark { svg: format!("<path d=\"{data}\" class=\"{class}\"/>\n"), extents: Vec::new(), ink: None }
}

/// A circle. Circles do not grow the bounds: add a `bound`.
pub fn circle(c: Point, r: f64, class: &str) -> Mark {
    Mark { svg: format!("<circle cx=\"{}\" cy=\"{}\" r=\"{}\" class=\"{class}\"/>\n", n(c.0), n(c.1), n(r)), extents: Vec::new(), ink: None }
}

pub fn text(x: f64, y: f64, s: &str, style: TextStyle, anchor: Anchor) -> Mark {
    let (size, weight) = style.metrics();
    let w = if style == TextStyle::Keyword { keyword_width(s) } else { width(s, size, weight) };
    let left = match anchor {
        Anchor::Start => x,
        Anchor::Middle => x - w / 2.0,
        Anchor::End => x - w,
    };
    if face() == Face::Lettering {
        return lettered(left, y, w, s, style);
    }
    Mark {
        svg: format!("<text x=\"{}\" y=\"{}\" class=\"{}\" text-anchor=\"{}\">{}</text>\n", n(x), n(y), style.class(), anchor.as_str(), escape(s)),
        extents: vec![(left, y - size), (left + w, y + 3.0)],
        ink: Some(super::metrics::ink(left, w, y, y, size)),
    }
}

/// A text mark drawn in pen strokes (`core::lettering`): the same box as
/// the `<text>` it replaces, widened by the lean of inclined lettering.
fn lettered(left: f64, y: f64, w: f64, s: &str, style: TextStyle) -> Mark {
    let (size, _) = style.metrics();
    let italic = style == TextStyle::NameItalic;
    let lean = if italic { size * 0.3 } else { 0.0 };
    let hand = match style {
        TextStyle::Keyword => Hand::Keyword,
        TextStyle::Stereotype => Hand::Tag,
        _ => Hand::for_size(size),
    };
    Mark {
        svg: lettering::run(left, y, s, hand, italic, style == TextStyle::SmallHalo),
        extents: vec![(left - lean, y - size), (left + w + lean, y + 3.0)],
        ink: Some(super::metrics::ink(left, w, y, y, size)),
    }
}

/// Runs `step` over `items`, threading a state through and collecting the marks.
pub fn draw_with<S, I>(state: S, items: impl IntoIterator<Item = I>, mut step: impl FnMut(S, I) -> (S, Vec<Mark>)) -> (S, Vec<Mark>) {
    items.into_iter().fold((state, Vec::new()), |(state, mut marks), item| {
        let (state, new) = step(state, item);
        marks.extend(new);
        (state, marks)
    })
}

/// The style's rules for these extras, each scoped to the style's root class
/// so an inlined diagram cannot affect the page around it.
fn stylesheet(style: Style, extras: &[Extra]) -> String {
    style.rules(extras).iter().map(|rule| format!("{} {} {{ {} }}\n", style.scope(), rule.selector, rule.declarations)).collect()
}

/// The width and height that a document from `document` gives itself, for the
/// size measures: the first `width` and `height` attributes, on the `svg` tag.
#[cfg(test)]
pub fn frame(document: &str) -> Option<(f64, f64)> {
    let attribute = |name: &str| -> Option<f64> {
        let start = document.find(&format!(" {name}=\""))? + name.len() + 3;
        document[start..].split('"').next()?.parse().ok()
    };
    Some((attribute("width")?, attribute("height")?))
}

/// Assembles a standalone SVG document in `style`. `extras` names the classes
/// beyond the shared ones that the marks use.
pub fn document(style: Style, marks: &[Mark], title: Option<&str>, extras: &[Extra]) -> String {
    let (min, max) = marks.iter().flat_map(|mark| &mark.extents).fold(
        ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)),
        |(min, max): (Point, Point), &(x, y)| ((min.0.min(x), min.1.min(y)), (max.0.max(x), max.1.max(y))),
    );
    let left = min.0 - MARGIN;
    let (top, title_svg, right) = match title {
        None => (min.1 - MARGIN, String::new(), max.0),
        Some(title) => {
            let top = min.1 - MARGIN - 32.0;
            let svg = match face() {
                Face::Lettering => lettering::run(left + MARGIN, top + MARGIN + 12.0, title, Hand::Title, false, false).trim_end().to_string(),
                Face::Helvetica => format!("<text x=\"{}\" y=\"{}\" class=\"title\">{}</text>", n(left + MARGIN), n(top + MARGIN + 12.0), escape(title)),
            };
            let needed = width(title, TITLE, Weight::Bold) + left + 2.0 * MARGIN;
            (top, svg, max.0.max(needed - MARGIN))
        }
    };
    let (x, y, w, h) = (n(left), n(top), n(right + MARGIN - left), n(max.1 + MARGIN - top));
    let (root, sheet) = (style.root(), stylesheet(style, extras));
    let body: String = marks.iter().map(|mark| mark.svg.as_str()).collect();
    let defs = match face() {
        Face::Lettering => lettering::defs(&format!("{title_svg}{body}")),
        Face::Helvetica => String::new(),
    };
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"{root}\" viewBox=\"{x} {y} {w} {h}\" width=\"{w}\" height=\"{h}\">\n<style>\n{sheet}</style>\n{defs}<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"#fff\"/>\n{title_svg}\n{body}</svg>\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_reads_the_size_on_the_svg_tag() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"drumlin\" viewBox=\"0 0 343.79 120\" width=\"343.79\" height=\"120\">\n<rect x=\"0\" y=\"0\" width=\"9\" height=\"9\"/>";
        assert_eq!(frame(svg), Some((343.79, 120.0)));
        assert_eq!(frame("<svg>"), None);
    }
}
