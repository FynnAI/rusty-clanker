use bytes::{Buf, Bytes, BytesMut};

/// The five NET-D4 connection states. `Transfer` is not a state (it is a Handshake-phase
/// intention value that routes into `Login`, per NET-D4) and has no variant here.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConnectionState {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}

/// Which side sent a packet, matching the illustrative sketch's own `bound = "server"/"client"`
/// vocabulary (`"server"` = a packet the server receives, `"client"` = a packet the server sends).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PacketBound {
    Serverbound,
    Clientbound,
}

#[derive(Debug, thiserror::Error)]
pub enum PacketDecodeError {
    #[error("unexpected end of packet body while reading a field")]
    UnexpectedEof,
    #[error("malformed VarInt/VarLong field: {0}")]
    MalformedVarNum(#[from] crate::varint::VarNumError),
    #[error("string field decoded to {actual} chars, exceeding the {max}-char limit")]
    StringTooLong { actual: usize, max: usize },
    #[error("string field is not valid UTF-8")]
    InvalidUtf8,
    #[error(
        "a prefixed-array field declared {declared} elements but only {remaining} bytes remain"
    )]
    ArrayTooLong { declared: usize, remaining: usize },
    #[error("packet body has {remaining} trailing byte(s) after every declared field was read")]
    TrailingBytes { remaining: usize },
    #[error("unknown packet id {id} for state {state:?} bound {bound:?}")]
    UnknownPacketId {
        id: i32,
        state: ConnectionState,
        bound: PacketBound,
    },
}

/// One raw, id-and-body-only packet flowing across the reader-task/consumer boundary — this
/// blueprint's scope stops here: framing, decompression, and decryption are already fully
/// resolved by the time a `RawPacket` exists.
#[derive(Debug, Clone)]
pub struct RawPacket {
    pub id: i32,
    pub body: Bytes,
}

/// Implemented by `#[derive(RcPacket)]` for exactly one concrete packet struct. Never
/// implemented by hand except in a test (this blueprint's own derive-expansion tests do
/// exactly that, to prove the trait's shape independent of the macro).
pub trait RcPacket: Sized {
    const STATE: ConnectionState;
    const BOUND: PacketBound;
    const ID: i32;

    fn encode_body(&self, buf: &mut BytesMut);
    /// Decodes only this packet's own fields — does **not** check for trailing bytes after
    /// the last field; callers use `decode_one`, which adds that check, rather than calling
    /// this directly.
    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError>;
}

/// Decodes one packet of a single, statically-known `RcPacket` type `P`, additionally
/// asserting the body is fully consumed (no trailing bytes) — matching the reference's own
/// "a decoded packet consumes the entire frame" rule. The building block a `PacketCatalog`
/// impl's per-id match arms call; not itself part of `PacketCatalog`.
pub fn decode_one<P: RcPacket>(mut body: Bytes) -> Result<P, PacketDecodeError> {
    let value = P::decode_body(&mut body)?;
    if body.has_remaining() {
        return Err(PacketDecodeError::TrailingBytes {
            remaining: body.remaining(),
        });
    }
    Ok(value)
}

/// Encodes `packet` into its full outbound payload — packet-id `VarInt` followed by the
/// packet's own body — ready to hand to a `Connection`'s outbound channel (framing,
/// compression, and encryption are the Tokio writer task's job, not this function's).
pub fn encode_payload<P: RcPacket>(packet: &P) -> Bytes {
    let mut buf = BytesMut::new();
    crate::varint::VarInt::new(P::ID).encode(&mut buf);
    packet.encode_body(&mut buf);
    buf.freeze()
}

/// The seam a later blueprint's per-connection-state packet enum (e.g. a `HandshakePacket`
/// enum covering every packet legal in `ConnectionState::Handshake`) implements, so a
/// generic consumer of `RawPacket`s can dispatch to a typed value without this crate ever
/// knowing which concrete packet types exist. Not implemented anywhere in this blueprint.
pub trait PacketCatalog: Sized + Send + 'static {
    fn decode(
        state: ConnectionState,
        bound: PacketBound,
        id: i32,
        body: Bytes,
    ) -> Result<Self, PacketDecodeError>;
    fn packet_id(&self) -> i32;
    fn encode_body(&self, buf: &mut BytesMut);
}
