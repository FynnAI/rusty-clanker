use rc_core::{BlockPos, ChunkKey, RcEntityId};

/// ARCH-D11: a block/redstone update whose propagation crosses into a neighbor region.
/// Applied as the first sub-step of the destination's next Stage 4 (a later blueprint's
/// tick-driver responsibility — see this blueprint's Stage-1/Stage-10 contract note).
/// Embedded inline (no `Box`) in `RegionMessage` — together with `BorderUpdateKind`,
/// this keeps the whole variant comfortably inside ARCH-D28's 128-byte inline budget,
/// asserted by this blueprint's own acceptance tests.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BorderUpdateEvent {
    /// The neighbor-owned chunk this update targets.
    pub chunk: ChunkKey,
    /// Absolute block position of the update.
    pub pos: BlockPos,
    pub kind: BorderUpdateKind,
}

/// What kind of border-crossing update `BorderUpdateEvent` carries.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BorderUpdateKind {
    /// A block state changed; `new_state` is the raw global block-state numeric id
    /// (vanilla's own ID space). Stored as a raw `u32` rather than a typed
    /// `BlockStateId` because the block-state registry (`rc-registries`, WORLD-D3)
    /// does not exist yet at M0 — `rc-messaging` must not gain a dependency on it.
    BlockChanged { new_state: u32 },
    /// A neighbor-update notification only — no block changed at this position
    /// (e.g. a redstone signal-level recompute trigger, ARCH-D13's neighbor-changed
    /// fan-out).
    NeighborChanged,
}

/// ARCH-D10/D28: a full entity-component snapshot moving to a new owning region.
/// `component_data` is a placeholder (opaque bytes) until `05-game-mechanics.md`'s
/// concrete entity components exist (M4) — see this blueprint's Context section for
/// why that is safe to defer without breaking `RegionMessage`'s outer shape.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntitySnapshot {
    pub entity_id: RcEntityId,
    /// The chunk the entity was in immediately before the transfer, in the source
    /// region — carried for diagnostic/ordering purposes at the destination.
    pub source_chunk: ChunkKey,
    /// Opaque serialized component-bundle bytes. Replaced with concrete typed fields
    /// by the blueprint that first implements real entity-component snapshotting,
    /// without changing `RegionMessage::RegionTransferRequest`'s outer `Box<EntitySnapshot>`
    /// shape (ARCH-D25's extension-point framing, applied here by analogy).
    pub component_data: Vec<u8>,
}

/// The two native cross-region payload variants ARCH-D25 ships. `13-cluster-architecture.md`
/// may add cluster-only variants later without changing this envelope (ARCH-D25's stated
/// extension point) — not done by this blueprint.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RegionMessage {
    BorderUpdateEvent(BorderUpdateEvent),
    /// Boxed so a pooled allocator (`rc-transport-inproc`'s `SegQueue`-backed slot
    /// pool, ARCH-D28) can hand out a reused `Box<EntitySnapshot>` transparently —
    /// see this blueprint's "pooling seam" Context note.
    RegionTransferRequest(Box<EntitySnapshot>),
}
