//! Hand-computed bit-packing vectors (M2-B01 Deliverables, `bits.rs`).

use rc_chunk_storage::{ceil_log2, pack_bits, read_slot, unpack_bits, write_slot};

#[test]
fn pack_single_word_4_bits() {
    let packed = pack_bits(&[1, 2, 3], 4);
    assert_eq!(packed.len(), 1);
    assert_eq!(packed[0], 0x321);
}

#[test]
fn pack_zero_bits_is_empty() {
    assert_eq!(pack_bits(&[0, 0, 0], 0).len(), 0);
}

#[test]
fn pack_non_spanning_boundary_at_5_bits() {
    // entries_per_long = 64 / 5 = 12: 13 values need a second word, proving
    // non-spanning packing (a spanning packer could otherwise cram the 13th value's
    // remaining bits into word 0's unused high bits, since 12 * 5 = 60 <= 64).
    let values = [1u32; 13];
    let packed = pack_bits(&values, 5);
    assert_eq!(packed.len(), 2);
    assert_eq!(packed[0], 0x84210842108421u64);
    assert_eq!(packed[1], 1u64);
}

#[test]
fn unpack_inverts_pack_for_arbitrary_values() {
    let packed = pack_bits(&[7, 0, 5, 3], 3);
    assert_eq!(unpack_bits(&packed, 3, 4), vec![7, 0, 5, 3]);
}

#[test]
fn unpack_zero_bits_returns_zeros_without_reading_data() {
    assert_eq!(unpack_bits(&[], 0, 10), vec![0u32; 10]);
}

#[test]
fn read_write_slot_round_trip() {
    let mut data = pack_bits(&[0u32; 20], 6);
    write_slot(&mut data, 13, 42, 6);
    assert_eq!(read_slot(&data, 13, 6), 42);
    for i in 0..20 {
        if i != 13 {
            assert_eq!(read_slot(&data, i, 6), 0, "index {i} should still read 0");
        }
    }
}

#[test]
fn ceil_log2_known_values() {
    let cases: &[(u32, u32)] = &[
        (0, 0),
        (1, 0),
        (2, 1),
        (3, 2),
        (4, 2),
        (16, 4),
        (17, 5),
        (256, 8),
        (257, 9),
        (32366, 15),
    ];
    for &(n, expected) in cases {
        assert_eq!(ceil_log2(n), expected, "ceil_log2({n})");
    }
}
