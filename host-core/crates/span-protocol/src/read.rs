use crate::error::ProtocolError;

pub(crate) struct Reader<'a> {
    buf: &'a [u8],
    index: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Self { buf, index: 0 }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.index)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, ProtocolError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, ProtocolError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, ProtocolError> {
        let bytes = self.take(8)?;
        let mut wide = [0u8; 8];
        wide.copy_from_slice(bytes);
        Ok(u64::from_be_bytes(wide))
    }

    pub(crate) fn take(&mut self, len: usize) -> Result<&'a [u8], ProtocolError> {
        let end = self
            .index
            .checked_add(len)
            .ok_or(ProtocolError::UnexpectedEof)?;
        let slice = self
            .buf
            .get(self.index..end)
            .ok_or(ProtocolError::UnexpectedEof)?;
        self.index = end;
        Ok(slice)
    }
}
