//! ARCH-D14's random-tick position-selection mechanism (Context: "Random-tick position
//! selection"). Ships the deterministic per-chunk draw algorithm only — zero real receivers
//! (Constraints (d)).

use bevy_ecs::prelude::Resource;
use rc_core::BlockPos;

use crate::random::{RcRandom, chunk_random_seed};

/// Vanilla's `GameRules.RANDOM_TICK_SPEED` default (`08-redstone-ticking.md` §3.5). No
/// `GameRules` resource exists yet (MECH-D64) — callers pass this constant until a future
/// blueprint threads the real, mutable value through.
pub const DEFAULT_RANDOM_TICK_SPEED: u32 = 3;

/// The world's seed (a new, small `bevy_ecs::Resource` this blueprint introduces — M3-B01
/// defined `chunk_random_seed` but no resource to carry the seed itself, since it had no
/// Stage-5 consumer yet). `#[derive(Resource)]` is a zero-cost marker.
#[derive(Resource, Copy, Clone, Debug, Default)]
pub struct WorldSeed(pub i64);

/// One drawn candidate (Context: "Random-tick position selection").
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RandomTickPosition {
    pub pos: BlockPos,
}

/// Drains exactly `24 * random_tick_speed` draws from `rng` (Context's own algorithm,
/// restated in full there — one `rng.next_int()` per candidate, bit-extracted as `x = bits &
/// 15`, `z = (bits >> 8) & 15`, `y_local = (bits >> 16) & 15`), in ascending section-index
/// order (bottom to top), returning them in draw order. `chunk_min_x`/`chunk_min_z` are the
/// chunk's own world-space block origin (`chunk_x * 16`/`chunk_z * 16`); `section_min_y(i) =
/// -64 + i as i32 * 16` (M2-B01's own `WORLD_MIN_Y`/`SECTION_COUNT` constants, restated as
/// plain `i32` arithmetic here to avoid a `rc-chunk-storage` dependency in this pure,
/// allocation-free function).
pub fn draw_random_tick_positions(
    rng: &mut RcRandom,
    chunk_min_x: i32,
    chunk_min_z: i32,
    random_tick_speed: u32,
) -> Vec<RandomTickPosition> {
    todo!()
}

/// Convenience: `RcRandom::new(chunk_random_seed(seed.0, chunk_x, chunk_z, tick))` then
/// `draw_random_tick_positions(&mut rng, chunk_x * 16, chunk_z * 16, random_tick_speed)` (the
/// `* 16` is the chunk-to-block-space conversion `draw_random_tick_positions` itself does not
/// perform, since that function's own contract already takes the block-space origin
/// directly) — the single call a Stage-5 driver makes per chunk per tick.
pub fn random_tick_chunk(
    seed: &WorldSeed,
    chunk_x: i32,
    chunk_z: i32,
    tick_counter: u64,
    random_tick_speed: u32,
) -> Vec<RandomTickPosition> {
    todo!()
}
