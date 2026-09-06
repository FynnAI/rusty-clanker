//! M4-B10 (Context §E): the entity-presence census seam a pressure plate reads through --
//! mirrors `ContainerSignalSource`/`Tier1ContainerSignalSource` (M3-B06) in shape, ownership
//! and locking rationale. `rc-mechanics` structurally cannot see both `PlayerMarker` players
//! and `BaseEntity` mobs/items at once (WS-D3 rule 2 -- the same boundary M4-B02's own item-
//! pickup gap already hit), so this module ships only the trait and the `NoEntities` empty
//! default; the one production implementation is `rusty-clanker-server::play::entity_presence::
//! RegionEntityPresence`.

/// Which entity class a pressure plate counts (Context §D) -- vanilla's `Entity.class` vs
/// `LivingEntity.class` switch on `BlockSetType.pressurePlateSensitivity`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EntityClassFilter {
    AnyEntity,
    LivingOnly,
}

/// The census seam a pressure plate reads through (Context §E). Mirrors
/// `ContainerSignalSource`'s shape, ownership and locking rationale exactly. The one
/// production implementation lives in `rusty-clanker-server` (the only crate that can see
/// both players and `BaseEntity` entities); `rc-mechanics` ships only this trait and the
/// empty default below.
pub trait EntityPresenceSource: Send + Sync {
    /// Vanilla's `getEntityCount(level, box, class)`: owned (ARCH-D10), non-spectator,
    /// non-block-trigger-ignoring entities whose own AABB strictly intersects `region`.
    fn count_entities_in(&self, region: rc_physics::Aabb, filter: EntityClassFilter) -> usize;
}

/// Always `0` -- the composition-root default until a real census is wired, mirroring
/// `NoContainers`. A plate reading this never presses; it never panics.
pub struct NoEntities;

impl EntityPresenceSource for NoEntities {
    fn count_entities_in(&self, _region: rc_physics::Aabb, _filter: EntityClassFilter) -> usize {
        0
    }
}
