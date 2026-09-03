//! Item-entity spawn-on-drop (M4-B02, Context §I) — completing M3-B03's own explicitly
//! deferred drops stance (`mining::BreakOutcome::Applied`'s own `drop_eligible` field): maps
//! a broken block's own pre-break raw state to one of `rc-mechanics`'s `Tier1DroppableBlock`
//! values, rolls its loot table, and spawns one real item entity per resulting
//! `ItemStackRecord`, with Context §I's own spawn-geometry jitter.

use bevy_ecs::prelude::*;
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::entity::physics::ITEM_HEIGHT;
use rc_mechanics::entity::{
    BaseEntity, EntityPayload, EntityUuid, ItemBundle, NetworkEntityIdAllocator, Pose, loot,
};
use rc_mechanics::random::RcRandom;
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::default_state::{DIRT, GRASS_BLOCK, STONE};

use std::collections::HashMap;
use std::sync::OnceLock;

/// Context §I/§J — maps a broken block's own pre-break `BlockStateId` to one of
/// `rc-mechanics`' `Tier1DroppableBlock` values (a plain `match` over this server's own known
/// tier-1 id ranges, mirroring `rc_physics::tier1_shape_table`'s own full-range registration
/// precedent — property-value-independent for drop purposes, e.g. a powered redstone wire
/// drops the identical redstone dust an unpowered one does). `None` for any block-state id
/// outside the known tier-1 set.
pub fn tier1_block_for_state(state: BlockStateId) -> Option<loot::Tier1DroppableBlock> {
    tier1_drop_table().get(&state.0).copied()
}

static TIER1_DROP_TABLE: OnceLock<HashMap<u32, loot::Tier1DroppableBlock>> = OnceLock::new();

fn tier1_drop_table() -> &'static HashMap<u32, loot::Tier1DroppableBlock> {
    TIER1_DROP_TABLE.get_or_init(build_tier1_drop_table)
}

fn build_tier1_drop_table() -> HashMap<u32, loot::Tier1DroppableBlock> {
    use loot::Tier1DroppableBlock as T;
    use rc_registries::block_state_properties::range_of;

    let mut map = HashMap::new();
    map.insert(STONE.0, T::Stone);
    map.insert(DIRT.0, T::Dirt);
    map.insert(GRASS_BLOCK.0, T::GrassBlock);

    let mut extend = |block: rc_registries::generated_v776::block_state_properties::BlockId,
                      kind: loot::Tier1DroppableBlock| {
        let range = range_of(block);
        for id in range.first.0..=range.last.0 {
            map.insert(id, kind);
        }
    };
    extend(block_id::REDSTONE_WIRE, T::RedstoneWire);
    extend(block_id::REDSTONE_TORCH, T::RedstoneTorch);
    extend(block_id::REDSTONE_WALL_TORCH, T::RedstoneTorch);
    extend(block_id::REPEATER, T::Repeater);
    extend(block_id::COMPARATOR, T::Comparator);
    extend(block_id::PISTON, T::Piston);
    extend(block_id::STICKY_PISTON, T::StickyPiston);
    extend(block_id::CHEST, T::Chest);
    extend(block_id::HOPPER, T::Hopper);
    extend(block_id::FURNACE, T::Furnace);
    extend(block_id::BLAST_FURNACE, T::BlastFurnace);
    extend(block_id::SMOKER, T::Smoker);

    map
}

fn fresh_item_base_entity(pos: [f64; 3], velocity: [f64; 3]) -> BaseEntity {
    BaseEntity {
        pos,
        velocity,
        rotation: [0.0, 0.0],
        fall_distance: 0.0,
        fire_ticks: 0,
        status_flags: 0,
        // Context §F: item entities carry `air_ticks` but never consume it — the base
        // bundle's own default-full value is harmless, matching vanilla's own unused field
        // on a non-`LivingEntity` kind.
        air_ticks: 300,
        on_ground: false,
        invulnerable: false,
        portal_cooldown: 0,
        uuid: EntityUuid::new_random(),
        custom_name: None,
        custom_name_visible: false,
        silent: false,
        no_gravity: false,
        glowing: false,
        pose: Pose::Standing,
        ticks_frozen: 0,
        has_visual_fire: false,
    }
}

/// Context §I — rolls the loot table for `broken_state` (a no-op if `tier1_block_for_state`
/// returns `None`) and spawns one real item entity per resulting `ItemStackRecord`, with
/// Context §I's own spawn-geometry jitter. `region_random`/`world_seed` resolve the rolled
/// table's own `LootTable.sequence_id` into the `XoroshiroRandom` stream `roll_loot_table`
/// draws from; `region_entropy` supplies the independent spawn-jitter draws (Context §I's own
/// explicit split between the two RNG streams). `network_ids` is not directly consumed here —
/// this milestone's own tracking pipeline derives every entity's wire-facing network id from
/// `RcEntityId(Entity::to_bits())` (the same truncation `entity_tracking.rs`'s own
/// `stand_in_network_id` already establishes, `docs/findings-for-planning.md`), not from a
/// separately-drawn allocator value — kept in this function's own signature for
/// forward-compatibility with a future real `RcEntityId` directory (Deliverables' own doc
/// comment).
pub fn spawn_break_drop(
    world: &mut World,
    pos: BlockPos,
    broken_state: BlockStateId,
    region_random: &mut loot::RandomSequenceStore,
    world_seed: i64,
    region_entropy: &mut RcRandom,
    _network_ids: &NetworkEntityIdAllocator,
) {
    let Some(block) = tier1_block_for_state(broken_state) else {
        return;
    };
    let table = loot::tier1_loot_table(block);
    let rng = region_random.get_or_create(table.sequence_id, world_seed);
    let drops = loot::roll_loot_table(table, rng, 0.0);

    for stack in drops {
        // Context §I's own spawn-geometry jitter -- a non-deterministic-across-restarts,
        // gameplay-feel-only draw from the per-region `RcRandom` stream, deliberately
        // independent of the `random_sequence` stream `roll_loot_table` itself just drew
        // from.
        let jitter_x = region_entropy.next_float() as f64 * 0.5 + 0.25;
        let jitter_y = region_entropy.next_float() as f64 * 0.5 + 0.25 - (ITEM_HEIGHT / 2.0);
        let jitter_z = region_entropy.next_float() as f64 * 0.5 + 0.25;
        let spawn_pos = [
            pos.x as f64 + jitter_x,
            pos.y as f64 + jitter_y,
            pos.z as f64 + jitter_z,
        ];
        let velocity = [
            region_entropy.next_double() * 0.2 - 0.1,
            0.2,
            region_entropy.next_double() * 0.2 - 0.1,
        ];

        let base = fresh_item_base_entity(spawn_pos, velocity);
        let payload = EntityPayload::Item(ItemBundle {
            item: stack,
            pickup_delay_ticks: rc_mechanics::entity::pickup::PICKUP_DELAY_DEFAULT,
            age_ticks: 0,
        });
        world.spawn((base, payload));
    }
}
