//! Error types for Redis operations

use std::io;
use thiserror::Error;

/// Result type for Redis operations
pub type RedisResult<T> = Result<T, RedisError>;

/// Comprehensive error type for Redis operations
#[derive(Error, Debug)]
pub enum RedisError {
    /// IO error during network operations
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Protocol parsing error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Server returned an error
    #[error("Server error: {0}")]
    Server(String),

    /// MOVED redirect in cluster mode
    #[error("MOVED redirect: slot {slot} to {host}:{port}")]
    Moved {
        /// Slot number that was moved
        slot: u16,
        /// Target host
        host: String,
        /// Target port
        port: u16,
    },

    /// ASK redirect in cluster mode
    #[error("ASK redirect: slot {slot} to {host}:{port}")]
    Ask {
        /// Slot number for temporary redirect
        slot: u16,
        /// Target host
        host: String,
        /// Target port
        port: u16,
    },

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Type conversion error
    #[error("Type conversion error: {0}")]
    Type(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    Config(String),

    /// Cluster error
    #[error("Cluster error: {0}")]
    Cluster(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// Pool error
    #[error("Pool error: {0}")]
    Pool(String),

    /// Maximum retry attempts exceeded
    #[error("Maximum retry attempts ({0}) exceeded")]
    MaxRetriesExceeded(usize),

    /// Unexpected response from server
    #[error("Unexpected response: {0}")]
    UnexpectedResponse(String),
}

impl RedisError {
    /// Parse a Redis error message to check for MOVED or ASK redirects
    pub fn parse_redirect(msg: &str) -> Option<Self> {
        // Parse "MOVED 9916 10.90.6.213:6002"
        if let Some(moved_str) = msg.strip_prefix("MOVED ") {
            let parts: Vec<&str> = moved_str.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(slot) = parts[0].parse::<u16>() {
                    if let Some((host, port)) = parts[1].rsplit_once(':') {
                        if let Ok(port) = port.parse::<u16>() {
                            return Some(RedisError::Moved {
                                slot,
                                host: host.to_string(),
                                port,
                            });
                        }
                    }
                }
            }
        }

        // Parse "ASK 9916 10.90.6.213:6002"
        if let Some(ask_str) = msg.strip_prefix("ASK ") {
            let parts: Vec<&str> = ask_str.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(slot) = parts[0].parse::<u16>() {
                    if let Some((host, port)) = parts[1].rsplit_once(':') {
                        if let Ok(port) = port.parse::<u16>() {
                            return Some(RedisError::Ask {
                                slot,
                                host: host.to_string(),
                                port,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if this error is a redirect (MOVED or ASK)
    pub fn is_redirect(&self) -> bool {
        matches!(self, RedisError::Moved { .. } | RedisError::Ask { .. })
    }

    /// Get the target address from a redirect error
    pub fn redirect_target(&self) -> Option<(String, u16)> {
        match self {
            RedisError::Moved { host, port, .. } | RedisError::Ask { host, port, .. } => {
                Some((host.clone(), *port))
            }
            _ => None,
        }
    }

    /// Get the slot number from a redirect error
    pub fn redirect_slot(&self) -> Option<u16> {
        match self {
            RedisError::Moved { slot, .. } | RedisError::Ask { slot, .. } => Some(*slot),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_moved_redirect() {
        let error = RedisError::parse_redirect("MOVED 9916 10.90.6.213:6002");
        assert!(error.is_some());

        if let Some(RedisError::Moved { slot, host, port }) = error {
            assert_eq!(slot, 9916);
            assert_eq!(host, "10.90.6.213");
            assert_eq!(port, 6002);
        } else {
            panic!("Expected MOVED error");
        }
    }

    #[test]
    fn test_parse_ask_redirect() {
        let error = RedisError::parse_redirect("ASK 1234 192.168.1.1:7000");
        assert!(error.is_some());

        if let Some(RedisError::Ask { slot, host, port }) = error {
            assert_eq!(slot, 1234);
            assert_eq!(host, "192.168.1.1");
            assert_eq!(port, 7000);
        } else {
            panic!("Expected ASK error");
        }
    }

    #[test]
    fn test_parse_invalid_redirect() {
        assert!(RedisError::parse_redirect("ERR invalid").is_none());
        assert!(RedisError::parse_redirect("MOVED invalid").is_none());
        assert!(RedisError::parse_redirect("MOVED 1234").is_none());
    }

    #[test]
    fn test_is_redirect() {
        let moved = RedisError::Moved {
            slot: 100,
            host: "localhost".to_string(),
            port: 6379,
        };
        assert!(moved.is_redirect());

        let ask = RedisError::Ask {
            slot: 100,
            host: "localhost".to_string(),
            port: 6379,
        };
        assert!(ask.is_redirect());

        let other = RedisError::Connection("test".to_string());
        assert!(!other.is_redirect());
    }

    #[test]
    fn test_redirect_target() {
        let moved = RedisError::Moved {
            slot: 100,
            host: "10.90.6.213".to_string(),
            port: 6002,
        };
        let target = moved.redirect_target();
        assert_eq!(target, Some(("10.90.6.213".to_string(), 6002)));
    }

    #[test]
    fn test_redirect_slot() {
        let moved = RedisError::Moved {
            slot: 9916,
            host: "10.90.6.213".to_string(),
            port: 6002,
        };
        assert_eq!(moved.redirect_slot(), Some(9916));
    }
}
