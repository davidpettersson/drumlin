// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Package diagram grammar, in explicit stages: fold the statements into a
//! `Draft`, then resolve edges, notes and hints against the declared names.

use super::model::*;
use crate::core::geometry::Direction;
use crate::core::style::Style;
use crate::diagrams::engine::{self, Engine};
use crate::diagrams::style;
use crate::diagrams::box_grammar::{self, lookup, Names, RawHint};
use crate::syntax::{self, describe, err, name_of, Lexicon, Res, Tok, T};

const LEXICON: Lexicon = Lexicon {
    arrows: &["--|>", "<|--", "..|>", "<|..", "+--", "--+", "-->", "<--", "..>", "<..", "--", ".."],
    symbols: "{},;",
};

struct RawEdge {
    from: Tok,
    to: Tok,
    arrow: String,
    label: Option<String>,
}

/// Everything the statements declare, before any name is resolved.
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
    let root = Node { name: String::new(), stereotype: None, kind: Kind::Root, parent: None, children: vec![] };
    Draft {
        title: None,
        direction: Direction::Down,
        engine: engine::DEFAULT,
        style: Style::default(),
        nodes: vec![root],
        names: Names::new(),
        stack: vec![0],
        edges: vec![],
        hints: vec![],
        notes: vec![],
    }
}

fn current(draft: &Draft) -> usize {
    *draft.stack.last().unwrap()
}

fn apply(mut draft: Draft, toks: &[Tok]) -> Res<Draft> {
    let head = &toks[0];
    match syntax::keyword(toks) {
        "diagram" => Ok(Draft { title: syntax::header_title(toks, "package")?, ..draft }),
        "direction" => Ok(Draft { direction: box_grammar::direction(toks)?, ..draft }),
        "layout" if engine::is_statement(toks) => Ok(Draft { engine: engine::statement(toks)?, ..draft }),
        "style" if style::is_statement(toks) => Ok(Draft { style: style::statement(toks)?, ..draft }),
        "package" => declare(draft, toks, Kind::Package),
        "class" => declare(draft, toks, Kind::Class),
        "interface" => declare(draft, toks, Kind::Interface),
        "enum" => declare(draft, toks, Kind::Enum),
        "note" => {
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

fn declare(mut draft: Draft, toks: &[Tok], kind: Kind) -> Res<Draft> {
    let head = &toks[0];
    let (name_tok, name) = syntax::declared_name(toks)?;
    let options = syntax::decl_options(&toks[2..], syntax::DeclOptions::default())?;
    let id = options.alias.unwrap_or_else(|| name.clone());
    if draft.names.contains_key(&id) {
        return err(name_tok.line, name_tok.col, format!("`{id}` is already declared"));
    }
    if options.opens && kind != Kind::Package {
        return err(head.line, head.col, "only a package can have a `{ }` body");
    }
    let parent = current(&draft);
    let index = draft.nodes.len();
    draft.nodes.push(Node { name, stereotype: options.stereotype, kind, parent: Some(parent), children: vec![] });
    draft.nodes[parent].children.push(index);
    draft.names.insert(id, index);
    if options.opens {
        draft.stack.push(index);
    }
    Ok(draft)
}

fn relation(mut draft: Draft, toks: &[Tok]) -> Res<Draft> {
    let head = &toks[0];
    if name_of(head).is_none() {
        return err(head.line, head.col, format!("unexpected {}", describe(head)));
    }
    let (Some(middle), Some(right)) = (toks.get(1), toks.get(2)) else {
        return err(head.line, head.col, format!("incomplete statement starting with {}", describe(head)));
    };
    match &middle.t {
        T::Arrow(arrow) => {
            let label = match toks.get(3) {
                Some(Tok { t: T::Label(s), .. }) => Some(s.clone()),
                Some(tok @ Tok { t: T::Stereo(s), .. }) => {
                    return err(tok.line, tok.col, format!("put the stereotype in a label: `{} {arrow} {} : <<{s}>>`", name_of(head).unwrap_or_default(), name_of(right).unwrap_or_default()));
                }
                Some(tok) => return err(tok.line, tok.col, format!("unexpected {}", describe(tok))),
                None => None,
            };
            draft.edges.push(RawEdge { from: head.clone(), to: right.clone(), arrow: arrow.clone(), label });
            Ok(draft)
        }
        T::Ident(k) if box_grammar::is_placement(k) => {
            draft.hints.push(box_grammar::placement(k, head.clone(), right.clone()));
            Ok(draft)
        }
        _ => err(middle.line, middle.col, format!("expected an arrow or layout hint, found {}", describe(middle))),
    }
}

/// (forward, line, head) for an arrow. `forward` is false when it points left.
fn arrow_style(arrow: &str) -> (bool, Line, Head) {
    match arrow {
        "--|>" => (true, Line::Solid, Head::Triangle),
        "<|--" => (false, Line::Solid, Head::Triangle),
        "..|>" => (true, Line::Dashed, Head::Triangle),
        "<|.." => (false, Line::Dashed, Head::Triangle),
        "+--" => (true, Line::Containment, Head::None),
        "--+" => (false, Line::Containment, Head::None),
        "-->" => (true, Line::Solid, Head::Open),
        "<--" => (false, Line::Solid, Head::Open),
        "..>" => (true, Line::Dashed, Head::Open),
        "<.." => (false, Line::Dashed, Head::Open),
        ".." => (true, Line::Dashed, Head::None),
        _ => (true, Line::Solid, Head::None),
    }
}

fn resolve_edge(d: &Diagram, names: &Names, raw: RawEdge) -> Res<Edge> {
    let first = lookup(names, &raw.from)?;
    let second = lookup(names, &raw.to)?;
    let (forward, line, head) = arrow_style(&raw.arrow);
    let (from, to) = if forward { (first, second) } else { (second, first) };
    let at = (raw.from.line, raw.from.col);
    if from == to {
        return err(at.0, at.1, "a relation needs two different elements");
    }
    // UML allows any relation between an element and its enclosing package, but
    // containment drawn with a line would repeat what the nesting already shows.
    if line == Line::Containment && (d.contains(from, to) || d.contains(to, from)) {
        let (outer, inner) = if d.contains(from, to) { (from, to) } else { (to, from) };
        let (outer, inner) = (&d.nodes[outer].name, &d.nodes[inner].name);
        return err(at.0, at.1, format!("`{inner}` is drawn inside `{outer}`, so containment needs no line; declare `{inner}` outside the `{outer}` block"));
    }
    Ok(Edge { from, to, line, head, label: raw.label.as_deref().map(syntax::guillemets) })
}

/// Adds a note as a sibling of its target, linked and ranked with it.
fn attach_note(mut d: Diagram, names: &Names, target: &Tok, text: String) -> Res<Diagram> {
    let target = lookup(names, target)?;
    let parent = d.nodes[target].parent.unwrap();
    let note = d.nodes.len();
    d.nodes.push(Node { name: text, stereotype: None, kind: Kind::Note, parent: Some(parent), children: vec![] });
    d.nodes[parent].children.push(note);
    d.edges.push(Edge { from: note, to: target, line: Line::NoteLink, head: Head::None, label: None });
    d.hints.push(Hint::SameRank(note, target));
    Ok(d)
}

pub fn parse(source: &str) -> Res<Diagram> {
    let draft = syntax::statements(source, &LEXICON).try_fold(empty_draft(), |draft, statement| apply(draft, &statement?))?;
    if draft.stack.len() > 1 {
        let open = &draft.nodes[current(&draft)].name;
        return err(source.lines().count(), 1, format!("missing `}}` for `{open}`"));
    }
    let Draft { title, direction, engine, style, nodes, names, edges: raw_edges, hints: raw_hints, notes, .. } = draft;
    let outline = Diagram { style, title, direction, engine, nodes, edges: Vec::new(), hints: Vec::new() };
    let edges = raw_edges.into_iter().map(|raw| resolve_edge(&outline, &names, raw)).collect::<Res<Vec<Edge>>>()?;
    let connected = Diagram { edges, ..outline };
    let annotated = notes.into_iter().try_fold(connected, |d, (target, text)| attach_note(d, &names, &target, text))?;
    let hints = box_grammar::resolve_hints(annotated.direction, &names, &raw_hints)?;
    let all_hints = annotated.hints.iter().copied().chain(hints).collect();
    Ok(Diagram { hints: all_hints, ..annotated })
}
