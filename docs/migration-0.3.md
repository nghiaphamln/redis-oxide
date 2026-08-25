# Migrating to 0.3

`0.3.0-alpha.1` is a breaking reliability release. It removes unfinished
optimisation APIs and makes previously ignored connection settings effective.

## Removed APIs

Remove imports from these modules and use the supported client, pool, and
protocol codecs instead:

- `pool_optimized`
- `commands::optimized`
- `protocol::resp2_optimized`
- `ProtocolNegotiator`, `ProtocolNegotiation`, and `ProtocolConnection`

The old comparison demo and optimisation benchmark were removed with those
implementations.

## RESP3 values

`Resp3Value::BlobString` and `Resp3Value::BlobError` now contain `bytes::Bytes`
instead of `String`, preserving binary Redis values. Construct them with
`"value".into()` or `Bytes::from_static`, and call `as_string()` only when UTF-8
is required.

`Resp3Value::Map`, `Set`, and `Attribute::attrs` now use ordered vectors. This
preserves arbitrary RESP3 key types and the exact values sent by Redis; adapt
string-keyed collections like this:

```rust
let value = Resp3Value::Map(vec![
    (
        Resp3Value::SimpleString("key".into()),
        Resp3Value::Number(1),
    ),
]);
```

## Connection and Sentinel configuration

- `ConnectionConfig::parse_endpoints()` is fallible. It accepts only
  `redis://host[:port]` seed lists, including bracketed IPv6 addresses.
- `rediss://`, URI userinfo, paths, and query strings now fail explicitly;
  configure password and database with the existing builders.
- `SentinelConfig::add_sentinel(...)` now returns `RedisResult<Self>` so invalid
  endpoints cannot be silently discarded. Chain it with `?`.
- `ConnectionConfig::new_with_sentinel(...)` now works with `Client::connect`.
  `SentinelConfig::password` authenticates to Sentinel and
  `ConnectionConfig::password` authenticates to the discovered Redis master.

## Behavioral fixes

Transactions and Pub/Sub obtain dedicated connections. Pipelines use one
contiguous batch per target node, and pooled connections are discarded after a
transport, timeout, or protocol failure. Automatic reconnect never replays a
command whose server-side outcome is unknown.
