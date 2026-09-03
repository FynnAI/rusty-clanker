//! `PathType` classification and `WalkNodeEvaluator` neighbor generation (MECH-D33,
//! M4-B03 blueprint Context §F).

use std::collections::HashMap;
use std::sync::OnceLock;

use rc_core::BlockPos;
use rc_registries::block_state_properties::{properties, range_of};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::default_state;

use crate::world_access::BlockWorldAccess;

/// Vanilla's complete `PathType` classification (Context §F).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PathType {
    Blocked,
    PowderSnow,
    Fence,
    Lava,
    UnpassableRail,
    DoorWoodClosed,
    DoorIronClosed,
    Leaves,
    Damaging,
    Water,
    WaterBorder,
    FireInNeighbor,
    DamagingInNeighbor,
    StickyHoney,
    Fire,
    Breach,
    BigMobsCloseToDanger,
    Open,
    Walkable,
    WalkableDoor,
    Trapdoor,
    OnTopOfPowderSnow,
    Rail,
    DoorOpen,
    Cocoa,
    DamageCautious,
    OnTopOfTrapdoor,
}

impl PathType {
    /// Context §F's own fixed table, restated exactly.
    pub const fn default_malus(self) -> f32 {
        todo!()
    }
}

/// A hand-authored tier-1 `BlockStateId -> PathType` classifier, mirroring
/// `rc_physics::tier1_shape_table()`'s own precedent (Context §F). Only the small
/// hazard/special block set Context §F names carries a `direct` entry; every other
/// state (including every plain full-solid block and air) is classified by the
/// default-row solidity rule in `classify` below.
pub struct PathTypeTable {
    direct: HashMap<u32, PathType>,
}

impl PathTypeTable {
    pub fn classify(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> PathType {
        todo!()
    }
}

static TIER1_PATH_TYPE_TABLE: OnceLock<PathTypeTable> = OnceLock::new();

pub fn tier1_path_type_table() -> &'static PathTypeTable {
    todo!()
}

/// Vanilla's own `WalkNodeEvaluator` neighbor generation (Context §F, restated
/// field-precise).
pub trait NodeEvaluator {
    fn get_neighbors(
        &self,
        world: &dyn BlockWorldAccess,
        from: BlockPos,
        entity_height: f32,
        malus_overrides: &HashMap<PathType, f32>,
    ) -> Vec<(BlockPos, f32)>;
}

pub struct WalkNodeEvaluator;

impl NodeEvaluator for WalkNodeEvaluator {
    fn get_neighbors(
        &self,
        world: &dyn BlockWorldAccess,
        from: BlockPos,
        entity_height: f32,
        malus_overrides: &HashMap<PathType, f32>,
    ) -> Vec<(BlockPos, f32)> {
        todo!()
    }
}
