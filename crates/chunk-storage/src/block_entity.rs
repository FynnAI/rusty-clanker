use bevy_ecs::prelude::{Component, Entity};

/// WORLD-D6's generic, registry-agnostic persistence contract for one placed block entity.
/// `data` is the complete per-entity NBT compound (id/x/y/z plus every type-specific field) —
/// already exactly what `chunk_nbt::to_nbt` writes into the chunk's `block_entities` list.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockEntityRecord {
    pub pos: rc_core::BlockPos,
    pub id: String,
    pub data: rc_nbt::owned::NbtCompound,
}

#[derive(Debug, thiserror::Error)]
pub enum BlockEntityCodecError {
    #[error(transparent)]
    Nbt(#[from] rc_nbt::NbtError),
    #[error(transparent)]
    Schema(#[from] rc_nbt::schema::SchemaError),
}

/// The extension point WORLD-D6 reserves for `05-game-mechanics.md`. Implemented per
/// concrete block-entity type in `rc-mechanics`; never implemented in this crate (this
/// crate never interprets a block entity's field semantics).
pub trait BlockEntityCodec: Sized {
    fn to_record(&self, pos: rc_core::BlockPos) -> BlockEntityRecord;
    fn from_record(record: &BlockEntityRecord) -> Result<Self, BlockEntityCodecError>;
}

/// Every currently-live block entity's own resolved save record for this chunk, refreshed
/// once per tick by a `rc-mechanics`-owned Stage-7 system. Empty for a chunk with no block
/// entities. `Storage class: Table` (mirrors `BlockEntityIndex`).
#[derive(Component, Clone, Default)]
pub struct BlockEntitySaveRecords(pub Vec<BlockEntityRecord>);

/// Injected at `ChunkLifecycleManager::new` time: converts a freshly loaded/generated
/// chunk's on-disk `BlockEntityRecord`s into real ECS entities, pushed onto `chunk_entity`'s
/// own `BlockEntityIndex` in the given (on-disk load) order. A record whose `id` names an
/// unrecognized type is skipped, never panics.
pub trait BlockEntitySpawner: Send + Sync {
    fn spawn_loaded_block_entities(
        &self,
        world: &mut bevy_ecs::world::World,
        chunk_entity: bevy_ecs::prelude::Entity,
        records: &[BlockEntityRecord],
    );
}

/// The trivial default — used by this crate's own test suite and any caller with no real
/// mechanics-backed spawner available.
pub struct NoopBlockEntitySpawner;
impl BlockEntitySpawner for NoopBlockEntitySpawner {
    fn spawn_loaded_block_entities(
        &self,
        _: &mut bevy_ecs::world::World,
        _: bevy_ecs::prelude::Entity,
        _: &[BlockEntityRecord],
    ) {
    }
}

/// WORLD-D6's storage contract: a chunk's own placed-block-entity children, in
/// vanilla's own stable per-chunk load order (ARCH-D17). No `BlockEntityCodec`, no NBT
/// (de)serialization — `05-game-mechanics.md`'s job (Context). Storage class: `Table`.
#[derive(Component, Clone, Default)]
pub struct BlockEntityIndex {
    entities: Vec<Entity>,
}

impl BlockEntityIndex {
    pub fn new() -> Self {
        Self::default()
    }
    /// Appends `entity` at the end of the load order (the caller is responsible for
    /// vanilla-matching order at the point of insertion — this type only preserves
    /// whatever order it is given).
    pub fn push(&mut self, entity: Entity) {
        self.entities.push(entity);
    }
    /// Removes the first occurrence of `entity`, if present, preserving the relative
    /// order of every remaining entry. Returns `true` iff an entry was removed.
    pub fn remove(&mut self, entity: Entity) -> bool {
        if let Some(pos) = self.entities.iter().position(|&e| e == entity) {
            self.entities.remove(pos);
            true
        } else {
            false
        }
    }
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }
}
