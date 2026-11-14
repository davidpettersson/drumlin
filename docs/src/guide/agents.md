# Working with agents

This page is for people who let a coding agent write drumlin files, and for the agent itself. It gives
the loop to follow, an instruction to paste, and the rules an agent most often needs.

drumlin suits coding agents, because the text is the design and the picture is for the humans
reviewing it:

- **One regular grammar.** Each statement fits on one line, and nothing depends on visual layout in
  the source.
- **No layout work.** An agent states structure. drumlin places every box.
- **Deterministic output.** Regenerating an unchanged model changes no bytes.
- **Errors an agent can act on.** `drumlin check` reports a line, a column and a likely fix.

## A typical loop

The agent writes the model, `drumlin check` reports errors until the file is clean, and
`drumlin render` produces the SVG a person reviews:

```drumlin,render-only
diagram component "Agent feedback loop"
direction right

component Agent { provides IModel }
component "drumlin check" as check {
  requires IModel
  provides IDiagnostics
}
component "drumlin render" as render { requires IDiagnostics }
component Reviewer <<human>>

render --> Reviewer : SVG
```

## Suggested instruction for an agent

```text
Describe architecture in drumlin (.uml) files. Start each file with
`diagram <kind> "Title"`, where kind is component, package, sequence, state
or activity. In component diagrams, declare components with `provides` and
`requires`, and let matching interface names wire them; do not draw those
connectors yourself. Run `drumlin check <file>` after each edit and fix every
reported error before rendering. Do not add layout hints unless a reviewer
asks for a specific arrangement.
```

## Rules agents most often miss

- **Quote names with spaces**, and give them an alias with `as` to refer to them:
  `component "Order Service" as orders`.
- **Sequence diagrams:** declare every `participant` or `actor` before its first message. The
  declaration order is the left-to-right order.
- **State diagrams:** a transition is written with `-->`, and its label has the form
  `trigger [guard] / effect`. A transition leaving a choice or other pseudostate has no trigger.
- **Activity diagrams:** quote any name or guard with spaces (`action "Ship order"`,
  `if "card valid" { ... }`), put `start` first, and write nothing after `stop` or `end` in the same flow.
- **Declare both ends of a line.** In component and package diagrams each end must be declared somewhere
  in the file, before or after the line. In state diagrams, naming a state in a transition declares it.
- **One error at a time.** `check` reports the first error only; fix it and run again.

## Where to look things up

- [Grammar](../reference/grammar.md): every statement of every kind on one page.
- [Errors](../reference/errors.md): common messages and the source that causes them.
- The how-to page of each kind under **Diagrams**, and the examples beside it.
