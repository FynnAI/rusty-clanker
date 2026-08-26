use bevy_ecs::prelude::Component;

/// The chunk-entity identity tag (WORLD-D1). Wraps `rc_core::ChunkKey` — ARCH-D24's
/// `{dimension, x, z}` shape, completely unmodified — in a local newtype so it can
/// derive `bevy_ecs::component::Component` without adding a `bevy_ecs` dependency to
/// `rc-core` itself (Context). Storage class: `Table`.
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChunkKeyTag(pub rc_core::ChunkKey);
