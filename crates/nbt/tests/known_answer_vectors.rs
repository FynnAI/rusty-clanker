//! M2-B02 Acceptance tests: thirteen known-answer byte vectors, one per NBT tag type
//! plus the empty-compound case, each hand-derived from the vanilla NBT binary format
//! restated in the blueprint's Context (tag IDs, big-endian numeric encoding, the
//! `[count: i32 BE][elements]` list/array framing). Every case asserts both directions:
//! (a) `write_owned` on the hand-constructed value produces exactly the given bytes;
//! (b) `read_borrowed` and `read_owned` on those exact bytes decode back to a document
//! whose field matches the original value via the appropriate typed accessor.

use rc_nbt::{borrow, owned};

#[test]
fn byte_tag() {
    let bytes: Vec<u8> = vec![0x0A, 0x00, 0x00, 0x01, 0x00, 0x01, 0x62, 0xFF, 0x00];
    let compound = owned::NbtCompound::from_values(vec![("b".into(), owned::NbtTag::Byte(-1))]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => assert_eq!(base.as_compound().byte("b"), Some(-1)),
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => assert_eq!(base.byte("b"), Some(-1)),
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn short_tag() {
    let bytes: Vec<u8> = vec![0x0A, 0x00, 0x00, 0x02, 0x00, 0x01, 0x73, 0xCF, 0xC7, 0x00];
    let compound =
        owned::NbtCompound::from_values(vec![("s".into(), owned::NbtTag::Short(-12345))]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => assert_eq!(base.as_compound().short("s"), Some(-12345)),
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => assert_eq!(base.short("s"), Some(-12345)),
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn int_tag() {
    let bytes: Vec<u8> = vec![
        0x0A, 0x00, 0x00, 0x03, 0x00, 0x01, 0x69, 0xFF, 0xFF, 0xFF, 0xFF, 0x00,
    ];
    let compound = owned::NbtCompound::from_values(vec![("i".into(), owned::NbtTag::Int(-1))]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => assert_eq!(base.as_compound().int("i"), Some(-1)),
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => assert_eq!(base.int("i"), Some(-1)),
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn long_tag() {
    let bytes: Vec<u8> = vec![
        0x0A, 0x00, 0x00, 0x04, 0x00, 0x01, 0x6C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x00,
    ];
    let compound = owned::NbtCompound::from_values(vec![("l".into(), owned::NbtTag::Long(1))]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => assert_eq!(base.as_compound().long("l"), Some(1)),
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => assert_eq!(base.long("l"), Some(1)),
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn float_tag() {
    let bytes: Vec<u8> = vec![
        0x0A, 0x00, 0x00, 0x05, 0x00, 0x01, 0x66, 0x3F, 0x80, 0x00, 0x00, 0x00,
    ];
    let compound = owned::NbtCompound::from_values(vec![("f".into(), owned::NbtTag::Float(1.0))]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => assert_eq!(base.as_compound().float("f"), Some(1.0)),
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => assert_eq!(base.float("f"), Some(1.0)),
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn double_tag() {
    let bytes: Vec<u8> = vec![
        0x0A, 0x00, 0x00, 0x06, 0x00, 0x01, 0x64, 0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    let compound = owned::NbtCompound::from_values(vec![("d".into(), owned::NbtTag::Double(1.0))]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => assert_eq!(base.as_compound().double("d"), Some(1.0)),
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => assert_eq!(base.double("d"), Some(1.0)),
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn byte_array_tag() {
    let bytes: Vec<u8> = vec![
        0x0A, 0x00, 0x00, 0x07, 0x00, 0x02, 0x62, 0x61, 0x00, 0x00, 0x00, 0x03, 0x01, 0x02, 0x03,
        0x00,
    ];
    let compound = owned::NbtCompound::from_values(vec![(
        "ba".into(),
        owned::NbtTag::ByteArray(vec![1, 2, 3]),
    )]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => {
            assert_eq!(base.as_compound().byte_array("ba"), Some(&[1u8, 2, 3][..]))
        }
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => assert_eq!(base.byte_array("ba"), Some(&[1u8, 2, 3][..])),
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn string_tag() {
    let bytes: Vec<u8> = vec![
        0x0A, 0x00, 0x00, 0x08, 0x00, 0x02, 0x73, 0x74, 0x00, 0x02, 0x68, 0x69, 0x00,
    ];
    let compound = owned::NbtCompound::from_values(vec![(
        "st".into(),
        owned::NbtTag::String(rc_nbt::Mutf8String::from("hi")),
    )]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => {
            assert_eq!(base.as_compound().string("st").unwrap().to_str(), "hi");
        }
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => {
            assert_eq!(base.string("st").unwrap().to_str(), "hi");
        }
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn list_tag() {
    let bytes: Vec<u8> = vec![
        0x0A, 0x00, 0x00, 0x09, 0x00, 0x02, 0x6C, 0x69, 0x01, 0x00, 0x00, 0x00, 0x02, 0x07, 0x08,
        0x00,
    ];
    let compound = owned::NbtCompound::from_values(vec![(
        "li".into(),
        owned::NbtTag::List(owned::NbtList::Byte(vec![7, 8])),
    )]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => {
            let list = base.as_compound().list("li").unwrap();
            assert_eq!(list.bytes(), Some(&[7i8, 8][..]));
        }
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => {
            assert_eq!(base.list("li"), Some(&owned::NbtList::Byte(vec![7, 8])));
        }
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn compound_tag() {
    let bytes: Vec<u8> = vec![
        0x0A, 0x00, 0x00, 0x0A, 0x00, 0x01, 0x63, 0x01, 0x00, 0x01, 0x78, 0x09, 0x00, 0x00,
    ];
    let inner = owned::NbtCompound::from_values(vec![("x".into(), owned::NbtTag::Byte(9))]);
    let compound = owned::NbtCompound::from_values(vec![("c".into(), owned::NbtTag::Compound(inner))]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => {
            let inner = base.as_compound().compound("c").unwrap();
            assert_eq!(inner.byte("x"), Some(9));
        }
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => {
            let inner = base.compound("c").unwrap();
            assert_eq!(inner.byte("x"), Some(9));
        }
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn int_array_tag() {
    let bytes: Vec<u8> = vec![
        0x0A, 0x00, 0x00, 0x0B, 0x00, 0x02, 0x69, 0x61, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x02, 0x00,
    ];
    let compound =
        owned::NbtCompound::from_values(vec![("ia".into(), owned::NbtTag::IntArray(vec![1, 2]))]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => {
            assert_eq!(base.as_compound().int_array("ia"), Some(vec![1, 2]))
        }
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => assert_eq!(base.int_array("ia"), Some(&[1, 2][..])),
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn long_array_tag() {
    let bytes: Vec<u8> = vec![
        0x0A, 0x00, 0x00, 0x0C, 0x00, 0x02, 0x6C, 0x61, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
    ];
    let compound =
        owned::NbtCompound::from_values(vec![("la".into(), owned::NbtTag::LongArray(vec![1]))]);
    let root = owned::BaseNbt::new("", compound);

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => {
            assert_eq!(base.as_compound().long_array("la"), Some(vec![1]))
        }
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => assert_eq!(base.long_array("la"), Some(&[1][..])),
        owned::Nbt::None => panic!("expected Nbt::Some"),
    }
}

#[test]
fn empty_compound() {
    let bytes: Vec<u8> = vec![0x0A, 0x00, 0x00, 0x00];
    let root = owned::BaseNbt::new("", owned::NbtCompound::new());

    assert_eq!(rc_nbt::write_owned(&root), bytes);

    match rc_nbt::read_borrowed(&bytes).unwrap() {
        borrow::Nbt::Some(base) => assert_eq!(base.as_compound().len(), 0),
        borrow::Nbt::None => panic!("expected Nbt::Some (a valid, non-None, empty document)"),
    }
    match rc_nbt::read_owned(&bytes).unwrap() {
        owned::Nbt::Some(base) => assert_eq!(base.len(), 0),
        owned::Nbt::None => panic!("expected Nbt::Some (a valid, non-None, empty document)"),
    }
}
