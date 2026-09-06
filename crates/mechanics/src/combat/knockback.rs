//! The two-impulse knockback model (M4-B05 Context, "Knockback," exact).

use rc_physics::Vec3;

use crate::random::RcRandom;

/// The double-widened float threshold `9.999999747378752e-6` — the reference compares
/// against the float literal `1.0E-5F` widened to double, NOT the double literal `1e-5`
/// (Context's own cited correction) — computed via the `f32 -> f64` cast itself, rather than
/// transcribed as a decimal literal, so this constant can never silently drift from that cast.
const KNOCKBACK_DEGENERATE_THRESHOLD: f64 = 1.0e-5_f32 as f64;

/// `(attacker.AttributeMap[AttackKnockback] + knockback_enchant_bonus(level)) / 2.0` —
/// Context, exact.
pub fn get_knockback(attacker_attack_knockback: f64, knockback_enchant_level: u8) -> f64 {
    (attacker_attack_knockback + knockback_enchant_bonus(knockback_enchant_level)) / 2.0
}

/// `1.0 + 1.0 * (level - 1)` if `level > 0`, else `0.0` (Context, exact — `linear(base=1,
/// per_level=1)`).
fn knockback_enchant_bonus(level: u8) -> f64 {
    if level == 0 {
        0.0
    } else {
        1.0 + 1.0 * (level as f64 - 1.0)
    }
}

/// One impulse application (Context, "Knockback" — called twice per hit with different
/// `(power, dir_xz)` inputs, never merged into one call, reproducing vanilla's own
/// velocity-halving-per-call behavior). `rng` is only consulted on the rare
/// degenerate-direction fallback branch.
pub fn apply_knockback_impulse(
    velocity: Vec3,
    power: f64,
    dir_xz: (f64, f64),
    on_ground: bool,
    knockback_resistance: f64,
    rng: &mut RcRandom,
) -> Vec3 {
    let power = power * (1.0 - knockback_resistance);
    if power <= 0.0 {
        return velocity;
    }

    let (mut xd, mut zd) = dir_xz;
    // Re-drawn until the sample clears the threshold -- a `while`, not an `if` (Context's own
    // cited correction): RNG consumption is 4 draws per iteration, unbounded, not fixed.
    while xd * xd + zd * zd < KNOCKBACK_DEGENERATE_THRESHOLD {
        xd = (rng.next_double() - rng.next_double()) * 0.01;
        zd = (rng.next_double() - rng.next_double()) * 0.01;
    }

    let len = (xd * xd + zd * zd).sqrt();
    let dx = (xd / len) * power;
    let dz = (zd / len) * power;

    let new_vy = if on_ground {
        (velocity.y / 2.0 + power).min(0.4)
    } else {
        velocity.y
    };

    Vec3::new(velocity.x / 2.0 - dx, new_vy, velocity.z / 2.0 - dz)
}
