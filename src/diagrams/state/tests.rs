// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Invariants of laid-out state diagrams over every golden state case.

use super::model::{Kind, Line};
use super::parse::parse;
use super::render::all_labels;
use super::hybrid::settle;
use super::routing::grown;
use crate::core::geometry::{Point, Rect};
use std::fs;
use std::path::Path;

fn cases() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/state");
    let mut names: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "uml")).collect();
    names.sort();
    names.iter().map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), fs::read_to_string(p).unwrap())).collect()
}

fn hits_path(rect: &Rect, points: &[Point]) -> bool {
    points.windows(2).any(|pair| {
        let s = Rect { x: pair[0].0.min(pair[1].0) - 1.0, y: pair[0].1.min(pair[1].1) - 1.0, w: (pair[0].0 - pair[1].0).abs() + 2.0, h: (pair[0].1 - pair[1].1).abs() + 2.0 };
        s.intersects(rect)
    })
}

#[test]
fn labels_are_clear_of_each_other_states_and_lines() {
    let all = cases();
    assert!(!all.is_empty());
    for (name, source) in all {
        let (d, drawn) = super::drawing(&source, None).unwrap();
        let (g, r) = (&drawn.geometry, &drawn.routes);
        assert!(r.failed.is_empty(), "{name}: routing failed for {:?}", r.failed);
        let labels = all_labels(&d, g, r, &drawn.placed);
        for (i, a) in labels.iter().enumerate() {
            for b in &labels[i + 1..] {
                assert!(!a.rect.intersects(&b.rect), "{name}: labels {:?} and {:?} overlap", a.lines, b.lines);
            }
            for n in 1..d.nodes.len() {
                let node = &d.nodes[n];
                if node.children.is_empty() && node.kind != Kind::Region {
                    assert!(!a.rect.intersects(&g.rects[n]), "{name}: label {:?} is on `{}`", a.lines, node.name);
                }
            }
            for (e, points) in r.edges.iter().enumerate() {
                if d.edges[e].line == Line::Transition {
                    assert!(!hits_path(&a.rect, points), "{name}: label {:?} is on the line of edge {e}", a.lines);
                }
            }
            for l in &g.loops {
                assert!(!hits_path(&a.rect, &l.points), "{name}: label {:?} is on a self-transition", a.lines);
            }
        }
    }
}

#[test]
fn nested_vertices_lie_inside_their_parents() {
    for (name, source) in cases() {
        let d = parse(&source).unwrap();
        let g = super::routing::arranged(&d);
        for n in 1..d.nodes.len() {
            let Some(p) = d.nodes[n].parent.filter(|&p| p != 0 && !d.nodes[n].kind.is_connection_point()) else { continue };
            let (r, o) = (g.rects[n], g.rects[p]);
            assert!(r.x >= o.x && r.y >= o.y && r.right() <= o.right() && r.bottom() <= o.bottom(), "{name}: node {n} escapes its parent");
        }
    }
}

#[test]
fn siblings_do_not_overlap() {
    for (name, source) in cases() {
        let d = parse(&source).unwrap();
        let g = super::routing::arranged(&d);
        for a in 1..d.nodes.len() {
            for b in a + 1..d.nodes.len() {
                if d.nodes[a].parent == d.nodes[b].parent && !d.nodes[a].kind.is_connection_point() && !d.nodes[b].kind.is_connection_point() {
                    assert!(!g.rects[a].intersects(&g.rects[b]), "{name}: {a} and {b} overlap");
                }
            }
        }
    }
}

#[test]
fn the_initial_pseudostate_is_first_in_its_rank_order() {
    for (name, source) in cases() {
        let d = parse(&source).unwrap();
        let g = super::routing::arranged(&d);
        for n in 1..d.nodes.len() {
            if d.nodes[n].kind != Kind::Initial {
                continue;
            }
            for &s in &d.nodes[d.nodes[n].parent.unwrap()].children {
                if s != n && d.nodes[s].kind == Kind::State {
                    let earlier = match d.direction {
                        crate::core::geometry::Direction::Down => g.rects[n].y < g.rects[s].y,
                        crate::core::geometry::Direction::Right => g.rects[n].x < g.rects[s].x,
                    };
                    assert!(earlier, "{name}: the initial pseudostate is not before state {s}");
                }
            }
        }
    }
}

/// Prints the measures and lines of the golden state file named by
/// `DRUMLIN_STAGES` (such as `gallery-tcp`) after layout, box growth and label
/// settling, to find the stage that changed a line.
#[test]
fn debug_stages() {
    let Ok(name) = std::env::var("DRUMLIN_STAGES") else { return };
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/state").join(format!("{name}.uml"))).unwrap();
    let d = parse(&source).unwrap();
    let g = super::routing::arranged(&d);
    let r = super::routing::route(&d, &g);
    println!("plain {:?}", crate::core::metrics::measure(&r.edges));
    for (i, e) in r.edges.iter().enumerate() { println!("  {i}: {e:?}"); }
    let g2 = grown(&d, g);
    let r2 = super::routing::route(&d, &g2);
    println!("grown {:?}", crate::core::metrics::measure(&r2.edges));
    for (i, e) in r2.edges.iter().enumerate() { println!("  {i}: {e:?}"); }
    let s = settle(&d, g2);
    println!("settled {:?}", crate::core::metrics::measure(&s.routes.edges));
}

/// The transition into an entry point and on to the state it leads
/// to run straight; the composite state's name gives way to the entry point.
#[test]
fn an_entry_point_continues_its_chain() {
    let source = "diagram state\n[*] --> Idle\nIdle --> again : retry\nstate Login {\n  entrypoint again\n  exitpoint aborted\n  again --> Prompt\n  Prompt --> aborted : [locked out]\n}\naborted --> Idle\n";
    let (d, drawn) = super::drawing(source, None).unwrap();
    let id = |name: &str| d.nodes.iter().position(|n| n.name == name).unwrap();
    let (again, prompt, login) = (id("again"), id("Prompt"), id("Login"));
    let line = d.edges.iter().position(|e| e.from == again && e.to == prompt).unwrap();
    assert_eq!(drawn.routes.edges[line].len(), 2, "again to Prompt jogs: {:?}", drawn.routes.edges[line]);
    let (x, anchor) = super::layout::name_place(&d, &drawn.geometry.rects, login);
    assert!(anchor != crate::core::svg::Anchor::Middle, "Login's name moves aside, at {x}");
}

/// A return line runs round the outside and meets both states on the
/// side it runs along (Drawing principles, Lines: take a return line around the
/// outside). USB device: `bus reset` leaves Configured on the right, below its
/// compartment line, and enters Default on the right.
#[test]
fn a_return_line_runs_round_the_outside() {
    let source = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/state/gallery-usb.uml")).unwrap();
    let (d, drawn) = super::drawing(&source, None).unwrap();
    let id = |name: &str| d.nodes.iter().position(|n| n.name == name).unwrap();
    let (configured, default) = (id("Configured"), id("Default"));
    let line = d.edges.iter().position(|e| e.from == configured && e.to == default).unwrap();
    let path = &drawn.routes.edges[line];
    let (c, f) = (drawn.geometry.rects[configured], drawn.geometry.rects[default]);
    assert_eq!(path[0].0, c.right(), "leaves Configured on the right: {path:?}");
    assert!(path[0].1 >= c.y + super::layout::NAME_ROW + 16.0, "below the compartment line: {path:?}");
    assert_eq!(path[path.len() - 1].0, f.right(), "enters Default on the right: {path:?}");
    let leaves: Vec<Rect> = drawn.geometry.rects[1..].to_vec();
    assert!(crate::core::ends::doubled_back(&leaves, &drawn.routes.edges).is_empty());
}

/// The box moves under the line into an entry point, so the initial
/// state, Idle, the entry point, Prompt and the exit point share one vertical
/// line; and a fork whose branches meet one join passes the column through, so
/// the initial state, Ready, the choice, Done and the final state share one
/// (Drawing principles, Flow: keep the spine on one line).
#[test]
fn the_spine_keeps_one_line_through_points_and_bars() {
    let golden = |name: &str| std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/state").join(format!("{name}.uml"))).unwrap();
    for (name, chain) in [("element-entry-and-exit-points", vec!["Idle", "again", "Prompt", "aborted"]), ("element-pseudostates", vec!["Ready", "c", "Done"])] {
        let (d, drawn) = super::drawing(&golden(name), None).unwrap();
        let centre = |n: usize| drawn.geometry.rects[n].center().0;
        let mut columns: Vec<f64> = chain.iter().map(|&c| centre(d.nodes.iter().position(|n| n.name == c).unwrap_or_else(|| panic!("{name}: no {c}")))).collect();
        columns.extend((1..d.nodes.len()).filter(|&n| matches!(d.nodes[n].kind, Kind::Initial | Kind::Final)).map(centre));
        assert!(columns.iter().all(|&x| (x - columns[0]).abs() < 0.5), "{name}: the spine is not on one line: {columns:?}");
    }
}
