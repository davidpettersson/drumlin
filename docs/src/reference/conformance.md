# What follows UML, and what is drumlin's choice

drumlin says its notation is checked against UML 2.5.1. This page says exactly what that covers, because
the specification defines some things firmly and leaves others open.

## What UML 2.5.1 defines

- **The notation of each element.** The clauses on structured classifiers, packages, state machines,
  activities and interactions say how a component, a port, a pseudostate, a decision or a message is
  drawn. Each page under **Elements** cites the sub clause its symbol follows.
- **The keywords** between guillemets, such as `«import»` and `«merge»` (Annex C).
- **A taxonomy of diagram kinds, loosely.** Annex A names the kinds (activity, class, component,
  deployment, interaction, package, state machine, use case, and the more specific ones such as the
  sequence diagram) and says which clause describes the constructs of each. It also says that a diagram's
  kind comes from the primary symbols in it, that kinds may be mixed, and that "the boundaries between the
  various kinds of diagram types are not strictly enforced".
- **How a diagram's layout is interchanged between tools.** Annex B (UML Diagram Interchange), built on
  the OMG Diagram Definition specification, describes a file format for the positions of shapes and the
  points of lines. It has a class for each diagram kind, but it does not list which elements a kind may
  hold either.

## What UML 2.5.1 leaves open

- **Which elements a diagram kind accepts.** There is no normative list. A tool has to choose.
- **How a drawing looks beyond the symbols.** Line weight, colour, type, spacing, where a label sits and
  how a line is routed are not specified. Annex B gives a font name and size as the only style a modeller
  controls, and says the shapes themselves follow the notation clauses.
- **The frame.** A diagram may have a frame with a heading, such as `sd Checkout`. Annex A makes it
  optional.

## What drumlin does

| Matter | drumlin |
|---|---|
| Element notation | Follows the UML 2.5.1 sub clause cited on each element's page. Optional notation that drumlin leaves out is listed under [known limitations](../internals/roadmap.md#known-limitations). |
| Keywords | Written between guillemets as in Annex C. Stereotypes you write (`<<service>>`) are shown the same way. |
| Diagram kinds | Five: component, package, sequence, state and activity. They match the Annex A kinds component, package, sequence (an interaction diagram), state machine and activity. |
| What each kind accepts | **drumlin's own choice**: a small set per kind, listed in the [grammar](grammar.md). It is narrower than UML allows. A package may hold components in a component diagram and classifiers in a package diagram, but a state machine cannot sit inside a component, although UML would permit that mixture. |
| Frame and heading | Not drawn. The title is a caption above the drawing, not a UML heading. |
| Look | **drumlin's own choice**: the [drawing principles](../principles/introduction.md) and the [styles](../guide/styles.md). None of it comes from the specification, and none of it contradicts it. |
| Interchange | None. drumlin reads its own text format and writes SVG. It keeps no UML model, so it neither reads nor writes XMI or UML Diagram Interchange files. |

So a drumlin drawing uses standard UML symbols, in diagrams whose scope drumlin chose. Where this book says
"component diagram", it means the kind that Annex A names, restricted to what the grammar accepts.
