//! TEST-D26 item (2): "NBT decode — `simdnbt`'s zero-copy borrowed-buffer decode
//! entry point... must never panic, regardless of input." Raw-byte input (not a
//! derived `Arbitrary` struct) — the type under test *is* a byte buffer, per TEST-D26's
//! own "entry point = raw... bytes" framing for this exact fuzz-target class.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Must never panic and must never hang, for any input whatsoever. A successful
    // decode's further round-trip property (decode(encode(x)) == x) is exercised by
    // this crate's proptest suite instead (TEST-D27's own division of labor: raw-byte
    // fuzzing for "never panics on garbage," structured proptest for "round-trips on
    // valid values") — not duplicated here.
    let _ = rc_nbt::read_borrowed(data);
});
