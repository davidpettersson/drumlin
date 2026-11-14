// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Grammar shared by the box-and-line kinds: component, package and state. It
//! reads the `direction`, layout hint and `note` statements, closes blocks, and
//! resolves names and hints. Each parser keeps its own `Draft` and stores what
//! these functions return.

use super::boxes::Hint;
use crate::core::geometry::Direction;
use crate::syntax::{did_you_mean, err, name_of, Res, Tok, T};
use std::collections::BTreeMap;

/// Declared names, or aliases, to node indices.
pub(super) type Names = BTreeMap<String, usize>;

/// The words that place one node beside another: `a left-of b`.
const PLACEMENTS: [&str; 4] = ["left-of", "right-of", "above", "below"];

/// A layout hint before its names are resolved.
pub(super) struct RawHint {
    keyword: String,
    pub(super) refs: Vec<Tok>,
}

/// True when `word` is a placement hint such as `left-of`.
pub(super) fn is_placement(word: &str) -> bool {
    PLACEMENTS.contains(&word)
}

/// The hint `first <word> second`, for a placement word.
pub(super) fn placement(word: &str, first: Tok, second: Tok) -> RawHint {
    RawHint { keyword: word.to_string(), refs: vec![first, second] }
}

/// The hint of an `align a b ...` statement.
pub(super) fn align(toks: &[Tok]) -> RawHint {
    RawHint { keyword: "align".into(), refs: toks[1..].to_vec() }
}

/// The flow of a `direction right` or `direction down` statement.
pub(super) fn direction(toks: &[Tok]) -> Res<Direction> {
    let head = &toks[0];
    match toks.get(1).and_then(name_of).as_deref() {
        Some("right") => Ok(Direction::Right),
        Some("down") => Ok(Direction::Down),
        _ => err(head.line, head.col, "expected `direction right` or `direction down`"),
    }
}

/// The block stack after a `}`. The root, at the bottom, is never closed.
pub(super) fn close_block(mut stack: Vec<usize>, head: &Tok) -> Res<Vec<usize>> {
    if stack.len() == 1 {
        return err(head.line, head.col, "unmatched `}`");
    }
    stack.pop();
    Ok(stack)
}

/// The target and text of a `note <target>: <text>` statement.
pub(super) fn note(toks: &[Tok]) -> Res<(Tok, String)> {
    let head = &toks[0];
    let Some(target) = toks.get(1).cloned() else {
        return err(head.line, head.col, "expected a note target");
    };
    let Some(Tok { t: T::Label(text), .. }) = toks.get(2) else {
        return err(head.line, head.col, "expected `note <target>: <text>`");
    };
    Ok((target, text.clone()))
}

/// The error for a name that is not declared, at `tok`.
pub(super) fn unknown<X>(names: &Names, tok: &Tok, name: &str) -> Res<X> {
    let hint = did_you_mean(names.keys(), name);
    err(tok.line, tok.col, format!("unknown element `{name}`{hint}"))
}

/// The node a name token refers to.
pub(super) fn lookup(names: &Names, tok: &Tok) -> Res<usize> {
    let name = name_of(tok).unwrap_or_default();
    match names.get(&name) {
        Some(&n) => Ok(n),
        None => unknown(names, tok, &name),
    }
}

/// The hints a raw hint stands for, in a diagram that flows in `direction`.
fn resolve_hint(direction: Direction, names: &Names, raw: &RawHint) -> Res<Vec<Hint>> {
    let nodes = raw.refs.iter().map(|tok| lookup(names, tok)).collect::<Res<Vec<usize>>>()?;
    if raw.keyword == "align" {
        return Ok(nodes.windows(2).map(|pair| Hint::SameRank(pair[0], pair[1])).collect());
    }
    let (a, b) = (nodes[0], nodes[1]);
    Ok(vec![match (raw.keyword.as_str(), direction) {
        ("left-of", Direction::Right) => Hint::Main(a, b),
        ("right-of", Direction::Right) => Hint::Main(b, a),
        ("above", Direction::Right) => Hint::Cross(a, b),
        ("below", Direction::Right) => Hint::Cross(b, a),
        ("above", Direction::Down) => Hint::Main(a, b),
        ("below", Direction::Down) => Hint::Main(b, a),
        ("left-of", Direction::Down) => Hint::Cross(a, b),
        _ => Hint::Cross(b, a),
    }])
}

/// The hints of all raw hints, in order. The first unknown name is the error.
pub(super) fn resolve_hints(direction: Direction, names: &Names, raw: &[RawHint]) -> Res<Vec<Hint>> {
    let hints = raw.iter().map(|raw| resolve_hint(direction, names, raw)).collect::<Res<Vec<Vec<Hint>>>>()?;
    Ok(hints.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{tokenize, Lexicon};

    fn toks(text: &str) -> Vec<Tok> {
        tokenize(text, 1, &Lexicon { arrows: &["-->"], symbols: "{}," }).unwrap()
    }

    fn names() -> Names {
        [("A", 1), ("B", 2), ("C", 3)].map(|(name, n)| (name.to_string(), n)).into_iter().collect()
    }

    fn pairs(hints: &[Hint]) -> Vec<String> {
        hints.iter().map(|hint| format!("{hint:?}")).collect()
    }

    #[test]
    fn direction_reads_right_or_down() {
        assert_eq!(direction(&toks("direction right")), Ok(Direction::Right));
        assert_eq!(direction(&toks("direction down extra")), Ok(Direction::Down));
        let expected = Err("1:1: error: expected `direction right` or `direction down`".to_string());
        assert_eq!(direction(&toks("direction up")), expected);
        assert_eq!(direction(&toks("direction")), expected);
    }

    #[test]
    fn closing_the_root_is_unmatched() {
        let brace = &toks("}")[0];
        assert_eq!(close_block(vec![0, 4], brace), Ok(vec![0]));
        assert_eq!(close_block(vec![0], brace), Err("1:1: error: unmatched `}`".to_string()));
    }

    #[test]
    fn a_note_needs_a_target_and_text() {
        let (target, text) = note(&toks("note A: hello")).unwrap();
        assert_eq!((name_of(&target), text), (Some("A".to_string()), "hello".to_string()));
        assert_eq!(note(&toks("note")).map(|n| n.1), Err("1:1: error: expected a note target".to_string()));
        assert_eq!(note(&toks("note A")).map(|n| n.1), Err("1:1: error: expected `note <target>: <text>`".to_string()));
    }

    #[test]
    fn an_unknown_name_suggests_a_near_one() {
        assert_eq!(lookup(&names(), &toks("B")[0]), Ok(2));
        assert_eq!(lookup(&names(), &toks("Q")[0]), Err("1:1: error: unknown element `Q`; did you mean `A`?".to_string()));
    }

    #[test]
    fn placements_follow_the_flow() {
        let line = toks("A B");
        let hint = |word: &str| placement(word, line[0].clone(), line[1].clone());
        let all = ["left-of", "right-of", "above", "below"].map(hint);
        assert_eq!(pairs(&resolve_hints(Direction::Right, &names(), &all).unwrap()), ["Main(1, 2)", "Main(2, 1)", "Cross(1, 2)", "Cross(2, 1)"]);
        assert_eq!(pairs(&resolve_hints(Direction::Down, &names(), &all).unwrap()), ["Cross(1, 2)", "Cross(2, 1)", "Main(1, 2)", "Main(2, 1)"]);
        assert!(PLACEMENTS.iter().all(|word| is_placement(word)) && !is_placement("align"));
    }

    #[test]
    fn align_ranks_neighbours_together_and_reports_the_first_unknown() {
        let hints = resolve_hints(Direction::Down, &names(), &[align(&toks("align A B C"))]).unwrap();
        assert_eq!(pairs(&hints), ["SameRank(1, 2)", "SameRank(2, 3)"]);
        let raw = [align(&toks("align A X")), align(&toks("align Y A"))];
        assert_eq!(resolve_hints(Direction::Down, &names(), &raw).map(|hints| pairs(&hints)), Err("1:9: error: unknown element `X`; did you mean `A`?".to_string()));
    }
}
