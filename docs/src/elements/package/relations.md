# Relations

Relations between packages and classifiers in a package diagram.

| Arrow | Relation | Drawn as |
|---|---|---|
| `a ..> b` | Dependency | Dashed line, open head |
| `a --> b` | Directed association | Solid line, open head |
| `a -- b` | Association | Solid line, no head |
| `a .. b` | Plain dashed line | Dashed line, no head |
| `a --\|> b` | Generalization | Solid line, hollow triangle |
| `a ..\|> b` | Realization | Dashed line, hollow triangle |

```drumlin
diagram package "Relations"
direction right

interface Printable
class Report
class Invoice
class Ledger
Invoice --|> Report
Report ..|> Printable
Report --> Ledger : writes
Ledger ..> Printable : uses
```

## Syntax

```text
a ..> b [: label]   a --> b   a -- b   a .. b   a --|> b   a ..|> b
```

Reversed forms such as `<..`, `<--`, `<|--` and `<|..` swap the two ends. A `<<x>>` in a label becomes `«x»`.

## Specification

UML 2.5.1. Dependency and Realization: 7.7.4 (p. 39). Association, open arrowhead for navigable end:
11.5.4 (p. 202). Generalization: 9.2.4 (p. 102).

## Used in

[Plugin system](../../diagrams/package/examples.md#plugin-system),
[Domain model](../../diagrams/package/examples.md#domain-model).

## See also

[Package diagrams](../../diagrams/package/overview.md) (how to write the whole diagram), [Elements of package diagrams](overview.md), [Grammar](../../reference/grammar.md#package-diagrams).
