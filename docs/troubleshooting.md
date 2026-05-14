# Troubleshooting

This guide covers common local development and runtime issues.

## Connection Refused

Symptom:

```text
Connection refused
```

Checks:

```bash
redis-cli -h localhost -p 6379 ping
docker ps
```

Start Redis locally:

```bash
docker run --rm -p 6379:6379 redis:7
```

If Redis is listening on another host or port, update the connection string:

```rust
let config = ConnectionConfig::new("redis://127.0.0.1:6380");
```

## Authentication Failed

Use `with_password` when Redis requires authentication:

```rust
let config = ConnectionConfig::new("redis://localhost:6379")
    .with_password("secret");
```

If Redis does not require a password, do not send one.

## Integration Tests Fail

Most integration tests require Redis on `localhost:6379`.

```bash
docker run --rm -p 6379:6379 redis:7
cargo test --workspace --all-targets
```

If you use another Redis URL:

```bash
REDIS_URL=redis://localhost:6380 cargo test --workspace --all-targets
```

## TTL Looks Unexpected

`ttl` returns Redis sentinel values:

- `Some(-2)`: key does not exist
- `Some(-1)`: key exists without expiration
- `Some(n)`: key expires in `n` seconds

This mirrors Redis behavior instead of converting negative values to `None`.

## Pipeline Contains Errors

Redis server errors inside a pipeline are returned as `RespValue::Error` entries.
This lets later commands still produce results. Connection and protocol errors
still fail the whole pipeline.

## Transaction Is Discarded

When a watched key changes before `EXEC`, Redis returns a null transaction
result. `redis-oxide` exposes that as an empty result vector.

Use `WATCH` only for keys that must be protected by optimistic locking.

## Scripts Return NOSCRIPT

Direct `evalsha` requires the script to be loaded first:

```rust
let sha = client.script_load("return 'ok'").await?;
let value: String = client.evalsha(&sha, vec![], vec![]).await?;
```

`Script::execute` handles `NOSCRIPT` by falling back to `EVAL`.

## Cluster Redirects

`MOVED` and `ASK` redirects indicate cluster slot ownership changed or the
client has stale topology information. The client parses redirect errors and
retries according to `max_redirects`.

Check cluster state:

```bash
redis-cli cluster info
redis-cli cluster nodes
```

## Slow Commands

Start with Redis diagnostics:

```bash
redis-cli slowlog get 10
redis-cli info commandstats
```

Then review the application command pattern:

- replace repeated independent commands with a pipeline
- use the correct Redis data structure
- avoid broad key scans
- check network latency to Redis
