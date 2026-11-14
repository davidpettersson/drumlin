# Package diagrams

A package diagram shows how a system is grouped into packages and how those packages depend on each
other. This page walks through writing one. Start a file with `diagram package`.

The notation follows UML 2.5.1 clause 12. Each symbol has its own page under
[Elements](../../elements/package/overview.md), the full syntax is in the
[grammar](../../reference/grammar.md#package-diagrams), and there is a page of [examples](examples.md).

```text
diagram package "Title"
direction down
```

The default direction is `down`, which suits layered dependencies. `direction right`
is also available. Both lines are optional except for `diagram package`, because a
file with no header is a component diagram.

## Packages

A package is drawn as a folder. When it shows its contents its name sits in the
tab, and when it does not its name sits in the body, as UML 2.5.1 specifies.

```drumlin
diagram package "Names in the tab, or in the body"

package Empty
package Types {
  class Integer
  class Time
}
```

Packages nest to any depth, and a `<<stereotype>>` becomes a keyword above the name.

```drumlin
diagram package "Nesting and keywords"

package "Platform" as pf <<model>> {
  package "Kernel" as kernel {
    class Scheduler
    class Timer
  }
  package Drivers
  Drivers ..> kernel : <<access>>
}
package Empty
Empty left-of pf
```

## Classifiers

`class`, `interface` and `enum` declare [classifiers](../../elements/package/classifier.md): simple
name boxes. An interface carries the
`«interface»` keyword and an enumeration the `«enumeration»` keyword. A declared
stereotype joins that keyword list, except `<<abstract>>`, which sets the name in italics.

```text
class Order
interface Repository <<abstract>>
enum Status
```

## Relations

A [relation](../../elements/package/relations.md) is a line between two packages or classifiers. The
arrow you type chooses its kind:

| Arrow | Meaning | Drawn as |
|---|---|---|
| `a ..> b` | Dependency | Dashed line, open head |
| `a --> b` | Directed association | Solid line, open head |
| `a -- b`, `a .. b` | Plain line | Solid or dashed, no head |
| `a --\|> b` | Generalization | Solid line, hollow triangle |
| `a ..\|> b` | Realization | Dashed line, hollow triangle |
| `a +-- b` | Containment | Solid line, circle-plus on `a` |

Reversed forms such as `<..`, `<--`, `<|--`, `<|..` and `--+` swap the two ends.
A `<<x>>` in a label becomes `«x»`, which is how `«import»`, `«access»` and `«merge»`
are written. Any other label text is kept as it is.

```drumlin
diagram package "Relations"

package App
package Lib
package Util
package Base

App ..> Lib : <<import>>
App ..> Util : <<access>>
Lib ..> Base : <<merge>>
Util ..> Base : uses
```

```drumlin
diagram package "Classifiers"
direction right

interface Shape <<abstract>>
class Circle
class Square
class Base
enum Kind

Circle ..|> Shape
Square ..|> Shape
Circle --|> Base
Base --> Kind : tags
```

## Containment

`owner +-- member` draws UML's containment connector, with the circle-plus on the
owner. Use it for members declared outside the owner's block. Members in the next
rank share one trunk.

```drumlin
diagram package "Containment"

package Types
class Shape
class Point
class Color
Types +-- Shape
Types +-- Point
Types +-- Color
```

```drumlin
diagram package "Containment, left to right"
direction right

package "api" as api
package "core" as core
class Router
class Handler
core +-- Router
core +-- Handler
api ..> core : <<import>>
```

A dependency may run between an element and the package drawn around it, for example
a nested package importing its parent. The line runs to the inside of the package
boundary. Containment with `+--` needs no line when the member is already drawn inside
the owner, so that is an error.

```drumlin
diagram package "Nested import"
package Core {
  package Scheduler
  Scheduler ..> Core : <<import>>
}
```

```drumlin,error
diagram package "Containment"
package Types { class Shape }
Types +-- Shape
```

## Notes and layout hints

[Notes](../../elements/note.md) and [layout hints](../../reference/hints.md) work as they do in
component diagrams. A note attaches to a package or a classifier and sits next to it, inside the same
container. `left-of`, `right-of`, `above`, `below` and `align` set relative order.

```drumlin
diagram package "Notes and hints"

package Core {
  package Parser
  package Checker
  package Emitter
  package Runtime
  package Stdlib
  Parser left-of Checker
}
class Tool
Tool ..> Core : <<import>>
note Tool: Built separately.
```

Siblings with no relations between them are folded into a compact block, unless a
hint names them.

```drumlin
diagram package "Unconnected members fold into rows"

package Kernel {
  class Scheduler
  class Timer
  class Mutex
  class Channel
  class Pipe
  class Signal
}
```

## Statements on one line

A semicolon separates statements, which suits small packages.

```text
package "Core" as core { class Foo; interface IBar; enum Mode; package inner { } }
```

Next: the [package examples](examples.md), or look up any symbol under
[Elements of package diagrams](../../elements/package/overview.md).
