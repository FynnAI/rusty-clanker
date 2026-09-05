//! test-matrix: boundaries=waived(a data registry over `BlockStateId`, no world-position axis) orientations=waived(no placement/facing concept in this registry's own domain) self=waived(no actor/entity in this suite's own domain) composition=waived(single registry construction, no multi-component chain) nondefault-state=yes
//! M4-B07 field-report test-authoring: `rc_mechanics::light::production_registry()`'s own
//! acceptance tests (the production `LightPropertiesRegistry` the composition root now
//! populates -- `properties.rs`'s own module doc comment has the full classification-method
//! writeup, cited again per assertion below). Every expected value here is computed from the
//! ASSET-D18(f) reference (`net/minecraft/world/level/block/Blocks.java`'s own
//! `.lightLevel(...)` registrations, and `net/minecraft/world/level/block/state/
//! BlockBehaviour.java`'s own `getLightDampening`/`propagatesSkylightDown` default algorithm)
//! independently of `production_registry`'s own implementation -- never derived by calling the
//! function and pasting its output (TEST-D56).

use rc_chunk_storage::BlockStateId as StorageStateId;
use rc_mechanics::light::LightProperties;
use rc_registries::block_state_properties::{range_of, state_id};
use rc_registries::generated_v776::block_state_properties::{BLOCK_RANGES, block_id};
use rc_registries::generated_v776::block_states::BlockStateId as GenStateId;
use rc_registries::generated_v776::block_states::default_state;

fn storage_id(raw: GenStateId) -> StorageStateId {
    StorageStateId(raw.0)
}

/// Every one of the generated registry's 1196 blocks (`BLOCK_RANGES.len()`) is classified by
/// `production_registry`, never left to `resolve`'s own `LightProperties::AIR` fallback --
/// `LightPropertiesRegistry::is_registered`'s own doc comment explains why this needs its own
/// accessor rather than comparing `resolve`'s return value against `AIR` (a deliberately
/// fully-transparent registered range and the unregistered fallback are bit-identical).
/// `register_range` (called by every one of `production_registry`'s own registration helpers)
/// already panics on any overlap, so this test constructing the registry at all without a
/// panic is itself the "no overlapping ranges" half of this same acceptance criterion.
#[test]
fn every_generated_block_range_is_covered_and_no_ranges_overlap() {
    let registry = rc_mechanics::light::production_registry();
    for (index, &range) in BLOCK_RANGES.iter().enumerate() {
        assert!(
            registry.is_registered(storage_id(range.first)),
            "block index {index} (first state {:?}) is not covered by production_registry",
            range.first
        );
        assert!(
            registry.is_registered(storage_id(range.last)),
            "block index {index} (last state {:?}) is not covered by production_registry",
            range.last
        );
    }
}

/// Full-cube, ordinary solid blocks resolve to opacity 15, no emission -- vanilla's own
/// default `getLightDampening` algorithm (`BlockBehaviour.java`): `canOcclude` stays `true`
/// (neither `.noCollision()` nor `.noOcclusion()` is called for any of these in `Blocks.java`)
/// and each one's own shape is a full unit cube, so `isSolidRender()` is `true` and
/// `getLightDampening` returns `15` unconditionally.
#[test]
fn ordinary_full_cube_blocks_resolve_opacity_15_no_emission() {
    let registry = rc_mechanics::light::production_registry();
    for block in [
        block_id::STONE,
        block_id::DIRT,
        block_id::COBBLESTONE,
        block_id::OAK_PLANKS,
        block_id::OAK_LOG,
        block_id::GOLD_ORE,
        block_id::WHITE_WOOL,
        block_id::OBSIDIAN,
        block_id::NETHERRACK,
        // TINTED_GLASS: `TintedGlassBlock.getLightDampening` overrides back to an
        // unconditional `15` (module doc comment above) -- opaque despite looking like glass.
        block_id::TINTED_GLASS,
    ] {
        let default_id = storage_id(range_of(block).default);
        let props = registry.resolve(default_id);
        assert_eq!(
            props,
            LightProperties::OPAQUE,
            "{block:?}'s own default state should resolve to LightProperties::OPAQUE"
        );
    }
}

/// No-collision blocks (`.noCollision()` in `Blocks.java`, `canOcclude` forced `false`) whose
/// own raw shape is not itself a full cube resolve to opacity 0 -- `propagatesSkylightDown`'s
/// default (`!isShapeFullBlock(rawShape) && fluidState.isEmpty()`) is `true` for all four,
/// landing `getLightDampening` on its `0` branch.
#[test]
fn no_collision_blocks_resolve_opacity_0() {
    let registry = rc_mechanics::light::production_registry();

    // REDSTONE_WIRE: every reachable state (power 0..=15, each side none/side/up) is
    // non-collision -- spot-check a handful, including a fully-powered, fully-connected one
    // (a non-default state, TEST-D55(e)).
    let wire_range = range_of(block_id::REDSTONE_WIRE);
    for raw in [wire_range.default.0, wire_range.first.0, wire_range.last.0] {
        let props = registry.resolve(StorageStateId(raw));
        assert_eq!(
            props.opacity, 0,
            "redstone_wire state {raw} should be opacity 0"
        );
    }

    // LEVER: all 24 states (3 face x 4 facing x 2 powered) -- spot-check every `face` value
    // plus a `powered=true` non-default state.
    for face in ["floor", "wall", "ceiling"] {
        let id = state_id(block_id::LEVER, &[("face", face), ("powered", "true")])
            .expect("legal lever property set");
        assert_eq!(
            registry.resolve(storage_id(id)).opacity,
            0,
            "lever face={face} powered=true should be opacity 0"
        );
    }

    // Buttons (stone + a wood variant): non-collision.
    for block in [block_id::STONE_BUTTON, block_id::OAK_BUTTON] {
        let default_id = storage_id(range_of(block).default);
        assert_eq!(
            registry.resolve(default_id).opacity,
            0,
            "{block:?} should be opacity 0"
        );
    }
}

/// Uniform emitters, cited against `Blocks.java`'s own `.lightLevel(...)` registrations.
#[test]
fn uniform_emitter_values_match_the_reference() {
    let registry = rc_mechanics::light::production_registry();
    let cases: &[(
        rc_registries::generated_v776::block_state_properties::BlockId,
        u8,
        u8,
    )] = &[
        // (block, expected opacity, expected emission)
        (block_id::TORCH, 0, 14),        // Blocks.java ~1206
        (block_id::WALL_TORCH, 0, 14),   // Blocks.java ~1211
        (block_id::SOUL_TORCH, 0, 10),   // Blocks.java ~2058
        (block_id::GLOWSTONE, 15, 15),   // Blocks.java ~2082
        (block_id::SEA_LANTERN, 15, 15), // Blocks.java ~2993
        (block_id::LANTERN, 0, 15),      // Blocks.java ~4460
        (block_id::SOUL_LANTERN, 0, 10), // Blocks.java ~4472
        (block_id::SHROOMLIGHT, 15, 15), // Blocks.java ~4608
        (block_id::LAVA, 1, 15),         // Blocks.java ~307
        (block_id::FIRE, 0, 15),         // Blocks.java ~1221
        (block_id::MAGMA_BLOCK, 15, 3),  // Blocks.java ~3746
        (block_id::END_ROD, 0, 14),      // Blocks.java ~3620
        (block_id::BEACON, 15, 15),      // Blocks.java ~2664
    ];
    for &(block, expected_opacity, expected_emission) in cases {
        let default_id = storage_id(range_of(block).default);
        let props = registry.resolve(default_id);
        assert_eq!(props.opacity, expected_opacity, "{block:?} opacity");
        assert_eq!(
            props.block_emission, expected_emission,
            "{block:?} emission"
        );
    }
}

/// Lit-dependent emitters: `litBlockEmission(level)` (`Blocks.java`) -- `level` while
/// `lit == true`, `0` otherwise. Both non-default states (`lit=true`) count toward
/// TEST-D55(e)'s "at least one non-default block state" requirement.
#[test]
fn lit_dependent_emitters_match_the_reference() {
    let registry = rc_mechanics::light::production_registry();

    // REDSTONE_TORCH: litBlockEmission(7) (Blocks.java ~1923). Default state is lit=true.
    let lit_id = state_id(block_id::REDSTONE_TORCH, &[("lit", "true")]).unwrap();
    let unlit_id = state_id(block_id::REDSTONE_TORCH, &[("lit", "false")]).unwrap();
    assert_eq!(registry.resolve(storage_id(lit_id)).block_emission, 7);
    assert_eq!(registry.resolve(storage_id(unlit_id)).block_emission, 0);
    assert_eq!(registry.resolve(storage_id(lit_id)).opacity, 0);

    // FURNACE: litBlockEmission(13) (Blocks.java ~1311).
    let lit_furnace = state_id(block_id::FURNACE, &[("lit", "true")]).unwrap();
    let unlit_furnace = state_id(block_id::FURNACE, &[("lit", "false")]).unwrap();
    assert_eq!(registry.resolve(storage_id(lit_furnace)).block_emission, 13);
    assert_eq!(
        registry.resolve(storage_id(unlit_furnace)).block_emission,
        0
    );
    assert_eq!(registry.resolve(storage_id(lit_furnace)).opacity, 15);

    // REDSTONE_LAMP: litBlockEmission(15) (Blocks.java ~2602).
    let lit_lamp = state_id(block_id::REDSTONE_LAMP, &[("lit", "true")]).unwrap();
    assert_eq!(registry.resolve(storage_id(lit_lamp)).block_emission, 15);
}

/// Dynamic, per-property emitters that are not simple lit/unlit toggles.
#[test]
fn per_state_dynamic_emitters_match_the_reference() {
    let registry = rc_mechanics::light::production_registry();

    // SEA_PICKLE: isDead() = !waterlogged; emission = isDead ? 0 : 3 + 3*pickles
    // (SeaPickleBlock.java ~59-60; Blocks.java ~4219).
    let alive_4 = state_id(
        block_id::SEA_PICKLE,
        &[("pickles", "4"), ("waterlogged", "true")],
    )
    .unwrap();
    assert_eq!(registry.resolve(storage_id(alive_4)).block_emission, 15); // 3 + 3*4
    let dead_4 = state_id(
        block_id::SEA_PICKLE,
        &[("pickles", "4"), ("waterlogged", "false")],
    )
    .unwrap();
    assert_eq!(registry.resolve(storage_id(dead_4)).block_emission, 0);
    let alive_1 = state_id(
        block_id::SEA_PICKLE,
        &[("pickles", "1"), ("waterlogged", "true")],
    )
    .unwrap();
    assert_eq!(registry.resolve(storage_id(alive_1)).block_emission, 6); // 3 + 3*1

    // CANDLE: lit ? 3*candles : 0 (CandleBlock.java ~43).
    let candle_lit_4 = state_id(block_id::CANDLE, &[("candles", "4"), ("lit", "true")]).unwrap();
    assert_eq!(
        registry.resolve(storage_id(candle_lit_4)).block_emission,
        12
    );
    let candle_unlit_4 = state_id(block_id::CANDLE, &[("candles", "4"), ("lit", "false")]).unwrap();
    assert_eq!(
        registry.resolve(storage_id(candle_unlit_4)).block_emission,
        0
    );

    // LIGHT: emission = level (LightBlock.LIGHT_EMISSION, Blocks.java ~2949).
    let level_7 = state_id(block_id::LIGHT, &[("level", "7")]).unwrap();
    assert_eq!(registry.resolve(storage_id(level_7)).block_emission, 7);
    assert_eq!(registry.resolve(storage_id(level_7)).opacity, 0);

    // RESPAWN_ANCHOR: floor(charges/4.0 * 15) (RespawnAnchorBlock.java ~214/220).
    let charge_4 = state_id(block_id::RESPAWN_ANCHOR, &[("charges", "4")]).unwrap();
    assert_eq!(registry.resolve(storage_id(charge_4)).block_emission, 15);
    let charge_2 = state_id(block_id::RESPAWN_ANCHOR, &[("charges", "2")]).unwrap();
    assert_eq!(registry.resolve(storage_id(charge_2)).block_emission, 7); // floor(2/4*15)=7
    let charge_0 = state_id(block_id::RESPAWN_ANCHOR, &[("charges", "0")]).unwrap();
    assert_eq!(registry.resolve(storage_id(charge_0)).block_emission, 0);
}

/// Full-shaped-but-non-occluding blocks (module doc comment above): opacity 1, not 0 --
/// `LeavesBlock.getLightDampening` overrides to an unconditional `1` directly (leaves), and
/// glass/ice/water/slime/honey all reach the same value via vanilla's own default algorithm's
/// `canOcclude = false` + "raw shape is a full cube" combination.
#[test]
fn noocclusion_full_shape_blocks_resolve_opacity_1() {
    let registry = rc_mechanics::light::production_registry();
    for block in [
        block_id::OAK_LEAVES,
        block_id::GLASS,
        block_id::WHITE_STAINED_GLASS,
        block_id::ICE,
        block_id::PACKED_ICE,
        block_id::SLIME_BLOCK,
        block_id::HONEY_BLOCK,
        block_id::WATER,
    ] {
        let default_id = storage_id(range_of(block).default);
        let props = registry.resolve(default_id);
        assert_eq!(props.opacity, 1, "{block:?} should resolve opacity 1");
        assert_eq!(props.block_emission, 0, "{block:?} should not emit light");
    }
}

/// Every one of the two known-vanilla "always AIR" states resolves to `LightProperties::AIR`
/// (opacity 0, no emission) -- `production_registry` explicitly registers `AIR`/`CAVE_AIR`/
/// `VOID_AIR`'s own full ranges rather than leaving them to the unregistered fallback (this
/// test cannot itself distinguish the two paths -- `is_registered`, above, already does).
#[test]
fn air_variants_resolve_to_air_properties() {
    let registry = rc_mechanics::light::production_registry();
    for block in [block_id::AIR, block_id::CAVE_AIR, block_id::VOID_AIR] {
        let default_id = storage_id(range_of(block).default);
        assert_eq!(registry.resolve(default_id), LightProperties::AIR);
    }
    // `default_state::AIR` (the id `rc_registries::generated_v776::block_states::default_state`
    // exposes -- the same constant `crates/server/src/play/block_action.rs::seed_chunk_column`
    // fills every superflat column's own air layer with) resolves identically.
    assert_eq!(
        registry.resolve(StorageStateId(default_state::AIR.0)),
        LightProperties::AIR
    );
}
