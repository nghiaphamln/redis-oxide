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

    /// Sentinel error
    #[error("Sentinel error: {0}")]
    Sentinel(String),

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
    #[must_use]
    pub fn parse_redirect(msg: &str) -> Option<Self> {
        if let Some(moved_str) = msg.strip_prefix("MOVED ") {
            let parts: Vec<&str> = moved_str.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(slot) = parts[0].parse::<u16>() {
                    if let Some((host, port)) = parts[1].rsplit_once(':') {
                        if let Ok(port) = port.parse::<u16>() {
                            return Some(Self::Moved {
                                slot,
                                host: host.to_string(),
                                port,
                            });
                        }
                    }
                }
            }
        }

        if let Some(ask_str) = msg.strip_prefix("ASK ") {
            let parts: Vec<&str> = ask_str.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(slot) = parts[0].parse::<u16>() {
                    if let Some((host, port)) = parts[1].rsplit_once(':') {
                        if let Ok(port) = port.parse::<u16>() {
                            return Some(Self::Ask {
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
    #[must_use]
    pub const fn is_redirect(&self) -> bool {
        matches!(self, Self::Moved { .. } | Self::Ask { .. })
    }

    /// Get the target address from a redirect error
    #[must_use]
    pub fn redirect_target(&self) -> Option<(String, u16)> {
        match self {
            Self::Moved { host, port, .. } | Self::Ask { host, port, .. } => {
                Some((host.clone(), *port))
            }
            _ => None,
        }
    }

    /// Get the slot number from a redirect error
    #[must_use]
    pub const fn redirect_slot(&self) -> Option<u16> {
        match self {
            Self::Moved { slot, .. } | Self::Ask { slot, .. } => Some(*slot),
            _ => None,
        }
    }
}
