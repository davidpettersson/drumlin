# Activity diagram examples

Five activity diagrams from build, kernel and firmware work. Each is also a golden test, so what you see
is exactly what `drumlin` produces. For the notation, see [Activity diagrams](overview.md) and the
[activity elements](../../elements/activity/overview.md).

## Build and release pipeline

Compile, tests and lint run concurrently. The note attaches to the fork. A release only happens for a tagged
build, so the signing steps sit on the main axis and the empty `[else]` branch runs around them.

```drumlin
diagram activity "Build and release pipeline"
start
action "Fetch sources"
action "Resolve dependencies"
object "Lockfile"
fork {
  action "Compile"
  action "Link"
} and {
  action "Run unit tests"
} and {
  action "Lint and audit"
}
note: Compile, test and lint run concurrently.
action "Package artifacts"
if "tag build" {
  action "Sign artifacts"
  send "Publish release"
}
action "Notify the team"
stop
```

## Interrupt service with deferred work

The top half is the hard interrupt handler; the bottom half is the deferred work. The handler is started by an
accept event rather than an initial node, and hands off with a signal.

```drumlin
diagram activity "Interrupt service with deferred work"
accept "IRQ line raised"
action "Acknowledge at the controller"
action "Read status register"
if "FIFO has data" {
  action "Copy bytes to the ring buffer"
  send "Wake softirq"
} else "overrun" {
  action "Count overrun"
}
action "Return from interrupt"
accept "softirq scheduled"
while "ring buffer not empty" {
  action "Pop one packet"
  if "checksum valid" {
    action "Deliver to the stack"
  } else {
    action "Drop and count"
  }
}
end
```

## Connection retry

A `repeat` wraps the handshake with backoff, and early failures step aside so that the successful
path stays straight.

```drumlin
diagram activity "Connection retry with backoff"
start
action "Resolve host"
if "resolved" {
  repeat {
    action "Open socket"
    action "TLS handshake"
    if "failed" {
      action "Wait for backoff"
    }
  } until "connected or attempts exhausted"
  if "connected" {
    action "Send greeting"
  } else {
    action "Report unreachable"
    stop
  }
} else {
  action "Report DNS failure"
  stop
}
action "Mark session ready"
stop
```

## Deployment with parallel checks across lanes

The fork spans the lanes running the checks, and the release decision returns to the developer's lane.

```drumlin
diagram activity "Deployment with parallel checks"
start
lane "Developer" {
  action "Merge to main"
}
lane "CI" {
  action "Build image"
  send "Image ready"
}
lane "Staging" {
  accept "Image ready"
  action "Deploy to staging"
}
fork {
  lane "Staging" {
    action "Smoke tests"
  }
} and {
  lane "Security" {
    action "Scan image"
  }
} and {
  lane "CI" {
    action "Contract tests"
  }
}
lane "Developer" {
  if "all green" {
    lane "Production" {
      action "Roll out canary"
    }
  } else {
    action "Roll back and file issue"
  }
}
stop
```

## Firmware boot

A `switch` on the boot slot, with «decisionInput» naming what the guards test.

```drumlin
diagram activity "Firmware boot"
start
action "Clock and RAM init"
if "reset cause is brown-out" {
  action "Record fault"
}
switch "boot slot" {
  case "A" {
    action "Verify slot A"
  }
  case "B" {
    action "Verify slot B"
  }
  case "else" {
    action "Enter recovery"
    end
  }
}
if "signature valid" {
  action "Jump to application"
  note: The vector table is relocated first.
} else {
  action "Enter recovery"
}
stop
```
