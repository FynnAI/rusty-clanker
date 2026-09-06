//! M4-B10 (Context §E): the entity-presence census + `on_entity_inside` trigger driver a
//! pressure plate needs. Lives here, not `rc-mechanics`, because only this crate can see both
//! `PlayerMarker`/`PlayerMotion` players and `rc_mechanics::entity::BaseEntity` mobs/items at
//! once (WS-D3 rule 2 -- the same crate-boundary rule M4-B02's own item-pickup driver already
//! hit). Mirrors `entity_tracking::entity_pickup_step`'s established "manual tick-loop step,
//! no `bevy_ecs::System`" shape exactly.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use rc_core::{BlockPos, DimensionId};
use rc_mechanics::behavior::{EntityTouch, UpdateContext};
use rc_mechanics::block_event::BlockEventQueue;
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::entity::physics::entity_dimensions;
use rc_mechanics::entity::{BaseEntity, EntityPayload};
use rc_mechanics::neighbor_update::NeighborUpdateEngine;
use rc_mechanics::redstone::{EntityClassFilter, EntityPresenceSource};
use rc_mechanics::scheduled_tick::ScheduledTickQueue;
use rc_mechanics::stage4::ecs::{TickChangedPositions, TickSoundOutbox};
use rc_mechanics::{BlockBehaviorRegistry, LightDirtyQueue, SoundRequest};
use rc_messaging::{Address, RegionMessage};
use rc_physics::{Aabb, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PLAYER_HEIGHT_SNEAKING, Vec3};
use rc_scheduler::RegionMessageOutbox;

use super::movement::{PlayerInputState, PlayerMotion};
use super::world::{DirectBlockWorld, HARDCODED_REGION_ID};

/// One censused entity (Context §E). `is_spectator`/`ignores_block_triggers` are always
/// `false` at M4's own entity set (no spectator game mode, no armour-stand-style marker
/// exists yet) and exist so a future mechanic flips a value rather than discovering the
/// filter was never modelled at all -- mirrors vanilla's own `getEntityCount`'s
/// `EntitySelector.NO_SPECTATORS` plus "does not ignore block triggers" predicate pair.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EntityPresenceRecord {
    pub aabb: Aabb,
    pub is_living: bool,
    pub is_spectator: bool,
    pub ignores_block_triggers: bool,
}

/// The region's own `EntityPresenceSource` (Context §E). One instance per region, shared via
/// two `Arc` clones with `register_tier2_inputs` (read side, held by every `PressurePlateBehavior`)
/// and `entity_inside_step` (write side) -- the identical shape `Tier1ContainerSignalSource`
/// established. The `Mutex` is never contended: `refresh` and every Stage-4 read run strictly
/// sequentially within one region's own tick.
pub struct RegionEntityPresence {
    records: Mutex<Vec<EntityPresenceRecord>>,
}

impl RegionEntityPresence {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    /// Replaces the whole census. Called once per tick, first thing in `entity_inside_step`.
    pub fn refresh(&self, records: Vec<EntityPresenceRecord>) {
        *self.records.lock().unwrap() = records;
    }

    /// Read-only snapshot of the current census -- the driver's own cell-enumeration input.
    pub fn snapshot(&self) -> Vec<EntityPresenceRecord> {
        self.records.lock().unwrap().clone()
    }
}

impl Default for RegionEntityPresence {
    fn default() -> Self {
        Self::new()
    }
}

/// `AABB.intersects` (Context §E): strict inequality on both bounds, never inclusive equality
/// at either bound -- a box merely touching another's face never counts as intersecting it.
fn aabb_intersects(a: Aabb, b: Aabb) -> bool {
    a.min.x < b.max.x
        && a.max.x > b.min.x
        && a.min.y < b.max.y
        && a.max.y > b.min.y
        && a.min.z < b.max.z
        && a.max.z > b.min.z
}

impl EntityPresenceSource for RegionEntityPresence {
    /// Vanilla's `getEntityCount(level, box, class)`: owned (ARCH-D10 -- this census is built
    /// from one region's own `bevy_ecs::World`, so it contains exactly the entities that
    /// region owns, by construction), non-spectator, non-block-trigger-ignoring entities whose
    /// own AABB strictly intersects `region`.
    fn count_entities_in(&self, region: Aabb, filter: EntityClassFilter) -> usize {
        let records = self.records.lock().unwrap();
        records
            .iter()
            .filter(|record| !record.is_spectator && !record.ignores_block_triggers)
            .filter(|record| match filter {
                EntityClassFilter::AnyEntity => true,
                EntityClassFilter::LivingOnly => record.is_living,
            })
            .filter(|record| aabb_intersects(record.aabb, region))
            .count()
    }
}

/// `Resource` wrapper, inserted by the composition root (mirrors `ContainerSignalsResource`).
#[derive(Resource, Clone)]
pub struct EntityPresenceResource(pub Arc<RegionEntityPresence>);

/// `1.0E-5` (Context §E: "the entity's own post-tick AABB deflated by `1.0E-5`") -- shrinks
/// `aabb` inward on all six faces by `epsilon`.
fn deflate(aabb: Aabb, epsilon: f64) -> Aabb {
    Aabb {
        min: Vec3::new(
            aabb.min.x + epsilon,
            aabb.min.y + epsilon,
            aabb.min.z + epsilon,
        ),
        max: Vec3::new(
            aabb.max.x - epsilon,
            aabb.max.y - epsilon,
            aabb.max.z - epsilon,
        ),
    }
}

const INSIDE_BLOCKS_DEFLATE_EPSILON: f64 = 1.0e-5;

/// Context §E's manual tick-loop step: refresh the census, then dispatch
/// `BlockBehavior::on_entity_inside` at every cell any censused entity's own `1.0E-5`-deflated
/// AABB intersects (deduplicated per entity), settling the neighbour-update engine to a fixed
/// point after each dispatch, and merging `changed`/`sounds` into `TickChangedPositions`/
/// `TickSoundOutbox`. Runs after `entity_pickup_step`/`entity_resync_step` (so it observes this
/// tick's own fresh Stage-6b physics output) and before those two drains (so anything it
/// changes is broadcast this same tick with no call-site change).
pub fn entity_inside_step(world: &mut World, current_tick: u64) {
    // Step 1: rebuild the census directly off the live ECS world.
    let mut records: Vec<EntityPresenceRecord> = Vec::new();

    {
        let mut player_query = world.query::<(&PlayerMotion, &PlayerInputState)>();
        for (motion, input) in player_query.iter(world) {
            let height = if input.sneaking {
                PLAYER_HEIGHT_SNEAKING
            } else {
                PLAYER_HEIGHT
            };
            let aabb = Aabb::from_position(motion.position, PLAYER_HALF_WIDTH, height);
            records.push(EntityPresenceRecord {
                aabb,
                is_living: true,
                is_spectator: false,
                ignores_block_triggers: false,
            });
        }
    }

    {
        let mut entity_query = world.query::<(&BaseEntity, &EntityPayload)>();
        for (base, payload) in entity_query.iter(world) {
            let (half_width, height) = entity_dimensions(payload.kind());
            let pos = Vec3::new(base.pos[0], base.pos[1], base.pos[2]);
            let aabb = Aabb::from_position(pos, half_width, height);
            let is_living = !matches!(payload, EntityPayload::Item(_));
            records.push(EntityPresenceRecord {
                aabb,
                is_living,
                is_spectator: false,
                ignores_block_triggers: false,
            });
        }
    }

    let presence = world.resource::<EntityPresenceResource>().0.clone();
    presence.refresh(records.clone());

    // Step 2: pull this tick's own Stage-4 resources out of `world`, exactly as the
    // direct-action phase already does (`crates/server/src/play/world.rs`'s own doc comment on
    // that pattern).
    let mut engine = world
        .remove_resource::<NeighborUpdateEngine>()
        .expect("bootstrap_default_stage4_resources always inserts this");
    let mut scheduled = world
        .remove_resource::<ScheduledTickQueue>()
        .expect("bootstrap_default_stage4_resources always inserts this");
    let mut events = world
        .remove_resource::<BlockEventQueue>()
        .expect("bootstrap_default_stage4_resources always inserts this");
    let behaviors = world
        .remove_resource::<BlockBehaviorRegistry>()
        .expect("bootstrap_default_stage4_resources always inserts this");
    let mut light_dirty = world
        .remove_resource::<LightDirtyQueue>()
        .expect("bootstrap_default_stage4_resources always inserts this");

    let ownership = RegionOwnership::always_local(Address::Region(HARDCODED_REGION_ID));
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, rc_chunk_storage::BlockStateId)> = Vec::new();
    let mut sounds: Vec<SoundRequest> = Vec::new();

    // Step 3: for every censused entity, enumerate the block positions its own deflated AABB
    // intersects (deduplicated across that entity's own cells), dispatch `on_entity_inside`,
    // then drain the neighbour-update engine to a fixed point after each dispatch -- the same
    // per-entry settle `stage4::run_scheduled_phase` performs for a scheduled tick.
    for record in &records {
        let deflated = deflate(record.aabb, INSIDE_BLOCKS_DEFLATE_EPSILON);
        let mut visited: HashSet<BlockPos> = HashSet::new();
        for pos in deflated.overlapped_block_positions() {
            if !visited.insert(pos) {
                continue;
            }
            let touch = EntityTouch {
                aabb: record.aabb,
                is_living: record.is_living,
            };
            {
                let mut direct = DirectBlockWorld {
                    world,
                    dimension: DimensionId::OVERWORLD,
                    local: Address::Region(HARDCODED_REGION_ID),
                };
                let mut ctx = UpdateContext {
                    world: &mut direct,
                    engine: &mut engine,
                    scheduled: &mut scheduled,
                    events: &mut events,
                    outbound: &mut outbound,
                    changed: &mut changed,
                    ownership: &ownership,
                    current_tick,
                    light_dirty: &mut light_dirty,
                    sounds: &mut sounds,
                };
                if let Some(behavior_state) = ctx.get_block(pos) {
                    let behavior = behaviors.resolve(behavior_state);
                    behavior.on_entity_inside(&mut ctx, pos, &touch);
                }
            }
            super::mining::settle_neighbor_updates(
                &mut DirectBlockWorld {
                    world,
                    dimension: DimensionId::OVERWORLD,
                    local: Address::Region(HARDCODED_REGION_ID),
                },
                &mut engine,
                &mut scheduled,
                &mut events,
                &mut outbound,
                &mut changed,
                &mut light_dirty,
                &mut sounds,
                &ownership,
                &behaviors,
                current_tick,
            );
        }
    }

    // Step 4: merge into the tick-wide resources, then reinsert every resource this step took
    // out, mirroring the direct-action phase's own identical reinsert tail.
    if !outbound.is_empty() {
        let mut outbox = world.resource_mut::<RegionMessageOutbox>();
        for (to, msg) in outbound {
            outbox.send(to, msg);
        }
    }
    world.resource_mut::<TickChangedPositions>().merge(changed);
    world.resource_mut::<TickSoundOutbox>().merge(sounds);

    world.insert_resource(engine);
    world.insert_resource(scheduled);
    world.insert_resource(events);
    world.insert_resource(behaviors);
    world.insert_resource(light_dirty);
}
