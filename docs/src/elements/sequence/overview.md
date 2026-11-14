# Elements of sequence diagrams

One page per symbol of a sequence diagram: what it looks like, its syntax and the UML 2.5.1 clause it follows. Use these pages to look something up; to write a whole diagram, start with [Sequence diagrams](../../diagrams/sequence/overview.md).

| Element | Syntax |
|---|---|
| [Lifeline](lifeline.md) | `participant`, `actor` |
| [Message](message.md) | `a -> b`, `a ->> b`, `a --> b`, `[-> a`, `a ->]` |
| [Execution](execution.md) | `a -> b : call { ... }`, `return` |
| [Creation and destruction](creation-and-destruction.md) | `a -> new "B" as b`, `create`, `destroy` |
| [Combined fragment](fragment.md) | `alt`, `opt`, `loop`, `break`, `critical`, `neg`, `par` |
| [Interaction use](interaction-use.md) | `ref over a, b : Name` |
| [Note](../note.md) | `note over a: text` |

```drumlin
diagram sequence "All the sequence elements"

actor User as u
participant Server as s

u -> s : request() {
  opt "cached" {
    return hit
  }
  s -> new "Worker" as w : start
  destroy w
  return result
}
note over s: Stateless.
```

See [Sequence diagrams](../../diagrams/sequence/overview.md) for how to write one, and the [grammar](../../reference/grammar.md#sequence-diagrams) for every statement.
