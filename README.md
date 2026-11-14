# drumlin

drumlin turns a small text format into UML diagrams that look drafted by hand: aligned boxes, even
spacing, orthogonal lines and restrained lettering. You write what a diagram contains; drumlin lays it
out and writes SVG. It draws five kinds of diagram: component, package, sequence, state and activity.

![A component diagram rendered by drumlin from examples/orders.uml](examples/orders.svg)

The drawing above is [`examples/orders.svg`](examples/orders.svg), rendered from
[`examples/orders.uml`](examples/orders.uml).

drumlin is a prototype, at version 0.1.0, written in Rust with no dependencies. The input format, the
command line and the drawings may still change between versions. The symbols follow UML 2.5.1; which
elements each diagram kind accepts, and how a drawing looks, are drumlin's own choices (see
[UML conformance](docs/src/reference/conformance.md)).

## Install and use

Install the published crate with a Rust toolchain:

```
cargo install drumlin
```

Or build from source:

```
git clone https://github.com/davidpettersson/drumlin.git
cd drumlin
cargo build --release              # the binary is target/release/drumlin
cargo install --path . --locked    # optional: puts drumlin on your PATH
```

```
drumlin render examples/orders.uml -o orders.svg     # SVG to a file (stdout without -o)
drumlin check examples/orders.uml                    # validate only; exit code 0 when valid
drumlin render examples/orders.uml --style plain     # style: pens (default), plain or lettering
drumlin render examples/orders.uml --layout network-simplex   # layout engine: longest-path (default)
```

Output is deterministic: the same input gives byte-identical SVG. Errors have the form
`file:line:col: error: message` and suggest close matches for unknown names.

A small file:

```
diagram component "Hello"

component Client { requires IGreeting }
component Server { provides IGreeting }
```

`Client` and `Server` name the same interface, so drumlin wires them; you never draw that line.

## Documentation

The book in `docs/` covers getting started, every diagram kind and element, the grammar, layout hints,
error messages, the drawing principles and how layout works. Read it at
<https://davidpettersson.github.io/drumlin/>, or build and read it locally:

```
cargo install --path docs/mdbook-drumlin --locked
mdbook serve docs
```

The `mdbook-drumlin` preprocessor renders every `drumlin` code fence to inline SVG at build time, and a
diagram that fails to parse fails the build. The `Dockerfile` builds the book as a static site.

## Tests

```
cargo test --workspace
```

The unit tests sit beside the code. `tests/golden/` holds one `.uml` source and its expected `.svg` per
case, and the golden test fails when a drawing changes by a single byte. The quality test in
`src/diagrams/quality.rs` counts crossings, bends, label faults and drawing size over the golden files,
and fails when a change raises a total. When a change to the drawings is intended, regenerate the
golden files and review the new SVGs before you commit them:

```
DRUMLIN_BLESS=1 cargo test --test golden
```

## Contributing

Issues and pull requests are welcome at https://github.com/davidpettersson/drumlin. Please do not run
`cargo fmt` over existing files: the code is not rustfmt-formatted, and a reformat buries the real
change. New source files start with the copyright and SPDX lines that every existing file carries:

```
// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0
```

The [drawing principles](docs/src/principles/introduction.md) in the book state the layout rules a
change is judged against.

## License

drumlin is licensed under the [Apache License, Version 2.0](LICENSE). Copyright 2026 David Pettersson.

If you distribute drumlin, or software that includes it, in source or binary form, you must keep the
copyright notices, include a copy of the license, and pass on the attribution in [NOTICE](NOTICE). Files
that you change must say that you changed them. Each source file carries its copyright line and an
`SPDX-License-Identifier`, so a file copied on its own still says where it came from; keep those lines.

The glyph data of the `lettering` style, `src/core/lettering/glyphs.rs`, is derived from Routed Gothic
by Darren Embry (Copyright (c) 2017, Reserved Font Name "Routed Gothic") and modified; that one file is
licensed under the [SIL Open Font License 1.1](LICENSES/OFL-1.1.txt), not under the Apache License.
Everything else in drumlin is Apache-2.0. The lettering is drumlin's own and does not carry the name
Routed Gothic.

SVG drawings that drumlin produces from your diagram sources are yours. The license puts no condition on
them, and the Open Font License puts none on documents made with the lettering.
