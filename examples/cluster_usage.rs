//! Redis Cluster usage example

use redis_oxide::{Client, ConnectionConfig, TopologyMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Connect to Redis Cluster
    // Provide multiple seed nodes for redundancy
    let config = ConnectionConfig::new("redis://localhost:7000,localhost:7001,localhost:7002")
        .with_topology_mode(TopologyMode::Cluster)
        .with_max_redirects(5);

    let client = Client::connect(config).await?;

    println!("Connected to Redis Cluster");

    // The client automatically handles MOVED redirects
    // When a key is moved to another node, the client:
    // 1. Parses the MOVED error
    // 2. Updates the slot mapping
    // 3. Retries the command on the correct node

    // Example: Set values across different hash slots
    for i in 0..10 {
        let key = format!("key:{}", i);
        let value = format!("value:{}", i);
        client.set(&key, &value).await?;
        println!("SET {} = {}", key, value);
    }

    // Get values back
    for i in 0..10 {
        let key = format!("key:{}", i);
        if let Some(value) = client.get(&key).await? {
            println!("GET {} = {}", key, value);
        }
    }

    // Use hash tags to ensure keys go to the same slot
    // Keys with the same hash tag {...} will be in the same slot
    client.set("{user:1000}:name", "Alice").await?;
    client.set("{user:1000}:email", "alice@example.com").await?;
    client.set("{user:1000}:age", "30").await?;

    println!("\nUser data (same hash slot):");
    if let Some(name) = client.get("{user:1000}:name").await? {
        println!("Name: {}", name);
    }
    if let Some(email) = client.get("{user:1000}:email").await? {
        println!("Email: {}", email);
    }
    if let Some(age) = client.get("{user:1000}:age").await? {
        println!("Age: {}", age);
    }

    Ok(())
}
