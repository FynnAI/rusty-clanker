use bevy_ecs::prelude::Component;

/// WORLD-D23's own literal field pair. Storage class: `Table`.
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ChunkPersistenceState {
    pub dirty: bool,
    pub last_saved_tick: u64,
}

impl ChunkPersistenceState {
    pub fn new() -> Self {
        todo!()
    }
    /// Clears `dirty` and records `tick` as the last-saved tick.
    pub fn mark_saved(&mut self, tick: u64) {
        todo!()
    }
    pub fn mark_dirty(&mut self) {
        todo!()
    }
}
