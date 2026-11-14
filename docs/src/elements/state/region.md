# Region

A region is one orthogonal part of a composite state, drawn as a tile with dashed separators. Regions lie across
the flow: side by side when flowing down, stacked when flowing right.

```drumlin
diagram state "Regions"
state Active {
  region "input" {
    [*] --> Reading
    Reading --> [*]
  }
  region "output" {
    [*] --> Writing
    Writing --> [*]
  }
}
```

## Syntax

```text
region ["name"] { ... }        inside a state
```

A transition cannot join two regions of one state.

## Specification

UML 2.5.1, 14.2.4.3 (p. 319).

## Used in

[Elevator controller](../../diagrams/state/examples.md#elevator-controller).

## See also

[State diagrams](../../diagrams/state/overview.md) (how to write the whole diagram), [Elements of state diagrams](overview.md), [Grammar](../../reference/grammar.md#state-diagrams).
