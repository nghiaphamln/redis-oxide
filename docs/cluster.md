# Cluster Mode

redis-oxide provides built-in support for Redis Cluster with automatic redirect handling.

## Connecting to a Cluster

```rust
use redis_oxide::{Client, ConnectionConfig};

let config = ConnectionConfig::new("redis://localhost:6379");
let client = Client::connect(config).await?;
```

redis-oxide automatically detects cluster topology and handles connections.

## Automatic Redirect Handling

The client automatically handles MOVED and ASK redirects:

```rust
// Commands are automatically routed to the correct node
let value: Option<String> = client.get("key").await?;

// If the key is on another node, the client:
// 1. Receives the redirect error
// 2. Updates cluster topology
// 3. Retries on the correct node
```

## Configuration

```rust
use redis_oxide::ConnectionConfig;

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_max_redirects(5);  // Maximum redirects to follow
```

## Cluster Topology

The client maintains an internal topology map:

```rust
// Get client topology type
let topology = client.topology_type();
// Returns: TopologyType::Cluster or TopologyType::Standalone
```

## Key Slot Calculation

Keys are automatically hashed to determine the correct node:

```rust
// Keys with the same hash slot can be grouped for transactions
// Hash tags: {tag} ensures keys in the same slot
client.set("{user:1000}:name", "Alice").await?;
client.set("{user:1000}:email", "alice@example.com").await?;
```

## Troubleshooting

### Common Issues

**Connection Refused:**
```
Ensure all cluster nodes are accessible from the client.
Check firewall rules and network connectivity.
```

**Too Many Redirects:**
```
This may indicate:
- Cluster is rebalancing
- Node is temporarily unavailable
- Network partition

Try increasing max_redirects or check cluster health.
```

### Verify Cluster Health

```bash
# Check cluster nodes
redis-cli cluster nodes

# Check cluster state
redis-cli cluster info
```
