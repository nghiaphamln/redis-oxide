//! Redis Streams support for event sourcing and messaging
//!
//! Redis Streams provide a powerful data structure for handling time-series data,
//! event sourcing, and message queuing. This module provides a high-level API
//! for working with Redis Streams.
//!
//! # Examples
//!
//! ## Basic Stream Operations
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//! use std::collections::HashMap;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConnectionConfig::new("redis://localhost:6379");
//! let client = Client::connect(config).await?;
//!
//! // Add entries to a stream
//! let mut fields = HashMap::new();
//! fields.insert("user_id".to_string(), "123".to_string());
//! fields.insert("action".to_string(), "login".to_string());
//!
//! let entry_id = client.xadd("events", "*", fields).await?;
//! println!("Added entry: {}", entry_id);
//!
//! // Read from stream
//! let entries = client.xrange("events", "-", "+", Some(10)).await?;
//! for entry in entries {
//!     println!("Entry {}: {:?}", entry.id, entry.fields);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Consumer Groups
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConnectionConfig::new("redis://localhost:6379");
//! let client = Client::connect(config).await?;
//!
//! // Create a consumer group
//! client.xgroup_create("events", "processors", "$", true).await?;
//!
//! // Read from the group
//! let messages = client.xreadgroup(
//!     "processors",
//!     "worker-1",
//!     vec![("events".to_string(), ">".to_string())],
//!     Some(1),
//!     Some(std::time::Duration::from_secs(1))
//! ).await?;
//!
//! for (stream, entries) in messages {
//!     for entry in entries {
//!         println!("Processing {}: {:?}", entry.id, entry.fields);
//!         // Process the message...
//!         
//!         // Acknowledge the message
//!         client.xack("events", "processors", vec![entry.id]).await?;
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use std::collections::HashMap;
use std::time::Duration;

/// Represents a single entry in a Redis Stream
#[derive(Debug, Clone)]
pub struct StreamEntry {
    /// The unique ID of the entry (e.g., "1234567890123-0")
    pub id: String,
    /// The field-value pairs of the entry
    pub fields: HashMap<String, String>,
}

impl StreamEntry {
    /// Create a new stream entry
    pub fn new(id: String, fields: HashMap<String, String>) -> Self {
        Self { id, fields }
    }

    /// Get a field value by name
    #[must_use]
    pub fn get_field(&self, field: &str) -> Option<&String> {
        self.fields.get(field)
    }

    /// Check if the entry has a specific field
    #[must_use]
    pub fn has_field(&self, field: &str) -> bool {
        self.fields.contains_key(field)
    }

    /// Get the timestamp part of the entry ID
    ///
    /// # Examples
    ///
    /// ```
    /// use redis_oxide::StreamEntry;
    /// use std::collections::HashMap;
    ///
    /// let entry = StreamEntry::new("1234567890123-0".to_string(), HashMap::new());
    /// assert_eq!(entry.timestamp(), Some(1234567890123));
    /// ```
    #[must_use]
    pub fn timestamp(&self) -> Option<u64> {
        self.id.split('-').next()?.parse().ok()
    }

    /// Get the sequence number part of the entry ID
    ///
    /// # Examples
    ///
    /// ```
    /// use redis_oxide::StreamEntry;
    /// use std::collections::HashMap;
    ///
    /// let entry = StreamEntry::new("1234567890123-5".to_string(), HashMap::new());
    /// assert_eq!(entry.sequence(), Some(5));
    /// ```
    #[must_use]
    pub fn sequence(&self) -> Option<u64> {
        self.id.split('-').nth(1)?.parse().ok()
    }
}

/// Information about a Redis Stream
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// The length of the stream (number of entries)
    pub length: u64,
    /// The number of consumer groups
    pub groups: u64,
    /// The ID of the first entry
    pub first_entry: Option<String>,
    /// The ID of the last entry
    pub last_entry: Option<String>,
    /// The last generated ID
    pub last_generated_id: String,
}

/// Information about a consumer group
#[derive(Debug, Clone)]
pub struct ConsumerGroupInfo {
    /// The name of the consumer group
    pub name: String,
    /// The number of consumers in the group
    pub consumers: u64,
    /// The number of pending messages
    pub pending: u64,
    /// The ID of the last delivered message
    pub last_delivered_id: String,
}

/// Information about a consumer in a group
#[derive(Debug, Clone)]
pub struct ConsumerInfo {
    /// The name of the consumer
    pub name: String,
    /// The number of pending messages for this consumer
    pub pending: u64,
    /// The idle time in milliseconds
    pub idle: u64,
}

/// Pending message information
#[derive(Debug, Clone)]
pub struct PendingMessage {
    /// The message ID
    pub id: String,
    /// The consumer name
    pub consumer: String,
    /// The idle time in milliseconds
    pub idle_time: u64,
    /// The delivery count
    pub delivery_count: u64,
}

/// Stream range options for XRANGE and XREVRANGE
#[derive(Debug, Clone)]
pub struct StreamRange {
    /// Start ID (inclusive)
    pub start: String,
    /// End ID (inclusive)
    pub end: String,
    /// Maximum number of entries to return
    pub count: Option<u64>,
}

impl StreamRange {
    /// Create a new stream range
    pub fn new(start: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
            count: None,
        }
    }

    /// Set the maximum number of entries to return
    pub fn with_count(mut self, count: u64) -> Self {
        self.count = Some(count);
        self
    }

    /// Create a range for all entries
    pub fn all() -> Self {
        Self::new("-", "+")
    }

    /// Create a range from a specific ID to the end
    pub fn from(start: impl Into<String>) -> Self {
        Self::new(start, "+")
    }

    /// Create a range from the beginning to a specific ID
    pub fn to(end: impl Into<String>) -> Self {
        Self::new("-", end)
    }
}

/// Options for XREAD and XREADGROUP commands
#[derive(Debug, Clone)]
pub struct ReadOptions {
    /// Maximum number of entries per stream
    pub count: Option<u64>,
    /// Block timeout in milliseconds (None for non-blocking)
    pub block: Option<Duration>,
}

impl ReadOptions {
    /// Create new read options
    #[must_use]
    pub fn new() -> Self {
        Self {
            count: None,
            block: None,
        }
    }

    /// Set the maximum number of entries per stream
    pub fn with_count(mut self, count: u64) -> Self {
        self.count = Some(count);
        self
    }

    /// Set the block timeout
    pub fn with_block(mut self, timeout: Duration) -> Self {
        self.block = Some(timeout);
        self
    }

    /// Create blocking read options
    pub fn blocking(timeout: Duration) -> Self {
        Self::new().with_block(timeout)
    }

    /// Create non-blocking read options with count limit
    pub fn non_blocking(count: u64) -> Self {
        Self::new().with_count(count)
    }
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse stream entries from Redis response
pub fn parse_stream_entries(response: RespValue) -> RedisResult<Vec<StreamEntry>> {
    match response {
        RespValue::Array(items) => {
            let mut entries = Vec::new();

            for item in items {
                match item {
                    RespValue::Array(entry_data) if entry_data.len() == 2 => {
                        let id = entry_data[0].as_string()?;

                        match &entry_data[1] {
                            RespValue::Array(field_values) => {
                                let mut fields = HashMap::new();

                                // Fields are stored as [field1, value1, field2, value2, ...]
                                for chunk in field_values.chunks(2) {
                                    if chunk.len() == 2 {
                                        let field = chunk[0].as_string()?;
                                        let value = chunk[1].as_string()?;
                                        fields.insert(field, value);
                                    }
                                }

                                entries.push(StreamEntry::new(id, fields));
                            }
                            _ => {
                                return Err(RedisError::Type(format!(
                                    "Invalid stream entry field format: {:?}",
                                    entry_data[1]
                                )))
                            }
                        }
                    }
                    _ => {
                        return Err(RedisError::Type(format!(
                            "Invalid stream entry format: {:?}",
                            item
                        )))
                    }
                }
            }

            Ok(entries)
        }
        _ => Err(RedisError::Type(format!(
            "Expected array for stream entries, got: {:?}",
            response
        ))),
    }
}

/// Parse XREAD/XREADGROUP response
pub fn parse_xread_response(response: RespValue) -> RedisResult<HashMap<String, Vec<StreamEntry>>> {
    match response {
        RespValue::Array(streams) => {
            let mut result = HashMap::new();

            for stream in streams {
                match stream {
                    RespValue::Array(stream_data) if stream_data.len() == 2 => {
                        let stream_name = stream_data[0].as_string()?;
                        let entries = parse_stream_entries(stream_data[1].clone())?;
                        result.insert(stream_name, entries);
                    }
                    _ => {
                        return Err(RedisError::Type(format!(
                            "Invalid XREAD response format: {:?}",
                            stream
                        )))
                    }
                }
            }

            Ok(result)
        }
        RespValue::Null => Ok(HashMap::new()), // No new entries
        _ => Err(RedisError::Type(format!(
            "Expected array or null for XREAD response, got: {:?}",
            response
        ))),
    }
}

/// Parse stream info from XINFO STREAM response
pub fn parse_stream_info(response: RespValue) -> RedisResult<StreamInfo> {
    match response {
        RespValue::Array(items) => {
            let mut length = 0;
            let mut groups = 0;
            let mut first_entry = None;
            let mut last_entry = None;
            let mut last_generated_id = String::new();

            // Parse key-value pairs
            for chunk in items.chunks(2) {
                if chunk.len() == 2 {
                    let key = chunk[0].as_string()?;
                    match key.as_str() {
                        "length" => length = chunk[1].as_int()? as u64,
                        "groups" => groups = chunk[1].as_int()? as u64,
                        "first-entry" => {
                            if !chunk[1].is_null() {
                                if let RespValue::Array(entry) = &chunk[1] {
                                    if !entry.is_empty() {
                                        first_entry = Some(entry[0].as_string()?);
                                    }
                                }
                            }
                        }
                        "last-entry" => {
                            if !chunk[1].is_null() {
                                if let RespValue::Array(entry) = &chunk[1] {
                                    if !entry.is_empty() {
                                        last_entry = Some(entry[0].as_string()?);
                                    }
                                }
                            }
                        }
                        "last-generated-id" => last_generated_id = chunk[1].as_string()?,
                        _ => {} // Ignore unknown fields
                    }
                }
            }

            Ok(StreamInfo {
                length,
                groups,
                first_entry,
                last_entry,
                last_generated_id,
            })
        }
        _ => Err(RedisError::Type(format!(
            "Expected array for stream info, got: {:?}",
            response
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_entry_creation() {
        let mut fields = HashMap::new();
        fields.insert("user".to_string(), "alice".to_string());
        fields.insert("action".to_string(), "login".to_string());

        let entry = StreamEntry::new("1234567890123-0".to_string(), fields.clone());

        assert_eq!(entry.id, "1234567890123-0");
        assert_eq!(entry.fields, fields);
        assert_eq!(entry.get_field("user"), Some(&"alice".to_string()));
        assert!(entry.has_field("action"));
        assert!(!entry.has_field("nonexistent"));
    }

    #[test]
    fn test_stream_entry_timestamp_parsing() {
        let entry = StreamEntry::new("1234567890123-5".to_string(), HashMap::new());

        assert_eq!(entry.timestamp(), Some(1234567890123));
        assert_eq!(entry.sequence(), Some(5));
    }

    #[test]
    fn test_stream_entry_invalid_id() {
        let entry = StreamEntry::new("invalid-id".to_string(), HashMap::new());

        assert_eq!(entry.timestamp(), None);
        assert_eq!(entry.sequence(), None);
    }

    #[test]
    fn test_stream_range_creation() {
        let range = StreamRange::new("1000", "2000").with_count(10);

        assert_eq!(range.start, "1000");
        assert_eq!(range.end, "2000");
        assert_eq!(range.count, Some(10));
    }

    #[test]
    fn test_stream_range_presets() {
        let all = StreamRange::all();
        assert_eq!(all.start, "-");
        assert_eq!(all.end, "+");

        let from = StreamRange::from("1000");
        assert_eq!(from.start, "1000");
        assert_eq!(from.end, "+");

        let to = StreamRange::to("2000");
        assert_eq!(to.start, "-");
        assert_eq!(to.end, "2000");
    }

    #[test]
    fn test_read_options() {
        let options = ReadOptions::new()
            .with_count(5)
            .with_block(Duration::from_secs(1));

        assert_eq!(options.count, Some(5));
        assert_eq!(options.block, Some(Duration::from_secs(1)));

        let blocking = ReadOptions::blocking(Duration::from_millis(500));
        assert_eq!(blocking.block, Some(Duration::from_millis(500)));

        let non_blocking = ReadOptions::non_blocking(10);
        assert_eq!(non_blocking.count, Some(10));
        assert_eq!(non_blocking.block, None);
    }

    #[test]
    fn test_parse_stream_entries() {
        let response = RespValue::Array(vec![
            RespValue::Array(vec![
                RespValue::from("1234567890123-0"),
                RespValue::Array(vec![
                    RespValue::from("user"),
                    RespValue::from("alice"),
                    RespValue::from("action"),
                    RespValue::from("login"),
                ]),
            ]),
            RespValue::Array(vec![
                RespValue::from("1234567890124-0"),
                RespValue::Array(vec![
                    RespValue::from("user"),
                    RespValue::from("bob"),
                    RespValue::from("action"),
                    RespValue::from("logout"),
                ]),
            ]),
        ]);

        let entries = parse_stream_entries(response).unwrap();
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].id, "1234567890123-0");
        assert_eq!(entries[0].get_field("user"), Some(&"alice".to_string()));
        assert_eq!(entries[0].get_field("action"), Some(&"login".to_string()));

        assert_eq!(entries[1].id, "1234567890124-0");
        assert_eq!(entries[1].get_field("user"), Some(&"bob".to_string()));
        assert_eq!(entries[1].get_field("action"), Some(&"logout".to_string()));
    }

    #[test]
    fn test_parse_xread_response() {
        let response = RespValue::Array(vec![RespValue::Array(vec![
            RespValue::from("stream1"),
            RespValue::Array(vec![RespValue::Array(vec![
                RespValue::from("1234567890123-0"),
                RespValue::Array(vec![RespValue::from("field1"), RespValue::from("value1")]),
            ])]),
        ])]);

        let result = parse_xread_response(response).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("stream1"));

        let entries = &result["stream1"];
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "1234567890123-0");
        assert_eq!(entries[0].get_field("field1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_parse_xread_response_null() {
        let response = RespValue::Null;
        let result = parse_xread_response(response).unwrap();
        assert!(result.is_empty());
    }
}
