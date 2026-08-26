//! `rc-nbt` — the engine's one boundary onto `simdnbt` 0.10.0 for vanilla-schema NBT
//! (WORLD-D11): typed read/write entry points (`io`), a byte-level/schema-level error
//! taxonomy (`error`), and a hand-written schema-conversion helper layer (`schema`)
//! future blueprints build vanilla `level.dat`/player/chunk/entity schemas on top of.
//! No vanilla schema is implemented in this crate.

mod error;
mod io;
pub mod schema;

/// Re-exported unmodified — this crate's read/write entry points return these types
/// directly rather than wrapping them a second time (WORLD-D11: "thin wrapper").
pub use simdnbt::{Mutf8Str, Mutf8String};

/// Zero-copy, lifetime-tied tree types — the default read path (see Context's
/// "Zero-copy read-path policy").
pub mod borrow {
    pub use simdnbt::borrow::{BaseNbt, Nbt, NbtCompound, NbtCompoundIter, NbtList, NbtTag};
}

/// Heap-owned tree types — used for `level.dat`/player-data (always GZip, see
/// Context's "Compression stance") and anywhere a value must outlive its source buffer.
pub mod owned {
    pub use simdnbt::owned::{BaseNbt, Nbt, NbtCompound, NbtList, NbtTag};
}

pub use error::NbtError;
pub use io::{
    read_borrowed, read_borrowed_strict, read_gzip_owned, read_owned, write_gzip_owned, write_owned,
};
pub use schema::{FromNbtCompound, NbtCompoundExt, NbtPath, SchemaError, ToNbtCompound};
