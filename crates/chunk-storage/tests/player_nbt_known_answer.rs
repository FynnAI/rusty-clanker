//! M2-B06 Acceptance tests: hand-derived byte vectors proving `PlayerAbilities` and
//! `ItemStackRecord`'s own NBT encoding matches the exact wire shape byte-for-byte —
//! in particular that `components: None` never emits an empty `Compound` tag.

use rc_chunk_storage::{ItemStackRecord, PlayerAbilities};
use rc_nbt::owned;

#[test]
fn abilities_compound_known_bytes() {
    let abilities = PlayerAbilities {
        flying: true,
        fly_speed: 0.5,
        instabuild: false,
        invulnerable: true,
        may_build: false,
        may_fly: true,
        walk_speed: 1.0,
    };

    let compound = owned::NbtCompound::from_values(vec![
        ("flying".into(), owned::NbtTag::Byte(abilities.flying as i8)),
        ("flySpeed".into(), owned::NbtTag::Float(abilities.fly_speed)),
        (
            "instabuild".into(),
            owned::NbtTag::Byte(abilities.instabuild as i8),
        ),
        (
            "invulnerable".into(),
            owned::NbtTag::Byte(abilities.invulnerable as i8),
        ),
        (
            "mayBuild".into(),
            owned::NbtTag::Byte(abilities.may_build as i8),
        ),
        (
            "mayfly".into(),
            owned::NbtTag::Byte(abilities.may_fly as i8),
        ),
        (
            "walkSpeed".into(),
            owned::NbtTag::Float(abilities.walk_speed),
        ),
    ]);
    let root = owned::BaseNbt::new("", compound);

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        0x0A, 0x00, 0x00,
        0x01, 0x00, 0x06, 0x66, 0x6C, 0x79, 0x69, 0x6E, 0x67, 0x01,
        0x05, 0x00, 0x08, 0x66, 0x6C, 0x79, 0x53, 0x70, 0x65, 0x65, 0x64, 0x3F, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x0A, 0x69, 0x6E, 0x73, 0x74, 0x61, 0x62, 0x75, 0x69, 0x6C, 0x64, 0x00,
        0x01, 0x00, 0x0C, 0x69, 0x6E, 0x76, 0x75, 0x6C, 0x6E, 0x65, 0x72, 0x61, 0x62, 0x6C, 0x65, 0x01,
        0x01, 0x00, 0x08, 0x6D, 0x61, 0x79, 0x42, 0x75, 0x69, 0x6C, 0x64, 0x00,
        0x01, 0x00, 0x06, 0x6D, 0x61, 0x79, 0x66, 0x6C, 0x79, 0x01,
        0x05, 0x00, 0x09, 0x77, 0x61, 0x6C, 0x6B, 0x53, 0x70, 0x65, 0x65, 0x64, 0x3F, 0x80, 0x00, 0x00,
        0x00,
    ];

    assert_eq!(rc_nbt::write_owned(&root), expected);
}

#[test]
fn item_stack_no_components_known_bytes() {
    let item = ItemStackRecord {
        id: "minecraft:stick".into(),
        count: 3,
        components: None,
    };
    assert!(item.components.is_none());

    let compound = owned::NbtCompound::from_values(vec![
        ("id".into(), owned::NbtTag::String(item.id.as_str().into())),
        ("count".into(), owned::NbtTag::Int(item.count)),
    ]);
    let root = owned::BaseNbt::new("", compound);

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        0x0A, 0x00, 0x00,
        0x08, 0x00, 0x02, 0x69, 0x64, 0x00, 0x0F, 0x6D, 0x69, 0x6E, 0x65, 0x63, 0x72, 0x61, 0x66, 0x74, 0x3A, 0x73, 0x74, 0x69, 0x63, 0x6B,
        0x03, 0x00, 0x05, 0x63, 0x6F, 0x75, 0x6E, 0x74, 0x00, 0x00, 0x00, 0x03,
        0x00,
    ];

    assert_eq!(rc_nbt::write_owned(&root), expected);
}
