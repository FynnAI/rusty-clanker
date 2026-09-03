//! M4-B07 — light-section nibble-array helper acceptance tests (Context §10/§11):
//! index/coordinate math, single-nibble read/write, and cross-region face
//! extraction/injection.

use proptest::prelude::*;
use rc_chunk_storage::LightNibbles;
use rc_mechanics::direction::Direction;
use rc_mechanics::light::{
    extract_face, extract_face_from_nibbles, get_nibble, inject_face, light_nibble_index,
    light_section_index_for_y, nibble_at, set_nibble, uniform_array, uniform_face,
};

#[test]
fn nibble_get_set_round_trip() {
    let mut data = [0u8; 2048];
    set_nibble(&mut data, 0, 0xA);
    set_nibble(&mut data, 1, 0xB);
    set_nibble(&mut data, 4095, 0xF);

    assert_eq!(get_nibble(&data, 0), 0xA);
    assert_eq!(get_nibble(&data, 1), 0xB);
    assert_eq!(get_nibble(&data, 4095), 0xF);

    for i in 2..4095 {
        assert_eq!(get_nibble(&data, i), 0, "index {i} was corrupted");
    }
}

#[test]
fn light_nibble_index_matches_block_index_formula() {
    let samples: [(u8, u8, u8); 5] = [(0, 0, 0), (1, 0, 0), (0, 0, 1), (0, 1, 0), (15, 15, 15)];
    for (x, local_y, z) in samples {
        assert_eq!(
            light_nibble_index(x, local_y, z),
            rc_chunk_storage::block_index(x, local_y, z),
            "mismatch at (x={x}, local_y={local_y}, z={z})"
        );
    }
}

#[test]
fn light_section_index_for_y_padding_boundaries() {
    assert_eq!(light_section_index_for_y(-80), 0);
    assert_eq!(light_section_index_for_y(-65), 0);
    assert_eq!(light_section_index_for_y(-64), 1);
    assert_eq!(light_section_index_for_y(319), 24);
    assert_eq!(light_section_index_for_y(320), 25);
    assert_eq!(light_section_index_for_y(335), 25);
}

#[test]
#[should_panic]
fn light_section_index_for_y_panics_above_range() {
    let _ = light_section_index_for_y(336);
}

#[test]
#[should_panic]
fn light_section_index_for_y_panics_below_range() {
    let _ = light_section_index_for_y(-81);
}

#[test]
fn extract_face_west_matches_hand_computed_values() {
    let mut data = [0u8; 2048];
    for local_y in 0u8..16 {
        for z in 0u8..16 {
            let value = (local_y + z) % 16;
            set_nibble(&mut data, light_nibble_index(0, local_y, z), value);
        }
    }
    let face = extract_face(&data, Direction::West);
    // byte[0] = pack of local_y=0,z=0 -> 0 and local_y=0,z=1 -> 1.
    assert_eq!(face[0], 0x10);
    // A couple more early bytes, hand-derived the same way.
    assert_eq!(face[1], 0x32); // local_y=0,z=2->2 (low), local_y=0,z=3->3 (high)
    assert_eq!(face[2], 0x54); // local_y=0,z=4->4 (low), local_y=0,z=5->5 (high)
    // byte[127] = pack of local_y=15,z=14 -> 13 and local_y=15,z=15 -> 14.
    assert_eq!(face[127], 0xED);
    // byte[126] = pack of local_y=15,z=12 -> 11 (low, 0xB) and local_y=15,z=13 -> 12 (high, 0xC).
    assert_eq!(face[126], 0xCB);
}

proptest! {
    #[test]
    fn extract_then_inject_face_round_trips(bytes in proptest::collection::vec(any::<u8>(), 2048)) {
        let mut data = [0u8; 2048];
        data.copy_from_slice(&bytes);

        for dir in [Direction::West, Direction::East, Direction::North, Direction::South] {
            let face = extract_face(&data, dir);
            let mut data2 = [0u8; 2048];
            inject_face(&mut data2, dir, &face);

            // The 256 face-local indices `inject_face` is allowed to have touched, for
            // this direction.
            let mut face_indices = std::collections::HashSet::new();
            for local_y in 0u8..16 {
                for perp in 0u8..16 {
                    let (x, z) = match dir {
                        Direction::West => (0u8, perp),
                        Direction::East => (15u8, perp),
                        Direction::North => (perp, 0u8),
                        Direction::South => (perp, 15u8),
                        _ => unreachable!(),
                    };
                    face_indices.insert(light_nibble_index(x, local_y, z));
                }
            }
            prop_assert_eq!(face_indices.len(), 256);

            for idx in 0..4096usize {
                if face_indices.contains(&idx) {
                    prop_assert_eq!(get_nibble(&data2, idx), get_nibble(&data, idx));
                } else {
                    prop_assert_eq!(get_nibble(&data2, idx), 0);
                }
            }
        }
    }
}

#[test]
fn nibble_at_handles_all_light_nibbles_variants() {
    assert_eq!(nibble_at(&LightNibbles::Uninitialized, 0), 0);
    assert_eq!(nibble_at(&LightNibbles::Uninitialized, 4095), 0);

    assert_eq!(nibble_at(&LightNibbles::Filled(7), 0), 7);
    assert_eq!(nibble_at(&LightNibbles::Filled(7), 4095), 7);

    let mut arr = [0u8; 2048];
    set_nibble(&mut arr, 0, 0xA);
    let nibbles = LightNibbles::Data(Box::new(arr));
    assert_eq!(nibble_at(&nibbles, 0), 0xA);
    assert_eq!(nibble_at(&nibbles, 1), 0);
}

#[test]
fn uniform_array_and_uniform_face_pack_every_nibble() {
    let arr = uniform_array(9);
    for i in [0usize, 1, 4095] {
        assert_eq!(get_nibble(&arr, i), 9);
    }
    assert_eq!(uniform_face(9), [0x99u8; 128]);
}

#[test]
fn extract_face_from_nibbles_dispatches_per_variant() {
    assert_eq!(
        extract_face_from_nibbles(&LightNibbles::Uninitialized, Direction::West),
        None
    );
    assert_eq!(
        extract_face_from_nibbles(&LightNibbles::Filled(5), Direction::West),
        Some(uniform_face(5))
    );

    let mut arr = [0u8; 2048];
    for local_y in 0u8..16 {
        for z in 0u8..16 {
            let value = (local_y + z) % 16;
            set_nibble(&mut arr, light_nibble_index(0, local_y, z), value);
        }
    }
    let expected = extract_face(&arr, Direction::West);
    assert_eq!(
        extract_face_from_nibbles(&LightNibbles::Data(Box::new(arr)), Direction::West),
        Some(expected)
    );
}
