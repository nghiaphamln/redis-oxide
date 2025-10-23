//! Integration tests for Pipeline functionality
//!
//! These tests require a running Redis instance.
//! Set `REDIS_URL` environment variable or use default `redis://localhost:6379`

#![allow(clippy::uninlined_format_args)]

use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};
use std::time::Duration;

async fn create_test_client() -> Client {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let config = ConnectionConfig::new(&redis_url);
    Client::connect(config)
        .await
        .expect("Failed to connect to Redis")
}

#[tokio::test]
async fn test_pipeline_basic_operations() {
    let client = create_test_client().await;

    // Clean up any existing test keys
    let _ = client
        .del(vec![
            "pipeline_test1".to_string(),
            "pipeline_test2".to_string(),
        ])
        .await;

    // Create a pipeline and add commands
    let mut pipeline = client.pipeline();
    pipeline.set("pipeline_test1", "value1");
    pipeline.set("pipeline_test2", "value2");
    pipeline.get("pipeline_test1");
    pipeline.get("pipeline_test2");

    // Execute the pipeline
    let results = pipeline.execute().await.expect("Pipeline execution failed");

    // Verify results
    assert_eq!(results.len(), 4);

    // SET commands should return OK
    assert!(matches!(results[0], redis_oxide::RespValue::SimpleString(ref s) if s == "OK"));
    assert!(matches!(results[1], redis_oxide::RespValue::SimpleString(ref s) if s == "OK"));

    // GET commands should return the values
    assert!(matches!(results[2], redis_oxide::RespValue::BulkString(_)));
    assert!(matches!(results[3], redis_oxide::RespValue::BulkString(_)));

    // Clean up
    let _ = client
        .del(vec![
            "pipeline_test1".to_string(),
            "pipeline_test2".to_string(),
        ])
        .await;
}

#[tokio::test]
async fn test_pipeline_counter_operations() {
    let client = create_test_client().await;

    // Clean up any existing test key
    let _ = client.del(vec!["pipeline_counter".to_string()]).await;

    // Create a pipeline with counter operations
    let mut pipeline = client.pipeline();
    pipeline.set("pipeline_counter", "10");
    pipeline.incr("pipeline_counter");
    pipeline.incr_by("pipeline_counter", 5);
    pipeline.decr("pipeline_counter");
    pipeline.decr_by("pipeline_counter", 2);
    pipeline.get("pipeline_counter");

    // Execute the pipeline
    let results = pipeline.execute().await.expect("Pipeline execution failed");

    // Verify results
    assert_eq!(results.len(), 6);

    // SET should return OK
    assert!(matches!(results[0], redis_oxide::RespValue::SimpleString(ref s) if s == "OK"));

    // INCR should return 11
    assert!(matches!(results[1], redis_oxide::RespValue::Integer(11)));

    // INCRBY 5 should return 16
    assert!(matches!(results[2], redis_oxide::RespValue::Integer(16)));

    // DECR should return 15
    assert!(matches!(results[3], redis_oxide::RespValue::Integer(15)));

    // DECRBY 2 should return 13
    assert!(matches!(results[4], redis_oxide::RespValue::Integer(13)));

    // GET should return "13"
    if let redis_oxide::RespValue::BulkString(bytes) = &results[5] {
        let value = String::from_utf8_lossy(bytes);
        assert_eq!(value, "13");
    } else {
        panic!("Expected BulkString for GET result");
    }

    // Clean up
    let _ = client.del(vec!["pipeline_counter".to_string()]).await;
}

#[tokio::test]
async fn test_pipeline_exists_and_expire() {
    let client = create_test_client().await;

    // Clean up any existing test keys
    let _ = client
        .del(vec![
            "pipeline_exists1".to_string(),
            "pipeline_exists2".to_string(),
        ])
        .await;

    // Create a pipeline with EXISTS and EXPIRE operations
    let mut pipeline = client.pipeline();
    pipeline.set("pipeline_exists1", "value1");
    pipeline.set("pipeline_exists2", "value2");
    pipeline.exists(vec![
        "pipeline_exists1".to_string(),
        "pipeline_exists2".to_string(),
    ]);
    pipeline.expire("pipeline_exists1", Duration::from_secs(60));
    pipeline.ttl("pipeline_exists1");
    pipeline.del(vec!["pipeline_exists2".to_string()]);
    pipeline.exists(vec![
        "pipeline_exists1".to_string(),
        "pipeline_exists2".to_string(),
    ]);

    // Execute the pipeline
    let results = pipeline.execute().await.expect("Pipeline execution failed");

    // Verify results
    assert_eq!(results.len(), 7);

    // SET commands should return OK
    assert!(matches!(results[0], redis_oxide::RespValue::SimpleString(ref s) if s == "OK"));
    assert!(matches!(results[1], redis_oxide::RespValue::SimpleString(ref s) if s == "OK"));

    // EXISTS should return 2 (both keys exist)
    assert!(matches!(results[2], redis_oxide::RespValue::Integer(2)));

    // EXPIRE should return 1 (success)
    assert!(matches!(results[3], redis_oxide::RespValue::Integer(1)));

    // TTL should return a positive number (around 60)
    if let redis_oxide::RespValue::Integer(ttl) = results[4] {
        assert!(ttl > 0 && ttl <= 60);
    } else {
        panic!("Expected Integer for TTL result");
    }

    // DEL should return 1 (one key deleted)
    assert!(matches!(results[5], redis_oxide::RespValue::Integer(1)));

    // EXISTS should return 1 (only pipeline_exists1 remains)
    assert!(matches!(results[6], redis_oxide::RespValue::Integer(1)));

    // Clean up
    let _ = client.del(vec!["pipeline_exists1".to_string()]).await;
}

#[tokio::test]
async fn test_pipeline_empty() {
    let client = create_test_client().await;

    // Create an empty pipeline
    let mut pipeline = client.pipeline();

    // Executing empty pipeline should return error
    let result = pipeline.execute().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pipeline_with_multiplexed_pool() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let mut config = ConnectionConfig::new(&redis_url);
    config.pool = PoolConfig {
        strategy: PoolStrategy::Multiplexed,
        ..Default::default()
    };

    let client = Client::connect(config)
        .await
        .expect("Failed to connect to Redis");

    // Clean up any existing test keys
    let _ = client
        .del(vec![
            "pipeline_mux1".to_string(),
            "pipeline_mux2".to_string(),
        ])
        .await;

    // Create a pipeline
    let mut pipeline = client.pipeline();
    pipeline.set("pipeline_mux1", "mux_value1");
    pipeline.set("pipeline_mux2", "mux_value2");
    pipeline.get("pipeline_mux1");
    pipeline.get("pipeline_mux2");

    // Execute the pipeline
    let results = pipeline.execute().await.expect("Pipeline execution failed");

    // Verify results
    assert_eq!(results.len(), 4);

    // Clean up
    let _ = client
        .del(vec![
            "pipeline_mux1".to_string(),
            "pipeline_mux2".to_string(),
        ])
        .await;
}

#[tokio::test]
async fn test_pipeline_with_connection_pool() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let mut config = ConnectionConfig::new(&redis_url);
    config.pool = PoolConfig {
        strategy: PoolStrategy::Pool,
        max_size: 5,
        min_idle: 2,
        connection_timeout: Duration::from_secs(5),
    };

    let client = Client::connect(config)
        .await
        .expect("Failed to connect to Redis");

    // Clean up any existing test keys
    let _ = client
        .del(vec![
            "pipeline_pool1".to_string(),
            "pipeline_pool2".to_string(),
        ])
        .await;

    // Create a pipeline
    let mut pipeline = client.pipeline();
    pipeline.set("pipeline_pool1", "pool_value1");
    pipeline.set("pipeline_pool2", "pool_value2");
    pipeline.get("pipeline_pool1");
    pipeline.get("pipeline_pool2");

    // Execute the pipeline
    let results = pipeline.execute().await.expect("Pipeline execution failed");

    // Verify results
    assert_eq!(results.len(), 4);

    // Clean up
    let _ = client
        .del(vec![
            "pipeline_pool1".to_string(),
            "pipeline_pool2".to_string(),
        ])
        .await;
}

#[tokio::test]
async fn test_pipeline_reuse() {
    let client = create_test_client().await;

    // Clean up any existing test keys
    let _ = client
        .del(vec![
            "pipeline_reuse1".to_string(),
            "pipeline_reuse2".to_string(),
        ])
        .await;

    // Create a pipeline and use it multiple times
    let mut pipeline = client.pipeline();

    // First batch
    pipeline.set("pipeline_reuse1", "value1");
    pipeline.get("pipeline_reuse1");

    let results1 = pipeline
        .execute()
        .await
        .expect("First pipeline execution failed");
    assert_eq!(results1.len(), 2);

    // Pipeline should be empty after execution
    assert!(pipeline.is_empty());

    // Second batch
    pipeline.set("pipeline_reuse2", "value2");
    pipeline.get("pipeline_reuse2");

    let results2 = pipeline
        .execute()
        .await
        .expect("Second pipeline execution failed");
    assert_eq!(results2.len(), 2);

    // Clean up
    let _ = client
        .del(vec![
            "pipeline_reuse1".to_string(),
            "pipeline_reuse2".to_string(),
        ])
        .await;
}

#[tokio::test]
async fn test_pipeline_clear() {
    let client = create_test_client().await;

    // Create a pipeline and add commands
    let mut pipeline = client.pipeline();
    pipeline.set("test_key", "test_value");
    pipeline.get("test_key");

    assert_eq!(pipeline.len(), 2);
    assert!(!pipeline.is_empty());

    // Clear the pipeline
    pipeline.clear();

    assert_eq!(pipeline.len(), 0);
    assert!(pipeline.is_empty());

    // Executing cleared pipeline should return error
    let result = pipeline.execute().await;
    assert!(result.is_err());
}
