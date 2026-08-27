//! M3-B06 — hand-derived hopper transfer-order tick tables, incl. the classic hopper-chain
//! timing cases (Acceptance tests' own `hopper_transfer_order.rs` section, the task's own
//! required acceptance category).
//!
//! **Field-report correction to this blueprint's own Acceptance-tests prose (test 1):** the
//! blueprint's own text claims "tick A six more times (ticks 2..7)... an 8th time transfer_
//! cooldown is now 0" — but `cooldown = 7` after a push-into-empty needs *seven* decrement-only
//! ticks (not six) to reach `0` (7 -> 6 -> 5 -> 4 -> 3 -> 2 -> 1 -> 0), and the gate that decides
//! whether *this* tick attempts a transfer reads the cooldown's value from *before* this tick's
//! own decrement (Context's own algorithm, unambiguous) — so the next transfer attempt is the
//! 9th call after the push, not the 8th. This is the well-known, publicly documented "8 ticks
//! between successive transfers" vanilla hopper-clock timing exactly (8 ticks elapse between
//! the push, call 1, and the next transfer attempt, call 9); the blueprint's own prose undercounts
//! the cooldown-only phase by one call. This test follows the pseudocode (unambiguous, correct)
//! rather than the prose's own miscount.
//!
//! **Field-report correction (tests 5/6, furnace face rule):** the blueprint's own literal
//! pseudocode computes `from_above` as `push_target_pos.y > self.pos.y` — backwards. "From
//! above" means the *hopper* sits above the *destination*, i.e. the hopper's own Y is the
//! greater one: `self.pos.y > push_target_pos.y`. Verified against the furnace-face-rule
//! fixture below (a hopper directly above a furnace must target the input slot, per Context's
//! own "coal on the side, ore on top" auto-smelter framing) — the literal formula gives the
//! wrong slot for that exact fixture. `HopperBlockEntity::tick`'s own implementation uses the
//! corrected formula.

use std::collections::HashMap;

use rc_chunk_storage::ItemStackRecord;
use rc_core::{BlockPos, ChunkKey};
use rc_mechanics::block_entity::furnace::{
    FURNACE_SLOT_FUEL, FURNACE_SLOT_INPUT, FURNACE_SLOT_OUTPUT, FurnaceBlockEntity,
    FurnaceLitStateResolver,
};
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

/// A plain, unfaced generic container (chest/hopper-as-destination stand-in): every slot is
/// always both insertable and extractable (`TierOneContainer`'s own default).
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

/// `HashMap<BlockPos, Box<dyn TierOneContainer>>`-backed `BlockEntityWorldAccess` test double
/// (Acceptance tests' own opening paragraph for this file). `region_chunks`/
/// `block_entities_in_chunk`/`get_hopper_mut`/`get_furnace_mut`/`get_chest_mut`/
/// `swap_furnace_lit_state` are unused by this file's own tests (every one of them ticks a
/// standalone, locally-owned `HopperBlockEntity` directly, never fetched back out of `world`)
/// — trivial stubs satisfy the trait; `container_signal_source_wiring.rs` extends this same
/// pattern with real implementations of the ones it does need.
struct FakeContainerWorld {
    containers: HashMap<BlockPos, Box<dyn TierOneContainer>>,
    locked: HashMap<BlockPos, bool>,
}

impl FakeContainerWorld {
    fn new() -> Self {
        Self {
            containers: HashMap::new(),
            locked: HashMap::new(),
        }
    }

    fn insert(&mut self, pos: BlockPos, container: Box<dyn TierOneContainer>) {
        self.containers.insert(pos, container);
    }

    fn set_locked(&mut self, pos: BlockPos, locked: bool) {
        self.locked.insert(pos, locked);
    }

    fn slots_at(&self, pos: BlockPos) -> Option<&[Option<ItemStackRecord>]> {
        self.containers.get(&pos).map(|c| c.slots())
    }
}

impl BlockEntityWorldAccess for FakeContainerWorld {
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
    fn is_locked_by_redstone(&self, pos: BlockPos) -> bool {
        self.locked.get(&pos).copied().unwrap_or(false)
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
fn single_transfer_takes_exactly_eight_ticks_between_attempts() {
    let a_pos = BlockPos::new(0, 1, 0);
    let b_pos = BlockPos::new(0, 0, 0);

    let mut a = HopperBlockEntity::empty(Direction::Down);
    a.slots[0] = stack("minecraft:item", 5);

    let mut world = FakeContainerWorld::new();
    world.insert(b_pos, Box::new(HopperBlockEntity::empty(Direction::Down)));

    let outcome = a.tick(a_pos, &mut world, &DefaultMaxStackSize);
    assert_eq!(outcome, HopperTickOutcome::Pushed);
    assert_eq!(a.transfer_cooldown, 7);
    assert_eq!(a.slots[0], stack("minecraft:item", 4));
    assert_eq!(
        world.slots_at(b_pos).unwrap()[0],
        stack("minecraft:item", 1)
    );

    for expected in [6u8, 5, 4, 3, 2, 1, 0] {
        let outcome = a.tick(a_pos, &mut world, &DefaultMaxStackSize);
        assert_eq!(outcome, HopperTickOutcome::OnCooldown);
        assert_eq!(a.transfer_cooldown, expected);
        assert_eq!(
            world.slots_at(b_pos).unwrap()[0],
            stack("minecraft:item", 1)
        );
    }

    let outcome = a.tick(a_pos, &mut world, &DefaultMaxStackSize);
    assert_eq!(outcome, HopperTickOutcome::Pushed);
    assert_eq!(a.transfer_cooldown, 8);
    assert_eq!(
        world.slots_at(b_pos).unwrap()[0],
        stack("minecraft:item", 2)
    );
}

#[test]
fn push_is_attempted_before_pull_and_skips_pull_on_success() {
    let h_pos = BlockPos::new(0, 1, 0);
    let below_pos = BlockPos::new(0, 0, 0);
    let above_pos = BlockPos::new(0, 2, 0);

    let mut h = HopperBlockEntity::empty(Direction::Down);
    h.slots[0] = stack("minecraft:item", 1);

    let mut world = FakeContainerWorld::new();
    world.insert(below_pos, Box::new(TestContainer::empty(5)));
    let mut above = TestContainer::empty(5);
    above.slots[0] = stack("minecraft:other_item", 1);
    world.insert(above_pos, Box::new(above));

    let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize);
    assert_eq!(outcome, HopperTickOutcome::Pushed);
    assert!(h.slots.iter().all(Option::is_none));
    assert_eq!(
        world.slots_at(below_pos).unwrap()[0],
        stack("minecraft:item", 1)
    );
    assert_eq!(
        world.slots_at(above_pos).unwrap()[0],
        stack("minecraft:other_item", 1)
    );
}

#[test]
fn pull_is_attempted_only_when_push_has_nothing_to_move() {
    let h_pos = BlockPos::new(0, 1, 0);
    let below_pos = BlockPos::new(0, 0, 0);
    let above_pos = BlockPos::new(0, 2, 0);

    let mut h = HopperBlockEntity::empty(Direction::Down);

    let mut world = FakeContainerWorld::new();
    world.insert(below_pos, Box::new(TestContainer::empty(5)));
    let mut above = TestContainer::empty(5);
    above.slots[0] = stack("minecraft:other_item", 1);
    world.insert(above_pos, Box::new(above));

    let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize);
    assert_eq!(outcome, HopperTickOutcome::Pulled);
    assert_eq!(h.slots[0], stack("minecraft:other_item", 1));
    assert_eq!(world.slots_at(above_pos).unwrap()[0], None);
    assert_eq!(h.transfer_cooldown, 8);
}

#[test]
fn locked_hopper_transfers_nothing_but_cooldown_still_decrements_when_already_running() {
    let h_pos = BlockPos::new(0, 0, 0);
    let mut world = FakeContainerWorld::new();
    world.set_locked(h_pos, true);

    let mut h = HopperBlockEntity::empty(Direction::Down);
    h.transfer_cooldown = 3;
    let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize);
    assert_eq!(outcome, HopperTickOutcome::OnCooldown);
    assert_eq!(h.transfer_cooldown, 2);

    let mut h2 = HopperBlockEntity::empty(Direction::Down);
    let outcome2 = h2.tick(h_pos, &mut world, &DefaultMaxStackSize);
    assert_eq!(outcome2, HopperTickOutcome::Locked);
    assert_eq!(h2.transfer_cooldown, 0);
    assert!(h2.slots.iter().all(Option::is_none));
}

#[test]
fn furnace_face_rule_top_targets_input_side_targets_fuel() {
    let f_pos = BlockPos::new(0, 0, 0);
    let h_top_pos = BlockPos::new(0, 1, 0);
    let h_side_pos = BlockPos::new(-1, 0, 0);

    let mut world = FakeContainerWorld::new();
    world.insert(f_pos, Box::new(FurnaceBlockEntity::empty()));

    let mut h_top = HopperBlockEntity::empty(Direction::Down);
    h_top.slots[0] = stack("minecraft:coal", 1);
    let outcome = h_top.tick(h_top_pos, &mut world, &DefaultMaxStackSize);
    assert_eq!(outcome, HopperTickOutcome::Pushed);
    assert_eq!(
        world.slots_at(f_pos).unwrap()[FURNACE_SLOT_INPUT],
        stack("minecraft:coal", 1)
    );
    assert_eq!(world.slots_at(f_pos).unwrap()[FURNACE_SLOT_FUEL], None);

    world.insert(f_pos, Box::new(FurnaceBlockEntity::empty()));

    let mut h_side = HopperBlockEntity::empty(Direction::East);
    h_side.slots[0] = stack("minecraft:coal", 1);
    let outcome = h_side.tick(h_side_pos, &mut world, &DefaultMaxStackSize);
    assert_eq!(outcome, HopperTickOutcome::Pushed);
    assert_eq!(
        world.slots_at(f_pos).unwrap()[FURNACE_SLOT_FUEL],
        stack("minecraft:coal", 1)
    );
    assert_eq!(world.slots_at(f_pos).unwrap()[FURNACE_SLOT_INPUT], None);
}

#[test]
fn hopper_below_furnace_extracts_only_the_output_slot() {
    let f_pos = BlockPos::new(0, 1, 0);
    let h_pos = BlockPos::new(0, 0, 0);

    let mut f = FurnaceBlockEntity::empty();
    f.slots[FURNACE_SLOT_INPUT] = stack("minecraft:iron_ore", 1);
    f.slots[FURNACE_SLOT_OUTPUT] = stack("minecraft:iron_ingot", 1);

    let mut world = FakeContainerWorld::new();
    world.insert(f_pos, Box::new(f));

    let mut h = HopperBlockEntity::empty(Direction::North);
    let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize);
    assert_eq!(outcome, HopperTickOutcome::Pulled);
    assert_eq!(h.slots[0], stack("minecraft:iron_ingot", 1));
    assert_eq!(
        world.slots_at(f_pos).unwrap()[FURNACE_SLOT_INPUT],
        stack("minecraft:iron_ore", 1)
    );
    assert_eq!(world.slots_at(f_pos).unwrap()[FURNACE_SLOT_OUTPUT], None);
}

#[test]
fn leftmost_slot_selection_prefers_stacking_over_spreading() {
    let h_pos = BlockPos::new(0, 1, 0);
    let dest_pos = BlockPos::new(0, 0, 0);

    let mut h = HopperBlockEntity::empty(Direction::Down);
    h.slots[0] = stack("minecraft:item", 1);

    let mut dest = TestContainer::empty(5);
    dest.slots[2] = stack("minecraft:item", 1);

    let mut world = FakeContainerWorld::new();
    world.insert(dest_pos, Box::new(dest));

    h.tick(h_pos, &mut world, &DefaultMaxStackSize);

    let dest_slots = world.slots_at(dest_pos).unwrap();
    assert_eq!(dest_slots[2], stack("minecraft:item", 2));
    assert_eq!(dest_slots[0], None);
}

#[test]
fn unmovable_source_item_does_not_block_a_subsequent_pull_attempt_the_same_tick() {
    let h_pos = BlockPos::new(0, 1, 0);
    let below_pos = BlockPos::new(0, 0, 0);
    let above_pos = BlockPos::new(0, 2, 0);

    let mut h = HopperBlockEntity::empty(Direction::Down);
    h.slots[0] = stack("minecraft:unique_item", 1);

    let mut below = TestContainer::empty(5);
    for slot in below.slots.iter_mut() {
        *slot = stack("minecraft:different_item", 64);
    }
    let mut above = TestContainer::empty(5);
    above.slots[0] = stack("minecraft:pullable_item", 1);

    let mut world = FakeContainerWorld::new();
    world.insert(below_pos, Box::new(below));
    world.insert(above_pos, Box::new(above));

    let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize);
    assert_eq!(outcome, HopperTickOutcome::Pulled);
    assert_eq!(
        world.slots_at(below_pos).unwrap()[0],
        stack("minecraft:different_item", 64)
    );
    assert_eq!(h.slots[0], stack("minecraft:unique_item", 1));
    assert_eq!(h.slots[1], stack("minecraft:pullable_item", 1));
    assert_eq!(world.slots_at(above_pos).unwrap()[0], None);
}

#[test]
fn idle_hopper_with_nothing_to_move_and_nothing_to_pull_never_enters_cooldown() {
    let h_pos = BlockPos::new(0, 0, 0);
    let mut world = FakeContainerWorld::new();
    let mut h = HopperBlockEntity::empty(Direction::North);

    for _ in 0..5 {
        let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize);
        assert_eq!(outcome, HopperTickOutcome::Idle);
        assert_eq!(h.transfer_cooldown, 0);
    }
}
