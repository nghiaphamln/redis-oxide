//! Integration tests for Redis Streams functionality

#![allow(clippy::similar_names)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::collection_is_never_read)]
#![allow(clippy::manual_string_new)]
#![allow(clippy::absurd_extreme_comparisons)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::cast_possible_truncation)]

use redis_oxide::{Client, ConnectionConfig};
use std::collections::HashMap;
use std::time::Duration;

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

async fn setup_client() -> Result<Client, redis_oxide::RedisError> {
    let config = ConnectionConfig::new(redis_url().as_str());
    Client::connect(config).await
}

#[tokio::test]
async fn test_basic_stream_operations() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let stream_name = "test_stream";

    // Add entries to stream
    let mut fields1 = HashMap::new();
    fields1.insert("user_id".to_string(), "123".to_string());
    fields1.insert("action".to_string(), "login".to_string());
    fields1.insert("timestamp".to_string(), "1234567890".to_string());

    let entry_id1 = client.xadd(stream_name, "*", fields1).await?;
    assert!(!entry_id1.is_empty());
    assert!(entry_id1.contains('-')); // Format: timestamp-sequence

    let mut fields2 = HashMap::new();
    fields2.insert("user_id".to_string(), "456".to_string());
    fields2.insert("action".to_string(), "logout".to_string());

    let entry_id2 = client.xadd(stream_name, "*", fields2).await?;
    assert!(!entry_id2.is_empty());
    assert_ne!(entry_id1, entry_id2);

    // Check stream length
    let length = client.xlen(stream_name).await?;
    assert_eq!(length, 2);

    Ok(())
}

#[tokio::test]
async fn test_xrange_operations() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let stream_name = "range_stream";

    // Add multiple entries
    for i in 0..5 {
        let mut fields = HashMap::new();
        fields.insert("index".to_string(), i.to_string());
        fields.insert("data".to_string(), format!("value_{}", i));
        client.xadd(stream_name, "*", fields).await?;
    }

    // Get all entries
    let all_entries = client.xrange(stream_name, "-", "+", None).await?;
    assert_eq!(all_entries.len(), 5);

    // Verify entries are in order
    for (i, entry) in all_entries.iter().enumerate() {
        assert_eq!(entry.get_field("index"), Some(&i.to_string()));
        assert_eq!(entry.get_field("data"), Some(&format!("value_{}", i)));
    }

    // Get limited entries
    let limited_entries = client.xrange(stream_name, "-", "+", Some(3)).await?;
    assert_eq!(limited_entries.len(), 3);

    // Get entries from specific ID
    let first_id = &all_entries[0].id;
    let from_first = client.xrange(stream_name, first_id, "+", None).await?;
    assert_eq!(from_first.len(), 5); // Includes the first entry

    Ok(())
}

#[tokio::test]
async fn test_xread_operations() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let stream_name = "read_stream";

    // Add some initial entries
    let mut fields = HashMap::new();
    fields.insert("message".to_string(), "initial".to_string());
    let initial_id = client.xadd(stream_name, "*", fields).await?;

    // Read from beginning
    let streams = vec![(stream_name.to_string(), "0".to_string())];
    let messages = client.xread(streams.clone(), Some(10), None).await?;

    assert_eq!(messages.len(), 1);
    assert!(messages.contains_key(stream_name));
    let entries = &messages[stream_name];
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].get_field("message"),
        Some(&"initial".to_string())
    );

    // Add more entries
    for i in 1..=3 {
        let mut fields = HashMap::new();
        fields.insert("message".to_string(), format!("message_{}", i));
        client.xadd(stream_name, "*", fields).await?;
    }

    // Read new entries only
    let streams = vec![(stream_name.to_string(), initial_id)];
    let new_messages = client.xread(streams, Some(10), None).await?;

    assert_eq!(new_messages.len(), 1);
    let new_entries = &new_messages[stream_name];
    assert_eq!(new_entries.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_consumer_groups() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let stream_name = "consumer_stream";
    let group_name = "test_group";
    let consumer_name = "test_consumer";

    // Add some entries first
    for i in 0..3 {
        let mut fields = HashMap::new();
        fields.insert("order_id".to_string(), format!("order_{}", i));
        fields.insert("amount".to_string(), format!("{}.00", (i + 1) * 100));
        client.xadd(stream_name, "*", fields).await?;
    }

    // Create consumer group
    client
        .xgroup_create(stream_name, group_name, "0", false)
        .await?;

    // Read from consumer group (should get all existing messages)
    let streams = vec![(stream_name.to_string(), ">".to_string())];
    let messages = client
        .xreadgroup(group_name, consumer_name, streams, Some(10), None)
        .await?;

    assert_eq!(messages.len(), 1);
    let entries = &messages[stream_name];
    assert_eq!(entries.len(), 3);

    // Acknowledge messages
    let mut ids_to_ack = Vec::new();
    for entry in entries {
        ids_to_ack.push(entry.id.clone());
    }

    let acked = client.xack(stream_name, group_name, ids_to_ack).await?;
    assert_eq!(acked, 3);

    Ok(())
}

#[tokio::test]
async fn test_consumer_group_with_new_messages() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let stream_name = "new_messages_stream";
    let group_name = "processors";
    let consumer_name = "worker_1";

    // Create consumer group starting from latest messages
    client
        .xgroup_create(stream_name, group_name, "$", true)
        .await?;

    // Add new messages after group creation
    let mut message_ids = Vec::new();
    for i in 0..2 {
        let mut fields = HashMap::new();
        fields.insert("task_id".to_string(), format!("task_{}", i));
        fields.insert("priority".to_string(), "high".to_string());
        let id = client.xadd(stream_name, "*", fields).await?;
        message_ids.push(id);
    }

    // Read new messages from group
    let streams = vec![(stream_name.to_string(), ">".to_string())];
    let messages = client
        .xreadgroup(group_name, consumer_name, streams, Some(10), None)
        .await?;

    assert_eq!(messages.len(), 1);
    let entries = &messages[stream_name];
    assert_eq!(entries.len(), 2);

    // Verify message content
    for (i, entry) in entries.iter().enumerate() {
        assert_eq!(entry.get_field("task_id"), Some(&format!("task_{}", i)));
        assert_eq!(entry.get_field("priority"), Some(&"high".to_string()));
    }

    // Acknowledge one message
    let acked = client
        .xack(stream_name, group_name, vec![entries[0].id.clone()])
        .await?;
    assert_eq!(acked, 1);

    Ok(())
}

#[tokio::test]
async fn test_blocking_xread() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let stream_name = "blocking_stream";

    // Start a task that will add a message after a delay
    let client_clone = client.clone();
    let stream_name_clone = stream_name.to_string();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut fields = HashMap::new();
        fields.insert("delayed_message".to_string(), "hello".to_string());
        let _ = client_clone.xadd(&stream_name_clone, "*", fields).await;
    });

    // Blocking read with short timeout
    let streams = vec![(stream_name.to_string(), "$".to_string())];
    let messages = client
        .xread(streams, Some(1), Some(Duration::from_millis(200)))
        .await?;

    // Should receive the delayed message
    assert_eq!(messages.len(), 1);
    let entries = &messages[stream_name];
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].get_field("delayed_message"),
        Some(&"hello".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn test_multiple_streams() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let stream1 = "stream_1";
    let stream2 = "stream_2";

    // Add entries to both streams
    let mut fields1 = HashMap::new();
    fields1.insert("source".to_string(), "stream1".to_string());
    fields1.insert("data".to_string(), "data1".to_string());
    client.xadd(stream1, "*", fields1).await?;

    let mut fields2 = HashMap::new();
    fields2.insert("source".to_string(), "stream2".to_string());
    fields2.insert("data".to_string(), "data2".to_string());
    client.xadd(stream2, "*", fields2).await?;

    // Read from both streams
    let streams = vec![
        (stream1.to_string(), "0".to_string()),
        (stream2.to_string(), "0".to_string()),
    ];
    let messages = client.xread(streams, Some(10), None).await?;

    assert_eq!(messages.len(), 2);
    assert!(messages.contains_key(stream1));
    assert!(messages.contains_key(stream2));

    let entries1 = &messages[stream1];
    let entries2 = &messages[stream2];
    assert_eq!(entries1.len(), 1);
    assert_eq!(entries2.len(), 1);

    assert_eq!(
        entries1[0].get_field("source"),
        Some(&"stream1".to_string())
    );
    assert_eq!(
        entries2[0].get_field("source"),
        Some(&"stream2".to_string())
    );

    Ok(())
}

#[tokio::test]
async fn test_stream_entry_parsing() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let stream_name = "parsing_stream";

    // Add entry with various field types
    let mut fields = HashMap::new();
    fields.insert("string_field".to_string(), "hello world".to_string());
    fields.insert("number_field".to_string(), "42".to_string());
    fields.insert("json_field".to_string(), r#"{"key":"value"}"#.to_string());
    fields.insert("empty_field".to_string(), "".to_string());

    let entry_id = client.xadd(stream_name, "*", fields).await?;

    // Read back the entry
    let entries = client
        .xrange(stream_name, &entry_id, &entry_id, None)
        .await?;
    assert_eq!(entries.len(), 1);

    let entry = &entries[0];
    assert_eq!(entry.id, entry_id);
    assert_eq!(
        entry.get_field("string_field"),
        Some(&"hello world".to_string())
    );
    assert_eq!(entry.get_field("number_field"), Some(&"42".to_string()));
    assert_eq!(
        entry.get_field("json_field"),
        Some(&r#"{"key":"value"}"#.to_string())
    );
    assert_eq!(entry.get_field("empty_field"), Some(&"".to_string()));
    assert_eq!(entry.get_field("nonexistent"), None);

    // Test entry methods
    assert!(entry.has_field("string_field"));
    assert!(!entry.has_field("nonexistent"));

    // Test timestamp parsing (if ID format allows)
    if let Some(timestamp) = entry.timestamp() {
        assert!(timestamp > 0);
    }

    if let Some(_sequence) = entry.sequence() {
        // Sequence exists and is always non-negative (u64)
    }

    Ok(())
}

#[tokio::test]
async fn test_stream_error_conditions() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Test XLEN on non-existent stream
    let length = client.xlen("nonexistent_stream").await?;
    assert_eq!(length, 0);

    // Test XRANGE on non-existent stream
    let entries = client.xrange("nonexistent_stream", "-", "+", None).await?;
    assert!(entries.is_empty());

    // Test XREAD on non-existent stream with timeout
    let streams = vec![("nonexistent_stream".to_string(), "$".to_string())];
    let messages = client
        .xread(streams, Some(1), Some(Duration::from_millis(100)))
        .await?;
    assert!(messages.is_empty());

    // Test creating consumer group on non-existent stream without MKSTREAM
    let result = client
        .xgroup_create("nonexistent_stream", "test_group", "$", false)
        .await;
    assert!(result.is_err()); // Should fail without MKSTREAM

    // Test creating consumer group with MKSTREAM should succeed
    client
        .xgroup_create("auto_created_stream", "test_group", "$", true)
        .await?;
    let length = client.xlen("auto_created_stream").await?;
    assert_eq!(length, 0); // Stream exists but is empty

    Ok(())
}

#[tokio::test]
async fn test_stream_with_specific_ids() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let stream_name = "specific_id_stream";

    // Add entry with specific timestamp-based ID
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let specific_id = format!("{}-0", timestamp);

    let mut fields = HashMap::new();
    fields.insert("message".to_string(), "specific_id_message".to_string());

    let returned_id = client.xadd(stream_name, &specific_id, fields).await?;
    assert_eq!(returned_id, specific_id);

    // Add another entry with auto-generated ID
    let mut fields2 = HashMap::new();
    fields2.insert("message".to_string(), "auto_id_message".to_string());

    let auto_id = client.xadd(stream_name, "*", fields2).await?;
    assert_ne!(auto_id, specific_id);

    // Verify both entries exist
    let all_entries = client.xrange(stream_name, "-", "+", None).await?;
    assert_eq!(all_entries.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_stream_operations() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    let stream_name = "concurrent_stream";
    let num_tasks = 10;

    // Spawn multiple tasks that add entries concurrently
    let mut handles = Vec::new();
    for i in 0..num_tasks {
        let client_clone = client.clone();
        let stream_name_clone = stream_name.to_string();
        let handle = tokio::spawn(async move {
            let mut fields = HashMap::new();
            fields.insert("task_id".to_string(), i.to_string());
            fields.insert("data".to_string(), format!("data_from_task_{}", i));
            client_clone.xadd(&stream_name_clone, "*", fields).await
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut entry_ids = Vec::new();
    for handle in handles {
        let entry_id = handle.await??;
        entry_ids.push(entry_id);
    }

    // Verify all entries were added
    assert_eq!(entry_ids.len(), num_tasks);

    let final_length = client.xlen(stream_name).await?;
    assert_eq!(final_length, num_tasks as u64);

    // Verify all entries are unique
    let mut unique_ids = std::collections::HashSet::new();
    for id in &entry_ids {
        assert!(unique_ids.insert(id.clone()));
    }

    Ok(())
}
