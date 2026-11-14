// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Resolved state machine model. All references are indices.

use crate::core::geometry::Direction;
use crate::diagrams::engine::Engine;
use crate::core::text::{width, wrap, Weight, SMALL};

pub use crate::diagrams::boxes::Hint;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Root,
    State,
    Region,
    Initial,
    Final,
    Choice,
    Junction,
    Fork,
    Join,
    Shallow,
    Deep,
    Terminate,
    EntryPoint,
    ExitPoint,
    Note,
}

impl Kind {
    /// Drawn as a fixed small symbol, where all lines on a side meet at one point.
    pub fn is_point(self) -> bool {
        matches!(self, Kind::Initial | Kind::Final | Kind::Choice | Kind::Junction | Kind::Shallow | Kind::Deep | Kind::Terminate | Kind::EntryPoint | Kind::ExitPoint)
    }

    pub fn is_bar(self) -> bool {
        matches!(self, Kind::Fork | Kind::Join)
    }

    pub fn is_connection_point(self) -> bool {
        matches!(self, Kind::EntryPoint | Kind::ExitPoint)
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub stereotype: Option<String>,
    pub kind: Kind,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// Internal behaviours and internal transitions, in source order.
    pub activities: Vec<String>,
    /// Where the node was declared or first used.
    pub at: (usize, usize),
}

/// The parts of a transition label, per 14.2.4.9:
/// `[trigger {, trigger}] ['[' guard ']'] ['/' effect]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Label {
    pub trigger: Option<String>,
    pub guard: Option<String>,
    pub effect: Option<String>,
}

impl Label {
    /// The label as printed.
    pub fn text(&self) -> String {
        let guard = self.guard.as_ref().map(|g| format!("[{g}]"));
        let effect = self.effect.as_ref().map(|e| if e.is_empty() { "/".to_string() } else { format!("/ {e}") });
        [self.trigger.clone(), guard, effect].into_iter().flatten().collect::<Vec<_>>().join(" ")
    }

    /// The lines of a label: a long one breaks before its effect, then wraps.
    pub fn lines(&self, wrap_at: f64) -> Vec<String> {
        let whole = self.text();
        if width(&whole, SMALL, Weight::Regular) <= wrap_at {
            return vec![whole];
        }
        let head = [self.trigger.clone(), self.guard.as_ref().map(|g| format!("[{g}]"))].into_iter().flatten().collect::<Vec<_>>().join(" ");
        let head_lines = if head.is_empty() { Vec::new() } else { wrap(&head, SMALL, wrap_at) };
        // The slash stays with the first word of the effect.
        let effect_lines = self.effect.iter().flat_map(|e| wrap(e, SMALL, wrap_at - width("/ ", SMALL, Weight::Regular)).into_iter().enumerate().map(|(i, line)| if i == 0 { format!("/ {line}") } else { line }));
        head_lines.into_iter().chain(effect_lines).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Transition,
    NoteLink,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub line: Line,
    pub label: Option<Label>,
}

#[derive(Debug)]
pub struct Diagram {
    /// The drawing style: the `style` statement's, or the default.
    pub style: crate::core::style::Style,
    pub title: Option<String>,
    pub direction: Direction,
    /// The layout engine: the `layout` statement's, or the default.
    pub engine: Engine,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub hints: Vec<Hint>,
}

impl Diagram {
    /// True when `a` is `b` or an ancestor of `b`.
    pub fn contains(&self, a: usize, b: usize) -> bool {
        std::iter::successors(Some(b), |&n| self.nodes[n].parent).any(|n| n == a)
    }

    /// The orthogonal regions of a state, or nothing.
    pub fn regions(&self, n: usize) -> Vec<usize> {
        self.nodes[n].children.iter().copied().filter(|&c| self.nodes[c].kind == Kind::Region).collect()
    }

    /// True when the edge is a local transition, between a composite state and
    /// something inside it.
    pub fn is_local(&self, edge: &Edge) -> bool {
        edge.line == Line::Transition && edge.from != edge.to && (self.contains(edge.from, edge.to) || self.contains(edge.to, edge.from))
    }
}
