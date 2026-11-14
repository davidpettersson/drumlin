// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! The grid and the search: a sparse grid of candidate lines beside items,
//! between walls and through terminals, and A* over it with costs for length,
//! bends, crossings and shared lanes. `route` runs the whole pipeline.

use super::l_over_z::prefer_l;
use super::nudge::nudge;
use super::{manhattan, Body, Connection, Problem, Solution, Strips, Terminal};
use crate::core::geometry::{snap, snap_up, Point, Rect, Side, GRID};
use crate::core::metrics;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const BEND: i64 = 48;
const CROSS: i64 = 144;
const SHARED: i64 = 600;
const BOUNDARY: i64 = 400;
/// Terminals facing each other this close together get a straight or shared-lane path.
const NARROW: f64 = 48.0;
const ROUNDS: usize = 3;

type Lane = (usize, usize, bool);

/// A set of lanes on one grid, one flag per unit segment.
struct Lanes {
    ny: usize,
    flags: Vec<bool>,
}

impl Lanes {
    fn new(grid: &Grid) -> Lanes {
        Lanes { ny: grid.ys.len(), flags: vec![false; grid.xs.len() * grid.ys.len() * 2] }
    }

    fn index(&self, (ix, iy, horizontal): Lane) -> usize {
        (ix * self.ny + iy) * 2 + horizontal as usize
    }

    fn insert(&mut self, lane: Lane) {
        let i = self.index(lane);
        self.flags[i] = true;
    }

    fn contains(&self, lane: &Lane) -> bool {
        self.flags[self.index(*lane)]
    }
}

struct Grid {
    xs: Vec<f64>,
    ys: Vec<f64>,
}

const DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

fn dir_of(side: Side) -> usize {
    match side {
        Side::Right => 0,
        Side::Left => 1,
        Side::Bottom => 2,
        Side::Top => 3,
    }
}

fn grid_index(values: &[f64], v: f64) -> usize {
    values.partition_point(|&x| x < v - 0.01)
}

fn sorted(mut values: Vec<f64>) -> Vec<f64> {
    values.sort_by(f64::total_cmp);
    values.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    values
}

fn lanes_between(edges: Vec<f64>) -> Vec<f64> {
    sorted(edges)
        .windows(2)
        .flat_map(|pair| {
            let gap = pair[1] - pair[0];
            let middle = snap((pair[0] + pair[1]) / 2.0);
            let centre = (gap >= 16.0).then_some(vec![middle]);
            let near = (gap >= 48.0).then_some(vec![middle - 8.0, middle + 8.0]);
            let far = (gap >= 64.0).then_some(vec![middle - 16.0, middle + 16.0]);
            [centre, near, far].into_iter().flatten().flatten()
        })
        .collect()
}

pub(super) fn blocked(rects: &[Rect], a: Point, b: Point) -> bool {
    // Zero-width rect: travel exactly on an inflated boundary keeps clearance and is allowed.
    let segment = Rect { x: a.0.min(b.0), y: a.1.min(b.1), w: (a.0 - b.0).abs(), h: (a.1 - b.1).abs() };
    rects.iter().any(|o| o.intersects(&segment))
}

fn lane_of(ix: usize, iy: usize, dir: usize) -> Lane {
    // A lane names the unit grid segment leaving the lower-coordinate vertex.
    match dir {
        0 => (ix, iy, true),
        1 => (ix - 1, iy, true),
        2 => (ix, iy, false),
        _ => (ix, iy - 1, false),
    }
}

/// The search's working arrays, kept between the searches of one `route`.
/// Only the states a search touched are reset for the next.
struct Scratch {
    dist: Vec<i64>,
    previous: Vec<usize>,
    touched: Vec<usize>,
}

impl Scratch {
    fn new(grid: &Grid) -> Scratch {
        let cells = grid.xs.len() * grid.ys.len();
        Scratch { dist: vec![i64::MAX; cells * 4], previous: vec![usize::MAX; cells * 4], touched: Vec::new() }
    }

    fn reset(&mut self) {
        for &state in &self.touched {
            self.dist[state] = i64::MAX;
            self.previous[state] = usize::MAX;
        }
        self.touched.clear();
    }
}

/// A* between two stems, returning the grid path and the lanes it occupies.
fn search(grid: &Grid, from: &Terminal, to: &Terminal, connection: &Connection, used: &Lanes, crossable: &Lanes, scratch: &mut Scratch, segment: &mut [u8]) -> Option<(Vec<Point>, Vec<Lane>)> {
    let (nx, ny) = (grid.xs.len(), grid.ys.len());
    let (start_dir, goal_dir) = (dir_of(from.outward), dir_of(to.outward.opposite()));
    let state = |ix: usize, iy: usize, dir: usize| (ix * ny + iy) * 4 + dir;
    let (sx, sy) = (grid_index(&grid.xs, from.stem.0), grid_index(&grid.ys, from.stem.1));
    let (gx, gy) = (grid_index(&grid.xs, to.stem.0), grid_index(&grid.ys, to.stem.1));
    scratch.reset();
    let Scratch { dist, previous, touched } = scratch;
    // Each entry is an estimate and a state packed into one number, estimate
    // high, so the smallest pops first and equal estimates pop by state.
    let mut heap = BinaryHeap::new();
    let entry = |estimate: i64, state: usize| {
        debug_assert!((0..1 << 31).contains(&estimate) && state < 1 << 32);
        Reverse((estimate as u64) << 32 | state as u64)
    };
    // `segment` holds whether each unit segment is blocked (bit 0) or crosses a
    // strip (bit 1), worked out the first time a search for this connection
    // tries it (bit 2): the connection's obstacles and strips stay the same
    // through every round.
    let heuristic = |ix: usize, iy: usize| ((grid.xs[ix] - grid.xs[gx]).abs() + (grid.ys[iy] - grid.ys[gy]).abs()) as i64;
    let first = state(sx, sy, start_dir);
    dist[first] = 0;
    touched.push(first);
    heap.push(entry(heuristic(sx, sy), first));
    let mut finish = None;
    while let Some(Reverse(packed)) = heap.pop() {
        let current = (packed & 0xffff_ffff) as usize;
        let dir = current % 4;
        let (ix, iy) = (current / 4 / ny, current / 4 % ny);
        if ix == gx && iy == gy && dir == goal_dir {
            finish = Some(current);
            break;
        }
        // The stem may be entered from the side when its straight approach is
        // blocked. The turn happens in place at the goal cell.
        if ix == gx && iy == gy && dir != (goal_dir ^ 1) {
            let turned = state(gx, gy, goal_dir);
            let total = dist[current] + BEND;
            if total < dist[turned] {
                if dist[turned] == i64::MAX {
                    touched.push(turned);
                }
                dist[turned] = total;
                previous[turned] = current;
                heap.push(entry(total, turned));
            }
        }
        for (next_dir, &(dx, dy)) in DIRS.iter().enumerate() {
            if next_dir == (dir ^ 1) {
                continue;
            }
            let (jx, jy) = (ix as i32 + dx, iy as i32 + dy);
            if jx < 0 || jy < 0 || jx >= nx as i32 || jy >= ny as i32 {
                continue;
            }
            let (jx, jy) = (jx as usize, jy as usize);
            let (a, b) = ((grid.xs[ix], grid.ys[iy]), (grid.xs[jx], grid.ys[jy]));
            let lane = lane_of(ix, iy, next_dir);
            let index = used.index(lane);
            let known = &mut segment[index];
            if *known == 0 {
                let strips = if next_dir < 2 { &connection.strips.horizontal } else { &connection.strips.vertical };
                *known = 4 | blocked(&connection.obstacles, a, b) as u8 | (blocked(strips, a, b) as u8) << 1;
            }
            let known = *known;
            if known & 1 != 0 {
                continue;
            }
            let length = manhattan(a, b) as i64;
            let bend_cost = if next_dir != dir { BEND } else { 0 };
            let strip_cost = if known & 2 != 0 { BOUNDARY } else { 0 };
            let shared_cost = if used.contains(&lane) { SHARED + length * 4 } else { 0 };
            // Going straight through a vertex that another route passes through
            // at right angles is a crossing. A lane's flag is true when it runs in x.
            let vertical_move = next_dir >= 2;
            let (across_before, across_after) = if vertical_move {
                (ix > 0 && crossable.contains(&(ix - 1, iy, true)), crossable.contains(&(ix, iy, true)))
            } else {
                (iy > 0 && crossable.contains(&(ix, iy - 1, false)), crossable.contains(&(ix, iy, false)))
            };
            let cross_cost = if next_dir == dir && across_before && across_after { CROSS } else { 0 };
            let next = state(jx, jy, next_dir);
            let total = dist[current] + length + bend_cost + strip_cost + shared_cost + cross_cost;
            if total < dist[next] {
                if dist[next] == i64::MAX {
                    touched.push(next);
                }
                dist[next] = total;
                previous[next] = current;
                heap.push(entry(total + heuristic(jx, jy), next));
            }
        }
    }
    let end = finish?;
    let mut cells: Vec<usize> = std::iter::successors(Some(end), |&cell| (previous[cell] != usize::MAX).then_some(previous[cell])).collect();
    cells.reverse();
    let point_of = |cell: usize| (grid.xs[cell / 4 / ny], grid.ys[cell / 4 % ny]);
    let lanes = cells
        .windows(2)
        .filter(|w| w[1] / 4 != w[0] / 4)
        .map(|w| {
            let dir = w[1] % 4;
            let (dx, dy) = DIRS[dir];
            let (ix, iy) = (w[1] / 4 / ny, w[1] / 4 % ny);
            lane_of((ix as i32 - dx) as usize, (iy as i32 - dy) as usize, dir)
        })
        .collect();
    Some((cells.iter().map(|&cell| point_of(cell)).collect(), lanes))
}

/// The unit lanes of a straight run between two grid points. Direct lines and
/// stems never enter the search, but another route can still cross them.
fn run_lanes(grid: &Grid, a: Point, b: Point) -> Vec<Lane> {
    let (ax, ay, bx, by) = (grid_index(&grid.xs, a.0), grid_index(&grid.ys, a.1), grid_index(&grid.xs, b.0), grid_index(&grid.ys, b.1));
    let horizontal = (ax.min(bx)..ax.max(bx)).map(|ix| (ix, ay, true));
    let vertical = (ay.min(by)..ay.max(by)).map(|iy| (ax, iy, false));
    horizontal.chain(vertical).collect()
}

/// Drops repeated points and interior points of straight runs, including a run
/// that doubles back on itself, until none is left.
pub(super) fn simplify(points: Vec<Point>) -> Vec<Point> {
    let once = |points: &[Point]| {
        points.iter().fold(Vec::new(), |mut out: Vec<Point>, &p| {
            if let [.., a, b] = out[..] {
                if (a.0 == b.0 && b.0 == p.0) || (a.1 == b.1 && b.1 == p.1) {
                    out.pop();
                }
            }
            if out.last() != Some(&p) {
                out.push(p);
            }
            out
        })
    };
    let next = once(&points);
    if next.len() == points.len() { next } else { simplify(next) }
}

/// Distance from `from` to `to` along `from`'s outward direction, when the
/// terminals face each other. `None` when they do not.
fn facing_distance(from: &Terminal, to: &Terminal) -> Option<f64> {
    let (ux, uy) = from.outward.out();
    let along = (to.point.0 - from.point.0) * ux + (to.point.1 - from.point.1) * uy;
    (to.outward == from.outward.opposite() && along > 0.0).then_some(along)
}

/// Terminals after narrow-gap adjustment, with any added search line.
struct Settled {
    from: Terminal,
    to: Terminal,
    lane_x: Option<f64>,
    lane_y: Option<f64>,
}

/// In a narrow gap, both stems meet on one centre lane instead of overshooting.
/// Connections fanning out of one point then share a trunk.
fn settle_stems(connection: &Connection) -> Settled {
    let (from, to) = (connection.from, connection.to);
    let (a, b) = (from.point, to.point);
    let unchanged = Settled { from, to, lane_x: None, lane_y: None };
    // The shared lane, as near the middle as both ends' reach allows.
    let lane = |a: f64, b: f64| {
        let (lo, hi) = if a < b { (a + from.reach(), b - to.reach()) } else { (b + to.reach(), a - from.reach()) };
        let (middle, lo, hi) = (snap((a + b) / 2.0), snap_up(lo, GRID), (hi / GRID).floor() * GRID);
        if lo <= hi { middle.clamp(lo, hi) } else { middle }
    };
    match facing_distance(&from, &to) {
        Some(along) if along < NARROW && from.outward.is_vertical_edge() => {
            let x = lane(a.0, b.0);
            Settled { from: Terminal { stem: (x, a.1), ..from }, to: Terminal { stem: (x, b.1), ..to }, lane_x: Some(x), lane_y: None }
        }
        Some(along) if along < NARROW => {
            let y = lane(a.1, b.1);
            Settled { from: Terminal { stem: (a.0, y), ..from }, to: Terminal { stem: (b.0, y), ..to }, lane_x: None, lane_y: Some(y) }
        }
        _ => unchanged,
    }
}

fn build_grid(problem: &Problem, settled: &[Settled]) -> Grid {
    let beside = |lo: f64, hi: f64| [snap(lo - 16.0), snap(hi + 16.0), snap(lo - 24.0), snap(hi + 24.0)];
    let ends = |c: &Connection| [c.from.point, c.from.stem, c.to.point, c.to.stem];
    let xs = problem.bounds.iter().flat_map(|r| beside(r.x, r.right()));
    let ys = problem.bounds.iter().flat_map(|r| beside(r.y, r.bottom()));
    let wall_xs = lanes_between(problem.walls.iter().flat_map(|o| [o.x, o.right()]).collect());
    let wall_ys = lanes_between(problem.walls.iter().flat_map(|o| [o.y, o.bottom()]).collect());
    let term_xs = problem.connections.iter().flat_map(|c| ends(c).map(|p| p.0));
    let term_ys = problem.connections.iter().flat_map(|c| ends(c).map(|p| p.1));
    let extra_xs = settled.iter().filter_map(|s| s.lane_x);
    let extra_ys = settled.iter().filter_map(|s| s.lane_y);
    Grid {
        xs: sorted(xs.chain(wall_xs).chain(term_xs).chain(extra_xs).collect()),
        ys: sorted(ys.chain(wall_ys).chain(term_ys).chain(extra_ys).collect()),
    }
}

/// Facing, aligned terminals in a narrow gap are joined by one straight line.
fn is_direct(from: &Terminal, to: &Terminal) -> bool {
    let aligned = if from.outward.is_vertical_edge() { from.point.1 == to.point.1 } else { from.point.0 == to.point.0 };
    aligned && facing_distance(from, to).is_some_and(|along| along <= NARROW)
}

/// Problems remembered by `route` on each thread.
const REMEMBERED: usize = 32;

thread_local! {
    /// Recent problems and their solutions. Layout steps that compare layouts
    /// route the same layout many times over, and routing is the costly part of
    /// drawing.
    static SOLVED: std::cell::RefCell<std::collections::HashMap<Vec<u64>, Solution>> = std::cell::RefCell::default();
}

/// Every value in a problem, numbers by their exact bits, so two problems have
/// the same key only when they are the same. The patterns name every field, so
/// a field added later must be added here too.
fn key(problem: &Problem) -> Vec<u64> {
    let mut out = Vec::new();
    let rects = |out: &mut Vec<u64>, rects: &[Rect]| {
        out.push(rects.len() as u64);
        out.extend(rects.iter().flat_map(|&Rect { x, y, w, h }| [x, y, w, h].map(f64::to_bits)));
    };
    let terminal = |out: &mut Vec<u64>, &Terminal { point, stem, outward, slide, body }: &Terminal| {
        out.extend([point.0, point.1, stem.0, stem.1].map(f64::to_bits));
        out.push(outward as u64);
        match slide {
            Some((lo, hi)) => out.extend([1, lo.to_bits(), hi.to_bits()]),
            None => out.push(0),
        }
        match body {
            Some(Body { rect: Rect { x, y, w, h }, inset, sides, head }) => {
                out.push(1);
                out.extend([x, y, w, h, inset, head].map(f64::to_bits));
                out.extend(sides.map(u64::from));
            }
            None => out.push(0),
        }
    };
    let Problem { bounds, walls, connections, centred } = problem;
    rects(&mut out, bounds);
    rects(&mut out, walls);
    out.push(connections.len() as u64);
    for Connection { from, to, obstacles, strips: Strips { horizontal, vertical } } in connections {
        terminal(&mut out, from);
        terminal(&mut out, to);
        rects(&mut out, obstacles);
        rects(&mut out, horizontal);
        rects(&mut out, vertical);
    }
    out.push(u64::from(*centred));
    out
}

/// Routes all connections: short ones first, then two rip-up rounds to settle
/// shared-lane conflicts. The same problem always gets the same solution, so a
/// recent one is answered from memory.
pub fn route(problem: &Problem) -> Solution {
    let key = key(problem);
    if let Some(solution) = SOLVED.with(|solved| solved.borrow().get(&key).cloned()) {
        return solution;
    }
    let solution = route_afresh(problem);
    SOLVED.with(|solved| {
        let mut solved = solved.borrow_mut();
        if solved.len() >= REMEMBERED {
            solved.clear();
        }
        solved.insert(key, solution.clone());
    });
    solution
}

fn route_afresh(problem: &Problem) -> Solution {
    let settled: Vec<Settled> = problem.connections.iter().map(settle_stems).collect();
    let grid = build_grid(problem, &settled);
    let count = settled.len();
    let span = |e: usize| manhattan(settled[e].from.point, settled[e].to.point) as i64;
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by_key(|&e| (span(e), e));
    let mut paths: Vec<Vec<Point>> = vec![Vec::new(); count];
    let mut lanes: Vec<Vec<Lane>> = vec![Vec::new(); count];
    // What other routes must count as crossings: `lanes` plus direct lines and stems.
    let mut stubs: Vec<Vec<Lane>> = vec![Vec::new(); count];
    let mut failed = Vec::new();
    let mut scratch = Scratch::new(&grid);
    let mut segments: Vec<Vec<u8>> = vec![Vec::new(); count];
    for round in 0..ROUNDS {
        for &e in &order {
            let Settled { from, to, .. } = &settled[e];
            if is_direct(from, to) {
                paths[e] = vec![from.point, to.point];
                lanes[e] = Vec::new();
                stubs[e] = run_lanes(&grid, from.point, to.point);
                continue;
            }
            let mut used = Lanes::new(&grid);
            let mut crossable = Lanes::new(&grid);
            for o in (0..count).filter(|&o| o != e) {
                lanes[o].iter().for_each(|&lane| used.insert(lane));
                lanes[o].iter().chain(&stubs[o]).for_each(|&lane| crossable.insert(lane));
            }
            if segments[e].is_empty() {
                segments[e] = vec![0; grid.xs.len() * grid.ys.len() * 2];
            }
            let (points, found) = search(&grid, from, to, &problem.connections[e], &used, &crossable, &mut scratch, &mut segments[e]).unwrap_or_else(|| {
                if round == ROUNDS - 1 {
                    failed.push(e);
                }
                (vec![from.stem, to.stem], Vec::new())
            });
            let full = std::iter::once(from.point).chain(points).chain(std::iter::once(to.point)).collect();
            paths[e] = simplify(full);
            lanes[e] = found;
            stubs[e] = run_lanes(&grid, from.point, from.stem).into_iter().chain(run_lanes(&grid, to.stem, to.point)).collect();
        }
    }
    let fixed: Vec<bool> = (0..count).map(|e| failed.contains(&e) || is_direct(&settled[e].from, &settled[e].to)).collect();
    let paths = nudge(problem, paths, fixed.clone());
    let paths = prefer_l(problem, paths, &fixed);
    // An L can free the room two other lines were fighting over: lines still
    // lying on top of each other get one more nudge.
    let overlaps = |paths: &[Vec<Point>]| metrics::measure(paths).overlaps;
    let paths = match overlaps(&paths) {
        0 => paths,
        before => {
            let again = nudge(problem, paths.clone(), fixed);
            if overlaps(&again) < before { again } else { paths }
        }
    };
    Solution { paths, failed }
}
