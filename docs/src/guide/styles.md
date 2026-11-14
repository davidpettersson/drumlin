# Styles

A style decides how a drawing is inked: line weights, corners and lettering. This page shows the three
styles and how to choose one. You can skip it until you want a different look.

| Style | Look |
|---|---|
| `pens` (default) | Three pen weights in one warm black, sharp bends |
| `plain` | One line weight, grey small text, softened bends |
| `lettering` | The pens of `pens`, with all text lettered in single pen strokes |

Choose a style with a `style <name>` statement in the file, or with `--style <name>` on the command
line, which wins over the statement:

```sh
drumlin render shop.uml -o shop.svg --style plain
```

Styles apply to all five diagram kinds. Like `direction` and `layout`, the statement may stand on any
line, and the last one wins. An unknown name is an error that lists the styles. An element may still
be called `style`: only `style` followed by a name is the statement.

## pens

`pens` is the default; every diagram in this book that names no style uses it. It inks with three pen
weights in one warm black (`#22201c`): a heavy pen for boxes that hold other boxes, a medium pen for
ordinary boxes, symbols and solid lines, and a fine pen for dashed lines, notes and small details. Bends
are sharp. [Draw with three pens](../principles/shapes.md#draw-with-three-pens) says which pen draws
what.

```drumlin
diagram component "style pens"
style pens

component Shop {
  component Cart
  component Checkout
}
component Payments
Cart --> Checkout
Checkout ..> Payments
note Payments: Called once per order.
```

## plain

`plain` draws every outline and line with one weight, small text in grey, and bends slightly rounded.
Compare the same diagram above in `pens`.

```drumlin
diagram component "style plain"
style plain

component Shop {
  component Cart
  component Checkout
}
component Payments
Cart --> Checkout
Checkout ..> Payments
note Payments: Called once per order.
```

## lettering

`lettering` keeps the pens of `pens` and letters every piece of text in single pen strokes, as a
draftsman letters a drawing with a lettering guide: names and the title in spaced capitals, labels and
notes in upright mixed case, one light pen for all of it. See
[Letter with one pen](../principles/shapes.md#letter-with-one-pen).

```drumlin
diagram component "style lettering"
style lettering

component Shop {
  component Cart
  component Checkout
}
component Payments
Cart --> Checkout
Checkout ..> Payments : 3 retries
note Payments: Called once per order, 0 or 1 times per retry.
```

Things to know about `lettering`:

- The letters are drawn as paths, not set as text, so a browser cannot select them or find them with
  search. Each run of text keeps its words as an accessible name (`aria-label`) for screen readers.
- The lettering covers ASCII, Latin-1, much of Latin Extended-A, common punctuation, arrows and super-
  and subscript digits. Any other character is drawn as an open box.
- Files are about three to four times the size of the same drawing in `pens`.
- The glyphs are drumlin's lettering, derived from Routed Gothic by Darren Embry and licensed under the
  SIL Open Font License 1.1.

## What a style changes

`pens` and `plain` change only the strokes and the colour of the marks: every box, line and label stays
where the layout put it. `lettering` sizes boxes, labels and notes for its own letters, so its drawing
may be laid out a little differently.
