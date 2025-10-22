//! Example demonstrating different pool strategies

use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Multiplexed Connection Strategy ===");
    {
        let mut config = ConnectionConfig::new("redis://localhost:6379");
        config.pool = PoolConfig {
            strategy: PoolStrategy::Multiplexed,
            ..Default::default()
        };

        let client = Client::connect(config).await?;
        
        // All operations share a single connection
        for i in 0..5 {
            client.set(format!("mux:key{}", i), format!("value{}", i)).await?;
        }
        println!("Set 5 keys using multiplexed connection");
    }

    println!("\n=== Connection Pool Strategy ===");
    {
        let mut config = ConnectionConfig::new("redis://localhost:6379");
        config.pool = PoolConfig {
            strategy: PoolStrategy::Pool,
            max_size: 10,
            min_idle: 2,
            connection_timeout: Duration::from_secs(5),
        };

        let client = Client::connect(config).await?;
        
        // Operations can use different connections from the pool
        for i in 0..5 {
            client.set(format!("pool:key{}", i), format!("value{}", i)).await?;
        }
        println!("Set 5 keys using connection pool");
    }

    println!("\n=== Concurrent Operations ===");
    {
        let config = ConnectionConfig::new("redis://localhost:6379");
        let client = Client::connect(config).await?;

        // Spawn multiple concurrent operations
        let mut tasks = vec![];
        for i in 0..10 {
            let client = &client;
            tasks.push(async move {
                let key = format!("concurrent:key{}", i);
                client.set(&key, format!("value{}", i)).await?;
                client.get(&key).await
            });
        }

        let results = futures::future::join_all(tasks).await;
        println!("Completed {} concurrent operations", results.len());
    }

    Ok(())
}
