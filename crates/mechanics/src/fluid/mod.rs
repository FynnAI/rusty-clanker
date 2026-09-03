//! Water and lava flow (M4-B06): bit-exact spread/flow algorithm, MECH-D24.
//! ECS-agnostic core (`state`, `tables`, `occlusion`, `algorithm`, `reaction`, `spread`,
//! `waterlog`) built entirely over `crate::world_access::BlockWorldAccess` and
//! `crate::behavior::UpdateContext`, exactly mirroring M3-B01's own core/adapter split.
//! `behavior` is this module's single `BlockBehavior` adapter, registered into the *existing*
//! Stage-4 `BlockBehaviorRegistry` — no new Stage-4 system, no `rc-scheduler` change.

pub mod algorithm;
pub mod behavior;
pub mod occlusion;
pub mod reaction;
pub mod spread;
pub mod state;
pub mod tables;
pub mod waterlog;

pub use algorithm::{
    can_be_replaced_with, fluid_state_at, get_flow, get_height, get_new_liquid, get_own_height,
    get_spread,
};
pub use behavior::{FluidBehavior, register_fluids};
pub use state::{
    FLUID_HORIZONTAL_ORDER, FluidBlockRanges, FluidKind, FluidState, FluidVariant,
    LAVA_CONTACT_ORDER,
};
pub use tables::{
    BasaltConversion, FluidDimensionProfile, FluidGameRules, FluidTables, LevelRandom,
    ReactionBlocks,
};
pub use waterlog::{SimpleWaterlogged, WaterloggableBehavior, WaterloggableRegistry};
