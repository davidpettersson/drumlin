// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Quality measures for routed output: how often lines cross, run on top of each
//! other and bend. Pure functions over orthogonal polylines, used by tests to show
//! the effect of a layout or routing change in numbers, and by the router's
//! nudging stage to compare candidate positions. `assess` widens this to a whole
//! drawing described as a `Scene`: shapes, text and spacing as well as lines.

// `measure` and `Metrics` serve the quality test; the router uses the pair counts.
#![cfg_attr(not(test), allow(dead_code))]

use super::geometry::{Point, Rect};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Metrics {
    /// Pairs of segments from different lines that cross, each strictly inside
    /// the other. Lines meeting at a corner or an end do not count here.
    pub crossings: usize,
    /// Pairs of collinear segments from different lines that run on top of each
    /// other for some length. Lines that share an end, such as the branches of a
    /// tree drawn from one anchor, may merge, and do not count.
    pub overlaps: usize,
    /// Corners: points where a line changes direction.
    pub bends: usize,
}

impl std::ops::Add for Metrics {
    type Output = Metrics;
    fn add(self, other: Metrics) -> Metrics {
        Metrics { crossings: self.crossings + other.crossings, overlaps: self.overlaps + other.overlaps, bends: self.bends + other.bends }
    }
}

impl std::iter::Sum for Metrics {
    fn sum<I: Iterator<Item = Metrics>>(iter: I) -> Metrics {
        iter.fold(Metrics::default(), |a, b| a + b)
    }
}

/// An axis-parallel segment: the fixed coordinate and the span along the other axis.
#[derive(Clone, Copy, Debug)]
struct Run {
    at: f64,
    lo: f64,
    hi: f64,
}

const EPS: f64 = 0.01;

/// The horizontal and the vertical runs of a path. Diagonal pieces, which only a
/// failed route has, are ignored.
fn runs(path: &[Point]) -> (Vec<Run>, Vec<Run>) {
    let segments = path.windows(2).filter(|w| w[0] != w[1]);
    let horizontal = segments.clone().filter(|w| (w[0].1 - w[1].1).abs() < EPS).map(|w| Run { at: w[0].1, lo: w[0].0.min(w[1].0), hi: w[0].0.max(w[1].0) });
    let vertical = segments.filter(|w| (w[0].0 - w[1].0).abs() < EPS).map(|w| Run { at: w[0].0, lo: w[0].1.min(w[1].1), hi: w[0].1.max(w[1].1) });
    (horizontal.collect(), vertical.collect())
}

fn strictly_inside(v: f64, run: &Run) -> bool {
    v > run.lo + EPS && v < run.hi - EPS
}

fn crosses(h: &Run, v: &Run) -> bool {
    strictly_inside(v.at, h) && strictly_inside(h.at, v)
}

/// The overlap length below which two collinear runs only touch.
fn overlapping(a: &Run, b: &Run) -> bool {
    (a.at - b.at).abs() < EPS && a.lo.max(b.lo) < a.hi.min(b.hi) - EPS
}

/// Parallel runs closer than `gap` for some length, without lying on top of each other.
fn side_by_side(x: &Run, y: &Run, gap: f64) -> bool {
    let d = (x.at - y.at).abs();
    d > EPS && d < gap - EPS && x.lo.max(y.lo) < x.hi.min(y.hi) - EPS
}

fn same_point(a: Point, b: Point) -> bool {
    (a.0 - b.0).abs() < EPS && (a.1 - b.1).abs() < EPS
}

/// Whether two lines start or end at the same point.
fn share_an_end(a: &[Point], b: &[Point]) -> bool {
    match (a.first(), a.last(), b.first(), b.last()) {
        (Some(&a0), Some(&a1), Some(&b0), Some(&b1)) => [a0, a1].iter().any(|&p| same_point(p, b0) || same_point(p, b1)),
        _ => false,
    }
}

/// Points where consecutive segments change direction. Repeated points and
/// straight joints are not bends.
pub fn bends(path: &[Point]) -> usize {
    let points: Vec<Point> = path.iter().copied().fold(Vec::new(), |mut out, p| {
        if out.last().is_none_or(|&q| !same_point(q, p)) {
            out.push(p);
        }
        out
    });
    points
        .windows(3)
        .filter(|w| {
            let (a, b, c) = (w[0], w[1], w[2]);
            let cross = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
            cross.abs() > EPS
        })
        .count()
}

/// Proper crossings between two lines.
pub fn crossings_between(a: &[Point], b: &[Point]) -> usize {
    let ((ha, va), (hb, vb)) = (runs(a), runs(b));
    let count = |hs: &[Run], vs: &[Run]| hs.iter().map(|h| vs.iter().filter(|v| crosses(h, v)).count()).sum::<usize>();
    count(&ha, &vb) + count(&hb, &va)
}

/// Collinear runs of two lines that lie on top of each other, unless the lines
/// share an end.
pub fn overlaps_between(a: &[Point], b: &[Point]) -> usize {
    if share_an_end(a, b) {
        return 0;
    }
    let ((ha, va), (hb, vb)) = (runs(a), runs(b));
    let count = |xs: &[Run], ys: &[Run]| xs.iter().map(|x| ys.iter().filter(|y| overlapping(x, y)).count()).sum::<usize>();
    count(&ha, &hb) + count(&va, &vb)
}

/// Pairs of parallel segments, one from each line, that run side by side for
/// some length closer than `gap`, without lying on top of each other.
pub fn close_runs(a: &[Point], b: &[Point], gap: f64) -> usize {
    let ((ha, va), (hb, vb)) = (runs(a), runs(b));
    let near = |x: &Run, y: &Run| side_by_side(x, y, gap);
    let count = |xs: &[Run], ys: &[Run]| xs.iter().map(|x| ys.iter().filter(|y| near(x, y)).count()).sum::<usize>();
    count(&ha, &hb) + count(&va, &vb)
}

/// Crossings, overlaps and close runs of `a` against each of `others`, as
/// `crossings_between`, `overlaps_between` and `close_runs` would sum them.
pub fn against<'a>(a: &[Point], others: impl Iterator<Item = &'a [Point]>, gap: f64) -> (usize, usize, usize) {
    let (ha, va) = runs(a);
    let near = |x: &Run, y: &Run| side_by_side(x, y, gap);
    let count = |xs: &[Run], ys: &[Run], f: &dyn Fn(&Run, &Run) -> bool| xs.iter().map(|x| ys.iter().filter(|y| f(x, y)).count()).sum::<usize>();
    others.fold((0, 0, 0), |(c, o, n), b| {
        let (hb, vb) = runs(b);
        let crossings = count(&ha, &vb, &crosses) + count(&hb, &va, &crosses);
        let overlaps = if share_an_end(a, b) { 0 } else { count(&ha, &hb, &overlapping) + count(&va, &vb, &overlapping) };
        let close = count(&ha, &hb, &near) + count(&va, &vb, &near);
        (c + crossings, o + overlaps, n + close)
    })
}

/// Pairs of different lines that leave one point in the same direction, so they
/// run together from a shared end and cannot be told apart there. `overlaps`
/// excuses lines that share an end, so this counts what it lets through.
pub fn merged_ends(paths: &[Vec<Point>]) -> usize {
    let leaving = |p: &Vec<Point>| -> Vec<(Point, Point)> {
        if p.len() < 2 {
            return Vec::new();
        }
        vec![(p[0], p[1]), (p[p.len() - 1], p[p.len() - 2])]
    };
    let together = |(p, q): (Point, Point), (r, s): (Point, Point)| {
        let (u, v) = ((q.0 - p.0, q.1 - p.1), (s.0 - r.0, s.1 - r.1));
        same_point(p, r) && (u.0 * v.1 - u.1 * v.0).abs() < EPS && u.0 * v.0 + u.1 * v.1 > 0.0
    };
    let pairs = (0..paths.len()).flat_map(|i| (i + 1..paths.len()).map(move |j| (i, j)));
    pairs.filter(|&(i, j)| leaving(&paths[i]).into_iter().any(|a| leaving(&paths[j]).into_iter().any(|b| together(a, b)))).count()
}

/// The length of a polyline: its segments summed, a diagonal one (a note
/// link) at its straight length.
pub fn length(path: &[Point]) -> f64 {
    path.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum()
}

/// How big a drawing is, in whole pixels: the summed length of its lines, and
/// the width, height and area of its frame (the document's own size, margins
/// and title included).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    pub length: usize,
    pub width: usize,
    pub height: usize,
    pub area: usize,
}

/// The size of a drawing with `lines` in a frame `(width, height)`. Each
/// value is rounded once, from the exact sum or product.
pub fn size(lines: &[Vec<Point>], (width, height): (f64, f64)) -> Size {
    let whole = |v: f64| v.round() as usize;
    Size { length: whole(lines.iter().map(|l| length(l)).sum()), width: whole(width), height: whole(height), area: whole(width * height) }
}

/// Crossings and overlaps between one line and another.
pub fn between(a: &[Point], b: &[Point]) -> Metrics {
    Metrics { crossings: crossings_between(a, b), overlaps: overlaps_between(a, b), bends: 0 }
}

/// Crossings and overlaps between every pair of distinct lines, and bends in all of them.
pub fn measure(paths: &[Vec<Point>]) -> Metrics {
    let pairs = (0..paths.len()).flat_map(|i| (i + 1..paths.len()).map(move |j| (i, j)));
    let shared: Metrics = pairs.map(|(i, j)| between(&paths[i], &paths[j])).sum();
    Metrics { bends: paths.iter().map(|p| bends(p)).sum(), ..shared }
}

/// An opaque shape. `across` marks one that may lie across a border, such as a
/// fork bar spanning partitions.
#[derive(Clone, Copy, Debug)]
pub struct Solid {
    pub rect: Rect,
    pub across: bool,
}

/// A drawing reduced to what the whole-drawing measure needs. Diagram kinds
/// that do not route through `core::router` (sequence, activity) describe their
/// output this way, choosing what goes in each list for their notation.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    /// Connectors: their crossings, overlaps and bends count.
    pub lines: Vec<Vec<Point>>,
    /// False where bends are fixed by the notation, as in a self message.
    pub count_bends: bool,
    /// Opaque shapes. They must not overlap one another, and no line may pass
    /// through one; a line may end on its outline.
    pub solids: Vec<Solid>,
    /// Shapes drawn under the solids, such as execution bars: they may overlap
    /// one another and lines may cross them, but no solid may cover them.
    pub backs: Vec<Rect>,
    /// Strokes that text must keep clear of but shapes may cover: lifelines.
    pub rules: Vec<Vec<Point>>,
    /// Strokes that text and solids must keep clear of: frame edges, operand
    /// separators, partition dividers.
    pub borders: Vec<Vec<Point>>,
    /// Free-standing text, as ink boxes: message labels, guards.
    pub labels: Vec<Rect>,
    /// The line each label belongs to, indexed like `labels`, for `read_labels`.
    /// Labels beyond its end, or with `None`, belong to no line, such as port names.
    pub owners: Vec<Option<usize>>,
    /// Small glyphs at line ends that text must keep clear of: arrowheads, dots.
    pub marks: Vec<Rect>,
    /// Text inside a shape: its ink box and the room it must stay inside.
    pub texts: Vec<(Rect, Rect)>,
    /// Distances the notation means to be equal, such as the space between two
    /// consecutive messages or two actions in a straight flow.
    pub gaps: Vec<f64>,
}

/// What `assess` counts. Each field is a number of offending pairs or items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Quality {
    pub crossings: usize,
    pub overlaps: usize,
    pub bends: usize,
    /// Line segments that pass through a solid.
    pub through: usize,
    /// Pairs of shapes that overlap: solid on solid, solid on back, or a solid
    /// across a border.
    pub shapes: usize,
    /// Labels touching a line, rule, border or mark.
    pub label_lines: usize,
    /// Labels touching a solid or a back.
    pub label_shapes: usize,
    /// Pairs of labels touching each other.
    pub label_labels: usize,
    /// Text that runs outside its shape.
    pub overflow: usize,
    /// Gaps that differ from the most common gap.
    pub uneven: usize,
}

impl std::ops::Add for Quality {
    type Output = Quality;
    fn add(self, o: Quality) -> Quality {
        Quality {
            crossings: self.crossings + o.crossings,
            overlaps: self.overlaps + o.overlaps,
            bends: self.bends + o.bends,
            through: self.through + o.through,
            shapes: self.shapes + o.shapes,
            label_lines: self.label_lines + o.label_lines,
            label_shapes: self.label_shapes + o.label_shapes,
            label_labels: self.label_labels + o.label_labels,
            overflow: self.overflow + o.overflow,
            uneven: self.uneven + o.uneven,
        }
    }
}

impl std::iter::Sum for Quality {
    fn sum<I: Iterator<Item = Quality>>(iter: I) -> Quality {
        iter.fold(Quality::default(), |a, b| a + b)
    }
}

/// Helvetica's cap height and descender, as fractions of the font size.
const ASCENT: f64 = 0.72;
const DESCENT: f64 = 0.21;

/// The ink box of `lines` of text `size` px high whose widest line is `width`,
/// starting at `left`, with baselines `first` and `last`.
pub fn ink(left: f64, width: f64, first: f64, last: f64, size: f64) -> Rect {
    Rect { x: left, y: first - ASCENT * size, w: width, h: last - first + (ASCENT + DESCENT) * size }
}

/// The box around an arrowhead `back` long and `half` wide at the end of a line.
pub fn tip_box(points: &[Point], back: f64, half: f64) -> Rect {
    let (tip, before) = (points[points.len() - 1], points[points.len() - 2]);
    let l = ((tip.0 - before.0).powi(2) + (tip.1 - before.1).powi(2)).sqrt().max(0.001);
    let (dx, dy) = ((tip.0 - before.0) / l, (tip.1 - before.1) / l);
    let corners = [(tip.0 - dx * back - dy * half, tip.1 - dy * back + dx * half), (tip.0 - dx * back + dy * half, tip.1 - dy * back - dx * half), tip];
    let (x0, x1) = (corners.iter().map(|c| c.0).fold(f64::MAX, f64::min), corners.iter().map(|c| c.0).fold(f64::MIN, f64::max));
    let (y0, y1) = (corners.iter().map(|c| c.1).fold(f64::MAX, f64::min), corners.iter().map(|c| c.1).fold(f64::MIN, f64::max));
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// Whether two rectangles share some area, not just an edge.
pub fn rects_meet(a: &Rect, b: &Rect) -> bool {
    a.x < b.right() - EPS && b.x < a.right() - EPS && a.y < b.bottom() - EPS && b.y < a.bottom() - EPS
}

/// Whether a segment passes through the inside of a rectangle. A segment
/// along the outline, or ending on it, does not. Diagonal segments, which only
/// note links have, are clipped to the rectangle (Liang and Barsky).
fn segment_enters(a: Point, b: Point, r: &Rect) -> bool {
    let (x0, x1, y0, y1) = (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1));
    let boxed = x0 < r.right() - EPS && x1 > r.x + EPS && y0 < r.bottom() - EPS && y1 > r.y + EPS;
    if !boxed || a.0 == b.0 || a.1 == b.1 {
        return boxed;
    }
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let edges = [(-dx, a.0 - (r.x + EPS)), (dx, (r.right() - EPS) - a.0), (-dy, a.1 - (r.y + EPS)), (dy, (r.bottom() - EPS) - a.1)];
    let (lo, hi) = edges.iter().fold((0.0f64, 1.0f64), |(lo, hi), &(p, q)| if p < 0.0 { (lo.max(q / p), hi) } else { (lo, hi.min(q / p)) });
    lo < hi
}

fn path_enters(path: &[Point], r: &Rect) -> bool {
    path.windows(2).any(|w| segment_enters(w[0], w[1], r))
}

/// The clearance text needs from a stroke or a glyph: the width of the white
/// halo drawn around labels, so a halo never bites into a line.
const CLEAR: f64 = 2.0;

/// Whether a stroke comes closer to a text box than `CLEAR`.
pub fn path_touches(path: &[Point], r: &Rect) -> bool {
    path_enters(path, &r.inflate(CLEAR))
}

pub fn mark_touches(m: &Rect, r: &Rect) -> bool {
    rects_meet(m, &r.inflate(CLEAR))
}

fn inside(inner: &Rect, outer: &Rect) -> bool {
    inner.x >= outer.x - 0.5 && inner.y >= outer.y - 0.5 && inner.right() <= outer.right() + 0.5 && inner.bottom() <= outer.bottom() + 0.5
}

/// The difference below which two gaps look equal: half a grid step.
const UNEVEN: f64 = 4.0;

/// How many of `gaps` differ by more than half a grid step from the most common one
/// (the smallest, on a tie).
pub fn uneven(gaps: &[f64]) -> usize {
    let near = |a: f64, b: f64| (a - b).abs() <= UNEVEN;
    let count = |g: f64| gaps.iter().filter(|&&h| near(g, h)).count();
    let mode = gaps.iter().copied().fold(None, |best: Option<f64>, g| match best {
        Some(b) if count(b) > count(g) || (count(b) == count(g) && b <= g) => Some(b),
        _ => Some(g),
    });
    mode.map_or(0, |m| gaps.iter().filter(|&&g| !near(g, m)).count())
}

fn pairs(n: usize) -> impl Iterator<Item = (usize, usize)> {
    (0..n).flat_map(move |i| (i + 1..n).map(move |j| (i, j)))
}

/// One line per offending item or pair, for finding what `assess` counted.
pub fn explain(s: &Scene) -> Vec<String> {
    let strokes: Vec<(&str, &Vec<Point>)> = s.lines.iter().map(|l| ("line", l)).chain(s.rules.iter().map(|l| ("rule", l))).chain(s.borders.iter().map(|l| ("border", l))).collect();
    let through = s.lines.iter().flat_map(|l| s.solids.iter().filter(|o| path_enters(l, &o.rect)).map(move |o| format!("line {l:?} through {:?}", o.rect)));
    let solid_pairs = pairs(s.solids.len()).filter(|&(i, j)| rects_meet(&s.solids[i].rect, &s.solids[j].rect)).map(|(i, j)| format!("solid {:?} on solid {:?}", s.solids[i].rect, s.solids[j].rect));
    let covered = s.solids.iter().flat_map(|o| s.backs.iter().filter(|b| rects_meet(&o.rect, b)).map(move |b| format!("solid {:?} covers {b:?}", o.rect)));
    let across = s.solids.iter().filter(|o| !o.across).flat_map(|o| s.borders.iter().filter(|b| path_enters(b, &o.rect)).map(move |b| format!("solid {:?} across {b:?}", o.rect)));
    let on_strokes = s.labels.iter().flat_map(|r| strokes.iter().filter(|(_, p)| path_touches(p, r)).map(move |(k, p)| format!("label {r:?} on {k} {p:?}")));
    let on_marks = s.labels.iter().flat_map(|r| s.marks.iter().filter(|m| mark_touches(m, r)).map(move |m| format!("label {r:?} on mark {m:?}")));
    let on_shapes = s.labels.iter().flat_map(|r| s.solids.iter().map(|o| o.rect).chain(s.backs.iter().copied()).filter(|o| rects_meet(r, o)).map(move |o| format!("label {r:?} on shape {o:?}")));
    let on_labels = pairs(s.labels.len()).filter(|&(i, j)| rects_meet(&s.labels[i], &s.labels[j])).map(|(i, j)| format!("label {:?} on label {:?}", s.labels[i], s.labels[j]));
    let overflow = s.texts.iter().filter(|(t, room)| !inside(t, room)).map(|(t, room)| format!("text {t:?} outside {room:?}"));
    let gaps = format!("gaps {:?}", s.gaps);
    through.chain(solid_pairs).chain(covered).chain(across).chain(on_strokes).chain(on_marks).chain(on_shapes).chain(on_labels).chain(overflow).chain([gaps]).collect()
}

/// Everything a reader would call a flaw in a drawing that `Scene` describes.
pub fn assess(s: &Scene) -> Quality {
    let lines = measure(&s.lines);
    let through = s.lines.iter().flat_map(|l| l.windows(2)).map(|w| s.solids.iter().filter(|o| segment_enters(w[0], w[1], &o.rect)).count()).sum();
    let solid_pairs = pairs(s.solids.len()).filter(|&(i, j)| rects_meet(&s.solids[i].rect, &s.solids[j].rect)).count();
    let covered = s.solids.iter().map(|o| s.backs.iter().filter(|b| rects_meet(&o.rect, b)).count()).sum::<usize>();
    let across = s.solids.iter().filter(|o| !o.across).map(|o| s.borders.iter().filter(|b| path_enters(b, &o.rect)).count()).sum::<usize>();
    let strokes: Vec<&Vec<Point>> = s.lines.iter().chain(&s.rules).chain(&s.borders).collect();
    let label_lines = s.labels.iter().map(|r| strokes.iter().filter(|p| path_touches(p, r)).count() + s.marks.iter().filter(|m| mark_touches(m, r)).count()).sum();
    let label_shapes = s.labels.iter().map(|r| s.solids.iter().filter(|o| rects_meet(r, &o.rect)).count() + s.backs.iter().filter(|b| rects_meet(r, b)).count()).sum();
    let label_labels = pairs(s.labels.len()).filter(|&(i, j)| rects_meet(&s.labels[i], &s.labels[j])).count();
    Quality {
        crossings: lines.crossings,
        overlaps: lines.overlaps,
        bends: if s.count_bends { lines.bends } else { 0 },
        through,
        shapes: solid_pairs + covered + across,
        label_lines,
        label_shapes,
        label_labels,
        overflow: s.texts.iter().filter(|(t, room)| !inside(t, room)).count(),
        uneven: uneven(&s.gaps),
    }
}

/// How connector labels read, counted over the labels that belong to a line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reading {
    /// Labels on their line: how many there are.
    pub count: usize,
    /// Label and line pairs, any line, rule, border or mark, closer than the halo.
    pub on_lines: usize,
    /// Label and shape pairs that overlap.
    pub on_shapes: usize,
    /// Pairs of labels that overlap, where at least one belongs to a line.
    pub on_labels: usize,
    /// Labels with another line at least as near as their own.
    pub ambiguous: usize,
    /// Labels more than `FAR` from their own line.
    pub far: usize,
    /// The distances from each label to its own line, summed, in whole pixels.
    pub distance: usize,
}

impl std::ops::Add for Reading {
    type Output = Reading;
    fn add(self, o: Reading) -> Reading {
        Reading {
            count: self.count + o.count,
            on_lines: self.on_lines + o.on_lines,
            on_shapes: self.on_shapes + o.on_shapes,
            on_labels: self.on_labels + o.on_labels,
            ambiguous: self.ambiguous + o.ambiguous,
            far: self.far + o.far,
            distance: self.distance + o.distance,
        }
    }
}

impl std::iter::Sum for Reading {
    fn sum<I: Iterator<Item = Reading>>(iter: I) -> Reading {
        iter.fold(Reading::default(), |a, b| a + b)
    }
}

/// Beyond this distance from its line a label no longer reads as beside it.
pub const FAR: f64 = 12.0;

/// The distance from a rectangle to an axis-parallel segment.
fn rect_to_segment(r: &Rect, a: Point, b: Point) -> f64 {
    let (x0, x1, y0, y1) = (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1));
    let dx = (r.x - x1).max(x0 - r.right()).max(0.0);
    let dy = (r.y - y1).max(y0 - r.bottom()).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

/// The distance from a rectangle to the nearest segment of a line, or infinity
/// for a line with no segment.
pub fn rect_to_line(r: &Rect, path: &[Point]) -> f64 {
    path.windows(2).map(|w| rect_to_segment(r, w[0], w[1])).fold(f64::INFINITY, f64::min)
}

/// One line per flaw `read_labels` counts, for finding them.
pub fn explain_labels(s: &Scene) -> Vec<String> {
    owned(s)
        .flat_map(|(i, r, own)| {
            let d = rect_to_line(r, &s.lines[own]);
            let nearest = (0..s.lines.len()).filter(|&j| j != own).map(|j| (j, rect_to_line(r, &s.lines[j]))).fold((usize::MAX, f64::INFINITY), |a, b| if b.1 < a.1 { b } else { a });
            let flaws = [
                (label_strokes(s, r) > 0).then(|| "on a line"),
                (label_solids(s, r) > 0).then(|| "on a shape"),
                (s.labels.iter().enumerate().any(|(j, o)| j != i && rects_meet(r, o))).then(|| "on a label"),
                (nearest.1 <= d).then(|| "ambiguous"),
                (d > FAR).then(|| "far"),
            ];
            let flaws: Vec<&str> = flaws.into_iter().flatten().collect();
            (!flaws.is_empty()).then(|| format!("label {i} {r:?} of line {own}: {} (own {d:.1}, line {} at {:.1})", flaws.join(", "), nearest.0, nearest.1))
        })
        .collect()
}

/// Labels that belong to a line: index, ink box and the line.
fn owned(s: &Scene) -> impl Iterator<Item = (usize, &Rect, usize)> {
    s.labels.iter().enumerate().filter_map(|(i, r)| s.owners.get(i).copied().flatten().filter(|&own| own < s.lines.len()).map(|own| (i, r, own)))
}

fn label_strokes(s: &Scene, r: &Rect) -> usize {
    let strokes = s.lines.iter().chain(&s.rules).chain(&s.borders);
    strokes.filter(|p| path_touches(p, r)).count() + s.marks.iter().filter(|m| mark_touches(m, r)).count()
}

fn label_solids(s: &Scene, r: &Rect) -> usize {
    s.solids.iter().filter(|o| rects_meet(r, &o.rect)).count() + s.backs.iter().filter(|b| rects_meet(r, b)).count()
}

/// Collisions, ambiguity and distance for the labels of a `Scene` that belong
/// to a line (`Scene::owners`).
pub fn read_labels(s: &Scene) -> Reading {
    let mine: Vec<(usize, &Rect, usize)> = owned(s).collect();
    let is_owned = |i: usize| mine.iter().any(|&(j, _, _)| j == i);
    let distances: Vec<(f64, f64)> = mine
        .iter()
        .map(|&(_, r, own)| (rect_to_line(r, &s.lines[own]), (0..s.lines.len()).filter(|&j| j != own).map(|j| rect_to_line(r, &s.lines[j])).fold(f64::INFINITY, f64::min)))
        .collect();
    Reading {
        count: mine.len(),
        on_lines: mine.iter().map(|&(_, r, _)| label_strokes(s, r)).sum(),
        on_shapes: mine.iter().map(|&(_, r, _)| label_solids(s, r)).sum(),
        on_labels: pairs(s.labels.len()).filter(|&(i, j)| (is_owned(i) || is_owned(j)) && rects_meet(&s.labels[i], &s.labels[j])).count(),
        ambiguous: distances.iter().filter(|(own, other)| other <= own).count(),
        far: distances.iter().filter(|(own, _)| *own > FAR).count(),
        distance: distances.iter().map(|(own, _)| own.round() as usize).sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn against_sums_the_pairwise_counts() {
        let a = vec![(0.0, 20.0), (100.0, 20.0), (100.0, 80.0)];
        let others = [
            vec![(50.0, 0.0), (50.0, 60.0)],
            vec![(20.0, 20.0), (60.0, 20.0)],
            vec![(10.0, 24.0), (90.0, 24.0)],
            vec![(0.0, 20.0), (0.0, 90.0)],
            vec![(104.0, 30.0), (104.0, 70.0), (130.0, 70.0)],
        ];
        let expected = others.iter().fold((0, 0, 0), |(c, o, n), b| (c + crossings_between(&a, b), o + overlaps_between(&a, b), n + close_runs(&a, b, 8.0)));
        assert_eq!(against(&a, others.iter().map(|p| p.as_slice()), 8.0), expected);
        assert!(expected.0 > 0 && expected.1 > 0 && expected.2 > 0, "every count is exercised: {expected:?}");
    }

    #[test]
    fn size_sums_line_lengths_and_rounds_once() {
        let lines = vec![vec![(0.0, 0.0), (0.0, 40.0), (30.0, 40.0)], vec![(0.0, 0.0), (3.0, 4.0)], vec![(5.0, 5.0)]];
        assert_eq!(length(&lines[0]), 70.0);
        assert_eq!(length(&lines[1]), 5.0);
        assert_eq!(length(&lines[2]), 0.0);
        assert_eq!(size(&lines, (100.4, 50.5)), Size { length: 75, width: 100, height: 51, area: 5070 });
        assert_eq!(size(&[], (0.0, 0.0)), Size::default());
    }

    #[test]
    fn a_plus_is_one_crossing() {
        let paths = vec![vec![(0.0, 50.0), (100.0, 50.0)], vec![(50.0, 0.0), (50.0, 100.0)]];
        assert_eq!(measure(&paths), Metrics { crossings: 1, overlaps: 0, bends: 0 });
    }

    #[test]
    fn touching_is_not_crossing() {
        let t = vec![vec![(0.0, 50.0), (100.0, 50.0)], vec![(50.0, 50.0), (50.0, 100.0)]];
        assert_eq!(measure(&t), Metrics::default());
    }

    #[test]
    fn shared_runs_overlap_unless_the_lines_share_an_end() {
        let apart = vec![vec![(0.0, 0.0), (100.0, 0.0)], vec![(20.0, 0.0), (80.0, 0.0)]];
        assert_eq!(measure(&apart).overlaps, 1);
        let tree = vec![vec![(0.0, 0.0), (0.0, 20.0), (-50.0, 20.0), (-50.0, 40.0)], vec![(0.0, 0.0), (0.0, 20.0), (50.0, 20.0), (50.0, 40.0)]];
        assert_eq!(measure(&tree), Metrics { crossings: 0, overlaps: 0, bends: 4 });
    }

    #[test]
    fn bends_count_corners_only() {
        let path = vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (20.0, 10.0), (20.0, 10.0), (30.0, 10.0)];
        assert_eq!(bends(&path), 2);
        let paths = vec![path.clone(), vec![(15.0, -5.0), (15.0, 5.0)]];
        assert_eq!(measure(&paths), Metrics { crossings: 1, overlaps: 0, bends: 2 });
    }

    #[test]
    fn merged_ends_leave_one_point_together() {
        let fan = vec![vec![(0.0, 0.0), (0.0, 20.0), (-50.0, 20.0)], vec![(0.0, 0.0), (0.0, 30.0), (50.0, 30.0)]];
        assert_eq!(merged_ends(&fan), 1);
        let apart = vec![vec![(0.0, 0.0), (0.0, 20.0)], vec![(0.0, 0.0), (20.0, 0.0)]];
        assert_eq!(merged_ends(&apart), 0);
        let chain = vec![vec![(0.0, 0.0), (0.0, 20.0)], vec![(0.0, 20.0), (0.0, 40.0)]];
        assert_eq!(merged_ends(&chain), 0);
    }

    #[test]
    fn close_runs_are_near_but_apart() {
        let a = vec![(0.0, 0.0), (100.0, 0.0)];
        assert_eq!(close_runs(&a, &[(0.0, 4.0), (100.0, 4.0)], 8.0), 1);
        assert_eq!(close_runs(&a, &[(0.0, 8.0), (100.0, 8.0)], 8.0), 0);
        assert_eq!(close_runs(&a, &[(10.0, 0.0), (90.0, 0.0)], 8.0), 0);
    }

    fn boxed(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn a_line_ending_on_a_shape_does_not_pass_through_it() {
        let s = Scene { lines: vec![vec![(50.0, 0.0), (50.0, 20.0)], vec![(50.0, 60.0), (50.0, 120.0)]], solids: vec![Solid { rect: boxed(0.0, 20.0, 100.0, 40.0), across: false }], ..Scene::default() };
        assert_eq!(assess(&s).through, 0);
        let s = Scene { lines: vec![vec![(50.0, 0.0), (50.0, 120.0)]], ..s };
        assert_eq!(assess(&s).through, 1);
    }

    #[test]
    fn labels_keep_clear_of_strokes_marks_and_each_other() {
        let label = boxed(0.0, 0.0, 40.0, 10.0);
        let s = Scene { labels: vec![label], rules: vec![vec![(41.0, -20.0), (41.0, 20.0)]], ..Scene::default() };
        assert_eq!(assess(&s).label_lines, 1, "a stroke 1px away bites into the halo");
        let s = Scene { rules: vec![vec![(43.0, -20.0), (43.0, 20.0)]], ..s };
        assert_eq!(assess(&s).label_lines, 0);
        let s = Scene { labels: vec![label, boxed(30.0, 5.0, 20.0, 10.0)], marks: vec![boxed(-6.0, 0.0, 5.0, 5.0)], ..Scene::default() };
        assert_eq!((assess(&s).label_labels, assess(&s).label_lines), (1, 1));
    }

    #[test]
    fn shapes_must_not_cover_backs_or_cross_borders_unless_allowed() {
        let border = vec![vec![(50.0, 0.0), (50.0, 100.0)]];
        let bar = Solid { rect: boxed(0.0, 40.0, 100.0, 6.0), across: true };
        let note = Solid { rect: boxed(40.0, 60.0, 30.0, 20.0), across: false };
        let s = Scene { solids: vec![bar, note], backs: vec![boxed(60.0, 70.0, 12.0, 40.0)], borders: border, ..Scene::default() };
        assert_eq!(assess(&s).shapes, 2, "the note crosses the border and covers the back; the bar may span");
    }

    #[test]
    fn a_label_reads_with_the_nearest_line() {
        let own = vec![(0.0, 0.0), (0.0, 100.0)];
        let other = vec![(60.0, 0.0), (60.0, 100.0)];
        let label = boxed(8.0, 40.0, 30.0, 10.0);
        let s = Scene { lines: vec![own, other], labels: vec![label], owners: vec![Some(0)], ..Scene::default() };
        assert_eq!(read_labels(&s), Reading { count: 1, distance: 8, ..Reading::default() });
        let between = Scene { labels: vec![boxed(15.0, 40.0, 30.0, 10.0)], ..s.clone() };
        assert_eq!(read_labels(&between).ambiguous, 1, "15px from each line");
        let far = Scene { labels: vec![boxed(14.0, 40.0, 10.0, 10.0)], ..s.clone() };
        assert_eq!((read_labels(&far).far, read_labels(&far).ambiguous), (1, 0));
        let free = Scene { owners: vec![None], ..s };
        assert_eq!(read_labels(&free), Reading::default());
    }

    #[test]
    fn text_must_fit_and_gaps_should_match() {
        let s = Scene { texts: vec![(boxed(2.0, 2.0, 10.0, 10.0), boxed(0.0, 0.0, 20.0, 20.0)), (boxed(15.0, 2.0, 10.0, 10.0), boxed(0.0, 0.0, 20.0, 20.0))], gaps: vec![32.0, 34.0, 32.0, 80.0], ..Scene::default() };
        assert_eq!((assess(&s).overflow, assess(&s).uneven), (1, 1));
    }
}
