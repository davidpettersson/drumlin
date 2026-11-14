# Activity diagrams

An activity diagram shows the flow of control and data through a behaviour, like a flowchart. This
page walks through writing one. Start with a `diagram activity` header, then describe the flow from top
to bottom in blocks, much as you would write code. The layout follows the structure you write, so
nothing needs positioning, and activity diagrams take no layout hints.

The notation follows UML 2.5.1 clause 15 and, for signals, clause 16. Each symbol has its own page under
[Elements](../../elements/activity/overview.md), the full syntax is in the
[grammar](../../reference/grammar.md#activity-diagrams), and there is a page of [examples](examples.md).

```text
diagram activity "Title"
```

## Actions and flow

`start` is the initial node, a filled circle. `action` is a rounded rectangle. Statements in
order are joined by flows with open arrowheads. `stop` is the activity final node, a bullseye, and `end`
is the flow final node, a circled X: `stop` ends the whole activity, `end` ends only its own
flow. Quote a name that has several words.

```drumlin
diagram activity "Actions in a flow"
start
action "Validate request"
action Handle
object "Order"
action "Store order"
stop
```

## Object nodes

`object` draws an object node, a plain rectangle, on the flow. Flows into and out of it are
object flows. Since an initial node only starts control flows, `object` cannot directly follow `start`.

## Decisions

`if "guard" { ... }` draws a decision and a merge. Guards go on the outgoing flows, in square
brackets, near the decision. `else` gives the other branch; the guard defaults to the predefined
`[else]`, and `else "guard"` changes it. Write `else if "guard" { ... }` for more branches on the
same decision node. An `if` with no `else` gets an empty `[else]` branch, so a decision always has a way on.

```drumlin
diagram activity "Decisions"
start
action "Parse header"
if "valid" {
  action "Route request"
} else "malformed" {
  action "Log error"
}
if "cached" {
  action "Serve from cache"
}
if "rejected" {
  action "Send 403"
  stop
}
action "Run handler"
if "2xx" {
  action "Cache"
} else if "4xx" {
  action "Count"
} else if "5xx" {
  action "Alert"
} else {
  action "Mystery"
}
stop
```

One branch always takes the main axis, so the flow passes straight through the decision and the merge
(see [Flow passes straight through a decision](../../principles/lines.md#flow-passes-straight-through-a-decision)).
When exactly one branch continues and the others end in `stop` or `end`, that branch takes the axis.
A body beside an empty branch sits on the axis, and the empty branch runs around it. Otherwise the
middle branch takes it: the first declared of two, so in `if ... else` the `if` body runs straight down
and the `else` body sits to its right. The other branches keep their declared order left to right and
rejoin the merge from its sides.

`switch "subject" { case "a" { ... } ... }` is a multi-way decision. The subject is shown as a
«decisionInput» note on the diamond, as Figure 15.33 draws it. The subject is optional.

```drumlin
diagram activity "Multi-way decision"
start
switch "HTTP method" {
  case "GET" {
    action "Read"
  }
  case "POST" {
    action "Create"
  }
  case "PUT" {
    action "Replace"
  }
  case "else" {
    action "405"
  }
}
stop
```

A branch that reaches `stop` or `end` does not rejoin the flow, and no merge is drawn for it. If only one
branch rejoins, no merge is needed at all.

## Loops

`while "guard" { ... }` tests before each pass and `repeat { ... } until "guard"` after it. Both
use a merge for the return and a decision for the test. The back edge runs along the left side,
and the loop's exit leaves on the right, so neither crosses the body.

```drumlin
diagram activity "Loops"
start
while "queue not empty" {
  action "Pop task"
  action "Run task"
}
repeat {
  action "Read block"
  action "Verify CRC"
} until "CRC ok"
stop
```

A `while` has its body on the `[guard]` flow and leaves on `[else]`. A `repeat` leaves on `[guard]` and returns
on `[else]`.

## Concurrency

`fork { ... } and { ... }` draws a fork bar, branches side by side, and a join bar. Every branch needs at
least one step. A branch ending in `end` or `stop` does not reach the join; if only one branch reaches
it, the join is left out, and the flow carries on from that branch.

```drumlin
diagram activity "Concurrent flows"
start
fork {
  action "Fetch profile"
} and {
  action "Fetch orders"
} and {
  action "Fetch feed"
}
action "Compose page"
fork {
  action "Audit"
  end
} and {
  action "Respond"
}
stop
```

## Signals

`send` draws a send signal action, a convex pentagon. `accept` draws an accept event action, a concave
pentagon. An `accept` needs no incoming flow, so it may begin an activity instead of `start`.

```drumlin
diagram activity "Signals"
start
action "Request payment"
send "PaymentRequested"
accept "PaymentConfirmed"
action "Ship goods"
stop
```

## Partitions

`lane "Name" { ... }` puts steps in a swimlane, drawn as a labelled band. Lanes appear left to right in the order
they are first named, so you can return to a lane by naming it again, and a `lane` block
inside a branch moves only that branch. Flows between lanes turn at right angles in the free row above their target.

```drumlin
diagram activity "Swimlanes"
start
lane "Customer" {
  action "Place order"
}
lane "Warehouse" {
  action "Pick items"
  if "in stock" {
    action "Ship"
  } else {
    lane "Customer" {
      action "Choose substitute"
    }
  }
}
stop
```

In a diagram that uses lanes, every action, signal and object must be inside a `lane` block. Control
nodes may stand outside one, and then belong to the lane the flow is in. A `lane` inside a `lane` is not a
sub-partition: it simply moves the steps inside it to that lane.

A decision stays in its own lane and its branches run to their lanes. A fork or join bar spans the lanes of
its branches, meaning it belongs to all of them (a node may be in several partitions). Branches that
interleave lanes are placed one above the other, so a flow crossing lanes never runs through a node.

## Notes

`note: text` attaches a note to the statement before it, with a dashed line. A note on a simple step sits beside it.
On a decision, `while`, `repeat` or `fork`, it sits above the opening node. A flow's first statement cannot have
a note, as there is nothing before it.

```drumlin
diagram activity "Notes"
start
action "Load config"
note: Falls back to defaults when the file is missing.
if "debug" {
  action "Enable traces"
}
note: Checked once at startup.
action "Run"
stop
```

## Rules from the specification

drumlin rejects flows that UML forbids and stops problems early:

- `start` must be first in the flow, because an initial node has no incoming edge.
- A flow cannot end on a merge, a join or a loop exit, because those need one outgoing edge. Add a step, or `stop` or `end`.
- Statements after `stop` or `end` are unreachable, and it is an error to write them.

`split` from other tools is not offered: use `fork` for concurrent flows, or `switch` for alternatives.
Common mistakes and their messages are in [Errors](../../reference/errors.md#activity-diagrams).

Next: the [activity examples](examples.md), or look up any symbol under
[Elements of activity diagrams](../../elements/activity/overview.md).
