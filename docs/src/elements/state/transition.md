# Transition

A solid line with an open head, labeled `trigger [guard] / effect`. Every part is optional. A self-transition is
a compact loop. A transition between a composite state and a vertex inside it is local, drawn to or from the
inside of the border.

```drumlin
diagram state "Transitions"
direction right
[*] --> Waiting
Waiting --> Armed : arm [ready] / log()
Armed --> Armed : tick / count()
Armed --> [*] : done
```

## Syntax

```text
a --> b [: trigger {, trigger} [guard] / effect]
b <-- a
```

A long label breaks before the effect, then wraps. Pseudostate transitions have limits on which parts they may
carry: see [Pseudostates](pseudostates.md) and the
[transition labels](../../diagrams/state/overview.md#transition-labels) section.

## Specification

UML 2.5.1. Notation and label syntax: 14.2.4.8 (p. 331). Local and external transitions: 14.2.4.9 (p. 334).

## Used in

[TCP connection](../../diagrams/state/examples.md#tcp-connection),
[Circuit breaker](../../diagrams/state/examples.md#circuit-breaker).

## See also

[State diagrams](../../diagrams/state/overview.md) (how to write the whole diagram), [Elements of state diagrams](overview.md), [Grammar](../../reference/grammar.md#state-diagrams).
