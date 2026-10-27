//! QR offer shown by the host and scanned by the device.
//!
//! The offer carries a certificate fingerprint and a one-time nonce. It does
//! not carry a private key. The TLS stack still has to pin the certificate
//! whose SHA-256 matches `fingerprint`.

use data_encoding::BASE32_NOPAD;

use crate::error::ProtocolError;

const PREFIX: &str = "span://1/";
const MAX_TEXT: usize = 220;
const MAX_NAME: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingOffer {
    pub fingerprint: [u8; 32],
    pub nonce: [u8; 16],
    pub port: u16,
    pub name: String,
}

impl PairingOffer {
    pub fn encode(&self) -> Result<String, ProtocolError> {
        if self.port == 0 {
            return Err(ProtocolError::BadPairing);
        }
        check_name(&self.name)?;
        let text = format!(
            "{PREFIX}{}/{}/{}/{}",
            base32(&self.fingerprint),
            base32(&self.nonce),
            self.port,
            percent_encode(self.name.as_bytes())
        );
        if text.len() > MAX_TEXT {
            return Err(ProtocolError::BadPairing);
        }
        Ok(text)
    }

    pub fn parse(text: &str) -> Result<Self, ProtocolError> {
        if text.len() > MAX_TEXT || !text.starts_with(PREFIX) {
            return Err(ProtocolError::BadPairing);
        }
        let rest = &text[PREFIX.len()..];
        let mut parts = rest.split('/');
        let fingerprint = decode_fixed::<32>(parts.next().ok_or(ProtocolError::BadPairing)?)?;
        let nonce = decode_fixed::<16>(parts.next().ok_or(ProtocolError::BadPairing)?)?;
        let port = parse_port(parts.next().ok_or(ProtocolError::BadPairing)?)?;
        let name_raw = parts.next().ok_or(ProtocolError::BadPairing)?;
        if parts.next().is_some() {
            return Err(ProtocolError::BadPairing);
        }
        let name_bytes = percent_decode(name_raw)?;
        if name_bytes.len() > MAX_NAME {
            return Err(ProtocolError::BadPairing);
        }
        let name = String::from_utf8(name_bytes).map_err(|_| ProtocolError::BadPairing)?;
        check_name(&name)?;
        Ok(Self {
            fingerprint,
            nonce,
            port,
            name,
        })
    }
}

fn check_name(name: &str) -> Result<(), ProtocolError> {
    if name.len() > MAX_NAME || name.chars().any(|ch| ch.is_control()) {
        return Err(ProtocolError::BadPairing);
    }
    Ok(())
}

fn base32(bytes: &[u8]) -> String {
    BASE32_NOPAD.encode(bytes).to_ascii_lowercase()
}

fn decode_fixed<const N: usize>(text: &str) -> Result<[u8; N], ProtocolError> {
    let decoded = BASE32_NOPAD
        .decode(text.to_ascii_uppercase().as_bytes())
        .map_err(|_| ProtocolError::BadPairing)?;
    let mut out = [0u8; N];
    if decoded.len() != N {
        return Err(ProtocolError::BadPairing);
    }
    out.copy_from_slice(&decoded);
    Ok(out)
}

fn parse_port(text: &str) -> Result<u16, ProtocolError> {
    if text.is_empty()
        || text.len() > 5
        || text.starts_with('0')
        || !text.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(ProtocolError::BadPairing);
    }
    text.parse::<u16>().map_err(|_| ProtocolError::BadPairing)
}

fn percent_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for byte in bytes {
        if is_unreserved(*byte) {
            out.push(*byte as char);
        } else {
            out.push('%');
            out.push(hex(byte >> 4));
            out.push(hex(byte & 0x0f));
        }
    }
    out
}

fn percent_decode(text: &str) -> Result<Vec<u8>, ProtocolError> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(ProtocolError::BadPairing);
            }
            let hi = unhex(bytes[index + 1])?;
            let lo = unhex(bytes[index + 2])?;
            out.push((hi << 4) | lo);
            index += 3;
        } else if is_unreserved(bytes[index]) {
            out.push(bytes[index]);
            index += 1;
        } else {
            return Err(ProtocolError::BadPairing);
        }
    }
    Ok(out)
}

fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

fn hex(nibble: u8) -> char {
    char::from(b"0123456789ABCDEF"[nibble as usize])
}

fn unhex(byte: u8) -> Result<u8, ProtocolError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ProtocolError::BadPairing),
    }
}
