# Elements of component diagrams

One page per symbol of a component diagram: what it looks like, its syntax and the UML 2.5.1 clause it follows. Use these pages to look something up; to write a whole diagram, start with [Component diagrams](../../diagrams/component/overview.md).

| Element | Syntax |
|---|---|
| [Component](component.md) | `component "Name" as a <<stereotype>> { ... }` |
| [Interface](interface.md) | `provides I`, `requires I` |
| [Port](port.md) | `port p left provides I` |
| [Connector](connector.md) | `a --> b : label` |
| [Package](../package/package.md) | `package "Name" { ... }` around components |
| [Note](../note.md) | `note a: text` |

```drumlin
diagram component "All the component elements"

component Client { requires IApi }

package "Backend" {
  component Service {
    port api provides IApi
    component Worker
    api --> Worker
  }
  component Database <<database>>
  Service ..> Database : queries
}
note Client: Retries on timeout.
```

See [Component diagrams](../../diagrams/component/overview.md) for how to write one, and the [grammar](../../reference/grammar.md#component-diagrams) for every statement.
