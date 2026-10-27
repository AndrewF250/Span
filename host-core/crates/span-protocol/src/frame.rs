//! Length-prefixed control frames carried inside the TLS session.
//!
//! Unknown message types are returned, not rejected, so a newer minor version
//! can pass through an older peer. Semantic checks live in [`crate::validate`].

use prost::Message;

use crate::codes::{self, CONTROL_MAX_PAYLOAD};
use crate::error::ProtocolError;
use crate::media;
use crate::proto;

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedControl {
    pub flags: u16,
    pub message: ControlMessage,
    pub consumed: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ControlMessage {
    Hello(proto::Hello),
    HelloAck(proto::HelloAck),
    Reject(proto::Reject),
    Ping(proto::Ping),
    Pong(proto::Pong),
    ClockSync(proto::ClockSync),
    ClockSyncReply(proto::ClockSyncReply),
    StreamOpen(proto::StreamOpen),
    StreamOpenAck(proto::StreamOpenAck),
    StreamClose(proto::StreamClose),
    KeyframeRequest(proto::KeyframeRequest),
    Input(proto::InputEvent),
    Panel(proto::PanelLayout),
    Macro(proto::MacroInvoke),
    Capabilities(proto::CapabilitiesChanged),
    Disconnect(proto::Disconnect),
    KeyConfirm(proto::KeyConfirm),
    MediaPath(proto::MediaPath),
    Media(Vec<u8>),
    Unsupported { type_id: u16, payload: Vec<u8> },
}

impl ControlMessage {
    pub fn type_id(&self) -> u16 {
        match self {
            Self::Hello(_) => codes::MSG_HELLO,
            Self::HelloAck(_) => codes::MSG_HELLO_ACK,
            Self::Reject(_) => codes::MSG_REJECT,
            Self::Ping(_) => codes::MSG_PING,
            Self::Pong(_) => codes::MSG_PONG,
            Self::ClockSync(_) => codes::MSG_CLOCK_SYNC,
            Self::ClockSyncReply(_) => codes::MSG_CLOCK_SYNC_REPLY,
            Self::StreamOpen(_) => codes::MSG_STREAM_OPEN,
            Self::StreamOpenAck(_) => codes::MSG_STREAM_OPEN_ACK,
            Self::StreamClose(_) => codes::MSG_STREAM_CLOSE,
            Self::KeyframeRequest(_) => codes::MSG_KEYFRAME_REQUEST,
            Self::Input(_) => codes::MSG_INPUT,
            Self::Panel(_) => codes::MSG_PANEL,
            Self::Macro(_) => codes::MSG_MACRO,
            Self::Capabilities(_) => codes::MSG_CAPS,
            Self::Disconnect(_) => codes::MSG_DISCONNECT,
            Self::KeyConfirm(_) => codes::MSG_KEY_CONFIRM,
            Self::MediaPath(_) => codes::MSG_MEDIA_PATH,
            Self::Media(_) => codes::MSG_MEDIA,
            Self::Unsupported { type_id, .. } => *type_id,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        self.encode_with_flags(0)
    }

    pub fn encode_with_flags(&self, flags: u16) -> Result<Vec<u8>, ProtocolError> {
        encode_owned(self, flags)
    }
}

fn encode_owned(message: &ControlMessage, flags: u16) -> Result<Vec<u8>, ProtocolError> {
    let payload = match message {
        ControlMessage::Hello(message) => message.encode_to_vec(),
        ControlMessage::HelloAck(message) => message.encode_to_vec(),
        ControlMessage::Reject(message) => message.encode_to_vec(),
        ControlMessage::Ping(message) => message.encode_to_vec(),
        ControlMessage::Pong(message) => message.encode_to_vec(),
        ControlMessage::ClockSync(message) => message.encode_to_vec(),
        ControlMessage::ClockSyncReply(message) => message.encode_to_vec(),
        ControlMessage::StreamOpen(message) => message.encode_to_vec(),
        ControlMessage::StreamOpenAck(message) => message.encode_to_vec(),
        ControlMessage::StreamClose(message) => message.encode_to_vec(),
        ControlMessage::KeyframeRequest(message) => message.encode_to_vec(),
        ControlMessage::Input(message) => message.encode_to_vec(),
        ControlMessage::Panel(message) => message.encode_to_vec(),
        ControlMessage::Macro(message) => message.encode_to_vec(),
        ControlMessage::Capabilities(message) => message.encode_to_vec(),
        ControlMessage::Disconnect(message) => message.encode_to_vec(),
        ControlMessage::KeyConfirm(message) => message.encode_to_vec(),
        ControlMessage::MediaPath(message) => message.encode_to_vec(),
        ControlMessage::Media(bytes) | ControlMessage::Unsupported { payload: bytes, .. } => {
            bytes.clone()
        }
    };
    wrap(message.type_id(), flags, &payload)
}

pub fn peek_len(buf: &[u8]) -> Result<Option<usize>, ProtocolError> {
    if buf.len() < 4 {
        return Ok(None);
    }
    let declared = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if declared > CONTROL_MAX_PAYLOAD {
        return Err(ProtocolError::PayloadTooLarge {
            declared,
            cap: CONTROL_MAX_PAYLOAD,
        });
    }
    Ok(Some(8 + declared as usize))
}

pub fn decode(buf: &[u8]) -> Result<DecodedControl, ProtocolError> {
    let Some(total) = peek_len(buf)? else {
        return Err(ProtocolError::UnexpectedEof);
    };
    if buf.len() < total {
        return Err(ProtocolError::UnexpectedEof);
    }
    let type_id = u16::from_be_bytes([buf[4], buf[5]]);
    let flags = u16::from_be_bytes([buf[6], buf[7]]);
    let payload = &buf[8..total];
    let message = decode_payload(type_id, payload)?;
    Ok(DecodedControl {
        flags,
        message,
        consumed: total,
    })
}

fn decode_payload(type_id: u16, payload: &[u8]) -> Result<ControlMessage, ProtocolError> {
    Ok(match type_id {
        codes::MSG_HELLO => ControlMessage::Hello(decode_proto(payload)?),
        codes::MSG_HELLO_ACK => ControlMessage::HelloAck(decode_proto(payload)?),
        codes::MSG_REJECT => ControlMessage::Reject(decode_proto(payload)?),
        codes::MSG_PING => ControlMessage::Ping(decode_proto(payload)?),
        codes::MSG_PONG => ControlMessage::Pong(decode_proto(payload)?),
        codes::MSG_CLOCK_SYNC => ControlMessage::ClockSync(decode_proto(payload)?),
        codes::MSG_CLOCK_SYNC_REPLY => ControlMessage::ClockSyncReply(decode_proto(payload)?),
        codes::MSG_STREAM_OPEN => ControlMessage::StreamOpen(decode_proto(payload)?),
        codes::MSG_STREAM_OPEN_ACK => ControlMessage::StreamOpenAck(decode_proto(payload)?),
        codes::MSG_STREAM_CLOSE => ControlMessage::StreamClose(decode_proto(payload)?),
        codes::MSG_KEYFRAME_REQUEST => ControlMessage::KeyframeRequest(decode_proto(payload)?),
        codes::MSG_INPUT => ControlMessage::Input(decode_proto(payload)?),
        codes::MSG_PANEL => ControlMessage::Panel(decode_proto(payload)?),
        codes::MSG_MACRO => ControlMessage::Macro(decode_proto(payload)?),
        codes::MSG_CAPS => ControlMessage::Capabilities(decode_proto(payload)?),
        codes::MSG_DISCONNECT => ControlMessage::Disconnect(decode_proto(payload)?),
        codes::MSG_KEY_CONFIRM => ControlMessage::KeyConfirm(decode_proto(payload)?),
        codes::MSG_MEDIA_PATH => ControlMessage::MediaPath(decode_proto(payload)?),
        codes::MSG_MEDIA => {
            media::decode(payload)?;
            ControlMessage::Media(payload.to_vec())
        }
        other => ControlMessage::Unsupported {
            type_id: other,
            payload: payload.to_vec(),
        },
    })
}

fn decode_proto<M: Message + Default>(payload: &[u8]) -> Result<M, ProtocolError> {
    M::decode(payload).map_err(|_| ProtocolError::MalformedPayload)
}

fn wrap(type_id: u16, flags: u16, payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    let len = u32::try_from(payload.len()).map_err(|_| ProtocolError::PayloadTooLarge {
        declared: u32::MAX,
        cap: CONTROL_MAX_PAYLOAD,
    })?;
    if len > CONTROL_MAX_PAYLOAD {
        return Err(ProtocolError::PayloadTooLarge {
            declared: len,
            cap: CONTROL_MAX_PAYLOAD,
        });
    }
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&type_id.to_be_bytes());
    out.extend_from_slice(&flags.to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}
