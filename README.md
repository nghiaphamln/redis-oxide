# redis-oxide

A high-performance Redis client library for Rust.

## Table of Contents

### Getting Started
- [Quick Start](docs/quick_start.md)
- [Connection Configuration](docs/configuration.md)

### Core Features
- [Commands Overview](docs/commands.md)
- [Connection Pooling](docs/pooling.md)
- [Cluster Mode](docs/cluster.md)

### Advanced Features
- [Pipeline](docs/pipeline.md)
- [Transactions](docs/transactions.md)
- [Pub/Sub](docs/pubsub.md)
- [Scripts](docs/scripts.md)
- [Streams](docs/streams.md)

### Reference
- [API Documentation](https://docs.rs/redis-oxide)
- [GitHub Repository](https://github.com/nghiaphamln/redis-oxide)

## Performance

redis-oxide is optimized for high throughput and low latency. The following benchmarks were measured on a Macbook M1 Pro running Redis in Docker locally.

### Protocol Encoding

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Encode Simple String | 47 ns | 21.3M ops/sec |
| Encode Bulk String | 52 ns | 19.2M ops/sec |
| Encode Array (3 items) | 101 ns | 9.9M ops/sec |
| Encode Command (SET) | 73 ns | 13.7M ops/sec |

### Protocol Decoding

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Decode Simple String | 38 ns | 26.3M ops/sec |
| Decode Bulk String | 61 ns | 16.4M ops/sec |
| Decode Array | 240 ns | 4.2M ops/sec |

### Key Optimizations

- **Buffer Pre-sizing**: Encoder allocates buffers based on estimated command size
- **Zero-copy Parsing**: Bulk strings are parsed directly into `Bytes`
- **Buffer Reuse**: Encoder instance can be reused across operations
- **Minimal Allocations**: Critical path avoids unnecessary memory allocations

## Quick Start

### Add to Your Project

```toml
[dependencies]
redis-oxide = "0.2"
```

### Basic Example

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;

    // Basic operations
    client.set("key", "value").await?;
    let value: Option<String> = client.get("key").await?;
    println!("Value: {:?}", value);

    // Increment counter
    client.incr("counter", 1).await?;
    let count: i64 = client.incr("counter", 1).await?;
    println!("Counter: {}", count);

    Ok(())
}
```

### Connection Pool Example

```rust
use redis_oxide::{Client, ConnectionConfig, PoolStrategy};

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_strategy(PoolStrategy::Pool)
    .with_max_pool_size(20)
    .with_min_idle(5);

let client = Client::connect(config).await?;
```

### Pipeline Example

```rust
use redis_oxide::Pipeline;

let mut pipeline = client.pipeline();
pipeline.set("key1", "value1");
pipeline.set("key2", "value2");
pipeline.get("key1");
pipeline.get("key2");

let results = pipeline.execute().await?;
```

## Features

- **Async/Tokio**: Built on Tokio for asynchronous operation
- **Connection Pooling**: Multiplexed or traditional pool strategies
- **Cluster Support**: Automatic MOVED/ASK redirect handling
- **Pipeline**: Batch multiple commands for improved throughput
- **Transactions**: WATCH/MULTI/EXEC support
- **Pub/Sub**: Subscribe to channels and patterns
- **Lua Scripts**: EVAL and EVALSHA for server-side scripting
- **Streams**: Full XADD, XREAD, XGROUP support

## Minimum Rust Version

Requires Rust 1.82 or later.

## License

MIT License
