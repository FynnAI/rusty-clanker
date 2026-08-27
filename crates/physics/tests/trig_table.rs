//! M3-B02 acceptance tests: vanilla's `Mth.sin`/`Mth.cos` lookup table is internally
//! consistent by construction (`cos` reads the same table a quarter-turn ahead of `sin`),
//! and lands near the well-known values at the angles every caller actually exercises.

use rc_physics::{mth_cos, mth_sin};

#[test]
fn sin_cos_are_internally_consistent() {
    let angles = [
        0.0,
        std::f64::consts::PI / 6.0,
        std::f64::consts::PI / 4.0,
        std::f64::consts::PI / 2.0,
        std::f64::consts::PI,
        3.0 * std::f64::consts::PI / 2.0,
    ];
    for angle in angles {
        let s = mth_sin(angle) as f64;
        let c = mth_cos(angle) as f64;
        assert!(
            (s * s + c * c - 1.0).abs() < 1e-3,
            "angle {angle}: sin={s} cos={c} sin^2+cos^2={}",
            s * s + c * c
        );
    }
}

#[test]
fn sin_zero_and_pi_half_are_near_expected() {
    assert!(mth_sin(0.0).abs() < 1e-3, "sin(0) = {}", mth_sin(0.0));
    let sin_half_pi = mth_sin(std::f64::consts::FRAC_PI_2) as f64;
    assert!(
        (sin_half_pi - 1.0).abs() < 1e-3,
        "sin(pi/2) = {sin_half_pi}"
    );
    let cos_zero = mth_cos(0.0) as f64;
    assert!((cos_zero - 1.0).abs() < 1e-3, "cos(0) = {cos_zero}");
}
