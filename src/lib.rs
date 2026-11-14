// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! drumlin: text-to-SVG UML diagrams.
//!
//! Errors from `check` and `render` have the form `line:col: error: message`.
//! `check_with` and `render_with` take `Options` from outside the source, such
//! as the command line's `--layout` and `--style`.

mod core;
mod diagrams;
mod syntax;

use core::style::{Style, STYLES};
use diagrams::engine::{self, Engine};
use diagrams::{activity, component, package, sequence, state};
use syntax::Res;

/// The diagram kinds that have an implementation.
enum Kind {
    Component,
    Activity,
    Package,
    Sequence,
    State,
}

/// Reads the `diagram <kind>` header. An absent header means a component diagram.
fn kind_of(source: &str) -> Res<Kind> {
    match syntax::header(source) {
        None => Ok(Kind::Component),
        Some(header) if header.kind == "component" => Ok(Kind::Component),
        Some(header) if header.kind == "package" => Ok(Kind::Package),
        Some(header) if header.kind == "sequence" => Ok(Kind::Sequence),
        Some(header) if header.kind == "state" => Ok(Kind::State),
        Some(header) if header.kind == "activity" => Ok(Kind::Activity),
        Some(header) => syntax::unsupported_kind(header.line, header.col, &header.kind),
    }
}

/// Choices made outside the diagram source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// A layout engine by name, for component, package and state diagrams. It
    /// wins over a `layout` statement in the source. `None` keeps the source's
    /// engine, or the default.
    pub layout: Option<String>,
    /// A drawing style by name, for every kind. It wins over a `style`
    /// statement in the source. `None` keeps the source's style, or the
    /// default.
    pub style: Option<String>,
}

/// The layout engines, as (name, what it does), the default first.
pub fn layout_engines() -> Vec<(&'static str, &'static str)> {
    engine::ENGINES.iter().map(|engine| (engine.name, engine.about)).collect()
}

/// The drawing styles, as (name, what it looks like), the default first.
pub fn styles() -> Vec<(&'static str, &'static str)> {
    STYLES.iter().map(|style| (style.name(), style.about())).collect()
}

/// Checks the options on their own, before any source is read. An unknown
/// engine or style name gives `error: message`, with no position.
pub fn check_options(options: &Options) -> Result<(), String> {
    chosen_engine(options)?;
    chosen_style(options).map(|_| ())
}

fn chosen_style(options: &Options) -> Res<Option<Style>> {
    match options.style.as_deref() {
        None => Ok(None),
        Some(name) => Style::named(name).map(Some).ok_or_else(|| format!("error: {}", diagrams::style::unknown(name))),
    }
}

fn chosen_engine(options: &Options) -> Res<Option<Engine>> {
    match options.layout.as_deref() {
        None => Ok(None),
        Some(name) => engine::named(name).map(Some).ok_or_else(|| format!("error: {}", engine::unknown(name))),
    }
}

/// The error for an engine chosen for a kind with its own layout, at its
/// header, once the source itself is valid (`checked`).
fn own_layout<X>(checked: Res<()>, source: &str, kind: &str) -> Res<X> {
    checked?;
    let (line, col) = syntax::header(source).map_or((1, 1), |header| (header.line, header.col));
    engine::not_layered(kind, line, col)
}

/// Validates diagram source without laying it out.
pub fn check(source: &str) -> Result<(), String> {
    check_with(source, &Options::default())
}

/// Validates diagram source and options without laying it out.
pub fn check_with(source: &str, options: &Options) -> Result<(), String> {
    let chosen = chosen_engine(options)?;
    chosen_style(options)?;
    match kind_of(source)? {
        Kind::Component => component::check(source),
        Kind::Package => package::check(source),
        Kind::State => state::check(source),
        Kind::Sequence if chosen.is_some() => own_layout(sequence::check(source), source, "sequence"),
        Kind::Activity if chosen.is_some() => own_layout(activity::check(source), source, "activity"),
        Kind::Sequence => sequence::check(source),
        Kind::Activity => activity::check(source),
    }
}

/// Renders diagram source to a standalone SVG document.
pub fn render(source: &str) -> Result<String, String> {
    render_with(source, &Options::default())
}

/// Renders diagram source to a standalone SVG document, with options.
pub fn render_with(source: &str, options: &Options) -> Result<String, String> {
    let chosen = chosen_engine(options)?;
    let style = chosen_style(options)?;
    match kind_of(source)? {
        Kind::Component => component::render(source, chosen, style),
        Kind::Package => package::render(source, chosen, style),
        Kind::State => state::render(source, chosen, style),
        Kind::Sequence if chosen.is_some() => own_layout(sequence::check(source), source, "sequence"),
        Kind::Activity if chosen.is_some() => own_layout(activity::check(source), source, "activity"),
        Kind::Sequence => sequence::render(source, style),
        Kind::Activity => activity::render(source, style),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(layout: &str) -> Options {
        Options { layout: Some(layout.to_string()), ..Options::default() }
    }

    const CHAIN: &str = "diagram state\n[*] --> A\nA --> B\nB --> [*]";

    #[test]
    fn the_default_engine_by_name_draws_the_same() {
        let plain = render(CHAIN).unwrap();
        assert_eq!(render_with(CHAIN, &with("longest-path")).unwrap(), plain);
        assert_eq!(render(&CHAIN.replace("diagram state", "diagram state\nlayout longest-path")).unwrap(), plain);
        assert_eq!(layout_engines()[0].0, "longest-path");
    }

    /// A chosen engine reaches the layered layout through each kind, and wins
    /// over the source's `layout` statement. The test-only flat ranking puts
    /// every element in one rank, so the drawing changes.
    #[test]
    fn a_chosen_engine_wins_over_the_statement() {
        use crate::core::layered::{Ranking, Strategy};
        let flat = Engine { name: "flat", about: "test only", strategy: Strategy { ranking: Ranking::Flat } };
        let stated = |source: &str| source.replacen('\n', "\nlayout longest-path\n", 1);
        let component = stated("diagram component\ncomponent A\ncomponent B\nA --> B");
        assert_ne!(component::render(&component, Some(flat), None), component::render(&component, None, None));
        let package = stated("diagram package\npackage A\npackage B\nA ..> B");
        assert_ne!(package::render(&package, Some(flat), None), package::render(&package, None, None));
        let state = stated(CHAIN);
        assert_ne!(state::render(&state, Some(flat), None), state::render(&state, None, None));
        assert_eq!(state::render(&state, Some(engine::DEFAULT), None), state::render(&state, None, None));
    }

    /// Network simplex draws a chain that longest path already ranks
    /// optimally the same, and moves a node with one parent high up next to
    /// its children far below.
    #[test]
    fn network_simplex_moves_a_node_next_to_its_children() {
        assert_eq!(render_with(CHAIN, &with("network-simplex")).unwrap(), render(CHAIN).unwrap());
        let names = ["Web", "Gateway", "Cart", "Pricing", "Orders", "Billing", "Flags"].map(|name| format!("component {name}\n")).concat();
        let source = &format!("diagram component\ndirection down\n{names}Web --> Gateway\nGateway --> Cart\nCart --> Pricing\nPricing --> Orders\nPricing --> Billing\nWeb ..> Flags\nFlags ..> Orders\nFlags ..> Billing");
        let top = |svg: &str, name: &str| -> f64 {
            let at = svg.find(&format!(">{name}</text>")).unwrap();
            let y = svg[..at].rfind(" y=\"").unwrap() + 4;
            svg[y..].split('"').next().unwrap().parse().unwrap()
        };
        let (longest, simplex) = (render(source).unwrap(), render_with(source, &with("network-simplex")).unwrap());
        assert_eq!(top(&longest, "Flags"), top(&longest, "Gateway"), "longest path ranks Flags under Web");
        assert_eq!(top(&simplex, "Flags"), top(&simplex, "Pricing"), "network simplex ranks Flags over its targets");
        assert_eq!(layout_engines()[1].0, "network-simplex");
    }

    #[test]
    fn an_unknown_engine_option_is_rejected_before_the_source() {
        let expected = Err("error: unknown layout engine `dot`; the engines are `longest-path`, `network-simplex`".to_string());
        assert_eq!(check_options(&with("dot")), expected);
        assert_eq!(render_with(CHAIN, &with("dot")), expected.clone().map(|()| String::new()));
        assert_eq!(check_with("diagram timing", &with("dot")), expected);
        assert_eq!(check_options(&Options::default()), Ok(()));
    }

    fn styled(style: &str) -> Options {
        Options { style: Some(style.to_string()), ..Options::default() }
    }

    /// Every kind takes a style, by statement or by option, and the option
    /// wins. `pens` is the default; `plain` draws differently.
    #[test]
    fn a_chosen_style_wins_over_the_statement_in_every_kind() {
        let sources = [
            "diagram component\ncomponent A\ncomponent B\nA --> B",
            "diagram package\npackage A\npackage B\nA ..> B",
            CHAIN,
            "diagram sequence\nparticipant A\nparticipant B\nA -> B : go",
            "diagram activity\nstart\naction \"Go\"\nstop",
        ];
        for source in sources {
            let stated = |style: &str| source.replacen('\n', &format!("\nstyle {style}\n"), 1);
            let (pens, plain) = (render(source).unwrap(), render_with(source, &styled("plain")).unwrap());
            assert_ne!(pens, plain, "{source}");
            assert!(pens.contains("#22201c") && !pens.contains("#52606d"), "{source}");
            assert_eq!(render_with(source, &styled("pens")).unwrap(), pens);
            assert_eq!(render(&stated("plain")).unwrap(), plain);
            assert_eq!(render_with(&stated("plain"), &styled("pens")).unwrap(), pens);
            assert_eq!(render_with(&stated("pens"), &styled("plain")).unwrap(), plain);
        }
        assert_eq!(styles()[0].0, "pens");
    }

    /// `lettering` draws every piece of text as pen strokes, in every kind,
    /// by statement or by option, and lays the drawing out for its own widths.
    #[test]
    fn lettering_strokes_the_text_of_every_kind() {
        let sources = [
            "diagram component \"T\"\ncomponent Alpha\ncomponent B\nAlpha --> B : go",
            "diagram package\npackage Alpha\npackage B\nAlpha ..> B",
            "diagram state\n[*] --> Alpha\nAlpha --> [*]",
            "diagram sequence\nparticipant Alpha\nparticipant B\nAlpha -> B : go",
            "diagram activity\nstart\naction \"Alpha\"\nstop",
        ];
        for source in sources {
            let lettered = render_with(source, &styled("lettering")).unwrap();
            assert!(!lettered.contains("<text"), "{lettered}");
            assert!(lettered.contains("class=\"drumlin lettering\"") && lettered.contains("<defs>") && lettered.contains("aria-label=\"Alpha\""), "{lettered}");
            assert!(lettered.contains("<use href=\"#dl-n41\"/>"), "names in capitals: {lettered}");
            let stated = source.replacen('\n', "\nstyle lettering\n", 1);
            assert_eq!(render(&stated).unwrap(), lettered);
            assert_eq!(render_with(&stated, &styled("pens")).unwrap(), render(source).unwrap(), "the option wins, and the face does not leak");
        }
        assert_eq!(styles()[2].0, "lettering");
    }

    #[test]
    fn an_unknown_style_option_is_rejected_before_the_source() {
        let expected = Err("error: unknown style `sketch`; the styles are `pens`, `plain`, `lettering`".to_string());
        assert_eq!(check_options(&styled("sketch")), expected);
        assert_eq!(check_with(CHAIN, &styled("sketch")), expected);
        assert_eq!(render_with("diagram sequence\nparticipant A", &styled("plan")), Err("error: unknown style `plan`; did you mean `plain`?".to_string()));
        assert_eq!(check_with(CHAIN, &styled("plain")), Ok(()));
    }

    #[test]
    fn sequence_and_activity_take_no_engine() {
        let sequence = "diagram sequence\nparticipant A";
        let message = "1:1: error: sequence diagrams have their own layout; layout engines apply to component, package and state diagrams";
        assert_eq!(check_with(sequence, &with("longest-path")), Err(message.to_string()));
        assert_eq!(render_with(sequence, &with("longest-path")), Err(message.to_string()));
        assert!(render_with(sequence, &Options::default()).is_ok());
        let activity = "\n  diagram activity\nstart\nstop";
        assert_eq!(check_with(activity, &with("longest-path")), Err("2:3: error: activity diagrams have their own layout; layout engines apply to component, package and state diagrams".to_string()));
        // A source error comes first: it is what the file needs fixed.
        assert_eq!(check_with("diagram activity", &with("longest-path")), check("diagram activity"));
    }
}
