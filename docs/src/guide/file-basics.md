# File basics

This page covers what every drumlin file shares: the header, the drawing direction, the layout engine,
the style and comments. Read it after [Getting started](getting-started.md). The statements of each
diagram kind are on its page under **Diagrams**.

A file may begin with a header line that names the diagram kind and a title, and a few statements that
set up the drawing:

```text
diagram component "Title"
direction right
layout longest-path
style pens
```

All four lines are optional:

- **`diagram <kind> ["Title"]`** names the [diagram kind](../reference/glossary.md#diagram-kind):
  `component`, `package`, `sequence`, `state` or `activity`. A file without it is a component diagram.
  The title is drawn as a caption above the diagram.
- **`direction`** sets which way the diagram flows (below).
- **`layout`** picks a [layout engine](#layout-engine).
- **`style`** picks how the drawing is inked: see [Styles](styles.md).

`direction`, `layout` and `style` may stand on any line, and the last one wins. Put them under the
header, where a reader expects them.

## Direction

`direction right` places dependencies left to right. It is the default for component diagrams, and suits
most of them, because labels read naturally on horizontal lines.

```drumlin
diagram component "direction right"
direction right

component A
component B
component C
A --> B
B --> C
```

`direction down` places dependencies top to bottom. It is the default for package and state diagrams,
and suits pipelines and layers.

```drumlin
diagram component "direction down"
direction down

component A
component B
component C
A --> B
B --> C
```

Sequence and activity diagrams always run top to bottom and take no `direction`.

## Layout engine

A layout engine decides where the boxes of a component, package or state diagram go. Routing, labels and
drawing are the same for every engine, so you can compare two engines on one file. There are two:

- **`longest-path`**, the default. Each box goes one rank after the furthest box that points to it, and
  a box that nothing points to moves down next to what it points to.
- **`network-simplex`** ranks the boxes so that the lines are as short as possible in total (network
  simplex, after Gansner and others, 1993). A box with one line in from high up and several lines out
  to boxes far below moves down next to them.

For nearly every small diagram the two draw the same:

```drumlin
diagram state "layout longest-path"
layout longest-path

[*] --> Idle
Idle --> Running
Running --> Stopping
Stopping --> [*]
```

Try `network-simplex` when a box sits far above the boxes it points to and its lines run long. Here
`Flags` points to `Orders` and `Billing` at the bottom. Longest path puts it under `Web`, so both its
lines run the height of the drawing:

```drumlin
diagram component "Checkout, longest-path"
direction down
layout longest-path

component Web
component Gateway
component Cart
component Pricing
component Orders
component Billing
component Flags
Web --> Gateway
Gateway --> Cart
Cart --> Pricing
Pricing --> Orders
Pricing --> Billing
Web ..> Flags
Flags ..> Orders
Flags ..> Billing
```

Network simplex moves `Flags` down beside `Pricing`, which makes one line longer and two shorter:

```drumlin
diagram component "Checkout, network-simplex"
direction down
layout network-simplex

component Web
component Gateway
component Cart
component Pricing
component Orders
component Billing
component Flags
Web --> Gateway
Gateway --> Cart
Cart --> Pricing
Pricing --> Orders
Pricing --> Billing
Web ..> Flags
Flags ..> Orders
Flags ..> Billing
```

To choose an engine from the command line, use `drumlin render --layout <engine>` or
`drumlin check --layout <engine>`. The option wins over the file's `layout` statement. An unknown name
is an error that lists the engines. Sequence and activity diagrams have layouts of their own, so they
reject both the statement and the option.

An element may still be called `layout`: `layout --> parser` is a connector, and only `layout` followed
by a name is the statement. How the engines work is in
[How layout works](../internals/layout.md#layout-engines).

## Style

`style <name>` picks how the drawing is inked: `pens` (the default), `plain` or `lettering`. It applies
to every diagram kind, and `--style <name>` on the command line wins over it. [Styles](styles.md) shows
each one.

## Comments

`#` and `//` start a comment that runs to the end of the line.

## Names and quoting

A name without spaces needs no quotes. A bare name may contain letters, digits, `_`, and single `-` or
`.` between them. Any other name is written in double quotes, and can be given a short alias with `as`
for use in the rest of the file:

```text
component "Order Service" as orders
```
