//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only)) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain; wires one chunk's block-entity list, not a ≥3-component chain) nondefault-state=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only); exercises the dispatch/wiring mechanism, not a block-state property)
//! M3-B06 — proves this blueprint's own fix closes M3-B04's `ContainerSignalSource` seam
//! (Acceptance tests' own `container_signal_source_wiring.rs` section).

use std::collections::HashMap;
use std::sync::Arc;

use rc_chunk_storage::ItemStackRecord;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::block_entity::chest::ChestBlockEntity;
use rc_mechanics::block_entity::container_signal_source::Tier1ContainerSignalSource;
use rc_mechanics::block_entity::furnace::{
    FURNACE_SLOT_FUEL, FURNACE_SLOT_INPUT, FURNACE_SLOT_OUTPUT, FuelTable, FurnaceBlockEntity,
    FurnaceLitStateResolver, SmeltingRecipeTable,
};
use rc_mechanics::block_entity::hopper::HopperBlockEntity;
use rc_mechanics::block_entity::{BlockEntityKind, BlockEntityWorldAccess};
use rc_mechanics::container::{DefaultMaxStackSize, TierOneContainer};
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::ContainerSignalSource;
use rc_mechanics::stage7::run_block_entity_tick;

fn full_stack(id: &str) -> Option<ItemStackRecord> {
    Some(ItemStackRecord {
        id: id.to_string(),
        count: 64,
        components: None,
    })
}

/// Extends `hopper_transfer_order.rs`'s own `FakeContainerWorld` test-double pattern with real
/// `region_chunks`/`block_entities_in_chunk`/`get_*_mut` implementations — the pieces
/// `run_block_entity_tick` itself needs, which that file's own tests (each ticking a
/// standalone, locally-owned block entity directly) never exercised.
struct FakeContainerWorld {
    chests: HashMap<BlockPos, ChestBlockEntity>,
    furnaces: HashMap<BlockPos, FurnaceBlockEntity>,
    hoppers: HashMap<BlockPos, HopperBlockEntity>,
}

impl FakeContainerWorld {
    fn new() -> Self {
        Self {
            chests: HashMap::new(),
            furnaces: HashMap::new(),
            hoppers: HashMap::new(),
        }
    }

    fn chunk_of(pos: BlockPos) -> ChunkKey {
        pos.chunk_key(DimensionId::OVERWORLD)
    }
}

impl BlockEntityWorldAccess for FakeContainerWorld {
    fn region_chunks(&self) -> Vec<ChunkKey> {
        let mut keys: Vec<ChunkKey> = self
            .chests
            .keys()
            .chain(self.furnaces.keys())
            .chain(self.hoppers.keys())
            .map(|&pos| Self::chunk_of(pos))
            .collect();
        keys.sort_unstable_by_key(|k| (k.x, k.z));
        keys.dedup();
        keys
    }

    fn block_entities_in_chunk(&self, chunk: ChunkKey) -> Vec<(BlockPos, BlockEntityKind)> {
        let mut out = Vec::new();
        for &pos in self.hoppers.keys() {
            if Self::chunk_of(pos) == chunk {
                out.push((pos, BlockEntityKind::Hopper));
            }
        }
        for &pos in self.furnaces.keys() {
            if Self::chunk_of(pos) == chunk {
                out.push((pos, BlockEntityKind::Furnace));
            }
        }
        for &pos in self.chests.keys() {
            if Self::chunk_of(pos) == chunk {
                out.push((pos, BlockEntityKind::Chest));
            }
        }
        out
    }

    fn container_at_mut(&mut self, pos: BlockPos) -> Option<&mut dyn TierOneContainer> {
        if let Some(h) = self.hoppers.get_mut(&pos) {
            return Some(h);
        }
        if let Some(f) = self.furnaces.get_mut(&pos) {
            return Some(f);
        }
        if let Some(c) = self.chests.get_mut(&pos) {
            return Some(c);
        }
        None
    }

    fn get_hopper_mut(&mut self, pos: BlockPos) -> Option<&mut HopperBlockEntity> {
        self.hoppers.get_mut(&pos)
    }
    fn get_furnace_mut(&mut self, pos: BlockPos) -> Option<&mut FurnaceBlockEntity> {
        self.furnaces.get_mut(&pos)
    }
    fn get_chest_mut(&mut self, pos: BlockPos) -> Option<&mut ChestBlockEntity> {
        self.chests.get_mut(&pos)
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
fn unrecorded_position_reads_none() {
    let source = Tier1ContainerSignalSource::new();
    assert_eq!(source.container_signal(BlockPos::new(0, 0, 0)), None);
}

#[test]
fn record_then_read_round_trips() {
    let source = Tier1ContainerSignalSource::new();
    let pos = BlockPos::new(1, 2, 3);

    source.record(pos, 8);
    assert_eq!(source.container_signal(pos), Some(8));

    source.record(pos, 3);
    assert_eq!(source.container_signal(pos), Some(3));
}

#[test]
fn forget_clears_a_previously_recorded_position() {
    let source = Tier1ContainerSignalSource::new();
    let pos = BlockPos::new(1, 2, 3);

    source.record(pos, 8);
    source.forget(pos);
    assert_eq!(source.container_signal(pos), None);
}

/// Section C (M3 field-report fix): a position's *first-ever* `record` counts as a change --
/// vanilla's own `BlockEntity.setChanged -> updateNeighbourForOutputSignal` fires the instant a
/// container's contents first become observable too (`docs/findings-for-planning.md`'s own
/// "Stage7->Stage4 container notify" entry).
#[test]
fn first_record_at_a_position_counts_as_changed() {
    let source = Tier1ContainerSignalSource::new();
    let pos = BlockPos::new(1, 2, 3);

    source.record(pos, 0);

    assert_eq!(source.take_changed(), vec![pos]);
}

/// A `record` call with the *same* signal value as last time is not a real content change --
/// mirrors vanilla's own `setChanged` being called unconditionally by every `getItems()` mutator
/// even when the net result is unchanged, while the actual comparator re-evaluation this drives
/// only matters when the analog value itself moved.
#[test]
fn record_with_an_unchanged_signal_is_not_reported_as_changed() {
    let source = Tier1ContainerSignalSource::new();
    let pos = BlockPos::new(1, 2, 3);

    source.record(pos, 5);
    source.take_changed(); // drain the initial change.

    source.record(pos, 5); // same value again.

    assert_eq!(source.take_changed(), Vec::<BlockPos>::new());
}

/// A `record` call with a genuinely different signal value is reported as changed.
#[test]
fn record_with_a_different_signal_is_reported_as_changed() {
    let source = Tier1ContainerSignalSource::new();
    let pos = BlockPos::new(1, 2, 3);

    source.record(pos, 5);
    source.take_changed(); // drain the initial change.

    source.record(pos, 6);

    assert_eq!(source.take_changed(), vec![pos]);
}

/// `take_changed` drains -- a second call with nothing new recorded in between returns empty.
#[test]
fn take_changed_drains_and_does_not_repeat() {
    let source = Tier1ContainerSignalSource::new();
    let pos = BlockPos::new(1, 2, 3);

    source.record(pos, 5);
    assert_eq!(source.take_changed(), vec![pos]);
    assert_eq!(source.take_changed(), Vec::<BlockPos>::new());
}

#[test]
fn implements_the_m3_b04_trait_object_unmodified() {
    let concrete = Arc::new(Tier1ContainerSignalSource::new());
    let trait_object: Arc<dyn ContainerSignalSource> =
        Arc::clone(&concrete) as Arc<dyn ContainerSignalSource>;

    let pos = BlockPos::new(5, 6, 7);
    concrete.record(pos, 5);

    assert_eq!(trait_object.container_signal(pos), Some(5));
}

#[test]
fn run_block_entity_tick_records_every_kind_including_chest() {
    let mut world = FakeContainerWorld::new();

    let chest_pos = BlockPos::new(0, 0, 0);
    let mut chest = ChestBlockEntity::empty();
    chest.slots[0] = Some(ItemStackRecord {
        id: "minecraft:item".to_string(),
        count: 13,
        components: None,
    });
    let expected_chest_signal = chest.comparator_signal(&DefaultMaxStackSize);
    world.chests.insert(chest_pos, chest);

    let furnace_pos = BlockPos::new(16, 0, 0);
    let mut furnace = FurnaceBlockEntity::empty();
    furnace.slots[FURNACE_SLOT_INPUT] = full_stack("minecraft:item");
    furnace.slots[FURNACE_SLOT_FUEL] = full_stack("minecraft:item");
    furnace.slots[FURNACE_SLOT_OUTPUT] = full_stack("minecraft:item");
    let expected_furnace_signal = furnace.comparator_signal(&DefaultMaxStackSize);
    world.furnaces.insert(furnace_pos, furnace);

    let hopper_pos = BlockPos::new(32, 0, 0);
    let hopper = HopperBlockEntity::empty(Direction::Down);
    let expected_hopper_signal = hopper.comparator_signal(&DefaultMaxStackSize);
    world.hoppers.insert(hopper_pos, hopper);

    let recipes = SmeltingRecipeTable::minimal_tier1();
    let fuels = FuelTable::minimal_tier1();
    let container_signals = Tier1ContainerSignalSource::new();

    run_block_entity_tick(
        &mut world,
        &recipes,
        &fuels,
        &DefaultMaxStackSize,
        None,
        &container_signals,
    );

    assert_eq!(
        container_signals.container_signal(chest_pos),
        Some(expected_chest_signal)
    );
    assert_eq!(
        container_signals.container_signal(furnace_pos),
        Some(expected_furnace_signal)
    );
    assert_eq!(
        container_signals.container_signal(hopper_pos),
        Some(expected_hopper_signal)
    );
}
