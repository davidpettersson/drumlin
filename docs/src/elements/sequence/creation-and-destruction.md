# Creation and destruction

A created lifeline's head is drawn at the moment of creation, and the creating message is dashed with an
open head. `destroy` ends a lifeline with an X.

```drumlin
diagram sequence "Worker lifetime"

participant Pool
Pool -> new "Worker" as w : spawn()
Pool ->> w : stop
destroy w
```

## Syntax

```text
<a> -> new <name> [as <alias>] [<<stereotype>>] [: label] [{ ... }]
create <name> [as <alias>] [<<stereotype>>] [: label] [{ ... }]    inside a block
destroy <a>
```

A destroyed lifeline cannot be used afterwards. A `destroy` in one `alt` branch leaves the lifeline alive in the
others.

## Specification

UML 2.5.1. Creation messages: 17.4.4 (p. 577). DestructionOccurrenceSpecification: 17.4.4.2 (p. 578).

## Used in

[Page fault in a demand-paged kernel](../../diagrams/sequence/examples.md#page-fault-in-a-demand-paged-kernel).

## See also

[Sequence diagrams](../../diagrams/sequence/overview.md) (how to write the whole diagram), [Elements of sequence diagrams](overview.md), [Grammar](../../reference/grammar.md#sequence-diagrams).
