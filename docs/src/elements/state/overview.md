# Elements of state diagrams

One page per symbol of a state diagram: what it looks like, its syntax and the UML 2.5.1 clause it follows. Use these pages to look something up; to write a whole diagram, start with [State diagrams](../../diagrams/state/overview.md).

| Element | Syntax |
|---|---|
| [State](state.md) | `state Name { ... }`, `entry /`, `do /`, `exit /` |
| [Region](region.md) | `region "name" { ... }` |
| [Pseudostates](pseudostates.md) | `[*]`, `choice`, `junction`, `fork`, `join`, `terminate`, `[H]`, `[H*]` |
| [Entry and exit points](entry-and-exit-points.md) | `entrypoint e`, `exitpoint x` |
| [Transition](transition.md) | `a --> b : trigger [guard] / effect` |
| [Note](../note.md) | `note a: text` |

```drumlin
diagram state "All the state elements"
[*] --> Idle
Idle --> Busy : start [ready] / log()
state Busy {
  entry / open()
  [*] --> Work
  Work --> [*]
}
Busy --> [*]
```

See [State diagrams](../../diagrams/state/overview.md) for how to write one, and the [grammar](../../reference/grammar.md#state-diagrams) for every statement.
