# Transactions

Redis transactions allow you to execute a group of commands atomically.

## Basic Transaction

```rust
use redis_oxide::Transaction;

let mut tx = client.transaction()?;

// Queue commands
tx.set("key1", "value1");
tx.set("key2", "value2");
tx.incr("counter", 1);

// Execute all commands
let results: Vec<RespValue> = tx.execute().await?;
```

## WATCH (Optimistic Locking)

Use WATCH to implement optimistic locking for CAS operations:

```rust
use redis_oxide::Transaction;

let key = "mykey";
let max_retries = 3;

for attempt in 0..max_retries {
    // Watch the key for changes
    client.watch(vec![key]).await?;

    let current: Option<String> = client.get(key).await?;
    let new_value = current.map(|v| format!("{}_updated", v));

    let mut tx = client.transaction()?;

    if let Some(value) = &new_value {
        tx.set(key, value);
    }

    let result = tx.exec().await;

    match result {
        Ok(_) => {
            println!("Transaction successful!");
            break;
        }
        Err(_) => {
            println!("Transaction failed, retrying...");
            continue;
        }
    }
}
```

## Transaction Commands

```rust
let mut tx = client.transaction()?;

// MULTI is automatic when first command is queued
tx.set("key", "value");

// EXEC executes all queued commands
let results = tx.exec().await?;

// DISCARD cancels the transaction
// tx.discard().await?;
```

## Transaction Result

```rust
use redis_oxide::core::value::RespValue;

let mut tx = client.transaction()?;
tx.set("key1", "value1");
tx.get("key1");
tx.incr("counter", 1);

let results = tx.exec().await?;

for (i, result) in results.iter().enumerate() {
    match result {
        RespValue::SimpleString(s) => println!("Result {}: {}", i, s),
        RespValue::Integer(n) => println!("Result {}: {}", i, n),
        RespValue::BulkString(b) => println!("Result {}: {:?}", i, b),
        RespValue::Array(arr) => println!("Result {}: {:?}", i, arr),
        RespValue::Error(e) => println!("Error {}: {}", i, e),
        RespValue::Null => println!("Result {}: null", i),
    }
}
```

## Best Practices

1. **Use WATCH** for values that may be modified by other clients
2. **Keep transactions short** to minimize conflict probability
3. **Handle failures gracefully** with retry logic
4. **Consider Lua scripts** as an alternative for complex atomic operations

## When to Use Transactions

- **Batch operations**: Group related commands
- **Atomic updates**: Ensure all or nothing
- **Optimistic locking**: CAS operations with WATCH

## Limitations

- No rollback on syntax errors
- Commands inside transaction are queued, not executed immediately
- Failed commands do not stop subsequent commands
