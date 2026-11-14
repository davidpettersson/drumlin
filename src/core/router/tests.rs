// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for the router.

use super::grid::{blocked, route_afresh, simplify};
use super::nudge::{is_orthogonal, SEPARATION};
use super::*;
use crate::core::{ends, metrics};

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

fn across(obstacles: Vec<Rect>) -> Problem {
    let (from, to) = (rect(0.0, 80.0, 96.0, 48.0), rect(400.0, 80.0, 96.0, 48.0));
    let connection = Connection {
        from: Terminal::at((from.right(), 104.0), Side::Right, 0.0),
        to: Terminal::at((to.x, 104.0), Side::Left, 0.0),
        obstacles: obstacles.iter().map(|r| r.inflate(8.0)).collect(),
        strips: Strips::default(),
    };
    let bounds: Vec<Rect> = [from, to].into_iter().chain(obstacles.iter().copied()).collect();
    Problem { walls: bounds.iter().map(|r| r.inflate(8.0)).collect(), bounds, connections: vec![connection], centred: false }
}

fn segment_hits(path: &[Point], rect: &Rect) -> bool {
    path.windows(2).any(|pair| blocked(std::slice::from_ref(rect), pair[0], pair[1]))
}

/// A box above and left of another, joined bottom to top.
fn stairs(movable: bool) -> Problem {
    let (a, b) = (rect(0.0, 0.0, 96.0, 48.0), rect(200.0, 160.0, 96.0, 48.0));
    let to = Terminal::at((224.0, b.y), Side::Top, 0.0);
    let connection = Connection {
        from: Terminal::at((48.0, a.bottom()), Side::Bottom, 0.0),
        to: if movable { to.on(Body::all(b, 16.0)) } else { to },
        obstacles: [a, b].iter().map(|r| r.inflate(8.0)).collect(),
        strips: Strips::default(),
    };
    Problem { walls: [a, b].iter().map(|r| r.inflate(8.0)).collect(), bounds: vec![a, b], connections: vec![connection], centred: false }
}

#[test]
fn a_z_becomes_an_l_when_its_end_may_move() {
    let fixed = route(&stairs(false));
    assert_eq!(metrics::bends(&fixed.paths[0]), 2);
    let free = route(&stairs(true));
    assert_eq!(metrics::bends(&free.paths[0]), 1);
    let end = free.paths[0][free.paths[0].len() - 1];
    assert_eq!(end.0, 200.0, "the line meets the left side of the lower box");
}

#[test]
fn a_head_keeps_a_straight_run_longer_than_itself() {
    let t = Terminal::at((100.0, 50.0), Side::Right, 6.0).reaching(18.0);
    assert_eq!(t.stem, (128.0, 50.0), "the first grid line at least 18px past the point at 106");
    let mut problem = stairs(false);
    problem.connections[0].to = problem.connections[0].to.reaching(18.0);
    let path = &route(&problem).paths[0];
    let last = path[path.len() - 2];
    assert!(path[path.len() - 1].1 - last.1 >= 18.0, "{path:?}");
}

#[test]
fn merging_lines_meet_at_one_junction() {
    let (a, b, c) = (rect(0.0, 0.0, 96.0, 48.0), rect(0.0, 160.0, 96.0, 48.0), rect(400.0, 72.0, 96.0, 64.0));
    let into = |from: &Rect| Connection {
        from: Terminal::at((from.right(), from.center().1), Side::Right, 0.0),
        to: Terminal::at((c.x, c.center().1), Side::Left, 0.0),
        obstacles: [a, b, c].iter().map(|r| r.inflate(8.0)).collect(),
        strips: Strips::default(),
    };
    let problem = Problem { bounds: vec![a, b, c], walls: [a, b, c].iter().map(|r| r.inflate(8.0)).collect(), connections: vec![into(&a), into(&b)], centred: false };
    let paths = route(&problem).paths;
    assert!(ends::near_misses(&paths).is_empty(), "{paths:?}");
}

#[test]
fn unobstructed_connection_is_straight() {
    let solution = route(&across(vec![]));
    assert_eq!(solution.paths[0], vec![(96.0, 104.0), (400.0, 104.0)]);
    assert!(solution.failed.is_empty());
}

#[test]
fn routing_is_deterministic() {
    let problem = across(vec![rect(200.0, 40.0, 96.0, 128.0)]);
    assert_eq!(route_afresh(&problem), route_afresh(&problem));
}

#[test]
fn a_remembered_route_is_the_route_afresh() {
    let problem = across(vec![rect(200.0, 40.0, 96.0, 128.0)]);
    let first = route(&problem);
    assert_eq!(first, route_afresh(&problem));
    assert_eq!(route(&problem), first);
}

#[test]
fn obstacles_are_avoided() {
    let obstacle = rect(200.0, 40.0, 96.0, 128.0);
    let solution = route(&across(vec![obstacle]));
    assert!(solution.failed.is_empty());
    assert!(is_orthogonal(&solution.paths[0]));
    assert!(!segment_hits(&solution.paths[0], &obstacle));
}

#[test]
fn soft_strips_divert_a_route_when_possible() {
    let mut problem = across(vec![]);
    problem.bounds.push(rect(200.0, 0.0, 96.0, 48.0));
    problem.connections[0].strips.horizontal = vec![rect(150.0, 100.0, 200.0, 8.0)];
    let path = &route(&problem).paths[0];
    assert!(path.len() > 2, "expected a detour: {path:?}");
}

#[test]
fn an_enclosed_goal_is_reported_as_failed() {
    let mut problem = across(vec![]);
    let wall = rect(360.0, 40.0, 16.0, 128.0);
    problem.connections[0].obstacles = vec![wall, rect(344.0, 40.0, 128.0, 16.0), rect(344.0, 152.0, 128.0, 16.0), rect(464.0, 40.0, 16.0, 128.0)];
    let solution = route(&problem);
    assert_eq!(solution.failed, vec![0]);
    assert_eq!(solution.paths[0].len(), 2);
}

#[test]
fn simplify_removes_spikes_and_straight_joints() {
    let path = vec![(0.0, 0.0), (0.0, 10.0), (8.0, 10.0), (8.0, 4.0), (8.0, 10.0), (20.0, 10.0), (20.0, 10.0), (20.0, 30.0)];
    assert_eq!(simplify(path), vec![(0.0, 0.0), (0.0, 10.0), (20.0, 10.0), (20.0, 30.0)]);
}

#[test]
fn nudging_separates_lines_sharing_a_lane() {
    // Two connections between the same rows: A* puts both jogs on one line.
    let boxes = [rect(0.0, 0.0, 96.0, 48.0), rect(200.0, 0.0, 96.0, 48.0), rect(100.0, 200.0, 96.0, 48.0), rect(300.0, 200.0, 96.0, 48.0)];
    let link = |a: &Rect, b: &Rect| Connection {
        from: Terminal::at((a.center().0, a.bottom()), Side::Bottom, 0.0),
        to: Terminal::at((b.center().0, b.y), Side::Top, 0.0),
        obstacles: boxes.iter().map(|r| r.inflate(8.0)).collect(),
        strips: Strips::default(),
    };
    let problem = Problem { bounds: boxes.to_vec(), walls: boxes.iter().map(|r| r.inflate(8.0)).collect(), connections: vec![link(&boxes[0], &boxes[2]), link(&boxes[1], &boxes[3])], centred: false };
    let paths = route(&problem).paths;
    assert!(paths.iter().all(|p| is_orthogonal(p)));
    assert_eq!(metrics::measure(&paths).overlaps, 0);
    assert_eq!(metrics::close_runs(&paths[0], &paths[1], SEPARATION), 0);
}

#[test]
fn a_sliding_end_straightens_its_line() {
    let (a, b) = (rect(0.0, 0.0, 96.0, 48.0), rect(40.0, 200.0, 96.0, 48.0));
    let connection = Connection {
        from: Terminal::at((48.0, a.bottom()), Side::Bottom, 0.0).sliding(16.0, 80.0),
        to: Terminal::at((88.0, b.y), Side::Top, 0.0).sliding(56.0, 120.0),
        obstacles: vec![a.inflate(8.0), b.inflate(8.0)],
        strips: Strips::default(),
    };
    let problem = Problem { bounds: vec![a, b], walls: vec![a.inflate(8.0), b.inflate(8.0)], connections: vec![connection], centred: false };
    let path = &route(&problem).paths[0];
    assert_eq!(path.len(), 2, "expected a straight line: {path:?}");
    assert!((16.0..=80.0).contains(&path[0].0) && (56.0..=120.0).contains(&path[1].0));
}

#[test]
fn slots_keep_their_spacing_on_one_side() {
    let slots = (0..3).map(|i| Slot { item: 0, side: Side::Left, coord: i as f64 }).collect();
    let settled = settle_slots(slots, 24.0, |_, _| (16.0, 96.0), |_, _| 56.0);
    let mut coords: Vec<f64> = settled.iter().map(|s| s.coord).collect();
    coords.sort_by(f64::total_cmp);
    assert!(coords.windows(2).all(|p| p[1] - p[0] >= 24.0));
    assert!(coords.iter().all(|c| (16.0..=96.0).contains(c)));
}

#[test]
fn boundary_strips_follow_a_lowered_top() {
    let strips = boundary_strips([(rect(0.0, 0.0, 100.0, 80.0), 24.0)]);
    let tops: Vec<f64> = strips.horizontal.iter().map(|r| r.y + 10.0).collect();
    assert_eq!(tops, vec![24.0, 80.0]);
    let sides: Vec<f64> = strips.vertical.iter().map(|r| r.x + 10.0).collect();
    assert_eq!(sides, vec![0.0, 100.0]);
    assert!(strips.vertical.iter().all(|r| r.y == -10.0 && r.h == 100.0));
}

#[test]
fn lone_slots_are_alone_on_their_side() {
    let r = rect(0.0, 0.0, 100.0, 40.0);
    let slots = vec![Slot::centred(1, Side::Top, r), Slot::centred(1, Side::Top, r), Slot::centred(1, Side::Left, r)];
    assert_eq!(slots[0].coord, 50.0);
    assert_eq!(slots[2].coord, 20.0);
    assert_eq!((0..3).map(|i| lone(&slots, i)).collect::<Vec<_>>(), vec![false, false, true]);
}

/// A long line whose last three runs come from beside a box and turn
/// back into its top meets the side it comes from instead, at its middle, with
/// a bend fewer (Drawing principles, Lines: never double back into a box).
#[test]
fn a_line_meets_the_side_it_arrives_from() {
    let b = rect(0.0, 200.0, 96.0, 48.0);
    let from = Terminal::at((400.0, 0.0), Side::Left, 0.0);
    let to = Terminal::at((48.0, b.y), Side::Top, 0.0).on(Body::all(b, 16.0));
    let connection = Connection { from, to, obstacles: vec![b.inflate(8.0)], strips: Strips::default() };
    let problem = Problem { walls: vec![b.inflate(8.0)], bounds: vec![b], connections: vec![connection], centred: false };
    let doubled = vec![(400.0, 0.0), (300.0, 0.0), (300.0, 160.0), (48.0, 160.0), (48.0, 200.0)];
    assert_eq!(ends::doubled_back(&[b], std::slice::from_ref(&doubled)).len(), 1);
    let paths = super::l_over_z::prefer_l(&problem, vec![doubled.clone()], &[false]);
    let path = &paths[0];
    // The end leaves the top, which the line came down beside; with nothing in
    // the way the line then also loses its Z and meets the top as an L.
    assert!(metrics::bends(path) < metrics::bends(&doubled), "{path:?}");
    assert!(ends::doubled_back(&[b], &paths).is_empty());
}

/// A box held only by its top band of a header takes no side end
/// there (`Body::head`).
#[test]
fn a_header_keeps_side_ends_below_it() {
    let b = rect(0.0, 200.0, 96.0, 48.0);
    let from = Terminal::at((400.0, 0.0), Side::Left, 0.0);
    let to = Terminal::at((48.0, b.y), Side::Top, 0.0).on(Body { head: 40.0, ..Body::all(b, 16.0) });
    let connection = Connection { from, to, obstacles: vec![b.inflate(8.0)], strips: Strips::default() };
    let problem = Problem { walls: vec![b.inflate(8.0)], bounds: vec![b], connections: vec![connection], centred: false };
    let doubled = vec![(400.0, 0.0), (300.0, 0.0), (300.0, 160.0), (48.0, 160.0), (48.0, 200.0)];
    let paths = super::l_over_z::prefer_l(&problem, vec![doubled.clone()], &[false]);
    assert_eq!(paths[0], doubled, "no room below a 40px header on a 48px side");
}

/// With the start fixed so that no L into the top is possible, the
/// line meets the right side, the one it comes down beside, at its middle.
#[test]
fn a_line_turns_into_the_side_beside_it() {
    let b = rect(0.0, 200.0, 96.0, 48.0);
    let from = Terminal::at((400.0, 280.0), Side::Top, 0.0);
    let to = Terminal::at((48.0, b.y), Side::Top, 0.0).on(Body::all(b, 16.0));
    let connection = Connection { from, to, obstacles: vec![b.inflate(8.0)], strips: Strips::default() };
    let problem = Problem { walls: vec![b.inflate(8.0)], bounds: vec![b], connections: vec![connection], centred: false };
    let doubled = vec![(400.0, 280.0), (400.0, 120.0), (300.0, 120.0), (300.0, 160.0), (48.0, 160.0), (48.0, 200.0)];
    let paths = super::l_over_z::prefer_l(&problem, vec![doubled.clone()], &[false]);
    let path = &paths[0];
    assert_eq!(path[path.len() - 1], (b.right(), b.center().1), "{path:?}");
    assert!(ends::doubled_back(&[b], &paths).is_empty());
}
