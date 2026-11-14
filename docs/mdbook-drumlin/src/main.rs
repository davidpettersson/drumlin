// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! mdBook preprocessor. Replaces `drumlin` code fences with inline SVG.
//!
//! Fence forms:
//!   ```drumlin               source, then the rendered diagram
//!   ```drumlin,render-only   the rendered diagram alone
//!   ```drumlin,error         source, then the real error; the build fails if it parses
//!
//! Any diagram that fails to parse fails the whole book build, so a published
//! example can never be broken.

use serde_json::Value;
use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("supports") {
        return ExitCode::SUCCESS;
    }
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("mdbook-drumlin: cannot read stdin: {e}");
        return ExitCode::FAILURE;
    }
    let mut pair: Value = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("mdbook-drumlin: invalid input: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut book = pair[1].take();
    let mut errors = Vec::new();
    let mut count = 0;
    walk(&mut book, &mut errors, &mut count);
    if !errors.is_empty() {
        for error in errors {
            eprintln!("mdbook-drumlin: {error}");
        }
        return ExitCode::FAILURE;
    }
    eprintln!("mdbook-drumlin: rendered {count} diagrams");
    println!("{book}");
    ExitCode::SUCCESS
}

/// Finds chapters structurally, so the book JSON layout can vary by mdBook version.
fn walk(value: &mut Value, errors: &mut Vec<String>, count: &mut usize) {
    match value {
        Value::Array(items) => items.iter_mut().for_each(|v| walk(v, errors, count)),
        Value::Object(map) => {
            let name = map.get("name").and_then(Value::as_str).unwrap_or("?").to_string();
            if let Some(Value::String(content)) = map.get_mut("content") {
                *content = transform(content, &name, errors, count);
            }
            map.values_mut().for_each(|v| walk(v, errors, count));
        }
        _ => {}
    }
}

fn fence_width(line: &str) -> usize {
    line.trim_start().chars().take_while(|&c| c == '`').count()
}

fn figure(svg: &str) -> String {
    // A blank line would end the CommonMark HTML block early.
    let body: Vec<&str> = svg.lines().filter(|l| !l.trim().is_empty()).collect();
    format!("<figure class=\"drumlin-figure\">\n{}\n</figure>\n\n", body.join("\n"))
}

fn transform(content: &str, chapter: &str, errors: &mut Vec<String>, count: &mut usize) -> String {
    let mut out = String::with_capacity(content.len());
    let mut lines = content.lines();
    while let Some(line) = lines.next() {
        let width = fence_width(line);
        if width < 3 {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let info = line.trim_start()[width..].trim().to_string();
        let mut body = Vec::new();
        let mut closing = String::new();
        for inner in lines.by_ref() {
            if fence_width(inner) >= width && inner.trim_start()[fence_width(inner)..].trim().is_empty() {
                closing = inner.to_string();
                break;
            }
            body.push(inner);
        }
        let source = body.join("\n");
        let mut options = info.split(',').map(str::trim);
        if width != 3 || options.next() != Some("drumlin") {
            out.push_str(line);
            out.push('\n');
            out.push_str(&source);
            out.push('\n');
            out.push_str(&closing);
            out.push('\n');
            continue;
        }
        let options: Vec<&str> = options.collect();
        let shown = format!("```text\n{source}\n```\n\n");
        *count += 1;
        if options.contains(&"error") {
            match drumlin::check(&source) {
                Ok(()) => errors.push(format!("chapter `{chapter}`: an error example parsed successfully:\n{source}")),
                Err(message) => out.push_str(&format!("{shown}<p class=\"drumlin-output\">drumlin says:</p>\n\n```text\n{message}\n```\n\n")),
            }
            continue;
        }
        match drumlin::render(&source) {
            Ok(svg) => {
                if !options.contains(&"render-only") {
                    out.push_str(&shown);
                }
                out.push_str(&figure(&svg));
            }
            Err(message) => errors.push(format!("chapter `{chapter}`: {message}\n{source}")),
        }
    }
    out
}
