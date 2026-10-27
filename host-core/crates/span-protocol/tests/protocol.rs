use span_protocol::codes::{
    self, CODEC_H264, CODEC_PCM_S16LE, DIR_FROM_DEVICE, DIR_FROM_HOST, GESTURE_TAP, INPUT_KEY,
    KIND_CAMERA, KIND_EXTEND, PHASE_DOWN, ROLE_DEVICE,
};
use span_protocol::{
    caps, clock_sample, decode_control, decode_media, derive_media_key, nack_sequences,
    nv12_bitrate_bps, open_media, parse_preset, peek_control_len, seal_media, validate_control,
    CameraHeader, ControlMessage, MediaHeader, MicHeader, PairingOffer, ProtocolError,
    FLAG_KEYFRAME, PACKET_KEEPALIVE, PACKET_NACK, PACKET_VIDEO,
};

fn hello() -> span_protocol::messages::Hello {
    span_protocol::messages::Hello {
        protocol_major: codes::PROTOCOL_MAJOR,
        protocol_minor: 4,
        role: ROLE_DEVICE,
        device_id: vec![7; 16],
        display_name: "Камера".to_string(),
        capabilities: caps::CAMERA | caps::CODEC_H264 | caps::TRANSPORT_USB,
        max_video_width: 1920,
        max_video_height: 1080,
        max_fps: 60,
        codecs: vec![span_protocol::messages::CodecInfo {
            codec: CODEC_H264,
            profile: 0,
        }],
        resume_token: Vec::new(),
    }
}

#[test]
fn control_hello_roundtrip_preserves_unknown_flags() {
    let frame = ControlMessage::Hello(hello())
        .encode_with_flags(0x00ff)
        .unwrap();
    assert_eq!(peek_control_len(&frame[..2]).unwrap(), None);
    assert_eq!(peek_control_len(&frame).unwrap(), Some(frame.len()));
    let decoded = decode_control(&frame).unwrap();
    assert_eq!(decoded.flags, 0x00ff);
    assert_eq!(decoded.consumed, frame.len());
    validate_control(&decoded.message).unwrap();
    let ControlMessage::Hello(message) = decoded.message else {
        panic!("hello");
    };
    assert_eq!(message.display_name, "Камера");
    assert_eq!(message.protocol_minor, 4);
}

#[test]
fn unknown_control_type_is_skipped_by_the_decoder() {
    let mut frame = ControlMessage::Ping(span_protocol::messages::Ping {
        cookie: 9,
        sender_us: 1,
    })
    .encode()
    .unwrap();
    frame[4] = 0;
    frame[5] = 200;
    let decoded = decode_control(&frame).unwrap();
    assert!(matches!(
        decoded.message,
        ControlMessage::Unsupported { type_id: 200, .. }
    ));
    validate_control(&decoded.message).unwrap();
}

#[test]
fn oversized_control_length_is_rejected_before_allocation() {
    let err = peek_control_len(&1_048_577u32.to_be_bytes()).unwrap_err();
    assert!(matches!(err, ProtocolError::PayloadTooLarge { .. }));
}

#[test]
fn media_header_vector_matches_the_spec() {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../protocol/vectors/media_keyframe.bin"
    ))
    .unwrap();
    let packet = decode_media(&bytes).unwrap();
    assert_eq!(packet.header.packet_type, PACKET_VIDEO);
    assert_eq!(packet.header.flags, FLAG_KEYFRAME);
    assert_eq!(packet.header.stream_id, 7);
    assert_eq!(packet.header.sequence, 100);
    assert_eq!(packet.header.timestamp_us, 1_000_000);
    assert_eq!(packet.payload, &[0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(packet.header.encode(packet.payload).unwrap(), bytes);
}

#[test]
fn truncated_media_is_eof_and_trailing_byte_is_a_length_mismatch() {
    let header = sample_header(4);
    let mut bytes = header.encode(&[1, 2, 3, 4]).unwrap();
    assert!(matches!(
        decode_media(&bytes[..bytes.len() - 1]),
        Err(ProtocolError::UnexpectedEof)
    ));
    bytes.push(0);
    assert!(matches!(
        decode_media(&bytes),
        Err(ProtocolError::PayloadLengthMismatch { .. })
    ));
}

#[test]
fn seal_open_and_reject_a_tampered_datagram() {
    let key = [9u8; 32];
    let mut header = sample_header(0);
    header.packet_type = PACKET_VIDEO;
    let datagram = seal_media(&key, &header, &[1, 2, 3, 4]).unwrap();
    let (opened, plain) = open_media(&key, &datagram).unwrap();
    assert_eq!(plain, vec![1, 2, 3, 4]);
    assert_eq!(opened.stream_id, 7);
    let mut bad = datagram.clone();
    bad[40] ^= 1;
    assert_eq!(
        open_media(&key, &bad).unwrap_err(),
        ProtocolError::Authentication
    );
}

#[test]
fn nack_plaintext_lists_sequences() {
    let mut plain = Vec::new();
    plain.extend_from_slice(&20u64.to_be_bytes());
    plain.extend_from_slice(&21u64.to_be_bytes());
    let mut header = sample_header(0);
    header.packet_type = PACKET_NACK;
    let datagram = seal_media(&[4u8; 32], &header, &plain).unwrap();
    let (_, opened) = open_media(&[4u8; 32], &datagram).unwrap();
    assert_eq!(nack_sequences(&opened).unwrap(), vec![20, 21]);
}

#[test]
fn keepalive_seals_empty_plaintext() {
    let mut header = sample_header(0);
    header.packet_type = PACKET_KEEPALIVE;
    header.stream_id = 0;
    let datagram = seal_media(&[1u8; 32], &header, &[]).unwrap();
    let (_, plain) = open_media(&[1u8; 32], &datagram).unwrap();
    assert!(plain.is_empty());
}

#[test]
fn media_key_matches_the_published_vector() {
    let host = [1u8; 32];
    let device = [2u8; 32];
    let session: [u8; 16] = core::array::from_fn(|index| index as u8);
    let key = derive_media_key(&host, &device, &session);
    assert_eq!(
        hex(&key),
        "9a3927e1e872f2ba52c7450970b59985c7ded2813268aa2a8d016316c119271a"
    );
}

#[test]
fn clock_offset_truncates_toward_zero() {
    let sample = clock_sample(0, 0, 0, 101).unwrap();
    assert_eq!(sample.offset_us, -50);
    assert_eq!(sample.delay_us, 101);
    let sample = clock_sample(1000, 1200, 1300, 1600).unwrap();
    assert_eq!(sample.offset_us, -50);
    assert_eq!(sample.delay_us, 500);
    assert_eq!(
        clock_sample(1000, 5, 4, 2000).unwrap_err(),
        ProtocolError::InconsistentClock
    );
}

#[test]
fn nv12_1080p30_is_the_documented_rate() {
    assert_eq!(nv12_bitrate_bps(1920, 1080, 30).unwrap(), 746_496_000);
    assert!(nv12_bitrate_bps(1920, 1080, 0).is_err());
}

#[test]
fn pairing_offer_is_canonical_and_accepts_a_cyrillic_name() {
    let offer = PairingOffer {
        fingerprint: [0x11; 32],
        nonce: [0x22; 16],
        port: 47000,
        name: String::new(),
    };
    let text = offer.encode().unwrap();
    assert_eq!(
        text,
        "span://1/ceirceirceirceirceirceirceirceirceirceirceirceirceiq/eirceirceirceirceirceircei/47000/"
    );
    assert_eq!(
        PairingOffer::parse(&text.to_ascii_uppercase().replace("SPAN", "span"))
            .unwrap()
            .port,
        47000
    );
    let named = PairingOffer {
        name: "Зал".to_string(),
        ..offer
    };
    let encoded = named.encode().unwrap();
    assert!(encoded.contains('%'));
    assert_eq!(PairingOffer::parse(&encoded).unwrap().name, "Зал");
    assert!(PairingOffer::parse("span://1/nope").is_err());
}

#[test]
fn local_advertisement_clears_the_unshipped_airplay_bit() {
    let caps = caps::CAMERA | caps::MIRROR_AIRPLAY | caps::CODEC_HEVC;
    assert_eq!(
        caps::local_advertisement(caps),
        caps::CAMERA | caps::CODEC_HEVC
    );
    assert!(!caps::allows_legacy(caps));
    assert!(caps::allows_legacy(
        caps::CAMERA | caps::CODEC_H264 | caps::HAPTIC_VISUAL
    ));
    assert_eq!(caps::cap_name(caps::PANEL), Some("panel"));
}

#[test]
fn extend_stream_must_come_from_the_host() {
    let bad = span_protocol::messages::StreamOpen {
        stream_id: 3,
        direction: DIR_FROM_DEVICE,
        kind: KIND_EXTEND,
        video: Some(video()),
        audio: None,
    };
    let err = validate_control(&ControlMessage::StreamOpen(bad)).unwrap_err();
    assert_eq!(err, ProtocolError::InvalidField { field: "direction" });

    let good = span_protocol::messages::StreamOpen {
        stream_id: 3,
        direction: DIR_FROM_HOST,
        kind: KIND_EXTEND,
        video: Some(video()),
        audio: None,
    };
    validate_control(&ControlMessage::StreamOpen(good)).unwrap();
}

#[test]
fn camera_stream_rejects_hevc_on_the_audio_slot() {
    let open = span_protocol::messages::StreamOpen {
        stream_id: 1,
        direction: DIR_FROM_DEVICE,
        kind: KIND_CAMERA,
        video: Some(span_protocol::messages::VideoParams {
            codec: CODEC_PCM_S16LE,
            ..video()
        }),
        audio: None,
    };
    assert!(validate_control(&ControlMessage::StreamOpen(open)).is_err());
}

#[test]
fn input_key_uses_hid_usages() {
    let event = span_protocol::messages::InputEvent {
        event_us: 10,
        kind: INPUT_KEY,
        key: Some(span_protocol::messages::KeyEvent {
            usage_page: 0x07,
            usage: 0x04,
            pressed: true,
        }),
        pointer: None,
        axis: None,
    };
    validate_control(&ControlMessage::Input(event)).unwrap();
}

#[test]
fn pointer_outside_the_surface_is_rejected() {
    let event = span_protocol::messages::InputEvent {
        event_us: 10,
        kind: span_protocol::codes::INPUT_POINTER,
        key: None,
        pointer: Some(span_protocol::messages::PointerEvent {
            pointer_id: 1,
            phase: PHASE_DOWN,
            x_norm: 1.2,
            y_norm: 0.0,
            wheel: 0.0,
            surface_id: 0,
        }),
        axis: None,
    };
    assert!(validate_control(&ControlMessage::Input(event)).is_err());
}

#[test]
fn preset_example_parses_and_overlap_fails() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../protocol/examples/browser.json"
    );
    let bytes = std::fs::read(path).unwrap();
    let preset = parse_preset(&bytes).unwrap();
    assert_eq!(preset.id, "example.browser");
    let mut overlap = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap();
    overlap["controls"][1]["col"] = serde_json::json!(0);
    let encoded = serde_json::to_vec(&overlap).unwrap();
    assert_eq!(
        parse_preset(&encoded).unwrap_err(),
        ProtocolError::BadPreset { reason: "overlap" }
    );
}

#[test]
fn macro_invoke_roundtrips() {
    let message = ControlMessage::Macro(span_protocol::messages::MacroInvoke {
        preset_id: "example.browser".to_string(),
        control_id: "back".to_string(),
        gesture: GESTURE_TAP,
    });
    let frame = message.encode().unwrap();
    let decoded = decode_control(&frame).unwrap();
    validate_control(&decoded.message).unwrap();
}

#[test]
fn shared_memory_headers_roundtrip() {
    let camera = CameraHeader {
        width: 64,
        height: 64,
        stride: 64,
        fps_num: 30,
        fps_den: 1,
        slot_count: 3,
        slot_stride: 6144,
        active_index: 2,
        last_timestamp_us: 99,
    };
    let mut bytes = vec![0; 64];
    camera.write_to(&mut bytes).unwrap();
    assert_eq!(CameraHeader::read_from(&bytes).unwrap(), camera);
    assert_eq!(camera.slot_offset(2).unwrap(), 64 + 2 * 6144);

    let mic = MicHeader {
        sample_rate: 44_100,
        channels: 2,
        capacity_frames: 64,
        write_frame: 10,
    };
    let mut bytes = vec![0; 48];
    mic.write_to(&mut bytes).unwrap();
    assert_eq!(MicHeader::read_from(&bytes).unwrap(), mic);
    assert_eq!(mic.frame_byte_offset(10).unwrap(), 48 + 40);
}

#[test]
fn parsers_do_not_panic_on_random_bytes() {
    span_protocol::exercise_random(0x5a1e, 300);
}

#[test]
fn usb_media_envelope_requires_a_real_datagram() {
    let header = sample_header(2);
    let datagram = header.encode(&[9, 8]).unwrap();
    let frame = ControlMessage::Media(datagram.clone()).encode().unwrap();
    let decoded = decode_control(&frame).unwrap();
    let ControlMessage::Media(bytes) = decoded.message else {
        panic!("media");
    };
    assert_eq!(bytes, datagram);
    let bad = ControlMessage::Media(vec![1, 2, 3]).encode().unwrap();
    assert!(decode_control(&bad).is_err());
}

#[test]
fn key_codes_are_sorted() {
    let keys = span_protocol::key_codes();
    let mut sorted = keys.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, keys);
}

fn sample_header(payload_len: u32) -> MediaHeader {
    MediaHeader {
        version: 1,
        packet_type: PACKET_VIDEO,
        flags: FLAG_KEYFRAME,
        stream_id: 7,
        sequence: 100,
        timestamp_us: 1_000_000,
        fragment_index: 0,
        fragment_count: 1,
        payload_len,
    }
}

fn video() -> span_protocol::messages::VideoParams {
    span_protocol::messages::VideoParams {
        codec: CODEC_H264,
        width: 1920,
        height: 1080,
        fps: 30,
        bitrate_bps: 8_000_000,
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}
