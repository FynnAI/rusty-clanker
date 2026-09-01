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

    let mut entries = vec![
        // `air`'s own raw id (0) -- an explicit entry, not left to the registry's own
        // default-full-cube fallback (which is for "any *other* unlisted block," implicitly
        // assumed ordinary terrain, never air itself; without this row every `air` lookup
        // would wrongly resolve as a solid full cube).
        (0, empty),                          // air
        (2263, full.clone()),                // piston (extended = false)
        (2241, full.clone()),                // sticky_piston (extended = false)
        (3988, flat(chest_shape())),         // chest, facing North (direction_offset 0)
        (11313, flat(hopper_shape.clone())), // hopper, facing North (direction_offset 0)
        // Every other horizontal orientation `play::mining::apply_placement` can actually
        // write for these five blocks (M3 field-report fix, Root Cause 1 re-derivation --
        // supersedes this table's own former `direction_offset`-based rows, which used the
        // wrong facing value order/stride and so registered the wrong literal ids): real
        // vanilla 26.2 (protocol 776) ids, `play::mining::tier1_oriented_entries()`'s own
        // identical `<default-state id> + facing_idx*stride` arithmetic (chest stride 6,
        // facing order `[north, south, west, east]`; hopper stride 1, facing order `[down,
        // north, south, west, east]`) -- restated here by hand since this crate cannot import
        // that function (this table's own doc comment above). Each of these five blocks' own
        // shape is rotationally identical across every horizontal facing in this milestone's
        // own simplified per-block boxes (Context table) -- only the *id* changes per
        // facing, never the box -- so every row below reuses the same shape value the
        // facing-North (chest) / facing-Down (hopper) row above already registers.
        (3994, flat(chest_shape())),         // chest, facing South
        (4000, flat(chest_shape())),         // chest, facing West
        (4006, flat(chest_shape())),         // chest, facing East
        (11314, flat(hopper_shape.clone())), // hopper, facing North
        (11315, flat(hopper_shape.clone())), // hopper, facing South
        (11316, flat(hopper_shape.clone())), // hopper, facing West
        (11317, flat(hopper_shape)),         // hopper, facing East
        // piston_head (M3 field-report fix, Task 3: own-state writeback now writes the real
        // `minecraft:piston_head` id -- `crates/mechanics/src/redstone/piston.rs`'s own
        // `piston_head_id`/`PISTON_HEAD_BASE` doc comment has the full arithmetic citation,
        // read directly off `datagen-output/26.2/generated/reports/blocks.json`, protocol 776).
        // Twelve entries: one per (facing, sticky) pair, `short=false` only -- this project's
        // own writes never produce `short=true` (no intermediate `MOVING_PISTON` placeholder is
        // modeled, Context §D/§E), so that half of the real 24-state range is deliberately not
        // registered here (an unregistered id simply falls through to `default_full_cube()`,
        // never actually reached by any real write this table needs to serve). `sticky` never
        // changes the box (only `crates/mechanics/src/redstone/piston.rs`'s own `classify`
        // needs the distinction, for `Immovable`) -- both ids per facing reuse the identical
        // `piston_head_shape` call the facing alone determines, kept in sync by hand with that
        // module's own `piston_head_id` and with `piston_shape_table.rs`'s own local copy.
        (2271, flat(piston_head_shape(2, false))), // piston_head, normal, facing = North
        (2272, flat(piston_head_shape(2, false))), // piston_head, sticky, facing = North
        (2275, flat(piston_head_shape(0, true))),  // piston_head, normal, facing = East
        (2276, flat(piston_head_shape(0, true))),  // piston_head, sticky, facing = East
        (2279, flat(piston_head_shape(2, true))),  // piston_head, normal, facing = South
        (2280, flat(piston_head_shape(2, true))),  // piston_head, sticky, facing = South
        (2283, flat(piston_head_shape(0, false))), // piston_head, normal, facing = West
        (2284, flat(piston_head_shape(0, false))), // piston_head, sticky, facing = West
        (2287, flat(piston_head_shape(1, true))),  // piston_head, normal, facing = Up
        (2288, flat(piston_head_shape(1, true))),  // piston_head, sticky, facing = Up
        (2291, flat(piston_head_shape(1, false))), // piston_head, normal, facing = Down
        (2292, flat(piston_head_shape(1, false))), // piston_head, sticky, facing = Down
        (5328, full.clone()),                      // furnace
        (20763, full.clone()),                     // blast_furnace
        (20755, full),                             // smoker
    ];

    // `redstone_wire`'s own *entire* reachable id range (M3 field-report fix: wire's conductor
    // classification) -- blocks.json's own `minecraft:redstone_wire` entry (protocol 776) is
    // wire's full `power` (0..=15) x `east`/`north`/`south`/`west` (`up`/`side`/`none` each)
    // cross-product, ids `4011..=5306` contiguous (1296 states) -- follows this table's own
    // established "one row per real reachable id" range-registration precedent (the M3
    // oriented-shapes fix's own repeater/comparator/wall-torch/chest/hopper rows above), but
    // generated in a loop rather than hand-enumerated: every one of these 1296 states shares
    // the identical flat, non-full `wire_shape()` box (only the *id* varies, exactly as for
    // every other oriented-id row in this table). Registering only the single default id
    // (5171, the M3-B02/M3-B04-era placeholder this fix removes above) made
    // `rc_mechanics::redstone::signal::is_conductor` wrongly resolve every *other* reachable
    // wire id -- i.e. almost every powered or connected wire tile -- as a `default_full_cube()`
    // conductor, spuriously leaking quasi-connectivity through wire tiles vanilla never treats
    // as conductors at all (M3 field-report finding, `docs/findings-for-planning.md`'s own
    // "wire own-state writeback attempt reverted" entry) -- this is what let a wire's own
    // *stored* id ever move off 5171 (own-state writeback, this same field-report wave) without
    // corrupting every later signal computation at that position.
    entries.extend((4011u32..=5306).map(|id| (id, flat(wire_shape()))));

    // `repeater`'s and `comparator`'s own *entire* reachable id ranges (M3 field-report fix,
    // Rule D depower correctness investigation: "repeater/comparator conductor
    // misclassification" -- the same class of gap `docs/findings-for-planning.md`'s own "wire
    // own-state writeback attempt reverted" entry already named for wire above, but never
    // closed for these two diodes). The four hand-picked rows this table used to carry per
    // block (one per horizontal facing, always at `delay=1`/`mode=compare`, always
    // `locked=false`/`powered=false`) covered only each diode's own freshly-*placed* default
    // id -- every other reachable id (any other `delay` setting, `locked=true`, `powered=true`,
    // `mode=subtract`, ...) fell through `lookup`'s own `default_full_cube()` fallback,
    // wrongly classifying an ordinary locked-or-powered-or-non-default-delay repeater (or a
    // `subtract`-mode/powered comparator) as a *solid conductor* -- `is_conductor` reusing this
    // exact table (this file's own module doc comment above), so `emitted_toward`'s own
    // conductor branch would then relay a signal straight *through* that diode's `direct_
    // signal_to` scan of its six faces, bypassing the diode's own lock/power gating entirely.
    // Confirmed directly against this exact gap: `repeater_lock_release_repropagates`'s own
    // `(0, 1, 1)` wire reads a permanently-on `redstone_block` straight through a `LOCKED`
    // repeater sitting at delay=3 (id 7069, never one of the four hand-picked rows) the moment
    // that repeater's `LOCKED` bit flips, well before the repeater's own real `POWERED`
    // transition. `REPEATER_BASE..=REPEATER_MAX` / `COMPARATOR_BASE..=COMPARATOR_MAX`
    // (`crates/mechanics/src/redstone/repeater.rs`'s / `comparator.rs`'s own identical
    // constants, restated here by hand since this crate cannot import them, this table's own
    // doc comment above) both share the identical flat `low_slab()` box regardless of id (only
    // the *id* varies across `delay`/`facing`/`locked`/`powered` or `facing`/`mode`/`powered`,
    // exactly as for wire's own range just above).
    entries.extend((7034u32..=7097).map(|id| (id, flat(low_slab()))));
    entries.extend((11263u32..=11278).map(|id| (id, flat(low_slab()))));

    // `redstone_torch`'s and `redstone_wall_torch`'s own *entire* reachable id ranges (M3
    // field-report fix, same "conductor misclassification" gap class as the repeater/comparator
    // fix just above): the floor torch's own two hand-picked rows used to cover only `lit=true`
    // (`6885`), leaving `lit=false` (`6886`) to fall through to `default_full_cube()`; the wall
    // torch's own four hand-picked rows (one per horizontal facing) used to cover only that same
    // single default `lit` value per facing (`6887..=6890`), leaving the other four `lit=false`
    // combinations (`6891..=6894`) to fall through the exact same way. Confirmed directly
    // against this exact gap: `wire_strong_vs_weak_power_door`'s own `(9, 1, 0)` wall torch
    // turns `lit=false` (a real, reachable state this fixture's own contraption legitimately
    // reaches) and immediately gets misclassified as a solid conductor, re-broadcasting a
    // neighbor's direct signal through itself the same way an unregistered repeater/comparator
    // id did. Both torch shapes are identical regardless of `lit` (`torch_shape()`'s own doc
    // comment: floor and wall torches "share the same box" already) -- only the *id* varies.
    entries.extend((6885u32..=6886).map(|id| (id, flat(torch_shape()))));
    entries.extend((6887u32..=6894).map(|id| (id, flat(torch_shape()))));

    ShapeTable::from_entries(entries)
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
