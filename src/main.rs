// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

use std::process::ExitCode;

/// One line per (name, description), the first marked as the default.
fn listed(entries: &[(&str, &str)]) -> String {
    let lines: Vec<String> = entries.iter().enumerate().map(|(i, (name, about))| format!("  {name:16} {about}{}", if i == 0 { " (default)" } else { "" })).collect();
    lines.join("\n")
}

fn usage() -> String {
    format!(
        "usage:
  drumlin render <input.uml> [-o <output.svg>] [--layout <engine>] [--style <name>]   render to SVG (stdout by default)
  drumlin check <input.uml> [--layout <engine>] [--style <name>]                      validate only
layout engines, for component, package and state diagrams (--layout wins over a `layout` statement):
{}
styles, for every diagram kind (--style wins over a `style` statement):
{}",
        listed(&drumlin::layout_engines()),
        listed(&drumlin::styles())
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// What follows the command: the input path, `-o <path>` (render only),
/// `--layout <engine>` and `--style <name>`, in any order.
#[derive(Default)]
struct Args {
    input: Option<String>,
    output: Option<String>,
    options: drumlin::Options,
}

fn parse_args(command: &str, rest: &[String], parsed: Args) -> Result<Args, String> {
    match rest {
        [] => Ok(parsed),
        [flag, tail @ ..] if flag == "-o" && command == "render" => match tail {
            [out, tail @ ..] => parse_args(command, tail, Args { output: Some(out.clone()), ..parsed }),
            [] => Err("error: -o needs a path".to_string()),
        },
        [flag, tail @ ..] if flag == "--layout" => match tail {
            [name, tail @ ..] => parse_args(command, tail, Args { options: drumlin::Options { layout: Some(name.clone()), ..parsed.options }, ..parsed }),
            [] => Err("error: --layout needs an engine name".to_string()),
        },
        [flag, tail @ ..] if flag == "--style" => match tail {
            [name, tail @ ..] => parse_args(command, tail, Args { options: drumlin::Options { style: Some(name.clone()), ..parsed.options }, ..parsed }),
            [] => Err("error: --style needs a style name".to_string()),
        },
        [input, tail @ ..] if parsed.input.is_none() && !input.starts_with('-') => parse_args(command, tail, Args { input: Some(input.clone()), ..parsed }),
        [other, ..] => Err(format!("error: unknown argument `{other}`\n{}", usage())),
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let Some((command, rest)) = args.split_first() else {
        return Err(usage());
    };
    if command != "render" && command != "check" {
        return match rest.first() {
            Some(_) => Err(format!("error: unknown command `{command}`\n{}", usage())),
            None => Err(usage()),
        };
    }
    let parsed = parse_args(command, rest, Args::default())?;
    let Some(path) = parsed.input.as_deref() else {
        return Err(usage());
    };
    drumlin::check_options(&parsed.options)?;
    let source = std::fs::read_to_string(path).map_err(|e| format!("{path}: error: {e}"))?;
    if command == "check" {
        return drumlin::check_with(&source, &parsed.options).map_err(|e| format!("{path}:{e}"));
    }
    let svg = drumlin::render_with(&source, &parsed.options).map_err(|e| format!("{path}:{e}"))?;
    match parsed.output {
        Some(out) => std::fs::write(&out, svg).map_err(|e| format!("{out}: error: {e}")),
        None => {
            print!("{svg}");
            Ok(())
        }
    }
}
