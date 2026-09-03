//! The real `DomainGroup::EntityPhysicsIntegration` registration (Context §A) — the first
//! content in that group, `order_tag = 0`. `system_entity_physics_integration` drives every
//! non-player entity's per-tick physics (item vs. living tick shape), fluid interaction,
//! drowning/air, the fall-damage hook, item-vs-item merge, the item pickup-delay countdown,
//! and item age-despawn — all inside this one system.
//!
//! **Two necessary, documented deviations from this blueprint's own literal text**
//! (`docs/findings-for-planning.md`):
//!
//! 1. Player-touching PICKUP (an item entity's own AABB against a `PlayerMarker`'s own AABB)
//!    is **not** implemented here, even though this blueprint's own prose says pickup runs
//!    "all inside this one system" — `rc-mechanics` structurally cannot see `PlayerMarker`/
//!    `PlayerMotion` (`rusty-clanker-server`, WS-D3 rule 2's binding crate-boundary rule,
//!    restated in this blueprint's own Context §A: "this system never touches a player, and
//!    cannot even express a player-exclusion filter"). This system owns only the
//!    player-independent half of pickup (the `pickup_delay_ticks` countdown); the
//!    player-touching eligibility check, the item entity's own despawn-on-pickup, the `Take
//!    Item Entity` broadcast, and the `PickedUpItems` append all live in
//!    `rusty-clanker-server::play::entity_tracking::entity_pickup_step`, a new manual
//!    tick-loop step (mirroring `entity_resync_step`'s own established shape) that CAN see
//!    both sides.
//! 2. The swimming/viscosity DRAG-REPLACEMENT formulas (Context §E's water `(0.8,0.8,0.8)`
//!    and lava shallow/deep branches) are not implemented — only the fluid PUSH (velocity
//!    addition) this blueprint's own six `fluid_push_vectors.rs` acceptance tests actually
//!    assert. Two independent reasons this file's own Context §E cannot be honored literally:
//!    (a) for a living tier-2 kind, "in place of the kind-specific drag step" means replacing
//!    a step *inside* `rc_physics::step_living_entity_tick`'s own body — a function Context
//!    §B itself mandates is "reused completely unmodified," and `rc-physics` is outside this
//!    blueprint's own Crates-touched list; (b) for an item entity, Context §E's own push
//!    ordering computes submersion from the entity's *post-move* AABB (after
//!    `step_item_entity_tick` has already run its own drag step to completion), so the
//!    submersion value the swim/viscosity branch would need is not yet known at the point
//!    that same tick's drag step itself runs — an unresolved forward reference this
//!    blueprint's own text does not work through. No acceptance test in this blueprint's own
//!    suite exercises either drag-replacement path.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, ChunkKeyTag};
use rc_core::{BlockPos, DimensionId, RcEntityId};
use rc_physics::aabb::Axis;
use rc_physics::{
    Aabb, BlockPhysicsProperties, BlockShapeSource, LivingMotionState, MovementIntent, ShapeTable,
    Vec3, step_living_entity_tick,
};
use rc_scheduler::{CurrentTick, DomainGroup, RcExecutorBuilder, SystemFactory};

use super::fluid_interaction::{
    LAVA_PUSH_SCALE_FAST, LAVA_PUSH_SCALE_SLOW, WATER_PUSH_SCALE, apply_fluid_push, eyes_in_fluid,
    scan_fluid_interaction,
};
use super::item::{ITEM_HALF_WIDTH, ITEM_HEIGHT, ItemMotionState, step_item_entity_tick};
use super::world_bridge::ReadOnlyBlockWorld;
use super::{PendingEnvironmentalDamage, PendingEnvironmentalDamageQueue};
use crate::entity::pickup::{DESPAWN_AGE_TICKS, MERGE_RADIUS, stacks_can_combine};
use crate::entity::{BaseEntity, EntityKind, EntityPayload, ItemStackRecord, LivingEntity};
use crate::fluid::{FluidKind, FluidTables};
use crate::stage4::ecs::ChunkIndex;
use crate::world_access::BlockWorldAccess;

/// `09-entities-ai.md`'s own documented drowning constants (Context §F).
const TOTAL_AIR_SUPPLY: i32 = 300;
const AIR_FLOOR: i32 = -20;

/// Context §D's per-kind (half_width, height) table — restated here since `rc_physics::
/// step_living_entity_tick` itself has no per-kind dimension parameter (this file's own
/// module doc comment, deviation 2's sibling note): every AABB *this file* builds directly
/// (fluid scan, eye position) uses the entity's own real dimensions; only the sealed
/// `step_living_entity_tick` call itself falls back to that function's own internal
/// player-shaped geometry.
fn living_dimensions(kind: EntityKind) -> (f64, f64) {
    match kind {
        EntityKind::Zombie | EntityKind::Villager => (0.3, 1.95),
        EntityKind::Cow => (0.45, 1.4),
        EntityKind::Item => (ITEM_HALF_WIDTH, ITEM_HEIGHT),
    }
}

fn eye_height(height: f64) -> f64 {
    height * 0.85
}

fn aabb_overlaps(a: Aabb, b: Aabb) -> bool {
    a.overlaps_on(Axis::X, b, 0.0)
        && a.overlaps_on(Axis::Y, b, 0.0)
        && a.overlaps_on(Axis::Z, b, 0.0)
}

fn inflate(aabb: Aabb, dx: f64, dy: f64, dz: f64) -> Aabb {
    Aabb {
        min: Vec3::new(aabb.min.x - dx, aabb.min.y - dy, aabb.min.z - dz),
        max: Vec3::new(aabb.max.x + dx, aabb.max.y + dy, aabb.max.z + dz),
    }
}

/// The supporting block's own friction — the block directly beneath `position`'s own feet, a
/// small epsilon below to reliably resolve the block a resting entity's feet sit exactly on
/// top of (mirrors `crate::stage4::ecs::EcsBlockWorld`'s own boundary-epsilon convention).
fn ground_friction_at(shapes: &dyn BlockShapeSource, position: Vec3) -> f64 {
    let below = BlockPos::new(
        position.x.floor() as i32,
        (position.y - 1e-3).floor() as i32,
        position.z.floor() as i32,
    );
    shapes.properties_at(below).friction
}

fn lava_push_scale(tables: &FluidTables) -> f64 {
    if tables.dimension.fast_lava {
        LAVA_PUSH_SCALE_FAST
    } else {
        LAVA_PUSH_SCALE_SLOW
    }
}

fn entity_to_rc_id(entity: Entity) -> RcEntityId {
    RcEntityId(entity.to_bits())
}

/// A thin `rc_physics::BlockShapeSource` adapter over `ReadOnlyBlockWorld` +
/// `rc_physics::tier1_shape_table()`, mirroring `rusty-clanker-server`'s own
/// `ChunkBlockShapeSource` (M3-B02) exactly, defined here since `rc-mechanics` cannot depend
/// on `rusty-clanker-server` and this system needs its own copy of the identical bridge.
struct EntityBlockShapeSource<'a> {
    world: &'a ReadOnlyBlockWorld<'a, 'a>,
    shape_table: &'static ShapeTable,
}

impl<'a> rc_physics::BlockShapeSource for EntityBlockShapeSource<'a> {
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties {
        match self.world.get_block(pos) {
            Some(state) => self.shape_table.lookup(state.0),
            None => BlockPhysicsProperties::air(),
        }
    }
}

/// Composition-root-supplied wrapper resources (Context §K's own "the mechanism now, the
/// real data-driven wiring later" precedent, mirroring `FluidDimensionProfile`).
#[derive(Resource)]
pub struct ShapeTableResource(pub &'static ShapeTable);
#[derive(Resource)]
pub struct DimensionResource(pub DimensionId);

/// Registers `system_entity_physics_integration` into `DomainGroup::EntityPhysicsIntegration`
/// at `order_tag = 0` (Context §A). Not the group's only member — M4-B04's own
/// `system_mob_despawn` and M4-B05's mob-combat system each land in this same group; this
/// function must be called first so this system keeps `order_tag = 0` regardless of which of
/// the other two land afterward.
pub fn register_stage6b(builder: &mut RcExecutorBuilder) {
    builder.register_system(
        DomainGroup::EntityPhysicsIntegration,
        entity_physics_integration_factory(),
        vec![],
    );
}

fn entity_physics_integration_factory() -> SystemFactory {
    Box::new(|| Box::new(IntoSystem::into_system(system_entity_physics_integration)))
}

/// One item entity's post-tick snapshot — the merge scan's own input (Pass 2 needs no live
/// `Query` borrow, since Pass 1's own `query.iter_mut()` loop has already ended by the time
/// Pass 2 runs).
struct ItemSnapshot {
    entity: Entity,
    pre_position: Vec3,
    position: Vec3,
    stack: ItemStackRecord,
    age_ticks: i16,
}

/// The Stage 6b system itself (Context §A–§N, this file's own module doc comment has the
/// two documented scope deviations). Never matches a player entity — a player carries
/// `PlayerMotion`/`TeleportState` (`rusty-clanker-server`, M3-B02), not `BaseEntity`, so this
/// system's own `Query<(Entity, &mut BaseEntity, ...)>` structurally cannot select one.
#[allow(clippy::too_many_arguments)]
fn system_entity_physics_integration(
    mut query: Query<(
        Entity,
        &'static mut BaseEntity,
        Option<&'static mut LivingEntity>,
        &'static mut EntityPayload,
    )>,
    world_query: Query<(&'static ChunkKeyTag, &'static BlockStateColumn)>,
    chunk_index: Res<ChunkIndex>,
    shape_table: Res<ShapeTableResource>,
    fluid_tables: Res<FluidTables>,
    dimension: Res<DimensionResource>,
    current_tick: Res<CurrentTick>,
    mut damage_queue: ResMut<PendingEnvironmentalDamageQueue>,
    mut commands: Commands,
) {
    let world = ReadOnlyBlockWorld::new(world_query, &chunk_index, dimension.0);
    let shapes = EntityBlockShapeSource {
        world: &world,
        shape_table: shape_table.0,
    };
    let lava_scale = lava_push_scale(&fluid_tables);

    let mut item_snapshots: Vec<ItemSnapshot> = Vec::new();

    for (entity, mut base, living, mut payload) in query.iter_mut() {
        let old_fall_distance = base.fall_distance;
        let kind = payload.kind();

        match (&mut *payload, living) {
            (EntityPayload::Item(item), None) => {
                let pre_position = Vec3::new(base.pos[0], base.pos[1], base.pos[2]);
                let friction = ground_friction_at(&shapes, pre_position);
                let input_state = ItemMotionState {
                    position: pre_position,
                    velocity: Vec3::new(base.velocity[0], base.velocity[1], base.velocity[2]),
                    on_ground: base.on_ground,
                    fall_distance: base.fall_distance,
                    no_gravity: base.no_gravity,
                };
                let mut new_state = step_item_entity_tick(input_state, &shapes, friction);

                // Fluid push lands AFTER this tick's own move/drag block, scanned against the
                // entity's now-updated, post-move AABB (Context §E's per-kind "after" rule).
                let post_move_aabb =
                    Aabb::from_position(new_state.position, ITEM_HALF_WIDTH, ITEM_HEIGHT);
                let water =
                    scan_fluid_interaction(post_move_aabb, &world, &fluid_tables, FluidKind::Water);
                let lava =
                    scan_fluid_interaction(post_move_aabb, &world, &fluid_tables, FluidKind::Lava);
                let mut velocity = new_state.velocity;
                velocity = apply_fluid_push(velocity, &water, WATER_PUSH_SCALE);
                velocity = apply_fluid_push(velocity, &lava, lava_scale);
                new_state.velocity = velocity;

                // Fall-damage hook: item entities never take fall damage in vanilla (Context
                // §G) — `ItemEntity` simply inherits the base, damage-free `Entity.
                // causeFallDamage`, so no event is ever pushed for an item entity.

                base.pos = [
                    new_state.position.x,
                    new_state.position.y,
                    new_state.position.z,
                ];
                base.velocity = [
                    new_state.velocity.x,
                    new_state.velocity.y,
                    new_state.velocity.z,
                ];
                base.on_ground = new_state.on_ground;
                base.fall_distance = new_state.fall_distance;

                // Context §M's own player-independent half: the pickup-delay countdown.
                if item.pickup_delay_ticks > 0 {
                    item.pickup_delay_ticks -= 1;
                }
                // Context §N: unconditional per-tick age increment.
                item.age_ticks += 1;

                item_snapshots.push(ItemSnapshot {
                    entity,
                    pre_position,
                    position: new_state.position,
                    stack: item.item.clone(),
                    age_ticks: item.age_ticks,
                });
            }
            (
                EntityPayload::Zombie(_) | EntityPayload::Villager(_) | EntityPayload::Cow(_),
                Some(_living),
            ) => {
                let (half_width, height) = living_dimensions(kind);
                let start_position = Vec3::new(base.pos[0], base.pos[1], base.pos[2]);
                let start_velocity =
                    Vec3::new(base.velocity[0], base.velocity[1], base.velocity[2]);

                // Fluid interaction is scanned against the entity's own START-of-tick AABB
                // (its own real Context §D dimensions, not `step_living_entity_tick`'s own
                // internal player-shaped geometry -- this file's own module doc comment,
                // deviation 2), and the push is folded into velocity BEFORE the tick call
                // (Context §E's per-kind "before" rule).
                let start_aabb = Aabb::from_position(start_position, half_width, height);
                let water =
                    scan_fluid_interaction(start_aabb, &world, &fluid_tables, FluidKind::Water);
                let lava =
                    scan_fluid_interaction(start_aabb, &world, &fluid_tables, FluidKind::Lava);
                let mut pushed_velocity = start_velocity;
                pushed_velocity = apply_fluid_push(pushed_velocity, &water, WATER_PUSH_SCALE);
                pushed_velocity = apply_fluid_push(pushed_velocity, &lava, lava_scale);

                let friction = ground_friction_at(&shapes, start_position);
                let input_state = LivingMotionState {
                    position: start_position,
                    velocity: pushed_velocity,
                    on_ground: base.on_ground,
                    fall_distance: base.fall_distance,
                };
                let new_state = step_living_entity_tick(
                    input_state,
                    MovementIntent::default(),
                    friction,
                    &shapes,
                );

                base.pos = [
                    new_state.position.x,
                    new_state.position.y,
                    new_state.position.z,
                ];
                base.velocity = [
                    new_state.velocity.x,
                    new_state.velocity.y,
                    new_state.velocity.z,
                ];
                base.on_ground = new_state.on_ground;
                base.fall_distance = new_state.fall_distance;

                if new_state.on_ground && old_fall_distance > 0.0 {
                    damage_queue.0.push(PendingEnvironmentalDamage::FallImpact {
                        entity: entity_to_rc_id(entity),
                        fall_distance: old_fall_distance,
                    });
                }

                // Drowning/air (Context §F) -- living tier-2 kinds only.
                let eye_pos = Vec3::new(
                    new_state.position.x,
                    new_state.position.y + eye_height(height),
                    new_state.position.z,
                );
                if eyes_in_fluid(eye_pos, &world, &fluid_tables, FluidKind::Water) {
                    base.air_ticks -= 1;
                } else {
                    base.air_ticks = (base.air_ticks + 4).min(TOTAL_AIR_SUPPLY);
                }
                if base.air_ticks == AIR_FLOOR {
                    base.air_ticks = 0;
                    damage_queue.0.push(PendingEnvironmentalDamage::Drowning {
                        entity: entity_to_rc_id(entity),
                        suggested_magnitude: 2.0,
                    });
                }
            }
            _ => {
                // Structurally unreachable for the four tier-2 kinds this blueprint ships
                // (`Item` never carries `LivingEntity`; `Zombie`/`Villager`/`Cow` always do) --
                // skip silently rather than panic, so a future kind added without its own
                // `LivingEntity` rung degrades gracefully instead of crashing every tick.
            }
        }
    }

    run_merge_scan(&mut query, &mut commands, &item_snapshots, current_tick.0);
    despawn_aged_items(&mut commands, &item_snapshots);
}

/// Context §L — the merge scan, cadence-gated: every 2nd tick when an item entity crossed an
/// integer block-cell boundary this tick, otherwise every 40th tick. Runs over the pass-1
/// snapshot (no live `Query` borrow needed for the pairwise comparison itself); mutation
/// (count bump, despawn) goes through a fresh `query.get_mut`/`commands.entity` call per
/// decided pair, after the snapshot comparison is complete.
fn run_merge_scan(
    query: &mut Query<(
        Entity,
        &'static mut BaseEntity,
        Option<&'static mut LivingEntity>,
        &'static mut EntityPayload,
    )>,
    commands: &mut Commands,
    snapshots: &[ItemSnapshot],
    current_tick: u64,
) {
    let mut absorbed: HashSet<Entity> = HashSet::new();

    for i in 0..snapshots.len() {
        if absorbed.contains(&snapshots[i].entity) {
            continue;
        }
        for j in (i + 1)..snapshots.len() {
            if absorbed.contains(&snapshots[j].entity) {
                continue;
            }
            let a = &snapshots[i];
            let b = &snapshots[j];

            if !due_this_tick(a, current_tick) && !due_this_tick(b, current_tick) {
                continue;
            }
            if !stacks_can_combine(&a.stack, &b.stack) {
                continue;
            }

            let a_aabb = Aabb::from_position(a.position, ITEM_HALF_WIDTH, ITEM_HEIGHT);
            let b_aabb = Aabb::from_position(b.position, ITEM_HALF_WIDTH, ITEM_HEIGHT);
            let a_inflated = inflate(a_aabb, MERGE_RADIUS, 0.0, MERGE_RADIUS);
            if !aabb_overlaps(a_inflated, b_aabb) {
                continue;
            }

            // The pair with the greater `age_ticks` survives; ties broken by the lower
            // `Entity::to_bits()` value (this project's own deterministic tie-break, Context
            // §L).
            let (survivor, absorbed_one) = if a.age_ticks != b.age_ticks {
                if a.age_ticks > b.age_ticks {
                    (a, b)
                } else {
                    (b, a)
                }
            } else if a.entity.to_bits() < b.entity.to_bits() {
                (a, b)
            } else {
                (b, a)
            };

            if let Ok((_, _, _, mut survivor_payload)) = query.get_mut(survivor.entity)
                && let EntityPayload::Item(survivor_item) = &mut *survivor_payload
            {
                survivor_item.item.count = survivor_item
                    .item
                    .count
                    .saturating_add(absorbed_one.stack.count);
            }
            commands.entity(absorbed_one.entity).despawn();
            absorbed.insert(absorbed_one.entity);
        }
    }
}

/// `true` iff `snapshot`'s own item entity crossed an integer block-cell boundary on any axis
/// this tick (Context §L's own cadence rule).
fn crossed_cell_this_tick(snapshot: &ItemSnapshot) -> bool {
    let cell = |v: Vec3| (v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32);
    cell(snapshot.pre_position) != cell(snapshot.position)
}

fn due_this_tick(snapshot: &ItemSnapshot, current_tick: u64) -> bool {
    if crossed_cell_this_tick(snapshot) {
        current_tick.is_multiple_of(2)
    } else {
        current_tick.is_multiple_of(40)
    }
}

/// Context §N — unconditional age-despawn at `age_ticks >= DESPAWN_AGE_TICKS`.
fn despawn_aged_items(commands: &mut Commands, snapshots: &[ItemSnapshot]) {
    for snapshot in snapshots {
        if snapshot.age_ticks >= DESPAWN_AGE_TICKS {
            commands.entity(snapshot.entity).despawn();
        }
    }
}
