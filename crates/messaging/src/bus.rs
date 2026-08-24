use std::collections::HashMap;

use crate::{Address, Message, RegionId, RegionMessage};

/// Per-invocation outbound send buffer (ARCH-D30: "injected like `Commands`" —
/// restated in `bevy_ecs`-free form; see this blueprint's Context section for why).
/// Whoever hands one of these to a running domain system gives each system its own
/// private instance, so concurrently-running systems never contend over shared
/// mutable state through this type — that integration is `rc-scheduler`'s job, not
/// implemented by this blueprint.
#[derive(Debug, Default)]
pub struct RegionMessageBus {
    pending: Vec<(Address, RegionMessage)>,
}

impl RegionMessageBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Buffer an outbound message. Not visible anywhere else (not in any
    /// `RegionMessageState`, not flushed to `dyn Transport`) until this whole buffer
    /// is passed to `RegionMessageState::merge`.
    pub fn send(&mut self, to: Address, message: RegionMessage) {
        self.pending.push((to, message));
    }
}

/// The region-owned canonical message state (ARCH-D30): every finished system's
/// `RegionMessageBus` merged in order, plus the current tick's drained inbound queue.
/// One instance per region (ARCH-D5) — placing it there and driving the Stage-1/
/// Stage-10 contract below is `rc-scheduler`'s job, not implemented by this blueprint.
#[derive(Debug, Default)]
pub struct RegionMessageState {
    outbox: Vec<(Address, RegionMessage)>,
    inbox: Vec<RegionMessage>,
    seq_counters: HashMap<Address, u32>,
}

impl RegionMessageState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one finished system's buffered sends onto the outbox, preserving
    /// emission order (this call's entries appended after everything already
    /// merged this tick). Consumes `bus`.
    pub fn merge(&mut self, bus: RegionMessageBus) {
        self.outbox.extend(bus.pending);
    }

    /// Stage 10: stamp and drain every message merged so far this tick into
    /// ready-to-send envelopes, in emission (merge) order. `from`/`tick_stamp` are
    /// supplied by the caller (the region's own identity and current tick counter —
    /// owned by `rc-scheduler`, not this crate). `seq` is assigned here: a monotonic
    /// counter per distinct `to: Address` value, persisting across ticks (see
    /// Context). Empties the outbox; `seq` counters are **not** reset.
    pub fn drain_outbox(&mut self, from: RegionId, tick_stamp: u64) -> Vec<Message<RegionMessage>> {
        let drained = std::mem::take(&mut self.outbox);
        drained
            .into_iter()
            .map(|(to, payload)| {
                let counter = self.seq_counters.entry(to).or_insert(0);
                let seq = *counter;
                *counter += 1;
                Message {
                    from,
                    to,
                    tick_stamp,
                    seq,
                    payload,
                }
            })
            .collect()
    }

    /// Stage 1: install this tick's freshly-drained inbound queue (ARCH-D30),
    /// **replacing** whatever was left from last tick (not appending).
    pub fn set_inbox(&mut self, messages: Vec<RegionMessage>) {
        self.inbox = messages;
    }

    /// Read-only inbound access for any Stage-1..N system.
    pub fn inbox(&self) -> &[RegionMessage] {
        &self.inbox
    }
}
