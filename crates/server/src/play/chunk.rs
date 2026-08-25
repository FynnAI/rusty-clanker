//! The hand-built superflat placeholder chunk content (M1-B05 blueprint Context, "The
//! superflat placeholder content" / "`LevelChunkWithLight` — field layout and the section
//! wire encoding" / "Heightmaps — a minimal, hand-rolled network-NBT writer"). Every chunk
//! this blueprint ever sends is byte-identical, computed once per connection by these pure
//! functions — nothing here touches `rc-chunk-storage` or persists anything.

#[cfg(test)]
use bytes::{Buf, Bytes};
use bytes::{BufMut, BytesMut};
use rc_registries::generated_v776::block_states::{self, default_state as blocks};

use super::packets::LightArray;

pub const WORLD_MIN_Y: i32 = -64;
pub const SECTION_COUNT: usize = 24;
/// `-1..=1` on both axes -- a 3x3 = 9 chunk grid.
pub const PLACEHOLDER_RADIUS_CHUNKS: i32 = 1;

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
    let mut coords = Vec::with_capacity(9);
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
pub fn build_placeholder_chunk_data() -> Vec<u8> {
    let mut data = Vec::new();

    // Section 0 (world y in [WORLD_MIN_Y, WORLD_MIN_Y + 16)): the layer table (Context).
    let mut block_state_ids = [blocks::AIR.0; 4096];
    for local_y in 0..16i32 {
        let world_y = WORLD_MIN_Y + local_y;
        let block = match world_y {
            -64 => blocks::BEDROCK.0,
            -63..=-61 => blocks::DIRT.0,
            -60 => blocks::GRASS_BLOCK.0,
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

    #[test]
    fn placeholder_chunk_coords_are_row_major_ascending() {
        let coords = placeholder_chunk_coords();
        assert_eq!(coords.len(), 9);
        assert_eq!(coords[0], (-1, -1));
        assert_eq!(coords[8], (1, 1));
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
}
