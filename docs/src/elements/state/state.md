# State

A state is a rectangle with rounded corners. Internal behaviours go in a compartment under the name. A
composite state holds other states.

```drumlin
diagram state "Simple and composite"
[*] --> Idle
Idle --> Running : start
state Running {
  entry / open()
  do / work()
  [*] --> Step
  Step --> [*]
}
Running --> [*]
```

## Syntax

```text
state <name> [as <alias>] [<<stereotype>>] [{ ... }]
entry|do|exit [/ expression]        inside a state, one per line
trigger {, trigger} [guard] / effect    internal transition, one per line
```

A name used in a transition declares a state. See
[State diagrams](../../diagrams/state/overview.md).

## Specification

UML 2.5.1, 14.2.4.4 (p. 319). The final state is 14.2.4.5 (p. 327).

## Used in

[USB device](../../diagrams/state/examples.md#usb-device),
[Process lifecycle](../../diagrams/state/examples.md#process-lifecycle).

## See also

[State diagrams](../../diagrams/state/overview.md) (how to write the whole diagram), [Elements of state diagrams](overview.md), [Grammar](../../reference/grammar.md#state-diagrams).
