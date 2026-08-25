//! `rc-protocol-macros` — `#[derive(RcPacket)]` (NET-D3). See `rc-protocol`'s `packet`
//! module for the trait this macro implements and this blueprint's Context, "`#[derive(
//! RcPacket)]` — exact expansion algorithm," for this macro's complete, binding codegen
//! specification.

/// Implements `rc_protocol::RcPacket` for a struct carrying `#[packet(state = "...",
/// bound = "...", id = ...)]` and, per field, an optional `#[rc(varint)] |
/// #[rc(prefixed_array = "VarInt")] | #[rc(nbt)]` attribute.
#[proc_macro_derive(RcPacket, attributes(packet, rc))]
pub fn derive_rc_packet(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    todo!()
}
