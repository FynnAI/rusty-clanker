//! `rc-transport-inproc` — `InProcessTransport` (ARCH-D27): the monolithic-mode
//! `Transport` implementation, one bounded `crossbeam-channel` MPSC per live `RegionId`
//! under a `parking_lot`-guarded region table (ARCH-D23), plus `EntitySnapshotPool`
//! (ARCH-D28), the global `SegQueue`-backed slot pool for large `RegionTransferRequest`
//! payloads.

mod entity_snapshot_pool;
mod transport;

pub use entity_snapshot_pool::EntitySnapshotPool;
pub use transport::{InProcessTransport, InProcessTransportConfig};
