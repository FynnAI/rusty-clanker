//! `rc-messaging` — location-transparent addressing, the `Message<RegionMessage>`
//! envelope, the `Transport` trait, the `RegionMessage` payload enum, and the
//! ECS-facing send/receive bus (ARCH-D24-D26, D28-D30). No transport implementation
//! and no network/IO dependency (`xtask lint-deps` Rule 3, M0-B01): this crate's
//! complete normal-dependency set is `{rc-core, serde, thiserror}`.

mod address;
mod bus;
mod envelope;
mod region_message;
mod transport;

pub use address::{Address, RegionId};
pub use bus::{RegionMessageBus, RegionMessageState};
pub use envelope::Message;
pub use region_message::{
    BorderUpdateEvent, BorderUpdateKind, EntitySnapshot, LightBorderUpdate, RegionMessage,
};
pub use transport::{Transport, TransportError};
