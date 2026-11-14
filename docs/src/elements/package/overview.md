# Elements of package diagrams

One page per symbol of a package diagram: what it looks like, its syntax and the UML 2.5.1 clause it follows. Use these pages to look something up; to write a whole diagram, start with [Package diagrams](../../diagrams/package/overview.md).

| Element | Syntax |
|---|---|
| [Package](package.md) | `package "Name" { ... }` |
| [Classifier](classifier.md) | `class`, `interface`, `enum` |
| [Relations](relations.md) | `a ..> b`, `a --> b`, `a --\|> b`, `a ..\|> b` |
| [Import and merge](import-and-merge.md) | `a ..> b : <<import>>` |
| [Containment](containment.md) | `a +-- b` |
| [Note](../note.md) | `note a: text` |

```drumlin
diagram package "All the package elements"
direction right

package App {
  class Handler
}
package Core {
  interface Store
  class SqlStore
  SqlStore ..|> Store
}
App ..> Core : <<import>>
note App: Only depends on Core.
```

See [Package diagrams](../../diagrams/package/overview.md) for how to write one, and the [grammar](../../reference/grammar.md#package-diagrams) for every statement.
