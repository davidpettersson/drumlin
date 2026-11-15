// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Golden-file test. Every `tests/golden/<kind>/<name>.uml` must render to exactly
//! the bytes in its sibling `<name>.svg`. Set `DRUMLIN_BLESS=1` to rewrite the files.
//!
//! The `plain` and `lettering` styles have their own expected drawings: every
//! `tests/style/<style>/<name>.uml` renders with `--style <style>` to the
//! bytes of its sibling `<name>.svg`. `DRUMLIN_BLESS=1` rewrites these too.

use std::fs;
use std::path::{Path, PathBuf};

/// Whether this run rewrites the expected drawings (`DRUMLIN_BLESS=1`).
fn bless() -> bool {
    std::env::var("DRUMLIN_BLESS").is_ok_and(|v| v == "1")
}

fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    entries
}

fn cases() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    sorted_entries(&root)
        .into_iter()
        .filter(|path| path.is_dir())
        .flat_map(|kind| sorted_entries(&kind))
        .filter(|path| path.extension().is_some_and(|e| e == "uml"))
        .collect()
}

/// Returns a description of the mismatch, or `None` when the case passes.
fn check_case(input: &Path, bless: bool) -> Option<String> {
    let source = fs::read_to_string(input).unwrap();
    let expected_path = input.with_extension("svg");
    let actual = match drumlin::render(&source) {
        Ok(svg) => svg,
        Err(message) => return Some(format!("{}: {message}", input.display())),
    };
    if bless {
        fs::write(&expected_path, actual).unwrap();
        return None;
    }
    match fs::read_to_string(&expected_path) {
        Ok(expected) if expected == actual => None,
        Ok(_) => Some(format!("{}: output differs from golden file", input.display())),
        Err(e) => Some(format!("{}: {e}", expected_path.display())),
    }
}

#[test]
fn golden_files_match() {
    let bless = bless();
    let cases = cases();
    assert!(!cases.is_empty(), "no golden cases found");
    let failures: Vec<String> = cases.iter().filter_map(|case| check_case(case, bless)).collect();
    assert!(failures.is_empty(), "{} of {} golden cases failed:\n{}", failures.len(), cases.len(), failures.join("\n"));
}

#[test]
fn the_plain_style_draws_as_expected() {
    let bless = bless();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/style/plain");
    let cases: Vec<PathBuf> = sorted_entries(&root).into_iter().filter(|path| path.extension().is_some_and(|e| e == "uml")).collect();
    assert!(cases.len() >= 5, "plain fixtures for every kind");
    let options = drumlin::Options { style: Some("plain".to_string()), ..drumlin::Options::default() };
    let failures: Vec<String> = cases
        .iter()
        .filter(|input| {
            let actual = drumlin::render_with(&fs::read_to_string(input).unwrap(), &options).unwrap();
            if bless {
                fs::write(input.with_extension("svg"), &actual).unwrap();
            }
            fs::read_to_string(input.with_extension("svg")).map_or(true, |expected| expected != actual)
        })
        .map(|input| input.display().to_string())
        .collect();
    assert!(failures.is_empty(), "plain style differs from the fixtures:\n{}", failures.join("\n"));
}

#[test]
fn the_lettering_style_draws_as_expected() {
    let bless = bless();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/style/lettering");
    let cases: Vec<PathBuf> = sorted_entries(&root).into_iter().filter(|path| path.extension().is_some_and(|e| e == "uml")).collect();
    assert!(cases.len() >= 6, "lettering fixtures for every kind and the text case");
    let options = drumlin::Options { style: Some("lettering".to_string()), ..drumlin::Options::default() };
    let failures: Vec<String> = cases
        .iter()
        .filter(|input| {
            let actual = drumlin::render_with(&fs::read_to_string(input).unwrap(), &options).unwrap();
            if bless {
                fs::write(input.with_extension("svg"), &actual).unwrap();
            }
            fs::read_to_string(input.with_extension("svg")).map_or(true, |expected| expected != actual)
        })
        .map(|input| input.display().to_string())
        .collect();
    assert!(failures.is_empty(), "lettering style differs from the fixtures:\n{}", failures.join("\n"));
}
