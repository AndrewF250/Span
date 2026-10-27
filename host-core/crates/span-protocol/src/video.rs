use crate::error::ProtocolError;

/// Bits per second of unpacked NV12 at the given geometry.
///
/// 1920×1080 at 30 fps is 746_496_000. That does not fit through Lightning
/// (USB 2) and is rejected by the session when the measured path is slower
/// than the returned rate. This function only does the arithmetic.
pub fn nv12_bitrate_bps(width: u32, height: u32, fps: u32) -> Result<u64, ProtocolError> {
    if width == 0 || height == 0 || fps == 0 || width % 2 != 0 || height % 2 != 0 {
        return Err(ProtocolError::InvalidField {
            field: "nv12 geometry",
        });
    }
    if width > crate::codes::MAX_EDGE
        || height > crate::codes::MAX_EDGE
        || fps > crate::codes::MAX_FPS
    {
        return Err(ProtocolError::InvalidField {
            field: "nv12 geometry",
        });
    }
    let frame = u128::from(width) * u128::from(height) * 3 / 2;
    let bytes = frame * u128::from(fps);
    let bits = bytes.saturating_mul(8);
    u64::try_from(bits).map_err(|_| ProtocolError::InvalidField {
        field: "nv12 geometry",
    })
}
