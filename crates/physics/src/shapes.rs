//! `VoxelShape` and the hand-authored tier-1 block-shape table (MECH-D38/D39, Context:
//! "VoxelShape representation and the tier-1 block-shape table").

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::Aabb;
use crate::vec3::Vec3;
use rc_core::BlockPos;

/// A set of axis-aligned sub-boxes in block-local `[0,1]^3` space (Context: "VoxelShape
/// representation" -- deliberately the simplest correct form, `Vec<Aabb>`, no grid/bitset
/// optimization).
#[derive(Clone, Debug, PartialEq)]
pub struct VoxelShape {
    boxes: Vec<Aabb>,
}

impl VoxelShape {
    pub const fn empty() -> Self {
        todo!()
    }

    /// The single-box full unit cube.
    pub fn full_cube() -> Self {
        todo!()
    }

    pub fn from_boxes(boxes: Vec<Aabb>) -> Self {
        todo!()
    }

    pub fn boxes(&self) -> &[Aabb] {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

/// Physics-relevant per-block-state properties (Context: shape-source seam).
#[derive(Clone, Debug, PartialEq)]
pub struct BlockPhysicsProperties {
    pub shape: VoxelShape,
    pub friction: f64,
    pub speed_factor: f64,
    pub jump_factor: f64,
}

impl BlockPhysicsProperties {
    /// `VoxelShape::empty()`, friction/speed/jump irrelevant (never read when the shape is
    /// empty) -- used both for `air` and for `BlockShapeSource`'s own out-of-bounds default.
    pub fn air() -> Self {
        todo!()
    }

    /// `VoxelShape::full_cube()`, `friction: 0.6, speed_factor: 1.0, jump_factor: 1.0` -- the
    /// registry's own default fallback row.
    pub fn default_full_cube() -> Self {
        todo!()
    }
}

/// Supplies block physics properties by position -- implemented outside this crate (Context:
/// "Shape-source seam"), never by `rc-physics` itself.
pub trait BlockShapeSource {
    /// Physics-relevant properties for the block at `pos`. Never panics; a caller with no
    /// data for `pos` (outside any currently-loaded chunk) returns `BlockPhysicsProperties::
    /// AIR` (Context: "Unloaded-position policy").
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties;
}

/// A closed, hand-authored `BlockStateId -> BlockPhysicsProperties` table (Context: the
/// tier-1 shape table). Not itself a `BlockShapeSource` -- a caller combines this table with
/// a chunk lookup (mapping a world position to the block-state id stored there) to build one.
pub struct ShapeTable {
    entries: HashMap<u32, BlockPhysicsProperties>,
}

impl ShapeTable {
    /// Builds a table from caller-supplied `(block_state_id, properties)` pairs -- the
    /// constructor-injection alternative Implementation step 2 reserves for when hardcoding
    /// raw ids directly into this crate proves awkward; `tier1_shape_table()` below uses it
    /// with this crate's own hardcoded tier-1 literals.
    pub fn from_entries(entries: Vec<(u32, BlockPhysicsProperties)>) -> Self {
        todo!()
    }

    /// `BlockPhysicsProperties::default_full_cube()` for any id with no explicit entry.
    pub fn lookup(&self, block_state_id: u32) -> BlockPhysicsProperties {
        todo!()
    }
}

static TIER1_TABLE: OnceLock<ShapeTable> = OnceLock::new();

/// The complete tier-1 table (Context's own listing table), built once. Every raw
/// `block_state_id` below except `air`'s own `0` is one of `rc_registries::generated_v776::
/// block_states::default_state::*`'s own constants (REDSTONE_WIRE, REDSTONE_TORCH,
/// REDSTONE_WALL_TORCH, REPEATER, COMPARATOR, PISTON, STICKY_PISTON, CHEST, HOPPER, FURNACE,
/// BLAST_FURNACE, SMOKER), read out-of-band and hardcoded here as plain `u32` literals --
/// this crate has no dependency on `rc-registries` (WS-D3 rule 1's shared-crate isolation),
/// so it cannot import those constants directly (Deliverables' own Implementation-step-2
/// guidance: "literal ids hardcoded directly into this crate... acceptable and preferred").
/// `air`'s own id (`0`, `AIR.0`, stable by protocol convention -- every registry's own id 0
/// is always its "empty"/default entry) is the one entry not itself part of Context's own
/// listing table but load-bearing all the same: without an explicit row, `lookup`'s own
/// default-full-cube fallback (correct for "any *other* unlisted block," i.e. ordinary
/// terrain) would wrongly resolve air as a solid block too.
///
/// **Default-state-only scope, a documented deviation from the blueprint's literal
/// "enumerated over every one of that block's own registered block states" instruction**:
/// `rc_registries::generated_v776::block_states` publishes only each block's *default*
/// state id, never a per-block id range or a reverse id -> block-type lookup, so there is no
/// way for this table (or its caller) to enumerate a listed block's other property
/// combinations (a repeater's other facings/delays/locked/powered states, etc.) without
/// either a registry-range table `xtask codegen` does not emit or an `rc-registries`
/// dependency this crate's isolation rule forbids. A raw id for one of these blocks in a
/// non-default state falls through to `default_full_cube()`, same as any ordinary block --
/// a real but narrow parity gap, bounded to this milestone's own hand-authored source (a)
/// (source (b), `xtask extract-shapes`, is explicitly deferred, Open Questions) and to a
/// state no code path in this crate's own consumer (`rusty-clanker-server`) currently
/// produces (`block_action.rs`'s own placement content is always plain `STONE`/`AIR`; none
/// of these tier-1 special-shaped blocks are ever placed anywhere in M3's world yet).
pub fn tier1_shape_table() -> &'static ShapeTable {
    todo!()
}

fn flat(shape: VoxelShape) -> BlockPhysicsProperties {
    todo!()
}

fn build_tier1_table() -> ShapeTable {
    todo!()
}
