# Pseudostates

| Pseudostate | Syntax | Symbol |
|---|---|---|
| Initial | `[*] --> x` | Filled circle |
| Final state | `x --> [*]` | Bull's-eye |
| Choice | `choice c` | Diamond |
| Junction | `junction j` | Small filled circle |
| Fork | `fork f` | Solid bar, one in and at least two out |
| Join | `join j` | Solid bar, at least two in and one out |
| Terminate | `terminate t` | Cross |
| Shallow history | `[H]`, `Name[H]` | Circle with H |
| Deep history | `[H*]`, `Name[H*]` | Circle with H* |

```drumlin
diagram state "Choice, fork, join"
[*] --> Ready
Ready --> c : request
choice c
c --> f : [ok]
c --> [*] : [denied]
fork f
f --> A
f --> B
A --> j
B --> j
join j
j --> Done
Done --> [*]
```

The final state is not strictly a pseudostate but is listed here because it is drawn with them.

## Syntax

```text
choice|junction|fork|join|terminate <name>
[*] --> a    a --> [*]    [H]    [H*]    Name[H]    Name[H*]
```

## Specification

UML 2.5.1, 14.2.4.6 (p. 327). FinalState: 14.2.4.5 (p. 327). Multiplicities of fork, join, choice and
junction, and the forbidden triggers on pseudostate transitions, are constraints of clause 14.5, which
drumlin checks and reports as errors.

## Used in

[OTA firmware update](../../diagrams/state/examples.md#ota-firmware-update),
[State diagrams: pseudostates](../../diagrams/state/overview.md#pseudostates).

## See also

[State diagrams](../../diagrams/state/overview.md) (how to write the whole diagram), [Elements of state diagrams](overview.md), [Grammar](../../reference/grammar.md#state-diagrams).
