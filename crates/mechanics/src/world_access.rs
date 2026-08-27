use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::Address;

/// The ECS-agnostic block-read/write boundary (Context: "mirroring `rc-physics`'s own
/// established shape"). A production adapter (`stage4::ecs`) and a test double both
/// implement this; the core algorithms in this crate depend on nothing else.
pub trait BlockWorldAccess {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId>;
    /// Returns `true` iff the stored value at `pos` actually changed. `pos` must already be
    /// known-local to the caller (callers route non-local writes through `RegionOwnership`
    /// *before* ever calling this — see `border.rs`); implementations may `debug_assert!`
    /// this but are not required to re-check ownership themselves.
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool;
    /// This region's single dimension (a region never spans dimensions, M0-B06's own
    /// `GridCell` invariant) — the missing piece `border.rs`'s `chunk_of(pos) =
    /// pos.chunk_key(world.dimension())` needs to turn a `BlockPos` into the `ChunkKey`
    /// `owner_of` expects.
    fn dimension(&self) -> DimensionId;
    fn owner_of(&self, chunk: ChunkKey) -> Address;
    fn local_identity(&self) -> Address;
}
