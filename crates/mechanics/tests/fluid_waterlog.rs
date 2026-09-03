//! M4-B06 — waterlogging substrate (Context §J): `WaterloggableRegistry`'s range dispatch,
//! `SimpleWaterlogged`'s water-only reference implementation, and `spread_to`'s own
//! waterlog-before-hard-overwrite check ordering.

use std::collections::HashMap;
use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::block_event::BlockEventQueue;
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::direction::Direction;
use rc_mechanics::fluid::spread::spread_to;
use rc_mechanics::fluid::state::{FluidKind, FluidState};
use rc_mechanics::fluid::waterlog::{
    SimpleWaterlogged, WaterloggableBehavior, WaterloggableRegistry,
};
use rc_mechanics::fluid::{FluidBlockRanges, FluidDimensionProfile, FluidTables, ReactionBlocks};
use rc_mechanics::neighbor_update::NeighborUpdateEngine;
use rc_mechanics::scheduled_tick::ScheduledTickQueue;
use rc_mechanics::{BlockWorldAccess, UpdateContext};
use rc_messaging::{Address, RegionId, RegionMessage};

const AIR: BlockStateId = BlockStateId(50);
const STONE: BlockStateId = BlockStateId(51);
const DRY: BlockStateId = BlockStateId(70);
const WET: BlockStateId = BlockStateId(71);

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
    local: Address,
}

impl FakeWorld {
    fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            local: Address::Region(RegionId(0)),
        }
    }
    fn set(&mut self, pos: BlockPos, state: BlockStateId) {
        self.blocks.insert(pos, state);
    }
}

impl BlockWorldAccess for FakeWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.blocks.get(&pos).copied()
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

#[test]
fn unregistered_target_is_not_waterloggable() {
    let registry = WaterloggableRegistry::new();
    assert!(registry.resolve(BlockStateId(12345)).is_none());
    assert!(registry.resolve(DRY).is_none());
}

#[test]
fn simple_waterlogged_accepts_only_water() {
    let simple = SimpleWaterlogged::new(vec![(DRY, WET)]);
    let world = FakeWorld::new();
    let pos = BlockPos::new(0, 0, 0);
    assert!(simple.can_place_liquid(&world, pos, DRY, FluidKind::Water));
    assert!(!simple.can_place_liquid(&world, pos, DRY, FluidKind::Lava));
}

#[test]
fn simple_waterlogged_state_lookup_round_trips() {
    let simple = SimpleWaterlogged::new(vec![(DRY, WET)]);
    let world = FakeWorld::new();
    let pos = BlockPos::new(0, 0, 0);
    assert_eq!(
        simple.waterlogged_state(&world, pos, DRY, FluidKind::Water),
        Some(WET)
    );
    // Already-waterlogged no-op: WET is not itself a registered dry key.
    assert_eq!(
        simple.waterlogged_state(&world, pos, WET, FluidKind::Water),
        None
    );
}

#[test]
fn spread_to_waterlogs_a_registered_target_instead_of_overwriting() {
    let t = tables();
    let mut registry = WaterloggableRegistry::new();
    registry.register_range(
        DRY,
        BlockStateId(DRY.0 + 1),
        Arc::new(SimpleWaterlogged::new(vec![(DRY, WET)])),
    );

    let target_pos = BlockPos::new(1, 5, 0);
    let mut world = FakeWorld::new();
    world.set(target_pos, DRY);

    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
    {
        let mut ctx = UpdateContext {
            world: &mut world,
            engine: &mut engine,
            scheduled: &mut scheduled,
            events: &mut events,
            outbound: &mut outbound,
            changed: &mut changed,
            ownership: &ownership,
            current_tick: 0,
        };
        spread_to(
            &mut ctx,
            &t,
            &registry,
            FluidKind::Water,
            target_pos,
            Direction::East,
            Some(FluidState::flowing(FluidKind::Water, 7, false)),
        );
    }

    assert_eq!(world.get_block(target_pos), Some(WET));
    assert_ne!(
        world.get_block(target_pos),
        Some(fluid_id(
            &t,
            FluidState::flowing(FluidKind::Water, 7, false)
        ))
    );
    assert!(scheduled.is_fluid_tick_pending(target_pos));
}

#[test]
fn spread_to_hard_overwrites_an_unregistered_non_air_target() {
    let t = tables();
    let registry = WaterloggableRegistry::new();

    let target_pos = BlockPos::new(1, 5, 0);
    let mut world = FakeWorld::new();
    world.set(target_pos, STONE);

    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
    {
        let mut ctx = UpdateContext {
            world: &mut world,
            engine: &mut engine,
            scheduled: &mut scheduled,
            events: &mut events,
            outbound: &mut outbound,
            changed: &mut changed,
            ownership: &ownership,
            current_tick: 0,
        };
        spread_to(
            &mut ctx,
            &t,
            &registry,
            FluidKind::Water,
            target_pos,
            Direction::East,
            Some(FluidState::flowing(FluidKind::Water, 7, false)),
        );
    }

    assert_eq!(
        world.get_block(target_pos),
        Some(fluid_id(
            &t,
            FluidState::flowing(FluidKind::Water, 7, false)
        ))
    );
}
