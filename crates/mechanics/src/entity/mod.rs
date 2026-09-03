//! Entity component bundles, identity, the entity-type registry seam, the metadata
//! wire protocol, NBT persistence, tracking, and `EntitySnapshot` serialization
//! (MECH-D29/D30, ARCH-D10/D15/D24/D25/D28). Zero AI/pathfinding/combat/spawning
//! content — every system slot this module's `Stage`/`DomainGroup` extension opens
//! (rc-scheduler, `server-systems` feature only) stays unregistered.

pub mod base;
pub mod ids;
pub mod kinds;
pub mod living;
pub mod loot;
pub mod metadata;
pub mod nbt;
pub mod physics;
pub mod pickup;
pub mod snapshot;
pub mod tracking;

pub use base::BaseEntity;
pub use ids::{EntityUuid, NetworkEntityIdAllocator};
pub use kinds::{
    AiSystemKind, CowBundle, EntityKind, EntityPayload, ItemBundle, ItemStackRecord, MobMarker,
    VillagerBundle, ZombieBundle,
};
pub use living::LivingEntity;
pub use loot::{
    CountProvider, LootEntry, LootPool, LootRandom, LootTable, RandomSequenceStore, RollProvider,
    roll_loot_table, tier1_loot_table,
};
pub use metadata::{EntityMetadataFields, MetadataValue, Pose};
pub use nbt::{EntityNbtFields, EntityRecord, FromNbtField, ToNbtField};
pub use physics::{
    ITEM_AIR_DRAG, ITEM_GRAVITY, ITEM_HALF_WIDTH, ITEM_HEIGHT, ITEM_STEP_HEIGHT, ItemMotionState,
    PendingEnvironmentalDamage, step_item_entity_tick,
};
pub use pickup::PickedUpItems;
pub use snapshot::{
    ComponentBlob, ComponentKind, ENTITY_SNAPSHOT_FORMAT_VERSION, SnapshotError, SnapshotPayload,
    deserialize_entity_snapshot, serialize_entity_snapshot,
};
pub use tracking::{TrackingDelta, compute_tracking_delta};
