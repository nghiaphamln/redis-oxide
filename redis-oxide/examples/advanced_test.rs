//! Advanced test example for redis-oxide library
//! This example demonstrates advanced functionality including pools, transactions, etc.

use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing advanced Redis-oxide features...");
    
    // Test with connection pool strategy
    println!("Testing with Connection Pool strategy...");
    let mut config = ConnectionConfig::new("redis://localhost:6379");
    config.pool = PoolConfig {
        strategy: PoolStrategy::Pool,
        max_size: 10,
        min_idle: 2,
        connection_timeout: Duration::from_secs(5),
    };
    
    let pool_client = Client::connect(config).await?;
    println!("Connected with pool strategy!");
    
    // Test transactions
    println!("Testing transaction operations...");
    let mut transaction = pool_client.transaction().await?;
    
    // Add commands to the transaction (note: no await for adding commands)
    transaction.set("tx:key1", "value1");
    transaction.set("tx:key2", "value2");
    transaction.set("tx:key3", "value3");
    
    // Execute the transaction
    let results = transaction.exec().await?;
    println!("Transaction executed with {} results", results.len());
    
    // Verify transaction results
    if let Some(value) = pool_client.get("tx:key1").await? {
        println!("tx:key1 = {}", value);
    }
    if let Some(value) = pool_client.get("tx:key2").await? {
        println!("tx:key2 = {}", value);
    }
    if let Some(value) = pool_client.get("tx:key3").await? {
        println!("tx:key3 = {}", value);
    }
    
    // Test pipeline operations
    println!("Testing pipeline operations...");
    let mut pipeline = pool_client.pipeline();
    
    // Add commands to the pipeline (note: no await for adding commands)
    pipeline.set("pipeline:key1", "pipeline_value1");
    pipeline.get("pipeline:key1");
    pipeline.incr("pipeline:counter");
    pipeline.expire("pipeline:key1", Duration::from_secs(30));
    
    // Execute all commands in the pipeline
    let results = pipeline.execute().await?;
    println!("Pipeline executed with {} results", results.len());
    
    // Test hash operations
    println!("Testing hash operations...");
    pool_client.hset("hash:test", "field1", "value1").await?;
    pool_client.hset("hash:test", "field2", "value2").await?;
    
    if let Some(value) = pool_client.hget("hash:test", "field1").await? {
        println!("Hash field1 = {}", value);
    }
    
    let all_fields = pool_client.hgetall("hash:test").await?;
    println!("All hash fields: {:?}", all_fields);
    
    // Test list operations
    println!("Testing list operations...");
    pool_client.lpush("list:test", vec!["item1".to_string()]).await?;
    pool_client.lpush("list:test", vec!["item2".to_string()]).await?;
    pool_client.lpush("list:test", vec!["item3".to_string()]).await?;
    
    let items = pool_client.lrange("list:test", 0, -1).await?;
    println!("List items: {:?}", items);
    
    // Test multiplexed connection strategy as well
    println!("Testing with Multiplexed connection strategy...");
    let config = ConnectionConfig::new("redis://localhost:6379");
    let multiplexed_client = Client::connect(config).await?;
    
    // Run concurrent operations to test multiplexed connection
    let mut tasks = Vec::new();
    for i in 0..10 {
        let client_clone = multiplexed_client.clone();
        let task = tokio::spawn(async move {
            client_clone
                .set(format!("mux_test:{}", i), format!("mux_value:{}", i))
                .await
                .map_err(|e| format!("Error setting key {}: {}", i, e))
        });
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    for (i, task) in tasks.into_iter().enumerate() {
        match task.await {
            Ok(Ok(_)) => println!("Task {} completed successfully", i),
            Ok(Err(e)) => eprintln!("Task {} failed: {}", i, e),
            Err(e) => eprintln!("Task {} panicked: {}", i, e),
        }
    }
    
    // Verify multiplexed operations
    for i in 0..10 {
        if let Some(value) = multiplexed_client.get(&format!("mux_test:{}", i)).await? {
            println!("mux_test:{} = {}", i, value);
        }
    }
    
    // Clean up
    println!("Cleaning up test keys...");
    pool_client.del(vec![
        "tx:key1".to_string(),
        "tx:key2".to_string(),
        "tx:key3".to_string(),
        "pipeline:key1".to_string(),
        "pipeline:counter".to_string(),
        "hash:test".to_string(),
        "list:test".to_string(),
    ]).await?;
    
    let mux_keys: Vec<String> = (0..10).map(|i| format!("mux_test:{}", i)).collect();
    multiplexed_client.del(mux_keys).await?;
    
    println!("Advanced tests completed successfully!");
    
    Ok(())
}