//! M4-B03 Acceptance tests: A* pathfinding over a `WalkNodeEvaluator`-classified
//! navigation graph (MECH-D33, Context §F) -- hand-derived golden-path node sequences
//! plus the qualitative corridor/multi-obstacle cases M4 roadmap criterion 3's own
//! standard covers.

use std::collections::HashMap;

use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::ai::pathfinding::node::{PathType, WalkNodeEvaluator};
use rc_mechanics::ai::{find_path, FUDGING};
use rc_mechanics::world_access::BlockWorldAccess;
use rc_messaging::Address;
use rc_registries::generated_v776::block_states::BlockStateId as RegBlockStateId;
use rc_registries::generated_v776::block_states::default_state;

/// `rc_chunk_storage::BlockStateId` and `rc_registries::generated_v776::block_states::
/// BlockStateId` are numerically identical but textually distinct types (WORLD-D3/D4's
/// own "resolved discrepancy") -- `BlockWorldAccess::get_block` returns the former,
/// `rc_registries`' own generated `default_state`/`block_state_properties` tables
/// speak the latter. This test double stores/exposes the registries-crate flavor
/// (matching every call site's own convenience) and converts only at the trait
/// boundary.
fn to_storage(id: RegBlockStateId) -> rc_chunk_storage::BlockStateId {
    rc_chunk_storage::BlockStateId(id.0)
}

/// A flat, `ground_y`-floored world (`default_state::STONE` at `ground_y`,
/// `default_state::AIR` everywhere else) with explicit per-position overrides for
/// obstacles/hazards -- mirrors `crates/mechanics/tests/support/mod.rs`'s own
/// `FluidFakeWorld` "default row + overrides" precedent.
struct PathfindingFakeWorld {
    ground_y: i32,
    overrides: HashMap<BlockPos, RegBlockStateId>,
}

impl PathfindingFakeWorld {
    fn new(ground_y: i32) -> Self {
        PathfindingFakeWorld {
            ground_y,
            overrides: HashMap::new(),
        }
    }

    fn set(&mut self, pos: BlockPos, state: RegBlockStateId) {
        self.overrides.insert(pos, state);
    }
}

impl BlockWorldAccess for PathfindingFakeWorld {
    fn get_block(&self, pos: BlockPos) -> Option<rc_chunk_storage::BlockStateId> {
        if let Some(&id) = self.overrides.get(&pos) {
            return Some(to_storage(id));
        }
        if pos.y == self.ground_y {
            Some(to_storage(default_state::STONE))
        } else {
            Some(to_storage(default_state::AIR))
        }
    }
    fn set_block(&mut self, _pos: BlockPos, _state: rc_chunk_storage::BlockStateId) -> bool {
        false
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        Address::Region(rc_messaging::RegionId(0))
    }
    fn local_identity(&self) -> Address {
        Address::Region(rc_messaging::RegionId(0))
    }
}

const ENTITY_HEIGHT: f32 = 1.95;

fn no_malus() -> HashMap<PathType, f32> {
    HashMap::new()
}

#[test]
fn straight_line_open_ground() {
    let world = PathfindingFakeWorld::new(63);
    let start = BlockPos::new(0, 64, 0);
    let target = BlockPos::new(5, 64, 0);

    let outcome = find_path(
        start,
        &[target],
        0.0,
        &WalkNodeEvaluator,
        &world,
        ENTITY_HEIGHT,
        &no_malus(),
        1000,
    );

    let path = outcome.path.expect("a path was found");
    assert_eq!(
        path.nodes(),
        &[
            BlockPos::new(0, 64, 0),
            BlockPos::new(1, 64, 0),
            BlockPos::new(2, 64, 0),
            BlockPos::new(3, 64, 0),
            BlockPos::new(4, 64, 0),
            BlockPos::new(5, 64, 0),
        ]
    );
    assert!(outcome.target_reached);
}

#[test]
fn single_block_obstacle_detours_around_not_through() {
    let mut world = PathfindingFakeWorld::new(63);
    world.set(BlockPos::new(2, 64, 0), default_state::STONE);
    world.set(BlockPos::new(2, 65, 0), default_state::STONE);
    let start = BlockPos::new(0, 64, 0);
    let target = BlockPos::new(4, 64, 0);

    let outcome = find_path(
        start,
        &[target],
        0.0,
        &WalkNodeEvaluator,
        &world,
        ENTITY_HEIGHT,
        &no_malus(),
        1000,
    );

    let path = outcome.path.expect("a path was found");
    assert!(outcome.target_reached);
    assert!(
        !path.nodes().contains(&BlockPos::new(2, 64, 0)),
        "never steps into the solid wall"
    );
    let detoured = path.nodes().contains(&BlockPos::new(2, 64, 1))
        || path.nodes().contains(&BlockPos::new(2, 64, -1));
    assert!(detoured, "routes laterally around the wall");
    // A naive doubled-back route (e.g. retreating and re-approaching) would need
    // strictly more nodes than the shortest one-step lateral detour + return.
    assert!(path.nodes().len() < 10);
}

#[test]
fn diagonal_corner_cutting_is_rejected() {
    let mut world = PathfindingFakeWorld::new(63);
    world.set(BlockPos::new(1, 64, 0), default_state::STONE);
    world.set(BlockPos::new(1, 65, 0), default_state::STONE);
    world.set(BlockPos::new(0, 64, 1), default_state::STONE);
    world.set(BlockPos::new(0, 65, 1), default_state::STONE);
    let start = BlockPos::new(0, 64, 0);
    let target = BlockPos::new(2, 64, 2);

    let outcome = find_path(
        start,
        &[target],
        0.0,
        &WalkNodeEvaluator,
        &world,
        ENTITY_HEIGHT,
        &no_malus(),
        1000,
    );

    let path = outcome.path.expect("a path was found");
    let nodes = path.nodes();
    for window in nodes.windows(2) {
        if window[0] == BlockPos::new(0, 64, 0) {
            assert_ne!(
                window[1],
                BlockPos::new(1, 64, 1),
                "must not cut the solid corner diagonally"
            );
        }
    }
}

#[test]
fn step_up_one_block_is_free_traversal() {
    let mut world = PathfindingFakeWorld::new(63);
    // A single-block-high step: ground rises to y=64 for x >= 3, so the walkable node
    // above it is y=65.
    for z in 0..3 {
        for x in 3..6 {
            world.set(BlockPos::new(x, 64, z), default_state::STONE);
        }
    }
    let start = BlockPos::new(0, 64, 0);
    let target = BlockPos::new(5, 65, 0);

    let outcome = find_path(
        start,
        &[target],
        0.0,
        &WalkNodeEvaluator,
        &world,
        ENTITY_HEIGHT,
        &no_malus(),
        1000,
    );

    let path = outcome.path.expect("a path was found");
    assert!(outcome.target_reached);
    assert!(
        path.nodes().contains(&BlockPos::new(2, 64, 0)) && path.nodes().contains(&BlockPos::new(3, 65, 0)),
        "steps directly from the flat run onto the raised step in one hop"
    );
    assert!(outcome.nodes_visited < 50);
}

#[test]
fn max_visited_nodes_budget_is_honored() {
    // A long, unobstructed straight corridor (50 blocks) with a deliberately tiny
    // budget: reaching the target requires far more than `max_visited_nodes`
    // expansions, so the search must stop early and fall back to its own best-effort
    // closest-approach route rather than returning `None`.
    let world = PathfindingFakeWorld::new(63);
    let start = BlockPos::new(0, 64, 0);
    let target = BlockPos::new(49, 64, 0);

    let outcome = find_path(
        start,
        &[target],
        0.0,
        &WalkNodeEvaluator,
        &world,
        ENTITY_HEIGHT,
        &no_malus(),
        5,
    );

    assert!(outcome.nodes_visited <= 5);
    assert!(!outcome.target_reached);
    assert!(outcome.path.is_some(), "best-effort route, never None");
}

#[test]
fn corridor_with_multiple_obstacles_reaches_the_far_end() {
    let mut world = PathfindingFakeWorld::new(63);
    // A 20-block corridor, 3 lanes wide, with 3 staggered single-block obstacles.
    let obstacle_z = [1i32, -1, 1];
    for (i, &z) in obstacle_z.iter().enumerate() {
        let x = 5 + (i as i32) * 6;
        world.set(BlockPos::new(x, 64, z), default_state::STONE);
        world.set(BlockPos::new(x, 65, z), default_state::STONE);
    }
    let start = BlockPos::new(0, 64, 1);
    let target = BlockPos::new(19, 64, 1);

    let outcome = find_path(
        start,
        &[target],
        0.0,
        &WalkNodeEvaluator,
        &world,
        ENTITY_HEIGHT,
        &no_malus(),
        2000,
    );

    assert!(outcome.target_reached);
    assert_eq!(outcome.path.expect("a path").nodes().last(), Some(&target));
}

#[test]
fn impassable_lava_lake_forces_a_detour_never_a_crossing() {
    let mut world = PathfindingFakeWorld::new(63);
    for x in 3..6 {
        for z in -1..=1 {
            world.set(BlockPos::new(x, 64, z), default_state::LAVA);
        }
    }
    let start = BlockPos::new(0, 64, 0);
    let target = BlockPos::new(8, 64, 0);

    let outcome = find_path(
        start,
        &[target],
        0.0,
        &WalkNodeEvaluator,
        &world,
        ENTITY_HEIGHT,
        &no_malus(),
        2000,
    );

    let path = outcome.path.expect("a path was found");
    assert!(outcome.target_reached);
    for x in 3..6 {
        for z in -1..=1 {
            assert!(
                !path.nodes().contains(&BlockPos::new(x, 64, z)),
                "never crosses a lava-classified node"
            );
        }
    }
}

#[test]
fn fudging_constant_is_one_point_five() {
    assert_eq!(FUDGING, 1.5);
}
