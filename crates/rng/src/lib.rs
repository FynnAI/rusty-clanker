//! `rc-rng` — the shared bit-exact Java-RNG stack (`12-workspace-structure.md`'s WS-D14).
//!
//! **Forward-pulled, minimal scope (M4-B02, Context §K).** `M5-B01` (Milestone 5) is this
//! crate's real owning blueprint and will eventually deliver the complete stack: vanilla's
//! legacy 48-bit LCG family, `WorldgenRandom`'s seed-derivation hierarchy, both positional
//! RNG factories, and seed-string parsing (`docs/findings-for-planning.md` records this
//! scope split). `M4-B02`'s own Context §K flags a narrower, explicitly accepted exception
//! to `PLAN-D2`'s milestone-readiness gate: only `RcXoroshiroRandom`, its `RcRandomSource`
//! trait surface, and the `random_sequence` seeding formula (`create_random_sequence`/
//! `create_random_sequence_default`) are pulled forward now, as the one RNG family
//! `rc-mechanics`' own loot-roll engine needs — restated fully, bit-exact, from
//! `docs/research/third-party/rng-parity-notes.md` §3/§5.2 (the same source `M5-B01`'s own
//! fuller implementation cites). Every other `M5-B01` deliverable (`RcLegacyRandom`,
//! `WorldgenRandom`, `LegacyPositionalFactory`/`XoroshiroPositionalFactory`,
//! `parse_seed_string`, `java_string_hash_code`, `mth_get_seed`, `seed_slime_chunk`,
//! `next_gaussian`) stays exactly that blueprint's own future scope — not implemented here.

pub mod xoroshiro;

pub use xoroshiro::{
    GOLDEN_RATIO_64, RcRandomSource, RcXoroshiroRandom, SILVER_RATIO_64, create_random_sequence,
    create_random_sequence_default, mix_stafford13, upgrade_seed_128_unmixed,
};
