# Getting started

This page takes you from nothing to a rendered SVG in about five minutes. You need a Rust toolchain
(`cargo`); drumlin has no other dependencies. There is no published package, so you build it from
source.

## Build and install

Get the source and build it:

```sh
git clone https://github.com/davidpettersson/drumlin.git
cd drumlin
cargo build --release
```

The binary is `target/release/drumlin`. To run it as `drumlin` from anywhere, install it into
Cargo's binary folder (usually `~/.cargo/bin`, which must be on your `PATH`):

```sh
cargo install --path . --locked
```

The commands below assume `drumlin` is on your `PATH`. Otherwise, write `target/release/drumlin`
instead.

## Your first diagram

Create a file `hello.uml`:

```drumlin
diagram component "Hello"

component Client { requires IGreeting }
component Server { provides IGreeting }
```

Render it:

```sh
drumlin render hello.uml -o hello.svg
```

Open `hello.svg` in a browser to see the drawing above. Both components name `IGreeting`, so drumlin
wires the socket on `Client` to the lollipop on `Server` with a dashed dependency arrow. You did not draw
that line. (Between two [ports](../elements/component/port.md) with one interface each, drumlin draws a
joined ball-and-socket instead. See [Interface](../elements/component/interface.md).)

## Check a file

`drumlin check` reads a file and reports the first error, without writing anything. It exits with 0
when the file is valid, so it suits scripts, editors and coding agents.

```sh
drumlin check hello.uml
```

A mistake is reported with its line and column, and a suggestion where drumlin has one. If you add a
fifth line, `Client --> Sever`, drumlin says:

```text
hello.uml:5:12: error: unknown element `Sever`; did you mean `Server`?
```

[Errors](../reference/errors.md) lists the common ones.

## Grow the diagram

Add a package and an ordinary connector, `-->`, with a label after `:`:

```drumlin
diagram component "Hello, again"

package "Backend" {
  component Server { provides IGreeting }
  component Store <<database>>
  Server --> Store : read
}

component Client { requires IGreeting }
```

## Choose a style

Drawings use the `pens` style unless you ask for another. Two more styles exist: `plain`, with one line
weight, and `lettering`, with hand-lettered text. Try one from the command line:

```sh
drumlin render hello.uml -o hello.svg --style lettering
```

See [Styles](styles.md) for pictures of each.

Component, package and state diagrams can also take a different layout engine with `--layout`. The
default suits nearly every diagram; [File basics](file-basics.md#layout-engine) explains when to try the
other.

## Where next

- [File basics](file-basics.md): the header, direction, layout engine and comments.
- A how-to page for the kind you want to draw:
  [component](../diagrams/component/overview.md), [package](../diagrams/package/overview.md),
  [sequence](../diagrams/sequence/overview.md), [state](../diagrams/state/overview.md) or
  [activity](../diagrams/activity/overview.md).
- The [component examples](../diagrams/component/examples.md), to see what larger diagrams look like.
- The [command line reference](../reference/cli.md), for every option.
