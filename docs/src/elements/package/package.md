# Package

A package is a folder: a rectangle with a small tab. When it shows its contents its name sits in the
tab, and when it shows none its name sits in the body. In component diagrams it groups components; in
[package diagrams](../../diagrams/package/overview.md) it is the main symbol.

Inside a package diagram:

```drumlin
diagram package "Tab or body"
package Empty
package Types { class Point }
```

In a component diagram:

```drumlin
package "Domain" {
  component Orders
  component Customers
  Orders --> Customers
}
```

Connectors may cross package boundaries.

```drumlin
package "Application" as app {
  component Handlers
}

package "Domain" as domain {
  component Model
}

package "Adapters" as adapters {
  component SQL
  component Kafka
  align SQL Kafka
}

Handlers --> Model
SQL ..> Model : implements
Kafka ..> Model : implements
```

Packages can nest.

```drumlin
package "Platform" {
  package "Storage" {
    component Blob
    component Index
    Index --> Blob
  }
  package "Compute" {
    component Runner
  }
  Runner --> Index
}
```

A package may also sit inside a component, as one of its packaged elements.

## Syntax

```text
package <name> [as <alias>] [<<stereotype>>] [{ ... }]
```

## Specification

UML 2.5.1, 12.2.4 (p. 248). Packaged elements in a component: 11.6.3 (p. 210).

## Used in

[Layered architecture](../../diagrams/package/examples.md#layered-architecture),
[Monolithic kernel](../../diagrams/component/examples.md#monolithic-kernel),
[RTOS layers](../../diagrams/component/examples.md#rtos-layers).

## See also

[Package diagrams](../../diagrams/package/overview.md) (how to write the whole diagram), [Elements of package diagrams](overview.md), [Grammar](../../reference/grammar.md#package-diagrams).
