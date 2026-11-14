# Execution

An execution is a thin white bar on a lifeline, showing where it is active. Write a block after a message
to open one. Bars nest and step to the right as calls go deeper.

```drumlin
diagram sequence "Nested executions"

participant App
participant Parser

App -> Parser : parse() {
  Parser -> Parser : next token
  return ast
}
```

`return` sends a reply from inside a block, to the block's caller, and ends the bar when it is
the last statement.

## Syntax

```text
<a> -> <b> : label { ... }
return [text]
```

A plain synchronous self-call also opens a short nested bar.

## Specification

UML 2.5.1, 17.2.4.4 (p. 569).

## Used in

[Interrupt handling with a nested IRQ](../../diagrams/sequence/examples.md#interrupt-handling-with-a-nested-irq),
[Raft log replication](../../diagrams/sequence/examples.md#raft-log-replication).

## See also

[Sequence diagrams](../../diagrams/sequence/overview.md) (how to write the whole diagram), [Elements of sequence diagrams](overview.md), [Grammar](../../reference/grammar.md#sequence-diagrams).
