# Command line

This page lists every command and option of the `drumlin` binary. For a first run, see
[Getting started](../guide/getting-started.md).

```text
drumlin render <input.uml> [-o <output.svg>] [--layout <engine>] [--style <name>]
drumlin check <input.uml> [--layout <engine>] [--style <name>]
```

## Commands

| Command | Effect |
|---|---|
| `drumlin render <file>` | Renders the file and writes the SVG to standard output. |
| `drumlin render <file> -o <out.svg>` | Writes the SVG to a file instead. |
| `drumlin check <file>` | Reads and checks the file, and writes nothing. |

drumlin reads one file per run. Options may come before or after the file name.

## Options

| Option | Effect |
|---|---|
| `-o <out.svg>` | With `render`: the file to write. |
| `--layout <engine>` | The [layout engine](../guide/file-basics.md#layout-engine): `longest-path` (the default) or `network-simplex`. Component, package and state diagrams only. Wins over the file's `layout` statement. |
| `--style <name>` | The [style](../guide/styles.md): `pens` (the default), `plain` or `lettering`. Every diagram kind. Wins over the file's `style` statement. |

Run `drumlin` with no arguments to print the usage text, with the engines and styles.

## Exit status and errors

Both commands exit with 0 on success and 1 on any error. An error in the file is printed to standard
error as `file:line:col: error: message`, and only the first error is reported:

```text
hello.uml:5:12: error: unknown element `Sever`; did you mean `Server`?
```

An unknown engine or style given on the command line is reported without a file or position, for example
``error: unknown style `plan`; did you mean `plain`?``. [Errors](errors.md) lists the common messages
with the source that causes them.
