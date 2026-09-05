//! Cross-region entity transfer (ARCH-D10) for the mob/item side — crossing detection,
//! the wire-format discriminator convention, and Stage-1 arrival application. Player
//! transfer is `rusty-clanker-server`'s own parallel, server-only mechanism (M4-B08
//! Context, Part 1.5) — this module never references `PlayerMarker`/`PlayerMotion`.

use std::sync::Arc;

use bevy_ecs::prelude::*;
use rc_core::{BlockPos, ChunkKey, DimensionId, RcEntityId};
use rc_messaging::{Address, EntitySnapshot, RegionId, RegionMessage};

use crate::border::RegionOwnership;
use crate::entity::ids::NetworkEntityIdAllocator;
use crate::entity::kinds::{
    AiSystemKind, CowBundle, ItemBundle, MobMarker, VillagerBundle, ZombieBundle,
};
use crate::entity::snapshot::{
    ComponentKind, SnapshotError, SnapshotPayload, deserialize_entity_snapshot,
    serialize_entity_snapshot,
};
use crate::entity::{BaseEntity, EntityKind, EntityPayload, LivingEntity};

pub const TRANSFER_PAYLOAD_KIND_MOB: u8 = 0;

/// Identity component attached to every mob/item entity this module spawns (fresh) or
/// re-inserts (arrival) — the query key every crossing-detection/arrival/despawn call
/// uses instead of a raw `bevy_ecs::Entity` (Context, "one cited gap this blueprint
/// closes"). Mirrors `BlockEntityHeader`'s identical role for block entities (M3-B06).
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq)]
pub struct EntityIdentity {
    pub rc_entity_id: RcEntityId,
    pub network_entity_id: i32,
    pub kind: EntityKind,
}

/// The process-wide-shared allocator wrapper (Context, Part 1.6 — the cited correction
/// to M4-B01's own per-region scope). Constructed once by a composition root that
/// intends to run more than one simultaneously-live region, and inserted, via the same
/// `Arc` clone, into every such region's own `World`.
#[derive(Resource, Clone)]
pub struct SharedNetworkEntityIdAllocator(pub Arc<NetworkEntityIdAllocator>);

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct MobTransferEnvelope {
    network_entity_id: i32,
    snapshot_bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum MobTransferDecodeError {
    #[error("postcard decode of the mob-transfer envelope failed: {0}")]
    Envelope(String),
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
}

/// The position's containing chunk — floor (not truncate-toward-zero) on every axis, the
/// identical convention `rusty-clanker-server::play::movement::feet_block_pos` already
/// establishes, restated here since `rc-mechanics` cannot depend on that server-only
/// helper.
fn chunk_key_of(pos: [f64; 3], dimension: DimensionId) -> ChunkKey {
    BlockPos::new(
        pos[0].floor() as i32,
        pos[1].floor() as i32,
        pos[2].floor() as i32,
    )
    .chunk_key(dimension)
}

/// Builds a mob/item `EntitySnapshot` ready to hand to
/// `RegionMessage::RegionTransferRequest(Box::new(...))` (Context, Part 1.3/1.4/1.6).
pub fn build_mob_entity_snapshot(
    entity_id: RcEntityId,
    source_chunk: ChunkKey,
    network_entity_id: i32,
    kind: EntityKind,
    base: &BaseEntity,
    living: Option<&LivingEntity>,
    payload: &EntityPayload,
) -> EntitySnapshot {
    let snapshot_bytes = serialize_entity_snapshot(kind, base, living, payload);
    let envelope = MobTransferEnvelope {
        network_entity_id,
        snapshot_bytes,
    };
    let mut component_data = vec![TRANSFER_PAYLOAD_KIND_MOB];
    component_data.extend(
        postcard::to_allocvec(&envelope)
            .expect("MobTransferEnvelope is always postcard-serializable"),
    );
    EntitySnapshot {
        entity_id,
        source_chunk,
        component_data,
    }
}

/// Inverse of `build_mob_entity_snapshot`. Returns `None` (not an error) if the leading
/// byte is not `TRANSFER_PAYLOAD_KIND_MOB` — the signal a combined driver (Context, Part
/// 1.3) uses to fall through to its own, non-mob decoding path; returns `Some(Err(...))`
/// for a leading byte of `TRANSFER_PAYLOAD_KIND_MOB` whose remaining bytes are malformed
/// (never a panic).
pub fn try_decode_mob_snapshot(
    component_data: &[u8],
) -> Option<Result<(i32, SnapshotPayload), MobTransferDecodeError>> {
    let (&kind_byte, rest) = component_data.split_first()?;
    if kind_byte != TRANSFER_PAYLOAD_KIND_MOB {
        return None;
    }
    let envelope: MobTransferEnvelope = match postcard::from_bytes(rest) {
        Ok(envelope) => envelope,
        Err(err) => return Some(Err(MobTransferDecodeError::Envelope(err.to_string()))),
    };
    match deserialize_entity_snapshot(&envelope.snapshot_bytes) {
        Ok(payload) => Some(Ok((envelope.network_entity_id, payload))),
        Err(err) => Some(Err(MobTransferDecodeError::Snapshot(err))),
    }
}

/// The per-`EntityKind` static `MobMarker` table (Context, Part 1.4 — reconstructed
/// fresh on every spawn/arrival, never carried across the wire). `None` for `Item`
/// (not a `Mob`-rung entity at all). Moderate confidence on `persistence_required`/
/// `can_pick_up_loot`'s exact per-kind values — flagged for reconciliation against a
/// live vanilla-behavior cross-check, the identical caveat class M4-B01's own
/// `client_tracking_range_blocks` constants already carry — except `Villager`'s
/// `persistence_required: true`, which is a high-confidence vanilla fact (`Villager`
/// unconditionally overrides `isPersistenceRequired` to `true`, never subject to the
/// difficulty-scaled random roll `Mob`'s own default implementation uses).
pub const fn default_mob_marker(kind: EntityKind) -> Option<MobMarker> {
    match kind {
        EntityKind::Item => None,
        EntityKind::Zombie => Some(MobMarker {
            ai_system: AiSystemKind::GoalSelector,
            persistence_required: false,
            can_pick_up_loot: false,
        }),
        EntityKind::Villager => Some(MobMarker {
            ai_system: AiSystemKind::Brain,
            persistence_required: true,
            can_pick_up_loot: false,
        }),
        EntityKind::Cow => Some(MobMarker {
            ai_system: AiSystemKind::GoalSelector,
            persistence_required: false,
            can_pick_up_loot: false,
        }),
    }
}

/// One entity's crossing decision (the ECS-agnostic core's own output — Context, Part
/// 1.1's "Leave-tick"). `destination` is always a concrete `RegionId`, never an
/// unresolved `Address` (Context: "RegionOwnership... narrower usage contract").
pub struct MobCrossing {
    pub entity: Entity,
    pub rc_entity_id: RcEntityId,
    pub network_entity_id: i32,
    pub kind: EntityKind,
    pub destination: RegionId,
    pub source_chunk: ChunkKey,
    pub base: BaseEntity,
    pub living: Option<LivingEntity>,
    pub payload: EntityPayload,
}

/// Pure crossing-detection core (no `bevy_ecs::World`/`Query` reference — mirrors
/// `BlockWorldAccess`/`compute_tracking_delta`'s own "ECS-agnostic core, adapter at the
/// production call site" pattern). For each `(entity, rc_entity_id, network_entity_id,
/// kind, base, living, payload)` whose `base.pos`'s chunk resolves, via `ownership`, to
/// a region other than `ownership.local`, returns one `MobCrossing`. Entities whose
/// resolved region is *not* `Address::Region(_)` (Context's own narrower contract) are
/// skipped, never transferred, never panicked on — a documented, logged (by the
/// production adapter, not this pure function) gap for whichever future blueprint
/// extends `RegionOwnership` with real `Address::Chunk` resolution.
pub fn detect_mob_crossings(
    entities: impl IntoIterator<
        Item = (
            Entity,
            RcEntityId,
            i32,
            EntityKind,
            BaseEntity,
            Option<LivingEntity>,
            EntityPayload,
        ),
    >,
    dimension: DimensionId,
    ownership: &RegionOwnership,
) -> Vec<MobCrossing> {
    let mut crossings = Vec::new();
    for (entity, rc_entity_id, network_entity_id, kind, base, living, payload) in entities {
        let chunk = chunk_key_of(base.pos, dimension);
        let owner = (ownership.resolve)(chunk);
        if owner == ownership.local {
            continue;
        }
        let Address::Region(destination) = owner else {
            continue;
        };
        crossings.push(MobCrossing {
            entity,
            rc_entity_id,
            network_entity_id,
            kind,
            destination,
            source_chunk: chunk,
            base,
            living,
            payload,
        });
    }
    crossings
}

/// Reconstructs the kind-specific `EntityPayload` variant from one decoded
/// `ComponentBlob`, or `None` for a `ComponentKind` this function does not itself
/// resolve to a payload variant (`Base`/`Living`, handled by the caller instead).
fn decode_payload_component(kind: ComponentKind, bytes: &[u8]) -> Option<EntityPayload> {
    match kind {
        ComponentKind::Item => postcard::from_bytes::<ItemBundle>(bytes)
            .ok()
            .map(EntityPayload::Item),
        ComponentKind::Zombie => postcard::from_bytes::<ZombieBundle>(bytes)
            .ok()
            .map(EntityPayload::Zombie),
        ComponentKind::Villager => postcard::from_bytes::<VillagerBundle>(bytes)
            .ok()
            .map(EntityPayload::Villager),
        ComponentKind::Cow => postcard::from_bytes::<CowBundle>(bytes)
            .ok()
            .map(EntityPayload::Cow),
        ComponentKind::Base | ComponentKind::Living => None,
    }
}

/// `EntityArrivalDriver`-shaped (Context, Part 1.2/1.4). Decodes every mob-kind arrival
/// (via `try_decode_mob_snapshot`; a non-mob-kind or malformed entry is silently
/// skipped — a combined driver, Context, is responsible for handling every entry this
/// function itself does not) and spawns it fresh into `world`: `(EntityIdentity, base,
/// living-if-present, the kind-specific bundle, default_mob_marker(kind)-if-Some)`.
pub fn mob_arrival_driver(world: &mut World, arrivals: Vec<EntitySnapshot>) {
    for snapshot in arrivals {
        let Some(Ok((network_entity_id, decoded))) =
            try_decode_mob_snapshot(&snapshot.component_data)
        else {
            continue;
        };
        spawn_from_snapshot(world, snapshot.entity_id, network_entity_id, decoded);
    }
}

fn spawn_from_snapshot(
    world: &mut World,
    rc_entity_id: RcEntityId,
    network_entity_id: i32,
    payload: SnapshotPayload,
) {
    let kind = payload.entity_kind;
    let mut base: Option<BaseEntity> = None;
    let mut living: Option<LivingEntity> = None;
    let mut entity_payload: Option<EntityPayload> = None;

    for component in &payload.components {
        match component.kind {
            ComponentKind::Base => base = postcard::from_bytes(&component.bytes).ok(),
            ComponentKind::Living => living = postcard::from_bytes(&component.bytes).ok(),
            other => {
                if let Some(decoded) = decode_payload_component(other, &component.bytes) {
                    entity_payload = Some(decoded);
                }
            }
        }
    }

    let (Some(base), Some(entity_payload)) = (base, entity_payload) else {
        // Malformed/incomplete snapshot -- never panics (Context's own "never a panic"
        // discipline, restated for the arrival path).
        return;
    };

    let identity = EntityIdentity {
        rc_entity_id,
        network_entity_id,
        kind,
    };
    let mut entity_mut = world.spawn((identity, base, entity_payload));
    if let Some(living) = living {
        entity_mut.insert(living);
    }
    if let Some(marker) = default_mob_marker(kind) {
        entity_mut.insert(marker);
    }
}

#[cfg(feature = "server-systems")]
pub mod ecs {
    use bevy_ecs::prelude::*;
    use rc_scheduler::{DomainGroup, RcExecutorBuilder, RegionMessageOutbox, SystemFactory};

    use super::*;
    use crate::entity::physics::ecs::DimensionResource;

    /// Registers this module's mob crossing-detection system into
    /// `DomainGroup::EntityPhysicsIntegration` (Stage 6b). This registration is scoped to
    /// `TwoRegionWorld`'s own separate, isolated `RcExecutor` (never `HardcodedWorld`'s) —
    /// it neither co-registers with nor needs any call-order coordination against
    /// M4-B02/M4-B04/M4-B05's own systems, which land only in `HardcodedWorld`'s distinct
    /// executor instance (a separate `[CompiledGroup; 8]` array with its own independent
    /// `order_tag` sequence).
    pub fn register_mob_crossing_detection(builder: &mut RcExecutorBuilder) {
        builder.register_system(
            DomainGroup::EntityPhysicsIntegration,
            mob_crossing_detection_factory(),
            vec![],
        );
    }

    fn mob_crossing_detection_factory() -> SystemFactory {
        Box::new(|| {
            Box::new(IntoSystem::into_system(system_mob_crossing_detection))
                as Box<dyn System<In = (), Out = ()>>
        })
    }

    /// Reads `Query<(Entity, &EntityIdentity, &BaseEntity, Option<&LivingEntity>,
    /// &EntityPayload)>`, `Res<RegionOwnership>`, `Res<DimensionResource>`; on a detected
    /// crossing, issues `commands.entity(e).despawn()` and
    /// `ResMut<RegionMessageOutbox>::send`. This system never declares a mutable `Query`
    /// against any component its own despawn structurally writes, so `structural_writes`
    /// is empty — there is no conflicting live borrow to guard against.
    fn system_mob_crossing_detection(
        query: Query<(
            Entity,
            &EntityIdentity,
            &BaseEntity,
            Option<&LivingEntity>,
            &EntityPayload,
        )>,
        ownership: Res<RegionOwnership>,
        dimension: Res<DimensionResource>,
        mut outbox: ResMut<RegionMessageOutbox>,
        mut commands: Commands,
    ) {
        let entities: Vec<_> = query
            .iter()
            .map(|(entity, identity, base, living, payload)| {
                (
                    entity,
                    identity.rc_entity_id,
                    identity.network_entity_id,
                    identity.kind,
                    base.clone(),
                    living.cloned(),
                    payload.clone(),
                )
            })
            .collect();

        let crossings = detect_mob_crossings(entities, dimension.0, &ownership);
        for crossing in crossings {
            let snapshot = build_mob_entity_snapshot(
                crossing.rc_entity_id,
                crossing.source_chunk,
                crossing.network_entity_id,
                crossing.kind,
                &crossing.base,
                crossing.living.as_ref(),
                &crossing.payload,
            );
            outbox.send(
                Address::Region(crossing.destination),
                RegionMessage::RegionTransferRequest(Box::new(snapshot)),
            );
            commands.entity(crossing.entity).despawn();
        }
    }
}
