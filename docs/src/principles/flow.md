# Flow and arrangement

Rules for the arrangement as a whole: which way the diagram flows, how boxes are ranked, ordered and
lined up, and where notes go.

## Flow one way

**Rule.** A diagram has one main direction, down or right, and most connectors point along it. Items
sit in ranks across that direction, and a connector joins items in later ranks wherever possible.

**Why.** A single direction lets the reader follow cause to effect without turning the page. Purchase
et al. ranked information flow third among UML layout preferences.

```drumlin
diagram component "One direction"
direction right

component Parser
component Checker
component Emitter
Parser --> Checker : AST
Checker --> Emitter : typed AST
```

**Source.** Purchase et al. 2002; Sugiyama et al. 1981 and Gansner et al. 1993 (layered drawing).

**Status:** enforced. [Ranking](../internals/layout.md#ranking) assigns ranks. Component diagrams flow
right by default, and package and state diagrams flow down, because their long labels then sit beside
vertical lines. `direction` changes it.

## Keep connected items close

**Rule.** Connected items sit in neighbouring ranks where the graph allows. A source pulls next to the
first thing it feeds.

**Why.** Short edges need fewer dummies, fewer bends and less searching.

**Source.** Gansner et al. 1993 (minimising weighted edge length).

**Status:** partial. The default engine, `longest-path`, pulls sources next to their targets.
Network-simplex ranking, which minimises the weighted edge length, is a selectable engine
(`layout network-simplex`), not the default.

## Order ranks to avoid crossings

**Rule.** Within each rank, items are ordered so that edges between ranks cross as little as possible.
Hints and notes are part of the ordering, not applied afterwards.

**Why.** Most crossings are decided by the order, before any line is routed.

**Source.** Gansner et al. 1993 (weighted median, transpose).

**Status:** enforced within a container ([Crossing reduction](../internals/layout.md#crossing-reduction)).

## Order across containers too

**Rule.** The order of items inside a container should take the lines that leave the container into
account.

**Why.** A container laid out on its own can put a child on the wrong side of its outside partners. The
Order Platform drawing has its one crossing for this reason, and the Compiler pipeline has a jog between
its two containers.

**Source.** Sander 1996; Forster 2002; [Layout levers](levers.md#the-five-crossings).

**Status:** planned, not scheduled. It needs a global layering across the hierarchy, which is a rewrite
of `arrange`.

## Keep long edges straight

**Rule.** An edge that spans several ranks runs as one straight line, where both ends can take it. Where
only one end can, it bends once, near the other end.

**Why.** Long edges that wander become spaghetti (Gansner et al.), and continuity helps path tracing
(Ware et al.).

**Source.** Gansner et al. 1993; Ware, Purchase, Colpoys and McGill 2002.

**Status:** enforced, by the straightening step of [Placement](../internals/layout.md#placement).

## Align a pendant with what it hangs from

**Rule.** An item joined to the rest by a single chain, such as an initial state, a final state or a
note-like leaf, sits in line with the item it hangs from, so the chain is straight.

**Why.** A short spur that jogs looks careless, and it is the easiest jog to remove.

```drumlin
diagram state "Pendants in line"

[*] --> Ready
Ready --> Done : finish
Done --> [*]
```

**Source.** Drafting practice.

**Status:** enforced, by the pendant step of [Placement](../internals/layout.md#placement).

## Keep a column

**Rule.** Items along the main flow sit centred over each other in one column, so the lines between them
run straight down (or across, when the flow runs right). Side branches move out of the column; the main
chain does not. The main chain is the longest path through the graph. Where a box has several onward
branches of equal length, it stays centred over them instead, unless it is a decision, which keeps its
first branch in its column (see [Flow passes straight through a decision](lines.md#flow-passes-straight-through-a-decision)).

**Why.** A straight spine is read at a glance, and every jog in it reads as a turn in the story: in
Process lifecycle, `New`, `Ready`, `Running` and `Zombie` would each jog a little left of the one before.

```drumlin
diagram state "A column"

[*] --> Idle
Idle --> Busy : start
Busy --> Done : finish
Busy --> Failed : error
Failed --> Idle : retry
Done --> [*]
```

**Source.** Brandes and Köpf 2001 (vertical alignment of blocks); Ware et al. 2002 (continuity).

**Status:** enforced in state diagrams, partial elsewhere. [Placement](../internals/layout.md#placement)
picks one column partner above and below each item and pulls partners onto one line ahead of anything
else.

- State diagrams keep their columns unless they route with more crossings, overlaps or failed lines
  than the layout without them (in Door, the line back from `Closing` to `Closed` would cross `lock`). A
  join bar, which has one line onward, passes the flow on into a column (Choice, fork, join: `Done` and the
  final state sit under the join); a fork bar starts none, so its branches stay symmetric.
- Component and package diagrams take the columns only when they route with fewer bends and nothing
  worse, so their own [alignment](../internals/layout.md#alignment) is kept elsewhere.
  [Line up the main branch](#line-up-the-main-branch) carries the column across containers.
- The quality test counts boxes whose flow continues into a chain child with none in line.

## Notes come last

**Rule.** Lay out the main elements and route their connectors first, as if there were no notes. Then put
each note beside its element, in the element's rank, on whichever side is free. A note never bends a main
connector and never moves a main element out of its column. Only when neither side has room does it push
the items beyond it in that rank outward.

Among the free places beside its element, a note takes the one in empty space: where it makes the
drawing no wider or taller, and where no line has to pass it. A place before the element, against the
flow, counts as well as one beside it.

**Why.** A note is a comment on the drawing, not part of it. A note that takes part in the layout can
hold boxes apart and push a whole chain sideways.

**Source.** A design decision of this project.

**Status:** enforced in state and package diagrams (`Constraint::Note` in the layered layout, placed by
`place_notes`), and in activity diagrams (see [Notes per diagram kind](kinds.md#keep-branches-close-notes-go-outside)).
The quality test counts note distortion: boxes a note moves against the rest, and bends it adds,
compared with the same source drawn without notes. Not done:

- Component notes still take part in ordering (`Adjacent`). Two component layouts that read well, Motor
  controller firmware and Storage engine, are as they are only because their notes push boxes apart, and
  drawing them without that push loses a straight line and a fan's symmetry.
- The choice of empty space: a note takes the first free side in its element's rank. In LSM storage
  engine the note belongs on top of `Write-Ahead Log`, where the drawing has room.

## Centre and balance

**Rule.** An item with several neighbours in the next rank sits centred over them. A rank is balanced
around the items it connects to, and the layout is symmetric where the graph is.

**Why.** Symmetry has a measurable but smaller effect than crossings and bends (Purchase 1997), and
it is what makes a tree look like a tree.

**Source.** Purchase 1997; Gansner et al. 1993; Brandes and Köpf 2001.

**Status:** partial. An order-preserving median solve aligns connected items. It is a heuristic: an
item can sit off the line of what it connects to. [Centre the parent](#centre-the-parent) moves parents
over their children afterwards. Not done: network-simplex or Brandes-Köpf coordinate assignment.

## Centre the parent

**Rule.** A box linked to several children further along the flow sits centred over them, at their
median or mean across the flow, so its lines fan out evenly. Likewise a box fed by several parents
sits centred under them. Moving a parent gives way to an alignment that matters as much: a lone
connector already running straight between two middles. Where one of the children continues the main
flow, the parent lines up with that child instead ([Line up the main branch](#line-up-the-main-branch)).

**Why.** A parent over one child alone makes the other branches run a long way round, and their lines
crowd the labels beside them.

**Source.** Gansner et al. 1993; Brandes and Köpf 2001.

**Status:** partial. [Alignment](../internals/layout.md#alignment) moves parents, with everything they
hold, in component and package diagrams, counting families across containers. A centred family and a
lined-up connector weigh the same, so neither is traded for the other. In state diagrams, the median
placement centres a parent over equal branches, and [Keep a column](#keep-a-column) ranks higher: a
parent in line with the child that continues its chain, and a decision, are not moved over the rest.
The quality test counts boxes with two or more children in their own container, further along the
flow, more than 4px from both the median and the mean (decisions and boxes in a column with one child
excepted).

## Line up the main branch

**Rule.** When the flow forks or merges at a box, the branch that carries the main flow runs straight
and the others sit beside it; only a fan with no main branch is centred, symmetric. The main branch is,
in order: the one branch on the longest path onward (or back, for a merge); else the one branch joined by
a lone connector, whose ends can meet on one line; else, when a chain runs through the box (one box on
its other side, and the box holds no others), the middle of the branches tied on the longest path, the
first of two in order across the flow. Straightness means the connection points line up, not the box
centres: a port that follows a line inside its component counts where that line runs. A box that moves
to line up takes along the boxes it is already lined up with on its far side, so a straight chain stays
straight while it moves.

**Why.** Without this rule the main flow is often not on one straight line: a box centred between its
neighbours bends both of its lines, and the chain reads as unbalanced.

```drumlin
diagram component "One straight, one beside"
direction right

component Loader
component Stage
component Verifier { provides IVerify }
component Boot { requires IVerify }
component Image
Loader --> Stage
Stage --> Boot
Boot --> Image
```

**Source.** Brandes and Köpf 2001 (blocks aligned along their longest path); Ware et al. 2002
(continuity).

**Status:** enforced in component and package diagrams ([Alignment](../internals/layout.md#alignment)).
The layout with leaders lined up is kept only when it routes with no more failures, crossings and bends
than the layout before alignment, and no more bends than the balanced fans of
[Centre the parent](#centre-the-parent); otherwise the balanced fans are kept (Battery sensor node,
Container runtime, Hexagonal service, Vehicle gateway). A container never leads by a chain through its
middle. State diagrams keep columns in the layered placement instead ([Keep a column](#keep-a-column)).

## Keep the spine on one line

**Rule.** The spine of a drawing is its longest chain of solid lines along the flow. The spine lies on
one straight line from end to end, also where it enters and leaves a container: the port it enters by,
the boxes inside, and the port it leaves by share that line. Boxes off the spine sit beside it. When
two chains are equally long, the one that the source lists first is the spine. A line that leaves a spine
box to the side gives the spine its facing side
([Use a free side before you share one](lines.md#use-a-free-side-before-you-share-one)).

**Why.** The spine is what the reader follows first, and every jog in it reads as a change of subject.
In Order Platform, port `api`, `API Handler`, `Outbox` and port `events` belong on one line, with
`Pricing Engine` above or below. In state diagrams the same holds: in Choice, fork, join the initial
state, `Ready`, the choice, `Done` and the final state belong on one vertical line.

**Source.** Brandes and Köpf 2001; Ware et al. 2002.

**Status:** partial. Each of the following is kept only when it routes with no more failures, crossings
and bends.

- Component diagrams: alignment sees a chain through a port as one chain. A line to a port that
  delegates counts as a link to the box the port leads to, inside or outside (`bridged_links`). After the
  boxes line up, a second pass lines up the line that leaves a spine port with a shared side beyond it.
  In Order Platform, `Web Gateway`, port `api`, `API Handler`, `Outbox`, port `events` and `Event Bus`
  share one line, and `Pricing Engine` sits above. Two equal lines into a port still make a balanced fan.
- State diagrams: a state moves under the line into or out of an entry or exit point, with the states
  lined up with it, and a fork whose branches all meet one join passes the column through, so the state
  after the join lines up with the state before the fork.
- Not done: when the spine and a branch inside the container are equally long, the branch inside wins
  (Replicated state machine: `peers` leaves `Consensus Module` at the bottom). A line that must turn back
  against the flow to stay on the spine (`Partitioned Log` to port `source` in Stream processor) is not
  part of it.

## Keep a side branch close

**Rule.** A box beside the main branch sits at the smallest gap that its line, its label and its
neighbours allow: one rank gap across the flow, not more. A branch is never pushed out to make a
drawing symmetric that has no symmetric fan.

**Why.** A side branch far from the box it leaves makes a long line and an empty band, and the drawing
leans. In Secure boot chain, `Kernel Image` sits further down than its line needs.

**Source.** Purchase 1997 (edge length); compactness in
[When rules conflict](introduction.md#when-rules-conflict).

**Status:** planned.

## Group what belongs together

**Rule.** Containers hold their contents. Notes sit beside what they annotate. Unconnected siblings fold
into a compact block instead of one long row. Orthogonal regions tile across the flow.

**Why.** Proximity groups things before any line is read.

**Source.** Gestalt proximity; UML 2.5.1, 14.2.4 (regions).

**Status:** enforced: [Compound layout](../internals/layout.md#compound-layout), the `Note` constraint
for notes (component notes: `Adjacent`), `wrap_constraints` for package siblings, and region tiling in
the state layout.

## Honour hints over heuristics

**Rule.** A layout hint (`left-of`, `above`, `align` and the others) always wins over what the heuristics would choose.

**Why.** The author knows what the diagram is for. A hint is the one way to say so without coordinates.

**Source.** A design decision of this project; [Layout hints](../reference/hints.md).

**Status:** enforced. Constraints apply inside every ordering sweep and win over edges.

## Draw the same input the same way

**Rule.** The same source always gives the same drawing, byte for byte, on every machine.

**Why.** A diagram in version control should change only when its source does, and readers should not
have to find their way around a drawing again after an unrelated edit.

**Source.** A design decision of this project; the D2 TALA author's caution about seed-dependent
layouts.

**Status:** enforced. There is no randomness, no HashMap order in the output, and text is measured with
embedded metrics. The golden tests check it.
