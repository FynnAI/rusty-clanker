//! M3.5-B05 — the new Stage-7 (`DomainGroup::BlockEntity`) system that refreshes every
//! resident chunk's own `BlockEntitySaveRecords` once per tick (Context 2.2 step 2):
//! resolves each of a chunk's own `BlockEntityIndex` entries to its real typed
//! component, calls the new `BlockEntityCodec::to_record` on it, appends any
//! `minecraft:comparator` records for that chunk (Context 2.5), and writes the result
//! into `BlockEntitySaveRecords` — the only place in the codebase that ever needs both
//! `rc-chunk-storage`'s and this crate's own block-entity type systems in scope at once.

use bevy_ecs::prelude::*;
use rc_chunk_storage::{
    BlockEntityCodec, BlockEntityIndex, BlockEntityRecord, BlockEntitySaveRecords, ChunkKeyTag,
};

use crate::block_entity::BlockEntityHeader;
use crate::block_entity::chest::ChestBlockEntity;
use crate::block_entity::furnace::FurnaceBlockEntity;
use crate::block_entity::hopper::HopperBlockEntity;
use crate::redstone::comparator;

/// Inserted by the composition root's `bootstrap_redstone_dispatch` (Context 2.5) — the
/// "lookup" a comparator's own `OutputSignal` is sourced from/seeded through at save/load
/// time. Mirrors `ContainerSignalsResource`'s own established shape exactly.
#[derive(Resource, Clone)]
pub struct ComparatorOutputsResource(
    pub std::sync::Arc<crate::redstone::comparator::ComparatorBehavior>,
);

type SaveRecordQueryData = (
    &'static BlockEntityHeader,
    Option<&'static HopperBlockEntity>,
    Option<&'static FurnaceBlockEntity>,
    Option<&'static ChestBlockEntity>,
);

/// Determinism (Constraints (d)): every chunk's own `BlockEntitySaveRecords` is built in
/// a stable, reproducible order every tick — `BlockEntityIndex`'s own already-stable
/// order first, then comparator records sorted ascending by `(x, y, z)`, never raw
/// `HashMap` iteration order.
fn system_block_entity_save_records(
    headers: Query<SaveRecordQueryData>,
    mut chunks: Query<(&ChunkKeyTag, &BlockEntityIndex, &mut BlockEntitySaveRecords)>,
    comparator_outputs: Res<ComparatorOutputsResource>,
) {
    let all_comparator_outputs = comparator_outputs.0.snapshot_outputs();

    for (chunk_key, index, mut save_records) in &mut chunks {
        let mut records: Vec<BlockEntityRecord> = Vec::with_capacity(index.entities().len());
        for &entity in index.entities() {
            let Ok((header, hopper, furnace, chest)) = headers.get(entity) else {
                continue;
            };
            let record = if let Some(hopper) = hopper {
                hopper.to_record(header.pos)
            } else if let Some(furnace) = furnace {
                furnace.to_record(header.pos)
            } else if let Some(chest) = chest {
                chest.to_record(header.pos)
            } else {
                continue;
            };
            records.push(record);
        }

        let mut comparator_records: Vec<BlockEntityRecord> = all_comparator_outputs
            .iter()
            .filter(|(pos, _)| pos.chunk_key(chunk_key.0.dimension) == chunk_key.0)
            .map(|&(pos, output)| comparator::comparator_record(pos, output))
            .collect();
        comparator_records.sort_by_key(|record| (record.pos.x, record.pos.y, record.pos.z));
        records.extend(comparator_records);

        save_records.0 = records;
    }
}

/// `rc-scheduler`/`bevy_ecs` adapter for the system above. Registered as
/// `register_stage7`'s third system, after `block_entity_tick_factory`/
/// `container_signal_notify_factory` (same-`DomainGroup` registration-order wave
/// discipline `register_stage7`'s own existing doc comment already establishes).
pub fn block_entity_save_records_factory() -> rc_scheduler::SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(system_block_entity_save_records))
            as Box<dyn System<In = (), Out = ()>>
    })
}
