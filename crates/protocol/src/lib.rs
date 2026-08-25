//! `rc-protocol` — wire codec foundation: VarInt/VarLong (`varint`), packet framing plus
//! zlib compression (`frame`, NET-D5), the `WireWrite`/`WireRead` field-encoding traits and
//! the `RcPacket` trait model (`wire`, `packet`, NET-D3), the `ConnectionState`/`PacketBound`
//! connection-state scaffolding (NET-D4), and the `ConnectionCipher` seam NET-D6's real
//! encryption implementation plugs into (`cipher`). Pure data/codec — no sockets, no Tokio
//! (`12-workspace-structure.md`'s WS-D2); the Tokio reader/writer task pair that drives this
//! codec over a real `TcpStream` lives in `rusty-clanker-server`'s `net` module.
//!
//! No concrete packet type is defined by this crate — every item here is generic
//! infrastructure a later milestone's per-connection-state packet catalog builds on.

pub mod cipher;
pub mod frame;
pub mod packet;
pub mod varint;
pub mod wire;

pub use bytes::{Bytes, BytesMut};
pub use cipher::ConnectionCipher;
pub use frame::{
    CompressionState, FrameError, MAX_FRAME_LENGTH, MAX_UNCOMPRESSED_LENGTH, encode_frame,
    try_decode_frame,
};
pub use packet::{
    ConnectionState, PacketBound, PacketCatalog, PacketDecodeError, RawPacket, RcPacket,
    decode_one, encode_payload,
};
/// Re-exported **without** renaming — required, not cosmetic. A `pub use path::Name as
/// Alias;` binds an item only under `Alias`, in whichever namespace(s) it occupies at that
/// site; a `#[proc_macro_derive(RcPacket, ...)]` item occupies only the macro namespace, so
/// renaming it here would make that namespace's `RcPacket` binding unreachable through this
/// crate (`RcPacket` the *trait*, re-exported above from `packet::RcPacket`, would remain
/// reachable, but `#[derive(RcPacket)]` would not — verified against `rustc` 1.94.1: a
/// renamed re-export reproduces `error: cannot find derive macro` at every downstream call
/// site). Leaving the name unrenamed is exactly what lets `use rc_protocol::RcPacket;` bring
/// both the trait (type namespace) and the derive macro (macro namespace) into scope at
/// once — the same pattern `serde`'s own `pub use serde_derive::{Deserialize, Serialize};`
/// (itself unrenamed) uses for its identically-named trait+derive pairs.
pub use rc_protocol_macros::RcPacket;
pub use varint::{VarInt, VarLong, VarNumError};
pub use wire::{
    MAX_STRING_LENGTH, WireRead, WireWrite, read_prefixed_vec, read_varint_field,
    read_varlong_field, write_prefixed_vec, write_varint_field, write_varlong_field,
};
