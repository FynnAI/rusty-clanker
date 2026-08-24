use std::collections::HashMap;

use parking_lot::RwLock;
use rc_messaging::{Address, Message, RegionId, RegionMessage, Transport, TransportError};

use crate::EntitySnapshotPool;

/// One live region's bidirectional inbound channel halves, created together by one
/// `crossbeam_channel::bounded` call and destroyed together (never independently) —
/// see this blueprint's Context section for why `crossbeam_channel::TryRecvError::Disconnected`
/// can never actually occur in this implementation.
struct RegionChannel {
    sender: crossbeam_channel::Sender<Message<RegionMessage>>,
    receiver: crossbeam_channel::Receiver<Message<RegionMessage>>,
}

/// Tunable knobs for `InProcessTransport`: ARCH-D27's per-region channel capacity
/// ("capacity 4096 messages, configurable") and ARCH-D28's `EntitySnapshotPool`
/// pre-sizing (this blueprint's own seed default — see Context).
#[derive(Copy, Clone, Debug)]
pub struct InProcessTransportConfig {
    pub channel_capacity: usize,
    pub entity_snapshot_pool_capacity: usize,
}

impl Default for InProcessTransportConfig {
    /// `channel_capacity: 4096` (ARCH-D27's literal number), `entity_snapshot_pool_capacity: 256`.
    fn default() -> Self {
        Self {
            channel_capacity: 4096,
            entity_snapshot_pool_capacity: 256,
        }
    }
}

/// ARCH-D27's monolithic-mode `Transport` implementation. One bounded
/// `crossbeam-channel` MPSC per live `RegionId`, under a `parking_lot::RwLock`-guarded
/// region table (ARCH-D23), plus one shared `EntitySnapshotPool` (ARCH-D28).
/// `Message<RegionMessage>` values move through the channel by value — never cloned,
/// never serialized.
pub struct InProcessTransport {
    channels: RwLock<HashMap<RegionId, RegionChannel>>,
    config: InProcessTransportConfig,
    entity_snapshot_pool: EntitySnapshotPool,
}

impl InProcessTransport {
    /// An empty transport (no regions registered) using `config`.
    pub fn new(config: InProcessTransportConfig) -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            entity_snapshot_pool: EntitySnapshotPool::new(config.entity_snapshot_pool_capacity),
            config,
        }
    }

    /// Create `id`'s inbound channel. Calling this again for an already-registered `id`
    /// silently replaces its channel — drops any still-in-flight messages and the old
    /// receiver. A correct caller never does this (`RegionId`'s own identity contract,
    /// `rc-messaging`'s Context, guarantees `RegionId` values are never reused). Intended
    /// call site: an ARCH-D6 split/merge boundary, owned by a later `rc-scheduler`
    /// blueprint — see Constraints.
    pub fn register_region(&self, id: RegionId) {
        let (sender, receiver) = crossbeam_channel::bounded(self.config.channel_capacity);
        self.channels
            .write()
            .insert(id, RegionChannel { sender, receiver });
    }

    /// Destroy `id`'s inbound channel. Any message already in flight toward `id` (sent
    /// but not yet drained) is dropped. Idempotent: deregistering an unregistered `id`
    /// is a no-op.
    pub fn deregister_region(&self, id: RegionId) {
        self.channels.write().remove(&id);
    }

    /// Whether `id` currently has a live channel.
    pub fn is_registered(&self, id: RegionId) -> bool {
        self.channels.read().contains_key(&id)
    }

    /// The shared, global `EntitySnapshotPool` (ARCH-D28).
    pub fn entity_snapshot_pool(&self) -> &EntitySnapshotPool {
        &self.entity_snapshot_pool
    }
}

impl Transport for InProcessTransport {
    /// Resolves `msg.to` to a destination `RegionId`: `Address::Region(id) => id`
    /// directly. `Address::Entity`/`Address::Chunk` are out of this blueprint's scope
    /// (see Context) and immediately return `Err(TransportError::Backpressure(msg))`,
    /// same as an unregistered `Address::Region` destination or a full channel — this
    /// blueprint's own deliberate unification of all three "cannot deliver right now"
    /// cases onto the one error variant `rc-messaging` provides. Never blocks
    /// (`crossbeam_channel::Sender::try_send`, non-blocking).
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError> {
        let destination = match msg.to {
            Address::Region(destination) => destination,
            Address::Entity(_) | Address::Chunk(_) => {
                return Err(TransportError::Backpressure(msg));
            }
        };

        let channels = self.channels.read();
        match channels.get(&destination) {
            Some(channel) => match channel.sender.try_send(msg) {
                Ok(()) => Ok(()),
                Err(crossbeam_channel::TrySendError::Full(returned))
                | Err(crossbeam_channel::TrySendError::Disconnected(returned)) => {
                    Err(TransportError::Backpressure(returned))
                }
            },
            None => Err(TransportError::Backpressure(msg)),
        }
    }

    /// Non-blocking single-message drain from `into`'s channel. Returns `None` if `into`
    /// has no live channel or its channel is currently empty — both cases are
    /// indistinguishable via this call alone (`is_registered` answers the first
    /// separately, if a caller needs to).
    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>> {
        let channels = self.channels.read();
        let channel = channels.get(&into)?;
        channel.receiver.try_recv().ok()
    }
}
