//! Persistent host identity: a P-256 certificate and a stable 16-byte id.
//!
//! The certificate is what a device pins. The QR offer carries only its
//! SHA-256 fingerprint, never the private key.

use std::fs;
use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

use rcgen::{CertificateParams, KeyPair, SanType};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig;
use sha2::{Digest, Sha256};

use crate::random_bytes;

pub struct Identity {
    pub host_id: [u8; 16],
    pub certificate_der: Vec<u8>,
    pub private_key_der: Vec<u8>,
}

impl Identity {
    pub fn load_or_create(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let id_path = dir.join("host-id");
        let cert_path = dir.join("cert.der");
        let key_path = dir.join("key.der");
        if id_path.is_file() && cert_path.is_file() && key_path.is_file() {
            return Ok(Self {
                host_id: read_array(&id_path)?,
                certificate_der: fs::read(&cert_path)?,
                private_key_der: fs::read(&key_path)?,
            });
        }
        let created = Self::generate()?;
        fs::write(&id_path, created.host_id)?;
        fs::write(&cert_path, &created.certificate_der)?;
        fs::write(&key_path, &created.private_key_der)?;
        fs::write(
            dir.join("cert.pem"),
            pem("CERTIFICATE", &created.certificate_der),
        )?;
        Ok(created)
    }

    pub fn generate() -> io::Result<Self> {
        let mut params =
            CertificateParams::new(vec!["span-host.local".to_string(), "localhost".to_string()])
                .map_err(invalid)?;
        params
            .subject_alt_names
            .push(SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        let key = KeyPair::generate().map_err(invalid)?;
        let cert = params.self_signed(&key).map_err(invalid)?;
        Ok(Self {
            host_id: random_bytes(),
            certificate_der: cert.der().to_vec(),
            private_key_der: key.serialize_der(),
        })
    }

    pub fn fingerprint(&self) -> [u8; 32] {
        fingerprint(&self.certificate_der)
    }

    pub fn server_config(&self) -> io::Result<ServerConfig> {
        let provider = rustls::crypto::ring::default_provider();
        let cert = CertificateDer::from(self.certificate_der.clone());
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.private_key_der.clone()));
        ServerConfig::builder_with_provider(provider.into())
            .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])
            .map_err(invalid)?
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .map_err(invalid)
    }
}

pub fn fingerprint(der: &[u8]) -> [u8; 32] {
    Sha256::digest(der).into()
}

fn pem(label: &str, der: &[u8]) -> String {
    let encoded = base64(der);
    let mut body = String::new();
    for chunk in encoded.as_bytes().chunks(64) {
        body.push_str(std::str::from_utf8(chunk).expect("base64 is ascii"));
        body.push('\n');
    }
    format!("-----BEGIN {label}-----\n{body}-----END {label}-----\n")
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut index = 0;
    while index + 3 <= bytes.len() {
        let n = (u32::from(bytes[index]) << 16)
            | (u32::from(bytes[index + 1]) << 8)
            | u32::from(bytes[index + 2]);
        push_group(&mut out, n, 4);
        index += 3;
    }
    let remain = bytes.len() - index;
    if remain == 1 {
        let n = u32::from(bytes[index]) << 16;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if remain == 2 {
        let n = (u32::from(bytes[index]) << 16) | (u32::from(bytes[index + 1]) << 8);
        push_group(&mut out, n, 3);
        out.push('=');
    }
    out
}

fn push_group(out: &mut String, n: u32, count: usize) {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let shifts = [18, 12, 6, 0];
    for shift in shifts.into_iter().take(count) {
        out.push(TABLE[((n >> shift) & 63) as usize] as char);
    }
}

fn read_array(path: &Path) -> io::Result<[u8; 16]> {
    let bytes = fs::read(path)?;
    let array: [u8; 16] = bytes.try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "identity file has the wrong length",
        )
    })?;
    Ok(array)
}

fn invalid(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

pub fn default_data_dir() -> PathBuf {
    if let Ok(path) = std::env::var("SPAN_DATA_DIR") {
        return PathBuf::from(path);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/span");
    }
    PathBuf::from("/var/lib/span")
}
