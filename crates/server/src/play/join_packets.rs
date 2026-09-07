//! PLAN-D12 NET-hardening changeset 1 ("the vanilla join sequence"). Every packet type
//! this file defines was reconciled against the frozen oracle protocol-diff capture
//! (`crates/testing/gametest/corpus/protocol-diff/known-divergences.ron`'s own
//! `"NET hardening: join sequence"` closer) byte-for-byte -- each struct's own doc
//! comment records the real observed body used to verify it, not merely a reading of
//! the ASSET-D18(f) reference. Field shapes are cross-checked against the pinned 26.2
//! decompiled reference (`network/protocol/game/Clientbound*.java`); ids against the
//! oracle's own real wire `packet_id` for that packet name, cross-validated against a
//! second, independent source: `rc_registries`' already-generated per-registry id
//! tables share the exact same "declaration-order id" numbering this project's own
//! `packets.rs` doc comments already established as this crate's own id-reconciliation
//! method (`SetChunkCacheCenter`'s doc comment).

use bytes::BufMut;
use rc_protocol::{
    Bytes, BytesMut, NbtTextComponent, PacketDecodeError, RcPacket, VarInt, WireWrite,
};

// ---------------------------------------------------------------------------------
// Simple, static-shape packets -- every field's real value is either a fixed vanilla
// default (a fresh, offline, peaceful-difficulty, creative-mode player on a flat
// preset world with no world-border configuration) or session-invariant.
// ---------------------------------------------------------------------------------

/// `ClientboundChangeDifficultyPacket(Difficulty, boolean)` -- real body observed at
/// join: `00 00` (difficulty `PEACEFUL` = the flat-preset oracle world's own real
/// configured difficulty, not locked). `Difficulty` is a small closed enum whose own
/// `STREAM_CODEC` is a single unsigned byte ordinal (0=peaceful,1=easy,2=normal,3=hard).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x0A)]
pub struct ChangeDifficulty {
    pub difficulty: u8,
    pub locked: bool,
}

pub const DIFFICULTY_PEACEFUL: u8 = 0;

/// `ClientboundPlayerAbilitiesPacket` (Context: distinct name from `SetCreativeModeSlot`'s
/// sibling serverbound `ServerboundPlayerAbilitiesPacket`, which this crate does not
/// define). Real body observed at join: `0d 3d4ccccd 3dcccccd` -- flags `0x0d`
/// (invulnerable|can_fly|instabuild, no fields for `flying` itself -- exactly creative
/// mode's own vanilla default), `flying_speed = 0.05`, `walking_speed = 0.1` (both
/// vanilla's own hardcoded `Abilities` defaults).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x40)]
pub struct PlayerAbilitiesClientbound {
    pub flags: u8,
    pub flying_speed: f32,
    pub walking_speed: f32,
}

pub const ABILITY_FLAG_INVULNERABLE: u8 = 0x01;
pub const ABILITY_FLAG_FLYING: u8 = 0x02;
pub const ABILITY_FLAG_CAN_FLY: u8 = 0x04;
pub const ABILITY_FLAG_INSTABUILD: u8 = 0x08;

/// `ClientboundSetHeldSlotPacket(int slot)` -- a genuinely new clientbound packet (not
/// this crate's own existing serverbound `SetCarriedItem`): informs the client which
/// hotbar slot the SERVER believes is selected, sent once at join. Real body observed:
/// `00` (slot 0 -- this crate's own `HotbarState::new()` join-time default, `connection.
/// rs`'s own doc comment).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x69)]
pub struct SetHeldSlotClientbound {
    #[rc(varint)]
    pub slot: i32,
}

/// `ClientboundSetExperiencePacket(float, int, int)` -- field order
/// `experience_progress, experience_level, total_experience` (decompiled-source-
/// verified; the class's own private-constructor read order, NOT its public
/// constructor's parameter order, which differs). Real body observed at join:
/// `00000000 00 00` -- all-zero (this crate's own persistence layer does not yet track
/// XP; a fresh player is honestly zero XP regardless).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x67)]
pub struct SetExperienceClientbound {
    pub experience_progress: f32,
    #[rc(varint)]
    pub experience_level: i32,
    #[rc(varint)]
    pub total_experience: i32,
}

/// `ClientboundTickingStatePacket(float tickRate, boolean isFrozen)` -- real body
/// observed at join: `41a00000 00` (`tick_rate = 20.0`, `is_frozen = false` -- vanilla's
/// own un-modified default tick rate manager state; this engine has no tick-freeze/
/// step-rate feature at all yet, so these two values are always this fixed default).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x7F)]
pub struct TickingState {
    pub tick_rate: f32,
    pub is_frozen: bool,
}

pub const DEFAULT_TICK_RATE: f32 = 20.0;

/// `ClientboundTickingStepPacket(int tickSteps)` -- real body observed at join: `00`
/// (no frozen ticks queued to run -- the tick-rate manager's own always-zero default,
/// same rationale as `TickingState`).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x80)]
pub struct TickingStep {
    #[rc(varint)]
    pub tick_steps: i32,
}

/// `ClientboundInitializeBorderPacket` -- field order decompiled-source-verified
/// (`ClientboundInitializeBorderPacket.write`). Real body observed at join (40 bytes):
/// `new_center_x/z = 0.0`, `old_size = new_size = 59999968.0` (vanilla's own real
/// default `WorldBorder` size for a freshly created world -- NOT the commonly-quoted
/// `6.0E7`), `lerp_time = 0`, `new_absolute_max_size = 29999984` (vanilla's own
/// hardcoded world-border hard limit), `warning_blocks = 5`, `warning_time = 300`
/// (vanilla's own defaults) -- every one of this engine's own worlds shares the
/// identical, un-configurable border today (no `/worldborder` command, no per-world
/// border persistence), so these are legitimately fixed constants, not a
/// per-world-state read.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x2B)]
pub struct InitializeBorder {
    pub new_center_x: f64,
    pub new_center_z: f64,
    pub old_size: f64,
    pub new_size: f64,
    #[rc(varint)]
    pub lerp_time: i64,
    #[rc(varint)]
    pub new_absolute_max_size: i32,
    #[rc(varint)]
    pub warning_blocks: i32,
    #[rc(varint)]
    pub warning_time: i32,
}

pub const DEFAULT_BORDER_SIZE: f64 = 59_999_968.0;
pub const DEFAULT_BORDER_ABSOLUTE_MAX_SIZE: i32 = 29_999_984;
pub const DEFAULT_BORDER_WARNING_BLOCKS: i32 = 5;
pub const DEFAULT_BORDER_WARNING_TIME: i32 = 300;

/// `ClientboundServerDataPacket(Component motd, Optional<byte[]> iconBytes)` --
/// decompiled-source-verified: in protocol 776 this record carries only these two
/// fields (`enforcesSecureChat` has moved elsewhere; the oracle's own real 22-byte body
/// -- a bare NBT `TAG_String` motd plus one trailing `0x00` byte -- has no room for a
/// third field). `icon_present` is always `false` -- this engine never generates a
/// server-list icon -- and `bool`'s own `WireWrite` already writes the exact
/// single-byte `0x00`/`0x01` `Optional<byte[]>` presence-flag encoding
/// (`ByteBufCodecs.optional`'s own presence-flag-then-payload shape degenerates to
/// exactly one byte, `0x00`, when the flag is `false`), so no dedicated
/// `Optional<Vec<u8>>` wire type is needed. Real body observed at join:
/// `08 0012 "A Minecraft Server" 00` -- a real-client protocol-diff run caught this
/// changeset's own first attempt naming (and setting) this field backwards
/// (`icon_absent: true` wrote the wrong trailing byte, `0x01`; the completion report
/// has the full field-report writeup).
#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x56)]
pub struct ServerData {
    pub motd: NbtTextComponent,
    pub icon_present: bool,
}

/// The oracle's own real vanilla default MOTD (`server.properties`' own default
/// `motd=A Minecraft Server`) -- every real capture this changeset verified against
/// used this exact value.
pub const DEFAULT_MOTD: &str = "A Minecraft Server";

/// `ClientboundRecipeBookSettingsPacket` -- eight plain `bool`s, one open+one filtering
/// flag per recipe-book category (crafting, furnace, blast furnace, smoker; real field
/// declaration order decompiled-source-verified). Real body observed at join: eight
/// `0x00` bytes -- a fresh player's recipe book starts fully closed and unfiltered in
/// every category, and this engine has no persisted recipe-book-UI-state to read back
/// yet, so this fixed all-false default is honest for every join today.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x4C)]
pub struct RecipeBookSettings {
    pub crafting_open: bool,
    pub crafting_filter: bool,
    pub furnace_open: bool,
    pub furnace_filter: bool,
    pub blast_furnace_open: bool,
    pub blast_furnace_filter: bool,
    pub smoker_open: bool,
    pub smoker_filter: bool,
}

impl RecipeBookSettings {
    pub const CLOSED: RecipeBookSettings = RecipeBookSettings {
        crafting_open: false,
        crafting_filter: false,
        furnace_open: false,
        furnace_filter: false,
        blast_furnace_open: false,
        blast_furnace_filter: false,
        smoker_open: false,
        smoker_filter: false,
    };
}

// ---------------------------------------------------------------------------------
// `ContainerSetContent` -- the player's own 46-slot inventory snapshot sent once at
// join. This engine has no real item-inventory system yet (M4's own future scope,
// `packets.rs`'s own `CreativeSlotItem` doc comment) -- every slot, and the carried
// item, is honestly empty for every join today.
// ---------------------------------------------------------------------------------

/// One `ItemStack.OPTIONAL_STREAM_CODEC` slot -- real oracle bytes confirm the
/// "absent" encoding is exactly one byte, `VarInt(0)` (the same "`count <= 0` -> no
/// further bytes" shape `packets.rs`'s own `CreativeSlotItem` already establishes for
/// the *serverbound* creative-slot packet; the *clientbound* `ItemStack` codec shares
/// the identical empty-stack encoding).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptySlot;

impl rc_protocol::WireWrite for EmptySlot {
    fn write_wire(&self, buf: &mut BytesMut) {
        VarInt::new(0).encode(buf);
    }
}
impl rc_protocol::WireRead for EmptySlot {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        let count = VarInt::decode(buf)?.get();
        if count > 0 {
            // A present, non-empty stack this type has no need to interpret in
            // production (this crate never sends one) -- decode support exists only
            // for this file's own round-trip test, which never constructs one.
            return Err(PacketDecodeError::UnexpectedEof);
        }
        Ok(EmptySlot)
    }
}

/// `ClientboundContainerSetContentPacket(int containerId, int stateId, List<ItemStack>
/// items, ItemStack carriedItem)`. `container_id` is `ByteBufCodecs.CONTAINER_ID`, a
/// plain unsigned byte (not VarInt -- decompiled-source-verified: `0` for the player's
/// own inventory menu, the only menu this engine ever opens at join). `state_id` is a
/// `AbstractContainerMenu`-tracked revision counter; real body observed at join is
/// `state_id = 1` (vanilla's own menu construction bumps it once before the initial
/// send). Real body observed: `00 01 2e` + 46×`00` (empty slots) + `00` (empty carried
/// item) = 50 bytes total, matching this struct's own field-by-field encoding exactly.
#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x12)]
pub struct ContainerSetContent {
    pub container_id: u8,
    #[rc(varint)]
    pub state_id: i32,
    #[rc(prefixed_array = "VarInt")]
    pub items: Vec<EmptySlot>,
    pub carried_item: EmptySlot,
}

/// The player-inventory container's own full slot count (`InventoryMenu`'s 4 crafting,
/// 4 armor, 27 main, 9 hotbar, and 1 offhand slot summing to 46 -- matches the
/// oracle's own real `items` count exactly).
pub const PLAYER_INVENTORY_SLOT_COUNT: usize = 46;

impl ContainerSetContent {
    /// The join-time snapshot every connection sends today: container `0` (the
    /// player's own inventory), `state_id = 1`, every slot and the carried item empty.
    pub fn empty_player_inventory() -> Self {
        ContainerSetContent {
            container_id: 0,
            state_id: 1,
            items: vec![EmptySlot; PLAYER_INVENTORY_SLOT_COUNT],
            carried_item: EmptySlot,
        }
    }
}

// ---------------------------------------------------------------------------------
// Play-state `Disconnect` and `PlayerInfoRemove` -- server-initiated-close and
// leave-broadcast plumbing (Context's own `ServerCommonPacketListenerImpl`/
// `PlayerList.remove` citation).
// ---------------------------------------------------------------------------------

/// `ClientboundDisconnectPacket(Component reason)` -- the SAME `network.protocol.
/// common` class the Configuration-state `Disconnect`
/// (`net::configuration_flow.rs::send_disconnect`) already sends via the identical
/// `NbtTextComponent`-collapsed-string encoding; only the enclosing Play-state packet
/// id differs (`GameProtocols`' own declaration-order id, cross-checked against
/// `PlayerInfoRemove`'s doc comment's identical method: row 32 of the clientbound
/// `addPacket` chain = `0x20`). Never observed with real content in any available
/// oracle capture (every scripted session step's own bot either stays connected for
/// its whole capture window or disconnects client-side, which sends nothing back) --
/// the wire SHAPE is decompiled-source-verified and matches the already-proven
/// Configuration-state sibling exactly; the reason TEXT this crate actually sends
/// (below) is this crate's own choice, not independently oracle-verified.
#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x20)]
pub struct Disconnect {
    pub reason: NbtTextComponent,
}

/// `ClientboundPlayerInfoRemovePacket(List<UUID> profileIds)` -- a VarInt-count-
/// prefixed list of raw 16-byte UUIDs (`UUIDUtil.STREAM_CODEC`, decompiled-source-
/// verified: `mostSigBits`/`leastSigBits` as two big-endian `i64`s back to back, the
/// same byte layout `u128::to_be_bytes` already produces for a Java-style UUID value).
/// Hand-implemented (a `Vec<u128>` has no default `#[rc(prefixed_array = ...)]` element
/// codec) rather than adding a project-wide `WireWrite for u128` this crate's other
/// packets have no need for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInfoRemove {
    pub profile_ids: Vec<u128>,
}

impl RcPacket for PlayerInfoRemove {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x45;

    fn encode_body(&self, buf: &mut BytesMut) {
        VarInt::new(self.profile_ids.len() as i32).encode(buf);
        for id in &self.profile_ids {
            buf.put_u128(*id);
        }
    }

    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        use bytes::Buf;
        let count = VarInt::decode(buf)?.get();
        if count < 0 {
            return Err(PacketDecodeError::UnexpectedEof);
        }
        let mut profile_ids = Vec::with_capacity(count as usize);
        for _ in 0..count {
            if buf.remaining() < 16 {
                return Err(PacketDecodeError::UnexpectedEof);
            }
            profile_ids.push(buf.get_u128());
        }
        Ok(PlayerInfoRemove { profile_ids })
    }
}

// ---------------------------------------------------------------------------------
// `SystemChat` join/leave messages -- `PlayerList.placeNewPlayer`/`remove`'s own
// `Component.translatable("multiplayer.player.joined"/"...left", displayName).
// withStyle(YELLOW)` broadcasts, restated in our own words. Decompiled-source-
// verified shape (`ComponentSerialization`'s own "collapse to a bare string" rule does
// NOT apply here -- a translatable component with an argument never collapses); no
// available oracle capture actually contains one (every captured session step is
// solo, and `PlayerList.placeNewPlayer` broadcasts the join message BEFORE adding the
// new player to its own `players` list -- decompiled-source-verified,
// `PlayerList.java` -- so a solo joiner never receives their own join message; a leave
// message is symmetric). This encoding is therefore reference-derived only, not
// byte-verified against a real capture; recorded as a deviation in the completion
// report.
// ---------------------------------------------------------------------------------

/// `ClientboundSystemChatPacket(Component content, boolean overlay)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemChat {
    pub content: JoinLeaveMessage,
    pub overlay: bool,
}

impl RcPacket for SystemChat {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x79;

    fn encode_body(&self, buf: &mut BytesMut) {
        self.content.write_wire(buf);
        buf.put_u8(if self.overlay { 1 } else { 0 });
    }

    fn decode_body(_buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        // Never decoded in production (this crate never receives a clientbound
        // packet) -- present only so `RcPacket` is fully implemented.
        Err(PacketDecodeError::UnexpectedEof)
    }
}

/// `Component.translatable(key, PlainText(name)).withStyle(YELLOW)` -- the exact
/// vanilla shape `PlayerList.placeNewPlayer`/`.remove` construct (`Advancement.name`'s
/// own sibling `TranslatableContents` codec, this module's own doc comment has the
/// full field-order derivation): an NBT compound with, in this order, `"translate"`
/// (the message key), `"with"` (a one-element list containing the player's own display
/// name, collapsed to a bare NBT string since a fresh player's display name carries no
/// style or siblings -- `TranslatableContents.ARG_CODEC`'s own `tryCollapseToString`
/// step), and `"color"` (`Style.Serializer`'s own first-declared field, `"yellow"` --
/// `ChatFormatting.YELLOW.getName()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinLeaveMessage {
    pub key: &'static str,
    pub player_name: String,
}

pub const JOIN_MESSAGE_KEY: &str = "multiplayer.player.joined";
pub const LEAVE_MESSAGE_KEY: &str = "multiplayer.player.left";

impl rc_protocol::WireWrite for JoinLeaveMessage {
    fn write_wire(&self, buf: &mut BytesMut) {
        buf.put_u8(0x0A); // TAG_Compound (unnamed root)

        buf.put_u8(0x08); // TAG_String
        write_nbt_key(buf, "translate");
        write_nbt_string_value(buf, self.key);

        buf.put_u8(0x09); // TAG_List
        write_nbt_key(buf, "with");
        buf.put_u8(0x08); // element type: TAG_String
        buf.put_i32(1); // one element
        write_nbt_string_value(buf, &self.player_name);

        buf.put_u8(0x08); // TAG_String
        write_nbt_key(buf, "color");
        write_nbt_string_value(buf, "yellow");

        buf.put_u8(0x00); // TAG_End
    }
}

fn write_nbt_key(buf: &mut BytesMut, key: &str) {
    buf.put_u16(key.len() as u16);
    buf.put_slice(key.as_bytes());
}

fn write_nbt_string_value(buf: &mut BytesMut, value: &str) {
    buf.put_u16(value.len() as u16);
    buf.put_slice(value.as_bytes());
}
