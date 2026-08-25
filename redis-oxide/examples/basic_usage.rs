//! Basic usage example for redis-oxide.

use redis_oxide::{Client, ConnectionConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let client = Client::connect(ConnectionConfig::new("redis://localhost:6379")).await?;

    client.set("mykey", "Hello, Redis!").await?;
    println!("GET mykey: {:?}", client.get("mykey").await?);
    client
        .set_ex("tempkey", "temporary value", Duration::from_secs(60))
        .await?;
    println!("SET NX: {}", client.set_nx("mykey", "new value").await?);
    println!("INCR: {}", client.incr("counter").await?);
    println!(
        "EXISTS: {}",
        client.exists(vec!["mykey".to_string()]).await?
    );
    println!("TTL: {:?}", client.ttl("tempkey").await?);
    client
        .del(vec!["mykey".to_string(), "counter".to_string()])
        .await?;
    Ok(())
}
