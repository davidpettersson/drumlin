# Layout hints

drumlin places everything for you. When you want a different arrangement, a layout hint adjusts the
result. Hints describe relations, never coordinates, so the source stays readable. They work in
component, package and state diagrams; sequence and activity diagrams take their order from the file.
Most diagrams need no hints: add one only when the drawing needs help.

| Hint | Effect |
|---|---|
| `a left-of b`, `a right-of b` | Horizontal order |
| `a above b`, `a below b` | Vertical order |
| `align a b c` | Places elements in the same rank |

## Same rank

Connectors normally give each element its own rank. `align` overrides that and
places elements in one rank, which shortens a wide diagram.

```drumlin
diagram component "Without align"
component Gateway
component Auth
component Orders
Gateway --> Auth
Auth --> Orders
```

```drumlin
diagram component "With align"
component Gateway
component Auth
component Orders
Gateway --> Auth
Auth --> Orders
align Auth Orders
```

## Forcing order

With `direction right`, `above` and `below` order elements within a rank.

```drumlin
diagram component "Metrics above Logs"
component App
component Logs
component Metrics
App --> Logs
App --> Metrics
Metrics above Logs
```

Along the diagram direction, a hint forces a rank order even with no connector.

```drumlin
diagram component "Order without connectors"
component Boot
component Init
component Run
Boot left-of Init
Init left-of Run
```

## Hints across containers

A hint between elements in different containers applies to the containers that
hold them. Here `Reader left-of Query` places the `Ingest` package left of `Serve`.

```drumlin
package "Ingest" { component Reader }
package "Serve" { component Query }
package "Admin" { component Console }
Reader left-of Query
Console left-of Query
```

## See also

Hints win over the layout's own choices: see
[Honour hints over heuristics](../principles/flow.md#honour-hints-over-heuristics). Package and state
diagrams use the same hints as the examples above: see
[Package diagrams](../diagrams/package/overview.md#notes-and-layout-hints) and
[State diagrams](../diagrams/state/overview.md#notes-and-hints).
