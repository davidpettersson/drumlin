// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Alignment after layout: boxes move across the flow, with everything inside
//! them, so that the two ends of a lone connector meet the middle of their sides
//! on one line, and a parent sits centred over its children (Drawing
//! principles, Lines: meet the middle; Flow: centre the parent). Moves are whole
//! grid steps, keep the ranks and the order within them, and never bring a box
//! nearer a neighbour than the item gap. No routing is needed: a move is judged
//! by how many connectors it lines up, then by how many parents it centres.

use super::ends::{centred_over, centres_over};
use super::geometry::{snap, Direction, Rect, GRID};
use super::layered::Insets;

/// Where an end may meet its side across the flow: a point for an end alone on
/// its side or at a port, a span for an end that shares its side with others.
/// It moves with box `node`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reach {
    pub node: usize,
    pub lo: f64,
    pub hi: f64,
}

/// What the alignment needs to know. Every vector is indexed like the rects.
#[derive(Clone)]
pub struct Align {
    pub direction: Direction,
    pub parents: Vec<Option<usize>>,
    /// Each container's insets: a moved box stays inside its container's content.
    pub insets: Vec<Insets>,
    /// Room each box takes before and after itself along the flow, for stubs
    /// and port names: boxes whose extents overlap along the flow keep apart.
    pub margins: Vec<(f64, f64)>,
    /// The least gap between neighbours across the flow.
    pub gap: f64,
    /// The two ends of each connector that faces along the flow.
    pub pulls: Vec<(Reach, Reach)>,
    /// Boxes linked to several others further along the flow, or from several
    /// others before it: each should sit centred across the flow on its group.
    pub families: Vec<(usize, Vec<usize>)>,
    /// `(note, anchor)` pairs: a note moves with its anchor.
    pub tied: Vec<(usize, usize)>,
    /// Whether a box may move with the boxes a lined-up line runs on to
    /// (`train`), as lining up leaders needs; balanced fans move one at a time.
    pub trains: bool,
}

/// The most rounds of moves.
const ROUNDS: usize = 24;

fn across(direction: Direction, r: &Rect) -> (f64, f64) {
    if direction == Direction::Right { (r.y, r.bottom()) } else { (r.x, r.right()) }
}

fn along(direction: Direction, r: &Rect) -> (f64, f64) {
    if direction == Direction::Right { (r.x, r.right()) } else { (r.y, r.bottom()) }
}

fn shifted(direction: Direction, r: Rect, d: f64) -> Rect {
    if direction == Direction::Right { Rect { y: r.y + d, ..r } } else { Rect { x: r.x + d, ..r } }
}

fn ancestry(a: &Align, n: usize) -> Vec<usize> {
    std::iter::successors(Some(n), |&x| a.parents[x]).collect()
}

/// The children of the deepest box holding both `x` and `y` that hold each,
/// when neither holds the other.
fn siblings(a: &Align, x: usize, y: usize) -> Option<(usize, usize)> {
    let (ux, uy) = (ancestry(a, x), ancestry(a, y));
    let common = ux.iter().find(|n| uy.contains(n))?;
    let below = |chain: &[usize]| chain.iter().position(|n| n == common).and_then(|i| i.checked_sub(1)).map(|i| chain[i]);
    Some((below(&ux)?, below(&uy)?))
}

/// A reach where it stands now, given the rects it started from.
fn placed(a: &Align, start: &[Rect], rects: &[Rect], r: &Reach) -> (f64, f64) {
    let d = across(a.direction, &rects[r.node]).0 - across(a.direction, &start[r.node]).0;
    (r.lo + d, r.hi + d)
}

fn meets(x: (f64, f64), y: (f64, f64)) -> bool {
    x.0 <= y.1 + 0.5 && y.0 <= x.1 + 0.5
}

/// Lined-up connectors and centred families, which count alike: lining up one
/// connector is not worth taking a parent off the middle of its children.
/// Higher is better.
fn score(a: &Align, start: &[Rect], rects: &[Rect]) -> usize {
    let lined = a.pulls.iter().filter(|(x, y)| meets(placed(a, start, rects, x), placed(a, start, rects, y))).count();
    let centred = a.families.iter().filter(|(p, children)| centred_over(a.direction, rects, *p, children)).count();
    lined + centred
}

/// Every box with two or more others linked to it from one side along the
/// flow, wholly before or wholly beyond it, in any container: a parent over its
/// children, or a box under the parents that feed it. Each comes with the
/// members it should sit centred on: all of them for a balanced fan, or the one
/// member that leads it (`lead`), so the main line runs straight and the other
/// members sit beside it. `lone` lists the pairs of boxes joined by a lone
/// connector (both ends alone on their sides, or at ports); a family led by
/// such a connector is left to it.
pub fn families(direction: Direction, rects: &[Rect], links: &[(usize, usize)], lone: &[(usize, usize)]) -> Vec<(usize, Vec<usize>)> {
    grouped(direction, rects, links, lone, &[], true)
}

/// As `families`, where some links pass through a port (`bridged`): a chain
/// through a container is one chain, so its length counts across the border.
/// A member that is reached only through a port never wins a tie by the
/// chain-through rule: equal lines into a port stay a balanced fan.
pub fn families_through(direction: Direction, rects: &[Rect], links: &[(usize, usize)], lone: &[(usize, usize)], bridged: &[(usize, usize)]) -> Vec<(usize, Vec<usize>)> {
    grouped(direction, rects, links, lone, bridged, true)
}

/// Every family centred on all its members, as a balanced fan, whatever leads
/// it: the fallback where lining up the leaders routes worse.
pub fn balanced(direction: Direction, rects: &[Rect], links: &[(usize, usize)]) -> Vec<(usize, Vec<usize>)> {
    grouped(direction, rects, links, &[], &[], false)
}

fn grouped(direction: Direction, rects: &[Rect], links: &[(usize, usize)], lone: &[(usize, usize)], bridged: &[(usize, usize)], led: bool) -> Vec<(usize, Vec<usize>)> {
    let before = |p: &Rect, c: &Rect| if direction == Direction::Right { c.x >= p.right() } else { c.y >= p.bottom() };
    let group = |n: usize, outgoing: bool| {
        let mut found: Vec<usize> = links
            .iter()
            .filter_map(|&(a, b)| match outgoing {
                true if a == n && b != n && before(&rects[a], &rects[b]) => Some(b),
                false if b == n && a != n && before(&rects[a], &rects[b]) => Some(a),
                _ => None,
            })
            .collect();
        found.sort_unstable();
        found.dedup();
        let centre_on = (found.len() >= 2).then(|| if led { lead(direction, rects, links, lone, bridged, n, found, outgoing) } else { found })?;
        // A lone connector to the leader lines it up already, at its ends,
        // which centring the boxes would undo when an end is off the middle.
        let pulled = matches!(centre_on[..], [c] if lone.contains(&(n, c)) || lone.contains(&(c, n)));
        (!pulled).then_some((n, centre_on))
    };
    (0..rects.len()).flat_map(|n| [group(n, true), group(n, false)].into_iter().flatten()).collect()
}

/// The boxes on the longest path of links from each box onward along the flow
/// (`forward`) or back against it, itself included. Links count in either
/// direction; only where their boxes lie wholly one beyond the other.
fn path_lengths(direction: Direction, rects: &[Rect], links: &[(usize, usize)], forward: bool) -> Vec<usize> {
    let start = |r: &Rect| if direction == Direction::Right { r.x } else { r.y };
    let beyond = |p: &Rect, c: &Rect| if direction == Direction::Right { c.x >= p.right() } else { c.y >= p.bottom() };
    let next = |x: usize, y: usize| if forward { beyond(&rects[x], &rects[y]) } else { beyond(&rects[y], &rects[x]) };
    let mut by_start: Vec<usize> = (0..rects.len()).collect();
    by_start.sort_by(|&a, &b| start(&rects[a]).total_cmp(&start(&rects[b])).then(a.cmp(&b)));
    if forward {
        by_start.reverse();
    }
    by_start.into_iter().fold(vec![1usize; rects.len()], |mut length, x| {
        let onward = links.iter().flat_map(|&(a, b)| [(a == x).then_some(b), (b == x).then_some(a)]).flatten().filter(|&y| y != x && next(x, y));
        length[x] = 1 + onward.map(|y| length[y]).max().unwrap_or(0);
        length
    })
}

/// The members a family's box should sit centred on (Drawing principles, Flow:
/// centre the parent; one straight, one beside). In order: the one member on
/// the longest path away from the box, which continues the main flow; else the
/// one member joined to it by a lone connector, which can run straight; else,
/// when the flow reaches the box from one box on the other side, so that a
/// chain runs through it and it holds no boxes, the middle of the members tied on the longest path in
/// their order across the flow (the first of two); else all of them, for a
/// balanced fan.
#[allow(clippy::too_many_arguments)]
fn lead(direction: Direction, rects: &[Rect], links: &[(usize, usize)], lone: &[(usize, usize)], bridged: &[(usize, usize)], n: usize, members: Vec<usize>, outgoing: bool) -> Vec<usize> {
    let length = path_lengths(direction, rects, links, outgoing);
    let best = members.iter().map(|&c| length[c]).max().unwrap_or(0);
    let mut tied: Vec<usize> = members.iter().copied().filter(|&c| length[c] == best).collect();
    if tied.len() == 1 {
        return tied;
    }
    let joined: Vec<usize> = tied.iter().copied().filter(|&c| lone.contains(&(n, c)) || lone.contains(&(c, n))).collect();
    if joined.len() == 1 {
        return joined;
    }
    if tied.iter().any(|&c| bridged.contains(&(n, c)) || bridged.contains(&(c, n))) {
        return members;
    }
    let beyond = |p: &Rect, c: &Rect| if direction == Direction::Right { c.x >= p.right() } else { c.y >= p.bottom() };
    let mut behind: Vec<usize> = links
        .iter()
        .flat_map(|&(a, b)| [(a == n).then_some(b), (b == n).then_some(a)])
        .flatten()
        .filter(|&y| y != n && if outgoing { beyond(&rects[y], &rects[n]) } else { beyond(&rects[n], &rects[y]) })
        .collect();
    behind.sort_unstable();
    behind.dedup();
    // A container's lines meet its contents, not its middle, so no chain runs
    // through its centre.
    let r = rects[n];
    let holds = rects.iter().enumerate().any(|(m, o)| m != n && o.x >= r.x && o.y >= r.y && o.right() <= r.right() && o.bottom() <= r.bottom() && (o.w < r.w || o.h < r.h));
    if behind.len() != 1 || holds {
        return members;
    }
    let across = |r: &Rect| if direction == Direction::Right { r.center().1 } else { r.center().0 };
    tied.sort_by(|&a, &b| across(&rects[a]).total_cmp(&across(&rects[b])).then(a.cmp(&b)));
    vec![tied[(tied.len() - 1) / 2]]
}

/// Box `s` with the notes tied to anything inside it.
fn with_notes(a: &Align, s: usize) -> Vec<usize> {
    let within = |n: usize, root: usize| ancestry(a, n).contains(&root);
    let notes = a.tied.iter().filter(|&&(note, anchor)| within(anchor, s) && a.parents[note] == a.parents[s] && !within(note, s)).map(|&(note, _)| note);
    std::iter::once(s).chain(notes).collect()
}

/// Every box inside one of `tops`.
fn inside_of(a: &Align, tops: &[usize]) -> Vec<usize> {
    (0..a.parents.len()).filter(|&n| ancestry(a, n).iter().any(|x| tops.contains(x))).collect()
}

/// The boxes that move when the boxes `train` (all in one container, the
/// first named `s`) move by `d`: they and their tied notes, and any neighbour
/// in the way, which is pushed along with them, each with all it holds.
fn moving(a: &Align, rects: &[Rect], train: &[usize], d: f64) -> Vec<usize> {
    let s = train[0];
    let container = a.parents[s];
    let extent = |n: usize, r: &Rect| {
        let (lo, hi) = along(a.direction, r);
        (lo - a.margins[n].0, hi + a.margins[n].1)
    };
    let gap = |p: (f64, f64), q: (f64, f64)| (q.0 - p.1).max(p.0 - q.1);
    let siblings: Vec<usize> = (0..rects.len()).filter(|&n| n != s && a.parents[n] == container && container.is_some()).collect();
    let mut tops: Vec<usize> = train.iter().flat_map(|&t| with_notes(a, t)).collect();
    loop {
        let pushed = siblings.iter().copied().filter(|o| !tops.contains(o)).find(|&o| {
            tops.iter().any(|&t| {
                let (old, new) = (rects[t], shifted(a.direction, rects[t], d));
                let (ot, oo) = (across(a.direction, &old), across(a.direction, &rects[o]));
                let ahead = if d > 0.0 { oo.0 >= ot.1 } else { oo.1 <= ot.0 };
                let side_by_side = gap(extent(t, &new), extent(o, &rects[o])) < 0.0;
                ahead && side_by_side && gap(across(a.direction, &new), oo) < a.gap.min(gap(ot, oo))
            })
        });
        match pushed {
            Some(o) => tops.extend(with_notes(a, o)),
            None => return inside_of(a, &tops),
        }
    }
}

/// The rects after moving the boxes `train` by `d`, pushing what is in the way, if every
/// moved box stays inside its container and apart from the boxes that stay.
fn shift(a: &Align, rects: &[Rect], train: &[usize], d: f64) -> Option<Vec<Rect>> {
    let moved = moving(a, rects, train, d);
    let container = a.parents[train[0]]?;
    let tops: Vec<usize> = moved.iter().copied().filter(|&n| a.parents[n] == Some(container)).collect();
    let others: Vec<usize> = (0..rects.len()).filter(|&n| a.parents[n] == Some(container) && !moved.contains(&n)).collect();
    let extent = |n: usize, r: &Rect| {
        let (lo, hi) = along(a.direction, r);
        (lo - a.margins[n].0, hi + a.margins[n].1)
    };
    let gap = |p: (f64, f64), q: (f64, f64)| (q.0 - p.1).max(p.0 - q.1);
    let fits = tops.iter().all(|&t| {
        let (old, new) = (rects[t], shifted(a.direction, rects[t], d));
        let apart = others.iter().all(|&o| {
            let side_by_side = gap(extent(t, &new), extent(o, &rects[o])) < 0.0;
            !side_by_side || gap(across(a.direction, &new), across(a.direction, &rects[o])) >= a.gap.min(gap(across(a.direction, &old), across(a.direction, &rects[o])))
        });
        let inside = container == 0 || {
            let i = a.insets[container];
            let c = rects[container];
            let room = Rect { x: c.x + i.left, y: c.y + i.top, w: c.w - i.left - i.right, h: c.h - i.top - i.bottom };
            let (lo, hi) = across(a.direction, &room);
            let (was, now) = (across(a.direction, &old), across(a.direction, &new));
            (now.0 >= lo || now.0 >= was.0) && (now.1 <= hi || now.1 <= was.1)
        };
        apart && inside
    });
    fits.then(|| (0..rects.len()).map(|n| if moved.contains(&n) { shifted(a.direction, rects[n], d) } else { rects[n] }).collect())
}

/// The whole grid step nearest zero in `[lo, hi]`, if there is one.
fn step_in(lo: f64, hi: f64) -> Option<f64> {
    let first = (lo / GRID).ceil() * GRID;
    let last = (hi / GRID).floor() * GRID;
    let nearest = if first > 0.0 { first } else if last < 0.0 { last } else { 0.0 };
    (first <= last && nearest != 0.0).then_some(nearest)
}

/// Moves that would line up a connector or centre a parent.
fn candidates(a: &Align, start: &[Rect], rects: &[Rect]) -> Vec<(usize, f64)> {
    let lines = a.pulls.iter().filter(|(x, y)| !meets(placed(a, start, rects, x), placed(a, start, rects, y))).flat_map(|(x, y)| {
        let (px, py) = (placed(a, start, rects, x), placed(a, start, rects, y));
        let pair = siblings(a, x.node, y.node);
        let options = pair.map(|(sx, sy)| [step_in(py.0 - px.1, py.1 - px.0).map(|d| (sx, d)), step_in(px.0 - py.1, px.1 - py.0).map(|d| (sy, d))]);
        options.into_iter().flatten().flatten()
    });
    let parents = a.families.iter().filter(|(p, children)| !centred_over(a.direction, rects, *p, children)).flat_map(|(p, children)| {
        let own = (across(a.direction, &rects[*p]).0 + across(a.direction, &rects[*p]).1) / 2.0;
        centres_over(a.direction, rects, children).into_iter().map(move |t| (*p, snap(t - own))).filter(|&(_, d)| d != 0.0)
    });
    // A family led by one member also lines up by moving that member.
    let leaders = a.families.iter().filter(|(p, children)| children.len() == 1 && !centred_over(a.direction, rects, *p, children)).map(|(p, children)| {
        let centre = |n: usize| (across(a.direction, &rects[n]).0 + across(a.direction, &rects[n]).1) / 2.0;
        (children[0], snap(centre(*p) - centre(children[0])))
    });
    let mut all: Vec<(usize, f64)> = lines.chain(parents).chain(leaders.filter(|&(_, d)| d != 0.0)).collect();
    all.sort_by(|p, q| p.0.cmp(&q.0).then(p.1.total_cmp(&q.1)));
    all.dedup();
    all
}

/// The boxes a straight line runs on to from `s`: those joined to it, one after
/// another, by lone connectors that are lined up now, wholly beyond `s` along
/// the flow (`forward`) or wholly before it, in its container. Moving `s` with
/// them keeps their lines straight (Drawing principles, Flow: keep a column).
fn train(a: &Align, start: &[Rect], rects: &[Rect], s: usize, forward: bool) -> Vec<usize> {
    let beyond = |p: &Rect, c: &Rect| if a.direction == Direction::Right { c.x >= p.right() } else { c.y >= p.bottom() };
    let onward = |x: usize, y: usize| if forward { beyond(&rects[x], &rects[y]) } else { beyond(&rects[y], &rects[x]) };
    let lined: Vec<(usize, usize)> = a
        .pulls
        .iter()
        .filter(|(x, y)| meets(placed(a, start, rects, x), placed(a, start, rects, y)) && a.parents[x.node] == a.parents[s] && a.parents[y.node] == a.parents[s])
        .map(|(x, y)| (x.node, y.node))
        .collect();
    let mut found = vec![s];
    let mut i = 0;
    while i < found.len() {
        let x = found[i];
        let next: Vec<usize> = lined.iter().flat_map(|&(p, q)| [(p == x).then_some(q), (q == x).then_some(p)]).flatten().filter(|&y| onward(x, y) && !found.contains(&y)).collect();
        found.extend(next);
        i += 1;
    }
    found
}

/// Moves boxes across the flow, one at a time or with the boxes a lined-up
/// line runs on to (`train`), while a move lines up more connectors, or as
/// many and centres more parents.
pub fn align(rects: Vec<Rect>, a: &Align) -> Vec<Rect> {
    let start = rects.clone();
    (0..ROUNDS).try_fold(rects, |rects, _| {
        let now = score(a, &start, &rects);
        let trains = candidates(a, &start, &rects).into_iter().flat_map(|(s, d)| {
            let mut options = if a.trains { vec![vec![s], train(a, &start, &rects, s, true), train(a, &start, &rects, s, false)] } else { vec![vec![s]] };
            options.dedup();
            options.into_iter().map(move |t| (t, d))
        });
        let tried = trains.filter_map(|(t, d)| {
            let next = shift(a, &rects, &t, d)?;
            let after = score(a, &start, &next);
            (after > now).then(|| (after, d.abs(), moving(a, &rects, &t, d).len(), t[0], next))
        });
        let best = tried.min_by(|p, q| q.0.cmp(&p.0).then(p.1.total_cmp(&q.1)).then(p.2.cmp(&q.2)).then(p.3.cmp(&q.3)));
        match best {
            Some((_, _, _, _, next)) => Ok(next),
            None => Err(rects),
        }
    })
    .unwrap_or_else(|rects| rects)
}

/// `rects` with the root, box 0, grown to hold every top-level box, which
/// may have moved past its edges.
pub fn with_root(rects: Vec<Rect>, parents: &[Option<usize>]) -> Vec<Rect> {
    let tops: Vec<Rect> = (1..rects.len()).filter(|&n| parents[n] == Some(0)).map(|n| rects[n]).collect();
    let root = tops.iter().fold(rects[0], |r, t| {
        let (x, y) = (r.x.min(t.x), r.y.min(t.y));
        Rect { x, y, w: r.right().max(t.right()) - x, h: r.bottom().max(t.bottom()) - y }
    });
    std::iter::once(root).chain(rects.into_iter().skip(1)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    fn room(pulls: Vec<(Reach, Reach)>, families: Vec<(usize, Vec<usize>)>, count: usize) -> Align {
        Align {
            direction: Direction::Right,
            parents: std::iter::once(None).chain(std::iter::repeat(Some(0)).take(count - 1)).collect(),
            insets: vec![Insets::default(); count],
            margins: vec![(0.0, 0.0); count],
            gap: 40.0,
            pulls,
            families,
            tied: Vec::new(),
            trains: false,
        }
    }

    #[test]
    fn a_box_moves_so_a_lone_connector_meets_both_middles() {
        let rects = vec![Rect::default(), rect(0.0, 0.0, 100.0, 48.0), rect(200.0, 24.0, 100.0, 64.0)];
        let point = |node: usize, at: f64| Reach { node, lo: at, hi: at };
        let a = room(vec![(point(1, 24.0), point(2, 56.0))], Vec::new(), 3);
        let out = align(rects, &a);
        assert_eq!(out[1].center().1, out[2].center().1);
    }

    #[test]
    fn a_parent_moves_over_its_children() {
        let rects = vec![Rect::default(), rect(0.0, 0.0, 100.0, 48.0), rect(200.0, 0.0, 100.0, 48.0), rect(200.0, 96.0, 100.0, 48.0)];
        let a = room(Vec::new(), vec![(1, vec![2, 3])], 4);
        let out = align(rects, &a);
        assert_eq!(out[1].center().1, 72.0);
    }

    #[test]
    fn a_box_keeps_its_gap_from_a_neighbour() {
        let rects = vec![Rect::default(), rect(0.0, 0.0, 100.0, 48.0), rect(0.0, 88.0, 100.0, 48.0), rect(200.0, 40.0, 100.0, 48.0)];
        let point = |node: usize, at: f64| Reach { node, lo: at, hi: at };
        let a = room(vec![(point(1, 24.0), point(3, 64.0))], Vec::new(), 4);
        let out = align(rects.clone(), &a);
        assert_eq!(out[1], rects[1], "the box below leaves no room");
        assert_eq!(out[3].center().1, 24.0, "the other end moves instead");
    }

    fn right(rects: &[Rect]) -> Vec<Rect> {
        std::iter::once(Rect::default()).chain(rects.iter().copied()).collect()
    }

    #[test]
    fn the_member_on_the_longest_path_leads_a_family() {
        // 1 fans out to 2 and 3; 3 goes on to 4, so 3 continues the flow.
        let rects = right(&[rect(0.0, 48.0, 100.0, 48.0), rect(200.0, 0.0, 100.0, 48.0), rect(200.0, 96.0, 100.0, 48.0), rect(400.0, 96.0, 100.0, 48.0)]);
        let links = [(1, 2), (1, 3), (3, 4)];
        assert_eq!(families(Direction::Right, &rects, &links, &[]), vec![(1, vec![3])]);
        assert_eq!(balanced(Direction::Right, &rects, &links), vec![(1, vec![2, 3])]);
    }

    #[test]
    fn a_chain_through_a_box_continues_into_the_first_of_equal_members() {
        // 1 feeds 2, which fans out to the equal leaves 3 (upper) and 4.
        let rects = right(&[rect(0.0, 48.0, 100.0, 48.0), rect(200.0, 48.0, 100.0, 48.0), rect(400.0, 0.0, 100.0, 48.0), rect(400.0, 96.0, 100.0, 48.0)]);
        let links = [(1, 2), (2, 3), (2, 4)];
        assert_eq!(families(Direction::Right, &rects, &links, &[]), vec![(2, vec![3])]);
    }

    #[test]
    fn a_source_over_equal_members_stays_a_balanced_fan() {
        let rects = right(&[rect(0.0, 48.0, 100.0, 48.0), rect(200.0, 0.0, 100.0, 48.0), rect(200.0, 96.0, 100.0, 48.0)]);
        let links = [(1, 2), (1, 3)];
        assert_eq!(families(Direction::Right, &rects, &links, &[]), vec![(1, vec![2, 3])]);
    }

    #[test]
    fn a_container_stays_a_balanced_fan() {
        // 2 holds 5, so no chain runs through its middle.
        let rects = right(&[rect(0.0, 48.0, 100.0, 48.0), rect(200.0, 0.0, 200.0, 160.0), rect(500.0, 0.0, 100.0, 48.0), rect(500.0, 112.0, 100.0, 48.0), rect(250.0, 50.0, 100.0, 48.0)]);
        let links = [(1, 2), (2, 3), (2, 4)];
        assert_eq!(families(Direction::Right, &rects, &links, &[]), vec![(2, vec![3, 4])]);
    }

    #[test]
    fn a_lone_connector_to_the_leader_replaces_the_family() {
        // 1 and 2 feed 3; 1's connector is lone, so it lines up by itself.
        let rects = right(&[rect(0.0, 0.0, 100.0, 48.0), rect(0.0, 96.0, 100.0, 48.0), rect(200.0, 48.0, 100.0, 48.0)]);
        let links = [(1, 3), (2, 3)];
        assert!(families(Direction::Right, &rects, &links, &[(1, 3)]).is_empty());
    }

    #[test]
    fn one_member_is_lined_up_and_the_other_sits_beside() {
        // Secure boot chain: 1 -> 2 -> {3, 4}; 2 lines up with 3, which moves
        // onto its line and pushes 4 along, rather than 2 leaving 1.
        let rects = right(&[rect(0.0, 48.0, 100.0, 48.0), rect(200.0, 48.0, 100.0, 48.0), rect(400.0, 0.0, 100.0, 48.0), rect(400.0, 96.0, 100.0, 48.0)]);
        let links = [(1, 2), (2, 3), (2, 4)];
        let point = |node: usize, at: f64| Reach { node, lo: at, hi: at };
        let mut a = room(vec![(point(1, 72.0), point(2, 72.0))], families(Direction::Right, &rects, &links, &[(1, 2)]), 5);
        a.trains = true;
        let out = align(rects, &a);
        assert_eq!(out[2].center().1, out[3].center().1, "the chain runs straight on");
        assert_eq!(out[1].center().1, out[2].center().1, "the line behind stays straight");
        assert!(out[4].y >= out[3].bottom() + 40.0, "the other member sits beside, a gap away");
    }

    #[test]
    fn a_box_moves_with_the_lines_it_is_lined_up_with() {
        // 1 and 2 should line up; 1 is lined up with 4 behind it and 2 with 3
        // beyond it. Moving either alone trades one straight line for
        // another; moving it with its lined-up partner straightens all three.
        let rects = right(&[rect(200.0, 0.0, 100.0, 48.0), rect(400.0, 96.0, 100.0, 48.0), rect(600.0, 96.0, 100.0, 48.0), rect(0.0, 0.0, 100.0, 48.0)]);
        let point = |node: usize, at: f64| Reach { node, lo: at, hi: at };
        let pulls = vec![(point(1, 24.0), point(2, 120.0)), (point(2, 120.0), point(3, 120.0)), (point(4, 24.0), point(1, 24.0))];
        let mut a = room(pulls, Vec::new(), 5);
        assert_eq!(align(rects.clone(), &a), rects, "one at a time, nothing gains");
        a.trains = true;
        let out = align(rects, &a);
        let centre = |n: usize| out[n].center().1;
        assert_eq!((centre(4), centre(1)), (centre(2), centre(3)));
    }


    #[test]
    fn a_chain_through_a_port_counts_across_the_border() {
        // 2 fans out to 3 and 4; 4 goes on through a port to 5 (bridged), so
        // 4 continues the flow. Two equal members reached through a port stay
        // a balanced fan.
        let rects = right(&[rect(0.0, 48.0, 100.0, 48.0), rect(200.0, 48.0, 100.0, 48.0), rect(400.0, 0.0, 100.0, 48.0), rect(400.0, 96.0, 100.0, 48.0), rect(600.0, 96.0, 100.0, 48.0)]);
        let links = [(1, 2), (2, 3), (2, 4), (4, 5)];
        assert_eq!(families_through(Direction::Right, &rects, &links, &[], &[(4, 5)]), vec![(2, vec![4])]);
        let fan = [(1, 3), (2, 3), (3, 4)];
        let rects = right(&[rect(0.0, 0.0, 100.0, 48.0), rect(0.0, 96.0, 100.0, 48.0), rect(200.0, 48.0, 100.0, 48.0), rect(400.0, 48.0, 100.0, 48.0)]);
        assert_eq!(families_through(Direction::Right, &rects, &fan, &[], &[(1, 3), (2, 3)]), vec![(3, vec![1, 2])]);
    }
}
