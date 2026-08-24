//! Shared synthetic marker components and test-only helpers reused across this
//! blueprint's acceptance test files (M0-B05's own Acceptance tests section),
//! mirroring `rc-messaging`'s own `MockTransport`-in-test-file convention
//! (`crates/messaging/tests/fifo_property.rs`).

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use bevy_ecs::prelude::*;
use rc_messaging::{Message, RegionId, RegionMessage, Transport, TransportError};

#[derive(Component, Default)]
pub struct A(pub i64);
#[derive(Component, Default)]
pub struct B(pub i64);
#[derive(Component, Default)]
pub struct Marker;

pub fn empty_bootstrap(_world: &mut World) {}

/// A `MockTransport` identical in shape to M0-B02's own `fifo_property.rs` mock
/// (bounded per-`RegionId` `VecDeque` behind a `Mutex`), reused here as this
/// blueprint's own test-only `Transport` implementation -- not a dependency on
/// `rc-transport-inproc` (which `rc-scheduler` must never depend on, `xtask
/// lint-deps` Rule 2).
pub struct MockTransport {
    inboxes: Mutex<HashMap<RegionId, VecDeque<Message<RegionMessage>>>>,
    sent: Mutex<Vec<Message<RegionMessage>>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            inboxes: Mutex::new(HashMap::new()),
            sent: Mutex::new(Vec::new()),
        }
    }

    /// Test helper: pushes `msg` directly into `into`'s inbox queue, bypassing `send`.
    pub fn seed(&self, into: RegionId, msg: Message<RegionMessage>) {
        self.inboxes
            .lock()
            .unwrap()
            .entry(into)
            .or_default()
            .push_back(msg);
    }

    /// Test helper: returns every message ever passed to `send`, in call order.
    pub fn sent(&self) -> Vec<Message<RegionMessage>> {
        self.sent.lock().unwrap().clone()
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for MockTransport {
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError> {
        self.sent.lock().unwrap().push(msg);
        Ok(())
    }

    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>> {
        self.inboxes
            .lock()
            .unwrap()
            .get_mut(&into)
            .and_then(|q| q.pop_front())
    }
}
