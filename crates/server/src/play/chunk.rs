//! The hand-built superflat placeholder chunk content (M1-B05 blueprint Context, "The
//! superflat placeholder content" / "`LevelChunkWithLight` — field layout and the section
//! wire encoding" / "Heightmaps — a minimal, hand-rolled network-NBT writer"). Every chunk
//! this blueprint ever sends is byte-identical, computed once per connection by these pure
//! functions — nothing here touches `rc-chunk-storage` or persists anything.

#[cfg(test)]
use bytes::{Buf, Bytes};
use bytes::{BufMut, BytesMut};
use rc_registries::generated_v776::block_states::{self, default_state as blocks};

use super::mining;
use super::packets::{BlockEntityInfo, LightArray};

/// M2 integration note: `#[cfg(test)]` -- this constant's only remaining production-code
/// consumer, `build_placeholder_chunk_data` (below), lost its own last production call
/// site when `connection.rs`'s Play-entry sequence was rewired onto real, storage-backed
/// content (`encode_live_chunk_data`'s own doc comment); both now live on purely as this
/// file's own regression-test oracle (`encode_live_chunk_data_matches_the_placeholder_
/// blob_for_a_freshly_filled_column`, this file's own test module).
#[cfg(test)]
pub const WORLD_MIN_Y: i32 = -64;
pub const SECTION_COUNT: usize = 24;
/// M1 integration fix, round 4: this blueprint's own first implementation attempt set
/// this to `1` (a 3x3 = 9 chunk grid, `-1..=1` on both axes) -- every one of those 9
/// chunks decodes without error (a real client, and this crate's own `play_chunk_set.rs`
/// acceptance test, both confirmed that already), yet driving a *real, graphical* vanilla
/// client against it rendered textured blocks in the spawn chunk `(0, 0)` only: every one
/// of the other 8 chunks showed collision and a block outline on look (block state data
/// clearly arrived and was applied) but no texture at all -- a real client's own barrier-
/// block-like rendering of a chunk section it has not yet built a mesh for.
///
/// Root cause: this blueprint's own placeholder content is deliberately byte-identical
/// across every chunk sent (`placeholder_chunk_coords`'s own doc comment, "content is
/// identical across all 9 chunks, so only chunk coordinates differ between packets" --
/// still true verbatim of every chunk sent today), and `enter_play` never varies `data`/
/// `heightmaps`/any light field by chunk position either (every one of those fields is
/// one `Vec` built once and `.clone()`d per packet, `play::connection`'s own `enter_play`
/// loop) -- so a defect in the block/biome palette encoding, the block-state ids, or the
/// light arrays could only ever affect *every* chunk identically, never explain a
/// spawn-vs-outer difference by itself. The one thing that *does* differ per chunk is
/// exactly `(chunk_x, chunk_z)` -- and with radius `1`, `(0, 0)` is the unique chunk of
/// the 9 sent whose own full 3x3 neighborhood (Chebyshev distance 1, all 8 neighbors) is
/// entirely covered by that same 9-chunk set; every other of the 9 has at least one
/// neighbor outside it that this server never sends at all. A real vanilla client will
/// not build a chunk section's render mesh -- though it happily accepts the section's raw
/// block data for collision/outline purposes immediately, exactly the reported symptom --
/// until every one of that chunk's own neighboring columns has also been received; this
/// is the same well-documented reason a real dedicated server must advertise (and send)
/// a strictly larger radius than the area it actually wants to appear rendered. Radius
/// `2` (a 5x5 = 25 chunk grid) gives every chunk of the original 3x3 "visible" area its
/// own full neighbor coverage from this same sent set, with no change needed to
/// `LoginPlay.view_distance` (already `2`, `play::connection`'s own `enter_play` --
/// exactly large enough for a real client's own chunk-cache array to hold a 5x5 grid).
///
/// M1 integration fix, round 5: round 4's fix worked exactly as diagnosed -- the real
/// client's own 3x3 area around spawn rendered fully textured -- but round 4's own radius
/// `2` put the render-safe boundary (Context above: render-safe radius is always
/// `PLACEHOLDER_RADIUS_CHUNKS - 1`) only one ring out, so the unmeshed edge this project's
/// own round-4 fix could never fully eliminate (some ring always exists at the boundary
/// of whatever is sent, by the same reasoning above) sat close enough to spawn to be
/// immediately, jarringly visible. Raised to `5` (an 11x11 = 121 chunk grid): the
/// render-safe area becomes a 9x9 grid (radius `4`) around spawn, and the unmeshed ring
/// moves out to radius `5` -- far enough out to sit outside a player's immediate view at
/// spawn, matching how this exact edge behaves on a real vanilla server (a large view
/// distance does not eliminate the phenomenon, only pushes it past what's normally
/// visible). `LoginPlay.view_distance` raised to `5` alongside it (`play::connection`'s
/// own `enter_play`, same reasoning as round 4: large enough for a real client's own
/// chunk-cache array to hold an 11x11 grid).
pub const PLACEHOLDER_RADIUS_CHUNKS: i32 = 5;

/// `minecraft:worldgen/biome` is a dynamic/datapack registry -- Configuration's own
/// registry sync (`net::run_configuration`'s `worldgen_registries` parameter), never one of
/// the fixed built-in registries `rc-registries`' generated `registries.rs` module covers.
/// Confirmed against the actual committed codegen output: no `worldgen_biome` module exists
/// there (only the unrelated `worldgen_biome_source` registry, and `villager_type::PLAINS`,
/// neither of which is this blueprint's biome). This blueprint's own Context assumed
/// `rc_registries::generated_v776::registries::worldgen_biome::PLAINS` existed; it does not
/// -- a confirmed deviation, resolved by hardcoding the placeholder biome's wire id instead,
/// consistent with `minecraft:plains` sitting at index `0` of whatever `minecraft:worldgen/
/// biome` list a real composition root's `worldgen_registries` argument supplies (matching
/// M1-B04's own established test-fixture convention, `crates/server/tests/
/// login_configuration_flow.rs`'s `TEST_WORLDGEN_REGISTRIES`, where `"minecraft:plains"` is
/// listed first).
pub const PLACEHOLDER_BIOME_ID: u32 = 0;

/// A nominal registry size, used only to compute the biome paletted container's *Direct*-
/// path bit width -- never actually exercised by this blueprint's own fixtures, since this
/// blueprint's biome content is always a single value everywhere (Constraints: the Direct
/// arm exists for completeness/future reuse but is untested here). Picked in the ballpark
/// of vanilla's own biome count; its exact value is inconsequential precisely because the
/// Direct path is never reached.
const PLACEHOLDER_BIOME_REGISTRY_COUNT: u32 = 64;

/// `ceil(log2(count))`, exact and allocation-free for `count >= 1` (WORLD-D2's threshold
/// rule, restated as this blueprint's own shared helper -- `count == 1` yields `0`).
fn ceil_log2(count: u32) -> u32 {
    if count <= 1 {
        0
    } else {
        32 - (count - 1).leading_zeros()
    }
}

/// Every `(chunk_x, chunk_z)` this blueprint sends, in the exact clientbound send order
/// (Context, step 8): `cx` outer ascending, `cz` inner ascending.
pub fn placeholder_chunk_coords() -> Vec<(i32, i32)> {
    // M1 integration fix, round 5: this capacity hint used to be a literal number
    // (`25`, matching round 4's own radius `2`) that silently went stale the moment the
    // radius changed again -- computed from `PLACEHOLDER_RADIUS_CHUNKS` itself instead,
    // so it can never drift from the loop below again.
    let side = (2 * PLACEHOLDER_RADIUS_CHUNKS + 1) as usize;
    let mut coords = Vec::with_capacity(side * side);
    for cx in -PLACEHOLDER_RADIUS_CHUNKS..=PLACEHOLDER_RADIUS_CHUNKS {
        for cz in -PLACEHOLDER_RADIUS_CHUNKS..=PLACEHOLDER_RADIUS_CHUNKS {
            coords.push((cx, cz));
        }
    }
    coords
}

/// Bit-packs `values` (each `< 2^bits_per_entry`) into big-endian i64 longs, non-spanning
/// (Context: "Non-spanning bit packing"). `bits_per_entry == 0` returns an empty `Vec`.
pub fn pack_bits(values: &[u32], bits_per_entry: u32) -> Vec<i64> {
    if bits_per_entry == 0 {
        return Vec::new();
    }
    let entries_per_long = (64 / bits_per_entry) as usize;
    let mut out = Vec::with_capacity(values.len().div_ceil(entries_per_long));
    for chunk in values.chunks(entries_per_long) {
        let mut long: u64 = 0;
        for (i, &v) in chunk.iter().enumerate() {
            long |= (v as u64) << (i as u32 * bits_per_entry);
        }
        out.push(long as i64);
    }
    out
}

/// Encodes one WORLD-D2 paletted container. Distinct-value count `== 1` always takes the
/// `SingleValue` (0-bit) path regardless of `indirect_floor_bits`. Otherwise `bits =
/// max(indirect_floor_bits, ceil(log2(distinct_count)))`, then `Indirect` if `bits <=
/// max_indirect_bits`, else `Direct` at `direct_bits` (Context's exact threshold table).
///
/// M1 integration fix: this blueprint's own first implementation attempt wrote an explicit
/// `VarInt` "data array length" field after the palette, before the packed longs, in every
/// branch (`SingleValue` included, as a literal `VarInt(0)`). Driving a real client
/// (azalea) against it produced a downstream panic inside the client's own chunk-loading
/// system (`assertion failed: (1..=32).contains(&bits)`, `azalea-world`'s `bit_storage.rs`)
/// while parsing an *already successfully framed* `LevelChunkWithLight` packet's `data`
/// blob -- reading azalea's own real container decoder directly
/// (`azalea-world/src/palette/container.rs::PalettedContainer::read`, Constraints (d))
/// confirms protocol 776 carries **no such field on the wire at all**: a real client
/// computes the data array's exact length itself, deterministically, from `bits_per_entry`
/// and the container's own fixed size (4096 for blocks, 64 for biomes) --
/// `size.div_ceil(64 / bits_per_entry)`, azalea's own `BitStorage::new` -- and reads
/// exactly that many longs unconditionally; our extra `VarInt` byte(s) shifted every
/// following byte, corrupting the next container's own `bits_per_entry` read. Removed in
/// all three branches; `pack_bits`'s own output length already exactly equals that same
/// `div_ceil` formula for this blueprint's fixed-size `entries` slices (4096/64), so no
/// other change is needed for a real client to compute the identical length on its own.
pub fn encode_paletted_container(
    out: &mut BytesMut,
    entries: &[u32],
    indirect_floor_bits: u32,
    max_indirect_bits: u32,
    direct_bits: u32,
) {
    // Palette order is first-encountered order over `entries` -- a small linear scan is
    // more than fast enough for this blueprint's own tiny (<= 4) distinct-value content.
    let mut palette: Vec<u32> = Vec::new();
    for &v in entries {
        if !palette.contains(&v) {
            palette.push(v);
        }
    }

    if palette.len() == 1 {
        out.put_u8(0);
        rc_protocol::VarInt::new(palette[0] as i32).encode(out);
        return;
    }

    let raw_bits = ceil_log2(palette.len() as u32);
    let bits = raw_bits.max(indirect_floor_bits);

    if bits <= max_indirect_bits {
        out.put_u8(bits as u8);
        rc_protocol::VarInt::new(palette.len() as i32).encode(out);
        for &entry in &palette {
            rc_protocol::VarInt::new(entry as i32).encode(out);
        }
        let indices: Vec<u32> = entries
            .iter()
            .map(|v| palette.iter().position(|p| p == v).unwrap() as u32)
            .collect();
        let longs = pack_bits(&indices, bits);
        for long in &longs {
            out.put_i64(*long);
        }
    } else {
        out.put_u8(direct_bits as u8);
        let longs = pack_bits(entries, direct_bits);
        for long in &longs {
            out.put_i64(*long);
        }
    }
}

/// One full section (Context's `block_count` + two paletted containers).
///
/// M1 integration fix: this blueprint's own first implementation attempt wrote only
/// `block_count` before the two paletted containers. Reading azalea's own real `Section`
/// decoder directly (`azalea-world/src/chunk/mod.rs::Section::azalea_read`, Constraints
/// (d)) shows a second `u16` field, `fluid_count`, sits between `block_count` and the
/// block `PalettedContainer` -- omitting it shifted every following byte in every section
/// by two, corrupting the next paletted container's own `bits_per_entry` read (the
/// concrete failure this produced: a downstream panic inside the client's own chunk-
/// loading system, `assertion failed: (1..=32).contains(&bits)`,
/// `azalea-world`'s `bit_storage.rs`, while parsing an already successfully framed
/// `LevelChunkWithLight` packet). This blueprint's own placeholder world has no fluid
/// content anywhere, so `fluid_count` is always `0` -- a real, always-correct value, not a
/// placeholder approximation.
pub fn encode_section(block_state_ids: &[u32; 4096], biome_ids: &[u32; 64]) -> Vec<u8> {
    let mut out = BytesMut::new();

    let air = blocks::AIR.0;
    let block_count: i16 = block_state_ids.iter().filter(|&&id| id != air).count() as i16;
    out.put_i16(block_count);
    out.put_i16(0); // fluid_count -- always 0, no fluid content exists in this world.

    let block_registry_bits = ceil_log2(block_states::BLOCK_STATE_COUNT);
    encode_paletted_container(&mut out, block_state_ids, 4, 8, block_registry_bits);

    let biome_registry_bits = ceil_log2(PLACEHOLDER_BIOME_REGISTRY_COUNT);
    encode_paletted_container(&mut out, biome_ids, 1, 3, biome_registry_bits);

    out.to_vec()
}

/// This blueprint's fixed superflat content (Context's layer table) as one 24-section
/// `data` blob, identical for every chunk. Section 0 (`y in [-64, -48)`) carries the real
/// layer content; every other section is pure air.
///
/// M2 integration note: `#[cfg(test)]` -- `connection.rs` no longer calls this (real,
/// storage-backed content now flows through `encode_live_chunk_data` instead,
/// `M2-COMPLETION-REPORT.md`'s own diagnosed gap); kept as this file's own regression-test
/// oracle, proving the two encoders agree byte-for-byte on a freshly filled column (this
/// file's own test module).
#[cfg(test)]
pub fn build_placeholder_chunk_data() -> Vec<u8> {
    let mut data = Vec::new();

    // Section 0 (world y in [WORLD_MIN_Y, WORLD_MIN_Y + 16)): the layer table (Context).
    let mut block_state_ids = [blocks::AIR.0; 4096];
    for local_y in 0..16i32 {
        let world_y = WORLD_MIN_Y + local_y;
        let block = match world_y {
            -64 => blocks::BEDROCK.0,
            -63..=-62 => blocks::DIRT.0,
            -61 => blocks::GRASS_BLOCK.0,
            _ => blocks::AIR.0,
        };
        for z in 0..16usize {
            for x in 0..16usize {
                block_state_ids[local_y as usize * 256 + z * 16 + x] = block;
            }
        }
    }
    let biome_ids = [PLACEHOLDER_BIOME_ID; 64];
    data.extend(encode_section(&block_state_ids, &biome_ids));

    // Sections 1..24: pure air, single biome value.
    let air_block_state_ids = [blocks::AIR.0; 4096];
    for _ in 1..SECTION_COUNT {
        data.extend(encode_section(&air_block_state_ids, &biome_ids));
    }

    data
}

/// M2 integration addition: encodes one real, storage-backed chunk column's block/biome
/// content into the exact same wire shape `build_placeholder_chunk_data` produces --
/// closing `rc_chunk_storage::palette::PalettedContainer`'s own documented "wire-encoder
/// reuse is a future blueprint's integration" gap (M2-B01's Resolved discrepancy). Reads
/// the real M2-B01 `BlockStateColumn`/`BiomeColumn` components (`play::world`'s own
/// resident chunk entities) instead of a fixed layer table; `encode_paletted_container`
/// rebuilds its own first-encountered-order palette straight from these raw ids on every
/// call, so the resulting bytes depend only on the per-index values below, never on
/// whichever internal `Palette` representation the source `PalettedContainer` happens to
/// carry -- `PalettedContainer::iter()`'s own index-ascending order already matches this
/// file's own wire section layout exactly (both use `(local_y << 8) | (z << 4) | x` for
/// blocks and the matching 4x4x4 order for biomes, `rc_chunk_storage::column`'s own
/// `block_index`/`biome_index`), so no reshaping beyond the raw-id extraction
/// `encode_section` already performs for the placeholder path is needed.
pub fn encode_live_chunk_data(
    blocks: &rc_chunk_storage::BlockStateColumn,
    biomes: &rc_chunk_storage::BiomeColumn,
) -> Vec<u8> {
    use rc_chunk_storage::RegistryId;

    let mut data = Vec::new();
    for section_index in 0..SECTION_COUNT {
        let block_ids: Vec<u32> = blocks
            .section(section_index)
            .iter()
            .map(|id| id.to_raw())
            .collect();
        let biome_ids: Vec<u32> = biomes
            .section(section_index)
            .iter()
            .map(|id| id.to_raw())
            .collect();
        let block_arr: [u32; 4096] = block_ids
            .try_into()
            .expect("BlockStateColumn's own section entry_count is always 4096 (M2-B01)");
        let biome_arr: [u32; 64] = biome_ids
            .try_into()
            .expect("BiomeColumn's own section entry_count is always 64 (M2-B01)");
        data.extend(encode_section(&block_arr, &biome_arr));
    }
    data
}

/// M3-B0X block-entity production wiring (Context, "Chunk packet block-entity list"): one
/// `BlockEntityInfo` per block in `blocks` whose raw state id is one of the six
/// `BlockEntityWireKind`s' reachable ids (`play::mining::block_entity_wire_kind_for_raw_state`)
/// -- derived directly from the column's own live block-state content, not from `rc-chunk-
/// storage`'s `BlockEntityIndex` (which only tracks the five ECS-backed kinds this project
/// spawns a real component for, never `Comparator` -- Context, "Comparator BE"): scanning raw
/// state instead means a comparator's chunk-list entry needs no separate tracking, and the
/// furnace/blast_furnace/smoker distinction (one shared `FurnaceBlockEntity` component, three
/// distinct `minecraft:block_entity_type` ids) falls out for free, since the id alone already
/// tells the three apart. Iterated in the same per-section, index-ascending order `encode_live_
/// chunk_data` already visits every entry in (`block_index`'s own `(local_y<<8)|(z<<4)|x`
/// order, `rc_chunk_storage::column`'s own doc comment) -- no extra pass over the column beyond
/// what that function already pays for, only a cheap `HashMap` lookup per entry. `packed_xz`/
/// `y` match `BlockEntityInfo`'s own doc comment exactly.
pub fn encode_block_entities(blocks: &rc_chunk_storage::BlockStateColumn) -> Vec<BlockEntityInfo> {
    use rc_chunk_storage::{RegistryId, WORLD_MIN_Y as CHUNK_STORAGE_WORLD_MIN_Y};

    let mut out = Vec::new();
    for section_index in 0..SECTION_COUNT {
        let section = blocks.section(section_index);
        // M3 field-report fix (CI hang, 2026-09-02): a full 4096-cell walk over all 26
        // sections is 106k `block_entity_wire_kind_for_raw_state` calls per chunk -- 12.9M
        // per join (121 chunks) in an unoptimized test build, which turned every
        // two-player integration test into a 30s..300s stall on the CI runners. The
        // section's own palette already says whether any block-entity kind can occur in
        // it at all: `SingleValue`/`Indirect` palettes are checked in O(palette) and the
        // cell walk only runs for sections that can actually contain one (`Direct`
        // palettes -- huge, never seen in this project's own worlds -- keep the full walk).
        let may_hold_block_entity = match section.palette() {
            rc_chunk_storage::Palette::SingleValue(id) => {
                mining::block_entity_wire_kind_for_raw_state(id.to_raw()).is_some()
            }
            rc_chunk_storage::Palette::Indirect { entries, .. } => entries
                .iter()
                .any(|id| mining::block_entity_wire_kind_for_raw_state(id.to_raw()).is_some()),
            rc_chunk_storage::Palette::Direct { .. } => true,
        };
        if !may_hold_block_entity {
            continue;
        }
        let section_base_y = CHUNK_STORAGE_WORLD_MIN_Y + (section_index as i32) * 16;
        for (i, id) in section.iter().enumerate() {
            let Some(kind) = mining::block_entity_wire_kind_for_raw_state(id.to_raw()) else {
                continue;
            };
            let local_y = (i >> 8) as i32;
            let z = ((i >> 4) & 0x0F) as u8;
            let x = (i & 0x0F) as u8;
            out.push(BlockEntityInfo {
                packed_xz: (x << 4) | z,
                y: (section_base_y + local_y) as i16,
                type_id: kind.registry_type_id(),
            });
        }
    }
    out
}

/// M1 integration fix: this blueprint's own first implementation attempt hand-rolled
/// `heightmaps` as a network-NBT compound (`WORLD_SURFACE`/`MOTION_BLOCKING`/
/// `MOTION_BLOCKING_NO_LEAVES`, each a `TAG_Long_Array`) -- a fundamentally wrong wire
/// shape. Driving a real client (azalea) against `LevelChunkWithLight` carrying that NBT
/// blob produced `Error reading packet level_chunk_with_light (id 45): failed to fill
/// whole buffer` on every single chunk: `ClientboundLevelChunkPacketData.heightmaps` is a
/// plain `VarInt`-prefixed list of `(HeightmapKind, Box<[u64]>)` tuples (`azalea-protocol`'s
/// own source, Constraints (d)), never NBT at all -- a real client reads our NBT blob's own
/// leading byte count as a tuple count and desyncs immediately trying to decode bogus
/// tuples from it. `crates/testing/test-harness/src/fake_server.rs`'s own
/// `SendPlayLogin`/`encode_play_entry_sequence` step had already independently discovered
/// and worked around this (sending an empty heightmaps list, `VarInt(0)`) -- ported here
/// into the real production path instead of only the test double. Sending no precomputed
/// heightmaps is legal, parity-neutral vanilla wire behavior, not an approximation: a real
/// client recomputes the exact same heightmap values itself from the chunk's own block
/// data whenever none are supplied, so this changes nothing observable, only which side
/// does the (identical) computation.
pub fn build_placeholder_heightmaps() -> Vec<u8> {
    Vec::new()
}

/// Every section index (26, WORLD-D8's "+2 padding") set to full sky light, zero block
/// light (Context's light-data simplification).
#[allow(clippy::type_complexity)]
pub fn build_placeholder_light() -> (
    Vec<i64>,
    Vec<i64>,
    Vec<i64>,
    Vec<i64>,
    Vec<LightArray>,
    Vec<LightArray>,
) {
    let all_26_set = pack_bits(&[1u32; 26], 1);
    let sky_light_mask = all_26_set.clone();
    let block_light_mask = Vec::new();
    let empty_sky_light_mask = Vec::new();
    let empty_block_light_mask = all_26_set;
    let sky_light_arrays = vec![LightArray([0xFFu8; 2048]); 26];
    let block_light_arrays = Vec::new();
    (
        sky_light_mask,
        block_light_mask,
        empty_sky_light_mask,
        empty_block_light_mask,
        sky_light_arrays,
        block_light_arrays,
    )
}

/// Test-only decode helper (this module's own encode is one-directional in production --
/// `chunk.rs`'s acceptance tests decode a paletted container back out to assert its shape).
#[cfg(test)]
pub(crate) struct DecodedPalettedContainer {
    pub bits_per_entry: u8,
    pub palette: Vec<u32>,
    pub indices: Vec<u32>,
}

#[cfg(test)]
pub(crate) fn decode_paletted_container(
    buf: &mut Bytes,
    entry_count: usize,
) -> DecodedPalettedContainer {
    // M1 integration fix: no explicit "data array length" `VarInt` exists on the wire for
    // protocol 776 (`encode_paletted_container`'s own doc comment) -- a real client
    // computes it deterministically from `bits_per_entry` and the container's own fixed
    // `entry_count`, exactly mirroring `pack_bits`'s own `div_ceil` formula here.
    let bits_per_entry = buf.get_u8();
    match bits_per_entry {
        0 => {
            let value = rc_protocol::VarInt::decode(buf).unwrap().get() as u32;
            DecodedPalettedContainer {
                bits_per_entry,
                palette: vec![value],
                indices: vec![0; entry_count],
            }
        }
        bits => {
            let palette_length = rc_protocol::VarInt::decode(buf).unwrap().get() as usize;
            let mut palette = Vec::with_capacity(palette_length);
            for _ in 0..palette_length {
                palette.push(rc_protocol::VarInt::decode(buf).unwrap().get() as u32);
            }
            let entries_per_long = (64 / bits as u32) as usize;
            let data_array_length = entry_count.div_ceil(entries_per_long);
            let mut longs = Vec::with_capacity(data_array_length);
            for _ in 0..data_array_length {
                longs.push(buf.get_i64());
            }
            let mask = (1u64 << bits) - 1;
            let mut indices = Vec::with_capacity(entry_count);
            'outer: for long in &longs {
                for i in 0..entries_per_long {
                    if indices.len() == entry_count {
                        break 'outer;
                    }
                    indices.push((((*long as u64) >> (i as u32 * bits as u32)) & mask) as u32);
                }
            }
            DecodedPalettedContainer {
                bits_per_entry,
                palette,
                indices,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_bits_round_trips_non_spanning() {
        let values = [1u32, 2, 3, 4, 5];
        let longs = pack_bits(&values, 4);
        assert_eq!(longs.len(), 1);
        assert_eq!(longs[0] & 0xF, 1);
        assert_eq!((longs[0] >> 4) & 0xF, 2);
    }

    #[test]
    fn encode_paletted_container_single_value_is_zero_bits() {
        let mut out = BytesMut::new();
        encode_paletted_container(&mut out, &[7u32; 16], 4, 8, 15);
        let mut bytes = out.freeze();
        let decoded = decode_paletted_container(&mut bytes, 16);
        assert_eq!(decoded.bits_per_entry, 0);
        assert_eq!(decoded.palette, vec![7]);
    }

    /// M1 integration fix, round 5 prep: parameterized off `PLACEHOLDER_RADIUS_CHUNKS`
    /// directly (in-crate, unlike the cross-crate mirrors `play_chunk_set.rs`/
    /// `chunk_decode_diagnostic.rs` need, `mod.rs`'s own doc comment on why `chunk` stays
    /// crate-internal) rather than hard-coding the radius-2 grid's own shape -- a future
    /// radius change needs zero edits here ever again.
    #[test]
    fn placeholder_chunk_coords_are_row_major_ascending() {
        let coords = placeholder_chunk_coords();
        let side = (2 * PLACEHOLDER_RADIUS_CHUNKS + 1) as usize;
        assert_eq!(coords.len(), side * side);
        assert_eq!(
            coords[0],
            (-PLACEHOLDER_RADIUS_CHUNKS, -PLACEHOLDER_RADIUS_CHUNKS)
        );
        assert_eq!(
            coords[coords.len() - 1],
            (PLACEHOLDER_RADIUS_CHUNKS, PLACEHOLDER_RADIUS_CHUNKS)
        );
    }

    /// M1 integration fix, round 4 (`PLACEHOLDER_RADIUS_CHUNKS`'s own doc comment): every
    /// chunk of the render-safe "visible" area -- radius `PLACEHOLDER_RADIUS_CHUNKS - 1`,
    /// one ring inside the actual send radius -- must have its own full 3x3 neighborhood
    /// -- all 8 Chebyshev-distance-1 neighbors -- present in the sent set, or a real
    /// client will not build a render mesh for it. This is the exact regression guard for
    /// the round-4 root cause: radius `1` fails this check for every one of its 8
    /// non-center chunks (each has at least one neighbor outside the 9-chunk set); every
    /// radius `>= 2` passes it for its own render-safe area -- parameterized (round 5
    /// prep) so raising the radius further never needs to touch this test again.
    #[test]
    fn every_render_safe_chunk_has_full_neighbor_coverage() {
        let sent: std::collections::HashSet<(i32, i32)> =
            placeholder_chunk_coords().into_iter().collect();
        let render_safe_radius = PLACEHOLDER_RADIUS_CHUNKS - 1;
        for cx in -render_safe_radius..=render_safe_radius {
            for cz in -render_safe_radius..=render_safe_radius {
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        assert!(
                            sent.contains(&(cx + dx, cz + dz)),
                            "chunk ({cx}, {cz})'s own neighbor ({}, {}) is missing from the \
                             sent set -- a real client will not render ({cx}, {cz})",
                            cx + dx,
                            cz + dz
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn encode_paletted_container_indirect_round_trips_indices() {
        let entries = [10u32, 20, 10, 30, 20, 10];
        let mut out = BytesMut::new();
        encode_paletted_container(&mut out, &entries, 4, 8, 15);
        let mut bytes = out.freeze();
        let decoded = decode_paletted_container(&mut bytes, entries.len());
        assert_eq!(decoded.bits_per_entry, 4);
        assert_eq!(decoded.palette.len(), 3);
        let resolved: Vec<u32> = decoded
            .indices
            .iter()
            .map(|&i| decoded.palette[i as usize])
            .collect();
        assert_eq!(resolved, entries);
    }

    /// M2 integration addition: proves `encode_live_chunk_data`'s claim (its own doc
    /// comment) that reading real, storage-backed `BlockStateColumn`/`BiomeColumn`
    /// content through `encode_section` produces byte-identical output to the fixed
    /// placeholder table, for a freshly `SuperflatFiller`-seeded column -- the two
    /// tables encode the exact same per-position values (`superflat.rs`'s own doc
    /// comment: "Restated exactly from M1-B05's own already-merged, byte-verified layer
    /// table"), so the two encoders must agree regardless of the source
    /// `PalettedContainer`'s own internal palette representation.
    #[test]
    fn encode_live_chunk_data_matches_the_placeholder_blob_for_a_freshly_filled_column() {
        use rc_chunk_storage::RegistryId;

        let block_direct_bits = ceil_log2(block_states::BLOCK_STATE_COUNT) as u16;
        let filler = rc_chunk_storage::superflat::SuperflatFiller {
            air: rc_chunk_storage::BlockStateId::from_raw(blocks::AIR.0),
            bedrock: rc_chunk_storage::BlockStateId::from_raw(blocks::BEDROCK.0),
            dirt: rc_chunk_storage::BlockStateId::from_raw(blocks::DIRT.0),
            grass: rc_chunk_storage::BlockStateId::from_raw(blocks::GRASS_BLOCK.0),
            biome: rc_chunk_storage::BiomeId::from_raw(PLACEHOLDER_BIOME_ID),
            block_thresholds: rc_chunk_storage::PaletteThresholds::blocks(block_direct_bits),
            biome_thresholds: rc_chunk_storage::PaletteThresholds::biomes(ceil_log2(64) as u16),
        };
        let (blocks_col, biomes_col, _heightmaps, _light, _status) = filler.fill();

        let live = encode_live_chunk_data(&blocks_col, &biomes_col);
        assert_eq!(live, build_placeholder_chunk_data());
    }
}
