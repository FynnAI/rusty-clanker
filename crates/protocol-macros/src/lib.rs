//! `rc-protocol-macros` — `#[derive(RcPacket)]` (NET-D3). See `rc-protocol`'s `packet`
//! module for the trait this macro implements and this blueprint's Context, "`#[derive(
//! RcPacket)]` — exact expansion algorithm," for this macro's complete, binding codegen
//! specification.

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

/// Implements `rc_protocol::RcPacket` for a struct carrying `#[packet(state = "...",
/// bound = "...", id = ...)]` and, per field, an optional `#[rc(varint)] |
/// #[rc(prefixed_array = "VarInt")] | #[rc(nbt)]` attribute.
#[proc_macro_derive(RcPacket, attributes(packet, rc))]
pub fn derive_rc_packet(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match expand(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// The parsed `#[packet(state = "...", bound = "...", id = ...)]` container attribute.
struct PacketAttr {
    state_variant: syn::Ident,
    bound_variant: syn::Ident,
    id: syn::LitInt,
}

/// The parsed, priority-resolved `#[rc(...)]` field attribute (or its absence).
enum FieldAttr {
    None,
    Varint,
    PrefixedArray(syn::LitStr),
    Nbt,
}

fn expand(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let packet_attr = parse_packet_attr(&input)?;
    let struct_name = &input.ident;

    let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(named_fields),
        ..
    }) = &input.data
    else {
        return Err(syn::Error::new(
            input.ident.span(),
            "#[derive(RcPacket)] only supports a struct with named fields",
        ));
    };

    let mut encode_stmts = Vec::with_capacity(named_fields.named.len());
    let mut decode_stmts = Vec::with_capacity(named_fields.named.len());
    let mut field_names = Vec::with_capacity(named_fields.named.len());

    for field in &named_fields.named {
        let field_name = field
            .ident
            .as_ref()
            .expect("Fields::Named field always has an ident")
            .clone();
        let field_ty = &field.ty;
        let field_attr = parse_field_attr(field)?;

        let (encode_stmt, decode_stmt) = match field_attr {
            FieldAttr::Nbt => {
                return Err(syn::Error::new(
                    field.span(),
                    "#[rc(nbt)] is recognized but not yet implemented by rc-protocol-macros — deferred to the blueprint that wires rc-nbt encoding into the derive macro",
                ));
            }
            FieldAttr::Varint => match last_segment_ident(field_ty).as_deref() {
                Some("i32") => (
                    quote! { rc_protocol::write_varint_field(self.#field_name, buf); },
                    quote! { let #field_name = rc_protocol::read_varint_field(buf)?; },
                ),
                Some("i64") => (
                    quote! { rc_protocol::write_varlong_field(self.#field_name, buf); },
                    quote! { let #field_name = rc_protocol::read_varlong_field(buf)?; },
                ),
                _ => {
                    return Err(syn::Error::new(
                        field_ty.span(),
                        "#[rc(varint)] may only be applied to an i32 or i64 field",
                    ));
                }
            },
            FieldAttr::PrefixedArray(kind) => {
                if kind.value() != "VarInt" {
                    return Err(syn::Error::new(
                        kind.span(),
                        "#[rc(prefixed_array = \"...\")] only supports \"VarInt\" at this time",
                    ));
                }
                if last_segment_ident(field_ty).as_deref() != Some("Vec") {
                    return Err(syn::Error::new(
                        field_ty.span(),
                        "#[rc(prefixed_array = ...)] may only be applied to a Vec<T> field",
                    ));
                }
                (
                    quote! { rc_protocol::write_prefixed_vec(&self.#field_name, buf); },
                    quote! { let #field_name = rc_protocol::read_prefixed_vec(buf)?; },
                )
            }
            FieldAttr::None => match last_segment_ident(field_ty).as_deref() {
                Some("Vec") => {
                    return Err(syn::Error::new(
                        field_ty.span(),
                        "a Vec<T> field requires #[rc(prefixed_array = \"VarInt\")] — Vec has no default wire encoding",
                    ));
                }
                Some("Option") => {
                    return Err(syn::Error::new(
                        field_ty.span(),
                        "Option<T> fields are not supported by #[derive(RcPacket)] yet",
                    ));
                }
                _ => (
                    quote! { rc_protocol::WireWrite::write_wire(&self.#field_name, buf); },
                    quote! { let #field_name = <#field_ty as rc_protocol::WireRead>::read_wire(buf)?; },
                ),
            },
        };

        encode_stmts.push(encode_stmt);
        decode_stmts.push(decode_stmt);
        field_names.push(field_name);
    }

    let PacketAttr {
        state_variant,
        bound_variant,
        id,
    } = packet_attr;

    Ok(quote! {
        impl rc_protocol::RcPacket for #struct_name {
            const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::#state_variant;
            const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::#bound_variant;
            const ID: i32 = #id;

            fn encode_body(&self, buf: &mut rc_protocol::BytesMut) {
                #(#encode_stmts)*
            }

            fn decode_body(buf: &mut rc_protocol::Bytes) -> Result<Self, rc_protocol::PacketDecodeError> {
                #(#decode_stmts)*
                Ok(Self { #(#field_names,)* })
            }
        }
    })
}

/// Parses the `#[packet(state = "...", bound = "...", id = ...)]` container attribute into
/// its resolved `ConnectionState`/`PacketBound` variant identifiers and the `id` literal.
fn parse_packet_attr(input: &syn::DeriveInput) -> syn::Result<PacketAttr> {
    let attr = input.attrs.iter().find(|a| a.path().is_ident("packet")).ok_or_else(|| {
        syn::Error::new(
            input.ident.span(),
            "#[derive(RcPacket)] requires a #[packet(state = \"...\", bound = \"...\", id = ...)] container attribute",
        )
    })?;

    let mut state: Option<syn::LitStr> = None;
    let mut bound: Option<syn::LitStr> = None;
    let mut id: Option<syn::LitInt> = None;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("state") {
            let value = meta.value()?;
            state = Some(value.parse()?);
            Ok(())
        } else if meta.path.is_ident("bound") {
            let value = meta.value()?;
            bound = Some(value.parse()?);
            Ok(())
        } else if meta.path.is_ident("id") {
            let value = meta.value()?;
            id = Some(value.parse()?);
            Ok(())
        } else {
            Err(meta.error("unrecognized #[packet(...)] key; expected state, bound, or id"))
        }
    })?;

    let state =
        state.ok_or_else(|| syn::Error::new_spanned(attr, "#[packet(...)] is missing `state`"))?;
    let bound =
        bound.ok_or_else(|| syn::Error::new_spanned(attr, "#[packet(...)] is missing `bound`"))?;
    let id = id.ok_or_else(|| syn::Error::new_spanned(attr, "#[packet(...)] is missing `id`"))?;

    let state_variant = match state.value().as_str() {
        "handshake" => syn::Ident::new("Handshake", state.span()),
        "status" => syn::Ident::new("Status", state.span()),
        "login" => syn::Ident::new("Login", state.span()),
        "configuration" => syn::Ident::new("Configuration", state.span()),
        "play" => syn::Ident::new("Play", state.span()),
        other => {
            return Err(syn::Error::new(
                state.span(),
                format!(
                    "unknown packet state {other:?}; expected one of \"handshake\", \"status\", \"login\", \"configuration\", \"play\""
                ),
            ));
        }
    };
    let bound_variant = match bound.value().as_str() {
        "server" => syn::Ident::new("Serverbound", bound.span()),
        "client" => syn::Ident::new("Clientbound", bound.span()),
        other => {
            return Err(syn::Error::new(
                bound.span(),
                format!("unknown packet bound {other:?}; expected \"server\" or \"client\""),
            ));
        }
    };

    Ok(PacketAttr {
        state_variant,
        bound_variant,
        id,
    })
}

/// Parses a field's (at most one) `#[rc(...)]` attribute into a `FieldAttr`.
fn parse_field_attr(field: &syn::Field) -> syn::Result<FieldAttr> {
    let mut result = FieldAttr::None;
    for attr in &field.attrs {
        if !attr.path().is_ident("rc") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("varint") {
                result = FieldAttr::Varint;
                Ok(())
            } else if meta.path.is_ident("nbt") {
                result = FieldAttr::Nbt;
                Ok(())
            } else if meta.path.is_ident("prefixed_array") {
                let value = meta.value()?;
                let lit: syn::LitStr = value.parse()?;
                result = FieldAttr::PrefixedArray(lit);
                Ok(())
            } else {
                Err(meta.error(
                    "unrecognized #[rc(...)] attribute; expected varint, prefixed_array = \"...\", or nbt",
                ))
            }
        })?;
    }
    Ok(result)
}

/// A field type's "last path segment identifier," extracted textually (e.g. both `Vec<u8>`
/// and `std::vec::Vec<u8>` yield `"Vec"`; both `VarInt` and `rc_protocol::VarInt` yield
/// `"VarInt"`) — a simple, robust heuristic sufficient for this blueprint's own closed set
/// of special cases, not full type resolution (which a proc macro cannot do reliably
/// regardless).
fn last_segment_ident(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string()),
        _ => None,
    }
}
