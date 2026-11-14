# Initial and final nodes

`start` is the initial node, a filled circle. `stop` is the activity final, a bull's-eye, ending the whole activity.
`end` is the flow final, a circled X, ending only its own flow.

```drumlin
diagram activity "Control nodes"
start
fork {
  action "Audit"
  end
} and {
  action "Respond"
}
stop
```

## Syntax

```text
start
stop
end
```

`start` must be first in the flow. Statements after `stop` or `end` are unreachable and an error.

## Specification

UML 2.5.1, 15.3.4 (p. 391).

## Used in

[Firmware boot](../../diagrams/activity/examples.md#firmware-boot),
[Interrupt service with deferred work](../../diagrams/activity/examples.md#interrupt-service-with-deferred-work).

## See also

[Activity diagrams](../../diagrams/activity/overview.md) (how to write the whole diagram), [Elements of activity diagrams](overview.md), [Grammar](../../reference/grammar.md#activity-diagrams).
