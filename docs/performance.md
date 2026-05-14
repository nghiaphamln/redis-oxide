# Performance

Performance depends mostly on connection strategy, command shape, data model,
and Redis deployment topology.

## Connection Strategy

Use the multiplexed strategy for most applications. It avoids managing many
connections and works well for common async request patterns.

Use the pool strategy when:

- commands can block for longer periods
- independent connections are required
- high concurrency benefits from multiple Redis sockets

```rust
use redis_oxide::{ConnectionConfig, PoolConfig, PoolStrategy};
use std::time::Duration;

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_pool_config(PoolConfig {
        strategy: PoolStrategy::Pool,
        max_size: 16,
        min_idle: 4,
        connection_timeout: Duration::from_secs(5),
    });
```

## Pipelining

Use pipelines to reduce round trips when commands are independent:

```rust
let mut pipeline = client.pipeline();

for i in 0..100 {
    pipeline.set(format!("key:{i}"), format!("value:{i}"));
}

let results = pipeline.execute().await?;
```

Pipeline results preserve command order. Redis server errors are returned in the
result vector as `RespValue::Error`.

## Data Modeling

Prefer Redis data structures that match access patterns:

- use hashes for grouped object fields
- use lists for ordered queues
- use sets for membership checks
- use sorted sets for ranking and time-window queries
- use streams for append-only event processing

Avoid broad key scans in production paths. Prefer explicit indexes, sets, sorted
sets, or stream consumers.

## Scripts

Use Lua scripts when a sequence of operations must be atomic and server-side.
`Script::execute` uses `EVALSHA` and falls back to `EVAL` when the script is not
cached.

## Benchmarks

The repository includes Criterion benchmarks:

```bash
cargo bench
```

Benchmarks are useful for comparing local changes, but Redis server settings,
network latency, and hardware dominate real production results.

## Operational Checks

For Redis-side diagnosis:

```bash
redis-cli slowlog get 10
redis-cli info commandstats
redis-cli client list
redis-cli monitor
```

Use `monitor` only in development or short diagnostic windows.
