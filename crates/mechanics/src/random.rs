//! `RcRandom` — a bit-exact port of `java.util.Random`'s 48-bit LCG (MECH-D5), restated in
//! full by this blueprint's own Context section from `docs/research/third-party/
//! rng-parity-notes.md` §1. `chunk_random_seed` is ARCH-D14's own, explicitly non-vanilla,
//! per-chunk-per-tick seed derivation (Context: "this blueprint's own design freedom").

const MULTIPLIER: i64 = 0x5DEECE66D;
const ADDEND: i64 = 0xB;
const MASK: i64 = 0xFFFFFFFFFFFF;

/// Bit-exact `java.util.Random` 48-bit LCG (MECH-D5), restated in full in Context. No
/// `next_gaussian` — no M3 tier-1 consumer needs it.
#[derive(Clone, Debug)]
pub struct RcRandom {
    seed: i64,
}

impl RcRandom {
    pub fn new(seed: i64) -> Self {
        let mut r = Self { seed: 0 };
        r.set_seed(seed);
        r
    }

    pub fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ MULTIPLIER) & MASK;
    }

    /// The LCG's own single step, returning the top `bits` bits of the newly-advanced
    /// 48-bit state (Context: "unsigned/logical shift... always non-negative"). Every
    /// other `next_*` method is built from this one primitive, exactly as
    /// `java.util.Random` itself is.
    fn next(&mut self, bits: u32) -> i32 {
        self.seed = self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND) & MASK;
        ((self.seed as u64) >> (48 - bits)) as i32
    }

    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// Power-of-two fast path + rejection sampling (Context §1.5). Panics if `bound <= 0`.
    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(
            bound > 0,
            "RcRandom::next_int_bounded: bound must be positive"
        );

        if (bound & bound.wrapping_neg()) == bound {
            // Power-of-two fast path.
            return (((bound as i64).wrapping_mul(self.next(31) as i64)) >> 31) as i32;
        }

        loop {
            let bits = self.next(31);
            let val = bits % bound;
            // Rejection test, wrapping 32-bit signed arithmetic (Context, restated
            // exactly): reject (loop again) while `bits - val + (bound - 1) < 0`.
            if bits.wrapping_sub(val).wrapping_add(bound - 1) >= 0 {
                return val;
            }
        }
    }

    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    pub fn next_float(&mut self) -> f32 {
        (self.next(24) as f32) * (1.0f32 / (1u32 << 24) as f32)
    }

    pub fn next_double(&mut self) -> f64 {
        let high = (self.next(26) as i64) << 27;
        let low = self.next(27) as i64;
        (high.wrapping_add(low)) as f64 * (1.0f64 / (1u64 << 53) as f64)
    }

    pub fn next_bool(&mut self) -> bool {
        self.next(1) != 0
    }
}

const GOLDEN_RATIO_64: i64 = -7046029254386353131; // 0x9E3779B97F4A7C15, rng-parity-notes.md §3.1

/// `rng-parity-notes.md` §3.1, restated verbatim.
fn stafford_mix13(z_in: i64) -> i64 {
    let mut z = z_in;
    z = (z ^ ((z as u64) >> 30) as i64).wrapping_mul(-4658895280553007687);
    z = (z ^ ((z as u64) >> 27) as i64).wrapping_mul(-7723592293110705685);
    z ^ ((z as u64) >> 31) as i64
}

/// ARCH-D14's per-chunk-per-tick seed (Context: this blueprint's own, non-vanilla,
/// documented derivation — algorithm shape, not any specific LCG output, is the parity
/// requirement here).
pub fn chunk_random_seed(world_seed: i64, chunk_x: i32, chunk_z: i32, tick_counter: u64) -> i64 {
    let mut h = world_seed;
    h ^= (chunk_x as i64).wrapping_mul(341873128712);
    h ^= (chunk_z as i64).wrapping_mul(132897987541);
    h ^= (tick_counter as i64).wrapping_mul(GOLDEN_RATIO_64);
    stafford_mix13(h)
}
