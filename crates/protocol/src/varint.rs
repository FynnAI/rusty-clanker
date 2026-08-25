use bytes::{Buf, BufMut};

/// One VarInt/VarLong decode failure mode — shared by both types (the algorithm is
/// identical in shape, only the byte-width cap differs). See the crate-level Context
/// this blueprint restates: no zigzag encoding, raw two's-complement bit pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VarNumError {
    #[error(
        "VarInt/VarLong value used more continuation bytes than its type's maximum encoded width allows"
    )]
    TooLong,
    #[error("buffer ran out of bytes before the VarInt/VarLong's continuation bit cleared")]
    UnexpectedEof,
}

/// A 32-bit signed integer encoded as Minecraft's variable-length VarInt (no zigzag, raw
/// two's-complement bit pattern, 7 data bits per byte with the MSB as a continuation flag).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct VarInt(pub i32);

impl VarInt {
    /// Maximum bytes one encoded `VarInt` ever occupies.
    pub const MAX_ENCODED_LEN: usize = 5;

    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i32 {
        self.0
    }

    /// Number of bytes this specific value encodes to, `1..=Self::MAX_ENCODED_LEN`.
    pub fn encoded_len(self) -> usize {
        let mut v = self.0 as u32;
        let mut len = 1usize;
        while v & !0x7F != 0 {
            v >>= 7;
            len += 1;
        }
        len
    }

    /// Never fails — every `i32` fits within `MAX_ENCODED_LEN` bytes.
    pub fn encode(self, buf: &mut impl BufMut) {
        let mut v = self.0 as u32;
        loop {
            if v & !0x7F == 0 {
                buf.put_u8(v as u8);
                return;
            }
            buf.put_u8((v as u8 & 0x7F) | 0x80);
            v >>= 7;
        }
    }

    /// Decodes one `VarInt` from the front of `buf`, advancing it by exactly the bytes
    /// consumed on success. Never consumes more than `MAX_ENCODED_LEN` bytes.
    pub fn decode(buf: &mut impl Buf) -> Result<Self, VarNumError> {
        let mut result: i32 = 0;
        for i in 0..Self::MAX_ENCODED_LEN {
            if !buf.has_remaining() {
                return Err(VarNumError::UnexpectedEof);
            }
            let byte = buf.get_u8();
            result |= ((byte & 0x7F) as i32) << (7 * i);
            if byte & 0x80 == 0 {
                return Ok(Self(result));
            }
        }
        Err(VarNumError::TooLong)
    }
}

/// A 64-bit signed integer encoded the same way as `VarInt`, capped at 10 bytes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct VarLong(pub i64);

impl VarLong {
    pub const MAX_ENCODED_LEN: usize = 10;

    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i64 {
        self.0
    }

    pub fn encoded_len(self) -> usize {
        let mut v = self.0 as u64;
        let mut len = 1usize;
        while v & !0x7F != 0 {
            v >>= 7;
            len += 1;
        }
        len
    }

    pub fn encode(self, buf: &mut impl BufMut) {
        let mut v = self.0 as u64;
        loop {
            if v & !0x7F == 0 {
                buf.put_u8(v as u8);
                return;
            }
            buf.put_u8((v as u8 & 0x7F) | 0x80);
            v >>= 7;
        }
    }

    pub fn decode(buf: &mut impl Buf) -> Result<Self, VarNumError> {
        let mut result: i64 = 0;
        for i in 0..Self::MAX_ENCODED_LEN {
            if !buf.has_remaining() {
                return Err(VarNumError::UnexpectedEof);
            }
            let byte = buf.get_u8();
            result |= ((byte & 0x7F) as i64) << (7 * i);
            if byte & 0x80 == 0 {
                return Ok(Self(result));
            }
        }
        Err(VarNumError::TooLong)
    }
}
