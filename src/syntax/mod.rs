// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Shared lexing, tokens, error formatting and did-you-mean suggestions.
//!
//! Lexing is lazy, line by line: an error on an earlier line is always reported
//! before an error on a later one, whichever stage would find it.
//!
//! `statement` holds the statement helpers every grammar shares.

mod statement;

pub use statement::{declared_name, header_title, keyword, keyword_hint, reject_rest, Stmts};

#[derive(Clone, Debug, PartialEq)]
pub enum T {
    Ident(String),
    Str(String),
    Stereo(String),
    Arrow(String),
    Sym(char),
    Label(String),
}

#[derive(Clone, Debug)]
pub struct Tok {
    pub t: T,
    pub line: usize,
    pub col: usize,
}

pub type Res<X> = Result<X, String>;

/// What a diagram kind's grammar treats as arrows and single-character symbols.
/// List longer arrows before their prefixes.
pub struct Lexicon {
    pub arrows: &'static [&'static str],
    pub symbols: &'static str,
}

pub fn err<X>(line: usize, col: usize, message: impl AsRef<str>) -> Res<X> {
    Err(format!("{line}:{col}: error: {}", message.as_ref()))
}

/// The diagram kinds that have an implementation.
pub const IMPLEMENTED: [&str; 5] = ["component", "package", "sequence", "state", "activity"];

fn implemented_list() -> String {
    IMPLEMENTED.map(|kind| format!("`{kind}`")).join(", ")
}

pub fn unsupported_kind<X>(line: usize, col: usize, kind: &str) -> Res<X> {
    err(line, col, format!("diagram kind `{kind}` is not implemented; this prototype supports {}", implemented_list()))
}

/// The error for a `diagram <kind>` line that is not the file's first statement.
pub fn misplaced_kind<X>(line: usize, col: usize, kind: &str) -> Res<X> {
    if IMPLEMENTED.contains(&kind) {
        err(line, col, "the `diagram` header must be the first statement")
    } else {
        unsupported_kind(line, col, kind)
    }
}

/// Splits one source line into tokens.
pub fn tokenize(text: &str, line: usize, lexicon: &Lexicon) -> Res<Vec<Tok>> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let col = i + 1;
        let next = chars.get(i + 1).copied();
        if c.is_whitespace() {
            i += 1;
        } else if c == '#' || (c == '/' && next == Some('/')) {
            break;
        } else if c == '"' {
            let start = i + 1;
            let Some(end) = chars[start..].iter().position(|&x| x == '"') else {
                return err(line, col, "unterminated string");
            };
            out.push(Tok { t: T::Str(chars[start..start + end].iter().collect()), line, col });
            i = start + end + 1;
        } else if c == ':' {
            let rest: String = chars[i + 1..].iter().collect();
            out.push(Tok { t: T::Label(rest.trim().to_string()), line, col });
            break;
        } else if c == '<' && next == Some('<') {
            let rest: String = chars[i + 2..].iter().collect();
            let Some(end) = rest.find(">>") else { return err(line, col, "unterminated `<<`") };
            let inner = &rest[..end];
            out.push(Tok { t: T::Stereo(inner.trim().to_string()), line, col });
            i += 2 + inner.chars().count() + 2;
        } else if let Some(arrow) = lexicon.arrows.iter().find(|a| chars[i..].starts_with(&a.chars().collect::<Vec<_>>())) {
            out.push(Tok { t: T::Arrow(arrow.to_string()), line, col });
            i += arrow.len();
        } else if c.is_alphanumeric() || c == '_' {
            let end = ident_end(&chars, i);
            out.push(Tok { t: T::Ident(chars[i..end].iter().collect()), line, col });
            i = end;
        } else if lexicon.symbols.contains(c) {
            out.push(Tok { t: T::Sym(c), line, col });
            i += 1;
        } else {
            return err(line, col, format!("unexpected character `{c}`"));
        }
    }
    Ok(out)
}

/// Identifiers may contain `-` and `.` when an identifier character follows.
fn ident_end(chars: &[char], start: usize) -> usize {
    let joins = |x: char, after: char| (x == '-' || x == '.') && (after.is_alphanumeric() || after == '_');
    (start..chars.len())
        .find(|&i| {
            let x = chars[i];
            let after = chars.get(i + 1).copied().unwrap_or(' ');
            !(x.is_alphanumeric() || x == '_' || joins(x, after))
        })
        .unwrap_or(chars.len())
}

/// Cuts a line's tokens into statements: `{` ends one, `}` stands alone.
pub fn split_statements(toks: Vec<Tok>) -> Vec<Vec<Tok>> {
    let mut statements = Vec::new();
    let mut piece: Vec<Tok> = Vec::new();
    for tok in toks {
        match tok.t {
            T::Sym('{') => {
                piece.push(tok);
                statements.push(std::mem::take(&mut piece));
            }
            T::Sym('}') => {
                if !piece.is_empty() {
                    statements.push(std::mem::take(&mut piece));
                }
                statements.push(vec![tok]);
            }
            T::Sym(';') if !piece.is_empty() => statements.push(std::mem::take(&mut piece)),
            T::Sym(';') => {}
            _ => piece.push(tok),
        }
    }
    if !piece.is_empty() {
        statements.push(piece);
    }
    statements
}

/// The statements of a source file, lazily and in order.
pub fn statements<'a>(source: &'a str, lexicon: &'a Lexicon) -> impl Iterator<Item = Res<Vec<Tok>>> + 'a {
    source.lines().enumerate().flat_map(move |(index, text)| match tokenize(text, index + 1, lexicon) {
        Ok(toks) => split_statements(toks).into_iter().map(Ok).collect::<Vec<_>>(),
        Err(message) => vec![Err(message)],
    })
}

pub struct Header {
    pub kind: String,
    pub line: usize,
    pub col: usize,
}

/// Finds a `diagram <kind>` header on the first line that has tokens. Returns
/// `None` when that line is anything else or cannot be lexed, so the default
/// grammar reports the problem.
pub fn header(source: &str) -> Option<Header> {
    const HEADER: Lexicon = Lexicon { arrows: &[], symbols: "{},;" };
    source
        .lines()
        .enumerate()
        .map(|(index, text)| tokenize(text, index + 1, &HEADER))
        .find(|toks| !matches!(toks, Ok(toks) if toks.is_empty()))?
        .ok()
        .and_then(|toks| match toks.as_slice() {
            [Tok { t: T::Ident(word), line, col }, kind, ..] if word == "diagram" => {
                name_of(kind).map(|kind| Header { kind, line: *line, col: *col })
            }
            _ => None,
        })
}

/// What may follow the name in a declaration.
#[derive(Default)]
pub struct DeclOptions {
    pub alias: Option<String>,
    pub stereotype: Option<String>,
    pub opens: bool,
}

/// Reads `as alias`, `<<stereotype>>` and a trailing `{`.
pub fn decl_options(rest: &[Tok], options: DeclOptions) -> Res<DeclOptions> {
    match rest {
        [] => Ok(options),
        [Tok { t: T::Ident(k), .. }, ..] if k == "as" => match rest.get(1).and_then(name_of) {
            Some(alias) => decl_options(&rest[2..], DeclOptions { alias: Some(alias), ..options }),
            None => err(rest[0].line, rest[0].col, "expected an alias after `as`"),
        },
        [Tok { t: T::Stereo(s), .. }, tail @ ..] => decl_options(tail, DeclOptions { stereotype: Some(s.clone()), ..options }),
        [Tok { t: T::Sym('{'), .. }] => Ok(DeclOptions { opens: true, ..options }),
        [tok, ..] => err(tok.line, tok.col, format!("unexpected {}", describe(tok))),
    }
}

pub fn name_of(tok: &Tok) -> Option<String> {
    match &tok.t {
        T::Ident(s) | T::Str(s) => Some(s.clone()),
        _ => None,
    }
}

pub fn describe(tok: &Tok) -> String {
    match &tok.t {
        T::Ident(s) => format!("`{s}`"),
        T::Str(s) => format!("\"{s}\""),
        T::Stereo(s) => format!("<<{s}>>"),
        T::Arrow(s) => format!("`{s}`"),
        T::Sym(c) => format!("`{c}`"),
        T::Label(_) => "label".to_string(),
    }
}

/// Writes the `<<keyword>>` forms in a label with guillemets (UML 2.5.1, 7.2.4 and Annex C).
pub fn guillemets(label: &str) -> String {
    label.replace("<<", "\u{ab}").replace(">>", "\u{bb}")
}

/// Levenshtein distance. Only called on identifiers, so its cost is small.
pub fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let last = a.iter().fold((0..=b.len()).collect::<Vec<usize>>(), |row, &ca| {
        b.iter().enumerate().fold(vec![row[0] + 1], |mut next, (j, &cb)| {
            let value = (row[j + 1] + 1).min(next[j] + 1).min(row[j] + usize::from(ca != cb));
            next.push(value);
            next
        })
    });
    last[b.len()]
}

/// The `; did you mean ...?` suffix for an unknown name, or an empty string.
pub fn did_you_mean<'a>(candidates: impl Iterator<Item = &'a String>, name: &str) -> String {
    match candidates.min_by_key(|candidate| distance(candidate, name)) {
        Some(best) if distance(best, name) <= 2 + name.len() / 4 => format!("; did you mean `{best}`?"),
        _ => String::new(),
    }
}
