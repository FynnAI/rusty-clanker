//! Non-spanning bit-packing primitives shared by `PalettedContainer` (WORLD-D2) and
//! `HeightmapSet` (WORLD-D5) — identical algorithm to `M1-B05`'s own hand-rolled
//! wire-format `pack_bits` (`crates/server/src/play/chunk.rs`), reused here as this
//! crate's own canonical, shared implementation (Context: "Non-spanning bit packing").

/// `ceil(log2(n))` for `n >= 1`; returns `0` for `n <= 1` (both "no bits needed" cases:
/// zero and one distinct value). Exact, allocation-free formula, identical to the one
/// `M1-B05`'s own hand-rolled encoder already uses (`32 - (n - 1).leading_zeros()` for
/// `n >= 2`), reused here so both crates' palette bit-width decisions can never diverge.
pub const fn ceil_log2(n: u32) -> u32 {
    if n <= 1 {
        0
    } else {
        32 - (n - 1).leading_zeros()
    }
}

/// Non-spanning bit-packs `values` into `u64` words (WORLD-D2/WORLD-D5's shared packing
/// primitive — Context). `entries_per_long = 64 / bits_per_entry` values per word, least-
/// significant-bits-first; once a word holds `entries_per_long` values, start a fresh
/// word instead — **never** split one value's bits across two words, even if that leaves
/// unused high bits in the word just filled. `bits_per_entry == 0` (the `SingleValue`
/// state) stores zero words. Panics (`debug_assert!`) if any `value >= 2^bits_per_entry`
/// or if `bits_per_entry > 64`.
pub fn pack_bits(values: &[u32], bits_per_entry: u32) -> Box<[u64]> {
    debug_assert!(
        bits_per_entry <= 64,
        "bits_per_entry {bits_per_entry} exceeds 64"
    );
    if bits_per_entry == 0 {
        return Box::new([]);
    }
    debug_assert!(
        bits_per_entry >= 32
            || values
                .iter()
                .all(|&v| (v as u64) < (1u64 << bits_per_entry)),
        "a value does not fit in {bits_per_entry} bits"
    );

    let entries_per_long = (64 / bits_per_entry) as usize;
    let word_count = values.len().div_ceil(entries_per_long);
    let mut data = vec![0u64; word_count];
    for (i, &v) in values.iter().enumerate() {
        let word = i / entries_per_long;
        let slot = i % entries_per_long;
        let shift = (slot as u32) * bits_per_entry;
        data[word] |= (v as u64) << shift;
    }
    data.into_boxed_slice()
}

/// Inverse of `pack_bits`: unpacks exactly `count` values at `bits_per_entry` from `data`.
/// `data` must hold at least `ceil(count / (64 / bits_per_entry))` words for
/// `bits_per_entry > 0` (debug-asserted); for `bits_per_entry == 0`, returns `count`
/// zeros without reading `data` at all (matches `SingleValue`'s "no data words" shape —
/// callers needing the actual single value read it from the palette, not from this
/// function).
pub fn unpack_bits(data: &[u64], bits_per_entry: u32, count: usize) -> Vec<u32> {
    if bits_per_entry == 0 {
        return vec![0u32; count];
    }
    let entries_per_long = (64 / bits_per_entry) as usize;
    debug_assert!(
        data.len() >= count.div_ceil(entries_per_long),
        "data too short for {count} entries at {bits_per_entry} bits"
    );
    (0..count)
        .map(|i| read_slot(data, i, bits_per_entry))
        .collect()
}

/// Reads one packed slot at `index` (0-based) out of `data`, at `bits_per_entry`. Used
/// internally by `PalettedContainer::get`; exposed publicly since a future persistence
/// blueprint's NBT/wire reader needs the identical single-slot read primitive.
pub fn read_slot(data: &[u64], index: usize, bits_per_entry: u32) -> u32 {
    debug_assert!(
        bits_per_entry > 0 && bits_per_entry <= 64,
        "bits_per_entry {bits_per_entry} out of range"
    );
    let entries_per_long = (64 / bits_per_entry) as usize;
    let word = index / entries_per_long;
    let slot = index % entries_per_long;
    let shift = (slot as u32) * bits_per_entry;
    ((data[word] >> shift) & mask_for(bits_per_entry)) as u32
}

/// Writes one packed slot at `index` in place, without touching any other slot in the
/// same word. `data` must already be sized for at least `index + 1` entries at
/// `bits_per_entry` (debug-asserted).
pub fn write_slot(data: &mut [u64], index: usize, value: u32, bits_per_entry: u32) {
    debug_assert!(
        bits_per_entry > 0 && bits_per_entry <= 64,
        "bits_per_entry {bits_per_entry} out of range"
    );
    let entries_per_long = (64 / bits_per_entry) as usize;
    let word = index / entries_per_long;
    let slot = index % entries_per_long;
    let shift = (slot as u32) * bits_per_entry;
    debug_assert!(
        word < data.len(),
        "index {index} out of range for data length {}",
        data.len()
    );
    let mask = mask_for(bits_per_entry);
    data[word] = (data[word] & !(mask << shift)) | ((value as u64 & mask) << shift);
}

/// The `bits_per_entry`-wide low-bit mask (`u64::MAX` at `bits_per_entry == 64`, where
/// `1u64 << 64` would otherwise overflow-panic).
fn mask_for(bits_per_entry: u32) -> u64 {
    if bits_per_entry >= 64 {
        u64::MAX
    } else {
        (1u64 << bits_per_entry) - 1
    }
}
