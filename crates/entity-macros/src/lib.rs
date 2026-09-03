//! `rc-entity-macros` — `#[derive(EntityNbtFields)]`/`#[derive(EntityMetadataFields)]`
//! (MECH-D30): per-field `#[nbt(name = "...")]`/`#[net_metadata(index = N, kind = "...")]`
//! attributes, each independently optional, each read by its own derive only. Generated
//! code references `crate::entity::{nbt, metadata}::...` (crate-relative, not
//! `rc_mechanics::...`) — every real use of either derive in this milestone's own
//! Deliverables lives inside `rc-mechanics` itself, mirroring `rc-protocol-macros`'
//! `RcPacket` derive's own already-documented "generated code names the consuming
//! crate's dependency by its literal crate name" limitation (M1-B01), sidestepped here by
//! emitting a crate-relative path instead (M4-B01's own Context).

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

/// See this blueprint's "`#[derive(EntityNbtFields)]` — exact expansion algorithm".
#[proc_macro_derive(EntityNbtFields, attributes(nbt))]
pub fn derive_entity_nbt_fields(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match expand_nbt_fields(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// See this blueprint's "`#[derive(EntityMetadataFields)]` — exact expansion algorithm".
/// Emits a compile error if two `#[net_metadata(index = ...)]` attributes on the same
/// struct are not in strictly ascending numeric order by declaration.
#[proc_macro_derive(EntityMetadataFields, attributes(net_metadata))]
pub fn derive_entity_metadata_fields(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match expand_metadata_fields(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Extracts the `syn::Fields::Named` field list of a plain struct, or a `syn::Error`
/// naming this restriction — both derives share this identical shape requirement.
fn named_fields<'a>(
    input: &'a syn::DeriveInput,
    derive_name: &str,
) -> syn::Result<&'a syn::FieldsNamed> {
    match &input.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(named),
            ..
        }) => Ok(named),
        _ => Err(syn::Error::new(
            input.ident.span(),
            format!("#[derive({derive_name})] only supports a struct with named fields"),
        )),
    }
}

/// The parsed `#[nbt(name = "...")]` field attribute, or its absence.
fn parse_nbt_attr(field: &syn::Field) -> syn::Result<Option<syn::LitStr>> {
    let mut result: Option<syn::LitStr> = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("nbt") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let value = meta.value()?;
                result = Some(value.parse()?);
                Ok(())
            } else {
                Err(meta.error("unrecognized #[nbt(...)] key; expected name"))
            }
        })?;
    }
    Ok(result)
}

fn expand_nbt_fields(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = named_fields(&input, "EntityNbtFields")?;

    let mut write_stmts = Vec::with_capacity(fields.named.len());
    let mut read_stmts = Vec::with_capacity(fields.named.len());
    let mut field_names = Vec::with_capacity(fields.named.len());

    for field in &fields.named {
        let field_name = field
            .ident
            .as_ref()
            .expect("Fields::Named field always has an ident")
            .clone();
        let field_ty = &field.ty;
        let nbt_attr = parse_nbt_attr(field)?;

        match nbt_attr {
            Some(name_lit) => {
                write_stmts.push(quote! {
                    crate::entity::nbt::ToNbtField::to_nbt_field(&self.#field_name, #name_lit, out);
                });
                read_stmts.push(quote! {
                    let #field_name = <#field_ty as crate::entity::nbt::FromNbtField>::from_nbt_field(compound, path, #name_lit)?;
                });
            }
            None => {
                read_stmts.push(quote! {
                    let #field_name = Default::default();
                });
            }
        }

        field_names.push(field_name);
    }

    Ok(quote! {
        impl crate::entity::nbt::EntityNbtFields for #struct_name {
            fn write_nbt_fields(&self, out: &mut rc_nbt::owned::NbtCompound) {
                #(#write_stmts)*
            }

            fn read_nbt_fields(
                compound: &rc_nbt::borrow::NbtCompound<'_, '_>,
                path: &rc_nbt::NbtPath,
            ) -> Result<Self, rc_nbt::SchemaError> {
                #(#read_stmts)*
                Ok(Self { #(#field_names,)* })
            }
        }
    })
}

/// The parsed `#[net_metadata(index = N, kind = "...")]` field attribute, or its absence.
/// `kind` is parsed (so the attribute's own full syntax is validated) but not otherwise
/// consumed by codegen — the concrete `MetadataValue` variant a field encodes to is
/// determined structurally by that field type's own `Into<MetadataValue>` impl, never by
/// this string.
fn parse_net_metadata_attr(field: &syn::Field) -> syn::Result<Option<syn::LitInt>> {
    let mut index: Option<syn::LitInt> = None;
    let mut kind: Option<syn::LitStr> = None;
    let mut found = false;
    for attr in &field.attrs {
        if !attr.path().is_ident("net_metadata") {
            continue;
        }
        found = true;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("index") {
                let value = meta.value()?;
                index = Some(value.parse()?);
                Ok(())
            } else if meta.path.is_ident("kind") {
                let value = meta.value()?;
                kind = Some(value.parse()?);
                Ok(())
            } else {
                Err(meta.error("unrecognized #[net_metadata(...)] key; expected index or kind"))
            }
        })?;
    }
    if !found {
        return Ok(None);
    }
    let index = index
        .ok_or_else(|| syn::Error::new(field.span(), "#[net_metadata(...)] is missing `index`"))?;
    let _ = kind
        .ok_or_else(|| syn::Error::new(field.span(), "#[net_metadata(...)] is missing `kind`"))?;
    Ok(Some(index))
}

fn expand_metadata_fields(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let fields = named_fields(&input, "EntityMetadataFields")?;

    let mut push_stmts = Vec::with_capacity(fields.named.len());
    let mut previous_index: Option<i64> = None;

    for field in &fields.named {
        let field_name = field
            .ident
            .as_ref()
            .expect("Fields::Named field always has an ident")
            .clone();
        let Some(index_lit) = parse_net_metadata_attr(field)? else {
            continue;
        };

        let index_value: i64 = index_lit.base10_parse()?;
        if let Some(previous) = previous_index
            && index_value <= previous
        {
            return Err(syn::Error::new(
                index_lit.span(),
                format!(
                    "net_metadata indices must be declared in ascending order within one \
                     struct (found {index_value} after {previous})"
                ),
            ));
        }
        previous_index = Some(index_value);

        push_stmts.push(quote! {
            entries.push((#index_lit, self.#field_name.clone().into()));
        });
    }

    Ok(quote! {
        impl crate::entity::metadata::EntityMetadataFields for #struct_name {
            fn metadata_entries(&self) -> Vec<(u8, crate::entity::metadata::MetadataValue)> {
                let mut entries = Vec::new();
                #(#push_stmts)*
                entries
            }
        }
    })
}
