// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Choosing a drawing style (`core::style`). A diagram of any kind picks one
//! with a `style <name>` statement. A caller may choose one instead
//! (`drumlin render --style <name>`), which wins over the statement.

use super::engine::is_setting;
use crate::core::style::{Style, STYLES};
use crate::syntax::{did_you_mean, err, name_of, reject_rest, Res, Tok};

/// The style names, quoted and comma-separated.
fn name_list() -> String {
    STYLES.iter().map(|style| format!("`{}`", style.name())).collect::<Vec<String>>().join(", ")
}

/// The message for a name that is no style: a near name, or else every name.
pub fn unknown(name: &str) -> String {
    let names: Vec<String> = STYLES.iter().map(|style| style.name().to_string()).collect();
    match did_you_mean(names.iter(), name) {
        hint if hint.is_empty() => format!("unknown style `{name}`; the styles are {}", name_list()),
        hint => format!("unknown style `{name}`{hint}"),
    }
}

/// True when a statement is `style <name>`, rather than a relation, hint or
/// message whose first element happens to be called `style`.
pub(super) fn is_statement(toks: &[Tok]) -> bool {
    is_setting(toks, "style")
}

/// The style a `style <name>` statement names.
pub(super) fn statement(toks: &[Tok]) -> Res<Style> {
    let head = &toks[0];
    let Some(name_tok) = toks.get(1) else {
        return err(head.line, head.col, format!("expected `style <name>`; the styles are {}", name_list()));
    };
    let name = name_of(name_tok).unwrap_or_default();
    let style = Style::named(&name).ok_or_else(|| unknown(&name));
    let style = style.or_else(|message| err(name_tok.line, name_tok.col, message))?;
    reject_rest(&toks[2..])?;
    Ok(style)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{tokenize, Lexicon};

    fn toks(text: &str) -> Vec<Tok> {
        tokenize(text, 1, &Lexicon { arrows: &["-->"], symbols: "{}," }).unwrap()
    }

    #[test]
    fn an_unknown_name_suggests_a_near_one_or_lists_them_all() {
        assert_eq!(unknown("pen"), "unknown style `pen`; did you mean `pens`?");
        assert_eq!(unknown("sketch"), "unknown style `sketch`; the styles are `pens`, `plain`, `lettering`");
    }

    #[test]
    fn the_statement_names_a_style() {
        assert_eq!(statement(&toks("style plain")), Ok(Style::Plain));
        assert_eq!(statement(&toks("style \"pens\"")), Ok(Style::Pens));
        assert_eq!(statement(&toks("style sketch")), Err("1:7: error: unknown style `sketch`; the styles are `pens`, `plain`, `lettering`".to_string()));
        assert_eq!(statement(&toks("style")), Err("1:1: error: expected `style <name>`; the styles are `pens`, `plain`, `lettering`".to_string()));
        assert_eq!(statement(&toks("style plain now")), Err("1:13: error: unexpected `now`".to_string()));
    }

    #[test]
    fn an_element_called_style_is_not_the_statement() {
        assert!(is_statement(&toks("style plain")));
        assert!(!is_statement(&toks("style --> parser")));
        assert!(!is_statement(&toks("style left-of parser")));
        assert!(!is_statement(&toks("layout plain")));
    }
}
