//! Set commands for Redis
//!
//! This module provides command builders for Redis Set operations.

use crate::core::{error::RedisResult, value::RespValue};
use crate::pipeline::PipelineCommand;
use super::Command;
use std::collections::HashSet;

/// SADD command - Add one or more members to a set
#[derive(Debug, Clone)]
pub struct SAddCommand {
    key: String,
    members: Vec<String>,
}

impl SAddCommand {
    /// Create a new SADD command
    pub fn new(key: impl Into<String>, members: Vec<String>) -> Self {
        Self {
            key: key.into(),
            members,
        }
    }
}

impl Command for SAddCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "SADD"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.as_str())];
        for member in &self.members {
            args.push(RespValue::from(member.as_str()));
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

impl PipelineCommand for SAddCommand {
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

/// SREM command - Remove one or more members from a set
#[derive(Debug, Clone)]
pub struct SRemCommand {
    key: String,
    members: Vec<String>,
}

impl SRemCommand {
    /// Create a new SREM command
    pub fn new(key: impl Into<String>, members: Vec<String>) -> Self {
        Self {
            key: key.into(),
            members,
        }
    }
}

impl Command for SRemCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "SREM"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.as_str())];
        for member in &self.members {
            args.push(RespValue::from(member.as_str()));
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

impl PipelineCommand for SRemCommand {
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

/// SMEMBERS command - Get all members in a set
#[derive(Debug, Clone)]
pub struct SMembersCommand {
    key: String,
}

impl SMembersCommand {
    /// Create a new SMEMBERS command
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
        }
    }
}

impl Command for SMembersCommand {
    type Output = HashSet<String>;

    fn command_name(&self) -> &str {
        "SMEMBERS"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.as_str())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Array(items) => {
                let mut result = HashSet::new();
                for item in items {
                    match item {
                        RespValue::BulkString(bytes) => {
                            let s = String::from_utf8_lossy(&bytes).to_string();
                            result.insert(s);
                        }
                        _ => return Err(crate::core::error::RedisError::Type(format!(
                            "Unexpected item type in SMEMBERS response: {:?}",
                            item
                        ))),
                    }
                }
                Ok(result)
            }
            _ => Err(crate::core::error::RedisError::Type(format!(
                "Unexpected response type for SMEMBERS: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for SMembersCommand {
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

/// SISMEMBER command - Determine if a given value is a member of a set
#[derive(Debug, Clone)]
pub struct SIsMemberCommand {
    key: String,
    member: String,
}

impl SIsMemberCommand {
    /// Create a new SISMEMBER command
    pub fn new(key: impl Into<String>, member: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            member: member.into(),
        }
    }
}

impl Command for SIsMemberCommand {
    type Output = bool;

    fn command_name(&self) -> &str {
        "SISMEMBER"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.member.as_str()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Integer(1) => Ok(true),
            RespValue::Integer(0) => Ok(false),
            _ => Err(crate::core::error::RedisError::Type(format!(
                "Unexpected response type for SISMEMBER: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for SIsMemberCommand {
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

/// SCARD command - Get the number of members in a set
#[derive(Debug, Clone)]
pub struct SCardCommand {
    key: String,
}

impl SCardCommand {
    /// Create a new SCARD command
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
        }
    }
}

impl Command for SCardCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "SCARD"
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

impl PipelineCommand for SCardCommand {
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

/// SPOP command - Remove and return one or more random members from a set
#[derive(Debug, Clone)]
pub struct SPopCommand {
    key: String,
    count: Option<i64>,
}

impl SPopCommand {
    /// Create a new SPOP command
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            count: None,
        }
    }

    /// Create a new SPOP command with count
    pub fn with_count(key: impl Into<String>, count: i64) -> Self {
        Self {
            key: key.into(),
            count: Some(count),
        }
    }
}

impl Command for SPopCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "SPOP"
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
                "Unexpected response type for SPOP: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for SPopCommand {
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

/// SRANDMEMBER command - Get one or more random members from a set
#[derive(Debug, Clone)]
pub struct SRandMemberCommand {
    key: String,
    count: Option<i64>,
}

impl SRandMemberCommand {
    /// Create a new SRANDMEMBER command
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            count: None,
        }
    }

    /// Create a new SRANDMEMBER command with count
    pub fn with_count(key: impl Into<String>, count: i64) -> Self {
        Self {
            key: key.into(),
            count: Some(count),
        }
    }
}

impl Command for SRandMemberCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "SRANDMEMBER"
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
                "Unexpected response type for SRANDMEMBER: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for SRandMemberCommand {
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
    fn test_sadd_command() {
        let cmd = SAddCommand::new("myset", vec!["member1".to_string(), "member2".to_string()]);
        assert_eq!(cmd.command_name(), "SADD");
        assert_eq!(cmd.keys(), vec![b"myset"]);
        
        let args = <SAddCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + 2 members
    }

    #[test]
    fn test_srem_command() {
        let cmd = SRemCommand::new("myset", vec!["member1".to_string(), "member2".to_string()]);
        assert_eq!(cmd.command_name(), "SREM");
        assert_eq!(cmd.keys(), vec![b"myset"]);
        
        let args = <SRemCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + 2 members
    }

    #[test]
    fn test_smembers_command() {
        let cmd = SMembersCommand::new("myset");
        assert_eq!(cmd.command_name(), "SMEMBERS");
        assert_eq!(cmd.keys(), vec![b"myset"]);
        
        let args = <SMembersCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1); // key only
    }

    #[test]
    fn test_sismember_command() {
        let cmd = SIsMemberCommand::new("myset", "member1");
        assert_eq!(cmd.command_name(), "SISMEMBER");
        assert_eq!(cmd.keys(), vec![b"myset"]);
        
        let args = <SIsMemberCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2); // key + member
    }

    #[test]
    fn test_scard_command() {
        let cmd = SCardCommand::new("myset");
        assert_eq!(cmd.command_name(), "SCARD");
        assert_eq!(cmd.keys(), vec![b"myset"]);
        
        let args = <SCardCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1); // key only
    }

    #[test]
    fn test_spop_command() {
        let cmd = SPopCommand::new("myset");
        assert_eq!(cmd.command_name(), "SPOP");
        assert_eq!(cmd.keys(), vec![b"myset"]);
        
        let args = <SPopCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1); // key only
    }

    #[test]
    fn test_spop_command_with_count() {
        let cmd = SPopCommand::with_count("myset", 3);
        assert_eq!(cmd.command_name(), "SPOP");
        assert_eq!(cmd.keys(), vec![b"myset"]);
        
        let args = <SPopCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2); // key + count
    }

    #[test]
    fn test_srandmember_command() {
        let cmd = SRandMemberCommand::new("myset");
        assert_eq!(cmd.command_name(), "SRANDMEMBER");
        assert_eq!(cmd.keys(), vec![b"myset"]);
        
        let args = <SRandMemberCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1); // key only
    }

    #[test]
    fn test_srandmember_command_with_count() {
        let cmd = SRandMemberCommand::with_count("myset", 3);
        assert_eq!(cmd.command_name(), "SRANDMEMBER");
        assert_eq!(cmd.keys(), vec![b"myset"]);
        
        let args = <SRandMemberCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2); // key + count
    }
}
