# Containment

Membership is usually shown by nesting. When it is clearer to draw a line, `owner +-- member` puts a
circle-plus on the owner. Members in the next rank share one trunk.

```drumlin
diagram package "Containment"

package Types
class Shape
class Point
Types +-- Shape
Types +-- Point
```

## Syntax

```text
owner +-- member
member --+ owner
```

Use it for members declared outside the owner's block. A member drawn inside its owner needs no line, so
`+--` to it is an error.

## Specification

UML 2.5.1, 12.2.4 (p. 248).

## Used in

[Package diagrams: containment](../../diagrams/package/overview.md#containment).

## See also

[Package diagrams](../../diagrams/package/overview.md) (how to write the whole diagram), [Elements of package diagrams](overview.md), [Grammar](../../reference/grammar.md#package-diagrams).
