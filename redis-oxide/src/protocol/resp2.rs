//! RESP2 protocol implementation
//!
//! This module implements the Redis Serialization Protocol (RESP2) for
//! encoding and decoding Redis commands and responses.
//!
//! The encoder uses buffer pre-sizing and zero-copy optimizations for
//! reduced memory allocations.

#![allow(missing_docs)]

use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io::Cursor;

const CRLF: &[u8] = b"\r\n";

pub struct RespEncoder {
    buffer: BytesMut,
}

impl RespEncoder {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(1024),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(capacity),
        }
    }

    fn estimate_size(value: &RespValue) -> usize {
        match value {
            RespValue::SimpleString(s) => 1 + s.len() + 2,
            RespValue::Error(e) => 1 + e.len() + 2,
            RespValue::Integer(i) => 1 + i.to_string().len() + 2,
            RespValue::BulkString(b) => {
                let len_str = b.len().to_string();
                1 + len_str.len() + 2 + b.len() + 2
            }
            RespValue::Null => 5,
            RespValue::Array(arr) => {
                let len_str = arr.len().to_string();
                let mut size = 1 + len_str.len() + 2;
                for item in arr {
                    size += Self::estimate_size(item);
                }
                size
            }
        }
    }

    fn estimate_command_size(command: &str, args: &[RespValue]) -> usize {
        let total_items = 1 + args.len();
        let array_header = 1 + total_items.to_string().len() + 2;
        let cmd_size = 1 + command.len().to_string().len() + 2 + command.len() + 2;
        let args_size: usize = args.iter().map(Self::estimate_size).sum();
        array_header + cmd_size + args_size
    }

    pub fn encode(&mut self, value: &RespValue) -> RedisResult<Bytes> {
        let estimated_size = Self::estimate_size(value);
        if self.buffer.capacity() < estimated_size {
            self.buffer.reserve(estimated_size);
        }
        self.buffer.clear();
        self.encode_value(value)?;
        Ok(self.buffer.split().freeze())
    }

    pub fn encode_command(&mut self, command: &str, args: &[RespValue]) -> RedisResult<Bytes> {
        let estimated_size = Self::estimate_command_size(command, args);
        if self.buffer.capacity() < estimated_size {
            self.buffer.reserve(estimated_size);
        }
        self.buffer.clear();
        let total_len = 1 + args.len();
        self.buffer.put_u8(b'*');
        self.put_integer_bytes(total_len);
        self.buffer.put_slice(CRLF);
        self.buffer.put_u8(b'$');
        self.put_integer_bytes(command.len());
        self.buffer.put_slice(CRLF);
        self.buffer.put_slice(command.as_bytes());
        self.buffer.put_slice(CRLF);
        for arg in args {
            self.encode_value(arg)?;
        }
        Ok(self.buffer.split().freeze())
    }

    fn encode_value(&mut self, value: &RespValue) -> RedisResult<()> {
        match value {
            RespValue::SimpleString(s) => {
                self.buffer.put_u8(b'+');
                self.buffer.put_slice(s.as_bytes());
                self.buffer.put_slice(CRLF);
            }
            RespValue::Error(e) => {
                self.buffer.put_u8(b'-');
                self.buffer.put_slice(e.as_bytes());
                self.buffer.put_slice(CRLF);
            }
            RespValue::Integer(i) => {
                self.buffer.put_u8(b':');
                self.put_integer_bytes(*i);
                self.buffer.put_slice(CRLF);
            }
            RespValue::BulkString(data) => {
                self.buffer.put_u8(b'$');
                self.put_integer_bytes(data.len());
                self.buffer.put_slice(CRLF);
                self.buffer.put_slice(data);
                self.buffer.put_slice(CRLF);
            }
            RespValue::Null => {
                self.buffer.put_slice(b"$-1\r\n");
            }
            RespValue::Array(arr) => {
                self.buffer.put_u8(b'*');
                self.put_integer_bytes(arr.len());
                self.buffer.put_slice(CRLF);
                for item in arr {
                    self.encode_value(item)?;
                }
            }
        }
        Ok(())
    }

    fn put_integer_bytes<T: itoa::Integer>(&mut self, value: T) {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        self.buffer.put_slice(s.as_bytes());
    }

    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buffer.reserve(additional);
    }
}

impl Default for RespEncoder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RespDecoder;

impl RespDecoder {
    pub fn decode(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        if !buf.has_remaining() {
            return Ok(None);
        }

        let type_byte = buf.chunk()[0];

        match type_byte {
            b'+' => Self::decode_simple_string(buf),
            b'-' => Self::decode_error(buf),
            b':' => Self::decode_integer(buf),
            b'$' => Self::decode_bulk_string(buf),
            b'*' => Self::decode_array(buf),
            _ => Err(RedisError::Protocol(format!(
                "Invalid RESP type byte: {}",
                type_byte as char
            ))),
        }
    }

    fn decode_simple_string(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1);
        if let Some(line) = Self::read_line(buf)? {
            Ok(Some(RespValue::SimpleString(
                String::from_utf8(line.to_vec())
                    .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?,
            )))
        } else {
            Ok(None)
        }
    }

    fn decode_error(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1);
        if let Some(line) = Self::read_line(buf)? {
            Ok(Some(RespValue::Error(
                String::from_utf8(line.to_vec())
                    .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?,
            )))
        } else {
            Ok(None)
        }
    }

    fn decode_integer(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1);
        if let Some(line) = Self::read_line(buf)? {
            let num_str = String::from_utf8(line.to_vec())
                .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?;
            let num = num_str
                .parse::<i64>()
                .map_err(|e| RedisError::Protocol(format!("Invalid integer: {}", e)))?;
            Ok(Some(RespValue::Integer(num)))
        } else {
            Ok(None)
        }
    }

    fn decode_bulk_string(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1);
        let len_line = match Self::read_line(buf)? {
            Some(line) => line,
            None => return Ok(None),
        };
        let len_str = String::from_utf8(len_line.to_vec())
            .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?;
        let len = len_str
            .parse::<i64>()
            .map_err(|e| RedisError::Protocol(format!("Invalid bulk string length: {}", e)))?;
        if len == -1 {
            return Ok(Some(RespValue::Null));
        }
        let len = len as usize;
        if buf.remaining() < len + 2 {
            return Ok(None);
        }
        let data = buf.chunk()[..len].to_vec();
        buf.advance(len);
        if buf.remaining() < 2 {
            return Ok(None);
        }
        buf.advance(2);
        Ok(Some(RespValue::BulkString(Bytes::from(data))))
    }

    fn decode_array(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1);
        let len_line = match Self::read_line(buf)? {
            Some(line) => line,
            None => return Ok(None),
        };
        let len_str = String::from_utf8(len_line.to_vec())
            .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?;
        let len = len_str
            .parse::<i64>()
            .map_err(|e| RedisError::Protocol(format!("Invalid array length: {}", e)))?;
        if len == -1 {
            return Ok(Some(RespValue::Null));
        }
        let len = len as usize;
        let mut arr = Vec::with_capacity(len);
        for _ in 0..len {
            match Self::decode(buf)? {
                Some(value) => arr.push(value),
                None => return Ok(None),
            }
        }
        Ok(Some(RespValue::Array(arr)))
    }

    fn read_line(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<Vec<u8>>> {
        let start = buf.position() as usize;
        let slice = buf.get_ref();
        for i in start..slice.len().saturating_sub(1) {
            if slice[i] == b'\r' && slice[i + 1] == b'\n' {
                let line = slice[start..i].to_vec();
                buf.set_position((i + 2) as u64);
                return Ok(Some(line));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_simple_string() {
        let mut encoder = RespEncoder::new();
        let value = RespValue::SimpleString("OK".to_string());
        let encoded = encoder.encode(&value).unwrap();
        assert_eq!(&encoded[..], b"+OK\r\n");
    }

    #[test]
    fn test_encode_error() {
        let mut encoder = RespEncoder::new();
        let value = RespValue::Error("ERR unknown command".to_string());
        let encoded = encoder.encode(&value).unwrap();
        assert_eq!(&encoded[..], b"-ERR unknown command\r\n");
    }

    #[test]
    fn test_encode_integer() {
        let mut encoder = RespEncoder::new();
        let value = RespValue::Integer(1000);
        let encoded = encoder.encode(&value).unwrap();
        assert_eq!(&encoded[..], b":1000\r\n");
    }

    #[test]
    fn test_encode_bulk_string() {
        let mut encoder = RespEncoder::new();
        let value = RespValue::BulkString(Bytes::from("foobar"));
        let encoded = encoder.encode(&value).unwrap();
        assert_eq!(&encoded[..], b"$6\r\nfoobar\r\n");
    }

    #[test]
    fn test_encode_null() {
        let mut encoder = RespEncoder::new();
        let value = RespValue::Null;
        let encoded = encoder.encode(&value).unwrap();
        assert_eq!(&encoded[..], b"$-1\r\n");
    }

    #[test]
    fn test_encode_array() {
        let mut encoder = RespEncoder::new();
        let value = RespValue::Array(vec![
            RespValue::BulkString(Bytes::from("foo")),
            RespValue::BulkString(Bytes::from("bar")),
        ]);
        let encoded = encoder.encode(&value).unwrap();
        assert_eq!(&encoded[..], b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    }

    #[test]
    fn test_encode_command() {
        let mut encoder = RespEncoder::new();
        let bytes = encoder
            .encode_command("GET", &[RespValue::BulkString(Bytes::from("mykey"))])
            .unwrap();
        assert_eq!(&bytes[..], b"*2\r\n$3\r\nGET\r\n$5\r\nmykey\r\n");
    }

    #[test]
    fn test_decode_simple_string() {
        let data = b"+OK\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, RespValue::SimpleString("OK".to_string()));
    }

    #[test]
    fn test_decode_error() {
        let data = b"-ERR unknown\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, RespValue::Error("ERR unknown".to_string()));
    }

    #[test]
    fn test_decode_integer() {
        let data = b":1000\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, RespValue::Integer(1000));
    }

    #[test]
    fn test_decode_bulk_string() {
        let data = b"$6\r\nfoobar\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, RespValue::BulkString(Bytes::from("foobar")));
    }

    #[test]
    fn test_decode_null() {
        let data = b"$-1\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, RespValue::Null);
    }

    #[test]
    fn test_decode_array() {
        let data = b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(
            value,
            RespValue::Array(vec![
                RespValue::BulkString(Bytes::from("foo")),
                RespValue::BulkString(Bytes::from("bar")),
            ])
        );
    }

    #[test]
    fn test_decode_incomplete_data() {
        let data = b"+OK\r";
        let mut cursor = Cursor::new(&data[..]);
        let result = RespDecoder::decode(&mut cursor).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_roundtrip() {
        let original = RespValue::Array(vec![
            RespValue::SimpleString("OK".to_string()),
            RespValue::Integer(42),
            RespValue::BulkString(Bytes::from("test")),
            RespValue::Null,
        ]);
        let mut encoder = RespEncoder::new();
        let encoded = encoder.encode(&original).unwrap();
        let mut cursor = Cursor::new(&encoded[..]);
        let decoded = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_size_estimation() {
        let value = RespValue::SimpleString("OK".to_string());
        let estimated = RespEncoder::estimate_size(&value);
        assert_eq!(estimated, 5);

        let value = RespValue::BulkString(Bytes::from("hello"));
        let estimated = RespEncoder::estimate_size(&value);
        assert_eq!(estimated, 11);
    }
}
