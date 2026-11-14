# State diagram examples

Six state machines from protocols, devices and resilience patterns. For the notation, see
[State diagrams](overview.md) and the [state elements](../../elements/state/overview.md).

## TCP connection

The TCP connection states from RFC 793, with the segment each transition sends.

```drumlin
diagram state "TCP connection"
[*] --> Closed
Closed --> Listen : passive open / create TCB
Closed --> SynSent : active open / send SYN
Listen --> SynRcvd : rcv SYN / send SYN+ACK
SynSent --> Established : rcv SYN+ACK / send ACK
SynRcvd --> Established : rcv ACK
Established --> FinWait1 : close / send FIN
Established --> CloseWait : rcv FIN / send ACK
FinWait1 --> FinWait2 : rcv ACK
FinWait2 --> TimeWait : rcv FIN / send ACK
CloseWait --> LastAck : close / send FIN
LastAck --> Closed : rcv ACK
TimeWait --> Closed : timeout 2MSL
```

## USB device

The device states of the USB specification. `Configured` carries its internal behaviours in a compartment.

```drumlin
diagram state "USB device"
[*] --> Attached
Attached --> Powered : hub configured
Powered --> Default : bus reset
Default --> Address : set address
Address --> Configured : set configuration [config != 0]
Configured --> Address : set configuration [config == 0]
Configured --> Suspended : bus idle 3 ms
Suspended --> Configured : bus activity
Powered --> Attached : hub deconfigured
Configured --> Default : bus reset
state Configured {
  entry / enable endpoints
  exit / disable endpoints
  SOF / update frame counter
}
```

## Process lifecycle

A Unix-style process. The self-transition is a compact loop, and a note marks the zombie state.

```drumlin
diagram state "Process lifecycle"
[*] --> New
New --> Ready : admitted
Ready --> Running : dispatch
Running --> Ready : preempt [quantum expired]
Running --> Waiting : wait(event) / block()
Waiting --> Ready : event occurs
Running --> Zombie : exit() / release resources
Zombie --> [*] : reaped by parent
Running --> Running : syscall [fast path] / handle inline
note Zombie: Holds only its exit status until the parent calls wait.
```

## Elevator controller

Motion and doors run in orthogonal regions of one composite state.

```drumlin
diagram state "Elevator controller"
[*] --> Idle
Idle --> Operating : call received / plan route
Operating --> Idle : queue empty
Operating --> Fault : motor overcurrent / cut power
Fault --> Idle : technician reset
state Operating {
  entry / release brake
  exit / engage brake
  region "motion" {
    [*] --> Moving
    Moving --> Stopping : near target floor
    Stopping --> Levelled : speed zero
    Levelled --> Moving : next call
  }
  region "doors" {
    [*] --> Closed
    Closed --> Opening : levelled at floor
    Opening --> Open : fully open
    Open --> Closing : door timer
    Closing --> Closed : fully closed
    Closing --> Opening : obstruction
  }
}
```

## OTA firmware update

A download with a nested choice, and a junction that fans out after reboot.

```drumlin
diagram state "OTA firmware update"
[*] --> Idle
Idle --> Downloading : update available / open session
state Downloading {
  entry / start watchdog
  exit / stop watchdog
  chunk received / append to flash
  [*] --> Fetching
  Fetching --> Verifying : last chunk
  Verifying --> c1 
  choice c1
  c1 --> [*] : [hash ok]
  c1 --> Fetching : [hash bad] / clear buffer
}
Downloading --> Installing
Downloading --> Idle : network lost
Installing --> Rebooting : image written / mark pending
Rebooting --> j1 : boot complete
junction j1
j1 --> Confirmed : [self test passed]
j1 --> RolledBack : [self test failed] / restore slot A
Confirmed --> [*]
RolledBack --> [*]
```

## Circuit breaker

The resilience pattern, with compact self-transitions on `Closed` and `Open`.

```drumlin
diagram state "Circuit breaker"
[*] --> Closed
Closed --> Closed : success / reset failure count
Closed --> Open : failures >= threshold / start timer
Open --> Open : call / fail fast
Open --> HalfOpen : timer expired
HalfOpen --> Closed : probe succeeded / reset failure count
HalfOpen --> Open : probe failed / restart timer
state Closed {
  do / count failures
}
```

