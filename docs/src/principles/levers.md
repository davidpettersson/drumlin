# Background: the levers study

This page summarises a study that chose which layout improvements to build first. It is background for
contributors; you do not need it to use drumlin. The study measured the layout of an earlier version, so
its numbers describe that baseline, not today's drawings. Every lever it recommended is now built, and
the principles it backs say what drumlin does today.

## What was measured

Three faults showed in the rendered component, package and state diagrams of the time:

- edge labels forced bends that were not needed;
- container and composite names blocked lines that could run straight if the name moved;
- some boxes were a little too small, so a connector jogged instead of running straight.

Each fix (a *lever*) was tested as a counterfactual over the 103 golden files of the time: its upper
bound, which removes the cause outright, and a realistic variant that keeps every rule in the
principles (no label touching ink or reading ambiguously, no arrow through a name, no two connectors
merged). Crossings, overlaps and bends were counted with the same metrics the quality test uses, against
a baseline of 5 crossings, 0 overlaps and 341 bends. The experiment code was not kept.

| Lever | Bends saved at the time | Became |
|---|---|---|
| Grow end boxes, up to 2 grid steps | 90 | [Grow a box rather than bend a line](shapes.md#grow-a-box-rather-than-bend-a-line) |
| Grow end boxes, up to 4 grid steps | 128, with uneven rows | Not built |
| Corner inset of 8px on square boxes | 12 | [Keep ends off corners](lines.md#keep-ends-off-corners) |
| Arrows may end on a package tab | 6 | [Let arrows end on a package's tab](names.md#let-arrows-end-on-a-packages-tab) |
| State names give way to entry points | 4 | [Let a name give way to a line](names.md#let-a-name-give-way-to-a-line) |
| State labels placed after routing | 26 | [Never let a label force a bend](labels.md#never-let-a-label-force-a-bend) |
| An L instead of a Z where the L is clear | 28 (counted by cause, not tested as a variant) | [Prefer an L to a Z when the L is clear](lines.md#prefer-an-l-to-a-z-when-the-l-is-clear) |

Rejected: dropping label reservation altogether (8 new crossings and many label collisions), shrinking
the 16px gap between ends (two connectors then leave one point together), and moving the names of
component composites (no gain, and UML 9.2.4.1 asks for them centred).

## Labels

Component and package diagrams already placed labels after routing, so labels never caused a bend there.
In state diagrams a label was a dummy in the layered layout, the line was routed through the label's
anchor, and every label was an obstacle for every other line. Routing freely saved many bends but added
crossings, and placing the labels afterwards left one label in twelve colliding and one in five as close
to a foreign line as to its own. The workable variant, a *hybrid*, frees a label only where it can be
placed clear, near and unambiguous, and keeps the reserved drawing for any diagram where the result is
worse. That is how [Labels after routing](../internals/layout.md#labels-after-routing) works.

## Titles

As routing obstacles alone, composite names and package tabs cost no bend at all. They cost bends only by
narrowing the span of a side where ends may attach. Arrows ending on top of a blank package tab, and a
composite state's name moving aside for an entry point, recover most of that. Ignoring names outright is
not drawable: arrows then pierce package tabs and run through state names.

## Box size

For each line with bends, one end box grew by one to N grid steps toward the other end, into free space,
when the totals got no worse and no two connectors merged. The diagram's bounds did not change. Two
steps added about 1.6% to the box area and was hard to notice; four steps saved more bends, but boxes in
one rank visibly lost their common height. A smaller corner inset of 8px saved bends on square boxes; on
rounded state corners it put the arrowhead on the curve. An 8px gap between ends let two ends meet.

## The five crossings

The baseline had five crossings in three drawings:

| Drawing | Cause |
|---|---|
| Order Platform (two golden files) | A container orders its children without seeing the lines outside it, so `Event Bus` and `Postgres` sit in an order that makes `Billing`'s line cross the `SQL` dependency. |
| Compiler pipeline (two golden files) | The slot of `invoke` on `Code Generator` was ordered by the position of `Driver` before routing, though the route came down the other side. |
| Hexagonal service | Slots on the right side of `Use Cases` were in the wrong order for their routes. |

Box growth with the corner inset removed the Compiler pipeline crossings, and spreading ends in the order
of their lines removed the Hexagonal service one. The Order Platform crossing remains: it needs ordering
across containers ([Order across containers too](flow.md#order-across-containers-too)).

## Images checked

Every variant was rasterized and looked at, not only counted. Freed labels straightened lines in
Elevator controller and TCP connection, but some then lay on other lines or floated between two parallel
returns, which is why the hybrid keeps those reserved. Arrows ending on a package tab read correctly;
ignoring names let arrows pierce tabs. Every bend saved by box growth read better, but uneven heights
showed at four grid steps. An 8px inset on a rounded state put the arrowhead into the corner, and an 8px
gap between ends merged two connectors in Hexagonal service.
