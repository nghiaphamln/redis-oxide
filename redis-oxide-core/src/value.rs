//! RESP (REdis Serialization Protocol) value types

use crate::error::{RedisError, RedisResult};
use bytes::Bytes;

/// RESP protocol value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespValue {
    /// Simple string: +OK\r\n
    SimpleString(String),
    /// Error: -ERR message\r\n
    Error(String),
    /// Integer: :1000\r\n
    Integer(i64),
    /// Bulk string: $6\r\nfoobar\r\n
    BulkString(Bytes),
    /// Null bulk string: $-1\r\n
    Null,
    /// Array: *2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n
    Array(Vec<RespValue>),
}

impl RespValue {
    /// Convert to a string if possible
    pub fn as_string(&self) -> RedisResult<String> {
        match self {
            RespValue::SimpleString(s) => Ok(s.clone()),
            RespValue::BulkString(b) => String::from_utf8(b.to_vec())
                .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {}", e))),
            RespValue::Null => Err(RedisError::Type("Value is null".to_string())),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to string",
                self
            ))),
        }
    }

    /// Convert to an integer if possible
    pub fn as_int(&self) -> RedisResult<i64> {
        match self {
            RespValue::Integer(i) => Ok(*i),
            RespValue::BulkString(b) => {
                let s = String::from_utf8(b.to_vec())
                    .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {}", e)))?;
                s.parse::<i64>()
                    .map_err(|e| RedisError::Type(format!("Cannot parse integer: {}", e)))
            }
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to integer",
                self
            ))),
        }
    }

    /// Convert to bytes if possible
    pub fn as_bytes(&self) -> RedisResult<Bytes> {
        match self {
            RespValue::BulkString(b) => Ok(b.clone()),
            RespValue::SimpleString(s) => Ok(Bytes::from(s.as_bytes().to_vec())),
            RespValue::Null => Err(RedisError::Type("Value is null".to_string())),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to bytes",
                self
            ))),
        }
    }

    /// Convert to an array if possible
    pub fn as_array(&self) -> RedisResult<Vec<RespValue>> {
        match self {
            RespValue::Array(arr) => Ok(arr.clone()),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to array",
                self
            ))),
        }
    }

    /// Check if this is a null value
    pub fn is_null(&self) -> bool {
        matches!(self, RespValue::Null)
    }

    /// Check if this is an error
    pub fn is_error(&self) -> bool {
        matches!(self, RespValue::Error(_))
    }

    /// Extract error message if this is an error
    pub fn into_error(self) -> Option<String> {
        match self {
            RespValue::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

impl From<String> for RespValue {
    fn from(s: String) -> Self {
        RespValue::BulkString(Bytes::from(s.into_bytes()))
    }
}

impl From<&str> for RespValue {
    fn from(s: &str) -> Self {
        RespValue::BulkString(Bytes::from(s.as_bytes().to_vec()))
    }
}

impl From<i64> for RespValue {
    fn from(i: i64) -> Self {
        RespValue::Integer(i)
    }
}

impl From<Vec<u8>> for RespValue {
    fn from(b: Vec<u8>) -> Self {
        RespValue::BulkString(Bytes::from(b))
    }
}

impl From<Bytes> for RespValue {
    fn from(b: Bytes) -> Self {
        RespValue::BulkString(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_string() {
        let val = RespValue::SimpleString("OK".to_string());
        assert_eq!(val.as_string().unwrap(), "OK");

        let val = RespValue::BulkString(Bytes::from("test"));
        assert_eq!(val.as_string().unwrap(), "test");

        let val = RespValue::Null;
        assert!(val.as_string().is_err());
    }

    #[test]
    fn test_as_int() {
        let val = RespValue::Integer(42);
        assert_eq!(val.as_int().unwrap(), 42);

        let val = RespValue::BulkString(Bytes::from("123"));
        assert_eq!(val.as_int().unwrap(), 123);

        let val = RespValue::Null;
        assert!(val.as_int().is_err());
    }

    #[test]
    fn test_is_null() {
        assert!(RespValue::Null.is_null());
        assert!(!RespValue::Integer(1).is_null());
    }

    #[test]
    fn test_is_error() {
        assert!(RespValue::Error("ERR".to_string()).is_error());
        assert!(!RespValue::Null.is_error());
    }
}
