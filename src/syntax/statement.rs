// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Statement helpers every grammar shares: the first keyword, the `diagram`
//! header line, a declared name, rejecting what is left over, and suggesting a
//! keyword for a misspelt one. They take a
//! statement's tokens and return plain data, so each parser stores the result.

use super::{describe, did_you_mean, err, name_of, Res, Tok, T};

/// The statements still to read, for a parser that walks blocks by recursive descent.
pub type Stmts<'a> = &'a [Res<Vec<Tok>>];

/// The identifier a statement starts with, or `""` when it starts with anything else.
pub fn keyword(toks: &[Tok]) -> &str {
    match toks.first() {
        Some(Tok { t: T::Ident(word), .. }) => word,
        _ => "",
    }
}

/// An error at the first of `rest`, if there is one.
pub fn reject_rest(rest: &[Tok]) -> Res<()> {
    match rest.first() {
        None => Ok(()),
        Some(tok) => err(tok.line, tok.col, format!("unexpected {}", describe(tok))),
    }
}

/// The title of a `diagram <kind> ["Title"]` statement in a file of `kind`.
/// Another kind's header here is misplaced, since the header must come first.
pub fn header_title(toks: &[Tok], kind: &str) -> Res<Option<String>> {
    let head = &toks[0];
    match toks.get(1).and_then(name_of).as_deref() {
        Some(word) if word == kind => Ok(toks.get(2).and_then(name_of)),
        Some(other) => super::misplaced_kind(head.line, head.col, other),
        None => err(head.line, head.col, "expected a diagram kind"),
    }
}

/// The name after a declaring keyword, with its token.
pub fn declared_name(toks: &[Tok]) -> Res<(&Tok, String)> {
    let head = &toks[0];
    let Some(name_tok) = toks.get(1) else {
        return err(head.line, head.col, "expected a name after the keyword");
    };
    match name_of(name_tok) {
        Some(name) => Ok((name_tok, name)),
        None => err(name_tok.line, name_tok.col, format!("expected a name, found {}", describe(name_tok))),
    }
}

/// The `; did you mean ...?` suffix for a misspelt statement keyword, or an empty string.
pub fn keyword_hint(keywords: &[&str], word: &str) -> String {
    let words: Vec<String> = keywords.iter().map(|k| k.to_string()).collect();
    did_you_mean(words.iter(), word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{tokenize, Lexicon};

    fn toks(text: &str) -> Vec<Tok> {
        tokenize(text, 1, &Lexicon { arrows: &["-->"], symbols: "{}," }).unwrap()
    }

    #[test]
    fn the_keyword_is_a_leading_identifier() {
        assert_eq!(keyword(&toks("state A")), "state");
        assert_eq!(keyword(&toks("\"A\" --> B")), "");
        assert_eq!(keyword(&toks("}")), "");
        assert_eq!(keyword(&[]), "");
    }

    #[test]
    fn leftover_tokens_are_rejected_at_the_first() {
        assert_eq!(reject_rest(&[]), Ok(()));
        assert_eq!(reject_rest(&toks("stop now {")[1..]), Err("1:6: error: unexpected `now`".to_string()));
    }

    #[test]
    fn a_header_gives_its_title_or_names_the_problem() {
        assert_eq!(header_title(&toks("diagram state \"Door\""), "state"), Ok(Some("Door".to_string())));
        assert_eq!(header_title(&toks("diagram state"), "state"), Ok(None));
        assert_eq!(header_title(&toks("diagram package"), "state"), Err("1:1: error: the `diagram` header must be the first statement".to_string()));
        assert!(header_title(&toks("diagram timing"), "state").unwrap_err().starts_with("1:1: error: diagram kind `timing` is not implemented"));
        assert_eq!(header_title(&toks("diagram {"), "state"), Err("1:1: error: expected a diagram kind".to_string()));
    }

    #[test]
    fn a_misspelt_keyword_gets_the_nearest() {
        assert_eq!(keyword_hint(&["actor", "participant"], "actro"), "; did you mean `actor`?");
        assert_eq!(keyword_hint(&["actor", "participant"], "zzzzzz"), "");
    }

    #[test]
    fn a_declared_name_needs_a_name_token() {
        let found = toks("state \"Busy\" as b");
        assert_eq!(declared_name(&found).map(|(tok, name)| (tok.col, name)), Ok((7, "Busy".to_string())));
        assert_eq!(declared_name(&toks("state")).map(|(_, name)| name), Err("1:1: error: expected a name after the keyword".to_string()));
        assert_eq!(declared_name(&toks("state {")).map(|(_, name)| name), Err("1:7: error: expected a name, found `{`".to_string()));
    }
}
