use crate::frame;
use crate::media;
use crate::pairing::PairingOffer;
use crate::preset;
use crate::shmem::{CameraHeader, MicHeader};

/// Runs every byte-oriented parser. A panic here is a codec bug.
pub fn exercise_parsers(data: &[u8]) {
    let _ = media::decode(data);
    let _ = frame::decode(data);
    let _ = frame::peek_len(data);
    let _ = CameraHeader::read_from(data);
    let _ = MicHeader::read_from(data);
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = PairingOffer::parse(text);
    }
    let _ = preset::parse(data);
}

pub fn exercise_random(seed: u64, iters: u32) {
    let mut state = seed;
    for _ in 0..iters {
        state = step(state);
        let len = ((state >> 33) as usize) % 2048;
        let mut buf = vec![0u8; len];
        for byte in &mut buf {
            state = step(state);
            *byte = (state >> 33) as u8;
        }
        exercise_parsers(&buf);
    }
}

fn step(state: u64) -> u64 {
    state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1)
}
