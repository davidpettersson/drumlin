# Signals

`send` draws a send signal action, a convex pentagon. `accept` draws an accept event action, a concave pentagon. An
`accept` needs no incoming flow, so it may begin an activity instead of `start`.

```drumlin
diagram activity "Signals"
start
action "Request payment"
send "PaymentRequested"
accept "PaymentConfirmed"
action "Ship goods"
stop
```

## Syntax

```text
send "name"
accept "name"
```

## Specification

UML 2.5.1. Send signal: 16.3.4.3 (p. 455). Accept event: 16.10.4 (p. 474).

## Used in

[Interrupt service with deferred work](../../diagrams/activity/examples.md#interrupt-service-with-deferred-work),
[Deployment with parallel checks](../../diagrams/activity/examples.md#deployment-with-parallel-checks-across-lanes).

## See also

[Activity diagrams](../../diagrams/activity/overview.md) (how to write the whole diagram), [Elements of activity diagrams](overview.md), [Grammar](../../reference/grammar.md#activity-diagrams).
