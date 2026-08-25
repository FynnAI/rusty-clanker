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
        todo!()
    }

    pub const fn get(self) -> i32 {
        todo!()
    }

    /// Number of bytes this specific value encodes to, `1..=Self::MAX_ENCODED_LEN`.
    pub fn encoded_len(self) -> usize {
        todo!()
    }

    /// Never fails — every `i32` fits within `MAX_ENCODED_LEN` bytes.
    pub fn encode(self, buf: &mut impl BufMut) {
        todo!()
    }

    /// Decodes one `VarInt` from the front of `buf`, advancing it by exactly the bytes
    /// consumed on success. Never consumes more than `MAX_ENCODED_LEN` bytes.
    pub fn decode(buf: &mut impl Buf) -> Result<Self, VarNumError> {
        todo!()
    }
}

/// A 64-bit signed integer encoded the same way as `VarInt`, capped at 10 bytes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct VarLong(pub i64);

impl VarLong {
    pub const MAX_ENCODED_LEN: usize = 10;

    pub const fn new(value: i64) -> Self {
        todo!()
    }

    pub const fn get(self) -> i64 {
        todo!()
    }

    pub fn encoded_len(self) -> usize {
        todo!()
    }

    pub fn encode(self, buf: &mut impl BufMut) {
        todo!()
    }

    pub fn decode(buf: &mut impl Buf) -> Result<Self, VarNumError> {
        todo!()
    }
}
