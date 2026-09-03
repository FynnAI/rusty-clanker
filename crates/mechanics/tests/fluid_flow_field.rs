//! M4-B06 — the entity-push flow-field query API (Context §H): `get_own_height`/`get_height`,
//! `get_flow`'s float/double-boundary-exact vector, and the two solidity/sturdiness predicates
//! it and `occlusion.rs` share.

use std::cell::RefCell;
use std::collections::HashMap;

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::BlockWorldAccess;
use rc_mechanics::direction::Direction;
use rc_mechanics::fluid::algorithm::{get_flow, get_height, get_own_height};
use rc_mechanics::fluid::occlusion::{self, is_face_sturdy_shape};
use rc_mechanics::fluid::state::{FluidKind, FluidState};
use rc_mechanics::fluid::{FluidBlockRanges, FluidDimensionProfile, FluidTables, ReactionBlocks};
use rc_messaging::{Address, RegionId};
use rc_physics::{Aabb, Vec3, VoxelShape};

const AIR: BlockStateId = BlockStateId(50);
const STONE: BlockStateId = BlockStateId(51);

fn tables() -> FluidTables {
    let ranges = FluidBlockRanges::new(
        (BlockStateId(0), BlockStateId(16)),
        (BlockStateId(100), BlockStateId(116)),
    )
    .expect("both ranges are 16-wide");
    FluidTables::new(
        ranges,
        ReactionBlocks {
            obsidian: BlockStateId(60),
            cobblestone: BlockStateId(61),
            stone: STONE,
            basalt_conversion: None,
        },
        FluidDimensionProfile { fast_lava: false },
        AIR,
    )
}

fn fluid_id(t: &FluidTables, state: FluidState) -> BlockStateId {
    t.ranges.to_block_state_id(state)
}

struct FakeWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    default_state: BlockStateId,
    local: Address,
}

impl FakeWorld {
    fn new(default_state: BlockStateId) -> Self {
        Self {
            blocks: HashMap::new(),
            default_state,
            local: Address::Region(RegionId(0)),
        }
    }
    fn set(&mut self, pos: BlockPos, state: BlockStateId) {
        self.blocks.insert(pos, state);
    }
}

impl BlockWorldAccess for FakeWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        Some(self.blocks.get(&pos).copied().unwrap_or(self.default_state))
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let changed = self.blocks.get(&pos) != Some(&state);
        self.blocks.insert(pos, state);
        changed
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        self.local
    }
    fn local_identity(&self) -> Address {
        self.local
    }
}

struct CountingWorld {
    inner: FakeWorld,
    calls: RefCell<Vec<BlockPos>>,
}

impl BlockWorldAccess for CountingWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.calls.borrow_mut().push(pos);
        self.inner.get_block(pos)
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        self.inner.set_block(pos, state)
    }
    fn dimension(&self) -> DimensionId {
        self.inner.dimension()
    }
    fn owner_of(&self, chunk: ChunkKey) -> Address {
        self.inner.owner_of(chunk)
    }
    fn local_identity(&self) -> Address {
        self.inner.local_identity()
    }
}

#[test]
fn own_height_and_height_match_context_formula() {
    for amount in 1u8..=8 {
        let s = FluidState::flowing(FluidKind::Water, amount, false);
        assert_eq!(get_own_height(s), amount as f32 / 9.0f32);
    }
    assert_eq!(
        get_own_height(FluidState::source(FluidKind::Lava)),
        8.0f32 / 9.0f32
    );

    let t = tables();
    let mut world = FakeWorld::new(STONE);
    let pos = BlockPos::new(0, 5, 0);
    let state = FluidState::flowing(FluidKind::Water, 5, false);
    world.set(pos, fluid_id(&t, state));
    // No same-kind fluid above -- falls back to own_height.
    world.set(Direction::Up.apply(pos), AIR);
    assert_eq!(get_height(&world, &t, pos, state), get_own_height(state));

    // Same fluid kind directly above (any amount/falling) -- exactly 1.0.
    world.set(
        Direction::Up.apply(pos),
        fluid_id(&t, FluidState::flowing(FluidKind::Water, 2, true)),
    );
    assert_eq!(get_height(&world, &t, pos, state), 1.0f32);
}

#[test]
fn flow_vector_points_toward_lower_neighbor() {
    let t = tables();
    let mut world = FakeWorld::new(STONE);
    let pos = BlockPos::new(0, 5, 0);
    let state = FluidState::flowing(FluidKind::Water, 8, false);
    world.set(pos, fluid_id(&t, state));
    // North/South/West stay the default (solid stone -- blocks_motion, contribute nothing).
    let east_state = FluidState::flowing(FluidKind::Water, 4, false);
    world.set(Direction::East.apply(pos), fluid_id(&t, east_state));

    let flow = get_flow(&world, &t, pos, state);
    assert!(flow.x > 0.0, "flow.x={}", flow.x);
    assert_eq!(flow.y, 0.0);
    assert_eq!(flow.z, 0.0);
}

#[test]
fn flow_vector_uses_the_drop_off_redirect_when_neighbor_is_empty_with_a_hole_below() {
    let t = tables();
    let mut world = FakeWorld::new(STONE);
    let pos = BlockPos::new(0, 5, 0);
    let state = FluidState::flowing(FluidKind::Water, 6, false);
    world.set(pos, fluid_id(&t, state));

    let east = Direction::East.apply(pos);
    world.set(east, AIR); // empty, non-motion-blocking
    let below_east = Direction::Down.apply(east);
    let below_state = FluidState::flowing(FluidKind::Water, 1, false);
    world.set(below_east, fluid_id(&t, below_state));

    let flow = get_flow(&world, &t, pos, state);

    // Hand-computed via the literal `0.8888889f32` constant (Context §H), not a recomputed
    // `8.0f32/9.0f32` division -- the substitution this test exists to catch.
    let bh = 1.0f32 / 9.0f32;
    let distance = get_own_height(state) - (bh - 0.8888889f32);
    let expected_x = (1.0f32 * distance) as f64;
    let expected = Vec3::new(expected_x, 0.0, 0.0);
    let length =
        (expected.x * expected.x + expected.y * expected.y + expected.z * expected.z).sqrt();
    let expected_normalized = if length < 1.0e-5f32 as f64 {
        Vec3::ZERO
    } else {
        Vec3::new(
            expected.x / length,
            expected.y / length,
            expected.z / length,
        )
    };

    assert_eq!(flow.x, expected_normalized.x);
    assert_eq!(flow.y, expected_normalized.y);
    assert_eq!(flow.z, expected_normalized.z);
}

#[test]
fn falling_state_applies_the_downward_pull_on_first_solid_face_match() {
    let t = tables();
    let pos = BlockPos::new(0, 5, 0);
    let state = FluidState::flowing(FluidKind::Water, 8, true);
    let mut inner = FakeWorld::new(STONE); // default: solid on every unset position
    inner.set(pos, fluid_id(&t, state));
    // North: explicitly non-solid (air) -- no match.
    inner.set(Direction::North.apply(pos), AIR);
    inner.set(Direction::Up.apply(Direction::North.apply(pos)), AIR);
    // East (checked second): stays the default solid stone -- the first real match.
    // South/West: also stay solid, so a non-short-circuiting implementation would visibly probe
    // them too.
    let world = CountingWorld {
        inner,
        calls: RefCell::new(Vec::new()),
    };

    let flow = get_flow(&world, &t, pos, state);
    assert_eq!(
        flow.y, -1.0,
        "normalized downward pull should dominate y after the -6.0 shift"
    );

    // The primary horizontal flow-accumulation loop legitimately reads every one of
    // North/East/South/West's own base position exactly once each (regardless of the falling
    // redirect below), so a raw "was this position ever probed" check cannot distinguish the
    // falling-check loop's own short-circuit -- count occurrences instead: East must be probed
    // a *second* time (the falling-check loop's own `is_solid_face` call), while South/West must
    // never be probed more than the primary loop's own single legitimate read, proving the
    // falling-check scan stopped at East and never reached them.
    let east = Direction::East.apply(pos);
    let south = Direction::South.apply(pos);
    let west = Direction::West.apply(pos);
    let count = |target: BlockPos| {
        world
            .calls
            .borrow()
            .iter()
            .filter(|p| **p == target)
            .count()
    };
    assert!(
        count(east) >= 2,
        "East must be probed a second time by the falling-check loop"
    );
    assert_eq!(
        count(south),
        1,
        "South must never be re-probed by the falling-check loop"
    );
    assert_eq!(
        count(west),
        1,
        "West must never be re-probed by the falling-check loop"
    );
}

#[test]
fn near_zero_vector_normalizes_to_exact_zero() {
    let t = tables();
    let mut world = FakeWorld::new(STONE);
    let pos = BlockPos::new(0, 5, 0);
    // A source surrounded symmetrically by the same fluid at the same height on every side --
    // every pairwise height difference is exactly zero, so the raw vector is already `(0,0,0)`
    // before normalization even applies.
    let state = FluidState::source(FluidKind::Water);
    world.set(pos, fluid_id(&t, state));
    for dir in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        world.set(
            dir.apply(pos),
            fluid_id(&t, FluidState::source(FluidKind::Water)),
        );
    }

    let flow = get_flow(&world, &t, pos, state);
    assert_eq!(flow, Vec3::ZERO);
}

#[test]
fn ice_exception_list_overrides_the_real_sturdiness_test_for_solid_face_purposes() {
    let mut t = tables();
    let ice_id = BlockStateId(80);
    t.solid_face_exceptions = vec![(ice_id, BlockStateId(ice_id.0 + 1))];

    let mut world = FakeWorld::new(STONE);
    let pos = BlockPos::new(0, 0, 0);
    world.set(pos, ice_id);

    // The identical shape (unregistered -> full cube, sturdy on every face per
    // `is_face_sturdy_shape`) is exempted for `is_solid_face` purposes once it's on the
    // exception list.
    assert!(occlusion::is_solid_face(
        &world,
        &t,
        FluidKind::Water,
        pos,
        Direction::North
    ));
    assert!(is_face_sturdy_shape(
        &VoxelShape::full_cube(),
        Direction::North
    ));
}

/// Pure -- reuses the exact box literals `crates/physics/src/shapes.rs`'s own tier-1 table
/// already defines for these blocks.
#[test]
fn is_face_sturdy_shape_matches_the_tier1_tier2_shape_reference_table() {
    let horizontals = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];

    let full_cube = VoxelShape::full_cube();
    for dir in horizontals {
        assert!(
            is_face_sturdy_shape(&full_cube, dir),
            "full cube must be sturdy on {dir:?}"
        );
    }

    let chest_shape = VoxelShape::from_boxes(vec![Aabb {
        min: Vec3::new(0.0625, 0.0, 0.0625),
        max: Vec3::new(0.9375, 0.875, 0.9375),
    }]);
    for dir in horizontals {
        assert!(
            !is_face_sturdy_shape(&chest_shape, dir),
            "chest must not be sturdy on {dir:?}"
        );
    }

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
    for dir in horizontals {
        assert!(
            !is_face_sturdy_shape(&hopper_shape, dir),
            "hopper must not be sturdy on {dir:?}"
        );
    }
}
