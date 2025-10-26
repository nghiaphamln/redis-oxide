//! Integration tests for all Redis data types
//!
//! These tests verify that all Redis data type operations work correctly
//! with both standalone and cluster configurations.
//! Set `REDIS_URL` environment variable or use default `redis://localhost:6379`

#![allow(clippy::uninlined_format_args)]

use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

async fn setup_client(
    config_modifier: Option<fn(&mut ConnectionConfig)>,
) -> Result<Client, redis_oxide::RedisError> {
    let mut config = ConnectionConfig::new(redis_url().as_str());
    if let Some(modifier) = config_modifier {
        modifier(&mut config);
    }
    Client::connect(config).await
}

#[tokio::test]
async fn test_string_operations_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client(None).await?;

    // Clean up any leftover keys
    let _ = client
        .del(vec![
            "str:basic".to_string(),
            "str:expire".to_string(),
            "str:nx".to_string(),
            "str:counter".to_string(),
            "str:multi1".to_string(),
            "str:multi2".to_string(),
        ])
        .await;

    // Basic SET/GET
    client.set("str:basic", "hello world").await?;
    let value: Option<String> = client.get("str:basic").await?;
    assert_eq!(value, Some("hello world".to_string()));

    // SET with expiration
    client
        .set_ex("str:expire", "temporary", Duration::from_secs(1))
        .await?;
    let value: Option<String> = client.get("str:expire").await?;
    assert_eq!(value, Some("temporary".to_string()));

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let value: Option<String> = client.get("str:expire").await?;
    assert_eq!(value, None);

    // SET NX (only if not exists)
    let set1: bool = client.set_nx("str:nx", "first").await?;
    assert!(set1);
    let set2: bool = client.set_nx("str:nx", "second").await?;
    assert!(!set2);
    let value: Option<String> = client.get("str:nx").await?;
    assert_eq!(value, Some("first".to_string()));

    // INCR/DECR operations
    client.set("str:counter", "10").await?;
    let new_val: i64 = client.incr("str:counter").await?;
    assert_eq!(new_val, 11);
    let new_val: i64 = client.decr("str:counter").await?;
    assert_eq!(new_val, 10);
    let new_val: i64 = client.incr_by("str:counter", 5).await?;
    assert_eq!(new_val, 15);
    let new_val: i64 = client.decr_by("str:counter", 3).await?;
    assert_eq!(new_val, 12);

    // Multiple operations
    let keys = vec!["str:multi1".to_string(), "str:multi2".to_string()];
    client.set("str:multi1", "value1").await?;
    client.set("str:multi2", "value2").await?;

    let exists_count: i64 = client.exists(keys.clone()).await?;
    assert_eq!(exists_count, 2);

    let deleted: i64 = client.del(keys).await?;
    assert_eq!(deleted, 2);

    Ok(())
}

#[tokio::test]
async fn test_hash_operations_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client(None).await?;

    // Clean up any leftover keys
    let _ = client.del(vec!["hash:test".to_string()]).await;

    let hash_key = "hash:test";

    // HSET/HGET operations
    let set_result: i64 = client.hset(hash_key, "field1", "value1").await?;
    assert_eq!(set_result, 1); // New field

    let value: Option<String> = client.hget(hash_key, "field1").await?;
    assert_eq!(value, Some("value1".to_string()));

    // HMSET/HMGET operations
    let mut fields = HashMap::new();
    fields.insert("field2".to_string(), "value2".to_string());
    fields.insert("field3".to_string(), "value3".to_string());
    client.hmset(hash_key, fields).await?;

    let multi_values: Vec<Option<String>> = client
        .hmget(
            hash_key,
            vec![
                "field1".to_string(),
                "field2".to_string(),
                "field3".to_string(),
            ],
        )
        .await?;
    assert_eq!(
        multi_values,
        vec![
            Some("value1".to_string()),
            Some("value2".to_string()),
            Some("value3".to_string())
        ]
    );

    // HGETALL operation
    let all_fields: HashMap<String, String> = client.hgetall(hash_key).await?;
    assert_eq!(all_fields.len(), 3);
    assert_eq!(all_fields.get("field1"), Some(&"value1".to_string()));
    assert_eq!(all_fields.get("field2"), Some(&"value2".to_string()));
    assert_eq!(all_fields.get("field3"), Some(&"value3".to_string()));

    // HLEN operation
    let length: i64 = client.hlen(hash_key).await?;
    assert_eq!(length, 3);

    // HEXISTS operation
    let exists1: bool = client.hexists(hash_key, "field1").await?;
    assert!(exists1);
    let exists_nonexistent: bool = client.hexists(hash_key, "nonexistent").await?;
    assert!(!exists_nonexistent);

    // HDEL operation
    let deleted: i64 = client.hdel(hash_key, vec!["field2".to_string()]).await?;
    assert_eq!(deleted, 1);

    let length_after_del: i64 = client.hlen(hash_key).await?;
    assert_eq!(length_after_del, 2);

    Ok(())
}

#[tokio::test]
async fn test_list_operations_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client(None).await?;

    // Clean up any leftover keys
    let _ = client.del(vec!["list:test".to_string()]).await;

    let list_key = "list:test";

    // LPUSH operations
    let length1: i64 = client
        .lpush(list_key, vec!["item1".to_string(), "item2".to_string()])
        .await?;
    assert_eq!(length1, 2);

    // RPUSH operations
    let length2: i64 = client.rpush(list_key, vec!["item3".to_string()]).await?;
    assert_eq!(length2, 3);

    // LLEN operation
    let length: i64 = client.llen(list_key).await?;
    assert_eq!(length, 3);

    // LRANGE operation (get all items)
    let items: Vec<String> = client.lrange(list_key, 0, -1).await?;
    assert_eq!(items, vec!["item2", "item1", "item3"]); // LPUSH reverses order

    // LINDEX operation
    let first_item: Option<String> = client.lindex(list_key, 0).await?;
    assert_eq!(first_item, Some("item2".to_string()));

    // LSET operation
    client.lset(list_key, 1, "modified_item").await?;
    let modified_item: Option<String> = client.lindex(list_key, 1).await?;
    assert_eq!(modified_item, Some("modified_item".to_string()));

    // LPOP operation
    let popped: Option<String> = client.lpop(list_key).await?;
    assert_eq!(popped, Some("item2".to_string()));

    // RPOP operation
    let popped: Option<String> = client.rpop(list_key).await?;
    assert_eq!(popped, Some("item3".to_string()));

    // Final length check
    let final_length: i64 = client.llen(list_key).await?;
    assert_eq!(final_length, 1);

    Ok(())
}

#[tokio::test]
async fn test_set_operations_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client(None).await?;

    // Clean up any leftover keys
    let _ = client.del(vec!["set:test".to_string()]).await;

    let set_key = "set:test";

    // SADD operations
    let added1: i64 = client
        .sadd(
            set_key,
            vec![
                "member1".to_string(),
                "member2".to_string(),
                "member3".to_string(),
            ],
        )
        .await?;
    assert_eq!(added1, 3);

    // Adding duplicate member
    let added2: i64 = client.sadd(set_key, vec!["member1".to_string()]).await?;
    assert_eq!(added2, 0); // No new members added

    // SCARD operation
    let cardinality: i64 = client.scard(set_key).await?;
    assert_eq!(cardinality, 3);

    // SISMEMBER operation
    let is_member1: bool = client.sismember(set_key, "member1").await?;
    assert!(is_member1);
    let is_member_nonexistent: bool = client.sismember(set_key, "nonexistent").await?;
    assert!(!is_member_nonexistent);

    // SMEMBERS operation
    let members: HashSet<String> = client.smembers(set_key).await?;
    let expected: HashSet<String> = ["member1", "member2", "member3"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert_eq!(members, expected);

    // SRANDMEMBER operation
    let random_member: Option<String> = client.srandmember(set_key).await?;
    assert!(random_member.is_some());
    assert!(members.contains(&random_member.unwrap()));

    // SPOP operation
    let popped: Option<String> = client.spop(set_key).await?;
    assert!(popped.is_some());

    let cardinality_after_pop: i64 = client.scard(set_key).await?;
    assert_eq!(cardinality_after_pop, 2);

    // SREM operation
    let removed: i64 = client.srem(set_key, vec!["member1".to_string()]).await?;
    assert!(removed <= 1); // May be 0 if member1 was popped

    Ok(())
}

#[tokio::test]
async fn test_sorted_set_operations_comprehensive() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client(None).await?;

    // Clean up any leftover keys
    let _ = client.del(vec!["zset:test".to_string()]).await;

    let zset_key = "zset:test";

    // ZADD operations
    let mut members = HashMap::new();
    members.insert("member1".to_string(), 1.0);
    members.insert("member2".to_string(), 2.0);
    members.insert("member3".to_string(), 3.0);

    let added: i64 = client.zadd(zset_key, members).await?;
    assert_eq!(added, 3);

    // ZCARD operation
    let cardinality: i64 = client.zcard(zset_key).await?;
    assert_eq!(cardinality, 3);

    // ZSCORE operation
    let score: Option<f64> = client.zscore(zset_key, "member2").await?;
    assert_eq!(score, Some(2.0));

    // ZRANK operation
    let rank: Option<i64> = client.zrank(zset_key, "member2").await?;
    assert_eq!(rank, Some(1)); // 0-indexed, so member2 (score 2.0) is at rank 1

    // ZREVRANK operation
    let rev_rank: Option<i64> = client.zrevrank(zset_key, "member2").await?;
    assert_eq!(rev_rank, Some(1)); // From highest to lowest

    // ZRANGE operation
    let range: Vec<String> = client.zrange(zset_key, 0, -1).await?;
    assert_eq!(range, vec!["member1", "member2", "member3"]);

    // ZREM operation
    let removed: i64 = client.zrem(zset_key, vec!["member2".to_string()]).await?;
    assert_eq!(removed, 1);

    let cardinality_after_rem: i64 = client.zcard(zset_key).await?;
    assert_eq!(cardinality_after_rem, 2);

    Ok(())
}

#[tokio::test]
async fn test_operations_with_connection_pool() -> Result<(), Box<dyn std::error::Error>> {
    // Test with connection pool strategy
    let client = setup_client(Some(|config| {
        config.pool = PoolConfig {
            strategy: PoolStrategy::Pool,
            max_size: 10,
            min_idle: 2,
            connection_timeout: Duration::from_secs(5),
        };
    }))
    .await?;

    // Clean up any leftover keys
    let _ = client.del(vec!["pool:test".to_string()]).await;

    // Run basic operations to ensure pool works
    client.set("pool:test", "value").await?;
    let value: Option<String> = client.get("pool:test").await?;
    assert_eq!(value, Some("value".to_string()));

    // Test concurrent operations
    let mut handles = Vec::new();
    for i in 0..10 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let key = format!("concurrent:{}", i);
            let value = format!("value{}", i);
            client_clone.set(&key, &value).await?;
            let retrieved: Option<String> = client_clone.get(&key).await?;
            assert_eq!(retrieved, Some(value));
            Ok::<(), redis_oxide::RedisError>(())
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await??;
    }

    Ok(())
}

#[tokio::test]
async fn test_operations_with_multiplexed_connection() -> Result<(), Box<dyn std::error::Error>> {
    // Test with multiplexed strategy (default)
    let client = setup_client(Some(|config| {
        config.pool = PoolConfig {
            strategy: PoolStrategy::Multiplexed,
            max_size: 1, // Only one connection
            min_idle: 1,
            connection_timeout: Duration::from_secs(5),
        };
    }))
    .await?;

    // Clean up any leftover keys
    let _ = client
        .del(vec![
            "mux:0".to_string(),
            "mux:1".to_string(),
            "mux:2".to_string(),
            "mux:3".to_string(),
            "mux:4".to_string(),
        ])
        .await;

    // Test that multiplexed connection can handle concurrent operations
    let mut handles = Vec::new();
    for i in 0..5 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let key = format!("mux:{}", i);
            let value = format!("value{}", i);
            client_clone.set(&key, &value).await?;
            let retrieved: Option<String> = client_clone.get(&key).await?;
            assert_eq!(retrieved, Some(value));
            Ok::<(), redis_oxide::RedisError>(())
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await??;
    }

    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client(None).await?;

    // Clean up any leftover keys
    let _ = client
        .del(vec![
            "string_key".to_string(),
            "nonexistent_key".to_string(),
            "nonexistent_hash".to_string(),
        ])
        .await;

    // Test operations on wrong data types
    client.set("string_key", "string_value").await?;

    // Try to use string key as hash - should return appropriate error or empty result
    let hash_result: Option<String> = client.hget("string_key", "field").await?;
    assert_eq!(hash_result, None); // Redis returns nil for wrong type operations

    // Test operations on non-existent keys
    let nonexistent: Option<String> = client.get("nonexistent_key").await?;
    assert_eq!(nonexistent, None);

    let nonexistent_hash: HashMap<String, String> = client.hgetall("nonexistent_hash").await?;
    assert!(nonexistent_hash.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_expiration_and_ttl() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client(None).await?;

    // Clean up any leftover keys
    let _ = client
        .del(vec!["expire_test".to_string(), "expire_test2".to_string()])
        .await;

    // Set key with expiration
    client
        .set_ex("expire_test", "value", Duration::from_secs(2))
        .await?;

    // Check TTL
    let ttl: Option<i64> = client.ttl("expire_test").await?;
    assert!(ttl.is_some());
    assert!(ttl.unwrap() > 0 && ttl.unwrap() <= 2);

    // Set expiration on existing key
    client.set("expire_test2", "value").await?;
    client
        .expire("expire_test2", Duration::from_secs(1))
        .await?;

    let ttl2: Option<i64> = client.ttl("expire_test2").await?;
    assert!(ttl2.is_some());
    assert!(ttl2.unwrap() > 0 && ttl2.unwrap() <= 1);

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let expired: Option<String> = client.get("expire_test2").await?;
    assert_eq!(expired, None);

    // Check TTL of expired key
    let ttl_expired: Option<i64> = client.ttl("expire_test2").await?;
    assert_eq!(ttl_expired, Some(-2)); // -2 indicates key doesn't exist

    Ok(())
}
