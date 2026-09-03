use bevy_ecs::prelude::Component;

/// `SECTION_COUNT + 2` (WORLD-D8's padding — one section below the lowest real block
/// section, one above the highest).
pub const LIGHT_SECTION_COUNT: usize = crate::column::SECTION_COUNT + 2;

/// One light channel's nibble-packed state for one 16³ section (WORLD-D8, amended).
/// Three states, matching vanilla's own `DataLayer` structural distinctions exactly:
/// `Uninitialized` is vanilla's own "not yet initialized" shortcut -- no `DataLayer`
/// object at all for this section/channel; `Filled(v)` is vanilla's own
/// allocated-but-implicit-value layer whose backing array was never materialized
/// (`v == 0` is exactly vanilla's own structural "empty" case -- a chunk-packet light
/// mask consumer dispatches on this variant alone, never a scan of the 4096 nibbles);
/// `Data` is a fully materialized, heterogeneous per-nibble array.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum LightNibbles {
    #[default]
    Uninitialized,
    Filled(u8),
    Data(Box<[u8; 2048]>),
}

/// One 16³ light section's nibble-packed sky/block state (WORLD-D8, amended).
#[derive(Clone, Debug, Default)]
pub struct LightSection {
    pub sky: LightNibbles,
    pub block: LightNibbles,
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
        Self {
            sections: (0..LIGHT_SECTION_COUNT)
                .map(|_| LightSection::default())
                .collect(),
        }
    }

    pub fn sections(&self) -> &[LightSection] {
        &self.sections
    }
    pub fn sections_mut(&mut self) -> &mut [LightSection] {
        &mut self.sections
    }
    pub fn section(&self, index: usize) -> &LightSection {
        &self.sections[index]
    }
    pub fn section_mut(&mut self, index: usize) -> &mut LightSection {
        &mut self.sections[index]
    }
}
