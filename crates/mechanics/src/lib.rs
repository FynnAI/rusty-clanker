//! `rc-mechanics` — concrete domain systems for every ARCH-D8 group (`05-game-mechanics.md`).
//! This blueprint (M3-B01) is the crate's first content: the Stage-4 block-update substrate.
//! ECS-agnostic core algorithms live behind `BlockWorldAccess`; the `bevy_ecs`/`rc-scheduler`
//! adapter lives in `stage4::ecs`, feature-gated `server-systems` (default).

pub mod behavior;
pub mod block_entity;
pub mod block_event;
pub mod border;
pub mod container;
pub mod direction;
pub mod item_stack;
pub mod neighbor_update;
pub mod random;
pub mod random_tick;
pub mod redstone;
pub mod scheduled_tick;
#[cfg(feature = "server-systems")]
pub mod stage4;
#[cfg(feature = "server-systems")]
pub mod stage5;
#[cfg(feature = "server-systems")]
pub mod stage7;
pub mod world_access;

pub use behavior::{
    BlockBehavior, BlockBehaviorRegistry, NoOpBehavior, RandomTickContext, UpdateContext,
};
pub use block_entity::{
    BlockEntityHeader, BlockEntityKind, BlockEntityWorldAccess,
    chest::ChestBlockEntity,
    container_signal_source::{ContainerSignalsResource, Tier1ContainerSignalSource},
    furnace::{
        FuelTable, FurnaceBlockEntity, FurnaceLitStateResolver, SmeltingRecipe, SmeltingRecipeTable,
    },
    hopper::HopperBlockEntity,
};
pub use block_event::{BlockEvent, BlockEventQueue};
pub use border::{BorderHalo, RegionOwnership};
pub use container::{
    DefaultMaxStackSize, ItemMaxStackSize, MaxStackSizeResource, TierOneContainer,
    comparator_signal_from_slots, decrement_or_clear, find_leftmost_extract_slot,
    find_leftmost_insert_slot, move_one_item, place_or_stack_output,
};
pub use direction::Direction;
pub use item_stack::{item_stack_from_nbt, item_stack_to_nbt};
pub use neighbor_update::{NeighborUpdateEngine, PendingUpdate};
pub use random::{RcRandom, chunk_random_seed};
pub use random_tick::{
    DEFAULT_RANDOM_TICK_SPEED, RandomTickPosition, WorldSeed, draw_random_tick_positions,
};
pub use scheduled_tick::{ScheduledTickEntry, ScheduledTickQueue, TickPriority};
pub use world_access::BlockWorldAccess;
