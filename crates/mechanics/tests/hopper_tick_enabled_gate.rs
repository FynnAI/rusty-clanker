//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(single canonical facing, Direction::Down, not a four-way sweep) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single hopper plus a single destination container, no ≥3-component chain) nondefault-state=yes
//! M3.5-B06 — `HopperBlockEntity::tick`'s own new `ENABLED` gate (Context §3.2, TEST-D57
//! CONFIRMED against the pinned oracle jar's own `HopperBlockEntity.tryMoveItems`, whose
//! combined condition this gate mirrors: checked after the cooldown gate, before the
//! pre-existing `is_locked_by_redstone` gate). Standalone `BlockEntityWorldAccess` test double
//! (no shared support module exists for `block_entity` tests — `hopper_transfer_order.rs`'s own
//! `FakeContainerWorld` is this file's closest sibling, restated locally here).

use std::collections::{HashMap, HashSet};

use rc_chunk_storage::ItemStackRecord;
use rc_core::{BlockPos, ChunkKey};
use rc_mechanics::block_entity::furnace::{FurnaceBlockEntity, FurnaceLitStateResolver};
use rc_mechanics::block_entity::hopper::{HopperBlockEntity, HopperTickOutcome};
use rc_mechanics::block_entity::{
    BlockEntityKind, BlockEntityWorldAccess, chest::ChestBlockEntity,
};
use rc_mechanics::container::{DefaultMaxStackSize, TierOneContainer};
use rc_mechanics::direction::Direction;

fn stack(id: &str, count: i32) -> Option<ItemStackRecord> {
    Some(ItemStackRecord {
        id: id.to_string(),
        count,
        components: None,
    })
}

/// A plain, unfaced generic container (mirrors `hopper_transfer_order.rs`'s own identical
/// `TestContainer`, restated here per this file's own no-shared-support-module convention).
struct TestContainer {
    slots: Vec<Option<ItemStackRecord>>,
}

impl TestContainer {
    fn empty(n: usize) -> Self {
        Self {
            slots: vec![None; n],
        }
    }
}

impl TierOneContainer for TestContainer {
    fn slots(&self) -> &[Option<ItemStackRecord>] {
        &self.slots
    }
    fn slots_mut(&mut self) -> &mut [Option<ItemStackRecord>] {
        &mut self.slots
    }
}

/// `BlockEntityWorldAccess` double with a settable `hopper_enabled`/`is_locked_by_redstone`
/// answer (both fixed, independent of `pos`) — this file's own entire point is exercising the
/// new gate in isolation from the pre-existing redstone-lock gate (Constraints (f): the two
/// must never be conflated).
struct EnabledGateWorld {
    containers: HashMap<BlockPos, Box<dyn TierOneContainer>>,
    enabled: bool,
    locked: bool,
}

impl EnabledGateWorld {
    fn new(enabled: bool, locked: bool) -> Self {
        Self {
            containers: HashMap::new(),
            enabled,
            locked,
        }
    }

    fn insert(&mut self, pos: BlockPos, container: Box<dyn TierOneContainer>) {
        self.containers.insert(pos, container);
    }

    fn slots_at(&self, pos: BlockPos) -> Option<&[Option<ItemStackRecord>]> {
        self.containers.get(&pos).map(|c| c.slots())
    }
}

impl BlockEntityWorldAccess for EnabledGateWorld {
    fn region_chunks(&self) -> Vec<ChunkKey> {
        Vec::new()
    }
    fn block_entities_in_chunk(&self, _chunk: ChunkKey) -> Vec<(BlockPos, BlockEntityKind)> {
        Vec::new()
    }
    fn container_at_mut(&mut self, pos: BlockPos) -> Option<&mut dyn TierOneContainer> {
        match self.containers.get_mut(&pos) {
            Some(boxed) => Some(boxed.as_mut()),
            None => None,
        }
    }
    fn get_hopper_mut(&mut self, _pos: BlockPos) -> Option<&mut HopperBlockEntity> {
        None
    }
    fn get_furnace_mut(&mut self, _pos: BlockPos) -> Option<&mut FurnaceBlockEntity> {
        None
    }
    fn get_chest_mut(&mut self, _pos: BlockPos) -> Option<&mut ChestBlockEntity> {
        None
    }
    fn is_locked_by_redstone(&self, _pos: BlockPos) -> bool {
        self.locked
    }
    fn hopper_enabled(&self, _pos: BlockPos) -> bool {
        self.enabled
    }
    fn swap_furnace_lit_state(
        &mut self,
        _pos: BlockPos,
        _now_lit: bool,
        _resolver: Option<&dyn FurnaceLitStateResolver>,
    ) {
    }
}

#[test]
fn disabled_hopper_transfers_nothing_and_reports_disabled_nondefault_case() {
    let hopper_pos = BlockPos::new(0, 1, 0);
    let dest_pos = BlockPos::new(0, 0, 0);

    let mut hopper = HopperBlockEntity::empty(Direction::Down);
    hopper.slots[0] = stack("minecraft:item", 5);
    let before_source = hopper.slots.clone();

    let mut world = EnabledGateWorld::new(false, false);
    world.insert(dest_pos, Box::new(TestContainer::empty(3)));
    let before_dest: Vec<Option<ItemStackRecord>> = world.slots_at(dest_pos).unwrap().to_vec();

    let no_ticked = HashSet::new();
    let outcome = hopper.tick(hopper_pos, &mut world, &DefaultMaxStackSize, &no_ticked);

    assert_eq!(outcome, HopperTickOutcome::Disabled);
    assert_eq!(
        hopper.slots, before_source,
        "a disabled hopper's own source slots must be byte-identical before/after — no \
         transfer attempted"
    );
    assert_eq!(
        world.slots_at(dest_pos).unwrap(),
        before_dest.as_slice(),
        "a disabled hopper's own destination slots must be byte-identical before/after — no \
         transfer attempted"
    );
}

#[test]
fn enabled_hopper_transfers_normally_unchanged_from_today() {
    let hopper_pos = BlockPos::new(0, 1, 0);
    let dest_pos = BlockPos::new(0, 0, 0);

    let mut hopper = HopperBlockEntity::empty(Direction::Down);
    hopper.slots[0] = stack("minecraft:item", 5);

    let mut world = EnabledGateWorld::new(true, false);
    world.insert(dest_pos, Box::new(TestContainer::empty(3)));

    let no_ticked = HashSet::new();
    let outcome = hopper.tick(hopper_pos, &mut world, &DefaultMaxStackSize, &no_ticked);

    assert_eq!(
        outcome,
        HopperTickOutcome::Pushed,
        "an enabled hopper's own tick must be unchanged from today's already-correct behavior"
    );
    assert_eq!(
        world.slots_at(dest_pos).unwrap()[0],
        stack("minecraft:item", 1),
        "the item actually moved into the destination"
    );
}
