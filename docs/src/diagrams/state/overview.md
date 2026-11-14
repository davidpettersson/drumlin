# State diagrams

A state machine diagram shows the states something can be in and the transitions between them. This
page walks through writing one. Start a file with `diagram state`, optionally followed by a title.

The notation follows UML 2.5.1 clause 14. Each symbol has its own page under
[Elements](../../elements/state/overview.md), the full syntax is in the
[grammar](../../reference/grammar.md#state-diagrams), and there is a page of [examples](examples.md).

The flow runs down the page by default, so transition labels sit beside vertical lines.
`direction right` places labels above horizontal lines instead, at the cost of wide gaps between ranks.
The layout reserves room for every label.

```drumlin
diagram state "Door"
[*] --> Closed
Closed --> Opening : open [unlocked] / start motor
Opening --> Open : fully open / stop motor
Open --> Closing : close
Closing --> Closed : fully closed
Closing --> Opening : obstruction / reverse motor
Closed --> Locked : lock
Locked --> Closed : unlock [valid code]
```

A name used in a transition declares a state in the deepest block that mentions it.
A name that appears only once and is at most two edits from another name is reported
as a typo. To declare such a state on purpose, write `state X` first.
Use `state "Display name" as alias` when a state needs a longer name or a body.

## Transition labels

A [transition](../../elements/state/transition.md) label follows `:` and has the UML form, in which
every part is optional: the trigger is the event that fires the transition, the
[guard](../../reference/glossary.md#guard) a condition that must hold, and the effect what happens.

```text
trigger {, trigger} [guard] / effect
```

A long label breaks before the effect, then wraps. Transitions between the same pair of
states run side by side with their labels on the outside. A self-transition is a loop on the
side of its state, with its label beside it.

```drumlin
diagram state "Transition labels"
direction right
[*] --> Waiting
Waiting --> Armed : arm, rearm [sensors ready] / log("armed")
Armed --> Waiting : disarm
Armed --> Armed : tick / count()
Armed --> Armed : drift [abs(x) > limit] / correct()
Armed --> Firing : trigger [armed and not locked] / ignite(); start_timer()
Firing --> [*] : done
```

UML restricts some labels, and drumlin reports a violation as an error:

- A transition from a nested initial pseudostate is unlabeled. At the top level it may name
  the creating event.
- A transition leaving a pseudostate has no trigger. It may have `[guard] / effect`.
- The segments into a join and out of a fork have no trigger or guard.

## Internal behaviours

Inside a `state` block, `entry`, `do` and `exit` lines go in a compartment under the name. The
expression after `/` is optional. Any other line with a `/` and no arrow is an internal
transition, in the same form as a transition label. Each of these takes a whole line.

```drumlin
diagram state "Password entry"
[*] --> Typing
state Typing {
  entry / set echo invisible
  exit / set echo normal
  character / handle character
  help / display help
}
Typing --> [*] : enter
```

## Composite states and history

A `state` block with vertices inside is a composite state, and its contents form its region.
`[*] --> X` starts the enclosing region at the initial pseudostate, which sits first in the
flow. `X --> [*]` ends it at the final state. Each region has one of each, scoped to the block
they are written in.

`[H]` is shallow history and `[H*]` is deep history. Written alone or in a transition inside a
block, they belong to that block. `Name[H]` and `Name[H*]` reach the history of a composite
state from outside it. A transition may also start at, end at or cross the border of a
composite state.

```drumlin
diagram state "Composite states and history"
[*] --> Off
Off --> Playing : power
state Playing {
  entry / spin up
  do / stream
  exit / spin down
  [*] --> Track
  Track --> Track : next / advance
  Track --> Seeking : seek
  Seeking --> Track : found
  [H]
}
Playing --> Paused : pause
Paused --> Playing[H] : resume
Playing --> Off : power
Paused --> Off : power
```

## Orthogonal regions

`region "name" { ... }` divides a state into regions, drawn tiled and separated by dashed
lines. The name is optional. Regions sit across the flow: side by side when flowing down,
stacked when flowing right. A transition cannot join two regions of one state.

```drumlin
diagram state "Elevator controller"
[*] --> Idle
Idle --> Operating : call received / plan route
Operating --> Idle : queue empty
Operating --> Fault : motor overcurrent / cut power
Fault --> Idle : technician reset
state Operating {
  entry / release brake
  exit / engage brake
  region "motion" {
    [*] --> Moving
    Moving --> Stopping : near target floor
    Stopping --> Levelled : speed zero
    Levelled --> Moving : next call
  }
  region "doors" {
    [*] --> Closed
    Closed --> Opening : levelled at floor
    Opening --> Open : fully open
    Open --> Closing : door timer
    Closing --> Closed : fully closed
    Closing --> Opening : obstruction
  }
}
```

## Pseudostates

```text
choice c         a diamond
junction j       a small filled circle
fork f           a thick bar across the flow; one line in, at least two out
join j           a thick bar across the flow; at least two lines in, one out
terminate t      a cross
```

A pseudostate is declared with a name, and is used like a state. Lines fan out from the
corners of a choice or junction. A bar widens over the states it connects.

```drumlin
diagram state "Every pseudostate"
[*] --> Ready
Ready --> c : request
choice c
c --> Worker : [workers free] / assign
c --> Queued : [all busy]
Queued --> j : worker free
Worker --> j : done
junction j
j --> f : / prepare
fork f
f --> A
f --> B
state Tasks {
  region "left" {
    A --> [*]
  }
  region "right" {
    B --> [*]
  }
}
Tasks --> T : fatal
terminate T
```

## Entry and exit points

`entrypoint name` and `exitpoint name`, written directly in a `state` block, place a small
circle or a crossed circle on the state's border, with its name beside it. Transitions from
outside attach on the outer side and transitions from inside on the inner side.

```drumlin
diagram state "Entry and exit points"
[*] --> Idle
Idle --> again : retry
state Login {
  entrypoint again
  exitpoint aborted
  again --> Prompt
  Prompt --> Check : submit
  Check --> aborted : [locked out]
  Check --> [*] : [ok]
}
aborted --> Idle
Login --> Done
```

## Local transitions

A transition between a composite state and a vertex inside it is a local transition, and is
drawn to or from the inside of the state's border, on its right side.

```drumlin
diagram state "Local transitions"
[*] --> Active
state Active {
  [*] --> One
  One --> Two : next
  Two --> Active : reset
}
Active --> [*]
```

## Notes and hints

`note <target>: text` attaches a [note](../../elements/note.md) to any state or pseudostate. The
[layout hints](../../reference/hints.md) `left-of`, `right-of`, `above`, `below` and `align` adjust the
layout when the automatic result needs help.

Next: the [state examples](examples.md), or look up any symbol under
[Elements of state diagrams](../../elements/state/overview.md).
