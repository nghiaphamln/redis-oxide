# Streams

Redis Streams provide a log-like data structure for handling high-throughput message passing.

## Adding to Streams

```rust
use redis_oxide::streams::{StreamEntry, StreamRange};

let entry = StreamEntry::new()
    .field("event", "user_login")
    .field("user_id", "12345")
    .field("timestamp", "2024-01-15T10:30:00Z");

let id = client.xadd("mystream", "*", vec![entry]).await?;
println!("Added entry with ID: {}", id);

// Add multiple entries
let entries = vec![
    StreamEntry::new().field("event", "event1").field("data", "value1"),
    StreamEntry::new().field("event", "event2").field("data", "value2"),
];

for entry in entries {
    let _ = client.xadd("mystream", "*", vec![entry]).await?;
}
```

## Reading from Streams

```rust
use redis_oxide::streams::StreamRange;

// Read all entries from a stream
let entries = client.xrange("mystream", "-", "+").await?;
for entry in &entries {
    println!("ID: {}, Fields: {:?}", entry.id, entry.fields);
}

// Read with range
let entries = client
    .xrange("mystream", "0-0", "100-0")
    .await?;

// Read only new entries (since last read)
let entries = client.xread("mystream", "last-id").await?;
```

## Reading with Blocking

```rust
use std::time::Duration;

// Block for up to 5 seconds waiting for new entries
let entries = client
    .xread_with_timeout("mystream", "0-0", Duration::from_secs(5))
    .await?;

if entries.is_empty() {
    println!("No new entries within timeout");
} else {
    for entry in entries {
        println!("Received: {:?}", entry);
    }
}
```

## Consumer Groups

### Create Consumer Group

```rust
// Create a consumer group for a stream
client.xgroup_create("mystream", "mygroup", "0-0").await?;
```

### Read from Consumer Group

```rust
use redis_oxide::streams::StreamGroupRange;

let entries = client
    .xgroup_read("mystream", "mygroup", "consumer1", "+", 10)
    .await?;
```

### Acknowledge Messages

```rust
// Acknowledge processed messages
for entry in &entries {
    client.xack("mystream", "mygroup", &entry.id).await?;
}
```

### Claim Pending Messages

```rust
// Claim messages that have been pending too long
let claimed = client
    .xclaim("mystream", "mygroup", "consumer1", 60000, vec!["0-1"])
    .await?;
```

## Stream Metadata

```rust
// Get stream length
let len: i64 = client.xlen("mystream").await?;

// Get stream info
let info = client.xinfo_stream("mystream").await?;
println!("Length: {}", info.length);
println!("Groups: {}", info.group_count);

// Get consumer groups
let groups = client.xinfo_groups("mystream").await?;
for group in &groups {
    println!("Group: {}, Consumers: {}", group.name, group.consumers);
}

// Get consumers in a group
let consumers = client.xinfo_consumers("mystream", "mygroup").await?;
for consumer in &consumers {
    println!("Consumer: {}, Pending: {}", consumer.name, consumer.pending);
}
```

## Delete Entries

```rust
// Delete specific entries by ID
let deleted = client.xdel("mystream", vec!["1526989814062-0", "1526989814062-1"]).await?;

// Trim stream to keep only last N entries
let trimmed = client.xtrim("mystream", 1000).await?;
```

## StreamEntry Helper

```rust
use redis_oxide::streams::StreamEntry;

let entry = StreamEntry::new()
    .id("0-1")  // Specify ID, or use "*" for auto-generation
    .field("field1", "value1")
    .field("field2", "value2");

let entries = vec![entry];
let id = client.xadd("mystream", "*", entries).await?;
```

## Best Practices

1. **Use consumer groups** for distributed processing
2. **Acknowledge messages** promptly to avoid redelivery
3. **Monitor pending messages** and claim stuck messages
4. **Use appropriate stream length** limits (xtrim)
5. **Use meaningful field names** for easier debugging

## Common Use Cases

- **Event sourcing**: Log all events in order
- **Message queue**: High-throughput message passing
- **Activity tracking**: Track user actions in real-time
- **Metrics collection**: Aggregate time-series data

## Stream ID Format

Stream IDs consist of two parts:
- **Timestamp**: Milliseconds since epoch
- **Sequence number**: Entry number within that millisecond

Format: `<timestamp>-<sequence>`
Examples: `0-1`, `1526989814062-0`, `1526989814062-5`
