# Partition

A swimlane band with its name at the top. Lanes appear in the order they are first named.

```drumlin
diagram activity "Swimlanes"
start
lane "Customer" {
  action "Place order"
}
lane "Warehouse" {
  action "Pick items"
}
stop
```

A `lane` inside a `lane` is not a sub-partition: it simply moves the steps inside it to that lane. In a diagram with lanes,
actions, signals and objects must be inside a `lane`.

## Syntax

```text
lane "name" { ... }
```

## Specification

UML 2.5.1, 15.6.4 (p. 408). A node may belong to several partitions, which is how a fork bar spans lanes.

## Used in

[Deployment with parallel checks](../../diagrams/activity/examples.md#deployment-with-parallel-checks-across-lanes).

## See also

[Activity diagrams](../../diagrams/activity/overview.md) (how to write the whole diagram), [Elements of activity diagrams](overview.md), [Grammar](../../reference/grammar.md#activity-diagrams).
