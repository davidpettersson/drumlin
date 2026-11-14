# Entry and exit points

A small circle on a state's border for an entry point, and a crossed circle for an exit point, each with its
name beside it. Outside transitions attach on the outer side, inside ones on the inner side.

```drumlin
diagram state "Entry and exit points"
[*] --> Idle
Idle --> again : retry
state Login {
  entrypoint again
  exitpoint aborted
  again --> Prompt
  Prompt --> aborted : [locked out]
}
aborted --> Idle
```

## Syntax

```text
entrypoint <name>        inside a state
exitpoint <name>         inside a state
```

## Specification

UML 2.5.1, 14.2.4.6 (p. 328).

## Used in

[State diagrams: entry and exit points](../../diagrams/state/overview.md#entry-and-exit-points).

## See also

[State diagrams](../../diagrams/state/overview.md) (how to write the whole diagram), [Elements of state diagrams](overview.md), [Grammar](../../reference/grammar.md#state-diagrams).
