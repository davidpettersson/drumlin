# Shapes and spacing

Rules for the grid, the pens and lettering, the size of boxes and the gaps between them.

## Snap to the grid

**Rule.** Every box corner, connection point and line segment lies on an 8px grid. Box sizes are
multiples of 16px.

**Why.** A grid makes alignment the default instead of an accident: two boxes of one height in one
rank share their top, centre and bottom, and parallel lines keep whole-step distances. It is the
squared paper of hand drafting.

**Source.** Drafting practice.

**Status:** enforced. [Placement](../internals/layout.md#placement) snaps positions; slots and lanes
snap in [Slots](../internals/layout.md#slots) and [Routing](../internals/layout.md#routing). Exception:
reserved label boxes are measured from their text, so their edges need not lie on the grid.

## Draw with three pens

**Rule.** The default style, `pens`, inks every line and every letter in one warm black (`#22201c`),
with white fills and no grey. Three pen weights make a hierarchy:

| Pen | Width | Draws |
|---|---|---|
| Heavy | 1.75px | Outlines of boxes that hold other boxes: a component with parts, a package with members (tab and body), a composite state, an activity partition with its name head |
| Medium | 1.25px | Outlines of ordinary boxes and symbols: leaf components, classifiers, leaf packages, simple states, actions, object nodes, decisions, lifeline heads, execution bars, ports, balls and sockets, pseudostates, history and final circles, entry and exit points, the circle-plus of containment, the actor figure. Solid connector lines, their open arrowheads and hollow triangles, and the strokes of symbols (the cross of an exit point, a terminate, a flow final or a destroyed lifeline) |
| Fine | 0.75px | Dashed lines: dependencies, note links, lifelines, replies and create messages, dividers between regions, operand dividers. The arrowhead or hollow triangle at the end of a dashed line. Notes and their folded corners. The component icon. Compartment dividers in a state. The frame and name tab of a combined fragment (`alt`, `opt`, `loop` and the rest) and of an interaction use (`ref`) |

Filled marks, such as the initial dot, fork and join bars, filled arrowheads and the dot of a lost or
found message, are solid ink. Dashes are 7 long with gaps of 3, so a fine dashed line still reads as
a line. Connector bends are sharp, with mitred joins and butt caps; the rounded corners of states and
actions stay, because they are notation. Text keeps one size per role and the same ink as the lines.

**Why.** A line weight hierarchy is standard drafting practice: ISO 128 sets line types and groups of
thicknesses, so that thick lines carry the visible outlines and thin lines carry dimensions, hatching
and other secondary information. Here the heaviest pen marks the structure the eye should find first,
the boxes that group others; the medium pen marks the things and the relations between them; the fine
pen marks what annotates rather than structures: notes, dependencies, dividers and icons. A fragment
frame annotates a stretch of an interaction, so it takes the fine pen. The weights stay within a ratio
of about 1:2.3, so the hierarchy is clear but not loud. Dashes still mean a dependency or comment, never
emphasis.

**Source.** ISO 128 (technical drawings: line types and line thicknesses); Tufte's smallest effective
difference.

**Status:** enforced, by the `pens` style in `core::style`: each drawn mark names its role, and the style
maps the role to a class and the class to a pen. `style plain` keeps one pen for every outline and line
(1.25px, `#1f2933`), with grey small text and softened bends. `style lettering` keeps the three pens
and letters the text with a fourth, lighter one: see [Letter with one pen](#letter-with-one-pen).

## Letter with one pen

**Rule.** In the `lettering` style every piece of text is single-stroke technical lettering, drawn in
the warm black ink with one round pen of 1.1px, lighter than the lightest line pen. Names and the title
are in capitals with 0.06em letter spacing (names on a 12.5px em, the title on 16px); labels, keywords,
notes and other small text are upright mixed case on an 11px em, whose x-height matches the default
style's. Bold names become capitals, abstract names lean 15 degrees, and a label drawn over a line
breaks the line with a white band. The digit one has a flag and no foot, and the zero is slashed and
narrower than the capital O, so neither reads as a letter.

**Why.** This is how a draftsman letters a drawing with a lettering guide: one pen, vertical or
uniformly inclined strokes, capitals for what must be found. ISO 3098 sets the inclined form at 15
degrees from the vertical and asks that similar characters stay distinct. Capitals read larger than
mixed case at one em, so names can be a little smaller than the default style's bold names. One pen for
all lettering keeps it apart from the line hierarchy: text is read, not followed.

**Source.** ISO 3098 (technical product documentation: lettering); ISO 128 for the pens.

**Status:** enforced, by the `lettering` style (`core::lettering`). The glyphs are drumlin's lettering,
derived from Routed Gothic by Darren Embry (SIL Open Font License 1.1). Text drawn as strokes cannot be
selected or searched in the SVG; each run carries its words as an accessible name.

## Size boxes to their text

**Rule.** A box is as wide as its name and keyword need, plus padding, and never smaller than the
kind's minimum (128 by 56 for a component, 96 by 40 for a state). Widths come from embedded Helvetica
metrics, so a box is the same size on every machine. In the `lettering` style they come from the
lettering's own advance widths.

**Why.** A box that fits its text reads as one unit. A fixed size either crowds long names or leaves
short ones lost. Measured text keeps the layout deterministic.

**Source.** A design decision of this project.

**Status:** enforced, by each adapter's size function (`leaf_size`, `state_size`).

## Leave room for what attaches

**Rule.** A side that carries several ends, ports or interface stubs is long enough to space them at
least 24px apart, 16px clear of its corners. Where a line on a left or right side carries a name, its
ends are 32px apart, so the name fits between two lines. A box grows across the flow until its sides
have that room: three named lines on one side make a box 96px tall, not 64px. A state with many
transitions is wider.

**Why.** Attachment points that crowd at a corner or touch each other make arrows unreadable, and
force lines to jog to reach them.

**Source.** Drafting practice; [Spread and order attachment points](lines.md#spread-and-order-attachment-points).

**Status:** enforced for stubs and ports, by `side_needs` (component), `slot_room` (package) and the
incident count in `state_size`. Partial for connector ends:

- Component diagrams count the ends the flow will put on each side (`side_needs`) and lay the box out
  again with that room. The taller layout is kept only where the first one has an end nearer a corner
  than 16px or two ends nearer than 24px, and the taller one routes no worse. Ends that share a side are
  spread 32px apart where a line carries a name (`NAMED_SLOT`).
- Not done: package and state diagrams do not count connector ends.

## Keep gaps even and generous

**Rule.** Ranks are at least 64px apart and neighbours in a rank 40px apart. A gap grows when a label
or interface needs it, and never shrinks below that.

**Why.** Even spacing reads as order. Generous gaps leave channels for lines to run in without
crowding, which is cheaper than any later repair.

**Source.** Gansner et al. 1993 (separation in layered layout); drafting practice.

**Status:** enforced, by `layered::Spacing` and the edge gaps each adapter passes.

## Grow a box rather than bend a line

**Rule.** When a connector jogs only because its two end boxes miss each other by a few grid steps,
grow one box by up to two grid steps on a side, toward the other end, into free space, so that the line
runs straight.

**Why.** Bends rank above uniform sizes (see [When rules conflict](introduction.md#when-rules-conflict)).
A box that is 16px wider costs almost nothing, while the jog it removes costs a turn in every reading of
that line. Below, `Message Bus` grows 8px at its top and bottom, and each target 8px toward it, so all
three connectors run straight; without growth, `Message Bus` is no taller than `Checkout` and two of the
connectors jog.

```drumlin
diagram component "Lines straightened by a taller box"

component Checkout
component "Message Bus" as bus
component Payments
component Inventory

Checkout --> bus
bus --> Payments : OrderPlaced
bus --> Inventory : OrderPlaced
```

A box grows only into free space: 16px clear of every other box, inside its container's padding, and
clear of reserved labels, self-transition loops, interface stubs and ports. A box with stubs, ports or
self-transitions keeps its size. Growth across the flow is tried before growth along it. The bounds of
the diagram do not change.

**Source.** Purchase et al. 2002 (bends 91%); [Layout levers](levers.md#box-size).

**Status:** enforced, by `core::grow` after layout ([Box growth](../internals/layout.md#box-growth)), at
up to two grid steps. A growth is kept only when routing again shows the connector lost a bend, no total
rose, and no two connectors came to leave one point together. Not done: growth by four steps, which the
[levers study](levers.md#box-size) measured as saving more bends at the cost of more uneven rows.

## Keep a rank even

**Rule.** Boxes in one rank share a centre line along the flow, and should look like one row. When a
box grows, prefer growth that keeps the row tidy, such as growing toward the gap between rows rather
than away from it.

**Why.** Uneven heights make a row read as a staircase. This is the uniformity that box growth trades
away, so it limits how far a box may grow.

**Source.** Drafting practice; the images in [Layout levers](levers.md#images-checked).

**Status:** partial. Items in a rank are centred on one line (`place_along`). Box growth tries growth
across the flow first, which keeps a rank's extent along the flow even, and grows at most 16px on a
side. Not done: a box may still end up a step taller or wider than its neighbours (`Kafka Adapter` in
Hexagonal service, `Mobile App` in API gateway), and a box that grows on one side only has its name up
to 8px off its neighbours' line.
