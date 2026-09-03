//! Entity NBT persistence — `entities/` region files, reusing `ChunkStorageBackend`
//! unmodified (M4-B01, WORLD-D29). This blueprint is `RegionFileKind::Entities`'s
//! first real payload producer/consumer — no change to `rc-chunk-storage` itself.
//! `write_entities_chunk`/`read_entities_chunk` build/parse the `{DataVersion,
//! Position, Entities}` root via `rc-nbt`'s `owned`/`borrow` API directly (mirroring
//! `play::persistence`'s own `level_dat`-style "pure, storage-agnostic producer/
//! consumer of exactly the byte shape `ChunkStorageBackend` expects" design), calling
//! `rc_mechanics::entity::nbt`'s per-kind `EntityRecord::to_nbt`/`EntityRecord::from_nbt`
//! for each entry's own inner compound.

use rc_chunk_storage::{ChunkStorageBackend, RegionFileKind};
use rc_core::{ChunkKey, DimensionId};
use rc_mechanics::entity::{EntityKind, EntityRecord};
use rc_nbt::{borrow, owned};

/// WORLD-D16, unmodified, reused verbatim.
const DATA_VERSION: i32 = 4903;

/// Builds one chunk's complete `entities/` payload (WORLD-D29's `{DataVersion,
/// Position, Entities}` root) from every currently-live entity in that chunk, and
/// hands the **raw, uncompressed** NBT bytes to `backend.write_chunk`
/// (`AnvilDiskBackend` compresses internally, M2-B03 — this function never GZips or
/// Zlib-compresses anything itself, unlike `level.dat`'s own GZip-pre-compressed
/// convention, M2-B06).
pub fn write_entities_chunk(
    backend: &dyn ChunkStorageBackend,
    dim: DimensionId,
    chunk: ChunkKey,
    entities: &[(EntityKind, EntityRecord)],
    epoch: Option<u64>,
) -> Result<(), rc_chunk_storage::StorageError> {
    let mut root = owned::NbtCompound::new();
    root.insert("DataVersion", DATA_VERSION);
    root.insert("Position", owned::NbtTag::IntArray(vec![chunk.x, chunk.z]));
    root.insert(
        "Entities",
        owned::NbtTag::List(owned::NbtList::Compound(
            entities
                .iter()
                .map(|(kind, record)| record.to_nbt(*kind))
                .collect(),
        )),
    );

    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", root));
    backend.write_chunk(
        dim,
        RegionFileKind::Entities,
        chunk.x,
        chunk.z,
        &bytes,
        epoch,
    )
}

/// Inverse: reads and decodes `RegionFileKind::Entities` for `chunk`. `Ok(None)` if
/// no such chunk has ever been written (matches `ChunkStorageBackend::read_chunk`'s
/// own `Option`-returning contract). Each returned tuple's `EntityKind` comes from
/// matching the compound's own `id` string against `EntityKind::namespaced_id`'s four
/// known values; an unrecognized `id` is `Err(EntityPersistenceError::UnknownKind)`,
/// never silently skipped.
pub fn read_entities_chunk(
    backend: &dyn ChunkStorageBackend,
    dim: DimensionId,
    chunk: ChunkKey,
) -> Result<Option<Vec<(EntityKind, EntityRecord)>>, EntityPersistenceError> {
    let Some(bytes) = backend.read_chunk(dim, RegionFileKind::Entities, chunk.x, chunk.z, None)?
    else {
        return Ok(None);
    };

    let nbt = rc_nbt::read_borrowed_strict(&bytes)?;
    let base = match nbt {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => return Ok(Some(Vec::new())),
    };
    let root = base.as_compound();
    let path = rc_nbt::NbtPath::root();

    use rc_nbt::schema::NbtCompoundExt;
    let entities_list = root.require_list(&path, "Entities")?;
    let entities_path = path.field("Entities");
    let compounds = entities_list
        .compounds()
        .ok_or_else(|| rc_nbt::SchemaError::WrongType {
            path: entities_path.clone(),
            field: "Entities",
            expected: "List<Compound>",
            actual_id: entities_list.id(),
        })?;

    let mut out = Vec::with_capacity(compounds.len());
    for (i, entry) in compounds.into_iter().enumerate() {
        let entry_path = entities_path.index(i);
        let id_str = entry.require_string(&entry_path, "id")?;
        let kind = entity_kind_from_namespaced_id(&id_str.to_str())
            .ok_or_else(|| EntityPersistenceError::UnknownKind(id_str.to_str().into_owned()))?;
        let record = EntityRecord::from_nbt(&entry, &entry_path, kind)?;
        out.push((kind, record));
    }

    Ok(Some(out))
}

fn entity_kind_from_namespaced_id(id: &str) -> Option<EntityKind> {
    [
        EntityKind::Item,
        EntityKind::Zombie,
        EntityKind::Villager,
        EntityKind::Cow,
    ]
    .into_iter()
    .find(|kind| kind.namespaced_id() == id)
}

#[derive(Debug, thiserror::Error)]
pub enum EntityPersistenceError {
    #[error(transparent)]
    Storage(#[from] rc_chunk_storage::StorageError),
    #[error(transparent)]
    Nbt(#[from] rc_nbt::NbtError),
    #[error(transparent)]
    Schema(#[from] rc_nbt::SchemaError),
    #[error("entities/ record has unrecognized id `{0}`")]
    UnknownKind(String),
}
