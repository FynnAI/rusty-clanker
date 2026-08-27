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
        VoxelShape { boxes: Vec::new() }
    }

    /// The single-box full unit cube.
    pub fn full_cube() -> Self {
        VoxelShape {
            boxes: vec![Aabb {
                min: Vec3::new(0.0, 0.0, 0.0),
                max: Vec3::new(1.0, 1.0, 1.0),
            }],
        }
    }

    pub fn from_boxes(boxes: Vec<Aabb>) -> Self {
        VoxelShape { boxes }
    }

    pub fn boxes(&self) -> &[Aabb] {
        &self.boxes
    }

    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty()
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
        BlockPhysicsProperties {
            shape: VoxelShape::empty(),
            friction: 0.6,
            speed_factor: 1.0,
            jump_factor: 1.0,
        }
    }

    /// `VoxelShape::full_cube()`, `friction: 0.6, speed_factor: 1.0, jump_factor: 1.0` -- the
    /// registry's own default fallback row.
    pub fn default_full_cube() -> Self {
        BlockPhysicsProperties {
            shape: VoxelShape::full_cube(),
            friction: 0.6,
            speed_factor: 1.0,
            jump_factor: 1.0,
        }
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
        ShapeTable {
            entries: entries.into_iter().collect(),
        }
    }

    /// `BlockPhysicsProperties::default_full_cube()` for any id with no explicit entry.
    pub fn lookup(&self, block_state_id: u32) -> BlockPhysicsProperties {
        self.entries
            .get(&block_state_id)
            .cloned()
            .unwrap_or_else(BlockPhysicsProperties::default_full_cube)
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
    TIER1_TABLE.get_or_init(build_tier1_table)
}

fn flat(shape: VoxelShape) -> BlockPhysicsProperties {
    BlockPhysicsProperties {
        shape,
        friction: 0.6,
        speed_factor: 1.0,
        jump_factor: 1.0,
    }
}

fn build_tier1_table() -> ShapeTable {
    // Repeater/comparator: box [0,1]x[0,0.125]x[0,1] (Context table -- repeater height
    // wiki-confirmed; comparator shares a repeater-shaped body).
    let low_slab = || {
        VoxelShape::from_boxes(vec![Aabb {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(1.0, 0.125, 1.0),
        }])
    };
    // Redstone wire: a flat layer, full x/z footprint, y: 0..0.0625 (1/16 block) -- M3-B04
    // Context §B, refining this table's own original M3-B02 placeholder (`empty`, which is
    // *also* non-full and so did not break M3-B02's own is-conductor-free scope, but is not
    // wire's real hitbox).
    let wire_shape = || {
        VoxelShape::from_boxes(vec![Aabb {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(1.0, 0.0625, 1.0),
        }])
    };
    // Redstone torch (floor and wall share the same box -- M3-B04 Context §B: "wall torches:
    // the same box, offset toward the attached wall -- this blueprint's tests never depend on
    // the wall-torch box's exact horizontal offset, only that it is non-full"): a centered
    // post, x: 0.3125..0.6875, y: 0..0.625, z: 0.3125..0.6875.
    let torch_shape = || {
        VoxelShape::from_boxes(vec![Aabb {
            min: Vec3::new(0.3125, 0.0, 0.3125),
            max: Vec3::new(0.6875, 0.625, 0.6875),
        }])
    };
    // Chest: box [0.0625,0.9375]x[0,0.875]x[0.0625,0.9375] (Context table).
    let chest_shape = VoxelShape::from_boxes(vec![Aabb {
        min: Vec3::new(0.0625, 0.0, 0.0625),
        max: Vec3::new(0.9375, 0.875, 0.9375),
    }]);
    // Hopper: union of a top rim box and a simplified single funnel box (Context table).
    let hopper_shape = VoxelShape::from_boxes(vec![
        Aabb {
            min: Vec3::new(0.0, 0.625, 0.0),
            max: Vec3::new(1.0, 1.0, 1.0),
        },
        Aabb {
            min: Vec3::new(0.25, 0.25, 0.25),
            max: Vec3::new(0.75, 0.625, 0.75),
        },
    ]);
    let empty = BlockPhysicsProperties::air();
    let full = BlockPhysicsProperties::default_full_cube();

    ShapeTable::from_entries(vec![
        // `air`'s own raw id (0) -- an explicit entry, not left to the registry's own
        // default-full-cube fallback (which is for "any *other* unlisted block," implicitly
        // assumed ordinary terrain, never air itself; without this row every `air` lookup
        // would wrongly resolve as a solid full cube).
        (0, empty),                  // air
        (5171, flat(wire_shape())),  // redstone_wire
        (6885, flat(torch_shape())), // redstone_torch
        (6887, flat(torch_shape())), // redstone_wall_torch
        (7037, flat(low_slab())),    // repeater
        (11264, flat(low_slab())),   // comparator
        (2263, full.clone()),        // piston (extended = false)
        (2241, full.clone()),        // sticky_piston (extended = false)
        (3988, flat(chest_shape)),   // chest
        (11313, flat(hopper_shape)), // hopper
        (5328, full.clone()),        // furnace
        (20763, full.clone()),       // blast_furnace
        (20755, full),               // smoker
    ])
}
