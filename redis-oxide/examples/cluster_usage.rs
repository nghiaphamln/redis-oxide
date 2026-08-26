//! Redis Cluster usage example.

use redis_oxide::{Client, ConnectionConfig, TopologyMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let config = ConnectionConfig::new("redis://localhost:7000,localhost:7001,localhost:7002")
        .with_topology_mode(TopologyMode::Cluster)
        .with_max_redirects(5);
    let client = Client::connect(config).await?;

    for index in 0..10 {
        let key = format!("key:{index}");
        client.set(&key, format!("value:{index}")).await?;
        println!("{key}: {:?}", client.get(&key).await?);
    }
    client.set("{user:1000}:name", "Alice").await?;
    client.set("{user:1000}:email", "alice@example.com").await?;
    Ok(())
}
