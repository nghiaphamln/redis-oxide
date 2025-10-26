# Architecture Guide

This document describes the internal architecture of redis-oxide.

## Overview

redis-oxide is designed with the following principles:

- **High Performance**: Minimized allocations, efficient protocol handling
- **Type Safety**: Compile-time safety through Rust's type system
- **Async-First**: Built on Tokio for efficient async operations
- **Flexibility**: Multiple connection strategies and configuration options
- **Reliability**: Robust error handling and automatic recovery

## Core Components

### 1. Client Layer

**Location**: `src/client.rs`

The top-level API that users interact with.

```
┌─────────────────────┐
│  redis_oxide::Client│
├─────────────────────┤
│ • Commands API      │
│ • Pipeline support  │
│ • Transaction API   │
│ • Pub/Sub support   │
└──────────┬──────────┘
           │
           ▼
```

**Key Methods:**
- `connect()` - Establish connection
- `set()`, `get()` - Basic operations
- `pipeline()` - Create pipeline
- `transaction()` - Create transaction
- `get_pubsub()` - Pub/Sub support

### 2. Connection Layer

**Location**: `src/connection.rs`

Manages the TCP connection to Redis.

```
┌──────────────────────────┐
│  Connection              │
├──────────────────────────┤
│ • TCP socket management  │
│ • Read/Write buffers     │
│ • RESP encoding/decoding │
│ • Error handling         │
└──────────┬───────────────┘
           │
           ▼
┌──────────────────────────┐
│  TcpStream               │
└──────────────────────────┘
```

### 3. Connection Pool

**Location**: `src/pool.rs` and `src/pool_optimized.rs`

Manages multiple connections for concurrent operations.

```
┌────────────────────────────┐
│  ConnectionPool            │
├────────────────────────────┤
│ • Connection management    │
│ • Queue management         │
│ • Health checks            │
└────────┬───────────────────┘
         │
         ├─ Connection 1
         ├─ Connection 2
         ├─ Connection 3
         └─ ...
```

**Strategies:**
- **Pool**: Traditional connection pooling (deadpool-based)
- **Multiplexed**: Single connection, multiple requests

### 4. Protocol Layer

**Location**: `src/protocol/`

Handles RESP (Redis Serialization Protocol) encoding/decoding.

```
┌────────────────────────────┐
│  Protocol                  │
├────────────────────────────┤
│ • RESP2 support            │
│ • RESP3 support            │
│ • Optimized parsing        │
│ • Type conversions         │
└────────┬───────────────────┘
         │
         ├─ resp2.rs
         ├─ resp2_optimized.rs
         ├─ resp3.rs
         └─ mod.rs
```

**Protocols:**
- **RESP2**: Standard Redis protocol (default)
- **RESP3**: Newer protocol with enhanced features

### 5. Command Builder

**Location**: `src/commands/`

Provides type-safe command builders.

```
┌──────────────────────────┐
│  Commands                │
├──────────────────────────┤
│ • String operations      │
│ • Hash operations        │
│ • List operations        │
│ • Set operations         │
│ • Sorted set operations  │
│ • Key operations         │
└──────────────────────────┘
```

### 6. Cluster Support

**Location**: `src/cluster.rs`

Handles Redis Cluster operations.

```
┌──────────────────────────────┐
│  ClusterManager              │
├──────────────────────────────┤
│ • Slot management            │
│ • Topology discovery         │
│ • MOVED/ASK handling         │
│ • Request routing            │
└──────────┬───────────────────┘
           │
           ▼
    ┌──────────────────┐
    │ Cluster Nodes    │
    ├──────────────────┤
    │ • Node 1 (slots) │
    │ • Node 2 (slots) │
    │ • Node 3 (slots) │
    └──────────────────┘
```

### 7. Advanced Features

**Pub/Sub** (`src/pubsub.rs`)
- Channel subscription
- Pattern matching
- Message delivery

**Streams** (`src/streams.rs`)
- Stream operations
- Consumer groups
- Message acknowledgment

**Scripting** (`src/script.rs`)
- Lua script execution
- EVALSHA caching
- Script management

**Transactions** (`src/transaction.rs`)
- MULTI/EXEC support
- WATCH mechanism
- Optimistic locking

**Sentinel** (`src/sentinel.rs`)
- Sentinel discovery
- Master/replica failover
- Monitoring

## Data Flow

### Basic Request Flow

```
┌──────────────┐
│ Application  │
└──────┬───────┘
       │ client.set("key", "value")
       ▼
┌──────────────────┐
│ Command Builder  │
└──────┬───────────┘
       │ "SET key value"
       ▼
┌──────────────────┐
│ RESP Encoder     │
└──────┬───────────┘
       │ "*3\r\n$3\r\nSET\r\n..."
       ▼
┌──────────────────┐
│ Connection       │
└──────┬───────────┘
       │ (TCP write)
       ▼
┌──────────────────┐
│ Redis Server     │
└──────┬───────────┘
       │ "+OK"
       ▼
┌──────────────────┐
│ Connection       │
└──────┬───────────┘
       │ (TCP read)
       ▼
┌──────────────────┐
│ RESP Decoder     │
└──────┬───────────┘
       │ Result::Ok("OK")
       ▼
┌──────────────────┐
│ Application      │
└──────────────────┘
```

### Cluster Request Flow

```
┌──────────────┐
│ Application  │
└──────┬───────┘
       │ client.set("key", "value")
       ▼
┌──────────────────┐
│ Command Builder  │
└──────┬───────────┘
       ▼
┌──────────────────┐
│ Cluster Manager  │
├──────────────────┤
│ Calculate slot:  │
│ CRC16("key")%16k │
└──────┬───────────┘
       │ Identify target node
       ▼
┌──────────────────┐
│ Connection Pool  │
└──────┬───────────┘
       │ Get connection to node
       ▼
┌──────────────────┐
│ RESP Encoder     │
└──────┬───────────┘
       ▼
┌──────────────────┐
│ Cluster Node     │
└──────┬───────────┘
       │ Result or MOVED
       ▼
┌──────────────────┐
│ Handle Redirect  │
│ (if needed)      │
└──────┬───────────┘
       ▼
┌──────────────────┐
│ Application      │
└──────────────────┘
```

## Error Handling

**Location**: `src/core/error.rs`

All errors are represented by `RedisError`:

```rust
pub enum RedisError {
    ConnectionError(String),
    TimeoutError,
    ProtocolError(String),
    ClusterError(String),
    AuthenticationError(String),
    IOError(io::Error),
    // ... other variants
}
```

**Error Handling Strategy:**
1. Errors are propagated up the call stack
2. Connection errors trigger reconnection
3. Cluster errors may trigger topology refresh
4. All errors are descriptive for debugging

## Type System

**Location**: `src/core/types.rs` and `src/core/value.rs`

Redis values are represented as:

```rust
pub enum RedisValue {
    Null,
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Vec<u8>),
    Array(Vec<RedisValue>),
    // RESP3 types
    Boolean(bool),
    Double(f64),
    BigNumber(String),
}
```

This allows type-safe operations while maintaining Redis compatibility.

## Configuration

**Location**: `src/core/config.rs`

Key configuration options:

```rust
pub struct ConnectionConfig {
    pub url: String,
    pub strategy: ConnectionStrategy,
    pub pool_config: PoolConfig,
    pub connect_timeout: Duration,
    pub response_timeout: Duration,
    pub reconnect_attempts: u32,
    pub password: Option<String>,
    pub database: u32,
}
```

## Performance Optimizations

### 1. Protocol Optimizations

- **Buffered I/O**: Uses BufReader/BufWriter for efficient I/O
- **Zero-Copy Parsing**: RESP3 parsing avoids unnecessary copies
- **Pre-allocated Buffers**: Reuses buffers to reduce allocations

### 2. Connection Optimizations

- **Multiplexing**: Single connection handles multiple requests
- **Connection Pooling**: Reuses connections to reduce overhead
- **Lazy Initialization**: Connections created on demand

### 3. Memory Optimizations

- **Object Pooling**: Reuses allocated objects
- **Small String Optimization**: Avoids heap allocation for small strings
- **Buffer Recycling**: Returns buffers to a pool

## Concurrency Model

```
┌────────────────────────────────┐
│ Tokio Runtime                  │
│ ┌──────────────────────────────┤
│ │ Task 1: Handle Request 1     │
│ │ Task 2: Handle Request 2     │
│ │ Task 3: Listen for Messages  │
│ │ ...                          │
│ └──────────────────────────────┤
│ │ Multiplexed Connection       │
│ │ (Shared across tasks)        │
│ └──────────────────────────────┤
└────────────────────────────────┘
```

**Concurrency Features:**
- All operations are async
- Safe concurrent access via Arc/Mutex
- No blocking operations
- Efficient task scheduling

## Module Structure

```
redis-oxide/
├── src/
│   ├── lib.rs              # Crate root
│   ├── client.rs           # Main Client API
│   ├── connection.rs       # Connection management
│   ├── cluster.rs          # Cluster support
│   ├── pipeline.rs         # Pipeline implementation
│   ├── transaction.rs      # Transaction support
│   ├── pubsub.rs          # Pub/Sub support
│   ├── streams.rs         # Redis Streams
│   ├── script.rs          # Lua scripting
│   ├── sentinel.rs        # Sentinel support
│   ├── pool.rs            # Connection pooling
│   ├── pool_optimized.rs  # Optimized pooling
│   ├── core/
│   │   ├── config.rs      # Configuration
│   │   ├── error.rs       # Error types
│   │   ├── types.rs       # Type definitions
│   │   ├── value.rs       # Redis value
│   │   └── mod.rs         # Core module
│   ├── protocol/
│   │   ├── resp2.rs       # RESP2 protocol
│   │   ├── resp2_optimized.rs
│   │   ├── resp3.rs       # RESP3 protocol
│   │   └── mod.rs         # Protocol module
│   ├── commands/
│   │   ├── mod.rs         # Commands module
│   │   ├── string.rs      # String commands
│   │   ├── hash.rs        # Hash commands
│   │   ├── list.rs        # List commands
│   │   ├── set.rs         # Set commands
│   │   ├── sorted_set.rs  # Sorted set commands
│   │   └── optimized.rs   # Optimized commands
│   └── mod.rs             # Module declarations
├── examples/              # Examples
├── tests/                 # Integration tests
└── benches/              # Benchmarks
```

## Design Decisions

### 1. Why Async?
- Better resource utilization
- Natural fit for I/O-bound operations
- Allows handling thousands of concurrent connections

### 2. Why Type-Safe Commands?
- Compile-time validation
- Prevents common errors
- Better IDE support

### 3. Why Cluster Support?
- Redis Cluster is widely used in production
- Transparent redirection handling
- Automatic topology discovery

### 4. Why Multiple Connection Strategies?
- Different use cases have different requirements
- Multiplexing for low-concurrency, high-throughput
- Pooling for high-concurrency scenarios

### 5. Why RESP3 Support?
- Better performance for newer use cases
- Support for newer Redis features
- Backward compatibility with RESP2

## Testing Strategy

```
Unit Tests (src/lib.rs)
├── Protocol tests
├── Command tests
├── Type conversion tests
└── Error handling tests

Integration Tests (tests/)
├── Basic operations
├── Cluster operations
├── Pub/Sub tests
├── Stream tests
├── Transaction tests
└── Full pipeline tests
```

## Future Improvements

1. **Performance**
   - Further optimize hot paths
   - Reduce allocations in protocol handling
   - Add benchmarking suite

2. **Features**
   - Complete command coverage
   - Advanced stream operations
   - Better monitoring/metrics

3. **Reliability**
   - Enhanced error recovery
   - Better cluster handling
   - Improved sentinel support
