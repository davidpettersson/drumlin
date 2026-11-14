# Rendering at docs build time

No diagram in this book is a checked-in image. Authors write a fenced code block, and a preprocessor
replaces it with inline SVG while the book builds. This page shows how to write such a block and how to
build the book, for contributors and for anyone who wants the same setup in their own mdBook.

```drumlin,render-only
diagram component "From chapter to page"
direction right

component "Chapter Markdown" as md <<input>>

component mdBook as book {
  port hook requires IPreprocess
  component "Book Loader" as loader
  component "HTML Renderer" as html
  loader --> html
}

component "mdbook-drumlin" as pre {
  provides IPreprocess
  requires IRender
}

component "drumlin crate" as lib { provides IRender }
component "Published Site" as site <<output>>

md --> book
book --> site : inline SVG
```

## How to write one

Use a code fence with the language `drumlin`:

````markdown
```drumlin
component Client { requires IGreeting }
component Server { provides IGreeting }
```
````

The page then shows the source and this diagram:

```drumlin,render-only
component Client { requires IGreeting }
component Server { provides IGreeting }
```

## Fence options

| Fence | Output |
|---|---|
| ` ```drumlin ` | Source, then diagram |
| ` ```drumlin,render-only ` | Diagram alone |
| ` ```drumlin,error ` | Source, then drumlin's real error message |

## Set up

The preprocessor is a small Rust program that links drumlin as a library.

```toml
# book.toml
[preprocessor.drumlin]
command = "mdbook-drumlin"
```

```sh
cargo install --path docs/mdbook-drumlin --locked
mdbook build docs
```

## Why render during the build

- **Examples cannot go stale.** Each image always comes from the source beside it.
- **A broken diagram breaks the build.** A parse error stops `mdbook build`. So
  does a `drumlin,error` example that parses successfully.
- **Error messages stay honest.** The [Errors](../reference/errors.md) page shows
  output captured at build time.
- **Pages need no client-side rendering.** Readers get plain SVG with no JavaScript.
- **The result is static.** The built book is plain HTML and SVG, which any web server can serve. The
  repository's `Dockerfile` builds it.
