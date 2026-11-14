// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! State machine grammar. Each source line is first cut into statements: internal
//! behaviour lines are taken as raw text, and everything else is tokenized. The
//! statements fold into a `Draft`, then transitions are resolved against the names
//! declared anywhere in the file and checked against the rules of UML 2.5.1 clause 14.

use super::model::*;
use crate::core::geometry::Direction;
use crate::core::style::Style;
use crate::diagrams::engine::{self, Engine};
use crate::diagrams::style;
use crate::diagrams::box_grammar::{self, lookup, Names, RawHint};
use crate::syntax::{self, describe, distance, err, name_of, Lexicon, Res, Tok, T};
use std::collections::{BTreeMap, BTreeSet};

const LEXICON: Lexicon = Lexicon { arrows: &["-->", "<--", "->"], symbols: "{},;[]*" };

const KEYWORDS: [&str; 17] = ["diagram", "direction", "state", "region", "choice", "junction", "fork", "join", "terminate", "entrypoint", "exitpoint", "note", "align", "left-of", "right-of", "above", "below"];

/// A transition end before names are resolved.
#[derive(Clone, Debug)]
enum End {
    /// `[*]`: the initial pseudostate as a source, the final state as a target.
    Star(usize, usize),
    /// `[H]` or `[H*]`, of the enclosing state or of a named one.
    History { owner: Option<Tok>, deep: bool, line: usize, col: usize },
    Name(Tok),
}

struct RawEdge {
    from: End,
    to: End,
    label: Option<(Label, usize, usize)>,
    /// The state, region or root the statement was written in.
    scope: usize,
}

struct Draft {
    title: Option<String>,
    direction: Direction,
    engine: Engine,
    style: Style,
    nodes: Vec<Node>,
    names: Names,
    stack: Vec<usize>,
    edges: Vec<RawEdge>,
    hints: Vec<RawHint>,
    notes: Vec<(Tok, String)>,
}

fn empty_draft() -> Draft {
    Draft {
        title: None,
        direction: Direction::Down,
        engine: engine::DEFAULT,
        style: Style::default(),
        nodes: vec![new_node(String::new(), Kind::Root, None, (1, 1))],
        names: Names::new(),
        stack: vec![0],
        edges: vec![],
        hints: vec![],
        notes: vec![],
    }
}

fn new_node(name: String, kind: Kind, parent: Option<usize>, at: (usize, usize)) -> Node {
    Node { name, stereotype: None, kind, parent, children: vec![], activities: vec![], at }
}

fn current(draft: &Draft) -> usize {
    *draft.stack.last().unwrap()
}

/// One statement of the source.
enum Statement {
    Tokens(Vec<Tok>),
    /// An internal behaviour or internal transition line, as written.
    Internal { text: String, line: usize, col: usize },
}

/// Drops a trailing comment. As in sequence diagrams, a `#` or `//` is a comment
/// only with whitespace on both sides, so `issue #4` survives in a label.
fn strip_comment(text: &str) -> &str {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut quoted = false;
    for (i, &(at, c)) in chars.iter().enumerate() {
        if c == '"' {
            quoted = !quoted;
        }
        let next = chars.get(i + 1).map(|p| p.1);
        let marker = match (c, next) {
            ('#', _) => 1,
            ('/', Some('/')) => 2,
            _ => 0,
        };
        let bounded = i == 0 || chars[i - 1].1.is_whitespace();
        let open_after = chars.get(i + marker).is_none_or(|p| p.1.is_whitespace());
        if !quoted && marker > 0 && bounded && open_after {
            return &text[..at];
        }
    }
    text
}

/// True for a line holding an internal behaviour (`entry`, `do`, `exit`, with or
/// without an expression) or an internal transition: a `/` with no arrow before it.
fn is_internal(text: &str) -> bool {
    let first = text.split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
    let rest = text[first.len()..].trim_start();
    if ["entry", "do", "exit"].contains(&first) {
        return rest.is_empty() || rest.starts_with('/');
    }
    match text.find('/') {
        Some(slash) => !text[..slash].contains("-->") && !text[..slash].contains("<--") && !text[..slash].contains('"') && !text[..slash].contains(':') && !KEYWORDS.contains(&first),
        None => false,
    }
}

fn line_statements(text: &str, line: usize) -> Res<Vec<Statement>> {
    let stripped = strip_comment(text);
    let trimmed = stripped.trim();
    if is_internal(trimmed) {
        let col = stripped.len() - stripped.trim_start().len() + 1;
        return Ok(vec![Statement::Internal { text: trimmed.to_string(), line, col }]);
    }
    Ok(syntax::split_statements(syntax::tokenize(stripped, line, &LEXICON)?).into_iter().map(Statement::Tokens).collect())
}

fn apply(draft: Draft, statement: Statement) -> Res<Draft> {
    match statement {
        Statement::Internal { text, line, col } => internal(draft, &text, line, col),
        Statement::Tokens(toks) => apply_tokens(draft, &toks),
    }
}

fn apply_tokens(mut draft: Draft, toks: &[Tok]) -> Res<Draft> {
    let head = &toks[0];
    let declares = toks.get(1).is_some_and(|t| !matches!(t.t, T::Arrow(_)));
    match syntax::keyword(toks) {
        "diagram" => Ok(Draft { title: syntax::header_title(toks, "state")?, ..draft }),
        "direction" => Ok(Draft { direction: box_grammar::direction(toks)?, ..draft }),
        "layout" if engine::is_statement(toks) => Ok(Draft { engine: engine::statement(toks)?, ..draft }),
        "style" if style::is_statement(toks) => Ok(Draft { style: style::statement(toks)?, ..draft }),
        "state" if declares => state(draft, toks),
        "region" => region(draft, toks),
        "choice" if declares => point(draft, toks, Kind::Choice),
        "junction" if declares => point(draft, toks, Kind::Junction),
        "fork" if declares => point(draft, toks, Kind::Fork),
        "join" if declares => point(draft, toks, Kind::Join),
        "terminate" if declares => point(draft, toks, Kind::Terminate),
        "entrypoint" if declares => connection_point(draft, toks, Kind::EntryPoint),
        "exitpoint" if declares => connection_point(draft, toks, Kind::ExitPoint),
        "note" if declares => {
            draft.notes.push(box_grammar::note(toks)?);
            Ok(draft)
        }
        "align" => {
            draft.hints.push(box_grammar::align(toks));
            Ok(draft)
        }
        _ if head.t == T::Sym('}') => Ok(Draft { stack: box_grammar::close_block(draft.stack, head)?, ..draft }),
        _ => relation(draft, toks),
    }
}

fn internal(mut draft: Draft, text: &str, line: usize, col: usize) -> Res<Draft> {
    let scope = current(&draft);
    if draft.nodes[scope].kind != Kind::State {
        return err(line, col, "internal behaviours and internal transitions belong directly inside a `state` block");
    }
    let first = text.split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
    let printed = if ["entry", "do", "exit"].contains(&first) {
        let expression = text[first.len()..].trim_start().trim_start_matches('/').trim();
        if expression.is_empty() { first.to_string() } else { format!("{first} / {expression}") }
    } else {
        parse_label(text, line, col)?.text()
    };
    draft.nodes[scope].activities.push(printed);
    Ok(draft)
}

fn add_node(mut draft: Draft, node: Node) -> (Draft, usize) {
    let index = draft.nodes.len();
    let parent = node.parent.unwrap();
    draft.nodes.push(node);
    draft.nodes[parent].children.push(index);
    (draft, index)
}

/// True when a vertex cannot be placed directly in the state because it has regions.
fn has_regions(draft: &Draft, scope: usize) -> bool {
    draft.nodes[scope].children.iter().any(|&c| draft.nodes[c].kind == Kind::Region)
}

fn vertex_scope(draft: &Draft, at: &Tok) -> Res<usize> {
    let scope = current(draft);
    if has_regions(draft, scope) {
        return err(at.line, at.col, format!("`{}` has regions, so this belongs inside a `region` block", draft.nodes[scope].name));
    }
    Ok(scope)
}

fn register(draft: Draft, id: String, name_tok: &Tok, node: Node) -> Res<(Draft, usize)> {
    if draft.names.contains_key(&id) {
        return err(name_tok.line, name_tok.col, format!("`{id}` is already declared"));
    }
    let (mut next, index) = add_node(draft, node);
    next.names.insert(id, index);
    Ok((next, index))
}

fn state(draft: Draft, toks: &[Tok]) -> Res<Draft> {
    let (name_tok, name) = syntax::declared_name(toks)?;
    let options = syntax::decl_options(&toks[2..], syntax::DeclOptions::default())?;
    let id = options.alias.unwrap_or_else(|| name.clone());
    let scope = vertex_scope(&draft, name_tok)?;
    let node = Node { stereotype: options.stereotype, ..new_node(name, Kind::State, Some(scope), (name_tok.line, name_tok.col)) };
    let (mut draft, index) = register(draft, id, name_tok, node)?;
    if options.opens {
        draft.stack.push(index);
    }
    Ok(draft)
}

fn region(draft: Draft, toks: &[Tok]) -> Res<Draft> {
    let head = &toks[0];
    let scope = current(&draft);
    if draft.nodes[scope].kind != Kind::State {
        return err(head.line, head.col, "a `region` belongs directly inside a `state` block");
    }
    let vertices = draft.nodes[scope].children.iter().any(|&c| !matches!(draft.nodes[c].kind, Kind::Region | Kind::EntryPoint | Kind::ExitPoint));
    if vertices {
        return err(head.line, head.col, format!("`{}` already has direct contents, so it cannot also have regions; move them into a `region`", draft.nodes[scope].name));
    }
    let (name, rest) = match toks.get(1) {
        Some(tok) if name_of(tok).is_some() => (name_of(tok).unwrap(), &toks[2..]),
        _ => (String::new(), &toks[1..]),
    };
    if !matches!(rest, [Tok { t: T::Sym('{'), .. }]) {
        return err(head.line, head.col, "expected `region \"name\" {` or `region {`");
    }
    let (mut draft, index) = add_node(draft, new_node(name, Kind::Region, Some(scope), (head.line, head.col)));
    draft.stack.push(index);
    Ok(draft)
}

fn point(draft: Draft, toks: &[Tok], kind: Kind) -> Res<Draft> {
    let (name_tok, name) = syntax::declared_name(toks)?;
    if let Some(extra) = toks.get(2) {
        return err(extra.line, extra.col, format!("unexpected {}", describe(extra)));
    }
    let scope = vertex_scope(&draft, name_tok)?;
    let node = new_node(name.clone(), kind, Some(scope), (name_tok.line, name_tok.col));
    Ok(register(draft, name, name_tok, node)?.0)
}

/// An entry or exit point sits on the border of a composite state.
fn connection_point(draft: Draft, toks: &[Tok], kind: Kind) -> Res<Draft> {
    let head = &toks[0];
    let (name_tok, name) = syntax::declared_name(toks)?;
    if let Some(extra) = toks.get(2) {
        return err(extra.line, extra.col, format!("unexpected {}", describe(extra)));
    }
    let scope = current(&draft);
    if draft.nodes[scope].kind != Kind::State {
        return err(head.line, head.col, "an entry or exit point belongs directly inside a `state` block");
    }
    let node = new_node(name.clone(), kind, Some(scope), (name_tok.line, name_tok.col));
    Ok(register(draft, name, name_tok, node)?.0)
}

/// Reads `[*]`, `[H]` and `[H*]`, returning the end and the tokens it took.
fn bracket_end(toks: &[Tok], owner: Option<Tok>) -> Option<(End, usize)> {
    let [open, rest @ ..] = toks else { return None };
    if open.t != T::Sym('[') {
        return None;
    }
    let is_h = |tok: &Tok| matches!(&tok.t, T::Ident(s) if s == "H");
    match rest {
        [a, close, ..] if a.t == T::Sym('*') && close.t == T::Sym(']') && owner.is_none() => Some((End::Star(open.line, open.col), 3)),
        [h, close, ..] if is_h(h) && close.t == T::Sym(']') => Some((End::History { owner, deep: false, line: open.line, col: open.col }, 3)),
        [h, star, close, ..] if is_h(h) && star.t == T::Sym('*') && close.t == T::Sym(']') => Some((End::History { owner, deep: true, line: open.line, col: open.col }, 4)),
        _ => None,
    }
}

/// Reads one transition end from the front of `toks`.
fn transition_end(toks: &[Tok]) -> Option<(End, usize)> {
    match toks.first() {
        Some(tok) if tok.t == T::Sym('[') => bracket_end(toks, None),
        Some(tok) if name_of(tok).is_some() => match bracket_end(&toks[1..], Some(tok.clone())) {
            Some((end, taken)) => Some((end, taken + 1)),
            None => Some((End::Name(tok.clone()), 1)),
        },
        _ => None,
    }
}

fn likely_keyword(word: &str) -> String {
    KEYWORDS.iter().filter(|k| distance(k, word) <= 2).min_by_key(|k| distance(k, word)).map_or(String::new(), |k| format!("; did you mean `{k}`?"))
}

fn relation(mut draft: Draft, toks: &[Tok]) -> Res<Draft> {
    let head = &toks[0];
    let Some((first, taken)) = transition_end(toks) else {
        return err(head.line, head.col, format!("unexpected {}", describe(head)));
    };
    if let (End::History { owner: None, deep, line, col }, None) = (&first, toks.get(taken)) {
        let scope = current(&draft);
        return Ok(scoped_vertex(draft, scope, if *deep { Kind::Deep } else { Kind::Shallow }, (*line, *col)).0);
    }
    let Some(middle) = toks.get(taken) else {
        return err(head.line, head.col, format!("incomplete statement starting with {}", describe(head)));
    };
    match &middle.t {
        T::Arrow(arrow) if arrow == "->" => err(middle.line, middle.col, "a transition is written with `-->`"),
        T::Arrow(arrow) => {
            let Some((second, used)) = transition_end(&toks[taken + 1..]) else {
                return err(middle.line, middle.col, format!("expected a state, `[*]`, `[H]` or `[H*]` after `{arrow}`"));
            };
            let label = match toks.get(taken + 1 + used) {
                Some(Tok { t: T::Label(text), line, col }) if !text.is_empty() => Some((parse_label(text, *line, *col)?, *line, *col)),
                Some(Tok { t: T::Label(_), .. }) | None => None,
                Some(tok) => return err(tok.line, tok.col, format!("unexpected {}", describe(tok))),
            };
            let (from, to) = if arrow == "-->" { (first, second) } else { (second, first) };
            let scope = current(&draft);
            draft.edges.push(RawEdge { from, to, label, scope });
            Ok(draft)
        }
        T::Ident(k) if box_grammar::is_placement(k) => {
            let Some(right) = toks.get(taken + 1).cloned() else {
                return err(middle.line, middle.col, format!("expected a name after `{k}`"));
            };
            draft.hints.push(box_grammar::placement(k, head.clone(), right));
            Ok(draft)
        }
        _ => {
            let word = name_of(head).unwrap_or_default();
            err(middle.line, middle.col, format!("expected an arrow after `{word}`, found {}{}", describe(middle), likely_keyword(&word)))
        }
    }
}

/// Parses `[trigger {, trigger}] ['[' guard ']'] ['/' effect]`.
fn parse_label(text: &str, line: usize, col: usize) -> Res<Label> {
    let text = text.trim();
    let open = text.find('[');
    let slash = text.find('/');
    let guard_first = matches!((open, slash), (Some(o), Some(s)) if o < s) || (open.is_some() && slash.is_none());
    let (trigger, guard, after) = if guard_first {
        let o = open.unwrap();
        let Some(close) = closing_bracket(text, o) else {
            return err(line, col, "a guard needs a closing `]`");
        };
        (text[..o].trim(), Some(text[o + 1..close].trim().to_string()), text[close + 1..].trim())
    } else {
        match slash {
            Some(s) => (text[..s].trim(), None, text[s..].trim()),
            None => (text, None, ""),
        }
    };
    let effect = match after.strip_prefix('/') {
        Some(rest) => Some(rest.trim().to_string()),
        None if after.is_empty() => None,
        None => return err(line, col, format!("expected `/` before `{after}`; a label has the form `trigger [guard] / effect`")),
    };
    let trigger = Some(trigger.to_string()).filter(|t| !t.is_empty());
    Ok(Label { trigger, guard, effect })
}

fn closing_bracket(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in text.char_indices().skip_while(|&(i, _)| i < open) {
        match c {
            '[' => depth += 1,
            ']' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return Some(i);
        }
    }
    None
}

/// The pseudostate of `kind` in `scope`, created on first use.
fn scoped_vertex(draft: Draft, scope: usize, kind: Kind, at: (usize, usize)) -> (Draft, usize) {
    match draft.nodes[scope].children.iter().copied().find(|&c| draft.nodes[c].kind == kind) {
        Some(found) => (draft, found),
        None => add_node(draft, new_node(String::new(), kind, Some(scope), at)),
    }
}

fn resolve_end(draft: Draft, end: &End, scope: usize, source: bool) -> Res<(Draft, usize)> {
    match end {
        End::Star(line, col) => {
            if has_regions(&draft, scope) {
                return err(*line, *col, format!("`{}` has regions, so `[*]` belongs inside a `region` block", draft.nodes[scope].name));
            }
            Ok(scoped_vertex(draft, scope, if source { Kind::Initial } else { Kind::Final }, (*line, *col)))
        }
        End::History { owner, deep, line, col } => {
            let target = match owner {
                Some(tok) => lookup(&draft.names, tok)?,
                None => scope,
            };
            let invalid = draft.nodes[target].kind == Kind::Root || has_regions(&draft, target);
            if invalid {
                return err(*line, *col, "history belongs to a region of a composite state; write it inside the state or `region` block");
            }
            Ok(scoped_vertex(draft, target, if *deep { Kind::Deep } else { Kind::Shallow }, (*line, *col)))
        }
        End::Name(tok) => {
            let n = lookup(&draft.names, tok)?;
            Ok((draft, n))
        }
    }
}

/// The enclosing region of a vertex below `ancestor`, if it lies in one.
fn region_under(nodes: &[Node], ancestor: usize, vertex: usize) -> Option<usize> {
    std::iter::successors(Some(vertex), |&n| nodes[n].parent).find(|&n| nodes[n].parent == Some(ancestor) && nodes[n].kind == Kind::Region)
}

fn ancestors(nodes: &[Node], n: usize) -> Vec<usize> {
    std::iter::successors(Some(n), |&x| nodes[x].parent).collect()
}

/// UML 14.2.3.9.6 forbids a transition between orthogonal regions of one state.
fn check_regions(nodes: &[Node], from: usize, to: usize, at: (usize, usize)) -> Res<()> {
    let above = ancestors(nodes, from);
    let shared = ancestors(nodes, to).into_iter().find(|n| above.contains(n)).unwrap_or(0);
    match (region_under(nodes, shared, from), region_under(nodes, shared, to)) {
        (Some(a), Some(b)) if a != b => err(at.0, at.1, format!("a transition cannot join two orthogonal regions of `{}`", nodes[shared].name)),
        _ => Ok(()),
    }
}

fn depth_of(draft: &Draft, scope: usize) -> usize {
    ancestors(&draft.nodes, scope).len()
}

/// Creates every state that transitions name but no statement declares, in the
/// deepest scope that mentions it, so a nested use wins over an outer one.
/// Every name written where a state is referred to, in source order.
fn name_references<'a>(raw_edges: &'a [RawEdge], hints: &'a [RawHint], notes: &'a [(Tok, String)]) -> Vec<&'a Tok> {
    let ends = raw_edges.iter().flat_map(|raw| [&raw.from, &raw.to]).filter_map(|end| match end {
        End::Name(tok) => Some(tok),
        End::History { owner, .. } => owner.as_ref(),
        End::Star(..) => None,
    });
    ends.chain(hints.iter().flat_map(|hint| &hint.refs)).chain(notes.iter().map(|note| &note.0)).collect()
}

/// A name mentioned once, never declared and within edit distance 2 of another
/// vertex's name is treated as a typo, because transitions declare implicitly.
/// The distance must be below the name's length: replacing every letter of a
/// one-letter name such as `A` for `B` is not a misspelling.
fn check_typos(draft: &Draft, raw_edges: &[RawEdge]) -> Res<()> {
    let references = name_references(raw_edges, &draft.hints, &draft.notes);
    let spelled: Vec<String> = references.iter().map(|tok| name_of(tok).unwrap_or_default()).collect();
    let count = |name: &str| spelled.iter().filter(|other| other.as_str() == name).count();
    let known: BTreeSet<&str> = draft.names.keys().map(String::as_str).chain(spelled.iter().map(String::as_str)).collect();
    let is_keyword = |name: &str| ["[*]", ""].contains(&name);
    let suspects = references.iter().zip(&spelled).filter(|(_, name)| !draft.names.contains_key(name.as_str()) && count(name) == 1 && !is_keyword(name));
    let nearest = |name: &str| {
        // Declared names first, then the most used, so a repeated spelling wins.
        known.iter().copied().filter(|&other| other != name && !is_keyword(other)).map(|other| (distance(name, other), !draft.names.contains_key(other), usize::MAX - count(other), other)).min()
    };
    let found = suspects.filter_map(|(tok, name)| nearest(name).filter(|best| best.0 <= 2 && best.0 < name.chars().count()).map(|best| (tok, name, best.3))).next();
    match found {
        Some((tok, name, like)) => err(tok.line, tok.col, format!("unknown state `{name}`; did you mean `{like}`? declare it with `state {name}` if intended")),
        None => Ok(()),
    }
}

fn declare_implicit(draft: Draft, raw_edges: &[RawEdge]) -> Res<Draft> {
    check_typos(&draft, raw_edges)?;
    let mut deepest: BTreeMap<String, (usize, usize, Tok)> = BTreeMap::new();
    for raw in raw_edges {
        for end in [&raw.from, &raw.to] {
            if let End::Name(tok) = end {
                let name = name_of(tok).unwrap_or_default();
                if !draft.names.contains_key(&name) {
                    let depth = depth_of(&draft, raw.scope);
                    let keep = deepest.get(&name).is_some_and(|(d, _, _)| *d >= depth);
                    if !keep {
                        deepest.insert(name, (depth, raw.scope, tok.clone()));
                    }
                }
            }
        }
    }
    deepest.into_iter().try_fold(draft, |draft, (name, (_, scope, tok))| {
        if has_regions(&draft, scope) {
            return err(tok.line, tok.col, format!("`{}` has regions, so a new state belongs inside a `region` block", draft.nodes[scope].name));
        }
        let node = new_node(name.clone(), Kind::State, Some(scope), (tok.line, tok.col));
        let (mut next, index) = add_node(draft, node);
        next.names.insert(name, index);
        Ok(next)
    })
}

fn resolve_edge(draft: Draft, raw: RawEdge) -> Res<(Draft, Edge)> {
    let (draft, from) = resolve_end(draft, &raw.from, raw.scope, true)?;
    let (draft, to) = resolve_end(draft, &raw.to, raw.scope, false)?;
    let at = raw.label.as_ref().map_or_else(|| end_position(&raw.from), |(_, line, col)| (*line, *col));
    check_regions(&draft.nodes, from, to, at)?;
    if from == to && draft.nodes[from].kind != Kind::State {
        return err(at.0, at.1, "only a state can have a self-transition");
    }
    Ok((draft, Edge { from, to, line: Line::Transition, label: raw.label.map(|(label, _, _)| label) }))
}

fn end_position(end: &End) -> (usize, usize) {
    match end {
        End::Star(line, col) | End::History { line, col, .. } => (*line, *col),
        End::Name(tok) => (tok.line, tok.col),
    }
}

/// Multiplicity constraints of 14.5.6.7 and the label rules of 14.2.4.6 and 14.5.11.
fn check_vertices(d: &Diagram) -> Res<()> {
    let count = |n: usize, outgoing: bool| d.edges.iter().filter(|e| e.line == Line::Transition && (if outgoing { e.from } else { e.to }) == n).count();
    d.nodes.iter().enumerate().try_for_each(|(n, node)| {
        let (incoming, outgoing) = (count(n, false), count(n, true));
        let fail = |message: String| err::<()>(node.at.0, node.at.1, message);
        match node.kind {
            Kind::Fork if incoming != 1 || outgoing < 2 => fail(format!("fork `{}` needs exactly one incoming and at least two outgoing transitions", node.name)),
            Kind::Join if incoming < 2 || outgoing != 1 => fail(format!("join `{}` needs at least two incoming transitions and exactly one outgoing", node.name)),
            Kind::Choice if incoming < 1 || outgoing < 1 => fail(format!("choice `{}` needs at least one incoming and one outgoing transition", node.name)),
            Kind::Junction if incoming < 1 || outgoing < 1 => fail(format!("junction `{}` needs at least one incoming and one outgoing transition", node.name)),
            Kind::Initial if outgoing > 1 => fail("a region has one initial pseudostate, with one outgoing transition".to_string()),
            Kind::Shallow | Kind::Deep if outgoing > 1 => fail("a history pseudostate can have at most one outgoing transition".to_string()),
            Kind::Terminate if outgoing > 0 => fail(format!("terminate `{}` cannot have outgoing transitions", node.name)),
            _ => Ok(()),
        }
    })?;
    d.edges.iter().filter(|edge| edge.line == Line::Transition).try_for_each(|edge| check_label(d, edge))
}

fn check_label(d: &Diagram, edge: &Edge) -> Res<()> {
    let (source, target) = (&d.nodes[edge.from], &d.nodes[edge.to]);
    let at = source.at.max(target.at);
    let label = edge.label.clone().unwrap_or_default();
    let has_label = edge.label.is_some();
    match () {
        _ if source.kind == Kind::Initial && source.parent != Some(0) && has_label => err(at.0, at.1, "a nested initial pseudostate's transition must be unlabeled"),
        _ if source.kind == Kind::Fork && target.kind != Kind::State => err(at.0, at.1, "a fork segment must target a state"),
        _ if target.kind == Kind::Join && source.kind != Kind::State => err(at.0, at.1, "a join segment must originate from a state"),
        _ if (source.kind == Kind::Fork || target.kind == Kind::Join) && (label.trigger.is_some() || label.guard.is_some()) => err(at.0, at.1, "a fork or join segment cannot have a trigger or guard"),
        _ if source.kind != Kind::Initial && source.kind.is_point() && source.kind != Kind::Final && !source.kind.is_connection_point() && label.trigger.is_some() => err(at.0, at.1, "a transition leaving a pseudostate cannot have a trigger; use `[guard] / effect`"),
        _ => Ok(()),
    }
}

/// Adds a note as a sibling of its target, linked and ranked with it.
fn attach_note(mut d: Diagram, names: &Names, target: &Tok, text: String) -> Res<Diagram> {
    let target = lookup(names, target)?;
    let mut parent = d.nodes[target].parent.unwrap();
    if d.nodes[target].kind.is_connection_point() {
        parent = d.nodes[parent].parent.unwrap();
    }
    let note = d.nodes.len();
    d.nodes.push(new_node(text, Kind::Note, Some(parent), d.nodes[target].at));
    d.nodes[parent].children.push(note);
    d.edges.push(Edge { from: note, to: target, line: Line::NoteLink, label: None });
    d.hints.push(Hint::SameRank(note, target));
    Ok(d)
}

pub fn parse(source: &str) -> Res<Diagram> {
    let statements = source.lines().enumerate().flat_map(|(index, text)| match line_statements(text, index + 1) {
        Ok(found) => found.into_iter().map(Ok).collect::<Vec<_>>(),
        Err(message) => vec![Err(message)],
    });
    let mut draft = statements.into_iter().try_fold(empty_draft(), |draft, statement: Res<Statement>| apply(draft, statement?))?;
    if draft.stack.len() > 1 {
        let open = draft.nodes[current(&draft)].name.clone();
        return err(source.lines().count().max(1), 1, format!("missing `}}` for `{open}`"));
    }
    let raw_edges = std::mem::take(&mut draft.edges);
    let draft = declare_implicit(draft, &raw_edges)?;
    let (draft, edges) = raw_edges.into_iter().try_fold((draft, Vec::new()), |(draft, mut edges), raw| {
        let (draft, edge) = resolve_edge(draft, raw)?;
        edges.push(edge);
        Ok::<_, String>((draft, edges))
    })?;
    let Draft { title, direction, engine, style, nodes, names, hints: raw_hints, notes, .. } = draft;
    let outline = Diagram { style, title, direction, engine, nodes, edges, hints: Vec::new() };
    check_vertices(&outline)?;
    let annotated = notes.into_iter().try_fold(outline, |d, (target, text)| attach_note(d, &names, &target, text))?;
    let hints = box_grammar::resolve_hints(annotated.direction, &names, &raw_hints)?;
    let all_hints = annotated.hints.iter().copied().chain(hints).collect();
    Ok(Diagram { hints: all_hints, ..annotated })
}
