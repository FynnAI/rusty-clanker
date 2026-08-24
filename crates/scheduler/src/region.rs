//! One region's `World` plus per-system instances and `rc-messaging` state.

use bevy_ecs::system::System;
use bevy_ecs::world::World;
use rc_messaging::{RegionId, RegionMessageState};

/// One region's `bevy_ecs::World` plus its per-system instances (Context:
/// "`ComponentId` consistency across regions" — never shared with any other
/// region) and its `rc-messaging` state. Constructed only via
/// `RcExecutor::spawn_region`. ARCH-D5/D6's region *lifecycle* (build/merge/split,
/// chunk ownership) is a separate, not-yet-written blueprint's job — this type is
/// deliberately minimal: one fixed `World` for the lifetime of the value.
pub struct RegionState {
    pub id: RegionId,
    pub world: World,
    pub tick_counter: u64,
    pub message_state: RegionMessageState,
    pub(crate) system_instances: [Vec<Box<dyn System<In = (), Out = ()>>>; 5],
}
