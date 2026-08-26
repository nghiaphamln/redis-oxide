# Troubleshooting

## Connection refused

Confirm Redis is reachable before changing client code:

```bash
redis-cli -h localhost -p 6379 ping
docker ps
```

For a local server:

```bash
docker run --rm -p 6379:6379 redis:7
```

## Authentication or configuration errors

Use `with_password` only when Redis requires authentication. Verify that every
configured timeout is nonzero and that `min_idle` does not exceed `max_size`.

TLS is unsupported in 0.3. Replace `rediss://` with an explicitly terminated
TLS proxy or use a supported `redis://` endpoint.

## Cluster redirects

`MOVED` and `ASK` indicate slot ownership changed. The client parses both and
refreshes its routing where appropriate. Check Redis when redirects persist:

```bash
redis-cli cluster info
redis-cli cluster nodes
```

## Integration tests

Most tests use Redis at `localhost:6379` by default. Set `REDIS_URL` to use a
different server:

```bash
REDIS_URL=redis://localhost:6380 cargo test --workspace --all-targets
```

Cluster and Sentinel topology tests are intentionally ignored locally and run
from Docker fixtures in CI.

## Slow commands

Use Redis diagnostics first:

```bash
redis-cli slowlog get 10
redis-cli info commandstats
```

Then inspect application command shape, data modeling, and network distance.
