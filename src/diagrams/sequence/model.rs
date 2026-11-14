// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Resolved sequence diagram model. Participants are referred to by index.

use crate::core::geometry::Side;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Figure {
    Box,
    Actor,
}

#[derive(Clone, Debug)]
pub struct Participant {
    pub name: String,
    pub stereotype: Option<String>,
    pub figure: Figure,
}

/// Where a message starts or ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Life(usize),
    /// The left or right outside of the diagram, drawn with a filled circle.
    Boundary(Side),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Sync,
    Async,
    Reply,
    Create,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub from: End,
    pub to: End,
    pub sort: Sort,
    pub label: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    Alt,
    Opt,
    Loop,
    Break,
    Critical,
    Neg,
    Par,
}

impl Operator {
    pub fn keyword(self) -> &'static str {
        match self {
            Operator::Alt => "alt",
            Operator::Opt => "opt",
            Operator::Loop => "loop",
            Operator::Break => "break",
            Operator::Critical => "critical",
            Operator::Neg => "neg",
            Operator::Par => "par",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Operand {
    pub guard: Option<String>,
    pub body: Vec<Item>,
}

#[derive(Clone, Debug)]
pub struct Fragment {
    pub operator: Operator,
    /// Iteration bounds, shown with the operator as in `loop(1,5)`.
    pub bounds: Option<String>,
    pub operands: Vec<Operand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotePlace {
    Over(Vec<usize>),
    LeftOf(usize),
    RightOf(usize),
}

#[derive(Clone, Debug)]
pub enum Item {
    Message(Message),
    /// A message that opens an execution on its receiver for `body`.
    Call(Message, Vec<Item>),
    /// A reply written with `return`; the sender and receiver come from the block.
    Return(Message),
    Destroy(usize),
    Fragment(Fragment),
    Ref(Vec<usize>, String),
    Note(NotePlace, String),
}

#[derive(Debug)]
pub struct Diagram {
    /// The drawing style: the `style` statement's, or the default.
    pub style: crate::core::style::Style,
    pub title: Option<String>,
    pub participants: Vec<Participant>,
    pub items: Vec<Item>,
}
