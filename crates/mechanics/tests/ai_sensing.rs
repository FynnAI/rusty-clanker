//! M4-B03 Acceptance tests: sensing -- nearest-player targeting, a coarse
//! line-of-sight raycast, and the per-tick seen/unseen cache (Context §H).

use std::cell::RefCell;

use rc_core::{BlockPos, ChunkKey, DimensionId, RcEntityId};
use rc_mechanics::ai::{Sensing, nearest_within_range, raycast_line_of_sight};
use rc_mechanics::world_access::BlockWorldAccess;
use rc_messaging::Address;
use rc_registries::generated_v776::block_states::default_state;

fn storage_id(
    id: rc_registries::generated_v776::block_states::BlockStateId,
) -> rc_chunk_storage::BlockStateId {
    rc_chunk_storage::BlockStateId(id.0)
}

struct AirWorld;
impl BlockWorldAccess for AirWorld {
    fn get_block(&self, _pos: BlockPos) -> Option<rc_chunk_storage::BlockStateId> {
        Some(storage_id(default_state::AIR))
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

struct WallWorld {
    wall_x: i32,
}
impl BlockWorldAccess for WallWorld {
    fn get_block(&self, pos: BlockPos) -> Option<rc_chunk_storage::BlockStateId> {
        if pos.x == self.wall_x {
            Some(storage_id(default_state::STONE))
        } else {
            Some(storage_id(default_state::AIR))
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

/// A call-counting `BlockWorldAccess` wrapper -- used to prove `Sensing`'s own cache is
/// consulted before falling through to `raycast_line_of_sight`.
struct CountingWorld {
    calls: RefCell<u32>,
}
impl BlockWorldAccess for CountingWorld {
    fn get_block(&self, _pos: BlockPos) -> Option<rc_chunk_storage::BlockStateId> {
        *self.calls.borrow_mut() += 1;
        Some(storage_id(default_state::AIR))
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

#[test]
fn nearest_within_range_picks_the_closest_candidate_in_range() {
    let candidates = vec![
        (RcEntityId(1), [3.0, 64.0, 0.0]),
        (RcEntityId(2), [7.0, 64.0, 0.0]),
        (RcEntityId(3), [40.0, 64.0, 0.0]),
    ];
    let result = nearest_within_range([0.0, 64.0, 0.0], candidates, 20.0);
    assert_eq!(result, Some(RcEntityId(1)));
}

#[test]
fn nearest_within_range_returns_none_when_all_out_of_range() {
    let candidates = vec![
        (RcEntityId(1), [30.0, 64.0, 0.0]),
        (RcEntityId(2), [40.0, 64.0, 0.0]),
    ];
    let result = nearest_within_range([0.0, 64.0, 0.0], candidates, 20.0);
    assert_eq!(result, None);
}

#[test]
fn raycast_line_of_sight_true_over_open_ground() {
    let world = AirWorld;
    assert!(raycast_line_of_sight(
        [0.0, 64.0, 0.0],
        [10.0, 64.0, 0.0],
        &world
    ));
}

#[test]
fn raycast_line_of_sight_false_through_a_solid_wall() {
    let world = WallWorld { wall_x: 5 };
    assert!(!raycast_line_of_sight(
        [0.0, 64.0, 0.0],
        [10.0, 64.0, 0.0],
        &world
    ));
}

#[test]
fn sensing_cache_is_reused_within_one_clear_cycle_and_reset_after_clear() {
    let world = CountingWorld {
        calls: RefCell::new(0),
    };
    let mut sensing = Sensing::default();
    let target = RcEntityId(42);

    sensing.has_line_of_sight([0.0, 64.0, 0.0], target, [5.0, 64.0, 0.0], &world);
    let calls_after_first = *world.calls.borrow();
    assert!(calls_after_first > 0);

    sensing.has_line_of_sight([0.0, 64.0, 0.0], target, [5.0, 64.0, 0.0], &world);
    assert_eq!(
        *world.calls.borrow(),
        calls_after_first,
        "cached, no new raycast"
    );

    sensing.clear();
    sensing.has_line_of_sight([0.0, 64.0, 0.0], target, [5.0, 64.0, 0.0], &world);
    assert!(
        *world.calls.borrow() > calls_after_first,
        "re-raycasts after clear()"
    );
}
