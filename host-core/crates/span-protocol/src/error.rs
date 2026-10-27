/// Failure of a Span codec, validator, or authenticator.
///
/// Variants describe what the peer or the local caller must change. They do
/// not include key material, plaintext, or certificate bytes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtocolError {
    #[error("buffer is shorter than a full message")]
    UnexpectedEof,
    #[error("unsupported version {found}")]
    UnsupportedVersion { found: u32 },
    #[error("unknown media packet type {found}")]
    UnknownPacketType { found: u8 },
    #[error("fragment {index} is outside count {count}")]
    InvalidFragment { index: u16, count: u16 },
    #[error("declared payload {declared} exceeds cap {cap}")]
    PayloadTooLarge { declared: u32, cap: u32 },
    #[error("payload length {declared} does not match the buffer")]
    PayloadLengthMismatch { declared: u32 },
    #[error("control payload is not valid protobuf")]
    MalformedPayload,
    #[error("field {field} is not valid")]
    InvalidField { field: &'static str },
    #[error("media payload failed authentication")]
    Authentication,
    #[error("clock sample is inconsistent")]
    InconsistentClock,
    #[error("pairing text is not a Span offer")]
    BadPairing,
    #[error("preset document is not valid: {reason}")]
    BadPreset { reason: &'static str },
}
