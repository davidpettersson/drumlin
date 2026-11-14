// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! The timeline layout. Three pure stages: `steps` resolves execution depth into
//! a flat tree of drawing steps, `columns` turns label and note widths into
//! lifeline positions, and `rows` walks the steps down the page, placing each one
//! on the 8px grid. Everything is deterministic; no hashing affects the output.

use super::model::*;
use crate::core::geometry::{snap_up, Point, Rect, Side, GRID};
use crate::core::glyphs::{note_size, note_size_wrapped, NOTE_WRAP};
use crate::core::svg::Anchor;
use crate::core::text::{keyword_width, width, wrap, Weight, FONT, SMALL};

pub const LINE: f64 = 15.0;
const BAR_HALF: f64 = 6.0;
const NEST: f64 = 6.0;
const LOOP_REACH: f64 = 26.0;
const LOOP_HEIGHT: f64 = 16.0;
const LOOP_LABEL_GAP: f64 = 6.0;
const STUB: f64 = 64.0;
const DOT_RADIUS: f64 = 4.0;
/// How far a label keeps from the tip of the arrowhead it points at: the 10px
/// head and 4px of air.
const HEAD_CLEAR: f64 = 14.0;
/// The room a boundary stub needs beyond its label: the dot, the arrowhead and
/// air on both sides of the label (17.4.4.1, Figure 17.11 lost and found).
const STUB_ENDS: f64 = 28.0;
const LABEL_WRAP: f64 = 240.0;
const SELF_WRAP: f64 = 168.0;
/// The most a single label may widen the gap beside its sender.
const LONG_LABEL: f64 = 200.0;
const NAME_WRAP: f64 = 132.0;
const FRAME_INSET: f64 = 12.0;
pub const TAB_HEIGHT: f64 = 22.0;
pub const REF_TAB_HEIGHT: f64 = 20.0;
const TAB_SLANT: f64 = 8.0;

/// The widest a bar group reaches from its lifeline, either side.
#[derive(Clone, Copy, Debug, Default)]
struct Reach {
    left: f64,
    right: f64,
}

/// A message with its execution depths resolved.
#[derive(Clone, Debug)]
pub struct ArrowStep {
    message: Message,
    from_depth: usize,
    to_depth: usize,
    /// True when the receiver gets a new execution starting at the arrow.
    opens: bool,
}

#[derive(Clone, Debug)]
enum Close {
    /// The bar ends at the reply drawn just above.
    AtReply,
    /// The bar ends this far below the cursor.
    After(f64),
}

#[derive(Clone, Debug)]
struct OperandSteps {
    guard: Option<String>,
    /// The lifeline of the first event, and its depth there, for the guard.
    first_event: Option<(usize, usize)>,
    steps: Vec<Step>,
}

#[derive(Clone, Debug)]
enum Step {
    Arrow(ArrowStep),
    Close(usize, Close),
    Destroy(usize),
    Frame(Operator, Option<String>, Vec<OperandSteps>),
    Ref(usize, usize, String),
    Note(NotePlace, Vec<usize>, String),
}

fn first_event(step: &Step) -> Option<(usize, usize)> {
    match step {
        Step::Arrow(a) => match a.message.from {
            End::Life(l) => Some((l, a.from_depth)),
            End::Boundary(_) => match a.message.to {
                End::Life(l) => Some((l, a.to_depth.saturating_sub(usize::from(a.opens)))),
                End::Boundary(_) => None,
            },
        },
        Step::Close(..) => None,
        Step::Destroy(l) => Some((*l, 0)),
        Step::Frame(_, _, operands) => operands.iter().find_map(|o| o.first_event),
        Step::Ref(lo, _, _) => Some((*lo, 0)),
        Step::Note(NotePlace::Over(list), _, _) => list.first().map(|&l| (l, 0)),
        Step::Note(NotePlace::LeftOf(l) | NotePlace::RightOf(l), _, _) => Some((*l, 0)),
    }
}

fn lifeline(end: End) -> Option<usize> {
    match end {
        End::Life(l) => Some(l),
        End::Boundary(_) => None,
    }
}

/// Expands items into steps, threading the execution depth of each lifeline.
fn walk(items: &[Item], depth: Vec<usize>) -> (Vec<Step>, Vec<usize>) {
    items.iter().fold((Vec::new(), depth), |(mut steps, depth), item| {
        let (new_steps, depth) = walk_item(item, depth);
        steps.extend(new_steps);
        (steps, depth)
    })
}

fn at(depth: &[usize], end: End) -> usize {
    lifeline(end).map_or(0, |l| depth[l])
}

fn arrow_step(message: &Message, depth: &[usize], opens: bool) -> ArrowStep {
    ArrowStep {
        message: message.clone(),
        from_depth: at(depth, message.from),
        to_depth: at(depth, message.to) + usize::from(opens),
        opens,
    }
}

fn walk_item(item: &Item, depth: Vec<usize>) -> (Vec<Step>, Vec<usize>) {
    match item {
        Item::Message(message) => {
            let self_sync = message.from == message.to && message.sort == Sort::Sync;
            let arrow = Step::Arrow(arrow_step(message, &depth, self_sync));
            let steps = match (self_sync, lifeline(message.to)) {
                (true, Some(l)) => vec![arrow, Step::Close(l, Close::After(12.0))],
                _ => vec![arrow],
            };
            (steps, depth)
        }
        Item::Return(message) => (vec![Step::Arrow(arrow_step(message, &depth, false))], depth),
        Item::Call(message, body) => {
            let opened = lifeline(message.to).map_or_else(|| depth.clone(), |l| {
                let mut next = depth.clone();
                next[l] += 1;
                next
            });
            let first = Step::Arrow(arrow_step(message, &depth, true));
            let (inner, after_body) = walk(body, opened);
            let closes = match (lifeline(message.to), body.last()) {
                (Some(l), Some(Item::Return(_))) => vec![Step::Close(l, Close::AtReply)],
                (Some(l), Some(_)) => vec![Step::Close(l, Close::After(8.0))],
                (Some(l), None) => vec![Step::Close(l, Close::After(16.0))],
                (None, _) => Vec::new(),
            };
            let mut back = after_body;
            if let Some(l) = lifeline(message.to) {
                back[l] -= 1;
            }
            let steps = std::iter::once(first).chain(inner).chain(closes).collect();
            (steps, back)
        }
        Item::Destroy(l) => (vec![Step::Destroy(*l)], depth),
        Item::Ref(over, text) => {
            let (lo, hi) = (*over.iter().min().unwrap(), *over.iter().max().unwrap());
            (vec![Step::Ref(lo, hi, text.clone())], depth)
        }
        Item::Note(place, text) => (vec![Step::Note(place.clone(), depth.clone(), text.clone())], depth),
        Item::Fragment(fragment) => {
            let (operands, depth) = fragment.operands.iter().fold((Vec::new(), depth), |(mut out, depth), operand| {
                let (steps, depth) = walk(&operand.body, depth);
                let first_event = steps.iter().find_map(first_event);
                out.push(OperandSteps { guard: operand.guard.clone(), first_event, steps });
                (out, depth)
            });
            (vec![Step::Frame(fragment.operator, fragment.bounds.clone(), operands)], depth)
        }
    }
}

fn walk_max(steps: &[Step], participants: usize) -> Vec<usize> {
    fn visit(step: &Step, max: &mut [usize]) {
        match step {
            Step::Arrow(a) => {
                if let End::Life(l) = a.message.from {
                    max[l] = max[l].max(a.from_depth);
                }
                if let End::Life(l) = a.message.to {
                    max[l] = max[l].max(a.to_depth);
                }
            }
            Step::Frame(_, _, operands) => operands.iter().flat_map(|o| &o.steps).for_each(|s| visit(s, max)),
            _ => {}
        }
    }
    let mut max = vec![0; participants];
    steps.iter().for_each(|step| visit(step, &mut max));
    max
}

fn label_width(s: &str) -> f64 {
    width(s, SMALL, Weight::Regular)
}

fn wrapped(s: &str, max: f64) -> Vec<String> {
    wrap(s, SMALL, max)
}

fn widest(lines: &[String]) -> f64 {
    lines.iter().map(|l| label_width(l)).fold(0.0, f64::max)
}

fn operator_text(operator: Operator, bounds: &Option<String>) -> String {
    format!("{}{}", operator.keyword(), bounds.as_deref().unwrap_or(""))
}

pub fn tab_width(text: &str, height: f64) -> f64 {
    snap_up(keyword_width(text) + 22.0 + height * 0.25, GRID)
}

/// Height of the text-carrying rectangular heads: the same for every lifeline.
fn head_height(diagram: &Diagram) -> f64 {
    let lines = |p: &Participant| wrapped(&p.name, NAME_WRAP).len() as f64;
    let height = |p: &Participant| lines(p) * 16.0 + if p.stereotype.is_some() { 14.0 } else { 0.0 } + 18.0;
    let tallest = diagram.participants.iter().filter(|p| p.figure == Figure::Box).map(height).fold(40.0, f64::max);
    snap_up(tallest, GRID)
}

fn head_width(p: &Participant) -> f64 {
    let (name, stereo) = (wrap(&p.name, FONT, NAME_WRAP), p.stereotype.as_deref().map_or(0.0, |s| label_width(&format!("\u{ab}{s}\u{bb}"))));
    let text = name.iter().map(|l| width(l, FONT, Weight::Bold)).fold(0.0, f64::max).max(stereo);
    match p.figure {
        Figure::Box => snap_up((text + 24.0).max(72.0), 2.0 * GRID),
        Figure::Actor => snap_up(text.max(24.0) + 8.0, 2.0 * GRID),
    }
}

fn reach_of(depth: usize) -> Reach {
    Reach { left: if depth > 0 { BAR_HALF } else { 0.0 }, right: depth as f64 * NEST }
}

/// `xs[j] - xs[i] >= gap`, with `i < j`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Gap {
    i: usize,
    j: usize,
    gap: f64,
}

struct ColumnScan<'a> {
    reach: &'a [Reach],
    /// Head widths: a creation message ends on the new head's side, not its lifeline.
    heads: &'a [f64],
    participants: usize,
    /// How many fragment frames enclose this step.
    frames: usize,
}

/// Room for enclosing frame edges, so a frame's edge stays off the next lifeline.
fn frame_margin(scan: &ColumnScan) -> f64 {
    if scan.frames == 0 { 0.0 } else { scan.frames as f64 * FRAME_INSET + 8.0 }
}

fn right_of(scan: &ColumnScan, l: usize, need: f64) -> Option<Gap> {
    (l + 1 < scan.participants).then(|| Gap { i: l, j: l + 1, gap: need + frame_margin(scan) + scan.reach[l].right + scan.reach[l + 1].left })
}

fn left_of(scan: &ColumnScan, l: usize, need: f64) -> Option<Gap> {
    (l > 0).then(|| Gap { i: l - 1, j: l, gap: need + frame_margin(scan) + scan.reach[l - 1].right + scan.reach[l].left })
}

fn arrow_gaps(scan: &ColumnScan, a: &ArrowStep) -> Vec<Gap> {
    let label = a.message.label.as_deref().unwrap_or("");
    let own = |max| widest(&wrapped(label, max));
    let m = &a.message;
    match (m.from, m.to) {
        (End::Boundary(side), End::Life(l)) | (End::Life(l), End::Boundary(side)) => {
            let need = stub_length(label) + 16.0;
            match side {
                Side::Left => left_of(scan, l, need).into_iter().collect(),
                _ => right_of(scan, l, need).into_iter().collect(),
            }
        }
        (End::Life(from), End::Life(to)) if from == to => {
            let need = LOOP_REACH + LOOP_LABEL_GAP + own(SELF_WRAP) + 12.0;
            right_of(scan, from, need).into_iter().collect()
        }
        (End::Life(from), End::Life(to)) => {
            let (i, j) = (from.min(to), from.max(to));
            let head = if m.sort == Sort::Create { scan.heads[to] / 2.0 } else { 0.0 };
            let body = own(LABEL_WRAP) + 28.0 + scan.reach[i].right + scan.reach[j].left + head;
            // Crossing lifelines: keep the label beside the sender, in one gap.
            let (k, l) = if from < to { (from, from + 1) } else { (from - 1, from) };
            let first_gap = (j > i + 1).then(|| Gap { i: k, j: l, gap: own(LABEL_WRAP).min(LONG_LABEL) + 28.0 + scan.reach[k].right + scan.reach[l].left });
            std::iter::once(Gap { i, j, gap: body }).chain(first_gap).collect()
        }
        _ => Vec::new(),
    }
}

fn stub_length(label: &str) -> f64 {
    let own = widest(&wrapped(label, LABEL_WRAP));
    snap_up((own + STUB_ENDS).max(STUB), GRID)
}

/// The columns a step's content touches.
fn step_columns(step: &Step) -> Vec<usize> {
    match step {
        Step::Arrow(a) => [lifeline(a.message.from), lifeline(a.message.to)].into_iter().flatten().collect(),
        Step::Close(..) => Vec::new(),
        Step::Destroy(l) => vec![*l],
        Step::Frame(_, _, operands) => operands.iter().flat_map(|o| &o.steps).flat_map(step_columns).collect(),
        Step::Ref(lo, hi, _) => vec![*lo, *hi],
        Step::Note(place, _, _) => match place {
            NotePlace::Over(list) => list.clone(),
            NotePlace::LeftOf(l) | NotePlace::RightOf(l) => vec![*l],
        },
    }
}

fn step_gaps(scan: &ColumnScan, step: &Step) -> Vec<Gap> {
    match step {
        Step::Arrow(a) => arrow_gaps(scan, a),
        Step::Close(..) | Step::Destroy(_) => Vec::new(),
        Step::Note(place, _, text) => {
            let (w, _) = note_size(text);
            match place {
                NotePlace::LeftOf(l) => left_of(scan, *l, w + 16.0).into_iter().collect(),
                NotePlace::RightOf(l) => right_of(scan, *l, w + 16.0).into_iter().collect(),
                NotePlace::Over(list) if list.len() == 1 => {
                    [left_of(scan, list[0], w / 2.0 + 8.0), right_of(scan, list[0], w / 2.0 + 8.0)].into_iter().flatten().collect()
                }
                NotePlace::Over(list) => {
                    let (lo, hi) = (*list.iter().min().unwrap(), *list.iter().max().unwrap());
                    if lo == hi { Vec::new() } else { vec![Gap { i: lo, j: hi, gap: w }] }
                }
            }
        }
        Step::Ref(lo, hi, text) => {
            let need = widest(&wrapped(text, LABEL_WRAP)) + 2.0 * tab_width("ref", REF_TAB_HEIGHT) + 16.0;
            if lo == hi {
                [left_of(scan, *lo, need / 2.0 + 8.0), right_of(scan, *lo, need / 2.0 + 8.0)].into_iter().flatten().collect()
            } else {
                vec![Gap { i: *lo, j: *hi, gap: need }]
            }
        }
        Step::Frame(operator, bounds, operands) => frame_gaps(scan, step, *operator, bounds, operands),
    }
}

/// Room for the tab and the longest guard, then the nested contents' own needs.
fn frame_gaps(scan: &ColumnScan, frame: &Step, operator: Operator, bounds: &Option<String>, operands: &[OperandSteps]) -> Vec<Gap> {
    let columns = step_columns(frame);
    let (lo, hi) = (columns.iter().copied().min().unwrap_or(0), columns.iter().copied().max().unwrap_or(0));
    let tab = tab_width(&operator_text(operator, bounds), TAB_HEIGHT);
    let guard = operands.iter().filter_map(|o| o.guard.as_deref()).map(|g| widest(&wrapped(&format!("[{g}]"), LABEL_WRAP))).fold(0.0, f64::max);
    let need = tab + 8.0 + guard + 20.0;
    let own = if lo == hi {
        right_of(scan, lo, need - FRAME_INSET).into_iter().collect()
    } else {
        vec![Gap { i: lo, j: hi, gap: need }]
    };
    let height = nesting(frame) as f64 + 1.0;
    let inset = height * FRAME_INSET + 8.0;
    let edges = [left_of(scan, lo, inset), right_of(scan, hi, inset)].into_iter().flatten();
    let nested = ColumnScan { reach: scan.reach, heads: scan.heads, participants: scan.participants, frames: scan.frames + 1 };
    let inner: Vec<Gap> = operands.iter().flat_map(|o| &o.steps).flat_map(|s| step_gaps(&nested, s)).collect();
    own.into_iter().chain(edges).chain(inner).collect()
}

fn nesting(step: &Step) -> usize {
    match step {
        Step::Frame(_, _, operands) => operands.iter().flat_map(|o| &o.steps).map(|s| 1 + nesting(s)).max().unwrap_or(0),
        _ => 0,
    }
}

/// Lifeline x positions. Adjacent gaps are the maxima of their constraints, then
/// spanning constraints spread their deficit over the gaps they cross.
fn solve(heads: &[f64], gaps: &[Gap]) -> Vec<f64> {
    let n = heads.len();
    let mut width: Vec<f64> = (1..n).map(|j| heads[j - 1] / 2.0 + heads[j] / 2.0 + 16.0).collect();
    gaps.iter().filter(|g| g.j == g.i + 1).for_each(|g| width[g.i] = width[g.i].max(g.gap));
    let mut spans: Vec<Gap> = gaps.iter().copied().filter(|g| g.j > g.i + 1).collect();
    spans.sort_by(|a, b| (a.j - a.i, a.i, a.j).cmp(&(b.j - b.i, b.i, b.j)));
    spans.iter().for_each(|g| {
        let deficit = g.gap - width[g.i..g.j].iter().sum::<f64>();
        if deficit > 0.0 {
            let share = deficit / (g.j - g.i) as f64;
            width[g.i..g.j].iter_mut().for_each(|w| *w += share);
        }
    });
    (0..n).scan(0.0, |x, j| {
        if j > 0 {
            *x += snap_up(width[j - 1], GRID);
        }
        Some(*x)
    }).collect()
}

/// A placed multi-line label. `baseline` is that of the last line.
#[derive(Clone, Debug)]
pub struct Label {
    pub lines: Vec<String>,
    pub x: f64,
    pub baseline: f64,
    pub anchor: Anchor,
    pub rect: Rect,
}

#[derive(Clone, Debug)]
pub struct DrawnArrow {
    pub points: Vec<Point>,
    pub sort: Sort,
    pub dot: Option<Point>,
    pub label: Option<Label>,
}

#[derive(Clone, Copy, Debug)]
pub struct Bar {
    pub rect: Rect,
}

#[derive(Clone, Debug)]
pub struct DrawnGuard {
    pub lines: Vec<String>,
    pub rect: Rect,
}

#[derive(Clone, Debug)]
pub struct DrawnFrame {
    pub rect: Rect,
    pub tab: Rect,
    pub operator: String,
    pub separators: Vec<f64>,
    pub guards: Vec<DrawnGuard>,
}

#[derive(Clone, Debug)]
pub struct DrawnRef {
    pub rect: Rect,
    pub tab: Rect,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DrawnNote {
    pub rect: Rect,
    pub text: String,
    pub wrap: f64,
}

#[derive(Clone, Debug)]
pub struct Lifeline {
    pub x: f64,
    pub top: f64,
    pub bottom: f64,
    pub head: Rect,
}

#[derive(Debug)]
pub struct Layout {
    pub lifelines: Vec<Lifeline>,
    pub bars: Vec<Bar>,
    pub arrows: Vec<DrawnArrow>,
    pub crosses: Vec<Point>,
    pub frames: Vec<DrawnFrame>,
    pub refs: Vec<DrawnRef>,
    pub notes: Vec<DrawnNote>,
}

/// Everything fixed across the walk.
struct Grid {
    xs: Vec<f64>,
    head_h: f64,
    head_w: Vec<f64>,
    reach: Vec<Reach>,
}

/// The state threaded down the page.
struct Rows {
    /// The lowest point drawn so far.
    y: f64,
    /// The horizontal extent of the current frame's contents.
    span: Option<(f64, f64)>,
    open: Vec<(usize, usize, f64)>,
    last_event: Vec<f64>,
    tops: Vec<Option<f64>>,
    destroyed: Vec<Option<f64>>,
    bars: Vec<Bar>,
    arrows: Vec<DrawnArrow>,
    crosses: Vec<Point>,
    frames: Vec<DrawnFrame>,
    refs: Vec<DrawnRef>,
    notes: Vec<DrawnNote>,
}

fn widen(span: Option<(f64, f64)>, values: impl IntoIterator<Item = f64>) -> Option<(f64, f64)> {
    values.into_iter().fold(span, |span, v| match span {
        None => Some((v, v)),
        Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
    })
}

/// The x of the outer edge of a stack of `depth` bars on the lifeline at `x`.
fn edge(x: f64, depth: usize, side: Side) -> f64 {
    match (depth, side) {
        (0, _) => x,
        (d, Side::Left) => x + (d - 1) as f64 * NEST - BAR_HALF,
        (d, _) => x + d as f64 * NEST,
    }
}

/// Multi-line label extent, with the last baseline at `baseline`.
fn label_rect(lines: &[String], x: f64, anchor: Anchor, baseline: f64) -> Rect {
    let w = widest(lines);
    let h = lines.len() as f64 * LINE;
    let left = match anchor {
        Anchor::Start => x,
        Anchor::Middle => x - w / 2.0,
        Anchor::End => x - w,
    };
    Rect { x: left, y: baseline + 4.0 - h, w, h }
}

/// The free horizontal intervals along a message between `lo` and `hi`: one per
/// lifeline gap, so a label can sit clear of every lifeline it does not touch.
fn free_intervals(g: &Grid, lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let inner: Vec<f64> = g.xs.iter().copied().filter(|&x| x > lo + 1.0 && x < hi - 1.0).collect();
    let margin = |k: usize, side: Side| match side {
        Side::Left => g.reach[k].left,
        _ => g.reach[k].right,
    };
    let bounds: Vec<(f64, f64)> = std::iter::once((lo, 0.0))
        .chain(inner.iter().map(|&x| {
            let k = g.xs.iter().position(|&v| v == x).unwrap();
            (x, margin(k, Side::Right).max(margin(k, Side::Left)))
        }))
        .chain([(hi, 0.0)])
        .collect();
    bounds.windows(2).map(|w| (w[0].0 + w[0].1 + 8.0, w[1].0 - w[1].1 - 8.0)).filter(|(a, b)| b > a).collect()
}

/// Picks the interval for a label: the one nearest the sender that holds it on
/// one line, else the one that gives the fewest lines.
fn label_slot(text: &str, intervals: &[(f64, f64)], toward_right: bool) -> (Vec<String>, f64) {
    let ordered: Vec<(f64, f64)> = if toward_right { intervals.to_vec() } else { intervals.iter().rev().copied().collect() };
    let room = |i: &(f64, f64)| i.1 - i.0;
    let natural = label_width(text);
    if let Some(i) = ordered.iter().find(|i| room(i) >= natural) {
        return (vec![text.to_string()], (i.0 + i.1) / 2.0);
    }
    let by_lines = ordered.iter().map(|i| (wrapped(text, room(i).max(64.0)).len(), i));
    let (_, best) = by_lines.fold(None, |best: Option<(usize, &(f64, f64))>, (n, i)| match best {
        Some((m, _)) if m <= n => best,
        _ => Some((n, i)),
    }).expect("at least one interval");
    (wrapped(text, room(best).max(64.0)), (best.0 + best.1) / 2.0)
}

fn advance(y: f64, gap: f64) -> f64 {
    snap_up(y + gap, GRID)
}

/// The arrow row: enough air for the label above, at least 24 below the last row.
fn row_y(y: f64, lines: usize) -> f64 {
    let room = lines as f64 * LINE + 8.0 + 4.0;
    advance(y, room.max(24.0))
}

fn place_label(lines: Vec<String>, x: f64, y: f64) -> Label {
    let baseline = y - 5.0 - (lines.len() - 1) as f64 * 0.0;
    let rect = label_rect(&lines, x, Anchor::Middle, baseline);
    Label { lines, x, baseline, anchor: Anchor::Middle, rect }
}

fn self_step(g: &Grid, mut s: Rows, a: &ArrowStep, life: usize) -> Rows {
    let x = g.xs[life];
    let from = edge(x, a.from_depth, Side::Right);
    let land = edge(x, a.to_depth, Side::Right);
    let reach_x = from.max(land) + LOOP_REACH;
    let lines = a.message.label.as_deref().map_or(Vec::new(), |t| wrapped(t, SELF_WRAP));
    let label_h = lines.len() as f64 * LINE;
    let height = snap_up(LOOP_HEIGHT.max(label_h - 4.0), GRID);
    let air = (label_h / 2.0 - height / 2.0 + 8.0).max(16.0);
    let top = advance(s.y, air);
    let bottom = top + height;
    let label = (!lines.is_empty()).then(|| {
        let (lx, centre) = (reach_x + LOOP_LABEL_GAP, top + height / 2.0);
        let baseline = centre + label_h / 2.0 - 4.0;
        let rect = label_rect(&lines, lx, Anchor::Start, baseline);
        Label { lines, x: lx, baseline, anchor: Anchor::Start, rect }
    });
    s.span = widen(s.span, [x - BAR_HALF, reach_x].into_iter().chain(label.iter().map(|l| l.rect.right())));
    s.arrows.push(DrawnArrow {
        points: vec![(from, top), (reach_x, top), (reach_x, bottom), (land, bottom)],
        sort: a.message.sort,
        dot: None,
        label,
    });
    if a.opens {
        s.open.push((life, a.to_depth, bottom));
    }
    s.last_event[life] = bottom;
    s.y = bottom;
    s
}

fn boundary_arrow(g: &Grid, mut s: Rows, a: &ArrowStep, side: Side, life: usize, incoming: bool) -> Rows {
    let m = &a.message;
    let depth = if incoming { a.to_depth } else { a.from_depth };
    let near = edge(g.xs[life], depth, side);
    let label = m.label.as_deref().unwrap_or("");
    let length = stub_length(label);
    let far = match side {
        Side::Left => near - length,
        _ => near + length,
    };
    let lines = if label.is_empty() { Vec::new() } else { wrapped(label, length - STUB_ENDS) };
    let y = row_y(s.y, lines.len());
    // The label sits between the dot and the arrowhead, clear of both: a found
    // message's head is at the lifeline, a lost message's at the dot.
    let outward = if side == Side::Left { -1.0 } else { 1.0 };
    let (from_near, from_far) = if incoming { (HEAD_CLEAR, DOT_RADIUS + 6.0) } else { (8.0, DOT_RADIUS + HEAD_CLEAR) };
    let centre = ((near + outward * from_near) + (far - outward * from_far)) / 2.0;
    let drawn_label = (!lines.is_empty()).then(|| place_label(lines, centre, y));
    let dot_x = match (incoming, side) {
        (true, _) => far,
        (false, Side::Left) => far + DOT_RADIUS,
        (false, _) => far - DOT_RADIUS,
    };
    let points = if incoming { vec![(far, y), (near, y)] } else { vec![(near, y), (dot_x, y)] };
    s.span = widen(s.span, [near, far, g.xs[life] - BAR_HALF, g.xs[life] + BAR_HALF]);
    s.arrows.push(DrawnArrow { points, sort: m.sort, dot: Some((if incoming { far } else { far }, y)), label: drawn_label });
    if a.opens && incoming {
        s.open.push((life, a.to_depth, y));
    }
    s.last_event[life] = y;
    s.y = y;
    s
}

fn lifeline_arrow(g: &Grid, mut s: Rows, a: &ArrowStep, from: usize, to: usize) -> Rows {
    let m = &a.message;
    let rightward = g.xs[to] >= g.xs[from];
    let (out_side, in_side) = if rightward { (Side::Right, Side::Left) } else { (Side::Left, Side::Right) };
    let sender = edge(g.xs[from], a.from_depth, out_side);
    let creating = m.sort == Sort::Create;
    let receiver = if creating {
        g.xs[to] + if rightward { -g.head_w[to] / 2.0 } else { g.head_w[to] / 2.0 }
    } else {
        edge(g.xs[to], a.to_depth, in_side)
    };
    let (lo, hi) = (sender.min(receiver), sender.max(receiver));
    let slots: Vec<(f64, f64)> = free_intervals(g, lo, hi).into_iter().filter(|i| i.1 > i.0).collect();
    let slots = if slots.is_empty() { vec![(lo, hi)] } else { slots };
    let (lines, centre) = m.label.as_deref().map_or((Vec::new(), (lo + hi) / 2.0), |t| label_slot(t, &slots, rightward));
    // Keep the label off the arrowhead when it nearly fills its gap.
    let half_w = widest(&lines) / 2.0;
    let centre = if rightward { centre.min(receiver - HEAD_CLEAR - half_w) } else { centre.max(receiver + HEAD_CLEAR + half_w) };
    let air = if creating { snap_up(g.head_h / 2.0 + 12.0, GRID) } else { 0.0 };
    let y = row_y(s.y, lines.len()).max(advance(s.y, air));
    let label = (!lines.is_empty()).then(|| place_label(lines, centre, y));
    s.span = widen(
        s.span,
        [sender, receiver, g.xs[from] - BAR_HALF, g.xs[from] + BAR_HALF, g.xs[to] - BAR_HALF, g.xs[to] + BAR_HALF].into_iter().chain(label.iter().flat_map(|l| [l.rect.x, l.rect.right()])),
    );
    s.arrows.push(DrawnArrow { points: vec![(sender, y), (receiver, y)], sort: m.sort, dot: None, label });
    let top = y;
    if a.opens {
        s.open.push((to, a.to_depth, top));
    }
    s.last_event[from] = s.last_event[from].max(y);
    s.last_event[to] = y;
    if creating {
        let head_bottom = y + g.head_h / 2.0;
        s.tops[to] = Some(y - g.head_h / 2.0);
        s.last_event[to] = head_bottom;
        s.y = head_bottom;
    } else {
        s.y = y;
    }
    s
}

fn place_arrow(g: &Grid, s: Rows, a: &ArrowStep) -> Rows {
    let m = &a.message;
    match (m.from, m.to) {
        (End::Life(from), End::Life(to)) if from == to => self_step(g, s, a, from),
        (End::Life(from), End::Life(to)) => lifeline_arrow(g, s, a, from, to),
        (End::Boundary(side), End::Life(l)) => boundary_arrow(g, s, a, side, l, true),
        (End::Life(l), End::Boundary(side)) => boundary_arrow(g, s, a, side, l, false),
        (End::Boundary(_), End::Boundary(_)) => s,
    }
}

fn close_bar(g: &Grid, mut s: Rows, life: usize, close: &Close) -> Rows {
    let Some(pos) = s.open.iter().rposition(|o| o.0 == life) else { return s };
    let (_, depth, top) = s.open.remove(pos);
    let bottom = match close {
        Close::AtReply => s.last_event[life].max(top + 8.0),
        Close::After(gap) => s.y + gap,
    }
    .max(top + 8.0);
    let clipped = s.destroyed[life].map_or(bottom, |d| bottom.min(d));
    s.y = s.y.max(clipped);
    let bar_x = g.xs[life] + (depth - 1) as f64 * NEST - BAR_HALF;
    s.bars.push(Bar { rect: Rect { x: bar_x, y: top, w: 2.0 * BAR_HALF, h: clipped - top } });
    s
}

fn place_ref(g: &Grid, mut s: Rows, lo: usize, hi: usize, text: &str) -> Rows {
    let (left, right) = (g.xs[lo] - FRAME_INSET, g.xs[hi] + FRAME_INSET);
    let tab = tab_width("ref", REF_TAB_HEIGHT);
    let room = (right - left - tab * 2.0 - 8.0).max(120.0);
    let lines = wrapped(text, room.max(LABEL_WRAP.min(right - left)));
    let need = widest(&lines) + 2.0 * tab + 16.0;
    let centre = (left + right) / 2.0;
    let half = ((right - left).max(need)) / 2.0;
    let (x0, x1) = (centre - half, centre + half);
    let h = snap_up((lines.len() as f64 * LINE + 16.0).max(REF_TAB_HEIGHT + 16.0), GRID);
    let y = advance(s.y, 16.0);
    let rect = Rect { x: x0, y, w: x1 - x0, h };
    s.span = widen(s.span, [x0, x1]);
    s.refs.push(DrawnRef { rect, tab: Rect { x: x0, y, w: tab, h: REF_TAB_HEIGHT }, lines });
    s.y = y + h;
    s
}

fn place_note(g: &Grid, mut s: Rows, place: &NotePlace, depth: &[usize], text: &str) -> Rows {
    let y = advance(s.y, 12.0);
    let (rect, wrap) = match place {
        NotePlace::Over(list) => {
            let (lo, hi) = (*list.iter().min().unwrap(), *list.iter().max().unwrap());
            if lo == hi {
                let (w, h) = note_size(text);
                (Rect { x: g.xs[lo] - w / 2.0, y, w, h }, NOTE_WRAP)
            } else {
                let (left, right) = (g.xs[lo] - 16.0, g.xs[hi] + 16.0);
                let wrap = (right - left - 32.0).max(NOTE_WRAP);
                let (w, h) = note_size_wrapped(text, wrap);
                let w = w.max(right - left);
                (Rect { x: (left + right - w) / 2.0, y, w, h }, wrap)
            }
        }
        NotePlace::LeftOf(l) => {
            let (w, h) = note_size(text);
            (Rect { x: edge(g.xs[*l], depth[*l], Side::Left) - 8.0 - w, y, w, h }, NOTE_WRAP)
        }
        NotePlace::RightOf(l) => {
            let (w, h) = note_size(text);
            (Rect { x: edge(g.xs[*l], depth[*l], Side::Right) + 8.0, y, w, h }, NOTE_WRAP)
        }
    };
    s.span = widen(s.span, [rect.x, rect.right()]);
    s.notes.push(DrawnNote { rect, text: text.to_string(), wrap });
    s.y = rect.bottom();
    s
}

fn place_destroy(g: &Grid, mut s: Rows, life: usize) -> Rows {
    let y = advance(s.y, 16.0);
    s.destroyed[life] = Some(y);
    s.last_event[life] = y;
    s.span = widen(s.span, [g.xs[life] - 8.0, g.xs[life] + 8.0]);
    s.crosses.push((g.xs[life], y));
    s.y = y + 6.0;
    s
}

/// A guard sits above the first event, just right of that lifeline's bars and
/// clear of the operator tab, as 17.6.4.2 places an interaction constraint.
fn guard_at(g: &Grid, guard: &Option<String>, first: Option<(usize, usize)>, frame_left: f64, tab_right: f64, frame_right: f64, y: f64) -> Option<DrawnGuard> {
    let guard = guard.as_deref()?;
    let text = format!("[{guard}]");
    let lines = wrapped(&text, LABEL_WRAP);
    let w = widest(&lines);
    let beside = first.map(|(l, d)| edge(g.xs[l], d, Side::Right) + 8.0);
    let x = beside.unwrap_or(frame_left + 12.0).max(tab_right + 8.0).min(frame_right - w - 8.0).max(tab_right + 8.0);
    let rect = Rect { x, y: y + 4.0, w, h: lines.len() as f64 * LINE };
    Some(DrawnGuard { lines, rect })
}

fn place_frame(g: &Grid, mut s: Rows, operator: Operator, bounds: &Option<String>, operands: &[OperandSteps]) -> Rows {
    let op_text = operator_text(operator, bounds);
    let tab_w = tab_width(&op_text, TAB_HEIGHT);
    let top = advance(s.y, 16.0);
    let outer_span = s.span.take();
    let guard_lines = |o: &OperandSteps| o.guard.as_deref().map_or(0, |t| wrapped(&format!("[{t}]"), LABEL_WRAP).len());
    let first_extra = guard_lines(&operands[0]).saturating_sub(1) as f64 * LINE;
    s.y = top + TAB_HEIGHT + first_extra - 4.0;
    let (mut s, separators) = operands.iter().enumerate().fold((s, Vec::new()), |(s, mut seps), (k, operand)| {
        let mut s = operand.steps.iter().fold(s, |s, step| place_step(g, s, step));
        if k + 1 < operands.len() {
            let sep = snap_up(s.y + 8.0, GRID);
            seps.push(sep);
            let guard = guard_lines(&operands[k + 1]);
            s.y = sep + if guard > 0 { guard as f64 * LINE + 2.0 } else { 4.0 };
            s.y -= 4.0;
        }
        (s, seps)
    });
    let bottom = snap_up(s.y + 12.0, GRID).max(top + TAB_HEIGHT + 8.0);
    let (lo, hi) = s.span.unwrap_or((0.0, 0.0));
    let (x0, mut x1) = (lo - FRAME_INSET, hi + FRAME_INSET);
    let tab_right = x0 + tab_w;
    let guard_tops: Vec<f64> = std::iter::once(top).chain(separators.iter().copied()).collect();
    let first_guard = operands.iter().zip(&guard_tops).map(|(o, &y)| guard_at(g, &o.guard, o.first_event, x0, tab_right, f64::MAX, y)).collect::<Vec<_>>();
    x1 = first_guard.iter().flatten().map(|d| d.rect.right() + 12.0).fold(x1, f64::max).max(x0 + tab_w + 24.0);
    let guards: Vec<DrawnGuard> = operands.iter().zip(&guard_tops).filter_map(|(o, &y)| guard_at(g, &o.guard, o.first_event, x0, tab_right, x1, y)).collect();
    let rect = Rect { x: x0, y: top, w: x1 - x0, h: bottom - top };
    s.frames.push(DrawnFrame { rect, tab: Rect { x: x0, y: top, w: tab_w, h: TAB_HEIGHT }, operator: op_text, separators, guards });
    s.span = widen(outer_span, [x0, x1]);
    s.y = bottom;
    s
}

fn place_step(g: &Grid, s: Rows, step: &Step) -> Rows {
    match step {
        Step::Arrow(a) => place_arrow(g, s, a),
        Step::Close(life, close) => close_bar(g, s, *life, close),
        Step::Destroy(life) => place_destroy(g, s, *life),
        Step::Frame(operator, bounds, operands) => place_frame(g, s, *operator, bounds, operands),
        Step::Ref(lo, hi, text) => place_ref(g, s, *lo, *hi, text),
        Step::Note(place, depth, text) => place_note(g, s, place, depth, text),
    }
}

fn lifelines(g: &Grid, s: &Rows, end_y: f64) -> Vec<Lifeline> {
    (0..g.xs.len())
        .map(|l| {
            let w = g.head_w[l];
            let head_top = s.tops[l].unwrap_or(0.0);
            let head = Rect { x: g.xs[l] - w / 2.0, y: head_top, w, h: g.head_h };
            let bottom = s.destroyed[l].map_or(end_y, |d| d.max(s.last_event[l]));
            Lifeline { x: g.xs[l], top: head.bottom(), bottom, head }
        })
        .collect()
}

pub fn layout(diagram: &Diagram) -> Layout {
    let n = diagram.participants.len();
    let (steps, _) = walk(&diagram.items, vec![0; n]);
    let reach: Vec<Reach> = walk_max(&steps, n).into_iter().map(reach_of).collect();
    let head_w: Vec<f64> = diagram.participants.iter().map(head_width).collect();
    let scan = ColumnScan { reach: &reach, heads: &head_w, participants: n, frames: 0 };
    let gaps: Vec<Gap> = steps.iter().flat_map(|s| step_gaps(&scan, s)).collect();
    let xs = solve(&head_w, &gaps);
    let head_h = head_height(diagram);
    let grid = Grid { xs, head_h, head_w, reach };
    let start = Rows {
        y: head_h,
        span: None,
        open: Vec::new(),
        last_event: vec![0.0; n],
        tops: vec![None; n],
        destroyed: vec![None; n],
        bars: Vec::new(),
        arrows: Vec::new(),
        crosses: Vec::new(),
        frames: Vec::new(),
        refs: Vec::new(),
        notes: Vec::new(),
    };
    let rows = steps.iter().fold(start, |s, step| place_step(&grid, s, step));
    let end_y = snap_up(rows.y + 16.0, GRID);
    let lifelines = lifelines(&grid, &rows, end_y);
    Layout { lifelines, bars: rows.bars, arrows: rows.arrows, crosses: rows.crosses, frames: rows.frames, refs: rows.refs, notes: rows.notes }
}

// The tab geometry used by the drawing stage.
pub fn tab_slant() -> f64 {
    TAB_SLANT
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
