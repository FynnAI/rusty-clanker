//! `rc-registries` — Canonical block-state/item/biome/entity-type/dimension registry types + generated tables. Also serves as the client's world-model data.
//!
//! M0 scaffold placeholder (M0-B01). Real types land in a later M0 blueprint.

/// M0-B07's codegen output (`crates/registries/generated/v776/`), wired into this crate's
/// module tree for the first time by M1-B05. Covers only the *static*, built-in registries
/// enumerated by the pinned version's `registries.json` report (blocks, items, particle
/// types, …) plus the block-state table — `minecraft:dimension_type` and
/// `minecraft:worldgen/biome` are dynamic/datapack registries with no fixed id space in that
/// report and are therefore absent here; a caller needing either synchronizes them itself
/// (`rusty_clanker_server::net::run_configuration`'s own `worldgen_registries` parameter).
#[path = "../generated/v776/mod.rs"]
pub mod generated_v776;
