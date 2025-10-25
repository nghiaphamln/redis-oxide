//! Basic test example for redis-oxide library
//! This example demonstrates basic functionality of the library

use redis_oxide::{Client, ConnectionConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Redis...");
    
    // Create configuration with default Redis URL
    let config = ConnectionConfig::new("redis://localhost:6379");
    
    // Connect to Redis
    let client = Client::connect(config).await?;
    println!("Connected successfully!");
    
    // Test basic SET and GET operations
    println!("Testing basic SET/GET operations...");
    client.set("test:key", "Hello, Redis Oxide!").await?;
    
    if let Some(value) = client.get("test:key").await? {
        println!("GET result: {}", value);
    } else {
        println!("Key not found");
    }
    
    // Test INCR operation
    println!("Testing INCR operation...");
    client.set("counter", "0").await?;
    let new_value = client.incr("counter").await?;
    println!("Counter after INCR: {}", new_value);
    
    // Test SET with expiration
    println!("Testing SET with expiration...");
    client
        .set_ex("temp:key", "temporary value", Duration::from_secs(5))
        .await?;
    
    let ttl = client.ttl("temp:key").await?;
    match ttl {
        Some(ttl) => println!("TTL for temp:key: {} seconds", ttl),
        None => println!("No TTL set for temp:key"),
    }
    
    // Test EXIST operation
    println!("Testing EXISTS operation...");
    let exists_count = client.exists(vec![
        "test:key".to_string(),
        "temp:key".to_string(),
        "nonexistent:key".to_string(),
    ]).await?;
    println!("Number of existing keys: {}", exists_count);
    
    // Test multiple operations concurrently
    println!("Testing concurrent operations...");
    let mut tasks = Vec::new();
    for i in 0..5 {
        let client_clone = client.clone();
        let task = tokio::spawn(async move {
            client_clone
                .set(format!("concurrent:key:{}", i), format!("value:{}", i))
                .await
        });
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    for task in tasks {
        task.await??;
    }
    
    // Verify concurrent operations worked
    for i in 0..5 {
        if let Some(value) = client.get(&format!("concurrent:key:{}", i)).await? {
            println!("concurrent:key:{} = {}", i, value);
        }
    }
    
    // Clean up
    println!("Cleaning up test keys...");
    client.del(vec![
        "test:key".to_string(),
        "temp:key".to_string(),
        "counter".to_string(),
    ]).await?;
    
    for i in 0..5 {
        client.del(vec![format!("concurrent:key:{}", i)]).await.ok();
    }
    
    println!("All tests completed successfully!");
    
    Ok(())
}