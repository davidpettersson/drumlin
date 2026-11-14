# Lines and routing

Rules for the lines between boxes: their shape, where they leave and enter a box, and how they share
space. They apply to component, package and state diagrams, which share the layered layout and the
router. Sequence and activity diagrams have their own layouts; see [Notes per diagram kind](kinds.md).

## Draw lines orthogonally

**Rule.** Every connector is a chain of horizontal and vertical runs. In the default style, `pens`, the
bends are sharp mitred corners, as a pen drawn along a ruler leaves them; in `plain` they are small
rounded corners (6px radius, less on very short runs). There are no diagonals, except in a note's link,
which is an annotation (see [Note links may be diagonal](#note-links-may-be-diagonal)).

**Why.** Orthogonal lines match the grid and the boxes, cross at right angles, which readers barely
notice (Huang), and ranked second in UML diagram preference (Purchase et al. 2002).

**Source.** Purchase, Allder and Carrington 2002; Huang 2008; a design decision of this project. UML
gives segments no meaning ("The individual segments of the line itself have no semantic significance",
11.5.4, p. 201), so the style is free.

**Status:** enforced. [Routing](../internals/layout.md#routing) searches only orthogonal moves, and
`Style::line` draws the corners: straight joins for `pens` and `lettering`, `glyphs::rounded` for `plain`.

## Keep lines out of shapes and names

**Rule.** A line never passes through a box, a symbol, a name, a reserved label or an interface stub.
It may cross a container's border only to reach something inside it.

**Why.** A line through a shape reads as a connection to that shape. A line through text makes both
unreadable.

**Source.** Castelló, Mili and Tollis 2000 (their motivating figure shows edges through states as the
failure to avoid).

**Status:** enforced. Each connection carries its own obstacles in [Routing](../internals/layout.md#routing),
and [Nudging](../internals/layout.md#nudging) never moves a segment through one.

## Never lay lines on top of each other

**Rule.** Two different connectors never share a run. Lines may join only where the notation joins
them: the branches of a containment tree from one circle-plus, or several connectors meeting at one
port.

**Why.** Two lines on one lane cannot be told apart, and a shared start reads as one connector that
forks. That is worse than a crossing, which is still readable.

Bad: two connectors leave `A` from one point and share a run, so they read as one line that forks.

<figure class="drumlin-figure">
<svg xmlns="http://www.w3.org/2000/svg" class="drumlin pens" viewBox="0 0 420 170" width="420" height="170">
<style>
.drumlin.pens text { font-family: Helvetica, Arial, "Liberation Sans", sans-serif; fill: #22201c; }
.drumlin.pens .name { font-size: 13px; font-weight: 600; }
.drumlin.pens .small { font-size: 11px; fill: #22201c; }
.drumlin.pens .shape { fill: #fff; stroke: #22201c; stroke-width: 1.25; stroke-linejoin: miter; }
.drumlin.pens .edge { fill: none; stroke: #22201c; stroke-width: 1.25; stroke-linecap: butt; stroke-linejoin: miter; }
.drumlin.pens .dashed { stroke-dasharray: 7 3; }
</style>
<rect x="0" y="0" width="420" height="170" fill="#fff"/>
<rect x="16" y="56" width="112" height="56" class="shape"/>
<text x="72.0" y="88.5" class="name" text-anchor="middle">A</text>
<rect x="288" y="16" width="112" height="48" class="shape"/>
<text x="344.0" y="44.5" class="name" text-anchor="middle">B</text>
<rect x="288" y="104" width="112" height="48" class="shape"/>
<text x="344.0" y="132.5" class="name" text-anchor="middle">C</text>
<path d="M128 84 L200 84 L200 40 L288 40" class="edge"/>
<path d="M278 35.5 L288 40 L278 44.5" class="edge"/>
<path d="M128 84 L200 84 L200 128 L288 128" class="edge"/>
<path d="M278 123.5 L288 128 L278 132.5" class="edge"/>
</svg>
</figure>

**Source.** Wybrow, Marriott and Stuckey 2009, section 5.

**Status:** enforced. Nudging ranks overlaps above crossings, and the quality test allows no overlap.
Ends that slide may not come closer than 16px; an 8px gap lets two ends meet
([levers study](levers.md#box-size)). The quality test also counts merged ends, two connectors leaving
one point together, which the overlap count excuses; only the intended fan-ins to one port are allowed,
and box growth is never kept if it merges two connectors.

## Avoid crossings

**Rule.** Lines should not cross. When a crossing cannot be avoided, it is a clean right angle
away from any corner, label or arrowhead.

**Why.** Crossings have the largest effect of any aesthetic on reading graphs (Purchase 1997), and
93% of subjects preferred the UML diagram with fewer (Purchase et al. 2002).

Bad: the lines to `C` and `D` cross, although swapping the order of their ends would separate them.

<figure class="drumlin-figure">
<svg xmlns="http://www.w3.org/2000/svg" class="drumlin pens" viewBox="0 0 330 200" width="330" height="200">
<style>
.drumlin.pens text { font-family: Helvetica, Arial, "Liberation Sans", sans-serif; fill: #22201c; }
.drumlin.pens .name { font-size: 13px; font-weight: 600; }
.drumlin.pens .small { font-size: 11px; fill: #22201c; }
.drumlin.pens .shape { fill: #fff; stroke: #22201c; stroke-width: 1.25; stroke-linejoin: miter; }
.drumlin.pens .edge { fill: none; stroke: #22201c; stroke-width: 1.25; stroke-linecap: butt; stroke-linejoin: miter; }
.drumlin.pens .dashed { stroke-dasharray: 7 3; }
</style>
<rect x="0" y="0" width="330" height="200" fill="#fff"/>
<rect x="16" y="16" width="112" height="48" class="shape"/>
<text x="72.0" y="44.5" class="name" text-anchor="middle">A</text>
<rect x="200" y="16" width="112" height="48" class="shape"/>
<text x="256.0" y="44.5" class="name" text-anchor="middle">B</text>
<rect x="16" y="136" width="112" height="48" class="shape"/>
<text x="72.0" y="164.5" class="name" text-anchor="middle">C</text>
<rect x="200" y="136" width="112" height="48" class="shape"/>
<text x="256.0" y="164.5" class="name" text-anchor="middle">D</text>
<path d="M72 64 L72 100 L256 100 L256 136" class="edge"/>
<path d="M251.5 126 L256 136 L260.5 126" class="edge"/>
<path d="M256 64 L256 84 L240 84 L240 116 L72 116 L72 136" class="edge"/>
<path d="M67.5 126 L72 136 L76.5 126" class="edge"/>
</svg>
</figure>

**Source.** Purchase 1997; Purchase, Allder and Carrington 2002.

**Status:** partial. Ordering ([Crossing reduction](../internals/layout.md#crossing-reduction)), the
router's crossing cost, nudging and box growth remove most crossings. Ends on a shared side are spread
in the order of the lines they start, so lines leave a box without crossing. Not done: the Order
Platform drawing in the [introduction](../introduction.md) keeps one crossing, because removing it needs
ordering across containers, which the recursive layout cannot see
([Order across containers too](flow.md#order-across-containers-too),
[the five crossings](levers.md#the-five-crossings)).

## Bend only when you must

**Rule.** A line turns only when its ends are not aligned and cannot be. A Z between two ranks is
acceptable when the ends cannot share a coordinate; a jog that a slightly different layout would
remove is a defect.

**Why.** Bends are second only to crossings in the preference data (91%), and a continuous path is
traced faster (Ware et al. 2002).

**Source.** Purchase et al. 2002; Ware, Purchase, Colpoys and McGill 2002.

**Status:** partial. Most lines run straight through these steps: straight long edges
([Placement](../internals/layout.md#placement)), slots that slide toward their peers
([Slots](../internals/layout.md#slots)), jog folding ([Nudging](../internals/layout.md#nudging)),
[box growth](../internals/layout.md#box-growth), the 8px corner inset, ends on package tabs,
[L over Z](../internals/layout.md#l-over-z), state labels that follow their lines,
[columns](flow.md#keep-a-column) and [alignment](../internals/layout.md#alignment).
[Meet the middle](#meet-the-middle) ranks above this rule: an end alone on its side does not slide off
the middle to run straight. Where alignment cannot line the boxes up, the line jogs once. The quality
test fails when the total number of bends rises.

## Give long lines room

**Rule.** A connector running beside a shape keeps a clear gap of at least two grid steps (16px), and
three (24px) beside a composite state or a container.

**Why.** A line hugging a box reads as part of its outline: the eye follows the box's border instead of
the line. A long return line beside a large composite, such as `technician reset` from `Fault` back to
`Idle` in Elevator controller, shows it most.

**Source.** Drafting practice.

**Status:** partial. In state diagrams, [Routing](../internals/layout.md#routing) gives each composite
state that is not an end of the line a soft band 24px deep around it: a line may run there only at a
cost, and nudging does not move one into it. Simple states keep lines off by their 8px wall, and their
spacing holds most lines 16px away. Component diagrams give the same band to every container that is
not an end of the line, and to a container the line reaches at a port. The quality test counts lines
that run beside a shape for more than 8px, closer than 16px (24px for containers). Not done: package
diagrams have no bands; in Composite states, `resume` runs inside `Playing` near its border.

## Flow passes straight through a decision

**Rule.** At a choice, junction or decision diamond, one branch is the main one, and it leaves on the side
opposite the incoming line, so the flow runs straight through the symbol. The other branches leave from
the two sides across the flow. The main branch is the one whose target continues the column; failing
that, the first declared. The same holds for merges, whose incoming lines meet on the incoming side.

**Why.** A decision that turns the flow hides which way is the normal case. Below, `[ok]` leaves the
choice straight down to `Installing`, opposite the line from `Verifying`, and `[corrupt]` leaves from the
side to return.

```drumlin
diagram state "Straight through"

[*] --> Downloading
Downloading --> Verifying
Verifying --> c
choice c
c --> Installing : [ok]
c --> Downloading : [corrupt]
Installing --> [*]
```

In an activity diagram the same holds for `if` and `switch`. Below, both arms of `paid` have a body and
both continue, so the first declared takes the axis and `[else]` leaves from the side; it comes back into
the merge from the side, and `[paid]` enters the merge from above. The `unauthorised` arm stops, so the arm
that continues takes the axis although it is declared second.

```drumlin
diagram activity "Straight through"

start
if "unauthorised" {
  action "Send 401"
  stop
} else {
  action "Load account"
  action "Check limits"
}
if "paid" {
  action "Ship"
} else {
  action "Send reminder"
}
stop
```

**Source.** Drafting practice for flowcharts.

**Status:** enforced in state and activity diagrams.

- State diagrams: a choice or junction is a decision (`Constraint::Pivot`) to the layered layout, so its
  first branch keeps its column even against an equal branch, and a bar below it may end that column. In
  routing, the branch whose target lies straight ahead leaves from the forward side (`spoke_sides`), and
  the others from the corners on their sides.
- Activity diagrams: a decision puts one arm on its axis (`arm_plan`): the one arm that continues while
  the others end; else the body beside an empty arm; else the one arm in the decision's own lane; else the
  middle arm, the first declared of two and the left of the middle two of four. The other arms keep their
  declared order left to right about it (the side arm of a two-armed `if` leaves on the right), or leave
  towards their own lane, and rejoin the merge from the side.
- A fork is not a decision and keeps an even count of branches symmetric.
- Not done: turning a whole decision so the flow enters from the side; no drawing needs it yet.

## Note links may be diagonal

**Rule.** A note's dashed link is one short straight line from the nearest point of the note to the
element, diagonal when they share no span. It never jogs and is never a stub. Where the note and its
element share a span, the link leaves the middle of the note's facing side when that middle lies in
the shared span.

**Why.** A note link is an annotation, not a flow. UML 2.5.1 puts no direction on it (7.4.4), and an
orthogonal stub of a few pixels reads as a mistake. A link that jogs around the note reads worse than a
diagonal link.

**Source.** UML 2.5.1, 7.4.4; drafting practice.

**Status:** enforced. Component, package and state note links are routed, so other lines keep clear of
them, but drawn as `geometry::note_link`: across the middle of the span the note and its element share,
or corner to nearest point when they share none. In activity diagrams a note risen over a decision sits
24px beside the axis and 16px above it, and its link meets the diamond's upper edge.

## Leave from the facing side

**Rule.** A connector leaves its box on the side that faces the other end. When the boxes are apart on
both axes, the side along the flow wins.

**Why.** The line then takes the short way, and the flow direction stays visible at both ends.

**Source.** Purchase et al. 2002 (information flow ranked third for UML).

**Status:** enforced, by `router::facing`, used by every adapter, as the first choice. Two rules below
may move an end off the facing side: a dashed line that must go round
([Give the main line the main side](#give-the-main-line-the-main-side)), and, after routing, an end
whose line arrives from beside the box ([Never double back into a box](#never-double-back-into-a-box)).

## Give the main line the main side

**Rule.** Lines that leave one box are served in order of weight: a solid line (a flow, an assembly, an
association) before a dashed one (a dependency), and among equals the line on the main branch first
([Line up the main branch](flow.md#line-up-the-main-branch)). The heaviest line takes the side that
faces along the flow and runs straight. A lighter line takes a side that is still free. A dashed line
that passes other boxes on its way leaves on the outer side it will travel along and enters its target
on that same side.

**Why.** A dashed line is secondary to a solid one: the reader follows the solid chain first. In
Compiler pipeline, the dashed `invoke` leaves `Driver` on the right and enters `Code Generator` on the
right, with the same number of bends as a route beside the main chain, and leaves the main chain alone.
A symbol that would hang on the crowded side, such as the `ITarget` ball, moves to the room the dashed
line gave up.

**Source.** Ware et al. 2002 (continuity of the main path); a design decision of this project.

**Status:** partial, component diagrams only. A dashed dependency between two leaf components that
overlap across the flow, whose straight way runs through a box linked to neither end, and whose end
shares its facing side with another line or a stub, leaves by a free side across the flow instead: the
right one (the bottom one when the flow runs right) unless it is taken
([Slots](../internals/layout.md#slots), `secondary_sides`). Where it arrives,
[Never double back into a box](#never-double-back-into-a-box) turns it into its target's side. A stub
that shares a side with lines goes to the end of the side its name faces. Compiler pipeline in the
[component examples](../diagrams/component/examples.md#compiler-pipeline) shows it. Not done:

- the outer side is chosen by rule (right or bottom first), not from the route the line will take;
- the rule does not apply when the far box lies wholly to one side, where the facing side and L over Z
  already give that line an L;
- solid lines are not ordered by the main branch beyond what
  [Line up the main branch](flow.md#line-up-the-main-branch) does;
- package and state diagrams have no dashed rule.

## Never double back into a box

**Rule.** A line enters a box on the side it arrives from. It never passes the box, or runs along one of
its sides, to turn back into another side. When the facing side is full, the line takes the side next
to it that is nearer to where the line comes from.

**Why.** A line that turns back reads as if it came from somewhere else, and it spends two bends beside
the box, where ends are already close together.

**Source.** Drafting practice.

**Status:** enforced where the side is clear, in component, package and state diagrams.
[L over Z](../internals/layout.md#l-over-z) looks at the last three runs of every line: when the first of
them runs the same way as the last (a Z at the end) or back (a U), and the run across starts beyond the
box's side, the end moves to that side, at its middle, and the line loses a bend. The same tests as for
an L hold. Ends may move on leaf components (also beside stubs and ports, not onto their sides), on
interface lines, on leaf packages and classes, and on simple states; a state with internal behaviours
takes them on its left and right sides only below its compartment line (`Body::head`). The test is
`ends::doubled_back`. Not done: a side held by a stub, port or self-transition loop, or a line so close
to the box that the side has no 16px run, keeps the turn.

## Use a free side before you share one

**Rule.** When a box has more than one line toward the same direction, and a line can leave by a free
neighbouring side with no more bends and no new crossing, that line takes the free side instead of a
second slot on the shared side. A line that must bend anyway bends at the box: it leaves by the side
that faces its target across the flow and makes an L, and leaves the facing side to the line that can
be straight.

**Why.** Two ends on one side force at least one of the lines off the middle, and usually into a Z.

**Source.** Purchase et al. 2002 (bends); Schulze, Spönemann and von Hanxleden 2014 (free port sides in
layered layout).

**Status:** partial. The free side is taken after routing, by
[L over Z](../internals/layout.md#l-over-z) on the start of a line: a line that leaves a shared side and
at once turns past the box's corner leaves by the side it turns toward, and makes its L there. In the
component examples: in Hexagonal service the lines to `repo` and `publish` leave `Use Cases` at the top
and the bottom while the `Domain Model` line keeps the right side, straight; `Gateway`'s three lines in
API gateway leave by three sides; `IImages` leaves `Daemon` at the top in Container runtime. Ends that
share a side are judged together with their partners and spread 32px apart where the lines carry names.
Not done: there is no side choice before routing for solid lines, so a line that could run straight from
a free side is not moved there; Secure boot chain keeps two ends on one side of `Second Stage`.

## Take a return line around the outside

**Rule.** A line that runs against the flow (a return, a retry, a reset) is drawn as one loop on one
outer side of the drawing: it leaves its source on that side, runs outside the boxes it passes, and
enters its target on the same side. It does not leave by the side that faces along the flow, and it does
not share a side with the forward lines when the outer side is free. At a decision, the branch that
returns leaves the diamond by the side it returns along, not by the side of the forward flow. A return
that cannot use a side (the side is taken by a line that runs straight) goes all the way round and
enters its target from behind, on the side the forward flow came in by.

**Why.** Forward lines and return lines then never compete for a side, and the return reads as one
bracket beside the flow.

**Source.** Gansner et al. 1993 (reversed edges kept to one side); drafting practice.

**Status:** partial.

- State diagrams: [Never double back into a box](#never-double-back-into-a-box) moves both ends of a
  return to the outer side it runs along. In USB device, `bus reset` leaves `Configured` on the right,
  below its compartment line, and enters `Default` on the right; in Elevator controller,
  `technician reset` leaves `Fault` and enters `Idle` on the right.
- Activity diagrams: a `while` loop whose body ends in a merge with a free left corner returns from that
  corner to the left (Interrupt service with deferred work).
- Not done: a side that holds a self-transition loop is not freed for the return, so Process
  lifecycle's `wait(event)` and Circuit breaker's `probe succeeded` still go round (see
  [Draw self-transitions as compact loops away from the flow](kinds.md#draw-self-transitions-as-compact-loops-away-from-the-flow)).
  When no clear L exists, the return keeps the start the router's search gave it.

See also [Give the main line the main side](#give-the-main-line-the-main-side), which states the same
for a dashed line that passes other boxes.

## Prefer an L to a Z when the L is clear

**Rule.** When two boxes are apart on both axes and an L-shaped line between a side and a top (or
bottom) is clear of everything, use the L, with one bend, instead of a Z with two.

**Why.** It saves a bend without hiding the flow, as long as the last run is long enough for its
arrowhead.

**Source.** Purchase et al. 2002 (bends).

**Status:** enforced for plain boxes. State diagrams do this for final states, choices, junctions and
terminate symbols when choosing sides. After routing, [L over Z](../internals/layout.md#l-over-z) moves
one end of a Z to the side of its box that faces the line, when the L then meets no box, name, reserved
label or boundary, keeps 16px from other ends, gains no crossing, overlap or close run, and merges no
ends. An end that lands alone on a side lands at its middle. It applies to leaf components (also those
with stubs or ports, onto the sides that hold none, and to interface lines), leaf packages and classes
(not a package's top, which holds its tab), and simple states (a state with internal behaviours only
below its compartment line). Exception, in component and package diagrams: an end alone on its side does
not move when the far end shares its side, because that line is a branch of a fan, and an L that would
leave an end alone off the middle of its old side is dropped.

## Spread and order attachment points

**Rule.** Ends on one side are spread at least 24px apart, in the order that keeps their lines from
crossing, and each slides toward the line that makes its connector straight.

**Why.** Evenly spaced ends read as separate connectors. Their order decides whether lines cross just
outside the box.

**Source.** Schulze, Spönemann and von Hanxleden 2014 (port constraints in layered layout).

**Status:** partial. [Slots](../internals/layout.md#slots) spreads and orders ends by the position of
the other end. In component diagrams, ends that share a side are then spread evenly and centred on it
as a group, unless that takes an end off a straight line to another box. Not done: ordering by the side
the route arrives from.

## Keep ends off corners

**Rule.** An end stays at least 8px from a square box corner, and 16px from the curve of a rounded
state corner. Ends are first spread 16px from corners; nudging may slide them to 8px on square boxes.

**Why.** An arrowhead at a corner is ambiguous about which side it meets, and on a rounded corner it
meets a curve.

Bad: the arrowhead meets `B` in its rounded corner.

<figure class="drumlin-figure">
<svg xmlns="http://www.w3.org/2000/svg" class="drumlin pens" viewBox="0 0 260 170" width="260" height="170">
<style>
.drumlin.pens text { font-family: Helvetica, Arial, "Liberation Sans", sans-serif; fill: #22201c; }
.drumlin.pens .name { font-size: 13px; font-weight: 600; }
.drumlin.pens .small { font-size: 11px; fill: #22201c; }
.drumlin.pens .shape { fill: #fff; stroke: #22201c; stroke-width: 1.25; stroke-linejoin: miter; }
.drumlin.pens .edge { fill: none; stroke: #22201c; stroke-width: 1.25; stroke-linecap: butt; stroke-linejoin: miter; }
.drumlin.pens .dashed { stroke-dasharray: 7 3; }
</style>
<rect x="0" y="0" width="260" height="170" fill="#fff"/>
<rect x="16" y="16" width="112" height="40" rx="10" class="shape"/>
<text x="72" y="40.5" class="name" text-anchor="middle">A</text>
<rect x="136" y="114" width="112" height="40" rx="10" class="shape"/>
<text x="192" y="138.5" class="name" text-anchor="middle">B</text>
<path d="M72 56 L72 84 L144 84 L144 116" class="edge"/>
<path d="M139.5 106 L144 116 L148.5 106" class="edge"/>
</svg>
</figure>

**Source.** Drafting practice; [Layout levers](levers.md#box-size) (images of an 8px inset).

**Status:** enforced: 8px on square component and package boxes (`CORNER_INSET` in their routing),
16px on states, whose corners have a 10px radius, so that an arrowhead never lands on the curve. The
16px gap between ends stays.

## Keep arrowheads apart

**Rule.** Two ends on one side are at least 16px apart after nudging, and 24px when slots are first spread.

**Why.** Two arrowheads closer than their own width merge into one mark.

**Source.** Drafting practice; a design decision of this project (`SLIDE_GAP`).

**Status:** enforced, by `router::SLIDE_GAP` and the slot spacing in each adapter.

## End with a straight run

**Rule.** Every line leaves and enters a box with a straight run of at least 16px, so the arrowhead
sits on a straight piece and points squarely at the side.

**Why.** An arrowhead on a corner points in two directions at once.

**Source.** Drafting practice.

**Status:** enforced, by the 16px stems of `router::Terminal` and, at state pseudostates, the
`ARRIVAL_RUN`. Ends with a head or start decoration need more: see
[Leave room for the head](#leave-room-for-the-head).

## Leave room for the head

**Rule.** The last run before an arrowhead or triangle, and the first run after a start decoration
such as the containment circle-plus, is at least the decoration's length plus 8px, rounded up to the
grid: 24px for an open arrowhead (10px), a triangle (12px) or a circle-plus (12px).

**Why.** When the last bend sits right at the head, the corner runs into it and the arrow looks
squashed.

**Source.** Drafting practice.

**Status:** partial. `Terminal::reaching` puts the stem of a decorated end that far out, on the grid,
and nudging, L over Z and the narrow-gap lane all keep it. Component containers pad a side whose port
takes an arrow from inside by 32px, so the run fits between the port and the boxes inside. The quality
test counts runs shorter than the decoration plus 8px. Not done: state transitions that keep a reserved
label keep the 16px stem, because in the 64px gap between two states, two labelled lines going opposite
ways would cross if each needed 24px.

## Meet the middle

**Rule.** An end alone on a side of its box, or a port alone on a side of its component, sits at the
middle of that side. A lone connector between two boxes leaves and enters at the middles of its sides.
Where the boxes are not in line, first move them so their centres line up; failing that, keep both ends
centred and jog once, midway. Only ends that share a side spread along it, evenly and centred as a
group, and those may slide toward their lines. Where a line is held in place by its other end or by a
straight chain, the box moves under the line, not the line along the side: the line stays, and the box
shifts until the end (or the group of ends) is the same distance from both corners.

**Why.** An end off the middle of an otherwise empty side reads as careless: the line looks as if it
slid to avoid something. A box in line with its neighbour and a line through both middles look drawn
on purpose.

**Source.** Drafting practice.

**Status:** partial.

- Component and package diagrams: a lone end's slot is the middle of its side, and it does not slide
  when nudging straightens a line. A port sits at the middle unless connectors inside its component lead
  to it, when it lines up with them. [Alignment](../internals/layout.md#alignment) moves boxes across the
  flow so lone connectors run straight between middles. Component ends that share a side are centred as
  a group together with the group at the other end of their lines, so two boxes joined by several lines
  keep every line straight. A side that lines from a container's contents cross counts as shared; note
  links do not count.
- State diagrams: a state moves under a line into or out of an entry or exit point, rather than the line
  sliding off the state's middle (Entry and exit points: `Idle` under `retry`).
- The quality test counts ends alone on a side but more than 4px from its middle.
- Not done: state diagrams still slide other lone ends to straighten lines. Some component ends stay off
  the middle to keep clear of a package's tab or a stub, or because no growth or L could centre them.

## One junction per merge

**Rule.** Lines that merge, such as two connectors into one port, join at one point. A line that joins
another turns onto it where the other turns, or onto its straight run, never a few pixels before or
after.

**Why.** Two junctions a step apart look like a drawing error and make the reader wonder whether the
lines meet at all.

**Source.** Drafting practice.

**Status:** enforced. The last nudging pass ([Nudging](../internals/layout.md#nudging)) moves the
turning segment of the line that runs on back to where the other turns, if its line stands no worse.
The quality test counts pairs of lines that share an end and part less than 24px apart, and allows none.
Vehicle gateway shows it: the two `IFrames` lines join the trunk to port `bus` at one point.

## Space parallel lines evenly

**Rule.** Lines that share a channel are spread evenly across it, at least 8px apart, and a lone jog
sits in the middle of its gap.

**Why.** Wybrow et al. report that this nudging step has "a significant impact on the perception of
layout quality". Even spacing looks deliberate. Lines bunched on one side look accidental.

**Source.** Wybrow, Marriott and Stuckey 2009, section 5.

**Status:** enforced. The spread pass of [Nudging](../internals/layout.md#nudging) does this.

## Do not run along a boundary

**Rule.** A line never travels along a container's border. It crosses the border at a right angle.

**Why.** A line along a border merges with it, and reads as part of the container.

**Source.** Drafting practice.

**Status:** enforced. Container borders are soft strips in [Routing](../internals/layout.md#routing),
and [Nudging](../internals/layout.md#nudging) never moves a segment onto one.

## Join lines only where the notation does

**Rule.** Containment drawn with branching lines has one trunk from the owner's circle-plus. Other
relations each get their own line, even when they share an end.

**Why.** A shared trunk says "these belong together", which is true for containment (UML 12.2.4) and
false for, say, two dependencies.

**Source.** UML 2.5.1, 12.2.4, p. 248; Purchase et al. 2002 (76% preferred joined inheritance arcs,
where the notation joins them).

**Status:** enforced. Package containment draws a tree when every member sits in the next rank, and
otherwise routes each branch from the shared anchor.

## Put arrowheads where the notation says

**Rule.** Arrowheads, triangles, circles and sockets follow UML 2.5.1 for each relation: an open head
for a dependency, a hollow triangle for generalization, a filled head for a synchronous message, the
circle-plus on the owner of a containment.

**Why.** The notation is the meaning, so it is never traded for looks. See
[When rules conflict](introduction.md#when-rules-conflict).

**Source.** UML 2.5.1.

**Status:** enforced, by an audit of every glyph against the specification and the glyphs in
`core::glyphs`.
