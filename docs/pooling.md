# Connection Pooling

redis-oxide supports multiple connection pooling strategies optimized for different use cases.

## Pool Strategies

### Multiplexed Pool (Default)

A single connection shared across all async tasks using multiplexing. Best for most use cases.

```rust
use redis_oxide::{Client, ConnectionConfig, PoolStrategy};

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_strategy(PoolStrategy::Multiplexed);  // Default

let client = Client::connect(config).await?;
```

**Benefits:**
- Low memory footprint (one connection)
- No connection overhead
- Ideal for most applications

**Considerations:**
- Single connection point of failure
- Commands execute sequentially

### Connection Pool

Multiple connections managed in a pool. Best for high-throughput scenarios.

```rust
use redis_oxide::{Client, ConnectionConfig, PoolStrategy, PoolConfig};

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_strategy(PoolStrategy::Pool)
    .with_pool_config(
        PoolConfig::new()
            .max_size(50)
            .min_idle(10)
    );

let client = Client::connect(config).await?;
```

**Benefits:**
- Parallel command execution
- Higher throughput under load
- Connection fault tolerance

**Considerations:**
- Higher memory usage
- Requires more Redis connections

## Pool Statistics

```rust
use redis_oxide::PoolStats;

// Get pool statistics
let stats = client.pool_stats().await?;

println!("Active connections: {}", stats.active_connections);
println!("Pending requests: {}", stats.pending_requests);
println!("Total requests: {}", stats.total_requests);
println!("Failed requests: {}", stats.failed_requests);
println!("Avg response time: {:.2}ms", stats.average_response_time_ms);
```

## Pool Configuration

```rust
use redis_oxide::PoolConfig;

let config = PoolConfig::new()
    .max_size(100)      // Maximum connections in pool
    .min_idle(20)       // Minimum idle connections to maintain
    .validation_interval(30);  // Seconds between health checks
```

## Health Checks

Both pool strategies perform periodic health checks:

```rust
// Multiplexed pool health check
// - PING command every 30 seconds
// - Automatic reconnection on failure
```

## Best Practices

1. **Default to Multiplexed** for most applications
2. **Use Connection Pool** when:
   - High throughput requirements (>10K ops/sec)
   - Parallel command execution needed
   - Fault tolerance is critical
3. **Size pool appropriately**: max_size should be 2-3x expected concurrent operations
