//! One control-channel handshake over an already connected byte stream.
//!
//! TLS sits outside this module. The host speaks HelloAck and KeyConfirm;
//! the device speaks Hello, KeyConfirm, and MediaPath.

use std::io;
use std::time::Duration;

use span_protocol::codes::PROTOCOL_MAJOR;
use span_protocol::codes::{self, ROLE_DEVICE};
use span_protocol::{decode_control, validate_control, ControlMessage};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::identity::Identity;
use crate::random_bytes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Established {
    pub device_name: String,
    pub session_id: [u8; 16],
    pub media_key: [u8; 32],
}

pub async fn host_handshake<S>(mut stream: S, host_name: &str) -> io::Result<Established>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let hello = match read_message(&mut stream).await? {
        ControlMessage::Hello(hello) => hello,
        _ => {
            reject(&mut stream, codes::FAILURE_UNSUPPORTED, "expected hello").await?;
            return Err(proto_error("expected hello"));
        }
    };
    if let Err(error) = validate_control(&ControlMessage::Hello(hello.clone())) {
        let code = if hello.protocol_major != PROTOCOL_MAJOR {
            codes::FAILURE_VERSION
        } else {
            codes::FAILURE_UNSUPPORTED
        };
        reject(&mut stream, code, &error.to_string()).await?;
        return Err(proto_error(error));
    }
    if hello.role != ROLE_DEVICE {
        reject(&mut stream, codes::FAILURE_UNSUPPORTED, "role").await?;
        return Err(proto_error("peer is not a device"));
    }

    let session_id: [u8; 16] = random_bytes();
    let resume_token: [u8; 32] = random_bytes();
    let host_nonce: [u8; 32] = random_bytes();
    write_message(
        &mut stream,
        &ControlMessage::HelloAck(span_protocol::messages::HelloAck {
            protocol_major: PROTOCOL_MAJOR,
            protocol_minor: codes::PROTOCOL_MINOR,
            capabilities: host_capabilities(),
            session_id: session_id.to_vec(),
            resume_token: resume_token.to_vec(),
            udp_port: 0,
        }),
    )
    .await?;
    write_message(
        &mut stream,
        &ControlMessage::KeyConfirm(span_protocol::messages::KeyConfirm {
            nonce: host_nonce.to_vec(),
        }),
    )
    .await?;

    let mut device_nonce = None;
    let mut saw_path = false;
    for _ in 0..8 {
        if device_nonce.is_some() && saw_path {
            break;
        }
        match read_message(&mut stream).await? {
            ControlMessage::KeyConfirm(confirm) if confirm.nonce.len() == 32 => {
                let mut nonce = [0u8; 32];
                nonce.copy_from_slice(&confirm.nonce);
                device_nonce = Some(nonce);
            }
            ControlMessage::MediaPath(path) if path.udp_port <= u32::from(u16::MAX) => {
                saw_path = true;
            }
            ControlMessage::Ping(ping) => {
                write_message(
                    &mut stream,
                    &ControlMessage::Pong(span_protocol::messages::Pong {
                        cookie: ping.cookie,
                        sender_us: ping.sender_us,
                        echo_us: ping.sender_us,
                    }),
                )
                .await?;
            }
            _ => {
                reject(&mut stream, codes::FAILURE_UNSUPPORTED, "handshake").await?;
                return Err(proto_error("unexpected handshake message"));
            }
        }
    }
    let Some(device_nonce) = device_nonce else {
        return Err(proto_error("device nonce missing"));
    };
    if !saw_path {
        return Err(proto_error("media path missing"));
    }
    let media_key = span_protocol::derive_media_key(&host_nonce, &device_nonce, &session_id);
    Ok(Established {
        device_name: if hello.display_name.is_empty() {
            host_name.to_string()
        } else {
            hello.display_name
        },
        session_id,
        media_key,
    })
}

pub async fn device_handshake<S>(
    mut stream: S,
    identity_name: &str,
    device_id: [u8; 16],
) -> io::Result<Established>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    write_message(
        &mut stream,
        &ControlMessage::Hello(span_protocol::messages::Hello {
            protocol_major: PROTOCOL_MAJOR,
            protocol_minor: codes::PROTOCOL_MINOR,
            role: ROLE_DEVICE,
            device_id: device_id.to_vec(),
            display_name: identity_name.to_string(),
            capabilities: span_protocol::caps::CAMERA
                | span_protocol::caps::CODEC_H264
                | span_protocol::caps::AUDIO_PCM
                | span_protocol::caps::PANEL
                | span_protocol::caps::TRANSPORT_USB
                | span_protocol::caps::HAPTIC_VISUAL,
            max_video_width: 1920,
            max_video_height: 1080,
            max_fps: 30,
            codecs: vec![span_protocol::messages::CodecInfo {
                codec: codes::CODEC_H264,
                profile: 0,
            }],
            resume_token: Vec::new(),
        }),
    )
    .await?;

    let ack = match read_message(&mut stream).await? {
        ControlMessage::HelloAck(ack) => ack,
        ControlMessage::Reject(reject) => {
            return Err(proto_error(format!("rejected: {}", reject.detail)));
        }
        _ => return Err(proto_error("expected hello ack")),
    };
    validate_control(&ControlMessage::HelloAck(ack.clone())).map_err(proto_error)?;
    let session_id: [u8; 16] = ack
        .session_id
        .as_slice()
        .try_into()
        .map_err(|_| proto_error("session id"))?;
    let host_nonce = match read_message(&mut stream).await? {
        ControlMessage::KeyConfirm(confirm) if confirm.nonce.len() == 32 => {
            let mut nonce = [0u8; 32];
            nonce.copy_from_slice(&confirm.nonce);
            nonce
        }
        _ => return Err(proto_error("expected host key confirm")),
    };
    let device_nonce: [u8; 32] = random_bytes();
    write_message(
        &mut stream,
        &ControlMessage::KeyConfirm(span_protocol::messages::KeyConfirm {
            nonce: device_nonce.to_vec(),
        }),
    )
    .await?;
    write_message(
        &mut stream,
        &ControlMessage::MediaPath(span_protocol::messages::MediaPath { udp_port: 0 }),
    )
    .await?;
    Ok(Established {
        device_name: identity_name.to_string(),
        session_id,
        media_key: span_protocol::derive_media_key(&host_nonce, &device_nonce, &session_id),
    })
}

pub fn host_capabilities() -> u64 {
    span_protocol::caps::local_advertisement(
        span_protocol::caps::EXTEND_DISPLAY
            | span_protocol::caps::PANEL
            | span_protocol::caps::CODEC_H264
            | span_protocol::caps::AUDIO_PCM
            | span_protocol::caps::TRANSPORT_USB
            | span_protocol::caps::TRANSPORT_UDP
            | span_protocol::caps::MODE_LOW_LATENCY
            | span_protocol::caps::CURSOR_PLANE,
    )
}

pub fn offer(identity: &Identity, port: u16, name: &str) -> io::Result<String> {
    let mut nonce = [0u8; 16];
    nonce.copy_from_slice(&random_bytes::<32>()[..16]);
    span_protocol::PairingOffer {
        fingerprint: identity.fingerprint(),
        nonce,
        port,
        name: name.to_string(),
    }
    .encode()
    .map_err(proto_error)
}

async fn reject<S>(stream: &mut S, code: i32, detail: &str) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let detail = detail.chars().take(200).collect::<String>();
    write_message(
        stream,
        &ControlMessage::Reject(span_protocol::messages::Reject { code, detail }),
    )
    .await
}

async fn write_message<S>(stream: &mut S, message: &ControlMessage) -> io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    let bytes = message.encode().map_err(proto_error)?;
    stream.write_all(&bytes).await?;
    stream.flush().await
}

async fn read_message<S>(stream: &mut S) -> io::Result<ControlMessage>
where
    S: AsyncRead + Unpin,
{
    let mut prefix = [0u8; 4];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut prefix))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "handshake timed out"))??;
    let declared = u32::from_be_bytes(prefix);
    if declared > codes::CONTROL_MAX_PAYLOAD {
        return Err(proto_error("frame too large"));
    }
    let mut buf = vec![0u8; 8 + declared as usize];
    buf[..4].copy_from_slice(&prefix);
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut buf[4..]))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "handshake timed out"))??;
    let decoded = decode_control(&buf).map_err(proto_error)?;
    Ok(decoded.message)
}

fn proto_error(error: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

#[cfg(test)]
mod tests {
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn handshake_agrees_on_the_media_key() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            super::host_handshake(stream, "Span").await
        });
        let client = tokio::spawn(async move {
            let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            super::device_handshake(stream, "test-device", [9u8; 16]).await
        });
        let host = server.await.unwrap().unwrap();
        let device = client.await.unwrap().unwrap();
        assert_eq!(host.media_key, device.media_key);
        assert_eq!(host.session_id, device.session_id);
    }
}
