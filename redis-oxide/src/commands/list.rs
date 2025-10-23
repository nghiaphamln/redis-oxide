//! List commands for Redis
//!
//! This module provides command builders for Redis List operations.

use crate::core::{error::RedisResult, value::RespValue};
use crate::pipeline::PipelineCommand;
use super::Command;

/// LPUSH command - Insert one or more values at the head of a list
#[derive(Debug, Clone)]
pub struct LPushCommand {
    key: String,
    values: Vec<String>,
}

impl LPushCommand {
    /// Create a new LPUSH command
    pub fn new(key: impl Into<String>, values: Vec<String>) -> Self {
        Self {
            key: key.into(),
            values,
        }
    }
}

impl Command for LPushCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "LPUSH"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.as_str())];
        for value in &self.values {
            args.push(RespValue::from(value.as_str()));
        }
        args
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for LPushCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

/// RPUSH command - Insert one or more values at the tail of a list
#[derive(Debug, Clone)]
pub struct RPushCommand {
    key: String,
    values: Vec<String>,
}

impl RPushCommand {
    /// Create a new RPUSH command
    pub fn new(key: impl Into<String>, values: Vec<String>) -> Self {
        Self {
            key: key.into(),
            values,
        }
    }
}

impl Command for RPushCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "RPUSH"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.as_str())];
        for value in &self.values {
            args.push(RespValue::from(value.as_str()));
        }
        args
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for RPushCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

/// LPOP command - Remove and return the first element of a list
#[derive(Debug, Clone)]
pub struct LPopCommand {
    key: String,
    count: Option<i64>,
}

impl LPopCommand {
    /// Create a new LPOP command
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            count: None,
        }
    }

    /// Create a new LPOP command with count
    pub fn with_count(key: impl Into<String>, count: i64) -> Self {
        Self {
            key: key.into(),
            count: Some(count),
        }
    }
}

impl Command for LPopCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "LPOP"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.as_str())];
        if let Some(count) = self.count {
            args.push(RespValue::from(count.to_string()));
        }
        args
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Null => Ok(None),
            RespValue::BulkString(bytes) => {
                let s = String::from_utf8_lossy(&bytes).to_string();
                Ok(Some(s))
            }
            _ => Err(crate::core::error::RedisError::Type(format!(
                "Unexpected response type for LPOP: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for LPopCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

/// RPOP command - Remove and return the last element of a list
#[derive(Debug, Clone)]
pub struct RPopCommand {
    key: String,
    count: Option<i64>,
}

impl RPopCommand {
    /// Create a new RPOP command
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            count: None,
        }
    }

    /// Create a new RPOP command with count
    pub fn with_count(key: impl Into<String>, count: i64) -> Self {
        Self {
            key: key.into(),
            count: Some(count),
        }
    }
}

impl Command for RPopCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "RPOP"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.as_str())];
        if let Some(count) = self.count {
            args.push(RespValue::from(count.to_string()));
        }
        args
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Null => Ok(None),
            RespValue::BulkString(bytes) => {
                let s = String::from_utf8_lossy(&bytes).to_string();
                Ok(Some(s))
            }
            _ => Err(crate::core::error::RedisError::Type(format!(
                "Unexpected response type for RPOP: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for RPopCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

/// LRANGE command - Get a range of elements from a list
#[derive(Debug, Clone)]
pub struct LRangeCommand {
    key: String,
    start: i64,
    stop: i64,
}

impl LRangeCommand {
    /// Create a new LRANGE command
    pub fn new(key: impl Into<String>, start: i64, stop: i64) -> Self {
        Self {
            key: key.into(),
            start,
            stop,
        }
    }
}

impl Command for LRangeCommand {
    type Output = Vec<String>;

    fn command_name(&self) -> &str {
        "LRANGE"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.start.to_string()),
            RespValue::from(self.stop.to_string()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Array(items) => {
                let mut result = Vec::new();
                for item in items {
                    match item {
                        RespValue::BulkString(bytes) => {
                            let s = String::from_utf8_lossy(&bytes).to_string();
                            result.push(s);
                        }
                        _ => return Err(crate::core::error::RedisError::Type(format!(
                            "Unexpected item type in LRANGE response: {:?}",
                            item
                        ))),
                    }
                }
                Ok(result)
            }
            _ => Err(crate::core::error::RedisError::Type(format!(
                "Unexpected response type for LRANGE: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for LRangeCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

/// LLEN command - Get the length of a list
#[derive(Debug, Clone)]
pub struct LLenCommand {
    key: String,
}

impl LLenCommand {
    /// Create a new LLEN command
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
        }
    }
}

impl Command for LLenCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "LLEN"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.as_str())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for LLenCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

/// LINDEX command - Get an element from a list by its index
#[derive(Debug, Clone)]
pub struct LIndexCommand {
    key: String,
    index: i64,
}

impl LIndexCommand {
    /// Create a new LINDEX command
    pub fn new(key: impl Into<String>, index: i64) -> Self {
        Self {
            key: key.into(),
            index,
        }
    }
}

impl Command for LIndexCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "LINDEX"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.index.to_string()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Null => Ok(None),
            RespValue::BulkString(bytes) => {
                let s = String::from_utf8_lossy(&bytes).to_string();
                Ok(Some(s))
            }
            _ => Err(crate::core::error::RedisError::Type(format!(
                "Unexpected response type for LINDEX: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for LIndexCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

/// LSET command - Set the value of an element in a list by its index
#[derive(Debug, Clone)]
pub struct LSetCommand {
    key: String,
    index: i64,
    value: String,
}

impl LSetCommand {
    /// Create a new LSET command
    pub fn new(key: impl Into<String>, index: i64, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            index,
            value: value.into(),
        }
    }
}

impl Command for LSetCommand {
    type Output = String;

    fn command_name(&self) -> &str {
        "LSET"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.index.to_string()),
            RespValue::from(self.value.as_str()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_string()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for LSetCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lpush_command() {
        let cmd = LPushCommand::new("mylist", vec!["value1".to_string(), "value2".to_string()]);
        assert_eq!(cmd.command_name(), "LPUSH");
        assert_eq!(cmd.keys(), vec![b"mylist"]);
        
        let args = <LPushCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + 2 values
    }

    #[test]
    fn test_rpush_command() {
        let cmd = RPushCommand::new("mylist", vec!["value1".to_string(), "value2".to_string()]);
        assert_eq!(cmd.command_name(), "RPUSH");
        assert_eq!(cmd.keys(), vec![b"mylist"]);
        
        let args = <RPushCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + 2 values
    }

    #[test]
    fn test_lpop_command() {
        let cmd = LPopCommand::new("mylist");
        assert_eq!(cmd.command_name(), "LPOP");
        assert_eq!(cmd.keys(), vec![b"mylist"]);
        
        let args = <LPopCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1); // key only
    }

    #[test]
    fn test_lpop_command_with_count() {
        let cmd = LPopCommand::with_count("mylist", 3);
        assert_eq!(cmd.command_name(), "LPOP");
        assert_eq!(cmd.keys(), vec![b"mylist"]);
        
        let args = <LPopCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2); // key + count
    }

    #[test]
    fn test_rpop_command() {
        let cmd = RPopCommand::new("mylist");
        assert_eq!(cmd.command_name(), "RPOP");
        assert_eq!(cmd.keys(), vec![b"mylist"]);
        
        let args = <RPopCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1); // key only
    }

    #[test]
    fn test_lrange_command() {
        let cmd = LRangeCommand::new("mylist", 0, -1);
        assert_eq!(cmd.command_name(), "LRANGE");
        assert_eq!(cmd.keys(), vec![b"mylist"]);
        
        let args = <LRangeCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + start + stop
    }

    #[test]
    fn test_llen_command() {
        let cmd = LLenCommand::new("mylist");
        assert_eq!(cmd.command_name(), "LLEN");
        assert_eq!(cmd.keys(), vec![b"mylist"]);
        
        let args = <LLenCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1); // key only
    }

    #[test]
    fn test_lindex_command() {
        let cmd = LIndexCommand::new("mylist", 0);
        assert_eq!(cmd.command_name(), "LINDEX");
        assert_eq!(cmd.keys(), vec![b"mylist"]);
        
        let args = <LIndexCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2); // key + index
    }

    #[test]
    fn test_lset_command() {
        let cmd = LSetCommand::new("mylist", 0, "newvalue");
        assert_eq!(cmd.command_name(), "LSET");
        assert_eq!(cmd.keys(), vec![b"mylist"]);
        
        let args = <LSetCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + index + value
    }
}
