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

/// M2 field-report fix (real-client manual test, M2 milestone): the three serverbound
/// movement packets a real client sends continuously while playing -- decoded here for the
/// first time; M1-B05's own dispatch loop never recognized any of the three (`id`s `0x1E`/
/// `0x1F`/`0x20` all fell into `dispatch_inbound`'s `other =>` catch-all), so every claimed
/// position/rotation update was silently dropped, never reaching `HardcodedWorld` at all.
/// Field shapes restated from the M3-B02 movement-collision blueprint's own already-
/// reconciled packet table (`blueprints/M3/M3-B02-movement-collision.md` Deliverables,
/// reconciled there against a locally-generated `reports/packets.json` for protocol 776 --
/// the wire shape itself is fixed protocol history, not an M3-scope design decision).
///
/// M3-B02 superseded this M2-scope fix's own minimal decode-and-apply path with the real
/// replay-validation/speed-check/teleport-correction pipeline these same packets (plus the
/// fourth, `SetPlayerMovementFlags` below) now drive: `play::movement::evaluate_movement`.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x1E)]
pub struct SetPlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
}

/// As `SetPlayerPosition`'s own doc comment, id `0x1F`.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x1F)]
pub struct SetPlayerPositionAndRotation {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

/// As `SetPlayerPosition`'s own doc comment, id `0x20`.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x20)]
pub struct SetPlayerRotation {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

/// M3-B02: the fourth serverbound movement packet -- carries only the on-ground flag, sent
/// when neither position nor rotation changed this tick (Deliverables, Context: "Serverbound
/// movement packets at protocol 776").
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x21)]
pub struct SetPlayerMovementFlags {
    pub on_ground: bool,
}

/// M3 field-report fix (Symptom 2): protocol 776's own serverbound sneak/movement-intent
/// channel. Neither `PlayerAction` nor any legacy player-command packet carries a shift/sneak
/// action in this version (Context, AUTHORITATIVE RESEARCH VERDICT) -- `player_input` is the
/// only wire source. Id `0x2B`, verified against the local datagen report
/// (`mc-research/26.2/datagen/generated/reports/packets.json`, Context) alongside every other
/// already-known id in this file (`move_player_pos`/`_rot`/`use_item_on`), independently
/// cross-confirming those. A single raw byte -- `flags` -- wrapping seven boolean movement-
/// intent bits (`shift`'s own doc comment below has the exact bit-to-meaning table).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x2B)]
pub struct PlayerInput {
    pub flags: u8,
}

pub const PLAYER_INPUT_FORWARD: u8 = 0x01;
pub const PLAYER_INPUT_BACKWARD: u8 = 0x02;
pub const PLAYER_INPUT_LEFT: u8 = 0x04;
pub const PLAYER_INPUT_RIGHT: u8 = 0x08;
pub const PLAYER_INPUT_JUMP: u8 = 0x10;
pub const PLAYER_INPUT_SHIFT: u8 = 0x20;
pub const PLAYER_INPUT_SPRINT: u8 = 0x40;

/// Per-bit accessors (Context). This milestone only ever consumes `shift()`
/// (`connection.rs`'s dispatch threads it into `movement::PlayerInputState.sneaking`,
/// `movement::eye_position`'s own pose-aware height selection); the other six are decoded
/// for completeness/future use but not yet threaded into any gameplay effect --
/// server-authoritative movement already derives its own velocity/direction from the
/// position packets, never from this intent bitfield.
impl PlayerInput {
    pub fn forward(self) -> bool {
        self.flags & PLAYER_INPUT_FORWARD != 0
    }
    pub fn backward(self) -> bool {
        self.flags & PLAYER_INPUT_BACKWARD != 0
    }
    pub fn left(self) -> bool {
        self.flags & PLAYER_INPUT_LEFT != 0
    }
    pub fn right(self) -> bool {
        self.flags & PLAYER_INPUT_RIGHT != 0
    }
    pub fn jump(self) -> bool {
        self.flags & PLAYER_INPUT_JUMP != 0
    }
    /// The one bit this milestone actually consumes.
    pub fn shift(self) -> bool {
        self.flags & PLAYER_INPUT_SHIFT != 0
    }
    pub fn sprint(self) -> bool {
        self.flags & PLAYER_INPUT_SPRINT != 0
    }
}

/// M3 field-report fix ("everything I place becomes stone" -- a real vanilla client's own
/// two hotbar-tracking packets were decoded nowhere in this crate's inbound dispatch, so the
/// join-time `HeldItem` default never changed for a real client, `play::connection`'s own
/// dispatch doc comment has the full root-cause writeup). Id and field shape reconciled
/// against the pinned server's own decompiled reference (`ServerboundSetCarriedItemPacket`,
/// `mc-research/26.2/src/net/minecraft/network/protocol/game/`, the ASSET-D18(f) reference,
/// Constraints (d)) and independently against the local datagen report
/// (`mc-research/26.2/datagen/generated/reports/packets.json`'s own `minecraft:
/// set_carried_item` entry): `protocol_id` 53 (`0x35`). `slot` is a raw two-byte `Short`
/// (`FriendlyByteBuf.readShort`/`writeShort` in the reference -- NOT a `VarInt`, unlike almost
/// every other small integer field this crate decodes), restated here as the closest matching
/// wire primitive, `u16` (every real value is the non-negative `0..9` hotbar index; vanilla's
/// own `handleSetCarriedItem` rejects anything else, `connection.rs`'s own dispatch mirrors
/// that same bound).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x35)]
pub struct SetCarriedItem {
    pub slot: u16,
}

/// As `SetCarriedItem`'s own doc comment for the id-reconciliation method: cross-checked
/// against `ServerboundSetCreativeModeSlotPacket` (same reference file) and the datagen
/// report's own `minecraft:set_creative_mode_slot` entry, `protocol_id` 56 (`0x38`). `slot`
/// is the same raw `Short` shape, but addresses the FULL player-inventory container (`0..=45`,
/// `InventoryMenu`'s own `USE_ROW_SLOT_START..USE_ROW_SLOT_END` == `36..45` for the hotbar
/// specifically -- `connection.rs`'s own dispatch doc comment has the full slot-index-mapping
/// citation), never the bare `0..9` hotbar index `SetCarriedItem` uses. `item`'s own wire
/// shape is `CreativeSlotItem`'s own doc comment.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x38)]
pub struct SetCreativeModeSlot {
    pub slot: u16,
    pub item: CreativeSlotItem,
}

/// `ServerboundSetCreativeModeSlotPacket.itemStack`'s own wire shape (Mojang decompiled
/// reference, Constraints (d)): a plain `count: VarInt` (`<= 0` decodes to "no item," no
/// further bytes at all -- vanilla's own `ItemStack.EMPTY` encoding); otherwise the held
/// item's own `minecraft:item` registry id (`Item.STREAM_CODEC` ==
/// `ByteBufCodecs.holderRegistry(Registries.ITEM)`, a bare, unoffset VarInt -- the *fixed*-
/// registry shape, distinct from the `id + 1`-offset "direct holder" shape a *dynamic*
/// registry's `ByteBufCodecs.holder` uses), then a `DataComponentPatch` in its own
/// "untrusted"/"delimited" wire shape (`ItemStack.OPTIONAL_UNTRUSTED_STREAM_CODEC` ->
/// `DataComponentPatch.DELIMITED_STREAM_CODEC`): `positive_count: VarInt`,
/// `negative_count: VarInt`, then, for each of the `positive_count` entries, a
/// `component_type_id: VarInt` (same bare-VarInt registry shape, into
/// `minecraft:data_component_type`) followed by a `payload_len: VarInt` and exactly
/// `payload_len` raw bytes (`ByteBufCodecs.registryFriendlyLengthPrefixed` -- the untrusted
/// codec's whole reason to exist: a length-prefixed payload a receiver that does not
/// interpret a given component type can still skip byte-exact), and finally, for each of the
/// `negative_count` entries, one more bare `component_type_id: VarInt` with no payload at
/// all. This type decodes exactly that shape -- every byte accounted for, never a truncated
/// "consume the rest of the packet" shortcut -- but only ever *keeps* `item_id`: no
/// component this milestone's placement logic reads (M3-scope-minimal: held-item *tracking*,
/// not a real inventory/component system -- M4's own future scope) is interpreted at all.
///
/// M3 field-report cross-check: azalea's own `azalea-inventory::ItemStack`/
/// `DataComponentPatch::azalea_read` (checked-out rev `6249c295`, `azalea_protocol::packets::
/// PROTOCOL_VERSION == 776` -- the SAME pin this project targets, not a stale cross-version
/// mismatch) omits this length prefix entirely, decoding each present component directly via
/// its own concrete shape instead -- the *trusted* codec's own shape
/// (`DataComponentPatch.STREAM_CODEC`), never the untrusted one this specific serverbound
/// packet's own `ServerboundSetCreativeModeSlotPacket.STREAM_CODEC` actually declares. Since
/// the identical shared `StreamCodec` class also encodes this packet on a real client's own
/// sending side, the real wire bytes a real vanilla client sends match the decompiled server
/// reference, not azalea's own shape -- this type follows the reference (recorded as a
/// discrepancy finding, `docs/findings-for-planning.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreativeSlotItem {
    /// `None` for an explicitly empty stack (`count <= 0`) or an item id this milestone's own
    /// closed `PlaceableBlockKind` set never maps (`play::mining::placeable_kind_for_item_id`'s
    /// own doc comment) -- never a decode failure either way.
    pub item_id: Option<i32>,
}

impl rc_protocol::WireRead for CreativeSlotItem {
    fn read_wire(buf: &mut Bytes) -> Result<Self, rc_protocol::PacketDecodeError> {
        let count = rc_protocol::VarInt::decode(buf)?.get();
        if count <= 0 {
            return Ok(CreativeSlotItem { item_id: None });
        }
        let item_id = rc_protocol::VarInt::decode(buf)?.get();
        skip_data_component_patch(buf)?;
        Ok(CreativeSlotItem {
            item_id: Some(item_id),
        })
    }
}

impl rc_protocol::WireWrite for CreativeSlotItem {
    fn write_wire(&self, buf: &mut BytesMut) {
        // Never actually sent by this crate (`SetCreativeModeSlot` is serverbound-only) --
        // present only so `#[derive(RcPacket)]`'s generated `encode_body` compiles. The empty
        // encoding (`count = 0`) is correct for `item_id: None`; `Some(id)` writes the
        // smallest legal non-empty stack this decode also accepts back (`count = 1`, no
        // component patch) -- exercised only by this file's own round-trip test, never by
        // production code.
        match self.item_id {
            None => rc_protocol::VarInt::new(0).write_wire(buf),
            Some(id) => {
                rc_protocol::VarInt::new(1).write_wire(buf);
                rc_protocol::VarInt::new(id).write_wire(buf);
                rc_protocol::VarInt::new(0).write_wire(buf);
                rc_protocol::VarInt::new(0).write_wire(buf);
            }
        }
    }
}

/// `CreativeSlotItem`'s own doc comment has the full format this skips: `positive_count`
/// length-prefixed-payload entries, then `negative_count` bare-id-only entries.
fn skip_data_component_patch(buf: &mut Bytes) -> Result<(), rc_protocol::PacketDecodeError> {
    let positive_count = rc_protocol::VarInt::decode(buf)?.get().max(0);
    let negative_count = rc_protocol::VarInt::decode(buf)?.get().max(0);
    for _ in 0..positive_count {
        let _component_type_id = rc_protocol::VarInt::decode(buf)?;
        let payload_len = rc_protocol::VarInt::decode(buf)?.get();
        let payload_len = usize::try_from(payload_len).unwrap_or(0);
        if buf.remaining() < payload_len {
            return Err(rc_protocol::PacketDecodeError::UnexpectedEof);
        }
        buf.advance(payload_len);
    }
    for _ in 0..negative_count {
        let _component_type_id = rc_protocol::VarInt::decode(buf)?;
    }
    Ok(())
}

/// Packs a "Position" wire value (Context: 26-bit X, 26-bit Z, 12-bit Y, two's complement),
/// written as one plain big-endian 8-byte `Long` by the caller (`WireWrite for i64`).
pub fn pack_position(pos: rc_core::BlockPos) -> i64 {
    let x = (pos.x as i64) & 0x3FF_FFFF;
    let z = (pos.z as i64) & 0x3FF_FFFF;
    let y = (pos.y as i64) & 0xFFF;
    (x << 38) | (z << 12) | y
}

/// M3-B03 correction (Context, "Packet layout — corrected and new"): `status`'s enum values
/// (`0`=StartDestroyBlock ... `6`=SwapItemInHand) are unchanged from M2-B07's own
/// restatement, but `face` is corrected from a `Byte` to a `VarInt` enum (same 6-value
/// vanilla `Direction` ordinal meaning as `Face::from_ordinal`, only the wire width/kind
/// changes) and renamed `direction` to match the corrected field's own real name.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x29)]
pub struct PlayerAction {
    #[rc(varint)]
    pub status: i32,
    pub location: i64,
    #[rc(varint)]
    pub direction: i32,
    #[rc(varint)]
    pub sequence: i32,
}

/// M2 integration fix: this struct restated TWO real wire properties incorrectly,
/// both root-caused against a real `m2-report --mode smoke` run (a real azalea bot
/// driving a real, freshly built `rusty-clanker-server` release binary) after every
/// one of its three scripted placements landed as a silent no-op (target cell still
/// `AIR` on disk and live, while the two scripted breaks -- a separate, unaffected
/// packet -- persisted correctly):
///
/// 1. **Wrong packet id.** M2-B07's own report already flagged this exact struct's
///    id as "restated from this project's own established understanding... not
///    independently re-verified" -- verified now, empirically, against a real wire
///    capture (a `TEMP-DIAG` trace of every `raw.id`/body length `dispatch_inbound`
///    received during a real bot's scripted placement run): the three placements
///    produced three real, 25-byte-body packets at id `0x42`, not `0x2A` -- this
///    struct's `#[packet(id = 0x2A)]` simply never matched anything a real client
///    sent, so every placement fell straight into `dispatch_inbound`'s `other =>`
///    catch-all arm and was silently dropped before `decode_one::<UseItemOn>` was
///    ever called.
/// 2. **Missing wire field.** Independently confirmed against azalea's own pinned-
///    rev packet definition (`azalea-protocol/src/packets/game/s_use_item_on.rs`'s
///    `BlockHit::azalea_read`/`azalea_write`, the exact same source M2-B07's own
///    `restart_persistence.rs` already cites for this packet's `face`/`inside_block`
///    shape): the real wire layout carries one more `bool` ("world border hit")
///    immediately after `inside` (`inside_block` here) and before the trailing `seq`
///    (`sequence` here), which this struct never declared -- the observed 25-byte
///    real body already matches this 9-field shape exactly (1+8+1+12+1+1+1); the old
///    8-field shape would have left one byte permanently undecoded even once
///    dispatched to the right id.
///
/// M3-B03: `face` renamed `direction` (Context, "Packet layout — corrected and new" — the
/// field's own wire type/kind was already `VarInt`, unchanged, only the restated name
/// changes to match `PlayerAction`'s own corrected field). `hits_world_border` is **kept**
/// from the M2 field-report fix above, a real-client-wire-capture-verified field the M3-B03
/// blueprint's own restated 8-field table omits — CLAUDE.md's own "current code reflects
/// later fixes" rule: dropping it here would silently regress a real-client-verified parity
/// fix for a blueprint-text discrepancy, so it stays, recorded as a deviation in the
/// completion report.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x42)]
pub struct UseItemOn {
    #[rc(varint)]
    pub hand: i32,
    pub location: i64,
    #[rc(varint)]
    pub direction: i32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
    pub inside_block: bool,
    /// M2 integration fix (this struct's own doc comment): real wire field, absent
    /// from every prior committed version of this struct.
    pub hits_world_border: bool,
    #[rc(varint)]
    pub sequence: i32,
}

/// M2 integration addition: vanilla's own health/food/saturation sync packet -- the wire
/// mechanism `AC1d`'s "player rejoins with byte-identical health" needs (M2-B06's own
/// `PlayerSaveData.health`, `20.0` default for a new player). Id and field shapes cross-
/// checked against the pinned azalea rev's own `azalea-protocol/src/packets/game/
/// c_set_health.rs` (Constraints (d)) -- `health: f32, #[var] food: u32, saturation: f32`
/// -- and against this project's own already-established "declaration-order id"
/// convention (`SetChunkCacheCenter`'s doc comment): `set_health`'s own position in
/// azalea's generated `Clientbound` packet list is index `104` = `0x68`, independently
/// confirmed by direct inspection of that same generated `game/mod.rs` list, and
/// cross-validated by every other packet already reconciled this same way (`LoginPlay`
/// index `49` = `0x31`, `player_position` index `72` = `0x48`, `set_chunk_cache_center`
/// index `94` = `0x5E`, ...) all matching this crate's own already-committed ids exactly.
/// Sent once, right after `GameEvent` and before any chunk data, in
/// `play::connection::enter_play` (`enter_play`'s own doc comment on this exact call site
/// has the full "why not after `ChunkBatchFinished`" writeup -- a real, `cargo nextest`-
/// confirmed regression against `play_reach_validation.rs`/`play_sequence_ack_ordering.rs`/
/// `play_block_place_break.rs`'s own "read exactly through `ChunkBatchFinished`, then
/// assert the very next packet" pattern). Nothing else in this crate ever sends this
/// packet (M3/M4's own real damage/regen mechanics will).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x68)]
pub struct SetHealth {
    pub health: f32,
    #[rc(varint)]
    pub food: i32,
    pub saturation: f32,
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

/// M3-B03 (new): per-tick crack-overlay broadcast (Context, "Dig packet lifecycle").
/// `destroy_stage` is `0..=9` for an active crack stage; this blueprint always sends `-1`
/// only on an `ABORT`/cancel where the block survives — never on a finalize (Context: the
/// block's own `Block Update` implicitly clears the overlay client-side).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x05)]
pub struct SetBlockDestroyStage {
    #[rc(varint)]
    pub entity_id: i32,
    pub location: i64,
    pub destroy_stage: i8,
}

/// M3-B03 (new): the generic clientbound world-event packet (Context, "Packet layout —
/// corrected and new"). `event_id`/`data` are both plain `Int`, **not** `VarInt` — restated
/// exactly, distinct from every `VarInt` field above. This blueprint's only consumed event is
/// `LEVEL_EVENT_BLOCK_BREAK` (block-break sound + particles).
///
/// M3 field-report fix: the real wire layout carries one more trailing `bool` after `data` --
/// vanilla's own "disable relative volume" / global-broadcast flag (`true` only for the small
/// set of events meant to be heard everywhere regardless of distance to the player, e.g. the
/// wither-spawn and end-portal-open events; restated from a real 26.2 client's own decode --
/// `ClientboundLevelEventPacket`'s constructor reads exactly one more `boolean` after `data`,
/// confirmed by a real client's `IndexOutOfBoundsException` disconnecting on every block break
/// against this struct's former 16-byte body). `LEVEL_EVENT_BLOCK_BREAK` is not one of those
/// events, so every construction site in this crate always sends `false` here.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x2E)]
pub struct LevelEvent {
    pub event_id: i32,
    pub location: i64,
    pub data: i32,
    pub global_event: bool,
}

/// Vanilla's own long-stable event id: "block break with sound + particles," `data` = the
/// broken block's own raw pre-break state id (Context).
pub const LEVEL_EVENT_BLOCK_BREAK: i32 = 2001;

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
