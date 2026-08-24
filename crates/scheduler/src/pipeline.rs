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

/// The five ARCH-D8 domain groups. `stage()` is this blueprint's own concrete,
/// cited stage mapping (Context: "The five domain groups and their stage mapping").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DomainGroup {
    BlockRedstone,
    AiPhysics,
    Lighting,
    ChunkSerialize,
    NetCodec,
}

impl DomainGroup {
    pub const ALL: [DomainGroup; 5] = [
        DomainGroup::BlockRedstone,
        DomainGroup::AiPhysics,
        DomainGroup::Lighting,
        DomainGroup::ChunkSerialize,
        DomainGroup::NetCodec,
    ];

    /// The Deliverables' cited stage mapping table.
    pub const fn stage(self) -> Stage {
        todo!()
    }

    /// 0-based index into `RcExecutor`'s internal 5-element group array; stable,
    /// matches `Self::ALL`'s declaration order.
    pub const fn index(self) -> usize {
        todo!()
    }
}
