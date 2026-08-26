//! Oracle-compatibility check against a real vanilla-produced chunk (TEST-D47, honest
//! gap -- restated from M2-B02's own `oracle_compatibility.rs` precedent, Context).

#[ignore = "requires a vanilla-produced chunk NBT sample from rc-test-harness (TEST-D7), not yet implemented — see the M2-B08 acceptance-harness blueprint for when this is wired up"]
#[test]
fn decodes_a_real_vanilla_full_chunk_without_error() {
    // Path convention matches rc-nbt's own oracle_compatibility.rs precedent (M2-B02):
    // oracle/26.2/harness/samples/region/r.0.0.mca, read + decompressed by this test's
    // own minimal inline zlib-unwrap (this blueprint does not depend on rc-anvil), then
    // decoded via rc_nbt::read_owned and asserted: DataVersion == 4903, Status ==
    // "minecraft:full", yPos == -4.
    unimplemented!("pending rc-test-harness (TEST-D7)");
}
