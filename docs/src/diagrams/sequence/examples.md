# Sequence diagram examples

Six sequence diagrams from protocols and systems work. For the notation, see
[Sequence diagrams](overview.md) and the [sequence elements](../../elements/sequence/overview.md).

## TCP connection: handshake, data and teardown

The three-way handshake, a segment and its acknowledgement, and a graceful close.
A lost segment leaves through a lost message.

```drumlin
diagram sequence "TCP connection"

participant "Client" as c
participant "Server" as s

note over c, s: Three-way handshake
c ->> s : SYN seq=x
s ->> c : SYN+ACK seq=y, ack=x+1
c ->> s : ACK ack=y+1
note over c, s: Established
c ->> s : data seq=x+1, len=512
s ->> c : ACK ack=x+513
opt "segment lost" {
  c ->] : data seq=x+513
  c ->> s : retransmit after RTO
}
note over c, s: Graceful close
c ->> s : FIN
s ->> c : ACK
s ->> c : FIN
c ->> s : ACK
note right of c: TIME_WAIT for 2 MSL before the socket is freed.
```

## Raft log replication

The leader appends locally, replicates to followers in parallel and commits once a
majority has acknowledged. One follower rejects the entry.

```drumlin
diagram sequence "Raft: replicating one log entry"

actor "Client" as client
participant "Leader" as leader <<raft>>
participant "Follower 1" as f1
participant "Follower 2" as f2
participant "State machine" as sm

client -> leader : command(x) {
  leader -> leader : append to log
  par {
    leader -> f1 : AppendEntries(term, entry) {
      f1 -> f1 : append and persist
      return success
    }
  } and {
    leader -> f2 : AppendEntries(term, entry) {
      alt "log matches prevLogIndex" {
        f2 -> f2 : append and persist
        return success
      } else "prevLogTerm mismatch" {
        return reject (conflict index)
        note right of f2: Leader walks nextIndex back and retries.
      }
    }
  }
  critical "majority acknowledged" {
    leader -> leader : advance commitIndex
    leader -> sm : apply(x) {
      return result
    }
  }
  return result
}
```

## Interrupt handling with a nested IRQ

A device interrupt enters the CPU as a found message, and a higher priority interrupt
preempts the handler. The hardware restores context on return from interrupt.

```drumlin
diagram sequence "Nested interrupt handling"

participant "Timer" as timer
participant "CPU core" as cpu
participant "Kernel entry" as entry
participant "UART ISR" as isr
participant "Timer ISR" as tisr
participant "Scheduler" as sched

[-> cpu : UART IRQ {
  cpu ->> entry : vector through IDT
  entry -> entry : save registers
  entry -> isr : uart_isr() {
    isr -> isr : read FIFO
    timer ->> cpu : timer IRQ
    note over cpu, tisr: Higher priority: the ISR is preempted.
    cpu -> tisr : timer_isr() {
      tisr ->> sched : mark reschedule
      return eoi
    }
    return eoi
  }
  entry -> entry : check need_resched
  opt "reschedule pending" {
    entry -> sched : schedule() {
      return next task
    }
  }
  entry ->> cpu : iret
  return resumes interrupted task
}
```

## TLS 1.3 handshake

The handshake with a ServerHello carrying the key share, and Finished from both sides.
Application data flows once the keys are in place. A failed verification ends with an
alert.

```drumlin
diagram sequence "TLS 1.3 handshake"

participant "Client" as c
participant "Server" as s
participant "CA store" as ca

c -> s : ClientHello(key_share, sni, supported_versions) {
  s -> s : derive handshake secrets
  return ServerHello(key_share)
}
note over c, s: Encrypted from here on: EncryptedExtensions, Certificate, CertificateVerify, Finished
s ->> c : EncryptedExtensions
s ->> c : Certificate
s ->> c : CertificateVerify
c -> ca : verify chain(certificate) {
  return ok
}
alt "certificate valid" {
  s ->> c : Finished
  c ->> s : Finished
  note over c, s: Application data, protected by traffic keys
  c ->> s : HTTP request
  s ->> c : HTTP response
} else "verification failed" {
  c ->> s : alert(bad_certificate)
  c ->] : close connection
}
```

## RPC with retry and exponential backoff

A client retries a call that times out and gives up after three attempts. A circuit
breaker records the outcome.

```drumlin
diagram sequence "RPC with retry"

participant "Client stub" as stub
participant "Circuit breaker" as cb
participant "Service" as svc

loop "(1,3)" {
  stub -> cb : allow? {
    return true
  }
  stub -> svc : GetOrder(id, deadline=200ms) {
    alt "reply before deadline" {
      return order
      break "success" {
        stub ->> cb : record success
      }
    } else "deadline exceeded" {
      stub -> stub : sleep(backoff with jitter)
      stub ->> cb : record failure
    }
  }
}
stub --> stub : surface error to caller
note right of stub: The stub surfaces the last error once retries run out.
```

## Page fault in a demand-paged kernel

A page fault handled through a lookup and swap-in, showing creation and destruction of a
transient object.

```drumlin
diagram sequence "Page fault"

participant "MMU" as mmu
participant "Kernel" as k
participant "Swap device" as swap

[-> mmu : translate(vaddr)
mmu -> k : page fault(vaddr) {
  k -> new "Fault context" as fc <<per fault>> : allocate
  k -> k : find VMA
  alt "anonymous page" {
    k -> swap : read(slot) {
      note right of swap: The request is queued and the task sleeps.
      return page
    }
    k -> k : map page
  } else "invalid access" {
    k ->] : SIGSEGV to task
  }
  destroy fc
  return retry the access
}
```
