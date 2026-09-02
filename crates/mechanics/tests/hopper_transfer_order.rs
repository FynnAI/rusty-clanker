//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(single canonical value/facing asserted, not a four-way sweep; furnace_face_rule_top_targets_input_side_targets_fuel is one face pairing, not a sweep) self=waived(no player/actor entity in this suite's own domain model) composition=yes nondefault-state=yes
//! M3-B06 — hand-derived hopper transfer-order tick tables, incl. the classic hopper-chain
//! timing cases (Acceptance tests' own `hopper_transfer_order.rs` section, the task's own
//! required acceptance category).
//!
//! **Field-report correction to this blueprint's own Acceptance-tests prose (test 1), superseded
//! by a second, deeper correction (`docs/findings-for-planning.md`'s own hopper-cadence entry,
//! verified against the real oracle via `redstone/clock/hopper_clock_basic`):** the blueprint's
//! own literal cooldown-gate pseudocode gated the decrement itself on the cooldown's
//! *pre*-decrement value and returned immediately whenever it fired, never re-checking the
//! *post*-decrement value within that same call — silently adding one whole extra idle tick
//! after every cooldown. Real vanilla decrements unconditionally and re-checks the post-decrement
//! value the very same call: a cooldown that reaches `0` this tick attempts its transfer this
//! tick, not the next one. The blueprint's pseudocode also read the 7-tick "pushed into an empty
//! container" exception onto the *source* hopper's own cooldown; real vanilla's source cooldown
//! is *always* `8` on any successful transfer — the shorter cooldown is a distinct,
//! destination-side effect that applies only when the destination is itself a hopper, and only
//! seeds *its* cooldown (7 if it already ticked earlier this same game tick, else 8) —
//! `HopperBlockEntity::tick`'s own doc comment has the full citation. Test 1 below now exercises
//! the corrected source-side timing (still 8 ticks between successive transfers, since the two
//! corrections cancel out for a plain, non-hopper destination); the new
//! `chained_hopper_push_into_empty_seeds_destination_cooldown_by_tick_order` test below exercises
//! the destination-side quirk itself, which test 1's own plain `TestContainer`-shaped destination
//! cannot (only a real `HopperBlockEntity` destination has a cooldown to seed).
//!
//! **Field-report correction (tests 5/6, furnace face rule):** the blueprint's own literal
//! pseudocode computes `from_above` as `push_target_pos.y > self.pos.y` — backwards. "From
//! above" means the *hopper* sits above the *destination*, i.e. the hopper's own Y is the
//! greater one: `self.pos.y > push_target_pos.y`. Verified against the furnace-face-rule
//! fixture below (a hopper directly above a furnace must target the input slot, per Context's
//! own "coal on the side, ore on top" auto-smelter framing) — the literal formula gives the
//! wrong slot for that exact fixture. `HopperBlockEntity::tick`'s own implementation uses the
//! corrected formula.

use std::collections::{HashMap, HashSet};

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

/// A `BlockEntityWorldAccess` double whose containers are *only* ever real
/// `HopperBlockEntity`s, addressable both generically (`container_at_mut`, for the item-move
/// itself) and specifically (`get_hopper_mut`, for the chained-hopper cooldown-seed quirk) --
/// unlike `FakeContainerWorld` above (whose own `get_hopper_mut` is a permanent stub, since none
/// of its own tests ever need a destination hopper's own post-push cooldown), this double exists
/// specifically so `chained_hopper_push_into_empty_seeds_destination_cooldown_by_tick_order` and
/// its sibling test can observe it. `hoppers` is `pub` so a test can inspect a destination's own
/// `transfer_cooldown` directly after a push, without needing a further accessor.
struct HopperChainWorld {
    hoppers: HashMap<BlockPos, HopperBlockEntity>,
}

impl HopperChainWorld {
    fn new() -> Self {
        Self {
            hoppers: HashMap::new(),
        }
    }

    fn insert(&mut self, pos: BlockPos, hopper: HopperBlockEntity) {
        self.hoppers.insert(pos, hopper);
    }
}

impl BlockEntityWorldAccess for HopperChainWorld {
    fn region_chunks(&self) -> Vec<ChunkKey> {
        Vec::new()
    }
    fn block_entities_in_chunk(&self, _chunk: ChunkKey) -> Vec<(BlockPos, BlockEntityKind)> {
        Vec::new()
    }
    fn container_at_mut(&mut self, pos: BlockPos) -> Option<&mut dyn TierOneContainer> {
        self.hoppers
            .get_mut(&pos)
            .map(|h| h as &mut dyn TierOneContainer)
    }
    fn get_hopper_mut(&mut self, pos: BlockPos) -> Option<&mut HopperBlockEntity> {
        self.hoppers.get_mut(&pos)
    }
    fn get_furnace_mut(&mut self, _pos: BlockPos) -> Option<&mut FurnaceBlockEntity> {
        None
    }
    fn get_chest_mut(&mut self, _pos: BlockPos) -> Option<&mut ChestBlockEntity> {
        None
    }
    fn is_locked_by_redstone(&self, _pos: BlockPos) -> bool {
        false
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

    let no_ticked = HashSet::new();

    let outcome = a.tick(a_pos, &mut world, &DefaultMaxStackSize, &no_ticked);
    assert_eq!(outcome, HopperTickOutcome::Pushed);
    // The acting hopper's own cooldown is always 8 on a successful push, regardless of
    // whether the destination was empty (`hopper.rs`'s own doc comment has the full
    // citation) -- the shorter 7-tick cooldown is a *destination*-side effect this test's own
    // `FakeContainerWorld::get_hopper_mut` stub cannot observe (see the dedicated
    // `chained_hopper_push_into_empty_seeds_destination_cooldown_by_tick_order` test below).
    assert_eq!(a.transfer_cooldown, 8);
    assert_eq!(a.slots[0], stack("minecraft:item", 4));
    assert_eq!(
        world.slots_at(b_pos).unwrap()[0],
        stack("minecraft:item", 1)
    );

    for expected in [7u8, 6, 5, 4, 3, 2, 1] {
        let outcome = a.tick(a_pos, &mut world, &DefaultMaxStackSize, &no_ticked);
        assert_eq!(outcome, HopperTickOutcome::OnCooldown);
        assert_eq!(a.transfer_cooldown, expected);
        assert_eq!(
            world.slots_at(b_pos).unwrap()[0],
            stack("minecraft:item", 1)
        );
    }

    // The 9th call (8 ticks after the first push): the cooldown's post-decrement value reaches
    // `0` this same call, so the next transfer is attempted this tick, not the one after.
    let outcome = a.tick(a_pos, &mut world, &DefaultMaxStackSize, &no_ticked);
    assert_eq!(outcome, HopperTickOutcome::Pushed);
    assert_eq!(a.transfer_cooldown, 8);
    assert_eq!(
        world.slots_at(b_pos).unwrap()[0],
        stack("minecraft:item", 2)
    );
}

#[test]
fn chained_hopper_push_into_empty_seeds_destination_cooldown_by_tick_order() {
    // A pushes into B, an empty hopper. Whether B's own freshly-seeded cooldown is 7 or 8
    // depends on whether B's own `tick` already ran earlier this same game tick -- exercised
    // directly here rather than through a full Stage-7 pass, since only `HopperChainWorld`
    // (below) can expose a destination hopper's own post-push cooldown at all.
    let a_pos = BlockPos::new(0, 1, 0);
    let b_pos = BlockPos::new(1, 1, 0);

    // Case 1: B has NOT yet ticked this game tick (empty `already_ticked` set) -- seeded to 8.
    {
        let mut world = HopperChainWorld::new();
        world.insert(a_pos, {
            let mut a = HopperBlockEntity::empty(Direction::East);
            a.slots[0] = stack("minecraft:redstone", 1);
            a
        });
        world.insert(b_pos, HopperBlockEntity::empty(Direction::West));

        let mut a = world.hoppers.remove(&a_pos).unwrap();
        let outcome = a.tick(a_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());
        assert_eq!(outcome, HopperTickOutcome::Pushed);
        assert_eq!(world.hoppers.get(&b_pos).unwrap().transfer_cooldown, 8);
    }

    // Case 2: B already ticked earlier this same game tick -- seeded to 7.
    {
        let mut world = HopperChainWorld::new();
        world.insert(a_pos, {
            let mut a = HopperBlockEntity::empty(Direction::East);
            a.slots[0] = stack("minecraft:redstone", 1);
            a
        });
        world.insert(b_pos, HopperBlockEntity::empty(Direction::West));

        let mut already_ticked = HashSet::new();
        already_ticked.insert(b_pos);

        let mut a = world.hoppers.remove(&a_pos).unwrap();
        let outcome = a.tick(a_pos, &mut world, &DefaultMaxStackSize, &already_ticked);
        assert_eq!(outcome, HopperTickOutcome::Pushed);
        assert_eq!(world.hoppers.get(&b_pos).unwrap().transfer_cooldown, 7);
    }
}

#[test]
fn chained_hopper_push_into_nonempty_never_seeds_destination_cooldown() {
    // B already holds an item -- the 7/8 destination seed is documented ("pushing into an
    // *empty* destination hopper") as applying only when the destination was empty; B's own
    // pre-existing cooldown must be left completely untouched by A's push.
    let a_pos = BlockPos::new(0, 1, 0);
    let b_pos = BlockPos::new(1, 1, 0);

    let mut world = HopperChainWorld::new();
    world.insert(a_pos, {
        let mut a = HopperBlockEntity::empty(Direction::East);
        a.slots[0] = stack("minecraft:redstone", 1);
        a
    });
    world.insert(b_pos, {
        let mut b = HopperBlockEntity::empty(Direction::West);
        b.slots[1] = stack("minecraft:redstone", 3);
        b.transfer_cooldown = 5;
        b
    });

    let mut a = world.hoppers.remove(&a_pos).unwrap();
    let outcome = a.tick(a_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());
    assert_eq!(outcome, HopperTickOutcome::Pushed);
    assert_eq!(world.hoppers.get(&b_pos).unwrap().transfer_cooldown, 5);
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

    let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());
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

    let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());
    assert_eq!(outcome, HopperTickOutcome::Pulled);
    assert_eq!(h.slots[0], stack("minecraft:other_item", 1));
    assert_eq!(world.slots_at(above_pos).unwrap()[0], None);
    assert_eq!(h.transfer_cooldown, 8);
}

#[test]
fn locked_hopper_transfers_nothing_but_cooldown_still_decrements_when_already_running_nondefault_case()
 {
    let h_pos = BlockPos::new(0, 0, 0);
    let mut world = FakeContainerWorld::new();
    world.set_locked(h_pos, true);

    let mut h = HopperBlockEntity::empty(Direction::Down);
    h.transfer_cooldown = 3;
    let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());
    assert_eq!(outcome, HopperTickOutcome::OnCooldown);
    assert_eq!(h.transfer_cooldown, 2);

    let mut h2 = HopperBlockEntity::empty(Direction::Down);
    let outcome2 = h2.tick(h_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());
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
    let outcome = h_top.tick(h_top_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());
    assert_eq!(outcome, HopperTickOutcome::Pushed);
    assert_eq!(
        world.slots_at(f_pos).unwrap()[FURNACE_SLOT_INPUT],
        stack("minecraft:coal", 1)
    );
    assert_eq!(world.slots_at(f_pos).unwrap()[FURNACE_SLOT_FUEL], None);

    world.insert(f_pos, Box::new(FurnaceBlockEntity::empty()));

    let mut h_side = HopperBlockEntity::empty(Direction::East);
    h_side.slots[0] = stack("minecraft:coal", 1);
    let outcome = h_side.tick(
        h_side_pos,
        &mut world,
        &DefaultMaxStackSize,
        &HashSet::new(),
    );
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
    let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());
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

    h.tick(h_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());

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

    let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());
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
        let outcome = h.tick(h_pos, &mut world, &DefaultMaxStackSize, &HashSet::new());
        assert_eq!(outcome, HopperTickOutcome::Idle);
        assert_eq!(h.transfer_cooldown, 0);
    }
}

// TEST-D55(d) composition case, added retroactively (M3.5-B04, §2.7): every existing
// test above is a single A -> B pair; this is the file's own first and only ≥3-hopper
// chain, proving an item actually traverses the full line rather than merely that each
// pairwise push independently works. Mirrors `single_transfer_takes_exactly_eight_ticks_
// between_attempts`'s own cooldown-respecting drive loop, driven across `HopperChainWorld`
// (the same real-`HopperBlockEntity`-backed double the destination-cooldown-seeding tests
// above already use) instead of `FakeContainerWorld` -- no production code changes: the
// mechanic already supports chained transfer end-to-end, only the test itself was missing.
#[test]
fn hopper_chain_of_three_relays_an_item_end_to_end() {
    let a_pos = BlockPos::new(0, 1, 0);
    let b_pos = BlockPos::new(1, 1, 0);
    let c_pos = BlockPos::new(2, 1, 0);

    let mut world = HopperChainWorld::new();
    world.insert(a_pos, {
        let mut a = HopperBlockEntity::empty(Direction::East);
        a.slots[0] = stack("minecraft:redstone", 1);
        a
    });
    world.insert(b_pos, HopperBlockEntity::empty(Direction::East));
    world.insert(c_pos, HopperBlockEntity::empty(Direction::East));

    let mut reached_c = false;
    for _ in 0..32 {
        let mut already_ticked = HashSet::new();
        for &pos in &[a_pos, b_pos, c_pos] {
            let mut hopper = world.hoppers.remove(&pos).unwrap();
            hopper.tick(pos, &mut world, &DefaultMaxStackSize, &already_ticked);
            world.hoppers.insert(pos, hopper);
            already_ticked.insert(pos);
        }
        if world.hoppers.get(&c_pos).unwrap().slots[0].is_some() {
            reached_c = true;
            break;
        }
    }

    assert!(
        reached_c,
        "item never reached the third hopper in the chain within 32 simulated ticks"
    );
    assert_eq!(
        world.hoppers.get(&c_pos).unwrap().slots[0],
        stack("minecraft:redstone", 1)
    );
    // The item actually left the source -- C did not merely gain one independently.
    assert_eq!(world.hoppers.get(&a_pos).unwrap().slots[0], None);
}
