// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Resolved diagram model. All references are indices.

use crate::core::geometry::{Direction, Side};
use crate::diagrams::engine::Engine;

pub use crate::diagrams::boxes::Hint;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Root,
    Package,
    Component,
    Note,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub stereotype: Option<String>,
    pub kind: Kind,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub ports: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Port {
    pub name: String,
    pub owner: usize,
    pub side: Option<Side>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum End {
    Node(usize),
    Port(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Solid,
    Dashed,
    NoteLink,
    /// Ball-and-socket connector between two simple ports. `from` requires, `to` provides.
    Assembly,
    /// A dependency from a socket to a lollipop, for ends without simple ports.
    Wired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Head {
    None,
    Open,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub from: End,
    pub to: End,
    pub kind: EdgeKind,
    pub head: Head,
    pub label: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Provided,
    Required,
}

/// An interface with no partner, drawn as a lollipop or socket stub.
#[derive(Clone, Debug)]
pub struct Stub {
    pub at: End,
    pub interface: String,
    pub role: Role,
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
    pub ports: Vec<Port>,
    pub edges: Vec<Edge>,
    pub stubs: Vec<Stub>,
    pub hints: Vec<Hint>,
}

impl Diagram {
    pub fn owner(&self, end: End) -> usize {
        match end {
            End::Node(n) => n,
            End::Port(p) => self.ports[p].owner,
        }
    }

    /// True when `a` is `b` or an ancestor of `b`.
    pub fn contains(&self, a: usize, b: usize) -> bool {
        std::iter::successors(Some(b), |&n| self.nodes[n].parent).any(|n| n == a)
    }

    pub fn start_side(&self) -> Side {
        match self.direction {
            Direction::Right => Side::Left,
            Direction::Down => Side::Top,
        }
    }

    pub fn end_side(&self) -> Side {
        self.start_side().opposite()
    }

    /// The boundary side of a port: explicit, else from its connections.
    pub fn port_side(&self, port: usize) -> Side {
        let owner = self.ports[port].owner;
        let from_stub = || {
            self.stubs.iter().find(|stub| stub.at == End::Port(port)).map(|stub| match stub.role {
                Role::Provided => self.start_side(),
                Role::Required => self.end_side(),
            })
        };
        let from_edge = || {
            self.edges.iter().find_map(|edge| {
                let (is_from, peer) = match () {
                    _ if edge.from == End::Port(port) => (true, edge.to),
                    _ if edge.to == End::Port(port) => (false, edge.from),
                    _ => return None,
                };
                let inward = self.contains(owner, self.owner(peer));
                Some(if is_from != inward { self.end_side() } else { self.start_side() })
            })
        };
        self.ports[port].side.or_else(from_stub).or_else(from_edge).unwrap_or_else(|| self.start_side())
    }
}
