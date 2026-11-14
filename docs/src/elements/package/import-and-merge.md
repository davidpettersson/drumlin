# Import and merge

A dashed arrow with an open head and a keyword in guillemets. `«import»` makes another package's names
public here, `«access»` makes them private, and `«merge»` merges the target into the source. The arrow
points from the importing or receiving package to the imported or merged one.

```drumlin
diagram package "Import, access, merge"
direction right

package App
package Lib
package Util
package Base

App ..> Lib : <<import>>
App ..> Util : <<access>>
Lib ..> Base : <<merge>>
```

A nested package may import its parent, with the line ending on the inside of the boundary:

```drumlin
diagram package "Nested import"
package Core {
  package Scheduler
  Scheduler ..> Core : <<import>>
}
```

## Syntax

```text
a ..> b : <<import>>
a ..> b : <<access>>
a ..> b : <<merge>>
```

Any `<<x>>` in a relation label becomes `«x»`, so other keywords work the same way.

## Specification

UML 2.5.1. Import and access: 7.4.4.3 (p. 29). Merge: 12.2.4 (p. 248).

## Used in

[Package merge](../../diagrams/package/examples.md#package-merge),
[Library dependency graph](../../diagrams/package/examples.md#library-dependency-graph),
[Layered architecture](../../diagrams/package/examples.md#layered-architecture).

## See also

[Package diagrams](../../diagrams/package/overview.md) (how to write the whole diagram), [Elements of package diagrams](overview.md), [Grammar](../../reference/grammar.md#package-diagrams).
