//! Stable numeric identifiers shared with `protocol/messages.proto`.

pub const PROTOCOL_MAJOR: u32 = 1;
pub const PROTOCOL_MINOR: u32 = 0;

pub const MSG_HELLO: u16 = 1;
pub const MSG_HELLO_ACK: u16 = 2;
pub const MSG_REJECT: u16 = 3;
pub const MSG_PING: u16 = 4;
pub const MSG_PONG: u16 = 5;
pub const MSG_CLOCK_SYNC: u16 = 6;
pub const MSG_CLOCK_SYNC_REPLY: u16 = 7;
pub const MSG_STREAM_OPEN: u16 = 8;
pub const MSG_STREAM_OPEN_ACK: u16 = 9;
pub const MSG_STREAM_CLOSE: u16 = 10;
pub const MSG_KEYFRAME_REQUEST: u16 = 11;
pub const MSG_INPUT: u16 = 12;
pub const MSG_PANEL: u16 = 13;
pub const MSG_MACRO: u16 = 14;
pub const MSG_CAPS: u16 = 15;
pub const MSG_DISCONNECT: u16 = 16;
pub const MSG_KEY_CONFIRM: u16 = 17;
pub const MSG_MEDIA_PATH: u16 = 18;
pub const MSG_MEDIA: u16 = 100;

pub const ROLE_HOST: i32 = 1;
pub const ROLE_DEVICE: i32 = 2;

pub const CODEC_H264: i32 = 1;
pub const CODEC_HEVC: i32 = 2;
pub const CODEC_RAW_NV12: i32 = 3;
pub const CODEC_PCM_S16LE: i32 = 4;
pub const CODEC_OPUS: i32 = 5;

pub const FAILURE_VERSION: i32 = 1;
pub const FAILURE_PIN_MISMATCH: i32 = 2;
pub const FAILURE_BUSY: i32 = 3;
pub const FAILURE_UNSUPPORTED: i32 = 4;
pub const FAILURE_UNKNOWN_CONTROL: i32 = 5;
pub const FAILURE_BAD_STREAM: i32 = 6;
pub const FAILURE_NOT_ACCEPTED: i32 = 7;

pub const DIR_FROM_DEVICE: i32 = 1;
pub const DIR_FROM_HOST: i32 = 2;

pub const KIND_CAMERA: i32 = 1;
pub const KIND_MICROPHONE: i32 = 2;
pub const KIND_MIRROR: i32 = 3;
pub const KIND_EXTEND: i32 = 4;
pub const KIND_DESKTOP_AUDIO: i32 = 5;

pub const INPUT_KEY: i32 = 1;
pub const INPUT_POINTER: i32 = 2;
pub const INPUT_AXIS: i32 = 3;

pub const PHASE_DOWN: i32 = 1;
pub const PHASE_MOVE: i32 = 2;
pub const PHASE_UP: i32 = 3;
pub const PHASE_CANCEL: i32 = 4;
pub const PHASE_WHEEL: i32 = 5;

pub const GESTURE_TAP: i32 = 1;
pub const GESTURE_DOUBLE_TAP: i32 = 2;
pub const GESTURE_LONG_PRESS: i32 = 3;
pub const GESTURE_SWIPE_UP: i32 = 4;
pub const GESTURE_SWIPE_DOWN: i32 = 5;
pub const GESTURE_SWIPE_LEFT: i32 = 6;
pub const GESTURE_SWIPE_RIGHT: i32 = 7;

pub const CLOSE_USER: i32 = 1;
pub const CLOSE_REPLACED: i32 = 2;
pub const CLOSE_ERROR: i32 = 3;

pub const BYE_USER: i32 = 1;
pub const BYE_REPLACED: i32 = 2;
pub const BYE_ERROR: i32 = 3;

pub const CONTROL_MAX_PAYLOAD: u32 = 1_048_576;
pub const DEVICE_ID_LEN: usize = 16;
pub const SESSION_ID_LEN: usize = 16;
pub const RESUME_TOKEN_LEN: usize = 32;
pub const KEY_NONCE_LEN: usize = 32;
pub const MAX_DISPLAY_NAME: usize = 128;
pub const MAX_DETAIL: usize = 256;
pub const MAX_CODECS: usize = 8;
pub const MAX_EDGE: u32 = 8192;
pub const MAX_FPS: u32 = 240;
pub const MAX_BITRATE_BPS: u32 = 200_000_000;
pub const MAX_PRESET_BYTES: usize = 256 * 1024;
