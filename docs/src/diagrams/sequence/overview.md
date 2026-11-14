# Sequence diagrams

A sequence diagram shows the messages exchanged between participants over time, with time running
down the page. This page walks through writing one. Start a file with `diagram sequence`; the optional
title follows it.

The notation follows UML 2.5.1 clause 17. Each symbol has its own page under
[Elements](../../elements/sequence/overview.md), the full syntax is in the
[grammar](../../reference/grammar.md#sequence-diagrams), and there is a page of
[examples](examples.md).

```drumlin
diagram sequence "Ordering a pizza"

actor "Customer" as c
participant "Web app" as web
participant "Kitchen" as k

c -> web : place(order) {
  web ->> k : cook(order)
  return confirmation
}
```

## Participants

`participant` declares a [lifeline](../../elements/sequence/lifeline.md) drawn as a box, and `actor`
declares one drawn as a stick figure. A participant has a name, optionally an alias with `as`, and optionally a
`<<stereotype>>`. Participants become columns in declaration order, so the file controls
the left-to-right reading order, and a name must be declared before it is used.

```drumlin
diagram sequence "Heads"

actor "Operator" as op
participant "Controller" as ctl <<service>>
participant Disk

op -> ctl : mount()
ctl ->> Disk : spin up
```

## Messages

The arrow you type selects the kind of [message](../../elements/sequence/message.md), following UML
2.5.1 clause 17.4.4.

| Arrow | Message | Line and head |
|---|---|---|
| `a -> b` | synchronous call | solid, filled head |
| `a ->> b` | asynchronous message | solid, open head |
| `a --> b` | reply | dashed, open head |

A message may carry a label after `:`. Labels wrap when they are long, and the column
spacing grows to fit the widest label between two lifelines. A label that holds a literal
`{` should be quoted: `: "set { a }"`.

A message from a lifeline to itself is drawn as a compact loop with its label on the
right. A synchronous self-call also opens a short nested execution bar.

```drumlin
diagram sequence "Kinds of message"

participant Client
participant Server

Client -> Server : request()
Server --> Client : response
Client ->> Server : fire and forget
Server -> Server : log the request
Server ->> Server : schedule cleanup
```

## Execution blocks

Follow a message with `{` to open an [execution](../../elements/sequence/execution.md) on the
receiver: a bar that shows the receiver is active. The block closes with
`}`, and execution bars nest and step to the right as calls go deeper. `return` sends a
reply from inside a block, to the block's caller. If `return` is the last statement of
the block, the bar ends there; otherwise it ends where the block ends. Inside a block,
`return` takes free text, so it needs no quoting. The block may be written on one line.

```drumlin
diagram sequence "Nested calls"

participant "main()" as m
participant Parser
participant Lexer

m -> Parser : parse(source) {
  Parser -> Lexer : next() {
    Lexer -> Lexer : skip whitespace
    return token
  }
  Parser -> Parser : build tree
  return ast
}
```

## Creating and destroying

A dashed arrow to a head drawn at the moment of creation shows object creation: write
`sender -> new "Name" as alias : label`, with optional `<<stereotype>>`. Inside a block
the short form `create Name : label` is also accepted, and it uses the block's receiver
as the sender. `destroy name` ends a lifeline with an X. A participant that has been
destroyed cannot be used afterwards; a `destroy` in only one `alt` branch stays open for
the other branches.

```drumlin
diagram sequence "Worker lifetime"

participant Pool
Pool -> new "Worker" as w : spawn()
Pool -> w : run(job) {
  return result
}
Pool ->> w : stop
destroy w
```

## Found and lost messages

`[-> a` is a found message: its sender is outside the diagram, drawn as a filled circle
to the left. `a ->]` is a lost message, leaving towards a filled circle on the right.
Inside a block opened by a found message, `return` goes back to the outside.

```drumlin
diagram sequence "Boundaries"

participant Bus
participant Handler

[-> Bus : interrupt
Bus -> Handler : dispatch() {
  Handler ->] : retransmit (no ack)
  return handled
}
```

## Combined fragments

A [combined fragment](../../elements/sequence/fragment.md) groups messages under an operator.
`alt`, `opt`, `loop`, `break`, `critical` and `neg` wrap messages in a frame with the
operator in a pentagon at the top left and a `[guard]` above the first event. `else`
adds operands to `alt`, `and` adds them to `par`, and each operand is split by a dashed
line. Frames size themselves around their contents and nest. A loop guard written in
parentheses, such as `(1,5)`, joins the operator as `loop(1,5)`, following the
textual syntax of clause 17.6.4.

```drumlin
diagram sequence "Retries and alternatives"

participant Client
participant Server
participant Cache

loop "(1,3)" {
  Client -> Server : fetch(key) {
    alt "cached" {
      Server -> Cache : get(key)
      Cache --> Server : value
    } else "miss" {
      Server -> Server : recompute
      opt "worth caching" {
        Server ->> Cache : put(key, value)
      }
    }
    return value
  }
}
```

```drumlin
diagram sequence "Parallel and critical regions"

participant "Task A" as a
participant Lock
participant "Task B" as b

par {
  a -> Lock : acquire()
} and {
  b -> Lock : acquire()
}
critical "mutex held" {
  Lock --> a : granted
  a -> a : update state
  a -> Lock : release()
}
```

## Interaction references and notes

`ref over a, b : Name` draws an interaction use: a frame with a `ref` tab that stands
for another interaction.

`note over a: text`, `note left of a: text` and `note right of a: text` add notes with a
folded corner, on their own row. `note over a, b: text` spans several lifelines.

```drumlin
diagram sequence "Boot"

participant Bootloader as bl
participant Kernel as k
participant Init as i

ref over bl, k : Verify signature
bl -> k : jump(entry)
note over k: Interrupts stay masked until the scheduler starts.
k -> new "PID 1" as p : exec
note right of p: Everything runs under this process.
```

## Layout

Columns are spaced from the content between them: labels, self-message loops, notes and
frame contents. Rows follow an 8px grid, and each row grows to fit its label. Message
labels sit above their line, in the gap beside the sender when a message crosses other
lifelines, so that text never runs over a lifeline or an execution bar.

Heads stay at the top and are not repeated at the bottom. A diagram has no `sd` frame:
UML Annex A lets the frame and its heading be omitted when the page bounds the drawing,
and a `diagram sequence "Title"` title is a caption outside the diagram. A found message
starts, and a lost message ends, on a filled circle (17.4.4), and both keep the arrowhead
of their kind: filled for `->`, open for `->>`.

One detail of reading labels is worth knowing: a label runs to the end of the line, except for a
comment (` # ` or ` // `) and a space-separated `{` or `}`. Quote the label to keep those as text.

Next: the [sequence examples](examples.md), or look up any symbol under
[Elements of sequence diagrams](../../elements/sequence/overview.md).
