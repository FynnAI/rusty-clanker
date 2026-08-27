//! The fixed 11-stage tick pipeline (ARCH-D12) and the five ARCH-D8 domain
//! groups. This blueprint's own concrete group -> stage mapping is Context's
//! "The five domain groups and their stage mapping" table.

/// The fixed 11-stage tick pipeline (ARCH-D12), identical for every region, every
/// 50ms tick. Numeric values match the pipeline table 1:1 so `Stage as u8` sorts in
/// pipeline order (used by Stage 10's `(stage, order_tag)` apply-order key).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Stage {
    PreTickSync = 1,
    WorldUpdate = 2,
    NetworkInboundApply = 3,
    ScheduledBlockTick = 4,
    RandomBlockTick = 5,
    EntityAiPhysics = 6,
    BlockEntityTick = 7,
    Lighting = 8,
    ChunkSnapshot = 9,
    PostTickFlush = 10,
    NetworkOutboundEncode = 11,
}

/// The seven ARCH-D8 domain groups. `stage()` is this blueprint's own concrete,
/// cited stage mapping (Context: "The five domain groups and their stage mapping",
/// extended by M3-B06 to seven).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DomainGroup {
    BlockRedstone,
    AiPhysics,
    Lighting,
    ChunkSerialize,
    NetCodec,
    /// M3-B06: Stage 5, Random Block Tick (ARCH-D14). "Conflict-graph-batched,
    /// deferred" dispatch, identical in kind to `AiPhysics`/`Lighting`/`ChunkSerialize`
    /// — see that blueprint's own Context for why exactly one system is ever
    /// registered here.
    RandomTick,
    /// M3-B06: Stage 7, Block Entity Tick (ARCH-D17). Same dispatch kind as
    /// `RandomTick` above.
    BlockEntity,
}

impl DomainGroup {
    pub const ALL: [DomainGroup; 7] = [
        DomainGroup::BlockRedstone,
        DomainGroup::AiPhysics,
        DomainGroup::Lighting,
        DomainGroup::ChunkSerialize,
        DomainGroup::NetCodec,
        DomainGroup::RandomTick,
        DomainGroup::BlockEntity,
    ];

    /// The Deliverables' cited stage mapping table.
    pub const fn stage(self) -> Stage {
        match self {
            DomainGroup::BlockRedstone => Stage::ScheduledBlockTick,
            DomainGroup::AiPhysics => Stage::EntityAiPhysics,
            DomainGroup::Lighting => Stage::Lighting,
            DomainGroup::ChunkSerialize => Stage::ChunkSnapshot,
            DomainGroup::NetCodec => Stage::NetworkOutboundEncode,
            DomainGroup::RandomTick => Stage::RandomBlockTick,
            DomainGroup::BlockEntity => Stage::BlockEntityTick,
        }
    }

    /// 0-based index into `RcExecutor`'s internal 7-element group array; stable,
    /// matches `Self::ALL`'s declaration order. **Not** the same number as
    /// `Stage`'s own discriminant (a different axis, a pipeline-stage ordinal).
    pub const fn index(self) -> usize {
        match self {
            DomainGroup::BlockRedstone => 0,
            DomainGroup::AiPhysics => 1,
            DomainGroup::Lighting => 2,
            DomainGroup::ChunkSerialize => 3,
            DomainGroup::NetCodec => 4,
            DomainGroup::RandomTick => 5,
            DomainGroup::BlockEntity => 6,
        }
    }
}
