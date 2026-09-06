//! M4-B05 acceptance tests: fall damage (§3.15) and the food/exhaustion/natural-regeneration
//! system (§3.18).

use rc_mechanics::combat::damage::Difficulty;
use rc_mechanics::combat::fall::calculate_fall_damage;
use rc_mechanics::combat::food::{FoodStats, FoodTickOutcome, tick_food};

#[test]
fn fall_damage_golden_table() {
    let cases: [(f64, f64, f64, i32); 4] = [
        (3.0, 3.0, 1.0, 0),
        (10.0, 3.0, 1.0, 7),
        (5.5, 3.0, 1.0, 2),
        (10.0, 3.0, 0.0, 0),
    ];
    for (fall_distance, safe_fall_distance, multiplier, expected) in cases {
        let got = calculate_fall_damage(fall_distance, 1.0, safe_fall_distance, multiplier);
        assert_eq!(
            got, expected,
            "calculate_fall_damage({fall_distance}, 1.0, {safe_fall_distance}, {multiplier})"
        );
    }
}

#[test]
fn food_tick_fast_regen_branch() {
    let mut stats = FoodStats {
        food_level: 20,
        saturation: 6.0,
        exhaustion: 0.0,
        regen_tick_timer: 9,
    };
    let outcome = tick_food(&mut stats, 15.0, 20.0, Difficulty::Normal, false);
    assert_eq!(outcome, FoodTickOutcome::Healed { amount: 1.0 });
    assert_eq!(stats.regen_tick_timer, 0);
}

#[test]
fn food_tick_starvation_gated_by_difficulty() {
    let mut stats = FoodStats {
        food_level: 0,
        saturation: 0.0,
        exhaustion: 0.0,
        regen_tick_timer: 79,
    };
    let outcome = tick_food(&mut stats, 10.0, 20.0, Difficulty::Normal, false);
    assert_eq!(outcome, FoodTickOutcome::Starved);

    let mut stats = FoodStats {
        food_level: 0,
        saturation: 0.0,
        exhaustion: 0.0,
        regen_tick_timer: 79,
    };
    let outcome = tick_food(&mut stats, 1.0, 20.0, Difficulty::Normal, false);
    assert_eq!(outcome, FoodTickOutcome::NoChange);
    assert_eq!(stats.regen_tick_timer, 0, "the timer still resets");
}

#[test]
fn food_tick_skips_entirely_for_creative() {
    let mut stats = FoodStats {
        food_level: 0,
        saturation: 0.0,
        exhaustion: 10.0,
        regen_tick_timer: 79,
    };
    let before = stats.clone();
    let outcome = tick_food(&mut stats, 1.0, 20.0, Difficulty::Hard, true);
    assert_eq!(outcome, FoodTickOutcome::NoChange);
    assert_eq!(
        stats, before,
        "every field of stats unchanged for a creative player"
    );
}
