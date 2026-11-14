# Component diagrams

A component diagram shows a system's parts, the interfaces they offer and need, and how
they are wired. This page walks through writing one, section by section. It is drumlin's default
kind: a file with no `diagram` line is a component diagram. Write the header anyway when you want a
title.

The notation follows UML 2.5.1 clauses 11.2, 11.3 and 11.6. Each symbol has its own page under
[Elements](../../elements/component/overview.md), the full syntax is in the
[grammar](../../reference/grammar.md#component-diagrams), and the [examples](examples.md) come from
real kinds of system.

```text
diagram component "Title"
direction right
```

The default direction is `right`, with `direction down` for layered pictures. See
[File basics](../../guide/file-basics.md#direction).

## Parts

Declare a [component](../../elements/component/component.md) by name. Quote a name with spaces,
and give it a short alias with `as`. A block nests components, and a `<<stereotype>>` shows as a
keyword. Group loosely related parts in a [package](../../elements/package/package.md), which is
a folder around them.

```drumlin
diagram component "Parts and grouping"

package "Platform" {
  component "Request Router" as router <<service>> {
    component Parser
    component Dispatcher
    Parser --> Dispatcher
  }
  component "Orders DB" as db <<database>>
  router --> db
}
```

## Connecting parts

A [connector](../../elements/component/connector.md) joins two elements. `-->` is a solid
connector with an open arrow and `..>` is a dashed dependency. Text after `:` becomes the label.

```drumlin
diagram component "Connectors"

component Producer
component Consumer
component Metrics
Producer --> Consumer : batches
Producer ..> Metrics : reports
```

## Interfaces

Inside a component, `provides` and `requires` declare
[interfaces](../../elements/component/interface.md). You name them and drumlin draws them. When one
component requires an interface and another provides one of the same name, drumlin wires them. What
it draws depends on where the two ends are declared, as UML requires:

| Ends | What is drawn |
|---|---|
| Two simple ports (one interface each) | Ball-and-socket: the ball seated in the socket |
| A port with several interfaces at either end | A plain connector named after the interface |
| Anything else (a component with no port at one end) | Socket and lollipop kept apart, joined by a dashed dependency arrow |

An interface with no partner stays a stub on the boundary: a lollipop if it is provided, a socket
if it is required.

```drumlin
diagram component "Matched by name, no ports"

component Client { requires IOrders }
component Server { provides IOrders, IHealth }
```

## Ports and delegation

A [port](../../elements/component/port.md) is a named point on a component's boundary, drawn as a
small square. A connector from a composite's port to a part inside it is a delegation: the port
forwards to the part. Two ports with one interface each, matched by name, form a ball-and-socket.

```drumlin
diagram component "Ports"

component Gateway {
  port uplink requires IEvents
}

component Bus as bus {
  port in provides IEvents
  component Queue
  in --> Queue
}
```

## Notes and layout hints

`note target: text` attaches a [note](../../elements/note.md). [Layout hints](../../reference/hints.md)
such as `left-of` and `align` adjust the arrangement and never set coordinates.

```drumlin
diagram component "Notes and hints"

component Cache
component Store
Cache --> Store : writes through
note Cache: Entries expire after 30 seconds.
```

Next: the [component examples](examples.md), or look up any symbol under
[Elements of component diagrams](../../elements/component/overview.md).
