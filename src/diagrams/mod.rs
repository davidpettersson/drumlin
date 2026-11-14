// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! One module per diagram kind. Each owns its grammar, model and notation.

pub mod activity;
pub mod component;
pub mod package;
pub mod sequence;
pub mod state;
mod box_grammar;
mod boxes;
mod connectors;
pub mod engine;
pub mod style;
#[cfg(test)]
mod quality;

/// Reports each connector, by index, that the router found no clear way for.
/// It is still drawn, directly, so this is the one `warning:` drumlin prints.
/// `what` names the connector as the kind calls it.
fn warn_unrouted(what: &str, failed: &[usize]) {
    for connector in failed {
        eprintln!("warning: no clear route for {what} {connector}; drawing it directly");
    }
}
