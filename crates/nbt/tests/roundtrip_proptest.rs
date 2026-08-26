//! M2-B02 Acceptance tests: round-trip properties over arbitrary NBT compound trees
//! (bounded to max depth 4, max 6 entries per compound — this crate's own testing-scope
//! choice, unrelated to `simdnbt`'s 512-level parse-time depth cap), covering all
//! twelve tag types including nested `Compound` and every `List` element type.

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Union};
use rc_nbt::{Mutf8String, borrow, owned};

const MAX_DEPTH: u32 = 4;
const MAX_ENTRIES: usize = 6;

fn arb_name() -> impl Strategy<Value = Mutf8String> {
    "[a-zA-Z0-9_]{0,8}".prop_map(|s| Mutf8String::from(s.as_str()))
}

fn arb_nbt_tag(depth: u32) -> BoxedStrategy<owned::NbtTag> {
    let leaves = vec![
        any::<i8>().prop_map(owned::NbtTag::Byte).boxed(),
        any::<i16>().prop_map(owned::NbtTag::Short).boxed(),
        any::<i32>().prop_map(owned::NbtTag::Int).boxed(),
        any::<i64>().prop_map(owned::NbtTag::Long).boxed(),
        any::<f32>().prop_map(owned::NbtTag::Float).boxed(),
        any::<f64>().prop_map(owned::NbtTag::Double).boxed(),
        proptest::collection::vec(any::<u8>(), 0..MAX_ENTRIES)
            .prop_map(owned::NbtTag::ByteArray)
            .boxed(),
        arb_name().prop_map(owned::NbtTag::String).boxed(),
        proptest::collection::vec(any::<i32>(), 0..MAX_ENTRIES)
            .prop_map(owned::NbtTag::IntArray)
            .boxed(),
        proptest::collection::vec(any::<i64>(), 0..MAX_ENTRIES)
            .prop_map(owned::NbtTag::LongArray)
            .boxed(),
    ];

    if depth == 0 {
        Union::new(leaves).boxed()
    } else {
        let mut variants = leaves;
        variants.push(
            arb_nbt_list(depth - 1)
                .prop_map(owned::NbtTag::List)
                .boxed(),
        );
        variants.push(
            arb_nbt_compound(depth - 1)
                .prop_map(owned::NbtTag::Compound)
                .boxed(),
        );
        Union::new(variants).boxed()
    }
}

/// A homogeneous `NbtList` — one of the twelve `simdnbt::owned::NbtList` variants
/// (list-of-lists and list-of-compounds only offered once `depth > 0`, matching
/// `arb_nbt_tag`'s own recursion-bound shape).
fn arb_nbt_list(depth: u32) -> BoxedStrategy<owned::NbtList> {
    let mut variants: Vec<BoxedStrategy<owned::NbtList>> = vec![
        Just(owned::NbtList::Empty).boxed(),
        proptest::collection::vec(any::<i8>(), 0..MAX_ENTRIES)
            .prop_map(owned::NbtList::Byte)
            .boxed(),
        proptest::collection::vec(any::<i16>(), 0..MAX_ENTRIES)
            .prop_map(owned::NbtList::Short)
            .boxed(),
        proptest::collection::vec(any::<i32>(), 0..MAX_ENTRIES)
            .prop_map(owned::NbtList::Int)
            .boxed(),
        proptest::collection::vec(any::<i64>(), 0..MAX_ENTRIES)
            .prop_map(owned::NbtList::Long)
            .boxed(),
        proptest::collection::vec(any::<f32>(), 0..MAX_ENTRIES)
            .prop_map(owned::NbtList::Float)
            .boxed(),
        proptest::collection::vec(any::<f64>(), 0..MAX_ENTRIES)
            .prop_map(owned::NbtList::Double)
            .boxed(),
        proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..4), 0..4)
            .prop_map(owned::NbtList::ByteArray)
            .boxed(),
        proptest::collection::vec(arb_name(), 0..MAX_ENTRIES)
            .prop_map(owned::NbtList::String)
            .boxed(),
        proptest::collection::vec(proptest::collection::vec(any::<i32>(), 0..4), 0..4)
            .prop_map(owned::NbtList::IntArray)
            .boxed(),
        proptest::collection::vec(proptest::collection::vec(any::<i64>(), 0..4), 0..4)
            .prop_map(owned::NbtList::LongArray)
            .boxed(),
    ];

    if depth > 0 {
        variants.push(
            proptest::collection::vec(arb_nbt_compound(depth - 1), 0..4)
                .prop_map(owned::NbtList::Compound)
                .boxed(),
        );
        variants.push(
            proptest::collection::vec(arb_nbt_list(depth - 1), 0..4)
                .prop_map(owned::NbtList::List)
                .boxed(),
        );
    }

    Union::new(variants).boxed()
}

fn arb_nbt_compound(depth: u32) -> BoxedStrategy<owned::NbtCompound> {
    proptest::collection::vec((arb_name(), arb_nbt_tag(depth)), 0..MAX_ENTRIES)
        .prop_map(owned::NbtCompound::from_values)
        .boxed()
}

proptest! {
    #[test]
    fn compound_round_trips_through_owned_write_then_owned_read(c in arb_nbt_compound(MAX_DEPTH)) {
        let root = owned::BaseNbt::new("", c);
        let bytes = rc_nbt::write_owned(&root);
        let decoded = rc_nbt::read_owned(&bytes).unwrap();
        match decoded {
            owned::Nbt::Some(base) => prop_assert_eq!(base, root),
            owned::Nbt::None => prop_assert!(false, "expected Nbt::Some"),
        }
    }

    #[test]
    fn compound_round_trips_through_owned_write_then_borrowed_read(c in arb_nbt_compound(MAX_DEPTH)) {
        let root = owned::BaseNbt::new("", c);
        let bytes = rc_nbt::write_owned(&root);
        let decoded = rc_nbt::read_borrowed(&bytes).unwrap();
        match decoded {
            borrow::Nbt::Some(base) => prop_assert_eq!(base.to_owned(), root),
            borrow::Nbt::None => prop_assert!(false, "expected Nbt::Some"),
        }
    }
}
