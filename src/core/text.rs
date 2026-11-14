// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Text measurement from embedded Helvetica metrics, so layout needs no font
//! files and gives identical output on every platform.
//!
//! A drawing in the `lettering` style draws its text as pen strokes, so it
//! measures with the lettering's own advance widths (`core::lettering`). The
//! face is chosen once per drawing with `with_face`, around layout and
//! rendering, and every measure below follows it; outside such a scope text is
//! measured in Helvetica.

use std::cell::Cell;

pub const FONT: f64 = 13.0;
pub const SMALL: f64 = 11.0;
pub const TITLE: f64 = 16.0;

const WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Bold,
}

/// What text is measured and drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Face {
    /// A browser font with Helvetica metrics, set as SVG `<text>`.
    #[default]
    Helvetica,
    /// drumlin's single-stroke lettering, drawn as pen strokes.
    Lettering,
}

thread_local! {
    static FACE: Cell<Face> = const { Cell::new(Face::Helvetica) };
}

/// The face of the drawing being made on this thread.
pub fn face() -> Face {
    FACE.with(Cell::get)
}

/// Runs `f` with text measured and drawn in `face`, and restores the face
/// before, even if `f` panics.
pub fn with_face<R>(face: Face, f: impl FnOnce() -> R) -> R {
    struct Restore(Face);
    impl Drop for Restore {
        fn drop(&mut self) {
            FACE.with(|cell| cell.set(self.0));
        }
    }
    let _restore = Restore(FACE.with(|cell| cell.replace(face)));
    f()
}

/// The width of `s` at `size` px in `weight`, in the current face. In
/// lettering the size picks the hand: names, small text or the title.
pub fn width(s: &str, size: f64, weight: Weight) -> f64 {
    match face() {
        Face::Helvetica => helvetica_width(s, size, weight),
        Face::Lettering => super::lettering::width(s, super::lettering::Hand::for_size(size)),
    }
}

fn helvetica_width(s: &str, size: f64, weight: Weight) -> f64 {
    let units: f64 = s
        .chars()
        .map(|c| match c as usize {
            code @ 32..=126 => WIDTHS[code - 32] as f64,
            _ => 600.0,
        })
        .sum();
    let factor = match weight {
        Weight::Regular => 1.0,
        Weight::Bold => 1.07,
    };
    units / 1000.0 * size * factor
}

/// The width of `s` in the small style (`TextStyle::Small`): labels, keywords
/// and other secondary text.
pub fn small_width(s: &str) -> f64 {
    width(s, SMALL, Weight::Regular)
}

/// The width of `s` in the name style (`TextStyle::Name`): the bold names of
/// shapes.
pub fn name_width(s: &str) -> f64 {
    width(s, FONT, Weight::Bold)
}

/// The width of `s` in the keyword style (`TextStyle::Keyword`): a name's
/// size and weight in Helvetica, lower case in lettering.
pub fn keyword_width(s: &str) -> f64 {
    match face() {
        Face::Helvetica => helvetica_width(s, FONT, Weight::Bold),
        Face::Lettering => super::lettering::width(s, super::lettering::Hand::Keyword),
    }
}

/// Greedy word wrap. A word wider than `max` keeps its own line.
pub fn wrap(s: &str, size: f64, max: f64) -> Vec<String> {
    s.split_whitespace().fold(vec![String::new()], |mut lines, word| {
        let last = lines.last_mut().unwrap();
        let candidate = if last.is_empty() { word.to_string() } else { format!("{last} {word}") };
        if last.is_empty() || width(&candidate, size, Weight::Regular) <= max {
            *last = candidate;
        } else {
            lines.push(word.to_string());
        }
        lines
    })
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// `escape` for an attribute value in double quotes.
pub fn escape_attribute(s: &str) -> String {
    escape(s).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Helvetica measure is the one drumlin always had: these widths are
    /// what every default drawing is laid out with.
    #[test]
    fn the_default_face_measures_as_before() {
        assert_eq!(face(), Face::Helvetica);
        assert_eq!(width("Order Service", FONT, Weight::Bold), 6168.0 / 1000.0 * 13.0 * 1.07);
        assert_eq!(small_width("\u{ab}x\u{bb}"), 1700.0 / 1000.0 * 11.0);
        assert_eq!(name_width("A"), helvetica_width("A", FONT, Weight::Bold));
    }

    #[test]
    fn the_face_holds_only_inside_its_scope() {
        let inside = with_face(Face::Lettering, || (face(), small_width("Ab")));
        assert_eq!(inside, (Face::Lettering, crate::core::lettering::width("Ab", crate::core::lettering::Hand::Small)));
        assert_eq!(face(), Face::Helvetica);
        assert_ne!(inside.1, small_width("Ab"));
        let caught = std::panic::catch_unwind(|| with_face(Face::Lettering, || panic!("inside")));
        assert!(caught.is_err());
        assert_eq!(face(), Face::Helvetica, "restored after a panic");
    }
}
