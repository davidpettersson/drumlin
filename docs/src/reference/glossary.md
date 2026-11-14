# Glossary

The terms this book uses, one meaning each. UML terms are given in plain words; the
[Elements](../elements/component/overview.md) pages show each symbol.

## Terms of the format

### Diagram kind

One of the five diagrams drumlin draws: `component`, `package`, `sequence`, `state` and `activity`,
named in the [header](../guide/file-basics.md). The book says *kind*, not *type*.

### Element

Anything you declare in the source and drumlin draws: a component, a package, a state, a lifeline, an
action. A *symbol* is how an element looks.

### Name, alias, title

The *name* of an element is the text drawn on it. An *alias*, given with `as`, is a short identifier for
the rest of the file. The *title* is the caption of the whole diagram, from the header.

### Label

Text written after `:` on a line between elements: a connector label, a message, a transition's
trigger, guard and effect, or a keyword such as `«import»`.

### Keyword and stereotype

A *stereotype* is what you write between `<<` and `>>`, such as `<<database>>`. drumlin draws it as a
*keyword* in guillemets, `«database»`, above the name. UML defines some keywords itself, such as
`«interface»` and `«import»`.

### Layout hint

A statement that adjusts the arrangement without coordinates: `left-of`, `right-of`, `above`, `below`
and `align`. See [Layout hints](hints.md).

### Layout engine

The method that places the boxes of a component, package or state diagram: `longest-path` or
`network-simplex`. See [File basics](../guide/file-basics.md#layout-engine).

### Style

How a drawing is inked: `pens`, `plain` or `lettering`. See [Styles](../guide/styles.md).

## Terms of the drawing

### Box

The outline drawn for an element: a component's rectangle, a state's rounded rectangle, a package's
folder. A *container* is a box that holds other boxes (a component with parts, a package with members,
a composite state).

### Line

Any drawn connection between two elements. Each diagram kind has its own UML name for it, used on its
pages: a *connector* in component diagrams, a *relation* in package diagrams, a *message* in sequence
diagrams, a *transition* in state diagrams and a *flow* in activity diagrams. *Edge* is used only for the
layout's internal graph, and *arrow* only for the arrow you type (`-->`) and the arrowhead at a line's end.

### Connector

In a component diagram, a line between two components, ports or packages. A *dependency* is a dashed
connector (`..>`). An *assembly* connector joins a required interface to a provided one; a *delegation*
connector runs from a component's port to a part inside it.

### Interface, lollipop, socket

An *interface* is a named set of services. A component *provides* it (drawn as a *lollipop*, a circle on
a stem) or *requires* it (a *socket*, a half circle). Two matched in a joined symbol make a
*ball-and-socket*.

### Port

A named point on a component's boundary, drawn as a small square. A *simple* port has one interface.

### Lifeline, execution

In a sequence diagram, a *lifeline* is a participant: a head with a dashed line down the page. An
*execution* is the thin bar on a lifeline while it is active.

### Combined fragment

In a sequence diagram, a frame around messages with an operator such as `alt`, `loop` or `par`.

### Pseudostate

In a state diagram, a small symbol that steers transitions but is not a state: initial, choice,
junction, fork, join, terminate, history.

### Region

One of the parallel parts of a composite state, separated by dashed lines. Regions run at the same time.

### Guard

A condition in square brackets, `[ok]`, that must hold for a transition or flow to be taken.

### Partition

In an activity diagram, a swimlane: a band that holds the steps one participant performs.

### Rank

One step of the layout's main direction. Boxes in one rank sit side by side across the flow.

### Golden file

A source file and its expected SVG in `tests/golden/`. The tests fail when a drawing changes by a byte.
