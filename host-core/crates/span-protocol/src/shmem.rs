//! Shared-memory headers read by the future Windows AVStream drivers and by
//! the Linux v4l2loopback / PulseAudio feeders.
//!
//! Layout is little-endian and packed at a fixed size. The process that writes
//! frames publishes the active slot or the write counter only after the bytes
//! of that frame are visible. This module checks the header; it does not map a
//! file.

use crate::error::ProtocolError;

pub const CAMERA_MAGIC: &[u8; 8] = b"SPANCAM1";
pub const MIC_MAGIC: &[u8; 8] = b"SPANMIC1";
pub const CAMERA_HEADER_LEN: usize = 64;
pub const MIC_HEADER_LEN: usize = 48;
pub const FOURCC_NV12: u32 = u32::from_le_bytes(*b"NV12");
pub const PCM_S16LE: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraHeader {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    pub slot_count: u32,
    pub slot_stride: u32,
    pub active_index: u32,
    pub last_timestamp_us: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MicHeader {
    pub sample_rate: u32,
    pub channels: u16,
    pub capacity_frames: u32,
    pub write_frame: u64,
}

impl CameraHeader {
    pub fn write_to(&self, dst: &mut [u8]) -> Result<(), ProtocolError> {
        if dst.len() < CAMERA_HEADER_LEN {
            return Err(ProtocolError::UnexpectedEof);
        }
        self.check()?;
        dst[..CAMERA_HEADER_LEN].fill(0);
        dst[..8].copy_from_slice(CAMERA_MAGIC);
        put_u32(dst, 8, 1);
        put_u32(dst, 12, self.width);
        put_u32(dst, 16, self.height);
        put_u32(dst, 20, FOURCC_NV12);
        put_u32(dst, 24, self.stride);
        put_u32(dst, 28, self.fps_num);
        put_u32(dst, 32, self.fps_den);
        put_u32(dst, 36, self.slot_count);
        put_u32(dst, 40, self.slot_stride);
        put_u32(dst, 44, self.active_index);
        put_u64(dst, 48, self.last_timestamp_us);
        Ok(())
    }

    pub fn read_from(src: &[u8]) -> Result<Self, ProtocolError> {
        if src.len() < CAMERA_HEADER_LEN {
            return Err(ProtocolError::UnexpectedEof);
        }
        if &src[..8] != CAMERA_MAGIC {
            return Err(ProtocolError::InvalidField {
                field: "camera magic",
            });
        }
        let version = get_u32(src, 8);
        if version != 1 {
            return Err(ProtocolError::UnsupportedVersion { found: version });
        }
        if get_u32(src, 20) != FOURCC_NV12 {
            return Err(ProtocolError::InvalidField { field: "fourcc" });
        }
        if get_u32(src, 56) != 0 || get_u32(src, 60) != 0 {
            return Err(ProtocolError::InvalidField {
                field: "camera reserved",
            });
        }
        let header = Self {
            width: get_u32(src, 12),
            height: get_u32(src, 16),
            stride: get_u32(src, 24),
            fps_num: get_u32(src, 28),
            fps_den: get_u32(src, 32),
            slot_count: get_u32(src, 36),
            slot_stride: get_u32(src, 40),
            active_index: get_u32(src, 44),
            last_timestamp_us: get_u64(src, 48),
        };
        header.check()?;
        Ok(header)
    }

    pub fn slot_offset(&self, index: u32) -> Result<usize, ProtocolError> {
        self.check()?;
        if index >= self.slot_count {
            return Err(ProtocolError::InvalidField { field: "slot" });
        }
        CAMERA_HEADER_LEN
            .checked_add(
                (index as usize)
                    .checked_mul(self.slot_stride as usize)
                    .ok_or(ProtocolError::InvalidField { field: "slot" })?,
            )
            .ok_or(ProtocolError::InvalidField { field: "slot" })
    }

    fn check(&self) -> Result<(), ProtocolError> {
        if self.width == 0
            || self.height == 0
            || self.width > crate::codes::MAX_EDGE
            || self.height > crate::codes::MAX_EDGE
            || self.width % 2 != 0
            || self.height % 2 != 0
            || self.stride < self.width
            || self.stride % 2 != 0
            || self.stride > 16_384
        {
            return Err(ProtocolError::InvalidField {
                field: "camera geometry",
            });
        }
        if self.fps_den == 0 || self.fps_num / self.fps_den > crate::codes::MAX_FPS {
            return Err(ProtocolError::InvalidField {
                field: "camera fps",
            });
        }
        if !(2..=4).contains(&self.slot_count) || self.active_index >= self.slot_count {
            return Err(ProtocolError::InvalidField {
                field: "camera slots",
            });
        }
        let plane = u64::from(self.stride) * u64::from(self.height);
        let nv12 = plane
            .checked_mul(3)
            .and_then(|value| value.checked_div(2))
            .ok_or(ProtocolError::InvalidField {
                field: "camera geometry",
            })?;
        if u64::from(self.slot_stride) < nv12 || self.slot_stride % 64 != 0 {
            return Err(ProtocolError::InvalidField {
                field: "slot stride",
            });
        }
        Ok(())
    }
}

impl MicHeader {
    pub fn write_to(&self, dst: &mut [u8]) -> Result<(), ProtocolError> {
        if dst.len() < MIC_HEADER_LEN {
            return Err(ProtocolError::UnexpectedEof);
        }
        self.check()?;
        dst[..MIC_HEADER_LEN].fill(0);
        dst[..8].copy_from_slice(MIC_MAGIC);
        put_u32(dst, 8, 1);
        put_u32(dst, 12, self.sample_rate);
        put_u16(dst, 16, self.channels);
        put_u16(dst, 18, PCM_S16LE);
        put_u32(dst, 20, u32::from(self.channels) * 2);
        put_u32(dst, 24, self.capacity_frames);
        put_u64(dst, 32, self.write_frame);
        Ok(())
    }

    pub fn read_from(src: &[u8]) -> Result<Self, ProtocolError> {
        if src.len() < MIC_HEADER_LEN {
            return Err(ProtocolError::UnexpectedEof);
        }
        if &src[..8] != MIC_MAGIC {
            return Err(ProtocolError::InvalidField { field: "mic magic" });
        }
        let version = get_u32(src, 8);
        if version != 1 {
            return Err(ProtocolError::UnsupportedVersion { found: version });
        }
        if get_u16(src, 18) != PCM_S16LE {
            return Err(ProtocolError::InvalidField {
                field: "mic format",
            });
        }
        let header = Self {
            sample_rate: get_u32(src, 12),
            channels: get_u16(src, 16),
            capacity_frames: get_u32(src, 24),
            write_frame: get_u64(src, 32),
        };
        let frame_bytes = get_u32(src, 20);
        if frame_bytes != u32::from(header.channels) * 2
            || get_u32(src, 28) != 0
            || get_u64(src, 40) != 0
        {
            return Err(ProtocolError::InvalidField {
                field: "mic header",
            });
        }
        header.check()?;
        Ok(header)
    }

    pub fn frame_byte_offset(&self, frame_index: u64) -> Result<usize, ProtocolError> {
        self.check()?;
        let slot = frame_index % u64::from(self.capacity_frames);
        let frame_bytes = u64::from(self.channels) * 2;
        MIC_HEADER_LEN
            .checked_add(usize::try_from(slot * frame_bytes).map_err(|_| {
                ProtocolError::InvalidField {
                    field: "mic offset",
                }
            })?)
            .ok_or(ProtocolError::InvalidField {
                field: "mic offset",
            })
    }

    fn check(&self) -> Result<(), ProtocolError> {
        if !(8_000..=192_000).contains(&self.sample_rate) || !(1..=2).contains(&self.channels) {
            return Err(ProtocolError::InvalidField {
                field: "mic format",
            });
        }
        if self.capacity_frames < 32
            || !self.capacity_frames.is_power_of_two()
            || self.capacity_frames > 65_536
        {
            return Err(ProtocolError::InvalidField {
                field: "mic capacity",
            });
        }
        Ok(())
    }
}

fn put_u16(dst: &mut [u8], at: usize, value: u16) {
    dst[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(dst: &mut [u8], at: usize, value: u32) {
    dst[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(dst: &mut [u8], at: usize, value: u64) {
    dst[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn get_u16(src: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([src[at], src[at + 1]])
}

fn get_u32(src: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([src[at], src[at + 1], src[at + 2], src[at + 3]])
}

fn get_u64(src: &[u8], at: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&src[at..at + 8]);
    u64::from_le_bytes(bytes)
}
