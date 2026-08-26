use bevy_ecs::prelude::{Component, Entity};

/// WORLD-D6's storage contract: a chunk's own placed-block-entity children, in
/// vanilla's own stable per-chunk load order (ARCH-D17). No `BlockEntityCodec`, no NBT
/// (de)serialization — `05-game-mechanics.md`'s job (Context). Storage class: `Table`.
#[derive(Component, Clone, Default)]
pub struct BlockEntityIndex {
    entities: Vec<Entity>,
}

impl BlockEntityIndex {
    pub fn new() -> Self {
        todo!()
    }
    /// Appends `entity` at the end of the load order (the caller is responsible for
    /// vanilla-matching order at the point of insertion — this type only preserves
    /// whatever order it is given).
    pub fn push(&mut self, entity: Entity) {
        todo!()
    }
    /// Removes the first occurrence of `entity`, if present, preserving the relative
    /// order of every remaining entry. Returns `true` iff an entry was removed.
    pub fn remove(&mut self, entity: Entity) -> bool {
        todo!()
    }
    pub fn entities(&self) -> &[Entity] {
        todo!()
    }
}
