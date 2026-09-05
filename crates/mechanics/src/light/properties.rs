//! Block-state light properties and the shape-occlusion veto (M4-B07 Context §3).
//! Simplified relative to vanilla's full geometric shape model -- `occludes_face` is a
//! per-direction boolean veto, not a `VoxelShape` union test -- since no shape/registry
//! source exists yet (mirrors M3-B01's `BlockBehaviorRegistry` "no generated registry"
//! resolution).

use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;

use crate::direction::Direction;

/// One block-state's light-relevant properties (Context §3).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LightProperties {
    pub block_emission: u8,
    pub opacity: u8,
    pub occludes_face: [bool; 6],
}

impl LightProperties {
    /// Fully transparent, non-emitting (the default for any unregistered state --
    /// matches vanilla's own air convention).
    pub const AIR: LightProperties = LightProperties {
        block_emission: 0,
        opacity: 0,
        occludes_face: [false; 6],
    };
    /// Fully solid, opaque, non-emitting (opacity 15, no shape veto needed since
    /// scalar opacity alone already blocks everything).
    pub const OPAQUE: LightProperties = LightProperties {
        block_emission: 0,
        opacity: 15,
        occludes_face: [false; 6],
    };

    /// `opacity.max(1)` -- `MIN_OPACITY` floor (Context §2/§3).
    pub fn get_opacity(self) -> u8 {
        self.opacity.max(1)
    }
}

/// `Direction`'s own declaration order, restated as a plain index function (this
/// crate does not add an `ordinal`/index method to `rc_mechanics::direction::
/// Direction` itself -- Constraints (d)). West=0, East=1, North=2, South=3, Down=4,
/// Up=5 -- the sole place a numeric index is derived from a `Direction` value;
/// every other file that needs one calls this function, never re-deriving its own
/// mapping.
pub fn direction_index(dir: Direction) -> usize {
    match dir {
        Direction::West => 0,
        Direction::East => 1,
        Direction::North => 2,
        Direction::South => 3,
        Direction::Down => 4,
        Direction::Up => 5,
    }
}

/// `true` iff `from_props`'s face in `dir`, or `to_props`'s face in `dir.opposite()`,
/// is declared to fully occlude the shared face (Context §3's veto formula).
pub fn shape_occludes(
    from_props: LightProperties,
    to_props: LightProperties,
    dir: Direction,
) -> bool {
    from_props.occludes_face[direction_index(dir)]
        || to_props.occludes_face[direction_index(dir.opposite())]
}

/// Range-based dispatch (mirrors `crate::behavior::BlockBehaviorRegistry` exactly --
/// M3-B01's own established pattern for "no generated registry yet"). `Resource`
/// (M4-B07 field-report note, not shown in the blueprint's own Deliverables snippet
/// but required for Context §8's own "reads `LightPropertiesRegistry`... as
/// Resources" `run_stage8_lighting` contract to actually compile/insert into a real
/// `bevy_ecs::World` -- recorded in `docs/findings-for-planning.md`).
#[derive(Clone, Default, Resource)]
pub struct LightPropertiesRegistry {
    ranges: Vec<(BlockStateId, BlockStateId, LightProperties)>,
}

impl LightPropertiesRegistry {
    pub fn new() -> Self {
        Self { ranges: Vec::new() }
    }

    /// Panics on overlap with an already-registered range (mirrors
    /// `BlockBehaviorRegistry::register_range` exactly).
    pub fn register_range(
        &mut self,
        start: BlockStateId,
        end_exclusive: BlockStateId,
        props: LightProperties,
    ) {
        let overlaps = self
            .ranges
            .iter()
            .any(|(s, e, _)| start < *e && *s < end_exclusive);
        assert!(
            !overlaps,
            "LightPropertiesRegistry::register_range: [{start:?}, {end_exclusive:?}) overlaps an already-registered range"
        );
        self.ranges.push((start, end_exclusive, props));
        self.ranges.sort_by_key(|(start, _, _)| *start);
    }

    pub fn register_one(&mut self, state: BlockStateId, props: LightProperties) {
        self.register_range(state, BlockStateId(state.0 + 1), props);
    }

    /// Binary-searches `self.ranges` (kept sorted by `start` -- `register_range`'s own
    /// post-push `sort_by_key`) for the sole range that could possibly contain `state`:
    /// the last range whose own `start` is `<= state` (`partition_point`'s own "count of
    /// elements before the first one failing the predicate" contract, applied to the
    /// predicate "starts at or before `state`"). `production_registry`'s own ~1300+
    /// registered ranges (many single-state entries for `lit`/`candles`/`pickles`/...
    /// per-property emitters) made the former linear scan here the dominant per-tick cost
    /// once Stage 8 actually resolves millions of block states a tick against a real,
    /// fully-populated registry (M4-B07 field-report implementation's own composition-root
    /// wiring change -- found by exactly that wiring, `docs/findings-for-planning.md`'s own
    /// entry on this changeset has the full writeup) -- this is a pure internal-lookup-
    /// strategy change, `resolve`'s own public contract (return value for every input) is
    /// unchanged.
    fn find_index(&self, state: BlockStateId) -> Option<usize> {
        let candidate = self.ranges.partition_point(|(start, _, _)| *start <= state);
        if candidate == 0 {
            return None;
        }
        let index = candidate - 1;
        let (_, end_exclusive, _) = self.ranges[index];
        (state < end_exclusive).then_some(index)
    }

    /// Returns the matching range's properties, or `LightProperties::AIR`.
    pub fn resolve(&self, state: BlockStateId) -> LightProperties {
        match self.find_index(state) {
            Some(index) => self.ranges[index].2,
            None => LightProperties::AIR,
        }
    }

    /// `true` iff `state` falls inside a range this registry actually registered (as
    /// opposed to `resolve`'s own `LightProperties::AIR` fallback for an unregistered id,
    /// which is bit-identical to a deliberately-registered fully-transparent range and so
    /// cannot be told apart from `resolve`'s own return value alone) -- `production_
    /// registry`'s own acceptance test uses this to verify every one of the generated
    /// registry's 1196 blocks was actually classified, not merely defaulted.
    pub fn is_registered(&self, state: BlockStateId) -> bool {
        self.find_index(state).is_some()
    }
}

// ---------------------------------------------------------------------------------------
// M4-B07 field-report implementation: the production `LightPropertiesRegistry` composition
// root's own light-wiring changeset requires (`docs/findings-for-planning.md`'s own entry
// on this changeset has the full writeup). Covers every one of the generated registry's
// 1196 blocks (`rc_registries::generated_v776::block_state_properties::BLOCK_RANGES`) --
// every range not explicitly classified below resolves to `LightProperties::OPAQUE`
// (opacity 15, no emission), correct for the overwhelming majority (ordinary terrain,
// ores, logs/planks/wood-as-blocks, wool/concrete/terracotta/glazed-terracotta, and every
// other "ordinary furniture" block whose real vanilla shape is a full cube).
//
// Opacity classification method (verified against the ASSET-D18(f) reference,
// `net/minecraft/world/level/block/state/BlockBehaviour.java`'s own `getLightDampening`
// default body and `net/minecraft/world/level/lighting/LightEngine.java`'s own
// `isEmptyShape`/`getLightDampeningInto`): vanilla's real default is `state.isSolidRender()
// ? 15 : (state.propagatesSkylightDown() ? 0 : 1)`, where `isSolidRender()` asks whether
// the block's own occlusion shape -- empty whenever `canOcclude` is false, i.e. whenever
// `BlockBehaviour.Properties.noCollision()`/`.noOcclusion()` was called -- is itself a full
// unit cube, and `propagatesSkylightDown()`'s own default is `!isShapeFullBlock(rawShape) &&
// fluidState.isEmpty()`. Every non-full-shape block below (redstone components, plants,
// stairs/slabs/fences/walls/doors/trapdoors/signs/banners/beds/carpets/candles/heads,
// torches, rails, and the many small "furniture" blocks whose real hitbox is not a full
// cube) therefore resolves to opacity 0 by this same rule (`propagatesSkylightDown` true
// off a non-full raw shape). A small, explicitly enumerated exception set -- glass/stained
// glass, ice/packed ice/blue ice/frosted ice, leaves, slime block, honey block, water, lava
// -- is full-shaped in vanilla but still marked `.noOcclusion()` (or, for water/lava, has a
// non-empty fluid state, the other half of `propagatesSkylightDown`'s own test): both halves
// of vanilla's own test then land on opacity 1, not 0 -- functionally identical to opacity 0
// once `LightProperties::get_opacity`'s own `MIN_OPACITY = 1` floor applies (every ordinary
// BFS hop already costs at least 1 regardless), but distinct for `is_sky_edge_occluded`'s own
// un-floored `opacity != 0` boundary test (M4-B07 Context §6) -- reproducing this handful of
// cases exactly is what makes a glass-roofed room's own sky-source boundary sit at the glass
// itself rather than one block above it, matching vanilla. `TINTED_GLASS` is its own explicit
// override back to opacity 15 (`TintedGlassBlock.getLightDampening`'s own unconditional `15`
// override in the reference -- deliberately blocks light despite looking similar to glass).
//
// Emission table (every value read from the reference's own `Blocks.java` `.lightLevel(...)`
// lambdas, cited per group below; state-dependent lambdas -- `lit`/`candles`/`pickles`/
// `berries`/`charges`/`level`/`trial_spawner_state`/`vault_state` -- are reproduced by
// resolving each real generated state's own property list, never by a second per-block
// lambda mechanism of this crate's own).
// ---------------------------------------------------------------------------------------

use rc_registries::block_state_properties::{properties as generated_state_properties, range_of};
use rc_registries::generated_v776::block_state_properties::{BlockId, block_id};
use rc_registries::generated_v776::block_states::BlockStateId as GeneratedStateId;

const TRANSPARENT: LightProperties = LightProperties {
    block_emission: 0,
    opacity: 0,
    occludes_face: [false; 6],
};

/// Vanilla's own "full-shaped but `.noOcclusion()`-marked (or a non-empty fluid state)"
/// exception (module doc comment above) -- opacity 1, not 0; `get_opacity()`'s `MIN_OPACITY`
/// floor makes the two indistinguishable for ordinary propagation, but `is_sky_edge_occluded`
/// reads the un-floored value directly.
const NOOCCLUSION_FULL_SHAPE: LightProperties = LightProperties {
    block_emission: 0,
    opacity: 1,
    occludes_face: [false; 6],
};

/// Registers `block`'s entire generated id range with uniform `props`, and marks it as
/// handled in `covered` (this module's own duplicate-registration guard -- every block id is
/// classified by exactly one of this function's own callers, `register_per_state`'s, or the
/// final default-fill pass below, never more than one).
fn register_full_range(
    reg: &mut LightPropertiesRegistry,
    covered: &mut [bool],
    block: BlockId,
    props: LightProperties,
) {
    let range = range_of(block);
    reg.register_range(
        BlockStateId(range.first.0),
        BlockStateId(range.last.0 + 1),
        props,
    );
    covered[block.0 as usize] = true;
}

/// As `register_full_range`, for every block in `blocks`.
fn register_full_range_many(
    reg: &mut LightPropertiesRegistry,
    covered: &mut [bool],
    blocks: &[BlockId],
    props: LightProperties,
) {
    for &b in blocks {
        register_full_range(reg, covered, b, props);
    }
}

/// `props`'s own value for the property named `name` -- panics if `props` (a resolved real
/// generated state's own property list) does not carry it, a config-time bug (every call
/// site below names a property the resolved block-state range is already known to carry).
fn prop_value<'a>(props: &'a [(&'a str, &'a str)], name: &str) -> &'a str {
    props
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
        .unwrap_or_else(|| panic!("production_registry: state missing property {name:?}"))
}

fn prop_bool(props: &[(&str, &str)], name: &str) -> bool {
    prop_value(props, name) == "true"
}

fn prop_u8(props: &[(&str, &str)], name: &str) -> u8 {
    let raw = prop_value(props, name);
    raw.parse()
        .unwrap_or_else(|_| panic!("production_registry: property {name:?} = {raw:?} is not a u8"))
}

/// Registers `block`'s entire generated id range one real state at a time, with a uniform
/// `opacity` (state-dependent opacity does not occur anywhere in this registry) and each
/// state's own `emission_of`-computed emission (reading that state's own resolved property
/// list, `rc_registries::block_state_properties::properties`).
fn register_per_state(
    reg: &mut LightPropertiesRegistry,
    covered: &mut [bool],
    block: BlockId,
    opacity: u8,
    emission_of: impl Fn(&[(&str, &str)]) -> u8,
) {
    let range = range_of(block);
    for raw in range.first.0..=range.last.0 {
        let props = generated_state_properties(GeneratedStateId(raw));
        let emission = emission_of(props);
        reg.register_one(
            BlockStateId(raw),
            LightProperties {
                block_emission: emission,
                opacity,
                occludes_face: [false; 6],
            },
        );
    }
    covered[block.0 as usize] = true;
}

/// `litBlockEmission(level)`'s own restatement (`Blocks.java`, `private static
/// ToIntFunction<BlockState> litBlockEmission`): `level` while `lit == true`, `0` otherwise.
fn lit_emission(props: &[(&str, &str)], level: u8) -> u8 {
    if prop_bool(props, "lit") { level } else { 0 }
}

/// The complete production `LightPropertiesRegistry` (M4-B07 field-report implementation).
pub fn production_registry() -> LightPropertiesRegistry {
    let block_count = rc_registries::generated_v776::block_state_properties::BLOCK_RANGES.len();
    let mut reg = LightPropertiesRegistry::new();
    let mut covered = vec![false; block_count];

    // --- Per-state dynamic emitters (opacity 0: non-full shape) ---------------------------
    // SEA_PICKLE: `SeaPickleBlock`'s own `isDead` = `!waterlogged`; emission = `isDead ? 0 :
    // 3 + 3 * pickles` (`Blocks.java` line ~4219).
    register_per_state(&mut reg, &mut covered, block_id::SEA_PICKLE, 0, |p| {
        if prop_bool(p, "waterlogged") {
            3 + 3 * prop_u8(p, "pickles")
        } else {
            0
        }
    });
    // CAVE_VINES/CAVE_VINES_PLANT: `CaveVines.emission(14)` -- `berries ? 14 : 0`
    // (`Blocks.java` lines ~5367/~5378).
    for &block in &[block_id::CAVE_VINES, block_id::CAVE_VINES_PLANT] {
        register_per_state(&mut reg, &mut covered, block, 0, |p| {
            if prop_bool(p, "berries") { 14 } else { 0 }
        });
    }
    // LIGHT: `LightBlock.LIGHT_EMISSION = state -> state.getValue(LEVEL)` (`Blocks.java` line
    // ~2949) -- opacity 0 (this crate's own deliberate choice, module doc comment above: the
    // block's entire purpose is to pass light through, unlike an ordinary solid).
    register_per_state(&mut reg, &mut covered, block_id::LIGHT, 0, |p| {
        prop_u8(p, "level")
    });
    // CANDLE (+ 16 dyed variants): `CandleBlock.LIGHT_EMISSION = lit ? 3 * candles : 0`
    // (`CandleBlock.java` line ~43).
    let candles: &[BlockId] = &[
        block_id::BLACK_CANDLE,
        block_id::BLUE_CANDLE,
        block_id::BROWN_CANDLE,
        block_id::CANDLE,
        block_id::CYAN_CANDLE,
        block_id::GRAY_CANDLE,
        block_id::GREEN_CANDLE,
        block_id::LIGHT_BLUE_CANDLE,
        block_id::LIGHT_GRAY_CANDLE,
        block_id::LIME_CANDLE,
        block_id::MAGENTA_CANDLE,
        block_id::ORANGE_CANDLE,
        block_id::PINK_CANDLE,
        block_id::PURPLE_CANDLE,
        block_id::RED_CANDLE,
        block_id::WHITE_CANDLE,
        block_id::YELLOW_CANDLE,
    ];
    for &block in candles {
        register_per_state(&mut reg, &mut covered, block, 0, |p| {
            if prop_bool(p, "lit") {
                3 * prop_u8(p, "candles")
            } else {
                0
            }
        });
    }
    // CANDLE_CAKE (+ 16 dyed variants): `litBlockEmission(3)` (`Blocks.java` line ~4970).
    let candle_cakes: &[BlockId] = &[
        block_id::BLACK_CANDLE_CAKE,
        block_id::BLUE_CANDLE_CAKE,
        block_id::BROWN_CANDLE_CAKE,
        block_id::CANDLE_CAKE,
        block_id::CYAN_CANDLE_CAKE,
        block_id::GRAY_CANDLE_CAKE,
        block_id::GREEN_CANDLE_CAKE,
        block_id::LIGHT_BLUE_CANDLE_CAKE,
        block_id::LIGHT_GRAY_CANDLE_CAKE,
        block_id::LIME_CANDLE_CAKE,
        block_id::MAGENTA_CANDLE_CAKE,
        block_id::ORANGE_CANDLE_CAKE,
        block_id::PINK_CANDLE_CAKE,
        block_id::PURPLE_CANDLE_CAKE,
        block_id::RED_CANDLE_CAKE,
        block_id::WHITE_CANDLE_CAKE,
        block_id::YELLOW_CANDLE_CAKE,
    ];
    for &block in candle_cakes {
        register_per_state(&mut reg, &mut covered, block, 0, |p| lit_emission(p, 3));
    }
    // REDSTONE_TORCH/REDSTONE_WALL_TORCH: `litBlockEmission(7)` (`Blocks.java` lines
    // ~1923/~1928).
    for &block in &[block_id::REDSTONE_TORCH, block_id::REDSTONE_WALL_TORCH] {
        register_per_state(&mut reg, &mut covered, block, 0, |p| lit_emission(p, 7));
    }
    // CAMPFIRE: `litBlockEmission(15)`, SOUL_CAMPFIRE: `litBlockEmission(10)` (`Blocks.java`
    // lines ~4498/~4510).
    register_per_state(&mut reg, &mut covered, block_id::CAMPFIRE, 0, |p| {
        lit_emission(p, 15)
    });
    register_per_state(&mut reg, &mut covered, block_id::SOUL_CAMPFIRE, 0, |p| {
        lit_emission(p, 10)
    });

    // --- Per-state dynamic emitters (opacity 15: full-cube shape) --------------------------
    // FURNACE/SMOKER/BLAST_FURNACE: `litBlockEmission(13)` (`Blocks.java` lines
    // ~1311/~4401/~4411).
    for &block in &[block_id::FURNACE, block_id::SMOKER, block_id::BLAST_FURNACE] {
        register_per_state(&mut reg, &mut covered, block, 15, |p| lit_emission(p, 13));
    }
    // REDSTONE_LAMP: `litBlockEmission(15)` (`Blocks.java` line ~2602).
    register_per_state(&mut reg, &mut covered, block_id::REDSTONE_LAMP, 15, |p| {
        lit_emission(p, 15)
    });
    // REDSTONE_ORE/DEEPSLATE_REDSTONE_ORE: `litBlockEmission(9)` (`Blocks.java` line ~1912;
    // `DEEPSLATE_REDSTONE_ORE` shares `REDSTONE_ORE`'s own `Properties`, `Blocks.java` line
    // ~1915).
    for &block in &[block_id::REDSTONE_ORE, block_id::DEEPSLATE_REDSTONE_ORE] {
        register_per_state(&mut reg, &mut covered, block, 15, |p| lit_emission(p, 9));
    }
    // RESPAWN_ANCHOR: `RespawnAnchorBlock.getScaledChargeLevel(state, 15) = floor(charges /
    // 4.0 * 15)` (`RespawnAnchorBlock.java` lines ~214/~220; `Blocks.java` line ~4874).
    register_per_state(&mut reg, &mut covered, block_id::RESPAWN_ANCHOR, 15, |p| {
        let charges = prop_u8(p, "charges") as u32;
        ((charges * 15) / 4) as u8
    });
    // TRIAL_SPAWNER: `TrialSpawnerState.lightLevel()` -- `inactive`/`cooldown` 0,
    // `waiting_for_players` 4, `active`/`waiting_for_reward_ejection`/`ejecting_reward` 8
    // (`TrialSpawnerState.java` lines ~34-39; `Blocks.java` line ~5610).
    register_per_state(&mut reg, &mut covered, block_id::TRIAL_SPAWNER, 15, |p| {
        match prop_value(p, "trial_spawner_state") {
            "waiting_for_players" => 4,
            "active" | "waiting_for_reward_ejection" | "ejecting_reward" => 8,
            _ => 0, // "inactive" | "cooldown"
        }
    });
    // VAULT: `VaultState.lightLevel()` -- `inactive` (`HALF_LIT`) 6, every other state
    // (`LIT`) 12 (`VaultState.java` lines ~14-23/~144-146; `Blocks.java` line ~5624).
    register_per_state(&mut reg, &mut covered, block_id::VAULT, 15, |p| {
        match prop_value(p, "vault_state") {
            "inactive" => 6,
            _ => 12, // "active" | "unlocking" | "ejecting"
        }
    });
    // COPPER_BULB weathering tiers: `litBlockEmission(switch(p) { UNAFFECTED -> 15; EXPOSED
    // -> 12; WEATHERED -> 8; OXIDIZED -> 4 })` (`Blocks.java` lines ~5271-5277); waxed
    // variants share their own unwaxed tier's value (waxing only stops further weathering,
    // it never changes the current tier).
    let copper_bulb_tiers: &[(BlockId, u8)] = &[
        (block_id::COPPER_BULB, 15),
        (block_id::WAXED_COPPER_BULB, 15),
        (block_id::EXPOSED_COPPER_BULB, 12),
        (block_id::WAXED_EXPOSED_COPPER_BULB, 12),
        (block_id::WEATHERED_COPPER_BULB, 8),
        (block_id::WAXED_WEATHERED_COPPER_BULB, 8),
        (block_id::OXIDIZED_COPPER_BULB, 4),
        (block_id::WAXED_OXIDIZED_COPPER_BULB, 4),
    ];
    for &(block, level) in copper_bulb_tiers {
        register_per_state(&mut reg, &mut covered, block, 15, move |p| {
            lit_emission(p, level)
        });
    }

    // --- Uniform emitters (opacity 0: non-full shape) --------------------------------------
    let uniform_nonfull_emitters: &[(BlockId, u8)] = &[
        (block_id::TORCH, 14),
        (block_id::WALL_TORCH, 14),
        (block_id::SOUL_TORCH, 10),
        (block_id::SOUL_WALL_TORCH, 10),
        (block_id::COPPER_TORCH, 14),
        (block_id::COPPER_WALL_TORCH, 14),
        (block_id::LANTERN, 15),
        (block_id::SOUL_LANTERN, 10),
        (block_id::END_ROD, 14),
        (block_id::FIRE, 15),
        (block_id::SOUL_FIRE, 10),
        (block_id::AMETHYST_CLUSTER, 5),
        (block_id::LARGE_AMETHYST_BUD, 4),
        (block_id::MEDIUM_AMETHYST_BUD, 2),
        (block_id::SMALL_AMETHYST_BUD, 1),
        (block_id::BREWING_STAND, 1),
        (block_id::ENCHANTING_TABLE, 7),
        (block_id::ENDER_CHEST, 7),
        (block_id::END_PORTAL_FRAME, 1),
        (block_id::DRAGON_EGG, 1),
        (block_id::SCULK_SENSOR, 1),
        // `GlowLichenBlock.emission(7)`'s own gate, `MultifaceBlock.hasAnyFace(state)`, is
        // `true` for every state this project's placement paths can actually produce (a
        // glow-lichen block with zero faces set is not a reachable placed state) -- treated
        // as an unconditional 7, a documented simplification (`docs/findings-for-planning.md`).
        (block_id::GLOW_LICHEN, 7),
        (block_id::FIREFLY_BUSH, 2),
        (block_id::LAVA_CAULDRON, 15),
        (block_id::END_PORTAL, 15),
        (block_id::END_GATEWAY, 15),
        (block_id::NETHER_PORTAL, 11),
    ];
    for &(block, level) in uniform_nonfull_emitters {
        register_full_range(
            &mut reg,
            &mut covered,
            block,
            LightProperties {
                block_emission: level,
                opacity: 0,
                occludes_face: [false; 6],
            },
        );
    }

    // --- Uniform emitters (opacity 15: full-cube shape) ------------------------------------
    let uniform_full_emitters: &[(BlockId, u8)] = &[
        (block_id::GLOWSTONE, 15),
        (block_id::SEA_LANTERN, 15),
        (block_id::SHROOMLIGHT, 15),
        (block_id::BEACON, 15),
        (block_id::JACK_O_LANTERN, 15),
        (block_id::MAGMA_BLOCK, 3),
        (block_id::CRYING_OBSIDIAN, 10),
        (block_id::OCHRE_FROGLIGHT, 15),
        (block_id::VERDANT_FROGLIGHT, 15),
        (block_id::PEARLESCENT_FROGLIGHT, 15),
        (block_id::SCULK_CATALYST, 6),
    ];
    for &(block, level) in uniform_full_emitters {
        register_full_range(
            &mut reg,
            &mut covered,
            block,
            LightProperties {
                block_emission: level,
                opacity: 15,
                occludes_face: [false; 6],
            },
        );
    }

    // --- Non-full-shape, non-emitting blocks (opacity 0) -----------------------------------
    // Suffix-pattern families: stairs/slabs/walls/fences/fence-gates/doors/trapdoors/
    // pressure-plates/buttons/signs (standing/wall/hanging/wall-hanging)/banners (+wall)/
    // beds/carpets/heads+skulls (+wall). Candle/candle-cake variants are emitters, handled
    // above.
    register_full_range_many(
        &mut reg,
        &mut covered,
        &[
            block_id::ACACIA_BUTTON,
            block_id::ACACIA_DOOR,
            block_id::ACACIA_FENCE,
            block_id::ACACIA_FENCE_GATE,
            block_id::ACACIA_HANGING_SIGN,
            block_id::ACACIA_PRESSURE_PLATE,
            block_id::ACACIA_SIGN,
            block_id::ACACIA_SLAB,
            block_id::ACACIA_STAIRS,
            block_id::ACACIA_TRAPDOOR,
            block_id::ACACIA_WALL_HANGING_SIGN,
            block_id::ACACIA_WALL_SIGN,
            block_id::ANDESITE_SLAB,
            block_id::ANDESITE_STAIRS,
            block_id::ANDESITE_WALL,
            block_id::BAMBOO_BUTTON,
            block_id::BAMBOO_DOOR,
            block_id::BAMBOO_FENCE,
            block_id::BAMBOO_FENCE_GATE,
            block_id::BAMBOO_HANGING_SIGN,
            block_id::BAMBOO_MOSAIC_SLAB,
            block_id::BAMBOO_MOSAIC_STAIRS,
            block_id::BAMBOO_PRESSURE_PLATE,
            block_id::BAMBOO_SIGN,
            block_id::BAMBOO_SLAB,
            block_id::BAMBOO_STAIRS,
            block_id::BAMBOO_TRAPDOOR,
            block_id::BAMBOO_WALL_HANGING_SIGN,
            block_id::BAMBOO_WALL_SIGN,
            block_id::BIRCH_BUTTON,
            block_id::BIRCH_DOOR,
            block_id::BIRCH_FENCE,
            block_id::BIRCH_FENCE_GATE,
            block_id::BIRCH_HANGING_SIGN,
            block_id::BIRCH_PRESSURE_PLATE,
            block_id::BIRCH_SIGN,
            block_id::BIRCH_SLAB,
            block_id::BIRCH_STAIRS,
            block_id::BIRCH_TRAPDOOR,
            block_id::BIRCH_WALL_HANGING_SIGN,
            block_id::BIRCH_WALL_SIGN,
            block_id::BLACKSTONE_SLAB,
            block_id::BLACKSTONE_STAIRS,
            block_id::BLACKSTONE_WALL,
            block_id::BLACK_BANNER,
            block_id::BLACK_BED,
            block_id::BLACK_CARPET,
            block_id::BLACK_WALL_BANNER,
            block_id::BLUE_BANNER,
            block_id::BLUE_BED,
            block_id::BLUE_CARPET,
            block_id::BLUE_WALL_BANNER,
            block_id::BRICK_SLAB,
            block_id::BRICK_STAIRS,
            block_id::BRICK_WALL,
            block_id::BROWN_BANNER,
            block_id::BROWN_BED,
            block_id::BROWN_CARPET,
            block_id::BROWN_WALL_BANNER,
            block_id::CHERRY_BUTTON,
            block_id::CHERRY_DOOR,
            block_id::CHERRY_FENCE,
            block_id::CHERRY_FENCE_GATE,
            block_id::CHERRY_HANGING_SIGN,
            block_id::CHERRY_PRESSURE_PLATE,
            block_id::CHERRY_SIGN,
            block_id::CHERRY_SLAB,
            block_id::CHERRY_STAIRS,
            block_id::CHERRY_TRAPDOOR,
            block_id::CHERRY_WALL_HANGING_SIGN,
            block_id::CHERRY_WALL_SIGN,
            block_id::CINNABAR_BRICK_SLAB,
            block_id::CINNABAR_BRICK_STAIRS,
            block_id::CINNABAR_BRICK_WALL,
            block_id::CINNABAR_SLAB,
            block_id::CINNABAR_STAIRS,
            block_id::CINNABAR_WALL,
            block_id::COBBLED_DEEPSLATE_SLAB,
            block_id::COBBLED_DEEPSLATE_STAIRS,
            block_id::COBBLED_DEEPSLATE_WALL,
            block_id::COBBLESTONE_SLAB,
            block_id::COBBLESTONE_STAIRS,
            block_id::COBBLESTONE_WALL,
            block_id::COPPER_DOOR,
            block_id::COPPER_TRAPDOOR,
            block_id::CREEPER_HEAD,
            block_id::CREEPER_WALL_HEAD,
            block_id::CRIMSON_BUTTON,
            block_id::CRIMSON_DOOR,
            block_id::CRIMSON_FENCE,
            block_id::CRIMSON_FENCE_GATE,
            block_id::CRIMSON_HANGING_SIGN,
            block_id::CRIMSON_PRESSURE_PLATE,
            block_id::CRIMSON_SIGN,
            block_id::CRIMSON_SLAB,
            block_id::CRIMSON_STAIRS,
            block_id::CRIMSON_TRAPDOOR,
            block_id::CRIMSON_WALL_HANGING_SIGN,
            block_id::CRIMSON_WALL_SIGN,
            block_id::CUT_COPPER_SLAB,
            block_id::CUT_COPPER_STAIRS,
            block_id::CUT_RED_SANDSTONE_SLAB,
            block_id::CUT_SANDSTONE_SLAB,
            block_id::CYAN_BANNER,
            block_id::CYAN_BED,
            block_id::CYAN_CARPET,
            block_id::CYAN_WALL_BANNER,
            block_id::DARK_OAK_BUTTON,
            block_id::DARK_OAK_DOOR,
            block_id::DARK_OAK_FENCE,
            block_id::DARK_OAK_FENCE_GATE,
            block_id::DARK_OAK_HANGING_SIGN,
            block_id::DARK_OAK_PRESSURE_PLATE,
            block_id::DARK_OAK_SIGN,
            block_id::DARK_OAK_SLAB,
            block_id::DARK_OAK_STAIRS,
            block_id::DARK_OAK_TRAPDOOR,
            block_id::DARK_OAK_WALL_HANGING_SIGN,
            block_id::DARK_OAK_WALL_SIGN,
            block_id::DARK_PRISMARINE_SLAB,
            block_id::DARK_PRISMARINE_STAIRS,
            block_id::DEEPSLATE_BRICK_SLAB,
            block_id::DEEPSLATE_BRICK_STAIRS,
            block_id::DEEPSLATE_BRICK_WALL,
            block_id::DEEPSLATE_TILE_SLAB,
            block_id::DEEPSLATE_TILE_STAIRS,
            block_id::DEEPSLATE_TILE_WALL,
            block_id::DIORITE_SLAB,
            block_id::DIORITE_STAIRS,
            block_id::DIORITE_WALL,
            block_id::DRAGON_HEAD,
            block_id::DRAGON_WALL_HEAD,
            block_id::END_STONE_BRICK_SLAB,
            block_id::END_STONE_BRICK_STAIRS,
            block_id::END_STONE_BRICK_WALL,
            block_id::EXPOSED_COPPER_DOOR,
            block_id::EXPOSED_COPPER_TRAPDOOR,
            block_id::EXPOSED_CUT_COPPER_SLAB,
            block_id::EXPOSED_CUT_COPPER_STAIRS,
            block_id::GRANITE_SLAB,
            block_id::GRANITE_STAIRS,
            block_id::GRANITE_WALL,
            block_id::GRAY_BANNER,
            block_id::GRAY_BED,
            block_id::GRAY_CARPET,
            block_id::GRAY_WALL_BANNER,
            block_id::GREEN_BANNER,
            block_id::GREEN_BED,
            block_id::GREEN_CARPET,
            block_id::GREEN_WALL_BANNER,
            block_id::HEAVY_WEIGHTED_PRESSURE_PLATE,
            block_id::IRON_DOOR,
            block_id::IRON_TRAPDOOR,
            block_id::JUNGLE_BUTTON,
            block_id::JUNGLE_DOOR,
            block_id::JUNGLE_FENCE,
            block_id::JUNGLE_FENCE_GATE,
            block_id::JUNGLE_HANGING_SIGN,
            block_id::JUNGLE_PRESSURE_PLATE,
            block_id::JUNGLE_SIGN,
            block_id::JUNGLE_SLAB,
            block_id::JUNGLE_STAIRS,
            block_id::JUNGLE_TRAPDOOR,
            block_id::JUNGLE_WALL_HANGING_SIGN,
            block_id::JUNGLE_WALL_SIGN,
            block_id::LIGHT_BLUE_BANNER,
            block_id::LIGHT_BLUE_BED,
            block_id::LIGHT_BLUE_CARPET,
            block_id::LIGHT_BLUE_WALL_BANNER,
            block_id::LIGHT_GRAY_BANNER,
            block_id::LIGHT_GRAY_BED,
            block_id::LIGHT_GRAY_CARPET,
            block_id::LIGHT_GRAY_WALL_BANNER,
            block_id::LIGHT_WEIGHTED_PRESSURE_PLATE,
            block_id::LIME_BANNER,
            block_id::LIME_BED,
            block_id::LIME_CARPET,
            block_id::LIME_WALL_BANNER,
            block_id::MAGENTA_BANNER,
            block_id::MAGENTA_BED,
            block_id::MAGENTA_CARPET,
            block_id::MAGENTA_WALL_BANNER,
            block_id::MANGROVE_BUTTON,
            block_id::MANGROVE_DOOR,
            block_id::MANGROVE_FENCE,
            block_id::MANGROVE_FENCE_GATE,
            block_id::MANGROVE_HANGING_SIGN,
            block_id::MANGROVE_PRESSURE_PLATE,
            block_id::MANGROVE_SIGN,
            block_id::MANGROVE_SLAB,
            block_id::MANGROVE_STAIRS,
            block_id::MANGROVE_TRAPDOOR,
            block_id::MANGROVE_WALL_HANGING_SIGN,
            block_id::MANGROVE_WALL_SIGN,
            block_id::MOSSY_COBBLESTONE_SLAB,
            block_id::MOSSY_COBBLESTONE_STAIRS,
            block_id::MOSSY_COBBLESTONE_WALL,
            block_id::MOSSY_STONE_BRICK_SLAB,
            block_id::MOSSY_STONE_BRICK_STAIRS,
            block_id::MOSSY_STONE_BRICK_WALL,
            block_id::MOSS_CARPET,
            block_id::MUD_BRICK_SLAB,
            block_id::MUD_BRICK_STAIRS,
            block_id::MUD_BRICK_WALL,
            block_id::NETHER_BRICK_FENCE,
            block_id::NETHER_BRICK_SLAB,
            block_id::NETHER_BRICK_STAIRS,
            block_id::NETHER_BRICK_WALL,
            block_id::OAK_BUTTON,
            block_id::OAK_DOOR,
            block_id::OAK_FENCE,
            block_id::OAK_FENCE_GATE,
            block_id::OAK_HANGING_SIGN,
            block_id::OAK_PRESSURE_PLATE,
            block_id::OAK_SIGN,
            block_id::OAK_SLAB,
            block_id::OAK_STAIRS,
            block_id::OAK_TRAPDOOR,
            block_id::OAK_WALL_HANGING_SIGN,
            block_id::OAK_WALL_SIGN,
            block_id::ORANGE_BANNER,
            block_id::ORANGE_BED,
            block_id::ORANGE_CARPET,
            block_id::ORANGE_WALL_BANNER,
            block_id::OXIDIZED_COPPER_DOOR,
            block_id::OXIDIZED_COPPER_TRAPDOOR,
            block_id::OXIDIZED_CUT_COPPER_SLAB,
            block_id::OXIDIZED_CUT_COPPER_STAIRS,
            block_id::PALE_MOSS_CARPET,
            block_id::PALE_OAK_BUTTON,
            block_id::PALE_OAK_DOOR,
            block_id::PALE_OAK_FENCE,
            block_id::PALE_OAK_FENCE_GATE,
            block_id::PALE_OAK_HANGING_SIGN,
            block_id::PALE_OAK_PRESSURE_PLATE,
            block_id::PALE_OAK_SIGN,
            block_id::PALE_OAK_SLAB,
            block_id::PALE_OAK_STAIRS,
            block_id::PALE_OAK_TRAPDOOR,
            block_id::PALE_OAK_WALL_HANGING_SIGN,
            block_id::PALE_OAK_WALL_SIGN,
            block_id::PETRIFIED_OAK_SLAB,
            block_id::PIGLIN_HEAD,
            block_id::PIGLIN_WALL_HEAD,
            block_id::PINK_BANNER,
            block_id::PINK_BED,
            block_id::PINK_CARPET,
            block_id::PINK_WALL_BANNER,
            block_id::PISTON_HEAD,
            block_id::PLAYER_HEAD,
            block_id::PLAYER_WALL_HEAD,
            block_id::POLISHED_ANDESITE_SLAB,
            block_id::POLISHED_ANDESITE_STAIRS,
            block_id::POLISHED_BLACKSTONE_BRICK_SLAB,
            block_id::POLISHED_BLACKSTONE_BRICK_STAIRS,
            block_id::POLISHED_BLACKSTONE_BRICK_WALL,
            block_id::POLISHED_BLACKSTONE_BUTTON,
            block_id::POLISHED_BLACKSTONE_PRESSURE_PLATE,
            block_id::POLISHED_BLACKSTONE_SLAB,
            block_id::POLISHED_BLACKSTONE_STAIRS,
            block_id::POLISHED_BLACKSTONE_WALL,
            block_id::POLISHED_CINNABAR_SLAB,
            block_id::POLISHED_CINNABAR_STAIRS,
            block_id::POLISHED_CINNABAR_WALL,
            block_id::POLISHED_DEEPSLATE_SLAB,
            block_id::POLISHED_DEEPSLATE_STAIRS,
            block_id::POLISHED_DEEPSLATE_WALL,
            block_id::POLISHED_DIORITE_SLAB,
            block_id::POLISHED_DIORITE_STAIRS,
            block_id::POLISHED_GRANITE_SLAB,
            block_id::POLISHED_GRANITE_STAIRS,
            block_id::POLISHED_SULFUR_SLAB,
            block_id::POLISHED_SULFUR_STAIRS,
            block_id::POLISHED_SULFUR_WALL,
            block_id::POLISHED_TUFF_SLAB,
            block_id::POLISHED_TUFF_STAIRS,
            block_id::POLISHED_TUFF_WALL,
            block_id::PRISMARINE_BRICK_SLAB,
            block_id::PRISMARINE_BRICK_STAIRS,
            block_id::PRISMARINE_SLAB,
            block_id::PRISMARINE_STAIRS,
            block_id::PRISMARINE_WALL,
            block_id::PURPLE_BANNER,
            block_id::PURPLE_BED,
            block_id::PURPLE_CARPET,
            block_id::PURPLE_WALL_BANNER,
            block_id::PURPUR_SLAB,
            block_id::PURPUR_STAIRS,
            block_id::QUARTZ_SLAB,
            block_id::QUARTZ_STAIRS,
            block_id::RED_BANNER,
            block_id::RED_BED,
            block_id::RED_CARPET,
            block_id::RED_NETHER_BRICK_SLAB,
            block_id::RED_NETHER_BRICK_STAIRS,
            block_id::RED_NETHER_BRICK_WALL,
            block_id::RED_SANDSTONE_SLAB,
            block_id::RED_SANDSTONE_STAIRS,
            block_id::RED_SANDSTONE_WALL,
            block_id::RED_WALL_BANNER,
            block_id::RESIN_BRICK_SLAB,
            block_id::RESIN_BRICK_STAIRS,
            block_id::RESIN_BRICK_WALL,
            block_id::SANDSTONE_SLAB,
            block_id::SANDSTONE_STAIRS,
            block_id::SANDSTONE_WALL,
            block_id::SKELETON_SKULL,
            block_id::SKELETON_WALL_SKULL,
            block_id::SMOOTH_QUARTZ_SLAB,
            block_id::SMOOTH_QUARTZ_STAIRS,
            block_id::SMOOTH_RED_SANDSTONE_SLAB,
            block_id::SMOOTH_RED_SANDSTONE_STAIRS,
            block_id::SMOOTH_SANDSTONE_SLAB,
            block_id::SMOOTH_SANDSTONE_STAIRS,
            block_id::SMOOTH_STONE_SLAB,
            block_id::SPRUCE_BUTTON,
            block_id::SPRUCE_DOOR,
            block_id::SPRUCE_FENCE,
            block_id::SPRUCE_FENCE_GATE,
            block_id::SPRUCE_HANGING_SIGN,
            block_id::SPRUCE_PRESSURE_PLATE,
            block_id::SPRUCE_SIGN,
            block_id::SPRUCE_SLAB,
            block_id::SPRUCE_STAIRS,
            block_id::SPRUCE_TRAPDOOR,
            block_id::SPRUCE_WALL_HANGING_SIGN,
            block_id::SPRUCE_WALL_SIGN,
            block_id::STONE_BRICK_SLAB,
            block_id::STONE_BRICK_STAIRS,
            block_id::STONE_BRICK_WALL,
            block_id::STONE_BUTTON,
            block_id::STONE_PRESSURE_PLATE,
            block_id::STONE_SLAB,
            block_id::STONE_STAIRS,
            block_id::SULFUR_BRICK_SLAB,
            block_id::SULFUR_BRICK_STAIRS,
            block_id::SULFUR_BRICK_WALL,
            block_id::SULFUR_SLAB,
            block_id::SULFUR_STAIRS,
            block_id::SULFUR_WALL,
            block_id::TUFF_BRICK_SLAB,
            block_id::TUFF_BRICK_STAIRS,
            block_id::TUFF_BRICK_WALL,
            block_id::TUFF_SLAB,
            block_id::TUFF_STAIRS,
            block_id::TUFF_WALL,
            block_id::WARPED_BUTTON,
            block_id::WARPED_DOOR,
            block_id::WARPED_FENCE,
            block_id::WARPED_FENCE_GATE,
            block_id::WARPED_HANGING_SIGN,
            block_id::WARPED_PRESSURE_PLATE,
            block_id::WARPED_SIGN,
            block_id::WARPED_SLAB,
            block_id::WARPED_STAIRS,
            block_id::WARPED_TRAPDOOR,
            block_id::WARPED_WALL_HANGING_SIGN,
            block_id::WARPED_WALL_SIGN,
            block_id::WAXED_COPPER_DOOR,
            block_id::WAXED_COPPER_TRAPDOOR,
            block_id::WAXED_CUT_COPPER_SLAB,
            block_id::WAXED_CUT_COPPER_STAIRS,
            block_id::WAXED_EXPOSED_COPPER_DOOR,
            block_id::WAXED_EXPOSED_COPPER_TRAPDOOR,
            block_id::WAXED_EXPOSED_CUT_COPPER_SLAB,
            block_id::WAXED_EXPOSED_CUT_COPPER_STAIRS,
            block_id::WAXED_OXIDIZED_COPPER_DOOR,
            block_id::WAXED_OXIDIZED_COPPER_TRAPDOOR,
            block_id::WAXED_OXIDIZED_CUT_COPPER_SLAB,
            block_id::WAXED_OXIDIZED_CUT_COPPER_STAIRS,
            block_id::WAXED_WEATHERED_COPPER_DOOR,
            block_id::WAXED_WEATHERED_COPPER_TRAPDOOR,
            block_id::WAXED_WEATHERED_CUT_COPPER_SLAB,
            block_id::WAXED_WEATHERED_CUT_COPPER_STAIRS,
            block_id::WEATHERED_COPPER_DOOR,
            block_id::WEATHERED_COPPER_TRAPDOOR,
            block_id::WEATHERED_CUT_COPPER_SLAB,
            block_id::WEATHERED_CUT_COPPER_STAIRS,
            block_id::WHITE_BANNER,
            block_id::WHITE_BED,
            block_id::WHITE_CARPET,
            block_id::WHITE_WALL_BANNER,
            block_id::WITHER_SKELETON_SKULL,
            block_id::WITHER_SKELETON_WALL_SKULL,
            block_id::YELLOW_BANNER,
            block_id::YELLOW_BED,
            block_id::YELLOW_CARPET,
            block_id::YELLOW_WALL_BANNER,
            block_id::ZOMBIE_HEAD,
            block_id::ZOMBIE_WALL_HEAD,
        ],
        TRANSPARENT,
    );

    // Hand-curated non-full-shape blocks not covered by any suffix pattern above (redstone
    // components, rails, plants/crops/saplings/flowers/potted variants, corals (non-block
    // forms), copper furniture (bars/chains/chests/golem statues/grates/lanterns, every
    // weathering tier + waxed), and every other "not a full cube" block this changeset's own
    // classification pass identified). Emitters among this same category are handled above,
    // never here (register_range panics on any overlap).
    register_full_range_many(
        &mut reg,
        &mut covered,
        &[
            block_id::ACTIVATOR_RAIL,
            block_id::AIR,
            block_id::ALLIUM,
            block_id::ANVIL,
            block_id::ATTACHED_MELON_STEM,
            block_id::ATTACHED_PUMPKIN_STEM,
            block_id::AZALEA,
            block_id::AZURE_BLUET,
            block_id::BAMBOO,
            block_id::BAMBOO_SAPLING,
            block_id::BEETROOTS,
            block_id::BELL,
            block_id::BIG_DRIPLEAF,
            block_id::BIG_DRIPLEAF_STEM,
            block_id::BLUE_ORCHID,
            block_id::BRAIN_CORAL,
            block_id::BRAIN_CORAL_FAN,
            block_id::BRAIN_CORAL_WALL_FAN,
            block_id::BROWN_MUSHROOM,
            block_id::BUBBLE_COLUMN,
            block_id::BUBBLE_CORAL,
            block_id::BUBBLE_CORAL_FAN,
            block_id::BUBBLE_CORAL_WALL_FAN,
            block_id::BUSH,
            block_id::CACTUS,
            block_id::CACTUS_FLOWER,
            block_id::CAKE,
            block_id::CALIBRATED_SCULK_SENSOR,
            block_id::CARROTS,
            block_id::CAULDRON,
            block_id::CAVE_AIR,
            block_id::CHEST,
            block_id::CHIPPED_ANVIL,
            block_id::CHORUS_FLOWER,
            block_id::CHORUS_PLANT,
            block_id::CLOSED_EYEBLOSSOM,
            block_id::COBWEB,
            block_id::COCOA,
            block_id::COMPARATOR,
            block_id::COMPOSTER,
            block_id::CONDUIT,
            block_id::COPPER_BARS,
            block_id::COPPER_CHAIN,
            block_id::COPPER_CHEST,
            block_id::COPPER_GOLEM_STATUE,
            block_id::COPPER_GRATE,
            block_id::COPPER_LANTERN,
            block_id::CORNFLOWER,
            block_id::CRIMSON_FUNGUS,
            block_id::CRIMSON_ROOTS,
            block_id::DAMAGED_ANVIL,
            block_id::DANDELION,
            block_id::DAYLIGHT_DETECTOR,
            block_id::DEAD_BRAIN_CORAL,
            block_id::DEAD_BRAIN_CORAL_FAN,
            block_id::DEAD_BRAIN_CORAL_WALL_FAN,
            block_id::DEAD_BUBBLE_CORAL,
            block_id::DEAD_BUBBLE_CORAL_FAN,
            block_id::DEAD_BUBBLE_CORAL_WALL_FAN,
            block_id::DEAD_BUSH,
            block_id::DEAD_FIRE_CORAL,
            block_id::DEAD_FIRE_CORAL_FAN,
            block_id::DEAD_FIRE_CORAL_WALL_FAN,
            block_id::DEAD_HORN_CORAL,
            block_id::DEAD_HORN_CORAL_FAN,
            block_id::DEAD_HORN_CORAL_WALL_FAN,
            block_id::DEAD_TUBE_CORAL,
            block_id::DEAD_TUBE_CORAL_FAN,
            block_id::DEAD_TUBE_CORAL_WALL_FAN,
            block_id::DECORATED_POT,
            block_id::DETECTOR_RAIL,
            block_id::DIRT_PATH,
            block_id::EXPOSED_COPPER_BARS,
            block_id::EXPOSED_COPPER_CHAIN,
            block_id::EXPOSED_COPPER_CHEST,
            block_id::EXPOSED_COPPER_GOLEM_STATUE,
            block_id::EXPOSED_COPPER_GRATE,
            block_id::EXPOSED_COPPER_LANTERN,
            block_id::EXPOSED_LIGHTNING_ROD,
            block_id::FARMLAND,
            block_id::FERN,
            block_id::FIRE_CORAL,
            block_id::FIRE_CORAL_FAN,
            block_id::FIRE_CORAL_WALL_FAN,
            block_id::FLOWERING_AZALEA,
            block_id::FLOWER_POT,
            block_id::FROGSPAWN,
            block_id::GLASS_PANE,
            block_id::GOLDEN_DANDELION,
            block_id::GRINDSTONE,
            block_id::HANGING_ROOTS,
            block_id::HEAVY_CORE,
            block_id::HOPPER,
            block_id::HORN_CORAL,
            block_id::HORN_CORAL_FAN,
            block_id::HORN_CORAL_WALL_FAN,
            block_id::IRON_BARS,
            block_id::IRON_CHAIN,
            block_id::KELP,
            block_id::KELP_PLANT,
            block_id::LADDER,
            block_id::LARGE_FERN,
            block_id::LEAF_LITTER,
            block_id::LECTERN,
            block_id::LEVER,
            block_id::LIGHTNING_ROD,
            block_id::LILAC,
            block_id::LILY_OF_THE_VALLEY,
            block_id::LILY_PAD,
            block_id::MANGROVE_PROPAGULE,
            block_id::MANGROVE_ROOTS,
            block_id::MELON_STEM,
            block_id::NETHER_WART,
            block_id::OPEN_EYEBLOSSOM,
            block_id::ORANGE_TULIP,
            block_id::OXEYE_DAISY,
            block_id::OXIDIZED_COPPER_BARS,
            block_id::OXIDIZED_COPPER_CHAIN,
            block_id::OXIDIZED_COPPER_CHEST,
            block_id::OXIDIZED_COPPER_GOLEM_STATUE,
            block_id::OXIDIZED_COPPER_GRATE,
            block_id::OXIDIZED_COPPER_LANTERN,
            block_id::OXIDIZED_LIGHTNING_ROD,
            block_id::PALE_HANGING_MOSS,
            block_id::PEONY,
            block_id::PINK_PETALS,
            block_id::PINK_TULIP,
            block_id::PITCHER_CROP,
            block_id::PITCHER_PLANT,
            block_id::POINTED_DRIPSTONE,
            block_id::POPPY,
            block_id::POTATOES,
            block_id::POTTED_ACACIA_SAPLING,
            block_id::POTTED_ALLIUM,
            block_id::POTTED_AZALEA_BUSH,
            block_id::POTTED_AZURE_BLUET,
            block_id::POTTED_BAMBOO,
            block_id::POTTED_BIRCH_SAPLING,
            block_id::POTTED_BLUE_ORCHID,
            block_id::POTTED_BROWN_MUSHROOM,
            block_id::POTTED_CACTUS,
            block_id::POTTED_CHERRY_SAPLING,
            block_id::POTTED_CLOSED_EYEBLOSSOM,
            block_id::POTTED_CORNFLOWER,
            block_id::POTTED_CRIMSON_FUNGUS,
            block_id::POTTED_CRIMSON_ROOTS,
            block_id::POTTED_DANDELION,
            block_id::POTTED_DARK_OAK_SAPLING,
            block_id::POTTED_DEAD_BUSH,
            block_id::POTTED_FERN,
            block_id::POTTED_FLOWERING_AZALEA_BUSH,
            block_id::POTTED_GOLDEN_DANDELION,
            block_id::POTTED_JUNGLE_SAPLING,
            block_id::POTTED_LILY_OF_THE_VALLEY,
            block_id::POTTED_MANGROVE_PROPAGULE,
            block_id::POTTED_OAK_SAPLING,
            block_id::POTTED_OPEN_EYEBLOSSOM,
            block_id::POTTED_ORANGE_TULIP,
            block_id::POTTED_OXEYE_DAISY,
            block_id::POTTED_PALE_OAK_SAPLING,
            block_id::POTTED_PINK_TULIP,
            block_id::POTTED_POPPY,
            block_id::POTTED_RED_MUSHROOM,
            block_id::POTTED_RED_TULIP,
            block_id::POTTED_SPRUCE_SAPLING,
            block_id::POTTED_TORCHFLOWER,
            block_id::POTTED_WARPED_FUNGUS,
            block_id::POTTED_WARPED_ROOTS,
            block_id::POTTED_WHITE_TULIP,
            block_id::POTTED_WITHER_ROSE,
            block_id::POWDER_SNOW,
            block_id::POWDER_SNOW_CAULDRON,
            block_id::POWERED_RAIL,
            block_id::PUMPKIN_STEM,
            block_id::RAIL,
            block_id::REDSTONE_WIRE,
            block_id::RED_MUSHROOM,
            block_id::RED_TULIP,
            block_id::REPEATER,
            block_id::RESIN_CLUMP,
            block_id::ROSE_BUSH,
            block_id::SCAFFOLDING,
            block_id::SCULK_VEIN,
            block_id::SEAGRASS,
            block_id::SHORT_DRY_GRASS,
            block_id::SHORT_GRASS,
            block_id::SMALL_DRIPLEAF,
            block_id::SNIFFER_EGG,
            block_id::SNOW,
            block_id::SPORE_BLOSSOM,
            block_id::STONECUTTER,
            block_id::STRUCTURE_VOID,
            block_id::SUGAR_CANE,
            block_id::SULFUR_SPIKE,
            block_id::SUNFLOWER,
            block_id::SWEET_BERRY_BUSH,
            block_id::TALL_DRY_GRASS,
            block_id::TALL_GRASS,
            block_id::TALL_SEAGRASS,
            block_id::TORCHFLOWER,
            block_id::TORCHFLOWER_CROP,
            block_id::TRAPPED_CHEST,
            block_id::TRIPWIRE,
            block_id::TRIPWIRE_HOOK,
            block_id::TUBE_CORAL,
            block_id::TUBE_CORAL_FAN,
            block_id::TUBE_CORAL_WALL_FAN,
            block_id::TURTLE_EGG,
            block_id::TWISTING_VINES,
            block_id::TWISTING_VINES_PLANT,
            block_id::VINE,
            block_id::VOID_AIR,
            block_id::WARPED_FUNGUS,
            block_id::WARPED_ROOTS,
            block_id::WATER_CAULDRON,
            block_id::WAXED_COPPER_BARS,
            block_id::WAXED_COPPER_CHAIN,
            block_id::WAXED_COPPER_CHEST,
            block_id::WAXED_COPPER_GOLEM_STATUE,
            block_id::WAXED_COPPER_GRATE,
            block_id::WAXED_COPPER_LANTERN,
            block_id::WAXED_EXPOSED_COPPER_BARS,
            block_id::WAXED_EXPOSED_COPPER_CHAIN,
            block_id::WAXED_EXPOSED_COPPER_CHEST,
            block_id::WAXED_EXPOSED_COPPER_GOLEM_STATUE,
            block_id::WAXED_EXPOSED_COPPER_GRATE,
            block_id::WAXED_EXPOSED_COPPER_LANTERN,
            block_id::WAXED_EXPOSED_LIGHTNING_ROD,
            block_id::WAXED_LIGHTNING_ROD,
            block_id::WAXED_OXIDIZED_COPPER_BARS,
            block_id::WAXED_OXIDIZED_COPPER_CHAIN,
            block_id::WAXED_OXIDIZED_COPPER_CHEST,
            block_id::WAXED_OXIDIZED_COPPER_GOLEM_STATUE,
            block_id::WAXED_OXIDIZED_COPPER_GRATE,
            block_id::WAXED_OXIDIZED_COPPER_LANTERN,
            block_id::WAXED_OXIDIZED_LIGHTNING_ROD,
            block_id::WAXED_WEATHERED_COPPER_BARS,
            block_id::WAXED_WEATHERED_COPPER_CHAIN,
            block_id::WAXED_WEATHERED_COPPER_CHEST,
            block_id::WAXED_WEATHERED_COPPER_GOLEM_STATUE,
            block_id::WAXED_WEATHERED_COPPER_GRATE,
            block_id::WAXED_WEATHERED_COPPER_LANTERN,
            block_id::WAXED_WEATHERED_LIGHTNING_ROD,
            block_id::WEATHERED_COPPER_BARS,
            block_id::WEATHERED_COPPER_CHAIN,
            block_id::WEATHERED_COPPER_CHEST,
            block_id::WEATHERED_COPPER_GOLEM_STATUE,
            block_id::WEATHERED_COPPER_GRATE,
            block_id::WEATHERED_COPPER_LANTERN,
            block_id::WEATHERED_LIGHTNING_ROD,
            block_id::WEEPING_VINES,
            block_id::WEEPING_VINES_PLANT,
            block_id::WHEAT,
            block_id::WHITE_TULIP,
            block_id::WILDFLOWERS,
            block_id::WITHER_ROSE,
        ],
        TRANSPARENT,
    );

    // --- Full-shaped-but-non-occluding blocks (opacity 1, module doc comment above) --------
    register_full_range_many(
        &mut reg,
        &mut covered,
        &[
            block_id::ACACIA_LEAVES,
            block_id::AZALEA_LEAVES,
            block_id::BIRCH_LEAVES,
            block_id::CHERRY_LEAVES,
            block_id::DARK_OAK_LEAVES,
            block_id::FLOWERING_AZALEA_LEAVES,
            block_id::JUNGLE_LEAVES,
            block_id::MANGROVE_LEAVES,
            block_id::OAK_LEAVES,
            block_id::PALE_OAK_LEAVES,
            block_id::SPRUCE_LEAVES,
        ],
        NOOCCLUSION_FULL_SHAPE,
    );
    register_full_range_many(
        &mut reg,
        &mut covered,
        &[
            block_id::BLACK_STAINED_GLASS,
            block_id::BLUE_STAINED_GLASS,
            block_id::BROWN_STAINED_GLASS,
            block_id::CYAN_STAINED_GLASS,
            block_id::GLASS,
            block_id::GRAY_STAINED_GLASS,
            block_id::GREEN_STAINED_GLASS,
            block_id::LIGHT_BLUE_STAINED_GLASS,
            block_id::LIGHT_GRAY_STAINED_GLASS,
            block_id::LIME_STAINED_GLASS,
            block_id::MAGENTA_STAINED_GLASS,
            block_id::ORANGE_STAINED_GLASS,
            block_id::PINK_STAINED_GLASS,
            block_id::PURPLE_STAINED_GLASS,
            block_id::RED_STAINED_GLASS,
            block_id::WHITE_STAINED_GLASS,
            block_id::YELLOW_STAINED_GLASS,
        ],
        NOOCCLUSION_FULL_SHAPE,
    );
    register_full_range_many(
        &mut reg,
        &mut covered,
        &[
            block_id::BLUE_ICE,
            block_id::FROSTED_ICE,
            block_id::ICE,
            block_id::PACKED_ICE,
            block_id::SLIME_BLOCK,
            block_id::HONEY_BLOCK,
            block_id::WATER,
        ],
        NOOCCLUSION_FULL_SHAPE,
    );
    // LAVA shares WATER's own "non-empty fluid state forces `propagatesSkylightDown` false"
    // opacity-1 case (module doc comment above), but -- unlike every other entry in this
    // section -- is also a real emitter: `.lightLevel(statex -> 15)` (`Blocks.java` line
    // ~307).
    register_full_range(
        &mut reg,
        &mut covered,
        block_id::LAVA,
        LightProperties {
            block_emission: 15,
            opacity: 1,
            occludes_face: [false; 6],
        },
    );

    // --- Default fill: every remaining block is an ordinary, solid, opaque, non-emitting
    // full cube (`LightProperties::OPAQUE`) -- ordinary terrain, ores, logs/planks/wood-as-
    // blocks, wool/concrete/terracotta/glazed-terracotta, and every other block this pass's
    // own classification above did not name. `TINTED_GLASS` (module doc comment above,
    // `TintedGlassBlock.getLightDampening`'s own explicit `15` override) is correctly opaque
    // by this same default fill -- it needs no explicit entry of its own.
    let uncovered: Vec<u32> = covered
        .iter()
        .enumerate()
        .filter(|&(_, &done)| !done)
        .map(|(index, _)| index as u32)
        .collect();
    for index in uncovered {
        register_full_range(
            &mut reg,
            &mut covered,
            BlockId(index),
            LightProperties::OPAQUE,
        );
    }

    reg
}
