use bevy_ecs::prelude::Component;

/// A single placeholder covering every not-yet-`Full` rung of vanilla's real 12-rung
/// generation ladder (Context — `04-worldgen-parity.md` has not landed; this is the
/// minimal distinction WORLD-D22's own load/generate routing needs to exist
/// structurally today).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChunkGenStatus {
    Generating,
    Full,
}

/// WORLD-D1's `ChunkStatus` storage slot — `04` owns every value's meaning, this crate
/// only persists/exposes it. Storage class: `Table`.
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChunkStatus(pub ChunkGenStatus);
