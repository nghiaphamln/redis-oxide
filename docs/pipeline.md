# Pipeline

Pipelining allows you to send multiple commands to Redis without waiting for each response, improving throughput.

## Basic Pipeline

```rust
use redis_oxide::Pipeline;

let mut pipeline = client.pipeline();

// Queue multiple commands
pipeline.set("key1", "value1");
pipeline.set("key2", "value2");
pipeline.get("key1");
pipeline.get("key2");
pipeline.incr("counter", 1);

// Execute all commands
let results: Vec<RespValue> = pipeline.execute().await?;
```

## Pipeline Response Parsing

```rust
use redis_oxide::{Client, Pipeline};
use redis_oxide::core::value::RespValue;

let mut pipeline = client.pipeline();
pipeline.set("key", "value");
pipeline.get("key");
pipeline.incr("counter", 1);

let results = pipeline.execute().await?;

// Results are in RESP format
let set_result: () = results[0].as_simple_string()?;
let get_result: Option<String> = results[1].as_bulk_string().map(|s| s.to_string())?;
let incr_result: i64 = results[2].as_int()?;
```

## Typed Pipeline

For better type safety, use command types:

```rust
use redis_oxide::commands::{SetCommand, GetCommand, IncrCommand};

let mut pipeline = client.pipeline();

// Using command builders
pipeline.command(SetCommand::new("key", "value"));
pipeline.command(GetCommand::new("key"));
pipeline.command(IncrCommand::new("counter", 1));

let results = pipeline.execute().await?;
```

## Empty Pipeline

```rust
// Execute pipeline with no commands
let results: Vec<RespValue> = client.pipeline().execute().await?;
// Returns empty vector
```

## Pool Strategies with Pipeline

```rust
use redis_oxide::{Client, ConnectionConfig, PoolStrategy};

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_strategy(PoolStrategy::Pool);  // Best for batch operations

let client = Client::connect(config).await?;
```

## Best Practices

1. **Group related commands** in a single pipeline
2. **Avoid very large pipelines** (100+ commands) to prevent timeouts
3. **Use Pool strategy** for maximum throughput
4. **Consider transaction** (MULTI/EXEC) if atomicity is needed

## Performance Comparison

| Approach | RTT | Commands | Total Time |
|----------|-----|----------|------------|
| Sequential | 1ms | 10 | ~10ms |
| Pipeline | 1ms | 10 | ~1ms |

Pipelining eliminates round-trip overhead for subsequent commands.
