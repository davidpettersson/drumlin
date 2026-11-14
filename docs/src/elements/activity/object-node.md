# Object node

A plain rectangle on the flow. Flows into and out of it are object flows.

```drumlin
diagram activity "Object on the flow"
start
action "Resolve dependencies"
object "Lockfile"
action "Compile"
stop
```

## Syntax

```text
object "name"
```

An `object` cannot directly follow `start`, as an initial node starts only control flows.

## Specification

UML 2.5.1, 15.4.4 (p. 399).

## Used in

[Build and release pipeline](../../diagrams/activity/examples.md#build-and-release-pipeline).

## See also

[Activity diagrams](../../diagrams/activity/overview.md) (how to write the whole diagram), [Elements of activity diagrams](overview.md), [Grammar](../../reference/grammar.md#activity-diagrams).
