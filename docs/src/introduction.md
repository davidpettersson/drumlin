# drumlin

drumlin turns a small text format into UML diagrams that look drafted by hand: aligned boxes, even
spacing, orthogonal lines and restrained lettering. You write what the diagram contains; drumlin decides
where everything goes and writes an SVG file.

It draws five kinds of diagram: component, package, sequence, state and activity. Here is a component
diagram, with the source that produced it:

```drumlin
diagram component "Order Platform"
direction right

package "Edge" {
  component "Web Gateway" as gateway {
    requires IOrders
    requires IIdentity
  }
}

component "Order Service" as orders {
  port api provides IOrders
  port events requires IEventBus

  component "API Handler" as handler { requires IPricing }
  component "Pricing Engine" as pricing { provides IPricing }
  component "Outbox" as outbox

  api --> handler
  handler --> outbox : persist
  outbox --> events
}

component "Identity" as identity <<external>> { provides IIdentity }

package "Infrastructure" as infra {
  component "Event Bus" as bus { provides IEventBus }
  component "Postgres" as db <<database>>
  align bus db
}

component "Billing" as billing {
  requires IEventBus
  provides IInvoices
}

orders ..> db : SQL
note outbox: Publishes at least once. Consumers deduplicate on event id.
```

Every diagram in this book is rendered from the source shown with it, while the book is built (see
[Rendering at docs build time](internals/build.md)). In the diagram above you never draw the dashed
arrows between the interfaces: a component that `requires IOrders` is wired to one that
`provides IOrders` by name. See [Interface](elements/component/interface.md).

## Why drumlin

- **One small binary.** It is written in Rust and needs no JVM, browser or Graphviz.
- **A readable format.** One statement per line, in the spirit of PlantUML, with one regular grammar.
- **No layout work.** You state structure. [Layout hints](reference/hints.md) adjust the arrangement
  when you need to, and never take coordinates.
- **Deterministic output.** The same input gives byte-identical SVG, so diagrams diff cleanly in review.
- **Precise errors.** Each error has a line, a column and, where possible, a suggested fix.

## Get started

Build drumlin from source with a Rust toolchain, write a three-line file and render it:
[Getting started](guide/getting-started.md) takes about five minutes.

## Finding your way

The book has four main parts. Each answers a different question:

| Part | Answers | Use it when |
|---|---|---|
| **Guide** | How do I start? | You are new, or you want the file header, [styles](guide/styles.md) or [agent instructions](guide/agents.md). |
| **Diagrams** | How do I write a whole diagram of this kind? | You are drawing. One how-to page and one page of examples per kind. |
| **Elements** | What does this one symbol look like, and what is its syntax? | You are looking something up. One page per symbol, with its UML clause. |
| **Reference** | What exactly is allowed? | You need the [grammar](reference/grammar.md), the [command line](reference/cli.md), an [error message](reference/errors.md), a [layout hint](reference/hints.md) or a [term](reference/glossary.md). |

Two further parts explain how drumlin works. [Drawing principles](principles/introduction.md) states the
rules a good drawing follows, with their sources. [Internals](internals/layout.md) describes the layout
pipeline, the documentation build and the [roadmap](internals/roadmap.md).

## Status

drumlin is a prototype, at version 0.2.0. The input format, the command line and the drawings may still
change. The symbols follow UML 2.5.1. Which elements each diagram kind accepts, and how a drawing looks,
are drumlin's own choices: see [UML conformance](reference/conformance.md). Known limitations are listed
on the [roadmap](internals/roadmap.md#known-limitations).
