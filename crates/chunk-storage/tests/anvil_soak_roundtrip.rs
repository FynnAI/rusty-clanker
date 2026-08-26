//! Acceptance test: the milestone's own 10,000-round-trip soak test (M2's milestone
//! acceptance criterion 2), backed by this crate's own `content_checksum` (M2-B03
//! Deliverables, `checksum.rs`).

mod support;

use rc_chunk_storage::{
    AnvilDiskBackend, ChunkStorageBackend, CompressionScheme, RegionFileKind, content_checksum,
};
use rc_core::DimensionId;
use rc_nbt::owned::{BaseNbt, NbtCompound, NbtTag};
use support::TempWorldDir;

const KINDS: [RegionFileKind; 3] = [
    RegionFileKind::Terrain,
    RegionFileKind::Entities,
    RegionFileKind::Poi,
];
const DIMS: [DimensionId; 3] = [
    DimensionId::OVERWORLD,
    DimensionId::THE_NETHER,
    DimensionId::THE_END,
];

#[test]
fn ten_thousand_chunk_write_read_round_trips_have_zero_checksum_mismatches() {
    let dir = TempWorldDir::new("ten_thousand_chunk_write_read_round_trips_have_zero_checksum_mismatches");
    let backend = AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    let mut mismatches = 0u32;

    for i in 0..10_000u32 {
        let compound = NbtCompound::from_values(vec![
            ("marker".into(), NbtTag::Int(i as i32)),
            ("kind".into(), NbtTag::String("soak".into())),
        ]);
        let root = BaseNbt::new("", compound);
        let encoded = rc_nbt::write_owned(&root);
        let pre = content_checksum(&encoded);

        let kind = KINDS[(i % 3) as usize];
        let dim = DIMS[((i / 7) % 3) as usize];
        let x = (i % 4096) as i32;
        let z = (i / 4096) as i32;

        backend.write_chunk(dim, kind, x, z, &encoded, None).unwrap();
        let bytes = backend
            .read_chunk(dim, kind, x, z, None)
            .unwrap()
            .unwrap_or_else(|| panic!("chunk {i} at ({dim:?}, {kind:?}, {x}, {z}) must read back"));
        let post = content_checksum(&bytes);

        if pre != post || bytes != encoded {
            mismatches += 1;
        }
        assert_eq!(pre, post, "checksum mismatch at iteration {i}");
        assert_eq!(bytes, encoded, "byte mismatch at iteration {i}");
    }

    assert_eq!(mismatches, 0, "soak test must have zero checksum mismatches");
}
