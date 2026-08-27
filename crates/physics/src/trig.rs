//! Vanilla's own `Mth.sin`/`Mth.cos` 65536-entry lookup table (Context: exact algorithm,
//! 18-float-determinism.md §3.1/§4). Built once, lazily, on first use.

use std::sync::OnceLock;

/// Table size (18 §4).
const SIN_QUANTIZATION: usize = 65536;
/// `0xFFFF` -- masks a scaled angle down to a table index.
const SIN_MASK: u32 = 65535;
/// Quarter-turn in table units -- `cos` is `sin` read this many slots ahead, internally
/// consistent by construction, never independently rounded.
const COS_OFFSET: f64 = 16384.0;
/// `65536 / (2*PI)` -- converts radians to table units.
const SIN_SCALE: f64 = 10430.378350470453;

static SIN_TABLE: OnceLock<Box<[f32; SIN_QUANTIZATION]>> = OnceLock::new();

fn table() -> &'static [f32; SIN_QUANTIZATION] {
    todo!()
}

/// Angular resolution ~0.0055 deg. Input `angle_radians: f64`, multiply in `f64`, truncate
/// (not round) to `i64`, mask to `u16` index, return `f32` -- exact type discipline per
/// 18-float-determinism.md §3.9.
pub fn mth_sin(angle_radians: f64) -> f32 {
    todo!()
}

pub fn mth_cos(angle_radians: f64) -> f32 {
    todo!()
}
