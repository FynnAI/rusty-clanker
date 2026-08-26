use bevy_ecs::prelude::Component;

/// `SECTION_COUNT + 2` (WORLD-D8's padding — one section below the lowest real block
/// section, one above the highest).
pub const LIGHT_SECTION_COUNT: usize = crate::column::SECTION_COUNT + 2;

/// One 16³ light section's nibble-packed sky/block arrays. `None` = vanilla's own
/// "not yet initialized" shortcut (WORLD-D8).
#[derive(Clone, Debug, Default)]
pub struct LightSection {
    pub sky: Option<Box<[u8; 2048]>>,
    pub block: Option<Box<[u8; 2048]>>,
}

/// Stored light data only (WORLD-D8) — no BFS propagator, no cross-chunk seeding
/// (WORLD-D7/D9/D10 are explicitly out of this blueprint's scope, Context). Storage
/// class: `Table`.
#[derive(Component, Clone)]
pub struct LightColumn {
    sections: Vec<LightSection>,
}

impl LightColumn {
    /// `LIGHT_SECTION_COUNT` sections, every one `LightSection::default()`
    /// (uninitialized).
    pub fn new_uninitialized() -> Self {
        todo!()
    }

    pub fn sections(&self) -> &[LightSection] {
        todo!()
    }
    pub fn sections_mut(&mut self) -> &mut [LightSection] {
        todo!()
    }
    pub fn section(&self, index: usize) -> &LightSection {
        todo!()
    }
    pub fn section_mut(&mut self, index: usize) -> &mut LightSection {
        todo!()
    }
}
