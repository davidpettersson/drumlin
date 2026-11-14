# Message

A message is a horizontal arrow between lifelines, its kind shown by the line and head.

| Arrow | Message | Line and head |
|---|---|---|
| `a -> b` | synchronous call | solid, filled head |
| `a ->> b` | asynchronous | solid, open head |
| `a --> b` | reply | dashed, open head |
| `[-> a` | found (from outside the diagram) | filled circle at the start |
| `a ->]` | lost (leaves the diagram) | filled circle at the end |

```drumlin
diagram sequence "Message kinds"

participant Client
participant Server

[-> Client : wake
Client -> Server : call()
Server --> Client : reply
Client ->> Server : notify
Server ->] : lost
```

A self-message draws a compact loop on its lifeline.

## Syntax

```text
<a> (-> | ->> | -->) <b> [: label] [{ ... }]
[-> <a> [: label] [{ ... }]
<a> ->] [: label]
```

A label runs to the end of the line, apart from a comment or a space-separated `{` or `}`; quote it to
keep those as text.

## Specification

UML 2.5.1, 17.4.4.1 (p. 576): filled heads for synchronous, open for asynchronous and dashed lines for
reply. Found and lost messages end on a small black circle (17.4.4, p. 577).

## Used in

[TCP connection](../../diagrams/sequence/examples.md#tcp-connection-handshake-data-and-teardown),
[TLS 1.3 handshake](../../diagrams/sequence/examples.md#tls-13-handshake).

## See also

[Sequence diagrams](../../diagrams/sequence/overview.md) (how to write the whole diagram), [Elements of sequence diagrams](overview.md), [Grammar](../../reference/grammar.md#sequence-diagrams).
