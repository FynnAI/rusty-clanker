//! M4-B05 acceptance tests: the full melee damage order of operations (armor/toughness,
//! enchantment-protection-factor, the invulnerability top-up gate, the player/mob absorption
//! asymmetry, difficulty scaling, the creative-invulnerability gate).

use rc_mechanics::combat::attributes::{AttributeKind, AttributeMap};
use rc_mechanics::combat::damage::{
    DamageOutcome, DamageSource, DamageTarget, DamageTypeKind, Difficulty, EnchantLevels,
    apply_damage_pipeline, armor_effective_damage, difficulty_scale_incoming, epf_reduction,
    feather_falling_epf,
};

fn attributes_with(armor: f64, armor_toughness: f64) -> AttributeMap {
    let mut map = rc_mechanics::combat::attributes::default_attributes_for(
        rc_mechanics::entity::EntityKind::Zombie,
    );
    map.set_base(AttributeKind::Armor, armor);
    map.set_base(AttributeKind::ArmorToughness, armor_toughness);
    map
}

fn player_attack_source() -> DamageSource {
    DamageSource {
        kind: DamageTypeKind::PlayerAttack,
        causing_entity: None,
        source_position: None,
        causing_entity_is_living_non_player: false,
    }
}

fn mob_attack_source() -> DamageSource {
    DamageSource {
        kind: DamageTypeKind::MobAttack,
        causing_entity: None,
        source_position: None,
        causing_entity_is_living_non_player: true,
    }
}

fn fall_source() -> DamageSource {
    DamageSource {
        kind: DamageTypeKind::Fall,
        causing_entity: None,
        source_position: None,
        causing_entity_is_living_non_player: true,
    }
}

fn assert_close(what: &str, got: f32, want: f32, tol: f32) {
    assert!(
        (got - want).abs() < tol,
        "{what}: got {got}, want {want} (within {tol})"
    );
}

#[test]
fn armor_toughness_golden_table() {
    // Hand-derived per Context step 4's own formula: toughness = 2.0 + armor_toughness/4.0;
    // real_armor = clamp(total_armor - damage/toughness, total_armor*0.2, 20.0); armor_fraction
    // = clamp(real_armor/25.0, 0.0, 1.0); damage *= (1.0 - armor_fraction).
    // Case 5's own real_armor = clamp(20.0 - 100.0/7.0, 4.0, 20.0) = 20.0 - 14.285714...
    // = 5.714286...; armor_fraction = 5.714286.../25.0 = 0.2285714...; damage = 100.0 *
    // (1.0 - 0.2285714...) = 77.14286....
    let case5_real_armor = 20.0_f32 - 100.0_f32 / 7.0_f32;
    let case5_expected = 100.0_f32 * (1.0 - case5_real_armor / 25.0);
    let cases: [(f32, f64, f64, f32); 5] = [
        (10.0, 0.0, 0.0, 10.0),
        (10.0, 10.0, 0.0, 8.0),
        (10.0, 20.0, 0.0, 4.0),
        (10.0, 15.0, 8.0, 5.0),
        (100.0, 20.0, 20.0, case5_expected),
    ];
    for (damage, total_armor, armor_toughness, expected) in cases {
        let got = armor_effective_damage(damage, total_armor, armor_toughness);
        assert_close(
            &format!("armor_effective_damage({damage}, {total_armor}, {armor_toughness})"),
            got,
            expected,
            1e-4,
        );
    }
}

#[test]
fn epf_reduction_golden_table() {
    // 1.0 - clamp(epf_sum, 0, 20) / 25.0, including the saturation case (24.0 and 20.0 must
    // produce identical output).
    let cases: [(f32, f32); 5] = [
        (0.0, 20.0),
        (4.0, 20.0 * 0.84),
        (16.0, 20.0 * 0.36),
        (20.0, 20.0 * 0.2),
        (24.0, 20.0 * 0.2), // saturates identically to epf_sum = 20.0
    ];
    for (epf_sum, expected) in cases {
        let got = epf_reduction(20.0, epf_sum);
        assert_close(
            &format!("epf_reduction(20.0, {epf_sum})"),
            got,
            expected,
            1e-4,
        );
    }
}

#[test]
fn invulnerability_top_up_sequence() {
    let attributes = attributes_with(0.0, 0.0);
    let mut health = 20.0_f32;
    let mut absorption = 0.0_f32;
    let mut invulnerable_time = 0_i32;
    let mut last_hurt = 0.0_f32;

    // Hit 1: fresh hit, full damage.
    let outcome = apply_damage_pipeline(
        DamageTarget {
            health: &mut health,
            max_health: 20.0,
            absorption: &mut absorption,
            invulnerable_time: &mut invulnerable_time,
            last_hurt: &mut last_hurt,
            is_player: false,
            is_creative: false,
            attributes: &attributes,
        },
        &mob_attack_source(),
        6.0,
        EnchantLevels::default(),
        EnchantLevels::default(),
    );
    assert!(matches!(outcome, DamageOutcome::Dealt { .. }));
    assert_eq!(invulnerable_time, 20);
    assert_eq!(last_hurt, 6.0);
    assert_close("health after hit 1", health, 14.0, 1e-6);

    // Hit 2: same simulated tick (invulnerable_time still 20 > 10), raw_damage <= last_hurt.
    let outcome = apply_damage_pipeline(
        DamageTarget {
            health: &mut health,
            max_health: 20.0,
            absorption: &mut absorption,
            invulnerable_time: &mut invulnerable_time,
            last_hurt: &mut last_hurt,
            is_player: false,
            is_creative: false,
            attributes: &attributes,
        },
        &mob_attack_source(),
        4.0,
        EnchantLevels::default(),
        EnchantLevels::default(),
    );
    assert_eq!(outcome, DamageOutcome::NoOp);
    assert_close("health unchanged after hit 2", health, 14.0, 1e-6);

    // Hit 3: invulnerable_time manually decremented to the boundary 10 (not > 10) -- full
    // fresh damage, not a delta.
    invulnerable_time = 10;
    let outcome = apply_damage_pipeline(
        DamageTarget {
            health: &mut health,
            max_health: 20.0,
            absorption: &mut absorption,
            invulnerable_time: &mut invulnerable_time,
            last_hurt: &mut last_hurt,
            is_player: false,
            is_creative: false,
            attributes: &attributes,
        },
        &mob_attack_source(),
        4.0,
        EnchantLevels::default(),
        EnchantLevels::default(),
    );
    assert!(matches!(outcome, DamageOutcome::Dealt { .. }));
    assert_eq!(
        invulnerable_time, 20,
        "boundary hit resets invulnerable_time"
    );
    assert_eq!(last_hurt, 4.0);
    assert_close("health after hit 3", health, 10.0, 1e-6);

    // Hit 4: invulnerable_time at 15 (> 10, top-up branch) -- only the delta is dealt.
    invulnerable_time = 15;
    let outcome = apply_damage_pipeline(
        DamageTarget {
            health: &mut health,
            max_health: 20.0,
            absorption: &mut absorption,
            invulnerable_time: &mut invulnerable_time,
            last_hurt: &mut last_hurt,
            is_player: false,
            is_creative: false,
            attributes: &attributes,
        },
        &mob_attack_source(),
        9.0,
        EnchantLevels::default(),
        EnchantLevels::default(),
    );
    assert!(matches!(outcome, DamageOutcome::Dealt { .. }));
    assert_eq!(
        invulnerable_time, 15,
        "top-up branch never resets invulnerable_time"
    );
    assert_eq!(last_hurt, 9.0);
    assert_close(
        "health after hit 4 (only the delta 5.0 dealt)",
        health,
        5.0,
        1e-6,
    );
}

#[test]
fn absorption_asymmetry_mob_vs_player() {
    let attributes = attributes_with(0.0, 0.0);

    // Mob path: a second subtraction from absorption.
    let mut health = 20.0_f32;
    let mut absorption = 3.0_f32;
    let mut invulnerable_time = 0_i32;
    let mut last_hurt = 0.0_f32;
    apply_damage_pipeline(
        DamageTarget {
            health: &mut health,
            max_health: 20.0,
            absorption: &mut absorption,
            invulnerable_time: &mut invulnerable_time,
            last_hurt: &mut last_hurt,
            is_player: false,
            is_creative: false,
            attributes: &attributes,
        },
        &mob_attack_source(),
        5.0,
        EnchantLevels::default(),
        EnchantLevels::default(),
    );
    assert_close("mob absorption", absorption, 0.0, 1e-6);
    assert_close("mob health", health, 18.0, 1e-6);

    // Player path: identical inputs, only ONE subtraction.
    let mut health = 20.0_f32;
    let mut absorption = 3.0_f32;
    let mut invulnerable_time = 0_i32;
    let mut last_hurt = 0.0_f32;
    apply_damage_pipeline(
        DamageTarget {
            health: &mut health,
            max_health: 20.0,
            absorption: &mut absorption,
            invulnerable_time: &mut invulnerable_time,
            last_hurt: &mut last_hurt,
            is_player: true,
            is_creative: false,
            attributes: &attributes,
        },
        &mob_attack_source(),
        5.0,
        EnchantLevels::default(),
        EnchantLevels::default(),
    );
    assert_close("player absorption", absorption, 0.0, 1e-6);
    assert_close("player health", health, 18.0, 1e-6);
}

#[test]
fn epf_and_armor_bypass_are_independent_for_fall_damage() {
    let attributes = attributes_with(20.0, 0.0);
    let mut health = 20.0_f32;
    let mut absorption = 0.0_f32;
    let mut invulnerable_time = 0_i32;
    let mut last_hurt = 0.0_f32;

    let target_epf_enchants = EnchantLevels {
        feather_falling: 4,
        ..Default::default()
    };

    let outcome = apply_damage_pipeline(
        DamageTarget {
            health: &mut health,
            max_health: 20.0,
            absorption: &mut absorption,
            invulnerable_time: &mut invulnerable_time,
            last_hurt: &mut last_hurt,
            is_player: true,
            is_creative: false,
            attributes: &attributes,
        },
        &fall_source(),
        10.0,
        EnchantLevels::default(),
        target_epf_enchants,
    );
    // Armor (20.0) has zero effect (bypassed entirely); EPF (feather_falling_epf(4) = 12.0)
    // still reduces the damage: 10.0 * (1.0 - 12.0/25.0) = 5.2.
    let expected_damage = 10.0 * (1.0 - feather_falling_epf(4) / 25.0);
    assert!(matches!(outcome, DamageOutcome::Dealt { .. }));
    assert_close(
        "fall damage reflects EPF but not armor",
        20.0 - health,
        expected_damage,
        1e-5,
    );
}

#[test]
fn difficulty_scaling_three_branches() {
    let mob_source = mob_attack_source();
    assert_eq!(
        difficulty_scale_incoming(10.0, Difficulty::Peaceful, &mob_source),
        0.0
    );
    assert_close(
        "easy",
        difficulty_scale_incoming(10.0, Difficulty::Easy, &mob_source),
        6.0,
        1e-6,
    );
    assert_close(
        "hard",
        difficulty_scale_incoming(10.0, Difficulty::Hard, &mob_source),
        15.0,
        1e-6,
    );
    assert_close(
        "normal",
        difficulty_scale_incoming(10.0, Difficulty::Normal, &mob_source),
        10.0,
        1e-6,
    );

    // PlayerAttack (causing_entity_is_living_non_player: false) never scales, even on Hard.
    let player_source = player_attack_source();
    assert_close(
        "player-vs-player never scales",
        difficulty_scale_incoming(10.0, Difficulty::Hard, &player_source),
        10.0,
        1e-6,
    );
}

#[test]
fn creative_player_is_fully_invulnerable() {
    let attributes = attributes_with(0.0, 0.0);
    let mut health = 20.0_f32;
    let mut absorption = 0.0_f32;
    let mut invulnerable_time = 0_i32;
    let mut last_hurt = 0.0_f32;

    let outcome = apply_damage_pipeline(
        DamageTarget {
            health: &mut health,
            max_health: 20.0,
            absorption: &mut absorption,
            invulnerable_time: &mut invulnerable_time,
            last_hurt: &mut last_hurt,
            is_player: true,
            is_creative: true,
            attributes: &attributes,
        },
        &mob_attack_source(),
        1000.0,
        EnchantLevels::default(),
        EnchantLevels::default(),
    );
    assert_eq!(outcome, DamageOutcome::Invulnerable);
    assert_close("health unchanged", health, 20.0, 1e-6);
    assert_close("absorption unchanged", absorption, 0.0, 1e-6);
    assert_eq!(invulnerable_time, 0, "invulnerability window not consumed");
}
