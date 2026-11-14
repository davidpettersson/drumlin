# Lifeline

A lifeline is a box or a stick figure at the top, with a vertical dashed line below it. Declaration order
sets the left-to-right order, and a name must be declared before use.

```drumlin
diagram sequence "Heads"

actor Operator as op
participant "Controller" as ctl <<service>>
op -> ctl : mount()
```

## Syntax

```text
participant <name> [as <alias>] [<<stereotype>>]
actor <name> [as <alias>] [<<stereotype>>]
```

Heads stay at the top and are not repeated at the bottom.

## Specification

UML 2.5.1, 17.3.4.1 (p. 572). The head is a rectangle, or the classifier's shape; a stick figure for an actor.

## Used in

[Raft log replication](../../diagrams/sequence/examples.md#raft-log-replication),
[TCP connection](../../diagrams/sequence/examples.md#tcp-connection-handshake-data-and-teardown).

## See also

[Sequence diagrams](../../diagrams/sequence/overview.md) (how to write the whole diagram), [Elements of sequence diagrams](overview.md), [Grammar](../../reference/grammar.md#sequence-diagrams).
