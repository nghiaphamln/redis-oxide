//! Optimized RESP2 protocol implementation
//!
//! This module provides optimized versions of RESP2 encoding and decoding
//! with focus on memory allocation reduction and performance improvements.

#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(missing_docs)]

use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io::Cursor;

const CRLF: &[u8] = b"\r\n";

// Static strings for common responses to avoid allocations
const OK_RESPONSE: &str = "OK";
const PONG_RESPONSE: &str = "PONG";
const QUEUED_RESPONSE: &str = "QUEUED";

/// Optimized RESP2 encoder with buffer pre-sizing and zero-copy optimizations
pub struct OptimizedRespEncoder {
    // Reusable buffer to avoid allocations
    buffer: BytesMut,
}

impl OptimizedRespEncoder {
    /// Create a new optimized encoder
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(1024), // Start with reasonable capacity
        }
    }

    /// Create a new encoder with specific initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(capacity),
        }
    }

    /// Estimate buffer size needed for a RESP value
    fn estimate_size(value: &RespValue) -> usize {
        match value {
            RespValue::SimpleString(s) => 1 + s.len() + 2, // +str\r\n
            RespValue::Error(e) => 1 + e.len() + 2,        // -err\r\n
            RespValue::Integer(i) => 1 + i.to_string().len() + 2, // :num\r\n
            RespValue::BulkString(b) => {
                let len_str = b.len().to_string();
                1 + len_str.len() + 2 + b.len() + 2 // $len\r\ndata\r\n
            }
            RespValue::Null => 5, // $-1\r\n
            RespValue::Array(arr) => {
                let len_str = arr.len().to_string();
                let mut size = 1 + len_str.len() + 2; // *len\r\n
                for item in arr {
                    size += Self::estimate_size(item);
                }
                size
            }
        }
    }

    /// Estimate buffer size needed for a command with arguments
    fn estimate_command_size(command: &str, args: &[RespValue]) -> usize {
        let total_items = 1 + args.len();
        let array_header = 1 + total_items.to_string().len() + 2; // *count\r\n

        // Command size
        let cmd_size = 1 + command.len().to_string().len() + 2 + command.len() + 2; // $len\r\ncmd\r\n

        // Arguments size
        let args_size: usize = args.iter().map(Self::estimate_size).sum();

        array_header + cmd_size + args_size
    }

    /// Encode a RESP value into the internal buffer with pre-sizing
    pub fn encode(&mut self, value: &RespValue) -> RedisResult<Bytes> {
        let estimated_size = Self::estimate_size(value);

        // Reserve capacity if needed
        if self.buffer.capacity() < estimated_size {
            self.buffer.reserve(estimated_size);
        }

        self.buffer.clear();
        self.encode_value(value)?;
        Ok(self.buffer.split().freeze())
    }

    /// Encode a command with arguments using pre-sizing
    pub fn encode_command(&mut self, command: &str, args: &[RespValue]) -> RedisResult<Bytes> {
        let estimated_size = Self::estimate_command_size(command, args);

        // Reserve capacity if needed
        if self.buffer.capacity() < estimated_size {
            self.buffer.reserve(estimated_size);
        }

        self.buffer.clear();

        // Create array with command + args
        let total_len = 1 + args.len();
        self.buffer.put_u8(b'*');
        self.put_integer_bytes(total_len);
        self.buffer.put_slice(CRLF);

        // Encode command as bulk string
        self.buffer.put_u8(b'$');
        self.put_integer_bytes(command.len());
        self.buffer.put_slice(CRLF);
        self.buffer.put_slice(command.as_bytes());
        self.buffer.put_slice(CRLF);

        // Encode arguments
        for arg in args {
            self.encode_value(arg)?;
        }

        Ok(self.buffer.split().freeze())
    }

    /// Internal method to encode a value into the buffer
    fn encode_value(&mut self, value: &RespValue) -> RedisResult<()> {
        match value {
            RespValue::SimpleString(s) => {
                self.buffer.put_u8(b'+');
                // Use static strings for common responses
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

    /// Optimized integer to bytes conversion
    fn put_integer_bytes<T: itoa::Integer>(&mut self, value: T) {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        self.buffer.put_slice(s.as_bytes());
    }

    /// Get the current buffer capacity
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Clear the internal buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Reserve additional capacity
    pub fn reserve(&mut self, additional: usize) {
        self.buffer.reserve(additional);
    }
}

impl Default for OptimizedRespEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimized RESP2 decoder with streaming support and reduced allocations
pub struct OptimizedRespDecoder {
    buffer: BytesMut,
    // String cache for frequently used strings
    string_cache: std::collections::HashMap<Vec<u8>, String>,
    max_cache_size: usize,
}

impl OptimizedRespDecoder {
    /// Create a new optimized decoder
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(4096),
            string_cache: std::collections::HashMap::new(),
            max_cache_size: 1000, // Limit cache size to prevent memory leaks
        }
    }

    /// Create a new decoder with specific buffer capacity and cache size
    pub fn with_config(buffer_capacity: usize, max_cache_size: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(buffer_capacity),
            string_cache: std::collections::HashMap::new(),
            max_cache_size,
        }
    }

    /// Decode data with streaming support
    pub fn decode_streaming(&mut self, data: &[u8]) -> RedisResult<Vec<RespValue>> {
        self.buffer.extend_from_slice(data);
        let mut results = Vec::new();

        loop {
            let buffer_len = self.buffer.len();
            if buffer_len == 0 {
                break;
            }

            let buffer_slice = self.buffer.clone().freeze();
            let mut cursor = Cursor::new(&buffer_slice[..]);

            match self.try_decode_value(&mut cursor)? {
                Some(value) => {
                    let consumed = cursor.position() as usize;
                    self.buffer.advance(consumed);
                    results.push(value);
                }
                None => break, // Need more data
            }
        }

        Ok(results)
    }

    /// Try to decode a single value, returning None if more data is needed
    fn try_decode_value(&mut self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        if !cursor.has_remaining() {
            return Ok(None);
        }

        let type_byte = cursor.chunk()[0];
        cursor.advance(1);

        match type_byte {
            b'+' => self.try_decode_simple_string(cursor),
            b'-' => self.try_decode_error(cursor),
            b':' => self.try_decode_integer(cursor),
            b'$' => self.try_decode_bulk_string(cursor),
            b'*' => self.try_decode_array(cursor),
            _ => Err(RedisError::Protocol(format!(
                "Invalid RESP type byte: {}",
                type_byte as char
            ))),
        }
    }

    /// Try to decode a simple string with caching
    fn try_decode_simple_string(
        &mut self,
        cursor: &mut Cursor<&[u8]>,
    ) -> RedisResult<Option<RespValue>> {
        if let Some(line) = self.try_read_line(cursor)? {
            let string = self.bytes_to_string_cached(&line)?;
            Ok(Some(RespValue::SimpleString(string)))
        } else {
            Ok(None)
        }
    }

    /// Try to decode an error string
    fn try_decode_error(&mut self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        if let Some(line) = self.try_read_line(cursor)? {
            let string = String::from_utf8(line)
                .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8 in error: {e}")))?;
            Ok(Some(RespValue::Error(string)))
        } else {
            Ok(None)
        }
    }

    /// Try to decode an integer
    fn try_decode_integer(&mut self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        if let Some(line) = self.try_read_line(cursor)? {
            let s = String::from_utf8(line)
                .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8 in integer: {e}")))?;
            let num = s
                .parse::<i64>()
                .map_err(|e| RedisError::Protocol(format!("Invalid integer format: {e}")))?;
            Ok(Some(RespValue::Integer(num)))
        } else {
            Ok(None)
        }
    }

    /// Try to decode a bulk string
    fn try_decode_bulk_string(
        &mut self,
        cursor: &mut Cursor<&[u8]>,
    ) -> RedisResult<Option<RespValue>> {
        let len_line = match self.try_read_line(cursor)? {
            Some(line) => line,
            None => return Ok(None),
        };

        let len_str = String::from_utf8(len_line).map_err(|e| {
            RedisError::Protocol(format!("Invalid UTF-8 in bulk string length: {e}"))
        })?;
        let len = len_str
            .parse::<isize>()
            .map_err(|e| RedisError::Protocol(format!("Invalid bulk string length: {e}")))?;

        if len == -1 {
            return Ok(Some(RespValue::Null));
        }

        if len < 0 {
            return Err(RedisError::Protocol(
                "Invalid bulk string length".to_string(),
            ));
        }

        let len = len as usize;
        if cursor.remaining() < len + 2 {
            return Ok(None); // Need more data
        }

        let data = cursor.chunk()[..len].to_vec();
        cursor.advance(len);

        // Check for CRLF
        if cursor.remaining() < 2 || &cursor.chunk()[..2] != CRLF {
            return Err(RedisError::Protocol(
                "Missing CRLF after bulk string".to_string(),
            ));
        }
        cursor.advance(2);

        Ok(Some(RespValue::BulkString(Bytes::from(data))))
    }

    /// Try to decode an array
    fn try_decode_array(&mut self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        let len_line = match self.try_read_line(cursor)? {
            Some(line) => line,
            None => return Ok(None),
        };

        let len_str = String::from_utf8(len_line)
            .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8 in array length: {e}")))?;
        let len = len_str
            .parse::<isize>()
            .map_err(|e| RedisError::Protocol(format!("Invalid array length: {e}")))?;

        if len == -1 {
            return Ok(Some(RespValue::Null));
        }

        if len < 0 {
            return Err(RedisError::Protocol("Invalid array length".to_string()));
        }

        let len = len as usize;
        let mut elements = Vec::with_capacity(len);

        for _ in 0..len {
            match self.try_decode_value(cursor)? {
                Some(element) => elements.push(element),
                None => return Ok(None), // Need more data
            }
        }

        Ok(Some(RespValue::Array(elements)))
    }

    /// Try to read a line, returning None if incomplete
    fn try_read_line(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Option<Vec<u8>>> {
        let start_pos = cursor.position() as usize;
        let remaining = cursor.get_ref();

        // Look for CRLF
        for (i, window) in remaining[start_pos..].windows(2).enumerate() {
            if window == CRLF {
                let line_end = start_pos + i;
                let line = remaining[start_pos..line_end].to_vec();
                cursor.advance(i + 2); // Skip line + CRLF
                return Ok(Some(line));
            }
        }

        Ok(None) // No complete line found
    }

    /// Convert bytes to string with caching for frequently used strings
    fn bytes_to_string_cached(&mut self, bytes: &[u8]) -> RedisResult<String> {
        // Check cache first
        if let Some(cached) = self.string_cache.get(bytes) {
            return Ok(cached.clone());
        }

        let string = String::from_utf8(bytes.to_vec())
            .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {e}")))?;

        // Cache the string if cache isn't full
        if self.string_cache.len() < self.max_cache_size {
            self.string_cache.insert(bytes.to_vec(), string.clone());
        }

        Ok(string)
    }

    /// Clear the string cache
    pub fn clear_cache(&mut self) {
        self.string_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.string_cache.len(), self.max_cache_size)
    }
}

impl Default for OptimizedRespDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_encoder_simple_string() {
        let mut encoder = OptimizedRespEncoder::new();
        let value = RespValue::SimpleString("OK".to_string());
        let encoded = encoder.encode(&value).unwrap();
        assert_eq!(encoded, Bytes::from("+OK\r\n"));
    }

    #[test]
    fn test_optimized_encoder_command() {
        let mut encoder = OptimizedRespEncoder::new();
        let args = vec![RespValue::from("mykey")];
        let encoded = encoder.encode_command("GET", &args).unwrap();

        let expected = "*2\r\n$3\r\nGET\r\n$5\r\nmykey\r\n";
        assert_eq!(encoded, Bytes::from(expected));
    }

    #[test]
    fn test_optimized_decoder_streaming() {
        let mut decoder = OptimizedRespDecoder::new();

        // Test partial data
        let partial1 = b"+OK\r\n:42\r\n$5\r\nhel";
        let results1 = decoder.decode_streaming(partial1).unwrap();
        assert_eq!(results1.len(), 2); // Should decode +OK and :42

        // Complete the bulk string
        let partial2 = b"lo\r\n";
        let results2 = decoder.decode_streaming(partial2).unwrap();
        assert_eq!(results2.len(), 1); // Should decode the bulk string

        match &results2[0] {
            RespValue::BulkString(b) => assert_eq!(b, &Bytes::from("hello")),
            _ => panic!("Expected bulk string"),
        }
    }

    #[test]
    fn test_size_estimation() {
        let value = RespValue::SimpleString("OK".to_string());
        let estimated = OptimizedRespEncoder::estimate_size(&value);
        assert_eq!(estimated, 5); // +OK\r\n

        let value = RespValue::BulkString(Bytes::from("hello"));
        let estimated = OptimizedRespEncoder::estimate_size(&value);
        assert_eq!(estimated, 11); // $5\r\nhello\r\n
    }

    #[test]
    fn test_string_caching() {
        let mut decoder = OptimizedRespDecoder::new();

        // Decode the same string multiple times
        let data = b"+OK\r\n+OK\r\n";
        let results = decoder.decode_streaming(data).unwrap();

        assert_eq!(results.len(), 2);
        let (cache_size, _) = decoder.cache_stats();
        assert_eq!(cache_size, 1); // "OK" should be cached
    }
}
