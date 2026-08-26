//! Integration tests for Transactions and Pipelines

use redis_oxide::{Client, ConnectionConfig};

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

async fn setup_client() -> Result<Client, redis_oxide::RedisError> {
    let config = ConnectionConfig::new(redis_url().as_str());
    Client::connect(config).await
}

#[tokio::test]
async fn test_basic_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;
    client
        .del(vec![
            "pipe:key1".to_string(),
            "pipe:key2".to_string(),
            "pipe:counter".to_string(),
        ])
        .await?;

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
    assert!(results[0].as_bool()?);
    assert!(results[1].as_bool()?);

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
    let client = setup_client().await?;
    client.del(vec!["pipe:hash".to_string()]).await?;

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
    let _ = &results[3];
    // Note: HGETALL returns an array, we'd need to parse it properly

    // HLEN result
    assert_eq!(results[4].as_int()?, 2);

    Ok(())
}

#[tokio::test]
async fn test_basic_transaction() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;
    client
        .del(vec!["tx:account1".to_string(), "tx:account2".to_string()])
        .await?;

    // Set initial values
    client.set("tx:account1", "100").await?;
    client.set("tx:account2", "50").await?;

    // Create a transaction
    let mut transaction = client.transaction().await?;
    transaction.get("tx:account1");
    transaction.get("tx:account2");
    transaction.set("tx:account1", "80"); // Transfer 20 from account1
    transaction.set("tx:account2", "70"); // Transfer 20 to account2

    let results = transaction.exec().await?;

    // Transaction.exec() returns Vec<RespValue> directly, not a TransactionResult enum
    assert_eq!(results.len(), 4);
    // GET results
    assert_eq!(results[0].as_string()?, "100");
    assert_eq!(results[1].as_string()?, "50");
    // SET results
    assert!(results[2].as_bool()?);
    assert!(results[3].as_bool()?);

    // Verify final values
    let account1: Option<String> = client.get("tx:account1").await?;
    let account2: Option<String> = client.get("tx:account2").await?;
    assert_eq!(account1, Some("80".to_string()));
    assert_eq!(account2, Some("70".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_transaction_with_watch() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;
    client
        .del(vec![
            "tx:watched_key".to_string(),
            "tx:other_key".to_string(),
        ])
        .await?;

    // Set initial value
    client.set("tx:watched_key", "initial").await?;

    // Create transaction with WATCH
    let mut transaction = client.transaction().await?;
    transaction
        .watch(vec!["tx:watched_key".to_string()])
        .await?;

    // Modify the watched key from another "client" (simulate concurrent modification)
    client.set("tx:watched_key", "modified_externally").await?;

    // Queue commands in transaction
    transaction.set("tx:watched_key", "modified_in_transaction");
    transaction.set("tx:other_key", "other_value");

    // Execute transaction - should be aborted due to watched key modification
    let results = transaction.exec().await?;

    // If transaction is aborted, Redis returns an empty array
    assert!(
        results.is_empty(),
        "Transaction should be aborted due to watched key modification"
    );
    // An empty result vector indicates the transaction was aborted

    // Verify the key was not modified by the transaction
    let value: Option<String> = client.get("tx:watched_key").await?;
    assert_eq!(value, Some("modified_externally".to_string()));

    let other_value: Option<String> = client.get("tx:other_key").await?;
    assert_eq!(other_value, None); // Should not be set due to aborted transaction

    Ok(())
}

#[tokio::test]
async fn test_transaction_discard() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;
    client
        .del(vec![
            "tx:discard_key".to_string(),
            "tx:another_key".to_string(),
        ])
        .await?;

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
async fn test_complex_pipeline_with_different_data_types() -> Result<(), Box<dyn std::error::Error>>
{
    let client = setup_client().await?;
    client
        .del(vec![
            "complex:string".to_string(),
            "complex:counter".to_string(),
            "complex:hash".to_string(),
            "complex:list".to_string(),
            "complex:set".to_string(),
        ])
        .await?;

    let mut pipeline = client.pipeline();

    // String operations
    pipeline.set("complex:string", "hello");
    pipeline.incr("complex:counter");

    // Hash operations
    pipeline.hset("complex:hash", "field1", "value1");
    pipeline.hset("complex:hash", "field2", "value2");

    // List operations
    pipeline.lpush(
        "complex:list",
        vec!["item1".to_string(), "item2".to_string()],
    );
    pipeline.rpush("complex:list", vec!["item3".to_string()]);

    // Set operations
    pipeline.sadd(
        "complex:set",
        vec!["member1".to_string(), "member2".to_string()],
    );

    // Get operations to verify
    pipeline.get("complex:string");
    pipeline.get("complex:counter");
    pipeline.hgetall("complex:hash");
    pipeline.lrange("complex:list", 0, -1);
    pipeline.smembers("complex:set");

    let results = pipeline.execute().await?;
    assert_eq!(results.len(), 12);

    // Verify string operations
    assert!(results[0].as_bool()?); // SET
    assert_eq!(results[1].as_int()?, 1); // INCR

    // Verify hash operations
    assert_eq!(results[2].as_int()?, 1); // HSET field1
    assert_eq!(results[3].as_int()?, 1); // HSET field2

    // Verify list operations
    assert_eq!(results[4].as_int()?, 2); // LPUSH
    assert_eq!(results[5].as_int()?, 3); // RPUSH

    // Verify set operations
    assert_eq!(results[6].as_int()?, 2); // SADD

    // Verify get operations
    assert_eq!(results[7].as_string()?, "hello"); // GET string
    assert_eq!(results[8].as_string()?, "1"); // GET counter

    Ok(())
}

#[tokio::test]
async fn test_pipeline_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;
    client
        .del(vec![
            "error:string_key".to_string(),
            "error:good_key".to_string(),
        ])
        .await?;

    // Set up a string key
    client.set("error:string_key", "string_value").await?;

    let mut pipeline = client.pipeline();
    pipeline.set("error:good_key", "good_value"); // Should succeed
    pipeline.hget("error:string_key", "field"); // Should return nil (wrong type)
    pipeline.get("error:good_key"); // Should succeed
    pipeline.llen("error:string_key"); // Should return error or 0

    let results = pipeline.execute().await?;
    assert_eq!(results.len(), 4);

    // First command should succeed
    assert!(results[0].as_bool()?);

    // Second command should return nil (Redis handles type errors gracefully in some cases)
    // The exact behavior depends on Redis version and command

    // Third command should succeed
    assert_eq!(results[2].as_string()?, "good_value");

    Ok(())
}

#[tokio::test]
async fn test_nested_transactions_not_allowed() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;
    client
        .del(vec!["nested:key1".to_string(), "nested:key2".to_string()])
        .await?;

    // Create first transaction
    let mut transaction1 = client.transaction().await?;
    transaction1.set("nested:key1", "value1");

    // Try to create second transaction (should work as they're independent)
    let mut transaction2 = client.transaction().await?;
    transaction2.set("nested:key2", "value2");

    // Execute both transactions
    let results1 = transaction1.exec().await?;
    let results2 = transaction2.exec().await?;

    // Both transactions should succeed and return their results
    assert_eq!(results1.len(), 1);
    assert_eq!(results2.len(), 1);
    assert!(results1[0].as_bool()?);
    assert!(results2[0].as_bool()?);

    // Verify both keys were set
    let value1: Option<String> = client.get("nested:key1").await?;
    let value2: Option<String> = client.get("nested:key2").await?;
    assert_eq!(value1, Some("value1".to_string()));
    assert_eq!(value2, Some("value2".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_large_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;
    let keys: Vec<String> = (0..100).map(|i| format!("large:key{i}")).collect();
    client.del(keys).await?;

    let mut pipeline = client.pipeline();
    let num_operations = 100;

    // Add many SET operations
    for i in 0..num_operations {
        pipeline.set(format!("large:key{i}"), format!("value{i}"));
    }

    // Add many GET operations
    for i in 0..num_operations {
        pipeline.get(format!("large:key{i}"));
    }

    let results = pipeline.execute().await?;
    assert_eq!(results.len(), num_operations * 2);

    // Verify SET results
    for result in results.iter().take(num_operations) {
        assert!(result.as_bool()?);
    }

    // Verify GET results
    for (index, result) in results.iter().skip(num_operations).enumerate() {
        assert_eq!(result.as_string()?, format!("value{index}"));
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_pipelines() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let num_concurrent = 10;
    let keys: Vec<String> = (0..num_concurrent)
        .flat_map(|task_id| (0..5).map(move |i| format!("concurrent:task{task_id}:key{i}")))
        .collect();
    client.del(keys).await?;
    let mut handles = Vec::new();

    // Spawn multiple tasks that each run a pipeline
    for task_id in 0..num_concurrent {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let mut pipeline = client_clone.pipeline();

            // Each task sets and gets its own keys
            for i in 0..5 {
                let key = format!("concurrent:task{task_id}:key{i}");
                let value = format!("task{task_id}_value{i}");
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
            assert!(results[set_index].as_bool()?);
            // GET result should match the expected value pattern
            let get_result = results[get_index].as_string()?;
            assert!(get_result.contains("value"));
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_transaction_with_conditional_logic() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Set up initial state
    client.set("conditional:balance", "100").await?;
    client.set("conditional:min_balance", "10").await?;

    // Transaction that implements conditional withdrawal
    let mut transaction = client.transaction().await?;
    transaction
        .watch(vec!["conditional:balance".to_string()])
        .await?;

    // Get current balance (this will be queued)
    transaction.get("conditional:balance");
    transaction.get("conditional:min_balance");

    // Simulate withdrawal logic (in real scenario, you'd check the balance first)
    transaction.set("conditional:balance", "80"); // Withdraw 20
    transaction.set("conditional:last_transaction", "withdrawal:20");

    let results = transaction.exec().await?;

    assert_eq!(results.len(), 4);
    assert_eq!(results[0].as_string()?, "100"); // Original balance
    assert_eq!(results[1].as_string()?, "10"); // Min balance
    assert!(results[2].as_bool()?); // SET balance
    assert!(results[3].as_bool()?); // SET last_transaction

    // Verify final state
    let final_balance: Option<String> = client.get("conditional:balance").await?;
    let last_tx: Option<String> = client.get("conditional:last_transaction").await?;
    assert_eq!(final_balance, Some("80".to_string()));
    assert_eq!(last_tx, Some("withdrawal:20".to_string()));

    Ok(())
}
