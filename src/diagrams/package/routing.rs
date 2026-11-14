// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Adapts a package diagram to the generic router. Ordinary relations get a slot
//! per end. Containment relations of one owner share a slot, and when all members
//! sit in the rank after the owner they form a tree with one trunk.

use super::layout::{insets, tab_size, SLOT, SLOT_INSET as INSET};
use super::model::*;
use crate::core::align::{self, Align, Reach};
use crate::core::ends::HEAD_ROOM;
use crate::core::geometry::{self, along_side, from_flow, note_link, side_middle, snap, to_flow, Direction, Point, Rect, Side};
use crate::core::glyphs::{ARROWHEAD, TRIANGLE};
use crate::core::grow::{self, content, Room};
use crate::core::layered::Layout;
use crate::core::router::{self, boundary_strips, facing, lone, settle_slots, slide_ranges, straighten, Body, Connection, Problem, Slot, Strips, Terminal};
use crate::diagrams::boxes;

/// How near a corner nudging may slide an end: an arrow still meets the straight
/// side, so it reads as attached to that side. Slots are first spread `INSET`
/// from the corners.
const CORNER_INSET: f64 = 8.0;
/// The circle-plus's radius, as drawn (`render`).
const ANCHOR_RADIUS: f64 = 6.0;

pub struct Routes {
    pub edges: Vec<Vec<Point>>,
    /// Connectors with no clear route, drawn directly.
    pub failed: Vec<usize>,
}

/// All connection points, with each edge's `[from, to]` slots.
struct Slots {
    slots: Vec<Slot>,
    /// The edge and end each slot serves, used to find its peer.
    holders: Vec<(usize, usize)>,
    ends: Vec<[usize; 2]>,
}

fn body_top(d: &Diagram, g: &Layout, n: usize) -> f64 {
    let r = g.rects[n];
    if d.is_package(n) { r.y + tab_size(&d.nodes[n]).1 } else { r.y }
}

/// Whether a leaf package takes ends on top of its tab. UML 12.2.4 draws the
/// tab as part of the package symbol, so an arrow from above may end on it.
fn tab_takes_ends(d: &Diagram, n: usize) -> bool {
    d.is_package(n) && d.nodes[n].children.is_empty()
}

/// The x where a package's tab ends and its body's top begins.
fn tab_right(d: &Diagram, g: &Layout, n: usize) -> f64 {
    g.rects[n].x + tab_size(&d.nodes[n]).0
}

/// The point of a slot on its box. A package's top side is below its tab,
/// except on top of the tab of a leaf package.
fn side_point(g: &Layout, d: &Diagram, slot: &Slot) -> Point {
    let on_tab = tab_takes_ends(d, slot.item) && slot.coord < tab_right(d, g, slot.item);
    match slot.side {
        Side::Top if !on_tab => (slot.coord, body_top(d, g, slot.item)),
        side => geometry::side_point(g.rects[slot.item], side, slot.coord),
    }
}

/// The usable span of a side, `inset` from its corners. The top of a package
/// with contents starts right of its tab; a leaf package's top includes the tab.
fn side_range(g: &Layout, d: &Diagram, node: usize, side: Side, inset: f64) -> (f64, f64) {
    let r = g.rects[node];
    match side {
        Side::Left | Side::Right => (body_top(d, g, node) + inset, r.bottom() - inset),
        Side::Top if tab_takes_ends(d, node) => (r.x + inset, r.right() - inset),
        Side::Top if d.is_package(node) => {
            let (lo, hi) = (tab_right(d, g, node) + INSET, r.right() - inset);
            (lo.min(hi), hi)
        }
        Side::Top => {
            let hi = r.right() - inset;
            (under_tab(d, g, node, hi).unwrap_or(r.x + inset), hi)
        }
        Side::Bottom => (r.x + inset, r.right() - inset),
    }
}

/// The span along a leaf package's top where no end may sit: the tab's corner
/// and the step down beside it, which an arrow would touch.
fn tab_step(d: &Diagram, g: &Layout, node: usize) -> Option<(f64, f64)> {
    tab_takes_ends(d, node).then(|| (tab_right(d, g, node) - CORNER_INSET, tab_right(d, g, node) + INSET))
}

/// The part of a slide range on the same side of the tab step as `coord`, so
/// nudging never slides an end onto the step.
fn clear_of_step(d: &Diagram, g: &Layout, slot: &Slot, (lo, hi): (f64, f64)) -> (f64, f64) {
    match tab_step(d, g, slot.item).filter(|_| slot.side == Side::Top) {
        Some((step_lo, _)) if slot.coord <= step_lo => (lo, hi.min(step_lo)),
        Some((_, step_hi)) if slot.coord >= step_hi => (lo.max(step_hi), hi),
        _ => (lo, hi),
    }
}

/// Moves slots off a leaf package's tab step, to the nearer free side of it.
fn off_tab_step(d: &Diagram, g: &Layout, slots: Vec<Slot>) -> Vec<Slot> {
    (0..slots.len()).fold(slots, |mut slots, i| {
        let own = slots[i].clone();
        let Some((lo, hi)) = tab_step(d, g, own.item).filter(|_| own.side == Side::Top) else { return slots };
        if own.coord <= lo || own.coord >= hi {
            return slots;
        }
        let (min, max) = side_range(g, d, own.item, own.side, INSET);
        let free = |c: f64| slots.iter().enumerate().all(|(k, s)| k == i || s.item != own.item || s.side != own.side || (s.coord - c).abs() >= SLOT);
        let mut options = [lo, hi];
        options.sort_by(|a, b| (a - own.coord).abs().total_cmp(&(b - own.coord).abs()));
        if let Some(c) = options.into_iter().find(|&c| c >= min && c <= max && free(c)) {
            slots[i].coord = c;
        }
        slots
    })
}

/// The start of a child's top side when it sits right under its parent's tab,
/// so connectors arriving from above clear the tab.
fn under_tab(d: &Diagram, g: &Layout, node: usize, hi: f64) -> Option<f64> {
    let parent = d.nodes[node].parent.filter(|&p| d.is_package(p))?;
    let (p, r) = (g.rects[parent], g.rects[node]);
    let tab_right = p.x + tab_size(&d.nodes[parent]).0;
    let near = r.y - (p.y + tab_size(&d.nodes[parent]).1) <= 32.0;
    let start = (r.x + INSET).max(tab_right + INSET);
    (near && start <= hi && r.x < tab_right + INSET).then_some(start)
}

/// Boxes that keep `straighten` from moving a slot beside a package's tab,
/// unless the tab takes ends itself.
fn body_bounds(d: &Diagram, g: &Layout) -> Vec<Rect> {
    (0..d.nodes.len())
        .map(|n| {
            let r = g.rects[n];
            if d.is_package(n) {
                let (w, h) = tab_size(&d.nodes[n]);
                let left = if tab_takes_ends(d, n) { 0.0 } else { w };
                Rect { x: r.x + left, y: r.y + h, w: r.w - left, h: r.h - h }
            } else {
                r
            }
        })
        .collect()
}

/// One slot per edge end, except that an owner's containment edges share one.
fn allocate(d: &Diagram, g: &Layout) -> Slots {
    let mut slots: Vec<Slot> = Vec::new();
    let mut holders: Vec<(usize, usize)> = Vec::new();
    let mut trunks: Vec<Option<usize>> = vec![None; d.nodes.len()];
    let ends = d
        .edges
        .iter()
        .enumerate()
        .map(|(e, edge)| {
            // A nested end leaves toward the enclosing boundary on the flow's far side.
            let (side_a, side_b) = if d.nests(edge) { (d.end_side(), d.end_side()) } else { facing(d.direction, &g.rects[edge.from], &g.rects[edge.to]) };
            let first = match (edge.line, trunks[edge.from]) {
                (Line::Containment, Some(shared)) => shared,
                _ => {
                    slots.push(Slot::centred(edge.from, side_a, g.rects[edge.from]));
                    holders.push((e, 0));
                    let slot = slots.len() - 1;
                    if edge.line == Line::Containment {
                        trunks[edge.from] = Some(slot);
                    }
                    slot
                }
            };
            slots.push(Slot::centred(edge.to, side_b, g.rects[edge.to]));
            holders.push((e, 1));
            [first, slots.len() - 1]
        })
        .collect();
    Slots { slots, holders, ends }
}

/// The middle of a side, or the nearest place to it that the side offers.
fn middle(g: &Layout, d: &Diagram, node: usize, side: Side) -> f64 {
    side_middle(g.rects[node], body_top(d, g, node), side, side_range(g, d, node, side, INSET))
}

/// Lets slots settle toward their peers. An end alone on its side sits at its
/// middle (Drawing principles, Lines: meet the middle), so containment ends
/// stay centred and the members of a tree sit evenly under its trunk.
fn settle(d: &Diagram, g: &Layout, table: Slots) -> Slots {
    let wanted = |slots: &[Slot], i: usize| {
        let (edge, end) = table.holders[i];
        if d.edges[edge].line == Line::Containment || (lone(slots, i) && d.edges[edge].line != Line::NoteLink) {
            middle(g, d, slots[i].item, slots[i].side)
        } else {
            along_side(side_point(g, d, &slots[table.ends[edge][1 - end]]), slots[i].side)
        }
    };
    let settled = settle_slots(table.slots.clone(), SLOT, |n, s| side_range(g, d, n, s, INSET), wanted);
    let straight = d.edges.iter().zip(&table.ends).filter(|(edge, &[i, j])| (edge.line != Line::Containment || members(d, edge.from).len() == 1) && !lone(&table.slots, i) && !lone(&table.slots, j));
    let bounds = body_bounds(d, g);
    let slots = straight.fold(settled, |slots, (_, &[i, j])| straighten(slots, i, j, &bounds, SLOT));
    Slots { slots: off_tab_step(d, g, separate_nested(d, g, &table, slots)), ..table }
}

/// The slots for a layout: allocated, then settled.
fn settled(d: &Diagram, g: &Layout) -> Slots {
    settle(d, g, allocate(d, g))
}

/// Moves a package's slot aside when a connector to one of its children would
/// otherwise run on the same line and read as a single arrow.
fn separate_nested(d: &Diagram, g: &Layout, table: &Slots, slots: Vec<Slot>) -> Vec<Slot> {
    (0..slots.len()).fold(slots, |mut slots, i| {
        let own = slots[i].clone();
        if d.nests(&d.edges[table.holders[i].0]) {
            return slots;
        }
        let (lo, hi) = side_range(g, d, own.item, own.side, INSET);
        let taken = |slots: &[Slot], coord: f64| {
            slots.iter().enumerate().any(|(k, s)| {
                k != i && s.side == own.side && ((s.item == own.item && (s.coord - coord).abs() < SLOT) || (s.item != own.item && d.contains(own.item, s.item) && (s.coord - coord).abs() < 1.0))
            })
        };
        if !d.nodes[own.item].children.is_empty() && taken(&slots, own.coord) {
            let options = [1.0, 2.0, 3.0].into_iter().flat_map(|k: f64| [own.coord + k * SLOT, own.coord - k * SLOT]);
            if let Some(coord) = options.filter(|&c| (lo..=hi).contains(&c)).find(|&c| !taken(&slots, c)) {
                slots[i].coord = coord;
                let (edge, end) = table.holders[i];
                let peer = table.ends[edge][1 - end];
                let (peer_lo, peer_hi) = side_range(g, d, slots[peer].item, slots[peer].side, INSET);
                let clear = |s: &Slot| s.item != slots[peer].item || s.side != slots[peer].side || (s.coord - coord).abs() >= SLOT;
                let free = slots.iter().enumerate().all(|(k, s)| k == peer || clear(s));
                if slots[peer].side == own.side.opposite() && (peer_lo..=peer_hi).contains(&coord) && free {
                    slots[peer].coord = coord;
                }
            }
        }
        slots
    })
}

fn members(d: &Diagram, owner: usize) -> Vec<usize> {
    (0..d.edges.len()).filter(|&e| d.edges[e].line == Line::Containment && d.edges[e].from == owner).collect()
}

fn owners(d: &Diagram) -> Vec<usize> {
    let mut found: Vec<usize> = d.edges.iter().filter(|edge| edge.line == Line::Containment).map(|edge| edge.from).collect();
    found.sort_unstable();
    found.dedup();
    found
}

/// A containment tree: its edges and their paths.
struct Tree {
    edges: Vec<usize>,
    paths: Vec<Vec<Point>>,
}

/// Drops consecutive duplicate points, which a straight drop produces.
fn tidy(points: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    out
}

/// A tree for `owner`, when all of its members sit past it along the flow and
/// nothing else occupies the band the rail runs through.
fn tree(d: &Diagram, g: &Layout, table: &Slots, owner: usize) -> Option<Tree> {
    let edges = members(d, owner);
    let dir = d.direction;
    let outer = d.end_side();
    let trunk_slot = &table.slots[table.ends[*edges.first()?][0]];
    let drops: Vec<&Slot> = edges.iter().map(|&e| &table.slots[table.ends[e][1]]).collect();
    if trunk_slot.side != outer || drops.iter().any(|s| s.side != outer.opposite()) {
        return None;
    }
    let origin = side_point(g, d, trunk_slot);
    let (owner_end, trunk_cross) = to_flow(dir, origin);
    let tips: Vec<Point> = drops.iter().map(|s| side_point(g, d, s)).collect();
    let nearest = tips.iter().map(|&p| to_flow(dir, p).0).fold(f64::MAX, f64::min);
    if nearest - owner_end < 32.0 {
        return None;
    }
    let rail = snap((owner_end + nearest) / 2.0);
    let crosses: Vec<f64> = tips.iter().map(|&p| to_flow(dir, p).1).chain([trunk_cross]).collect();
    let (lo, hi) = (crosses.iter().cloned().fold(f64::MAX, f64::min), crosses.iter().cloned().fold(f64::MIN, f64::max));
    let (a, b) = (from_flow(dir, owner_end, lo), from_flow(dir, nearest, hi));
    let band = Rect { x: a.0.min(b.0), y: a.1.min(b.1), w: (a.0 - b.0).abs(), h: (a.1 - b.1).abs() };
    let related = |n: usize| n == 0 || d.contains(n, owner) || edges.iter().any(|&e| d.contains(n, d.edges[e].to));
    if (1..d.nodes.len()).any(|n| !related(n) && g.rects[n].intersects(&band)) {
        return None;
    }
    let paths = tips
        .iter()
        .map(|&tip| {
            let drop_cross = to_flow(dir, tip).1;
            tidy(vec![origin, from_flow(dir, rail, trunk_cross), from_flow(dir, rail, drop_cross), tip])
        })
        .collect();
    Some(Tree { edges, paths })
}

/// Soft strips that keep other connectors from running along a tree's lines.
fn tree_strips(trees: &[Tree]) -> Strips {
    let segments: Vec<(Point, Point)> = trees.iter().flat_map(|t| t.paths.iter().flat_map(|p| p.windows(2).map(|w| (w[0], w[1])))).collect();
    let strip = |&(a, b): &(Point, Point)| Rect { x: a.0.min(b.0) - 4.0, y: a.1.min(b.1) - 4.0, w: (a.0 - b.0).abs() + 8.0, h: (a.1 - b.1).abs() + 8.0 };
    Strips {
        horizontal: segments.iter().filter(|(a, b)| a.1 == b.1).map(strip).collect(),
        vertical: segments.iter().filter(|(a, b)| a.0 == b.0).map(strip).collect(),
    }
}

/// A node's outline. A package keeps its tab free as a separate box.
fn node_obstacles(d: &Diagram, g: &Layout, n: usize) -> Vec<Rect> {
    boxes::outline(g.rects[n], d.is_package(n).then(|| tab_size(&d.nodes[n])))
}

fn connection(d: &Diagram, g: &Layout, table: &Slots, slides: &[(f64, f64)], avoid: &Strips, e: usize) -> Connection {
    let (a, b) = (d.edges[e].from, d.edges[e].to);
    let encloses = |n: usize| (n != a && d.contains(n, a)) || (n != b && d.contains(n, b));
    let (containers, solid): (Vec<usize>, Vec<usize>) = (1..d.nodes.len()).partition(|&n| encloses(n));
    let tabs = containers.iter().filter(|&&n| d.is_package(n)).map(|&n| node_obstacles(d, g, n)[1]);
    let walls = solid.iter().flat_map(|&n| node_obstacles(d, g, n));
    let boundary = boundary_strips(containers.iter().map(|&n| (g.rects[n], body_top(d, g, n))));
    let terminal = |index: usize| {
        let at = table.ends[e][index];
        let slot = &table.slots[at];
        // The enclosing end of a nested relation faces into the package.
        let inward = d.nests(&d.edges[e]) && !d.nodes[slot.item].children.is_empty() && d.contains(slot.item, if index == 0 { b } else { a });
        // A head or a circle-plus needs a straight run longer than itself.
        let decoration = match (index, d.edges[e].line, d.edges[e].head) {
            (0, Line::Containment, _) => 2.0 * ANCHOR_RADIUS,
            (1, _, Head::Open) => ARROWHEAD,
            (1, _, Head::Triangle) => TRIANGLE,
            _ => 0.0,
        };
        let placed = Terminal::at(side_point(g, d, slot), if inward { slot.side.opposite() } else { slot.side }, 0.0);
        let fixed = if decoration > 0.0 { placed.reaching(decoration + HEAD_ROOM) } else { placed };
        // Containment ends share a slot; other ends may slide when nudging straightens a line.
        if inward || d.edges[e].line == Line::Containment {
            return fixed;
        }
        let sliding = fixed.sliding(slides[at].0, slides[at].1);
        // An end on a plain leaf may move to another side to turn a Z into an
        // L; a package's top, which holds its tab, keeps the ends it has. An
        // end alone on its side stays there when the other end shares its
        // side: that line is a fan's branch. Every end knows its box, so an end
        // left alone can return to the middle.
        let n = slot.item;
        let other = table.ends[e][1 - index];
        let fan_branch = lone(&table.slots, at) && !lone(&table.slots, other);
        let leaf = d.nodes[n].children.is_empty() && d.nodes[n].kind != Kind::Note && !d.nests(&d.edges[e]) && !fan_branch;
        let r = g.rects[n];
        let sides = if leaf { [true, true, !d.is_package(n), true] } else { [false; 4] };
        sliding.on(Body { rect: Rect { y: body_top(d, g, n), h: r.bottom() - body_top(d, g, n), ..r }, inset: INSET, sides, head: 0.0 })
    };
    Connection {
        from: terminal(0),
        to: terminal(1),
        obstacles: walls.chain(tabs).collect(),
        strips: Strips {
            horizontal: boundary.horizontal.into_iter().chain(avoid.horizontal.iter().copied()).collect(),
            vertical: boundary.vertical.into_iter().chain(avoid.vertical.iter().copied()).collect(),
        },
    }
}

/// Moves boxes across the flow so that lone relations meet the middles of
/// their sides on one line and parents sit centred over their children
/// (`core::align`). Only lone relations pull: both ends alone on their sides.
pub fn aligned(d: &Diagram, g: Layout) -> Layout {
    let table = settled(d, &g);
    let across = |side: Side| side.is_vertical_edge() == (d.direction == Direction::Right);
    let kept: Vec<usize> = (0..d.edges.len()).filter(|&e| d.edges[e].line != Line::NoteLink && !d.nests(&d.edges[e])).collect();
    let reach = |e: usize, index: usize| {
        let i = table.ends[e][index];
        let slot = &table.slots[i];
        (lone(&table.slots, i) && across(slot.side)).then_some(Reach { node: slot.item, lo: slot.coord, hi: slot.coord })
    };
    let pulls: Vec<(Reach, Reach)> = kept.iter().filter_map(|&e| Some((reach(e, 0)?, reach(e, 1)?))).collect();
    let lone: Vec<(usize, usize)> = pulls.iter().map(|(a, b)| (a.node, b.node)).collect();
    let links: Vec<(usize, usize)> = kept.iter().map(|&e| (d.edges[e].from, d.edges[e].to)).collect();
    let tied = d.edges.iter().filter(|edge| edge.line == Line::NoteLink).map(|edge| (edge.from, edge.to)).collect();
    let room = Align {
        direction: d.direction,
        families: align::families(d.direction, &g.rects, &links, &lone),
        parents: d.nodes.iter().map(|node| node.parent).collect(),
        insets: (0..d.nodes.len()).map(|n| insets(d, n)).collect(),
        margins: vec![(0.0, 0.0); d.nodes.len()],
        gap: crate::core::layered::Spacing::default().item_gap,
        pulls,
        tied,
        trains: true,
    };
    // Alignment cannot see routes: keep it only when no route fails or crosses
    // more. Families are lined up with their leaders, or else centred on all
    // their members as balanced fans, whichever then routes with fewer bends
    // (the fans on a tie, so a drawing changes only for straighter lines; the
    // leaders only when they add no bends).
    let fallback = Align { families: align::balanced(d.direction, &g.rects, &links), trains: false, ..room.clone() };
    let standing = |l: &Layout| {
        let r = route(d, l);
        let m = crate::core::metrics::measure(&r.edges);
        ((r.failed.len(), m.crossings), m.bends)
    };
    let start = standing(&g);
    let tried = [room, fallback].map(|room| {
        let moved = Layout { rects: align::with_root(align::align(g.rects.clone(), &room), &room.parents), ..g.clone() };
        let s = standing(&moved);
        (moved, s)
    });
    let [(led, a), (fan, b)] = tried;
    // Lining up leaders must also not add bends overall; a balanced fan may
    // add bends, to meet the middles.
    match (a.0 <= start.0 && a.1 <= start.1, b.0 <= start.0) {
        (true, true) if b.1 <= a.1 => fan,
        (true, _) => led,
        (false, true) => fan,
        (false, false) => g,
    }
}

/// The aligned layout with chains in columns (Drawing principles, Flow: keep a
/// column) when it routes with fewer bends and no more failures, crossings or
/// overlaps than the layout without; otherwise the layout without. Long lines
/// back past a column can need more room than the column leaves, and a column
/// that straightens nothing is not worth moving boxes that were placed well.
pub fn arranged(d: &Diagram) -> Layout {
    let score = |l: &Layout| {
        let r = route(d, l);
        let m = crate::core::metrics::measure(&r.edges);
        ((r.failed.len(), m.crossings, m.overlaps), m.bends)
    };
    let [columned, plain] = super::layout::layouts(d).map(|l| aligned(d, l));
    let (c, p) = (score(&columned), score(&plain));
    if c.0 <= p.0 && c.1 < p.1 { columned } else { plain }
}

/// Grows end boxes where that lets a relation run straight (`core::grow`).
/// Notes keep their size.
pub fn grown(d: &Diagram, g: Layout) -> Layout {
    let room = Room {
        direction: d.direction,
        growable: (0..d.nodes.len()).map(|n| n != 0 && d.nodes[n].children.is_empty() && d.nodes[n].kind != Kind::Note).collect(),
        parents: d.nodes.iter().map(|node| node.parent).collect(),
        within: d.nodes.iter().map(|node| node.parent.map_or(g.rects[0], |p| content(g.rects[p], insets(d, p), 8.0))).collect(),
        keep_out: Vec::new(),
        ends: d.edges.iter().map(|edge| (edge.from, edge.to)).collect(),
        via: vec![None; d.edges.len()],
    };
    let rects = grow::grow(g.rects.clone(), &room, |rects| route(d, &Layout { rects: rects.to_vec(), labels: g.labels.clone() }).edges);
    Layout { rects, ..g }
}

pub fn route(d: &Diagram, g: &Layout) -> Routes {
    let table = settled(d, g);
    let trees: Vec<Tree> = owners(d).into_iter().filter_map(|owner| tree(d, g, &table, owner)).collect();
    let in_tree = |e: usize| trees.iter().any(|t| t.edges.contains(&e));
    let free: Vec<usize> = (0..d.edges.len()).filter(|&e| !in_tree(e)).collect();
    let avoid = tree_strips(&trees);
    let ranges = slide_ranges(&table.slots, router::SLIDE_GAP, |n, side| side_range(g, d, n, side, CORNER_INSET));
    // An end alone on its side stays at its middle.
    let slides: Vec<(f64, f64)> = table.slots.iter().zip(ranges).enumerate().map(|(i, (slot, range))| if lone(&table.slots, i) && d.edges[table.holders[i].0].line != Line::NoteLink { (slot.coord, slot.coord) } else { clear_of_step(d, g, slot, range) }).collect();
    let problem = Problem {
        bounds: g.rects[1..].to_vec(),
        walls: (1..d.nodes.len()).flat_map(|n| node_obstacles(d, g, n)).collect(),
        connections: free.iter().map(|&e| connection(d, g, &table, &slides, &avoid, e)).collect(),
        centred: true,
    };
    let solution = router::route(&problem);
    let routed = free.iter().copied().zip(solution.paths);
    let from_trees = trees.iter().flat_map(|t| t.edges.iter().copied().zip(t.paths.iter().cloned()));
    let mut edges = vec![Vec::new(); d.edges.len()];
    for (e, path) in routed.chain(from_trees) {
        // Note links are routed, so other lines keep clear of them, but drawn
        // straight (`geometry::note_link`).
        edges[e] = if d.edges[e].line == Line::NoteLink { note_link(g.rects[d.edges[e].from], g.rects[d.edges[e].to]).to_vec() } else { path };
    }
    let failed = solution.failed.into_iter().map(|i| free[i]).collect();
    Routes { edges, failed }
}
