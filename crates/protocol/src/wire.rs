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

fn need(buf: &Bytes, n: usize) -> Result<(), PacketDecodeError> {
    if buf.remaining() < n {
        Err(PacketDecodeError::UnexpectedEof)
    } else {
        Ok(())
    }
}

impl WireWrite for bool {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_u8(if *self { 0x01 } else { 0x00 });
    }
}
impl WireRead for bool {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 1)?;
        Ok(buf.get_u8() != 0)
    }
}
impl WireWrite for u8 {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_u8(*self);
    }
}
impl WireRead for u8 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 1)?;
        Ok(buf.get_u8())
    }
}
impl WireWrite for i8 {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_i8(*self);
    }
}
impl WireRead for i8 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 1)?;
        Ok(buf.get_i8())
    }
}
impl WireWrite for u16 {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_u16(*self);
    }
}
impl WireRead for u16 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 2)?;
        Ok(buf.get_u16())
    }
}
impl WireWrite for i16 {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_i16(*self);
    }
}
impl WireRead for i16 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 2)?;
        Ok(buf.get_i16())
    }
}
impl WireWrite for i32 {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_i32(*self);
    }
}
impl WireRead for i32 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 4)?;
        Ok(buf.get_i32())
    }
}
impl WireWrite for i64 {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_i64(*self);
    }
}
impl WireRead for i64 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 8)?;
        Ok(buf.get_i64())
    }
}
impl WireWrite for f32 {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_f32(*self);
    }
}
impl WireRead for f32 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 4)?;
        Ok(buf.get_f32())
    }
}
impl WireWrite for f64 {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_f64(*self);
    }
}
impl WireRead for f64 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 8)?;
        Ok(buf.get_f64())
    }
}
impl WireWrite for VarInt {
    fn write_wire(&self, buf: &mut BytesMut) {
        self.encode(buf);
    }
}
impl WireRead for VarInt {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        Ok(VarInt::decode(buf)?)
    }
}
impl WireWrite for VarLong {
    fn write_wire(&self, buf: &mut BytesMut) {
        self.encode(buf);
    }
}
impl WireRead for VarLong {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        Ok(VarLong::decode(buf)?)
    }
}
impl WireWrite for String {
    fn write_wire(&self, buf: &mut BytesMut) {
        let bytes = self.as_bytes();
        VarInt::new(bytes.len() as i32).encode(buf);
        buf.put_slice(bytes);
    }
}
impl WireRead for String {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        let declared_bytes = VarInt::decode(buf)?.get();
        let declared_bytes = usize::try_from(declared_bytes).unwrap_or(usize::MAX);
        // Conservative pre-allocation-size sanity bound: no valid string within
        // MAX_STRING_LENGTH chars can ever need more than 4 bytes per char (UTF-8's own
        // maximum encoded width), so this rejects a hostile declared length before any
        // allocation or byte read is attempted.
        if declared_bytes > MAX_STRING_LENGTH * 4 {
            return Err(PacketDecodeError::StringTooLong {
                actual: declared_bytes,
                max: MAX_STRING_LENGTH,
            });
        }
        need(buf, declared_bytes)?;
        let raw = buf.copy_to_bytes(declared_bytes);
        let s = String::from_utf8(raw.to_vec()).map_err(|_| PacketDecodeError::InvalidUtf8)?;
        let char_count = s.chars().count();
        if char_count > MAX_STRING_LENGTH {
            return Err(PacketDecodeError::StringTooLong {
                actual: char_count,
                max: MAX_STRING_LENGTH,
            });
        }
        Ok(s)
    }
}

/// Emitted by `#[derive(RcPacket)]` for an `#[rc(varint)]`-attributed `i32` field.
pub fn write_varint_field(value: i32, buf: &mut BytesMut) {
    VarInt::new(value).encode(buf);
}
pub fn read_varint_field(buf: &mut Bytes) -> Result<i32, PacketDecodeError> {
    Ok(VarInt::decode(buf)?.get())
}
/// Emitted by `#[derive(RcPacket)]` for an `#[rc(varint)]`-attributed `i64` field.
pub fn write_varlong_field(value: i64, buf: &mut BytesMut) {
    VarLong::new(value).encode(buf);
}
pub fn read_varlong_field(buf: &mut Bytes) -> Result<i64, PacketDecodeError> {
    Ok(VarLong::decode(buf)?.get())
}

/// Emitted by `#[derive(RcPacket)]` for an `#[rc(prefixed_array = "VarInt")]` field:
/// `VarInt` element count followed by each element's own `WireWrite`/`WireRead` encoding.
/// Decode rejects a declared count exceeding `buf.remaining()` (every `WireRead` type needs
/// at least one byte, so this is always a safe, non-false-positive sanity bound against a
/// malicious huge count paired with too few actual bytes).
pub fn write_prefixed_vec<T: WireWrite>(items: &[T], buf: &mut BytesMut) {
    VarInt::new(items.len() as i32).encode(buf);
    for item in items {
        item.write_wire(buf);
    }
}
pub fn read_prefixed_vec<T: WireRead>(buf: &mut Bytes) -> Result<Vec<T>, PacketDecodeError> {
    let declared = VarInt::decode(buf)?.get();
    let declared = usize::try_from(declared).unwrap_or(usize::MAX);
    if declared > buf.remaining() {
        return Err(PacketDecodeError::ArrayTooLong {
            declared,
            remaining: buf.remaining(),
        });
    }
    let mut items = Vec::with_capacity(declared);
    for _ in 0..declared {
        items.push(T::read_wire(buf)?);
    }
    Ok(items)
}
