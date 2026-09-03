//! Xoroshiro128++ (vanilla's modern RNG family) and the `random_sequence` (loot) seeding
//! formula, restated bit-exact from `docs/research/third-party/rng-parity-notes.md` §3/§5.2,
//! as consumed by `M4-B02`'s Context §K (`crates/rng/src/lib.rs`'s own module doc comment has
//! the full forward-pull scope note).

/// `rng-parity-notes.md` §3.1, `0x9E3779B97F4A7C15`.
pub const GOLDEN_RATIO_64: i64 = -7046029254386353131;
/// `rng-parity-notes.md` §3.1, `0x6A09E667F3BCC909`.
pub const SILVER_RATIO_64: i64 = 7640891576956012809;

const STAFFORD_MUL_1: i64 = -4658895280553007687;
const STAFFORD_MUL_2: i64 = -7723592293110705685;

/// Stafford variant 13 finalizer (`rng-parity-notes.md` §3.1/§6, restated exactly): three
/// wrapping multiplies interleaved with three UNSIGNED (logical) right-shifts.
pub fn mix_stafford13(z_in: i64) -> i64 {
    let mut z = z_in;
    z = (z ^ ((z as u64) >> 30) as i64).wrapping_mul(STAFFORD_MUL_1);
    z = (z ^ ((z as u64) >> 27) as i64).wrapping_mul(STAFFORD_MUL_2);
    z ^ ((z as u64) >> 31) as i64
}

/// Upgrades a legacy 64-bit seed to the unmixed 128-bit Xoroshiro seed pair (`rng-parity-
/// notes.md` §3.1): `lo = seed XOR SILVER_RATIO_64`, `hi = wrapping_add(lo, GOLDEN_RATIO_64)`.
/// Does **not** apply `mix_stafford13` — callers that need the fully-mixed pair (a fresh
/// top-level `RcXoroshiroRandom::new`) apply it themselves afterward.
pub fn upgrade_seed_128_unmixed(legacy_seed: i64) -> (i64, i64) {
    let lo = legacy_seed ^ SILVER_RATIO_64;
    let hi = lo.wrapping_add(GOLDEN_RATIO_64);
    (lo, hi)
}

fn upgrade_seed_128(seed: i64) -> (i64, i64) {
    let (lo, hi) = upgrade_seed_128_unmixed(seed);
    (mix_stafford13(lo), mix_stafford13(hi))
}

/// Xoroshiro128++ (Blackman & Vigna), vanilla's modern default RNG family — two `i64` words
/// of state (`rng-parity-notes.md` §3.1/§3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RcXoroshiroRandom {
    lo: i64,
    hi: i64,
}

impl RcXoroshiroRandom {
    /// A fresh, top-level construction from one legacy-style `i64` seed — applies the full
    /// upgrade-then-mix pipeline (`upgrade_seed_128`: `upgrade_seed_128_unmixed` then
    /// `mix_stafford13` on both resulting words).
    pub fn new(seed: i64) -> Self {
        let (lo, hi) = upgrade_seed_128(seed);
        Self { lo, hi }
    }

    /// Constructs directly from an already-derived raw state pair — no further mixing is
    /// applied here; the caller (e.g. `create_random_sequence`) has already mixed if its own
    /// derivation requires it.
    pub fn from_raw_pair(lo: i64, hi: i64) -> Self {
        Self { lo, hi }
    }
}

/// The `next_*` surface both vanilla RNG families expose (`M5-B01`'s own fuller
/// `RcRandomSource`, Context §B, narrowed here to exactly the methods `M4-B02`'s own loot
/// engine and `random_sequence` support need — `next_gaussian` and the two `BitSource`-level
/// primitives stay `M5-B01`'s own future scope, `lib.rs`'s module doc comment).
pub trait RcRandomSource {
    fn next_long(&mut self) -> i64;
    fn next_int(&mut self) -> i32;
    /// Panics if `bound <= 0` (mirrors vanilla's own `IllegalArgumentException` — a genuine
    /// caller-error condition).
    fn next_int_bounded(&mut self, bound: i32) -> i32;
    fn next_bool(&mut self) -> bool;
    /// Uniform `[0.0, 1.0)`.
    fn next_float(&mut self) -> f32;
    /// Uniform `[0.0, 1.0)`.
    fn next_double(&mut self) -> f64;
}

impl RcRandomSource for RcXoroshiroRandom {
    /// The core xoroshiro128++ step (`rng-parity-notes.md` §3.2): output rotation 17, `s0`
    /// rotation 49, XOR-shift 21, `s1` rotation 28 — canonical reference constants, restated
    /// exactly.
    fn next_long(&mut self) -> i64 {
        let (s0, s1) = (self.lo, self.hi);
        let result = s0.wrapping_add(s1).rotate_left(17).wrapping_add(s0);
        let s1n = s1 ^ s0;
        let new_lo = s0.rotate_left(49) ^ s1n ^ (s1n << 21);
        let new_hi = s1n.rotate_left(28);
        self.lo = new_lo;
        self.hi = new_hi;
        result
    }

    /// Low 32 bits of a FRESH `next_long()` — a full state transition every call, the
    /// opposite end of the word from the legacy family's own top-bits `next_int` (`rng-
    /// parity-notes.md` §3.3, restated).
    fn next_int(&mut self) -> i32 {
        self.next_long() as i32
    }

    /// Lemire-style multiply-and-reject — genuinely different from the legacy family's own
    /// rejection loop, no power-of-two fast path (`rng-parity-notes.md` §3.3, restated
    /// exactly).
    fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(
            bound > 0,
            "RcXoroshiroRandom::next_int_bounded: bound must be positive"
        );
        let bound_u: u64 = (bound as u32) as u64;
        let mut random_bits: u64 = (self.next_int() as u32) as u64;
        let mut product = random_bits * bound_u;
        let mut fractional = product & 0xFFFF_FFFF;
        if fractional < bound_u {
            // Genuinely unsigned 32-bit remainder (`Integer.remainderUnsigned(-bound,
            // bound)`) — a signed remainder here silently produces the wrong threshold for
            // roughly half of all bound values (`rng-parity-notes.md` §3.3/§6).
            let threshold = (0u32.wrapping_sub(bound as u32) as u64) % bound_u;
            while fractional < threshold {
                random_bits = (self.next_int() as u32) as u64;
                product = random_bits * bound_u;
                fractional = product & 0xFFFF_FFFF;
            }
        }
        (product >> 32) as i32
    }

    fn next_bool(&mut self) -> bool {
        (self.next_long() & 1) != 0
    }

    /// `next_bits(24)` (top 24 of 64 bits from one fresh `next_long()`) scaled by the exact
    /// power of two `2f32.powi(-24)` — never the truncated decimal literal (`rng-parity-
    /// notes.md` §6 point 6).
    fn next_float(&mut self) -> f32 {
        let bits = (self.next_long() as u64) >> (64 - 24);
        (bits as f32) * (1.0f32 / (1u32 << 24) as f32)
    }

    /// `next_bits(53)` (top 53 of 64 bits from one fresh `next_long()`) scaled by the exact
    /// power of two `2f64.powi(-53)` — same rule as `next_float`.
    fn next_double(&mut self) -> f64 {
        let bits = (self.next_long() as u64) >> (64 - 53);
        (bits as f64) * (1.0f64 / (1u64 << 53) as f64)
    }
}

/// MD5 digest of `name`'s UTF-8 bytes, split into two BIG-ENDIAN `i64` halves (`rng-parity-
/// notes.md` §5.2/§3.4/§6 point 7) — `i64::from_be_bytes`, never `from_ne_bytes`/
/// `from_le_bytes`.
fn md5_seed(name: &str) -> (i64, i64) {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(name.as_bytes());
    let digest: [u8; 16] = hasher.finalize().into();
    let lo = i64::from_be_bytes(digest[0..8].try_into().expect("md5 digest is 16 bytes"));
    let hi = i64::from_be_bytes(digest[8..16].try_into().expect("md5 digest is 16 bytes"));
    (lo, hi)
}

/// The `random_sequence` (loot) seeding formula (`rng-parity-notes.md` §5.2/§3.4, restated
/// exactly): fold `salt` into the base seed (gated by `include_world_seed`) →
/// `upgrade_seed_128_unmixed` (not yet mixed) → optionally XOR in the sequence id's raw,
/// also-unmixed MD5 halves (gated by `include_sequence_id`) → `mix_stafford13` on BOTH
/// halves exactly once, at the very end, regardless of which optional steps ran. Always
/// resolves to Xoroshiro, never the legacy family, regardless of dimension.
pub fn create_random_sequence(
    sequence_id: &str,
    world_seed: i64,
    salt: i32,
    include_world_seed: bool,
    include_sequence_id: bool,
) -> RcXoroshiroRandom {
    let base = (if include_world_seed { world_seed } else { 0 }) ^ (salt as i64);
    let (mut lo, mut hi) = upgrade_seed_128_unmixed(base);
    if include_sequence_id {
        let (id_lo, id_hi) = md5_seed(sequence_id);
        lo ^= id_lo;
        hi ^= id_hi;
    }
    RcXoroshiroRandom::from_raw_pair(mix_stafford13(lo), mix_stafford13(hi))
}

/// This project's fixed per-world defaults: `salt = 0`, both `include_*` flags `true` — no
/// `/random`-command-equivalent exists at `M4`'s own scope.
pub fn create_random_sequence_default(sequence_id: &str, world_seed: i64) -> RcXoroshiroRandom {
    create_random_sequence(sequence_id, world_seed, 0, true, true)
}
