//! Every Play-state packet this blueprint defines (M1-B05 blueprint Deliverables,
//! "Packet ID table and its verification caveat"). Field **shapes** are the author's
//! confident restatement from stable, long-unchanged protocol history; numeric **ids**
//! were reconciled against a locally-generated `reports/packets.json` for protocol 776
//! per Implementation step 12 (see the implementation commit body for the reconciliation
//! record).

use bytes::Buf;
use rc_protocol::{Bytes, BytesMut, RcPacket};

/// M1 integration fix: `online_mode` (between the `CommonPlayerSpawnInfo`-equivalent
/// fields, ending at `sea_level`, and `enforces_secure_chat`) was missing entirely from
/// this blueprint's own field list — discovered by driving a real client (azalea, M1-B06)
/// and reading its own `ClientboundLogin` struct directly. Without it, every field from
/// `enforces_secure_chat` onward decoded one field short on a real client.
#[derive(RcPacket, Debug, Clone, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x31)]
pub struct LoginPlay {
    pub entity_id: i32,
    pub is_hardcore: bool,
    #[rc(prefixed_array = "VarInt")]
    pub dimension_names: Vec<String>,
    #[rc(varint)]
    pub max_players: i32,
    #[rc(varint)]
    pub view_distance: i32,
    #[rc(varint)]
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub do_limited_crafting: bool,
    #[rc(varint)]
    pub dimension_type: i32,
    pub dimension_name: String,
    pub hashed_seed: i64,
    pub game_mode: u8,
    pub previous_game_mode: i8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub has_death_location: bool,
    #[rc(varint)]
    pub portal_cooldown: i32,
    #[rc(varint)]
    pub sea_level: i32,
    pub online_mode: bool,
    pub enforces_secure_chat: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x61)]
pub struct SetDefaultSpawnPosition {
    pub location: i64,
    pub angle: u8,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x48)]
pub struct SynchronizePlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub relative_arguments: u8,
    #[rc(varint)]
    pub teleport_id: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x26)]
pub struct GameEvent {
    pub event: u8,
    pub value: f32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x58)]
pub struct SetChunkCacheCenter {
    #[rc(varint)]
    pub chunk_x: i32,
    #[rc(varint)]
    pub chunk_z: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x0C)]
pub struct ChunkBatchStart {}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x0B)]
pub struct ChunkBatchFinished {
    #[rc(varint)]
    pub batch_size: i32,
}

/// Individually `VarInt(2048)`-prefixed 2048-byte nibble-packed light array (Context —
/// required as a local newtype by Rust's orphan rule; never reuse outside this module).
#[derive(Debug, Clone, PartialEq)]
pub struct LightArray(pub [u8; 2048]);

impl rc_protocol::WireWrite for LightArray {
    fn write_wire(&self, buf: &mut BytesMut) {
        rc_protocol::VarInt::new(2048).encode(buf);
        buf.extend_from_slice(&self.0);
    }
}
impl rc_protocol::WireRead for LightArray {
    fn read_wire(buf: &mut Bytes) -> Result<Self, rc_protocol::PacketDecodeError> {
        let declared = rc_protocol::VarInt::decode(buf)?.get();
        let declared = usize::try_from(declared).unwrap_or(usize::MAX);
        if declared != 2048 {
            return Err(rc_protocol::PacketDecodeError::ArrayTooLong {
                declared,
                remaining: buf.remaining(),
            });
        }
        if buf.remaining() < 2048 {
            return Err(rc_protocol::PacketDecodeError::UnexpectedEof);
        }
        let mut bytes = [0u8; 2048];
        buf.copy_to_slice(&mut bytes);
        Ok(LightArray(bytes))
    }
}

#[derive(RcPacket, Debug, Clone, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x2D)]
pub struct LevelChunkWithLight {
    pub chunk_x: i32,
    pub chunk_z: i32,
    #[rc(prefixed_array = "VarInt")]
    pub heightmaps: Vec<u8>,
    #[rc(prefixed_array = "VarInt")]
    pub data: Vec<u8>,
    #[rc(prefixed_array = "VarInt")]
    pub block_entities: Vec<u8>,
    #[rc(prefixed_array = "VarInt")]
    pub sky_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")]
    pub block_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")]
    pub empty_sky_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")]
    pub empty_block_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")]
    pub sky_light_arrays: Vec<LightArray>,
    #[rc(prefixed_array = "VarInt")]
    pub block_light_arrays: Vec<LightArray>,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x2C)]
pub struct KeepAliveClientbound {
    pub id: i64,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x1C)]
pub struct KeepAliveServerbound {
    pub id: i64,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x00)]
pub struct ConfirmTeleportation {
    #[rc(varint)]
    pub teleport_id: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x0A)]
pub struct ChunkBatchReceived {
    pub chunks_per_tick: f32,
}

/// Packs a "Position" wire value (Context: 26-bit X, 26-bit Z, 12-bit Y, two's complement),
/// written as one plain big-endian 8-byte `Long` by the caller (`WireWrite for i64`).
pub fn pack_position(pos: rc_core::BlockPos) -> i64 {
    let x = (pos.x as i64) & 0x3FF_FFFF;
    let z = (pos.z as i64) & 0x3FF_FFFF;
    let y = (pos.y as i64) & 0xFFF;
    (x << 38) | (z << 12) | y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_position_round_trips_negative_y() {
        let packed = pack_position(rc_core::BlockPos::new(0, -59, 0));
        // y is the low 12 bits, two's-complement: -59 as u12 == 4096 - 59 == 4037.
        assert_eq!(packed & 0xFFF, 4037);
    }
}
