//! TEST-D26 item (3), never-panics half: "decode never panics on arbitrary bytes
//! claiming to be a region file."
#![no_main]
use libfuzzer_sys::fuzz_target;
use rc_chunk_storage::RegionFile;

fuzz_target!(|data: &[u8]| {
    let path = std::env::temp_dir().join(format!(
        "rc-anvil-fuzz-decode-{}-{:?}.mca",
        std::process::id(),
        std::thread::current().id()
    ));
    if std::fs::write(&path, data).is_ok() {
        if let Ok(mut rf) = RegionFile::open(path.clone(), 0, 0) {
            for local_z in 0..32u8 {
                for local_x in 0..32u8 {
                    let _ = rf.read_record(local_x, local_z); // must never panic, Ok/Err both fine
                }
            }
        }
    }
    let _ = std::fs::remove_file(&path);
});
