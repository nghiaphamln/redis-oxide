//! Basic usage example for redis-oxide

use redis_oxide::{Client, ConnectionConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    // Create configuration
    let config = ConnectionConfig::new("redis://localhost:6379");

    // Connect to Redis (automatically detects Standalone vs Cluster)
    let client = Client::connect(config).await?;

    println!("Connected to Redis ({:?})", client.topology_type());

    // Basic SET and GET
    client.set("mykey", "Hello, Redis!").await?;
    if let Some(value) = client.get("mykey").await? {
        println!("GET mykey: {}", value);
    }

    // SET with expiration
    client
        .set_ex("tempkey", "temporary value", Duration::from_secs(60))
        .await?;
    println!("SET tempkey with 60s expiration");

    // SET NX (only if not exists)
    let set = client.set_nx("mykey", "new value").await?;
    println!("SET NX mykey: {}", set); // Should be false since key exists

    // INCREMENT
    client.set("counter", "0").await?;
    let value = client.incr("counter").await?;
    println!("INCR counter: {}", value);

    let value = client.incr_by("counter", 10).await?;
    println!("INCRBY counter 10: {}", value);

    // EXISTS
    let exists = client.exists(vec!["mykey".to_string()]).await?;
    println!("EXISTS mykey: {}", exists);

    // TTL
    if let Some(ttl) = client.ttl("tempkey").await? {
        println!("TTL tempkey: {} seconds", ttl);
    }

    // DELETE
    let deleted = client.del(vec!["mykey".to_string(), "counter".to_string()]).await?;
    println!("DEL mykey counter: {} keys deleted", deleted);

    Ok(())
}
