//! Cleartext media datagram header.
//!
//! The header is authenticated as AEAD additional data by [`crate::seal_media`]
//! but is readable before decryption so a receiver can notice a lost sequence
//! and ask for a new keyframe. Payload bytes are opaque here.

use crate::error::ProtocolError;
use crate::read::Reader;

pub const HEADER_LEN: usize = 32;
pub const TAG_LEN: usize = 16;
pub const MAX_DATAGRAM: usize = 1200;
pub const MAX_PAYLOAD: usize = MAX_DATAGRAM - HEADER_LEN;
pub const MAX_PLAINTEXT: usize = MAX_PAYLOAD - TAG_LEN;

pub const PACKET_VIDEO: u8 = 0;
pub const PACKET_AUDIO: u8 = 1;
pub const PACKET_NACK: u8 = 2;
pub const PACKET_KEEPALIVE: u8 = 3;

pub const FLAG_KEYFRAME: u16 = 1 << 0;
pub const FLAG_END_OF_FRAME: u16 = 1 << 1;
pub const FLAG_CONFIG: u16 = 1 << 2;
pub const FLAG_DISCONTINUITY: u16 = 1 << 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaHeader {
    pub version: u8,
    pub packet_type: u8,
    pub flags: u16,
    pub stream_id: u32,
    pub sequence: u64,
    pub timestamp_us: u64,
    pub fragment_index: u16,
    pub fragment_count: u16,
    pub payload_len: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaPacket<'a> {
    pub header: MediaHeader,
    pub payload: &'a [u8],
}

impl MediaHeader {
    pub fn check_structure(&self) -> Result<(), ProtocolError> {
        if self.version != 1 {
            return Err(ProtocolError::UnsupportedVersion {
                found: u32::from(self.version),
            });
        }
        if self.packet_type > PACKET_KEEPALIVE {
            return Err(ProtocolError::UnknownPacketType {
                found: self.packet_type,
            });
        }
        if self.fragment_count == 0 || self.fragment_index >= self.fragment_count {
            return Err(ProtocolError::InvalidFragment {
                index: self.fragment_index,
                count: self.fragment_count,
            });
        }
        if self.payload_len as usize > MAX_PAYLOAD {
            return Err(ProtocolError::PayloadTooLarge {
                declared: self.payload_len,
                cap: MAX_PAYLOAD as u32,
            });
        }
        if self.packet_type != PACKET_KEEPALIVE && self.stream_id == 0 {
            return Err(ProtocolError::InvalidField { field: "stream_id" });
        }
        Ok(())
    }

    pub fn encode(&self, payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        if payload.len() != self.payload_len as usize {
            return Err(ProtocolError::PayloadLengthMismatch {
                declared: self.payload_len,
            });
        }
        self.check_structure()?;
        let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
        out.push(self.version);
        out.push(self.packet_type);
        out.extend_from_slice(&self.flags.to_be_bytes());
        out.extend_from_slice(&self.stream_id.to_be_bytes());
        out.extend_from_slice(&self.sequence.to_be_bytes());
        out.extend_from_slice(&self.timestamp_us.to_be_bytes());
        out.extend_from_slice(&self.fragment_index.to_be_bytes());
        out.extend_from_slice(&self.fragment_count.to_be_bytes());
        out.extend_from_slice(&self.payload_len.to_be_bytes());
        out.extend_from_slice(payload);
        Ok(out)
    }
}

pub fn decode(buf: &[u8]) -> Result<MediaPacket<'_>, ProtocolError> {
    let mut reader = Reader::new(buf);
    let header = MediaHeader {
        version: reader.u8()?,
        packet_type: reader.u8()?,
        flags: reader.u16()?,
        stream_id: reader.u32()?,
        sequence: reader.u64()?,
        timestamp_us: reader.u64()?,
        fragment_index: reader.u16()?,
        fragment_count: reader.u16()?,
        payload_len: reader.u32()?,
    };
    header.check_structure()?;
    let available = reader.remaining();
    if available < header.payload_len as usize {
        return Err(ProtocolError::UnexpectedEof);
    }
    if available != header.payload_len as usize {
        return Err(ProtocolError::PayloadLengthMismatch {
            declared: header.payload_len,
        });
    }
    let payload = reader.take(header.payload_len as usize)?;
    Ok(MediaPacket { header, payload })
}

pub fn check_plaintext(packet_type: u8, plaintext: &[u8]) -> Result<(), ProtocolError> {
    match packet_type {
        PACKET_VIDEO | PACKET_AUDIO => {
            if plaintext.is_empty() {
                return Err(ProtocolError::InvalidField {
                    field: "media plaintext",
                });
            }
        }
        PACKET_NACK => {
            nack_sequences(plaintext).map(|_| ())?;
        }
        PACKET_KEEPALIVE => {
            if !plaintext.is_empty() {
                return Err(ProtocolError::InvalidField { field: "keepalive" });
            }
        }
        other => {
            return Err(ProtocolError::UnknownPacketType { found: other });
        }
    }
    if plaintext.len() > MAX_PLAINTEXT {
        return Err(ProtocolError::PayloadTooLarge {
            declared: plaintext.len() as u32,
            cap: MAX_PLAINTEXT as u32,
        });
    }
    Ok(())
}

/// NACK plaintext is a list of 1 to 32 big-endian sequence numbers.
pub fn nack_sequences(plaintext: &[u8]) -> Result<Vec<u64>, ProtocolError> {
    if plaintext.is_empty() || plaintext.len() % 8 != 0 || plaintext.len() > 32 * 8 {
        return Err(ProtocolError::InvalidField { field: "nack" });
    }
    let mut reader = Reader::new(plaintext);
    let mut seqs = Vec::with_capacity(plaintext.len() / 8);
    while reader.remaining() > 0 {
        seqs.push(reader.u64()?);
    }
    Ok(seqs)
}
