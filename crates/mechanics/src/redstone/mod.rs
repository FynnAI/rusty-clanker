//! Tier-1 redstone components (M3-B04): wire, torch, repeater, comparator, plus the shared
//! power-query substrate (`signal`) every one of them — and piston, M3-B05 — builds on.

pub mod comparator;
pub mod registration;
pub mod repeater;
pub mod signal;
pub mod torch;
pub mod wire;

pub use comparator::{ComparatorBehavior, ComparatorMode, ContainerSignalSource};
pub use registration::{Tier1RedstoneHandles, Tier1RedstoneStateIds, register_tier1_redstone};
pub use repeater::RepeaterBehavior;
pub use signal::{
    NoSignalSource, RedstoneSignalSource, SignalSourceRegistry, best_neighbor_signal,
    direct_signal_to, emitted_toward, has_signal, is_conductor, notify_neighbor_changed_only,
    signal_into,
};
pub use torch::{TorchAttachment, TorchBehavior};
pub use wire::WireBehavior;
