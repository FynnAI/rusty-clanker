//! The fluid state model (M4-B06 Context §A): a fluid cell has no storage of its own — its
//! `FluidState` is hosted entirely through the cell's `BlockState`, via the legacy `LEVEL`
//! property (`BlockStateProperties.LEVEL`, ranged `[0,15]`). Water and lava each own a
//! contiguous 16-wide `BlockStateId` range, one id per `LEVEL` value.

use rc_chunk_storage::BlockStateId;

use crate::direction::Direction;

/// The two vanilla `FlowingFluid` kinds this blueprint implements (Context §A).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FluidKind {
    Water,
    Lava,
}

/// Source (amount always 8, not stored) or Flowing (stored amount 1-8, plus the falling bit,
/// present on both variants in real vanilla but only meaningful — and only ever `true` — for
/// Flowing here, Context §A).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FluidVariant {
    Source,
    Flowing { amount: u8, falling: bool },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FluidState {
    pub kind: FluidKind,
    pub variant: FluidVariant,
}

impl FluidState {
    pub fn source(kind: FluidKind) -> Self {
        let _ = kind;
        todo!()
    }

    /// Panics (debug-only `debug_assert!`) if `amount` is outside `1..=8`.
    pub fn flowing(kind: FluidKind, amount: u8, falling: bool) -> Self {
        let _ = (kind, amount, falling);
        todo!()
    }

    pub fn is_source(self) -> bool {
        todo!()
    }

    pub fn falling(self) -> bool {
        todo!()
    }

    /// `8` for a source (Context §A: hardcoded, not stored).
    pub fn amount(self) -> u8 {
        todo!()
    }

    /// `amount as f32 / 9.0f32` (Context §A — `getOwnHeight`, float division).
    pub fn own_height(self) -> f32 {
        todo!()
    }

    /// Context §A's exact formula, restated: `Source => 0`, `Flowing{amount,falling} =>
    /// (8 - amount.min(8)) + if falling {8} else {0}`.
    pub fn to_legacy_level(self) -> u8 {
        todo!()
    }

    /// The documented vanilla quirk (Context §A): `level == 0` always decodes to `Source`,
    /// never to `Flowing{amount:8, falling:false}` even though both encode to the same level.
    pub fn from_legacy_level(kind: FluidKind, level: u8) -> Self {
        let _ = (kind, level);
        todo!()
    }
}

/// `Direction.Plane.HORIZONTAL` (Context §B) — reused by every core algorithm in this module.
/// Distinct from `crate::direction::{NEIGHBOR_CHANGED_ORDER, SHAPE_UPDATE_ORDER}`.
pub const FLUID_HORIZONTAL_ORDER: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

/// `LiquidBlock.POSSIBLE_FLOW_DIRECTIONS`'s effective checked order (Context §I(A)) — used only
/// by the lava+water contact-conversion scan, never by the ordinary spread algorithm.
pub const LAVA_CONTACT_ORDER: [Direction; 5] = [
    Direction::Up,
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

/// A fluid's own contiguous 16-wide `BlockStateId` range, `(start, end_exclusive)`, one id per
/// legacy `LEVEL` value 0-15 (Context §A; range *width* high-confidence, id *ordering within
/// the range* moderate-confidence, flagged for reconciliation).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FluidBlockRanges {
    pub water: (BlockStateId, BlockStateId),
    pub lava: (BlockStateId, BlockStateId),
}

impl FluidBlockRanges {
    /// `None` if either range is not exactly 16 wide (a constructor-time sanity check, not a
    /// vanilla rule) — every composition root/test must supply exactly-16-wide ranges.
    pub fn new(
        water: (BlockStateId, BlockStateId),
        lava: (BlockStateId, BlockStateId),
    ) -> Option<Self> {
        let _ = (water, lava);
        todo!()
    }

    pub fn to_block_state_id(&self, state: FluidState) -> BlockStateId {
        let _ = state;
        todo!()
    }

    pub fn kind_of(&self, id: BlockStateId) -> Option<FluidKind> {
        let _ = id;
        todo!()
    }

    pub fn state_of(&self, id: BlockStateId) -> Option<FluidState> {
        let _ = id;
        todo!()
    }
}
