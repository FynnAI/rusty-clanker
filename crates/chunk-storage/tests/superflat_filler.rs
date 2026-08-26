//! M2-B05 acceptance tests: `rc_chunk_storage::superflat::SuperflatFiller`.

use rc_chunk_storage::superflat::SuperflatFiller;
use rc_chunk_storage::{
    BiomeId, BlockStateId, ChunkGenStatus, ChunkStatus, HeightmapKind, Palette, PaletteThresholds,
};

fn filler() -> SuperflatFiller {
    SuperflatFiller {
        air: BlockStateId(0),
        bedrock: BlockStateId(1),
        dirt: BlockStateId(2),
        grass: BlockStateId(3),
        biome: BiomeId(0),
        block_thresholds: PaletteThresholds::blocks(15),
        biome_thresholds: PaletteThresholds::biomes(4),
    }
}

#[test]
fn layer_table_matches_m1_b05_exactly() {
    let f = filler();
    let (blocks, _, _, _, _) = f.fill();

    for &(x, z) in &[(0u8, 0u8), (15, 15), (7, 3)] {
        assert_eq!(blocks.get(x, -64, z), f.bedrock, "bedrock at ({x},{z})");
        for y in -63..=-61 {
            assert_eq!(blocks.get(x, y, z), f.dirt, "dirt at ({x},{y},{z})");
        }
        assert_eq!(blocks.get(x, -60, z), f.grass, "grass at ({x},{z})");
        assert_eq!(blocks.get(x, -59, z), f.air, "air at ({x},-59,{z})");
        assert_eq!(blocks.get(x, 100, z), f.air, "air at ({x},100,{z})");
    }
}

#[test]
fn biome_is_single_value_everywhere() {
    let f = filler();
    let (_, biomes, _, _, _) = f.fill();

    for section in 0..rc_chunk_storage::SECTION_COUNT {
        assert_eq!(
            *biomes.section(section).palette(),
            Palette::SingleValue(f.biome)
        );
    }
}

#[test]
fn heightmap_reports_first_air_y() {
    let f = filler();
    let (_, _, heightmaps, _, _) = f.fill();

    for x in 0u8..16 {
        for z in 0u8..16 {
            assert_eq!(heightmaps.world_y(HeightmapKind::WorldSurface, x, z), -59);
        }
    }
}

#[test]
fn status_is_full() {
    let f = filler();
    let (_, _, _, _, status) = f.fill();
    assert_eq!(status, ChunkStatus(ChunkGenStatus::Full));
}
