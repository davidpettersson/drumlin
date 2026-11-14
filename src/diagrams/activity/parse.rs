// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Activity diagram grammar: recursive descent over the statements of the file.
//! A `lane` block has no node of its own. It sets the lane of what it holds and
//! its statements join the surrounding flow, so a flow is checked as one list.

use super::model::*;
use crate::syntax::{self, describe, err, keyword, name_of, reject_rest, Lexicon, Res, Stmts, Tok, T};

const LEXICON: Lexicon = Lexicon { arrows: &[], symbols: "{},;" };

const KEYWORDS: [&str; 17] = [
    "start", "stop", "end", "action", "send", "accept", "object", "note", "if", "else", "while", "repeat", "until",
    "fork", "and", "switch", "case",
];

/// A statement while parsing: its lane is still optional and its position kept.
#[derive(Clone, Debug)]
struct Draft {
    kind: DraftKind,
    lane: Option<usize>,
    note: Option<String>,
    at: (usize, usize),
    word: String,
}

#[derive(Clone, Debug)]
enum DraftKind {
    Start,
    Stop,
    End,
    Action(String),
    Send(String),
    Accept(String),
    Object(String),
    Decision { subject: Option<String>, arms: Vec<(Option<String>, Vec<Draft>)> },
    Fork(Vec<Vec<Draft>>),
    While { guard: String, body: Vec<Draft> },
    Repeat { body: Vec<Draft>, guard: String },
}

fn draft_falls(stmt: &Draft) -> bool {
    match &stmt.kind {
        DraftKind::Stop | DraftKind::End => false,
        DraftKind::Decision { arms, .. } => arms.iter().any(|(_, body)| flow_falls(body)),
        DraftKind::Fork(branches) => branches.iter().any(|branch| flow_falls(branch)),
        _ => true,
    }
}

fn flow_falls(flow: &[Draft]) -> bool {
    flow.last().is_none_or(draft_falls)
}

struct Ctx {
    style: crate::core::style::Style,
    title: Option<String>,
    lanes: Vec<String>,
    line_count: usize,
}

/// Where a block is being read: the lane it runs in and whether it is the top level.
#[derive(Clone, Copy)]
struct Scope {
    lane: Option<usize>,
    top: bool,
}

/// A guard may be written with its brackets, which are drawn anyway.
fn guard_text(text: &str) -> String {
    text.trim().strip_prefix('[').and_then(|t| t.strip_suffix(']')).unwrap_or(text.trim()).trim().to_string()
}

/// `<word> "text"`, where a one-word text may go unquoted.
fn named(toks: &[Tok]) -> Res<String> {
    let head = &toks[0];
    let word = keyword(toks);
    match toks.get(1).and_then(name_of) {
        Some(name) => {
            if let Some(tok) = toks.get(2) {
                return err(tok.line, tok.col, format!("unexpected {}; quote a name that has several words", describe(tok)));
            }
            Ok(name)
        }
        None => err(head.line, head.col, format!("expected a name after `{word}`")),
    }
}

/// `<word> {`, for headers that take nothing else.
fn bare_header(toks: &[Tok]) -> Res<()> {
    let head = &toks[0];
    let word = keyword(toks);
    match &toks[1..] {
        [Tok { t: T::Sym('{'), .. }] => Ok(()),
        [] => err(head.line, head.col, format!("expected `{{` after `{word}`")),
        [tok, ..] => err(tok.line, tok.col, format!("unexpected {} after `{word}`; expected `{{`", describe(tok))),
    }
}

/// `<word> "guard" {`, starting at `at` tokens into `toks`.
fn guarded_header(toks: &[Tok], at: usize, word: &str) -> Res<String> {
    let head = &toks[0];
    match &toks[at..] {
        [guard, Tok { t: T::Sym('{'), .. }] if name_of(guard).is_some() => Ok(guard_text(&name_of(guard).unwrap_or_default())),
        [Tok { t: T::Sym('{'), .. }] => err(head.line, head.col, format!("`{word}` needs a condition, for example `{word} \"ready\" {{`")),
        [] | [_] => err(head.line, head.col, format!("expected `{word} \"condition\" {{`")),
        [_, tok, ..] => err(tok.line, tok.col, format!("unexpected {} in `{word}`", describe(tok))),
    }
}

fn draft(kind: DraftKind, toks: &[Tok], scope: Scope) -> Draft {
    Draft { kind, lane: scope.lane, note: None, at: (toks[0].line, toks[0].col), word: keyword(toks).to_string() }
}

/// The next statement if it starts with `word`.
fn upcoming<'a>(rest: Stmts<'a>, word: &str) -> Option<&'a Vec<Tok>> {
    match rest.first() {
        Some(Ok(toks)) if keyword(toks) == word => Some(toks),
        _ => None,
    }
}

fn nested(scope: Scope) -> Scope {
    Scope { top: false, ..scope }
}

/// An `if` with its `else if` and `else` arms, all on one decision node.
fn decision<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>, scope: Scope) -> Res<(Ctx, Draft, Stmts<'a>)> {
    let guard = guarded_header(toks, 1, "if")?;
    let (ctx, body, rest) = block(ctx, rest, Some("if"), nested(scope), Vec::new())?;
    let (ctx, arms, rest) = else_arms(ctx, rest, scope, vec![(Some(guard), body)])?;
    if arms.iter().all(|(_, body)| body.is_empty()) {
        return err(toks[0].line, toks[0].col, "every branch of this `if` is empty; add a step to one of them");
    }
    Ok((ctx, draft(DraftKind::Decision { subject: None, arms }, toks, scope), rest))
}

/// Reads `else if`, `else "guard"` and `else` arms. Without a final `else`, an
/// empty one is added, so the decision always offers a way on.
fn else_arms<'a>(ctx: Ctx, rest: Stmts<'a>, scope: Scope, arms: Vec<(Option<String>, Vec<Draft>)>) -> Res<(Ctx, Vec<(Option<String>, Vec<Draft>)>, Stmts<'a>)> {
    let Some(next) = upcoming(rest, "else") else {
        let arms = arms.into_iter().chain([(Some("else".to_string()), Vec::new())]).collect();
        return Ok((ctx, arms, rest));
    };
    let after = &rest[1..];
    let is_else_if = next.get(1).is_some_and(|t| matches!(&t.t, T::Ident(w) if w == "if"));
    if is_else_if {
        let guard = guarded_header(next, 2, "else if")?;
        let (ctx, body, after) = block(ctx, after, Some("else if"), nested(scope), Vec::new())?;
        return else_arms(ctx, after, scope, arms.into_iter().chain([(Some(guard), body)]).collect());
    }
    let guard = match &next[1..] {
        [Tok { t: T::Sym('{'), .. }] => "else".to_string(),
        [guard, Tok { t: T::Sym('{'), .. }] if name_of(guard).is_some() => guard_text(&name_of(guard).unwrap_or_default()),
        [] | [_] => return err(next[0].line, next[0].col, "expected `else {`, `else \"guard\" {` or `else if \"condition\" {`"),
        [_, tok, ..] => return err(tok.line, tok.col, format!("unexpected {} in `else`", describe(tok))),
    };
    let (ctx, body, after) = block(ctx, after, Some("else"), nested(scope), Vec::new())?;
    if let Some(toks) = upcoming(after, "else") {
        return err(toks[0].line, toks[0].col, "no branch can follow a final `else`; write `else if` for the earlier one");
    }
    Ok((ctx, arms.into_iter().chain([(Some(guard), body)]).collect(), after))
}

fn switch_statement<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>, scope: Scope) -> Res<(Ctx, Draft, Stmts<'a>)> {
    let head = &toks[0];
    let subject = match &toks[1..] {
        [Tok { t: T::Sym('{'), .. }] => None,
        [subject, Tok { t: T::Sym('{'), .. }] if name_of(subject).is_some() => name_of(subject),
        [] | [_] => return err(head.line, head.col, "expected `switch \"subject\" {` or `switch {`"),
        [_, tok, ..] => return err(tok.line, tok.col, format!("unexpected {} in `switch`", describe(tok))),
    };
    let (ctx, arms, rest) = case_arms(ctx, rest, scope, head, Vec::new())?;
    if arms.iter().all(|(_, body)| body.is_empty()) {
        return err(head.line, head.col, "every `case` of this `switch` is empty; add a step to one of them");
    }
    Ok((ctx, draft(DraftKind::Decision { subject, arms }, toks, scope), rest))
}

fn case_arms<'a>(ctx: Ctx, rest: Stmts<'a>, scope: Scope, head: &Tok, arms: Vec<(Option<String>, Vec<Draft>)>) -> Res<(Ctx, Vec<(Option<String>, Vec<Draft>)>, Stmts<'a>)> {
    let Some((next, after)) = rest.split_first() else {
        return err(ctx.line_count, 1, "missing `}` for `switch`");
    };
    let toks = next.as_ref().map_err(String::clone)?;
    match keyword(toks) {
        "case" => {
            let guard = guarded_header(toks, 1, "case")?;
            let (ctx, body, after) = block(ctx, after, Some("case"), nested(scope), Vec::new())?;
            case_arms(ctx, after, scope, head, arms.into_iter().chain([(Some(guard), body)]).collect())
        }
        _ if toks[0].t == T::Sym('}') => match arms.len() {
            0 | 1 => err(head.line, head.col, "a `switch` needs at least two `case` branches; use `if` for one"),
            _ => Ok((ctx, arms, after)),
        },
        _ => err(toks[0].line, toks[0].col, format!("expected `case` inside `switch`, found {}", describe(&toks[0]))),
    }
}

fn fork_statement<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>, scope: Scope) -> Res<(Ctx, Draft, Stmts<'a>)> {
    bare_header(toks)?;
    let (ctx, first, rest) = block(ctx, rest, Some("fork"), nested(scope), Vec::new())?;
    let (ctx, branches, rest) = and_branches(ctx, rest, scope, vec![(toks[0].clone(), first)])?;
    if branches.len() < 2 {
        return err(toks[0].line, toks[0].col, "a `fork` needs at least two branches; add `and { ... }`");
    }
    if let Some((at, _)) = branches.iter().find(|(_, body)| body.is_empty()) {
        return err(at.line, at.col, "this `fork` branch is empty; each branch needs a step, or remove the branch");
    }
    let bodies = branches.into_iter().map(|(_, body)| body).collect();
    Ok((ctx, draft(DraftKind::Fork(bodies), toks, scope), rest))
}

type Branches = Vec<(Tok, Vec<Draft>)>;

fn and_branches<'a>(ctx: Ctx, rest: Stmts<'a>, scope: Scope, branches: Branches) -> Res<(Ctx, Branches, Stmts<'a>)> {
    match upcoming(rest, "and") {
        None => Ok((ctx, branches, rest)),
        Some(toks) => {
            bare_header(toks)?;
            let (ctx, body, after) = block(ctx, &rest[1..], Some("and"), nested(scope), Vec::new())?;
            and_branches(ctx, after, scope, branches.into_iter().chain([(toks[0].clone(), body)]).collect())
        }
    }
}

fn check_body(body: &[Draft], toks: &[Tok], word: &str) -> Res<()> {
    let head = &toks[0];
    if body.is_empty() {
        return err(head.line, head.col, format!("the body of this `{word}` is empty"));
    }
    if !flow_falls(body) {
        return err(head.line, head.col, format!("the body of this `{word}` always ends the flow, so it can never loop; use `if`"));
    }
    Ok(())
}

fn while_statement<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>, scope: Scope) -> Res<(Ctx, Draft, Stmts<'a>)> {
    let guard = guarded_header(toks, 1, "while")?;
    let (ctx, body, rest) = block(ctx, rest, Some("while"), nested(scope), Vec::new())?;
    check_body(&body, toks, "while")?;
    Ok((ctx, draft(DraftKind::While { guard, body }, toks, scope), rest))
}

fn repeat_statement<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>, scope: Scope) -> Res<(Ctx, Draft, Stmts<'a>)> {
    bare_header(toks)?;
    let (ctx, body, rest) = block(ctx, rest, Some("repeat"), nested(scope), Vec::new())?;
    check_body(&body, toks, "repeat")?;
    let Some(until) = upcoming(rest, "until") else {
        return err(toks[0].line, toks[0].col, "a `repeat` block needs `until \"condition\"` after its `}`");
    };
    let guard = match until.get(1).and_then(name_of) {
        Some(text) if until.len() == 2 => guard_text(&text),
        _ => return err(until[0].line, until[0].col, "expected `until \"condition\"`"),
    };
    Ok((ctx, draft(DraftKind::Repeat { body, guard }, toks, scope), &rest[1..]))
}

fn lane_statement<'a>(mut ctx: Ctx, toks: &[Tok], rest: Stmts<'a>, scope: Scope, items: Vec<Draft>) -> Res<(Ctx, Vec<Draft>, Stmts<'a>)> {
    let head = &toks[0];
    let (Some(name), Some(Tok { t: T::Sym('{'), .. }), None) = (toks.get(1).and_then(name_of), toks.get(2), toks.get(3)) else {
        return err(head.line, head.col, "expected `lane \"Name\" {`");
    };
    let index = ctx.lanes.iter().position(|lane| *lane == name).unwrap_or_else(|| {
        ctx.lanes.push(name);
        ctx.lanes.len() - 1
    });
    block(ctx, rest, Some("lane"), Scope { lane: Some(index), ..scope }, items)
}

/// Attaches a note to the statement before it, or to a compound's opening node.
fn attach_note(toks: &[Tok], mut items: Vec<Draft>) -> Res<Vec<Draft>> {
    let head = &toks[0];
    let Some(Tok { t: T::Label(text), .. }) = toks.get(1) else {
        return err(head.line, head.col, "expected `note: text`");
    };
    let Some(last) = items.last_mut() else {
        return err(head.line, head.col, "a note attaches to the step before it, and this flow has no step yet");
    };
    if last.note.is_some() {
        return err(head.line, head.col, "this step already has a note");
    }
    last.note = Some(text.clone());
    Ok(items)
}

fn unreachable_error(toks: &[Tok], items: &[Draft]) -> Option<String> {
    let last = items.last().filter(|last| !draft_falls(last))?;
    let (line, col) = (toks[0].line, toks[0].col);
    let why = match last.kind {
        DraftKind::Stop | DraftKind::End => format!("the flow already ended with `{}` on line {}", last.word, last.at.0),
        _ => format!("every branch of the `{}` on line {} ends the flow", last.word, last.at.0),
    };
    Some(format!("{line}:{col}: error: this statement can never run: {why}; remove it, or move it before the end"))
}

/// A word that is not a statement. Words that look like the header get a precise hint.
fn unknown_word<X>(toks: &[Tok], word: &str) -> Res<X> {
    let head = &toks[0];
    if word == "split" {
        return err(head.line, head.col, "`split` is not available; use `fork` for concurrent flows or `switch` for alternatives");
    }
    if toks[0].t == T::Sym('{') {
        return err(head.line, head.col, "unexpected `{`");
    }
    err(head.line, head.col, format!("expected a statement, found {}{}", describe(head), syntax::keyword_hint(&KEYWORDS, word)))
}

/// The word that a block-closing statement must follow, for one that stands alone.
fn orphan_error(word: &str) -> Option<&'static str> {
    match word {
        "else" => Some("`else` must follow the `}` of an `if`"),
        "and" => Some("`and` must follow the `}` of a `fork` branch"),
        "case" => Some("`case` belongs inside a `switch`"),
        "until" => Some("`until` must follow the `}` of a `repeat`"),
        _ => None,
    }
}

fn leaf(word: &str, toks: &[Tok]) -> Res<Option<DraftKind>> {
    Ok(Some(match word {
        "start" => {
            reject_rest(&toks[1..])?;
            DraftKind::Start
        }
        "stop" => {
            reject_rest(&toks[1..])?;
            DraftKind::Stop
        }
        "end" => {
            reject_rest(&toks[1..])?;
            DraftKind::End
        }
        "action" => DraftKind::Action(named(toks)?),
        "send" => DraftKind::Send(named(toks)?),
        "accept" => DraftKind::Accept(named(toks)?),
        "object" => DraftKind::Object(named(toks)?),
        _ => return Ok(None),
    }))
}

fn compound<'a>(word: &str, ctx: Ctx, toks: &[Tok], rest: Stmts<'a>, scope: Scope) -> Option<Res<(Ctx, Draft, Stmts<'a>)>> {
    Some(match word {
        "if" => decision(ctx, toks, rest, scope),
        "switch" => switch_statement(ctx, toks, rest, scope),
        "fork" => fork_statement(ctx, toks, rest, scope),
        "while" => while_statement(ctx, toks, rest, scope),
        "repeat" => repeat_statement(ctx, toks, rest, scope),
        _ => return None,
    })
}

/// The rules for where the first statement of the top-level flow can be.
fn check_placement(word: &str, toks: &[Tok], items: &[Draft], scope: Scope) -> Res<()> {
    let head = &toks[0];
    let first = items.is_empty() && scope.top;
    match word {
        "start" if !first => err(head.line, head.col, "an initial node has no incoming flow, so `start` must be the first statement"),
        "object" if matches!(items.last(), Some(Draft { kind: DraftKind::Start, .. })) => {
            err(head.line, head.col, "an initial node can only begin a control flow, so `object` cannot follow `start`; put an action or `accept` between them")
        }
        "if" | "switch" | "fork" | "while" | "repeat" if first => {
            err(head.line, head.col, format!("this `{word}` needs an incoming flow, so it cannot begin the activity; put `start` before it"))
        }
        _ => Ok(()),
    }
}

fn statement<'a>(ctx: Ctx, toks: &[Tok], rest: Stmts<'a>, scope: Scope, items: Vec<Draft>) -> Res<(Ctx, Vec<Draft>, Stmts<'a>)> {
    let word = keyword(toks).to_string();
    if word == "diagram" {
        return Ok((Ctx { title: syntax::header_title(toks, "activity")?, ..ctx }, items, rest));
    }
    if crate::diagrams::style::is_statement(toks) {
        return Ok((Ctx { style: crate::diagrams::style::statement(toks)?, ..ctx }, items, rest));
    }
    if crate::diagrams::engine::is_statement(toks) {
        return crate::diagrams::engine::not_layered("activity", toks[0].line, toks[0].col);
    }
    if word == "note" {
        return Ok((ctx, attach_note(toks, items)?, rest));
    }
    if word == "lane" {
        return lane_statement(ctx, toks, rest, scope, items);
    }
    if let Some(problem) = orphan_error(&word) {
        return err(toks[0].line, toks[0].col, problem);
    }
    if let Some(message) = unreachable_error(toks, &items) {
        return Err(message);
    }
    check_placement(&word, toks, &items, scope)?;
    if let Some(kind) = leaf(&word, toks)? {
        let items = items.into_iter().chain([draft(kind, toks, scope)]).collect();
        return Ok((ctx, items, rest));
    }
    match compound(&word, ctx, toks, rest, scope) {
        Some(parsed) => {
            let (ctx, stmt, rest) = parsed?;
            Ok((ctx, items.into_iter().chain([stmt]).collect(), rest))
        }
        None => unknown_word(toks, &word),
    }
}

/// Reads statements onto `items`, up to the closing `}` or the end of the file.
fn block<'a>(mut ctx: Ctx, mut rest: Stmts<'a>, open: Option<&str>, scope: Scope, mut items: Vec<Draft>) -> Res<(Ctx, Vec<Draft>, Stmts<'a>)> {
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
        (ctx, items, rest) = statement(ctx, toks, after, scope, items)?;
    }
}

/// The lane of the first statement in this flow that has one.
fn first_lane(flow: &[Draft]) -> Option<usize> {
    flow.iter().find_map(|stmt| stmt.lane.or_else(|| children(stmt).iter().find_map(|f| first_lane(f))))
}

fn children(stmt: &Draft) -> Vec<&[Draft]> {
    match &stmt.kind {
        DraftKind::Decision { arms, .. } => arms.iter().map(|(_, body)| body.as_slice()).collect(),
        DraftKind::Fork(branches) => branches.iter().map(Vec::as_slice).collect(),
        DraftKind::While { body, .. } | DraftKind::Repeat { body, .. } => vec![body.as_slice()],
        _ => Vec::new(),
    }
}

/// With lanes in use, an action, signal or object must say which lane it is in.
/// Control nodes may stand outside and take a neighbour's lane in `settle_flow`.
fn check_lanes(flow: &[Draft]) -> Res<()> {
    flow.iter().try_for_each(|stmt| {
        let needs_lane = matches!(stmt.kind, DraftKind::Action(_) | DraftKind::Send(_) | DraftKind::Accept(_) | DraftKind::Object(_));
        if needs_lane && stmt.lane.is_none() {
            return err(stmt.at.0, stmt.at.1, format!("this diagram uses lanes, so this `{}` must be inside a `lane` block", stmt.word));
        }
        children(stmt).into_iter().try_for_each(check_lanes)
    })
}

fn settle_arm(arm: (Option<String>, Vec<Draft>), lane: usize) -> Arm {
    Arm { guard: arm.0, body: settle_flow(arm.1, Some(lane)) }
}

fn settle_kind(kind: DraftKind, lane: usize) -> Kind {
    let inner = |body: Vec<Draft>| settle_flow(body, Some(lane));
    match kind {
        DraftKind::Start => Kind::Start,
        DraftKind::Stop => Kind::Stop,
        DraftKind::End => Kind::End,
        DraftKind::Action(text) => Kind::Action(text),
        DraftKind::Send(text) => Kind::Send(text),
        DraftKind::Accept(text) => Kind::Accept(text),
        DraftKind::Object(text) => Kind::Object(text),
        DraftKind::Decision { subject, arms } => Kind::Decision { subject, arms: arms.into_iter().map(|arm| settle_arm(arm, lane)).collect() },
        DraftKind::Fork(branches) => Kind::Fork(branches.into_iter().map(inner).collect()),
        DraftKind::While { guard, body } => Kind::While { guard, body: inner(body) },
        DraftKind::Repeat { body, guard } => Kind::Repeat { body: inner(body), guard },
    }
}

/// Gives every statement its final lane: its own, else where the flow already
/// is, else where the flow goes next, else the enclosing statement's lane.
fn settle_flow(flow: Vec<Draft>, inherit: Option<usize>) -> Vec<Stmt> {
    let lanes: Vec<Option<usize>> = flow.iter().map(|stmt| stmt.lane).collect();
    let nexts: Vec<Option<usize>> = (0..flow.len()).map(|i| first_lane(&flow[i..])).collect();
    let (_, settled) = flow.into_iter().enumerate().fold((inherit, Vec::new()), |(previous, mut out), (i, stmt)| {
        let lane = lanes[i].or(previous).or(first_lane(std::slice::from_ref(&stmt))).or(nexts[i]).or(inherit).unwrap_or(0);
        out.push(Stmt { lane, note: stmt.note, kind: settle_kind(stmt.kind, lane) });
        (Some(lane), out)
    });
    settled
}

/// A merge, join, or loop exit at the very end of the top level has an outgoing
/// flow with nowhere to go, which UML forbids (15.7.20, 15.7.21).
fn check_open_end(flow: &[Draft]) -> Res<()> {
    let Some(last) = flow.last() else { return Ok(()) };
    let dangling = match &last.kind {
        DraftKind::While { .. } | DraftKind::Repeat { .. } => true,
        DraftKind::Decision { arms, .. } => arms.iter().filter(|(_, body)| flow_falls(body)).count() > 1 || arms.iter().any(|(_, body)| body.is_empty()),
        DraftKind::Fork(branches) => branches.iter().filter(|branch| flow_falls(branch)).count() > 1,
        _ => false,
    };
    if dangling {
        return err(last.at.0, last.at.1, format!("the flow continues after this `{}` but nothing follows; add a step, or end it with `stop` or `end`", last.word));
    }
    Ok(())
}

pub fn parse(source: &str) -> Res<Diagram> {
    let stmts: Vec<Res<Vec<Tok>>> = syntax::statements(source, &LEXICON).collect();
    let ctx = Ctx { style: Default::default(), title: None, lanes: Vec::new(), line_count: source.lines().count() };
    let (ctx, flow, _) = block(ctx, &stmts, None, Scope { lane: None, top: true }, Vec::new())?;
    if flow.is_empty() {
        return err(1, 1, "an activity needs at least one step, for example `start`");
    }
    if !ctx.lanes.is_empty() {
        check_lanes(&flow)?;
    }
    check_open_end(&flow)?;
    let flow = settle_flow(flow, None);
    Ok(Diagram { style: ctx.style, title: ctx.title, lanes: ctx.lanes, flow })
}
