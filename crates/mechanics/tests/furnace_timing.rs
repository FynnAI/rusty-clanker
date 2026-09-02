//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only)) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain) nondefault-state=yes
//! M3-B06 — hand-derived furnace timing goldens (Acceptance tests' own `furnace_timing.rs`
//! section, the task's own required acceptance category).

use rc_chunk_storage::ItemStackRecord;
use rc_mechanics::block_entity::furnace::{
    FURNACE_SLOT_FUEL, FURNACE_SLOT_INPUT, FURNACE_SLOT_OUTPUT, FuelTable, FurnaceBlockEntity,
    LitStateChange, SmeltingRecipe, SmeltingRecipeTable,
};
use rc_mechanics::container::DefaultMaxStackSize;

fn stack(id: &str, count: i32) -> Option<ItemStackRecord> {
    Some(ItemStackRecord {
        id: id.to_string(),
        count,
        components: None,
    })
}

#[test]
fn cold_furnace_with_fuel_and_valid_recipe_ignites_on_first_tick() {
    let mut furnace = FurnaceBlockEntity::empty();
    furnace.slots[FURNACE_SLOT_INPUT] = stack("minecraft:cobblestone", 1);
    furnace.slots[FURNACE_SLOT_FUEL] = stack("minecraft:coal", 1);

    let recipes = SmeltingRecipeTable::minimal_tier1();
    let fuels = FuelTable::minimal_tier1();
    let outcome = furnace.tick(&recipes, &fuels, &DefaultMaxStackSize);

    assert_eq!(furnace.lit_time_remaining, 1600);
    assert_eq!(furnace.lit_total_time, 1600);
    assert!(furnace.slots[FURNACE_SLOT_FUEL].is_none());
    assert_eq!(outcome, LitStateChange::NowLit);
}

#[test]
fn cook_completes_at_exactly_two_hundred_ticks() {
    let mut furnace = FurnaceBlockEntity::empty();
    furnace.lit_time_remaining = 1600;
    furnace.lit_total_time = 1600;
    furnace.slots[FURNACE_SLOT_INPUT] = stack("minecraft:cobblestone", 1);

    let recipes = SmeltingRecipeTable::minimal_tier1();
    let fuels = FuelTable::minimal_tier1();

    for _ in 0..199 {
        furnace.tick(&recipes, &fuels, &DefaultMaxStackSize);
    }
    assert!(furnace.slots[FURNACE_SLOT_OUTPUT].is_none());
    assert_eq!(furnace.cook_time, 199);

    furnace.tick(&recipes, &fuels, &DefaultMaxStackSize);
    assert_eq!(
        furnace.slots[FURNACE_SLOT_OUTPUT],
        stack("minecraft:stone", 1)
    );
    assert!(furnace.slots[FURNACE_SLOT_INPUT].is_none());
    assert_eq!(furnace.cook_time, 0);
}

#[test]
fn cook_progress_drains_by_two_per_tick_when_fuel_runs_out_mid_cook_nondefault_case() {
    let mut furnace = FurnaceBlockEntity::empty();
    furnace.lit_time_remaining = 1;
    furnace.lit_total_time = 1;
    furnace.slots[FURNACE_SLOT_INPUT] = stack("minecraft:cobblestone", 1);
    furnace.slots[FURNACE_SLOT_FUEL] = None;
    furnace.cook_time = 50;
    furnace.cooking_recipe_output_id = Some("minecraft:stone".to_string());

    let recipes = SmeltingRecipeTable::minimal_tier1();
    let fuels = FuelTable::minimal_tier1();

    furnace.tick(&recipes, &fuels, &DefaultMaxStackSize);
    assert_eq!(furnace.cook_time, 48);
    furnace.tick(&recipes, &fuels, &DefaultMaxStackSize);
    assert_eq!(furnace.cook_time, 46);
    furnace.tick(&recipes, &fuels, &DefaultMaxStackSize);
    assert_eq!(furnace.cook_time, 44);
}

#[test]
fn changing_input_item_mid_cook_resets_progress() {
    let mut furnace = FurnaceBlockEntity::empty();
    furnace.lit_time_remaining = 1000;
    furnace.lit_total_time = 1600;
    furnace.cook_time = 100;
    furnace.cooking_recipe_output_id = Some("minecraft:stone".to_string());
    furnace.slots[FURNACE_SLOT_INPUT] = stack("minecraft:iron_ore", 1);

    let recipes = SmeltingRecipeTable::minimal_tier1();
    let fuels = FuelTable::minimal_tier1();
    furnace.tick(&recipes, &fuels, &DefaultMaxStackSize);

    assert_eq!(furnace.cook_time, 1);
}

#[test]
fn furnace_comparator_signal_matches_generic_formula() {
    let empty = FurnaceBlockEntity::empty();
    assert_eq!(empty.comparator_signal(&DefaultMaxStackSize), 0);

    let mut one_full_input = FurnaceBlockEntity::empty();
    one_full_input.slots[FURNACE_SLOT_INPUT] = stack("minecraft:cobblestone", 64);
    // average = (64/64) / 3 = 0.3333...; floor(0.3333 * 14) + 1 = floor(4.666) + 1 = 4 + 1 = 5
    assert_eq!(one_full_input.comparator_signal(&DefaultMaxStackSize), 5);

    let mut full = FurnaceBlockEntity::empty();
    full.slots[FURNACE_SLOT_INPUT] = stack("minecraft:cobblestone", 64);
    full.slots[FURNACE_SLOT_FUEL] = stack("minecraft:coal", 64);
    full.slots[FURNACE_SLOT_OUTPUT] = stack("minecraft:stone", 64);
    assert_eq!(full.comparator_signal(&DefaultMaxStackSize), 15);
}

#[test]
fn fuel_table_and_recipe_table_minimal_tier1_lookups() {
    let fuels = FuelTable::minimal_tier1();
    assert_eq!(fuels.lookup("minecraft:coal"), Some(1600));
    // source: blocks.json
    assert_eq!(fuels.lookup("minecraft:lava_bucket"), Some(20000));
    assert_eq!(fuels.lookup("minecraft:diamond"), None);

    let recipes = SmeltingRecipeTable::minimal_tier1();
    assert_eq!(
        recipes.lookup("minecraft:iron_ore"),
        Some(SmeltingRecipe {
            output_id: "minecraft:iron_ingot",
            output_count: 1,
            cook_ticks: 200,
        })
    );
    assert_eq!(recipes.lookup("minecraft:dirt"), None);
}
