// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Invariants of the timeline. Golden files only guard regressions; these check
//! the properties a careful drawing must have.

use super::super::parse::parse;
use super::*;

fn sample(body: &str) -> Layout {
    layout(&parse(&format!("diagram sequence\nparticipant A as a\nparticipant B as b\nparticipant C as c\n{body}")).unwrap())
}

fn label_rects(l: &Layout) -> Vec<Rect> {
    l.arrows.iter().filter_map(|a| a.label.as_ref()).map(|lab| lab.rect).collect()
}

fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.inflate(-0.5).intersects(&b.inflate(-0.5))
}

fn no_collisions(l: &Layout) {
    let labels = label_rects(l);
    labels.iter().enumerate().for_each(|(i, a)| {
        labels.iter().skip(i + 1).for_each(|b| assert!(!overlaps(a, b), "labels overlap: {a:?} {b:?}"));
        l.bars.iter().for_each(|bar| assert!(!overlaps(a, &bar.rect), "label {a:?} on bar {:?}", bar.rect));
        l.frames.iter().for_each(|f| assert!(!overlaps(a, &f.tab), "label {a:?} on tab {:?}", f.tab));
        l.refs.iter().for_each(|r| assert!(!overlaps(a, &r.rect), "label {a:?} on ref {:?}", r.rect));
    });
    l.arrows.iter().for_each(|a| {
        if let Some(lab) = &a.label {
            l.arrows.iter().flat_map(|o| o.points.windows(2)).filter(|seg| seg[0].1 == seg[1].1).for_each(|seg| {
                let y = seg[0].1;
                let (x0, x1) = (seg[0].0.min(seg[1].0), seg[0].0.max(seg[1].0));
                let r = lab.rect;
                assert!(!(y > r.y && y < r.bottom() && x1 > r.x && x0 < r.right()), "line {seg:?} through label {r:?}");
            });
        }
    });
}

/// Stricter checks for finished diagrams: every golden case, and text against
/// notes, heads and operand separators as well.
fn drawn_clean(l: &Layout) {
    no_collisions(l);
    let labels = label_rects(l);
    let guards: Vec<Rect> = l.frames.iter().flat_map(|f| f.guards.iter().map(|g| g.rect)).collect();
    labels.iter().chain(&guards).for_each(|t| {
        l.notes.iter().for_each(|n| assert!(!overlaps(t, &n.rect), "text {t:?} on note {:?}", n.rect));
        l.frames.iter().for_each(|f| {
            assert!(!overlaps(t, &f.tab), "text {t:?} on tab");
            f.separators.iter().for_each(|&y| assert!(!(y > t.y + 0.5 && y < t.bottom() - 0.5 && t.right() > f.rect.x && t.x < f.rect.right()), "text {t:?} on separator {y}"));
        });
    });
    labels.iter().for_each(|t| {
        l.lifelines.iter().for_each(|life| {
            assert!(!overlaps(t, &life.head), "label {t:?} on head {:?}", life.head);
            assert!(!(life.x > t.x + 1.0 && life.x < t.right() - 1.0 && t.y < life.bottom && t.bottom() > life.top), "label {t:?} crosses lifeline at {}", life.x);
        });
    });
}

#[test]
fn every_golden_case_draws_clean() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/sequence");
    let cases: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|x| x == "uml")).collect();
    assert!(cases.len() >= 15, "golden cases missing");
    for case in cases {
        let source = std::fs::read_to_string(&case).unwrap();
        let diagram = parse(&source).unwrap();
        println!("{}", case.display());
        drawn_clean(&layout(&diagram));
    }
}

#[test]
fn messages_only() {
    no_collisions(&sample("a -> b : first\nb ->> c : second\nc --> a : a third, rather long message for wrapping purposes\n"));
}

#[test]
fn nested_blocks_step_right_and_nest() {
    let l = sample("a -> b : outer() {\n  b -> b : inner() {\n    b -> c : call\n  }\n  return done\n}\n");
    let (outer, inner) = (l.bars[1].rect, l.bars[0].rect);
    assert!(inner.x > outer.x, "nested bar steps right");
    assert!(outer.y <= inner.y && outer.bottom() >= inner.bottom(), "nested bar lies inside its parent: {outer:?} {inner:?}");
    no_collisions(&l);
}

#[test]
fn bar_ends_at_the_reply() {
    let l = sample("a -> b : ask() {\n  return ok\n}\nb -> c : later\n");
    let reply_y = l.arrows[1].points[0].1;
    assert_eq!(l.bars[0].rect.bottom(), reply_y);
}

#[test]
fn self_message_label_clear_of_the_next_lifeline() {
    let l = sample("a -> a : run a fairly long and considered self call\n");
    let label = l.arrows[0].label.as_ref().unwrap();
    assert!(label.rect.right() < l.lifelines[1].x - 2.0);
    no_collisions(&l);
}

#[test]
fn rows_lie_on_the_grid_and_go_down() {
    let l = sample("a -> b : x\nb -> c : y\nc -> a : z\n");
    let ys: Vec<f64> = l.arrows.iter().map(|a| a.points[0].1).collect();
    assert!(ys.windows(2).all(|w| w[1] > w[0]));
    assert!(ys.iter().all(|y| y.rem_euclid(GRID) == 0.0), "{ys:?}");
}

#[test]
fn a_frame_contains_its_contents_with_inset_and_nests() {
    let l = sample("alt \"yes\" {\n  a -> b : ask\n  opt \"more\" {\n    b -> c : tell\n  }\n} else \"no\" {\n  b -> a : refuse\n}\n");
    let (inner, outer) = (l.frames[0].rect, l.frames[1].rect);
    assert!(outer.x < inner.x && outer.right() > inner.right() && outer.y < inner.y && outer.bottom() > inner.bottom());
    assert!(l.arrows.iter().flat_map(|a| a.points.iter()).all(|p| p.0 > outer.x + 8.0 - 1e-6 && p.0 < outer.right() - 8.0 + 1e-6));
    assert_eq!(l.frames[1].separators.len(), 1);
    no_collisions(&l);
}

#[test]
fn a_frame_spans_only_the_lifelines_it_uses() {
    let l = sample("loop \"forever\" {\n  a -> b : ping\n}\n");
    assert!(l.frames[0].rect.right() < l.lifelines[2].x);
}

#[test]
fn guard_stays_clear_of_the_tab_and_frame_edge() {
    let l = sample("loop \"a very long guard that demands some room\" {\n  a -> b : ping\n}\n");
    let f = &l.frames[0];
    let g = &f.guards[0];
    assert!(g.rect.x >= f.tab.right());
    assert!(g.rect.right() <= f.rect.right());
}

#[test]
fn created_lifeline_starts_at_its_head() {
    let l = layout(&parse("diagram sequence\nparticipant A as a\na -> new \"W\" as w : make\na -> a : after\n").unwrap());
    assert_eq!(l.lifelines[1].head.center().1, l.arrows[0].points[0].1);
    assert_eq!(l.lifelines[1].top, l.lifelines[1].head.bottom());
    assert!(l.arrows[1].points[0].1 >= l.lifelines[1].head.bottom());
}

#[test]
fn destruction_ends_the_lifeline() {
    let l = sample("a -> b : stop\ndestroy b\nc -> a : on\n");
    assert_eq!(l.lifelines[1].bottom, l.crosses[0].1);
    assert!(l.lifelines[2].bottom > l.crosses[0].1);
}

#[test]
fn deterministic() {
    let src = "diagram sequence\nactor U as u\nparticipant P as p\nu -> p : x {\n  p -> p : y\n  return z\n}\nnote over u, p: hello there\n";
    let a = format!("{:?}", layout(&parse(src).unwrap()));
    let b = format!("{:?}", layout(&parse(src).unwrap()));
    assert_eq!(a, b);
}

#[test]
fn parse_one_line_block_and_trailing_brace() {
    let a = parse("diagram sequence\nparticipant a\nparticipant b\nalt \"x\" { a -> b : hi }\na -> b : go {\n  return done\n}\n").unwrap();
    assert_eq!(a.items.len(), 2);
    assert!(matches!(&a.items[1], Item::Call(m, body) if m.label.as_deref() == Some("go") && body.len() == 1));
    assert!(matches!(&a.items[0], Item::Fragment(f) if matches!(&f.operands[0].body[0], Item::Message(m) if m.label.as_deref() == Some("hi"))));
}

#[test]
fn parse_inline_message_body_with_label() {
    let a = parse("diagram sequence\nparticipant a\nparticipant b\na -> b : q() { b -> a : r }\n").unwrap();
    assert!(matches!(&a.items[0], Item::Call(m, body) if m.label.as_deref() == Some("q()") && body.len() == 1));
}

#[test]
fn return_takes_free_text_and_labels_keep_their_text() {
    let a = parse("diagram sequence\nparticipant a\nparticipant b\na -> b : x() {\n  return f(x, y) # hint\n}\na -> b : issue #4 # note\n").unwrap();
    let Item::Call(_, body) = &a.items[0] else { panic!() };
    assert!(matches!(&body[0], Item::Return(m) if m.label.as_deref() == Some("f(x, y)")));
    assert!(matches!(&a.items[1], Item::Message(m) if m.label.as_deref() == Some("issue #4")));
}

#[test]
fn destroy_in_one_branch_allows_use_in_the_other() {
    parse("diagram sequence\nparticipant a\nparticipant b\nalt \"x\" {\n  destroy b\n} else \"y\" {\n  a -> b : hi\n}\na -> b : still\n").unwrap();
}
