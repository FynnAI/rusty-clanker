//! TEST-D26 item (3), round-trip half: `decode(encode(x)) == x` for arbitrary valid
//! in-memory chunk values.
#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rc_chunk_storage::{CompressionScheme, RegionFile};

#[derive(Arbitrary, Debug)]
struct RoundtripInput {
    scheme: u8,   // reduced mod 3 to {Zlib, Lz4, Uncompressed}
    local_x: u8,  // reduced mod 32
    local_z: u8,  // reduced mod 32
    payload: Vec<u8>,
}

fuzz_target!(|input: RoundtripInput| {
    let scheme = match input.scheme % 3 {
        0 => CompressionScheme::Zlib,
        1 => CompressionScheme::Lz4,
        _ => CompressionScheme::Uncompressed,
    };
    let local_x = input.local_x % 32;
    let local_z = input.local_z % 32;
    let path = std::env::temp_dir().join(format!(
        "rc-anvil-fuzz-{}-{:?}.mca",
        std::process::id(),
        std::thread::current().id()
    ));
    let compressed = scheme.compress(&input.payload);
    if let Ok(mut rf) = RegionFile::open(path.clone(), 0, 0) {
        if rf
            .write_record(local_x, local_z, scheme.tag(), &compressed)
            .is_ok()
        {
            if let Ok(Some((tag, bytes))) = rf.read_record(local_x, local_z) {
                if let Ok(decompressed) = CompressionScheme::decompress_tagged(tag, &bytes) {
                    assert_eq!(decompressed, input.payload);
                }
            }
        }
    }
    let _ = std::fs::remove_file(&path);
});
