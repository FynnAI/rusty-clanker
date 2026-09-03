use rc_core::{BlockPos, ChunkKey, RcEntityId};

/// `serde`'s own derive only implements `Serialize`/`Deserialize` for fixed-size
/// arrays up to a small bound (well under 128) -- this hand-written `#[serde(with =
/// "...")]` module is `LightBorderUpdate`'s own workaround, going through
/// `Option<Vec<u8>>` (already fully `serde`-supported) rather than adding a new
/// external dependency (`serde_big_array` or similar) this crate's own `xtask
/// lint-deps` Rule 3 (`{rc-core, serde, thiserror}` only) would otherwise forbid.
mod fixed_face_array {
    use serde::{Deserialize, Serialize};

    pub fn serialize<S: serde::Serializer>(
        value: &Option<[u8; 128]>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let as_vec: Option<Vec<u8>> = value.map(|arr| arr.to_vec());
        as_vec.serialize(serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<[u8; 128]>, D::Error> {
        let as_vec: Option<Vec<u8>> = Option::<Vec<u8>>::deserialize(deserializer)?;
        match as_vec {
            None => Ok(None),
            Some(bytes) => {
                let len = bytes.len();
                let arr: [u8; 128] = bytes
                    .try_into()
                    .map_err(|_| serde::de::Error::invalid_length(len, &"128"))?;
                Ok(Some(arr))
            }
        }
    }
}

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

/// WORLD-D10: one `LightSection`'s single-face nibble slice, sent once its sending
/// region's own Stage-8 BSP rounds converge (`stage8.rs` step 10) and that face
/// changed since the last send. `edge_face` matches
/// `rc_mechanics::direction::Direction`'s own declaration order (West=0, East=1,
/// North=2, South=3, Down=4, Up=5) as a plain `u8` — `rc-messaging` cannot depend on
/// `rc-mechanics` (WS-D3 Rule 3: `rc-messaging`'s exact dependency set stays
/// `{rc-core, serde, thiserror}`), the identical resolution `BorderUpdateKind::
/// BlockChanged`'s own raw-`u32` `new_state` field already established for the same
/// reason (M0-B02). Only `West`/`East`/`North`/`South` (`0..=3`) are ever
/// constructed by this blueprint's own emitting code — light never crosses a
/// *region* boundary vertically, since one region owns a chunk column's full height
/// (ARCH-D5/D6's own 2D, horizontal-only grid-cell partitioning).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LightBorderUpdate {
    /// The **receiving** region's own chunk that needs to seed round 0.
    pub chunk: ChunkKey,
    /// `LightColumn`'s own `0..26` section index (WORLD-D8's `+2`-padded indexing,
    /// unmodified — this blueprint's own `light_section_index_for_y` is the exact
    /// function that produces this value on the sending side).
    pub section_index: u8,
    pub edge_face: u8,
    /// Nibble-packed 16×16 face slice (256 4-bit entries, 128 bytes). `None`
    /// matches `LightNibbles::Uninitialized` (WORLD-D8) — this specific
    /// section/channel had no tracked data on the sending side. `Some` covers
    /// both `LightNibbles::Data` (extracted as-is) and `LightNibbles::Filled(v)`
    /// (materialized into a uniformly-packed `[u8; 128]` on the sending side) —
    /// this message carries only materialized bytes, never a third variant of its
    /// own, since a one-tick-latency cross-region seed has no laziness requirement
    /// to preserve (unlike the client-facing wire payload).
    #[serde(with = "fixed_face_array")]
    pub sky: Option<[u8; 128]>,
    #[serde(with = "fixed_face_array")]
    pub block: Option<[u8; 128]>,
}

/// The three native cross-region payload variants ARCH-D25/WORLD-D10 ship.
/// `13-cluster-architecture.md` may add cluster-only variants later without
/// changing this envelope (ARCH-D25's stated extension point) — not done by this
/// blueprint.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RegionMessage {
    BorderUpdateEvent(BorderUpdateEvent),
    /// Boxed so a pooled allocator (`rc-transport-inproc`'s `SegQueue`-backed slot
    /// pool, ARCH-D28) can hand out a reused `Box<EntitySnapshot>` transparently —
    /// see this blueprint's "pooling seam" Context note.
    RegionTransferRequest(Box<EntitySnapshot>),
    /// Boxed to keep `RegionMessage`'s own overall size within the already-asserted
    /// ARCH-D28 ≤128-byte inline budget (`size_of::<RegionMessage>() <= 128`,
    /// M0-B02's own committed regression test, unmodified by this blueprint) —
    /// `LightBorderUpdate` is itself roughly 260 bytes unboxed, comfortably past
    /// that budget on its own.
    LightBorderUpdate(Box<LightBorderUpdate>),
}
