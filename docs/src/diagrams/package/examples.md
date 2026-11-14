# Package diagram examples

Five package diagrams of common software structures. For the notation, see
[Package diagrams](overview.md) and the [package elements](../../elements/package/overview.md).

## Layered architecture

A note marks the rule for the domain layer: it depends on nothing else.

```drumlin
diagram package "Layered architecture"

package "Presentation" as ui {
  package Web
  package Mobile
}
package "Application" as app {
  package Commands
  package Queries
}
package "Domain" as domain {
  class Order
  class Customer
  interface Repository
  enum Status
  Order --> Customer
  Order ..> Status
}
package "Infrastructure" as infra {
  package Persistence
  package Messaging
}

ui ..> app : <<import>>
app ..> domain : <<import>>
infra ..> domain : <<access>>
note domain: Depends on nothing else. Keep it free of framework types.
```

## Library dependency graph

Left-to-right flow fits a dependency graph with labelled imports.

```drumlin
diagram package "Library dependency graph"
direction right

package app
package "http-client" as http
package "json" as json
package "tls" as tls
package "logging" as log
package "platform" as plat

app ..> http : <<import>>
app ..> json : <<import>>
app ..> log : <<import>>
http ..> tls : <<access>>
http ..> log
json ..> plat
tls ..> plat
log ..> plat
```

## Package merge

Dashed arrows with the `«merge»` keyword point from the receiving package to the merged one.

```drumlin
diagram package "Package merge"

package P { class A; class B }
package Q { class C }
package R { class D }
package S
package T

R ..> P : <<merge>>
R ..> Q : <<merge>>
S ..> Q : <<merge>>
T ..> R : <<merge>>
T ..> S : <<merge>>
```

## Plugin system

Realizations cross package boundaries. `align` keeps the interface in reach of the plugins.

```drumlin
diagram package "Plugin system"

package Host {
  class Registry
  interface Plugin
  Registry --> Plugin : loads
  align Registry Plugin
}
package "plugins" as plugins <<framework>> {
  package Markdown { class MarkdownPlugin }
  package Search { class SearchPlugin }
}
MarkdownPlugin ..|> Plugin
SearchPlugin ..|> Plugin
note plugins: Discovered at startup from the plugin directory.
```

## Domain model

Classifiers relate across packages, and the packages show their dependency too.

```drumlin
diagram package "Domain model"

package Ordering {
  class Order
  class LineItem
  enum OrderState
  interface Pricing
  Order --> LineItem : contains
  Order ..> OrderState
  Order ..> Pricing
}
package Catalog {
  class Product
  class Category
  Product --> Category
}
LineItem --> Product
Ordering ..> Catalog : <<import>>
```
