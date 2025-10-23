//! Comprehensive demo of redis-oxide features
//!
//! This example demonstrates all major features of redis-oxide:
//! - String, Hash, List, Set, and Sorted Set operations
//! - Pipeline support for batch operations
//! - Transactions with MULTI/EXEC/WATCH
//! - Pub/Sub messaging
//! - Lua scripting with EVAL/EVALSHA
//! - Redis Streams with consumer groups
//! - RESP3 protocol support
//! - Error handling and configuration

use redis_oxide::{
    Client, ConnectionConfig, PoolConfig, PoolStrategy, ProtocolVersion, 
    TransactionResult, Script, ScriptManager
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better debugging
    tracing_subscriber::init();

    println!("🚀 redis-oxide Comprehensive Demo");
    println!("==================================\n");

    // Configuration with RESP3 and connection pool
    let config = ConnectionConfig::new("redis://localhost:6379")
        .with_protocol_version(ProtocolVersion::Resp3)
        .with_pool_config(PoolConfig {
            strategy: PoolStrategy::Multiplexed,
            max_size: 10,
            min_idle: 2,
            connection_timeout: Duration::from_secs(5),
        })
        .with_operation_timeout(Duration::from_secs(30));

    let client = Client::connect(config).await?;
    println!("✅ Connected to Redis ({})", client.topology_type());

    // Demo all features
    demo_string_operations(&client).await?;
    demo_hash_operations(&client).await?;
    demo_list_operations(&client).await?;
    demo_set_operations(&client).await?;
    demo_sorted_set_operations(&client).await?;
    demo_pipeline_operations(&client).await?;
    demo_transaction_operations(&client).await?;
    demo_lua_scripting(&client).await?;
    demo_redis_streams(&client).await?;
    demo_error_handling(&client).await?;

    println!("\n🎉 All demos completed successfully!");
    Ok(())
}

async fn demo_string_operations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 String Operations Demo");
    println!("-----------------------");

    // Basic SET/GET
    client.set("demo:string", "Hello, redis-oxide!").await?;
    let value: Option<String> = client.get("demo:string").await?;
    println!("SET/GET: {:?}", value);

    // SET with expiration
    client.set_ex("demo:temp", "temporary", Duration::from_secs(2)).await?;
    let ttl = client.ttl("demo:temp").await?;
    println!("SET EX: TTL = {:?} seconds", ttl);

    // SET NX (only if not exists)
    let set1 = client.set_nx("demo:nx", "first").await?;
    let set2 = client.set_nx("demo:nx", "second").await?;
    println!("SET NX: first={}, second={}", set1, set2);

    // Increment operations
    client.set("demo:counter", "10").await?;
    let val = client.incr("demo:counter").await?;
    println!("INCR: {}", val);
    let val = client.incr_by("demo:counter", 5).await?;
    println!("INCRBY 5: {}", val);

    // Multiple key operations
    let exists = client.exists(vec!["demo:string".to_string(), "demo:counter".to_string()]).await?;
    println!("EXISTS (2 keys): {}", exists);

    println!();
    Ok(())
}

async fn demo_hash_operations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("🗂️  Hash Operations Demo");
    println!("----------------------");

    let hash_key = "demo:user:1";

    // Set individual fields
    client.hset(hash_key, "name", "Alice").await?;
    client.hset(hash_key, "age", "30").await?;
    client.hset(hash_key, "city", "New York").await?;

    // Get individual field
    let name: Option<String> = client.hget(hash_key, "name").await?;
    println!("HGET name: {:?}", name);

    // Get all fields
    let all_fields: HashMap<String, String> = client.hgetall(hash_key).await?;
    println!("HGETALL: {:?}", all_fields);

    // Multiple field operations
    let fields = vec!["name".to_string(), "age".to_string()];
    let values: Vec<Option<String>> = client.hmget(hash_key, fields).await?;
    println!("HMGET: {:?}", values);

    // Hash utilities
    let len = client.hlen(hash_key).await?;
    let exists = client.hexists(hash_key, "email").await?;
    println!("HLEN: {}, HEXISTS email: {}", len, exists);

    println!();
    Ok(())
}

async fn demo_list_operations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 List Operations Demo");
    println!("---------------------");

    let list_key = "demo:tasks";

    // Push operations
    let len = client.lpush(list_key, vec!["task1".to_string(), "task2".to_string()]).await?;
    println!("LPUSH: length = {}", len);

    let len = client.rpush(list_key, vec!["task3".to_string()]).await?;
    println!("RPUSH: length = {}", len);

    // Range operations
    let items: Vec<String> = client.lrange(list_key, 0, -1).await?;
    println!("LRANGE: {:?}", items);

    // Index operations
    let first: Option<String> = client.lindex(list_key, 0).await?;
    println!("LINDEX 0: {:?}", first);

    client.lset(list_key, 1, "modified_task").await?;
    let modified: Option<String> = client.lindex(list_key, 1).await?;
    println!("LSET then LINDEX 1: {:?}", modified);

    // Pop operations
    let popped = client.lpop(list_key).await?;
    println!("LPOP: {:?}", popped);

    let final_len = client.llen(list_key).await?;
    println!("Final LLEN: {}", final_len);

    println!();
    Ok(())
}

async fn demo_set_operations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Set Operations Demo");
    println!("--------------------");

    let set_key = "demo:tags";

    // Add members
    let added = client.sadd(set_key, vec!["rust".to_string(), "redis".to_string(), "async".to_string()]).await?;
    println!("SADD: {} members added", added);

    // Get all members
    let members = client.smembers(set_key).await?;
    println!("SMEMBERS: {:?}", members);

    // Check membership
    let is_member = client.sismember(set_key, "rust").await?;
    println!("SISMEMBER rust: {}", is_member);

    // Set cardinality
    let cardinality = client.scard(set_key).await?;
    println!("SCARD: {}", cardinality);

    // Random member
    let random = client.srandmember(set_key).await?;
    println!("SRANDMEMBER: {:?}", random);

    // Remove member
    let removed = client.srem(set_key, vec!["async".to_string()]).await?;
    println!("SREM: {} members removed", removed);

    println!();
    Ok(())
}

async fn demo_sorted_set_operations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Sorted Set Operations Demo");
    println!("----------------------------");

    let zset_key = "demo:leaderboard";

    // Add members with scores
    let mut members = HashMap::new();
    members.insert("alice".to_string(), 100.0);
    members.insert("bob".to_string(), 85.0);
    members.insert("charlie".to_string(), 92.0);

    let added = client.zadd(zset_key, members).await?;
    println!("ZADD: {} members added", added);

    // Get range (sorted by score)
    let range: Vec<String> = client.zrange(zset_key, 0, -1).await?;
    println!("ZRANGE: {:?}", range);

    // Get score and rank
    let score = client.zscore(zset_key, "alice").await?;
    let rank = client.zrank(zset_key, "alice").await?;
    let rev_rank = client.zrevrank(zset_key, "alice").await?;
    println!("alice - score: {:?}, rank: {:?}, rev_rank: {:?}", score, rank, rev_rank);

    // Cardinality
    let cardinality = client.zcard(zset_key).await?;
    println!("ZCARD: {}", cardinality);

    println!();
    Ok(())
}

async fn demo_pipeline_operations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Pipeline Operations Demo");
    println!("-------------------------");

    // Create pipeline with multiple commands
    let mut pipeline = client.pipeline();
    pipeline.set("pipe:key1", "value1");
    pipeline.set("pipe:key2", "value2");
    pipeline.get("pipe:key1");
    pipeline.get("pipe:key2");
    pipeline.incr("pipe:counter");
    pipeline.hset("pipe:hash", "field", "value");

    // Execute all commands in one round-trip
    let results = pipeline.execute().await?;
    println!("Pipeline executed {} commands", results.len());
    
    for (i, result) in results.iter().enumerate() {
        println!("  Result {}: {:?}", i, result);
    }

    println!();
    Ok(())
}

async fn demo_transaction_operations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("💳 Transaction Operations Demo");
    println!("----------------------------");

    // Set up initial state
    client.set("tx:balance", "100").await?;

    // Transaction with WATCH
    let mut transaction = client.transaction().await?;
    transaction.watch(vec!["tx:balance".to_string()]).await?;

    // Queue commands
    transaction.get("tx:balance");
    transaction.set("tx:balance", "80");  // Withdraw 20
    transaction.set("tx:last_transaction", "withdrawal");

    // Execute transaction
    match transaction.exec().await? {
        TransactionResult::Success(results) => {
            println!("Transaction succeeded with {} results", results.len());
            for (i, result) in results.iter().enumerate() {
                println!("  Result {}: {:?}", i, result);
            }
        }
        TransactionResult::Aborted => {
            println!("Transaction was aborted (watched key changed)");
        }
    }

    // Verify final state
    let balance: Option<String> = client.get("tx:balance").await?;
    println!("Final balance: {:?}", balance);

    println!();
    Ok(())
}

async fn demo_lua_scripting(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧮 Lua Scripting Demo");
    println!("--------------------");

    // Atomic increment with expiration script
    let script_source = r#"
        local key = KEYS[1]
        local increment = tonumber(ARGV[1])
        local expiration = tonumber(ARGV[2])
        
        local current = redis.call('GET', key) or 0
        local new_value = tonumber(current) + increment
        redis.call('SET', key, new_value)
        redis.call('EXPIRE', key, expiration)
        
        return new_value
    "#;

    // Execute with EVAL
    let result: i64 = client.eval(
        script_source,
        vec!["script:counter".to_string()],
        vec!["5".to_string(), "60".to_string()]
    ).await?;
    println!("EVAL result: {}", result);

    // Load script and execute with EVALSHA
    let sha = client.script_load(script_source).await?;
    println!("Script loaded with SHA: {}", sha);

    let result: i64 = client.evalsha(
        &sha,
        vec!["script:counter".to_string()],
        vec!["3".to_string(), "60".to_string()]
    ).await?;
    println!("EVALSHA result: {}", result);

    // Script management
    let script = Script::new(script_source);
    let manager = ScriptManager::new();
    manager.register("atomic_incr", script).await;

    let result: i64 = manager.execute(
        "atomic_incr",
        &client,
        vec!["script:counter".to_string()],
        vec!["2".to_string(), "60".to_string()]
    ).await?;
    println!("ScriptManager result: {}", result);

    println!();
    Ok(())
}

async fn demo_redis_streams(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("🌊 Redis Streams Demo");
    println!("--------------------");

    let stream_name = "demo:events";
    let group_name = "demo_processors";
    let consumer_name = "demo_worker";

    // Add entries to stream
    let mut fields1 = HashMap::new();
    fields1.insert("user_id".to_string(), "123".to_string());
    fields1.insert("action".to_string(), "login".to_string());
    fields1.insert("timestamp".to_string(), "1234567890".to_string());

    let entry_id1 = client.xadd(stream_name, "*", fields1).await?;
    println!("XADD entry 1: {}", entry_id1);

    let mut fields2 = HashMap::new();
    fields2.insert("user_id".to_string(), "456".to_string());
    fields2.insert("action".to_string(), "logout".to_string());

    let entry_id2 = client.xadd(stream_name, "*", fields2).await?;
    println!("XADD entry 2: {}", entry_id2);

    // Stream length
    let length = client.xlen(stream_name).await?;
    println!("XLEN: {}", length);

    // Read range
    let entries = client.xrange(stream_name, "-", "+", Some(10)).await?;
    println!("XRANGE: {} entries", entries.len());
    for entry in &entries {
        println!("  {}: {:?}", entry.id, entry.fields);
    }

    // Create consumer group
    client.xgroup_create(stream_name, group_name, "0", false).await?;
    println!("Consumer group '{}' created", group_name);

    // Read from consumer group
    let streams = vec![(stream_name.to_string(), ">".to_string())];
    let messages = client.xreadgroup(
        group_name,
        consumer_name,
        streams,
        Some(10),
        None
    ).await?;

    println!("XREADGROUP: {} streams", messages.len());
    for (stream, entries) in &messages {
        println!("  Stream {}: {} entries", stream, entries.len());
        for entry in entries {
            println!("    {}: {:?}", entry.id, entry.fields);
        }
    }

    // Acknowledge messages
    if let Some(entries) = messages.get(stream_name) {
        if !entries.is_empty() {
            let ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
            let acked = client.xack(stream_name, group_name, ids).await?;
            println!("XACK: {} messages acknowledged", acked);
        }
    }

    println!();
    Ok(())
}

async fn demo_error_handling(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚠️  Error Handling Demo");
    println!("----------------------");

    // Demonstrate different error types
    use redis_oxide::RedisError;

    // Try to get a non-existent key (not an error, returns None)
    let result: Option<String> = client.get("nonexistent:key").await?;
    println!("GET nonexistent key: {:?}", result);

    // Try operations on wrong data types
    client.set("error:string_key", "string_value").await?;
    
    // This will return None/empty rather than error (Redis behavior)
    let hash_result: Option<String> = client.hget("error:string_key", "field").await?;
    println!("HGET on string key: {:?}", hash_result);

    // Demonstrate error matching
    match client.get("some:key").await {
        Ok(Some(value)) => println!("Found value: {}", value),
        Ok(None) => println!("Key not found (this is normal)"),
        Err(RedisError::Connection(msg)) => println!("Connection error: {}", msg),
        Err(RedisError::Protocol(msg)) => println!("Protocol error: {}", msg),
        Err(RedisError::Cluster(msg)) => println!("Cluster error: {}", msg),
        Err(RedisError::Sentinel(msg)) => println!("Sentinel error: {}", msg),
        Err(e) => println!("Other error: {}", e),
    }

    println!("✅ Error handling demo completed");
    println!();
    Ok(())
}
