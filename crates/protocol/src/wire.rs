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

/// A plain-text chat/text-component field encoded as a **JSON string** on the wire: one
/// VarInt-length-prefixed UTF-8 `String` holding `{"text": "..."}` — protocol 776's real
/// shape for the Login-phase `LoginDisconnect` reason (`ClientboundLoginDisconnectPacket`'s
/// stream codec is a lenient-JSON string codec, ASSET-D18(f) reference). M2 field-report
/// fix, discovered by a real vanilla client rejecting the network-NBT shape
/// (`NbtTextComponent`) during a rejoin: NBT text components are correct only from the
/// Configuration phase onward — the Login phase predates the registry/NBT context and still
/// speaks JSON. azalea tolerated the NBT shape; the real client is the oracle.
///
/// Writes escape quotes, backslashes, and control characters (`\u00XX`); the reader accepts
/// exactly the `{"text": "..."}` shape this type writes (mirroring `NbtTextComponent`'s
/// single-shape minimalism — not a general JSON parser). Vanilla's codec caps this field at
/// 262144 bytes; every reason this codebase sends is a short fixed diagnostic, far below
/// both that cap and the wire `String` limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonTextComponent(pub String);

impl JsonTextComponent {
    fn escape_into(text: &str, out: &mut String) {
        for c in text.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                c if (c as u32) < 0x20 => {
                    use std::fmt::Write;
                    let _ = write!(out, "\\u{:04x}", c as u32);
                }
                c => out.push(c),
            }
        }
    }
}

impl WireWrite for JsonTextComponent {
    fn write_wire(&self, buf: &mut BytesMut) {
        let mut json = String::with_capacity(self.0.len() + 11);
        json.push_str("{\"text\":\"");
        Self::escape_into(&self.0, &mut json);
        json.push_str("\"}");
        json.write_wire(buf);
    }
}

impl WireRead for JsonTextComponent {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        let json = String::read_wire(buf)?;
        let inner = json
            .strip_prefix("{\"text\":\"")
            .and_then(|rest| rest.strip_suffix("\"}"))
            .ok_or_else(|| {
                PacketDecodeError::MalformedJsonTextComponent(format!(
                    "expected {{\"text\":\"...\"}}, got {json:?}"
                ))
            })?;
        let mut text = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                if c == '"' {
                    return Err(PacketDecodeError::MalformedJsonTextComponent(
                        "unescaped quote inside the text value".to_string(),
                    ));
                }
                text.push(c);
                continue;
            }
            match chars.next() {
                Some('"') => text.push('"'),
                Some('\\') => text.push('\\'),
                Some('/') => text.push('/'),
                Some('n') => text.push('\n'),
                Some('t') => text.push('\t'),
                Some('r') => text.push('\r'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    let code = (hex.len() == 4)
                        .then(|| u32::from_str_radix(&hex, 16).ok())
                        .flatten()
                        .and_then(char::from_u32)
                        .ok_or_else(|| {
                            PacketDecodeError::MalformedJsonTextComponent(format!(
                                "bad \\u escape: {hex:?}"
                            ))
                        })?;
                    text.push(code);
                }
                other => {
                    return Err(PacketDecodeError::MalformedJsonTextComponent(format!(
                        "unsupported escape: \\{other:?}"
                    )));
                }
            }
        }
        Ok(JsonTextComponent(text))
    }
}

/// A plain-text chat/text-component field encoded as network NBT (`{"text": "..."}`'s NBT
/// equivalent: an unnamed root `TAG_Compound` holding one `TAG_String` field named `text`),
/// per protocol 776's real wire shape for text components from the **Configuration phase
/// onward** (the Configuration-phase `Disconnect` reason today) — the Login phase instead
/// speaks JSON (`JsonTextComponent`, above). M1 integration fix, discovered by driving a
/// real client (azalea) against `rusty-clanker-server`: a raw `WireWrite`-`String`
/// (VarInt-length-prefixed UTF-8) reason is what M1-B04 originally shipped for the
/// Configuration disconnect, but a real client's NBT decoder chokes on it (a short
/// JSON reason's own VarInt length byte gets misread as an invalid raw NBT tag id). No real
/// `rc-nbt` integration exists yet (`configuration.rs`'s own `#[rc(nbt)]` deferral, M1-B04)
/// — this is a minimal, purpose-built stand-in for exactly this one field shape, not a
/// general NBT codec; a later blueprint wiring real `rc-nbt` support should replace it.
/// Every text this type actually carries in this codebase today is plain ASCII, so a raw
/// UTF-8 byte count (rather than Java's "modified UTF-8" scheme, which only differs for
/// astral-plane characters and embedded NULs) is exact for every real call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NbtTextComponent(pub String);

const NBT_TAG_END: u8 = 0x00;
const NBT_TAG_STRING: u8 = 0x08;
const NBT_TAG_COMPOUND: u8 = 0x0A;

impl WireWrite for NbtTextComponent {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_u8(NBT_TAG_COMPOUND);
        buf.put_u8(NBT_TAG_STRING);
        let key = b"text";
        buf.put_u16(key.len() as u16);
        buf.put_slice(key);
        let value = self.0.as_bytes();
        buf.put_u16(value.len() as u16);
        buf.put_slice(value);
        buf.put_u8(NBT_TAG_END);
    }
}
impl WireRead for NbtTextComponent {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 1)?;
        let root_tag = buf.get_u8();
        if root_tag != NBT_TAG_COMPOUND {
            return Err(PacketDecodeError::MalformedNbtTextComponent(format!(
                "expected root TAG_Compound (0x0A), got {root_tag:#04x}"
            )));
        }
        let mut text: Option<String> = None;
        loop {
            need(buf, 1)?;
            let tag = buf.get_u8();
            if tag == NBT_TAG_END {
                break;
            }
            need(buf, 2)?;
            let key_len = buf.get_u16() as usize;
            need(buf, key_len)?;
            let key = buf.copy_to_bytes(key_len);
            if tag != NBT_TAG_STRING {
                return Err(PacketDecodeError::MalformedNbtTextComponent(format!(
                    "unsupported field tag {tag:#04x} for key {key:?}"
                )));
            }
            need(buf, 2)?;
            let value_len = buf.get_u16() as usize;
            need(buf, value_len)?;
            let value = buf.copy_to_bytes(value_len);
            let value = String::from_utf8(value.to_vec()).map_err(|_| {
                PacketDecodeError::MalformedNbtTextComponent("value not valid UTF-8".to_string())
            })?;
            if key.as_ref() == b"text" {
                text = Some(value);
            }
        }
        text.map(NbtTextComponent).ok_or_else(|| {
            PacketDecodeError::MalformedNbtTextComponent("missing \"text\" field".to_string())
        })
    }
}

/// Tag ids for the binary NBT format (network variant: unnamed root, no name field) —
/// `crates/registries/generated/v776/registry_entries.rs`'s own future content and every
/// `RegistryDataEntryOut` inline payload this crate ever sends is shaped this way. Unlike
/// `NbtTextComponent` (which only understands one fixed `{"text": ...}` shape), a registry
/// entry's payload is, in general, an arbitrary Mojang registry element
/// (`DimensionKindElement` and friends) whose exact field set this crate does not model —
/// decoding it therefore only needs to measure/skip it byte-for-byte, never interpret it.
/// No real `rc-nbt` integration exists yet (`configuration.rs`'s own `#[rc(nbt)]` deferral,
/// M1-B04); this is a minimal, purpose-built stand-in for exactly this one need, mirroring
/// `NbtTextComponent`'s own precedent (M1 integration fix) rather than a general NBT codec.
mod nbt_raw {
    use super::{Buf, Bytes, PacketDecodeError, need};

    const TAG_END: u8 = 0x00;
    const TAG_BYTE: u8 = 0x01;
    const TAG_SHORT: u8 = 0x02;
    const TAG_INT: u8 = 0x03;
    const TAG_LONG: u8 = 0x04;
    const TAG_FLOAT: u8 = 0x05;
    const TAG_DOUBLE: u8 = 0x06;
    const TAG_BYTE_ARRAY: u8 = 0x07;
    const TAG_STRING: u8 = 0x08;
    const TAG_LIST: u8 = 0x09;
    const TAG_COMPOUND: u8 = 0x0A;
    const TAG_INT_ARRAY: u8 = 0x0B;
    const TAG_LONG_ARRAY: u8 = 0x0C;

    fn skip_nbt_string(buf: &mut Bytes) -> Result<(), PacketDecodeError> {
        need(buf, 2)?;
        let len = buf.get_u16() as usize;
        need(buf, len)?;
        buf.advance(len);
        Ok(())
    }

    fn skip_nbt_payload(tag: u8, buf: &mut Bytes) -> Result<(), PacketDecodeError> {
        match tag {
            TAG_BYTE => {
                need(buf, 1)?;
                buf.advance(1);
            }
            TAG_SHORT => {
                need(buf, 2)?;
                buf.advance(2);
            }
            TAG_INT | TAG_FLOAT => {
                need(buf, 4)?;
                buf.advance(4);
            }
            TAG_LONG | TAG_DOUBLE => {
                need(buf, 8)?;
                buf.advance(8);
            }
            TAG_BYTE_ARRAY => {
                need(buf, 4)?;
                let len = buf.get_i32().max(0) as usize;
                need(buf, len)?;
                buf.advance(len);
            }
            TAG_STRING => skip_nbt_string(buf)?,
            TAG_LIST => {
                need(buf, 1)?;
                let element_tag = buf.get_u8();
                need(buf, 4)?;
                let count = buf.get_i32().max(0);
                if element_tag != TAG_END {
                    for _ in 0..count {
                        skip_nbt_payload(element_tag, buf)?;
                    }
                }
            }
            TAG_COMPOUND => skip_nbt_compound_body(buf)?,
            TAG_INT_ARRAY => {
                need(buf, 4)?;
                let len = buf.get_i32().max(0) as usize;
                let bytes = len.saturating_mul(4);
                need(buf, bytes)?;
                buf.advance(bytes);
            }
            TAG_LONG_ARRAY => {
                need(buf, 4)?;
                let len = buf.get_i32().max(0) as usize;
                let bytes = len.saturating_mul(8);
                need(buf, bytes)?;
                buf.advance(bytes);
            }
            other => {
                return Err(PacketDecodeError::MalformedRegistryEntryNbt(format!(
                    "unsupported NBT tag {other:#04x}"
                )));
            }
        }
        Ok(())
    }

    fn skip_nbt_compound_body(buf: &mut Bytes) -> Result<(), PacketDecodeError> {
        loop {
            need(buf, 1)?;
            let tag = buf.get_u8();
            if tag == TAG_END {
                return Ok(());
            }
            skip_nbt_string(buf)?; // field name
            skip_nbt_payload(tag, buf)?;
        }
    }

    /// Reads one full network-NBT value (unnamed root `TAG_Compound`, `TAG_End`-terminated —
    /// protocol 776's Registry Data entry-payload shape) starting at `buf`'s current
    /// position, returning the exact raw bytes consumed (leading tag byte through the
    /// matching `TAG_End`) without interpreting any field's meaning.
    pub fn read_raw_compound(buf: &mut Bytes) -> Result<Vec<u8>, PacketDecodeError> {
        let start_remaining = buf.remaining();
        let snapshot = buf.clone();
        need(buf, 1)?;
        let root_tag = buf.get_u8();
        if root_tag != TAG_COMPOUND {
            return Err(PacketDecodeError::MalformedRegistryEntryNbt(format!(
                "expected root TAG_Compound (0x0A), got {root_tag:#04x}"
            )));
        }
        skip_nbt_compound_body(buf)?;
        let consumed = start_remaining - buf.remaining();
        Ok(snapshot.slice(0..consumed).to_vec())
    }
}
pub(crate) use nbt_raw::read_raw_compound;

/// Java's UUID is the standard RFC 4122 big-endian 16-byte layout (most-significant 8
/// bytes, then least-significant 8 bytes) — `uuid::Uuid::as_bytes`/`from_bytes` already use
/// exactly that layout, so no byte reordering is needed. No length prefix (M1-B04).
impl WireWrite for uuid::Uuid {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_slice(self.as_bytes());
    }
}
impl WireRead for uuid::Uuid {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        need(buf, 16)?;
        let mut bytes = [0u8; 16];
        buf.copy_to_slice(&mut bytes);
        Ok(uuid::Uuid::from_bytes(bytes))
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
