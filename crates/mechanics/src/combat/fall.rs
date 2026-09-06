//! Fall damage (M4-B05 Context, "Fall damage," §3.15, exact).

/// `fall_power = fall_distance + 1e-6 - safe_fall_distance`;
/// `fall_damage = floor(fall_power * damage_modifier * fall_damage_multiplier)` — an integer
/// result (never negative — caller discards non-positive results without applying any
/// damage). `damage_modifier` is fixed at vanilla's own default `1.0` at this blueprint's own
/// scope (Context: per-block landing modifiers like Bed/Hay Bale/Slime Block/Powder Snow are
/// not implemented, deferred to a future block-behavior blueprint).
pub fn calculate_fall_damage(
    fall_distance: f64,
    damage_modifier: f32,
    safe_fall_distance: f64,
    fall_damage_multiplier: f64,
) -> i32 {
    let fall_power = fall_distance + 1e-6 - safe_fall_distance;
    (fall_power * damage_modifier as f64 * fall_damage_multiplier).floor() as i32
}
