//! Media key derivation and ChaCha20-Poly1305.
//!
//! The control channel is TLS 1.2. iOS 9 does not have a public AES-GCM API
//! and its TLS exporter is not part of the contract, so the media key is
//! derived inside the already authenticated session. Sequence numbers are
//! AEAD nonces: the session must not reuse a sequence on the same key.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

use crate::error::ProtocolError;
use crate::media::{self, MediaHeader};

const INFO_PREFIX: &[u8] = b"span-media-v1";

pub fn derive_media_key(
    nonce_host: &[u8; 32],
    nonce_device: &[u8; 32],
    session_id: &[u8; 16],
) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(nonce_host), nonce_device);
    let mut info = Vec::with_capacity(INFO_PREFIX.len() + session_id.len());
    info.extend_from_slice(INFO_PREFIX);
    info.extend_from_slice(session_id);
    let mut okm = [0u8; 32];
    // 32 bytes is a single SHA-256 block, inside the HKDF-Expand limit.
    hk.expand(&info, &mut okm)
        .expect("32-byte HKDF output is within the expand limit");
    okm
}

pub fn nonce(stream_id: u32, sequence: u64) -> [u8; 12] {
    let mut out = [0u8; 12];
    out[..4].copy_from_slice(&stream_id.to_be_bytes());
    out[4..].copy_from_slice(&sequence.to_be_bytes());
    out
}

pub fn seal(
    key: &[u8; 32],
    header: &MediaHeader,
    plaintext: &[u8],
) -> Result<Vec<u8>, ProtocolError> {
    media::check_plaintext(header.packet_type, plaintext)?;
    let cipher_len =
        plaintext
            .len()
            .checked_add(media::TAG_LEN)
            .ok_or(ProtocolError::PayloadTooLarge {
                declared: u32::MAX,
                cap: media::MAX_PAYLOAD as u32,
            })?;
    if cipher_len > media::MAX_PAYLOAD {
        return Err(ProtocolError::PayloadTooLarge {
            declared: cipher_len as u32,
            cap: media::MAX_PAYLOAD as u32,
        });
    }
    let mut sealed_header = *header;
    sealed_header.version = 1;
    sealed_header.payload_len = cipher_len as u32;
    sealed_header.check_structure()?;

    let mut packet = Vec::with_capacity(media::HEADER_LEN + cipher_len);
    packet.extend_from_slice(&header_bytes(&sealed_header));
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| ProtocolError::InvalidField { field: "media key" })?;
    let nonce_bytes = nonce(sealed_header.stream_id, sealed_header.sequence);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad: &packet,
            },
        )
        .map_err(|_| ProtocolError::Authentication)?;
    packet.extend_from_slice(&ciphertext);
    Ok(packet)
}

pub fn open(key: &[u8; 32], datagram: &[u8]) -> Result<(MediaHeader, Vec<u8>), ProtocolError> {
    let parsed = media::decode(datagram)?;
    if parsed.payload.len() < media::TAG_LEN {
        return Err(ProtocolError::Authentication);
    }
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|_| ProtocolError::InvalidField { field: "media key" })?;
    let nonce_bytes = nonce(parsed.header.stream_id, parsed.header.sequence);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: parsed.payload,
                aad: &datagram[..media::HEADER_LEN],
            },
        )
        .map_err(|_| ProtocolError::Authentication)?;
    media::check_plaintext(parsed.header.packet_type, &plaintext)?;
    Ok((parsed.header, plaintext))
}

fn header_bytes(header: &MediaHeader) -> [u8; media::HEADER_LEN] {
    let mut out = [0u8; media::HEADER_LEN];
    out[0] = header.version;
    out[1] = header.packet_type;
    out[2..4].copy_from_slice(&header.flags.to_be_bytes());
    out[4..8].copy_from_slice(&header.stream_id.to_be_bytes());
    out[8..16].copy_from_slice(&header.sequence.to_be_bytes());
    out[16..24].copy_from_slice(&header.timestamp_us.to_be_bytes());
    out[24..26].copy_from_slice(&header.fragment_index.to_be_bytes());
    out[26..28].copy_from_slice(&header.fragment_count.to_be_bytes());
    out[28..32].copy_from_slice(&header.payload_len.to_be_bytes());
    out
}
