//! Span protocol major 1.
//!
//! The normative text is `protocol/spec.md`. This crate is the host-side
//! codec: control frames, media datagrams, the pairing offer, preset
//! documents, and the shared-memory headers the drivers will map later.
//! It does not open sockets.

#![forbid(unsafe_code)]

pub mod caps;
pub mod codes;

mod clock;
mod crypto;
mod error;
mod frame;
mod fuzz;
mod media;
mod pairing;
mod preset;
mod read;
mod shmem;
mod validate;
mod video;

#[allow(clippy::all)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/span.rs"));
}

pub use clock::{clock_sample, ClockSample};
pub use crypto::{derive_media_key, open as open_media, seal as seal_media};
pub use error::ProtocolError;
pub use frame::{
    decode as decode_control, peek_len as peek_control_len, ControlMessage, DecodedControl,
};
pub use fuzz::{exercise_parsers, exercise_random};
pub use media::{
    decode as decode_media, nack_sequences, MediaHeader, MediaPacket, FLAG_CONFIG,
    FLAG_DISCONTINUITY, FLAG_END_OF_FRAME, FLAG_KEYFRAME, HEADER_LEN, MAX_DATAGRAM, MAX_PAYLOAD,
    MAX_PLAINTEXT, PACKET_AUDIO, PACKET_KEEPALIVE, PACKET_NACK, PACKET_VIDEO, TAG_LEN,
};
pub use pairing::PairingOffer;
pub use preset::{
    key_codes, parse as parse_preset, Action, AnalogOutput, Control, Gestures, Locale,
    MediaCommand, PresetDocument,
};
pub use proto as messages;
pub use shmem::{
    CameraHeader, MicHeader, CAMERA_HEADER_LEN, FOURCC_NV12, MIC_HEADER_LEN, PCM_S16LE,
};
pub use validate::control as validate_control;
pub use video::nv12_bitrate_bps;
