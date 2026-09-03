//! The fixed 12-stage tick pipeline (ARCH-D12, widened by M4-B01's own Stage 6a/6b
//! split, ARCH-D15) and the eight ARCH-D8 domain groups. This blueprint's own concrete
//! group -> stage mapping is Context's "The five domain groups and their stage mapping"
//! table, extended first by M3-B06 (5 -> 7) and now by M4-B01 (7 -> 8).

/// The fixed 12-stage tick pipeline (ARCH-D12), identical for every region, every
/// 50ms tick. Numeric values match the pipeline table 1:1 so `Stage as u8` sorts in
/// pipeline order (used by Stage 11's `(stage, order_tag)` apply-order key).
///
/// M4-B01: the old single `EntityAiPhysics = 6` discriminant is replaced by two new
/// discriminants, `EntityAiSelection = 6` and `EntityPhysicsIntegration = 7`
/// (ARCH-D15's Stage 6a/6b split — Context, "Stage-6a/6b system registration model"),
/// and every stage after the old `BlockEntityTick = 7` shifts up by one. A real, cited,
/// necessary breaking change — see `DomainGroup`'s own doc comment below.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Stage {
    PreTickSync = 1,
    WorldUpdate = 2,
    NetworkInboundApply = 3,
    ScheduledBlockTick = 4,
    RandomBlockTick = 5,
    /// New (M4-B01): ARCH-D15's Stage 6a. Dispatched read-only — see `DomainGroup::
    /// EntityAiSelection`'s own doc comment.
    EntityAiSelection = 6,
    /// New (M4-B01): ARCH-D15's Stage 6b.
    EntityPhysicsIntegration = 7,
    /// Renumbered from `= 7` (M4-B01) — every field/method elsewhere in `rc-scheduler`
    /// that maps `DomainGroup::BlockEntity` to this variant is updated identically;
    /// no other crate stores this discriminant's raw numeric value anywhere.
    BlockEntityTick = 8,
    Lighting = 9,
    ChunkSnapshot = 10,
    PostTickFlush = 11,
    NetworkOutboundEncode = 12,
}

/// The eight ARCH-D8 domain groups. `stage()` is this blueprint's own concrete,
/// cited stage mapping (Context: "The five domain groups and their stage mapping",
/// extended by M3-B06 to seven, and by M4-B01 to eight).
///
/// M4-B01: `AiPhysics` is **replaced** (not merely renamed — nothing in the merged
/// codebase registers a real production system into it, though several already-merged
/// test files construct instrumented no-op systems in it and are mechanically renamed
/// alongside this change, restated in `docs/findings-for-planning.md`) by
/// `EntityAiSelection`/`EntityPhysicsIntegration`, mapping onto the two new `Stage`
/// values one-to-one.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DomainGroup {
    BlockRedstone,
    /// New (M4-B01), replaces the old `AiPhysics`. Dispatched via the identical
    /// read-only code path `NetCodec`/Stage 12 already uses — MECH-D32's "never
    /// mutates World state" rule enforced structurally.
    EntityAiSelection,
    /// New (M4-B01). Ordinary conflict-graph-batched, deferred dispatch (`AiPhysics`'s
    /// old dispatch style, unchanged) — ARCH-D15's own second-phase, entity-id-ordered
    /// reconciliation pass is *not* provided by this dispatch; it is a deliberate,
    /// cited, bounded deferral to whichever future blueprint first ships real
    /// entity-entity movement contention (Context, "ARCH-D15's own second phase").
    EntityPhysicsIntegration,
    Lighting,
    ChunkSerialize,
    NetCodec,
    /// M3-B06: Stage 5, Random Block Tick (ARCH-D14). "Conflict-graph-batched,
    /// deferred" dispatch, identical in kind to `EntityPhysicsIntegration`/`Lighting`/
    /// `ChunkSerialize` — see that blueprint's own Context for why exactly one system
    /// is ever registered here.
    RandomTick,
    /// M3-B06: Stage 8 (renumbered from 7 by M4-B01), Block Entity Tick (ARCH-D17).
    /// Same dispatch kind as `RandomTick` above.
    BlockEntity,
}

impl DomainGroup {
    pub const ALL: [DomainGroup; 8] = [
        DomainGroup::BlockRedstone,
        DomainGroup::EntityAiSelection,
        DomainGroup::EntityPhysicsIntegration,
        DomainGroup::Lighting,
        DomainGroup::ChunkSerialize,
        DomainGroup::NetCodec,
        DomainGroup::RandomTick,
        DomainGroup::BlockEntity,
    ];

    /// `EntityAiSelection => Stage::EntityAiSelection`, `EntityPhysicsIntegration =>
    /// Stage::EntityPhysicsIntegration`; every other arm's mapping is unchanged in
    /// effect (`BlockEntity => Stage::BlockEntityTick`, now discriminant `8`, still
    /// the same `Stage` variant name it already was).
    pub const fn stage(self) -> Stage {
        match self {
            DomainGroup::BlockRedstone => Stage::ScheduledBlockTick,
            DomainGroup::EntityAiSelection => Stage::EntityAiSelection,
            DomainGroup::EntityPhysicsIntegration => Stage::EntityPhysicsIntegration,
            DomainGroup::Lighting => Stage::Lighting,
            DomainGroup::ChunkSerialize => Stage::ChunkSnapshot,
            DomainGroup::NetCodec => Stage::NetworkOutboundEncode,
            DomainGroup::RandomTick => Stage::RandomBlockTick,
            DomainGroup::BlockEntity => Stage::BlockEntityTick,
        }
    }

    /// 0-based index into `RcExecutor`'s internal 8-element group array; stable,
    /// matches `Self::ALL`'s declaration order (`BlockRedstone=0,
    /// EntityAiSelection=1, EntityPhysicsIntegration=2, Lighting=3, ChunkSerialize=4,
    /// NetCodec=5, RandomTick=6, BlockEntity=7`). **Not** the same number as `Stage`'s
    /// own discriminant (a different axis, a pipeline-stage ordinal).
    pub const fn index(self) -> usize {
        match self {
            DomainGroup::BlockRedstone => 0,
            DomainGroup::EntityAiSelection => 1,
            DomainGroup::EntityPhysicsIntegration => 2,
            DomainGroup::Lighting => 3,
            DomainGroup::ChunkSerialize => 4,
            DomainGroup::NetCodec => 5,
            DomainGroup::RandomTick => 6,
            DomainGroup::BlockEntity => 7,
        }
    }
}
