// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Layout quality across the golden diagrams: crossings, overlaps and bends in
//! the routed lines of every component, package and state golden file, the
//! whole-drawing measure (`core::metrics::assess`) for every sequence and
//! activity golden file, and the size of every golden drawing (line length,
//! area, width). Run with `cargo test --lib quality -- --nocapture` to
//! see the tables.
//!
//! The budgets hold for the drawings as the golden sources ask for them, with
//! the default layout engine unless a source names another. To compare layout
//! engines, set `DRUMLIN_ENGINES` to `all` or to engine names separated by
//! commas and run `compare_engines`: it prints every component, package and
//! state table for each engine, and a summary of the totals side by side.
//!
//! The `lettering` style lays drawings out for its own text widths. Set
//! `DRUMLIN_LETTERING` and run `compare_lettering` to see every total for it
//! beside the default's; `lettering_text_fits_its_boxes` checks that no text
//! run in a lettering drawing sticks out of its box or note.

use crate::core::ends::{count, Counts};
use crate::core::flow::{flow, passes, Flow};
use crate::core::geometry::{Point, Rect};
use crate::core::metrics::{assess, measure, merged_ends, read_labels, Metrics, Quality, Reading, Size};
use super::engine::{self, Engine};
use std::fs;
use std::path::{Path, PathBuf};

/// Totals measured when this budget was last lowered. A change that raises a
/// total fails here; lower the budget when a change improves it.
const CROSSINGS_BUDGET: usize = 2;
const OVERLAPS_BUDGET: usize = 0;
/// Raised from 193 to 280 when ends alone on their side began to meet its
/// middle, and where boxes cannot line up the line jogs once.
/// Lowered from 280: chains keep a column and notes come last.
/// Lowered from 263: families line up with the member that leads them, boxes
/// move with the lines they are lined up with, entry points keep their column.
/// Lowered from 232: lines meet the side they come from, spread over free
/// sides, and loops return round the outside.
/// Lowered from 199: the spine runs on one line through ports, and a straight
/// line keeps its side when another line enters beside it.
const BENDS_BUDGET: usize = 191;
/// Pairs of connectors leaving one point together in component and state
/// diagrams. Both are intended fan-ins to one port. Package containment trees
/// share their trunk by design and are not counted.
const MERGED_BUDGET: usize = 2;

type Lines = fn(&str, Option<Engine>) -> crate::syntax::Res<Vec<Vec<Point>>>;

/// Totals against their budgets: `(measure, total, budget)`.
type Totals = Vec<(String, usize, usize)>;

/// One message per total above its budget.
fn over_budget(totals: &Totals) -> Vec<String> {
    totals.iter().filter(|(_, value, limit)| value > limit).map(|(name, value, limit)| format!("{name} rose to {value} (budget {limit})")).collect()
}

fn assert_within(totals: Totals) {
    let failures = over_budget(&totals);
    assert!(failures.is_empty(), "{}", failures.join("; "));
}

/// The engine named by `DRUMLIN_ENGINE`, for the `explain_*` and `dump_lines`
/// tests; unset, each source's own engine.
fn explained_engine() -> Option<Engine> {
    let name = std::env::var("DRUMLIN_ENGINE").ok()?;
    Some(engine::named(&name).unwrap_or_else(|| panic!("{}", engine::unknown(&name))))
}

fn sources(kind: &str) -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(kind);
    let mut found: Vec<PathBuf> = fs::read_dir(&dir).unwrap().map(|entry| entry.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "uml")).collect();
    found.sort();
    found
}

/// One row per golden file: `(kind/name, metrics, merged ends)`.
fn rows(chosen: Option<Engine>) -> Vec<(String, Metrics, usize)> {
    let kinds: [(&str, Lines); 3] = [("component", super::component::lines), ("package", super::package::lines), ("state", super::state::lines)];
    kinds
        .iter()
        .flat_map(|&(kind, lines)| {
            sources(kind).into_iter().map(move |path| {
                let source = fs::read_to_string(&path).unwrap();
                let paths = lines(&source, chosen).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                let merged = if kind == "package" { 0 } else { merged_ends(&paths) };
                (format!("{kind}/{}", path.file_stem().unwrap().to_string_lossy()), measure(&paths), merged)
            })
        })
        .collect()
}

/// Prints the crossings and bends table and returns its totals.
fn lines_table(chosen: Option<Engine>) -> Totals {
    let rows = rows(chosen);
    for (name, m, merged) in &rows {
        println!("{name:40} crossings {:3}  overlaps {:3}  bends {:4}  merged {merged}", m.crossings, m.overlaps, m.bends);
    }
    let total: Metrics = rows.iter().map(|(_, m, _)| *m).sum();
    let merged: usize = rows.iter().map(|(_, _, merged)| merged).sum();
    println!("TOTAL crossings {} overlaps {} bends {} merged {merged} over {} diagrams", total.crossings, total.overlaps, total.bends, rows.len());
    let fields = [("crossings", total.crossings, CROSSINGS_BUDGET), ("overlaps", total.overlaps, OVERLAPS_BUDGET), ("bends", total.bends, BENDS_BUDGET), ("merged ends", merged, MERGED_BUDGET)];
    fields.into_iter().map(|(name, value, limit)| (name.to_string(), value, limit)).collect()
}

#[test]
fn crossings_and_bends_stay_within_budget() {
    assert_within(lines_table(None));
}

type Scenes = fn(&str) -> crate::syntax::Res<crate::core::metrics::Scene>;

/// Sequence and activity totals measured when these budgets were last lowered.
/// Sequence: the 11 shape overlaps are frame tabs and notes drawn
/// over execution bars. Activity: one crossing in `edge-stacked-lanes`, and
/// uneven drops after a branch whose siblings end, under a risen note, and
/// between a while loop's merge and decision.
const SEQUENCE_BUDGET: Quality = Quality { crossings: 0, overlaps: 0, bends: 0, through: 0, shapes: 11, label_lines: 0, label_shapes: 0, label_labels: 0, overflow: 0, uneven: 0 };
const ACTIVITY_BUDGET: Quality = Quality { crossings: 1, overlaps: 0, bends: 93, through: 0, shapes: 0, label_lines: 0, label_shapes: 0, label_labels: 0, overflow: 0, uneven: 9 };

fn scene_rows(kind: &str, scene: Scenes) -> Vec<(String, Quality)> {
    sources(kind)
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path).unwrap();
            let s = scene(&source).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            (format!("{kind}/{}", path.file_stem().unwrap().to_string_lossy()), assess(&s))
        })
        .collect()
}

fn within(kind: &str, total: Quality, budget: Quality) {
    let fields = [
        ("crossings", total.crossings, budget.crossings),
        ("overlaps", total.overlaps, budget.overlaps),
        ("bends", total.bends, budget.bends),
        ("lines through shapes", total.through, budget.through),
        ("shape overlaps", total.shapes, budget.shapes),
        ("labels on lines", total.label_lines, budget.label_lines),
        ("labels on shapes", total.label_shapes, budget.label_shapes),
        ("labels on labels", total.label_labels, budget.label_labels),
        ("text overflow", total.overflow, budget.overflow),
        ("uneven gaps", total.uneven, budget.uneven),
    ];
    for (name, value, limit) in fields {
        assert!(value <= limit, "{kind}: {name} rose to {value} (budget {limit})");
    }
}

#[test]
fn sequence_and_activity_stay_within_budget() {
    let kinds: [(&str, Scenes, Quality); 2] = [("sequence", super::sequence::scene, SEQUENCE_BUDGET), ("activity", super::activity::scene, ACTIVITY_BUDGET)];
    for (kind, scene, budget) in kinds {
        let rows = scene_rows(kind, scene);
        println!("{:36} cross over bends thru shape lab-line lab-shape lab-lab overflow uneven", "");
        for (name, q) in &rows {
            println!(
                "{name:36} {:5} {:4} {:5} {:4} {:5} {:8} {:9} {:7} {:8} {:6}",
                q.crossings, q.overlaps, q.bends, q.through, q.shapes, q.label_lines, q.label_shapes, q.label_labels, q.overflow, q.uneven
            );
        }
        let total: Quality = rows.iter().map(|(_, q)| *q).sum();
        println!("TOTAL {kind} {total:?} over {} diagrams", rows.len());
        within(kind, total, budget);
    }
}

/// Connector label totals per kind (component, package, state) measured when
/// these budgets were last lowered. `count` is not a budget: it shows how many
/// labels were read. Distance is the sum over labels, in pixels. Component
/// distance raised from 386: four labels sit 3px further from lines that
/// alignment straightened. Raised from 398: the `SSA` label in Compiler
/// pipeline (twice) sits 2px further from its line, which runs straight into
/// Code Generator's middle. Lowered since from 402.
const LABEL_BUDGETS: [(&str, Reading); 3] = [
    ("component", Reading { count: 69, on_lines: 0, on_shapes: 0, on_labels: 0, ambiguous: 0, far: 0, distance: 399 }),
    ("package", Reading { count: 27, on_lines: 1, on_shapes: 0, on_labels: 0, ambiguous: 0, far: 0, distance: 186 }),
    ("state", Reading { count: 118, on_lines: 0, on_shapes: 0, on_labels: 0, ambiguous: 1, far: 4, distance: 1008 }),
];

type LabelScenes = fn(&str, Option<Engine>) -> crate::syntax::Res<crate::core::metrics::Scene>;

fn label_scenes(kind: &str) -> LabelScenes {
    match kind {
        "component" => super::component::label_scene,
        "package" => super::package::label_scene,
        _ => super::state::label_scene,
    }
}

/// Prints the connector label tables and returns their totals.
fn labels_table(chosen: Option<Engine>) -> Totals {
    let mut totals = Totals::new();
    for (kind, budget) in LABEL_BUDGETS {
        let rows: Vec<(String, Reading)> = sources(kind)
            .into_iter()
            .map(|path| {
                let source = fs::read_to_string(&path).unwrap();
                let s = label_scenes(kind)(&source, chosen).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                (format!("{kind}/{}", path.file_stem().unwrap().to_string_lossy()), read_labels(&s))
            })
            .collect();
        println!("{:40} count on-line on-shape on-label ambiguous far distance", "");
        for (name, q) in rows.iter().filter(|(_, q)| q.count > 0) {
            println!("{name:40} {:5} {:7} {:8} {:8} {:9} {:3} {:8}", q.count, q.on_lines, q.on_shapes, q.on_labels, q.ambiguous, q.far, q.distance);
        }
        let total: Reading = rows.iter().map(|(_, q)| *q).sum();
        println!("TOTAL labels {kind} {total:?}");
        let fields = [
            ("labels on lines", total.on_lines, budget.on_lines),
            ("labels on shapes", total.on_shapes, budget.on_shapes),
            ("labels on labels", total.on_labels, budget.on_labels),
            ("ambiguous labels", total.ambiguous, budget.ambiguous),
            ("far labels", total.far, budget.far),
            ("label distance", total.distance, budget.distance),
        ];
        totals.extend(fields.into_iter().map(|(name, value, limit)| (format!("{kind}: {name}"), value, limit)));
    }
    totals
}

#[test]
fn connector_labels_stay_within_budget() {
    assert_within(labels_table(None));
}

type Figures = fn(&str, Option<Engine>) -> crate::syntax::Res<crate::core::ends::Figure>;

fn figures(kind: &str) -> Figures {
    match kind {
        "component" => super::component::figure,
        "package" => super::package::figure,
        _ => super::state::figure,
    }
}

/// End measure totals per kind (`core::ends`) measured when these budgets were
/// last lowered: ends alone on their side but off its middle, merging lines
/// that part a few pixels apart, parents not centred over their children, and
/// heads or start decorations on too short a run. State off centre raised from
/// 27: in both entry and exit point drawings the return line to Idle enters
/// Idle's side, which leaves the straight `retry` line alone on Idle's bottom,
/// where it was one of two, 16 and 32px off the middle. Lowered since from 29:
/// Idle moves under that line instead.
const END_BUDGETS: [(&str, Counts); 3] = [
    ("component", Counts { off_centre: 15, near_misses: 0, uncentred: 3, cramped: 0 }),
    ("package", Counts { off_centre: 2, near_misses: 0, uncentred: 3, cramped: 0 }),
    ("state", Counts { off_centre: 26, near_misses: 0, uncentred: 2, cramped: 1 }),
];

/// Prints the end measure tables and returns their totals.
fn ends_table(chosen: Option<Engine>) -> Totals {
    let mut totals = Totals::new();
    for (kind, budget) in END_BUDGETS {
        let rows: Vec<(String, Counts)> = sources(kind)
            .into_iter()
            .map(|path| {
                let source = fs::read_to_string(&path).unwrap();
                let f = figures(kind)(&source, chosen).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                (format!("{kind}/{}", path.file_stem().unwrap().to_string_lossy()), count(&f))
            })
            .collect();
        println!("{:40} off-centre near-miss uncentred cramped", "");
        for (name, c) in rows.iter().filter(|(_, c)| *c != Counts::default()) {
            println!("{name:40} {:10} {:9} {:9} {:7}", c.off_centre, c.near_misses, c.uncentred, c.cramped);
        }
        let total: Counts = rows.iter().map(|(_, c)| *c).sum();
        println!("TOTAL ends {kind} {total:?}");
        let fields = [
            ("ends off centre", total.off_centre, budget.off_centre),
            ("near-miss junctions", total.near_misses, budget.near_misses),
            ("uncentred parents", total.uncentred, budget.uncentred),
            ("cramped heads", total.cramped, budget.cramped),
        ];
        totals.extend(fields.into_iter().map(|(name, value, limit)| (format!("{kind}: {name}"), value, limit)));
    }
    totals
}

#[test]
fn ends_stay_within_budget() {
    assert_within(ends_table(None));
}

/// Prints the end flaws counted in the golden file named by `DRUMLIN_ENDS`,
/// such as `component/gallery-embedded-05`.
#[test]
fn explain_ends() {
    let Ok(name) = std::env::var("DRUMLIN_ENDS") else { return };
    let (kind, _) = name.split_once('/').unwrap();
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.uml"))).unwrap();
    for line in crate::core::ends::explain(&figures(kind)(&source, explained_engine()).unwrap()) {
        println!("{line}");
    }
}

/// Prints the label flaws counted in the component, package or state golden
/// file named by `DRUMLIN_LABELS`, such as `state/gallery-tcp`.
#[test]
fn explain_labels() {
    let Ok(name) = std::env::var("DRUMLIN_LABELS") else { return };
    let (kind, _) = name.split_once('/').unwrap();
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.uml"))).unwrap();
    for line in crate::core::metrics::explain_labels(&label_scenes(kind)(&source, explained_engine()).unwrap()) {
        println!("{line}");
    }
}

/// Prints what the whole-drawing measure counted in the sequence or activity
/// golden file named by `DRUMLIN_SCENE`, such as `activity/language-if`.
#[test]
fn explain_scene() {
    let Ok(name) = std::env::var("DRUMLIN_SCENE") else { return };
    let scene: Scenes = if name.starts_with("sequence/") { super::sequence::scene } else { super::activity::scene };
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.uml"))).unwrap();
    for line in crate::core::metrics::explain(&scene(&source).unwrap()) {
        println!("{line}");
    }
}

/// Prints the routed lines of the golden file named by `DRUMLIN_DUMP`, such as
/// `state/gallery-process`.
#[test]
fn dump_lines() {
    let Ok(name) = std::env::var("DRUMLIN_DUMP") else { return };
    let (kind, _) = name.split_once('/').unwrap();
    let lines: Lines = match kind {
        "component" => super::component::lines,
        "package" => super::package::lines,
        _ => super::state::lines,
    };
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.uml"))).unwrap();
    for (i, path) in lines(&source, explained_engine()).unwrap().iter().enumerate() {
        println!("{i}: {path:?}");
    }
}

/// Flow measure totals per kind (`core::flow`) measured when these budgets were
/// last lowered: boxes whose chain leaves their column (and the summed offset),
/// lines closer than the clearance beside a shape, decisions the flow turns in,
/// and note distortion: boxes a note moves and bends it adds (`note_distortion`).
/// Component note distortion raised from 46: with leaders lined up, Order
/// Platform (twice) and Event-driven checkout move one box more without their
/// notes.
const FLOW_BUDGETS: [(&str, Flow, usize); 3] = [
    ("component", Flow { off_column: 34, drift: 1776, crowded: 2, turned: 0 }, 47),
    ("package", Flow { off_column: 15, drift: 1488, crowded: 0, turned: 0 }, 0),
    ("state", Flow { off_column: 14, drift: 1376, crowded: 1, turned: 0 }, 0),
];
/// Activity: decisions the flow turns in (none, since every decision takes a
/// main arm on its axis), and note distortion.
const ACTIVITY_FLOW_BUDGET: (usize, usize) = (0, 0);

/// The source with every note statement left out.
fn without_notes(source: &str) -> String {
    source.lines().filter(|l| !(l.trim_start().starts_with("note ") || l.trim_start().starts_with("note:"))).collect::<Vec<&str>>().join("\n")
}

/// Boxes moved by more than half a grid step against the rest: each box's
/// displacement between two drawings, less the median displacement, which a
/// note beside the whole drawing causes without distorting it. Empty boxes are
/// left out. `vertical` false compares only across the page.
fn moved(with: &[Rect], without: &[Rect], vertical: bool) -> usize {
    let pairs: Vec<(f64, f64)> = with.iter().zip(without).filter(|(_, b)| b.w > 0.0).map(|(a, b)| (a.center().0 - b.center().0, a.center().1 - b.center().1)).collect();
    if pairs.is_empty() {
        return 0;
    }
    let median = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    };
    let (mx, my) = (median(pairs.iter().map(|p| p.0).collect()), median(pairs.iter().map(|p| p.1).collect()));
    pairs.iter().filter(|p| (p.0 - mx).abs() > 4.0 || (vertical && (p.1 - my).abs() > 4.0)).count()
}

fn bends_of(lines: &[Vec<Point>]) -> usize {
    lines.iter().map(|l| crate::core::metrics::bends(l)).sum()
}

/// How much the notes of a diagram distort it: boxes they move against the
/// rest and bends they add to connectors, against the same source without
/// notes. Notes are added after every other box, so the boxes line up.
fn note_distortion(kind: &str, source: &str, chosen: Option<Engine>) -> usize {
    let bare = without_notes(source);
    if bare.lines().count() == source.lines().count() {
        return 0;
    }
    let (Ok(with), Ok(without)) = (figures(kind)(source, chosen), figures(kind)(&bare, chosen)) else { return 0 };
    let n = without.boxes.len();
    let rects = |f: &crate::core::ends::Figure, n: usize| f.boxes[..n].iter().map(|b| b.0).collect::<Vec<Rect>>();
    moved(&rects(&with, n), &rects(&without, n), true) + bends_of(&with.lines).saturating_sub(bends_of(&without.lines))
}

/// Prints the flow measure tables and returns their totals.
fn flow_table(chosen: Option<Engine>) -> Totals {
    let mut totals = Totals::new();
    for (kind, budget, distortion_budget) in FLOW_BUDGETS {
        let rows: Vec<(String, Flow, usize)> = sources(kind)
            .into_iter()
            .map(|path| {
                let source = fs::read_to_string(&path).unwrap();
                let f = figures(kind)(&source, chosen).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                (format!("{kind}/{}", path.file_stem().unwrap().to_string_lossy()), flow(&f), note_distortion(kind, &source, chosen))
            })
            .collect();
        println!("{:40} off-column drift crowded turned notes", "");
        for (name, c, n) in rows.iter().filter(|(_, c, n)| *c != Flow::default() || *n > 0) {
            println!("{name:40} {:10} {:5} {:7} {:6} {:5}", c.off_column, c.drift, c.crowded, c.turned, n);
        }
        let total: Flow = rows.iter().map(|(_, c, _)| *c).sum();
        let distortion: usize = rows.iter().map(|(_, _, n)| n).sum();
        println!("TOTAL flow {kind} {total:?} note distortion {distortion}");
        let fields = [
            ("boxes off column", total.off_column, budget.off_column),
            ("column drift", total.drift, budget.drift),
            ("crowded lines", total.crowded, budget.crowded),
            ("turned decisions", total.turned, budget.turned),
            ("note distortion", distortion, distortion_budget),
        ];
        totals.extend(fields.into_iter().map(|(name, value, limit)| (format!("{kind}: {name}"), value, limit)));
    }
    totals
}

#[test]
fn flow_stays_within_budget() {
    assert_within(flow_table(None));
}

/// Activity decisions the flow turns in, and note distortion across the page.
fn activity_flow(source: &str) -> (usize, usize) {
    let (nodes, edges) = super::activity::flow_parts(source).unwrap();
    let on = |r: &Rect, p: Point| p.0 >= r.x - 0.5 && p.0 <= r.right() + 0.5 && p.1 >= r.y - 0.5 && p.1 <= r.bottom() + 0.5;
    let turned = nodes
        .iter()
        .filter(|(_, decision)| *decision)
        .filter(|(r, _)| {
            let arriving: Vec<Point> = edges.iter().filter(|e| e.len() >= 2 && on(r, e[e.len() - 1])).map(|e| e[e.len() - 1]).collect();
            let leaving: Vec<Point> = edges.iter().filter(|e| e.len() >= 2 && on(r, e[0])).map(|e| e[0]).collect();
            passes(r, &arriving, &leaving) == Some(false)
        })
        .count();
    let bare = without_notes(source);
    let distortion = match super::activity::flow_parts(&bare) {
        Ok((plain, plain_edges)) if bare.lines().count() != source.lines().count() && plain.len() == nodes.len() => {
            let rects = |v: &[(Rect, bool)]| v.iter().map(|n| n.0).collect::<Vec<Rect>>();
            moved(&rects(&nodes), &rects(&plain), false) + bends_of(&edges).saturating_sub(bends_of(&plain_edges))
        }
        _ => 0,
    };
    (turned, distortion)
}

#[test]
fn activity_flow_stays_within_budget() {
    let rows: Vec<(String, (usize, usize))> = sources("activity")
        .into_iter()
        .map(|path| (format!("activity/{}", path.file_stem().unwrap().to_string_lossy()), activity_flow(&fs::read_to_string(&path).unwrap())))
        .collect();
    println!("{:40} turned notes", "");
    for (name, (t, n)) in rows.iter().filter(|(_, (t, n))| t + n > 0) {
        println!("{name:40} {t:6} {n:5}");
    }
    let (turned, distortion) = rows.iter().fold((0, 0), |(a, b), (_, (t, n))| (a + t, b + n));
    println!("TOTAL flow activity turned {turned} note distortion {distortion}");
    assert!(turned <= ACTIVITY_FLOW_BUDGET.0, "activity: turned decisions rose to {turned} (budget {})", ACTIVITY_FLOW_BUDGET.0);
    assert!(distortion <= ACTIVITY_FLOW_BUDGET.1, "activity: note distortion rose to {distortion} (budget {})", ACTIVITY_FLOW_BUDGET.1);
}

/// Prints the flow flaws counted in the golden file named by `DRUMLIN_FLOW`,
/// such as `state/gallery-process`.
#[test]
fn explain_flow() {
    let Ok(name) = std::env::var("DRUMLIN_FLOW") else { return };
    let (kind, _) = name.split_once('/').unwrap();
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.uml"))).unwrap();
    for line in crate::core::flow::explain(&figures(kind)(&source, explained_engine()).unwrap()) {
        println!("{line}");
    }
}

/// Size totals per kind (`core::metrics::size`) measured when these budgets
/// were last set (component length and area raised when interface ends got
/// their clearance and gallery-data-02 grew by 8px; package length raised when
/// lines into Domain in gallery-layered and the containment tree in
/// language-direction-right lined up and the side lines grew longer; the other
/// totals lowered then):
/// `(kind, line length, area, too wide)`. Line
/// length is the summed length of every line in px, area the summed area of
/// every drawing's frame in px², and too wide the count of drawings wider than
/// the book's column (`COLUMN`). Component line length raised from 37890: an L that leaves a free side runs round the box it used to cross
/// beside (Hexagonal service, API gateway, Build system, Debugger, Container
/// runtime, Replicated state machine, Order Platform twice); every one of those
/// drawings loses bends. Component area raised from 13490500: App
/// and Runtime in the third interfaces example grow from 64 to 96px so that
/// their three named lines keep 32px apart and 16px off the corners (+9216).
/// State line length raised from 27442: Idle in both entry and
/// exit point drawings, and Done in Choice, fork, join, move under the straight
/// spine, so the return lines round the outside run 16 to 48px further.
/// Component line length lowered then from 38434.
const SIZE_BUDGETS: [(&str, usize, usize, usize); 5] = [
    ("component", 38178, 13499716, 21),
    ("package", 6840, 3251456, 3),
    ("state", 27546, 7234575, 2),
    ("sequence", 27636, 6941820, 5),
    ("activity", 27985, 6417856, 1),
];

/// The width the book gives a drawing before scaling it down: mdBook's 750px
/// content column less the figure's 8px padding and 1px border on each side.
const COLUMN: usize = 732;

type Sizes = fn(&str, Option<Engine>) -> crate::syntax::Res<Size>;

fn sizes(kind: &str) -> Sizes {
    match kind {
        "component" => super::component::size,
        "package" => super::package::size,
        "state" => super::state::size,
        "sequence" => |source, _| super::sequence::size(source),
        _ => |source, _| super::activity::size(source),
    }
}

/// One row per golden file of `kind`: its name and size.
fn size_rows(kind: &str, chosen: Option<Engine>) -> Vec<(String, Size)> {
    sources(kind)
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path).unwrap();
            let s = sizes(kind)(&source, chosen).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            (format!("{kind}/{}", path.file_stem().unwrap().to_string_lossy()), s)
        })
        .collect()
}

/// Prints the size tables of `kinds` (the drawings wider than the column, and
/// per kind the totals and the widest drawing) and returns their totals: line
/// length, area, and the count of drawings wider than the column.
fn size_table(kinds: &[(&str, usize, usize, usize)], chosen: Option<Engine>) -> Totals {
    let mut totals = Totals::new();
    for &(kind, length_budget, area_budget, wide_budget) in kinds {
        let rows = size_rows(kind, chosen);
        println!("{:40} length   width height     area", "");
        for (name, s) in rows.iter().filter(|(_, s)| s.width > COLUMN) {
            println!("{name:40} {:6} {:7} {:6} {:8}", s.length, s.width, s.height, s.area);
        }
        let (length, area) = rows.iter().fold((0, 0), |(l, a), (_, s)| (l + s.length, a + s.area));
        let wide = rows.iter().filter(|(_, s)| s.width > COLUMN).count();
        let widest = rows.iter().max_by_key(|(_, s)| s.width).map(|(name, s)| format!("{name} {}x{}", s.width, s.height)).unwrap_or_default();
        println!("TOTAL size {kind} length {length} area {area} wider than {COLUMN} {wide} of {} widest {widest}", rows.len());
        let fields = [("line length", length, length_budget), ("area", area, area_budget), ("drawings too wide", wide, wide_budget)];
        totals.extend(fields.into_iter().map(|(name, value, limit)| (format!("{kind}: {name}"), value, limit)));
    }
    totals
}

/// The size tables of the kinds with layout engines.
fn engine_size_table(chosen: Option<Engine>) -> Totals {
    size_table(&SIZE_BUDGETS[..3], chosen)
}

#[test]
fn size_stays_within_budget() {
    assert_within(size_table(&SIZE_BUDGETS, None));
}

/// Prints the sizes counted for the golden files named by `DRUMLIN_SIZE`: a
/// kind, such as `state`, lists every drawing's line length, frame, area and
/// aspect (width over height); one file, such as `state/gallery-tcp`, lists
/// each of its lines' length as well.
#[test]
fn explain_size() {
    let Ok(name) = std::env::var("DRUMLIN_SIZE") else { return };
    let (kind, file) = name.split_once('/').map_or((name.as_str(), None), |(kind, file)| (kind, Some(file)));
    let rows = size_rows(kind, explained_engine());
    for (row, s) in rows.iter().filter(|(row, _)| file.is_none_or(|_| *row == name)) {
        println!("{row:40} length {:6}  frame {:5} x {:5}  area {:8}  aspect {:.2}{}", s.length, s.width, s.height, s.area, s.width as f64 / s.height.max(1) as f64, if s.width > COLUMN { "  wider than the column" } else { "" });
    }
    let Some(file) = file else { return };
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(kind).join(format!("{file}.uml"))).unwrap();
    let lines = match kind {
        "component" => super::component::lines(&source, explained_engine()),
        "package" => super::package::lines(&source, explained_engine()),
        "state" => super::state::lines(&source, explained_engine()),
        "sequence" => super::sequence::scene(&source).map(|s| s.lines),
        _ => super::activity::scene(&source).map(|s| s.lines),
    };
    for (i, line) in lines.unwrap().iter().enumerate() {
        println!("{i:3}: length {:7.1}  from {:?} to {:?}", crate::core::metrics::length(line), line.first().unwrap(), line.last().unwrap());
    }
}

/// The size measures see what was first found by hand and no other measure
/// showed: on a source where the engines differ, network simplex draws
/// shorter lines than longest path. In the nested probe the moved
/// box also leaves its container's far corner, so the drawing gets narrower.
#[test]
fn network_simplex_draws_shorter_lines_where_the_engines_differ() {
    let names = ["Web", "Gateway", "Cart", "Pricing", "Orders", "Billing", "Flags"].map(|name| format!("component {name}\n")).concat();
    let flat = format!("diagram component\ndirection down\n{names}Web --> Gateway\nGateway --> Cart\nCart --> Pricing\nPricing --> Orders\nPricing --> Billing\nWeb ..> Flags\nFlags ..> Orders\nFlags ..> Billing");
    let nested = "diagram component \"Nested checkout\"\ndirection down\ncomponent Web\ncomponent Platform {\n  component Gateway\n  component Cart\n  component Pricing\n  component Orders\n  component Billing\n  component \"Feature Flags\" as flags\n  Gateway --> Cart\n  Cart --> Pricing : quote\n  Pricing --> Orders\n  Pricing --> Billing\n  flags ..> Orders\n  flags ..> Billing\n}\ncomponent Audit\nWeb --> Gateway\nGateway ..> flags\nWeb --> Audit\nAudit ..> Orders\nAudit ..> Billing\nnote flags: toggles per tenant\nAudit below Web";
    let drawn = |source: &str, name: &str| super::component::size(source, engine::named(name)).unwrap();
    for source in [flat.as_str(), nested] {
        let (longest, simplex) = (drawn(source, "longest-path"), drawn(source, "network-simplex"));
        println!("longest-path {longest:?}\nnetwork-simplex {simplex:?}");
        assert!(simplex.length < longest.length, "line length {} under network-simplex, {} under longest-path", simplex.length, longest.length);
        assert_eq!(super::component::size(source, None).unwrap(), longest, "the default engine is longest path");
    }
    let (longest, simplex) = (drawn(nested, "longest-path"), drawn(nested, "network-simplex"));
    assert!(simplex.width < longest.width && simplex.area < longest.area, "nested: {simplex:?} under network-simplex, {longest:?} under longest-path");
}

/// The tables measured per layout engine: every measure of component, package
/// and state drawings. Sequence and activity diagrams have no engines.
const ENGINE_TABLES: [fn(Option<Engine>) -> Totals; 5] = [lines_table, labels_table, ends_table, flow_table, engine_size_table];

/// The engines named by `DRUMLIN_ENGINES`: `all`, or names separated by commas.
fn compared_engines(names: &str) -> Vec<Engine> {
    match names.trim() {
        "all" => engine::ENGINES.to_vec(),
        names => names.split(',').map(str::trim).map(|name| engine::named(name).unwrap_or_else(|| panic!("{}", engine::unknown(name)))).collect(),
    }
}

/// The summary: one row per measure, one column per engine, then the budget.
fn summary(engines: &[Engine], totals: &[Totals]) -> Vec<String> {
    let header = engines.iter().fold(format!("{:44}", "measure"), |line, engine| format!("{line} {:>16}", engine.name)) + &format!(" {:>8}", "budget");
    let rows = totals[0].iter().enumerate().map(|(i, (name, _, limit))| totals.iter().fold(format!("{name:44}"), |line, t| format!("{line} {:>16}", t[i].1)) + &format!(" {limit:>8}"));
    std::iter::once(header).chain(rows).collect()
}

/// Prints every engine table for each engine named by `DRUMLIN_ENGINES` (see
/// the module comment), then the totals side by side with the default
/// engine's budgets. Budgets are not enforced here: the other tests enforce
/// them for the drawings as the sources ask for them.
#[test]
fn compare_engines() {
    let Ok(names) = std::env::var("DRUMLIN_ENGINES") else { return };
    let engines = compared_engines(&names);
    let totals: Vec<Totals> = engines
        .iter()
        .map(|&engine| {
            println!("== layout engine {}: {}", engine.name, engine.about);
            ENGINE_TABLES.iter().flat_map(|table| table(Some(engine))).collect()
        })
        .collect();
    for line in summary(&engines, &totals) {
        println!("{line}");
    }
}

#[test]
fn engine_names_select_engines_for_comparison() {
    assert_eq!(compared_engines("all"), engine::ENGINES.to_vec());
    assert_eq!(compared_engines(" longest-path "), vec![engine::DEFAULT]);
    let totals = vec![vec![("bends".to_string(), 3, 4)], vec![("bends".to_string(), 5, 4)]];
    let lines = summary(&[engine::DEFAULT, engine::DEFAULT], &totals);
    assert_eq!(lines[1].split_whitespace().collect::<Vec<&str>>(), ["bends", "3", "5", "4"]);
    assert_eq!(over_budget(&totals[1]), ["bends rose to 5 (budget 4)"]);
}

/// The sequence and activity totals as `Totals`, against their budgets.
fn scene_table() -> Totals {
    let kinds: [(&str, Scenes, Quality); 2] = [("sequence", super::sequence::scene, SEQUENCE_BUDGET), ("activity", super::activity::scene, ACTIVITY_BUDGET)];
    kinds
        .into_iter()
        .flat_map(|(kind, scene, budget)| {
            let total: Quality = scene_rows(kind, scene).iter().map(|(_, q)| *q).sum();
            let fields = [
                ("crossings", total.crossings, budget.crossings),
                ("bends", total.bends, budget.bends),
                ("lines through shapes", total.through, budget.through),
                ("shape overlaps", total.shapes, budget.shapes),
                ("labels on lines", total.label_lines, budget.label_lines),
                ("labels on shapes", total.label_shapes, budget.label_shapes),
                ("labels on labels", total.label_labels, budget.label_labels),
                ("text overflow", total.overflow, budget.overflow),
                ("uneven gaps", total.uneven, budget.uneven),
            ];
            fields.into_iter().map(move |(name, value, limit)| (format!("{kind}: {name}"), value, limit))
        })
        .collect()
}

/// Every measure of every golden drawing, in the face of the caller's scope.
fn all_tables() -> Totals {
    [lines_table(None), labels_table(None), ends_table(None), flow_table(None), size_table(&SIZE_BUDGETS, None), scene_table()].concat()
}

/// With `DRUMLIN_LETTERING` set, prints every measure of the golden drawings
/// laid out for the `lettering` style's text widths beside the default's.
/// Lettering has no budgets of its own: the budgets hold for the default style.
#[test]
fn compare_lettering() {
    if std::env::var_os("DRUMLIN_LETTERING").is_none() {
        return;
    }
    let default = all_tables();
    let lettering = crate::core::text::with_face(crate::core::text::Face::Lettering, all_tables);
    println!("{:44} {:>10} {:>10} {:>8}", "measure", "default", "lettering", "budget");
    for ((name, value, limit), (_, lettered, _)) in default.iter().zip(&lettering) {
        println!("{name:44} {value:>10} {lettered:>10} {limit:>8}");
    }
}

/// A run of text found in a rendered drawing: what it says, and its box from
/// the cap height to the baseline.
#[derive(Debug)]
struct Run {
    text: String,
    ink: Rect,
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&amp;", "&")
}

/// The value of attribute `name` in the tag that starts `tag`.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let end = tag.find('>').unwrap_or(tag.len());
    let at = tag[..end].find(&format!(" {name}=\""))? + name.len() + 3;
    tag[at..].split('"').next()
}

fn numbers(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).filter_map(|v| v.parse().ok()).collect()
}

/// The text runs of a drawing: `<text>` elements measured in Helvetica, and
/// lettering runs (`<g role="img">`) measured in the lettering, each with the
/// box its letters cover. The title is left out: it holds no box.
fn text_runs(svg: &str) -> Vec<Run> {
    use crate::core::lettering::{self, Hand};
    use crate::core::text::{width, Weight, FONT, SMALL};
    let set = svg.match_indices("<text ").filter_map(|(at, _)| {
        let tag = &svg[at..];
        let class = attribute(tag, "class")?;
        let (size, weight) = match class {
            "title" => return None,
            c if c.starts_with("name") => (FONT, Weight::Bold),
            _ => (SMALL, Weight::Regular),
        };
        let text = unescape(&tag[tag.find('>')? + 1..tag.find("</text>")?]);
        let (x, y): (f64, f64) = (attribute(tag, "x")?.parse().ok()?, attribute(tag, "y")?.parse().ok()?);
        let w = width(&text, size, weight);
        let left = match attribute(tag, "text-anchor") {
            Some("middle") => x - w / 2.0,
            Some("end") => x - w,
            _ => x,
        };
        Some(Run { text, ink: Rect { x: left, y: y - 0.72 * size, w, h: 0.72 * size } })
    });
    let lettered = svg.match_indices("<g class=\"lt\" role=\"img\"").filter_map(|(at, _)| {
        let tag = &svg[at..];
        let text = unescape(attribute(tag, "aria-label")?);
        let transform = attribute(tag, "transform")?;
        let xy = numbers(transform.split(')').next()?);
        let key = tag.match_indices("href=\"#dl-").map(|(i, m)| tag[i + m.len()..].chars().next().unwrap_or('r')).find(|&k| k != 'r')?;
        let hand = match key {
            'n' => Hand::Name,
            'k' => Hand::Keyword,
            't' => return None,
            _ => Hand::Small,
        };
        let w = lettering::width(&text, hand);
        let cap = 736.0 / 1024.0 * hand.em();
        let lean = if transform.contains("skewX") { 15f64.to_radians().tan() * cap } else { 0.0 };
        Some(Run { text, ink: Rect { x: xy[0], y: xy[1] - cap, w: w + lean, h: cap } })
    });
    set.chain(lettered).collect()
}

/// The boxes of a drawing that can hold text: rectangles and closed paths,
/// without the white background.
fn holders(svg: &str) -> Vec<Rect> {
    let rects = svg.match_indices("<rect ").filter(|(at, _)| attribute(&svg[*at..], "fill").is_none()).filter_map(|(at, _)| {
        let tag = &svg[at..];
        let v = |name: &str| attribute(tag, name).and_then(|v| v.parse::<f64>().ok());
        Some(Rect { x: v("x")?, y: v("y")?, w: v("width")?, h: v("height")? })
    });
    let paths = svg.match_indices("<path d=\"M").filter_map(|(at, _)| {
        let d = attribute(&svg[at..], "d")?;
        if !d.trim_end().ends_with('Z') {
            return None;
        }
        let v = numbers(d);
        let (xs, ys): (Vec<f64>, Vec<f64>) = (v.iter().step_by(2).copied().collect(), v.iter().skip(1).step_by(2).copied().collect());
        let (x0, x1) = (xs.iter().copied().fold(f64::MAX, f64::min), xs.iter().copied().fold(f64::MIN, f64::max));
        let (y0, y1) = (ys.iter().copied().fold(f64::MAX, f64::min), ys.iter().copied().fold(f64::MIN, f64::max));
        Some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
    });
    rects.chain(paths).collect()
}

/// Text runs that do not fit the smallest box around their middle: wider than
/// it, or above or below it. Text whose middle lies in no box, such as a
/// connector label, holds no box and is not counted.
fn misfits(svg: &str) -> Vec<String> {
    let boxes = holders(svg);
    text_runs(svg)
        .into_iter()
        .filter_map(|run| {
            let (cx, cy) = (run.ink.x + run.ink.w / 2.0, run.ink.y + run.ink.h / 2.0);
            let holder = boxes.iter().filter(|b| cx > b.x && cx < b.right() && cy > b.y && cy < b.bottom()).min_by(|a, b| (a.w * a.h).total_cmp(&(b.w * b.h)))?;
            let fits = run.ink.x >= holder.x && run.ink.right() <= holder.right() && run.ink.y >= holder.y && run.ink.bottom() <= holder.bottom();
            (!fits).then(|| format!("{:?} {:?} in {:?}", run.text, run.ink, holder))
        })
        .collect()
}

/// Every golden drawing in `lettering`: no text run sticks out of the box or
/// note that holds it, as in the default style, whose drawings are checked
/// the same way for comparison.
#[test]
fn lettering_text_fits_its_boxes() {
    let lettering = crate::Options { style: Some("lettering".to_string()), ..crate::Options::default() };
    let (mut runs, mut default_misfits, mut lettering_misfits) = (0, Vec::new(), Vec::new());
    for kind in ["component", "package", "state", "sequence", "activity"] {
        for path in sources(kind) {
            let source = fs::read_to_string(&path).unwrap();
            let name = format!("{kind}/{}", path.file_stem().unwrap().to_string_lossy());
            let (plain, lettered) = (crate::render(&source).unwrap(), crate::render_with(&source, &lettering).unwrap());
            runs += text_runs(&lettered).len();
            let letters = |svg: &str| {
                let mut all: Vec<char> = text_runs(svg).iter().flat_map(|run| run.text.chars().filter(|c| !c.is_whitespace()).collect::<Vec<char>>()).collect();
                all.sort_unstable();
                all
            };
            assert_eq!(letters(&plain), letters(&lettered), "{name}: every text run is found in both styles, if wrapped otherwise");
            default_misfits.extend(misfits(&plain).into_iter().map(|m| format!("{name}: {m}")));
            lettering_misfits.extend(misfits(&lettered).into_iter().map(|m| format!("{name}: {m}")));
        }
    }
    println!("{runs} text runs; misfits: default {}, lettering {}", default_misfits.len(), lettering_misfits.len());
    for line in default_misfits.iter().map(|m| format!("default: {m}")).chain(lettering_misfits.iter().map(|m| format!("lettering: {m}"))) {
        println!("{line}");
    }
    assert!(runs > 1000);
    assert!(lettering_misfits.is_empty(), "{}", lettering_misfits.join("\n"));
}
