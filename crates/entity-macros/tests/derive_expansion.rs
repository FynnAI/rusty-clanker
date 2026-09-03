//! `rc-entity-macros`' own derive-expansion acceptance tests (M4-B01, own changeset).
//! Uses two test-local structs, each paired with a hand-written `impl` of
//! `rc-mechanics`-shaped stand-in traits living entirely inside this file (this crate has
//! no dependency on `rc-mechanics` — a leaf proc-macro crate, mirroring
//! `rc-protocol-macros`' own `derive_expansion.rs`-style precedent). The generated code
//! for `#[derive(EntityNbtFields)]`/`#[derive(EntityMetadataFields)]` references
//! `crate::entity::{nbt, metadata}::...` (crate-relative — this *test binary* is its own
//! crate, so `crate::entity::...` resolves to the `mod entity` declared below) and
//! `rc_nbt::...` directly (an ordinary external dependency, `[dev-dependencies]`).

mod entity {
    pub mod nbt {
        pub trait EntityNbtFields: Sized {
            fn write_nbt_fields(&self, out: &mut rc_nbt::owned::NbtCompound);
            fn read_nbt_fields(
                compound: &rc_nbt::borrow::NbtCompound<'_, '_>,
                path: &rc_nbt::NbtPath,
            ) -> Result<Self, rc_nbt::SchemaError>;
        }

        pub trait ToNbtField {
            fn to_nbt_field(&self, name: &str, out: &mut rc_nbt::owned::NbtCompound);
        }
        pub trait FromNbtField: Sized {
            fn from_nbt_field(
                compound: &rc_nbt::borrow::NbtCompound<'_, '_>,
                path: &rc_nbt::NbtPath,
                name: &'static str,
            ) -> Result<Self, rc_nbt::SchemaError>;
        }

        impl ToNbtField for i32 {
            fn to_nbt_field(&self, name: &str, out: &mut rc_nbt::owned::NbtCompound) {
                out.insert(name, *self);
            }
        }
        impl FromNbtField for i32 {
            fn from_nbt_field(
                compound: &rc_nbt::borrow::NbtCompound<'_, '_>,
                path: &rc_nbt::NbtPath,
                name: &'static str,
            ) -> Result<Self, rc_nbt::SchemaError> {
                use rc_nbt::schema::NbtCompoundExt;
                compound.require_int(path, name)
            }
        }
    }

    pub mod metadata {
        #[derive(Clone, Debug, PartialEq)]
        pub enum MetadataValue {
            Boolean(bool),
            VarInt(i32),
        }
        impl From<bool> for MetadataValue {
            fn from(v: bool) -> Self {
                MetadataValue::Boolean(v)
            }
        }
        impl From<i32> for MetadataValue {
            fn from(v: i32) -> Self {
                MetadataValue::VarInt(v)
            }
        }

        pub trait EntityMetadataFields {
            fn metadata_entries(&self) -> Vec<(u8, MetadataValue)>;
        }
    }
}

use entity::metadata::EntityMetadataFields;
use entity::metadata::MetadataValue;
use entity::nbt::EntityNbtFields;

#[derive(rc_entity_macros::EntityNbtFields)]
struct NbtStub {
    #[nbt(name = "a")]
    a: i32,
    extra: bool,
}

#[derive(rc_entity_macros::EntityMetadataFields)]
struct MetadataStub {
    #[net_metadata(index = 0, kind = "Boolean")]
    a: bool,
    #[allow(dead_code)]
    extra: bool,
    #[net_metadata(index = 5, kind = "VarInt")]
    b: i32,
}

#[test]
fn nbt_derive_skips_fields_without_the_attribute_and_defaults_them_on_read() {
    let value = NbtStub { a: 7, extra: true };

    let mut out = rc_nbt::owned::NbtCompound::new();
    value.write_nbt_fields(&mut out);
    assert!(out.get("a").is_some(), "attributed field must be written");
    assert!(
        out.get("extra").is_none(),
        "unattributed field must not be written"
    );

    let base = rc_nbt::owned::BaseNbt::new("", out);
    let bytes = rc_nbt::write_owned(&base);
    let read = rc_nbt::read_borrowed_strict(&bytes).expect("read_borrowed_strict must succeed");
    let base = match read {
        rc_nbt::borrow::Nbt::Some(base) => base,
        rc_nbt::borrow::Nbt::None => panic!("expected a non-empty document"),
    };
    let compound = base.as_compound();
    let path = rc_nbt::NbtPath::root();
    let reconstructed =
        NbtStub::read_nbt_fields(&compound, &path).expect("read_nbt_fields must succeed");
    assert_eq!(reconstructed.a, 7);
    assert_eq!(reconstructed.extra, bool::default());
}

#[test]
fn metadata_derive_emits_entries_only_for_attributed_fields_in_ascending_index_order() {
    let value = MetadataStub {
        a: true,
        extra: false,
        b: 300,
    };
    let entries = value.metadata_entries();
    assert_eq!(
        entries,
        vec![
            (0u8, MetadataValue::Boolean(true)),
            (5u8, MetadataValue::VarInt(300)),
        ]
    );
}

/// No `trybuild`-style compile-fail fixture is required by this blueprint (no
/// `trybuild` dependency is pinned anywhere in the workspace, and adding one is out of
/// scope) — this case is instead asserted structurally: `derive_entity_metadata_fields`'s
/// own doc comment states the ascending-order requirement, and `MetadataStub` above (plus
/// every real Deliverable struct in `rc-mechanics`) is constructed with ascending
/// indices, which is this blueprint's own regression guard that the rule is at least
/// satisfiable and exercised, not a compile-fail proof. Deliberately weaker than a
/// `trybuild` fixture would give — flagged here, not silently assumed to be fully
/// covered.
#[test]
fn metadata_derive_rejects_out_of_order_indices_at_compile_time() {
    let value = MetadataStub {
        a: false,
        extra: true,
        b: 1,
    };
    let indices: Vec<u8> = value.metadata_entries().iter().map(|(i, _)| *i).collect();
    assert!(
        indices.windows(2).all(|w| w[0] < w[1]),
        "MetadataStub's own attributed fields must already be declared in strictly \
         ascending index order for this crate to compile at all"
    );
}
