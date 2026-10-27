use crate::error::ProtocolError;

/// Offset of the receiver clock relative to the sender, in microseconds.
///
/// `t0` and `t3` are the sender's monotonic clock. `t1` and `t2` are the
/// receiver's monotonic clock. Integer division truncates toward zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSample {
    pub offset_us: i64,
    pub delay_us: u64,
}

pub fn clock_sample(t0: u64, t1: u64, t2: u64, t3: u64) -> Result<ClockSample, ProtocolError> {
    let t0 = i128::from(t0);
    let t1 = i128::from(t1);
    let t2 = i128::from(t2);
    let t3 = i128::from(t3);
    if t2 < t1 {
        return Err(ProtocolError::InconsistentClock);
    }
    let delay = (t3 - t0) - (t2 - t1);
    if delay < 0 || delay > i128::from(u64::MAX) {
        return Err(ProtocolError::InconsistentClock);
    }
    let offset = ((t1 - t0) + (t2 - t3)) / 2;
    if offset > i128::from(i64::MAX) || offset < i128::from(i64::MIN) {
        return Err(ProtocolError::InconsistentClock);
    }
    Ok(ClockSample {
        offset_us: offset as i64,
        delay_us: delay as u64,
    })
}
