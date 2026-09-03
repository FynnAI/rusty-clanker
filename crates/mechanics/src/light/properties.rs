//! Block-state light properties and the shape-occlusion veto (M4-B07 Context §3).
//! Simplified relative to vanilla's full geometric shape model -- `occludes_face` is a
//! per-direction boolean veto, not a `VoxelShape` union test -- since no shape/registry
//! source exists yet (mirrors M3-B01's `BlockBehaviorRegistry` "no generated registry"
//! resolution).

use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;

use crate::direction::Direction;

/// One block-state's light-relevant properties (Context §3).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LightProperties {
    pub block_emission: u8,
    pub opacity: u8,
    pub occludes_face: [bool; 6],
}

impl LightProperties {
    /// Fully transparent, non-emitting (the default for any unregistered state --
    /// matches vanilla's own air convention).
    pub const AIR: LightProperties = LightProperties {
        block_emission: 0,
        opacity: 0,
        occludes_face: [false; 6],
    };
    /// Fully solid, opaque, non-emitting (opacity 15, no shape veto needed since
    /// scalar opacity alone already blocks everything).
    pub const OPAQUE: LightProperties = LightProperties {
        block_emission: 0,
        opacity: 15,
        occludes_face: [false; 6],
    };

    /// `opacity.max(1)` -- `MIN_OPACITY` floor (Context §2/§3).
    pub fn get_opacity(self) -> u8 {
        self.opacity.max(1)
    }
}

/// `Direction`'s own declaration order, restated as a plain index function (this
/// crate does not add an `ordinal`/index method to `rc_mechanics::direction::
/// Direction` itself -- Constraints (d)). West=0, East=1, North=2, South=3, Down=4,
/// Up=5 -- the sole place a numeric index is derived from a `Direction` value;
/// every other file that needs one calls this function, never re-deriving its own
/// mapping.
pub fn direction_index(dir: Direction) -> usize {
    match dir {
        Direction::West => 0,
        Direction::East => 1,
        Direction::North => 2,
        Direction::South => 3,
        Direction::Down => 4,
        Direction::Up => 5,
    }
}

/// `true` iff `from_props`'s face in `dir`, or `to_props`'s face in `dir.opposite()`,
/// is declared to fully occlude the shared face (Context §3's veto formula).
pub fn shape_occludes(
    from_props: LightProperties,
    to_props: LightProperties,
    dir: Direction,
) -> bool {
    from_props.occludes_face[direction_index(dir)]
        || to_props.occludes_face[direction_index(dir.opposite())]
}

/// Range-based dispatch (mirrors `crate::behavior::BlockBehaviorRegistry` exactly --
/// M3-B01's own established pattern for "no generated registry yet"). `Resource`
/// (M4-B07 field-report note, not shown in the blueprint's own Deliverables snippet
/// but required for Context §8's own "reads `LightPropertiesRegistry`... as
/// Resources" `run_stage8_lighting` contract to actually compile/insert into a real
/// `bevy_ecs::World` -- recorded in `docs/findings-for-planning.md`).
#[derive(Clone, Default, Resource)]
pub struct LightPropertiesRegistry {
    ranges: Vec<(BlockStateId, BlockStateId, LightProperties)>,
}

impl LightPropertiesRegistry {
    pub fn new() -> Self {
        Self { ranges: Vec::new() }
    }

    /// Panics on overlap with an already-registered range (mirrors
    /// `BlockBehaviorRegistry::register_range` exactly).
    pub fn register_range(
        &mut self,
        start: BlockStateId,
        end_exclusive: BlockStateId,
        props: LightProperties,
    ) {
        let overlaps = self
            .ranges
            .iter()
            .any(|(s, e, _)| start < *e && *s < end_exclusive);
        assert!(
            !overlaps,
            "LightPropertiesRegistry::register_range: [{start:?}, {end_exclusive:?}) overlaps an already-registered range"
        );
        self.ranges.push((start, end_exclusive, props));
        self.ranges.sort_by_key(|(start, _, _)| *start);
    }

    pub fn register_one(&mut self, state: BlockStateId, props: LightProperties) {
        self.register_range(state, BlockStateId(state.0 + 1), props);
    }

    /// Returns the matching range's properties, or `LightProperties::AIR`.
    pub fn resolve(&self, state: BlockStateId) -> LightProperties {
        for (start, end_exclusive, props) in &self.ranges {
            if state >= *start && state < *end_exclusive {
                return *props;
            }
        }
        LightProperties::AIR
    }
}
