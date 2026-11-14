// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Layout engines for the kinds that use the layered layout: component, package
//! and state. An engine is a named set of method choices (`layered::Strategy`)
//! handed to `core::layered`. Each kind's layout adapter reads the engine from
//! its diagram (`Diagram::engine`) and builds its graph with that strategy, so
//! alignment, growth, routing and rendering are the same for every engine.
//!
//! A diagram picks an engine with a `layout <name>` statement. A caller may
//! choose one instead (`drumlin render --layout <name>`), which wins over the
//! statement. Sequence and activity diagrams have their own layouts and accept
//! neither.

use super::box_grammar::is_placement;
use crate::core::layered::{Ranking, Strategy};
use crate::syntax::{did_you_mean, err, name_of, reject_rest, Res, Tok, T};

/// A layout engine: its name in the `layout` statement and the `--layout`
/// option, one line on what it does, and the method choices it stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Engine {
    pub name: &'static str,
    pub about: &'static str,
    pub strategy: Strategy,
}

/// Every engine, the default first. Adding an engine is adding an entry here,
/// with the `core::layered` method it chooses.
pub const ENGINES: &[Engine] = &[
    Engine {
        name: "longest-path",
        about: "longest-path ranks, median ordering and placement, columns and notes last",
        strategy: Strategy { ranking: Ranking::LongestPath },
    },
    Engine {
        name: "network-simplex",
        about: "ranks that minimise total edge length (network simplex), then as longest-path",
        strategy: Strategy { ranking: Ranking::NetworkSimplex },
    },
];

/// The engine a diagram uses when nothing names one.
pub const DEFAULT: Engine = ENGINES[0];

/// The engine called `name`.
pub fn named(name: &str) -> Option<Engine> {
    ENGINES.iter().copied().find(|engine| engine.name == name)
}

/// The engine names, quoted and comma-separated.
fn name_list() -> String {
    ENGINES.iter().map(|engine| format!("`{}`", engine.name)).collect::<Vec<String>>().join(", ")
}

/// The message for a name that is no engine: a near name, or else every name.
pub fn unknown(name: &str) -> String {
    let names: Vec<String> = ENGINES.iter().map(|engine| engine.name.to_string()).collect();
    match did_you_mean(names.iter(), name) {
        hint if hint.is_empty() => format!("unknown layout engine `{name}`; the engines are {}", name_list()),
        hint => format!("unknown layout engine `{name}`{hint}"),
    }
}

/// True when a statement is `layout <name>`, rather than a relation, hint or
/// message whose first element happens to be called `layout`.
pub(super) fn is_statement(toks: &[Tok]) -> bool {
    is_setting(toks, "layout")
}

/// True when a statement is `<setting> <name>`: the word, then nothing, a
/// string or a name that is not a placement keyword.
pub(super) fn is_setting(toks: &[Tok], setting: &str) -> bool {
    let named = matches!(toks.first(), Some(Tok { t: T::Ident(word), .. }) if word == setting);
    named
        && match toks.get(1).map(|tok| &tok.t) {
            None | Some(T::Str(_)) => true,
            Some(T::Ident(word)) => !is_placement(word),
            Some(_) => false,
        }
}

/// The engine a `layout <name>` statement names.
pub(super) fn statement(toks: &[Tok]) -> Res<Engine> {
    let head = &toks[0];
    let Some(name_tok) = toks.get(1) else {
        return err(head.line, head.col, format!("expected `layout <engine>`; the engines are {}", name_list()));
    };
    let name = name_of(name_tok).unwrap_or_default();
    let engine = named(&name).ok_or_else(|| unknown(&name));
    let engine = engine.or_else(|message| err(name_tok.line, name_tok.col, message))?;
    reject_rest(&toks[2..])?;
    Ok(engine)
}

/// The error for a layout engine chosen for a diagram of `kind`, which has its
/// own layout, at the `layout` statement or the `diagram` header.
pub fn not_layered<X>(kind: &str, line: usize, col: usize) -> Res<X> {
    err(line, col, format!("{kind} diagrams have their own layout; layout engines apply to component, package and state diagrams"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{tokenize, Lexicon};

    fn toks(text: &str) -> Vec<Tok> {
        tokenize(text, 1, &Lexicon { arrows: &["-->"], symbols: "{}," }).unwrap()
    }

    #[test]
    fn engines_are_found_by_name_and_the_default_comes_first() {
        assert_eq!(named("longest-path"), Some(DEFAULT));
        assert_eq!(DEFAULT.strategy, Strategy::default());
        assert_eq!(named("simplex"), None);
        assert_eq!(named("network-simplex").map(|engine| engine.strategy.ranking), Some(Ranking::NetworkSimplex));
        let names: Vec<&str> = ENGINES.iter().map(|engine| engine.name).collect();
        let mut sorted = names.clone();
        sorted.dedup();
        assert_eq!(names, sorted, "engine names are unique");
    }

    #[test]
    fn an_unknown_name_suggests_a_near_one_or_lists_them_all() {
        assert_eq!(unknown("longest-pth"), "unknown layout engine `longest-pth`; did you mean `longest-path`?");
        assert_eq!(unknown("dot"), "unknown layout engine `dot`; the engines are `longest-path`, `network-simplex`");
    }

    #[test]
    fn the_statement_names_an_engine() {
        assert_eq!(statement(&toks("layout longest-path")), Ok(DEFAULT));
        assert_eq!(statement(&toks("layout \"longest-path\"")), Ok(DEFAULT));
        assert_eq!(statement(&toks("layout dot")), Err("1:8: error: unknown layout engine `dot`; the engines are `longest-path`, `network-simplex`".to_string()));
        assert_eq!(statement(&toks("layout")), Err("1:1: error: expected `layout <engine>`; the engines are `longest-path`, `network-simplex`".to_string()));
        assert_eq!(statement(&toks("layout longest-path now")), Err("1:21: error: unexpected `now`".to_string()));
    }

    #[test]
    fn an_element_called_layout_is_not_the_statement() {
        assert!(is_statement(&toks("layout longest-path")));
        assert!(is_statement(&toks("layout")));
        assert!(!is_statement(&toks("layout --> parser")));
        assert!(!is_statement(&toks("layout left-of parser")));
        assert!(!is_statement(&toks("direction down")));
    }
}
