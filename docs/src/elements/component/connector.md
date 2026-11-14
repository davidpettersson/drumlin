# Connector

A connector is a line between two elements. drumlin draws a solid line for a connector and a dashed
line for a dependency; both may end in an open arrow.

| Syntax | Meaning |
|---|---|
| `a --> b` | Solid line, open arrow at `b` |
| `a ..> b` | Dashed dependency, open arrow at `b` |
| `a -- b` | Solid line, no arrow |
| `a .. b` | Dashed line, no arrow |
| `a <-- b` | Same as `b --> a` |
| `a <.. b` | Same as `b ..> a` |

```drumlin
component A
component B
component C
component D
A --> B
A ..> C
B -- D
C .. D
```

## Labels

Text after `:` labels a connector.

```drumlin
component Producer
component Consumer
Producer --> Consumer : batches, 64 KiB max
```

## Direction and rank

A connector places its source before its target, along the diagram direction.
Cycles are allowed, and drumlin reverses one edge internally to rank them.

```drumlin
component Parser
component Lexer
component Preprocessor
Parser --> Lexer : next token
Lexer --> Preprocessor : include
Preprocessor --> Parser : macro expansion
```

## Connectors drawn from interface names

Interface matching makes connectors too, without any arrow in the source. Two simple ports form a
ball-and-socket assembly connector, a port with several interfaces gets a plain connector named after the
interface, and an end with no port keeps its socket and lollipop apart with a dashed arrow. See
[Interface](interface.md). A connector from a composite's port to a part inside is a
[delegation](port.md#delegation).

## Syntax

```text
<ref> (--> | ..> | -- | .. | <-- | <..) <ref> [: <label>]
```

A connector needs two different components. A `<<x>>` in a label is written with guillemets, `«x»`.

## Specification

UML 2.5.1. Connectors: 11.2.4 (p. 186). Dependency: 7.7.4 (p. 39). An arrow on a connector end is an
end adornment, as for an association (11.2.4, p. 186).

## Used in

[Cache-aside](../../diagrams/component/examples.md#cache-aside),
[Event-driven checkout](../../diagrams/component/examples.md#event-driven-checkout),
[Build system](../../diagrams/component/examples.md#build-system).

## See also

[Component diagrams](../../diagrams/component/overview.md) (how to write the whole diagram), [Elements of component diagrams](overview.md), [Grammar](../../reference/grammar.md#component-diagrams).
