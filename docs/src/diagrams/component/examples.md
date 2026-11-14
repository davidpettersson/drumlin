# Component diagram examples

Twenty-three component diagrams from systems software, embedded work, compilers and tools, services and
data platforms, grouped by domain. Each model is illustrative. Use them as starting points: copy the
source and change it. For the notation, see
[Component diagrams](overview.md) and the [component elements](../../elements/component/overview.md).

## Systems software

### Monolithic kernel

```drumlin
diagram component "Kernel subsystems"
direction down

component "System Call Interface" as sci

package "Core" as core {
  component "Process Scheduler" as sched
  component "Memory Manager" as mm
  sched --> mm : page faults
}

package "I/O" as io {
  component VFS
  component "Network Stack" as net
  component "Block Layer" as block
  VFS --> block
}

component "Device Drivers" as drivers

sci --> sched
sci --> VFS
sci --> net
block --> drivers
net --> drivers
```

### Storage engine

```drumlin
diagram component "LSM storage engine"
direction right

component Client { requires IKeyValue }

component Engine as engine {
  port kv provides IKeyValue
  component "Write Path" as write
  component "Read Path" as read
  component Memtable
  component "Write-Ahead Log" as wal
  component Compactor

  kv --> write
  kv --> read
  write --> wal : append
  write --> Memtable : insert
  read --> Memtable : probe
}

component "Block Cache" as cache { provides ICache }
component "SSTable Files" as sst <<filesystem>>

engine ..> cache : uses
engine --> sst : flush, read
note wal: Replayed on startup before the engine accepts writes.
```

### Hypervisor

```drumlin
diagram component "Type 1 hypervisor"
direction down

package "Guests" as guests {
  component "Linux Guest" as linux { requires IVirtio }
  component "RTOS Guest" as rtos { requires IVirtio }
}

component "Virtual Devices" as vdev {
  provides IVirtio
  requires IDriver
}

package "Hypervisor" as hv {
  component "vCPU Scheduler" as vcpu
  component "Stage-2 MMU" as mmu
  vcpu --> mmu
}

component "Hardware Drivers" as drv { provides IDriver }

guests ..> hv : traps
```

### Container runtime

```drumlin
diagram component "Container runtime"
direction right

component CLI { requires IRuntimeApi }

component Daemon {
  provides IRuntimeApi
  requires IOci
  requires IImages
}

component "Image Store" as images { provides IImages }

component Shim {
  provides IOci
  requires ISyscalls
}

component "Linux Kernel" as kernel <<platform>> { provides ISyscalls }
note Shim: Outlives the daemon so that containers survive restarts.
```

## Embedded

### Motor controller firmware

```drumlin
diagram component "Motor controller firmware"
direction right

package "Application" as app {
  component "Speed Loop" as speed
  component "Fault Monitor" as fault
  fault --> speed : limits
}

package "Drivers" as drivers {
  component PWM { provides IPwm }
  component ADC { provides IAdc }
  component CAN { provides ICan }
}

component "Control Task" as control {
  requires IPwm
  requires IAdc
}

component Telemetry { requires ICan }

speed --> control : setpoint
note control: Runs at 20 kHz in interrupt context.
```

### Sensor node

```drumlin
diagram component "Battery sensor node"
direction down

component "Sampling Task" as sampler {
  requires II2c
  provides ISamples
}

component "I2C Driver" as i2c { provides II2c }

component "Radio Task" as radio {
  requires ISamples
  requires IRadio
}

component "LoRa Driver" as lora { provides IRadio }
component "Power Manager" as power

power ..> sampler : wake
power ..> radio : wake
```

### RTOS layers

```drumlin
diagram component "RTOS layers"
direction down

package "Middleware" as mid {
  component "File System" as fs
  component "TCP/IP" as tcp
}

package "Kernel" as kernel {
  component Tasks
  component Queues
  component Timers
  Tasks --> Queues
  Tasks --> Timers
}

component HAL <<port layer>>

fs --> Tasks
tcp --> Tasks
Queues --> HAL
Timers --> HAL
```

### Bootloader

```drumlin
diagram component "Secure boot chain"
direction right

component "ROM" as rom <<immutable>>
component "First Stage" as fsbl
component "Verifier" as verify { provides IVerify }
component "Second Stage" as ssbl { requires IVerify }
component "Kernel Image" as image

rom --> fsbl : verifies
fsbl --> ssbl : loads
ssbl --> image : verifies, starts
note rom: Holds the root public key hash.
```

### Vehicle gateway

```drumlin
diagram component "Vehicle gateway"
direction right

component "Powertrain ECU" as ptrain { port can requires IFrames }
component "Body ECU" as body { port can requires IFrames }

component Gateway {
  port bus provides IFrames
  port uplink requires IEthernet

  component Router
  component Firewall

  bus --> Firewall
  Firewall --> Router
  Router --> uplink
}

component "Telematics Unit" as tcu { provides IEthernet }
```

## Compilers and tools

### Compiler pipeline

```drumlin
diagram component "Compiler pipeline"
direction down

component Driver { requires ITokens }

package "Front End" as front {
  component Lexer { provides ITokens }
  component Parser
  component "Type Checker" as sema
  Lexer --> Parser : tokens
  Parser --> sema : AST
}

package "Back End" as back {
  component "IR Builder" as ir
  component Optimizer
  component "Code Generator" as codegen { provides ITarget }
  ir --> Optimizer
  Optimizer --> codegen : SSA
}

sema --> ir : typed AST
Driver ..> codegen : invoke
```

### Language server

```drumlin
diagram component "Language server"
direction right

component Editor { requires ILsp }

component Server as server {
  port lsp provides ILsp
  component Dispatcher
  component "Incremental Parser" as parser
  component "Symbol Index" as index
  component Diagnostics

  lsp --> Dispatcher
  Dispatcher --> parser : didChange
  parser --> index : update
  parser --> Diagnostics : errors
}

component "File Watcher" as watcher
watcher --> server : changes
```

### Build system

```drumlin
diagram component "Build system"
direction right

component "Build Files" as files <<input>>
component "Graph Builder" as graph
component Scheduler { requires IExecutor, ICache }
component "Local Executor" as local { provides IExecutor }
component "Content Cache" as cache { provides ICache }

files --> graph : parse
graph --> Scheduler : actions
note cache: Keyed by a hash of each action's inputs.
```

### Debugger

```drumlin
diagram component "Debugger"
direction right

component "Debug UI" as ui { requires IDebug }
component Engine {
  provides IDebug
  requires ISymbols
  requires ITarget
}
component "DWARF Reader" as dwarf { provides ISymbols }
component "ptrace Backend" as ptrace { provides ITarget }
```

### This site's toolchain

```drumlin
diagram component "Documentation build"
direction right

component "Markdown Chapters" as md <<input>>
component mdBook { requires IPreprocessor }
component "mdbook-drumlin" as pre {
  provides IPreprocessor
  requires IRender
}
component "drumlin library" as lib { provides IRender }
component "Static Site" as site <<output>>

md --> mdBook
mdBook --> site : HTML with inline SVG
```

## Distributed services

### Hexagonal service

```drumlin
diagram component "Hexagonal service"
direction right

component "HTTP Adapter" as http { requires IUseCases }

component Core as core {
  port api provides IUseCases
  port repo requires IRepository
  port publish requires IEvents

  component "Use Cases" as cases
  component "Domain Model" as model

  api --> cases
  cases --> model
  cases --> repo
  cases --> publish
}

component "SQL Adapter" as sql { provides IRepository }
component "Kafka Adapter" as kafka { provides IEvents }
```

### Consensus group

```drumlin
diagram component "Replicated state machine"
direction right

component Client { requires IPropose }

component Replica as replica {
  port propose provides IPropose
  port peers requires IRaftRpc

  component "Consensus Module" as raft
  component Log
  component "State Machine" as sm

  propose --> raft
  raft --> Log : append
  Log --> sm : apply committed
  raft --> peers
}

component "Peer Replicas" as others { provides IRaftRpc }
```

### Cache-aside

```drumlin
diagram component "Cache-aside read"
direction right

component Service
component Redis <<cache>>
component Postgres <<database>>

Service --> Redis : 1. get
Service --> Postgres : 2. on miss
Service ..> Redis : 3. set, ttl 60s
```

### API gateway

```drumlin
diagram component "API gateway"
direction right

component "Mobile App" as mobile { requires IPublicApi }
component "Web App" as web { requires IPublicApi }

component Gateway {
  provides IPublicApi
  requires IAuth
  requires ICatalog
  requires ICheckout
}

package "Services" as services {
  component Auth { provides IAuth }
  component Catalog { provides ICatalog }
  component Checkout { provides ICheckout }
}
```

### Event-driven checkout

```drumlin
diagram component "Event-driven checkout"
direction right

component Checkout { requires IBus }
component "Message Bus" as bus { provides IBus }
component Payments
component Inventory
component Shipping

bus --> Payments : OrderPlaced
bus --> Inventory : OrderPlaced
Payments --> Shipping : PaymentCaptured
Inventory --> Shipping : StockReserved
note bus: Delivery is at least once. Handlers are idempotent.
```

## Data platforms

### ETL pipeline

```drumlin
diagram component "Nightly ETL"
direction right

component "Source Databases" as src <<database>>
component Extract
component Transform
component Load
component Warehouse <<database>>
component "Data Quality" as dq

src --> Extract
Extract --> Transform : parquet
Transform --> Load
Load --> Warehouse
Transform ..> dq : assertions
```

### Stream processor

```drumlin
diagram component "Stream processor"
direction right

component Producers { requires ILog }
component "Partitioned Log" as log {
  provides ILog
  provides ISubscribe
}

component Job as job {
  port source requires ISubscribe
  component Windowing
  component Aggregator
  component Sink
  source --> Windowing
  Windowing --> Aggregator
  Aggregator --> Sink
}

component "State Backend" as state <<rocksdb>>
job ..> state : checkpoints
```

### Query engine

```drumlin
diagram component "Query engine"
direction down

component "SQL Parser" as parser
component Planner { requires ICatalog }
component Catalog { provides ICatalog }
component Optimizer
component Executor
component "Storage Layer" as storage

parser --> Planner : AST
Planner --> Optimizer : logical plan
Optimizer --> Executor : physical plan
Executor --> storage : scans
```

### ML serving

```drumlin
diagram component "Model serving"
direction right

component Client { requires IPredict }

package "Serving" as serving {
  component Router { provides IPredict }
  component "Model A" as a
  component "Model B" as b
  Router --> a : 95%
  Router --> b : 5%
}

component "Feature Store" as features <<database>>
component "Metrics Sink" as metrics

serving ..> features : lookup
serving ..> metrics : latency
```

