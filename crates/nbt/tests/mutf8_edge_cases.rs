//! M2-B02 Acceptance tests: the two MUTF-8 encoding rules that differ from standard
//! UTF-8 (Context, "String encoding — the modified-UTF-8 (MUTF-8) caveat"), plus
//! lossless round-tripping and non-panicking decode of already-malformed bytes.

use rc_nbt::{Mutf8Str, Mutf8String, owned};

#[test]
fn nul_encodes_as_overlong_two_byte_sequence() {
    assert_eq!(Mutf8Str::from_str("\u{0}").as_bytes(), &[0xC0, 0x80]);
}

#[test]
fn supplementary_plane_encodes_as_surrogate_pair() {
    assert_eq!(
        Mutf8Str::from_str("\u{10000}").as_bytes(),
        &[0xED, 0xA0, 0x80, 0xED, 0xB0, 0x80]
    );
    assert_eq!(
        Mutf8Str::from_slice(&[0xED, 0xA0, 0x80, 0xED, 0xB0, 0x80])
            .to_str()
            .as_ref(),
        "\u{10000}"
    );
}

#[test]
fn nul_string_round_trips_through_full_write_read_cycle() {
    let compound = owned::NbtCompound::from_values(vec![(
        "n".into(),
        owned::NbtTag::String(Mutf8String::from("\u{0}")),
    )]);
    let root = owned::BaseNbt::new("", compound);

    let bytes = rc_nbt::write_owned(&root);
    let decoded = rc_nbt::read_owned(&bytes).unwrap();
    match decoded {
        owned::Nbt::Some(base) => {
            assert_eq!(base.string("n").unwrap().as_bytes(), &[0xC0, 0x80]);
        }
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn malformed_string_bytes_round_trip_without_corruption() {
    let compound = owned::NbtCompound::from_values(vec![(
        "n".into(),
        owned::NbtTag::String(Mutf8String::from_vec(vec![0xFF, 0xFE])),
    )]);
    let root = owned::BaseNbt::new("", compound);

    let bytes = rc_nbt::write_owned(&root);
    let decoded = rc_nbt::read_owned(&bytes).unwrap();
    match decoded {
        owned::Nbt::Some(base) => {
            assert_eq!(base.string("n").unwrap().as_bytes(), &[0xFF, 0xFE]);
        }
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn malformed_string_to_str_never_panics() {
    let malformed = Mutf8String::from_vec(vec![0xFF, 0xFE]);
    assert!(malformed.to_str().is_empty());
}
