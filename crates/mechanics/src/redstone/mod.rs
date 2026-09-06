//! Tier-1 redstone components (M3-B04): wire, torch, repeater, comparator, plus the shared
//! power-query substrate (`signal`) every one of them — and piston, M3-B05 — builds on. Tier-2
//! (M4-B10): the lever's own manual input, plus the button and pressure-plate family.

pub mod button;
pub mod comparator;
pub mod dispatch_ranges;
pub mod entity_presence;
pub mod lever;
pub mod piston;
pub mod pressure_plate;
pub mod redstone_block;
pub mod registration;
pub mod repeater;
pub mod signal;
pub mod torch;
pub mod wire;

pub use button::{BUTTON_BLOCKS, ButtonBehavior, ButtonKindConfig};
pub use comparator::{ComparatorBehavior, ComparatorMode, ContainerSignalSource};
pub use dispatch_ranges::{
    derive_hopper_state_ids, derive_piston_state_ids, derive_tier1_state_ids,
};
pub use entity_presence::{EntityClassFilter, EntityPresenceSource, NoEntities};
pub use lever::LeverBehavior;
pub use piston::{PistonBehavior, register_piston};
pub use pressure_plate::{
    PRESSURE_PLATE_BLOCKS, PlateKindConfig, PlateSignalModel, PressurePlateBehavior,
    weighted_plate_signal,
};
pub use redstone_block::RedstoneBlockSource;
pub use registration::{
    Tier1RedstoneHandles, Tier1RedstoneStateIds, register_redstone_block, register_tier1_redstone,
    register_tier2_inputs,
};
pub use repeater::RepeaterBehavior;
pub use signal::{
    NoSignalSource, RedstoneSignalSource, SignalSourceRegistry, best_neighbor_signal,
    direct_signal_to, emitted_toward, has_signal, is_conductor, notify_neighbor_changed_only,
    signal_into,
};
pub use torch::{TorchAttachment, TorchBehavior};
pub use wire::WireBehavior;
