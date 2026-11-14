# Notes per diagram kind

The general rules hold everywhere. This page lists where a diagram kind adds rules or bends them.
Component, package and state diagrams share the layered layout and router. Sequence and activity
diagrams have layouts of their own, built so that most line rules hold by construction.

## Component diagrams

### Flow right, and leave room for the wiring

**Rule.** Component diagrams flow right by default. A rank gap grows to fit a connector's label, and
an assembly or wired connector's ball, socket and arrow.

**Why.** Interfaces and their names sit along horizontal runs, where they have room.

**Source.** A design decision of this project.

**Status:** enforced (`edge_spec` in the component layout).

### Keep a delegation straight, and line the outer box up with the port

**Rule.** A port with connectors inside its component lines up with them, so the delegation is
straight. A port without them sits at the middle of its side. A box outside then moves across the flow
to meet the port's line where it can; otherwise the connector outside jogs once.

**Why.** Inner gaps are narrow, so a jog there crowds the contents. Outside there is room, and a box
outside is usually free to move.

**Source.** Drafting practice; a design decision of this project.

**Status:** enforced, by [Alignment](../internals/layout.md#alignment): `Client` lines up with port `kv`
in Storage engine, and `Telematics Unit` with `uplink` in Vehicle gateway.

### Draw ball and socket clear of names

**Rule.** The ball, the socket and the interface name sit on a straight run of the connector, clear of
other labels and names. A name must be clear of every ball and socket, not only of its own, and must sit
nearer to its own symbol than to any other.

**Why.** The glyph is notation. A name over it hides what it says.

**Source.** UML 2.5.1, 11.2.4 and 10.4.4.

**Status:** partial (`draw_assembly` and `draw_wired` choose a free straight run). Not done: an interface
name can still sit over another symbol's ball, so it is hard to tell which name belongs to which symbol:
in the [unconnected interfaces](../elements/component/interface.md#unconnected-interfaces) example,
`IEncode` lies on the `IDecode` ball.

### Line up the interface symbols of one fan

**Rule.** When several interface connectors meet one component on one side, their balls and sockets sit
on one line across the flow, at the same distance from that component.

**Why.** Symbols at different distances read as different kinds of connection. In the Several consumers
example on the [Interface](../elements/component/interface.md#several-consumers) page the lines are good,
but the sockets and balls could all sit on one vertical line.

**Source.** Drafting practice.

**Status:** planned. Each connector places its symbol on its own longest free straight run.

### Give an interface room

**Rule.** A socket, a ball or a ball and socket on a connector keeps at least 16px of plain line on each
side, to the component (or port) it hangs off and to the bend where its straight run ends, so it reads
as a symbol of its own with a visible stub each way. An interface connector also leaves a box or a port
on a straight run of at least 16px before it turns.

**Why.** Glued to the box, the socket looks like part of the component's outline. Nudging centres the
middle run of a bent line in its gap, which without this rule can leave the glyph almost touching the
box.

**Source.** Drafting practice.

**Status:** enforced where the router can give the room. Where a glyph's segment is too short, its end
gets a stem as long as the glyph and both stubs, and the line is routed again. Exception: where that
route would cross or bend more, the shorter route stays, and the dependency arrow shortens before the
stubs do.

## Package diagrams

### Draw containment as one tree

**Rule.** When every member sits in the next rank, an owner's members hang from one trunk with the
circle-plus on the owner. Otherwise each branch is routed from that shared anchor.

**Why.** UML draws membership as branching lines from one circle-plus (12.2.4).

**Source.** UML 2.5.1, 12.2.4, p. 248.

**Status:** enforced.

### Fold unrelated siblings into a block

**Rule.** Four or more siblings with no relations or hints between them fold into a roughly square block
of rows.

**Why.** One long row of unrelated packages wastes width and suggests an order that is not there.

**Source.** A design decision of this project.

**Status:** enforced (`wrap_constraints`).

## State diagrams

### Reserve label room and keep labels on the flowing side

See [Reserve room for labels](labels.md#reserve-room-for-labels) and
[Put labels on a consistent side](labels.md#put-labels-on-a-consistent-side). State diagrams are where
labels matter most (Castelló et al.), and where they cost the most bends:
[Never let a label force a bend](labels.md#never-let-a-label-force-a-bend).

### Draw self-transitions as compact loops away from the flow

**Rule.** A self-transition is a small loop on the side of the state away from the flow, with its label
beside it. A hidden box keeps the state centred in its rank.

A loop also gives way to a line. When a transition could leave the state straight by the side the loop
is on, the loop moves to another free side, and the transition takes the side. Two loops on one state
take two different free sides, and a side with empty space beyond it comes first.

**Why.** A loop on the flowing side would collide with the transitions in and out. A loop that does not
give way forces a line round it: in Process lifecycle the `syscall` loop belongs on the left of
`Running`, so that `wait(event)` runs straight out to the right; in Circuit breaker the loop belongs on
the left of `Closed`, so that `probe succeeded` can enter on the right.

**Source.** Drafting practice; a design decision of this project.

**Status:** partial. The loop on the side away from the flow is enforced. Planned: giving way to a line,
and spreading two loops over two sides (in Transition labels, one of `tick` and `drift` could sit on top
of `Armed`).

### Meet small symbols on their sides

**Rule.** A line into a final state, choice, junction or terminate symbol from a source that cannot line
up with it meets the symbol's facing side, as an L. Lines out of a choice fan from different corners.

**Why.** Lines meeting at one point of a small symbol merge into one ambiguous junction.

**Source.** UML 2.5.1, Figure 14.25; a design decision of this project.

**Status:** enforced.

### Tile regions across the flow

**Rule.** Orthogonal regions sit side by side across the flow, with equal extent along it, separated by
dashed lines.

**Why.** This matches Figures 14.9 and 14.10, and lets each region keep the diagram's direction.

**Source.** UML 2.5.1, 14.2.4.3 and Figures 14.9 and 14.10.

**Status:** enforced.

## Sequence diagrams

Sequence diagrams use a timeline layout: lifelines are columns, events are rows on the 8px grid, and
there is no router.

### Never cross a lifeline with text

**Rule.** A message label stays within one lifeline gap, the gap nearest the sender that fits it, and
the columns widen until it fits.

**Why.** Text across a lifeline is cut by it, and a reader cannot tell which side it belongs to.

```drumlin
diagram sequence "Labels between lifelines"

actor User as u
participant API as a
participant DB as db
u -> a : place order
a -> db : insert(order)
db --> a : id
a --> u : confirmation
```

**Source.** Drafting practice.

**Status:** enforced. The quality test counts any label within 2px of a lifeline and allows none.

### Keep labels off arrowheads and dots

**Rule.** A message label keeps 4px from the arrowhead it points at. A found or lost message's stub is
long enough to hold its label between the dot and the arrowhead, clear of both, and a creation message's
gap counts the new head's half width.

**Why.** Text that touches the arrowhead reads as part of it, and the halo bites into the head.

**Source.** UML 2.5.1, 17.4.4.1 and Figure 17.11 (lost and found messages); drafting practice.

**Status:** enforced; the quality test counts labels within 2px of an arrowhead or dot.

### Keep frame tabs and notes off execution bars

**Rule.** A fragment's operator tab, or a note across lifelines, should not hide part of an execution bar.

**Why.** A bar cut by a tab or a note looks like an execution that stopped and started again.

**Source.** UML 2.5.1, 17.2.4 and 17.6.4.

**Status:** planned. Tabs sit at the frame's corner, 12px left of the first lifeline's bar, so a tab over
an active bar hides a stretch of it. The quality test counts these cases.

### Keep messages horizontal and time downward

**Rule.** Every message is a horizontal arrow, and later events are lower. Nested executions step right.

**Why.** That is the notation (17.4.4). Horizontal arrows keep the time order unambiguous.

**Source.** UML 2.5.1, clause 17.

**Status:** enforced. Consecutive one-line messages are 32px apart; the quality test counts any other gap
between them with nothing drawn in between.

## Activity diagrams

Activity diagrams use a structured-flow layout driven by the block tree, so lines are straight by
construction and nothing is routed by search.

### Keep the main flow on one axis

**Rule.** The flow that continues takes the straight axis. Side arms branch off and rejoin beside it, and
a bypass runs on the right.

**Why.** The reader follows the main path down a straight line, and the exceptions read as detours.
Below, the `[no]` arm stops, so `Ship` and `Invoice` stay on the axis.

```drumlin
diagram activity "Main flow straight"

start
action "Receive order"
if "in stock" {
  action "Ship"
} else "no" {
  action "Cancel"
  stop
}
action "Invoice"
stop
```

**Source.** Drafting practice; Ware et al. 2002 (continuity).

**Status:** partial. The axis stays straight. Not done: the statement after a branch whose siblings stop
waits below the tallest sibling, so the drop to `Invoice` is longer than the others. The quality test
counts such drops as uneven gaps.

### Space a straight flow evenly

**Rule.** Consecutive nodes in a straight flow are 32px apart. A guard beside a drop adds the room it needs.

**Why.** Even spacing makes a straight flow read as one sequence, and a longer gap then signals something.

**Source.** Drafting practice.

**Status:** partial. Exceptions: a while loop's merge and decision sit 24px apart and read as one test;
a note rising over a decision lengthens the drop into it; a node with a taller note is centred in the
note's height; and the continuation after a branch whose siblings stop (above).

### Keep branches close; notes go outside

**Rule.** Side-by-side branches sit as close to the main flow as their contents allow. A note on a node in
a branch goes on the outer side of that branch, so it never widens the space between branches. A note in
the branch on the main axis goes on the side away from the other branches when they all leave on one
side, and otherwise keeps the side its flow has.

**Why.** A note between two arms pushes them far apart around an empty middle. In Firmware boot, the note
on `Jump to application` therefore sits on its left, away from the `[else]` arm on its right.

**Source.** A design decision of this project: [Notes come last](flow.md#notes-come-last), applied to
the activity layout.

**Status:** enforced. The split sizes its arms once to plan them, then again with each side arm's notes on
its outer side.

### End note links on the shape

**Rule.** A note's dashed link ends on the outline of the node it annotates.

**Why.** A link that stops short in white space does not say which node it means.

**Source.** UML 2.5.1, 7.4.4 (a note is attached by a dashed line).

**Status:** enforced: a note above a decision reaches the diamond's outline, with one diagonal line from
the note's corner to the middle of the diamond's upper edge (see
[Note links may be diagonal](lines.md#note-links-may-be-diagonal)).

### Put guards by the branch they guard

**Rule.** A guard sits at the start of its flow, by the corner where the flow leaves the decision. Arms
that share one run keep their guard over their own drop.

**Why.** A guard read near the decision is matched to the right arm.

**Source.** UML 2.5.1, 15.2.4 (guards on outgoing flows).

**Status:** enforced. The quality test counts guards touching a flow, node, divider or other label, and
allows none.

### Draw a merge only where flows rejoin

**Rule.** A merge diamond or join bar is drawn only where at least two flows meet.

**Why.** A merge with one input is ink without meaning.

**Source.** UML 2.5.1, 15.3.3.

**Status:** enforced.
