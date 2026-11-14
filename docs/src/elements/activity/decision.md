# Decision and merge

A diamond. A decision has guards in square brackets on its outgoing flows; a merge joins flows again. `if` draws
both. `while` and `repeat` build a loop from a merge and a decision.

```drumlin
diagram activity "Decision and loop"
start
if "valid" {
  action "Accept"
} else "malformed" {
  action "Reject"
}
while "more input" {
  action "Read"
}
stop
```

An omitted guard after `else` is the predefined `[else]`. `switch "subject"` shows the subject as a «decisionInput»
note on the diamond.

## Syntax

```text
if "guard" { ... } [else if "guard" { ... }]... [else ["guard"] { ... }]
switch ["subject"] { case "guard" { ... } ... }
while "guard" { ... }
repeat { ... } until "guard"
```

## Specification

UML 2.5.1. Decision and merge: 15.3.4 (p. 391). «decisionInput»: Figure 15.33 (p. 390). Guards and `[else]`:
15.2.4 (p. 380).

## Used in

[Firmware boot](../../diagrams/activity/examples.md#firmware-boot),
[Connection retry](../../diagrams/activity/examples.md#connection-retry).

## See also

[Activity diagrams](../../diagrams/activity/overview.md) (how to write the whole diagram), [Elements of activity diagrams](overview.md), [Grammar](../../reference/grammar.md#activity-diagrams).
