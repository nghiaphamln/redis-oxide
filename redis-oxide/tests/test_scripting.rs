//! Integration tests for Lua scripting functionality

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::uninlined_format_args)]

use redis_oxide::{Client, ConnectionConfig, Script, ScriptManager};

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

async fn setup_client() -> Result<Client, redis_oxide::RedisError> {
    let config = ConnectionConfig::new(redis_url().as_str());
    Client::connect(config).await
}

#[tokio::test]
async fn test_basic_script_execution() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Simple script that returns a string
    let script = "return 'Hello, World!'";
    let result: String = client.eval(script, vec![], vec![]).await?;
    assert_eq!(result, "Hello, World!");

    // Script that uses KEYS
    let script = "return redis.call('GET', KEYS[1])";
    client.set("test_key", "test_value").await?;
    let result: Option<String> = client
        .eval(script, vec!["test_key".to_string()], vec![])
        .await?;
    assert_eq!(result, Some("test_value".to_string()));

    // Script that uses ARGV
    let script = "return ARGV[1] .. ':' .. ARGV[2]";
    let result: String = client
        .eval(
            script,
            vec![],
            vec!["hello".to_string(), "world".to_string()],
        )
        .await?;
    assert_eq!(result, "hello:world");

    Ok(())
}

#[tokio::test]
async fn test_script_with_redis_operations() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Script that performs SET and GET operations
    let script = r#"
        redis.call('SET', KEYS[1], ARGV[1])
        return redis.call('GET', KEYS[1])
    "#;

    let result: String = client
        .eval(
            script,
            vec!["script_key".to_string()],
            vec!["script_value".to_string()],
        )
        .await?;
    assert_eq!(result, "script_value");

    // Verify the key was actually set
    let value: Option<String> = client.get("script_key").await?;
    assert_eq!(value, Some("script_value".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_script_load_and_evalsha() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Load a script
    let script = "return KEYS[1] .. ':' .. ARGV[1]";
    let sha = client.script_load(script).await?;
    assert_eq!(sha.len(), 40); // SHA1 hash is 40 characters

    // Execute using EVALSHA
    let result: String = client
        .evalsha(&sha, vec!["test".to_string()], vec!["value".to_string()])
        .await?;
    assert_eq!(result, "test:value");

    // Check if script exists
    let exists = client.script_exists(vec![sha.clone()]).await?;
    assert_eq!(exists, vec![true]);

    // Check non-existent script
    let fake_sha = "0000000000000000000000000000000000000000";
    let exists_fake = client.script_exists(vec![fake_sha.to_string()]).await?;
    assert_eq!(exists_fake, vec![false]);

    Ok(())
}

#[tokio::test]
async fn test_script_struct() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Create a Script instance
    let script = Script::new("return KEYS[1] .. ':' .. ARGV[1]");
    assert!(!script.sha().is_empty());
    assert_eq!(script.source(), "return KEYS[1] .. ':' .. ARGV[1]");

    // Execute the script (will use EVAL first time)
    let result: String = script
        .execute(
            &client,
            vec!["key1".to_string()],
            vec!["value1".to_string()],
        )
        .await?;
    assert_eq!(result, "key1:value1");

    // Load the script explicitly
    let sha = script.load(&client).await?;
    assert_eq!(sha, script.sha());

    // Execute again (should use EVALSHA now)
    let result2: String = script
        .execute(
            &client,
            vec!["key2".to_string()],
            vec!["value2".to_string()],
        )
        .await?;
    assert_eq!(result2, "key2:value2");

    Ok(())
}

#[tokio::test]
async fn test_script_manager() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let manager = ScriptManager::new();

    // Register scripts
    let script1 = Script::new("return 'script1:' .. ARGV[1]");
    let script2 = Script::new("return 'script2:' .. ARGV[1]");

    manager.register("test_script1", script1).await;
    manager.register("test_script2", script2).await;

    assert_eq!(manager.len().await, 2);
    assert!(!manager.is_empty().await);

    // Execute scripts by name
    let result1: String = manager
        .execute("test_script1", &client, vec![], vec!["hello".to_string()])
        .await?;
    assert_eq!(result1, "script1:hello");

    let result2: String = manager
        .execute("test_script2", &client, vec![], vec!["world".to_string()])
        .await?;
    assert_eq!(result2, "script2:world");

    // Load all scripts
    let loaded = manager.load_all(&client).await?;
    assert_eq!(loaded.len(), 2);
    assert!(loaded.contains_key("test_script1"));
    assert!(loaded.contains_key("test_script2"));

    // List scripts
    let script_names = manager.list_scripts().await;
    assert_eq!(script_names.len(), 2);
    assert!(script_names.contains(&"test_script1".to_string()));
    assert!(script_names.contains(&"test_script2".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_atomic_increment_script() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Atomic increment with expiration script
    let script = r#"
        local key = KEYS[1]
        local increment = tonumber(ARGV[1])
        local expiration = tonumber(ARGV[2])
        
        local current = redis.call('GET', key) or 0
        local new_value = tonumber(current) + increment
        redis.call('SET', key, new_value)
        redis.call('EXPIRE', key, expiration)
        
        return new_value
    "#;

    // First increment
    let result1: i64 = client
        .eval(
            script,
            vec!["counter".to_string()],
            vec!["5".to_string(), "60".to_string()],
        )
        .await?;
    assert_eq!(result1, 5);

    // Second increment
    let result2: i64 = client
        .eval(
            script,
            vec!["counter".to_string()],
            vec!["3".to_string(), "60".to_string()],
        )
        .await?;
    assert_eq!(result2, 8);

    // Verify the key exists and has correct value
    let value: Option<String> = client.get("counter").await?;
    assert_eq!(value, Some("8".to_string()));

    // Verify TTL is set
    let ttl: Option<i64> = client.ttl("counter").await?;
    assert!(ttl.is_some());
    assert!(ttl.unwrap() > 0);

    Ok(())
}

#[tokio::test]
async fn test_conditional_set_script() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Conditional set script
    let script = r#"
        local key = KEYS[1]
        local expected = ARGV[1]
        local new_value = ARGV[2]
        
        local current = redis.call('GET', key)
        
        if current == expected then
            redis.call('SET', key, new_value)
            return 1
        else
            return 0
        end
    "#;

    // Set initial value
    client.set("conditional_key", "initial").await?;

    // Successful conditional set
    let result1: i64 = client
        .eval(
            script,
            vec!["conditional_key".to_string()],
            vec!["initial".to_string(), "updated".to_string()],
        )
        .await?;
    assert_eq!(result1, 1);

    // Verify value was updated
    let value: Option<String> = client.get("conditional_key").await?;
    assert_eq!(value, Some("updated".to_string()));

    // Failed conditional set (wrong expected value)
    let result2: i64 = client
        .eval(
            script,
            vec!["conditional_key".to_string()],
            vec!["wrong".to_string(), "failed".to_string()],
        )
        .await?;
    assert_eq!(result2, 0);

    // Verify value was not changed
    let value2: Option<String> = client.get("conditional_key").await?;
    assert_eq!(value2, Some("updated".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_script_with_multiple_keys() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Script that operates on multiple keys
    let script = r#"
        local key1 = KEYS[1]
        local key2 = KEYS[2]
        local value = ARGV[1]
        
        redis.call('SET', key1, value)
        redis.call('SET', key2, value)
        
        return redis.call('MGET', key1, key2)
    "#;

    let result: Vec<String> = client
        .eval(
            script,
            vec!["multi_key1".to_string(), "multi_key2".to_string()],
            vec!["shared_value".to_string()],
        )
        .await?;

    assert_eq!(result, vec!["shared_value", "shared_value"]);

    // Verify keys were set
    let value1: Option<String> = client.get("multi_key1").await?;
    let value2: Option<String> = client.get("multi_key2").await?;
    assert_eq!(value1, Some("shared_value".to_string()));
    assert_eq!(value2, Some("shared_value".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_script_flush() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Load a script
    let script = "return 'test'";
    let sha = client.script_load(script).await?;

    // Verify script exists
    let exists_before = client.script_exists(vec![sha.clone()]).await?;
    assert_eq!(exists_before, vec![true]);

    // Flush all scripts
    client.script_flush().await?;

    // Verify script no longer exists
    let exists_after = client.script_exists(vec![sha]).await?;
    assert_eq!(exists_after, vec![false]);

    Ok(())
}

#[tokio::test]
async fn test_script_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Try to execute EVALSHA with non-existent script
    let fake_sha = "0000000000000000000000000000000000000000";
    let result = client
        .evalsha::<String>(fake_sha, vec!["key".to_string()], vec!["value".to_string()])
        .await;

    assert!(result.is_err());
    // The error should contain "NOSCRIPT" or similar

    // Test script with syntax error
    let bad_script = "return invalid lua syntax !!!";
    let result = client.eval::<String>(bad_script, vec![], vec![]).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_script_pattern_examples() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Test rate limiting pattern
    let rate_limit_script = r#"
        local key = KEYS[1]
        local window = tonumber(ARGV[1])
        local limit = tonumber(ARGV[2])
        local now = redis.call('TIME')[1]
        
        -- Remove old entries
        redis.call('ZREMRANGEBYSCORE', key, 0, now - window)
        
        -- Count current entries
        local current = redis.call('ZCARD', key)
        
        if current < limit then
            -- Add current request
            redis.call('ZADD', key, now, now)
            redis.call('EXPIRE', key, window)
            return {1, limit - current - 1}
        else
            return {0, 0}
        end
    "#;

    // Test rate limiting
    let result: Vec<i64> = client
        .eval(
            rate_limit_script,
            vec!["rate_limit:user1".to_string()],
            vec!["60".to_string(), "5".to_string()], // 5 requests per 60 seconds
        )
        .await?;

    assert_eq!(result[0], 1); // Request allowed
    assert_eq!(result[1], 4); // 4 requests remaining

    Ok(())
}
