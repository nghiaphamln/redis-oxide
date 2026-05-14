# Architecture

`redis-oxide` is structured around a typed client API, command builders, Redis
protocol encoders and decoders, and connection abstractions that support
standalone Redis and Redis Cluster.

## Main Layers

### Client

`Client` is the public entrypoint. It owns the connection configuration,
detected topology, standalone pool, and cluster pools. Public methods build
command objects or RESP argument lists, route the command, and convert Redis
responses into Rust types.

### Commands

Command modules under `redis-oxide/src/commands/` define command builders and
response parsers for Redis data structures. Commands expose:

- command name
- encoded arguments
- keys used for cluster routing
- typed response parsing

Pipelines and transactions reuse these command builders through their own
command traits.

### Protocol

RESP2 and RESP3 implementations live under `redis-oxide/src/protocol/`.
Command encoding treats Redis command arguments as bulk strings, while normal
RESP value encoding still preserves standalone RESP integer, array, error, and
bulk string forms.

### Connections and Pools

The connection layer owns TCP I/O and command execution. Pooling supports:

- multiplexed strategy: one shared connection with serialized command handling
- pool strategy: multiple Redis connections guarded by a semaphore

Cluster mode keeps per-node pools and routes commands by hash slot when a key is
available.

### Advanced Features

The crate includes dedicated modules for:

- Pub/Sub with `Subscriber` and `Publisher`
- Lua scripting with `Script` and `ScriptManager`
- Streams helpers and response parsing
- Sentinel discovery and failover configuration
- Transactions and pipelines

## Request Flow

1. User calls a `Client` method.
2. The method builds a command name and arguments.
3. The client chooses a standalone pool or cluster pool.
4. The connection encodes the command as RESP.
5. Redis returns a RESP response.
6. The command parser converts the response into the public return type.

## Error Model

`RedisError` separates transport, protocol, server, authentication, cluster,
Sentinel, pool, timeout, and type conversion failures.

Pipeline execution keeps Redis server errors as per-command `RespValue::Error`
entries. Connection-level failures still fail the entire pipeline. Lua
`Script::execute` falls back from `EVALSHA` to `EVAL` on `NOSCRIPT`.

## Testing Strategy

The test suite covers:

- protocol encoding and decoding
- standalone integration behavior against Redis
- data types
- scripts
- streams
- pipelines and transactions
- RESP3 parsing and negotiation helpers

Integration tests expect Redis on `localhost:6379`, or a compatible `REDIS_URL`
environment variable.
