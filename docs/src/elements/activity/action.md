# Action

A rectangle with rounded corners. Actions in order are joined by flows with open arrowheads.

```drumlin
diagram activity "Actions"
start
action "Validate request"
action Handle
stop
```

## Syntax

```text
action "name"
```

A name that is a single word needs no quotes.

## Specification

UML 2.5.1. Executable nodes: 15.5.4.1, Figure 15.61 (p. 404). Actions: 16.2.4 (p. 446). Flows: 15.2.4 (p. 379).

## Used in

[Build and release pipeline](../../diagrams/activity/examples.md#build-and-release-pipeline),
[Connection retry](../../diagrams/activity/examples.md#connection-retry).

## See also

[Activity diagrams](../../diagrams/activity/overview.md) (how to write the whole diagram), [Elements of activity diagrams](overview.md), [Grammar](../../reference/grammar.md#activity-diagrams).
