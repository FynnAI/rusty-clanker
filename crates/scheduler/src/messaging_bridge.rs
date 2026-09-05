//! The `RegionMessageBus`-inside-a-`bevy_ecs::System` bridge (M3-B01's own Context:
//! "Cross-region border updates (ARCH-D11) — the `RegionMessageBus`-in-a-system gap,
//! resolved"). `rc-messaging` cannot depend on `bevy_ecs` (WS-D3 rule 3), so the three
//! `Resource` types a registered Stage-4 (or any future) system reaches for live here,
//! in `rc-scheduler`, wired into `RcExecutor::spawn_region`/`tick_region`'s own Stage-1/
//! Stage-10 steps (`executor.rs`).

use bevy_ecs::prelude::Resource;
use rc_messaging::{
    Address, BorderUpdateEvent, EntitySnapshot, LightBorderUpdate, RegionMessage, RegionMessageBus,
};

/// This tick's inbound `BorderUpdateEvent` payloads, drained from `dyn Transport` at
/// `RcExecutor::tick_region`'s Stage-1 step (Context: "Cross-region border updates").
/// Auto-inserted (empty) by `RcExecutor::spawn_region`; overwritten (replace, not append)
/// every tick. Every other inbound `RegionMessage` variant is left in
/// `RegionState.message_state.inbox()`, untouched by this type.
#[derive(Resource, Default, Debug, Clone)]
pub struct BorderUpdateInbox(pub Vec<BorderUpdateEvent>);

/// M4-B07: this tick's inbound `LightBorderUpdate` payloads, drained at Stage 1
/// exactly as `BorderUpdateInbox` above already establishes for `BorderUpdateEvent`
/// — a second, independent inbox rather than folding light traffic into
/// `BorderUpdateInbox` itself, since the two payload types serve different Stage-8-
/// vs-Stage-4 consumers and WORLD-D10 explicitly frames `LightBorderUpdate` as its
/// own `RegionMessage` variant with its own consumption point (Stage 8's round-0
/// seeding, not Stage 4's first sub-step). Auto-inserted (empty) by
/// `RcExecutor::spawn_region`; overwritten (replace, not append) every tick.
#[derive(Resource, Default, Debug, Clone)]
pub struct LightBorderInbox(pub Vec<LightBorderUpdate>);

/// The in-`World`-reachable half of `RegionMessageBus` (Context: resolves M0-B02/M0-B05's
/// explicitly-deferred "how does a running system send a `RegionMessage`" question). Any
/// registered system may declare `ResMut<RegionMessageOutbox>` and call `.send`. Flushed into
/// `RegionState.message_state`'s own outbox by `RcExecutor::tick_region`'s Stage-10 step,
/// before that step's existing `drain_outbox`/`Transport::send` loop runs — so a send from
/// any system this tick is delivered within the same tick it was emitted.
#[derive(Resource, Default)]
pub struct RegionMessageOutbox(RegionMessageBus);

impl RegionMessageOutbox {
    /// Buffers one outbound message (ARCH-D30's own `RegionMessageBus::send` signature,
    /// reached from inside a real `bevy_ecs::System` for the first time).
    pub fn send(&mut self, to: Address, message: RegionMessage) {
        self.0.send(to, message);
    }

    /// Takes the buffered bus, leaving a fresh empty one — `RcExecutor::tick_region`'s own
    /// Stage-10 bridging step's only caller; not intended for direct use by a registered
    /// system (use `send` instead).
    pub fn take(&mut self) -> RegionMessageBus {
        std::mem::take(&mut self.0)
    }
}

/// Mirrors `RegionState.tick_counter`'s value as observed at Stage 1 (Context: "the ordinal
/// of the tick currently executing"). Auto-inserted (`CurrentTick(0)`) by `spawn_region`;
/// overwritten every tick's Stage-1 step, in the same pass that populates `BorderUpdateInbox`.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentTick(pub u64);

/// M4-B08 (Context, Part 1.2): this tick's inbound `RegionTransferRequest` payloads, drained
/// from `dyn Transport` at `RcExecutor::tick_region`'s Stage-1 step, mirroring
/// `BorderUpdateInbox`/`LightBorderInbox` exactly. Auto-inserted (empty) by
/// `RcExecutor::spawn_region`; overwritten (replace, not append) every tick — populated
/// *before* any registered `EntityArrivalDriver` runs, and stays readable afterward (a
/// driver does not clear it) so a test can assert what arrived a given tick without needing
/// the driver's own side effects as the only observable signal.
#[derive(Resource, Default, Debug, Clone)]
pub struct RegionTransferInbox(pub Vec<EntitySnapshot>);
