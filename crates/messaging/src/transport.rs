use crate::{Message, RegionId, RegionMessage};

/// One `RegionMessage` delivery failure mode (ARCH-D29's own name and the only variant
/// it pins). Carries the un-delivered message back to the caller — `Transport::send`
/// fully consumes `msg` by value per ARCH-D26's exact signature, so this is the only
/// way a caller can retry the *same* message next tick, mirroring
/// `std::sync::mpsc::SendError<T>`'s "give the value back" convention. See this
/// blueprint's Context section for why this is a deliberate, cited resolution of an
/// explicitly-deferred planning decision, not an invented deviation.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("destination inbox is full; message returned for retry (ARCH-D29 backpressure)")]
    Backpressure(Message<RegionMessage>),
}

/// The one substrate every cross-partition communication goes through, in either
/// deployment mode (ARCH-D26). Exact signature pinned there. Zero implementations of
/// this trait exist in this crate — `InProcessTransport` (`rc-transport-inproc`) and
/// `NetworkTransport` (`rc-transport-net`, cluster feature) are separate crates that
/// depend on this one, never the reverse (`xtask lint-deps` Rule 3).
///
/// Guarantees every implementation must uphold (ARCH-D29, restated in full in this
/// blueprint's Context section): FIFO and exactly-once per `(from, to)` pair for the
/// process's lifetime; no ordering guarantee across different pairs; never blocks the
/// caller (`Backpressure` instead).
pub trait Transport: Send + Sync + 'static {
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError>;
    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>>;
}
