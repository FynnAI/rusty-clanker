//! M2-B02 Acceptance test: the one real-vanilla-sample compatibility check, `#[ignore]`d
//! pending `rc-test-harness` (TEST-D7) actually producing a fresh world save (Context's
//! "Compatibility-check strategy against vanilla-produced NBT samples").

#[ignore = "requires a vanilla-produced level.dat sample from rc-test-harness (TEST-D7), not yet implemented — see issues/2"]
#[test]
fn decodes_real_vanilla_level_dat_without_error() {
    let path = std::path::Path::new("oracle/26.2/harness/samples/level.dat");
    let bytes = std::fs::read(path).expect("sample not present — see #[ignore] reason");
    let nbt =
        rc_nbt::read_gzip_owned(&bytes).expect("must decode a real vanilla level.dat cleanly");
    let root = match nbt {
        rc_nbt::owned::Nbt::Some(base) => base,
        rc_nbt::owned::Nbt::None => panic!("level.dat must not be an empty document"),
    };
    let data = root
        .compound("Data")
        .expect("level.dat root must contain a Data compound");
    assert!(
        data.contains("DataVersion"),
        "Data compound must contain DataVersion"
    );
    assert_eq!(
        data.int("DataVersion"),
        Some(4903),
        "sample must be the pinned DataVersion (WORLD-D16)"
    );
}
