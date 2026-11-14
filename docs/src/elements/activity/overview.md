# Elements of activity diagrams

One page per symbol of a activity diagram: what it looks like, its syntax and the UML 2.5.1 clause it follows. Use these pages to look something up; to write a whole diagram, start with [Activity diagrams](../../diagrams/activity/overview.md).

| Element | Syntax |
|---|---|
| [Action](action.md) | `action "Name"` |
| [Initial and final nodes](initial-and-final.md) | `start`, `stop`, `end` |
| [Decision and merge](decision.md) | `if`, `switch`, `while`, `repeat` |
| [Fork and join](fork-and-join.md) | `fork { ... } and { ... }` |
| [Object node](object-node.md) | `object "Name"` |
| [Signals](signals.md) | `send "Name"`, `accept "Name"` |
| [Partition](partition.md) | `lane "Name" { ... }` |
| [Note](../note.md) | `note: text` |

```drumlin
diagram activity "All the activity elements"
start
action "Receive"
if "valid" {
  action "Process"
} else {
  action "Reject"
}
stop
```

See [Activity diagrams](../../diagrams/activity/overview.md) for how to write one, and the [grammar](../../reference/grammar.md#activity-diagrams) for every statement.
