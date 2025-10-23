//! Simple integration tests without external dependencies

#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]

use redis_oxide::{
    commands::*,
    protocol::resp3::Resp3Value,
    script::{Script, ScriptManager},
    sentinel::{MasterInfo, SentinelConfig, SentinelEndpoint},
    streams::{ReadOptions, StreamEntry, StreamRange},
    ConnectionConfig, PoolConfig, PoolStrategy, ProtocolVersion, RedisError, RedisResult,
    TopologyMode,
};
use std::collections::HashMap;
use std::time::Duration;

// Mock tests that verify command building and parsing logic
// These tests don't require actual Redis server

#[tokio::test]
async fn test_command_building() -> Result<(), Box<dyn std::error::Error>> {
    // Test that commands can be built correctly
    use redis_oxide::commands::*;

    // Test GET command
    let get_cmd = GetCommand::new("test_key");
    assert_eq!(get_cmd.command_name(), "GET");
    assert_eq!(get_cmd.keys(), vec![b"test_key"]);

    // Test SET command
    let set_cmd = SetCommand::new("test_key", "test_value");
    assert_eq!(set_cmd.command_name(), "SET");
    assert_eq!(set_cmd.keys(), vec![b"test_key"]);

    // Test Hash commands
    let hset_cmd = HSetCommand::new("hash_key", "field", "value");
    assert_eq!(hset_cmd.command_name(), "HSET");

    let hget_command = HGetCommand::new("hash_key", "field");
    assert_eq!(hget_command.command_name(), "HGET");

    // Test List commands
    let lpush_cmd = LPushCommand::new("list_key", vec!["item1".to_string()]);
    assert_eq!(lpush_cmd.command_name(), "LPUSH");

    let lrange_cmd = LRangeCommand::new("list_key", 0, -1);
    assert_eq!(lrange_cmd.command_name(), "LRANGE");

    // Test Set commands
    let sadd_cmd = SAddCommand::new("set_key", vec!["member1".to_string()]);
    assert_eq!(sadd_cmd.command_name(), "SADD");

    let smembers_cmd = SMembersCommand::new("set_key");
    assert_eq!(smembers_cmd.command_name(), "SMEMBERS");

    // Test Sorted Set commands
    let mut members = HashMap::new();
    members.insert("member1".to_string(), 1.0);
    let zadd_cmd = ZAddCommand::new("zset_key", members);
    assert_eq!(zadd_cmd.command_name(), "ZADD");

    let zrange_cmd = ZRangeCommand::new("zset_key", 0, -1);
    assert_eq!(zrange_cmd.command_name(), "ZRANGE");

    Ok(())
}

#[tokio::test]
async fn test_response_parsing() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::core::value::RespValue;

    // Test GET command response parsing
    let get_cmd = GetCommand::new("test_key");

    // Test successful response
    let success_response = RespValue::BulkString(bytes::Bytes::from("test_value"));
    let parsed = get_cmd.parse_response(success_response)?;
    assert_eq!(parsed, Some("test_value".to_string()));

    // Test null response
    let null_response = RespValue::Null;
    let parsed = get_cmd.parse_response(null_response)?;
    assert_eq!(parsed, None);

    // Test SET command response parsing
    let set_cmd = SetCommand::new("test_key", "test_value");
    let ok_response = RespValue::SimpleString("OK".to_string());
    let parsed = set_cmd.parse_response(ok_response)?;
    assert!(parsed);

    // Test INCR command response parsing
    let incr_cmd = IncrCommand::new("counter");
    let int_response = RespValue::Integer(42);
    let parsed = incr_cmd.parse_response(int_response)?;
    assert_eq!(parsed, 42);

    Ok(())
}

#[tokio::test]
async fn test_pipeline_command_trait() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::pipeline::PipelineCommand;

    // Test that all commands implement PipelineCommand
    let get_cmd = GetCommand::new("test_key");
    assert_eq!(get_cmd.name(), "GET");
    assert_eq!(get_cmd.key(), Some("test_key".to_string()));

    let set_cmd = SetCommand::new("test_key", "test_value");
    assert_eq!(set_cmd.name(), "SET");
    assert_eq!(set_cmd.key(), Some("test_key".to_string()));

    let hset_cmd = HSetCommand::new("hash_key", "field", "value");
    assert_eq!(hset_cmd.name(), "HSET");
    assert_eq!(hset_cmd.key(), Some("hash_key".to_string()));

    let lpush_cmd = LPushCommand::new("list_key", vec!["item".to_string()]);
    assert_eq!(lpush_cmd.name(), "LPUSH");
    assert_eq!(lpush_cmd.key(), Some("list_key".to_string()));

    let sadd_cmd = SAddCommand::new("set_key", vec!["member".to_string()]);
    assert_eq!(sadd_cmd.name(), "SADD");
    assert_eq!(sadd_cmd.key(), Some("set_key".to_string()));

    let mut members = std::collections::HashMap::new();
    members.insert("member".to_string(), 1.0);
    let zadd_cmd = ZAddCommand::new("zset_key", members);
    assert_eq!(zadd_cmd.name(), "ZADD");
    assert_eq!(zadd_cmd.key(), Some("zset_key".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_configuration_building() -> Result<(), Box<dyn std::error::Error>> {
    // Test basic configuration
    let config = ConnectionConfig::new("redis://localhost:6379");
    assert_eq!(config.connection_string, "redis://localhost:6379");
    assert_eq!(config.database, 0);
    assert_eq!(config.topology_mode, TopologyMode::Auto);
    assert_eq!(config.protocol_version, ProtocolVersion::Resp2);

    // Test configuration with options
    let config = ConnectionConfig::new("redis://localhost:6379")
        .with_password("secret")
        .with_database(1)
        .with_connect_timeout(Duration::from_secs(10))
        .with_operation_timeout(Duration::from_secs(60))
        .with_topology_mode(TopologyMode::Standalone)
        .with_protocol_version(ProtocolVersion::Resp3)
        .with_max_redirects(5);

    assert_eq!(config.password, Some("secret".to_string()));
    assert_eq!(config.database, 1);
    assert_eq!(config.connect_timeout, Duration::from_secs(10));
    assert_eq!(config.operation_timeout, Duration::from_secs(60));
    assert_eq!(config.topology_mode, TopologyMode::Standalone);
    assert_eq!(config.protocol_version, ProtocolVersion::Resp3);
    assert_eq!(config.max_redirects, 5);

    // Test pool configuration
    let pool_config = PoolConfig {
        strategy: PoolStrategy::Pool,
        max_size: 20,
        min_idle: 5,
        connection_timeout: Duration::from_secs(5),
    };

    let config = ConnectionConfig::new("redis://localhost:6379").with_pool_config(pool_config);

    assert_eq!(config.pool.strategy, PoolStrategy::Pool);
    assert_eq!(config.pool.max_size, 20);
    assert_eq!(config.pool.min_idle, 5);

    Ok(())
}

#[tokio::test]
async fn test_error_types() -> Result<(), Box<dyn std::error::Error>> {
    // Test error creation and display
    let connection_error = RedisError::Connection("Connection failed".to_string());
    assert!(connection_error.to_string().contains("Connection failed"));

    let protocol_error = RedisError::Protocol("Invalid protocol".to_string());
    assert!(protocol_error.to_string().contains("Invalid protocol"));

    let cluster_error = RedisError::Cluster("Cluster error".to_string());
    assert!(cluster_error.to_string().contains("Cluster error"));

    let sentinel_error = RedisError::Sentinel("Sentinel error".to_string());
    assert!(sentinel_error.to_string().contains("Sentinel error"));

    // Test error conversion
    let result: RedisResult<String> = Err(connection_error);
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_resp3_value_types() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashSet;

    // Test basic types
    let bool_val = Resp3Value::Boolean(true);
    assert_eq!(bool_val.type_name(), "boolean");
    assert!(bool_val.as_bool()?);

    let double_val = Resp3Value::Double(std::f64::consts::PI);
    assert_eq!(double_val.type_name(), "double");
    assert!((double_val.as_float()? - std::f64::consts::PI).abs() < f64::EPSILON);

    let big_num = Resp3Value::BigNumber("123456789012345678901234567890".to_string());
    assert_eq!(big_num.type_name(), "big-number");
    assert_eq!(big_num.as_string()?, "123456789012345678901234567890");

    // Test complex types
    let mut map = HashMap::new();
    map.insert(
        "key".to_string(),
        Resp3Value::SimpleString("value".to_string()),
    );
    let map_val = Resp3Value::Map(map);
    assert_eq!(map_val.type_name(), "map");

    let mut set = HashSet::new();
    set.insert(Resp3Value::SimpleString("item".to_string()));
    let set_val = Resp3Value::Set(set);
    assert_eq!(set_val.type_name(), "set");

    // Test null
    let null_val = Resp3Value::Null;
    assert!(null_val.is_null());
    assert_eq!(null_val.type_name(), "null");

    Ok(())
}

#[tokio::test]
async fn test_streams_types() -> Result<(), Box<dyn std::error::Error>> {
    // Test StreamEntry
    let mut fields = HashMap::new();
    fields.insert("user_id".to_string(), "123".to_string());
    fields.insert("action".to_string(), "login".to_string());

    let entry = StreamEntry::new("1234567890123-0".to_string(), fields);
    assert_eq!(entry.id, "1234567890123-0");
    assert_eq!(entry.get_field("user_id"), Some(&"123".to_string()));
    assert_eq!(entry.get_field("action"), Some(&"login".to_string()));
    assert_eq!(entry.get_field("nonexistent"), None);
    assert!(entry.has_field("user_id"));
    assert!(!entry.has_field("nonexistent"));

    // Test StreamRange
    let range = StreamRange::all();
    assert_eq!(range.start, "-");
    assert_eq!(range.end, "+");

    let range = StreamRange::from("1234567890123-0");
    assert_eq!(range.start, "1234567890123-0");
    assert_eq!(range.end, "+");

    let range = StreamRange::new("start", "end");
    assert_eq!(range.start, "start");
    assert_eq!(range.end, "end");

    // Test ReadOptions
    let options = ReadOptions::new();
    assert_eq!(options.count, None);
    assert_eq!(options.block, None);

    let options = ReadOptions::new()
        .with_count(10)
        .with_block(std::time::Duration::from_secs(1));
    assert_eq!(options.count, Some(10));
    assert_eq!(options.block, Some(std::time::Duration::from_secs(1)));

    Ok(())
}

#[tokio::test]
async fn test_script_types() -> Result<(), Box<dyn std::error::Error>> {
    // Test Script
    let script = Script::new("return 'hello'");
    assert_eq!(script.source(), "return 'hello'");
    assert!(!script.sha().is_empty());
    assert_eq!(script.sha().len(), 40); // SHA1 is 40 characters

    // Test ScriptManager
    let manager = ScriptManager::new();
    assert!(manager.is_empty().await);
    assert_eq!(manager.len().await, 0);

    let script1 = Script::new("return 'script1'");
    let script2 = Script::new("return 'script2'");

    manager.register("script1", script1).await;
    manager.register("script2", script2).await;

    assert!(!manager.is_empty().await);
    assert_eq!(manager.len().await, 2);

    let script_list = manager.list_scripts().await;
    assert_eq!(script_list.len(), 2);
    assert!(script_list.contains(&"script1".to_string()));
    assert!(script_list.contains(&"script2".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_sentinel_types() -> Result<(), Box<dyn std::error::Error>> {
    // Test SentinelEndpoint
    let endpoint = SentinelEndpoint::new("localhost", 26379);
    assert_eq!(endpoint.host, "localhost");
    assert_eq!(endpoint.port, 26379);
    assert_eq!(endpoint.address(), "localhost:26379");

    let endpoint = SentinelEndpoint::from_address("127.0.0.1:26379")?;
    assert_eq!(endpoint.host, "127.0.0.1");
    assert_eq!(endpoint.port, 26379);

    // Test SentinelConfig
    let config = SentinelConfig::new("mymaster")
        .add_sentinel("127.0.0.1:26379")
        .add_sentinel("127.0.0.1:26380")
        .with_password("secret")
        .with_failover_timeout(Duration::from_secs(30))
        .with_max_retries(5);

    assert_eq!(config.master_name, "mymaster");
    assert_eq!(config.sentinels.len(), 2);
    assert_eq!(config.password, Some("secret".to_string()));
    assert_eq!(config.failover_timeout, Duration::from_secs(30));
    assert_eq!(config.max_retries, 5);

    // Test MasterInfo
    let master = MasterInfo {
        name: "mymaster".to_string(),
        host: "127.0.0.1".to_string(),
        port: 6379,
        flags: vec!["master".to_string()],
        num_slaves: 2,
        num_other_sentinels: 2,
        quorum: 2,
        failover_timeout: Duration::from_secs(60),
        parallel_syncs: 1,
    };

    assert_eq!(master.address(), "127.0.0.1:6379");
    assert!(!master.is_down());
    assert!(!master.is_failover_in_progress());

    Ok(())
}
