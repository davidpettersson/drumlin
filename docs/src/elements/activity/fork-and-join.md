# Fork and join

A solid bar. A fork splits one flow into concurrent flows, and a join waits for them all.

```drumlin
diagram activity "Fork and join"
start
fork {
  action "Fetch profile"
} and {
  action "Fetch orders"
}
action "Compose page"
stop
```

A branch ending in `end` or `stop` does not reach the join. When only one branch reaches it, the join is
left out. In lanes, a bar spans the lanes of its branches.

## Syntax

```text
fork { ... } and { ... } [and { ... }]...
```

Each branch needs at least one step.

## Specification

UML 2.5.1, 15.3.4 (p. 391).

## Used in

[Build and release pipeline](../../diagrams/activity/examples.md#build-and-release-pipeline),
[Deployment with parallel checks](../../diagrams/activity/examples.md#deployment-with-parallel-checks-across-lanes).

## See also

[Activity diagrams](../../diagrams/activity/overview.md) (how to write the whole diagram), [Elements of activity diagrams](overview.md), [Grammar](../../reference/grammar.md#activity-diagrams).
