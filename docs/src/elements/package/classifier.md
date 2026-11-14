# Classifier

`class`, `interface` and `enum` declare a named box. An interface carries the keyword
`«interface»`, and an enumeration `«enumeration»`. Other stereotypes join the keywords; `<<abstract>>`
sets the name in italics instead.

```drumlin
diagram package "Classifiers"
direction right

class Order
interface Repository
enum Status
class Shape <<abstract>>
```

Looking for lollipops and sockets? Those are the [interfaces of component diagrams](../component/interface.md).

## Syntax

```text
class|interface|enum <name> [as <alias>] [<<stereotype>>]
```

Classifiers cannot have bodies, so attributes and operations are not listed.

## Specification

UML 2.5.1. Classifier notation and italics for abstract classifiers: 9.2.4 (p. 101). «interface» keyword:
10.4.4 (p. 172). «enumeration» keyword: 10.2.4 (p. 168).

## Used in

[Domain model](../../diagrams/package/examples.md#domain-model),
[Plugin system](../../diagrams/package/examples.md#plugin-system).

## See also

[Package diagrams](../../diagrams/package/overview.md) (how to write the whole diagram), [Elements of package diagrams](overview.md), [Grammar](../../reference/grammar.md#package-diagrams).
