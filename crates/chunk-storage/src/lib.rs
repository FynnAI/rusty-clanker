//! `rc-chunk-storage` — chunk/section/palette data structures, the on-disk region-file
//! format, save scheduling, and a storage-backend abstraction (`03-world-chunks-
//! persistence.md`). This blueprint (M2-B01) implements the in-memory representation
//! only: `PalettedContainer<T>`, the seven WORLD-D1 chunk components, and their get/set/
//! dirty-tracking API. NBT (de)serialization, Anvil region files, and the storage-backend
//! trait are `M2-B02`'s scope.

mod bits;
mod block_entity;
mod chunk_key;
mod column;
mod heightmap;
mod light;
mod palette;
mod persistence;
mod registry_id;
mod status;

pub use bits::{ceil_log2, pack_bits, read_slot, unpack_bits, write_slot};
pub use block_entity::BlockEntityIndex;
pub use chunk_key::ChunkKeyTag;
pub use column::{
    biome_index, block_index, local_biome_quart_y, local_block_y, section_index_for_y,
    BiomeColumn, BlockStateColumn, SECTION_BIOME_CELLS, SECTION_BLOCKS, SECTION_COUNT,
    WORLD_HEIGHT, WORLD_MIN_Y,
};
pub use heightmap::{BlockOpacity, HeightmapKind, HeightmapSet};
pub use light::{LightColumn, LightSection, LIGHT_SECTION_COUNT};
pub use palette::{Palette, PalettedContainer};
pub use persistence::ChunkPersistenceState;
pub use registry_id::{BiomeId, BlockStateId, PaletteThresholds, RegistryId};
pub use status::{ChunkGenStatus, ChunkStatus};
