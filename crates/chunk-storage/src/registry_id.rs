//! Local, registry-agnostic id newtypes (Context's "Resolved discrepancy"): numerically
//! identical to Mojang's own global registry ids, but a textually distinct Rust type
//! from `rc_registries::generated_v776`'s own generated tables — this crate never
//! depends on those tables directly (WORLD-D3/D4).

/// A palette-storable global registry identifier. Implemented by this crate's own
/// `BlockStateId`/`BiomeId` (Context's Resolved discrepancy — never by
/// `rc_registries::generated_v776`'s types directly, which this crate deliberately
/// stays decoupled from).
pub trait RegistryId:
    Copy + Eq + Ord + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static
{
    /// The raw global/protocol id, numerically identical to Mojang's own assigned id
    /// for this entry (WORLD-D3's own "reused verbatim" contract).
    fn to_raw(self) -> u32;
    fn from_raw(raw: u32) -> Self;
}

/// Numerically identical to (but a distinct Rust type from)
/// `rc_registries::generated_v776::block_states::BlockStateId` — Context's Resolved
/// discrepancy explains why this crate does not use that type directly.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockStateId(pub u32);

/// Numerically identical to (but a distinct Rust type from, and narrower than —
/// `u16` vs. `u32` — since no biome registry remotely approaches 65536 entries)
/// `rc_registries::generated_v776::registries::RegistryEntryId` when that type holds a
/// biome-registry value. WORLD-D4's own originally-specified shape.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BiomeId(pub u16);

impl RegistryId for BlockStateId {
    fn to_raw(self) -> u32 {
        todo!()
    }
    fn from_raw(raw: u32) -> Self {
        todo!()
    }
}
impl RegistryId for BiomeId {
    fn to_raw(self) -> u32 {
        todo!()
    }
    fn from_raw(raw: u32) -> Self {
        todo!()
    }
}

/// The two threshold profiles WORLD-D2 pins (Context), plus the Direct-palette bit
/// width — which this crate never hardcodes (a registry's total entry count is not
/// this crate's data, see Context's Resolved discrepancy) — supplied explicitly by
/// every caller.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PaletteThresholds {
    pub indirect_floor_bits: u8,
    pub max_indirect_bits: u8,
    pub direct_bits: u16,
}

impl PaletteThresholds {
    /// `indirect_floor_bits = 4`, `max_indirect_bits = 8` (WORLD-D2). `direct_bits` is
    /// the caller's own `ceil_log2(<block-state registry's total entry count>)` — for
    /// the pinned DataVersion 4903 target, `docs/research/mc-26.2/03-world-chunks.md`
    /// §3.10/§5 records 32366 block states, i.e. `direct_bits = 15`; this crate does not
    /// bake that number in (it would silently go stale if the registry ever regenerates
    /// with a different count) — pass it explicitly, e.g. `PaletteThresholds::blocks(15)`.
    pub const fn blocks(direct_bits: u16) -> Self {
        todo!()
    }

    /// `indirect_floor_bits = 1`, `max_indirect_bits = 3` (WORLD-D2). `direct_bits` is
    /// the caller's own `ceil_log2(<biome registry's total entry count>)` — this
    /// blueprint does not know or assert the real pinned-version biome count (it is not
    /// recorded anywhere in this project's research corpus at the time of writing);
    /// pass it explicitly once a real generated biome registry is available.
    pub const fn biomes(direct_bits: u16) -> Self {
        todo!()
    }
}
