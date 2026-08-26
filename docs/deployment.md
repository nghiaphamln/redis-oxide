# Deployment and topologies

## Standalone Redis

Use a `redis://` URL. The default pool is one bounded multiplexed connection,
which is suitable for most request/response workloads.

```rust
use redis_oxide::{Client, ConnectionConfig};
use std::time::Duration;

let config = ConnectionConfig::new("redis://cache.internal:6379")
    .with_connect_timeout(Duration::from_secs(5))
    .with_operation_timeout(Duration::from_secs(30));
let client = Client::connect(config).await?;
```

Use `PoolStrategy::Pool` when independent sockets are required, such as for
long-running or stateful workloads. Transactions and subscribers already use
dedicated connections.

```rust
use redis_oxide::{ConnectionConfig, PoolConfig, PoolStrategy};

let config = ConnectionConfig::new("redis://cache.internal:6379")
    .with_pool_config(PoolConfig {
        strategy: PoolStrategy::Pool,
        max_size: 16,
        min_idle: 2,
        ..Default::default()
    });
```

## Redis Cluster

Pass one or more seed nodes and force Cluster mode when the topology is known.
The client bootstraps slots with `CLUSTER SLOTS` and handles `MOVED` and `ASK`
redirects.

```rust
use redis_oxide::{ConnectionConfig, TopologyMode};

let config = ConnectionConfig::new(
    "redis://cluster-1:7000,cluster-2:7001,cluster-3:7002",
)
.with_topology_mode(TopologyMode::Cluster);
```

Commands in one transaction and multi-stream reads must use keys in one hash
slot. Use Redis hash tags when related keys must share a slot.

## Redis Sentinel

Create Sentinel configuration with the monitored master name and each Sentinel
endpoint. Passwords for Sentinel and Redis are configured separately.

```rust
use redis_oxide::{ConnectionConfig, SentinelConfig};

let sentinels = SentinelConfig::new("mymaster")
    .add_sentinel("sentinel-1:26379")?
    .add_sentinel("sentinel-2:26379")?;
let config = ConnectionConfig::new_with_sentinel(sentinels);
```

The client discovers the current master before opening its command pool and
refreshes the pool after a connection-level failure. It never replays a command
whose server-side outcome is unknown.

## TLS

TLS is not implemented in 0.3. `rediss://` URLs fail validation instead of
opening an unencrypted connection unexpectedly.
