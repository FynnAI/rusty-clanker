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
        match self {
            PathType::Blocked
            | PathType::PowderSnow
            | PathType::Fence
            | PathType::Lava
            | PathType::UnpassableRail
            | PathType::DoorWoodClosed
            | PathType::DoorIronClosed
            | PathType::Leaves
            | PathType::Damaging => -1.0,
            PathType::Water
            | PathType::WaterBorder
            | PathType::FireInNeighbor
            | PathType::DamagingInNeighbor
            | PathType::StickyHoney => 8.0,
            PathType::Fire => 16.0,
            PathType::Breach | PathType::BigMobsCloseToDanger => 4.0,
            PathType::Open
            | PathType::Walkable
            | PathType::WalkableDoor
            | PathType::Trapdoor
            | PathType::OnTopOfPowderSnow
            | PathType::Rail
            | PathType::DoorOpen
            | PathType::Cocoa
            | PathType::DamageCautious
            | PathType::OnTopOfTrapdoor => 0.0,
        }
    }
}

fn reg_to_storage(id: rc_registries::generated_v776::block_states::BlockStateId) -> u32 {
    id.0
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
    /// `true` iff `id` is one of the special/hazard states this table classifies
    /// directly (never a full solid cube for clearance/floor purposes).
    fn is_direct(&self, id: u32) -> bool {
        self.direct.contains_key(&id)
    }

    fn is_air(&self, id: u32) -> bool {
        let range = range_of(block_id::AIR);
        id >= range.first.0 && id <= range.last.0
    }

    /// `true` iff `id` is a full solid opaque cube for the purposes of "does an
    /// entity fit here / does this support a floor" — every unlisted, non-air state
    /// (Context §F's own "default (unlisted) row").
    fn is_full_solid(&self, id: u32) -> bool {
        !self.is_direct(id) && !self.is_air(id)
    }

    /// The per-position `PathType`: the `direct` table's own entry if `pos`'s block
    /// state has one, else the default-row rule (Context §F): `Walkable` if the block
    /// below is a full solid cube and the block itself + one above are non-solid,
    /// else `Open` (self and floor both non-solid) or `Blocked` (self itself solid).
    pub fn classify(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> PathType {
        let id = world.get_block(pos).map(|s| s.to_raw()).unwrap_or_else(|| {
            reg_to_storage(default_state::AIR)
        });
        if let Some(&pt) = self.direct.get(&id) {
            return pt;
        }
        if self.is_full_solid(id) {
            return PathType::Blocked;
        }
        let below_id = world
            .get_block(BlockPos::new(pos.x, pos.y - 1, pos.z))
            .map(|s| s.to_raw())
            .unwrap_or_else(|| reg_to_storage(default_state::AIR));
        let above_id = world
            .get_block(BlockPos::new(pos.x, pos.y + 1, pos.z))
            .map(|s| s.to_raw())
            .unwrap_or_else(|| reg_to_storage(default_state::AIR));
        let below_solid = self.is_full_solid(below_id);
        let above_solid = self.is_full_solid(above_id) || self.direct.contains_key(&above_id);
        if below_solid && !above_solid {
            PathType::Walkable
        } else {
            PathType::Open
        }
    }
}

/// A minimal local trait bridging `BlockWorldAccess`'s own `rc_chunk_storage::
/// BlockStateId` return type to the raw `u32` this module's own `direct` table is
/// keyed by (`rc_chunk_storage::BlockStateId` and `rc_registries::generated_v776::
/// block_states::BlockStateId` are numerically identical but textually distinct types,
/// WORLD-D3/D4's own "resolved discrepancy").
trait RawId {
    fn to_raw(self) -> u32;
}
impl RawId for rc_chunk_storage::BlockStateId {
    fn to_raw(self) -> u32 {
        self.0
    }
}

static TIER1_PATH_TYPE_TABLE: OnceLock<PathTypeTable> = OnceLock::new();

pub fn tier1_path_type_table() -> &'static PathTypeTable {
    TIER1_PATH_TYPE_TABLE.get_or_init(build_tier1_path_type_table)
}

fn insert_range(
    direct: &mut HashMap<u32, PathType>,
    block: rc_registries::generated_v776::block_state_properties::BlockId,
    path_type: PathType,
) {
    let range = range_of(block);
    for raw in range.first.0..=range.last.0 {
        direct.insert(raw, path_type);
    }
}

fn build_tier1_path_type_table() -> PathTypeTable {
    let mut direct: HashMap<u32, PathType> = HashMap::new();

    insert_range(&mut direct, block_id::WATER, PathType::Water);
    insert_range(&mut direct, block_id::LAVA, PathType::Lava);
    insert_range(&mut direct, block_id::OAK_FENCE, PathType::Fence);
    insert_range(&mut direct, block_id::FIRE, PathType::Fire);
    insert_range(&mut direct, block_id::POWDER_SNOW, PathType::PowderSnow);
    insert_range(&mut direct, block_id::CACTUS, PathType::DamageCautious);

    // Oak door: split by its own `open` property (Context §F: closed -> DoorWoodClosed,
    // open -> DoorOpen).
    let door_range = range_of(block_id::OAK_DOOR);
    for raw in door_range.first.0..=door_range.last.0 {
        let id = rc_registries::generated_v776::block_states::BlockStateId(raw);
        let is_open = properties(id)
            .iter()
            .any(|(name, value)| *name == "open" && *value == "true");
        direct.insert(
            raw,
            if is_open {
                PathType::DoorOpen
            } else {
                PathType::DoorWoodClosed
            },
        );
    }

    PathTypeTable { direct }
}

/// `floor(max(1.0, STEP_HEIGHT))` where `STEP_HEIGHT` is every tier-2 kind's own
/// shared `0.6` attribute default (Context §I) — hardcoded here since
/// `NodeEvaluator::get_neighbors`'s own pinned signature (Context §F / Deliverables)
/// carries no `step_height` parameter to thread a per-instance override through. A
/// bounded, documented simplification: every tier-2 kind at M4 scope shares this
/// identical value, so this is not a parity loss for this milestone's own content.
const JUMP_SIZE: i32 = 1;
/// Bounded descent-scan depth (Context §F: "this blueprint's own bounded,
/// moderate-confidence descent limit").
const DESCENT_SCAN_DEPTH: i32 = 3;

/// N, E, S, W (Context §F / CLAIMS.md's own corrected cardinal order).
const CARDINALS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
/// NE, SE, SW, NW, each paired with its own two adjacent cardinal indices into
/// `CARDINALS` (Context §F: "a diagonal is only emitted if both its adjacent cardinal
/// neighbors are themselves valid").
const DIAGONALS: [((i32, i32), usize, usize); 4] = [
    ((1, -1), 1, 0), // NE = E + N
    ((1, 1), 1, 2),  // SE = E + S
    ((-1, 1), 3, 2), // SW = W + S
    ((-1, -1), 3, 0), // NW = W + N
];

fn is_step_up_disallowed(path_type: PathType) -> bool {
    matches!(
        path_type,
        PathType::Fence | PathType::UnpassableRail | PathType::Trapdoor | PathType::PowderSnow
    )
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

impl WalkNodeEvaluator {
    fn malus_of(path_type: PathType, malus_overrides: &HashMap<PathType, f32>) -> f32 {
        malus_overrides
            .get(&path_type)
            .copied()
            .unwrap_or_else(|| path_type.default_malus())
    }

    /// Entity clearance at `y`: the node cell itself, plus (for a taller-than-one-block
    /// entity) one cell above, must both classify to a non-impassable `PathType`.
    fn fits(
        world: &dyn BlockWorldAccess,
        x: i32,
        y: i32,
        z: i32,
        entity_height: f32,
        malus_overrides: &HashMap<PathType, f32>,
    ) -> bool {
        let table = tier1_path_type_table();
        let self_pt = table.classify(world, BlockPos::new(x, y, z));
        if Self::malus_of(self_pt, malus_overrides) < 0.0 {
            return false;
        }
        if entity_height > 1.0 {
            let above_pt = table.classify(world, BlockPos::new(x, y + 1, z));
            if Self::malus_of(above_pt, malus_overrides) < 0.0 {
                return false;
            }
        }
        true
    }

    /// The 3-way vertical placement search for one `(x, z)` candidate column (Context
    /// §F, restated field-precise): same-`Y` first, then step-up XOR downward-scan,
    /// selected by the same-`Y` classification's own `PathType`.
    fn place(
        world: &dyn BlockWorldAccess,
        x: i32,
        z: i32,
        base_y: i32,
        entity_height: f32,
        malus_overrides: &HashMap<PathType, f32>,
        step_up_disabled: bool,
    ) -> Option<(BlockPos, f32)> {
        let table = tier1_path_type_table();

        let same_pt = table.classify(world, BlockPos::new(x, base_y, z));
        let same_malus = Self::malus_of(same_pt, malus_overrides);
        if same_malus >= 0.0 && Self::fits(world, x, base_y, z, entity_height, malus_overrides) {
            return Some((BlockPos::new(x, base_y, z), same_malus));
        }

        if !step_up_disabled && JUMP_SIZE > 0 && !is_step_up_disallowed(same_pt) {
            let up_y = base_y + JUMP_SIZE;
            let up_pt = table.classify(world, BlockPos::new(x, up_y, z));
            let up_malus = Self::malus_of(up_pt, malus_overrides);
            if up_malus >= 0.0 && Self::fits(world, x, up_y, z, entity_height, malus_overrides) {
                return Some((BlockPos::new(x, up_y, z), up_malus));
            }
            // A failed step-up never falls through to the downward scan.
            return None;
        }

        if matches!(same_pt, PathType::Water | PathType::Open) {
            for dy in 1..=DESCENT_SCAN_DEPTH {
                let down_y = base_y - dy;
                let down_pt = table.classify(world, BlockPos::new(x, down_y, z));
                let down_malus = Self::malus_of(down_pt, malus_overrides);
                if down_malus >= 0.0
                    && Self::fits(world, x, down_y, z, entity_height, malus_overrides)
                {
                    return Some((BlockPos::new(x, down_y, z), down_malus));
                }
            }
        }

        None
    }
}

impl NodeEvaluator for WalkNodeEvaluator {
    fn get_neighbors(
        &self,
        world: &dyn BlockWorldAccess,
        from: BlockPos,
        entity_height: f32,
        malus_overrides: &HashMap<PathType, f32>,
    ) -> Vec<(BlockPos, f32)> {
        let table = tier1_path_type_table();
        let above_current = table.classify(world, BlockPos::new(from.x, from.y + 1, from.z));
        let step_up_disabled = Self::malus_of(above_current, malus_overrides) < 0.0;

        let mut cardinal_results: [Option<(BlockPos, f32)>; 4] = [None, None, None, None];
        for (i, (dx, dz)) in CARDINALS.iter().enumerate() {
            cardinal_results[i] = Self::place(
                world,
                from.x + dx,
                from.z + dz,
                from.y,
                entity_height,
                malus_overrides,
                step_up_disabled,
            );
        }

        let mut out = Vec::new();
        for cand in cardinal_results.iter().flatten() {
            out.push((cand.0, 1.0 + cand.1));
        }

        for (diag, c1_idx, c2_idx) in DIAGONALS.iter() {
            if cardinal_results[*c1_idx].is_none() || cardinal_results[*c2_idx].is_none() {
                continue;
            }
            let (dx, dz) = diag;
            if let Some((pos, malus)) = Self::place(
                world,
                from.x + dx,
                from.z + dz,
                from.y,
                entity_height,
                malus_overrides,
                step_up_disabled,
            ) {
                out.push((pos, std::f32::consts::SQRT_2 + malus));
            }
        }

        out
    }
}
