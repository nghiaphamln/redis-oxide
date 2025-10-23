//! Hash commands for Redis
//!
//! This module provides command builders for Redis Hash operations.

use crate::core::{error::RedisResult, value::RespValue};
use crate::pipeline::PipelineCommand;
use super::Command;
use std::collections::HashMap;

/// HGET command - Get the value of a hash field
#[derive(Debug, Clone)]
pub struct HGetCommand {
    key: String,
    field: String,
}

impl HGetCommand {
    /// Create a new HGET command
    pub fn new(key: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            field: field.into(),
        }
    }
}

impl Command for HGetCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "HGET"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.field.as_str()),
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
                "Unexpected response type for HGET: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for HGetCommand {
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

/// HSET command - Set the value of a hash field
#[derive(Debug, Clone)]
pub struct HSetCommand {
    key: String,
    field: String,
    value: String,
}

impl HSetCommand {
    /// Create a new HSET command
    pub fn new(key: impl Into<String>, field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            field: field.into(),
            value: value.into(),
        }
    }
}

impl Command for HSetCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "HSET"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.field.as_str()),
            RespValue::from(self.value.as_str()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for HSetCommand {
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

/// HDEL command - Delete one or more hash fields
#[derive(Debug, Clone)]
pub struct HDelCommand {
    key: String,
    fields: Vec<String>,
}

impl HDelCommand {
    /// Create a new HDEL command
    pub fn new(key: impl Into<String>, fields: Vec<String>) -> Self {
        Self {
            key: key.into(),
            fields,
        }
    }
}

impl Command for HDelCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "HDEL"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.as_str())];
        for field in &self.fields {
            args.push(RespValue::from(field.as_str()));
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

impl PipelineCommand for HDelCommand {
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

/// HGETALL command - Get all fields and values in a hash
#[derive(Debug, Clone)]
pub struct HGetAllCommand {
    key: String,
}

impl HGetAllCommand {
    /// Create a new HGETALL command
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
        }
    }
}

impl Command for HGetAllCommand {
    type Output = HashMap<String, String>;

    fn command_name(&self) -> &str {
        "HGETALL"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.as_str())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Array(items) => {
                let mut result = HashMap::new();
                let mut iter = items.into_iter();
                
                while let (Some(field), Some(value)) = (iter.next(), iter.next()) {
                    let field_str = field.as_string()?;
                    let value_str = value.as_string()?;
                    result.insert(field_str, value_str);
                }
                
                Ok(result)
            }
            _ => Err(crate::core::error::RedisError::Type(format!(
                "Unexpected response type for HGETALL: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for HGetAllCommand {
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

/// HMGET command - Get the values of multiple hash fields
#[derive(Debug, Clone)]
pub struct HMGetCommand {
    key: String,
    fields: Vec<String>,
}

impl HMGetCommand {
    /// Create a new HMGET command
    pub fn new(key: impl Into<String>, fields: Vec<String>) -> Self {
        Self {
            key: key.into(),
            fields,
        }
    }
}

impl Command for HMGetCommand {
    type Output = Vec<Option<String>>;

    fn command_name(&self) -> &str {
        "HMGET"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.as_str())];
        for field in &self.fields {
            args.push(RespValue::from(field.as_str()));
        }
        args
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Array(items) => {
                let mut result = Vec::new();
                for item in items {
                    match item {
                        RespValue::Null => result.push(None),
                        RespValue::BulkString(bytes) => {
                            let s = String::from_utf8_lossy(&bytes).to_string();
                            result.push(Some(s));
                        }
                        _ => return Err(crate::core::error::RedisError::Type(format!(
                            "Unexpected item type in HMGET response: {:?}",
                            item
                        ))),
                    }
                }
                Ok(result)
            }
            _ => Err(crate::core::error::RedisError::Type(format!(
                "Unexpected response type for HMGET: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for HMGetCommand {
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

/// HMSET command - Set multiple hash fields to multiple values
#[derive(Debug, Clone)]
pub struct HMSetCommand {
    key: String,
    fields: HashMap<String, String>,
}

impl HMSetCommand {
    /// Create a new HMSET command
    pub fn new(key: impl Into<String>, fields: HashMap<String, String>) -> Self {
        Self {
            key: key.into(),
            fields,
        }
    }
}

impl Command for HMSetCommand {
    type Output = String;

    fn command_name(&self) -> &str {
        "HMSET"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.as_str())];
        for (field, value) in &self.fields {
            args.push(RespValue::from(field.as_str()));
            args.push(RespValue::from(value.as_str()));
        }
        args
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_string()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for HMSetCommand {
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

/// HLEN command - Get the number of fields in a hash
#[derive(Debug, Clone)]
pub struct HLenCommand {
    key: String,
}

impl HLenCommand {
    /// Create a new HLEN command
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
        }
    }
}

impl Command for HLenCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "HLEN"
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

impl PipelineCommand for HLenCommand {
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

/// HEXISTS command - Determine if a hash field exists
#[derive(Debug, Clone)]
pub struct HExistsCommand {
    key: String,
    field: String,
}

impl HExistsCommand {
    /// Create a new HEXISTS command
    pub fn new(key: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            field: field.into(),
        }
    }
}

impl Command for HExistsCommand {
    type Output = bool;

    fn command_name(&self) -> &str {
        "HEXISTS"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.field.as_str()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Integer(1) => Ok(true),
            RespValue::Integer(0) => Ok(false),
            _ => Err(crate::core::error::RedisError::Type(format!(
                "Unexpected response type for HEXISTS: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for HExistsCommand {
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
    fn test_hget_command() {
        let cmd = HGetCommand::new("myhash", "field1");
        assert_eq!(cmd.command_name(), "HGET");
        assert_eq!(cmd.keys(), vec![b"myhash"]);
        
        let args = <HGetCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_hset_command() {
        let cmd = HSetCommand::new("myhash", "field1", "value1");
        assert_eq!(cmd.command_name(), "HSET");
        assert_eq!(cmd.keys(), vec![b"myhash"]);
        
        let args = <HSetCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_hdel_command() {
        let cmd = HDelCommand::new("myhash", vec!["field1".to_string(), "field2".to_string()]);
        assert_eq!(cmd.command_name(), "HDEL");
        assert_eq!(cmd.keys(), vec![b"myhash"]);
        
        let args = <HDelCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + 2 fields
    }

    #[test]
    fn test_hgetall_command() {
        let cmd = HGetAllCommand::new("myhash");
        assert_eq!(cmd.command_name(), "HGETALL");
        assert_eq!(cmd.keys(), vec![b"myhash"]);
        
        let args = <HGetAllCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_hmget_command() {
        let cmd = HMGetCommand::new("myhash", vec!["field1".to_string(), "field2".to_string()]);
        assert_eq!(cmd.command_name(), "HMGET");
        assert_eq!(cmd.keys(), vec![b"myhash"]);
        
        let args = <HMGetCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + 2 fields
    }

    #[test]
    fn test_hmset_command() {
        let mut fields = HashMap::new();
        fields.insert("field1".to_string(), "value1".to_string());
        fields.insert("field2".to_string(), "value2".to_string());
        
        let cmd = HMSetCommand::new("myhash", fields);
        assert_eq!(cmd.command_name(), "HMSET");
        assert_eq!(cmd.keys(), vec![b"myhash"]);
        
        let args = <HMSetCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 5); // key + 2 * (field + value)
    }

    #[test]
    fn test_hlen_command() {
        let cmd = HLenCommand::new("myhash");
        assert_eq!(cmd.command_name(), "HLEN");
        assert_eq!(cmd.keys(), vec![b"myhash"]);
        
        let args = <HLenCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_hexists_command() {
        let cmd = HExistsCommand::new("myhash", "field1");
        assert_eq!(cmd.command_name(), "HEXISTS");
        assert_eq!(cmd.keys(), vec![b"myhash"]);
        
        let args = <HExistsCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2);
    }
}
