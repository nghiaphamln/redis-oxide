//! RESP (`REdis` Serialization Protocol) value types

use crate::core::error::{RedisError, RedisResult};
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
    Array(Vec<Self>),
}

impl RespValue {
    /// Convert to a string if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to a string.
    pub fn as_string(&self) -> RedisResult<String> {
        match self {
            Self::SimpleString(s) => Ok(s.clone()),
            Self::BulkString(b) => String::from_utf8(b.to_vec())
                .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}"))),
            Self::Null => Err(RedisError::Type("Value is null".to_string())),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to string"
            ))),
        }
    }

    /// Convert to an integer if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to an integer.
    pub fn as_int(&self) -> RedisResult<i64> {
        match self {
            Self::Integer(i) => Ok(*i),
            Self::BulkString(b) => {
                let s = String::from_utf8(b.to_vec())
                    .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}")))?;
                s.parse::<i64>()
                    .map_err(|e| RedisError::Type(format!("Cannot parse integer: {e}")))
            }
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to integer"
            ))),
        }
    }

    /// Convert to bytes if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to bytes.
    pub fn as_bytes(&self) -> RedisResult<Bytes> {
        match self {
            Self::BulkString(b) => Ok(b.clone()),
            Self::SimpleString(s) => Ok(Bytes::from(s.as_bytes().to_vec())),
            Self::Null => Err(RedisError::Type("Value is null".to_string())),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to bytes"
            ))),
        }
    }

    /// Convert to an array if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to an array.
    pub fn as_array(&self) -> RedisResult<Vec<Self>> {
        match self {
            Self::Array(arr) => Ok(arr.clone()),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to array"
            ))),
        }
    }

    /// Check if this is a null value
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Check if this is an error
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Convert to a boolean if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to a boolean.
    pub fn as_bool(&self) -> RedisResult<bool> {
        match self {
            Self::Integer(1) => Ok(true),
            Self::Integer(0) => Ok(false),
            Self::SimpleString(s) if s == "OK" => Ok(true),
            Self::BulkString(b) => {
                let s = String::from_utf8(b.to_vec())
                    .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}")))?;
                Ok(s == "1" || s.to_lowercase() == "true")
            }
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to boolean"
            ))),
        }
    }

    /// Extract error message if this is an error
    #[must_use]
    pub fn into_error(self) -> Option<String> {
        match self {
            Self::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

impl From<String> for RespValue {
    fn from(s: String) -> Self {
        Self::BulkString(Bytes::from(s.into_bytes()))
    }
}
impl From<&str> for RespValue {
    fn from(s: &str) -> Self {
        Self::BulkString(Bytes::from(s.as_bytes().to_vec()))
    }
}
impl From<i64> for RespValue {
    fn from(i: i64) -> Self {
        Self::Integer(i)
    }
}
impl From<Vec<u8>> for RespValue {
    fn from(b: Vec<u8>) -> Self {
        Self::BulkString(Bytes::from(b))
    }
}
impl From<Bytes> for RespValue {
    fn from(b: Bytes) -> Self {
        Self::BulkString(b)
    }
}

impl TryFrom<RespValue> for String {
    type Error = RedisError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        value.as_string()
    }
}

impl TryFrom<RespValue> for i64 {
    type Error = RedisError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        value.as_int()
    }
}

impl TryFrom<RespValue> for bool {
    type Error = RedisError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        match value {
            RespValue::Integer(1) => Ok(true),
            RespValue::Integer(0) => Ok(false),
            RespValue::SimpleString(s) if s == "OK" => Ok(true),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to bool",
                value
            ))),
        }
    }
}

impl TryFrom<RespValue> for Option<String> {
    type Error = RedisError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        match value {
            RespValue::BulkString(b) => String::from_utf8(b.to_vec())
                .map(Some)
                .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}"))),
            RespValue::SimpleString(s) => Ok(Some(s)),
            RespValue::Null => Ok(None),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to Option<String>",
                value
            ))),
        }
    }
}

impl TryFrom<RespValue> for Vec<String> {
    type Error = RedisError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        match value {
            RespValue::Array(items) => {
                let mut result = Self::new();
                for item in items {
                    match item {
                        RespValue::BulkString(b) => {
                            let s = String::from_utf8(b.to_vec())
                                .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}")))?;
                            result.push(s);
                        }
                        RespValue::SimpleString(s) => result.push(s),
                        RespValue::Null => {} // Skip null values
                        _ => {
                            return Err(RedisError::Type(format!(
                                "Cannot convert array item {:?} to string",
                                item
                            )))
                        }
                    }
                }
                Ok(result)
            }
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to Vec<String>",
                value
            ))),
        }
    }
}

impl TryFrom<RespValue> for Vec<i64> {
    type Error = RedisError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        match value {
            RespValue::Array(items) => {
                let mut result = Self::new();
                for item in items {
                    let i = item.as_int()?;
                    result.push(i);
                }
                Ok(result)
            }
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to Vec<i64>",
                value
            ))),
        }
    }
}

#[cfg(test)]
mod value_edge_case_tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_resp_value_partial_eq_different_types() {
        let v1 = RespValue::SimpleString("42".to_string());
        let v2 = RespValue::Integer(42);
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_resp_value_array_clone() {
        let original = RespValue::Array(vec![
            RespValue::BulkString(Bytes::from("a")),
            RespValue::BulkString(Bytes::from("b")),
        ]);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_resp_value_as_string_from_utf8_error() {
        let invalid_utf8 = vec![0x80, 0x81, 0x82];
        let value = RespValue::BulkString(Bytes::from(invalid_utf8));
        let result = value.as_string();
        assert!(result.is_err());
    }

    #[test]
    fn test_resp_value_as_int_from_invalid_string() {
        let value = RespValue::BulkString(Bytes::from("not_a_number"));
        let result = value.as_int();
        assert!(result.is_err());
    }

    #[test]
    fn test_resp_value_as_bool_from_invalid_type() {
        let value = RespValue::SimpleString("maybe".to_string());
        let result = value.as_bool();
        assert!(result.is_err());
    }

    #[test]
    fn test_resp_value_as_array_from_non_array() {
        let value = RespValue::Integer(42);
        let result = value.as_array();
        assert!(result.is_err());
    }

    #[test]
    fn test_resp_value_as_bytes_from_simple_string() {
        let value = RespValue::SimpleString("hello".to_string());
        let result = value.as_bytes();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Bytes::from("hello"));
    }

    #[test]
    fn test_from_i64_negative() {
        let value: RespValue = (-100i64).into();
        assert_eq!(value, RespValue::Integer(-100));
    }

    #[test]
    fn test_from_vec_u8_empty() {
        let value: RespValue = Vec::<u8>::new().into();
        assert_eq!(value, RespValue::BulkString(Bytes::new()));
    }

    #[test]
    fn test_try_into_vec_string_with_null() {
        let value = RespValue::Array(vec![
            RespValue::BulkString(Bytes::from("a")),
            RespValue::Null,
            RespValue::BulkString(Bytes::from("b")),
        ]);
        let result: Result<Vec<String>, _> = value.try_into();
        assert!(result.is_ok());
        let vec = result.unwrap();
        assert_eq!(vec.len(), 2);
        assert_eq!(vec[0], "a");
        assert_eq!(vec[1], "b");
    }

    #[test]
    fn test_try_into_vec_string_with_mixed_types() {
        let value = RespValue::Array(vec![
            RespValue::BulkString(Bytes::from("valid")),
            RespValue::Integer(42), // Invalid
        ]);
        let result: Result<Vec<String>, _> = value.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_try_into_vec_i64_mixed_array() {
        let value = RespValue::Array(vec![
            RespValue::Integer(1),
            RespValue::Integer(2),
            RespValue::Integer(3),
        ]);
        let result: Result<Vec<i64>, _> = value.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_resp_value_null_equality() {
        let v1 = RespValue::Null;
        let v2 = RespValue::Null;
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_resp_value_error_message() {
        let error = RespValue::Error("ERR test error".to_string());
        let message = error.into_error();
        assert_eq!(message, Some("ERR test error".to_string()));
    }
}
