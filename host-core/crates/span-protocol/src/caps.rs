//! Capability bits carried in `Hello.capabilities`.
//!
//! Unknown bits are preserved. Bit 3 is reserved for an AirPlay mirror source
//! that this repository does not implement; [`local_advertisement`] clears it
//! so a Span build cannot advertise that source by accident.

pub const CAMERA: u64 = 1 << 0;
pub const MICROPHONE: u64 = 1 << 1;
pub const MIRROR_REPLAYKIT: u64 = 1 << 2;
pub const MIRROR_AIRPLAY: u64 = 1 << 3;
pub const MIRROR_FRAMEBUFFER: u64 = 1 << 4;
pub const EXTEND_DISPLAY: u64 = 1 << 5;
pub const INPUT_SELF: u64 = 1 << 6;
pub const INPUT_ACCESSIBILITY: u64 = 1 << 7;
pub const INPUT_HID_EVENT: u64 = 1 << 8;
pub const PANEL: u64 = 1 << 9;
pub const HAPTIC_IMPACT: u64 = 1 << 10;
pub const HAPTIC_VISUAL: u64 = 1 << 11;
pub const CODEC_H264: u64 = 1 << 12;
pub const CODEC_HEVC: u64 = 1 << 13;
pub const CODEC_RAW_NV12: u64 = 1 << 14;
pub const AUDIO_PCM: u64 = 1 << 15;
pub const AUDIO_OPUS: u64 = 1 << 16;
pub const TRANSPORT_UDP: u64 = 1 << 17;
pub const TRANSPORT_USB: u64 = 1 << 18;
pub const MODE_LOW_LATENCY: u64 = 1 << 19;
pub const MODE_BALANCED: u64 = 1 << 20;
pub const MODE_VISUALLY_LOSSLESS: u64 = 1 << 21;
pub const CURSOR_PLANE: u64 = 1 << 22;
pub const BACKGROUND_CAMERA: u64 = 1 << 23;

const NAMED: &[(u64, &str)] = &[
    (CAMERA, "camera"),
    (MICROPHONE, "microphone"),
    (MIRROR_REPLAYKIT, "mirror-replaykit"),
    (MIRROR_AIRPLAY, "mirror-airplay"),
    (MIRROR_FRAMEBUFFER, "mirror-framebuffer"),
    (EXTEND_DISPLAY, "extend-display"),
    (INPUT_SELF, "input-self"),
    (INPUT_ACCESSIBILITY, "input-accessibility"),
    (INPUT_HID_EVENT, "input-hidevent"),
    (PANEL, "panel"),
    (HAPTIC_IMPACT, "haptic-impact"),
    (HAPTIC_VISUAL, "haptic-visual"),
    (CODEC_H264, "codec-h264"),
    (CODEC_HEVC, "codec-hevc"),
    (CODEC_RAW_NV12, "codec-raw-nv12"),
    (AUDIO_PCM, "audio-pcm"),
    (AUDIO_OPUS, "audio-opus"),
    (TRANSPORT_UDP, "transport-udp"),
    (TRANSPORT_USB, "transport-usb"),
    (MODE_LOW_LATENCY, "mode-low-latency"),
    (MODE_BALANCED, "mode-balanced"),
    (MODE_VISUALLY_LOSSLESS, "mode-visually-lossless"),
    (CURSOR_PLANE, "cursor-plane"),
    (BACKGROUND_CAMERA, "background-camera"),
];

pub fn has(caps: u64, bit: u64) -> bool {
    caps & bit == bit
}

pub fn cap_name(bit: u64) -> Option<&'static str> {
    NAMED
        .iter()
        .find(|(value, _)| *value == bit)
        .map(|(_, name)| *name)
}

/// Clears capability bits this tree must not advertise.
pub fn local_advertisement(caps: u64) -> u64 {
    caps & !MIRROR_AIRPLAY
}

/// iOS 8 builds have no HEVC encoder, ReplayKit, or impact haptics.
pub fn legacy_forbidden() -> u64 {
    MIRROR_REPLAYKIT | CODEC_HEVC | HAPTIC_IMPACT | MIRROR_AIRPLAY
}

pub fn allows_legacy(caps: u64) -> bool {
    caps & legacy_forbidden() == 0
}
