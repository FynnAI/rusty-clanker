//! The four tier-2 entity kinds this blueprint ships (Context: "Tier-2 entity kind
//! list"): `EntityKind`, the `Mob`-rung `MobMarker`/`AiSystemKind` pair, and each
//! kind's own kind-specific bundle + `EntityPayload`.

use rc_registries::generated_v776::registries::RegistryEntryId;
use rc_registries::generated_v776::registries::entity_type;

/// The four tier-2 kinds this blueprint ships (Context: "Tier-2 entity kind list").
/// Extending this enum, its `namespaced_id`/`registry_id`/`client_tracking_range_blocks`
/// match arms, and adding one new `*Bundle` struct is the complete recipe a future
/// blueprint follows to add a fifth kind.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EntityKind {
    Item,
    Zombie,
    Villager,
    Cow,
}

impl EntityKind {
    /// Vanilla's own namespaced id string — the `entities/` region-file `id` field
    /// (Context, "Entity NBT persistence"). The one place this blueprint uses a real
    /// registry *name* rather than its numeric id.
    pub const fn namespaced_id(self) -> &'static str {
        match self {
            EntityKind::Item => "minecraft:item",
            EntityKind::Zombie => "minecraft:zombie",
            EntityKind::Villager => "minecraft:villager",
            EntityKind::Cow => "minecraft:cow",
        }
    }

    /// The wire-protocol numeric id (`Spawn Entity`'s `entity_type` field), read
    /// directly off the real, already-generated `rc_registries::generated_v776::
    /// registries::entity_type` table (`xtask codegen` output, already checked in —
    /// no hand-typed reconciliation caveat applies to these four constants).
    pub const fn registry_id(self) -> RegistryEntryId {
        match self {
            EntityKind::Item => entity_type::ITEM,
            EntityKind::Zombie => entity_type::ZOMBIE,
            EntityKind::Villager => entity_type::VILLAGER,
            EntityKind::Cow => entity_type::COW,
        }
    }

    /// `EntityType.clientTrackingRange`, in blocks (chunks x 16) — Context's own
    /// hand-typed, TEST-D57-verified per-kind values (`Villager`/`Cow` corrected to
    /// `10` chunks, `Item`/`Zombie` at the `6`/`8`-chunk values Context names).
    pub const fn client_tracking_range_blocks(self) -> f64 {
        match self {
            EntityKind::Item => 6.0 * 16.0,
            EntityKind::Zombie => 8.0 * 16.0,
            EntityKind::Villager => 10.0 * 16.0,
            EntityKind::Cow => 10.0 * 16.0,
        }
    }

    /// Whether this kind has a `LivingEntity` rung (`false` only for `Item`).
    pub const fn is_living(self) -> bool {
        !matches!(self, EntityKind::Item)
    }

    /// Which of MECH-D31's two AI systems a `Mob`-rung kind uses, or `None` for `Item`
    /// (no `Mob` rung at all). Vanilla never persists this choice — it is a static
    /// property of the vanilla type, not per-instance state — so this is the *sole*
    /// source `MobMarker.ai_system` is ever populated from: never `Default::default()`,
    /// never read from a compound.
    pub const fn ai_system(self) -> Option<AiSystemKind> {
        match self {
            EntityKind::Item => None,
            EntityKind::Zombie | EntityKind::Cow => Some(AiSystemKind::GoalSelector),
            EntityKind::Villager => Some(AiSystemKind::Brain),
        }
    }
}

/// Which of MECH-D31's two AI systems a `Mob`-rung kind uses. Not consulted by any
/// system this blueprint ships — a marker for a future AI blueprint to read. Carries no
/// `Default` impl and needs none: every `MobMarker` this blueprint ever constructs
/// receives an explicit `ai_system` value from `EntityKind::ai_system` (never from
/// `Default::default()` and never read from a compound — vanilla does not persist it).
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AiSystemKind {
    GoalSelector,
    Brain,
}

/// `Mob`'s own rung (MECH-D29 diagram: `PersistenceRequired`, `CanPickUpLoot`) plus the
/// `AiSystemKind` marker. In vanilla (`Mob.java`'s own `addAdditionalSaveData`/
/// `readAdditionalSaveData`), `PersistenceRequired`/`CanPickUpLoot` are **not**
/// internal-only bookkeeping — both are independently, unconditionally written as NBT
/// `Boolean`s and read back defaulting to `false` when absent (`getBooleanOr(key,
/// false)`); neither is synced via entity metadata. This blueprint's own `MobMarker`
/// round-trips both booleans through `EntityRecord` directly (`nbt.rs`'s own `to_nbt`/
/// `from_nbt`), not through `#[derive(EntityNbtFields)]`, since `ai_system` is never
/// itself an NBT field and so cannot round-trip through that derive's own all-fields-
/// or-`Default` rule. This struct is an ECS component every `Mob`-rung tier-2 kind's
/// `EntityRecord` now carries (`mob: Some(..)`), not an `EntityNbtFields`/
/// `EntityMetadataFields` implementer itself.
#[derive(
    Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, bevy_ecs::prelude::Component,
)]
pub struct MobMarker {
    pub ai_system: AiSystemKind,
    pub persistence_required: bool,
    pub can_pick_up_loot: bool,
}

/// The three-field item-stack record (Context: "Item-kind... NBT" — `item_id` stored
/// as `Int`, a cited, bounded deviation from vanilla's own `String` id, not `String`).
/// `PartialEq`, not `Eq` — `components: Option<rc_nbt::owned::NbtCompound>` cannot
/// derive `Eq` (`simdnbt` 0.10.0's own `owned::NbtCompound`/`NbtTag` derive only
/// `PartialEq`, since a compound can transitively hold an `f32`/`f64` leaf) — a cited
/// correction to this blueprint's own Deliverables text, which named `Eq` too.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ItemStackRecord {
    #[serde(with = "crate::entity::metadata::registry_entry_id_serde")]
    pub item_id: RegistryEntryId,
    pub count: u8,
    #[serde(with = "crate::entity::nbt::nbt_compound_serde")]
    pub components: Option<rc_nbt::owned::NbtCompound>,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    rc_entity_macros::EntityNbtFields,
    rc_entity_macros::EntityMetadataFields,
    bevy_ecs::prelude::Component,
)]
pub struct ItemBundle {
    #[nbt(name = "Item")]
    #[net_metadata(index = 8, kind = "Slot")]
    pub item: ItemStackRecord,
    #[nbt(name = "PickupDelay")]
    pub pickup_delay_ticks: i16,
    #[nbt(name = "Age")]
    pub age_ticks: i16,
}

/// No kind-specific NBT/metadata at this milestone's scope (Context) — a marker-only
/// bundle a future AI blueprint attaches real `Goal`/behavior state alongside. Vanilla's
/// real `Zombie`-rung indices (16-18: `DATA_BABY_ID`, `DATA_SPECIAL_TYPE_ID`,
/// `DATA_DROWNED_CONVERSION_ID`, Context: "Per-kind synced-data index table") stay
/// reserved, never sent — this struct declares none of them.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, bevy_ecs::prelude::Component,
)]
pub struct ZombieBundle;

#[derive(
    Clone,
    Debug,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    rc_entity_macros::EntityNbtFields,
    rc_entity_macros::EntityMetadataFields,
    bevy_ecs::prelude::Component,
)]
pub struct VillagerBundle {
    /// Index 19, not 16 — vanilla-exact per `Villager`'s own full class chain
    /// (`Entity`→`LivingEntity`→`Mob`(15)→`PathfinderMob`(none)→`AgeableMob`(16,17)→
    /// `AbstractVillager`(18)→`Villager`(19), Context: "Per-kind synced-data index
    /// table"), corrected from this blueprint's own prior internal, never-proven guess
    /// of 16. Vanilla's own `DATA_VILLAGER_DATA_FINALIZED` (index 20) stays reserved,
    /// never sent — this struct does not model it.
    #[nbt(name = "VillagerData")]
    #[net_metadata(index = 19, kind = "VillagerData")]
    pub villager_data: crate::entity::metadata::VillagerData,
}

/// No kind-specific NBT/metadata at this milestone's scope (Context). Vanilla's real
/// `Cow`-rung indices (18-19: `DATA_VARIANT_ID`, `DATA_SOUND_VARIANT_ID`, Context:
/// "Per-kind synced-data index table") stay reserved, never sent — this struct declares
/// none of them.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, bevy_ecs::prelude::Component,
)]
pub struct CowBundle;

/// The closed set of kind-specific payloads `EntityRecord`/`snapshot.rs` dispatch on.
#[derive(
    Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, bevy_ecs::prelude::Component,
)]
pub enum EntityPayload {
    Item(ItemBundle),
    Zombie(ZombieBundle),
    Villager(VillagerBundle),
    Cow(CowBundle),
}

impl EntityPayload {
    /// M4-B02 addition (`docs/findings-for-planning.md`): the `EntityKind` this payload's own
    /// variant carries — needed by any caller (`rusty-clanker-server`'s own tracking-delta
    /// integration) that only has a live `&EntityPayload` in hand (e.g. from a real `bevy_ecs`
    /// `Query`) and needs the matching `EntityKind` back, without threading a second,
    /// separately-stored value alongside every spawned entity.
    pub const fn kind(&self) -> EntityKind {
        match self {
            EntityPayload::Item(_) => EntityKind::Item,
            EntityPayload::Zombie(_) => EntityKind::Zombie,
            EntityPayload::Villager(_) => EntityKind::Villager,
            EntityPayload::Cow(_) => EntityKind::Cow,
        }
    }
}
