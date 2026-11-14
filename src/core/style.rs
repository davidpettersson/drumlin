// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Drawing styles. Layout decides where every mark goes; a style decides how
//! the marks are stroked: the ink, the line weights, the dashes and the joins.
//! Rendering receives one `Style` and asks it for three things: the class of a
//! mark from the role the mark plays (`Role`), the path data of a connector
//! line, and the stylesheet that gives those classes their strokes.
//!
//! `pens`, the default, draws with three pen weights in one warm black, as a
//! draftsman would (Drawing principles, Shapes: draw with three pens). `plain` is one
//! uniform line with grey small text and softened bends. `lettering` is `pens`
//! with every piece of text drawn in single pen strokes (`core::lettering`)
//! instead of set in a browser font; it is the one style that changes text
//! widths, so it also gives layout its face (`Style::face`).

use super::geometry::Point;
use super::glyphs::rounded;
use super::svg::n;
use super::text::{Face, FONT, SMALL, TITLE};

/// A drawing style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    /// Three pen weights, warm black ink, sharp bends.
    #[default]
    Pens,
    /// One line weight, grey small text, softened bends.
    Plain,
    /// As `Pens`, with text drawn as single-stroke technical lettering.
    Lettering,
}

/// Every style, the default first. Adding a style is adding a variant, an
/// entry here, and its answers in each `Style` method.
pub const STYLES: &[Style] = &[Style::Pens, Style::Plain, Style::Lettering];

/// The role a stroked mark plays in a drawing. The style chooses its pen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The outline of a box that holds other boxes: a component with parts, a
    /// package with members, a composite state, an activity partition.
    Container,
    /// The outline of an ordinary box or symbol.
    Box,
    /// A small icon inside a box, such as the component icon.
    Icon,
    /// A solid connector line, or a stroke that belongs to a symbol.
    Line,
    /// A dashed connector line: a dependency, a note link, a reply.
    DashedLine,
    /// An open arrowhead on a solid line.
    Head,
    /// An open arrowhead on a dashed line.
    DashedHead,
    /// A hollow triangle on a solid line.
    HollowHead,
    /// A hollow triangle on a dashed line.
    DashedHollowHead,
    /// The outline of a note.
    Note,
    /// The folded corner of a note.
    NoteFold,
    /// A divider inside a box, such as a compartment line.
    Rule,
    /// A dashed divider inside a box, between orthogonal regions.
    DashedRule,
    /// A filled shape: an initial dot, a fork bar, a filled arrowhead.
    Solid,
    /// A sequence lifeline.
    Lifeline,
    /// The frame of a combined fragment.
    Frame,
    /// The dashed line between the operands of a combined fragment.
    FrameRule,
    /// The name tab of a fragment or interaction use.
    FrameTab,
    /// The frame of an interaction use (`ref`).
    RefFrame,
}

/// A class that only some diagram kinds use, so its rule is written only
/// into the stylesheets of drawings that need it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extra {
    Lifeline,
    Frame,
    Solid,
}

/// One stylesheet rule. `selector` is scoped under `.drumlin` when written.
pub struct Rule {
    pub selector: &'static str,
    pub declarations: String,
}

/// Text is measured with Helvetica metrics, so the stack must start with
/// metric-compatible faces.
const FONT_STACK: &str = "Helvetica, Arial, \"Liberation Sans\", sans-serif";

/// The ink of `plain`, and its grey for small text.
const PLAIN_INK: &str = "#1f2933";
const PLAIN_MUTED: &str = "#52606d";
/// The ink of `pens`: one warm black for every line and every letter.
const PENS_INK: &str = "#22201c";
/// The one pen of `lettering`'s strokes, in pixels.
const LETTERING_PEN: f64 = 1.1;
/// The pen weights of `pens`, in pixels.
const HEAVY: f64 = 1.75;
const MEDIUM: f64 = 1.25;
const FINE: f64 = 0.75;
/// The dash of `pens`: long dashes and short gaps, so a fine line still reads.
const PENS_DASH: &str = "7 3";

impl Style {
    /// The name in the `style` statement and the `--style` option.
    pub fn name(self) -> &'static str {
        match self {
            Style::Pens => "pens",
            Style::Plain => "plain",
            Style::Lettering => "lettering",
        }
    }

    /// One line on what the style looks like.
    pub fn about(self) -> &'static str {
        match self {
            Style::Pens => "three pen weights in warm black: heavy containers, medium boxes and lines, fine details",
            Style::Plain => "one line weight, grey small text, softened bends",
            Style::Lettering => "as pens, with text drawn in single-stroke technical lettering",
        }
    }

    /// The style called `name`.
    pub fn named(name: &str) -> Option<Style> {
        STYLES.iter().copied().find(|style| style.name() == name)
    }

    /// The `class` attribute of a mark with this role.
    pub fn class(self, role: Role) -> &'static str {
        match self {
            Style::Plain => match role {
                Role::Container | Role::Box | Role::HollowHead | Role::DashedHollowHead | Role::Note | Role::FrameTab | Role::RefFrame => "shape",
                Role::Icon => "icon",
                Role::Line | Role::Head | Role::DashedHead | Role::NoteFold | Role::Rule => "edge",
                Role::DashedLine | Role::DashedRule => "edge dashed",
                Role::Solid => "solid",
                Role::Lifeline => "lifeline",
                Role::Frame => "frame",
                Role::FrameRule => "frame dashed",
            },
            Style::Pens | Style::Lettering => match role {
                Role::Container => "shape heavy",
                Role::Box => "shape",
                Role::Icon => "icon",
                Role::Line => "edge",
                Role::DashedLine | Role::DashedRule => "edge dashed fine",
                Role::Head => "edge head",
                Role::DashedHead => "edge head fine",
                Role::HollowHead => "shape head",
                Role::DashedHollowHead => "shape head fine",
                Role::Note | Role::FrameTab | Role::RefFrame => "shape fine",
                Role::NoteFold | Role::Rule => "edge fine",
                Role::Solid => "solid",
                Role::Lifeline => "lifeline",
                Role::Frame => "frame",
                Role::FrameRule => "frame dashed",
            },
        }
    }

    /// Path data for an orthogonal connector through `points`: softened
    /// corners in `plain`, sharp corners in `pens`.
    pub fn line(self, points: &[Point]) -> String {
        match self {
            Style::Plain => rounded(points),
            Style::Pens | Style::Lettering => points.iter().enumerate().map(|(i, p)| format!("{}{} {}", if i == 0 { "M" } else { " L" }, n(p.0), n(p.1))).collect(),
        }
    }

    /// The `class` of the `svg` element, which also scopes the stylesheet.
    /// `pens` adds its own name, so its rules reach only its own drawings and
    /// a page may show drawings in both styles: `plain`'s rules are less
    /// specific, and `pens` sets every property they set.
    pub fn root(self) -> &'static str {
        match self {
            Style::Plain => "drumlin",
            Style::Pens => "drumlin pens",
            Style::Lettering => "drumlin lettering",
        }
    }

    /// The selector every rule is written under.
    pub fn scope(self) -> &'static str {
        match self {
            Style::Plain => ".drumlin",
            Style::Pens => ".drumlin.pens",
            Style::Lettering => ".drumlin.lettering",
        }
    }

    /// The stylesheet: the shared rules, then the `extras` a kind uses.
    pub fn rules(self, extras: &[Extra]) -> Vec<Rule> {
        match self {
            Style::Plain => plain_rules(extras),
            Style::Pens => pens_rules(extras, false),
            Style::Lettering => pens_rules(extras, true),
        }
    }

    /// What text is measured and drawn in. Only `lettering` changes it, so
    /// only `lettering` changes where things go.
    pub fn face(self) -> Face {
        match self {
            Style::Pens | Style::Plain => Face::Helvetica,
            Style::Lettering => Face::Lettering,
        }
    }
}

fn rules(list: Vec<(&'static str, String)>) -> Vec<Rule> {
    list.into_iter().map(|(selector, declarations)| Rule { selector, declarations }).collect()
}

fn text_rules(ink: &str, small: String) -> Vec<(&'static str, String)> {
    vec![
        ("text", format!("font-family: {FONT_STACK}; fill: {ink};")),
        (".title", format!("font-size: {TITLE}px; font-weight: 600;")),
        (".name", format!("font-size: {FONT}px; font-weight: 600;")),
        (".italic", "font-style: italic;".to_string()),
        (".small", small),
        (".halo", "paint-order: stroke; stroke: #fff; stroke-width: 4px; stroke-linejoin: round;".to_string()),
    ]
}

fn plain_rules(extras: &[Extra]) -> Vec<Rule> {
    let ink = PLAIN_INK;
    let shared = [
        (".shape", format!("fill: #fff; stroke: {ink}; stroke-width: 1.25;")),
        (".icon", format!("fill: #fff; stroke: {ink}; stroke-width: 1;")),
        (".edge", format!("fill: none; stroke: {ink}; stroke-width: 1.25; stroke-linecap: round; stroke-linejoin: round;")),
        (".dashed", "stroke-dasharray: 6 4;".to_string()),
    ];
    let extra = extras.iter().map(|extra| match extra {
        Extra::Lifeline => (".lifeline", format!("fill: none; stroke: {PLAIN_MUTED}; stroke-width: 1; stroke-dasharray: 5 4;")),
        Extra::Frame => (".frame", format!("fill: none; stroke: {ink}; stroke-width: 1;")),
        Extra::Solid => (".solid", format!("fill: {ink}; stroke: none;")),
    });
    let small = format!("font-size: {SMALL}px; fill: {PLAIN_MUTED};");
    rules(text_rules(ink, small).into_iter().chain(shared).chain(extra).collect())
}

/// Three pens. The weight modifiers come last, so they win over the base
/// classes they are combined with. With `lettering`, text is pen strokes in
/// groups of class `lt` instead of `<text>`.
fn pens_rules(extras: &[Extra], lettering: bool) -> Vec<Rule> {
    let ink = PENS_INK;
    let shared = [
        (".shape", format!("fill: #fff; stroke: {ink}; stroke-width: {MEDIUM}; stroke-linejoin: miter;")),
        (".icon", format!("fill: #fff; stroke: {ink}; stroke-width: {FINE}; stroke-linejoin: miter;")),
        (".edge", format!("fill: none; stroke: {ink}; stroke-width: {MEDIUM}; stroke-linecap: butt; stroke-linejoin: miter;")),
        (".head", HEAD_JOIN.to_string()),
        (".dashed", format!("stroke-dasharray: {PENS_DASH};")),
    ];
    let extra = extras.iter().map(|extra| match extra {
        Extra::Lifeline => (".lifeline", format!("fill: none; stroke: {ink}; stroke-width: {FINE}; stroke-dasharray: {PENS_DASH};")),
        Extra::Frame => (".frame", format!("fill: none; stroke: {ink}; stroke-width: {FINE}; stroke-linejoin: miter;")),
        Extra::Solid => (".solid", format!("fill: {ink}; stroke: none;")),
    });
    let weights = [(".heavy", format!("stroke-width: {HEAVY};")), (".fine", format!("stroke-width: {FINE};"))];
    let text = match lettering {
        false => text_rules(ink, format!("font-size: {SMALL}px; fill: {ink};")),
        true => vec![
            (".lt", format!("fill: none; stroke: {ink}; stroke-width: {LETTERING_PEN}; stroke-linecap: round; stroke-linejoin: round;")),
            (".lt-halo", "fill: #fff; stroke: none;".to_string()),
        ],
    };
    rules(text.into_iter().chain(shared).chain(extra).chain(weights).collect())
}

/// How an arrowhead's tip is joined.
const HEAD_JOIN: &str = "stroke-linejoin: miter; stroke-miterlimit: 4;";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_are_found_by_name_and_the_default_comes_first() {
        assert_eq!(STYLES[0], Style::default());
        assert_eq!(Style::named("pens"), Some(Style::Pens));
        assert_eq!(Style::named("plain"), Some(Style::Plain));
        assert_eq!(Style::named("lettering"), Some(Style::Lettering));
        assert_eq!(Style::named("ink"), None);
    }

    #[test]
    fn pens_draw_bends_sharp_and_plain_softens_them() {
        let points = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)];
        assert_eq!(Style::Pens.line(&points), "M0 0 L20 0 L20 20");
        assert_eq!(Style::Plain.line(&points), "M0 0 L14 0 Q20 0 20 6 L20 20");
    }

    /// The weight modifiers must follow the classes they modify, since they
    /// are equally specific.
    #[test]
    fn pen_weights_come_after_the_classes_they_modify() {
        let sheet = Style::Pens.rules(&[Extra::Lifeline, Extra::Frame, Extra::Solid]);
        let at = |selector: &str| sheet.iter().position(|rule| rule.selector == selector).unwrap();
        for base in [".shape", ".edge", ".frame", ".icon"] {
            assert!(at(base) < at(".heavy") && at(base) < at(".fine"), "{base}");
        }
        assert!(!sheet.iter().any(|rule| rule.declarations.contains(PLAIN_MUTED)), "no grey in pens");
    }

    /// Lettering draws with the pens, and strokes its text instead of setting it.
    #[test]
    fn lettering_is_pens_with_stroked_text() {
        let extras = [Extra::Lifeline, Extra::Frame, Extra::Solid];
        let (pens, lettering) = (Style::Pens.rules(&extras), Style::Lettering.rules(&extras));
        let lines = |sheet: &[Rule]| sheet.iter().filter(|rule| rule.selector.starts_with('.') && !rule.selector.starts_with(".lt") && !["text", ".title", ".name", ".italic", ".small", ".halo"].contains(&rule.selector)).map(|rule| (rule.selector, rule.declarations.clone())).collect::<Vec<_>>();
        assert_eq!(lines(&pens), lines(&lettering));
        assert!(lettering.iter().any(|rule| rule.selector == ".lt") && !lettering.iter().any(|rule| rule.selector == "text"));
        for role in [Role::Container, Role::DashedHead, Role::Note, Role::Lifeline] {
            assert_eq!(Style::Lettering.class(role), Style::Pens.class(role));
        }
        assert_eq!((Style::Pens.face(), Style::Plain.face(), Style::Lettering.face()), (Face::Helvetica, Face::Helvetica, Face::Lettering));
    }
}
