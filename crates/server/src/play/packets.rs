//! Every Play-state packet this blueprint defines (M1-B05 blueprint Deliverables,
//! "Packet ID table and its verification caveat"). Field **shapes** are the author's
//! confident restatement from stable, long-unchanged protocol history; numeric **ids**
//! were reconciled against a locally-generated `reports/packets.json` for protocol 776
//! per Implementation step 12 (see the implementation commit body for the reconciliation
//! record).

use bytes::Buf;
use rc_core::BlockPos;
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

/// M1 integration fix: this blueprint's own first implementation attempt wrongly assumed a
/// bare packed-position-plus-angle-byte shape with no dimension identifier
/// (`location: i64, angle: u8`) -- driving a real client (azalea) against it produced
/// `Error reading packet set_default_spawn_position (id 97): failed to fill whole buffer`
/// (the client's own decoder expects strictly more bytes than that shape ever wrote).
/// Corrected to azalea's own real `ClientboundSetDefaultSpawnPosition { global_pos:
/// GlobalPos { dimension: Identifier, pos: BlockPos }, yaw: f32, pitch: f32 }`
/// (`azalea-protocol`/`azalea-core`'s own source, Constraints (d) sanctions reading a
/// client library's source this way) -- the exact same reference shape
/// `crates/testing/test-harness/src/fake_server.rs`'s own `SendFinishConfiguration` step
/// had already independently proven correct, now ported into this production packet
/// definition instead of only the test double.
#[derive(RcPacket, Debug, Clone, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x61)]
pub struct SetDefaultSpawnPosition {
    pub dimension: String,
    pub location: i64,
    pub yaw: f32,
    pub pitch: f32,
}

/// M1 integration fix: this blueprint's own first implementation attempt wrongly assumed a
/// flat x/y/z/yaw/pitch/flags/teleport_id shape with no velocity ("delta") fields and
/// `teleport_id` last -- driving a real client (azalea) against it produced
/// `Error reading packet player_position (id 72): failed to fill whole buffer`. Corrected
/// to azalea's own real `ClientboundPlayerPosition { #[var] id: u32, change:
/// PositionMoveRotation { pos: Vec3, delta: Vec3, look_direction: LookDirection { y_rot:
/// f32, x_rot: f32 } }, relative: RelativeMovements }` (`azalea-protocol`/`common/
/// movements.rs`'s own source, Constraints (d)) -- `teleport_id` first, three extra `delta`
/// `f64` fields, and `relative` as a raw 4-byte bitset (`i32`, not `u8` -- `rc_protocol`
/// has no `u32` `WireWrite` impl, `i32`'s identical 4-byte big-endian bit pattern for `0`
/// is used instead, matching `fake_server.rs`'s own already-proven encoding exactly).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x48)]
pub struct SynchronizePlayerPosition {
    #[rc(varint)]
    pub teleport_id: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub delta_x: f64,
    pub delta_y: f64,
    pub delta_z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub relative_arguments: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x26)]
pub struct GameEvent {
    pub event: u8,
    pub value: f32,
}

/// M1 integration fix: id `0x58` (this blueprint's own first implementation attempt) is
/// azalea's own `set_border_center`'s declaration-order id, not `set_chunk_cache_center`'s
/// -- driving a real client against it produced
/// `Error reading packet set_border_center (id 88): failed to fill whole buffer` (a real
/// client tried to decode this packet's `chunk_x`/`chunk_z` `VarInt`s as `set_border_center`'s
/// own two `f64` fields instead). Corrected to `0x5E`, `set_chunk_cache_center`'s real
/// declaration-order id (`azalea-protocol`'s own `declare_state_packets!` expansion,
/// Constraints (d)) -- the same id `fake_server.rs`'s own already-proven script uses.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x5E)]
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

/// M2-B07: one `PlayerAction`/`UseItemOn` (serverbound, block-modifying) — creative-style
/// instant break only, per MECH-D61's restated minimal scope (this milestone). `status`'s
/// enum values (`0`=StartDestroyBlock ... `6`=SwapItemInHand) and `face`'s wire type
/// (`Byte`, not `VarInt` -- a real, historically-documented asymmetry against `UseItemOn`'s
/// own `face`) are per this blueprint's own reconciliation caveat (Context).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x29)]
pub struct PlayerAction {
    #[rc(varint)]
    pub status: i32,
    pub location: i64,
    pub face: i8,
    #[rc(varint)]
    pub sequence: i32,
}

/// M2-B07: a single fixed placeholder block is placed on every successful `UseItemOn`
/// (Context: "Placement content" -- no real item/inventory model exists yet). `face` is a
/// `VarInt` here (unlike `PlayerAction.face`'s `Byte`) -- a real, long-standing asymmetry
/// between these two packets, not a copy/paste inconsistency (Context).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x2A)]
pub struct UseItemOn {
    #[rc(varint)]
    pub hand: i32,
    pub location: i64,
    #[rc(varint)]
    pub face: i32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
    pub inside_block: bool,
    #[rc(varint)]
    pub sequence: i32,
}

/// M2-B07: one block's new state, broadcast to every currently-connected player (Context:
/// "The M1-B05 interest/broadcast seam does not exist -- resolved here").
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x08)]
pub struct BlockUpdate {
    pub location: i64,
    #[rc(varint)]
    pub block_state_id: i32,
}

/// M2-B07: the vanilla per-action `sequence` acknowledgment (MECH-D63, restated) -- the
/// client allocates and stamps `sequence`, the server only validates and echoes it back,
/// unmodified, exactly once per received `PlayerAction`/`UseItemOn`, unconditionally
/// (Context).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x04)]
pub struct AcknowledgeBlockChange {
    #[rc(varint)]
    pub sequence: i32,
}

/// Inverse of this file's already-existing `pack_position` (Context: exact bit layout
/// restated). Sign-extends each two's-complement field back from its packed width.
pub fn unpack_position(packed: i64) -> BlockPos {
    let raw_x = (packed >> 38) & 0x3FF_FFFF;
    let raw_z = (packed >> 12) & 0x3FF_FFFF;
    let raw_y = packed & 0xFFF;
    BlockPos::new(
        sign_extend(raw_x, 26) as i32,
        sign_extend(raw_y, 12) as i32,
        sign_extend(raw_z, 26) as i32,
    )
}

/// `value` is an unsigned `bits`-wide field already isolated by the caller (masked, no
/// stray high bits). Returns its two's-complement signed interpretation.
fn sign_extend(value: i64, bits: u32) -> i64 {
    let sign_bit = 1i64 << (bits - 1);
    if value >= sign_bit {
        value - (1i64 << bits)
    } else {
        value
    }
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

    #[test]
    fn unpack_position_is_the_exact_inverse_of_pack_position() {
        let samples = [
            BlockPos::new(0, -59, 0),
            BlockPos::new(-1, -64, -1),
            BlockPos::new(20, 319, 20),
            BlockPos::new(-33_554_432, -64, 33_554_431), // full 26-bit x/z range
            BlockPos::new(2, -60, 2),
        ];
        for pos in samples {
            assert_eq!(
                unpack_position(pack_position(pos)),
                pos,
                "round trip of {pos:?}"
            );
        }
    }
}
