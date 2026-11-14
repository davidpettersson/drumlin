# Labels

Labels here are the text on lines: connector labels, keywords such as `«import»`, transition triggers
and guards, message names. Names of boxes and containers are in [Names and titles](names.md).

## Keep a label beside its own line

**Rule.** A label sits next to a straight run of its own line, 6 to 8px from it, and close to the
middle of that run.

**Why.** Closeness is what ties a label to its line (Gestalt proximity). A label far from its line has
to be matched by searching.

```drumlin
diagram state "Labels beside their lines"

[*] --> Idle
Idle --> Busy : start [ready] / log()
Busy --> Idle : done
```

**Source.** Gestalt proximity; Kakoulis and Tollis 1997.

**Status:** enforced. A reserved state label sits 8px from its anchor
([Placement](../internals/layout.md#placement)), and a state label placed after routing sits with its
ink 8px from its own segment. `labels::Placer` tries five positions along each segment, 6px off it, and
prefers the middle and longer runs. The quality test counts labels more than 12px from their own line.
Exceptions: a two-line label above a line in a rightward flow, whose upper line is 14px away, and a
self-transition label stacked under its loop.

## Never leave a label between two lines

**Rule.** No other line may be as near to a label as its own line. A label that sits between two
parallel lines must move, or one of the lines must.

**Why.** A label equally near two lines belongs to neither, and a reader guesses. This is the
*unambiguous reading* rule, second only to notation in [When rules conflict](introduction.md#when-rules-conflict).

Bad: `retry` could belong to either line.

<figure class="drumlin-figure">
<svg xmlns="http://www.w3.org/2000/svg" class="drumlin pens" viewBox="0 0 330 220" width="330" height="220">
<style>
.drumlin.pens text { font-family: Helvetica, Arial, "Liberation Sans", sans-serif; fill: #22201c; }
.drumlin.pens .name { font-size: 13px; font-weight: 600; }
.drumlin.pens .small { font-size: 11px; fill: #22201c; }
.drumlin.pens .shape { fill: #fff; stroke: #22201c; stroke-width: 1.25; stroke-linejoin: miter; }
.drumlin.pens .edge { fill: none; stroke: #22201c; stroke-width: 1.25; stroke-linecap: butt; stroke-linejoin: miter; }
.drumlin.pens .dashed { stroke-dasharray: 7 3; }
</style>
<rect x="0" y="0" width="330" height="220" fill="#fff"/>
<rect x="16" y="16" width="112" height="48" class="shape"/>
<text x="72.0" y="44.5" class="name" text-anchor="middle">A</text>
<rect x="200" y="16" width="112" height="48" class="shape"/>
<text x="256.0" y="44.5" class="name" text-anchor="middle">B</text>
<rect x="16" y="156" width="112" height="48" class="shape"/>
<text x="72.0" y="184.5" class="name" text-anchor="middle">C</text>
<rect x="200" y="156" width="112" height="48" class="shape"/>
<text x="256.0" y="184.5" class="name" text-anchor="middle">D</text>
<path d="M100 64 L100 156" class="edge"/>
<path d="M95.5 146 L100 156 L104.5 146" class="edge"/>
<path d="M228 64 L228 156" class="edge"/>
<path d="M223.5 146 L228 156 L232.5 146" class="edge"/>
<text x="164" y="114" class="small" text-anchor="middle">retry</text>
</svg>
</figure>

**Source.** Kakoulis and Tollis 1997 (a label must be associated with exactly one edge); drafting practice.

**Status:** partial. A state label placed after routing must be 12px nearer its own line than any
other, or it keeps its reserved spot. The quality test counts every connector label with another line at
least as near as its own. Not done: component and package labels are placed by fewest collisions and
are not checked for this when placed.

## Never let a label touch other ink

**Rule.** A label does not overlap a box, a name, another label or any line.

**Why.** Text over ink is hard to read, and a label on a line seems to belong to it.

**Source.** Purchase et al. 2002 (text legibility); Castelló, Mili and Tollis 2000.

**Status:** partial. Reserved state labels are obstacles for every line, so no line crosses them, and a
state label placed after routing must clear every line, outline, arrowhead, shape and label. The quality
test counts labels within 2px of a line or an outline, or on a shape or label. Not done: the placer for
component and package labels takes the position with the fewest collisions, but cannot make room when
every position collides.

## Write labels horizontally

**Rule.** All text runs left to right, horizontally, including labels on vertical lines.

**Why.** 86% of subjects preferred horizontal labels in UML diagrams.

**Source.** Purchase, Allder and Carrington 2002.

**Status:** enforced. No text is rotated.

## Put labels on a consistent side

**Rule.** In a diagram that flows down, a label sits right of its vertical line. In one that flows
right, it sits above its horizontal line. A reverse pair between two items keeps both labels on the
outside, so the two lines run close together.

**Why.** A consistent side lets the eye find each label without searching, and keeps lines and labels
from interleaving.

**Source.** Drafting practice; a design decision of this project.

**Status:** enforced for state diagrams (`label_side`, `order_label_sides`). Component labels go right of
vertical runs and above horizontal runs. Package labels may also take the left of a vertical run, to
avoid a collision.

## Reserve room for labels

**Rule.** The layout makes room for long labels before lines are routed, instead of hoping that a
spot will be free afterwards.

**Why.** A placer that chooses among positions cannot make space, and fails on dense diagrams. Label
dummies that take part in the layout guarantee room (dot, KLay).

**Source.** Gansner et al. 1993; Schulze, Wechselberg and von Hanxleden 2018; Castelló et al.
2000.

**Status:** partial. State transition labels are dummies in the layered layout
([Placement](../internals/layout.md#placement)). Component and package diagrams flowing right widen the
rank gap by the label's width. Elsewhere labels are placed after routing.

## Never let a label force a bend

**Rule.** A label follows its line, not the reverse. A line should run as straight as it would without
its label, and the label then takes a free spot beside it. When no spot is free, the label keeps its
reserved place, and the line goes through it.

**Why.** Labels serve lines. A transition routed through its label's reserved anchor jogs just to pass
it: with every label reserved, the lines of `near target floor`, `speed zero`, `levelled at floor` and
`fully open` in Elevator controller each jog for this reason alone.

**Source.** Purchase et al. 2002 (bends); [Layout levers](levers.md#labels).

**Status:** partial, for state diagrams
([Labels after routing](../internals/layout.md#labels-after-routing)). The layout still reserves every
label's room. Each line that bends is routed again without its waypoint, and its label is placed beside
the new line, on the segment and side that read best. A label keeps its reserved spot when no place is
clear, near (12px) and unambiguous, when its line gains no straightness, or when its line gains a
crossing. A diagram keeps the fully reserved drawing unless the result has fewer bends and no more
crossings, overlaps, merged ends or label flaws. Freed labels include `motor overcurrent / cut power`
(Elevator controller), `close / send FIN` (TCP connection) and `wait(event) / block()` (Process
lifecycle). Not done: `near target floor` keeps its spot, because right of its straightened line it
would lie on `next call`, and left of it on the region border. Component and package labels are already
placed after routing, so they never force a bend.
