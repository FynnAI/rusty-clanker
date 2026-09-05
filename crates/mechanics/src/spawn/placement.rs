//! `SpawnPlacementType::OnGround` legality, the Monster darkness gate, and the Creature
//! (animal) light rule — blueprint Context, "`SpawnPlacementType::OnGround` legality",
//! "Monster darkness gate", "Creature (animal) light rule".

use rc_core::BlockPos;

use crate::random::RcRandom;
use crate::spawn::cycle::SpawnWorldAccess;

/// Context: "`SpawnPlacementType::OnGround` legality" — this project's own bounded
/// approximation, using only the boolean primitives `SpawnWorldAccess` exposes.
pub fn is_on_ground_legal(world: &dyn SpawnWorldAccess, pos: BlockPos) -> bool {
    let _ = (world, pos);
    todo!()
}

pub fn is_valid_empty_spawn_block(world: &dyn SpawnWorldAccess, pos: BlockPos) -> bool {
    let _ = (world, pos);
    todo!()
}

/// Context: "Monster darkness gate" (overworld: `monster_spawn_block_light_limit = 0`,
/// `monster_spawn_light_test = UniformInt(0, 7)`, M4-B04-CLAIMS.md rows 26-27).
/// `sky_darken` is vanilla's real, explicit formula input (`max(block_light, sky_light -
/// sky_darken)`) — callers pass `world.sky_darken()`, which returns the fixed
/// current-scope value `0` until a future day/night cycle exists.
pub fn is_dark_enough_to_spawn(
    rng: &mut RcRandom,
    sky_light: u8,
    block_light: u8,
    sky_darken: i32,
) -> bool {
    let _ = (rng, sky_light, block_light, sky_darken);
    todo!()
}

/// Context: "Creature (animal) light rule" — zero RNG cost, moderate confidence
/// (M4-B04-CLAIMS.md row 28 confirms the formula, at zero RNG cost, independent of the
/// level's own `sky_darken`).
pub fn is_animal_light_ok(sky_light: u8, block_light: u8) -> bool {
    let _ = (sky_light, block_light);
    todo!()
}
