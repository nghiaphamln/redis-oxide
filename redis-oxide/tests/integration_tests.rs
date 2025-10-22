//! Integration tests for redis-oxide
//!
//! These tests require a running Redis instance.
//! Set REDIS_URL environment variable or use default redis://localhost:6379

use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};
use std::time::Duration;

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

#[tokio::test]
async fn test_basic_set_get() {
    let config = ConnectionConfig::new(redis_url());
    let client = Client::connect(config).await.expect("Failed to connect");

    // SET and GET
    client
        .set("test:basic:key", "test_value")
        .await
        .expect("SET failed");

    let value = client
        .get("test:basic:key")
        .await
        .expect("GET failed")
        .expect("Key not found");

    assert_eq!(value, "test_value");

    // Cleanup
    client
        .del(vec!["test:basic:key".to_string()])
        .await
        .expect("DEL failed");
}

#[tokio::test]
async fn test_set_with_expiration() {
    let config = ConnectionConfig::new(redis_url());
    let client = Client::connect(config).await.expect("Failed to connect");

    // SET with expiration
    client
        .set_ex("test:expire:key", "temp_value", Duration::from_secs(2))
        .await
        .expect("SET EX failed");

    // Should exist immediately
    let value = client
        .get("test:expire:key")
        .await
        .expect("GET failed")
        .expect("Key not found");
    assert_eq!(value, "temp_value");

    // Check TTL
    let ttl = client
        .ttl("test:expire:key")
        .await
        .expect("TTL failed")
        .expect("No TTL");
    assert!(ttl > 0 && ttl <= 2);

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Should be gone
    let value = client.get("test:expire:key").await.expect("GET failed");
    assert!(value.is_none());
}

#[tokio::test]
async fn test_set_nx() {
    let config = ConnectionConfig::new(redis_url());
    let client = Client::connect(config).await.expect("Failed to connect");

    // First SET NX should succeed
    let set = client
        .set_nx("test:nx:key", "value1")
        .await
        .expect("SET NX failed");
    assert!(set, "First SET NX should succeed");

    // Second SET NX should fail
    let set = client
        .set_nx("test:nx:key", "value2")
        .await
        .expect("SET NX failed");
    assert!(!set, "Second SET NX should fail");

    // Value should still be the first one
    let value = client
        .get("test:nx:key")
        .await
        .expect("GET failed")
        .expect("Key not found");
    assert_eq!(value, "value1");

    // Cleanup
    client
        .del(vec!["test:nx:key".to_string()])
        .await
        .expect("DEL failed");
}

#[tokio::test]
async fn test_incr_decr() {
    let config = ConnectionConfig::new(redis_url());
    let client = Client::connect(config).await.expect("Failed to connect");

    // Set initial value
    client.set("test:counter", "0").await.expect("SET failed");

    // INCR
    let value = client.incr("test:counter").await.expect("INCR failed");
    assert_eq!(value, 1);

    // INCRBY
    let value = client
        .incr_by("test:counter", 10)
        .await
        .expect("INCRBY failed");
    assert_eq!(value, 11);

    // DECR
    let value = client.decr("test:counter").await.expect("DECR failed");
    assert_eq!(value, 10);

    // DECRBY
    let value = client
        .decr_by("test:counter", 5)
        .await
        .expect("DECRBY failed");
    assert_eq!(value, 5);

    // Cleanup
    client
        .del(vec!["test:counter".to_string()])
        .await
        .expect("DEL failed");
}

#[tokio::test]
async fn test_exists() {
    let config = ConnectionConfig::new(redis_url());
    let client = Client::connect(config).await.expect("Failed to connect");

    // Set some keys
    client
        .set("test:exists:1", "value1")
        .await
        .expect("SET failed");
    client
        .set("test:exists:2", "value2")
        .await
        .expect("SET failed");

    // Check existence
    let count = client
        .exists(vec![
            "test:exists:1".to_string(),
            "test:exists:2".to_string(),
            "test:exists:3".to_string(),
        ])
        .await
        .expect("EXISTS failed");
    assert_eq!(count, 2);

    // Cleanup
    client
        .del(vec![
            "test:exists:1".to_string(),
            "test:exists:2".to_string(),
        ])
        .await
        .expect("DEL failed");
}

#[tokio::test]
async fn test_del_multiple() {
    let config = ConnectionConfig::new(redis_url());
    let client = Client::connect(config).await.expect("Failed to connect");

    // Set multiple keys
    for i in 1..=5 {
        client
            .set(format!("test:del:{}", i), format!("value{}", i))
            .await
            .expect("SET failed");
    }

    // Delete multiple keys
    let deleted = client
        .del(vec![
            "test:del:1".to_string(),
            "test:del:2".to_string(),
            "test:del:3".to_string(),
        ])
        .await
        .expect("DEL failed");
    assert_eq!(deleted, 3);

    // Verify deletion
    let exists = client
        .exists(vec!["test:del:1".to_string()])
        .await
        .expect("EXISTS failed");
    assert_eq!(exists, 0);

    // Cleanup remaining
    client
        .del(vec!["test:del:4".to_string(), "test:del:5".to_string()])
        .await
        .ok();
}

#[tokio::test]
async fn test_expire_and_ttl() {
    let config = ConnectionConfig::new(redis_url());
    let client = Client::connect(config).await.expect("Failed to connect");

    // Set a key
    client
        .set("test:ttl:key", "value")
        .await
        .expect("SET failed");

    // Set expiration
    let result = client
        .expire("test:ttl:key", Duration::from_secs(10))
        .await
        .expect("EXPIRE failed");
    assert!(result);

    // Check TTL
    let ttl = client
        .ttl("test:ttl:key")
        .await
        .expect("TTL failed")
        .expect("No TTL");
    assert!(ttl > 0 && ttl <= 10);

    // Cleanup
    client
        .del(vec!["test:ttl:key".to_string()])
        .await
        .expect("DEL failed");
}

#[tokio::test]
async fn test_multiplexed_pool() {
    let mut config = ConnectionConfig::new(redis_url());
    config.pool = PoolConfig {
        strategy: PoolStrategy::Multiplexed,
        ..Default::default()
    };

    let client = Client::connect(config).await.expect("Failed to connect");

    // Multiple concurrent operations
    let mut tasks = vec![];
    for i in 0..10 {
        let key = format!("test:mux:{}", i);
        let value = format!("value{}", i);
        tasks.push(async move {
            client.set(&key, &value).await?;
            client.get(&key).await
        });
    }

    let results = futures::future::join_all(tasks).await;
    assert_eq!(results.len(), 10);

    // Cleanup
    let keys: Vec<String> = (0..10).map(|i| format!("test:mux:{}", i)).collect();
    client.del(keys).await.ok();
}

#[tokio::test]
async fn test_connection_pool() {
    let mut config = ConnectionConfig::new(redis_url());
    config.pool = PoolConfig {
        strategy: PoolStrategy::Pool,
        max_size: 5,
        min_idle: 2,
        connection_timeout: Duration::from_secs(5),
    };

    let client = Client::connect(config).await.expect("Failed to connect");

    // Multiple concurrent operations
    let mut tasks = vec![];
    for i in 0..10 {
        let key = format!("test:pool:{}", i);
        let value = format!("value{}", i);
        tasks.push(async move {
            client.set(&key, &value).await?;
            client.get(&key).await
        });
    }

    let results = futures::future::join_all(tasks).await;
    assert_eq!(results.len(), 10);

    // Cleanup
    let keys: Vec<String> = (0..10).map(|i| format!("test:pool:{}", i)).collect();
    client.del(keys).await.ok();
}

#[tokio::test]
async fn test_nonexistent_key() {
    let config = ConnectionConfig::new(redis_url());
    let client = Client::connect(config).await.expect("Failed to connect");

    let value = client
        .get("test:nonexistent:key")
        .await
        .expect("GET failed");
    assert!(value.is_none());

    let ttl = client
        .ttl("test:nonexistent:key")
        .await
        .expect("TTL failed");
    assert!(ttl.is_none());
}

#[tokio::test]
async fn test_topology_detection() {
    let config = ConnectionConfig::new(redis_url());
    let client = Client::connect(config).await.expect("Failed to connect");

    // Just verify we can detect topology
    let topology = client.topology_type();
    println!("Detected topology: {:?}", topology);
}
