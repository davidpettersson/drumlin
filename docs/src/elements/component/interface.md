# Interface

A provided interface is a lollipop, a small circle on a stem. A required interface is a socket, a
half-circle on a stem. Interfaces are declared inside a component with `provides` and `requires`,
and drumlin decides how to draw them.

## Assembly

A required and a provided interface with the same name are wired together. With no ports, the socket
and the lollipop are joined by a dashed dependency arrow:

```drumlin
component Consumer { requires IQueue }
component Broker { provides IQueue }
```

Between two simple ports (ports with one interface each), the ball sits in the socket:

```drumlin
component Consumer { port in requires IQueue }
component Broker { port out provides IQueue }
```

Which notation drumlin uses depends on where the interfaces are declared, following UML 2.5.1, clause
11.2.4. The ball-and-socket is reserved for simple ports. Anything else keeps the socket and the
lollipop apart: when an end has no port, they are joined by a dashed dependency arrow from socket to
lollipop, as in Figure 11.46. A port with more than one interface is joined with a plain connector named
after the interface.

## Unconnected interfaces

An interface with no partner is drawn as a stub at the component boundary. A
provided interface has a lollipop and a required interface has a socket.

```drumlin
component Codec {
  provides IEncode
  provides IDecode
  requires IAllocator
}
```

## Several interfaces

Separate names with commas.

```drumlin
component Runtime { provides ITimer, IThreads, IFiles }
component App { requires ITimer, IThreads, IFiles }
```

## Several consumers

Each consumer gets its own connection to the provider.

```drumlin
component Logger { provides ILog }
component Net { requires ILog }
component Disk { requires ILog }
component UI { requires ILog }
```

## Inside composites

Matching also works inside a composite and across nesting levels.

```drumlin
component App {
  component Controller { requires IRepo }
  component Repo { provides IRepo }
}
```

## Syntax

```text
provides <interface> [, <interface>]...
requires <interface> [, <interface>]...
```

Both go inside a component. On a port, use `port p provides I` or `port p requires I`. See
[Port](port.md). In a package diagram an interface is a boxed classifier instead:
see [Classifier](../package/classifier.md).

## Specification

UML 2.5.1, 10.4.4 (p. 172): lollipop and socket; a dependency arrow from a usage to a realization may join
socket to lollipop. Components: 11.6.4 (p. 210) and Figure 11.46 (p. 214). Ball-and-socket: 11.2.4
(p. 186), which permits it only for simple ports and forbids it for complex ports and parts without ports.

## Used in

[Compiler pipeline](../../diagrams/component/examples.md#compiler-pipeline),
[Debugger](../../diagrams/component/examples.md#debugger),
[API gateway](../../diagrams/component/examples.md#api-gateway),
[Container runtime](../../diagrams/component/examples.md#container-runtime).

## See also

[Component diagrams](../../diagrams/component/overview.md) (how to write the whole diagram), [Elements of component diagrams](overview.md), [Grammar](../../reference/grammar.md#component-diagrams).
