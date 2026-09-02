//! Vanilla chunk NBT (de)serialization at the pinned DataVersion (WORLD-D11/D16), built
//! entirely on M2-B01's already-committed chunk components and M2-B02's already-committed
//! `rc-nbt` surface. Hand-written, never derived (WORLD-D11) -- see this blueprint's own
//! Context for the full schema, the on-disk paletted-container encoding, the registry-id
//! resolver seam, and the fixed-default/opaque-extra unknown-tag policy.

use std::collections::HashMap;

use crate::{
    BiomeColumn, BiomeId, BlockEntityRecord, BlockStateColumn, BlockStateId, ChunkGenStatus,
    ChunkKeyTag, ChunkPersistenceState, ChunkStatus, HeightmapKind, HeightmapSet, LightColumn,
    LightSection, PaletteThresholds, PalettedContainer,
};
use rc_core::{ChunkKey, DimensionId};
use rc_nbt::{Mutf8Str, Mutf8String, borrow, owned};

/// The pinned target's DataVersion (WORLD-D16). Every document this crate writes
/// stamps this value; a loaded document whose `DataVersion` differs is refused.
pub const DATA_VERSION: i32 = 4903;

/// The vanilla `yPos` value every document this crate writes or accepts must carry --
/// `WORLD_MIN_Y / 16` (Context).
pub const MIN_SECTION_Y: i32 = crate::WORLD_MIN_Y / 16;

/// WORLD-D5's own 256-column/9-bits-per-entry heightmap packing convention, restated
/// (M2-B01's own `HEIGHTMAP_BITS_PER_ENTRY`/`HEIGHTMAP_COLUMN_ENTRIES`, private to that
/// module -- this blueprint keeps its own copy rather than reaching into a sibling
/// module's private constants).
const HEIGHTMAP_ENTRIES: usize = 256;
const HEIGHTMAP_BITS: u32 = 9;

/// On-disk paletted-container bit-width floors (Context's "on-disk paletted-container
/// encoding" -- a fixed rule independent of any registry-supplied `PaletteThresholds`).
const BLOCK_FLOOR_BITS: u32 = 4;
const BIOME_FLOOR_BITS: u32 = 1;

/// Caller-supplied bridge from this crate's registry-agnostic `BlockStateId` to the
/// vanilla `{Name, Properties}` palette-entry shape (Context's Resolved discrepancy).
/// No implementation ships in this crate.
pub trait BlockStateNames {
    /// The block's namespaced id and its state's property key/value pairs, in **any**
    /// order -- this crate re-sorts them before writing (next subsection). `None` means
    /// "this crate's registry has no entry for `id`" (an incomplete/corrupt resolver,
    /// or a raw id from a newer registry this build does not know about).
    fn name_and_properties(
        &self,
        id: BlockStateId,
    ) -> Option<(Mutf8String, Vec<(Mutf8String, Mutf8String)>)>;
    /// The inverse: a name + property set (in whatever order the NBT document stored
    /// them) resolved back to a concrete id. `None` if no registered state matches.
    fn resolve(
        &self,
        name: &Mutf8Str,
        properties: &[(&Mutf8Str, &Mutf8Str)],
    ) -> Option<BlockStateId>;
}

/// As `BlockStateNames`, for biomes -- plain-string palette entries, no properties.
pub trait BiomeNames {
    fn name(&self, id: BiomeId) -> Option<Mutf8String>;
    fn resolve(&self, name: &Mutf8Str) -> Option<BiomeId>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChunkNbtError {
    #[error("unsupported DataVersion: expected {expected}, found {found}")]
    UnsupportedDataVersion { expected: i32, found: i32 },
    #[error("missing required field `{0}`")]
    MissingField(&'static str),
    #[error("field `{0}` has the wrong NBT tag type")]
    WrongFieldType(&'static str),
    #[error("yPos {found} does not match this engine's fixed world bounds (expected {expected})")]
    UnexpectedYPos { expected: i32, found: i32 },
    #[error("section Y {0} is out of the supported light/block range")]
    SectionYOutOfRange(i32),
    #[error("missing required block section for Y {0}")]
    MissingSection(i32),
    #[error("malformed palette in field `{0}`: {1}")]
    MalformedPalette(&'static str, String),
    #[error("unknown block state name `{0}` — the supplied BlockStateNames resolver has no match")]
    UnknownBlockStateName(String),
    #[error("unknown biome name `{0}` — the supplied BiomeNames resolver has no match")]
    UnknownBiomeName(String),
    #[error(transparent)]
    Nbt(#[from] rc_nbt::NbtError),
}

/// Every component `chunk_from_nbt` reconstructs, plus the two fields this crate does
/// not store anywhere else (Context: `isLightOn` is a plain passthrough; `extra` is the
/// opaque unknown-tag bag).
pub struct ChunkNbtDocument {
    pub chunk_key: ChunkKeyTag,
    pub blocks: BlockStateColumn,
    pub biomes: BiomeColumn,
    pub light: LightColumn,
    pub heightmaps: HeightmapSet,
    pub block_entity_records: Vec<BlockEntityRecord>,
    pub status: ChunkStatus,
    pub persistence: ChunkPersistenceState,
    pub is_light_on: bool,
    pub extra: Vec<(Mutf8String, owned::NbtTag)>,
}

/// Bundles the two registry resolvers and the two `PaletteThresholds` a caller must
/// supply (Context -- this crate never bakes in a registry's own size). One `to_nbt`/
/// `from_nbt` call pair per chunk; cheap to construct, holds only borrows and `Copy`
/// values.
pub struct ChunkNbtCodec<'a, N: BlockStateNames, B: BiomeNames> {
    pub block_names: &'a N,
    pub biome_names: &'a B,
    pub block_thresholds: PaletteThresholds,
    pub biome_thresholds: PaletteThresholds,
}

impl<'a, N: BlockStateNames, B: BiomeNames> ChunkNbtCodec<'a, N, B> {
    /// Builds the full vanilla chunk NBT compound (Context: schema, ordering, and the
    /// fixed-default/opaque-extra policy). `extra` is re-emitted verbatim, appended
    /// after every known and fixed-default field, in its given order -- pass `&[]` for
    /// a chunk with no captured unknown tags (e.g. one this engine created itself).
    /// `block_entity_records` is written into the `block_entities` list verbatim, in
    /// the given order (WORLD-D6) -- this crate never interprets a block entity's own
    /// field semantics, only carries the already-complete `data` compound through.
    /// Errors only on an `id` the resolvers cannot name.
    #[allow(clippy::too_many_arguments)]
    pub fn to_nbt(
        &self,
        chunk_key: ChunkKey,
        blocks: &BlockStateColumn,
        biomes: &BiomeColumn,
        light: &LightColumn,
        heightmaps: &HeightmapSet,
        block_entity_records: &[BlockEntityRecord],
        status: ChunkStatus,
        persistence: ChunkPersistenceState,
        is_light_on: bool,
        extra: &[(Mutf8String, owned::NbtTag)],
    ) -> Result<owned::NbtCompound, ChunkNbtError> {
        let mut fields: Vec<(Mutf8String, owned::NbtTag)> = vec![
            ("DataVersion".into(), owned::NbtTag::Int(DATA_VERSION)),
            ("xPos".into(), owned::NbtTag::Int(chunk_key.x)),
            ("zPos".into(), owned::NbtTag::Int(chunk_key.z)),
            ("yPos".into(), owned::NbtTag::Int(MIN_SECTION_Y)),
        ];
        let status_str = match status.0 {
            ChunkGenStatus::Full => "minecraft:full",
            ChunkGenStatus::Generating => "minecraft:empty",
        };
        fields.push(("Status".into(), owned::NbtTag::String(status_str.into())));
        fields.push((
            "LastUpdate".into(),
            owned::NbtTag::Long(persistence.last_saved_tick as i64),
        ));
        fields.push(("InhabitedTime".into(), owned::NbtTag::Long(0)));
        if is_light_on {
            fields.push(("isLightOn".into(), owned::NbtTag::Byte(1)));
        }

        let sections = self.build_sections(blocks, biomes, light)?;
        fields.push((
            "sections".into(),
            owned::NbtTag::List(owned::NbtList::Compound(sections)),
        ));

        fields.push((
            "block_entities".into(),
            owned::NbtTag::List(owned::NbtList::Compound(
                block_entity_records
                    .iter()
                    .map(|r| r.data.clone())
                    .collect(),
            )),
        ));

        fields.push((
            "Heightmaps".into(),
            owned::NbtTag::Compound(self.build_heightmaps(heightmaps)),
        ));

        fields.push((
            "block_ticks".into(),
            owned::NbtTag::List(owned::NbtList::Empty),
        ));
        fields.push((
            "fluid_ticks".into(),
            owned::NbtTag::List(owned::NbtList::Empty),
        ));
        fields.push((
            "structures".into(),
            owned::NbtTag::Compound(owned::NbtCompound::from_values(vec![
                (
                    "starts".into(),
                    owned::NbtTag::Compound(owned::NbtCompound::new()),
                ),
                (
                    "References".into(),
                    owned::NbtTag::Compound(owned::NbtCompound::new()),
                ),
            ])),
        ));
        fields.push((
            "PostProcessing".into(),
            owned::NbtTag::List(owned::NbtList::List(vec![owned::NbtList::Empty; 24])),
        ));

        for (name, tag) in extra {
            fields.push((name.clone(), tag.clone()));
        }

        Ok(owned::NbtCompound::from_values(fields))
    }

    /// The inverse. `dimension` is supplied by the caller (the region file the
    /// document was read from names it -- vanilla chunk NBT itself carries no
    /// dimension field, only `xPos`/`zPos`) and combined with the loaded `xPos`/`zPos`
    /// into the returned `ChunkKeyTag`.
    pub fn from_nbt(
        &self,
        tag: &borrow::NbtCompound<'_, '_>,
        dimension: DimensionId,
    ) -> Result<ChunkNbtDocument, ChunkNbtError> {
        // Fail fast on DataVersion/yPos, before touching anything else (Implementation
        // steps).
        let data_version = tag
            .int("DataVersion")
            .ok_or(ChunkNbtError::MissingField("DataVersion"))?;
        if data_version != DATA_VERSION {
            return Err(ChunkNbtError::UnsupportedDataVersion {
                expected: DATA_VERSION,
                found: data_version,
            });
        }
        let y_pos = tag.int("yPos").ok_or(ChunkNbtError::MissingField("yPos"))?;
        if y_pos != MIN_SECTION_Y {
            return Err(ChunkNbtError::UnexpectedYPos {
                expected: MIN_SECTION_Y,
                found: y_pos,
            });
        }

        let x_pos = tag.int("xPos").ok_or(ChunkNbtError::MissingField("xPos"))?;
        let z_pos = tag.int("zPos").ok_or(ChunkNbtError::MissingField("zPos"))?;
        let chunk_key = ChunkKeyTag(ChunkKey::new(dimension, x_pos, z_pos));

        let status_tag = tag
            .string("Status")
            .ok_or(ChunkNbtError::MissingField("Status"))?;
        let status = if status_tag.to_str().as_ref() == "minecraft:full" {
            ChunkStatus(ChunkGenStatus::Full)
        } else {
            ChunkStatus(ChunkGenStatus::Generating)
        };

        let last_update = tag
            .long("LastUpdate")
            .ok_or(ChunkNbtError::MissingField("LastUpdate"))?;
        let persistence = ChunkPersistenceState {
            dirty: false,
            last_saved_tick: last_update as u64,
        };

        let is_light_on = match tag.get("isLightOn") {
            None => false,
            Some(t) => t.byte().ok_or(ChunkNbtError::WrongFieldType("isLightOn"))? != 0,
        };

        let (blocks, biomes, light) = self.read_sections(tag)?;
        let heightmaps = self.read_heightmaps(tag)?;
        let block_entity_records = self.read_block_entities(tag)?;
        let extra = read_extra(tag);

        Ok(ChunkNbtDocument {
            chunk_key,
            blocks,
            biomes,
            light,
            heightmaps,
            block_entity_records,
            status,
            persistence,
            is_light_on,
            extra,
        })
    }

    fn build_sections(
        &self,
        blocks: &BlockStateColumn,
        biomes: &BiomeColumn,
        light: &LightColumn,
    ) -> Result<Vec<owned::NbtCompound>, ChunkNbtError> {
        let mut out = Vec::with_capacity(crate::SECTION_COUNT + 2);

        let below = light.section(0);
        if below.sky.is_some() || below.block.is_some() {
            let mut fields: Vec<(Mutf8String, owned::NbtTag)> =
                vec![("Y".into(), owned::NbtTag::Byte((MIN_SECTION_Y - 1) as i8))];
            fields.extend(light_field_pairs(below));
            out.push(owned::NbtCompound::from_values(fields));
        }

        for block_index in 0..crate::SECTION_COUNT {
            let vanilla_y = MIN_SECTION_Y + block_index as i32;

            let (block_palette, block_data) =
                disk_palette_and_data(blocks.section(block_index).iter(), BLOCK_FLOOR_BITS);
            let mut block_entries = Vec::with_capacity(block_palette.len());
            for id in block_palette {
                let (name, mut properties) = self
                    .block_names
                    .name_and_properties(id)
                    .ok_or_else(|| ChunkNbtError::UnknownBlockStateName(format!("{id:?}")))?;
                properties.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
                let mut entry_fields: Vec<(Mutf8String, owned::NbtTag)> =
                    vec![("Name".into(), owned::NbtTag::String(name))];
                if !properties.is_empty() {
                    let props: Vec<(Mutf8String, owned::NbtTag)> = properties
                        .into_iter()
                        .map(|(k, v)| (k, owned::NbtTag::String(v)))
                        .collect();
                    entry_fields.push((
                        "Properties".into(),
                        owned::NbtTag::Compound(owned::NbtCompound::from_values(props)),
                    ));
                }
                block_entries.push(owned::NbtCompound::from_values(entry_fields));
            }
            let mut block_states_fields: Vec<(Mutf8String, owned::NbtTag)> = vec![(
                "palette".into(),
                owned::NbtTag::List(owned::NbtList::Compound(block_entries)),
            )];
            if let Some(words) = block_data {
                block_states_fields.push((
                    "data".into(),
                    owned::NbtTag::LongArray(words.into_iter().map(|w| w as i64).collect()),
                ));
            }

            let (biome_palette, biome_data) =
                disk_palette_and_data(biomes.section(block_index).iter(), BIOME_FLOOR_BITS);
            let mut biome_names = Vec::with_capacity(biome_palette.len());
            for id in biome_palette {
                let name = self
                    .biome_names
                    .name(id)
                    .ok_or_else(|| ChunkNbtError::UnknownBiomeName(format!("{id:?}")))?;
                biome_names.push(name);
            }
            let mut biomes_fields: Vec<(Mutf8String, owned::NbtTag)> = vec![(
                "palette".into(),
                owned::NbtTag::List(owned::NbtList::String(biome_names)),
            )];
            if let Some(words) = biome_data {
                biomes_fields.push((
                    "data".into(),
                    owned::NbtTag::LongArray(words.into_iter().map(|w| w as i64).collect()),
                ));
            }

            let mut section_fields: Vec<(Mutf8String, owned::NbtTag)> = vec![
                ("Y".into(), owned::NbtTag::Byte(vanilla_y as i8)),
                (
                    "block_states".into(),
                    owned::NbtTag::Compound(owned::NbtCompound::from_values(block_states_fields)),
                ),
                (
                    "biomes".into(),
                    owned::NbtTag::Compound(owned::NbtCompound::from_values(biomes_fields)),
                ),
            ];
            section_fields.extend(light_field_pairs(light.section(block_index + 1)));
            out.push(owned::NbtCompound::from_values(section_fields));
        }

        let above = light.section(crate::LIGHT_SECTION_COUNT - 1);
        if above.sky.is_some() || above.block.is_some() {
            let mut fields: Vec<(Mutf8String, owned::NbtTag)> = vec![(
                "Y".into(),
                owned::NbtTag::Byte((MIN_SECTION_Y + crate::SECTION_COUNT as i32) as i8),
            )];
            fields.extend(light_field_pairs(above));
            out.push(owned::NbtCompound::from_values(fields));
        }

        Ok(out)
    }

    fn build_heightmaps(&self, heightmaps: &HeightmapSet) -> owned::NbtCompound {
        const PERSISTED: [(&str, HeightmapKind); 4] = [
            ("WORLD_SURFACE", HeightmapKind::WorldSurface),
            ("OCEAN_FLOOR", HeightmapKind::OceanFloor),
            ("MOTION_BLOCKING", HeightmapKind::MotionBlocking),
            (
                "MOTION_BLOCKING_NO_LEAVES",
                HeightmapKind::MotionBlockingNoLeaves,
            ),
        ];
        let mut fields: Vec<(Mutf8String, owned::NbtTag)> = Vec::with_capacity(4);
        for (name, kind) in PERSISTED {
            let mut values = [0u32; HEIGHTMAP_ENTRIES];
            for z in 0u8..16 {
                for x in 0u8..16 {
                    values[x as usize + z as usize * 16] = heightmaps.raw(kind, x, z) as u32;
                }
            }
            let words = crate::pack_bits(&values, HEIGHTMAP_BITS);
            let longs: Vec<i64> = words.iter().map(|&w| w as i64).collect();
            fields.push((name.into(), owned::NbtTag::LongArray(longs)));
        }
        owned::NbtCompound::from_values(fields)
    }

    fn read_sections(
        &self,
        tag: &borrow::NbtCompound<'_, '_>,
    ) -> Result<(BlockStateColumn, BiomeColumn, LightColumn), ChunkNbtError> {
        let sections_list = tag
            .list("sections")
            .ok_or(ChunkNbtError::MissingField("sections"))?;
        let compounds = sections_list
            .compounds()
            .ok_or(ChunkNbtError::WrongFieldType("sections"))?;

        let mut blocks = BlockStateColumn::new(BlockStateId(0), self.block_thresholds);
        let mut biomes = BiomeColumn::new(BiomeId(0), self.biome_thresholds);
        let mut light = LightColumn::new_uninitialized();
        let mut seen = [false; crate::SECTION_COUNT];

        for section in compounds {
            let y = section.byte("Y").ok_or(ChunkNbtError::MissingField("Y"))? as i32;
            if y == MIN_SECTION_Y - 1 {
                read_light_only(&section, light.section_mut(0))?;
            } else if y == MIN_SECTION_Y + crate::SECTION_COUNT as i32 {
                read_light_only(&section, light.section_mut(crate::LIGHT_SECTION_COUNT - 1))?;
            } else if (MIN_SECTION_Y..MIN_SECTION_Y + crate::SECTION_COUNT as i32).contains(&y) {
                let block_index = (y - MIN_SECTION_Y) as usize;
                *blocks.section_mut(block_index) = self.read_block_palette_section(&section)?;
                *biomes.section_mut(block_index) = self.read_biome_palette_section(&section)?;
                read_light_only(&section, light.section_mut(block_index + 1))?;
                seen[block_index] = true;
            } else {
                return Err(ChunkNbtError::SectionYOutOfRange(y));
            }
        }

        for (index, &was_seen) in seen.iter().enumerate() {
            if !was_seen {
                return Err(ChunkNbtError::MissingSection(MIN_SECTION_Y + index as i32));
            }
        }

        Ok((blocks, biomes, light))
    }

    fn read_block_palette_section(
        &self,
        section: &borrow::NbtCompound<'_, '_>,
    ) -> Result<PalettedContainer<BlockStateId>, ChunkNbtError> {
        let block_states = section
            .compound("block_states")
            .ok_or(ChunkNbtError::MissingField("block_states"))?;
        let palette_list = block_states
            .list("palette")
            .ok_or(ChunkNbtError::MissingField("palette"))?;
        let palette_compounds = palette_list
            .compounds()
            .ok_or(ChunkNbtError::WrongFieldType("palette"))?;

        let mut palette = Vec::new();
        for entry in palette_compounds {
            let name = entry
                .string("Name")
                .ok_or(ChunkNbtError::MissingField("Name"))?;
            let mut properties: Vec<(&Mutf8Str, &Mutf8Str)> = Vec::new();
            if let Some(props) = entry.compound("Properties") {
                for (key, value) in props.iter() {
                    let value_str = value
                        .string()
                        .ok_or(ChunkNbtError::WrongFieldType("Properties"))?;
                    properties.push((key, value_str));
                }
            }
            let id = self
                .block_names
                .resolve(name, &properties)
                .ok_or_else(|| ChunkNbtError::UnknownBlockStateName(name.to_str().into_owned()))?;
            palette.push(id);
        }
        if palette.is_empty() {
            return Err(ChunkNbtError::MalformedPalette(
                "block_states",
                "empty palette".to_owned(),
            ));
        }

        let data = block_states.long_array("data");
        let locals = decode_locals(
            data,
            palette.len(),
            crate::SECTION_BLOCKS as usize,
            BLOCK_FLOOR_BITS,
            "block_states",
        )?;

        let mut container =
            PalettedContainer::new_single(palette[0], crate::SECTION_BLOCKS, self.block_thresholds);
        for (index, &local) in locals.iter().enumerate() {
            container.set(index, palette[local as usize]);
        }
        Ok(container)
    }

    fn read_biome_palette_section(
        &self,
        section: &borrow::NbtCompound<'_, '_>,
    ) -> Result<PalettedContainer<BiomeId>, ChunkNbtError> {
        let biomes_compound = section
            .compound("biomes")
            .ok_or(ChunkNbtError::MissingField("biomes"))?;
        let palette_list = biomes_compound
            .list("palette")
            .ok_or(ChunkNbtError::MissingField("palette"))?;
        let names = palette_list
            .strings()
            .ok_or(ChunkNbtError::WrongFieldType("palette"))?;

        let mut palette = Vec::with_capacity(names.len());
        for &name in names {
            let id = self
                .biome_names
                .resolve(name)
                .ok_or_else(|| ChunkNbtError::UnknownBiomeName(name.to_str().into_owned()))?;
            palette.push(id);
        }
        if palette.is_empty() {
            return Err(ChunkNbtError::MalformedPalette(
                "biomes",
                "empty palette".to_owned(),
            ));
        }

        let data = biomes_compound.long_array("data");
        let locals = decode_locals(
            data,
            palette.len(),
            crate::SECTION_BIOME_CELLS as usize,
            BIOME_FLOOR_BITS,
            "biomes",
        )?;

        let mut container = PalettedContainer::new_single(
            palette[0],
            crate::SECTION_BIOME_CELLS,
            self.biome_thresholds,
        );
        for (index, &local) in locals.iter().enumerate() {
            container.set(index, palette[local as usize]);
        }
        Ok(container)
    }

    fn read_heightmaps(
        &self,
        tag: &borrow::NbtCompound<'_, '_>,
    ) -> Result<HeightmapSet, ChunkNbtError> {
        const PERSISTED: [(&str, HeightmapKind); 4] = [
            ("WORLD_SURFACE", HeightmapKind::WorldSurface),
            ("OCEAN_FLOOR", HeightmapKind::OceanFloor),
            ("MOTION_BLOCKING", HeightmapKind::MotionBlocking),
            (
                "MOTION_BLOCKING_NO_LEAVES",
                HeightmapKind::MotionBlockingNoLeaves,
            ),
        ];

        let heightmaps_compound = tag
            .compound("Heightmaps")
            .ok_or(ChunkNbtError::MissingField("Heightmaps"))?;
        let mut set = HeightmapSet::new_uniform(crate::WORLD_MIN_Y);
        let entries_per_long = (64 / HEIGHTMAP_BITS) as usize;
        let expected_longs = HEIGHTMAP_ENTRIES.div_ceil(entries_per_long);

        for (name, kind) in PERSISTED {
            let longs = heightmaps_compound
                .long_array(name)
                .ok_or(ChunkNbtError::MissingField(name))?;
            if longs.len() != expected_longs {
                return Err(ChunkNbtError::WrongFieldType(name));
            }
            let words: Vec<u64> = longs.iter().map(|&v| v as u64).collect();
            let values = crate::unpack_bits(&words, HEIGHTMAP_BITS, HEIGHTMAP_ENTRIES);
            for z in 0u8..16 {
                for x in 0u8..16 {
                    let raw = values[x as usize + z as usize * 16] as u16;
                    set.set_raw(kind, x, z, raw);
                    match kind {
                        HeightmapKind::WorldSurface => {
                            set.set_raw(HeightmapKind::WorldSurfaceWg, x, z, raw)
                        }
                        HeightmapKind::OceanFloor => {
                            set.set_raw(HeightmapKind::OceanFloorWg, x, z, raw)
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(set)
    }

    /// Decodes every `block_entities` list entry into a real `BlockEntityRecord`, in
    /// on-disk order (WORLD-D6) -- `pos` from `x`/`y`/`z`, `id` from `id`, `data` the
    /// whole entry compound via `.to_owned()`. This crate never interprets a record's
    /// own type-specific fields any further than that.
    fn read_block_entities(
        &self,
        tag: &borrow::NbtCompound<'_, '_>,
    ) -> Result<Vec<BlockEntityRecord>, ChunkNbtError> {
        let list = tag
            .list("block_entities")
            .ok_or(ChunkNbtError::MissingField("block_entities"))?;
        let Some(entries) = list.compounds() else {
            return Ok(Vec::new());
        };
        let mut records = Vec::with_capacity(entries.len());
        for entry in entries {
            let x = entry
                .int("x")
                .ok_or(ChunkNbtError::MissingField("block_entities[].x"))?;
            let y = entry
                .int("y")
                .ok_or(ChunkNbtError::MissingField("block_entities[].y"))?;
            let z = entry
                .int("z")
                .ok_or(ChunkNbtError::MissingField("block_entities[].z"))?;
            let id = entry
                .string("id")
                .ok_or(ChunkNbtError::MissingField("block_entities[].id"))?
                .to_str()
                .into_owned();
            records.push(BlockEntityRecord {
                pos: rc_core::BlockPos::new(x, y, z),
                id,
                data: entry.to_owned(),
            });
        }
        Ok(records)
    }
}

/// Every root-level field name this crate actively models or treats as a
/// fixed-default (Context's unknown-tag preservation policy) -- anything else is
/// captured opaquely into `ChunkNbtDocument::extra`.
const KNOWN_FIELDS: [&str; 10] = [
    "DataVersion",
    "xPos",
    "yPos",
    "zPos",
    "Status",
    "LastUpdate",
    "isLightOn",
    "sections",
    "block_entities",
    "Heightmaps",
];
const FIXED_DEFAULT_FIELDS: [&str; 5] = [
    "InhabitedTime",
    "structures",
    "block_ticks",
    "fluid_ticks",
    "PostProcessing",
];

fn read_extra(tag: &borrow::NbtCompound<'_, '_>) -> Vec<(Mutf8String, owned::NbtTag)> {
    let mut extra = Vec::new();
    for (name, value) in tag.iter() {
        let name_str = name.to_str();
        let as_str = name_str.as_ref();
        if KNOWN_FIELDS.contains(&as_str) || FIXED_DEFAULT_FIELDS.contains(&as_str) {
            continue;
        }
        extra.push((name.to_owned(), value.to_owned()));
    }
    extra
}

/// `BlockLight`/`SkyLight` field pairs for one section, in that fixed order -- shared by
/// every real and padding section entry `build_sections` writes.
fn light_field_pairs(section: &LightSection) -> Vec<(Mutf8String, owned::NbtTag)> {
    let mut fields = Vec::new();
    if let Some(block_light) = &section.block {
        fields.push((
            "BlockLight".into(),
            owned::NbtTag::ByteArray(block_light.to_vec()),
        ));
    }
    if let Some(sky_light) = &section.sky {
        fields.push((
            "SkyLight".into(),
            owned::NbtTag::ByteArray(sky_light.to_vec()),
        ));
    }
    fields
}

fn read_light_only(
    section: &borrow::NbtCompound<'_, '_>,
    out: &mut LightSection,
) -> Result<(), ChunkNbtError> {
    if let Some(block_light) = section.byte_array("BlockLight") {
        let array: [u8; 2048] = block_light
            .try_into()
            .map_err(|_| ChunkNbtError::WrongFieldType("BlockLight"))?;
        out.block = Some(Box::new(array));
    }
    if let Some(sky_light) = section.byte_array("SkyLight") {
        let array: [u8; 2048] = sky_light
            .try_into()
            .map_err(|_| ChunkNbtError::WrongFieldType("SkyLight"))?;
        out.sky = Some(Box::new(array));
    }
    Ok(())
}

/// The on-disk palette-derivation algorithm (Context's "Binding consequence"): always
/// re-derives the on-disk palette and packed data fresh from a `PalettedContainer`'s own
/// `iter()`, in first-encountered order, independent of whatever in-memory palette state
/// (`SingleValue`/`Indirect`/`Direct`) the container itself currently sits in.
fn disk_palette_and_data<T: Copy + Eq + std::hash::Hash>(
    values: impl Iterator<Item = T>,
    floor_bits: u32,
) -> (Vec<T>, Option<Vec<u64>>) {
    let mut palette: Vec<T> = Vec::new();
    let mut seen: HashMap<T, u32> = HashMap::new();
    let mut locals: Vec<u32> = Vec::new();
    for value in values {
        let local = match seen.get(&value) {
            Some(&index) => index,
            None => {
                let index = palette.len() as u32;
                palette.push(value);
                seen.insert(value, index);
                index
            }
        };
        locals.push(local);
    }
    if palette.len() <= 1 {
        (palette, None)
    } else {
        let bits = floor_bits.max(crate::ceil_log2(palette.len() as u32));
        (palette, Some(crate::pack_bits(&locals, bits).into_vec()))
    }
}

/// The inverse of `disk_palette_and_data`'s bit-packing half: recovers the packed local
/// index for every one of `entry_count` cells, validating that a present `data` array's
/// word count matches what `pack_bits` would have produced and that every recovered
/// local index is in range for `palette_len`.
fn decode_locals(
    data: Option<Vec<i64>>,
    palette_len: usize,
    entry_count: usize,
    floor_bits: u32,
    field: &'static str,
) -> Result<Vec<u32>, ChunkNbtError> {
    if palette_len <= 1 {
        return Ok(vec![0u32; entry_count]);
    }
    let Some(data) = data else {
        return Ok(vec![0u32; entry_count]);
    };
    let bits = floor_bits.max(crate::ceil_log2(palette_len as u32));
    let entries_per_long = (64 / bits) as usize;
    let expected_words = entry_count.div_ceil(entries_per_long);
    if data.len() != expected_words {
        return Err(ChunkNbtError::MalformedPalette(
            field,
            format!(
                "expected {expected_words} packed words at {bits} bits/entry, found {}",
                data.len()
            ),
        ));
    }
    let words: Vec<u64> = data.iter().map(|&v| v as u64).collect();
    let locals = crate::unpack_bits(&words, bits, entry_count);
    for &local in &locals {
        if local as usize >= palette_len {
            return Err(ChunkNbtError::MalformedPalette(
                field,
                format!("local index {local} out of range for palette length {palette_len}"),
            ));
        }
    }
    Ok(locals)
}
