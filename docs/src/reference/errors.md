# Errors

This page lists the errors you are most likely to meet, grouped by cause, with the source that causes
each. Search the page for the words of your message.

Errors have the form `file:line:col: error: message`, and drumlin reports only the first error in a file.
The messages below are real: drumlin produced them while this book was built, without the file prefix.
For the command-line errors, see [Command line](cli.md#exit-status-and-errors).

## Unknown names

An element must be declared somewhere in the file. drumlin suggests the closest declared name.

```drumlin,error
component Scheduler
component Worker
Scheduelr --> Worker
```

## Duplicate declarations

A name or alias may be declared once.

```drumlin,error
component Worker
component Worker
```

## Unclosed blocks

```drumlin,error
component Outer {
  component Inner
```

## Unmatched braces

```drumlin,error
component Lonely
}
```

## Invalid nesting

A member drawn inside its owner needs no containment line: see
[Containment](../elements/package/containment.md).

```drumlin,error
diagram package
package Outer { class Inner }
Outer +-- Inner
```

## Misplaced statements

`provides`, `requires` and `port` belong inside a component: see
[Interface](../elements/component/interface.md).

```drumlin,error
provides IFoo
```

## Ambiguous ports

A port name used by two components must be written as `<component>.<port>`: see
[Port](../elements/component/port.md#delegation).

```drumlin,error
component A { port data }
component B { port data }
component C
data --> C
```

## Unimplemented diagram kinds

drumlin draws five kinds: see [File basics](../guide/file-basics.md).

```drumlin,error
diagram timing "Handshake"
```

## Layout engines

An unknown engine name lists the engines, or suggests a near one. See
[Layout engine](../guide/file-basics.md#layout-engine).

```drumlin,error
diagram state
layout dot
[*] --> Idle
```

Sequence and activity diagrams have their own layouts.

```drumlin,error
diagram sequence
layout longest-path
participant Client
```

`drumlin render --layout <engine>` reports an unknown name as `error: unknown layout engine ...`, without a
file or position, and a sequence or activity file at its `diagram` header.

## Styles

An unknown style name lists the styles, or suggests a near one. Every kind takes a style; see
[Styles](../guide/styles.md).

```drumlin,error
diagram sequence
style sketch
participant Client
```

`drumlin render --style <name>` reports an unknown name as `error: unknown style ...`, without a file or
position, for example `error: unknown style `plan`; did you mean `plain`?`.

## Sequence diagrams

See [Sequence diagrams](../diagrams/sequence/overview.md) for the syntax. Names must be declared before
use, so a typo is caught where it happens.

```drumlin,error
diagram sequence
participant Server
Servre -> Server : ping
```

A `return` needs a message block to return from.

```drumlin,error
diagram sequence
participant A
return done
```

A fragment continuation must match its opener.

```drumlin,error
diagram sequence
participant A
participant B
opt "x" {
  A -> B : y
} else {
  B -> A : z
}
```

## State diagrams

See [State diagrams](../diagrams/state/overview.md#transition-labels) and
[Pseudostates](../elements/state/pseudostates.md) for the rules. Rules from UML 2.5.1 clause 14 are
checked. A fork, for example, needs one incoming and at
least two outgoing transitions.

```drumlin,error
diagram state
fork f
A --> f
f --> B
```

A name used only once, close to another name, is reported as a likely typo. Write
`state Idel` first if the state really has that name.

```drumlin,error
diagram state
state Idle
[*] --> Idle
Idle --> Idel : wait
```

A transition leaving a pseudostate cannot have a trigger.

```drumlin,error
diagram state
choice c
A --> c
c --> B : go
```

A transition cannot join two orthogonal regions of one state.

```drumlin,error
diagram state
state A {
  region {
    [*] --> B
  }
  region {
    [*] --> C
    C --> B
  }
}
```

## Activity diagrams

See [Rules from the specification](../diagrams/activity/overview.md#rules-from-the-specification).
Statements after the end of a flow can never run.

```drumlin,error
diagram activity
start
stop
action "Late"
```

Every branch of a `fork` needs a step.

```drumlin,error
diagram activity
start
fork {
  action "A"
} and {
}
stop
```

A flow must not end on a merge, because a merge has one outgoing flow.

```drumlin,error
diagram activity
start
if "x" {
  action "A"
}
```

When a diagram uses lanes, every action must be in one.

```drumlin,error
diagram activity
lane "A" {
}
start
action "B"
stop
```
