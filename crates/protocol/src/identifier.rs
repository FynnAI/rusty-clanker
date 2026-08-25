//! `Identifier` — a namespaced resource identifier ("namespace:path"), wire-identical to
//! `String` (M1-B04 Deliverables). A distinct newtype purely for call-site type safety
//! (channel names, registry ids, feature-flag ids), matching NET-D3's hand-written-types
//! philosophy. Performs no namespace/path validation of its own.

use bytes::{Bytes, BytesMut};

use crate::packet::PacketDecodeError;
use crate::wire::{WireRead, WireWrite};

/// A namespaced resource identifier ("namespace:path"). Wire-identical to `String`
/// (VarInt-length-prefixed UTF-8) — a distinct newtype purely for call-site type safety.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Identifier(pub String);

impl Identifier {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl WireWrite for Identifier {
    fn write_wire(&self, buf: &mut BytesMut) {
        self.0.write_wire(buf);
    }
}

impl WireRead for Identifier {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        Ok(Self(String::read_wire(buf)?))
    }
}
