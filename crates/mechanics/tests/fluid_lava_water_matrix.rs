//! M4-B06 — the full lava+water interaction matrix (Context §I): reaction A (synchronous
//! contact conversion, `LAVA_CONTACT_ORDER`) and reaction B (asynchronous downward-spread
//! conversion to stone), kept as the two genuinely distinct code paths vanilla runs.

mod support;

use std::cell::RefCell;
use std::collections::HashMap;

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::block_event::BlockEventQueue;
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::direction::Direction;
use rc_mechanics::fluid::reaction::check_lava_water_contact;
use rc_mechanics::fluid::spread::spread_to;
use rc_mechanics::fluid::state::{FluidKind, FluidState};
use rc_mechanics::fluid::waterlog::WaterloggableRegistry;
use rc_mechanics::fluid::{
    BasaltConversion, FluidBlockRanges, FluidDimensionProfile, FluidTables, ReactionBlocks,
};
use rc_mechanics::neighbor_update::NeighborUpdateEngine;
use rc_mechanics::scheduled_tick::ScheduledTickQueue;
use rc_mechanics::{BlockWorldAccess, LightDirtyQueue, UpdateContext};
use rc_messaging::{Address, RegionId, RegionMessage};

const AIR: BlockStateId = BlockStateId(0);
const STONE: BlockStateId = BlockStateId(999_999);
const OBSIDIAN: BlockStateId = BlockStateId(60);
const COBBLESTONE: BlockStateId = BlockStateId(61);
const BASALT: BlockStateId = BlockStateId(62);
const SOUL_SOIL: BlockStateId = BlockStateId(63);
const BLUE_ICE: BlockStateId = BlockStateId(64);

fn ranges() -> FluidBlockRanges {
    FluidBlockRanges::new(
        (BlockStateId(900_000), BlockStateId(900_016)),
        (BlockStateId(900_100), BlockStateId(900_116)),
    )
    .expect("both ranges are 16-wide")
}

fn tables(basalt: Option<BasaltConversion>) -> FluidTables {
    FluidTables::new(
        ranges(),
        ReactionBlocks {
            obsidian: OBSIDIAN,
            cobblestone: COBBLESTONE,
            stone: STONE,
            basalt_conversion: basalt,
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

/// Wraps a `FakeWorld`, recording every `get_block` call's own position (test 3's own
/// call-counting double, proving the scan short-circuits after the first match).
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

macro_rules! with_ctx {
    ($world:expr, $current_tick:expr, |$ctx:ident| $body:block) => {{
        let mut engine = NeighborUpdateEngine::new();
        let mut scheduled = ScheduledTickQueue::new();
        let mut events = BlockEventQueue::new();
        let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
        let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
        let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
        let mut light_dirty = LightDirtyQueue::new();
        let mut $ctx = UpdateContext {
            world: &mut $world,
            engine: &mut engine,
            scheduled: &mut scheduled,
            events: &mut events,
            outbound: &mut outbound,
            changed: &mut changed,
            light_dirty: &mut light_dirty,
            ownership: &ownership,
            current_tick: $current_tick,
        };
        $body
    }};
}

#[test]
fn contact_conversion_order_up_north_south_west_east_first_match_wins() {
    let t = tables(None);
    let mut world = FakeWorld::new();
    let lava_pos = BlockPos::new(0, 5, 0);
    world.set(lava_pos, fluid_id(&t, FluidState::source(FluidKind::Lava)));
    world.set(
        Direction::Up.apply(lava_pos),
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    world.set(
        Direction::East.apply(lava_pos),
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );

    let fired = with_ctx!(world, 0, |ctx| {
        check_lava_water_contact(&mut ctx, &t, lava_pos)
    });
    assert!(fired);
    // Up (checked first in LAVA_CONTACT_ORDER) matched -- a source converts to obsidian, not
    // cobblestone (which East's own match would have produced if it had won instead).
    assert_eq!(world.get_block(lava_pos), Some(OBSIDIAN));
}

#[test]
fn contact_conversion_source_becomes_obsidian_flowing_becomes_cobblestone() {
    let t = tables(None);

    let mut source_world = FakeWorld::new();
    let pos = BlockPos::new(0, 5, 0);
    source_world.set(pos, fluid_id(&t, FluidState::source(FluidKind::Lava)));
    source_world.set(
        Direction::North.apply(pos),
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    let fired = with_ctx!(source_world, 0, |ctx| {
        check_lava_water_contact(&mut ctx, &t, pos)
    });
    assert!(fired);
    assert_eq!(source_world.get_block(pos), Some(OBSIDIAN));

    let mut flowing_world = FakeWorld::new();
    flowing_world.set(
        pos,
        fluid_id(&t, FluidState::flowing(FluidKind::Lava, 5, false)),
    );
    flowing_world.set(
        Direction::North.apply(pos),
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    let fired = with_ctx!(flowing_world, 0, |ctx| {
        check_lava_water_contact(&mut ctx, &t, pos)
    });
    assert!(fired);
    assert_eq!(flowing_world.get_block(pos), Some(COBBLESTONE));
}

#[test]
fn contact_conversion_returns_immediately_remaining_positions_unchecked() {
    let t = tables(None);
    let lava_pos = BlockPos::new(0, 5, 0);
    let mut inner = FakeWorld::new();
    inner.set(lava_pos, fluid_id(&t, FluidState::source(FluidKind::Lava)));
    inner.set(
        Direction::North.apply(lava_pos),
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    inner.set(
        Direction::West.apply(lava_pos),
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    let mut world = CountingWorld {
        inner,
        calls: RefCell::new(Vec::new()),
    };

    let fired = with_ctx!(world, 0, |ctx| {
        check_lava_water_contact(&mut ctx, &t, lava_pos)
    });
    assert!(fired);

    let west_pos = Direction::West.apply(lava_pos);
    let east_pos = Direction::East.apply(lava_pos);
    let calls = world.calls.borrow();
    assert!(
        !calls.iter().any(|p| *p == west_pos || *p == east_pos),
        "West/East must never be looked up once North (checked earlier in LAVA_CONTACT_ORDER) matched: {calls:?}"
    );
}

#[test]
fn basalt_conversion_when_no_water_and_soul_soil_blue_ice_present() {
    let t = tables(Some(BasaltConversion {
        soul_soil: SOUL_SOIL,
        blue_ice: BLUE_ICE,
        basalt: BASALT,
    }));
    let lava_pos = BlockPos::new(0, 5, 0);
    let mut world = FakeWorld::new();
    world.set(lava_pos, fluid_id(&t, FluidState::source(FluidKind::Lava)));
    world.set(Direction::Down.apply(lava_pos), SOUL_SOIL);
    world.set(Direction::North.apply(lava_pos), BLUE_ICE);

    let fired = with_ctx!(world, 0, |ctx| {
        check_lava_water_contact(&mut ctx, &t, lava_pos)
    });
    assert!(fired);
    assert_eq!(world.get_block(lava_pos), Some(BASALT));
}

#[test]
fn basalt_conversion_absent_leaves_lava_unreacted() {
    let t = tables(None);
    let lava_pos = BlockPos::new(0, 5, 0);
    let mut world = FakeWorld::new();
    let lava_id = fluid_id(&t, FluidState::source(FluidKind::Lava));
    world.set(lava_pos, lava_id);
    world.set(Direction::Down.apply(lava_pos), SOUL_SOIL);
    world.set(Direction::North.apply(lava_pos), BLUE_ICE);

    let fired = with_ctx!(world, 0, |ctx| {
        check_lava_water_contact(&mut ctx, &t, lava_pos)
    });
    assert!(!fired);
    assert_eq!(world.get_block(lava_pos), Some(lava_id));
}

#[test]
fn no_reaction_when_below_is_not_soul_soil() {
    let t = tables(Some(BasaltConversion {
        soul_soil: SOUL_SOIL,
        blue_ice: BLUE_ICE,
        basalt: BASALT,
    }));
    let lava_pos = BlockPos::new(0, 5, 0);
    let mut world = FakeWorld::new();
    let lava_id = fluid_id(&t, FluidState::source(FluidKind::Lava));
    world.set(lava_pos, lava_id);
    world.set(Direction::Down.apply(lava_pos), STONE); // not soul soil
    world.set(Direction::North.apply(lava_pos), BLUE_ICE);

    let fired = with_ctx!(world, 0, |ctx| {
        check_lava_water_contact(&mut ctx, &t, lava_pos)
    });
    assert!(!fired);
    assert_eq!(world.get_block(lava_pos), Some(lava_id));
}

#[test]
fn downward_spread_into_water_becomes_stone_never_cobblestone_or_obsidian() {
    let t = tables(None);
    let waterlog = WaterloggableRegistry::new();
    let target_pos = BlockPos::new(0, 4, 0);
    let mut world = FakeWorld::new();
    world.set(
        target_pos,
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );

    with_ctx!(world, 0, |ctx| {
        spread_to(
            &mut ctx,
            &t,
            &waterlog,
            FluidKind::Lava,
            target_pos,
            Direction::Down,
            Some(FluidState::flowing(FluidKind::Lava, 7, true)),
        );
    });

    assert_eq!(world.get_block(target_pos), Some(STONE));
    assert_ne!(world.get_block(target_pos), Some(OBSIDIAN));
    assert_ne!(world.get_block(target_pos), Some(COBBLESTONE));
}

#[test]
fn sideways_spread_into_water_never_reaches_the_stone_reaction() {
    use rc_mechanics::fluid::algorithm::can_be_replaced_with;

    let t = tables(None);
    let target_pos = BlockPos::new(1, 5, 0);
    let mut world = FakeWorld::new();
    world.set(
        target_pos,
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );

    // Water's own `canBeReplacedWith` rejects any non-Down replacement attempt -- lava can
    // never structurally reach the stone reaction via a sideways `East` candidate.
    assert!(!can_be_replaced_with(
        &world,
        &t,
        target_pos,
        FluidKind::Lava,
        Direction::East
    ));

    // Confirm the target is left completely untouched -- `spread_to` is never even called for
    // this candidate in the real `spread_to_sides` pipeline once the replace check rejects it.
    assert_eq!(
        world.get_block(target_pos),
        Some(fluid_id(&t, FluidState::source(FluidKind::Water)))
    );
}

#[test]
fn newly_placed_lava_immediately_runs_the_contact_check() {
    let t = tables(None);
    let waterlog = WaterloggableRegistry::new();
    let target_pos = BlockPos::new(1, 5, 0);
    let mut world = FakeWorld::new();
    // Water sits adjacent to the target -- not adjacent to any *other* position -- so the
    // contact reaction can only possibly fire once lava is actually written to `target_pos`.
    world.set(
        Direction::North.apply(target_pos),
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );

    with_ctx!(world, 0, |ctx| {
        spread_to(
            &mut ctx,
            &t,
            &waterlog,
            FluidKind::Lava,
            target_pos,
            Direction::East,
            Some(FluidState::flowing(FluidKind::Lava, 6, false)),
        );
    });

    // The contact-conversion scan ran in the very same `spread_to` call that placed the lava --
    // the `onPlace`-equivalent trigger -- converting it to cobblestone (a flowing, non-source
    // lava cell) without waiting for a separate `on_neighbor_changed` dispatch.
    assert_eq!(world.get_block(target_pos), Some(COBBLESTONE));
}
