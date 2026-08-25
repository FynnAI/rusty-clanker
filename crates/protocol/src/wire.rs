use crate::packet::PacketDecodeError;
use crate::varint::{VarInt, VarLong};
use bytes::{Buf, BufMut, Bytes, BytesMut};

/// `FriendlyByteBuf.MAX_STRING_LENGTH` in the reference — the maximum **character** count
/// (not byte count) a `String` field may decode to.
pub const MAX_STRING_LENGTH: usize = 32_767;

/// Encodes one packet field's value onto `buf`, per the field-type -> wire-type mapping
/// table. Implemented for every default-mapped primitive type plus `VarInt`, `VarLong`,
/// and `String`.
pub trait WireWrite {
    fn write_wire(&self, buf: &mut BytesMut);
}

/// Decodes one packet field's value from the front of `buf`, per the same mapping table.
pub trait WireRead: Sized {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError>;
}

// WireWrite/WireRead are implemented in this file for: bool, u8, i8, u16, i16, i32, i64,
// f32, f64, VarInt, VarLong, String — per the exact per-type wire layout the mapping table
// fixes (bodies specified in Implementation steps, not restated here — every impl is a
// direct, mechanical application of that table's own row).
impl WireWrite for bool {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for bool {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for u8 {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for u8 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for i8 {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for i8 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for u16 {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for u16 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for i16 {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for i16 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for i32 {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for i32 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for i64 {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for i64 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for f32 {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for f32 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for f64 {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for f64 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for VarInt {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for VarInt {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for VarLong {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for VarLong {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}
impl WireWrite for String {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for String {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}

/// Emitted by `#[derive(RcPacket)]` for an `#[rc(varint)]`-attributed `i32` field.
pub fn write_varint_field(value: i32, buf: &mut BytesMut) {
    todo!()
}
pub fn read_varint_field(buf: &mut Bytes) -> Result<i32, PacketDecodeError> {
    todo!()
}
/// Emitted by `#[derive(RcPacket)]` for an `#[rc(varint)]`-attributed `i64` field.
pub fn write_varlong_field(value: i64, buf: &mut BytesMut) {
    todo!()
}
pub fn read_varlong_field(buf: &mut Bytes) -> Result<i64, PacketDecodeError> {
    todo!()
}

/// Emitted by `#[derive(RcPacket)]` for an `#[rc(prefixed_array = "VarInt")]` field:
/// `VarInt` element count followed by each element's own `WireWrite`/`WireRead` encoding.
/// Decode rejects a declared count exceeding `buf.remaining()` (every `WireRead` type needs
/// at least one byte, so this is always a safe, non-false-positive sanity bound against a
/// malicious huge count paired with too few actual bytes).
pub fn write_prefixed_vec<T: WireWrite>(items: &[T], buf: &mut BytesMut) {
    todo!()
}
pub fn read_prefixed_vec<T: WireRead>(buf: &mut Bytes) -> Result<Vec<T>, PacketDecodeError> {
    todo!()
}
