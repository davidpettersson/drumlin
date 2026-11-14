# Grammar

Every statement of every diagram kind, in one place, for looking things up. Each kind's section links to
its how-to page, where the same statements are explained with pictures.

## Common to every kind

```text
diagram <kind> ["Title"]          kind: component, package, sequence, state, activity
style (pens | plain | lettering)  every kind
direction (right | down)          component, package and state diagrams
layout <engine>                   component, package and state diagrams
# comment                         also // comment
```

- Each statement occupies one line. A `{ ... }` body may share the line with its braces.
- A file with no `diagram` line is a component diagram.
- A `<name>` is a bare word or a `"quoted string"`. Bare words may contain letters, digits, `_`, and
  single `-` or `.` between them.
- `<<stereotype>>` after a name is drawn as the keyword `«stereotype»`.
- `direction`, `layout` and `style` may stand on any line; the last one wins. See
  [File basics](../guide/file-basics.md) and [Styles](../guide/styles.md).

## Component diagrams

```text
diagram component ["Title"]
direction (right | down)
layout <engine>
style <name>

package <name> [as <alias>] [<<stereotype>>] [{ ... }]
component <name> [as <alias>] [<<stereotype>>] [{ ... }]

provides <interface> [, <interface>]...
requires <interface> [, <interface>]...
port <name> [left | right | top | bottom] [provides <i> | requires <i>]...

<ref> (--> | ..> | -- | .. | <-- | <..) <ref> [: <label>]
note <element>: <text>

align <element> <element>...
<element> (left-of | right-of | above | below) <element>
```

An element is referenced by its alias, or by its name when it has no alias.
A `<ref>` is an element, a unique port name, or `<element>.<port>`.

Component rules:

- `provides`, `requires` and `port` belong inside a component.
- A package may be nested inside a component (it is one of its packaged elements).
- A connector needs two different components.
- A requirer and a provider of the same interface name are wired automatically. See
  [Interface](../elements/component/interface.md).
- Declarations may come after the connectors that use them.
- `layout <engine>` picks the layout engine of component, package and state diagrams: `longest-path`
  (the default) or `network-simplex`. See [File basics](../guide/file-basics.md#layout-engine).

How-to: [Component diagrams](../diagrams/component/overview.md). Symbols:
[Elements of component diagrams](../elements/component/overview.md). [Examples](../diagrams/component/examples.md).

## Package diagrams

```text
diagram package ["Title"]
direction right|down
layout <engine>
style <name>
package <name> [as <alias>] [<<stereotype>>] [{ ... }]
class|interface|enum <name> [as <alias>] [<<stereotype>>]
a ..> b [: label]   a --> b   a -- b   a .. b   a --|> b   a ..|> b   a +-- b
note <target>: text
a left-of|right-of|above|below b
align a b c
```

Statements can be separated with `;` as well as newlines.

How-to: [Package diagrams](../diagrams/package/overview.md). Symbols:
[Elements of package diagrams](../elements/package/overview.md). [Examples](../diagrams/package/examples.md).

## Sequence diagrams

```text
diagram sequence ["Title"]
style <name>
actor|participant <name> [as <alias>] [<<stereotype>>]
<a> (-> | ->> | -->) <b> [: label] [{ ... }]
<a> -> new <name> [as <alias>] [<<stereotype>>] [: label] [{ ... }]
create <name> [as <alias>] [<<stereotype>>] [: label] [{ ... }]     inside a block
[-> <a> [: label] [{ ... }]     found         <a> ->] [: label]     lost
return [text]                 inside a block
destroy <a>
alt ["guard"] { ... } [else ["guard"] { ... }]...
par ["guard"] { ... } [and ["guard"] { ... }]...
opt|loop|break|critical|neg ["guard"] { ... }
ref over <a>[, <b>]... : <name>
note over <a>[, <b>]... : text        note left of <a>: text        note right of <a>: text
```

Participants must be declared before use. `;` separates statements outside labels. A label
extends to the end of the line, except for a comment after whitespace (` # ` or ` // `) or a
space-separated `{` or `}`; quote a label to keep these as text.

How-to: [Sequence diagrams](../diagrams/sequence/overview.md). Symbols:
[Elements of sequence diagrams](../elements/sequence/overview.md). [Examples](../diagrams/sequence/examples.md).

## State diagrams

```text
diagram state ["Title"]
direction right|down
layout <engine>
style <name>
state <name> [as <alias>] [<<stereotype>>] [{ ... }]
region ["name"] { ... }                                   inside a state
entry|do|exit [/ expression]                              inside a state, one per line
trigger {, trigger} [guard] / effect                      internal transition, one per line
[*] --> a    a --> [*]    a --> b [: label]    b <-- a
[H]   [H*]   Name[H]   Name[H*]                           shallow and deep history
choice|junction|fork|join|terminate <name>
entrypoint|exitpoint <name>                               inside a state
note <target>: text
a left-of|right-of|above|below b
align a b c
```

A transition label has the form `trigger {, trigger} [guard] / effect`, with every part
optional. `;` separates statements except internal behaviour lines and labels. A label extends
to the end of the line, except for a comment after whitespace.

How-to: [State diagrams](../diagrams/state/overview.md). Symbols:
[Elements of state diagrams](../elements/state/overview.md). [Examples](../diagrams/state/examples.md).

## Activity diagrams

```text
diagram activity ["Title"]
style <name>
start                                                     first statement of the flow
action|send|accept|object "name"
if "guard" { ... } [else if "guard" { ... }]... [else ["guard"] { ... }]
switch ["subject"] { case "guard" { ... } case "guard" { ... } ... }
while "guard" { ... }
repeat { ... } until "guard"
fork { ... } and { ... } [and { ... }]...
lane "name" { ... }
note: text
stop                                                      activity final
end                                                       flow final
```

A note attaches to the statement before it. Guards are written without brackets. In a diagram with lanes,
actions, signals and objects must be inside a `lane` block.

How-to: [Activity diagrams](../diagrams/activity/overview.md). Symbols:
[Elements of activity diagrams](../elements/activity/overview.md). [Examples](../diagrams/activity/examples.md).
