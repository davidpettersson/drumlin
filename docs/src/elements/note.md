# Note

A note is a rectangle with its upper right corner folded, joined to its target by a dashed line.
It attaches free text to an element, and drumlin wraps the text. Notes work in every diagram kind;
this page shows the component form first, then the forms of the other kinds.

```drumlin
component Cache
note Cache: Write-through. Entries expire after 30 seconds.
```

A note sits next to its target, inside the same container.

```drumlin
package "Storage" {
  component Journal
  component Pages
  Journal --> Pages
  note Journal: fsync before every commit record.
}
```

Attach several notes where they help.

```drumlin
component Reader
component Writer
Reader --> Writer : wakes
note Reader: Lock-free. May retry.
note Writer: Single owner only.
```

## Other kinds

Notes work in every diagram kind, with the syntax of that kind:

```drumlin
diagram state "A note on a state"
[*] --> Idle
Idle --> [*]
note Idle: Waits here between jobs.
```

In a sequence diagram, write `note over a: text`, `note left of a: text` or `note right of a: text`.
There the line is suppressed, as the spec allows when the context is clear. In an activity diagram,
write `note: text` after a statement to attach it.

## Syntax

```text
note <target>: text        component, package and state diagrams
note over <a>[, <b>]... : text        sequence diagrams
note left of <a>: text
note right of <a>: text
note: text                 activity diagrams, on the statement before it
```

## Specification

UML 2.5.1, 7.2.4 (p. 22): a Comment is a rectangle with the upper right corner bent, connected by a
dashed line, which may be suppressed if the context is clear.

## Used in

[Storage engine](../diagrams/component/examples.md#storage-engine),
[Layered architecture](../diagrams/package/examples.md#layered-architecture),
[Process lifecycle](../diagrams/state/examples.md#process-lifecycle),
[TCP connection](../diagrams/sequence/examples.md#tcp-connection-handshake-data-and-teardown),
[Build and release pipeline](../diagrams/activity/examples.md#build-and-release-pipeline).

## See also

[Grammar](../reference/grammar.md), where each kind lists its note statement.
