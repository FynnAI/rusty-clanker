//! M4-B07 — hand-derived canonical arrangements exercising the push-model BFS
//! propagator directly (Context §2), no ECS/executor involved.
//!
//! Ledger note (recorded in `docs/findings-for-planning.md` section B): two of this
//! file's own scenarios, as the blueprint's Acceptance tests section describes them,
//! do not actually produce the result their own prose asserts, once worked through
//! precisely:
//!
//! - `skylight_column_punch_through`'s own literal geometry (a single opaque block at
//!   `y=99` per side column, air everywhere else in that column) does not block
//!   horizontal light leakage below the roof -- the push-model BFS propagator spreads
//!   sideways through open air exactly as vanilla's own light engine does, so the
//!   open shaft's own full-sun value at `y=95` would propagate one hop sideways into
//!   an adjacent column with nothing but air there, reading `14`, not the `0` the
//!   blueprint's own prose asserts ("walled-off neighbor columns", "blocked to the
//!   sides"). This test instead makes each side column fully opaque for
//!   `y in 90..=99` (not only at `y=99`), which actually walls the shaft off as the
//!   blueprint's own prose describes, while leaving every one of the blueprint's own
//!   `boundary_y` assertions unchanged (the first occluding pair scanning downward
//!   from the heightmap seed is still the pair at `(100, 99)` regardless of what sits
//!   below `y=99`).
//! - `removal_darkness_propagation_with_surviving_source`'s own stated literal arrays
//!   contain arithmetic slips relative to its own stated formula: converged
//!   multi-source BFS light with uniform per-hop decay is provably the pointwise
//!   maximum of each source's own independent decay curve (`max(0, 14-x)` and
//!   `max(0, 8-|x-10|)` here) -- working that formula out at every `x` gives
//!   `[14,13,12,11,10,9,8,7,6,7,8,7,6,5,4,3]` for the combined array (the blueprint's
//!   own text has `8,8,8` at `x=7,8,9` instead of the formula's own `7,6,7`) and
//!   `[0,0,0,1,2,3,4,5,6,7,8,7,6,5,4,3]` for the array after the `x=0` source is
//!   removed (the blueprint's own text has `0,0,0,0,0` at `x=3..7` instead of the
//!   formula's own `1,2,3,4,5`). This test asserts the values the blueprint's own
//!   formula actually produces.

use rc_chunk_storage::{
    BlockStateColumn, BlockStateId, HeightmapKind, HeightmapSet, LightColumn, PaletteThresholds,
};
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::light::propagator::{LocalChunkLight, get_stored};
use rc_mechanics::light::{
    LightChannel, check_node_block, check_node_sky, propagate_decrease_step,
    propagate_increase_step,
};
use rc_mechanics::{
    ChannelState, LightProperties, LightPropertiesRegistry, SkyLightSourceColumn, direction_index,
};

const AIR: BlockStateId = BlockStateId(0);

fn air_column() -> BlockStateColumn {
    BlockStateColumn::new(AIR, PaletteThresholds::blocks(15))
}

fn uniform_heightmap(first_air_world_y: i32) -> HeightmapSet {
    HeightmapSet::new_uniform(first_air_world_y)
}

fn only_face(dir: Direction) -> [bool; 6] {
    let mut faces = [false; 6];
    faces[direction_index(dir)] = true;
    faces
}

/// Drives `state`'s two work queues to a fixed point: decrease drains fully before
/// increase is touched at all, each round, capped at 16 rounds (Test harness note).
fn drain_to_fixed_point(
    local: &mut LocalChunkLight,
    state: &mut ChannelState,
    channel: LightChannel,
) {
    for _ in 0..16 {
        if state.decrease.is_empty() && state.increase.is_empty() {
            return;
        }
        while let Some(entry) = state.decrease.pop_front() {
            propagate_decrease_step(local, entry, channel, state);
        }
        while let Some(entry) = state.increase.pop_front() {
            propagate_increase_step(local, entry, channel, state);
        }
    }
    assert!(
        state.decrease.is_empty() && state.increase.is_empty(),
        "drain_to_fixed_point: did not converge within 16 rounds"
    );
}

#[test]
fn single_torch_open_corridor() {
    let blocks = air_column();
    let heightmap = uniform_heightmap(rc_chunk_storage::WORLD_MIN_Y);
    let mut properties = LightPropertiesRegistry::new();
    properties.register_one(
        BlockStateId(1),
        LightProperties {
            block_emission: 14,
            opacity: 0,
            occludes_face: [false; 6],
        },
    );
    let mut sky_sources = SkyLightSourceColumn::recompute(&blocks, &heightmap, &properties);
    let mut light = LightColumn::new_uninitialized();
    let mut local = LocalChunkLight {
        light: &mut light,
        sky_sources: &mut sky_sources,
        blocks: &blocks,
        heightmap: &heightmap,
        properties: &properties,
        chunk_origin_x: 0,
        chunk_origin_z: 0,
    };
    let mut state = ChannelState::default();

    check_node_block(&mut local, BlockPos::new(0, 0, 0), 0, 14, &mut state);
    drain_to_fixed_point(&mut local, &mut state, LightChannel::Block);

    for x in 0..16i32 {
        let expected = 14u8.saturating_sub(x as u8);
        assert_eq!(
            get_stored(&local, BlockPos::new(x, 0, 0), LightChannel::Block),
            expected,
            "mismatch at x={x}"
        );
    }
}

#[test]
fn opaque_wall_stops_propagation() {
    let mut blocks = air_column();
    // A full Y/Z wall at x=3, not only the single (3,0,0) cell: the propagator is a
    // genuine 3D BFS (`check_node_block`'s own seed fans out in all six directions),
    // so a single opaque block leaves the corridor's light free to detour around it
    // vertically/laterally and re-enter past x=3 from a different y/z -- comfortably
    // covering the emitter's own 14-level decay budget (at most 14 hops can ever be
    // spent detouring before the value reaches zero) on both axes blocks every such
    // path, matching this test's own intent of a corridor genuinely sealed at x=3.
    for y in -16..=16i32 {
        for z in 0u8..16 {
            blocks.set(3, y, z, BlockStateId(2));
        }
    }
    let heightmap = uniform_heightmap(rc_chunk_storage::WORLD_MIN_Y);
    let mut properties = LightPropertiesRegistry::new();
    properties.register_one(
        BlockStateId(1),
        LightProperties {
            block_emission: 14,
            opacity: 0,
            occludes_face: [false; 6],
        },
    );
    properties.register_one(BlockStateId(2), LightProperties::OPAQUE);
    let mut sky_sources = SkyLightSourceColumn::recompute(&blocks, &heightmap, &properties);
    let mut light = LightColumn::new_uninitialized();
    let mut local = LocalChunkLight {
        light: &mut light,
        sky_sources: &mut sky_sources,
        blocks: &blocks,
        heightmap: &heightmap,
        properties: &properties,
        chunk_origin_x: 0,
        chunk_origin_z: 0,
    };
    let mut state = ChannelState::default();

    check_node_block(&mut local, BlockPos::new(0, 0, 0), 0, 14, &mut state);
    drain_to_fixed_point(&mut local, &mut state, LightChannel::Block);

    let expected = [14u8, 13, 12, 0, 0, 0];
    for (x, &want) in expected.iter().enumerate() {
        assert_eq!(
            get_stored(&local, BlockPos::new(x as i32, 0, 0), LightChannel::Block),
            want,
            "mismatch at x={x}"
        );
    }
}

#[test]
fn skylight_column_punch_through() {
    let mut blocks = air_column();
    let mut heightmap = uniform_heightmap(100);
    // Open shaft: first air Y sits at the top of the tested range.
    heightmap.set_raw(
        HeightmapKind::WorldSurface,
        1,
        1,
        (110 - rc_chunk_storage::WORLD_MIN_Y) as u16,
    );

    let roof: BlockStateId = BlockStateId(2);
    // See this file's own module doc comment: each side column is solid opaque for
    // y in 90..=99 (not only at y=99) so the shaft is genuinely walled off, matching
    // this test's own stated intent.
    for x in 0u8..3 {
        for z in 0u8..3 {
            if (x, z) == (1, 1) {
                continue;
            }
            for y in 90..=99i32 {
                blocks.set(x, y, z, roof);
            }
        }
    }

    let mut properties = LightPropertiesRegistry::new();
    properties.register_one(roof, LightProperties::OPAQUE);

    let mut sky_sources = SkyLightSourceColumn::recompute(&blocks, &heightmap, &properties);
    assert_eq!(sky_sources.boundary_y(1, 1), rc_chunk_storage::WORLD_MIN_Y);
    for x in 0u8..3 {
        for z in 0u8..3 {
            if (x, z) == (1, 1) {
                continue;
            }
            assert_eq!(
                sky_sources.boundary_y(x, z),
                100,
                "boundary mismatch at ({x},{z})"
            );
        }
    }

    let mut light = LightColumn::new_uninitialized();
    let mut local = LocalChunkLight {
        light: &mut light,
        sky_sources: &mut sky_sources,
        blocks: &blocks,
        heightmap: &heightmap,
        properties: &properties,
        chunk_origin_x: 0,
        chunk_origin_z: 0,
    };
    let mut state = ChannelState::default();

    for x in 0u8..3 {
        for z in 0u8..3 {
            let boundary = local.sky_sources.boundary_y(x, z).max(90);
            for y in boundary..=109 {
                check_node_sky(
                    &mut local,
                    BlockPos::new(x as i32, y, z as i32),
                    true,
                    &mut state,
                );
            }
        }
    }
    drain_to_fixed_point(&mut local, &mut state, LightChannel::Sky);

    for y in 90..110 {
        assert_eq!(
            get_stored(&local, BlockPos::new(1, y, 1), LightChannel::Sky),
            15,
            "shaft column mismatch at y={y}"
        );
    }
    for (x, z) in [(0i32, 1i32), (2, 1), (1, 0), (1, 2)] {
        assert_eq!(
            get_stored(&local, BlockPos::new(x, 95, z), LightChannel::Sky),
            0,
            "walled-off neighbor column mismatch at ({x},95,{z})"
        );
    }
}

#[test]
fn stairs_like_partial_occlusion() {
    // Sub-test A: horizontal propagation into the stairs-like block is governed by
    // scalar opacity only (occludes_face[West]/[East] both false).
    {
        let mut blocks = air_column();
        blocks.set(2, 0, 0, BlockStateId(3));
        let heightmap = uniform_heightmap(rc_chunk_storage::WORLD_MIN_Y);
        let mut properties = LightPropertiesRegistry::new();
        properties.register_one(
            BlockStateId(1),
            LightProperties {
                block_emission: 10,
                opacity: 0,
                occludes_face: [false; 6],
            },
        );
        properties.register_one(
            BlockStateId(3),
            LightProperties {
                block_emission: 0,
                opacity: 1,
                occludes_face: only_face(Direction::Down),
            },
        );
        let mut sky_sources = SkyLightSourceColumn::recompute(&blocks, &heightmap, &properties);
        let mut light = LightColumn::new_uninitialized();
        let mut local = LocalChunkLight {
            light: &mut light,
            sky_sources: &mut sky_sources,
            blocks: &blocks,
            heightmap: &heightmap,
            properties: &properties,
            chunk_origin_x: 0,
            chunk_origin_z: 0,
        };
        let mut state = ChannelState::default();

        check_node_block(&mut local, BlockPos::new(0, 0, 0), 0, 10, &mut state);
        drain_to_fixed_point(&mut local, &mut state, LightChannel::Block);

        let expected = [10u8, 9, 8, 7, 6];
        for (x, &want) in expected.iter().enumerate() {
            assert_eq!(
                get_stored(&local, BlockPos::new(x as i32, 0, 0), LightChannel::Block),
                want,
                "mismatch at x={x}"
            );
        }
    }

    // Sub-test B: a downward step into the stairs-like block is fully vetoed by its
    // own occludes_face[Up] -- restated precisely per this blueprint's own text.
    // `occludes_face` is a purely *directional* veto (Context §3): it blocks a
    // straight-down entry into (2,0,0) through its own top face but leaves every
    // other one of its faces open, so without actual walls the 3D BFS propagator
    // finds an alternate, unvetoed path in (e.g. sideways around the stairs-like
    // block, down a level, then back up into its own unvetoed bottom face) and
    // lights it anyway. Opaque blocks at (2,0,0)'s three sealable horizontal
    // neighbors plus the position directly below it close every such detour,
    // isolating the straight-down approach this sub-test actually means to exercise
    // (its own fourth horizontal neighbor, one step further negative in z, falls
    // outside this chunk's own local extent and is deferred away unused, never
    // reaching (2,0,0) either way).
    {
        let mut blocks = air_column();
        blocks.set(2, 0, 0, BlockStateId(3));
        blocks.set(2, 1, 0, BlockStateId(1));
        blocks.set(1, 0, 0, BlockStateId(2));
        blocks.set(3, 0, 0, BlockStateId(2));
        blocks.set(2, 0, 1, BlockStateId(2));
        blocks.set(2, -1, 0, BlockStateId(2));
        let heightmap = uniform_heightmap(rc_chunk_storage::WORLD_MIN_Y);
        let mut properties = LightPropertiesRegistry::new();
        properties.register_one(
            BlockStateId(1),
            LightProperties {
                block_emission: 10,
                opacity: 0,
                occludes_face: [false; 6],
            },
        );
        properties.register_one(
            BlockStateId(3),
            LightProperties {
                block_emission: 0,
                opacity: 1,
                occludes_face: only_face(Direction::Up),
            },
        );
        properties.register_one(BlockStateId(2), LightProperties::OPAQUE);
        let mut sky_sources = SkyLightSourceColumn::recompute(&blocks, &heightmap, &properties);
        let mut light = LightColumn::new_uninitialized();
        let mut local = LocalChunkLight {
            light: &mut light,
            sky_sources: &mut sky_sources,
            blocks: &blocks,
            heightmap: &heightmap,
            properties: &properties,
            chunk_origin_x: 0,
            chunk_origin_z: 0,
        };
        let mut state = ChannelState::default();

        check_node_block(&mut local, BlockPos::new(2, 1, 0), 0, 10, &mut state);
        drain_to_fixed_point(&mut local, &mut state, LightChannel::Block);

        assert_eq!(
            get_stored(&local, BlockPos::new(2, 0, 0), LightChannel::Block),
            0,
            "downward step into the stairs-like block must be fully vetoed"
        );
    }
}

#[test]
fn removal_darkness_propagation_no_survivor() {
    let blocks = air_column();
    let heightmap = uniform_heightmap(rc_chunk_storage::WORLD_MIN_Y);
    let mut properties = LightPropertiesRegistry::new();
    properties.register_one(
        BlockStateId(1),
        LightProperties {
            block_emission: 14,
            opacity: 0,
            occludes_face: [false; 6],
        },
    );
    let mut sky_sources = SkyLightSourceColumn::recompute(&blocks, &heightmap, &properties);
    let mut light = LightColumn::new_uninitialized();
    let mut local = LocalChunkLight {
        light: &mut light,
        sky_sources: &mut sky_sources,
        blocks: &blocks,
        heightmap: &heightmap,
        properties: &properties,
        chunk_origin_x: 0,
        chunk_origin_z: 0,
    };
    let mut state = ChannelState::default();

    check_node_block(&mut local, BlockPos::new(0, 0, 0), 0, 14, &mut state);
    drain_to_fixed_point(&mut local, &mut state, LightChannel::Block);

    check_node_block(&mut local, BlockPos::new(0, 0, 0), 14, 0, &mut state);
    drain_to_fixed_point(&mut local, &mut state, LightChannel::Block);

    for x in 0..16i32 {
        assert_eq!(
            get_stored(&local, BlockPos::new(x, 0, 0), LightChannel::Block),
            0,
            "mismatch at x={x}"
        );
    }
}

#[test]
fn removal_darkness_propagation_with_surviving_source() {
    let blocks = air_column();
    let heightmap = uniform_heightmap(rc_chunk_storage::WORLD_MIN_Y);
    let mut properties = LightPropertiesRegistry::new();
    properties.register_one(
        BlockStateId(1),
        LightProperties {
            block_emission: 14,
            opacity: 0,
            occludes_face: [false; 6],
        },
    );
    properties.register_one(
        BlockStateId(4),
        LightProperties {
            block_emission: 8,
            opacity: 0,
            occludes_face: [false; 6],
        },
    );
    let mut sky_sources = SkyLightSourceColumn::recompute(&blocks, &heightmap, &properties);
    let mut light = LightColumn::new_uninitialized();
    let mut local = LocalChunkLight {
        light: &mut light,
        sky_sources: &mut sky_sources,
        blocks: &blocks,
        heightmap: &heightmap,
        properties: &properties,
        chunk_origin_x: 0,
        chunk_origin_z: 0,
    };
    let mut state = ChannelState::default();

    check_node_block(&mut local, BlockPos::new(0, 0, 0), 0, 14, &mut state);
    check_node_block(&mut local, BlockPos::new(10, 0, 0), 0, 8, &mut state);
    drain_to_fixed_point(&mut local, &mut state, LightChannel::Block);

    // Pointwise max of max(0, 14-x) and max(0, 8-|x-10|) -- see this file's own module
    // doc comment for why this differs from the blueprint's own literal array.
    let combined: [u8; 16] = [14, 13, 12, 11, 10, 9, 8, 7, 6, 7, 8, 7, 6, 5, 4, 3];
    for (x, &want) in combined.iter().enumerate() {
        assert_eq!(
            get_stored(&local, BlockPos::new(x as i32, 0, 0), LightChannel::Block),
            want,
            "combined mismatch at x={x}"
        );
    }

    check_node_block(&mut local, BlockPos::new(0, 0, 0), 14, 0, &mut state);
    drain_to_fixed_point(&mut local, &mut state, LightChannel::Block);

    // max(0, 8-|x-10|) alone -- see this file's own module doc comment.
    let survivor: [u8; 16] = [0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 7, 6, 5, 4, 3];
    for (x, &want) in survivor.iter().enumerate() {
        assert_eq!(
            get_stored(&local, BlockPos::new(x as i32, 0, 0), LightChannel::Block),
            want,
            "survivor mismatch at x={x}"
        );
    }
}
