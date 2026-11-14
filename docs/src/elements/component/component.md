# Component

A component is a rectangle with the component icon at its top right and its name inside. The
simplest needs only a name.

```drumlin
component Scheduler
```

## Names and aliases

A name with spaces needs quotes. Use `as` to give it a short alias for references.

```drumlin
component "Memory Allocator" as alloc
component "Page Table" as pages
alloc --> pages
```

## Stereotypes

A stereotype goes in `<<` and `>>`.

```drumlin
component "Orders DB" as db <<database>>
component "Billing API" as api <<external>>
api --> db
```

## Nesting

Components can contain components. The outer one becomes a composite.

```drumlin
component "Network Stack" as net {
  component Socket
  component TCP
  component IP
  Socket --> TCP
  TCP --> IP
}
```

Nesting can go several levels deep.

```drumlin
component Kernel {
  component "I/O" as io {
    component VFS
    component Block
    VFS --> Block
  }
  component Memory
  io --> Memory
}
```

## One-line blocks

A short body can share the line with its braces.

```drumlin
component Sensor { provides ISample }
component Logger { requires ISample }
```

## Syntax

```text
component <name> [as <alias>] [<<stereotype>>] [{ ... }]
```

## Specification

UML 2.5.1, 11.6.4 (p. 210). A component is a classifier rectangle; showing the icon lets the
«component» keyword be hidden, which drumlin does. In a composite the name is centred, with any
keyword above it (9.2.4, p. 101).

## Used in

[Monolithic kernel](../../diagrams/component/examples.md#monolithic-kernel),
[Storage engine](../../diagrams/component/examples.md#storage-engine),
[Compiler pipeline](../../diagrams/component/examples.md#compiler-pipeline).

## See also

[Component diagrams](../../diagrams/component/overview.md) (how to write the whole diagram), [Elements of component diagrams](overview.md), [Grammar](../../reference/grammar.md#component-diagrams).
