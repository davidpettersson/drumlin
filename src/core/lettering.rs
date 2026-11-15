// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! drumlin's lettering: single-stroke technical lettering, as a draftsman
//! letters a drawing with a lettering guide. Every glyph is a few open pen
//! strokes of one weight with round ends, drawn with `fill: none`, not a
//! filled outline. The glyphs are derived from Routed Gothic by Darren Embry
//! and live in `lettering/glyphs.rs`, which is under the SIL Open Font
//! License 1.1 (see `LICENSES/OFL-1.1.txt`); this file is Apache-2.0.
//!
//! A drawing in the `lettering` style defines each glyph it uses once, at the
//! size of each hand, in its `<defs>`, and places it with `<use>`. Each text
//! run is a group with `role="img"` and the text as its `aria-label`, so the
//! words stay available to assistive technology. The group also holds the
//! text itself as an invisible `<text>`, stretched over the drawn letters, so
//! a viewer can select, copy and search the words as in any other drawing.

mod glyphs;

use super::svg::n;
use super::text::{escape, escape_attribute, FONT, TITLE};
use glyphs::GLYPHS;
use std::collections::BTreeSet;

/// The units of the glyph data to the em.
const UNITS: f64 = 1024.0;
/// The em of each hand, in px. Names and the title are in capitals, which
/// read larger than the mixed case of the default style, so their em is a
/// little smaller than its bold names; small text matches its x-height.
const NAME_EM: f64 = 12.5;
const SMALL_EM: f64 = 11.0;
const TITLE_EM: f64 = 16.0;
/// Letter spacing of capitals, in em.
const TRACKING: f64 = 0.06;
/// Inclined lettering leans 15 degrees from the vertical (ISO 3098).
const SLANT_DEGREES: f64 = 15.0;
/// The top of the ink of capitals and ascenders, and the bottom of the
/// descenders, in glyph units, and the white margin of a halo around them, in px.
const ASCENT: f64 = 736.0;
const DESCENT: f64 = 224.0;
const HALO: f64 = 1.5;
/// The middle of a capital's strokes above the baseline, in glyph units.
const CAP_MIDDLE: f64 = 368.0;
/// The glyph drawn for a character the lettering lacks: an open box.
const MISSING: (u16, &str) = (576, "M96 48L96 688L480 688L480 48L96 48");
/// The id prefix of glyph definitions. A page that shows several drawings
/// sees the same id for the same glyph in the same hand, defined identically.
const ID: &str = "dl-";

/// The kind of text a run is: it sets the size, the case and the spacing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hand {
    /// Names of shapes: spaced capitals.
    Name,
    /// Labels, keywords, notes and other small text: upright mixed case.
    Small,
    /// The drawing's title: larger spaced capitals.
    Title,
    /// A keyword in a name's place, such as a fragment operator: mixed
    /// case at a name's size, as UML writes keywords.
    Keyword,
    /// A keyword or stereotype above a name: small text in the finest pen.
    Tag,
}

impl Hand {
    /// The hand of text measured at `size` px: the name size, the title size,
    /// or else small text.
    pub fn for_size(size: f64) -> Hand {
        if size == FONT {
            Hand::Name
        } else if size == TITLE {
            Hand::Title
        } else {
            Hand::Small
        }
    }

    pub fn em(self) -> f64 {
        match self {
            Hand::Name | Hand::Keyword => NAME_EM,
            Hand::Small | Hand::Tag => SMALL_EM,
            Hand::Title => TITLE_EM,
        }
    }

    /// The width of the pen, in px: heavier for larger letters, as a
    /// draftsman changes pens with the lettering guide, and finest for
    /// keywords, which classify rather than state.
    pub fn pen(self) -> f64 {
        match self {
            Hand::Title => 1.5,
            Hand::Name | Hand::Keyword => 1.1,
            Hand::Small => 0.85,
            Hand::Tag => 0.75,
        }
    }

    fn capitals(self) -> bool {
        matches!(self, Hand::Name | Hand::Title)
    }

    /// The space added between letters, in px.
    pub fn tracking(self) -> f64 {
        if self.capitals() {
            TRACKING * self.em()
        } else {
            0.0
        }
    }

    fn key(self) -> char {
        match self {
            Hand::Name => 'n',
            Hand::Small => 's',
            Hand::Title => 't',
            Hand::Keyword => 'k',
            Hand::Tag => 'g',
        }
    }

    fn from_key(key: char) -> Option<Hand> {
        [Hand::Name, Hand::Small, Hand::Title, Hand::Keyword, Hand::Tag].into_iter().find(|hand| hand.key() == key)
    }
}

/// The glyph of `c`: its advance and its strokes, or `None` when the
/// lettering has no glyph for it.
pub fn glyph(c: char) -> Option<(u16, &'static str)> {
    GLYPHS.binary_search_by_key(&c, |&(g, _, _)| g).ok().map(|i| (GLYPHS[i].1, GLYPHS[i].2))
}

/// The glyph of `c`, or the open box.
fn drawn(c: char) -> (u16, &'static str) {
    glyph(c).unwrap_or(MISSING)
}

/// The letters `s` is drawn with in `hand`: capitals for names and titles.
pub fn letters(s: &str, hand: Hand) -> Vec<char> {
    if hand.capitals() {
        s.chars().flat_map(char::to_uppercase).collect()
    } else {
        s.chars().collect()
    }
}

/// The width of `s` in `hand`: the advances of its letters and the spacing
/// between them.
pub fn width(s: &str, hand: Hand) -> f64 {
    let letters = letters(s, hand);
    let units: f64 = letters.iter().map(|&c| drawn(c).0 as f64).sum();
    units / UNITS * hand.em() + hand.tracking() * letters.len().saturating_sub(1) as f64
}

/// How far the top of an inclined letter leans right of its foot, per px of height.
fn slant() -> f64 {
    SLANT_DEGREES.to_radians().tan()
}

/// A run of text: the glyph uses of `s` in `hand` from `left` on `baseline`,
/// in a group named by the text. `italic` leans the strokes. `halo` first
/// clears a white band behind the letters, so a line under a label breaks
/// for it, as a draftsman breaks a line for lettering.
pub fn run(left: f64, baseline: f64, s: &str, hand: Hand, italic: bool, halo: bool) -> String {
    if s.trim().is_empty() {
        return String::new();
    }
    let em = hand.em();
    let (mut x, mut uses) = (0.0_f64, String::new());
    for c in letters(s, hand) {
        let (advance, strokes) = drawn(c);
        if !strokes.is_empty() {
            let at = if x == 0.0 { String::new() } else { format!(" x=\"{}\"", (x * 10.0).round() / 10.0) };
            uses.push_str(&format!("<use href=\"#{ID}{}{:x}\"{at}/>", hand.key(), c as u32));
        }
        x += advance as f64 / UNITS * em + hand.tracking();
    }
    let transform = if italic {
        // Lean about the baseline, moved left so the middle of the capitals
        // stays where an upright run would have it.
        let lean = slant() * CAP_MIDDLE / UNITS * em;
        format!("translate({} {}) skewX(-{SLANT_DEGREES})", n(left - lean), n(baseline))
    } else {
        format!("translate({} {})", n(left), n(baseline))
    };
    let band = if halo {
        let (top, bottom) = (ASCENT / UNITS * em + HALO, DESCENT / UNITS * em + HALO);
        format!("<rect class=\"lt-halo\" x=\"-{HALO}\" y=\"-{}\" width=\"{}\" height=\"{}\"/>", n(top), n(width(s, hand) + 2.0 * HALO), n(top + bottom))
    } else {
        String::new()
    };
    // The words as text, invisible, stretched to the drawn run so that a
    // selection covers the letters it copies.
    let words = format!("<text font-size=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\">{}</text>", n(em), n(width(s, hand)), escape(s));
    format!("<g class=\"lt\" role=\"img\" aria-label=\"{}\" transform=\"{transform}\">{band}{uses}{words}</g>\n", escape_attribute(s))
}

/// The path data of `strokes` at `em` px, x right and y down from the
/// baseline, in tenths of a pixel and relative steps, to keep files small.
fn path_data(strokes: &str, em: f64) -> String {
    let tenths = |v: &str| (v.parse::<f64>().unwrap_or(0.0) * em / UNITS * 10.0).round() as i64;
    let mut out = String::new();
    for line in strokes.split('M').filter(|line| !line.is_empty()) {
        let points: Vec<(i64, i64)> = line
            .split('L')
            .map(|point| {
                let mut xy = point.split(' ');
                (tenths(xy.next().unwrap_or("0")), -tenths(xy.next().unwrap_or("0")))
            })
            .collect();
        let (x, y) = points[0];
        out.push('M');
        numbers(&mut out, &[x, y]);
        if points.len() == 1 {
            out.push_str("h0");
            continue;
        }
        out.push('l');
        let steps: Vec<i64> = points.windows(2).flat_map(|pair| [pair[1].0 - pair[0].0, pair[1].1 - pair[0].1]).collect();
        numbers(&mut out, &steps);
    }
    out
}

/// Writes numbers given in tenths, as short as SVG path syntax allows: no
/// leading zero, and no space before a minus sign.
fn numbers(out: &mut String, values: &[i64]) {
    for (i, &v) in values.iter().enumerate() {
        let (sign, v) = if v < 0 { ("-", -v) } else { ("", v) };
        let text = match (v / 10, v % 10) {
            (whole, 0) => format!("{whole}"),
            (0, tenth) => format!(".{tenth}"),
            (whole, tenth) => format!("{whole}.{tenth}"),
        };
        if i > 0 && sign.is_empty() {
            out.push(' ');
        }
        out.push_str(sign);
        out.push_str(&text);
    }
}

/// The `<defs>` for every glyph that `body` uses, or nothing when it uses none.
pub fn defs(body: &str) -> String {
    let marker = format!("href=\"#{ID}");
    let used: BTreeSet<(Hand, char)> = body
        .match_indices(&marker)
        .filter_map(|(at, _)| {
            let id = body[at + marker.len()..].split('"').next()?;
            let mut chars = id.chars();
            let hand = Hand::from_key(chars.next()?)?;
            char::from_u32(u32::from_str_radix(chars.as_str(), 16).ok()?).map(|c| (hand, c))
        })
        .collect();
    if used.is_empty() {
        return String::new();
    }
    let paths: String = used.iter().map(|&(hand, c)| format!("<path id=\"{ID}{}{:x}\" stroke-width=\"{}\" d=\"{}\"/>\n", hand.key(), c as u32, hand.pen(), path_data(drawn(c).1, hand.em()))).collect();
    format!("<defs>\n{paths}</defs>\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_are_sorted_and_parse() {
        assert!(GLYPHS.windows(2).all(|pair| pair[0].0 < pair[1].0), "sorted for the binary search");
        for &(c, _, strokes) in GLYPHS {
            for line in strokes.split('M').filter(|line| !line.is_empty()) {
                for point in line.split('L') {
                    let xy: Vec<&str> = point.split(' ').collect();
                    assert!(xy.len() == 2 && xy.iter().all(|v| v.parse::<i32>().is_ok()), "{c:?}: {point:?}");
                }
            }
        }
    }

    /// A width is the sum of the advances at the hand's em, and the spacing
    /// between letters.
    #[test]
    fn a_width_is_the_advances_and_the_spacing() {
        let advance = |c: char| glyph(c).unwrap().0 as f64;
        let small = (advance('x') + advance('+') + advance('1')) / UNITS * SMALL_EM;
        assert_eq!(width("x+1", Hand::Small), small);
        let name = (advance('A') + advance('B')) / UNITS * NAME_EM + TRACKING * NAME_EM;
        assert_eq!(width("ab", Hand::Name), name, "names in spaced capitals");
        assert_eq!(width("A", Hand::Title), advance('A') / UNITS * TITLE_EM, "no spacing after the last letter");
        assert_eq!(width("", Hand::Name), 0.0);
    }

    /// The one has a flag and no foot, and the zero is narrower than the O,
    /// so neither reads as a letter.
    #[test]
    fn digits_differ_from_letters() {
        let (zero, o) = (glyph('0').unwrap(), glyph('O').unwrap());
        assert!(zero.0 < o.0, "the zero is narrower than the O");
        let (one, i) = (glyph('1').unwrap(), glyph('I').unwrap());
        assert_ne!(one.1, i.1);
        assert_eq!(one.1.matches('M').count(), 1, "the flag and the stem are one stroke");
    }

    #[test]
    fn a_missing_character_draws_an_open_box() {
        assert_eq!(glyph('\u{4e2d}'), None);
        assert_eq!(width("\u{4e2d}", Hand::Small), MISSING.0 as f64 / UNITS * SMALL_EM);
        let svg = run(0.0, 0.0, "\u{4e2d}", Hand::Small, false, false);
        assert!(defs(&svg).contains(&format!("id=\"dl-s4e2d\" stroke-width=\"0.85\" d=\"{}\"", path_data(MISSING.1, SMALL_EM))));
    }

    #[test]
    fn a_run_is_named_by_its_text_and_uses_each_glyph() {
        let svg = run(10.0, 20.0, "a \"b\" & <c>", Hand::Small, false, false);
        assert!(svg.starts_with("<g class=\"lt\" role=\"img\" aria-label=\"a &quot;b&quot; &amp; &lt;c&gt;\" transform=\"translate(10 20)\"><use href=\"#dl-s61\"/>"), "{svg}");
        assert!(!svg.contains("#dl-s20"), "spaces only advance");
        let sheet = defs(&svg);
        assert_eq!(sheet.matches("<path ").count(), 7, "a \" b & < c >: {sheet}");
    }

    /// Each hand has its own pen, heavier for larger letters; a keyword is
    /// small text in the finest pen, so it takes the same room.
    #[test]
    fn each_hand_draws_with_its_own_pen() {
        let pens: Vec<f64> = [Hand::Title, Hand::Name, Hand::Small, Hand::Tag].iter().map(|hand| hand.pen()).collect();
        assert!(pens.windows(2).all(|pair| pair[0] > pair[1]), "{pens:?}");
        assert_eq!(width("entry", Hand::Tag), width("entry", Hand::Small));
        let sheet = defs(&run(0.0, 0.0, "a", Hand::Tag, false, false));
        assert!(sheet.contains("id=\"dl-g61\" stroke-width=\"0.75\""), "{sheet}");
    }

    #[test]
    fn a_run_carries_its_words_as_invisible_text_over_the_letters() {
        let svg = run(10.0, 20.0, "Order <A>", Hand::Name, false, false);
        let words = format!("<text font-size=\"12.5\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\">Order &lt;A&gt;</text></g>", n(width("Order <A>", Hand::Name)));
        assert!(svg.ends_with(&format!("{words}\n")), "as written, not in capitals, after the glyphs: {svg}");
        let leaning = run(0.0, 0.0, "ab", Hand::Small, true, true);
        assert!(leaning.contains("skewX(-15)") && leaning.ends_with(">ab</text></g>\n"), "the text leans with the letters: {leaning}");
    }

    #[test]
    fn names_are_capitals_and_halos_come_first() {
        let svg = run(0.0, 0.0, "ab", Hand::Name, true, true);
        assert!(svg.contains("#dl-n41") && svg.contains("#dl-n42") && !svg.contains("#dl-n61"));
        assert!(svg.contains("\"><rect class=\"lt-halo\" x=\"-1.5\""), "the halo comes first: {svg}");
        assert_eq!(svg.matches("aria-label").count(), 1);
        assert!(svg.contains("skewX(-15)"));
    }

    #[test]
    fn a_dot_is_a_point() {
        assert_eq!(path_data("M144 72", UNITS), "M144-72h0");
        assert_eq!(path_data("M0 0L10 20L10.04 20.5M5 5", UNITS), "M0 0l10-20 0-.5M5-5h0");
    }
}
