//! Stage 8's bulk-synchronous-parallel round scheduler (M4-B07 Context §8, WORLD-D9,
//! ARCH-D16). ECS-agnostic core: takes `&mut bevy_ecs::world::World` directly (an
//! unconditional `rc-mechanics` dependency) and a small local trait -- not
//! `RcWorkerPool` directly -- for the parallel-dispatch boundary, so the bulk of this
//! file compiles without the `server-systems` feature (mirrors `world_access.rs`'s
//! `BlockWorldAccess` decoupling pattern); the handful of statements that touch
//! `rc-scheduler`'s own `LightBorderInbox`/`RegionMessageOutbox` resources (both of
//! which live behind that same feature) are individually `#[cfg]`-gated, becoming
//! no-ops under a hypothetical `server-systems`-off build rather than pulling in a
//! hard, unconditional `rc-scheduler` dependency this crate's `Cargo.toml` does not
//! grant this file (Constraint (b) -- no new dependency edge).

use std::collections::HashMap;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use rc_chunk_storage::{
    BlockStateColumn, ChunkKeyTag, HeightmapSet, LightColumn, LightNibbles, WORLD_HEIGHT,
    WORLD_MIN_Y,
};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::Address;

use crate::border::RegionOwnership;
use crate::direction::Direction;
use crate::light::propagator::{
    LightChannel, LocalChunkLight, check_node_block, check_node_sky, propagate_decrease_step,
    propagate_increase_step,
};
use crate::light::properties::LightPropertiesRegistry;
use crate::light::queue::{LightDirtyQueue, LightPropagatorState};
use crate::light::sky_source::SkyLightSourceColumn;

/// The parallel-dispatch boundary Stage 8's round driver needs (Context §8).
pub trait ParallelDispatch {
    /// Mirrors `RcWorkerPool::run_batch`'s own signature exactly (M0-B04).
    fn run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>);
}

/// Diagnostic summary of one Stage-8 invocation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct LightTickReport {
    pub rounds_run: u32,
    pub converged: bool,
    pub chunks_touched: usize,
}

/// Every component reference one entity's own local propagator step needs, fetched
/// together via a single `bevy_ecs::QueryState::get_mut` call (component types never
/// alias each other on one entity, so `bevy_ecs` allows this safely).
type ChunkComponents<'a> = (
    &'a mut LightPropagatorState,
    &'a mut LightColumn,
    &'a mut SkyLightSourceColumn,
    &'a BlockStateColumn,
    &'a HeightmapSet,
);

/// As `ChunkComponents`, plus the owning `Entity` -- used by the round loop's own
/// single `Query::iter_mut` pass (Context §8 step 6), which needs `Entity` to filter
/// down to only the `touched` set.
type ChunkComponentsWithEntity<'a> = (
    Entity,
    &'a mut LightPropagatorState,
    &'a mut LightColumn,
    &'a mut SkyLightSourceColumn,
    &'a BlockStateColumn,
    &'a HeightmapSet,
);

/// Runs `f` against one entity's own bundled `LocalChunkLight` + `LightPropagatorState`,
/// silently doing nothing if `entity` no longer carries the full component set (should
/// never happen for an entity this module itself already resolved via `ChunkKeyTag`).
fn with_chunk<F: FnOnce(&mut LocalChunkLight, &mut LightPropagatorState)>(
    world: &mut World,
    entity: Entity,
    chunk_key: ChunkKey,
    properties: &LightPropertiesRegistry,
    f: F,
) {
    let mut query = world.query::<ChunkComponents>();
    let Ok((state, light, sky_sources, blocks, heightmap)) = query.get_mut(world, entity) else {
        return;
    };
    let state = state.into_inner();
    let mut local = LocalChunkLight {
        light: light.into_inner(),
        sky_sources: sky_sources.into_inner(),
        blocks,
        heightmap,
        properties,
        chunk_origin_x: chunk_key.x * 16,
        chunk_origin_z: chunk_key.z * 16,
    };
    f(&mut local, state);
}

fn is_fresh(column: &LightColumn) -> bool {
    column.sections().iter().all(|s| {
        matches!(s.sky, LightNibbles::Uninitialized)
            && matches!(s.block, LightNibbles::Uninitialized)
    })
}

/// Stage 8's complete driver (Context §8). See that section for the full 10-step
/// algorithm.
pub fn run_stage8_lighting(world: &mut World, pool: &dyn ParallelDispatch) -> LightTickReport {
    let mut key_to_entity: HashMap<ChunkKey, Entity> = HashMap::new();
    {
        let mut q = world.query::<(&ChunkKeyTag, Entity)>();
        for (tag, entity) in q.iter(world) {
            key_to_entity.insert(tag.0, entity);
        }
    }

    let properties = world
        .get_resource::<LightPropertiesRegistry>()
        .cloned()
        .unwrap_or_default();

    // --- Seeding (round -1, sequential) ---
    let mut fresh_handled: std::collections::HashSet<Entity> = std::collections::HashSet::new();

    let dirty = world
        .get_resource_mut::<LightDirtyQueue>()
        .map(|mut q| q.drain())
        .unwrap_or_default();

    for entry in &dirty {
        let chunk_key = entry.pos.chunk_key(DimensionId::OVERWORLD);
        let Some(&entity) = key_to_entity.get(&chunk_key) else {
            continue;
        };
        fresh_handled.insert(entity);

        let old_emission = properties.resolve(entry.old_state).block_emission;
        let new_emission = properties.resolve(entry.new_state).block_emission;

        with_chunk(world, entity, chunk_key, &properties, |local, state| {
            check_node_block(
                local,
                entry.pos,
                old_emission,
                new_emission,
                &mut state.block,
            );

            let local_x = (entry.pos.x - chunk_key.x * 16) as u8;
            let local_z = (entry.pos.z - chunk_key.z * 16) as u8;
            let (old_boundary, new_boundary) = local.sky_sources.recompute_column(
                local.blocks,
                local.heightmap,
                local.properties,
                local_x,
                local_z,
            );

            let lo = old_boundary.min(new_boundary);
            let hi = old_boundary.max(new_boundary);
            let mut covered_own_y = false;
            for world_y in lo..hi {
                if world_y == entry.pos.y {
                    covered_own_y = true;
                }
                let is_source = world_y >= new_boundary;
                check_node_sky(
                    local,
                    BlockPos::new(entry.pos.x, world_y, entry.pos.z),
                    is_source,
                    &mut state.sky,
                );
            }
            if !covered_own_y {
                let is_source = entry.pos.y >= new_boundary;
                check_node_sky(local, entry.pos, is_source, &mut state.sky);
            }
        });
    }

    // Step 2: bulk full-chunk recompute for every entity whose `LightColumn` is
    // still freshly `new_uninitialized()` and was not already covered by step 1.
    let mut all_chunks: Vec<(ChunkKey, Entity)> =
        key_to_entity.iter().map(|(&k, &e)| (k, e)).collect();
    all_chunks.sort_by_key(|(k, _)| (k.dimension.0, k.x, k.z));
    for (chunk_key, entity) in &all_chunks {
        if fresh_handled.contains(entity) {
            continue;
        }
        let fresh = world
            .get::<LightColumn>(*entity)
            .map(is_fresh)
            .unwrap_or(false);
        if !fresh {
            continue;
        }
        fresh_handled.insert(*entity);

        with_chunk(world, *entity, *chunk_key, &properties, |local, state| {
            for x in 0u8..16 {
                for z in 0u8..16 {
                    for world_y in WORLD_MIN_Y..(WORLD_MIN_Y + WORLD_HEIGHT) {
                        let raw = local.blocks.get(x, world_y, z);
                        let emission = local.properties.resolve(raw).block_emission;
                        if emission > 0 {
                            let pos = BlockPos::new(
                                chunk_key.x * 16 + x as i32,
                                world_y,
                                chunk_key.z * 16 + z as i32,
                            );
                            check_node_block(local, pos, 0, emission, &mut state.block);
                        }
                    }
                }
            }

            *local.sky_sources =
                SkyLightSourceColumn::recompute(local.blocks, local.heightmap, local.properties);
            for x in 0u8..16 {
                for z in 0u8..16 {
                    let boundary = local.sky_sources.boundary_y(x, z);
                    let start = boundary.max(WORLD_MIN_Y);
                    for world_y in start..(WORLD_MIN_Y + WORLD_HEIGHT) {
                        let pos = BlockPos::new(
                            chunk_key.x * 16 + x as i32,
                            world_y,
                            chunk_key.z * 16 + z as i32,
                        );
                        check_node_sky(local, pos, true, &mut state.sky);
                    }
                }
            }
        });
    }

    // Step 3: drain the inbound cross-region seed queue.
    #[cfg(feature = "server-systems")]
    {
        if let Some(mut inbox) = world.get_resource_mut::<rc_scheduler::LightBorderInbox>() {
            let inbound = std::mem::take(&mut inbox.0);
            for ev in &inbound {
                if let Some(entity) = key_to_entity.get(&ev.chunk)
                    && let Some(mut state) = world.get_mut::<LightPropagatorState>(*entity)
                {
                    crate::light::border::apply_inbound_light_border_update(&mut state, ev);
                }
            }
        }
    }

    // --- Rounds 0..16 (parallel via `ParallelDispatch`) ---
    let mut rounds_run: u32 = 0;
    let mut converged = false;
    // Every chunk touched at any point this invocation -- Stage 10's own candidate
    // set for cross-region emission (below).
    let mut ever_touched: std::collections::HashSet<Entity> = std::collections::HashSet::new();

    loop {
        let mut touched: Vec<(ChunkKey, Entity)> = Vec::new();
        for (key, entity) in &all_chunks {
            let idle = world
                .get::<LightPropagatorState>(*entity)
                .map(|s| s.is_idle())
                .unwrap_or(true);
            if !idle {
                touched.push((*key, *entity));
            }
        }
        touched.sort_by_key(|(k, _)| (k.dimension.0, k.x, k.z));

        if touched.is_empty() {
            converged = true;
            break;
        }
        for (_, entity) in &touched {
            ever_touched.insert(*entity);
        }

        // Step 6/7: collect disjoint mutable refs via one sequential Query pass,
        // build one boxed closure per touched chunk, dispatch onto `pool`.
        {
            let touched_entities: std::collections::HashSet<Entity> =
                touched.iter().map(|(_, e)| *e).collect();
            let touched_keys: HashMap<Entity, ChunkKey> =
                touched.iter().map(|(k, e)| (*e, *k)).collect();

            let mut query = world.query::<ChunkComponentsWithEntity>();
            let mut items: Vec<(ChunkKey, ChunkComponents)> = Vec::with_capacity(touched.len());
            for (entity, state, light, sky_sources, blocks, heightmap) in query.iter_mut(world) {
                if !touched_entities.contains(&entity) {
                    continue;
                }
                let key = touched_keys[&entity];
                items.push((
                    key,
                    (
                        state.into_inner(),
                        light.into_inner(),
                        sky_sources.into_inner(),
                        blocks,
                        heightmap,
                    ),
                ));
            }

            let mut tasks: Vec<Box<dyn FnOnce() + Send + '_>> = Vec::with_capacity(items.len());
            let mut remaining = items.as_mut_slice();
            while let Some((first, rest)) = remaining.split_first_mut() {
                remaining = rest;
                let (chunk_key, (state, light, sky_sources, blocks, heightmap)) = first;
                let chunk_key = *chunk_key;
                let properties = &properties;
                tasks.push(Box::new(move || {
                    let mut local = LocalChunkLight {
                        light,
                        sky_sources,
                        blocks,
                        heightmap,
                        properties,
                        chunk_origin_x: chunk_key.x * 16,
                        chunk_origin_z: chunk_key.z * 16,
                    };
                    run_one_round(&mut local, state);
                }));
            }
            pool.run_batch(tasks);
        }

        // Step 8: merge (sequential, deterministic ascending-ChunkKey order).
        for (_, entity) in &touched {
            let outgoing_sky = world
                .get_mut::<LightPropagatorState>(*entity)
                .map(|mut s| std::mem::take(&mut s.sky.outgoing))
                .unwrap_or_default();
            let outgoing_block = world
                .get_mut::<LightPropagatorState>(*entity)
                .map(|mut s| std::mem::take(&mut s.block.outgoing))
                .unwrap_or_default();

            merge_outgoing(
                world,
                &key_to_entity,
                &region_ownership_or_default(world),
                LightChannel::Sky,
                outgoing_sky,
            );
            merge_outgoing(
                world,
                &key_to_entity,
                &region_ownership_or_default(world),
                LightChannel::Block,
                outgoing_block,
            );
        }

        rounds_run += 1;
        if rounds_run == 16 {
            let any_not_idle = all_chunks.iter().any(|(_, entity)| {
                world
                    .get::<LightPropagatorState>(*entity)
                    .map(|s| !s.is_idle())
                    .unwrap_or(false)
            });
            if any_not_idle {
                break;
            }
        }
    }

    // --- Cross-region emission (sequential, once) ---
    emit_cross_region_updates(world, &key_to_entity, &ever_touched, &all_chunks);

    LightTickReport {
        rounds_run,
        converged,
        chunks_touched: ever_touched.len(),
    }
}

/// One round of local drain for one chunk: sky then block (order between channels
/// does not matter, Context §8 step 7), decrease fully before increase, per channel.
fn run_one_round(local: &mut LocalChunkLight, state: &mut LightPropagatorState) {
    for channel in [LightChannel::Sky, LightChannel::Block] {
        let channel_state = match channel {
            LightChannel::Sky => &mut state.sky,
            LightChannel::Block => &mut state.block,
        };
        while let Some(entry) = channel_state.decrease.pop_front() {
            propagate_decrease_step(local, entry, channel, channel_state);
        }
        while let Some(entry) = channel_state.increase.pop_front() {
            propagate_increase_step(local, entry, channel, channel_state);
        }
    }
}

/// A `RegionOwnership::always_local` fallback for a `World` that carries none --
/// mirrors the rest of this file's own "silently do nothing extra" tolerance for a
/// minimally-populated test `World` (Constraints (f): production wiring is a future
/// chunk-lifecycle blueprint's job).
fn region_ownership_or_default(world: &World) -> RegionOwnership {
    match world.get_resource::<RegionOwnership>() {
        Some(o) => RegionOwnership {
            local: o.local,
            resolve: Box::new({
                let local = o.local;
                move |_| local
            }),
        },
        None => RegionOwnership::always_local(Address::Region(rc_messaging::RegionId(0))),
    }
}

fn merge_outgoing(
    world: &mut World,
    key_to_entity: &HashMap<ChunkKey, Entity>,
    ownership: &RegionOwnership,
    channel: LightChannel,
    outgoing: Vec<(ChunkKey, crate::light::queue::QueueEntry)>,
) {
    for (target_key, entry) in outgoing {
        let owner = (ownership.resolve)(target_key);
        if owner != ownership.local {
            // A region border -- handled entirely by the cross-region emission pass,
            // never re-queued locally (avoids double-counting the same crossing).
            continue;
        }
        let Some(&target_entity) = key_to_entity.get(&target_key) else {
            continue;
        };
        if let Some(mut state) = world.get_mut::<LightPropagatorState>(target_entity) {
            let channel_state = match channel {
                LightChannel::Sky => &mut state.sky,
                LightChannel::Block => &mut state.block,
            };
            // See `propagator.rs`'s own deferred-entry convention (this module's own
            // doc comment on `border.rs` restates it): `increase_from_emission: true`
            // marks an entry `propagate_increase_step` itself deferred (belongs on
            // the target's own increase queue); `false` marks one
            // `propagate_decrease_step` deferred (belongs on the target's own
            // decrease queue) -- the two producers of this crate's own `outgoing`
            // list are mutually exclusive on this flag by construction.
            if entry.increase_from_emission {
                channel_state.increase.push_back(entry);
            } else {
                channel_state.decrease.push_back(entry);
            }
        }
    }
}

const BORDER_DIRECTIONS: [Direction; 4] = [
    Direction::West,
    Direction::East,
    Direction::North,
    Direction::South,
];

/// Step 10: for every chunk touched this invocation that directly borders a
/// chunk owned by a different region, emits one `LightBorderUpdate` per
/// `(chunk, section, face)` combination whose face currently carries tracked
/// (non-`Uninitialized`) data -- a conservative superset of "that face changed this
/// tick" (Context §8 step 10's own wording), safe since a resend of already-correct
/// data can only ever cost extra bytes, never propagate wrong light (mirrors
/// `build_update_light_payload`'s own identical "always send every tracked section"
/// conservative stance, Context §12).
fn emit_cross_region_updates(
    world: &mut World,
    key_to_entity: &HashMap<ChunkKey, Entity>,
    ever_touched: &std::collections::HashSet<Entity>,
    all_chunks: &[(ChunkKey, Entity)],
) {
    let Some(ownership) = world.get_resource::<RegionOwnership>() else {
        return;
    };
    let local = ownership.local;
    let resolve = &ownership.resolve;

    // Every `(sending chunk, sending-chunk-own-outward-face, receiving chunk)`
    // triple this invocation's own touched set borders a differently-owned chunk
    // through -- resolved once, up front, so `world` is never held both mutably
    // (below, per-chunk column reads) and immutably (this closure's own borrow of
    // `ownership`) at the same time.
    let border_triples: Vec<(ChunkKey, Direction, ChunkKey)> = {
        let mut results = Vec::new();
        for (chunk_key, entity) in all_chunks {
            if !ever_touched.contains(entity) {
                continue;
            }
            for dir in BORDER_DIRECTIONS {
                let (dx, _, dz) = dir.offset();
                let neighbor_key =
                    ChunkKey::new(chunk_key.dimension, chunk_key.x + dx, chunk_key.z + dz);
                if !key_to_entity.contains_key(&neighbor_key) {
                    continue;
                }
                if (resolve)(neighbor_key) == local {
                    continue;
                }
                results.push((*chunk_key, dir, neighbor_key));
            }
        }
        results
    };

    #[cfg(feature = "server-systems")]
    for (chunk_key, dir, neighbor_key) in border_triples {
        let Some(&entity) = key_to_entity.get(&chunk_key) else {
            continue;
        };
        let Some(column) = world.get::<LightColumn>(entity) else {
            continue;
        };
        let mut messages = Vec::new();
        for (index, section) in column.sections().iter().enumerate() {
            // `Filled(0)` is structurally tracked but carries no real light data
            // (vanilla's own empty case, WORLD-D8) -- excluded here the same way
            // `build_update_light_payload` excludes it from its own non-empty mask,
            // so a section nobody ever wrote a real value into never triggers a
            // spurious cross-region message.
            let has_data = |n: &LightNibbles| {
                !matches!(n, LightNibbles::Uninitialized | LightNibbles::Filled(0))
            };
            if !has_data(&section.sky) && !has_data(&section.block) {
                continue;
            }
            let update = crate::light::border::build_light_border_update(
                neighbor_key,
                index as u8,
                dir,
                column,
            );
            messages.push(update);
        }
        if let Some(mut outbox) = world.get_resource_mut::<rc_scheduler::RegionMessageOutbox>() {
            for update in messages {
                outbox.send(
                    Address::Chunk(neighbor_key),
                    rc_messaging::RegionMessage::LightBorderUpdate(Box::new(update)),
                );
            }
        }
    }
    #[cfg(not(feature = "server-systems"))]
    let _ = border_triples;
}
