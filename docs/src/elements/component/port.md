# Port

A port is a small square on a component's boundary, with its name beside it. It is a named
interaction point.

```drumlin
component Modem {
  port serial
  port radio
}
```

## Ports with interfaces

A port usually provides or requires one interface, which makes it a *simple* port. Two simple ports
matched by name form a ball-and-socket. A port listing more than one interface is complex: its
connector is drawn plain and named after the interface. See [Interface](interface.md#assembly).

```drumlin
component Radio { port rx provides IPackets }
component Stack { port input requires IPackets }
```

An unconnected port interface draws as a stub on the port.

```drumlin
component Controller {
  port ctl provides IControl
  port bus requires ICan
}
```

## Choosing a side

drumlin chooses a side from a port's connections. To override it, write `left`,
`right`, `top` or `bottom`.

```drumlin
component Board {
  port power top
  port ground bottom
  port uart left
  port spi right
}
```

## Delegation

A connector from a composite's port to an inner part delegates the interaction, and is drawn solid.
Refer to a port by its bare name when it is unique, or as `component.port`.

```drumlin
component Client { requires IApi }

component Service as svc {
  port api provides IApi
  port store requires IStore

  component Router
  component Worker

  api --> Router
  Router --> Worker
  Worker --> store
}

component Database { provides IStore }
```

## Syntax

```text
port <name> [left | right | top | bottom] [provides <i> | requires <i>]...
```

## Specification

UML 2.5.1, 11.3.4 (p. 191): a small square with the name near it, placed on or inside the boundary.
Delegation connector: 11.3.3 (p. 191) and Figure 11.47 (p. 214). Port type labels and multiple interfaces
on one lollipop are optional UML notation that drumlin does not draw.

## Used in

[Hexagonal service](../../diagrams/component/examples.md#hexagonal-service),
[Vehicle gateway](../../diagrams/component/examples.md#vehicle-gateway),
[Storage engine](../../diagrams/component/examples.md#storage-engine).

## See also

[Component diagrams](../../diagrams/component/overview.md) (how to write the whole diagram), [Elements of component diagrams](overview.md), [Grammar](../../reference/grammar.md#component-diagrams).
