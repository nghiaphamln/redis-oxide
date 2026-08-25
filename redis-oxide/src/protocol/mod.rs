//! Redis protocol codecs.

pub mod resp2;
pub mod resp3;

pub use crate::core::config::ProtocolVersion;
pub use resp2::{RespDecoder, RespEncoder};
pub use resp3::{Resp3Decoder, Resp3Encoder, Resp3Value};

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resp2 => formatter.write_str("RESP2"),
            Self::Resp3 => formatter.write_str("RESP3"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_protocol_versions() {
        assert_eq!(ProtocolVersion::Resp2.to_string(), "RESP2");
        assert_eq!(ProtocolVersion::Resp3.to_string(), "RESP3");
    }
}
