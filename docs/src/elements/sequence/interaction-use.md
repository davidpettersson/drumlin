# Interaction use

A frame with a `ref` tab that stands for another interaction, spanning the lifelines it involves.

```drumlin
diagram sequence "Boot"

participant Bootloader as bl
participant Kernel as k

ref over bl, k : Verify signature
bl -> k : jump(entry)
```

## Syntax

```text
ref over <a>[, <b>]... : <name>
```

## Specification

UML 2.5.1, 17.7.4.1 (p. 591).

## Used in

[Sequence diagrams: references and notes](../../diagrams/sequence/overview.md#interaction-references-and-notes).

## See also

[Sequence diagrams](../../diagrams/sequence/overview.md) (how to write the whole diagram), [Elements of sequence diagrams](overview.md), [Grammar](../../reference/grammar.md#sequence-diagrams).
