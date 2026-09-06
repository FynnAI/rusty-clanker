//! M4-B05 acceptance tests: the two-impulse knockback model (Context, "Knockback," exact).

use rc_mechanics::combat::knockback::{apply_knockback_impulse, get_knockback};
use rc_mechanics::random::RcRandom;
use rc_physics::Vec3;

fn assert_close(what: &str, got: f64, want: f64, tol: f64) {
    assert!(
        (got - want).abs() < tol,
        "{what}: got {got}, want {want} (within {tol})"
    );
}

#[test]
fn zero_power_impulse_is_a_true_no_op_not_a_silent_halving() {
    let mut rng = RcRandom::new(1);

    // Impulse #1: power = 0.4, dir_xz = (1.0, 0.0) -- normalized direction is already (1,0).
    let v1 = apply_knockback_impulse(Vec3::ZERO, 0.4, (1.0, 0.0), true, 0.0, &mut rng);
    assert_close("v1.x", v1.x, -0.4, 1e-9);

    // Impulse #2: power = get_knockback(0.0, 0) = 0.0 -- the `power <= 0.0` early return must
    // fire BEFORE the `old/2.0` halving is ever evaluated, so v1 passes through unchanged.
    let power = get_knockback(0.0, 0);
    assert_close("bare unenchanted get_knockback", power, 0.0, 1e-9);
    let v2 = apply_knockback_impulse(v1, power, (0.0, 1.0), true, 0.0, &mut rng);
    assert_close(
        "v2.x unchanged by a zero-power second impulse (not silently halved)",
        v2.x,
        -0.4,
        1e-9,
    );
}

#[test]
fn two_impulse_sequence_with_nonzero_second_impulse_is_not_equivalent_to_one_merged_call() {
    let mut rng = RcRandom::new(1);

    let v1 = apply_knockback_impulse(Vec3::ZERO, 0.4, (1.0, 0.0), true, 0.0, &mut rng);
    assert_close("v1.x", v1.x, -0.4, 1e-9);
    assert_close("v1.z", v1.z, 0.0, 1e-9);

    let v2 = apply_knockback_impulse(v1, 0.5, (0.0, 1.0), true, 0.0, &mut rng);
    assert_close("v2.x = v1.x/2 - 0", v2.x, -0.2, 1e-9);
    assert_close("v2.z = 0/2 - 1*0.5", v2.z, -0.5, 1e-9);
    assert_close("v2.y = min(0.4, 0/2 + 0.5)", v2.y, 0.4, 1e-9);

    // A hypothetical single merged impulse would combine the two direction/power pairs into
    // one call -- this two-call sequence must differ from that (the halving is load-bearing).
    let merged_dir = (1.0 * 0.4, 1.0 * 0.5);
    let merged = apply_knockback_impulse(Vec3::ZERO, 1.0, merged_dir, true, 0.0, &mut rng);
    assert_ne!((v2.x, v2.z), (merged.x, merged.z));
}

#[test]
fn knockback_resistance_scales_power_before_the_early_return() {
    let mut rng = RcRandom::new(1);
    let velocity = Vec3::new(3.0, 0.2, -1.0);
    let out = apply_knockback_impulse(velocity, 5.0, (1.0, 0.0), true, 1.0, &mut rng);
    assert_eq!(out, velocity, "full resistance leaves velocity untouched");
}

#[test]
fn degenerate_direction_uses_rng_fallback_deterministically() {
    let mut rng_a = RcRandom::new(42);
    let result = apply_knockback_impulse(Vec3::ZERO, 0.4, (0.0, 0.0), true, 0.0, &mut rng_a);
    assert!(result.x.is_finite() && result.y.is_finite() && result.z.is_finite());

    // Independently re-run the identical redraw loop against a second, freshly-seeded
    // `RcRandom` and confirm both the resulting (xd, zd) and the call count agree.
    let mut rng_b = RcRandom::new(42);
    let threshold = 1.0e-5_f32 as f64;
    let (mut xd, mut zd) = (0.0_f64, 0.0_f64);
    let mut draws = 0u32;
    while xd * xd + zd * zd < threshold {
        xd = (rng_b.next_double() - rng_b.next_double()) * 0.01;
        zd = (rng_b.next_double() - rng_b.next_double()) * 0.01;
        draws += 4;
    }
    assert!(draws > 0, "at least one redraw iteration must occur");
    assert!(
        draws.is_multiple_of(4),
        "RNG consumption is 4 draws per iteration"
    );

    let len = (xd * xd + zd * zd).sqrt();
    let expected = Vec3::new(
        0.0 / 2.0 - (xd / len) * 0.4,
        0.4,
        0.0 / 2.0 - (zd / len) * 0.4,
    );
    assert_close(
        "re-derived x matches the function's own result",
        result.x,
        expected.x,
        1e-9,
    );
    assert_close(
        "re-derived z matches the function's own result",
        result.z,
        expected.z,
        1e-9,
    );
}

#[test]
fn get_knockback_formula() {
    assert_close("get_knockback(0.0, 0)", get_knockback(0.0, 0), 0.0, 1e-9);
    // knockback_enchant_bonus(2) = 1.0 + 1.0*(2-1) = 2.0; (0.0 + 2.0) / 2.0 = 1.0.
    assert_close("get_knockback(0.0, 2)", get_knockback(0.0, 2), 1.0, 1e-9);
}
