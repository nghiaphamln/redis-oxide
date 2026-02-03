# Connection Configuration

This guide covers all configuration options for redis-oxide.

## Basic Configuration

```rust
use redis_oxide::ConnectionConfig;

let config = ConnectionConfig::new("redis://localhost:6379");
```

## Connection Options

### URL Format

redis-oxide supports standard Redis connection URLs:

```rust
// Standalone Redis
let config = ConnectionConfig::new("redis://localhost:6379");

// With password
let config = ConnectionConfig::new("redis://:password@localhost:6379");

// With database number
let config = ConnectionConfig::new("redis://localhost:6379/0");
```

### Programmatic Configuration

```rust
use redis_oxide::ConnectionConfig;
use std::time::Duration;

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_password("your_password")
    .with_database(1)
    .with_connect_timeout(Duration::from_secs(5))
    .with_response_timeout(Duration::from_secs(10));
```

## Pool Configuration

### Pool Strategy

redis-oxide supports two pool strategies:

```rust
use redis_oxide::{ConnectionConfig, PoolStrategy};

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_strategy(PoolStrategy::Multiplexed);  // Default: single connection

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_strategy(PoolStrategy::Pool);         // Connection pool
```

### Pool Size

```rust
use redis_oxide::{ConnectionConfig, PoolConfig};

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_pool_config(
        PoolConfig::new()
            .max_size(50)        // Maximum connections
            .min_idle(10)        // Minimum idle connections
    );
```

## Timeout Configuration

```rust
use redis_oxide::ConnectionConfig;
use std::time::Duration;

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_connect_timeout(Duration::from_secs(5))
    .with_response_timeout(Duration::from_secs(30));
```

## Complete Example

```rust
use redis_oxide::{Client, ConnectionConfig, PoolStrategy, PoolConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379")
        .with_password("secret")
        .with_database(0)
        .with_strategy(PoolStrategy::Pool)
        .with_pool_config(
            PoolConfig::new()
                .max_size(20)
                .min_idle(5)
        )
        .with_connect_timeout(Duration::from_secs(5))
        .with_response_timeout(Duration::from_secs(10));

    let client = Client::connect(config).await?;
    Ok(())
}
```

## Configuration Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `password` | `Option<String>` | `None` | Redis authentication password |
| `database` | `u8` | `0` | Database number to connect to |
| `strategy` | `PoolStrategy` | `Multiplexed` | Connection pooling strategy |
| `max_size` | `usize` | `50` | Maximum pool size |
| `min_idle` | `usize` | `5` | Minimum idle connections |
| `connect_timeout` | `Duration` | `5s` | Connection timeout |
| `response_timeout` | `Duration` | `30s` | Response timeout |
