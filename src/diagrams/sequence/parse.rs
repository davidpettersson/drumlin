// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Sequence diagram grammar. Lines are cut into chunks first, so that a block
//! brace in a label is recognised, then parsed by recursive descent over the
//! statements of the file. Sequence order is the meaning, so names are resolved
//! as the walk reaches them and a participant must be declared before its use.

use super::model::*;
use crate::core::geometry::Side;
use crate::syntax::{self, describe, did_you_mean, err, keyword, name_of, reject_rest, Lexicon, Res, Stmts, Tok, T};
use std::collections::{BTreeMap, BTreeSet};

const LEXICON: Lexicon = Lexicon { arrows: &["-->", "->>", "->"], symbols: "{},;[]" };

/// Words that start a statement, for did-you-mean on a misspelt one.
const KEYWORDS: [&str; 18] = [
    "actor", "participant", "create", "destroy", "return", "note", "ref", "alt", "else", "opt", "loop", "break",
    "critical", "neg", "par", "and", "diagram", "over",
];

enum Piece {
    Text(usize, String),
    Sym(usize, char),
}

fn is_comment(chars: &[char], i: usize, in_label: bool) -> bool {
    let marker = match (chars[i], chars.get(i + 1)) {
        ('#', _) => 1,
        ('/', Some('/')) => 2,
        _ => return false,
    };
    let bounded = i == 0 || chars[i - 1].is_whitespace();
    let open_after = chars.get(i + marker).is_none_or(|c| c.is_whitespace());
    bounded && (!in_label || open_after)
}

/// A `{` or `}` is structure outside a label anywhere, and inside a label only
/// when spaced apart from the words around it.
fn is_brace(chars: &[char], i: usize, in_label: bool) -> bool {
    let free = |c: Option<&char>| c.is_none_or(|c| c.is_whitespace());
    matches!(chars[i], '{' | '}') && (!in_label || (free(chars.get(i.wrapping_sub(1))) && free(chars.get(i + 1))))
}

/// Cuts one source line into statement text and braces, dropping any comment.
fn scan(chars: &[char]) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let (mut start, mut in_label, mut in_string) = (0, false, false);
    let flush = |pieces: &mut Vec<Piece>, from: usize, to: usize| {
        if chars[from..to].iter().any(|c| !c.is_whitespace()) {
            pieces.push(Piece::Text(from, chars[from..to].iter().collect()));
        }
    };
    let mut end = chars.len();
    for i in 0..chars.len() {
        let c = chars[i];
        if in_string {
            in_string = c != '"';
        } else if c == '"' {
            in_string = true;
        } else if is_comment(chars, i, in_label) {
            end = i;
            break;
        } else if c == ':' {
            in_label = true;
        } else if is_brace(chars, i, in_label) || (c == ';' && !in_label) {
            flush(&mut pieces, start, i);
            pieces.push(Piece::Sym(i, c));
            start = i + 1;
            in_label = false;
        }
    }
    flush(&mut pieces, start, end);
    pieces
}

/// `return` takes free text, so words like `f(x)` need no quoting.
fn return_tokens(col: usize, text: &str, line: usize) -> Option<Vec<Tok>> {
    let lead = text.chars().take_while(|c| c.is_whitespace()).count();
    let rest = text.trim_start().strip_prefix("return")?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let head = Tok { t: T::Ident("return".into()), line, col: col + lead + 1 };
    let body = rest.trim().strip_prefix(':').unwrap_or(rest).trim().to_string();
    Some(match body.is_empty() {
        true => vec![head],
        false => vec![head, Tok { t: T::Label(body), line, col: col + lead + 1 }],
    })
}

fn line_statements(text: &str, line: usize) -> Res<Vec<Vec<Tok>>> {
    let chars: Vec<char> = text.chars().collect();
    let tokens = scan(&chars).into_iter().try_fold(Vec::new(), |mut tokens, piece| {
        match piece {
            Piece::Sym(col, c) => tokens.push(Tok { t: T::Sym(c), line, col: col + 1 }),
            Piece::Text(col, s) => match return_tokens(col, &s, line) {
                Some(returned) => tokens.extend(returned),
                None => tokens.extend(syntax::tokenize(&s, line, &LEXICON)?.into_iter().map(|t| Tok { col: t.col + col, ..t })),
            },
        }
        Ok::<_, String>(tokens)
    })?;
    Ok(syntax::split_statements(tokens))
}

/// The statements of the file. Lex errors are kept in place, so they are only
/// reported when the walk reaches them.
fn statements(source: &str) -> Vec<Res<Vec<Tok>>> {
    source
        .lines()
        .enumerate()
        .flat_map(|(index, text)| match line_statements(text, index + 1) {
            Ok(list) => list.into_iter().map(Ok).collect::<Vec<_>>(),
            Err(message) => vec![Err(message)],
        })
        .collect()
}

/// One enclosing message block: who executes it and who it would reply to.
#[derive(Clone, Copy)]
struct Call {
    callee: End,
    caller: End,
}

/// Everything the walk has resolved so far.
struct Ctx {
    style: crate::core::style::Style,
    title: Option<String>,
    participants: Vec<Participant>,
    names: BTreeMap<String, usize>,
    destroyed: BTreeSet<usize>,
    calls: Vec<Call>,
    line_count: usize,
}

/// A fully quoted label keeps a literal `{`, and loses the quotes when drawn.
fn label_text(text: &str) -> String {
    let unquoted = match text.strip_prefix('"').and_then(|t| t.strip_suffix('"')).filter(|t| !t.contains('"')) {
        Some(inner) => inner,
        None => text,
    };
    syntax::guillemets(unquoted)
}

fn lookup(ctx: &Ctx, tok: &Tok) -> Res<usize> {
    let name = name_of(tok).ok_or_else(|| format!("{}:{}: error: expected a participant name, found {}", tok.line, tok.col, describe(tok)))?;
    match ctx.names.get(&name) {
        Some(&index) => Ok(index),
        None => {
            let hint = did_you_mean(ctx.names.keys(), &name);
            err(tok.line, tok.col, format!("unknown participant `{name}`{hint}; declare it with `participant` before its first use"))
        }
    }
}

fn alive(ctx: &Ctx, tok: &Tok) -> Res<usize> {
    let index = lookup(ctx, tok)?;
    if ctx.destroyed.contains(&index) {
        return err(tok.line, tok.col, format!("`{}` was already destroyed", ctx.participants[index].name));
    }
    Ok(index)
}

fn declare(mut ctx: Ctx, name_tok: &Tok, id: String, participant: Participant) -> Res<(Ctx, usize)> {
    if ctx.names.contains_key(&id) {
        return err(name_tok.line, name_tok.col, format!("`{id}` is already declared"));
    }
    let index = ctx.participants.len();
    ctx.participants.push(participant);
    ctx.names.insert(id, index);
    Ok((ctx, index))
}

/// `name [as alias] [<<stereotype>>]` at the start of `toks`, for a new participant.
fn new_participant(ctx: Ctx, toks: &[Tok], figure: Figure) -> Res<(Ctx, usize)> {
    let name_tok = &toks[0];
    let Some(name) = name_of(name_tok) else {
        return err(name_tok.line, name_tok.col, format!("expected a name, found {}", describe(name_tok)));
    };
    let options = syntax::decl_options(&toks[1..], syntax::DeclOptions::default())?;
    if options.opens {
        return err(name_tok.line, name_tok.col, "a participant has no `{ }` body");
    }
    let id = options.alias.unwrap_or_else(|| name.clone());
    declare(ctx, name_tok, id, Participant { name, stereotype: options.stereotype, figure })
}

/// Splits off a trailing `[: label] [{]`.
fn trailer(toks: &[Tok]) -> Res<(&[Tok], Option<String>, bool)> {
    let (toks, opens) = match toks.split_last() {
        Some((Tok { t: T::Sym('{'), .. }, front)) => (front, true),
        _ => (toks, false),
    };
    let (toks, label) = match toks.split_last() {
        Some((Tok { t: T::Label(text), .. }, front)) => (front, Some(label_text(text))),
        _ => (toks, None),
    };
    Ok((toks, label, opens))
}

fn sort_of(arrow: &str) -> Sort {
    match arrow {
        "-->" => Sort::Reply,
        "->>" => Sort::Async,
        _ => Sort::Sync,
    }
}

fn block_opener(sort: Sort, to: End) -> Option<&'static str> {
    match (sort, to) {
        (Sort::Reply, _) => Some("a reply cannot open a `{ }` block"),
        (_, End::Boundary(_)) => Some("a lost message has no receiver to run a `{ }` block"),
        _ => None,
    }
}

/// The message statement forms, with the participant created by `new` if any.
fn message(ctx: Ctx, toks: &[Tok]) -> Res<(Ctx, Message, bool)> {
    let head = &toks[0];
    let (front, label, opens) = trailer(toks)?;
    let (ctx, from, to, sort, at) = match front {
        [Tok { t: T::Sym('['), .. }, Tok { t: T::Arrow(arrow), .. }, target, rest @ ..] => {
            reject_rest(rest)?;
            let to = End::Life(alive(&ctx, target)?);
            (ctx, End::Boundary(Side::Left), to, sort_of(arrow), target)
        }
        [source, Tok { t: T::Arrow(arrow), .. }, Tok { t: T::Sym(']'), .. }, rest @ ..] => {
            reject_rest(rest)?;
            let from = End::Life(alive(&ctx, source)?);
            (ctx, from, End::Boundary(Side::Right), sort_of(arrow), source)
        }
        [source, Tok { t: T::Arrow(_), .. }, Tok { t: T::Ident(k), .. }, declared @ ..] if k == "new" && !declared.is_empty() => {
            let from = End::Life(alive(&ctx, source)?);
            let (ctx, created) = new_participant(ctx, declared, Figure::Box)?;
            (ctx, from, End::Life(created), Sort::Create, source)
        }
        [source, Tok { t: T::Arrow(arrow), .. }, target, rest @ ..] => {
            reject_rest(rest)?;
            let from = End::Life(alive(&ctx, source)?);
            let to = End::Life(alive(&ctx, target)?);
            (ctx, from, to, sort_of(arrow), source)
        }
        _ => return not_a_message(toks, head),
    };
    if opens {
        if let Some(problem) = block_opener(sort, to) {
            return err(at.line, at.col, problem);
        }
    }
    Ok((ctx, Message { from, to, sort, label }, opens))
}

fn not_a_message<X>(toks: &[Tok], head: &Tok) -> Res<X> {
    if let (T::Ident(word), Some(next)) = (&head.t, toks.get(1)) {
        return err(next.line, next.col, format!("expected an arrow after `{word}`, found {}{}", describe(next), syntax::keyword_hint(&KEYWORDS, word)));
    }
    if let (T::Ident(word), None) = (&head.t, toks.get(1)) {
        return err(head.line, head.col, format!("incomplete statement `{word}`{}", syntax::keyword_hint(&KEYWORDS, word)));
    }
    err(head.line, head.col, format!("unexpected {}", describe(head)))
}

/// `over a, b` / `left of a` / `right of a`, then the label.
fn place_and_text(ctx: &Ctx, toks: &[Tok]) -> Res<(NotePlace, String)> {
    let head = &toks[0];
    let (front, text, opens) = trailer(&toks[1..])?;
    let Some(text) = text.filter(|_| !opens) else {
        return err(head.line, head.col, "expected `note over <participant>: <text>`, or `left of` or `right of`");
    };
    let place = match front {
        [Tok { t: T::Ident(k), .. }, list @ ..] if k == "over" => NotePlace::Over(participant_list(ctx, list, head)?),
        [Tok { t: T::Ident(k), .. }, Tok { t: T::Ident(of), .. }, target] if of == "of" && k == "left" => NotePlace::LeftOf(alive(ctx, target)?),
        [Tok { t: T::Ident(k), .. }, Tok { t: T::Ident(of), .. }, target] if of == "of" && k == "right" => NotePlace::RightOf(alive(ctx, target)?),
        _ => return err(head.line, head.col, "expected `note over <participant>: <text>`, or `left of` or `right of`"),
    };
    Ok((place, text))
}

fn participant_list(ctx: &Ctx, list: &[Tok], head: &Tok) -> Res<Vec<usize>> {
    if list.is_empty() {
        return err(head.line, head.col, "expected at least one participant after `over`");
    }
    list.iter().filter(|tok| tok.t != T::Sym(',')).map(|tok| alive(ctx, tok)).collect()
}

fn reference(ctx: &Ctx, toks: &[Tok]) -> Res<(Vec<usize>, String)> {
    let head = &toks[0];
    let (front, text, _) = trailer(&toks[1..])?;
    match (front, text) {
        ([Tok { t: T::Ident(k), .. }, list @ ..], Some(text)) if k == "over" => Ok((participant_list(ctx, list, head)?, text)),
        _ => err(head.line, head.col, "expected `ref over <participant>[, <participant>]: <name>`"),
    }
}

fn operator_of(word: &str) -> Option<Operator> {
    match word {
        "alt" => Some(Operator::Alt),
        "opt" => Some(Operator::Opt),
        "loop" => Some(Operator::Loop),
        "break" => Some(Operator::Break),
        "critical" => Some(Operator::Critical),
        "neg" => Some(Operator::Neg),
        "par" => Some(Operator::Par),
        _ => None,
    }
}

/// `<keyword> ["guard"] {`. An unquoted one-word guard is also accepted.
fn operand_header(toks: &[Tok]) -> Res<Option<String>> {
    let head = &toks[0];
    let word = keyword(toks).to_string();
    match &toks[1..] {
        [Tok { t: T::Sym('{'), .. }] => Ok(None),
        [guard, Tok { t: T::Sym('{'), .. }] if name_of(guard).is_some() => Ok(name_of(guard)),
        [] | [_] => err(head.line, head.col, format!("expected `{{` after `{word}`")),
        [_, tok, ..] => err(tok.line, tok.col, format!("unexpected {} in `{word}`", describe(tok))),
    }
}

/// The word that adds an operand to `operator`, if it has one.
fn continuation(operator: Operator) -> Option<&'static str> {
    match operator {
        Operator::Alt => Some("else"),
        Operator::Par => Some("and"),
        _ => None,
    }
}

fn operand<'a>(ctx: Ctx, header: &[Tok], rest: Stmts<'a>, open: &str) -> Res<(Ctx, Operand, Stmts<'a>)> {
    let guard = operand_header(header)?;
    let (ctx, body, rest) = block(ctx, rest, Some(open))?;
    Ok((ctx, Operand { guard, body }, rest))
}

/// `(1,5)`-style loop bounds become part of the operator, as in `loop(1,5)`.
fn split_bounds(operator: Operator, guard: Option<String>) -> (Option<String>, Option<String>) {
    match guard {
        Some(g) if operator == Operator::Loop && g.starts_with('(') && g.ends_with(')') => (Some(g), None),
        other => (None, other),
    }
}

fn merged_destroyed(operator: Operator, entry: &BTreeSet<usize>, ends: Vec<BTreeSet<usize>>) -> BTreeSet<usize> {
    match operator {
        Operator::Alt if ends.len() > 1 => ends.into_iter().reduce(|a, b| a.intersection(&b).copied().collect()).unwrap_or_default(),
        Operator::Par => ends.into_iter().fold(entry.clone(), |a, b| a.union(&b).copied().collect()),
        _ => entry.clone(),
    }
}

fn fragment<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>) -> Res<(Ctx, Item, Stmts<'a>)> {
    let head = &toks[0];
    let word = keyword(toks).to_string();
    let operator = operator_of(&word).unwrap_or(Operator::Opt);
    let entry = ctx.destroyed.clone();
    let (ctx, first, rest) = operand(ctx, toks, rest, &word)?;
    let more = continuation(operator);
    let mut ends = vec![ctx.destroyed.clone()];
    let (mut ctx, mut operands, mut rest) = (Ctx { destroyed: entry.clone(), ..ctx }, vec![first], rest);
    while let Some(Ok(next)) = rest.first().filter(|next| matches!(next, Ok(n) if more == Some(keyword(n)))) {
        let (done, parsed, after) = operand(ctx, next, &rest[1..], &word)?;
        ends.push(done.destroyed.clone());
        ctx = Ctx { destroyed: entry.clone(), ..done };
        operands.push(parsed);
        rest = after;
    }
    if operands.iter().all(|o| o.body.is_empty()) {
        return err(head.line, head.col, format!("the `{word}` fragment is empty"));
    }
    let (bounds, guard) = split_bounds(operator, operands[0].guard.clone());
    operands[0].guard = guard;
    let destroyed = merged_destroyed(operator, &entry, ends);
    Ok((Ctx { destroyed, ..ctx }, Item::Fragment(Fragment { operator, bounds, operands }), rest))
}

fn call_body<'a>(ctx: Ctx, message: &Message, rest: Stmts<'a>, opener: &str) -> Res<(Ctx, Vec<Item>, Stmts<'a>)> {
    let mut ctx = ctx;
    ctx.calls.push(Call { callee: message.to, caller: message.from });
    let (mut ctx, body, rest) = block(ctx, rest, Some(opener))?;
    ctx.calls.pop();
    Ok((ctx, body, rest))
}

fn message_statement<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>) -> Res<(Ctx, Item, Stmts<'a>)> {
    let opener = message_name(toks);
    let (ctx, parsed, opens) = message(ctx, toks)?;
    if !opens {
        return Ok((ctx, Item::Message(parsed), rest));
    }
    let (ctx, body, rest) = call_body(ctx, &parsed, rest, &opener)?;
    Ok((ctx, Item::Call(parsed, body), rest))
}

fn message_name(toks: &[Tok]) -> String {
    toks.iter().find_map(|tok| name_of(tok)).unwrap_or_else(|| "message".to_string())
}

/// `create <name> [as alias] [<<stereotype>>] [: label] [{`, sent by the running block.
fn create<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>) -> Res<(Ctx, Item, Stmts<'a>)> {
    let head = &toks[0];
    if toks.len() < 2 {
        return err(head.line, head.col, "expected a name after `create`");
    }
    let Some(sender) = ctx.calls.last().map(|call| call.callee) else {
        return err(head.line, head.col, "`create` outside a block has no sender; write `<sender> -> new <name>`");
    };
    let (front, label, opens) = trailer(toks)?;
    let (ctx, created) = new_participant(ctx, &front[1..], Figure::Box)?;
    let parsed = Message { from: sender, to: End::Life(created), sort: Sort::Create, label };
    if !opens {
        return Ok((ctx, Item::Message(parsed), rest));
    }
    let (ctx, body, rest) = call_body(ctx, &parsed, rest, "create")?;
    Ok((ctx, Item::Call(parsed, body), rest))
}

fn return_message(ctx: &Ctx, toks: &[Tok]) -> Res<Item> {
    let head = &toks[0];
    let Some(call) = ctx.calls.last() else {
        return err(head.line, head.col, "`return` belongs inside a message block");
    };
    let label = match toks.get(1) {
        Some(Tok { t: T::Label(text), .. }) => Some(label_text(text)),
        _ => None,
    };
    Ok(Item::Return(Message { from: call.callee, to: call.caller, sort: Sort::Reply, label }))
}

fn declaration<'a>(ctx: Ctx, toks: &[Tok], figure: Figure, rest: Stmts<'a>) -> Res<(Ctx, Vec<Item>, Stmts<'a>)> {
    let head = &toks[0];
    if toks.len() < 2 {
        return err(head.line, head.col, "expected a name after the keyword");
    }
    let (ctx, _) = new_participant(ctx, &toks[1..], figure)?;
    Ok((ctx, Vec::new(), rest))
}

fn note<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>) -> Res<(Ctx, Vec<Item>, Stmts<'a>)> {
    let (place, text) = place_and_text(&ctx, toks)?;
    Ok((ctx, vec![Item::Note(place, text)], rest))
}

fn destroy<'a>(mut ctx: Ctx, toks: &[Tok], rest: Stmts<'a>) -> Res<(Ctx, Vec<Item>, Stmts<'a>)> {
    let head = &toks[0];
    let Some(target) = toks.get(1) else {
        return err(head.line, head.col, "expected a participant after `destroy`");
    };
    let index = alive(&ctx, target)?;
    reject_rest(&toks[2..])?;
    ctx.destroyed.insert(index);
    Ok((ctx, vec![Item::Destroy(index)], rest))
}

/// One statement, as zero or one item plus the statements left to read.
fn statement<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>) -> Res<(Ctx, Vec<Item>, Stmts<'a>)> {
    let head = &toks[0];
    let word = keyword(toks).to_string();
    let single = |(ctx, item, rest)| Ok((ctx, vec![item], rest));
    match word.as_str() {
        "diagram" => Ok((Ctx { title: syntax::header_title(toks, "sequence")?, ..ctx }, Vec::new(), rest)),
        "style" if crate::diagrams::style::is_statement(toks) => Ok((Ctx { style: crate::diagrams::style::statement(toks)?, ..ctx }, Vec::new(), rest)),
        "layout" if crate::diagrams::engine::is_statement(toks) => crate::diagrams::engine::not_layered("sequence", head.line, head.col),
        "participant" => declaration(ctx, toks, Figure::Box, rest),
        "actor" => declaration(ctx, toks, Figure::Actor, rest),
        "create" => create(ctx, toks, rest).and_then(single),
        "destroy" => destroy(ctx, toks, rest),
        "return" => {
            let item = return_message(&ctx, toks)?;
            Ok((ctx, vec![item], rest))
        }
        "note" => note(ctx, toks, rest),
        "ref" => {
            let (over, text) = reference(&ctx, toks)?;
            Ok((ctx, vec![Item::Ref(over, text)], rest))
        }
        "else" | "and" => err(head.line, head.col, format!("`{word}` must follow the `}}` of {}", if word == "else" { "an `alt`" } else { "a `par`" })),
        _ if operator_of(&word).is_some() => fragment(ctx, toks, rest).and_then(single),
        _ => message_statement(ctx, toks, rest).and_then(single),
    }
}

/// Reads statements up to the closing `}` of a block, or to the end of the file.
fn block<'a>(mut ctx: Ctx, mut rest: Stmts<'a>, open: Option<&str>) -> Res<(Ctx, Vec<Item>, Stmts<'a>)> {
    let mut items = Vec::new();
    loop {
        let Some((next, after)) = rest.split_first() else {
            return match open {
                Some(name) => err(ctx.line_count, 1, format!("missing `}}` for `{name}`")),
                None => Ok((ctx, items, rest)),
            };
        };
        let toks = next.as_ref().map_err(String::clone)?;
        if toks[0].t == T::Sym('}') {
            return match open {
                Some(_) => Ok((ctx, items, after)),
                None => err(toks[0].line, toks[0].col, "unmatched `}`"),
            };
        }
        let (updated, produced, remaining) = statement(ctx, toks, after)?;
        ctx = updated;
        items.extend(produced);
        rest = remaining;
    }
}

pub fn parse(source: &str) -> Res<Diagram> {
    let stmts = statements(source);
    let ctx = Ctx {
        style: Default::default(),
        title: None,
        participants: Vec::new(),
        names: BTreeMap::new(),
        destroyed: BTreeSet::new(),
        calls: Vec::new(),
        line_count: source.lines().count(),
    };
    let (ctx, items, _) = block(ctx, &stmts, None)?;
    if ctx.participants.is_empty() {
        return err(1, 1, "a sequence diagram needs at least one participant; declare one with `participant <name>`");
    }
    Ok(Diagram { style: ctx.style, title: ctx.title, participants: ctx.participants, items })
}
