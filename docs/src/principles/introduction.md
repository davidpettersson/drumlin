# Drawing principles

This part of the book states the rules a drumlin drawing follows, and why. Read it if you want to know
why a drawing looks the way it does, or if you change the layout: every change is judged against these
rules. You do not need it to write diagrams.

drumlin aims to draw like a careful draftsman: someone who knows the notation, works on squared paper
with a set of technical pens, and redraws a line rather than leave it crooked. Every line is straight
unless it has a reason to turn, every label sits where it can belong to only one thing, and the space
between shapes is even.

```drumlin,render-only
diagram state "A careful drawing"

[*] --> Idle
Idle --> Running : start
Running --> Paused : pause
Paused --> Running : resume
Running --> [*] : stop
```

The state diagram above follows the rules. It flows one way, from top to bottom. The lines are
orthogonal, and none crosses another. Each label sits beside its own line. The reverse pair between
`Running` and `Paused` runs as two parallel lines, with their labels on the outside. The one line that
turns, `stop`, turns because three lines share the bottom of `Running`.

## How rules are written

Each principle has:

- **a name**, in the imperative, such as *Avoid crossings*;
- **the rule** itself;
- **why** it matters;
- **a source**: the UML 2.5.1 specification, a published study of graph drawing or diagram reading
  (such as Purchase 1997 and 2002, Ware et al. 2002, Gansner et al. 1993, Wybrow et al. 2009 or
  Castelló et al. 2000), drafting practice, or, where none of these settles it, a design decision of
  this project. The [levers study](levers.md), measured on drumlin's own drawings, backs some rules too;
- **its status** in drumlin:
  - *enforced*: drumlin does this in every diagram it lays out, and the linked step of
    [How layout works](../internals/layout.md) is where;
  - *partial*: drumlin does this in some diagram kinds or cases, or tries but does not guarantee it;
    the status says what is not done;
  - *planned*: drumlin does not do this yet.

Where it helps, a rule shows a good drawing, rendered by drumlin, or a bad one, drawn by hand.

Where a status mentions a measure, the quality test in `src/diagrams/quality.rs` counts it over the
golden files and fails when a change makes the total worse. Run
`cargo test --lib quality -- --nocapture` for the current numbers; this book does not repeat them.

## When rules conflict

Rules conflict all the time. A label wants the straight line's gap, which another line wants too.
Growing a box straightens a connector but makes a row uneven. drumlin settles each conflict in this
order, highest first:

1. **Correct notation.** The UML 2.5.1 notation is never traded for looks. A package with contents
   keeps its name in the tab, even where a line would run straighter without the tab.
2. **Unambiguous reading.** Every line, label and name must belong visibly to one thing. A label
   floating between two lines, an arrowhead on a corner, or two connectors leaving one point together
   are worse than any jog.
3. **No overlaps.** No shape on a shape, no line on a line, and no line through a shape or a name.
   Two lines on one lane cannot be told apart, so this ranks above crossings.
4. **No crossings.** Crossings matter most in every human-factors study of graph drawing (Purchase
   1997; 93% preference in Purchase et al. 2002).
5. **Few bends.** Bends come next in the preference data (91%), and continuity of a path helps it be
   traced (Ware et al. 2002).
6. **Flow and alignment.** A single direction, ranks, centred and symmetric placement.
7. **Compactness.** Small area and short lines, but never at the cost of the rules above.
8. **Uniformity.** Even box sizes and even spacing. This is the first rule to give way: a box may grow
   to straighten a line.

Within one level, the main thing wins over the secondary thing. The spine of the drawing comes before a
side branch, a solid line before a dashed one, a forward line before a return, and any line before a
note. So when two lines want one side, or only one of them can be straight, the main one gets it
([Give the main line the main side](lines.md#give-the-main-line-the-main-side)). Symmetry (level 6)
gives way to a straight main branch (level 5): a fan is centred only when it has no main branch.

A rule that would make a drawing worse on a higher level is not applied to that drawing. drumlin lays
the drawing out with and without the rule and keeps the better one, so a status may say "enforced where
it costs no crossing and no bend".

The order follows the published studies: crossings first, then bends, with orthogonality held
throughout by construction. Readability and notation sit above everything, because a drawing that
reads wrongly is worse than one that reads slowly.

## The pages

- [Shapes and spacing](shapes.md): grid, pens, lettering, box sizes and box growth.
- [Lines and routing](lines.md): orthogonal lines, bends, crossings, overlaps, sides, attachment points,
  channels and arrowheads.
- [Labels](labels.md): closeness to their line, no ambiguity, and never forcing a bend.
- [Names and titles](names.md): where names go, and when they give way to lines.
- [Flow and arrangement](flow.md): direction, ranks, columns, alignment, symmetry and grouping.
- [Notes per diagram kind](kinds.md): where component, package, state, sequence and activity diagrams
  add or bend rules.
- [Background: the levers study](levers.md): the measurements that chose which rules to build first.
