# How layout works

This page describes how drumlin turns a parsed diagram into positions and lines: the steps of the
layered layout for component, package and state diagrams, the separate layouts of sequence and activity
diagrams, the layout engines, and how layout quality is measured. It is for contributors and curious
readers. The [drawing principles](../principles/introduction.md) say what a drawing should look like;
this page says how drumlin gets there, and each step links to the principles it enforces.

```drumlin,render-only
diagram component "Layout pipeline"
direction right

component Parser
component "Compound Layout" as layout
component "Columns and Notes" as columns
component Alignment
component "Slot Allocation" as slots
component Router
component Nudging
component "Label Placement" as labels
component "SVG Writer" as svg

Parser --> layout : model
layout --> columns : ranks
columns --> Alignment : rectangles
Alignment --> slots : rectangles
slots --> Router : terminals
Router --> Nudging : polylines
component "L over Z" as lz
Nudging --> lz : polylines
lz --> labels : polylines
labels --> svg
```

## Three layouts

- **Component, package and state diagrams** share the layered layout and router described under
  [Steps](#steps), and the small adapter helpers in `src/diagrams/boxes.rs`. Each kind keeps its own
  alignment, growth and routing stages. The first steps, up to placement, form a selectable
  [layout engine](#layout-engines).
- **Sequence diagrams** use a timeline layout: steps resolve execution depth, columns are spaced from
  labels and frames, and rows follow the page. Nothing is routed.
- **Activity diagrams** use a structured-flow layout driven by the block tree, with one measure stage and
  one place stage. In the measure stage each decision picks the arm that lies on its axis (`arm_plan`):
  the one arm that continues, the body beside an empty arm, the one arm in the decision's lane, or else
  the middle arm, the first declared of two. The other arms sit beside it in declared order and rejoin
  the merge from its sides, so the flow passes straight through both diamonds (see
  [Flow passes straight through a decision](../principles/lines.md#flow-passes-straight-through-a-decision)).
  A fork with an even count of branches stays symmetric about its axis. A `while` loop returns along a
  column left of its body; when the body ends in a merge whose left corner no branch rejoins, the return
  leaves by that corner (see
  [Take a return line around the outside](../principles/lines.md#take-a-return-line-around-the-outside)).

## Steps

The steps of the layered layout are described below in pipeline order: compound layout, ranking,
crossing reduction, placement (with columns and notes), alignment, box growth, slots, routing, nudging,
L over Z, and labels. Several steps (columns, alignment, box growth, labels after routing) route the
diagram more than once and keep the better result.

### Compound layout

Each container is laid out bottom-up as a layered graph of its direct children. An inner layout
becomes one large item in its parent. Component boxes are sized before layout to their name, stubs and
ports. A component diagram is also laid out with each side long enough for the connector
ends the flow will put on it (24px apart, 32px for named lines, 16px from the corners), and that layout
is kept when the other leaves an end nearer a corner than 16px or two ends nearer than 24px, and it routes
no worse. See [Leave room for what attaches](../principles/shapes.md#leave-room-for-what-attaches) and
[Group what belongs together](../principles/flow.md#group-what-belongs-together).

### Ranking

Connectors and hints assign each item a rank. Long edges get invisible dummy items that hold a
lane through the ranks between. See [Flow one way](../principles/flow.md#flow-one-way).

Hints come first, then connectors, and a connector that would close a cycle is turned round. The
default engine gives each item the rank after the furthest item that points to it, then moves an item
that nothing points to down next to what it points to (`rank::assign_ranks`). The `network-simplex`
engine solves the same problem for the least total weighted line length instead
(`simplex::network_simplex`); see [Layout engines](#layout-engines).

### Crossing reduction

Weighted median sweeps reorder each rank, alternating down and up, and a transpose pass swaps
neighbours while that removes crossings. The sweeps run twice, from the input order and from a
depth-first order, so a tree starts without crossings, and the best ordering seen wins. Hints and
notes are applied inside every sweep, so the ordering is scored as it will be drawn. See
[Order ranks to avoid crossings](../principles/flow.md#order-ranks-to-avoid-crossings).

### Placement

An order-preserving median solve aligns connected items and snaps everything to an 8px grid.
The dummies of each long edge then line up where both ends can take the line, so it runs
straight, or where one end can, so it bends only once. Edges between the same two items move as a
block. Pendant items, such as an initial state above its first state or a final state below its
last, move in line with what they hang from. State transition labels get a reserved place in the
gap between ranks. See [Keep long edges straight](../principles/flow.md#keep-long-edges-straight)
and [Reserve room for labels](../principles/labels.md#reserve-room-for-labels).

Before that, each item may get a column partner above and below it (`columns`). Every edge between
neighbouring ranks of items is a candidate, scored by the longest path through it, and the best go
first: an item with several onward edges takes one only when its path is strictly the longest or the item
is a decision (`Constraint::Pivot`, which takes its first branch), two columns between the same ranks
keep the order of their ends, and a bar (`Constraint::Spread`) starts no column unless it has one edge
onward (a join). In the median solve a
partner's position is the target, with a weight above everything else, and the dummies of the edge
between two partners follow it; afterwards each column moves onto one line where its rows have room
(`hold_columns`). A state diagram is laid out with and without columns and keeps them unless they route
with more crossings, overlaps or failed lines, first with every label reserved and, if that is worse,
with labels settled. Component and package diagrams keep columns only when they route with fewer bends
and no more crossings, overlaps, uncentred parents or cramped heads. See
[Keep a column](../principles/flow.md#keep-a-column).

### Notes

A note (`Constraint::Note`) takes no part in ranking, ordering or placement. It joins its anchor's rank
for the rank's depth only, and once everything else is placed (`place_notes`) it goes beside its anchor,
32px off, on a side where it keeps the item gap from every other item and the dummy gap from every long
edge in the rank; of two free sides the roomier wins. With neither free, it takes the side it misses by
less and pushes what lies beyond it outward. Component notes still use `Adjacent`. Note links are routed,
so other lines keep clear of them, and drawn as one straight line between the nearest points of note and
element (`geometry::note_link`). See [Notes come last](../principles/flow.md#notes-come-last) and
[Note links may be diagonal](../principles/lines.md#note-links-may-be-diagonal).

### Alignment

Component and package diagrams (`core::align`): after placement, boxes move across the flow, each with
everything it holds, in whole grid steps. A move is kept when it lines up more lone connectors or
centres more families than it costs; the two count alike. A lone connector is one whose two ends are
each alone on their side, or at a port: it lines up when both ends lie on one line, so it runs straight
between the middles of its sides. A port that follows a single connector inside its component counts as
part of the box that connector reaches, since it moves with it. A family is a box with two or more others
linked to it from one side along the flow, wholly before or wholly beyond it, in any container.

A family is centred on the member that leads it, when one does (`align::families`): the
one member on the longest path of links away from the box; else the one member joined to it by a lone
connector (then the connector alone lines it up and no family is counted); else, when exactly one box
lies on the box's other side and the box holds no others, so that a chain runs through it, the middle
of the members tied on the longest path in their order across the flow, the first of two. Otherwise the
family is centred on all its members, within 4px of their median or mean, as a balanced fan. A led family
may also line up by moving the leader. A box may move together with the boxes its lined-up lone
connectors run on to, on its far side or its near side (`train`), so that a straight chain stays
straight while one end of it moves.

The links a family and its longest path follow see through ports (`bridged_links`): a line
to a port that delegates links the box on its far side, the one box the port leads to inside or the one
box beyond it outside. A chain that runs through a container by its ports is then one chain, and a box
inside may lead a family of a box outside. A member reached only through a port never wins a tie by the
chain-through rule, so two equal lines into one port stay a balanced fan (`align::families_through`).
After the first alignment, a second one adds a pull for each line that leaves a delegating port to a box
whose side it shares with other ends: the line lines up when the port meets that side anywhere its ends
may sit (a span, not a point), so the spine runs straight on beyond the container where that breaks
nothing lined up already.

A moved box pushes the neighbours in its way along with it, never comes nearer a neighbour than the 40px
item gap (or the gap it already had), and stays inside its container's content area; the root grows to
hold what moved. Notes move with their anchors. No routing is needed for the moves. The diagram is then
routed three ways: unaligned, aligned with leaders (trains allowed), and aligned with every family a
balanced fan and boxes moving one at a time (alignment without leaders). Neither aligned layout may fail a
route or add a crossing; the leaders' layout may also add no bends and leave no more ends nearer than
16px to a corner of their box than the unaligned layout did, and where the leaders' layout has more such
ends than the balanced fans, the fans win. Of the two, the one with fewer bends
wins; on a tie, both layouts are grown ([Box growth](#box-growth)) and the one with fewer bends after
growth wins, the leaders on a further tie. See [Meet the middle](../principles/lines.md#meet-the-middle),
[Centre the parent](../principles/flow.md#centre-the-parent) and
[Line up the main branch](../principles/flow.md#line-up-the-main-branch).

State diagrams (`routing::points_lined_up`): after the column choice, a state moves across the
flow under a transition to or from an entry or exit point, and the state after a join lines up with the
state before its fork when every branch of the fork meets that join. Lines that already run between two
middles, and lines that meet a bar anywhere along it, hold. The moved layout is kept when it routes with
no more failures, crossings, overlaps or bends.

### Box growth

Where a connector jogs because its end boxes miss each other by a step or two, `core::grow` grows
one end box by up to 16px on a side toward the other end, into free space, and routes the diagram
again. A growth is kept only when that connector loses a bend, no crossing, overlap or bend total
rises, and no two connectors come to leave one point together. Growth across the flow is tried
first, and only growth that lets the two boxes share a span is routed. See
[Grow a box rather than bend a line](../principles/shapes.md#grow-a-box-rather-than-bend-a-line).

### Slots

Attachment points slide along box sides toward their peers. In component and package diagrams, an end
alone on its side sits at the middle instead, and nudging may not slide it; a port sits at the middle
unless connectors inside its component lead to it, and then lines up with them. Component ends that
share a side are spread evenly in their order and centred on the side as a group, unless that takes an
end off a straight line to another box. A container's side that lines from its contents leave through
counts as shared. Ends are spread 16px from corners, and nudging may slide a shared end to 8px from a
square corner. A package without visible contents also takes ends on top of its blank tab. A component
container pads a side whose port takes an arrow from inside to 32px, to fit the head's straight run. Ends
that share a left or right side are spread 32px apart where a line carries a name (`NAMED_SLOT`), else
24px. Groups are judged together: a group is centred when that keeps as many of its lines
straight as settling did, with the groups at the other ends of its lines centred too. A
stub that shares a side with lines goes to the end of the side its name faces (the top of a left or right
side, the right of a top or bottom side), leaving the middle to the lines.

Each end starts on the facing side (`router::facing`). One exception is made before
routing, in component diagrams (`secondary_sides`): a dashed dependency between two leaf components that
overlap across the flow, whose straight way runs through a box linked to neither end, and whose end shares
its side with another line or a stub, takes a free side across the flow (right, or bottom when the flow
runs right, unless taken). The solid lines keep the facing side, and the dashed line goes round outside
them. See
[Give the main line the main side](../principles/lines.md#give-the-main-line-the-main-side),
[Meet the middle](../principles/lines.md#meet-the-middle),
[Spread and order attachment points](../principles/lines.md#spread-and-order-attachment-points),
[Keep ends off corners](../principles/lines.md#keep-ends-off-corners) and
[Let arrows end on a package's tab](../principles/names.md#let-arrows-end-on-a-packages-tab).

An end with an arrowhead, a triangle or a circle-plus gets a stem that reaches the decoration's length
plus 8px from the box, on the grid (`Terminal::reaching`), and every later step keeps it. See
[Leave room for the head](../principles/lines.md#leave-room-for-the-head).

Component interface connectors (a wired dependency, or ball and socket between ports) keep their glyph
`CLEARANCE` (16px) clear of both ends of the straight segment it sits on, and the glyph's run
(`glyph_run`) is that clearance twice plus the glyph: 93px for a wired dependency at full length (socket
8px, dashed arrow 48px, ball 5px), 45px for ball and socket. Each interface end at a box leaves on a
run of at least 16px. The diagram is routed once like that. Each glyph whose segment is shorter than its
run, and is the first or last segment of its line, marks that end for a long stem reaching the whole run.
A second route then gives every marked end its long stem and every interface end at a port the 16px run
as well; it is kept when it has no more failures, crossings, overlaps or bends than the first. Otherwise,
if any end was marked, a third route tries the long stems alone under the same test, and failing that
the first route stays. Diagrams with neither marked ends nor interface ports route once, and
alignment, columns and box growth always compare layouts by the first route alone; only the drawn route
tries for more room. Trying beats a fixed rule because the router decides which of two lines in a
channel passes the other by its costs and order, and in Order Platform an 8px longer port stem was enough
to make two lines cross. The drawing places the glyph within the clearance where the segment allows, and
on a segment still too short it shortens the dependency arrow (down to 24px) before the stubs. See
[Give an interface room](../principles/kinds.md#give-an-interface-room).

### Routing

A* runs over a sparse grid with costs for length, bends, crossings, shared lanes and travel along
container boundaries. Three rip-up rounds settle conflicts. In state diagrams, each composite state that
is not an end of a line gets a soft band 24px deep around it, which costs like a boundary to run along,
so long lines keep their room (see [Give long lines room](../principles/lines.md#give-long-lines-room)).
Component diagrams give the same band to every container a line does not end on, and to a container it
reaches at a port.
A choice or junction sends the branch whose target lies straight ahead out of its forward side (see
[Flow passes straight through a decision](../principles/lines.md#flow-passes-straight-through-a-decision)). See
[Draw lines orthogonally](../principles/lines.md#draw-lines-orthogonally) and
[Avoid crossings](../principles/lines.md#avoid-crossings).

### Nudging

Each segment then moves along its own axis to where its line overlaps, crosses, crowds and bends
least, in that order, and never through a box, a reserved label or a container boundary. Two lines
tangled in one channel move together. Jogs fold away where a straighter path is clear, and an end
may slide along its side, between its neighbours, to straighten its line. Finally, the middle runs
of Z-shaped lines that share a channel spread evenly across it, and a lone one sits in the middle
of its gap. Last, where two lines that share an end part a few pixels apart, the line that runs on
turns where the other turns. If lines still lie on top of each other after [L over Z](#l-over-z), which
may free the room they were fighting over, they get one more nudge. See [Never lay lines on top of each other](../principles/lines.md#never-lay-lines-on-top-of-each-other),
[Space parallel lines evenly](../principles/lines.md#space-parallel-lines-evenly) and
[One junction per merge](../principles/lines.md#one-junction-per-merge).

### L over Z

After nudging, a line shaped like a Z, whose two end runs are parallel, becomes an L when one of its
ends may move to the side of its box that faces the line: the first run continues past the box's
corner and turns into that side. The same step looks at the last three runs at each end of
every longer line (`tail_shapes`): when the first of them is parallel to the last, so the line comes down
beside the box (a Z at the end) or past it (a U), and the run across starts beyond the box's side, the
end moves to that side and the line loses one or two bends. A line never doubles back into a box where
the side it arrives from is clear (see [Never double back into a box](../principles/lines.md#never-double-back-into-a-box)),
and a line that leaves a shared side and at once turns past the corner leaves by the side it turns toward
(see [Use a free side before you share one](../principles/lines.md#use-a-free-side-before-you-share-one)).
Each line takes up to four such steps. Moving the start of a longer line is also refused when the line
would cross the boundary of a container on another side than before (a line that left its container
through the side facing its target keeps that side). Adapters say which ends may move by giving a terminal its box
(`router::Body`), with the sides it may take and, for a state with internal behaviours, the band under
its top that its left and right sides keep free of ends (`Body::head`, down to 16px below the compartment
line). Component ends may move on every leaf component, onto sides that hold no stub or port, on any
line but a note link, interface lines included. The L is kept only when it meets no obstacle or strip more than the Z did, its new end
keeps `SLIDE_GAP` from other ends on that side and 16px from the corners, it gains no overlap,
crossing or close run, and no two lines come to leave one point together. An end that lands alone on a
side must land at its middle. Among the Ls that pass, the best standing wins, then the end nearest the
middle of its side. Component and package diagrams (`Problem::centred`) also give every end its box: an
end left alone on the side the L left returns to its middle, and an L that would leave one off the
middle is dropped. There, an end alone on its side does not move when the far end shares its side.
Because it is part of `router::route`, box growth and the label hybrid see the L. See
[Prefer an L to a Z when the L is clear](../principles/lines.md#prefer-an-l-to-a-z-when-the-l-is-clear).

### Labels after routing

State diagrams (`diagrams/state/hybrid.rs`): after box growth, the diagram is routed with every label
reserved. Each labelled line that still bends is then freed: routed without its label's waypoint, with
its label no longer an obstacle. Each freed label is placed beside its new line: five positions along
every segment, on both sides, ink 8px off the line, on a segment that reaches 8px past the label. A
place must touch no line, arrowhead, shape or label (4px air, 8px from other labels, 6px from outlines
and region dividers), stay inside the diagram, sit within 12px of its own line, keep the rest of its own
line as far, and keep every other line 12px further. The best place prefers the flowing side (right of
a vertical run when the flow is down, above a horizontal run when it is right), the middle of long runs,
and room from other lines. A label with no place, or whose line gains no straightness or gains a
crossing, is pinned back to its spot and the rest are routed again, up to four rounds. The result
replaces the reserved drawing only when it has fewer bends and no more crossings, overlaps, merged
ends, failed routes or label flaws (`metrics::read_labels`). Each round costs one route.

### Labels

Component and package labels, and state labels with no reserved room, take the position with the fewest
collisions. See [Labels](../principles/labels.md).

## Layout engines

The steps from compound layout to placement form a layout engine, which component, package and state
diagrams can swap. Everything after placement, from alignment to labels, and the drawing itself, is the
same for every engine, so two engines can be compared on one file and in the quality measures.

- **What an engine is.** An entry in `ENGINES` in `src/diagrams/engine.rs`: a name, one line on what it
  does, and a `core::layered::Strategy`, which picks the method of each layered stage that has a choice.
  Today only ranking has one: `Ranking::LongestPath` (`rank::assign_ranks`) or
  `Ranking::NetworkSimplex` (`simplex::network_simplex`). `core` knows no engine names; the names and
  their mapping to strategies live in `diagrams`.
- **The engines.** `longest-path`, the default: longest-path ranks, median ordering and placement,
  columns and notes last, as described above. `network-simplex`: ranks that make the total line length
  as short as possible, then the same steps. Try it when a box hangs far above the boxes it points to.
- **Network simplex ranking.** Gansner, Koutsofios, North and Vo (1993), section 2: minimise the sum of
  weight times length over the edges, each at least one rank long. Both rankings work on the same pairs
  (`rank::oriented`): same-rank groups are one node, a hint pair weighs 0 (it orders but does not pull), a
  connector's weight counts (4 for state transitions from an initial state or to or from a bar), and a
  pair that would close a cycle is turned round in the same order, so both engines agree on which edges
  point back. Longest path is the feasible start; a tight spanning tree, cut values and leave/enter pivots
  follow until no cut value is negative, at most `64 + 16 * pairs` pivots. Each connected part is solved
  alone and starts at rank 0. Where longest path is already optimal its ranks are kept, so the engines
  differ only where network simplex shortens lines. The paper's balancing step is left out (it broke
  parallel columns in the one diagram it changed). On the golden files and the book's diagrams longest
  path is already optimal, so both engines draw them the same.
- **Selection.** The parser stores the engine of a `layout <name>` statement in the diagram
  (`Diagram::engine`, the default when there is none). `drumlin::render_with` and `check_with` take
  `Options { layout }`, which the CLI fills from `--layout`; each kind's `drawing` replaces the diagram's
  engine with a chosen one, so the option wins. Each kind's layout adapter builds its `Graph` with
  `strategy: d.engine.strategy`. No global state carries the choice. Sequence and activity diagrams
  reject both. The mdBook preprocessor needs nothing more: a code block picks an engine with the
  statement.
- **Adding an engine.** Write the stage's method in `core::layered` (a ranking function beside
  `assign_ranks` in `rank.rs`, returning ranks from 0 with each same-rank group sharing one), add a
  variant to that stage's enum (`Ranking`) and its arm in `rank::ranks`, then add one `Engine` entry to
  `ENGINES`. A stage that gains its first choice (coordinate placement, for Brandes-Köpf or simplex
  coordinates) gets a field in `Strategy` and a `match` where `arrange` calls it. An engine that
  replaces the whole layered layout would make `Engine::strategy` an enum with a variant for it, matched
  in each kind's layout adapter (`layouts` for component and package, `settle_regions` for state), and return the
  same `layered::Layout`, so the steps after placement stay unchanged. The default engine must keep the
  golden files byte-identical.
- **Comparing engines.** `DRUMLIN_ENGINES=all cargo test --release --lib compare_engines -- --nocapture`
  (or a comma-separated list of names) prints every component, package and state table below once per
  engine, then each total side by side with its budget. The budget tests always measure the golden files
  as written, so the default engine's budgets are enforced unchanged. Set `DRUMLIN_ENGINE=<name>` to run
  `explain_ends`, `explain_labels`, `explain_flow`, `explain_size` or `dump_lines` with another engine, and use
  `drumlin render --layout <name>` to look at a drawing.

## Measuring quality

The quality test in `src/diagrams/quality.rs` measures every golden file and holds each total to a
budget: a change that raises a total fails the test, and a change that improves one lowers the budget.
Run `cargo test --lib quality -- --nocapture` to print the tables and the current totals.

For component, package and state diagrams it counts crossings, overlapping runs, bends and merged ends
(two connectors leaving one point together). It also measures how big every drawing is (see the size
measures below).

The same test reads every connector label in those files (`core::metrics::read_labels`): labels within
2px of a line, outline or arrowhead, labels on shapes or other labels, labels with another line at least
as near as their own (ambiguous), labels more than 12px from their own line, and the summed distance
from each label to its own line. Each kind builds this as a `Scene` with owners in
`src/diagrams/<kind>/scene.rs`, from the parts `src/diagrams/connectors.rs` assembles. Interface names on assembly connectors read with their ball and count
only as text to keep clear of. Set `DRUMLIN_LABELS=state/<name>` and run the `explain_labels` test to
list the flaws in one golden file.

Sequence and activity diagrams have no router, so the same test measures their whole drawing instead.
Each kind describes its output as a `Scene` (`src/diagrams/<kind>/scene.rs`): connectors, opaque shapes,
shapes drawn under them, strokes text must avoid, free text, contained text and gaps meant to be equal.
`core::metrics::assess` then counts, per kind, only what layout decides:

| Count | Sequence | Activity |
|---|---|---|
| Crossings and overlapping runs | between messages | between flows and note links |
| Bends | not counted: a self message always has two | every flow |
| Lines through shapes | messages through heads, notes, refs, tabs | flows through nodes and notes |
| Shape overlaps | tabs or notes over execution bars, shapes on shapes, shapes across frame edges | nodes and notes on each other or across partition dividers (fork bars may span) |
| Labels on lines | message labels and guards within 2px of a message, lifeline, frame edge, arrowhead or dot | guards within 2px of a flow, link, divider or arrowhead |
| Labels on shapes and labels | against heads, bars, notes, refs, tabs | against nodes, notes, lane heads |
| Text overflow | names in heads, note and ref text, operators in tabs | names in nodes, note text, lane names |
| Uneven gaps | rows of consecutive one-line messages with nothing between | straight drops from node to node, except into a merge or bar |

Messages crossing lifelines and bars are the notation, so they are not counted. Diamonds and circles
count as the largest square inside them, because flows end on their outline. A gap is uneven when it
differs from the most common gap by more than half a grid step. Set `DRUMLIN_SCENE=activity/<name>` and
run the `explain_scene` test to list what was counted in one golden file.

The same test counts, per kind, the end measures of `core::ends` on a `Figure` that the same
`scene.rs` builds: ends alone on their side but more than
4px from its middle (ports and stubs count as ends), pairs of lines that share an end and part less than
24px apart, boxes with two or more children in their container that are not centred over them, and
heads or start decorations on a run shorter than their length plus 8px. Decisions, and boxes in a column with one of their children, are not counted as uncentred. Set
`DRUMLIN_ENDS=component/<name>` and run the `explain_ends` test to list them in one golden file.

The flow measures of `core::flow` read the same figure: boxes whose flow continues into chain children
(a link from a box with one forward child, or to a box with one forward parent, or from a decision) with
none of them within 4px of its centre line, and the summed offsets; pairs of a line and a shape it runs
beside, inside or outside, for more than 8px, closer than 16px (24px for containers); and decisions where
no line leaves opposite a line that arrives. Note distortion draws each golden file again without its
notes and counts boxes that moved by more than 4px against the median move, and bends the notes added.
Activity diagrams get the decision and note counts too. Set `DRUMLIN_FLOW=state/<name>` and run the
`explain_flow` test to list the flaws in one golden file.

The size measures (`core::metrics::size`) cover all five kinds. Störrle (MODELS 2014, ESEM 2016)
reports that a diagram's size hurts readers more than its layout flaws, and every count above can stay
the same while a drawing grows. Per golden file, and totalled per kind:

- **Line length:** the summed length in pixels of every line the other measures read (for component,
  package and state every routed connector, note links and self-transitions included; for sequence the
  messages; for activity the flows and note links), each segment at its straight length, rounded once
  per drawing.
- **Area:** the width times the height of the SVG document, as its `width` and `height` attributes give
  it, margins and title included: the space the drawing takes on a page.
- **Too wide:** the drawings wider than 732px, the book's 750px content column less the figure's padding
  and border, which the book scales down. The widest drawing of each kind is printed too.

Each total has a budget. The test prints only the drawings wider than the column; set
`DRUMLIN_SIZE=state` to list every state drawing's line length, frame, area and aspect, or
`DRUMLIN_SIZE=state/<name>` to list the length of each of its lines as well, and run the
`explain_size` test. On the golden files both engines give equal totals; on a source where network
simplex moves a box, the line length falls (`network_simplex_draws_shorter_lines_where_the_engines_differ`).

## Style rules

These are stated with their reasons in [Shapes and spacing](../principles/shapes.md).

- Every box and line coordinate sits on an 8px grid.
- Lines are orthogonal. Their bends are sharp in every style.
- One ink colour and a white fill. In `pens`, three pen weights by the role of each mark; in `plain`,
  one line weight.
- Text widths come from embedded Helvetica metrics, so layout needs no font
  files and is identical on every platform. The `lettering` style measures with the advance widths of
  its own glyphs instead.

`pens` and `plain` never move a mark. Layout runs before they are consulted; rendering receives the
diagram's one `core::style::Style` (from a `style <name>` statement, or `Options { style }`, which the CLI
fills from `--style` and which wins) and asks it for three things: the class of each mark from its
`Role` (container, box, line, dashed line, head, note, rule, frame and so on), the path data of a
connector, and the stylesheet. `drumlin render --style plain` reproduces drumlin's output from before
styles existed byte for byte; the golden test keeps a few such files under `tests/style/plain`. The
`pens` stylesheet is scoped under `.drumlin.pens`, more specific than `plain`'s `.drumlin`, and sets every
property `plain` sets, so drawings in both styles can share one page.

`lettering` is the one style that reaches layout. Each kind's `render` wraps layout and rendering in
`core::text::with_face`, which sets the face of the drawing for the thread; every text measure
(`text::width`, `small_width`, `name_width`, `wrap`) asks the face, so box sizes, label boxes, note
wrapping and the title width come from the lettering's advance widths without passing the style to each
measure. Outside that scope, and in `pens` and `plain`, the face is Helvetica and the measures are the
ones drumlin always had. `core::svg::text` then draws a run as `core::lettering::run`: a group with
`role="img"` and the text as `aria-label`, holding one `<use>` per letter of a glyph defined once per
size in the drawing's `<defs>` (ids `dl-` plus the hand and the code point, so drawings on one page that
share a glyph share an identical definition). Names and the title are in spaced capitals, keywords and
small text in mixed case, inclined lettering is a 15 degree shear, and a halo is a white band under the
letters. The glyph data, `core/lettering/glyphs.rs`, is derived from Routed Gothic by Darren Embry and is
under the SIL Open Font License 1.1; the expected drawings are under `tests/style/lettering`, and
`lettering_text_fits_its_boxes` checks every golden source drawn in lettering for text that sticks out of
its box or note.

The study that chose the first of these steps to build is summarised in
[Background: the levers study](../principles/levers.md).
