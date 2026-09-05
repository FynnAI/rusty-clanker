//! test-matrix: boundaries=waived(no world Y=-64/319 limit involved — a chunk-x boundary, not a world-height boundary) orientations=waived(single canonical East-facing chain, not a four-way sweep) self=waived(no player/actor entity in this suite's own domain model) composition=yes nondefault-state=yes
//! M4-B08 — the border-crossing hopper-chain cadence test (Acceptance tests,
//! `hopper_cross_chunk_border.rs`): proves ARCH-D17's cross-chunk-same-region collapse
//! and vanilla tick cadence hold correctly when a hopper chain genuinely straddles a
//! chunk border, driven through `run_block_entity_tick`'s real, two-chunk
//! `region_chunks()`/`block_entities_in_chunk()` loop for the first time (M3-B06's own
//! test suite never exercises more than one chunk). **Zero changes to any M3-B06
//! production file** (Context, Part 2.1) — this file's own `TwoChunkContainerWorld` test
//! double is the entirety of this blueprint's own Part 2 contribution.

use std::collections::HashMap;

use rc_chunk_storage::ItemStackRecord;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::block_entity::chest::ChestBlockEntity;
use rc_mechanics::block_entity::container_signal_source::Tier1ContainerSignalSource;
use rc_mechanics::block_entity::furnace::{
    FuelTable, FurnaceBlockEntity, FurnaceLitStateResolver, SmeltingRecipeTable,
};
use rc_mechanics::block_entity::hopper::HopperBlockEntity;
use rc_mechanics::block_entity::{BlockEntityKind, BlockEntityWorldAccess};
use rc_mechanics::container::{DefaultMaxStackSize, TierOneContainer};
use rc_mechanics::direction::Direction;
use rc_mechanics::stage7::run_block_entity_tick;

fn stack(id: &str, count: i32) -> Option<ItemStackRecord> {
    Some(ItemStackRecord {
        id: id.to_string(),
        count,
        components: None,
    })
}

fn expected_slot(count: i32) -> Option<ItemStackRecord> {
    if count == 0 {
        None
    } else {
        stack("minecraft:redstone", count)
    }
}

/// A plain, unfaced generic container (a chest stand-in): every slot is always both
/// insertable and extractable (`TierOneContainer`'s own default) — identical shape to
/// `hopper_transfer_order.rs`'s own `TestContainer` (M3-B06), restated here since this
/// file's own test double is new (Acceptance tests: "extends... `FakeContainerWorld`
/// test-double shape").
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

enum Block {
    // Boxed (clippy::large_enum_variant): `HopperBlockEntity` is far larger than
    // `TestContainer`'s own thin `Vec`-backed shape; boxing keeps every `Block` value the
    // size of one pointer regardless of variant, purely a storage-representation choice
    // with no behavioral effect on any assertion below.
    Hopper(Box<HopperBlockEntity>),
    Chest(TestContainer),
}

/// `HashMap<BlockPos, Box<dyn TierOneContainer>>` plus a fixed `chunk_of: HashMap<BlockPos,
/// ChunkKey>` map the test populates explicitly (Acceptance tests' own required fixture
/// shape) — extends `hopper_transfer_order.rs`'s own `FakeContainerWorld`/`HopperChainWorld`
/// test-double shape with real `region_chunks()`/`block_entities_in_chunk()` (this file's
/// own new contribution; M3-B06's own doubles return empty `Vec`s for both, since that
/// blueprint's own test suite never drives `run_block_entity_tick`'s real outer per-chunk
/// loop with more than one chunk present).
struct TwoChunkContainerWorld {
    blocks: HashMap<BlockPos, Block>,
    chunk_of: HashMap<BlockPos, ChunkKey>,
    /// `BlockEntityIndex`'s own stored load order (Context: "the one ordering guarantee
    /// that *is* vanilla-observable") — the fixed insertion order this test's own fixture
    /// constructs blocks in.
    load_order: Vec<BlockPos>,
    /// `region_chunks()`'s own return value — explicit, test-set (ascending by default;
    /// case 3 below overrides it to descending, an artificial, non-default order that
    /// test constructs solely to prove chunk order never changes the observable outcome,
    /// Context Part 2.3).
    chunk_order: Vec<ChunkKey>,
}

impl TwoChunkContainerWorld {
    fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            chunk_of: HashMap::new(),
            load_order: Vec::new(),
            chunk_order: Vec::new(),
        }
    }

    fn insert_hopper(&mut self, pos: BlockPos, chunk: ChunkKey, hopper: HopperBlockEntity) {
        self.blocks.insert(pos, Block::Hopper(Box::new(hopper)));
        self.chunk_of.insert(pos, chunk);
        self.load_order.push(pos);
    }

    fn insert_chest(&mut self, pos: BlockPos, chunk: ChunkKey, chest: TestContainer) {
        self.blocks.insert(pos, Block::Chest(chest));
        self.chunk_of.insert(pos, chunk);
        self.load_order.push(pos);
    }

    fn hopper_at(&self, pos: BlockPos) -> Option<&HopperBlockEntity> {
        match self.blocks.get(&pos) {
            Some(Block::Hopper(hopper)) => Some(hopper.as_ref()),
            _ => None,
        }
    }

    fn slots_at(&self, pos: BlockPos) -> Option<&[Option<ItemStackRecord>]> {
        match self.blocks.get(&pos) {
            Some(Block::Hopper(hopper)) => Some(hopper.slots()),
            Some(Block::Chest(chest)) => Some(chest.slots()),
            None => None,
        }
    }
}

impl BlockEntityWorldAccess for TwoChunkContainerWorld {
    fn region_chunks(&self) -> Vec<ChunkKey> {
        self.chunk_order.clone()
    }

    fn block_entities_in_chunk(&self, chunk: ChunkKey) -> Vec<(BlockPos, BlockEntityKind)> {
        self.load_order
            .iter()
            .filter(|pos| self.chunk_of.get(pos) == Some(&chunk))
            .map(|&pos| {
                let kind = match self.blocks.get(&pos) {
                    Some(Block::Hopper(_)) => BlockEntityKind::Hopper,
                    Some(Block::Chest(_)) => BlockEntityKind::Chest,
                    None => unreachable!("load_order only ever holds inserted positions"),
                };
                (pos, kind)
            })
            .collect()
    }

    fn container_at_mut(&mut self, pos: BlockPos) -> Option<&mut dyn TierOneContainer> {
        match self.blocks.get_mut(&pos) {
            Some(Block::Hopper(hopper)) => Some(hopper.as_mut() as &mut dyn TierOneContainer),
            Some(Block::Chest(chest)) => Some(chest as &mut dyn TierOneContainer),
            None => None,
        }
    }

    fn get_hopper_mut(&mut self, pos: BlockPos) -> Option<&mut HopperBlockEntity> {
        match self.blocks.get_mut(&pos) {
            Some(Block::Hopper(hopper)) => Some(hopper.as_mut()),
            _ => None,
        }
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

fn tick_helpers() -> (SmeltingRecipeTable, FuelTable, Tier1ContainerSignalSource) {
    (
        SmeltingRecipeTable::minimal_tier1(),
        FuelTable::minimal_tier1(),
        Tier1ContainerSignalSource::new(),
    )
}

#[test]
fn single_hop_across_a_chunk_border_uses_the_ordinary_eight_tick_cadence() {
    let chunk_a = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
    let chunk_b = ChunkKey::new(DimensionId::OVERWORLD, 1, 0);
    let a_pos = BlockPos::new(15, 70, 0);
    let chest_pos = BlockPos::new(16, 70, 0);

    let mut world = TwoChunkContainerWorld::new();
    world.chunk_order = vec![chunk_a, chunk_b];
    let mut a = HopperBlockEntity::empty(Direction::East);
    a.slots[0] = stack("minecraft:redstone", 64);
    world.insert_hopper(a_pos, chunk_a, a);
    world.insert_chest(chest_pos, chunk_b, TestContainer::empty(27));

    let (recipes, fuels, signals) = tick_helpers();

    run_block_entity_tick(
        &mut world,
        &recipes,
        &fuels,
        &DefaultMaxStackSize,
        None,
        &signals,
    );
    assert_eq!(
        world.slots_at(chest_pos).unwrap()[0],
        stack("minecraft:redstone", 1)
    );
    assert_eq!(
        world.hopper_at(a_pos).unwrap().slots[0],
        stack("minecraft:redstone", 63)
    );
    assert_eq!(world.hopper_at(a_pos).unwrap().transfer_cooldown, 8);

    for expected in [7u8, 6, 5, 4, 3, 2, 1] {
        run_block_entity_tick(
            &mut world,
            &recipes,
            &fuels,
            &DefaultMaxStackSize,
            None,
            &signals,
        );
        assert_eq!(world.hopper_at(a_pos).unwrap().transfer_cooldown, expected);
        assert_eq!(
            world.slots_at(chest_pos).unwrap()[0],
            stack("minecraft:redstone", 1)
        );
    }

    // The 9th call: the cooldown's post-decrement value reaches 0 this same call, so the
    // next transfer is attempted this tick (the corrected, real-vanilla cadence M3-B06's
    // own `hopper_transfer_order.rs` already established — restated here across a real
    // chunk boundary for the first time).
    run_block_entity_tick(
        &mut world,
        &recipes,
        &fuels,
        &DefaultMaxStackSize,
        None,
        &signals,
    );
    assert_eq!(world.hopper_at(a_pos).unwrap().transfer_cooldown, 8);
    assert_eq!(
        world.slots_at(chest_pos).unwrap()[0],
        stack("minecraft:redstone", 2)
    );
}

#[test]
fn hand_derived_three_hopper_chain_tick_table() {
    let chunk_a = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
    let chunk_b = ChunkKey::new(DimensionId::OVERWORLD, 1, 0);
    let a_pos = BlockPos::new(15, 70, 0);
    let b_pos = BlockPos::new(16, 70, 0);
    let c_pos = BlockPos::new(17, 70, 0);

    let mut world = TwoChunkContainerWorld::new();
    world.chunk_order = vec![chunk_a, chunk_b];
    let mut a = HopperBlockEntity::empty(Direction::East);
    a.slots[0] = stack("minecraft:redstone", 64);
    world.insert_hopper(a_pos, chunk_a, a);
    world.insert_hopper(b_pos, chunk_b, HopperBlockEntity::empty(Direction::East));
    world.insert_chest(c_pos, chunk_b, TestContainer::empty(27));

    let (recipes, fuels, signals) = tick_helpers();

    // (A.slots[0], A.cooldown, B.slots[0], B.cooldown, C.slots[0]) per tick, 1-indexed —
    // the exact hand-derived table (Acceptance tests).
    let rows: [(i32, u8, i32, u8, i32); 10] = [
        (63, 8, 1, 7, 0),
        (63, 7, 1, 6, 0),
        (63, 6, 1, 5, 0),
        (63, 5, 1, 4, 0),
        (63, 4, 1, 3, 0),
        (63, 3, 1, 2, 0),
        (63, 2, 1, 1, 0),
        (63, 1, 0, 8, 1),
        (62, 8, 1, 7, 1),
        (62, 7, 1, 6, 1),
    ];

    for (i, (a_count, a_cd, b_count, b_cd, c_count)) in rows.into_iter().enumerate() {
        run_block_entity_tick(
            &mut world,
            &recipes,
            &fuels,
            &DefaultMaxStackSize,
            None,
            &signals,
        );
        let tick = i + 1;
        assert_eq!(
            world.slots_at(a_pos).unwrap()[0],
            expected_slot(a_count),
            "tick {tick}: A.slots[0]"
        );
        assert_eq!(
            world.hopper_at(a_pos).unwrap().transfer_cooldown,
            a_cd,
            "tick {tick}: A.cooldown"
        );
        assert_eq!(
            world.slots_at(b_pos).unwrap()[0],
            expected_slot(b_count),
            "tick {tick}: B.slots[0]"
        );
        assert_eq!(
            world.hopper_at(b_pos).unwrap().transfer_cooldown,
            b_cd,
            "tick {tick}: B.cooldown"
        );
        assert_eq!(
            world.slots_at(c_pos).unwrap()[0],
            expected_slot(c_count),
            "tick {tick}: C.slots[0]"
        );
    }
}

#[test]
fn chunk_iteration_order_never_changes_the_final_receiving_hoppers_cooldown_nondefault_case() {
    let chunk_a = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
    let chunk_b = ChunkKey::new(DimensionId::OVERWORLD, 1, 0);
    let a_pos = BlockPos::new(15, 70, 0);
    let b_pos = BlockPos::new(16, 70, 0);
    let c_pos = BlockPos::new(17, 70, 0);

    let mut world = TwoChunkContainerWorld::new();
    // Descending order — an artificial, non-default order this test constructs solely to
    // prove the point (Context, Part 2.3), never how the real adapter orders chunks.
    world.chunk_order = vec![chunk_b, chunk_a];
    let mut a = HopperBlockEntity::empty(Direction::East);
    a.slots[0] = stack("minecraft:redstone", 64);
    world.insert_hopper(a_pos, chunk_a, a);
    world.insert_hopper(b_pos, chunk_b, HopperBlockEntity::empty(Direction::East));
    world.insert_chest(c_pos, chunk_b, TestContainer::empty(27));

    let (recipes, fuels, signals) = tick_helpers();
    run_block_entity_tick(
        &mut world,
        &recipes,
        &fuels,
        &DefaultMaxStackSize,
        None,
        &signals,
    );

    // The *same* end-of-call value ascending order (case 2, tick 1) produces, reached via
    // the opposite mechanism (Context, Part 2.3): `B` ticks first this call, finds itself
    // empty with nothing to pull, settles at 0; `A`'s later push then assigns it `7`
    // directly, with no further decrement following.
    assert_eq!(
        world.hopper_at(b_pos).unwrap().transfer_cooldown,
        7,
        "descending chunk order must reach the identical shared end-of-call cooldown value"
    );
    assert_eq!(
        world.slots_at(c_pos).unwrap()[0],
        None,
        "no same-region-tick cascade occurs in either chunk order"
    );
}

#[test]
fn hopper_in_one_chunk_never_sees_a_hopper_in_another_region_entirely() {
    let chunk_a = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
    let a_pos = BlockPos::new(15, 70, 0);

    let mut world = TwoChunkContainerWorld::new();
    world.chunk_order = vec![chunk_a];
    let mut a = HopperBlockEntity::empty(Direction::East);
    a.slots[0] = stack("minecraft:redstone", 1);
    world.insert_hopper(a_pos, chunk_a, a);
    // The push target (16, 70, 0) — and the pull source (15, 71, 0) — are positions this
    // region simply does not own at all (a real region border, MECH-D19's own
    // `BorderUpdateEvent` mechanism, not a chunk border within one region, Context Part
    // 2's own scope boundary) — deliberately never registered in `world.blocks`/`chunk_of`
    // at all, so `container_at_mut` returns `None` for both, exactly as it would for any
    // position outside this world's own known set.

    let (recipes, fuels, signals) = tick_helpers();
    run_block_entity_tick(
        &mut world,
        &recipes,
        &fuels,
        &DefaultMaxStackSize,
        None,
        &signals,
    );

    assert_eq!(
        world.slots_at(a_pos).unwrap()[0],
        stack("minecraft:redstone", 1),
        "no push/pull target exists across the region border -- the item must stay in A"
    );
    assert_eq!(
        world.hopper_at(a_pos).unwrap().transfer_cooldown,
        0,
        "an Idle outcome never seeds a cooldown"
    );
}
