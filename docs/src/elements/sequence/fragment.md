# Combined fragment

A frame around messages with the operator in a pentagon at its top left. Each operand has a `[guard]` above
its first event, and dashed lines divide operands.

```drumlin
diagram sequence "Alternatives"

participant Client
participant Server

alt "ok" {
  Client -> Server : get
} else "retry" {
  loop "(1,3)" {
    Client ->> Server : retry
  }
}
```

Operators: `alt` with `else`, `opt`, `loop`, `break`, `critical`, `neg`, and `par` with `and`.
`loop "(1,3)"` becomes `loop(1,3)`, which is the spec's textual form.

## Syntax

```text
alt ["guard"] { ... } [else ["guard"] { ... }]...
par ["guard"] { ... } [and ["guard"] { ... }]...
opt|loop|break|critical|neg ["guard"] { ... }
```

## Specification

UML 2.5.1. Operands, constraints and frames: 17.6.4.1, 17.6.4.2, 17.6.4.3 (p. 585). Loop:
17.6.4.9 (p. 586). Coregions are an optional shorthand; `par` covers them.

## Used in

[Raft log replication](../../diagrams/sequence/examples.md#raft-log-replication),
[RPC with retry and exponential backoff](../../diagrams/sequence/examples.md#rpc-with-retry-and-exponential-backoff),
[TLS 1.3 handshake](../../diagrams/sequence/examples.md#tls-13-handshake).

## See also

[Sequence diagrams](../../diagrams/sequence/overview.md) (how to write the whole diagram), [Elements of sequence diagrams](overview.md), [Grammar](../../reference/grammar.md#sequence-diagrams).
