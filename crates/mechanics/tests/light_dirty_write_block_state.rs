//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(single canonical value/facing asserted, not a four-way sweep) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain) nondefault-state=yes
//! M4-B07 field-report test-authoring (ledger B, "write_block_state never marks the
//! light-dirty queue"): `UpdateContext::set_block` is the only caller of `LightDirtyQueue::
//! mark` — every settled redstone state flip goes through `write_block_state` instead (torch/
//! wire/repeater/comparator/piston own-state writeback, `write_block_state`'s own doc
//! comment), so a redstone torch relighting (light emission 7) never reaches the light engine
//! until an unrelated `set_block` nearby happens to mark the same position. Proven directly
//! against a torch's own scheduled-tick flip, which writes through `write_block_state`
//! (`TorchBehavior::reeval_tick`).

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    RedstoneSignalSource, SignalSourceRegistry, TorchAttachment, TorchBehavior,
};
use rc_mechanics::{
    BlockBehavior, BlockEventQueue, BlockWorldAccess, LightDirtyQueue, NeighborUpdateEngine,
    RegionOwnership, ScheduledTickQueue, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};

use support::{FakeWorld, TestSignalSource};

const TORCH_ID: BlockStateId = BlockStateId(6885); // lit=true, the real default state
const SUPPORT_ID: BlockStateId = BlockStateId(2);

struct Harness {
    world: FakeWorld,
    engine: NeighborUpdateEngine,
    scheduled: ScheduledTickQueue,
    events: BlockEventQueue,
    outbound: Vec<(Address, RegionMessage)>,
    changed: Vec<(BlockPos, BlockStateId)>,
    light_dirty: LightDirtyQueue,
    ownership: RegionOwnership,
}

impl Harness {
    fn new() -> Self {
        let world = FakeWorld::new();
        let local = world.local;
        Self {
            world,
            engine: NeighborUpdateEngine::new(),
            scheduled: ScheduledTickQueue::new(),
            events: BlockEventQueue::new(),
            outbound: Vec::new(),
            changed: Vec::new(),
            light_dirty: LightDirtyQueue::new(),
            ownership: RegionOwnership::always_local(local),
        }
    }

    fn ctx_at(&mut self, current_tick: u64) -> UpdateContext<'_> {
        UpdateContext {
            world: &mut self.world,
            engine: &mut self.engine,
            scheduled: &mut self.scheduled,
            events: &mut self.events,
            outbound: &mut self.outbound,
            changed: &mut self.changed,
            ownership: &self.ownership,
            current_tick,
            light_dirty: &mut self.light_dirty,
        }
    }
}

#[test]
fn a_torch_flipped_via_its_scheduled_tick_marks_the_light_dirty_queue_nondefault_case() {
    let torch = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    let support = Arc::new(TestSignalSource::fixed(0));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SUPPORT_ID,
        BlockStateId(SUPPORT_ID.0 + 1),
        Arc::clone(&support) as Arc<dyn RedstoneSignalSource>,
    );
    torch.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    h.world.set_block(t, TORCH_ID);
    h.world.set_block(Direction::Down.apply(t), SUPPORT_ID);

    // Power the support, schedule the re-eval, then let it fire — this writes the torch's own
    // new state through `write_block_state`, never `set_block` (`TorchBehavior::reeval_tick`'s
    // own doc comment).
    support.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    assert!(
        h.light_dirty.drain().is_empty(),
        "no light-relevant write has happened yet"
    );
    {
        let mut ctx = h.ctx_at(2);
        torch.on_scheduled_tick(&mut ctx, t);
    }
    assert!(!torch.lit(t), "the torch must have actually flipped off");
    assert_eq!(
        h.world.get_block(t),
        Some(BlockStateId(6886)),
        "torch's own stored id must flip to the real lit=false id"
    );

    let entries = h.light_dirty.drain();
    assert_eq!(
        entries.len(),
        1,
        "the torch's own scheduled-tick flip (write_block_state) must mark the light-dirty \
         queue exactly once — a redstone torch relighting changes its own light emission and \
         must reach the light engine, not stay stale until an unrelated set_block nearby"
    );
    assert_eq!(entries[0].pos, t);
    assert_eq!(entries[0].old_state, TORCH_ID);
    assert_eq!(entries[0].new_state, BlockStateId(6886));
}
