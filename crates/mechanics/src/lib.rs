//! `rc-mechanics` — concrete domain systems for every ARCH-D8 group (`05-game-mechanics.md`).
//! This blueprint (M3-B01) is the crate's first content: the Stage-4 block-update substrate.
//! ECS-agnostic core algorithms live behind `BlockWorldAccess`; the `bevy_ecs`/`rc-scheduler`
//! adapter lives in `stage4::ecs`, feature-gated `server-systems` (default).

pub mod behavior;
pub mod block_event;
pub mod border;
pub mod direction;
pub mod neighbor_update;
pub mod random;
pub mod scheduled_tick;
#[cfg(feature = "server-systems")]
pub mod stage4;
pub mod world_access;

pub use behavior::{BlockBehavior, BlockBehaviorRegistry, NoOpBehavior, UpdateContext};
pub use block_event::{BlockEvent, BlockEventQueue};
pub use border::{BorderHalo, RegionOwnership};
pub use direction::Direction;
pub use neighbor_update::{NeighborUpdateEngine, PendingUpdate};
pub use random::{RcRandom, chunk_random_seed};
pub use scheduled_tick::{ScheduledTickEntry, ScheduledTickQueue, TickPriority};
pub use world_access::BlockWorldAccess;
