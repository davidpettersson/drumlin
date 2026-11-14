// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Helpers shared by the box-and-line kinds that use the layered layout and the
//! router: component, package and state. Each kind keeps its own adapter stages
//! (`aligned`, `arranged`, `grown`, `route`) and passes plain data here.

use crate::core::geometry::Rect;
use crate::core::layered::{self, Constraint, Graph, Layout, Spacing};

/// The clear gap a line keeps from a box's outline.
pub(super) const CLEAR: f64 = 8.0;

/// A layout hint from the source, as node indices.
#[derive(Clone, Copy, Debug)]
pub enum Hint {
    /// First node is placed before the second on the main axis.
    Main(usize, usize),
    /// First node is placed before the second on the cross axis.
    Cross(usize, usize),
    SameRank(usize, usize),
}

/// The layered layout's constraint for a hint.
pub(super) fn hint_constraint(hint: &Hint) -> Constraint {
    match *hint {
        Hint::Main(a, b) => Constraint::Before(a, b),
        Hint::Cross(a, b) => Constraint::AcrossBefore(a, b),
        Hint::SameRank(a, b) => Constraint::SameRank(a, b),
    }
}

/// The layout of `g` with chains in columns, and the layout without.
pub(super) fn layouts(g: &Graph) -> [Layout; 2] {
    let plain = Graph { spacing: Spacing { columns: false, ..g.spacing }, ..g.clone() };
    [layered::layout(g), layered::layout(&plain)]
}

/// A box's outline as obstacles, `CLEAR` wide. A box with a tab of size
/// `(w, h)` on its top left, such as a package, keeps the tab free as a
/// separate box.
pub(super) fn outline(r: Rect, tab: Option<(f64, f64)>) -> Vec<Rect> {
    match tab {
        Some((w, h)) => vec![Rect { x: r.x, y: r.y + h, w: r.w, h: r.h - h }.inflate(CLEAR), Rect { x: r.x, y: r.y, w, h }.inflate(CLEAR)],
        None => vec![r.inflate(CLEAR)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tab_is_a_separate_obstacle() {
        let r = Rect { x: 0.0, y: 0.0, w: 100.0, h: 80.0 };
        assert_eq!(outline(r, None), vec![r.inflate(CLEAR)]);
        let tabbed = outline(r, Some((40.0, 16.0)));
        assert_eq!(tabbed[0], Rect { x: 0.0, y: 16.0, w: 100.0, h: 64.0 }.inflate(CLEAR));
        assert_eq!(tabbed[1], Rect { x: 0.0, y: 0.0, w: 40.0, h: 16.0 }.inflate(CLEAR));
    }
}
