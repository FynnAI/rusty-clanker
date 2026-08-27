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
/// **Every orientation the placement path can actually produce, not only each block's own
/// default state**: `rc_registries::generated_v776::block_states` publishes only each
/// block's *default* state id, never a per-block id range or a reverse id -> block-type
/// lookup, so this table cannot enumerate a listed block's *every* real registry state (a
/// repeater's own full facing/delay/locked/powered cross-product, etc. -- that gap is real
/// and stays open, `Open Questions`). It can, and now does, cover every id `rusty-clanker-
/// server`'s own placement path (`play::mining::apply_placement`, via `tier1_oriented_state_
/// table()`) actually writes into the world: repeater/comparator/redstone-wall-torch/chest
/// each get one row per `HORIZONTAL4` facing, and hopper one row per horizontal facing plus
/// its own `Full(Down)` id, below -- one row per `<default-state id> + direction_offset`,
/// the identical arithmetic `play::mining::tier1_oriented_entries()` uses to *write* that
/// same id, restated here by hand since this crate cannot import that function either (same
/// isolation rule). **The two tables must stay in sync by hand** -- `crates/server/tests/
/// mining_oriented_shape_table.rs` is the regression seam that catches them drifting apart
/// (M3 field-report Defect B: before this fix, every non-default orientation of these five
/// blocks silently fell through to `default_full_cube()`, wrongly making, e.g., a
/// South-facing repeater collide -- and, `rc_mechanics::redstone::signal::is_conductor`
/// reusing this exact table, conduct redstone -- like a solid block).
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
    let chest_shape = || {
        VoxelShape::from_boxes(vec![Aabb {
            min: Vec3::new(0.0625, 0.0, 0.0625),
            max: Vec3::new(0.9375, 0.875, 0.9375),
        }])
    };
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
        (0, empty),                          // air
        (5171, flat(wire_shape())),          // redstone_wire
        (6885, flat(torch_shape())),         // redstone_torch
        (6887, flat(torch_shape())),         // redstone_wall_torch
        (7037, flat(low_slab())),            // repeater, facing North (direction_offset 0)
        (11264, flat(low_slab())),           // comparator, facing North (direction_offset 0)
        (2263, full.clone()),                // piston (extended = false)
        (2241, full.clone()),                // sticky_piston (extended = false)
        (3988, flat(chest_shape())),         // chest, facing North (direction_offset 0)
        (11313, flat(hopper_shape.clone())), // hopper, facing North (direction_offset 0)
        // Every other horizontal orientation `play::mining::apply_placement` can actually
        // write for these five blocks (M3 field-report Defect B) -- `<default-state id> +
        // direction_offset`, `direction_offset(South) = 1, (East) = 2, (West) = 3`, the
        // identical arithmetic `play::mining::tier1_oriented_entries()` uses to *write* these
        // same ids (`direction_offset`'s own doc comment there: North=0, South=1, East=2,
        // West=3, Up=4, Down=5) -- restated here by hand since this crate cannot import that
        // function (this table's own doc comment above). Each of these five blocks' own
        // shape is rotationally identical across every horizontal facing in this milestone's
        // own simplified per-block boxes (Context table) -- only the *id* changes per
        // facing, never the box -- so every offset row below reuses the same shape value the
        // facing-North row above already registers.
        (6888, flat(torch_shape())), // redstone_wall_torch, facing South
        (6889, flat(torch_shape())), // redstone_wall_torch, facing East
        (6890, flat(torch_shape())), // redstone_wall_torch, facing West
        (7038, flat(low_slab())),    // repeater, facing South
        (7039, flat(low_slab())),    // repeater, facing East
        (7040, flat(low_slab())),    // repeater, facing West
        (11265, flat(low_slab())),   // comparator, facing South
        (11266, flat(low_slab())),   // comparator, facing East
        (11267, flat(low_slab())),   // comparator, facing West
        (3989, flat(chest_shape())), // chest, facing South
        (3990, flat(chest_shape())), // chest, facing East
        (3991, flat(chest_shape())), // chest, facing West
        (11314, flat(hopper_shape.clone())), // hopper, facing South
        (11315, flat(hopper_shape.clone())), // hopper, facing East
        (11316, flat(hopper_shape.clone())), // hopper, facing West
        // Hopper's own clamped-Down orientation (`play::mining::resolve_orientation`'s own
        // Hopper rule: a hopper placed against the top or bottom face of a neighbor always
        // faces Down, never Up) -- `HOPPER.0 + 10`, matching `tier1_oriented_entries()`'s
        // own identical `+ 10` offset (chosen there to sit safely past every `direction_
        // offset` value `0..=5` any *other* row for this same base id uses).
        (11323, flat(hopper_shape)), // hopper, facing Down
        // piston_head (M3-B05 Context §D) -- six placeholder ids, one per facing (no real
        // per-property-combination registry exists yet, Context §I; kept in sync by hand with
        // `crates/mechanics/src/redstone/piston.rs`'s own identical `PISTON_HEAD_IDS` table and
        // with `piston_shape_table.rs`'s own local copy). An *extended* piston/sticky_piston
        // base needs no entry of its own here (Context §D) -- it is an unchanged full cube,
        // already correctly produced by `lookup`'s own default-full-cube fallback.
        (900_001, flat(piston_head_shape(0, false))), // piston_head, facing = West
        (900_002, flat(piston_head_shape(0, true))),  // piston_head, facing = East
        (900_003, flat(piston_head_shape(2, false))), // piston_head, facing = North
        (900_004, flat(piston_head_shape(2, true))),  // piston_head, facing = South
        (900_005, flat(piston_head_shape(1, false))), // piston_head, facing = Down
        (900_006, flat(piston_head_shape(1, true))),  // piston_head, facing = Up
        (5328, full.clone()),                         // furnace
        (20763, full.clone()),                        // blast_furnace
        (20755, full),                                // smoker
    ])
}

/// `piston_head`'s own two-box shape (M3-B05 Context §D): a face plate (`PLATFORM_THICKNESS =
/// 4/16` thick, full footprint on the other two axes) at the *far* end of the block along
/// `axis` (the end `positive` points toward), plus a centered arm (`4/16 x 4/16` cross-section)
/// spanning the *near* `12/16`, connecting the face plate back toward the base. `axis`: `0` =
/// X, `1` = Y, `2` = Z. Worked reference case (`axis = 1, positive = true`, i.e. `facing = Up`):
/// face plate `[0,1]x[0.75,1]x[0,1]`; arm `[0.375,0.625]x[0,0.75]x[0.375,0.625]` -- matches
/// Context §D's own literal boxes exactly.
fn piston_head_shape(axis: usize, positive: bool) -> VoxelShape {
    const PLATFORM_THICKNESS: f64 = 0.25;
    const ARM_LO: f64 = 0.375;
    const ARM_HI: f64 = 0.625;

    let (plate_lo, plate_hi) = if positive {
        (1.0 - PLATFORM_THICKNESS, 1.0)
    } else {
        (0.0, PLATFORM_THICKNESS)
    };
    let (arm_lo, arm_hi) = if positive {
        (0.0, 1.0 - PLATFORM_THICKNESS)
    } else {
        (PLATFORM_THICKNESS, 1.0)
    };
    let other_axes: [usize; 2] = match axis {
        0 => [1, 2],
        1 => [0, 2],
        2 => [0, 1],
        _ => unreachable!("piston_head_shape: axis must be 0, 1, or 2"),
    };

    let make_box = |along: (f64, f64), a: (f64, f64), b: (f64, f64)| -> Aabb {
        let mut min = [0.0f64; 3];
        let mut max = [0.0f64; 3];
        min[axis] = along.0;
        max[axis] = along.1;
        min[other_axes[0]] = a.0;
        max[other_axes[0]] = a.1;
        min[other_axes[1]] = b.0;
        max[other_axes[1]] = b.1;
        Aabb {
            min: Vec3::new(min[0], min[1], min[2]),
            max: Vec3::new(max[0], max[1], max[2]),
        }
    };

    let plate = make_box((plate_lo, plate_hi), (0.0, 1.0), (0.0, 1.0));
    let arm = make_box((arm_lo, arm_hi), (ARM_LO, ARM_HI), (ARM_LO, ARM_HI));
    VoxelShape::from_boxes(vec![plate, arm])
}
