//! M4-B05 acceptance tests: the 1.9+ attack-cooldown charge curve, critical hits, the
//! enchant-bonus tables, sweep ratio, and the player/mob melee-damage assembly functions.

use rc_mechanics::combat::attributes::{AttributeKind, default_attributes_for};
use rc_mechanics::combat::damage::EnchantLevels;
use rc_mechanics::combat::melee::{
    assemble_mob_melee_damage, assemble_player_melee_damage, attack_cooldown_charge_scale,
    bane_bonus, can_critical_attack, sharpness_bonus, smite_bonus, sweeping_edge_ratio,
};
use rc_mechanics::entity::EntityKind;

fn assert_close(what: &str, got: f32, want: f32, tol: f32) {
    assert!(
        (got - want).abs() < tol,
        "{what}: got {got}, want {want} (within {tol})"
    );
}

#[test]
fn attack_cooldown_charge_curve_golden_vectors() {
    // attack_speed = 4.0 -> delay = 5.0 ticks.
    let cases: [(u32, f32, f32); 7] = [
        (0, 0.1, 0.208),
        (1, 0.3, 0.272),
        (2, 0.5, 0.4),
        (3, 0.7, 0.592),
        (4, 0.9, 0.848),
        (5, 1.0, 1.0),
        (10, 1.0, 1.0),
    ];
    for (ticker, expected_charge, expected_scale_factor) in cases {
        let charge = attack_cooldown_charge_scale(ticker, 4.0, 0.5);
        assert_close(
            &format!("charge at ticker={ticker}"),
            charge,
            expected_charge,
            1e-6,
        );
        let scale_factor = 0.2 + charge * charge * 0.8;
        assert_close(
            &format!("base_damage_scale_factor at ticker={ticker}"),
            scale_factor,
            expected_scale_factor,
            1e-6,
        );
    }
}

#[test]
fn critical_hit_requires_all_five_conditions() {
    // All five conditions hold -> true.
    assert!(can_critical_attack(1.0, false, false, false, true, false));

    // Each sub-case flips exactly one condition to false.
    assert!(!can_critical_attack(0.0, false, false, false, true, false)); // fall_distance > 0
    assert!(!can_critical_attack(1.0, true, false, false, true, false)); // !on_ground
    assert!(!can_critical_attack(1.0, false, true, false, true, false)); // !in_water
    assert!(!can_critical_attack(1.0, false, false, false, false, false)); // target_is_living
    assert!(!can_critical_attack(1.0, false, false, false, true, true)); // !is_sprinting
    // on_climbable, held for completeness even though the five-condition prose only names
    // five terms explicitly -- Context's own conjunction includes it too.
    assert!(!can_critical_attack(1.0, false, false, true, true, false));
}

#[test]
fn sharpness_smite_bane_golden_table() {
    let sharpness_expected = [0.0, 1.0, 1.5, 2.0, 2.5, 3.0];
    for (level, expected) in sharpness_expected.into_iter().enumerate() {
        assert_close(
            &format!("sharpness_bonus({level})"),
            sharpness_bonus(level as u8),
            expected,
            1e-6,
        );
    }

    let smite_bane_expected = [0.0, 2.5, 5.0, 7.5];
    for (level, expected) in smite_bane_expected.into_iter().enumerate() {
        assert_close(
            &format!("smite_bonus({level})"),
            smite_bonus(level as u8),
            expected,
            1e-6,
        );
        assert_close(
            &format!("bane_bonus({level})"),
            bane_bonus(level as u8),
            expected,
            1e-6,
        );
    }
}

#[test]
fn sweeping_edge_ratio_golden_table() {
    let expected = [0.0, 0.5, 0.6666667, 0.75];
    for (level, want) in expected.into_iter().enumerate() {
        assert_close(
            &format!("sweeping_edge_ratio({level})"),
            sweeping_edge_ratio(level as u8),
            want,
            1e-6,
        );
    }
}

#[test]
fn player_melee_full_assembly_undead_target_with_smite() {
    let mut attributes = default_attributes_for(EntityKind::Zombie);
    attributes.set_base(AttributeKind::AttackDamage, 7.0);

    let enchants = EnchantLevels {
        smite: 2,
        ..Default::default()
    };

    let result = assemble_player_melee_damage(
        &attributes,
        5,     // ticker -> full charge (charge_scale = 1.0)
        0.0,   // fall_distance -- not critical-eligible
        true,  // on_ground
        false, // in_water
        false, // on_climbable
        true,  // target_is_living
        false, // is_sprinting
        0.0,   // horizontal_speed_sq
        true,  // is_undead
        false, // is_arthropod
        false, // main_hand_is_sword -- structurally always false (no Sword ToolKind exists)
        enchants,
    );

    // magic_boost = 1.0 * (7.0 + smite_bonus(2) - 7.0) = 5.0 (Smite II = 2.5 + 2.5);
    // base_damage = 7.0 * 1.0 (full charge) = 7.0; total = 12.0.
    assert_close("total_damage", result.total_damage, 12.0, 1e-5);
    assert!(!result.is_critical);
}

#[test]
fn mob_melee_deterministic_no_rng_no_charge() {
    let mut attributes = default_attributes_for(EntityKind::Zombie);
    attributes.set_base(AttributeKind::AttackDamage, 3.0);

    let first = assemble_mob_melee_damage(&attributes, EnchantLevels::default());
    let second = assemble_mob_melee_damage(&attributes, EnchantLevels::default());
    assert_eq!(
        first.to_bits(),
        second.to_bits(),
        "bit-identical across calls"
    );
    assert_eq!(first, 3.0);
}
