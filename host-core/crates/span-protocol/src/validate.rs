use crate::codes::{
    self, CODEC_H264, CODEC_HEVC, CODEC_OPUS, CODEC_PCM_S16LE, CODEC_RAW_NV12, DIR_FROM_DEVICE,
    DIR_FROM_HOST, INPUT_AXIS, INPUT_KEY, INPUT_POINTER, KIND_CAMERA, KIND_DESKTOP_AUDIO,
    KIND_EXTEND, KIND_MICROPHONE, KIND_MIRROR,
};
use crate::error::ProtocolError;
use crate::frame::ControlMessage;
use crate::preset::{self, control_id_ok, preset_id_ok};
use crate::proto;

pub fn control(message: &ControlMessage) -> Result<(), ProtocolError> {
    match message {
        ControlMessage::Hello(hello) => hello_ok(hello),
        ControlMessage::HelloAck(ack) => hello_ack_ok(ack),
        ControlMessage::Reject(reject) => {
            require_set::<proto::Failure>(reject.code, "reject")?;
            plain_text(&reject.detail, codes::MAX_DETAIL, "detail")
        }
        ControlMessage::Ping(_) | ControlMessage::Pong(_) | ControlMessage::ClockSync(_) => Ok(()),
        ControlMessage::ClockSyncReply(reply) => {
            if reply.t2_receiver_us < reply.t1_receiver_us {
                Err(ProtocolError::InconsistentClock)
            } else {
                Ok(())
            }
        }
        ControlMessage::StreamOpen(open) => stream_open_ok(open),
        ControlMessage::StreamOpenAck(ack) => stream_ack_ok(ack),
        ControlMessage::StreamClose(close) => {
            stream_id(close.stream_id)?;
            require_known::<proto::Close>(close.reason, "close")
        }
        ControlMessage::KeyframeRequest(request) => stream_id(request.stream_id),
        ControlMessage::Input(event) => input_ok(event),
        ControlMessage::Panel(panel) => preset::parse(&panel.document_json).map(|_| ()),
        ControlMessage::Macro(invoke) => {
            if !preset_id_ok(&invoke.preset_id) || !control_id_ok(&invoke.control_id) {
                return Err(ProtocolError::InvalidField { field: "macro" });
            }
            require_set::<proto::Gesture>(invoke.gesture, "gesture")
        }
        ControlMessage::Capabilities(_)
        | ControlMessage::Unsupported { .. }
        | ControlMessage::Media(_) => Ok(()),
        ControlMessage::Disconnect(bye) => require_known::<proto::Bye>(bye.reason, "bye"),
        ControlMessage::KeyConfirm(confirm) => {
            exact_len(&confirm.nonce, codes::KEY_NONCE_LEN, "nonce")
        }
        ControlMessage::MediaPath(path) => udp_port(path.udp_port),
    }
}

fn hello_ok(hello: &proto::Hello) -> Result<(), ProtocolError> {
    if hello.protocol_major != codes::PROTOCOL_MAJOR {
        return Err(ProtocolError::UnsupportedVersion {
            found: hello.protocol_major,
        });
    }
    require_set::<proto::Role>(hello.role, "role")?;
    exact_len(&hello.device_id, codes::DEVICE_ID_LEN, "device_id")?;
    plain_text(&hello.display_name, codes::MAX_DISPLAY_NAME, "display_name")?;
    geometry_hint(hello.max_video_width, hello.max_video_height, hello.max_fps)?;
    if hello.codecs.len() > codes::MAX_CODECS {
        return Err(ProtocolError::InvalidField { field: "codecs" });
    }
    let mut seen = Vec::new();
    for codec in &hello.codecs {
        require_set::<proto::Codec>(codec.codec, "codec")?;
        if seen.contains(&codec.codec) {
            return Err(ProtocolError::InvalidField { field: "codecs" });
        }
        seen.push(codec.codec);
    }
    if !hello.resume_token.is_empty() {
        exact_len(&hello.resume_token, codes::RESUME_TOKEN_LEN, "resume_token")?;
    }
    Ok(())
}

fn hello_ack_ok(ack: &proto::HelloAck) -> Result<(), ProtocolError> {
    if ack.protocol_major != codes::PROTOCOL_MAJOR {
        return Err(ProtocolError::UnsupportedVersion {
            found: ack.protocol_major,
        });
    }
    exact_len(&ack.session_id, codes::SESSION_ID_LEN, "session_id")?;
    exact_len(&ack.resume_token, codes::RESUME_TOKEN_LEN, "resume_token")?;
    udp_port(ack.udp_port)
}

fn stream_open_ok(open: &proto::StreamOpen) -> Result<(), ProtocolError> {
    stream_id(open.stream_id)?;
    require_set::<proto::StreamDir>(open.direction, "direction")?;
    require_set::<proto::StreamKind>(open.kind, "kind")?;
    let expected = match open.kind {
        KIND_CAMERA | KIND_MICROPHONE | KIND_MIRROR => DIR_FROM_DEVICE,
        KIND_EXTEND | KIND_DESKTOP_AUDIO => DIR_FROM_HOST,
        _ => return Err(ProtocolError::InvalidField { field: "kind" }),
    };
    if open.direction != expected {
        return Err(ProtocolError::InvalidField { field: "direction" });
    }
    let video_kind = matches!(open.kind, KIND_CAMERA | KIND_MIRROR | KIND_EXTEND);
    if video_kind {
        let video = open
            .video
            .as_ref()
            .ok_or(ProtocolError::InvalidField { field: "video" })?;
        video_ok(video)?;
        if open.audio.is_some() {
            return Err(ProtocolError::InvalidField { field: "audio" });
        }
    } else {
        let audio = open
            .audio
            .as_ref()
            .ok_or(ProtocolError::InvalidField { field: "audio" })?;
        audio_ok(audio)?;
        if open.video.is_some() {
            return Err(ProtocolError::InvalidField { field: "video" });
        }
    }
    Ok(())
}

fn stream_ack_ok(ack: &proto::StreamOpenAck) -> Result<(), ProtocolError> {
    stream_id(ack.stream_id)?;
    match (ack.video.as_ref(), ack.audio.as_ref()) {
        (Some(video), None) => video_ok(video),
        (None, Some(audio)) => audio_ok(audio),
        _ => Err(ProtocolError::InvalidField {
            field: "stream ack",
        }),
    }
}

fn video_ok(video: &proto::VideoParams) -> Result<(), ProtocolError> {
    if !matches!(video.codec, CODEC_H264 | CODEC_HEVC | CODEC_RAW_NV12) {
        return Err(ProtocolError::InvalidField {
            field: "video codec",
        });
    }
    if video.width == 0
        || video.height == 0
        || video.width > codes::MAX_EDGE
        || video.height > codes::MAX_EDGE
        || video.width % 2 != 0
        || video.height % 2 != 0
        || video.fps == 0
        || video.fps > codes::MAX_FPS
    {
        return Err(ProtocolError::InvalidField {
            field: "video geometry",
        });
    }
    if video.bitrate_bps > codes::MAX_BITRATE_BPS {
        return Err(ProtocolError::InvalidField { field: "bitrate" });
    }
    Ok(())
}

fn audio_ok(audio: &proto::AudioParams) -> Result<(), ProtocolError> {
    if !matches!(audio.codec, CODEC_PCM_S16LE | CODEC_OPUS) {
        return Err(ProtocolError::InvalidField {
            field: "audio codec",
        });
    }
    if !(8_000..=192_000).contains(&audio.sample_rate) || !(1..=2).contains(&audio.channels) {
        return Err(ProtocolError::InvalidField {
            field: "audio format",
        });
    }
    Ok(())
}

fn input_ok(event: &proto::InputEvent) -> Result<(), ProtocolError> {
    match event.kind {
        INPUT_KEY => {
            let key = event
                .key
                .as_ref()
                .ok_or(ProtocolError::InvalidField { field: "key" })?;
            if event.pointer.is_some() || event.axis.is_some() {
                return Err(ProtocolError::InvalidField { field: "input" });
            }
            if key.usage_page == 0
                || key.usage == 0
                || key.usage_page > 0xffff
                || key.usage > 0xffff
            {
                return Err(ProtocolError::InvalidField { field: "hid usage" });
            }
            Ok(())
        }
        INPUT_POINTER => {
            let pointer = event
                .pointer
                .as_ref()
                .ok_or(ProtocolError::InvalidField { field: "pointer" })?;
            if event.key.is_some() || event.axis.is_some() {
                return Err(ProtocolError::InvalidField { field: "input" });
            }
            require_set::<proto::PointerPhase>(pointer.phase, "phase")?;
            if !unit(pointer.x_norm) || !unit(pointer.y_norm) || !pointer.wheel.is_finite() {
                return Err(ProtocolError::InvalidField { field: "pointer" });
            }
            Ok(())
        }
        INPUT_AXIS => {
            let axis = event
                .axis
                .as_ref()
                .ok_or(ProtocolError::InvalidField { field: "axis" })?;
            if event.key.is_some() || event.pointer.is_some() {
                return Err(ProtocolError::InvalidField { field: "input" });
            }
            if !control_id_ok(&axis.control_id) || !signed_unit(axis.x) || !signed_unit(axis.y) {
                return Err(ProtocolError::InvalidField { field: "axis" });
            }
            Ok(())
        }
        _ => Err(ProtocolError::InvalidField {
            field: "input kind",
        }),
    }
}

fn geometry_hint(width: u32, height: u32, fps: u32) -> Result<(), ProtocolError> {
    if width == 0 && height == 0 && fps == 0 {
        return Ok(());
    }
    if width == 0
        || height == 0
        || width > codes::MAX_EDGE
        || height > codes::MAX_EDGE
        || width % 2 != 0
        || height % 2 != 0
        || fps > codes::MAX_FPS
    {
        return Err(ProtocolError::InvalidField {
            field: "video hint",
        });
    }
    Ok(())
}

fn stream_id(id: u32) -> Result<(), ProtocolError> {
    if id == 0 {
        Err(ProtocolError::InvalidField { field: "stream_id" })
    } else {
        Ok(())
    }
}

fn udp_port(port: u32) -> Result<(), ProtocolError> {
    if port > u32::from(u16::MAX) {
        Err(ProtocolError::InvalidField { field: "udp_port" })
    } else {
        Ok(())
    }
}

fn exact_len(bytes: &[u8], expected: usize, field: &'static str) -> Result<(), ProtocolError> {
    if bytes.len() == expected {
        Ok(())
    } else {
        Err(ProtocolError::InvalidField { field })
    }
}

fn plain_text(text: &str, max: usize, field: &'static str) -> Result<(), ProtocolError> {
    if text.len() <= max && !text.chars().any(|ch| ch.is_control()) {
        Ok(())
    } else {
        Err(ProtocolError::InvalidField { field })
    }
}

fn require_set<E: TryFrom<i32>>(value: i32, field: &'static str) -> Result<(), ProtocolError> {
    if value == 0 || E::try_from(value).is_err() {
        Err(ProtocolError::InvalidField { field })
    } else {
        Ok(())
    }
}

fn require_known<E: TryFrom<i32>>(value: i32, field: &'static str) -> Result<(), ProtocolError> {
    if E::try_from(value).is_ok() {
        Ok(())
    } else {
        Err(ProtocolError::InvalidField { field })
    }
}

fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn signed_unit(value: f32) -> bool {
    value.is_finite() && (-1.0..=1.0).contains(&value)
}
