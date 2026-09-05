//! `VoxelShape` and the hand-authored tier-1 block-shape table (MECH-D38/D39, Context:
//! "VoxelShape representation and the tier-1 block-shape table").

use std::collections::HashMap;
use std::sync::OnceLock;

use rc_core::BlockPos;
use rc_registries::block_state_properties::{range_of, state_id};
use rc_registries::generated_v776::block_state_properties::{BlockId, block_id};
use rc_registries::generated_v776::block_states::default_state;

use crate::Aabb;
use crate::aabb::Axis;
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

    /// MECH-D84's per-face sturdiness predicate -- a literal port of vanilla's `SupportType`
    /// enum (`SupportType.FULL`/`CENTER`/`RIGID`, each an `isSupporting` override) together
    /// with the `VoxelShape.getFaceShape`/`calculateFace`/`SliceShape` machinery every one of
    /// those three reads through first.
    ///
    /// The face shape for `face` is the union, over every box of `self` whose extent on
    /// `face`'s own axis reaches that axis's literal boundary coordinate (`1.0` for `Up`/
    /// `South`/`East`, `0.0` for `Down`/`West`/`North`, within `crate::SHAPE_EPSILON`), of that
    /// box's rectangle projected onto the other two axes -- `calculateFace`'s own slice taken
    /// at the grid layer immediately adjacent to the boundary. A box that stops short of the
    /// boundary contributes nothing, and if no box reaches it at all the face shape is empty
    /// (`calculateFace`'s own `slice.isEmpty()` early return) -- e.g. an extended piston
    /// base's recessed end (the missing 4/16 slab) or a chest's own top (its hitbox tops out
    /// at 14/16, never touching `y = 1`) never touch the boundary at all, so nothing can rest,
    /// stand, or attach there regardless of `kind`. This is the SAME face shape for every
    /// `kind` -- vanilla computes `getFaceShape` once per `isSupporting` call, always from the
    /// block's own unmodified support shape, never a kind-dependent "closest surface"
    /// substitute.
    ///
    /// `kind` then asks whether that one face shape covers a required in-plane region
    /// (`Shapes.joinIsNotEmpty(faceShape, requiredShape, ONLY_SECOND)` being empty, i.e. no
    /// point of the required region lies outside the face shape) -- `SupportKind::
    /// face_shape_covers`'s own doc comment has the exact three regions.
    pub fn face_sturdy(&self, face: Face, kind: SupportKind) -> bool {
        let (axis, positive) = face.axis_and_sign();
        let boundary = if positive { 1.0 } else { 0.0 };
        let touching: Vec<&Aabb> = self
            .boxes
            .iter()
            .filter(|b| (face_extent(b, axis, positive) - boundary).abs() < crate::SHAPE_EPSILON)
            .collect();
        if touching.is_empty() {
            return false;
        }
        let (a1, a2) = axis.other_two();
        let rects: Vec<(f64, f64, f64, f64)> = touching
            .iter()
            .map(|b| (b.min(a1), b.max(a1), b.min(a2), b.max(a2)))
            .collect();
        kind.face_shape_covers(&rects)
    }
}

/// `b`'s own extent on `axis` in the direction `face` points -- `max` for a positive-axis
/// face (`Up`/`South`/`East`), `min` for a negative one (`Down`/`North`/`West`).
fn face_extent(b: &Aabb, axis: Axis, positive: bool) -> f64 {
    if positive { b.max(axis) } else { b.min(axis) }
}

/// `true` iff every point `required` selects (as `(a, b)` coordinates on the face's own two
/// in-plane axes, each in `[0, 1]`) is covered by the union of `rects` (each `(min_a, max_a,
/// min_b, max_b)`) -- an exact check, not a sampled approximation: `anchors` (the required
/// region's own breakpoints) plus every rectangle's own boundary coordinate become grid
/// lines, so testing one interior point per resulting grid cell against both `required` and
/// every rectangle is equivalent to testing the whole cell (no rectangle edge, and no
/// `required`-region breakpoint, ever crosses a cell's interior -- every tier-1 shape and
/// every `SupportKind` breakpoint used here is pixel-aligned, a multiple of 1/16, so this grid
/// construction is exact for every case this table can actually produce).
fn region_is_covered(
    rects: &[(f64, f64, f64, f64)],
    anchors: &[f64],
    required: impl Fn(f64, f64) -> bool,
) -> bool {
    let collect_lines = |pick_lo: fn(&(f64, f64, f64, f64)) -> f64,
                         pick_hi: fn(&(f64, f64, f64, f64)) -> f64|
     -> Vec<f64> {
        let mut lines: Vec<f64> = vec![0.0, 1.0];
        lines.extend_from_slice(anchors);
        for r in rects {
            let a = pick_lo(r);
            let b = pick_hi(r);
            if a > 0.0 && a < 1.0 {
                lines.push(a);
            }
            if b > 0.0 && b < 1.0 {
                lines.push(b);
            }
        }
        lines.sort_by(|x, y| x.partial_cmp(y).unwrap());
        lines.dedup_by(|x, y| (*x - *y).abs() < crate::SHAPE_EPSILON);
        lines
    };
    let a_lines = collect_lines(|r| r.0, |r| r.1);
    let b_lines = collect_lines(|r| r.2, |r| r.3);
    for a_win in a_lines.windows(2) {
        for b_win in b_lines.windows(2) {
            let ca = (a_win[0] + a_win[1]) / 2.0;
            let cb = (b_win[0] + b_win[1]) / 2.0;
            if !required(ca, cb) {
                continue;
            }
            let covered = rects
                .iter()
                .any(|&(a0, a1, b0, b1)| a0 <= ca && ca <= a1 && b0 <= cb && cb <= b1);
            if !covered {
                return false;
            }
        }
    }
    true
}

/// The six faces of a unit block (Context: MECH-D84's per-face sturdiness predicate) -- this
/// crate's own minimal direction vocabulary. `rc-mechanics::direction::Direction` cannot be
/// used here: `rc-physics` sits below `rc-mechanics` in the crate graph (WS-D3), so that
/// dependency edge would run backwards -- every caller translates its own `Direction` into
/// this type at the call site instead (`rc-mechanics::redstone::signal::is_face_sturdy`'s own
/// doc comment).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Face {
    /// The axis this face is perpendicular to, and whether it sits at that axis's
    /// positive-coordinate end (`Up`/`South`/`East`) or negative (`Down`/`North`/`West`).
    fn axis_and_sign(self) -> (Axis, bool) {
        match self {
            Face::Down => (Axis::Y, false),
            Face::Up => (Axis::Y, true),
            Face::North => (Axis::Z, false),
            Face::South => (Axis::Z, true),
            Face::West => (Axis::X, false),
            Face::East => (Axis::X, true),
        }
    }
}

/// Vanilla's three face-sturdiness kinds (Context/MECH-D84), each naming the in-plane region
/// a face shape must cover (`VoxelShape::face_sturdy`'s own doc comment has the exact
/// algorithm) -- `SupportType.FULL`/`CENTER`/`RIGID` in the reference (`Block.isFaceFull`,
/// `CENTER_SUPPORT_SHAPE`, `RIGID_SUPPORT_SHAPE`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SupportKind {
    Full,
    Center,
    Rigid,
}

impl SupportKind {
    /// `true` iff the union of `rects` (a face shape's in-plane footprint, `VoxelShape::
    /// face_sturdy`'s own doc comment) covers every point this kind requires: the whole unit
    /// square for `Full` (`Block.isFaceFull`/`isShapeFullBlock` -- the face shape must equal
    /// the full `[0,1]x[0,1]` square); the centred 2x2-pixel square, 7/16..9/16 on both
    /// in-plane axes, for `Center` (`CENTER_SUPPORT_SHAPE` = `Block.column(2, 0, 10)`);
    /// everything *outside* the centred 12x12-pixel square, 2/16..14/16 on both axes -- the
    /// outer 2px border frame -- for `Rigid` (`RIGID_SUPPORT_SHAPE` = the full block minus
    /// `Block.column(12, 0, 16)`, i.e. `Shapes.join(Shapes.block(), ..., ONLY_FIRST)`). All
    /// three reduce to `region_is_covered` below, each with its own `required` predicate and
    /// the breakpoints that predicate needs as extra grid anchors.
    fn face_shape_covers(self, rects: &[(f64, f64, f64, f64)]) -> bool {
        match self {
            SupportKind::Full => region_is_covered(rects, &[], |_, _| true),
            SupportKind::Center => {
                const LO: f64 = 7.0 / 16.0;
                const HI: f64 = 9.0 / 16.0;
                region_is_covered(rects, &[LO, HI], |a, b| {
                    (LO..=HI).contains(&a) && (LO..=HI).contains(&b)
                })
            }
            SupportKind::Rigid => {
                const LO: f64 = 2.0 / 16.0;
                const HI: f64 = 14.0 / 16.0;
                region_is_covered(rects, &[LO, HI], |a, b| {
                    !(LO..=HI).contains(&a) || !(LO..=HI).contains(&b)
                })
            }
        }
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

    /// MECH-D84: `true` iff `block_state_id`'s own shape is sturdy on `face` for `kind` --
    /// `VoxelShape::face_sturdy`'s own doc comment has the exact algorithm. An id with no
    /// explicit entry falls through to `lookup`'s own `default_full_cube` fallback, whose
    /// shape is sturdy on every face for every kind (correct for ordinary terrain).
    pub fn is_face_sturdy(&self, block_state_id: u32, face: Face, kind: SupportKind) -> bool {
        self.lookup(block_state_id).shape.face_sturdy(face, kind)
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
    // Hopper (M3 field-report fix, MECH-D84: the former 2-box version's own top rim box was a
    // solid, hole-less slab -- reading as a `Full`-sturdy top face, wrongly letting dust
    // survive on a hopper by shape alone rather than only through the dedicated hard-coded
    // exception `WireBehavior::should_pop` now applies. `HopperBlock`'s real reference shape
    // is the rim/funnel outline (unchanged from the former 2-box union, both boxes restated
    // below) minus a hollow scooped out of the rim's own top -- carved here as four border
    // strips plus the thin full-footprint floor slab beneath them, in place of one solid
    // top box, since `VoxelShape` has no boolean subtraction: floor slab
    // [0,1]x[0.625,0.6875]x[0,1] (the rim's own bottom 1/16, below the hollow, still solid
    // full-footprint), then the hollow's own four remaining border strips at
    // y:[0.6875,1.0] -- north/south full-width ([0,1]x[0,0.125] / [0.875,1]), west/east
    // restricted to the band between them ([0,0.125]/[0.875,1] x [0.125,0.875]) -- and the
    // funnel box, unchanged.
    let hopper_shape = VoxelShape::from_boxes(vec![
        Aabb {
            min: Vec3::new(0.0, 0.625, 0.0),
            max: Vec3::new(1.0, 0.6875, 1.0),
        },
        Aabb {
            min: Vec3::new(0.0, 0.6875, 0.0),
            max: Vec3::new(1.0, 1.0, 0.125),
        },
        Aabb {
            min: Vec3::new(0.0, 0.6875, 0.875),
            max: Vec3::new(1.0, 1.0, 1.0),
        },
        Aabb {
            min: Vec3::new(0.0, 0.6875, 0.125),
            max: Vec3::new(0.125, 1.0, 0.875),
        },
        Aabb {
            min: Vec3::new(0.875, 0.6875, 0.125),
            max: Vec3::new(1.0, 1.0, 0.875),
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

    // `(facing, axis, positive)` for every one of piston's six facings -- `axis`/`positive`
    // give the facing's own axis (`0`=X, `1`=Y, `2`=Z) and whether it points toward that
    // axis's positive end, shared by `piston_head_shape` (below) and the extended-base rows
    // (MECH-D84) alike, so the two never drift apart the way the former hand-duplicated pair
    // could.
    const PISTON_FACINGS: [(&str, usize, bool); 6] = [
        ("north", 2, false),
        ("east", 0, true),
        ("south", 2, true),
        ("west", 0, false),
        ("up", 1, true),
        ("down", 1, false),
    ];

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

    // --- Lever (M3 field-report wave 3, PLAN-D10/MECH-D13) — own clearly delimited section,
    // deliberately placed BEFORE piston_head below (a future blueprint's own `moving_piston`
    // rows land at the END of this function instead, per that blueprint's own convention). All
    // 24 real states (3 `face` values x 4 `facing` values x 2 `powered` values) -- non-full,
    // non-conductor, not sturdy on any face for any `SupportKind` (`lever_shape`'s own doc
    // comment, below, has the full per-`(face, facing)` box derivation).
    for face in ["floor", "wall", "ceiling"] {
        for facing in HORIZONTAL4 {
            let shape = flat(lever_shape(face, facing));
            for powered in ["true", "false"] {
                entries.push((
                    id_of(
                        block_id::LEVER,
                        &[("face", face), ("facing", facing), ("powered", powered)],
                    ),
                    shape.clone(),
                ));
            }
        }
    }
    // --- end Lever ---------------------------------------------------------------------------

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
    for (facing, axis, positive) in PISTON_FACINGS {
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

    // Extended piston/sticky_piston bases (M3 field-report fix, MECH-D84): twelve entries, one
    // per (block, facing) pair -- a real piston base id ever reaches `extended=true` only
    // through this project's own writeback (`piston.rs`'s `write_base_extended`), never
    // placement, so there is no `sticky`-crossed-with-`extended=false` case to add here.
    // `PistonBaseBlock`'s own real reference shape (`Block.boxZ(16,4,16)` rotated per facing
    // via `Shapes.rotateAll`): a single box, full on the two non-facing axes, and on the
    // facing axis `[0, 0.75]` when `positive` (the missing 4/16 slab sits at that axis's far,
    // positive end -- the piston pushed outward from there) or `[0.25, 1]` otherwise (the
    // missing slab at the negative end) -- verified against `piston_head_shape`'s own
    // identical per-facing `(axis, positive)` table just above, which the same twelve real ids
    // now share.
    for (facing, axis, positive) in PISTON_FACINGS {
        let shape = piston_base_extended_shape(axis, positive);
        entries.push((
            id_of(
                block_id::PISTON,
                &[("extended", "true"), ("facing", facing)],
            ),
            flat(shape.clone()),
        ));
        entries.push((
            id_of(
                block_id::STICKY_PISTON,
                &[("extended", "true"), ("facing", facing)],
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

    // M3 field-report wave 3 (PLAN-D10, moving_piston placeholder — MECH-D83/MECH-D84): the
    // twelve real `minecraft:moving_piston` states (six `facing` values crossed with `type=
    // normal|sticky`; no `short` property, unlike `piston_head`) each get the identical empty
    // shape, in a separate block at the end of this builder (kept deliberately last and
    // self-contained so a sibling changeset's own additions elsewhere in this function merge
    // cleanly). Verified directly against the decompiled reference: `MovingPistonBlock.getShape`
    // is unconditionally `Shapes.empty()`, regardless of the block entity's own progress-
    // dependent state -- a fresh `BlockPhysicsProperties::air()` per row (the same "shape absent,
    // other properties immaterial" value `air`'s own entry above already uses, though that one
    // is moved rather than cloned there, so a fresh instance is built here instead) gives every
    // moving_piston id no support on any face (`is_face_sturdy`, MECH-D84 -- the wire that
    // pops the instant this placeholder appears reads exactly this) and never resolves as the
    // redstone-conductor full-cube shape (`signal::is_conductor`'s own doc comment: any shape
    // other than a single `(0,0,0)..(1,1,1)` box, empty included, is never a conductor) --
    // `MovingPistonBlock`'s own real `getBlockSupportShape`/`isRedstoneConductor` are never
    // overridden either, and this project's own single-shape-table convention already makes an
    // empty shape both "no support" and "not a conductor" simultaneously, so no separate override
    // is needed here. The block entity's own real, progress-dependent COLLISION shape (entity
    // displacement during the animation) stays out of scope -- a separate M4 item, unmodeled by
    // this project's own simplified single-shape-table architecture regardless.
    let moving_piston_range = range_of(block_id::MOVING_PISTON);
    entries.extend(
        (moving_piston_range.first.0..=moving_piston_range.last.0)
            .map(|id| (id, BlockPhysicsProperties::air())),
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
/// An extended piston/sticky_piston base's own single-box shape (M3 field-report fix,
/// MECH-D84): full on the two axes other than `axis`, and on `axis` itself `[0, 0.75]` when
/// `positive` (the missing 4/16 slab at that axis's positive end) or `[0.25, 1]` otherwise (the
/// missing slab at the negative end) -- `axis`/`positive` share `piston_head_shape`'s own
/// convention (`0`=X, `1`=Y, `2`=Z).
fn piston_base_extended_shape(axis: usize, positive: bool) -> VoxelShape {
    let (lo, hi) = if positive { (0.0, 0.75) } else { (0.25, 1.0) };
    let mut min = [0.0f64; 3];
    let mut max = [1.0f64; 3];
    min[axis] = lo;
    max[axis] = hi;
    VoxelShape::from_boxes(vec![Aabb {
        min: Vec3::new(min[0], min[1], min[2]),
        max: Vec3::new(max[0], max[1], max[2]),
    }])
}

/// The lever's own single box for one `(face, facing)` pair (M3 field-report wave 3,
/// PLAN-D10/MECH-D13), identical for `powered=true`/`false` (Context: the client-visible
/// paddle angle never changes the collision hitbox). Base box, `wall`/`north` (in sixteenths):
/// `X[5,11] Y[4,12] Z[10,16]` — touches the South boundary (`z=1`), the mount side for
/// `facing=north` (mount = `facing.opposite()` = South, `lever.rs`'s own `mount_direction` doc
/// comment). Every other `(face, facing)` pair rotates this same box (verified against the
/// ASSET-D18(f) reference's own `Shapes.rotateAttachFace` + per-facing horizontal rotation,
/// restated here as plain box arithmetic rather than a generic rotation utility):
/// - `wall` rotates horizontally, around the vertical (Y) axis, per facing: the perpendicular
///   in-plane width (`X[5,11]` for north/south, the identical interval placed on Z for
///   east/west) is unaffected, only which boundary the depth axis touches changes (the mount
///   side, `facing.opposite()`).
/// - `floor`/`ceiling` tip the wall box 90 degrees about the horizontal axis perpendicular to
///   both the mount direction and the box's own vertical extent: the former "vertical along the
///   wall" interval (`Y[4,12]`) becomes the new in-plane interval along the facing axis
///   (`Z[4,12]` for north/south facing, `X[4,12]` for east/west), and the former "depth from the
///   wall" interval (`Z[10,16]`, touching the mount boundary at its high end) becomes the new
///   vertical interval touching the floor (`Y[0,6]`) or ceiling (`Y[10,16]`, the mirrored high
///   end) — the horizontal footprint is identical between `floor` and `ceiling` for the same
///   facing; only the vertical placement differs.
fn lever_shape(face: &str, facing: &str) -> VoxelShape {
    let one_box = |min: (f64, f64, f64), max: (f64, f64, f64)| {
        VoxelShape::from_boxes(vec![Aabb {
            min: Vec3::new(min.0, min.1, min.2),
            max: Vec3::new(max.0, max.1, max.2),
        }])
    };
    match (face, facing) {
        ("wall", "north") => one_box((0.3125, 0.25, 0.625), (0.6875, 0.75, 1.0)),
        ("wall", "south") => one_box((0.3125, 0.25, 0.0), (0.6875, 0.75, 0.375)),
        ("wall", "west") => one_box((0.625, 0.25, 0.3125), (1.0, 0.75, 0.6875)),
        ("wall", "east") => one_box((0.0, 0.25, 0.3125), (0.375, 0.75, 0.6875)),
        ("floor", "north") | ("floor", "south") => {
            one_box((0.3125, 0.0, 0.25), (0.6875, 0.375, 0.75))
        }
        ("floor", "west") | ("floor", "east") => {
            one_box((0.25, 0.0, 0.3125), (0.75, 0.375, 0.6875))
        }
        ("ceiling", "north") | ("ceiling", "south") => {
            one_box((0.3125, 0.625, 0.25), (0.6875, 1.0, 0.75))
        }
        ("ceiling", "west") | ("ceiling", "east") => {
            one_box((0.25, 0.625, 0.3125), (0.75, 1.0, 0.6875))
        }
        _ => {
            unreachable!("lever_shape: {face:?}/{facing:?} is not a real lever (face,facing) pair")
        }
    }
}

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
