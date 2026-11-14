// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Resolved package diagram model. All references are indices.

use crate::core::geometry::{Direction, Side};
use crate::diagrams::engine::Engine;

pub use crate::diagrams::boxes::Hint;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Root,
    Package,
    Class,
    Interface,
    Enum,
    Note,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub stereotype: Option<String>,
    pub kind: Kind,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Solid,
    Dashed,
    NoteLink,
    /// Owner to member, with a circle-plus anchor on the owner.
    Containment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Head {
    None,
    Open,
    /// Hollow triangle, for generalization and realization.
    Triangle,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub line: Line,
    pub head: Head,
    pub label: Option<String>,
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
    /// True when one end of `edge` encloses the other: a relation drawn from a
    /// nested element to the inside of its enclosing package, or the reverse.
    pub fn nests(&self, edge: &Edge) -> bool {
        self.contains(edge.from, edge.to) || self.contains(edge.to, edge.from)
    }

    /// True when `a` is `b` or an ancestor of `b`.
    pub fn contains(&self, a: usize, b: usize) -> bool {
        std::iter::successors(Some(b), |&n| self.nodes[n].parent).any(|n| n == a)
    }

    pub fn is_package(&self, n: usize) -> bool {
        self.nodes[n].kind == Kind::Package
    }

    /// The side edges leave along the flow.
    pub fn end_side(&self) -> Side {
        match self.direction {
            Direction::Right => Side::Right,
            Direction::Down => Side::Bottom,
        }
    }
}

/// True for a classifier declared `<<abstract>>`, whose name 9.2.4 sets in italics.
pub fn is_abstract(node: &Node) -> bool {
    node.kind != Kind::Package && node.stereotype.as_deref() == Some("abstract")
}

/// The keyword line a classifier shows above its name, if any. A declared
/// stereotype joins the kind's keyword, as UML's comma-separated keyword list.
pub fn keywords(node: &Node) -> Option<String> {
    let kind = match node.kind {
        Kind::Interface => Some("interface"),
        Kind::Enum => Some("enumeration"),
        _ => None,
    };
    let declared = node.stereotype.as_deref().filter(|s| *s != "abstract" || node.kind == Kind::Package);
    match (kind, declared) {
        (Some(k), Some(s)) => Some(format!("\u{ab}{k}, {s}\u{bb}")),
        (Some(k), None) => Some(format!("\u{ab}{k}\u{bb}")),
        (None, Some(s)) => Some(format!("\u{ab}{s}\u{bb}")),
        (None, None) => None,
    }
}
