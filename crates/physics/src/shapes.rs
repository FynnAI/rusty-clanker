//! `VoxelShape` and the hand-authored tier-1 block-shape table (MECH-D38/D39, Context:
//! "VoxelShape representation and the tier-1 block-shape table").

use std::collections::HashMap;
use std::sync::OnceLock;

use rc_core::BlockPos;
use rc_registries::block_state_properties::{range_of, state_id};
use rc_registries::generated_v776::block_state_properties::{BlockId, block_id};
use rc_registries::generated_v776::block_states::default_state;

use crate::Aabb;
use crate::vec3::Vec3;

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

/// The complete tier-1 table (Context's own listing table), built once. M3.5-B02 (WS-D15):
/// every entry's own raw `block_state_id` is computed directly off `rc-registries`' M3.5-B01
/// generated per-block-state-property registry (`rc_registries::block_state_properties::
/// state_id`/`range_of`), not hand-copied literals -- `rc-physics` now depends on
/// `rc-registries` normally (WS-D3 rule 1: both are `SHARED` crates, so this is a sanctioned
/// additional edge, not a new exception, `crates/physics/Cargo.toml`'s own doc comment).
/// `air`'s own id (`default_state::AIR.0`, stable by protocol convention -- every registry's
/// own id 0 is always its "empty"/default entry) is the one entry not itself part of Context's
/// own listing table but load-bearing all the same: without an explicit row, `lookup`'s own
/// default-full-cube fallback (correct for "any *other* unlisted block," i.e. ordinary
/// terrain) would wrongly resolve air as a solid block too.
///
/// **Every orientation the placement path can actually produce, not only each block's own
/// default state**: repeater/comparator/redstone-wall-torch/chest each get one row per
/// `HORIZONTAL4` facing, and hopper one row per horizontal facing plus its own `Full(Down)`
/// id, below -- one row per `state_id(<block>, [("facing", <dir>), ...])`, computed against
/// the same generated registry `play::mining::tier1_oriented_entries()` reads to *write* that
/// same id, so the two can no longer silently drift apart the way the former pair of
/// hand-derived-arithmetic tables could (M3 field-report Defect B, `crates/server/tests/
/// mining_oriented_shape_table.rs`'s own doc comment has the full historical citation).
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

    // M3.5-B02 (WS-D15): every id below is computed against `rc-registries`' M3.5-B01
    // generated per-block-state-property registry (`state_id`/`range_of`) instead of a
    // hand-copied literal or hand-derived stride formula.
    fn id_of(block: BlockId, props: &[(&str, &str)]) -> u32 {
        state_id(block, props)
            .unwrap_or_else(|| {
                panic!("build_tier1_table: {props:?} is not a legal property set for {block:?}")
            })
            .0
    }

    const HORIZONTAL4: [&str; 4] = ["north", "south", "west", "east"];

    let mut entries = vec![
        // `air`'s own raw id -- an explicit entry, not left to the registry's own
        // default-full-cube fallback (which is for "any *other* unlisted block," implicitly
        // assumed ordinary terrain, never air itself; without this row every `air` lookup
        // would wrongly resolve as a solid full cube).
        (default_state::AIR.0, empty), // air
        (
            id_of(
                block_id::PISTON,
                &[("extended", "false"), ("facing", "north")],
            ),
            full.clone(),
        ), // piston (extended = false)
        (
            id_of(
                block_id::STICKY_PISTON,
                &[("extended", "false"), ("facing", "north")],
            ),
            full.clone(),
        ), // sticky_piston (extended = false)
    ];

    // Every horizontal orientation `play::mining::apply_placement` can actually write for
    // chest/hopper (M3 field-report fix, Root Cause 1 re-derivation, now computed against the
    // generated registry instead of restated stride arithmetic -- restated here by hand since
    // this crate cannot import `play::mining::tier1_oriented_entries()`, `rc-physics ->
    // rusty-clanker-server` being a forbidden dependency direction). Each of these two blocks'
    // own shape is rotationally identical across every horizontal facing in this milestone's
    // own simplified per-block boxes (Context table) -- only the *id* changes per facing, never
    // the box.
    for facing in HORIZONTAL4 {
        entries.push((
            id_of(block_id::CHEST, &[("facing", facing)]),
            flat(chest_shape()),
        ));
        entries.push((
            id_of(block_id::HOPPER, &[("facing", facing), ("enabled", "true")]),
            flat(hopper_shape.clone()),
        ));
        // Hopper `enabled=false` (M3-B0X hopper-ENABLED-at-placement fix): `enabled` never
        // changes hopper's own hitbox, only the *id*.
        entries.push((
            id_of(
                block_id::HOPPER,
                &[("facing", facing), ("enabled", "false")],
            ),
            flat(hopper_shape.clone()),
        ));
    }
    // Hopper's own placement-time default orientation (`Full(Down)`, `enabled=true`) --
    // already the block's own generated default id -- plus its `enabled=false` counterpart.
    entries.push((default_state::HOPPER.0, flat(hopper_shape.clone())));
    entries.push((
        id_of(
            block_id::HOPPER,
            &[("facing", "down"), ("enabled", "false")],
        ),
        flat(hopper_shape),
    ));

    // piston_head (M3 field-report fix, Task 3: own-state writeback writes the real
    // `minecraft:piston_head` id -- `crates/mechanics/src/redstone/piston.rs`'s own
    // `piston_head_id` doc comment has the full property citation). Twelve entries: one per
    // (facing, sticky) pair, `short=false` only -- this project's own writes never produce
    // `short=true` (no intermediate `MOVING_PISTON` placeholder is modeled, Context §D/§E), so
    // that half of the real 24-state range is deliberately not registered here (an
    // unregistered id simply falls through to `default_full_cube()`, never actually reached by
    // any real write this table needs to serve). `sticky` never changes the box (only
    // `crates/mechanics/src/redstone/piston.rs`'s own `classify` needs the distinction, for
    // `Immovable`) -- both ids per facing reuse the identical `piston_head_shape` call the
    // facing alone determines, kept in sync by hand with that module's own `piston_head_id`
    // and with `piston_shape_table.rs`'s own local copy.
    for (facing, axis, positive) in [
        ("north", 2, false),
        ("east", 0, true),
        ("south", 2, true),
        ("west", 0, false),
        ("up", 1, true),
        ("down", 1, false),
    ] {
        let shape = piston_head_shape(axis, positive);
        entries.push((
            id_of(
                block_id::PISTON_HEAD,
                &[("facing", facing), ("short", "false"), ("type", "normal")],
            ),
            flat(shape.clone()),
        ));
        entries.push((
            id_of(
                block_id::PISTON_HEAD,
                &[("facing", facing), ("short", "false"), ("type", "sticky")],
            ),
            flat(shape),
        ));
    }

    entries.push((default_state::FURNACE.0, full.clone()));
    entries.push((default_state::BLAST_FURNACE.0, full.clone()));
    entries.push((default_state::SMOKER.0, full));

    // `redstone_wire`'s own *entire* reachable id range (M3 field-report fix: wire's conductor
    // classification) -- blocks.json's own `minecraft:redstone_wire` entry is wire's full
    // `power` (0..=15) x `east`/`north`/`south`/`west` (`up`/`side`/`none` each) cross-product,
    // 1296 contiguous states, generated in a loop over the registry's own real range rather
    // than hand-enumerated: every one of these states shares the identical flat, non-full
    // `wire_shape()` box (only the *id* varies). Registering only the single default id (the
    // M3-B02/M3-B04-era placeholder this fix removes) made `rc_mechanics::redstone::signal::
    // is_conductor` wrongly resolve every *other* reachable wire id -- i.e. almost every
    // powered or connected wire tile -- as a `default_full_cube()` conductor, spuriously
    // leaking quasi-connectivity through wire tiles vanilla never treats as conductors at all
    // (M3 field-report finding, `docs/findings-for-planning.md`'s own "wire own-state
    // writeback attempt reverted" entry).
    let wire_range = range_of(block_id::REDSTONE_WIRE);
    entries.extend((wire_range.first.0..=wire_range.last.0).map(|id| (id, flat(wire_shape()))));

    // `repeater`'s and `comparator`'s own *entire* reachable id ranges (M3 field-report fix,
    // Rule D depower correctness investigation: "repeater/comparator conductor
    // misclassification" -- the same class of gap as wire's own range just above, but never
    // closed for these two diodes until that same fix). Every reachable id (any `delay`
    // setting, `locked`, `powered`, `mode`, ...) must resolve to the identical flat `low_slab()`
    // box regardless of id (only the *id* varies) -- otherwise `is_conductor` reusing this
    // exact table would relay a signal straight *through* an unregistered diode's own six-face
    // scan, bypassing its lock/power gating entirely (`repeater_lock_release_repropagates`'s
    // own `(0, 1, 1)` regression, confirmed directly against this exact gap).
    let repeater_range = range_of(block_id::REPEATER);
    entries
        .extend((repeater_range.first.0..=repeater_range.last.0).map(|id| (id, flat(low_slab()))));
    let comparator_range = range_of(block_id::COMPARATOR);
    entries.extend(
        (comparator_range.first.0..=comparator_range.last.0).map(|id| (id, flat(low_slab()))),
    );

    // `redstone_torch`'s and `redstone_wall_torch`'s own *entire* reachable id ranges (M3
    // field-report fix, same "conductor misclassification" gap class as the repeater/comparator
    // fix above): every `lit` value, and every wall-torch `facing`, must resolve to the
    // identical non-full `torch_shape()` box (floor and wall torches "share the same box"
    // already, `torch_shape()`'s own doc comment) -- otherwise an unregistered `lit=false`
    // torch gets misclassified as a solid conductor, re-broadcasting a neighbor's direct
    // signal through itself (`wire_strong_vs_weak_power_door`'s own `(9, 1, 0)` regression).
    let torch_floor_range = range_of(block_id::REDSTONE_TORCH);
    entries.extend(
        (torch_floor_range.first.0..=torch_floor_range.last.0).map(|id| (id, flat(torch_shape()))),
    );
    let torch_wall_range = range_of(block_id::REDSTONE_WALL_TORCH);
    entries.extend(
        (torch_wall_range.first.0..=torch_wall_range.last.0).map(|id| (id, flat(torch_shape()))),
    );

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
