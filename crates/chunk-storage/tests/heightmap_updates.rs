//! `HeightmapSet` construction and the `note_block_change` incremental update rule
//! (M2-B01 Deliverables, `heightmap.rs`).

use rc_chunk_storage::{pack_bits, BlockOpacity, HeightmapKind, HeightmapSet, WORLD_MIN_Y};

/// A `column_opacity_below` stub for tests that never need the rescan branch.
fn always_false(_: HeightmapKind, _: i32) -> bool {
    false
}

fn no_opacity() -> BlockOpacity {
    BlockOpacity {
        world_surface: false,
        ocean_floor: false,
        motion_blocking: false,
        motion_blocking_no_leaves: false,
    }
}

#[test]
fn new_uniform_reports_the_given_height_everywhere() {
    let heightmaps = HeightmapSet::new_uniform(-59);
    for &(x, z) in &[(0u8, 0u8), (15, 15), (7, 3)] {
        for kind in HeightmapKind::ALL {
            assert_eq!(heightmaps.world_y(kind, x, z), -59);
        }
    }
}

#[test]
fn raise_above_current_height_is_an_o1_update() {
    let mut heightmaps = HeightmapSet::new_uniform(-59);
    let old = no_opacity();
    let new = BlockOpacity {
        world_surface: true,
        ..no_opacity()
    };
    heightmaps.note_block_change(3, 10, 4, old, new, always_false);

    assert_eq!(heightmaps.world_y(HeightmapKind::WorldSurface, 3, 4), 11);
    assert_eq!(heightmaps.world_y(HeightmapKind::OceanFloor, 3, 4), -59);
    assert_eq!(heightmaps.world_y(HeightmapKind::MotionBlocking, 3, 4), -59);
    assert_eq!(
        heightmaps.world_y(HeightmapKind::MotionBlockingNoLeaves, 3, 4),
        -59
    );
}

#[test]
fn placement_below_current_height_is_a_no_op() {
    let mut heightmaps = HeightmapSet::new_uniform(-59);
    let old = no_opacity();
    let new = BlockOpacity {
        world_surface: true,
        ocean_floor: true,
        motion_blocking: true,
        motion_blocking_no_leaves: true,
    };
    heightmaps.note_block_change(3, -64, 4, old, new, always_false);

    for kind in HeightmapKind::ALL {
        assert_eq!(heightmaps.world_y(kind, 3, 4), -59);
    }
}

#[test]
fn removal_at_current_height_triggers_rescan() {
    let mut heightmaps = HeightmapSet::new_uniform(-59);
    let old = BlockOpacity {
        world_surface: true,
        ..no_opacity()
    };
    let new = no_opacity();
    heightmaps.note_block_change(3, -60, 4, old, new, always_false);

    assert_eq!(
        heightmaps.world_y(HeightmapKind::WorldSurface, 3, 4),
        WORLD_MIN_Y
    );
}

#[test]
fn removal_at_current_height_with_a_lower_opaque_block_found_by_rescan() {
    let mut heightmaps = HeightmapSet::new_uniform(-59);
    let old = BlockOpacity {
        world_surface: true,
        ..no_opacity()
    };
    let new = no_opacity();
    heightmaps.note_block_change(3, -60, 4, old, new, |kind, y| {
        kind == HeightmapKind::WorldSurface && y == -63
    });

    assert_eq!(heightmaps.world_y(HeightmapKind::WorldSurface, 3, 4), -62);
}

#[test]
fn set_raw_and_raw_round_trip() {
    let mut heightmaps = HeightmapSet::new_uniform(0);
    heightmaps.set_raw(HeightmapKind::MotionBlocking, 2, 2, 100);
    assert_eq!(heightmaps.raw(HeightmapKind::MotionBlocking, 2, 2), 100);
    assert_eq!(
        heightmaps.world_y(HeightmapKind::MotionBlocking, 2, 2),
        100 + WORLD_MIN_Y
    );
}

#[test]
fn packed_word_count_matches_world_d5() {
    assert_eq!(pack_bits(&[5u32; 256], 9).len(), 37);
}
