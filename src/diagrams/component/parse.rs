// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Component diagram grammar, in explicit stages: fold the statements into a
//! `Draft`, resolve edges, attach notes, resolve hints, then pair interfaces.

use super::model::*;
use crate::core::geometry::{Direction, Side};
use crate::core::style::Style;
use crate::diagrams::engine::{self, Engine};
use crate::diagrams::style;
use crate::diagrams::box_grammar::{self, lookup, Names, RawHint};
use crate::syntax::{self, describe, err, name_of, Lexicon, Res, Tok, T};

const LEXICON: Lexicon = Lexicon { arrows: &["-->", "<--", "..>", "<..", "--", ".."], symbols: "{}," };

/// An interface named by a `provides`, `requires` or `port` statement.
struct Decl {
    node: usize,
    interface: String,
    port: Option<usize>,
    role: Role,
}

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
    ports: Vec<Port>,
    names: Names,
    stack: Vec<usize>,
    interfaces: Vec<Decl>,
    edges: Vec<RawEdge>,
    hints: Vec<RawHint>,
    notes: Vec<(Tok, String)>,
}

fn empty_draft() -> Draft {
    let root = Node { name: String::new(), stereotype: None, kind: Kind::Root, parent: None, children: vec![], ports: vec![] };
    Draft {
        title: None,
        direction: Direction::Right,
        engine: engine::DEFAULT,
        style: Style::default(),
        nodes: vec![root],
        ports: vec![],
        names: Names::new(),
        stack: vec![0],
        interfaces: vec![],
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
        "diagram" => Ok(Draft { title: syntax::header_title(toks, "component")?, ..draft }),
        "direction" => Ok(Draft { direction: box_grammar::direction(toks)?, ..draft }),
        "layout" if engine::is_statement(toks) => Ok(Draft { engine: engine::statement(toks)?, ..draft }),
        "style" if style::is_statement(toks) => Ok(Draft { style: style::statement(toks)?, ..draft }),
        "package" => declare(draft, toks, Kind::Package),
        "component" => declare(draft, toks, Kind::Component),
        "provides" => interface_list(draft, toks, Role::Provided),
        "requires" => interface_list(draft, toks, Role::Required),
        "port" => port(draft, toks),
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
    let (name_tok, name) = syntax::declared_name(toks)?;
    let options = syntax::decl_options(&toks[2..], syntax::DeclOptions::default())?;
    let id = options.alias.unwrap_or_else(|| name.clone());
    if draft.names.contains_key(&id) {
        return err(name_tok.line, name_tok.col, format!("`{id}` is already declared"));
    }
    let parent = current(&draft);
    let index = draft.nodes.len();
    draft.nodes.push(Node { name, stereotype: options.stereotype, kind, parent: Some(parent), children: vec![], ports: vec![] });
    draft.nodes[parent].children.push(index);
    draft.names.insert(id, index);
    if options.opens {
        draft.stack.push(index);
    }
    Ok(draft)
}

fn interface_list(mut draft: Draft, toks: &[Tok], role: Role) -> Res<Draft> {
    let head = &toks[0];
    let node = current(&draft);
    if draft.nodes[node].kind != Kind::Component {
        return err(head.line, head.col, "`provides` and `requires` belong inside a component");
    }
    let names = toks[1..].iter().filter(|tok| tok.t != T::Sym(',')).map(|tok| match name_of(tok) {
        Some(interface) => Ok(Decl { node, interface, port: None, role }),
        None => err(tok.line, tok.col, format!("expected an interface name, found {}", describe(tok))),
    });
    draft.interfaces.extend(names.collect::<Res<Vec<Decl>>>()?);
    Ok(draft)
}

/// What may follow a port's name: a side and any interfaces.
fn port_options(rest: &[Tok], owner: usize, port: usize, side: Option<Side>, mut decls: Vec<Decl>) -> Res<(Option<Side>, Vec<Decl>)> {
    let Some((first, tail)) = rest.split_first() else {
        return Ok((side, decls));
    };
    let word = name_of(first).unwrap_or_default();
    match word.as_str() {
        "left" => port_options(tail, owner, port, Some(Side::Left), decls),
        "right" => port_options(tail, owner, port, Some(Side::Right), decls),
        "top" => port_options(tail, owner, port, Some(Side::Top), decls),
        "bottom" => port_options(tail, owner, port, Some(Side::Bottom), decls),
        "provides" | "requires" => {
            let Some(interface) = tail.first().and_then(name_of) else {
                return err(first.line, first.col, "expected an interface name");
            };
            let role = if word == "provides" { Role::Provided } else { Role::Required };
            decls.push(Decl { node: owner, interface, port: Some(port), role });
            port_options(&tail[1..], owner, port, side, decls)
        }
        _ => err(first.line, first.col, format!("unexpected {}", describe(first))),
    }
}

fn port(mut draft: Draft, toks: &[Tok]) -> Res<Draft> {
    let head = &toks[0];
    let owner = current(&draft);
    if draft.nodes[owner].kind != Kind::Component {
        return err(head.line, head.col, "`port` belongs inside a component");
    }
    let Some(name) = toks.get(1).and_then(name_of) else {
        return err(head.line, head.col, "expected a port name");
    };
    let port = draft.ports.len();
    let (side, decls) = port_options(&toks[2..], owner, port, None, Vec::new())?;
    draft.interfaces.extend(decls);
    draft.ports.push(Port { name, owner, side });
    draft.nodes[owner].ports.push(port);
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

fn lookup_end(d: &Diagram, names: &Names, tok: &Tok) -> Res<End> {
    let name = name_of(tok).unwrap_or_default();
    if let Some(&n) = names.get(&name) {
        return Ok(End::Node(n));
    }
    if let Some((owner, port)) = name.rsplit_once('.') {
        if let Some(&n) = names.get(owner) {
            return match d.nodes[n].ports.iter().find(|&&p| d.ports[p].name == port) {
                Some(&p) => Ok(End::Port(p)),
                None => err(tok.line, tok.col, format!("`{owner}` has no port `{port}`")),
            };
        }
    }
    let matches: Vec<usize> = (0..d.ports.len()).filter(|&p| d.ports[p].name == name).collect();
    match matches.as_slice() {
        [p] => Ok(End::Port(*p)),
        [] => box_grammar::unknown(names, tok, &name),
        _ => err(tok.line, tok.col, format!("port `{name}` is ambiguous; write `<component>.{name}`")),
    }
}

fn resolve_edge(d: &Diagram, names: &Names, raw: RawEdge) -> Res<Edge> {
    let first = lookup_end(d, names, &raw.from)?;
    let second = lookup_end(d, names, &raw.to)?;
    let (from, to) = if raw.arrow.starts_with('<') { (second, first) } else { (first, second) };
    if d.owner(from) == d.owner(to) {
        return err(raw.from.line, raw.from.col, "a connector needs two different components");
    }
    let kind = if raw.arrow.contains('.') { EdgeKind::Dashed } else { EdgeKind::Solid };
    let head = if raw.arrow.contains('>') || raw.arrow.contains('<') { Head::Open } else { Head::None };
    Ok(Edge { from, to, kind, head, label: raw.label.as_deref().map(syntax::guillemets) })
}

/// Adds a note as a sibling of its target, linked and ranked with it.
fn attach_note(mut d: Diagram, names: &Names, target: &Tok, text: String) -> Res<Diagram> {
    let target = lookup(names, target)?;
    let parent = d.nodes[target].parent.unwrap();
    let note = d.nodes.len();
    d.nodes.push(Node { name: text, stereotype: None, kind: Kind::Note, parent: Some(parent), children: vec![], ports: vec![] });
    d.nodes[parent].children.push(note);
    d.edges.push(Edge { from: End::Node(note), to: End::Node(target), kind: EdgeKind::NoteLink, head: Head::None, label: None });
    d.hints.push(Hint::SameRank(note, target));
    Ok(d)
}

/// Indices into `set` of the declarations not nested inside another's owner.
fn outermost(d: &Diagram, set: &[&Decl]) -> Vec<usize> {
    let owners: Vec<usize> = set.iter().map(|decl| decl.node).collect();
    let is_outer = |o: usize| !owners.iter().any(|&other| other != o && d.contains(other, o));
    owners.iter().enumerate().filter(|&(_, &o)| is_outer(o)).map(|(i, _)| i).collect()
}

fn decl_end(decl: &Decl) -> End {
    decl.port.map_or(End::Node(decl.node), End::Port)
}

/// The connector a requirer and a provider of one interface get. Ball-and-socket
/// may only join two simple ports (11.2.4), and a complex port takes a plain
/// connector. Anywhere else the socket and lollipop stay separate, joined by a
/// dependency arrow (10.4.4, Figure 11.46).
fn wiring(decls: &[Decl], from: &Decl, to: &Decl, name: &str) -> Edge {
    let is_complex = |decl: &Decl| decl.port.is_some_and(|p| decls.iter().filter(|x| x.port == Some(p)).count() > 1);
    let kind = match () {
        _ if is_complex(from) || is_complex(to) => EdgeKind::Solid,
        _ if from.port.is_some() && to.port.is_some() => EdgeKind::Assembly,
        _ => EdgeKind::Wired,
    };
    Edge { from: decl_end(from), to: decl_end(to), kind, head: Head::None, label: Some(name.to_string()) }
}

/// The assembly connectors, or stubs, for one interface name.
fn match_interface(d: &Diagram, decls: &[Decl], name: &str) -> (Vec<Stub>, Vec<Edge>) {
    let with_role = |role: Role| -> Vec<&Decl> { decls.iter().filter(|x| x.role == role && x.interface == name).collect() };
    let (providers, requirers) = (with_role(Role::Provided), with_role(Role::Required));
    if providers.is_empty() || requirers.is_empty() {
        let stubs = providers.iter().chain(&requirers).map(|decl| Stub { at: decl_end(decl), interface: name.to_string(), role: decl.role }).collect();
        return (stubs, Vec::new());
    }
    let outer_providers = outermost(d, &providers);
    let edges = outermost(d, &requirers)
        .into_iter()
        .flat_map(|r| outer_providers.iter().map(move |&p| (r, p)))
        .map(|(r, p)| (requirers[r], providers[p]))
        .filter(|(from, to)| from.node != to.node && !d.contains(from.node, to.node) && !d.contains(to.node, from.node))
        .map(|(from, to)| wiring(decls, from, to, name))
        .collect();
    (Vec::new(), edges)
}

/// Pairs required and provided interfaces into assembly connectors. Any
/// interface without a partner becomes a stub.
fn match_interfaces(d: &Diagram, decls: &[Decl]) -> (Vec<Stub>, Vec<Edge>) {
    let mut names: Vec<&String> = decls.iter().map(|x| &x.interface).collect();
    names.sort();
    names.dedup();
    names.into_iter().map(|name| match_interface(d, decls, name)).fold((Vec::new(), Vec::new()), |(mut stubs, mut edges), (s, e)| {
        stubs.extend(s);
        edges.extend(e);
        (stubs, edges)
    })
}

pub fn parse(source: &str) -> Res<Diagram> {
    let draft = syntax::statements(source, &LEXICON).try_fold(empty_draft(), |draft, statement| apply(draft, &statement?))?;
    if draft.stack.len() > 1 {
        let open = &draft.nodes[current(&draft)].name;
        return err(source.lines().count(), 1, format!("missing `}}` for `{open}`"));
    }
    let Draft { title, direction, engine, style, nodes, ports, names, interfaces, edges: raw_edges, hints: raw_hints, notes, .. } = draft;
    let outline = Diagram { style, title, direction, engine, nodes, ports, edges: Vec::new(), stubs: Vec::new(), hints: Vec::new() };

    let edges = raw_edges.into_iter().map(|raw| resolve_edge(&outline, &names, raw)).collect::<Res<Vec<Edge>>>()?;
    let connected = Diagram { edges, ..outline };
    let annotated = notes.into_iter().try_fold(connected, |d, (target, text)| attach_note(d, &names, &target, text))?;
    let hints = box_grammar::resolve_hints(annotated.direction, &names, &raw_hints)?;
    let (stubs, assemblies) = match_interfaces(&annotated, &interfaces);

    let all_hints = annotated.hints.iter().copied().chain(hints).collect();
    let all_edges = annotated.edges.iter().cloned().chain(assemblies).collect();
    Ok(Diagram { edges: all_edges, stubs, hints: all_hints, ..annotated })
}
