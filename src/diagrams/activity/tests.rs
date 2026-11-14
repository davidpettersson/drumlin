// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Invariants of the structured layout, checked on every golden case and on
//! generated block trees. Golden files only guard regressions.

use super::layout::*;
use super::model::Diagram;
use super::parse::parse;
use crate::core::geometry::{Point, Rect, GRID};

fn shrink(r: &Rect) -> Rect {
    r.inflate(-0.5)
}

fn segments(l: &Layout) -> Vec<(Point, Point)> {
    l.edges.iter().flat_map(|e| e.points.windows(2).map(|w| (w[0], w[1]))).collect()
}

fn hits(seg: &(Point, Point), r: &Rect) -> bool {
    let (x0, x1) = (seg.0 .0.min(seg.1 .0), seg.0 .0.max(seg.1 .0));
    let (y0, y1) = (seg.0 .1.min(seg.1 .1), seg.0 .1.max(seg.1 .1));
    x1 >= r.x && x0 <= r.right() && y1 >= r.y && y0 <= r.bottom()
}

fn drawn_clean(l: &Layout, name: &str) {
    l.edges.iter().flat_map(|e| e.points.windows(2)).for_each(|w| assert!(w[0].0 == w[1].0 || w[0].1 == w[1].1, "{name}: oblique segment {w:?}"));
    // Note links may be diagonal, but are one short straight line, never a stub.
    l.links.iter().for_each(|[a, b]| assert!(((a.0 - b.0).hypot(a.1 - b.1)) >= 12.0 && ((a.0 - b.0).hypot(a.1 - b.1)) <= 48.0, "{name}: note link {a:?} {b:?} too short or long"));
    l.edges.iter().for_each(|e| assert!(e.points.len() >= 2, "{name}: degenerate edge"));
    l.nodes.iter().enumerate().for_each(|(i, a)| {
        assert_eq!(a.rect.x.rem_euclid(GRID), 0.0, "{name}: node off the grid: {:?}", a.rect);
        assert_eq!(a.rect.y.rem_euclid(GRID), 0.0, "{name}: node off the grid: {:?}", a.rect);
        l.nodes.iter().skip(i + 1).for_each(|b| assert!(!a.rect.intersects(&b.rect), "{name}: nodes overlap: {:?} {:?}", a.rect, b.rect));
        l.notes.iter().for_each(|n| assert!(!a.rect.intersects(&n.rect), "{name}: node on note: {:?}", a.rect));
    });
    let segs = segments(l);
    segs.iter().for_each(|s| {
        l.nodes.iter().for_each(|n| assert!(!hits(s, &shrink(&n.rect)), "{name}: line {s:?} through node {:?}", n.rect));
        l.notes.iter().for_each(|n| assert!(!hits(s, &n.rect), "{name}: line {s:?} through note {:?}", n.rect));
    });
    l.notes.iter().enumerate().for_each(|(i, a)| l.notes.iter().skip(i + 1).for_each(|b| assert!(!a.rect.intersects(&b.rect), "{name}: notes overlap")));
    l.labels.iter().enumerate().for_each(|(i, a)| {
        l.labels.iter().skip(i + 1).for_each(|b| assert!(!a.rect.intersects(&b.rect), "{name}: labels overlap: {:?}@{:?} {:?}@{:?}", a.lines, a.rect, b.lines, b.rect));
        l.nodes.iter().for_each(|n| assert!(!a.rect.intersects(&n.rect), "{name}: label {:?} on node {:?}", a.lines, n.rect));
        l.notes.iter().for_each(|n| assert!(!a.rect.intersects(&n.rect), "{name}: label {:?} on note", a.lines));
        l.links.iter().for_each(|[p, q]| assert!(!crate::core::metrics::path_touches(&[*p, *q], &a.rect), "{name}: label {:?} on note link", a.lines));
        segs.iter().for_each(|s| assert!(!hits(s, &a.rect), "{name}: label {:?} at {:?} on line {s:?}", a.lines, a.rect));
        l.bands.windows(2).for_each(|w| {
            let x = w[1].rect.x;
            assert!(!(x > a.rect.x && x < a.rect.right()), "{name}: label {:?} across a lane line", a.lines);
        });
    });
    l.bands.iter().for_each(|band| {
        l.nodes.iter().for_each(|n| {
            assert!(n.rect.x < band.rect.x || n.rect.x >= band.rect.x || n.rect.y >= band.head.bottom(), "{name}");
            let inside = n.rect.x >= band.rect.x && n.rect.right() <= band.rect.right();
            let outside = n.rect.right() <= band.rect.x || n.rect.x >= band.rect.right();
            assert!(inside || outside || n.kind == NodeKind::Bar, "{name}: node {:?} straddles a lane", n.rect);
        });
    });
}

fn golden() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/activity");
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "uml"))
        .map(|p| (p.display().to_string(), std::fs::read_to_string(&p).unwrap()))
        .collect()
}

#[test]
fn every_golden_case_draws_clean() {
    let cases = golden();
    assert!(cases.len() >= 15, "golden cases missing");
    cases.iter().for_each(|(name, source)| drawn_clean(&layout(&parse(source).unwrap()), name));
}

fn node_with(l: &Layout, text: &str) -> Rect {
    l.nodes.iter().find(|n| n.lines.iter().any(|t| t.contains(text))).unwrap().rect
}

fn laid(source: &str) -> (Diagram, Layout) {
    let d = parse(source).unwrap();
    let l = layout(&d);
    (d, l)
}

#[test]
fn an_empty_branch_leaves_the_body_on_the_axis() {
    let (_, l) = laid("diagram activity\nstart\nif \"x\" {\n  action \"Body\"\n}\naction \"After\"\nstop\n");
    let (body, after) = (node_with(&l, "Body"), node_with(&l, "After"));
    assert_eq!(body.center().0, after.center().0);
}

#[test]
fn a_branch_that_continues_takes_the_axis_when_the_other_stops() {
    let (_, l) = laid("diagram activity\nstart\nif \"bad\" {\n  action \"Reject\"\n  stop\n}\naction \"Next\"\nstop\n");
    let next = node_with(&l, "Next");
    let diamond = l.nodes.iter().find(|n| n.kind == NodeKind::Decision).unwrap();
    assert_eq!(diamond.rect.center().0, next.center().0);
    assert!(node_with(&l, "Reject").x > next.x);
    assert!(l.nodes.iter().all(|n| n.kind != NodeKind::Merge));
}

#[test]
fn the_first_of_two_filled_arms_continues_the_column() {
    let (_, l) = laid("diagram activity\nstart\nif \"x\" {\n  action \"Main\"\n} else {\n  action \"Side\"\n}\nstop\n");
    let (main, side) = (node_with(&l, "Main").center().0, node_with(&l, "Side").center().0);
    let centre = |kind: NodeKind| l.nodes.iter().find(|n| n.kind == kind).unwrap().rect.center().0;
    assert_eq!(centre(NodeKind::Decision), main);
    assert_eq!(centre(NodeKind::Merge), main);
    assert!(side > main);
}

#[test]
fn the_arm_that_continues_takes_the_axis_over_the_first_declared() {
    let (_, l) = laid("diagram activity\nstart\nif \"x\" {\n  action \"Ends\"\n  stop\n} else {\n  action \"Goes on\"\n}\naction \"Next\"\nstop\n");
    let d = l.nodes.iter().find(|n| n.kind == NodeKind::Decision).unwrap().rect.center().0;
    assert_eq!(node_with(&l, "Goes on").center().0, d);
    assert!(node_with(&l, "Ends").center().0 > d);
}

#[test]
fn a_fork_of_two_stays_symmetric() {
    let (_, l) = laid("diagram activity\nstart\nfork {\n  action \"Left\"\n} and {\n  action \"Right\"\n}\nstop\n");
    let (a, b) = (node_with(&l, "Left").center().0, node_with(&l, "Right").center().0);
    let bar = l.nodes.iter().find(|n| n.kind == NodeKind::Bar).unwrap().rect.center().0;
    assert_eq!(bar - a, b - bar);
}

#[test]
fn an_action_in_a_lane_sits_in_that_column() {
    let (_, l) = laid("diagram activity\nlane \"A\" {\n  action \"One\"\n}\nlane \"B\" {\n  action \"Two\"\n}\nlane \"A\" {\n  action \"Three\"\n}\n");
    let two = node_with(&l, "Two");
    assert!(two.x >= l.bands[1].rect.x && two.right() <= l.bands[1].rect.right());
    assert_eq!(node_with(&l, "One").center().0, node_with(&l, "Three").center().0);
}

#[test]
fn deterministic() {
    let src = &golden()[0].1;
    assert_eq!(format!("{:?}", layout(&parse(src).unwrap())), format!("{:?}", layout(&parse(src).unwrap())));
}

/// A small generator of block trees, so the invariants meet shapes no one wrote.
struct Gen {
    seed: u64,
    lanes: u64,
}

impl Gen {
    fn next(&mut self, n: u64) -> u64 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.seed >> 33) % n
    }

    fn body(&mut self, depth: u32, top: bool, tail: bool) -> String {
        let count = 1 + self.next(3);
        let body: Vec<String> = (0..count).map(|_| self.stmt(depth)).collect();
        let lane_wrap = self.lanes > 0 && (top || self.next(3) == 0);
        let lane = self.next(self.lanes.max(1));
        let text = if tail { format!("{}action \"tail\"\n", body.concat()) } else { body.concat() };
        match lane_wrap {
            true => format!("lane \"L{lane}\" {{\n{text}}}\n"),
            false => text,
        }
    }

    fn stmt(&mut self, depth: u32) -> String {
        let label = format!("step {}", self.next(9999));
        if depth == 0 {
            return format!("action \"{label}\"\n");
        }
        match self.next(12) {
            9 => format!("if \"{label}\" {{\n{}stop\n}}\n", self.body(depth - 1, false, false)),
            10 => format!("fork {{\n{}end\n}} and {{\n{}}}\n", self.body(depth - 1, false, false), self.body(depth - 1, false, false)),
            11 => format!("if \"{label}\" {{\n{}}} else if \"o\" {{\n{}}} else \"z\" {{\n}}\n", self.body(depth - 1, false, false), self.body(depth - 1, false, false)),
            0 => format!("if \"{label}\" {{\n{}}}\n", self.body(depth - 1, false, false)),
            1 => format!("if \"{label}\" {{\n{}}} else {{\n{}}}\n", self.body(depth - 1, false, false), self.body(depth - 1, false, false)),
            2 => format!("while \"{label}\" {{\n{}}}\n", self.body(depth - 1, false, false)),
            3 => format!("repeat {{\n{}}} until \"{label}\"\n", self.body(depth - 1, false, false)),
            4 => format!("fork {{\n{}}} and {{\n{}}}\n", self.body(depth - 1, false, false), self.body(depth - 1, false, false)),
            5 => format!("switch \"s\" {{\ncase \"a\" {{\n{}}}\ncase \"b\" {{\n{}}}\ncase \"c\" {{\n{}}}\n}}\n", self.body(depth - 1, false, false), self.body(depth - 1, false, false), self.body(depth - 1, false, false)),
            6 => format!("action \"{label}\"\nnote: a note about {label}\n"),
            7 => format!("send \"{label}\"\n"),
            _ => format!("action \"{label}\"\n"),
        }
    }

    fn diagram(&mut self) -> String {
        let lanes = self.lanes;
        let setup: String = (0..lanes).map(|i| format!("lane \"L{i}\" {{\n}}\n")).collect();
        let body = self.body(3, true, true);
        format!("diagram activity\n{setup}start\n{body}stop\n")
    }
}

#[test]
fn generated_flows_draw_clean() {
    (0..600u64).for_each(|seed| {
        let mut generator = Gen { seed: seed * 7919 + 1, lanes: seed % 4 };
        let source = generator.diagram();
        let d = parse(&source).unwrap_or_else(|e| panic!("{e}\n{source}"));
        let l = layout(&d);
        drawn_clean(&l, &format!("seed {seed}:\n{source}"));
    });
}

/// A loop whose body ends in a merge with a free left corner returns
/// from that corner, along the side it returns by, and never drops out of the
/// merge's bottom to turn back (Drawing principles, Lines: take a return line
/// around the outside). Interrupt service: the arrow after "Deliver to the
/// stack" leaves the merge to the left.
#[test]
fn a_loop_returns_from_the_merge_corner() {
    let src = "diagram activity\nstart\nwhile \"more?\" {\n  action \"Pop\"\n  if \"ok?\" {\n    action \"Deliver\"\n  } else {\n    action \"Drop\"\n  }\n}\nend";
    let l = layout(&parse(src).unwrap());
    let merges: Vec<Rect> = l.nodes.iter().filter(|n| n.kind == NodeKind::Merge).map(|n| n.rect).collect();
    let inner = merges.iter().copied().max_by(|a, b| a.y.total_cmp(&b.y)).unwrap();
    let back = l.edges.iter().find(|e| e.points[0] == (inner.x, inner.center().1)).expect("a line leaves the inner merge's left corner");
    assert!(back.points[1].0 < inner.x && back.points[1].1 == inner.center().1, "{:?}", back.points);
    assert!(!l.edges.iter().any(|e| e.points[0] == (inner.center().0, inner.bottom())), "nothing leaves the inner merge's bottom");
}
