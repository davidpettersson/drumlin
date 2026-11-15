# Roadmap

What drumlin does today, what is planned, and the known limitations of each diagram kind. Check here
before you report a drawing as a bug.

| Area | State |
|---|---|
| Component diagrams, with interfaces, ports and delegation | Prototype |
| Packages as containers | Prototype |
| Package diagrams with `«import»`, `«merge»` and containment | Prototype |
| Sequence diagrams with fragments, activations, creation and destruction | Prototype |
| State diagrams with composite states, regions and pseudostates | Prototype |
| Activity diagrams with decisions, loops, forks, signals, objects and partitions | Prototype |
| Notation audit against UML 2.5.1 | Done for all five kinds |
| JSON diagnostics | Under consideration |
| Selectable layout engines (`layout` statement, `--layout`) | Done: `longest-path`, the default |
| Network simplex ranking as a second engine | Done: `network-simplex`, opt-in |
| Drawing styles (`style` statement, `--style`) | Done: `pens`, the default, and `plain` |
| A lettering style: single-stroke technical lettering with its own text metrics | Done: `lettering`, opt-in |
| Cross-axis alignment: the main branch of a fork or merge runs straight | Done for component and package diagrams |
| Exit sides: no doubling back, a free side before a shared one, dashed lines and returns round the outside | Partial: after routing in component, package and state diagrams; dashed lines in component diagrams; loop returns in activity diagrams |
| Box growth by four grid steps, ordering across containers (see [the levers study](../principles/levers.md)) | Not scheduled |
| Rules marked *planned* in the [drawing principles](../principles/introduction.md) | Not scheduled |

## Known limitations

All kinds:

- No diagram has a UML frame, such as `sd` or `pkg`. UML makes the frame optional, and a title is a
  caption outside the drawing.
- Builds have been tested only on Linux so far.
- In the `lettering` style, text is drawn as pen strokes, with the words as invisible text over them
  for selecting and searching. A selection is highlighted in the viewer's sans-serif squeezed to the
  run's width, not traced around the strokes. Characters without a glyph (anything beyond Latin-1,
  Latin Extended-A in part, common punctuation, arrows and super- and subscript digits) are drawn as an
  open box, though the invisible text still holds them. Files are about three to four times the size of
  the same drawing in `pens`.

Component, package and state diagrams:

- Coordinates come from median placement, not an optimal solve, so an item can sit off the line of
  what it connects to, and the connector then bends. Alignment afterwards lines up the main branch of
  each fork or merge in component and package diagrams, but only where that adds no bends: a line whose
  end shares a box side with another keeps the slot offset as a jog, unless the line then turns
  off to a free side after routing (Container runtime). A fork's branches in a state diagram are centred on the fork, not on the column
  through it. Reserved labels can force a long edge
  to bend around them.

Component diagrams:

- Interface inheritance (`^`), port type labels, several interfaces on one lollipop and the optional
  compartments of a component are not expressible. All are optional in UML.
- Connectors sometimes carry a small jog near a box corner. A route can go around a composite's name.
- An interface glyph keeps 16px of line on each side unless the room for it would make the route
  cross or bend more; it then sits closer, with a shorter dependency arrow. A line passing a leaf
  component keeps only the router's usual distance from it; beside a container it keeps 24px.
- Exit sides change after routing. Alignment compares its layouts by routed bends, which include the
  turns, but it lines up only lines whose ends are alone on their sides before routing. A free side is used only by turning the start of a line
  that already bends; a line that could run straight from a free side is not moved there.

Package diagrams:

- Classifiers have no bodies, so attributes and operations cannot be listed.
- A dependency arrow between packages that do not overlap across the flow carries one jog.

Sequence diagrams:

- Gates and coregions are optional UML notation and are not drawn; found and lost messages and `par` cover the need.
- Lifeline heads are not repeated at the bottom.
- A fragment's operator tab can hide part of an active execution bar on the first lifeline it covers,
  and a note across several lifelines hides the bars under it.

State diagrams:

- Long edges in dense machines may still bend around other transitions' labels.
- Labels of several self-transitions on one side are stacked and offset, which makes them harder to
  match to their loops.
- A self-transition's loop always takes the side away from the flow, even when a transition could
  run straight from that side, so Process lifecycle's `wait(event)` and Circuit breaker's
  `probe succeeded` still go round.
- Local transitions are drawn on the right side of a composite.

Activity diagrams:

- Flows are block-structured, so unstructured flows cannot be written.
- Interruptible regions, pins, exception handlers, activity parameter nodes and hierarchical partitions are not drawn.
- The next statement after a side arm that stops still waits below that arm, because rows are shared.
- A note above a decision lengthens the flow into it, so a straight flow is not evenly spaced there.
- When branches stack because one crosses lanes another occupies, a flow can cross the stacked branch
  (Branches that interleave lanes).
