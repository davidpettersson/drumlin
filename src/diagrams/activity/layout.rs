// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Structured-flow layout: measure the block tree from the leaves up, then place
//! it from the root down. Stage one sizes every block as extents about one axis
//! per lane. Stage two walks down the page and emits absolute geometry.
//!
//! A block's reach holds, per lane, how far it extends left and right of that
//! lane's axis. A flow keeps one axis per lane, so everything in one lane lines
//! up down the page, and side-by-side branches shift the axes for their contents.

use super::model::*;
use crate::core::geometry::{snap, snap_up, Point, Rect, GRID};
use crate::core::glyphs::{note_size_wrapped, NOTE_WRAP};
use crate::core::text::{width, wrap, Weight, FONT, SMALL};
use std::collections::BTreeMap;

const GAP: f64 = 32.0;
const ARM_GAP: f64 = 32.0;
const LANE_CROSS: f64 = 16.0;
const LOOP_GAP: f64 = 24.0;
const SIDE_CLEAR: f64 = 24.0;
const HALF: f64 = 16.0;
const BAR: f64 = 6.0;
const BAR_ROW: f64 = 8.0;
const LANE_PAD: f64 = 24.0;
const LANE_HEAD: f64 = 32.0;
const LANE_MIN: f64 = 144.0;
const TEXT_WRAP: f64 = 184.0;
const GUARD_WRAP: f64 = 136.0;
const GUARD_LINE: f64 = 14.0;
const NOTE_LINK: f64 = 16.0;
/// A risen note sits this far above the decision or bar it annotates, and its
/// near edge this far beside the axis, so its link is a short straight line
/// clear of the incoming flow (Drawing principles, Lines: note links may be
/// diagonal).
const RISE_GAP: f64 = 16.0;
const RISE_SIDE: f64 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeKind {
    Action,
    Send,
    Accept,
    Object,
    Start,
    Stop,
    End,
    Decision,
    Merge,
    Bar,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub rect: Rect,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub points: Vec<Point>,
}

#[derive(Clone, Debug)]
pub struct Label {
    pub rect: Rect,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct NoteBox {
    pub rect: Rect,
    pub text: String,
    pub wrap: f64,
}

#[derive(Clone, Debug)]
pub struct Band {
    pub rect: Rect,
    pub head: Rect,
    pub name: String,
}

#[derive(Debug, Default)]
pub struct Layout {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub labels: Vec<Label>,
    pub notes: Vec<NoteBox>,
    pub links: Vec<[Point; 2]>,
    pub bands: Vec<Band>,
}

/// Left and right extents about each lane's axis.
type Reach = BTreeMap<usize, (f64, f64)>;

fn widen(mut reach: Reach, lane: usize, left: f64, right: f64) -> Reach {
    let entry = reach.entry(lane).or_insert((0.0, 0.0));
    *entry = (entry.0.max(left), entry.1.max(right));
    reach
}

fn union(a: Reach, b: &Reach) -> Reach {
    b.iter().fold(a, |acc, (&lane, &(l, r))| widen(acc, lane, l, r))
}

#[derive(Clone, Debug)]
struct Guard {
    lines: Vec<String>,
    w: f64,
    h: f64,
}

fn guard_of(text: &str) -> Guard {
    let lines = wrap(&format!("[{text}]"), SMALL, GUARD_WRAP);
    let w = lines.iter().map(|l| width(l, SMALL, Weight::Regular)).fold(0.0, f64::max).ceil();
    let h = lines.len() as f64 * GUARD_LINE + 2.0;
    Guard { lines, w, h }
}

/// A note above a decision or a bar, with its own place in the opening band.
#[derive(Clone, Debug)]
struct RisenNote {
    text: String,
    size: (f64, f64),
    wrap: f64,
    left: bool,
}

#[derive(Clone, Debug)]
enum Shape {
    /// A note beside the node, and whether it goes on the left.
    Leaf { kind: NodeKind, lines: Vec<String>, size: (f64, f64), note: Option<(String, (f64, f64))>, left: bool },
    Split(Box<Split>),
    While(Box<Looping>),
    Repeat(Box<Looping>),
}

#[derive(Clone, Debug)]
struct Sized {
    shape: Shape,
    lane: usize,
    exit_lane: Option<usize>,
    reach: Reach,
    height: f64,
    /// A guard waiting on the flow that leaves this block, which needs room below.
    exit_guard_h: f64,
}

#[derive(Clone, Debug, Default)]
struct Seq {
    blocks: Vec<Sized>,
    gaps: Vec<f64>,
    reach: Reach,
    height: f64,
}

impl Seq {
    fn entry_lane(&self) -> Option<usize> {
        self.blocks.first().map(|b| b.lane)
    }
    fn exit_lane(&self) -> Option<usize> {
        self.blocks.last().and_then(|b| b.exit_lane)
    }
    /// The height of a guard left on this flow's outgoing line, if any.
    fn exit_guard_h(&self) -> f64 {
        self.blocks.last().map_or(0.0, |b| b.exit_guard_h)
    }
    /// The vertical room needed before a node below this flow, in lane `to`.
    fn tail_gap(&self, to: usize) -> f64 {
        let cross = if self.exit_lane().is_some_and(|lane| lane != to) { LANE_CROSS } else { 0.0 };
        GAP.max(snap_up(self.exit_guard_h() + 20.0, GRID)) + cross
    }
}

#[derive(Clone, Debug)]
struct SizedArm {
    seq: Seq,
    guard: Option<Guard>,
    offsets: BTreeMap<usize, f64>,
    top: f64,
    falls: bool,
    tail: Tail,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Closure {
    None,
    Merge,
    Join,
}

#[derive(Clone, Debug)]
struct Split {
    is_fork: bool,
    arms: Vec<SizedArm>,
    notes: Vec<RisenNote>,
    rise: f64,
    head_gap: f64,
    band: f64,
    close_gap: f64,
    closure: Closure,
}

#[derive(Clone, Debug)]
struct Looping {
    guard: Guard,
    body: Seq,
    rise: f64,
    notes: Vec<RisenNote>,
    head_gap: f64,
    tail_gap: f64,
    /// The lane and distance of the back edge's column, left of the whole body.
    back: (usize, f64),
    /// The lane and distance of the exit column, right of the whole body.
    exit: (usize, f64),
}

fn half(w: f64) -> f64 {
    snap_up(w / 2.0, GRID)
}

fn text_box(text: &str, extra: f64) -> (Vec<String>, f64, f64) {
    let lines = wrap(text, FONT, TEXT_WRAP);
    let w = lines.iter().map(|l| width(l, FONT, Weight::Bold)).fold(0.0, f64::max);
    let h = snap_up(lines.len() as f64 * 16.0 + 16.0, 16.0);
    let box_w = snap_up(w + 32.0 + extra, 16.0).max(80.0);
    (lines, box_w, h)
}

fn leaf(kind: &Kind) -> Option<(NodeKind, Vec<String>, f64, f64)> {
    Some(match kind {
        Kind::Start => (NodeKind::Start, Vec::new(), 16.0, 16.0),
        Kind::Stop => (NodeKind::Stop, Vec::new(), 16.0, 16.0),
        Kind::End => (NodeKind::End, Vec::new(), 16.0, 16.0),
        Kind::Action(t) => {
            let (lines, w, h) = text_box(t, 0.0);
            (NodeKind::Action, lines, w, h)
        }
        Kind::Object(t) => {
            let (lines, w, h) = text_box(t, 0.0);
            (NodeKind::Object, lines, w, h)
        }
        Kind::Send(t) => {
            let (lines, w, h) = text_box(t, 16.0);
            (NodeKind::Send, lines, w, h)
        }
        Kind::Accept(t) => {
            let (lines, w, h) = text_box(t, 16.0);
            (NodeKind::Accept, lines, w, h)
        }
        _ => return None,
    })
}

/// Which side of its flow a node's note goes: -1 left, 1 right. A note goes
/// outside the branch it is in, so it never widens the space between
/// branches (Drawing principles, Flow: keep branches close, notes go outside).
type Outside = i8;

fn size_leaf(stmt: &Stmt, node: (NodeKind, Vec<String>, f64, f64), outside: Outside) -> Sized {
    let (kind, lines, w, h) = node;
    let note = stmt.note.as_ref().map(|text| (text.clone(), note_size_wrapped(text, NOTE_WRAP)));
    let beside = half(w) + note.as_ref().map_or(0.0, |(_, s)| NOTE_LINK + s.0);
    let left = outside < 0 && note.is_some();
    let (l, r) = if left { (beside, half(w)) } else { (half(w), beside) };
    let height = note.as_ref().map_or(h, |(_, s)| h.max(s.1));
    let shape = Shape::Leaf { kind, lines, size: (w, h), note, left };
    Sized { shape, lane: stmt.lane, exit_lane: Some(stmt.lane), reach: widen(Reach::new(), stmt.lane, l, r), height, exit_guard_h: 0.0 }
}

fn size_seq(flow: &[Stmt], outside: Outside) -> Seq {
    let blocks: Vec<Sized> = flow.iter().map(|stmt| size_stmt(stmt, outside)).collect();
    let gaps: Vec<f64> = blocks
        .windows(2)
        .map(|w| {
            let cross = if w[0].exit_lane != Some(w[1].lane) { LANE_CROSS } else { 0.0 };
            GAP.max(snap_up(w[0].exit_guard_h + 20.0, GRID)) + cross
        })
        .collect();
    let height = blocks.iter().map(|b| b.height).sum::<f64>() + gaps.iter().sum::<f64>();
    let reach = blocks.iter().fold(Reach::new(), |acc, b| union(acc, &b.reach));
    Seq { blocks, gaps, reach, height }
}

fn risen_notes(stmt: &Stmt, subject: Option<&str>) -> (Vec<RisenNote>, f64) {
    let keyword_note = subject.map(|s| {
        // Exactly the keyword's width, so it takes a line to itself.
        let longest = s.split_whitespace().map(|w| width(w, SMALL, Weight::Regular)).fold(0.0, f64::max);
        let wrap_at = width("\u{ab}decisionInput\u{bb}", SMALL, Weight::Regular).max(longest) + 1.0;
        (format!("\u{ab}decisionInput\u{bb} {s}"), wrap_at, false)
    });
    let user_note = stmt.note.as_ref().map(|text| (text.clone(), NOTE_WRAP, subject.is_some()));
    let notes: Vec<RisenNote> = keyword_note
        .into_iter()
        .chain(user_note)
        .map(|(text, wrap, left)| RisenNote { size: note_size_wrapped(&text, wrap), text, wrap, left })
        .collect();
    let rise = notes.iter().map(|n| n.size.1).fold(0.0, f64::max);
    let total = if rise > 0.0 { rise + RISE_GAP } else { 0.0 };
    (notes, total)
}

fn note_reach(notes: &[RisenNote]) -> (f64, f64) {
    notes.iter().fold((0.0, 0.0), |(l, r), n| {
        let w = n.size.0 + RISE_SIDE;
        if n.left {
            (l.max(w), r)
        } else {
            (l, r.max(w))
        }
    })
}

/// How the arms sit in the home lane: `pivot` lies on the axis, and `order` runs
/// left to right. Three rules keep a straight flow straight: the one arm that
/// continues takes the axis, an empty arm yields it to a body beside it, and
/// otherwise a decision's middle arm takes it, the first declared of two, so the
/// flow passes straight through the diamond (Drawing principles, Lines). A fork
/// is not a decision: with an even count its arms stay symmetric about the axis.
fn arm_plan(arms: &[(Seq, bool)], home: usize, is_fork: bool) -> (Option<usize>, Vec<usize>) {
    let on_home = |i: usize| arms[i].0.entry_lane().is_none_or(|lane| lane == home);
    let falling: Vec<usize> = (0..arms.len()).filter(|&i| arms[i].1).collect();
    if falling.len() == 1 && on_home(falling[0]) {
        let pivot = falling[0];
        return (Some(pivot), std::iter::once(pivot).chain((0..arms.len()).filter(|&i| i != pivot)).collect());
    }
    let nonempty: Vec<usize> = (0..arms.len()).filter(|&i| !arms[i].0.blocks.is_empty()).collect();
    if arms.len() == 2 && nonempty.len() == 1 && on_home(nonempty[0]) {
        let pivot = nonempty[0];
        return (Some(pivot), vec![pivot, 1 - pivot]);
    }
    let at_home: Vec<usize> = (0..arms.len()).filter(|&i| on_home(i)).collect();
    if at_home.len() == 1 {
        let pivot = at_home[0];
        return (Some(pivot), std::iter::once(pivot).chain((0..arms.len()).filter(|&i| i != pivot)).collect());
    }
    let order: Vec<usize> = (0..arms.len()).collect();
    let middle = (arms.len().max(1) - 1) / 2;
    match (arms.len() % 2 == 1 || !is_fork) && on_home(middle) {
        true => (Some(middle), order),
        false => (None, order),
    }
}

fn arm_reach(arm: &(Seq, bool), guard: Option<&Guard>, home: usize, place: Tail) -> Reach {
    let lane = arm.0.entry_lane().unwrap_or(home);
    let base = widen(arm.0.reach.clone(), lane, 0.0, 0.0);
    match (guard, place) {
        (Some(g), Tail::Pivot) => widen(base, lane, 0.0, snap_up(g.w + 8.0, GRID)),
        (Some(g), Tail::Shared) => widen(base, lane, half(g.w), half(g.w)),
        _ => base,
    }
}

/// Where a side arm's guard goes: alone on its side it sits by the corner, at
/// the tail of the flow; sharing a run it sits over its own drop.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Tail {
    Pivot,
    Corner,
    Shared,
}

/// Which side of the home axis each arm leaves on: -1, 0 or 1.
fn sides_of(arms: &[(Seq, bool)], home: usize, pivot: Option<usize>, order: &[usize]) -> Vec<i8> {
    let at = |i: usize| order.iter().position(|&o| o == i).unwrap_or(0);
    let pivot_at = pivot.map(at);
    let half_count = order.len() / 2;
    (0..arms.len())
        .map(|i| match arms[i].0.entry_lane() {
            Some(lane) if lane < home => -1,
            Some(lane) if lane > home => 1,
            _ => match pivot_at {
                Some(p) => (at(i).cmp(&p)) as i8,
                None => if at(i) < half_count { -1 } else { 1 },
            },
        })
        .collect()
}

fn tails(sides: &[i8], pivot: Option<usize>, is_fork: bool) -> Vec<Tail> {
    let count = |side: i8| sides.iter().filter(|&&s| s == side).count();
    (0..sides.len())
        .map(|i| match (is_fork || pivot == Some(i) || sides[i] == 0, count(sides[i])) {
            (true, _) => Tail::Pivot,
            (false, 1) => Tail::Corner,
            (false, _) => Tail::Shared,
        })
        .collect()
}

/// Places arms outward from `edge`, each at least at its minimum distance.
fn walk(indices: &[usize], extents: &[(f64, f64)], minimum: &[f64], edge: f64, sign: f64) -> Vec<(usize, f64)> {
    indices
        .iter()
        .scan(edge, |edge, &i| {
            let (inner, outer) = if sign > 0.0 { extents[i] } else { (extents[i].1, extents[i].0) };
            let at = (*edge + inner).max(minimum[i]);
            *edge = at + outer + ARM_GAP;
            Some((i, sign * at))
        })
        .collect()
}

fn lane_offsets(lane: usize, home: usize, reaches: &[Reach], order: &[usize], pivot: Option<usize>, minimum: &[f64]) -> Vec<(usize, f64)> {
    let touching: Vec<usize> = order.iter().copied().filter(|&i| reaches[i].contains_key(&lane)).collect();
    let extents: Vec<(f64, f64)> = reaches.iter().map(|r| r.get(&lane).copied().unwrap_or((0.0, 0.0))).collect();
    let no_minimum = vec![0.0; reaches.len()];
    if lane != home || touching.is_empty() {
        let placed = walk(&touching, &extents, &no_minimum, 0.0, 1.0);
        let right = placed.last().map_or(0.0, |&(i, x)| x + extents[i].1);
        let left = placed.first().map_or(0.0, |&(i, x)| x - extents[i].0);
        let shift = snap_up(-(left + right) / 2.0, GRID);
        return placed.into_iter().map(|(i, x)| (i, x + shift)).collect();
    }
    let at_pivot = pivot.and_then(|p| touching.iter().position(|&i| i == p));
    match at_pivot {
        Some(at) => {
            let p = touching[at];
            let lefts: Vec<usize> = touching[..at].iter().rev().copied().collect();
            let rights: Vec<usize> = touching[at + 1..].to_vec();
            let left = walk(&lefts, &extents, minimum, extents[p].0 + ARM_GAP, -1.0);
            let right = walk(&rights, &extents, minimum, extents[p].1 + ARM_GAP, 1.0);
            [left, vec![(p, 0.0)], right].concat()
        }
        None => {
            let mid = touching.len() / 2;
            let (lefts, rights): (Vec<usize>, Vec<usize>) = (touching[..mid].iter().rev().copied().collect(), touching[mid..].to_vec());
            let first_left = lefts.first().map_or(0.0, |&i| extents[i].1 + ARM_GAP / 2.0);
            let first_right = rights.first().map_or(0.0, |&i| extents[i].0 + ARM_GAP / 2.0);
            let first_min = lefts.first().into_iter().chain(rights.first()).map(|&i| minimum[i]).fold(0.0, f64::max);
            let d = first_left.max(first_right).max(first_min);
            let forced = |list: &[usize], base: &[f64]| {
                let mut m = base.to_vec();
                if let Some(&first) = list.first() {
                    m[first] = m[first].max(d);
                }
                m
            };
            [walk(&lefts, &extents, &forced(&lefts, minimum), 0.0, -1.0), walk(&rights, &extents, &forced(&rights, minimum), 0.0, 1.0)].concat()
        }
    }
}

fn span_of(reach: &Reach) -> Option<(usize, usize)> {
    Some((*reach.keys().next()?, *reach.keys().next_back()?))
}

/// True when arms must not share rows, because a flow in one arm crosses lanes
/// that another occupies. Stacking them keeps every horizontal run clear of nodes.
fn must_stack(reaches: &[Reach]) -> bool {
    let spans: Vec<(usize, usize)> = reaches.iter().filter_map(span_of).collect();
    spans.iter().enumerate().any(|(i, a)| {
        spans.iter().skip(i + 1).any(|b| (a.0 != a.1 || b.0 != b.1) && a.0 <= b.1 && b.0 <= a.1)
    })
}

/// Room beside the opener for corner guards, for arms that leave the home lane.
fn corner_room(guards: &[Option<Guard>], tails: &[Tail], sides: &[i8]) -> (f64, f64) {
    guards.iter().zip(tails).zip(sides).fold((0.0, 0.0), |(l, r), ((g, tail), &side)| match (g, tail) {
        (Some(g), Tail::Corner) => {
            let room = snap_up(HALF + 6.0 + g.w + 8.0, GRID);
            if side < 0 {
                (room.max(l), r)
            } else {
                (l, room.max(r))
            }
        }
        _ => (l, r),
    })
}

fn size_split(stmt: &Stmt, is_fork: bool, bodies: Vec<(Option<&String>, &[Stmt])>, subject: Option<&str>, outside: Outside) -> Sized {
    let home = stmt.lane;
    let first: Vec<(Seq, bool)> = bodies.iter().map(|(_, body)| (size_seq(body, outside), falls(body))).collect();
    let guards: Vec<Option<Guard>> = bodies.iter().map(|(g, _)| g.map(|t| guard_of(t))).collect();
    let (pivot, order) = arm_plan(&first, home, is_fork);
    let sides = sides_of(&first, home, pivot, &order);
    // Notes in a side arm go on its outer side. An arm on the axis turns its
    // notes away from side arms that all leave on one side, and otherwise keeps
    // the side its flow already has.
    let beside: Vec<i8> = sides.iter().copied().filter(|&s| s != 0).collect();
    let axis_outside = match beside.first() {
        Some(&s) if beside.iter().all(|&b| b == s) => -s,
        _ => outside,
    };
    let sized: Vec<(Seq, bool)> = bodies
        .iter()
        .zip(&sides)
        .zip(first)
        .map(|(((_, body), &side), arm)| match if side == 0 { axis_outside } else { side } {
            side if side == outside => arm,
            side => (size_seq(body, side), arm.1),
        })
        .collect();
    let tail_places = tails(&sides, pivot, is_fork);
    let reaches: Vec<Reach> = sized.iter().zip(&guards).zip(&tail_places).map(|((arm, g), &place)| arm_reach(arm, g.as_ref(), home, place)).collect();
    let minimum: Vec<f64> = guards
        .iter()
        .zip(&tail_places)
        .map(|(g, &place)| match (is_fork, g, place) {
            (true, _, _) => 0.0,
            (false, Some(g), Tail::Corner) => snap_up(HALF + 6.0 + g.w + 16.0, GRID),
            (false, Some(g), _) => HALF + SIDE_CLEAR + half(g.w),
            (false, None, _) => HALF + SIDE_CLEAR,
        })
        .collect();
    let lanes: Vec<usize> = reaches.iter().flat_map(|r| r.keys().copied()).chain([home]).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    let positions: BTreeMap<usize, Vec<(usize, f64)>> = lanes.iter().map(|&lane| (lane, lane_offsets(lane, home, &reaches, &order, pivot, &minimum))).collect();
    let offsets = |arm: usize| -> BTreeMap<usize, f64> {
        positions.iter().filter_map(|(&lane, list)| list.iter().find(|(i, _)| *i == arm).map(|&(_, x)| (lane, x))).collect()
    };
    let stack = must_stack(&reaches);
    let tops: Vec<f64> = sized
        .iter()
        .scan(0.0, |y, (seq, _)| {
            let top = *y;
            if stack {
                *y += seq.height + GAP;
            }
            Some(top)
        })
        .collect();
    let band = sized.iter().zip(&tops).map(|((seq, _), top)| top + seq.height).fold(0.0, f64::max);
    let arms: Vec<SizedArm> = sized
        .iter()
        .enumerate()
        .map(|(i, (seq, falls))| SizedArm { seq: seq.clone(), guard: guards[i].clone(), offsets: offsets(i), top: tops[i], falls: *falls, tail: tail_places[i] })
        .collect();
    let falling: Vec<usize> = (0..arms.len()).filter(|&i| arms[i].falls).collect();
    let closure = match (falling.len(), is_fork) {
        (0 | 1, _) => Closure::None,
        (_, true) => Closure::Join,
        (_, false) => Closure::Merge,
    };
    let (notes, rise) = risen_notes(stmt, subject);
    let pivot_guard_h = pivot.and_then(|p| guards[p].as_ref().map(|g| g.h)).unwrap_or(0.0);
    let head_gap = GAP.max(snap_up(pivot_guard_h + 20.0, GRID));
    let close_room = arms.iter().filter(|arm| arm.falls).map(|arm| arm.seq.tail_gap(home)).fold(GAP, f64::max);
    let (close_gap, close_h) = match closure {
        Closure::None => (0.0, 0.0),
        Closure::Merge => (close_room, 2.0 * HALF),
        Closure::Join => (close_room, BAR_ROW),
    };
    let open_h = if is_fork { BAR_ROW } else { 2.0 * HALF };
    let height = rise + open_h + head_gap + band + close_gap + close_h;
    let (note_l, note_r) = note_reach(&notes);
    let opener_half = if is_fork { GRID } else { HALF };
    let (guard_l, guard_r) = corner_room(&guards, &tail_places, &sides);
    let reach = arms.iter().zip(&reaches).fold(widen(Reach::new(), home, opener_half.max(note_l).max(guard_l), opener_half.max(note_r).max(guard_r)), |acc, (arm, reach)| {
        reach.iter().fold(acc, |acc, (&lane, &(l, r))| {
            let x = arm.offsets.get(&lane).copied().unwrap_or(0.0);
            widen(acc, lane, (l - x).max(0.0), (r + x).max(0.0))
        })
    });
    let exit_lane = match (falling.as_slice(), closure) {
        ([], _) => None,
        ([one], Closure::None) => arms[*one].seq.exit_lane().or(Some(home)),
        _ => Some(home),
    };
    let exit_guard_h = match falling.as_slice() {
        [one] if arms[*one].seq.blocks.is_empty() => arms[*one].guard.as_ref().map_or(0.0, |g| g.h),
        _ => 0.0,
    };
    Sized {
        shape: Shape::Split(Box::new(Split { is_fork, arms, notes, rise, head_gap, band, close_gap, closure })),
        lane: home,
        exit_lane,
        reach,
        height,
        exit_guard_h,
    }
}

/// A column beside a loop body, in the outermost lane the body reaches on that
/// side, so the line it carries can never run through a bar spanning the body.
fn back_column(reach: &Reach, home: usize, clear: f64) -> (usize, f64) {
    let lane = reach.keys().next().copied().map_or(home, |lane| lane.min(home));
    let l = reach.get(&lane).map_or(0.0, |r| r.0);
    (lane, if lane == home { l.max(clear) + LOOP_GAP } else { l + LOOP_GAP })
}

fn exit_column(reach: &Reach, home: usize, clear: f64) -> (usize, f64) {
    let lane = reach.keys().next_back().copied().map_or(home, |lane| lane.max(home));
    let r = reach.get(&lane).map_or(0.0, |r| r.1);
    (lane, if lane == home { r.max(clear) + LOOP_GAP } else { r + LOOP_GAP })
}

fn extend(reach: Reach, (lane, distance): (usize, f64), left: bool) -> Reach {
    match left {
        true => widen(reach, lane, distance, 0.0),
        false => widen(reach, lane, 0.0, distance),
    }
}

fn size_while(stmt: &Stmt, guard: &str, body: &[Stmt], outside: Outside) -> Sized {
    let home = stmt.lane;
    let body = size_seq(body, outside);
    let g = guard_of("else");
    let body_guard = guard_of(guard);
    let (notes, rise) = risen_notes(stmt, None);
    let (back, exit) = (back_column(&body.reach, home, HALF), exit_column(&body.reach, home, HALF + snap_up(g.w + 16.0, GRID).max(snap_up(body_guard.w + 16.0, GRID))));
    let tail_gap = if body.exit_guard_h() > 0.0 { snap_up(body.exit_guard_h() + 8.0, GRID) } else { 16.0 };
    let cross = if body.entry_lane().is_some_and(|lane| lane != home) { LANE_CROSS } else { 0.0 };
    let head_gap = GAP.max(snap_up(body_guard.h + 20.0, GRID)) + cross;
    let height = rise + 2.0 * HALF + LOOP_GAP + 2.0 * HALF + head_gap + body.height + tail_gap + 16.0;
    let (note_l, note_r) = note_reach(&notes);
    let extended = extend(extend(body.reach.clone(), back, true), exit, false);
    let reach = widen(extended, home, HALF.max(note_l), (HALF + snap_up(g.w + 16.0, GRID)).max(note_r).max(snap_up(body_guard.w + 8.0, GRID)));
    let shape = Shape::While(Box::new(Looping { guard: body_guard, body, rise, notes, head_gap, tail_gap, back, exit }));
    Sized { shape, lane: home, exit_lane: Some(home), reach, height, exit_guard_h: 0.0 }
}

fn size_repeat(stmt: &Stmt, body: &[Stmt], guard: &str, outside: Outside) -> Sized {
    let home = stmt.lane;
    let body = size_seq(body, outside);
    let else_guard = guard_of("else");
    let exit_guard = guard_of(guard);
    let (notes, rise) = risen_notes(stmt, None);
    let (_, br) = body.reach.get(&home).copied().unwrap_or((0.0, 0.0));
    let back = back_column(&body.reach, home, HALF + snap_up(else_guard.w + 16.0, GRID));
    let tail_gap = body.tail_gap(home);
    let height = rise + 2.0 * HALF + GAP + body.height + tail_gap + 2.0 * HALF;
    let (note_l, note_r) = note_reach(&notes);
    let reach = widen(extend(body.reach.clone(), back, true), home, (HALF + snap_up(else_guard.w + 16.0, GRID)).max(note_l), br.max(HALF).max(note_r).max(snap_up(exit_guard.w + 8.0, GRID)));
    let shape = Shape::Repeat(Box::new(Looping { guard: exit_guard.clone(), body, rise, notes, head_gap: GAP, tail_gap, back, exit: (home, 0.0) }));
    Sized { shape, lane: home, exit_lane: Some(home), reach, height, exit_guard_h: exit_guard.h }
}

fn size_stmt(stmt: &Stmt, outside: Outside) -> Sized {
    if let Some(node) = leaf(&stmt.kind) {
        return size_leaf(stmt, node, outside);
    }
    match &stmt.kind {
        Kind::Decision { subject, arms } => {
            let bodies = arms.iter().map(|arm| (arm.guard.as_ref(), arm.body.as_slice())).collect();
            size_split(stmt, false, bodies, subject.as_deref(), outside)
        }
        Kind::Fork(branches) => size_split(stmt, true, branches.iter().map(|b| (None, b.as_slice())).collect(), None, outside),
        Kind::While { guard, body } => size_while(stmt, guard, body, outside),
        Kind::Repeat { body, guard } => size_repeat(stmt, body, guard, outside),
        _ => unreachable!("leaf kinds return above"),
    }
}

/// One axis per lane.
type Origins = BTreeMap<usize, f64>;

#[derive(Clone, Debug)]
struct Port {
    x: f64,
    y: f64,
    /// The top of the block's band, above any note that rises over the entry.
    band: f64,
    span: Option<(f64, f64)>,
}

#[derive(Clone, Debug)]
struct Exit {
    path: Vec<Point>,
    span: Option<(f64, f64)>,
    /// The left corner of the merge the flow leaves, when no branch rejoins
    /// there: a loop's return line leaves by it (Drawing principles, Lines:
    /// take a return line around the outside).
    corner: Option<Point>,
}

#[derive(Default)]
struct Placed {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    labels: Vec<Label>,
    notes: Vec<NoteBox>,
    links: Vec<[Point; 2]>,
    entry: Option<Port>,
    exit: Option<Exit>,
}

fn concat(mut a: Placed, b: Placed) -> Placed {
    a.nodes.extend(b.nodes);
    a.edges.extend(b.edges);
    a.labels.extend(b.labels);
    a.notes.extend(b.notes);
    a.links.extend(b.links);
    a.entry = a.entry.or(b.entry);
    a.exit = b.exit;
    a
}

/// Drops repeated points and points on a straight run, so a path has only corners.
fn simplify(points: Vec<Point>) -> Vec<Point> {
    let deduped: Vec<Point> = points.into_iter().fold(Vec::new(), |mut out: Vec<Point>, p| {
        if out.last() != Some(&p) {
            out.push(p);
        }
        out
    });
    if deduped.len() < 3 {
        return deduped;
    }
    let inner = deduped.windows(3).filter(|w| !((w[0].0 == w[1].0 && w[1].0 == w[2].0) || (w[0].1 == w[1].1 && w[1].1 == w[2].1))).map(|w| w[1]);
    std::iter::once(deduped[0]).chain(inner).chain(std::iter::once(deduped[deduped.len() - 1])).collect()
}

fn edge(points: Vec<Point>) -> Edge {
    Edge { points: simplify(points) }
}

fn inside(span: Option<(f64, f64)>, x: f64) -> bool {
    span.is_some_and(|(a, b)| x >= a && x <= b)
}

/// The route from a dangling exit to a port. Between bars a flow drops straight
/// where it can, and otherwise it steps sideways in the row above the target.
fn connect(exit: &Exit, entry: &Port) -> Edge {
    let tail = exit.path[exit.path.len() - 1];
    let start_x = if inside(exit.span, entry.x) { entry.x } else { tail.0 };
    let end_x = if inside(entry.span, start_x) { start_x } else { entry.x };
    let lead: Vec<Point> = if start_x == tail.0 { exit.path.clone() } else { vec![(start_x, tail.1)] };
    let tail_y = lead[lead.len() - 1].1;
    let turn = if start_x == end_x {
        Vec::new()
    } else {
        let mid = snap(entry.band - GAP / 2.0).max(tail_y + GRID);
        vec![(start_x, mid), (end_x, mid)]
    };
    edge([lead, turn, vec![(end_x, entry.y)]].concat())
}

fn place_seq(seq: &Seq, origins: &Origins, top: f64, stacked: bool) -> Placed {
    let tops: Vec<f64> = seq
        .blocks
        .iter()
        .enumerate()
        .scan(top, |y, (i, block)| {
            let at = *y;
            *y += block.height + seq.gaps.get(i).copied().unwrap_or(0.0);
            Some(at)
        })
        .collect();
    seq.blocks.iter().zip(tops).fold(Placed::default(), |acc, (block, y)| {
        let next = place_block(block, origins, y, stacked);
        let link: Vec<Edge> = match (&acc.exit, &next.entry) {
            (Some(exit), Some(entry)) => vec![connect(exit, entry)],
            _ => Vec::new(),
        };
        let mut joined = concat(acc, next);
        joined.edges.extend(link);
        joined
    })
}

fn origin(origins: &Origins, lane: usize) -> f64 {
    origins.get(&lane).copied().unwrap_or(0.0)
}

/// `stacked` is true inside an arm that shares its rows with no sibling, where a
/// bar must stay in its own lane or it would lie across the sibling's lines.
fn place_block(block: &Sized, origins: &Origins, y: f64, stacked: bool) -> Placed {
    let axis = origin(origins, block.lane);
    match &block.shape {
        Shape::Leaf { kind, lines, size, note, left } => place_leaf(*kind, lines, *size, note.as_ref(), *left, axis, y, block.height),
        Shape::Split(split) => place_split(split, block, origins, axis, y, stacked),
        Shape::While(looping) => place_while(looping, origins, axis, y, stacked),
        Shape::Repeat(looping) => place_repeat(looping, origins, axis, y, stacked),
    }
}

#[allow(clippy::too_many_arguments)]
fn place_leaf(kind: NodeKind, lines: &[String], (w, h): (f64, f64), note: Option<&(String, (f64, f64))>, left: bool, axis: f64, y: f64, band: f64) -> Placed {
    let top = y + (band - h) / 2.0;
    let rect = Rect { x: axis - half(w), y: top, w: half(w) * 2.0, h };
    let cy = top + h / 2.0;
    let (notes, links): (Vec<NoteBox>, Vec<[Point; 2]>) = note
        .map(|(text, s)| {
            let (x, link) = if left { (axis - half(w) - NOTE_LINK - s.0, [(axis - half(w), cy), (axis - half(w) - NOTE_LINK, cy)]) } else { (axis + half(w) + NOTE_LINK, [(axis + half(w), cy), (axis + half(w) + NOTE_LINK, cy)]) };
            let r = Rect { x, y: cy - s.1 / 2.0, w: s.0, h: s.1 };
            (vec![NoteBox { rect: r, text: text.clone(), wrap: NOTE_WRAP }], vec![link])
        })
        .unwrap_or_default();
    let terminal = matches!(kind, NodeKind::Stop | NodeKind::End);
    Placed {
        nodes: vec![Node { kind, rect, lines: lines.to_vec() }],
        notes,
        links,
        entry: Some(Port { x: axis, y: top, band: y, span: None }),
        exit: if terminal { None } else { Some(Exit { path: vec![(axis, top + h)], span: None, corner: None }) },
        ..Placed::default()
    }
}

fn beside_vertical(g: &Guard, x: f64, y: f64) -> Label {
    Label { rect: Rect { x: x + 6.0, y: y + 4.0, w: g.w, h: g.h }, lines: g.lines.clone() }
}

fn above_at(g: &Guard, x: f64, y: f64) -> Label {
    Label { rect: Rect { x: x - g.w / 2.0, y: y - g.h - 3.0, w: g.w, h: g.h }, lines: g.lines.clone() }
}

/// Notes risen above a diamond or a bar whose top is at `target`. A link runs
/// straight from the note's nearest corner to the middle of the diamond's
/// upper edge on its side, at right angles to it, or straight down to the bar.
fn risen_marks(notes: &[RisenNote], axis: f64, y: f64, rise: f64, target: f64, diamond: bool) -> (Vec<NoteBox>, Vec<[Point; 2]>) {
    notes
        .iter()
        .map(|n| {
            let side = if n.left { -1.0 } else { 1.0 };
            let x = if n.left { axis - RISE_SIDE - n.size.0 } else { axis + RISE_SIDE };
            let top = y + rise - RISE_GAP - n.size.1;
            let corner = (axis + side * RISE_SIDE, top + n.size.1);
            let end = if diamond { (axis + side * HALF / 2.0, target + HALF / 2.0) } else { (corner.0, target) };
            let boxed = NoteBox { rect: Rect { x, y: top, w: n.size.0, h: n.size.1 }, text: n.text.clone(), wrap: n.wrap };
            (boxed, [corner, end])
        })
        .unzip()
}

fn diamond(kind: NodeKind, axis: f64, top: f64) -> Node {
    Node { kind, rect: Rect { x: axis - HALF, y: top, w: 2.0 * HALF, h: 2.0 * HALF }, lines: Vec::new() }
}

/// Where a side arm meets the opener's row: its corner, then along to its drop.
fn side_start(axis: f64, x: f64, cy: f64) -> Vec<Point> {
    let corner = if x < axis { axis - HALF } else { axis + HALF };
    vec![(corner, cy), (x, cy)]
}

/// A bar's extent over these positions, with a little overhang at each end.
fn spread(xs: &[f64]) -> (f64, f64) {
    (xs.iter().copied().fold(f64::MAX, f64::min) - HALF, xs.iter().copied().fold(f64::MIN, f64::max) + HALF)
}

fn place_split(split: &Split, block: &Sized, origins: &Origins, axis: f64, y: f64, stacked: bool) -> Placed {
    let open_top = y + split.rise;
    let open_h = if split.is_fork { BAR_ROW } else { 2.0 * HALF };
    let arms_top = open_top + open_h + split.head_gap;
    let inner_stacked = stacked || split.arms.iter().any(|arm| arm.top > 0.0);
    let placed_arms: Vec<(Placed, f64)> = split
        .arms
        .iter()
        .map(|arm| {
            let child: Origins = origins.iter().map(|(&lane, &x)| (lane, x + arm.offsets.get(&lane).copied().unwrap_or(0.0))).collect();
            let p = place_seq(&arm.seq, &child, arms_top + arm.top, inner_stacked);
            let home_x = axis + arm.offsets.get(&block.lane).copied().unwrap_or(0.0);
            let x = p.entry.as_ref().map_or(home_x, |e| e.x);
            (p, x)
        })
        .collect();
    let at_home = |arm: &SizedArm| arm.seq.entry_lane().is_none_or(|lane| lane == block.lane);
    let spanned = |x: f64, arm: &SizedArm| !stacked || at_home(arm) || x == axis;
    let xs: Vec<f64> = placed_arms.iter().zip(&split.arms).filter(|((_, x), arm)| spanned(*x, arm)).map(|((_, x), _)| *x).chain([axis]).collect();
    let bar_span = spread(&xs);
    let home_left = axis - block.reach[&block.lane].0;
    let home_right = axis + block.reach[&block.lane].1;
    let home_lane = |x: f64| x >= home_left && x <= home_right;
    let cy = open_top + HALF;
    let (notes, links) = risen_marks(&split.notes, axis, y, split.rise, open_top, !split.is_fork);
    let opener = match split.is_fork {
        true => Node { kind: NodeKind::Bar, rect: Rect { x: bar_span.0, y: open_top, w: bar_span.1 - bar_span.0, h: BAR }, lines: Vec::new() },
        false => diamond(NodeKind::Decision, axis, open_top),
    };
    let opener_bottom = if split.is_fork { open_top + BAR } else { open_top + 2.0 * HALF };
    // Each arm contributes an optional dangling exit once its entry edge is drawn.
    let wired: Vec<(Placed, Option<Exit>)> = split
        .arms
        .iter()
        .zip(placed_arms)
        .map(|(arm, (mut p, x))| {
            let exit = p.exit.take();
            let (lead, label) = if split.is_fork {
                match spanned(x, arm) {
                    true => (vec![(x, opener_bottom)], None),
                    false => {
                        // Away from the bar at once, so the drop misses other arms.
                        let row = snap(opener_bottom) + GRID;
                        (vec![(axis, opener_bottom), (axis, row), (x, row)], None)
                    }
                }
            } else if x == axis {
                (vec![(x, opener_bottom)], arm.guard.as_ref().map(|g| beside_vertical(g, x, opener_bottom)))
            } else {
                let label = arm.guard.as_ref().map(|g| match arm.tail {
                    Tail::Corner if x > axis => Label { rect: Rect { x: axis + HALF + 6.0, y: cy - g.h - 3.0, w: g.w, h: g.h }, lines: g.lines.clone() },
                    Tail::Corner => Label { rect: Rect { x: axis - HALF - 6.0 - g.w, y: cy - g.h - 3.0, w: g.w, h: g.h }, lines: g.lines.clone() },
                    _ => above_at(g, x, cy),
                });
                (side_start(axis, x, cy), label)
            };
            p.labels.extend(label);
            let exit = match &p.entry {
                Some(entry) => {
                    p.edges.push(connect(&Exit { path: lead, span: None, corner: None }, entry));
                    exit
                }
                None => arm.falls.then_some(Exit { path: lead, span: None, corner: None }),
            };
            (p, exit)
        })
        .collect();
    let bottom = arms_top + split.band;
    let close_y = bottom + split.close_gap;
    let exits: Vec<Exit> = wired.iter().filter_map(|(_, e)| e.clone()).collect();
    let (closer, close_edges, exit): (Vec<Node>, Vec<Edge>, Option<Exit>) = match split.closure {
        Closure::None => (Vec::new(), Vec::new(), exits.into_iter().next()),
        Closure::Merge => {
            let my = close_y + HALF;
            let edges = exits
                .iter()
                .map(|e| {
                    let x = e.path[e.path.len() - 1].0;
                    let tail: Vec<Point> = if x == axis {
                        vec![(axis, close_y)]
                    } else {
                        let corner = if x < axis { axis - HALF } else { axis + HALF };
                        vec![(x, my), (corner, my)]
                    };
                    edge([e.path.clone(), tail].concat())
                })
                .collect();
            let left_free = exits.iter().all(|e| e.path[e.path.len() - 1].0 >= axis);
            let corner = left_free.then_some((axis - HALF, my));
            (vec![diamond(NodeKind::Merge, axis, close_y)], edges, Some(Exit { path: vec![(axis, close_y + 2.0 * HALF)], span: None, corner }))
        }
        Closure::Join => {
            let joining: Vec<f64> = exits.iter().map(|e| e.path[e.path.len() - 1].0).filter(|&x| !stacked || home_lane(x)).chain([axis]).collect();
            let span = spread(&joining);
            let target = Port { x: axis, y: close_y, band: close_y, span: Some(span) };
            let edges = exits.iter().map(|e| connect(e, &target));
            let bar = Node { kind: NodeKind::Bar, rect: Rect { x: span.0, y: close_y, w: span.1 - span.0, h: BAR }, lines: Vec::new() };
            (vec![bar], edges.collect(), Some(Exit { path: vec![(axis, close_y + BAR)], span: Some(span), corner: None }))
        }
    };
    let opened = Placed { nodes: vec![opener], notes, links, entry: Some(Port { x: axis, y: open_top, band: y, span: split.is_fork.then_some(bar_span) }), ..Placed::default() };
    let body = wired.into_iter().fold(opened, |acc, (p, _)| {
        let entry = acc.entry.clone();
        Placed { entry, ..concat(acc, p) }
    });
    let mut done = body;
    done.nodes.extend(closer);
    done.edges.extend(close_edges);
    done.exit = exit;
    done
}

fn place_while(l: &Looping, origins: &Origins, axis: f64, y: f64, stacked: bool) -> Placed {
    let merge_top = y + l.rise;
    let decision_top = merge_top + 2.0 * HALF + LOOP_GAP;
    let decision_bottom = decision_top + 2.0 * HALF;
    let body_top = decision_bottom + l.head_gap;
    let cm = merge_top + HALF;
    let cd = decision_top + HALF;
    let body = place_seq(&l.body, origins, body_top, stacked);
    let (body_exit, body_entry) = (body.exit.clone().expect("a loop body continues"), body.entry.clone().expect("a loop body has a step"));
    let (back_y, exit_y) = (body_top + l.body.height + l.tail_gap, body_top + l.body.height + l.tail_gap + 16.0);
    let (x_back, x_exit) = (origin(origins, l.back.0) - l.back.1, origin(origins, l.exit.0) + l.exit.1);
    let xe = body_exit.path[body_exit.path.len() - 1].0;
    // A body that ends in a merge with a free left corner returns from that
    // corner, along the side it returns by, instead of dropping out of the
    // bottom and turning back.
    let back = match body_exit.corner {
        Some(c) if body_exit.path.len() == 1 && x_back < c.0 => edge(vec![c, (x_back, c.1), (x_back, cm), (axis - HALF, cm)]),
        _ => edge([body_exit.path.clone(), vec![(xe, back_y), (x_back, back_y), (x_back, cm), (axis - HALF, cm)]].concat()),
    };
    let test = connect(&Exit { path: vec![(axis, decision_bottom)], span: None, corner: None }, &body_entry);
    let in_loop = edge(vec![(axis, merge_top + 2.0 * HALF), (axis, decision_top)]);
    let else_guard = guard_of("else");
    let label_x = axis + HALF + 6.0;
    let labels = vec![
        beside_vertical(&l.guard, axis, decision_bottom),
        Label { rect: Rect { x: label_x, y: cd - else_guard.h - 3.0, w: else_guard.w, h: else_guard.h }, lines: else_guard.lines.clone() },
    ];
    let exit = Exit { path: vec![(axis + HALF, cd), (x_exit, cd), (x_exit, exit_y), (axis, exit_y)], span: None, corner: None };
    let (notes, links) = risen_marks(&l.notes, axis, y, l.rise, merge_top, true);
    let mut out = concat(
        Placed {
            nodes: vec![diamond(NodeKind::Merge, axis, merge_top), diamond(NodeKind::Decision, axis, decision_top)],
            labels,
            notes,
            links,
            edges: vec![back, test, in_loop],
            entry: Some(Port { x: axis, y: merge_top, band: y, span: None }),
            ..Placed::default()
        },
        body,
    );
    out.entry = Some(Port { x: axis, y: merge_top, band: y, span: None });
    out.exit = Some(exit);
    out
}

fn place_repeat(l: &Looping, origins: &Origins, axis: f64, y: f64, stacked: bool) -> Placed {
    let merge_top = y + l.rise;
    let body_top = merge_top + 2.0 * HALF + GAP;
    let decision_top = body_top + l.body.height + l.tail_gap;
    let decision_bottom = decision_top + 2.0 * HALF;
    let cm = merge_top + HALF;
    let cd = decision_top + HALF;
    let body = place_seq(&l.body, origins, body_top, stacked);
    let (body_exit, body_entry) = (body.exit.clone().expect("a loop body continues"), body.entry.clone().expect("a loop body has a step"));
    let x_back = origin(origins, l.back.0) - l.back.1;
    let back = edge(vec![(axis - HALF, cd), (x_back, cd), (x_back, cm), (axis - HALF, cm)]);
    let down = connect(&Exit { path: vec![(axis, merge_top + 2.0 * HALF)], span: None, corner: None }, &body_entry);
    let up = connect(&body_exit, &Port { x: axis, y: decision_top, band: decision_top, span: None });
    let else_guard = guard_of("else");
    let labels = vec![
        beside_vertical(&l.guard, axis, decision_bottom),
        Label { rect: Rect { x: axis - HALF - 6.0 - else_guard.w, y: cd - else_guard.h - 3.0, w: else_guard.w, h: else_guard.h }, lines: else_guard.lines.clone() },
    ];
    let (notes, links) = risen_marks(&l.notes, axis, y, l.rise, merge_top, true);
    let mut out = concat(
        Placed {
            nodes: vec![diamond(NodeKind::Merge, axis, merge_top), diamond(NodeKind::Decision, axis, decision_top)],
            labels,
            notes,
            links,
            edges: vec![back, down, up],
            entry: Some(Port { x: axis, y: merge_top, band: y, span: None }),
            ..Placed::default()
        },
        body,
    );
    out.entry = Some(Port { x: axis, y: merge_top, band: y, span: None });
    out.exit = Some(Exit { path: vec![(axis, decision_bottom)], span: None, corner: None });
    out
}

/// Lane columns: each is as wide as its reach, or its name, with content centred.
fn lane_columns(d: &Diagram, reach: &Reach) -> (Vec<(f64, f64)>, Origins) {
    let spans: Vec<(f64, f64, f64)> = d
        .lanes
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let (l, r) = reach.get(&i).copied().unwrap_or((0.0, 0.0));
            let name_w = snap_up(width(name, FONT, Weight::Bold) + 24.0, 2.0 * GRID);
            let w = (l + r + 2.0 * LANE_PAD).max(LANE_MIN).max(name_w);
            let w = snap_up(w, 2.0 * GRID);
            (l, r, w)
        })
        .collect();
    let (columns, origins, _) = spans.iter().enumerate().fold((Vec::new(), Origins::new(), 0.0), |(mut cols, mut origins, x), (i, &(l, r, w))| {
        cols.push((x, w));
        origins.insert(i, x + snap((w - (l + r)) / 2.0) + l);
        (cols, origins, x + w)
    });
    (columns, origins)
}

pub fn layout(d: &Diagram) -> Layout {
    let seq = size_seq(&d.flow, 1);
    if d.lanes.is_empty() {
        let (left, _) = seq.reach.get(&0).copied().unwrap_or((0.0, 0.0));
        let origins: Origins = [(0, left)].into_iter().collect();
        let placed = place_seq(&seq, &origins, 0.0, false);
        return Layout { nodes: placed.nodes, edges: placed.edges, labels: placed.labels, notes: placed.notes, links: placed.links, bands: Vec::new() };
    }
    let (columns, origins) = lane_columns(d, &seq.reach);
    let top = LANE_HEAD + LANE_PAD;
    let placed = place_seq(&seq, &origins, top, false);
    let height = top + seq.height + LANE_PAD;
    let bands = d
        .lanes
        .iter()
        .zip(columns)
        .map(|(name, (x, w))| Band {
            rect: Rect { x, y: 0.0, w, h: height },
            head: Rect { x, y: 0.0, w, h: LANE_HEAD },
            name: name.clone(),
        })
        .collect();
    Layout { nodes: placed.nodes, edges: placed.edges, labels: placed.labels, notes: placed.notes, links: placed.links, bands }
}
