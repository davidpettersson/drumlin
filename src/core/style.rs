// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Drawing styles. Layout decides where every mark goes; a style decides how
//! the marks are stroked: the ink, the line weights, the dashes and the joins.
//! Rendering receives one `Style` and asks it for three things: the class of a
//! mark from the role the mark plays (`Role`), the path data of a connector
//! line, and the stylesheet that gives those classes their strokes.
//!
//! `pens`, the default, draws with three pen weights in one warm black, as a
//! draftsman would (Drawing principles, Shapes: draw with three pens), and
//! sets its text in a hierarchy of weight, size and shade. `plain` is one ink,
//! one line width and one text weight, with only the font size varying.
//! `lettering` is `pens`
//! with every piece of text drawn in single pen strokes (`core::lettering`)
//! instead of set in a browser font; it is the one style that changes text
//! widths, so it also gives layout its face (`Style::face`).

use super::geometry::Point;
use super::svg::n;
use super::text::{Face, FONT, SMALL, TITLE};

/// A drawing style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    /// Three pen weights, warm black ink, sharp bends.
    #[default]
    Pens,
    /// One ink, one line width, one text weight: only font sizes differ.
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

/// The one ink of `plain`, for every line and every letter.
const PLAIN_INK: &str = "#000";
/// The one line width of `plain`, in pixels.
const PLAIN_WIDTH: f64 = 1.25;
/// The ink of `pens`: one warm black for every line and for text that states
/// the model.
const PENS_INK: &str = "#22201c";
/// The warm grey of `pens` for text that classifies or comments: keywords,
/// notes, region and pseudostate names.
const PENS_SOFT: &str = "#5c574f";
/// The size of keywords in `pens`, in pixels: a step below the small text.
/// The tracking stays under what the smaller size saves on the narrowest
/// glyph, so the text never outgrows the width measured at `SMALL`.
const TAG: f64 = 10.0;
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
            Style::Plain => "one ink, one line width and one text weight; only font sizes differ",
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

    /// Path data for an orthogonal connector through `points`, with sharp
    /// corners in every style.
    pub fn line(self, points: &[Point]) -> String {
        points.iter().enumerate().map(|(i, p)| format!("{}{} {}", if i == 0 { "M" } else { " L" }, n(p.0), n(p.1))).collect()
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

/// The text rules every Helvetica style shares: `weight` is the weight of
/// names and the title.
fn text_rules(ink: &str, weight: u16) -> Vec<(&'static str, String)> {
    vec![
        ("text", format!("font-family: {FONT_STACK}; fill: {ink}; font-weight: 400;")),
        (".title", format!("font-size: {TITLE}px; font-weight: {weight};")),
        (".name", format!("font-size: {FONT}px; font-weight: {weight};")),
        (".italic", "font-style: italic;".to_string()),
        (".small", format!("font-size: {SMALL}px;")),
        (".halo", "paint-order: stroke; stroke: #fff; stroke-width: 4px; stroke-linejoin: round;".to_string()),
    ]
}

/// One ink, one width, one dash and one text weight. Names stay regular:
/// layout measures them bold, so they only gain room.
fn plain_rules(extras: &[Extra]) -> Vec<Rule> {
    let (ink, width) = (PLAIN_INK, PLAIN_WIDTH);
    let shared = [
        (".shape", format!("fill: #fff; stroke: {ink}; stroke-width: {width};")),
        (".icon", format!("fill: #fff; stroke: {ink}; stroke-width: {width};")),
        (".edge", format!("fill: none; stroke: {ink}; stroke-width: {width}; stroke-linecap: round; stroke-linejoin: round;")),
        (".dashed", "stroke-dasharray: 6 4;".to_string()),
    ];
    let extra = extras.iter().map(|extra| match extra {
        Extra::Lifeline => (".lifeline", format!("fill: none; stroke: {ink}; stroke-width: {width}; stroke-dasharray: 6 4;")),
        Extra::Frame => (".frame", format!("fill: none; stroke: {ink}; stroke-width: {width};")),
        Extra::Solid => (".solid", format!("fill: {ink}; stroke: none;")),
    });
    rules(text_rules(ink, 400).into_iter().chain(shared).chain(extra).collect())
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
        false => text_rules(ink, 600)
            .into_iter()
            .chain([
                (".tag", format!("font-size: {TAG}px; letter-spacing: 0.2px; fill: {PENS_SOFT};")),
                (".aside", format!("fill: {PENS_SOFT};")),
            ])
            .collect(),
        true => vec![
            (".lt", format!("fill: none; stroke: {ink}; stroke-width: {LETTERING_PEN}; stroke-linecap: round; stroke-linejoin: round;")),
            (".lt-halo", "fill: #fff; stroke: none;".to_string()),
            (".lt text", format!("font-family: {FONT_STACK}; fill: transparent; stroke: none; white-space: pre;")),
        ],
    };
    rules(text.into_iter().chain(shared).chain(extra).chain(weights).collect())
}

/// How an arrowhead's tip is joined.
const HEAD_JOIN: &str = "stroke-linejoin: miter; stroke-miterlimit: 4;";

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn styles_are_found_by_name_and_the_default_comes_first() {
        assert_eq!(STYLES[0], Style::default());
        assert_eq!(Style::named("pens"), Some(Style::Pens));
        assert_eq!(Style::named("plain"), Some(Style::Plain));
        assert_eq!(Style::named("lettering"), Some(Style::Lettering));
        assert_eq!(Style::named("ink"), None);
    }

    #[test]
    fn every_style_draws_bends_sharp() {
        let points = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)];
        for style in STYLES {
            assert_eq!(style.line(&points), "M0 0 L20 0 L20 20");
        }
    }

    /// Plain is one ink, one line width and one text weight: only sizes vary.
    #[test]
    fn plain_has_one_ink_one_width_and_one_weight() {
        let sheet = Style::Plain.rules(&[Extra::Lifeline, Extra::Frame, Extra::Solid]);
        let values = |property: &str| -> BTreeSet<String> {
            sheet.iter().flat_map(|rule| rule.declarations.split(';')).filter_map(|d| d.trim().strip_prefix(property)).map(|v| v.trim().to_string()).collect()
        };
        let colours: BTreeSet<String> = values("fill:").into_iter().chain(values("stroke:")).filter(|v| v.starts_with('#') && v != "#fff").collect();
        assert_eq!(colours, BTreeSet::from([PLAIN_INK.to_string()]));
        let widths: BTreeSet<String> = values("stroke-width:").into_iter().filter(|v| v != "4px").collect();
        assert_eq!(widths, BTreeSet::from([PLAIN_WIDTH.to_string()]), "one width; the 4px is the white halo");
        assert_eq!(values("font-weight:"), BTreeSet::from(["400".to_string()]));
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
        assert!(!sheet.iter().any(|rule| rule.selector != ".tag" && rule.selector != ".aside" && rule.declarations.contains(PENS_SOFT)), "the soft ink is for keywords and asides only");
    }

    /// Lettering draws with the pens, and strokes its text instead of setting it.
    #[test]
    fn lettering_is_pens_with_stroked_text() {
        let extras = [Extra::Lifeline, Extra::Frame, Extra::Solid];
        let (pens, lettering) = (Style::Pens.rules(&extras), Style::Lettering.rules(&extras));
        let lines = |sheet: &[Rule]| sheet.iter().filter(|rule| rule.selector.starts_with('.') && !rule.selector.starts_with(".lt") && !["text", ".title", ".name", ".italic", ".small", ".halo", ".tag", ".aside"].contains(&rule.selector)).map(|rule| (rule.selector, rule.declarations.clone())).collect::<Vec<_>>();
        assert_eq!(lines(&pens), lines(&lettering));
        assert!(lettering.iter().any(|rule| rule.selector == ".lt") && !lettering.iter().any(|rule| rule.selector == "text"));
        for role in [Role::Container, Role::DashedHead, Role::Note, Role::Lifeline] {
            assert_eq!(Style::Lettering.class(role), Style::Pens.class(role));
        }
        assert_eq!((Style::Pens.face(), Style::Plain.face(), Style::Lettering.face()), (Face::Helvetica, Face::Helvetica, Face::Lettering));
    }
}
