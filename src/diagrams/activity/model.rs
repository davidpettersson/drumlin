// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! The block tree of an activity. Flows are lists of statements, and a compound
//! statement holds lists of its own, so every program is well formed by construction.

/// One statement in a flow. `lane` is an index into `Diagram::lanes` and is 0 when
/// the diagram has no lanes. `note` is attached to this statement's node.
#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: Kind,
    pub lane: usize,
    pub note: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Arm {
    /// The text of the guard, without brackets. `None` only on fork branches.
    pub guard: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub enum Kind {
    Start,
    Stop,
    End,
    Action(String),
    Send(String),
    Accept(String),
    Object(String),
    /// A decision. `subject` is shown as a «decisionInput» note (Figure 15.33).
    Decision { subject: Option<String>, arms: Vec<Arm> },
    Fork(Vec<Vec<Stmt>>),
    While { guard: String, body: Vec<Stmt> },
    Repeat { body: Vec<Stmt>, guard: String },
}

#[derive(Debug)]
pub struct Diagram {
    /// The drawing style: the `style` statement's, or the default.
    pub style: crate::core::style::Style,
    pub title: Option<String>,
    pub lanes: Vec<String>,
    pub flow: Vec<Stmt>,
}

/// True when control can pass the end of this statement.
pub fn stmt_falls(stmt: &Stmt) -> bool {
    match &stmt.kind {
        Kind::Stop | Kind::End => false,
        Kind::Decision { arms, .. } => arms.iter().any(|arm| falls(&arm.body)),
        Kind::Fork(branches) => branches.iter().any(|branch| falls(branch)),
        _ => true,
    }
}

/// True when control can pass the end of this flow.
pub fn falls(flow: &[Stmt]) -> bool {
    flow.last().is_none_or(stmt_falls)
}
