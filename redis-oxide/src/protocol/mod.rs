//! Redis protocol implementations
//!
//! This module contains implementations for both RESP2 and RESP3 protocols,
//! providing encoding and decoding functionality for Redis communication.

pub mod resp2;
pub mod resp2_optimized;
pub mod resp3;

// Re-export the existing protocol functionality
pub use resp2::{RespDecoder, RespEncoder};
pub use resp2_optimized::{OptimizedRespDecoder, OptimizedRespEncoder};
pub use resp3::{Resp3Decoder, Resp3Encoder, Resp3Value};

/// Protocol version enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtocolVersion {
    /// RESP2 (Redis Serialization Protocol version 2)
    #[default]
    Resp2,
    /// RESP3 (Redis Serialization Protocol version 3)
    Resp3,
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resp2 => write!(f, "RESP2"),
            Self::Resp3 => write!(f, "RESP3"),
        }
    }
}

/// Protocol negotiation result
#[derive(Debug, Clone)]
pub struct ProtocolNegotiation {
    /// The negotiated protocol version
    pub version: ProtocolVersion,
    /// Server capabilities (for RESP3)
    pub capabilities: Vec<String>,
}

impl ProtocolNegotiation {
    /// Create a new protocol negotiation result
    #[must_use]
    pub const fn new(version: ProtocolVersion) -> Self {
        Self {
            version,
            capabilities: Vec::new(),
        }
    }

    /// Create a RESP3 negotiation with capabilities
    #[must_use]
    pub fn resp3_with_capabilities(capabilities: Vec<String>) -> Self {
        Self {
            version: ProtocolVersion::Resp3,
            capabilities,
        }
    }

    /// Check if a capability is supported
    #[must_use]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
}

/// Protocol negotiator for handling RESP2/RESP3 protocol selection
pub struct ProtocolNegotiator {
    preferred_version: ProtocolVersion,
}

impl ProtocolNegotiator {
    /// Create a new protocol negotiator
    #[must_use]
    pub const fn new(preferred_version: ProtocolVersion) -> Self {
        Self { preferred_version }
    }

    /// Negotiate protocol version with the server
    ///
    /// This method attempts to negotiate the preferred protocol version.
    /// If RESP3 is preferred, it sends a HELLO command to negotiate.
    /// Falls back to RESP2 if negotiation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if protocol negotiation fails completely.
    pub async fn negotiate<T>(
        &self,
        connection: &mut T,
    ) -> crate::core::error::RedisResult<ProtocolNegotiation>
    where
        T: ProtocolConnection,
    {
        match self.preferred_version {
            ProtocolVersion::Resp2 => Ok(ProtocolNegotiation::new(ProtocolVersion::Resp2)),
            ProtocolVersion::Resp3 => {
                // Try to negotiate RESP3
                match self.try_negotiate_resp3(connection).await {
                    Ok(negotiation) => Ok(negotiation),
                    Err(_) => {
                        // Fall back to RESP2
                        Ok(ProtocolNegotiation::new(ProtocolVersion::Resp2))
                    }
                }
            }
        }
    }

    async fn try_negotiate_resp3<T>(
        &self,
        connection: &mut T,
    ) -> crate::core::error::RedisResult<ProtocolNegotiation>
    where
        T: ProtocolConnection,
    {
        // Send HELLO 3 command to negotiate RESP3
        let hello_cmd = crate::core::value::RespValue::Array(vec![
            crate::core::value::RespValue::BulkString(bytes::Bytes::from("HELLO")),
            crate::core::value::RespValue::BulkString(bytes::Bytes::from("3")),
        ]);

        connection.send_command(&hello_cmd).await?;
        let response = connection.read_response().await?;

        // Parse HELLO response to extract capabilities
        match response {
            crate::core::value::RespValue::Array(items) => {
                let mut capabilities = Vec::new();

                // HELLO response format: [server, version, proto, capabilities...]
                if items.len() >= 4 {
                    // Extract capabilities from the response
                    for item in items.iter().skip(3) {
                        if let crate::core::value::RespValue::BulkString(cap) = item {
                            if let Ok(cap_str) = String::from_utf8(cap.to_vec()) {
                                capabilities.push(cap_str);
                            }
                        }
                    }
                }

                Ok(ProtocolNegotiation::resp3_with_capabilities(capabilities))
            }
            _ => Err(crate::core::error::RedisError::Protocol(
                "Invalid HELLO response".to_string(),
            )),
        }
    }
}

impl Default for ProtocolNegotiator {
    fn default() -> Self {
        Self::new(ProtocolVersion::Resp2)
    }
}

/// Trait for connections that support protocol negotiation
#[async_trait::async_trait]
pub trait ProtocolConnection {
    /// Send a command to the server
    async fn send_command(
        &mut self,
        command: &crate::core::value::RespValue,
    ) -> crate::core::error::RedisResult<()>;

    /// Read a response from the server
    async fn read_response(
        &mut self,
    ) -> crate::core::error::RedisResult<crate::core::value::RespValue>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version_display() {
        assert_eq!(ProtocolVersion::Resp2.to_string(), "RESP2");
        assert_eq!(ProtocolVersion::Resp3.to_string(), "RESP3");
    }

    #[test]
    fn test_protocol_negotiation() {
        let negotiation = ProtocolNegotiation::new(ProtocolVersion::Resp2);
        assert_eq!(negotiation.version, ProtocolVersion::Resp2);
        assert!(negotiation.capabilities.is_empty());

        let negotiation = ProtocolNegotiation::resp3_with_capabilities(vec![
            "push".to_string(),
            "streams".to_string(),
        ]);
        assert_eq!(negotiation.version, ProtocolVersion::Resp3);
        assert!(negotiation.has_capability("push"));
        assert!(negotiation.has_capability("streams"));
        assert!(!negotiation.has_capability("unknown"));
    }

    #[test]
    fn test_protocol_negotiator() {
        let negotiator = ProtocolNegotiator::new(ProtocolVersion::Resp3);
        assert_eq!(negotiator.preferred_version, ProtocolVersion::Resp3);

        let negotiator = ProtocolNegotiator::default();
        assert_eq!(negotiator.preferred_version, ProtocolVersion::Resp2);
    }
}
