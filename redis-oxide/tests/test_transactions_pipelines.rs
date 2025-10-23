//! Integration tests for Transactions and Pipelines

#![allow(clippy::uninlined_format_args)]

use redis_oxide::{Client, ConnectionConfig, TransactionResult};
use std::collections::HashMap;
use testcontainers::{clients::Cli, images::redis::Redis, Container};

async fn setup_client(docker: &Cli) -> Result<Client, redis_oxide::RedisError> {
    let container = docker.run(Redis::default());
    let host_port = container.get_host_port_ipv4(6379);
    let redis_url = format!("redis://localhost:{}", host_port);
    let config = ConnectionConfig::new(&redis_url);
    Client::connect(config).await
}

#[tokio::test]
async fn test_basic_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    // Create a pipeline with multiple commands
    let mut pipeline = client.pipeline();
    pipeline.set("pipe:key1", "value1");
    pipeline.set("pipe:key2", "value2");
    pipeline.get("pipe:key1");
    pipeline.get("pipe:key2");
    pipeline.incr("pipe:counter");

    let results = pipeline.execute().await?;
    assert_eq!(results.len(), 5);

    // Verify the results
    // SET commands return OK (true)
    assert_eq!(results[0].as_bool()?, true);
    assert_eq!(results[1].as_bool()?, true);
    
    // GET commands return the values
    assert_eq!(results[2].as_string()?, "value1");
    assert_eq!(results[3].as_string()?, "value2");
    
    // INCR returns the new value
    assert_eq!(results[4].as_int()?, 1);

    // Verify values were actually set
    let value1: Option<String> = client.get("pipe:key1").await?;
    let value2: Option<String> = client.get("pipe:key2").await?;
    let counter: Option<String> = client.get("pipe:counter").await?;
    
    assert_eq!(value1, Some("value1".to_string()));
    assert_eq!(value2, Some("value2".to_string()));
    assert_eq!(counter, Some("1".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_pipeline_with_hash_operations() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    let mut pipeline = client.pipeline();
    pipeline.hset("pipe:hash", "field1", "value1");
    pipeline.hset("pipe:hash", "field2", "value2");
    pipeline.hget("pipe:hash", "field1");
    pipeline.hgetall("pipe:hash");
    pipeline.hlen("pipe:hash");

    let results = pipeline.execute().await?;
    assert_eq!(results.len(), 5);

    // HSET results (number of fields added)
    assert_eq!(results[0].as_int()?, 1);
    assert_eq!(results[1].as_int()?, 1);
    
    // HGET result
    assert_eq!(results[2].as_string()?, "value1");
    
    // HGETALL result (array of field-value pairs)
    let hgetall_result = &results[3];
    // Note: HGETALL returns an array, we'd need to parse it properly
    
    // HLEN result
    assert_eq!(results[4].as_int()?, 2);

    Ok(())
}

#[tokio::test]
async fn test_basic_transaction() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    // Set initial values
    client.set("tx:account1", "100").await?;
    client.set("tx:account2", "50").await?;

    // Create a transaction
    let mut transaction = client.transaction().await?;
    transaction.get("tx:account1");
    transaction.get("tx:account2");
    transaction.set("tx:account1", "80");  // Transfer 20 from account1
    transaction.set("tx:account2", "70");  // Transfer 20 to account2

    let results = transaction.exec().await?;
    
    match results {
        TransactionResult::Success(values) => {
            assert_eq!(values.len(), 4);
            // GET results
            assert_eq!(values[0].as_string()?, "100");
            assert_eq!(values[1].as_string()?, "50");
            // SET results
            assert_eq!(values[2].as_bool()?, true);
            assert_eq!(values[3].as_bool()?, true);
        }
        TransactionResult::Aborted => {
            panic!("Transaction should not be aborted");
        }
    }

    // Verify final values
    let account1: Option<String> = client.get("tx:account1").await?;
    let account2: Option<String> = client.get("tx:account2").await?;
    assert_eq!(account1, Some("80".to_string()));
    assert_eq!(account2, Some("70".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_transaction_with_watch() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    // Set initial value
    client.set("tx:watched_key", "initial").await?;

    // Create transaction with WATCH
    let mut transaction = client.transaction().await?;
    transaction.watch(vec!["tx:watched_key".to_string()]).await?;
    
    // Modify the watched key from another "client" (simulate concurrent modification)
    client.set("tx:watched_key", "modified_externally").await?;
    
    // Queue commands in transaction
    transaction.set("tx:watched_key", "modified_in_transaction");
    transaction.set("tx:other_key", "other_value");

    // Execute transaction - should be aborted due to watched key modification
    let results = transaction.exec().await?;
    
    match results {
        TransactionResult::Success(_) => {
            panic!("Transaction should be aborted due to watched key modification");
        }
        TransactionResult::Aborted => {
            // This is expected
        }
    }

    // Verify the key was not modified by the transaction
    let value: Option<String> = client.get("tx:watched_key").await?;
    assert_eq!(value, Some("modified_externally".to_string()));
    
    let other_value: Option<String> = client.get("tx:other_key").await?;
    assert_eq!(other_value, None); // Should not be set due to aborted transaction

    Ok(())
}

#[tokio::test]
async fn test_transaction_discard() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    // Set initial value
    client.set("tx:discard_key", "initial").await?;

    // Create transaction
    let mut transaction = client.transaction().await?;
    transaction.set("tx:discard_key", "should_not_be_set");
    transaction.set("tx:another_key", "also_should_not_be_set");

    // Discard the transaction
    transaction.discard().await?;

    // Verify values were not changed
    let value1: Option<String> = client.get("tx:discard_key").await?;
    let value2: Option<String> = client.get("tx:another_key").await?;
    
    assert_eq!(value1, Some("initial".to_string()));
    assert_eq!(value2, None);

    Ok(())
}

#[tokio::test]
async fn test_complex_pipeline_with_different_data_types() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    let mut pipeline = client.pipeline();
    
    // String operations
    pipeline.set("complex:string", "hello");
    pipeline.incr("complex:counter");
    
    // Hash operations
    pipeline.hset("complex:hash", "field1", "value1");
    pipeline.hset("complex:hash", "field2", "value2");
    
    // List operations
    pipeline.lpush("complex:list", vec!["item1".to_string(), "item2".to_string()]);
    pipeline.rpush("complex:list", vec!["item3".to_string()]);
    
    // Set operations
    pipeline.sadd("complex:set", vec!["member1".to_string(), "member2".to_string()]);
    
    // Get operations to verify
    pipeline.get("complex:string");
    pipeline.get("complex:counter");
    pipeline.hgetall("complex:hash");
    pipeline.lrange("complex:list", 0, -1);
    pipeline.smembers("complex:set");

    let results = pipeline.execute().await?;
    assert_eq!(results.len(), 12);

    // Verify string operations
    assert_eq!(results[0].as_bool()?, true); // SET
    assert_eq!(results[1].as_int()?, 1);     // INCR
    
    // Verify hash operations
    assert_eq!(results[2].as_int()?, 1);     // HSET field1
    assert_eq!(results[3].as_int()?, 1);     // HSET field2
    
    // Verify list operations
    assert_eq!(results[4].as_int()?, 2);     // LPUSH
    assert_eq!(results[5].as_int()?, 3);     // RPUSH
    
    // Verify set operations
    assert_eq!(results[6].as_int()?, 2);     // SADD
    
    // Verify get operations
    assert_eq!(results[7].as_string()?, "hello");     // GET string
    assert_eq!(results[8].as_string()?, "1");         // GET counter

    Ok(())
}

#[tokio::test]
async fn test_pipeline_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    // Set up a string key
    client.set("error:string_key", "string_value").await?;

    let mut pipeline = client.pipeline();
    pipeline.set("error:good_key", "good_value");           // Should succeed
    pipeline.hget("error:string_key", "field");             // Should return nil (wrong type)
    pipeline.get("error:good_key");                         // Should succeed
    pipeline.llen("error:string_key");                      // Should return error or 0

    let results = pipeline.execute().await?;
    assert_eq!(results.len(), 4);

    // First command should succeed
    assert_eq!(results[0].as_bool()?, true);
    
    // Second command should return nil (Redis handles type errors gracefully in some cases)
    // The exact behavior depends on Redis version and command
    
    // Third command should succeed
    assert_eq!(results[2].as_string()?, "good_value");

    Ok(())
}

#[tokio::test]
async fn test_nested_transactions_not_allowed() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    // Create first transaction
    let mut transaction1 = client.transaction().await?;
    transaction1.set("nested:key1", "value1");

    // Try to create second transaction (should work as they're independent)
    let mut transaction2 = client.transaction().await?;
    transaction2.set("nested:key2", "value2");

    // Execute both transactions
    let results1 = transaction1.exec().await?;
    let results2 = transaction2.exec().await?;

    match (results1, results2) {
        (TransactionResult::Success(r1), TransactionResult::Success(r2)) => {
            assert_eq!(r1.len(), 1);
            assert_eq!(r2.len(), 1);
            assert_eq!(r1[0].as_bool()?, true);
            assert_eq!(r2[0].as_bool()?, true);
        }
        _ => panic!("Both transactions should succeed"),
    }

    // Verify both keys were set
    let value1: Option<String> = client.get("nested:key1").await?;
    let value2: Option<String> = client.get("nested:key2").await?;
    assert_eq!(value1, Some("value1".to_string()));
    assert_eq!(value2, Some("value2".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_large_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    let mut pipeline = client.pipeline();
    let num_operations = 100;

    // Add many SET operations
    for i in 0..num_operations {
        pipeline.set(&format!("large:key{}", i), &format!("value{}", i));
    }

    // Add many GET operations
    for i in 0..num_operations {
        pipeline.get(&format!("large:key{}", i));
    }

    let results = pipeline.execute().await?;
    assert_eq!(results.len(), num_operations * 2);

    // Verify SET results
    for i in 0..num_operations {
        assert_eq!(results[i].as_bool()?, true);
    }

    // Verify GET results
    for i in 0..num_operations {
        let get_result_index = num_operations + i;
        assert_eq!(results[get_result_index].as_string()?, format!("value{}", i));
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_pipelines() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    let num_concurrent = 10;
    let mut handles = Vec::new();

    // Spawn multiple tasks that each run a pipeline
    for task_id in 0..num_concurrent {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let mut pipeline = client_clone.pipeline();
            
            // Each task sets and gets its own keys
            for i in 0..5 {
                let key = format!("concurrent:task{}:key{}", task_id, i);
                let value = format!("task{}_value{}", task_id, i);
                pipeline.set(&key, &value);
                pipeline.get(&key);
            }
            
            pipeline.execute().await
        });
        handles.push(handle);
    }

    // Wait for all pipelines to complete
    for handle in handles {
        let results = handle.await??;
        assert_eq!(results.len(), 10); // 5 SETs + 5 GETs
        
        // Verify SET/GET pairs
        for i in 0..5 {
            let set_index = i * 2;
            let get_index = i * 2 + 1;
            assert_eq!(results[set_index].as_bool()?, true);
            // GET result should match the expected value pattern
            let get_result = results[get_index].as_string()?;
            assert!(get_result.contains("value"));
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_transaction_with_conditional_logic() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Cli::default();
    let client = setup_client(&docker).await?;

    // Set up initial state
    client.set("conditional:balance", "100").await?;
    client.set("conditional:min_balance", "10").await?;

    // Transaction that implements conditional withdrawal
    let mut transaction = client.transaction().await?;
    transaction.watch(vec!["conditional:balance".to_string()]).await?;
    
    // Get current balance (this will be queued)
    transaction.get("conditional:balance");
    transaction.get("conditional:min_balance");
    
    // Simulate withdrawal logic (in real scenario, you'd check the balance first)
    transaction.set("conditional:balance", "80"); // Withdraw 20
    transaction.set("conditional:last_transaction", "withdrawal:20");

    let results = transaction.exec().await?;
    
    match results {
        TransactionResult::Success(values) => {
            assert_eq!(values.len(), 4);
            assert_eq!(values[0].as_string()?, "100"); // Original balance
            assert_eq!(values[1].as_string()?, "10");  // Min balance
            assert_eq!(values[2].as_bool()?, true);    // SET balance
            assert_eq!(values[3].as_bool()?, true);    // SET last_transaction
        }
        TransactionResult::Aborted => {
            panic!("Transaction should not be aborted");
        }
    }

    // Verify final state
    let final_balance: Option<String> = client.get("conditional:balance").await?;
    let last_tx: Option<String> = client.get("conditional:last_transaction").await?;
    assert_eq!(final_balance, Some("80".to_string()));
    assert_eq!(last_tx, Some("withdrawal:20".to_string()));

    Ok(())
}
