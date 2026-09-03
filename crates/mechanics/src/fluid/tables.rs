//! The bundled fluid configuration (Context §M/§C/§L) every core function in this module
//! takes: the scheduling table, gamerule defaults, reaction-block ids, and the dimension
//! profile/`LevelRandom` stand-ins for data this project does not generate yet.

use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;

use super::state::{FluidBlockRanges, FluidKind};
use crate::random::RcRandom;

/// `EnvironmentAttributes.FAST_LAVA` stand-in (Context §M) — no real `dimension_type` registry
/// exists yet (MECH-D66); a composition root supplies one instance per region.
#[derive(Copy, Clone, Debug, Default, Resource)]
pub struct FluidDimensionProfile {
    pub fast_lava: bool,
}

/// `WATER_SOURCE_CONVERSION`/`LAVA_SOURCE_CONVERSION` gamerule defaults (Context §C, `true`/
/// `false` respectively — real vanilla defaults, not this project's invention).
#[derive(Copy, Clone, Debug, Resource)]
pub struct FluidGameRules {
    pub water_source_conversion: bool,
    pub lava_source_conversion: bool,
}

impl Default for FluidGameRules {
    fn default() -> Self {
        todo!()
    }
}

impl FluidGameRules {
    pub fn allows_source_conversion(&self, kind: FluidKind) -> bool {
        let _ = kind;
        todo!()
    }
}

/// Soul-soil + blue-ice -> basalt (Context §I(A)) — optional; the primary, mandatory reaction is
/// obsidian/cobblestone.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BasaltConversion {
    pub soul_soil: BlockStateId,
    pub blue_ice: BlockStateId,
    pub basalt: BlockStateId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ReactionBlocks {
    pub obsidian: BlockStateId,
    pub cobblestone: BlockStateId,
    pub stone: BlockStateId,
    pub basalt_conversion: Option<BasaltConversion>,
}

/// The single bundled config every core function in this module takes (Context, throughout).
#[derive(Clone, Debug, Resource)]
pub struct FluidTables {
    pub ranges: FluidBlockRanges,
    pub reactions: ReactionBlocks,
    pub dimension: FluidDimensionProfile,
    pub gamerules: FluidGameRules,
    pub air: BlockStateId,
    /// Context §F — empty by default; vanilla's own fixed denylist has no matching content yet.
    pub deny_hold_fluid: Vec<(BlockStateId, BlockStateId)>,
    /// Context §F — empty by default; ice's real exception is reserved here for later content.
    pub solid_face_exceptions: Vec<(BlockStateId, BlockStateId)>,
    /// Context §F — `calculateSolid`'s own `forceSolidOn` override, checked first, empty by
    /// default: verified against the reference, real vanilla's `forceSolidOn` applies only to
    /// cobweb, the in-flight `moving_piston` block, and every sign/hanging-sign variant, none of
    /// which exist in this blueprint's own tier-1/tier-2 placeable set.
    pub force_solid_on: Vec<(BlockStateId, BlockStateId)>,
    /// Context §F — `calculateSolid`'s own `forceSolidOff` override, checked second, empty by
    /// default: verified against the reference, real vanilla's `forceSolidOff` applies only to
    /// ladder, which does not exist in this blueprint's own tier-1/tier-2 placeable set.
    pub force_solid_off: Vec<(BlockStateId, BlockStateId)>,
}

impl FluidTables {
    /// `gamerules: FluidGameRules::default()`, `deny_hold_fluid`/`solid_face_exceptions`/
    /// `force_solid_on`/`force_solid_off: vec![]`.
    pub fn new(
        ranges: FluidBlockRanges,
        reactions: ReactionBlocks,
        dimension: FluidDimensionProfile,
        air: BlockStateId,
    ) -> Self {
        let _ = (ranges, reactions, dimension, air);
        todo!()
    }

    /// Context §M's table: water 5, lava 30/10 by `dimension.fast_lava`.
    pub fn tick_delay(&self, kind: FluidKind) -> u64 {
        let _ = kind;
        todo!()
    }

    /// Context §C/§D: water 1, lava 2/1 by `dimension.fast_lava`.
    pub fn drop_off(&self, kind: FluidKind) -> u8 {
        let _ = kind;
        todo!()
    }

    /// Context §E: water 4 always, lava 4/2 by `dimension.fast_lava`.
    pub fn slope_find_distance(&self, kind: FluidKind) -> u32 {
        let _ = kind;
        todo!()
    }
}

/// `Level.random` stand-in (Context §L) — a shared, non-deterministically-seeded stream
/// distinct from `ARCH-D14`'s per-chunk stream. Held internally by `FluidBehavior`
/// (`Arc<Mutex<LevelRandom>>`), never threaded through `UpdateContext` (Context §L explains why).
#[derive(Clone, Debug)]
pub struct LevelRandom(RcRandom);

impl LevelRandom {
    /// Production seeding — mirrors vanilla's own non-reproducible-across-restart entropy
    /// source; never used by a determinism-sensitive test.
    pub fn from_entropy() -> Self {
        todo!()
    }

    /// Deterministic, test-only.
    pub fn from_seed(seed: i64) -> Self {
        let _ = seed;
        todo!()
    }

    /// `next_int_bounded(bound)`.
    pub fn roll_next_int(&mut self, bound: i32) -> i32 {
        let _ = bound;
        todo!()
    }
}
