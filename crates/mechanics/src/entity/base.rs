//! `BaseEntity` — the fixed bundle every entity carries (MECH-D29's "Base bundle"),
//! corrected per this blueprint's own cited addition of `Pos` (Context).

use crate::entity::ids::EntityUuid;
use crate::entity::metadata::Pose;

/// The fixed bundle every entity carries (MECH-D29's "Base bundle"), corrected per
/// this blueprint's own cited addition of `Pos` (Context). Every `#[net_metadata(...)]`-
/// carrying field below is declared in strictly ascending index order (0-7:
/// `status_flags`, `air_ticks`, `custom_name`, `custom_name_visible`, `silent`,
/// `no_gravity`, `pose`, `ticks_frozen`) — `#[derive(EntityMetadataFields)]` enforces
/// this at compile time by comparing each successive `#[net_metadata(...)]`-carrying
/// field's index to the previous one; fields without `#[net_metadata(...)]` (`pos`,
/// `velocity`, `rotation`, `fall_distance`, `fire_ticks`, `on_ground`, `invulnerable`,
/// `portal_cooldown`, `uuid`, `glowing`, `has_visual_fire`) are exempt from that check
/// and may be interleaved anywhere, but do not reorder the `#[net_metadata(...)]`-
/// carrying fields relative to each other.
#[derive(
    Clone,
    Debug,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    rc_entity_macros::EntityNbtFields,
    rc_entity_macros::EntityMetadataFields,
    bevy_ecs::prelude::Component,
)]
pub struct BaseEntity {
    #[nbt(name = "Pos")]
    pub pos: [f64; 3],
    #[nbt(name = "Motion")]
    pub velocity: [f64; 3],
    #[nbt(name = "Rotation")]
    pub rotation: [f32; 2],
    #[nbt(name = "fall_distance")]
    pub fall_distance: f64,
    #[nbt(name = "Fire")]
    pub fire_ticks: i16,
    /// Metadata-only (index 0, the shared status-flags byte) — computed from
    /// `on_ground`/`glowing`/etc. at encode time, never itself stored to NBT under
    /// this name. Bit layout (research doc §3.1 `DATA_SHARED_FLAGS_ID`): bit 0 = on
    /// fire, bit 1 = sneaking, bit 3 = sprinting, bit 4 = swimming, bit 5 = invisible,
    /// bit 6 = glowing, bit 7 = elytra-flying.
    #[net_metadata(index = 0, kind = "Byte")]
    pub status_flags: u8,
    #[nbt(name = "Air")]
    #[net_metadata(index = 1, kind = "VarInt")]
    pub air_ticks: i32,
    #[nbt(name = "OnGround")]
    pub on_ground: bool,
    #[nbt(name = "Invulnerable")]
    pub invulnerable: bool,
    #[nbt(name = "PortalCooldown")]
    pub portal_cooldown: i32,
    #[nbt(name = "UUID")]
    pub uuid: EntityUuid,
    /// Patch-preserving raw NBT (Context, corrected): stores whichever tag vanilla
    /// actually wrote (bare `TAG_String` for a plain-text name, `TAG_Compound` for a
    /// richer one), re-emitted unchanged on save. `custom_name_text` (below) is the
    /// plain-text-only accessor; the `#[net_metadata(...)]` wire value is derived from
    /// the same extraction rule via `MetadataValue`'s own `From<Option<owned::NbtTag>>`
    /// impl (`metadata.rs`), not from this field's raw type directly.
    #[nbt(name = "CustomName")]
    #[net_metadata(index = 2, kind = "OptionalTextComponent")]
    #[serde(with = "crate::entity::nbt::nbt_tag_serde")]
    pub custom_name: Option<rc_nbt::owned::NbtTag>,
    #[nbt(name = "CustomNameVisible")]
    #[net_metadata(index = 3, kind = "Boolean")]
    pub custom_name_visible: bool,
    #[nbt(name = "Silent")]
    #[net_metadata(index = 4, kind = "Boolean")]
    pub silent: bool,
    #[nbt(name = "NoGravity")]
    #[net_metadata(index = 5, kind = "Boolean")]
    pub no_gravity: bool,
    #[nbt(name = "Glowing")]
    pub glowing: bool,
    /// Metadata-only (index 6). Defaults to `Pose::Standing` on load (`EntityNbtFields`
    /// rule 2 — no `#[nbt(...)]` attribute present).
    #[net_metadata(index = 6, kind = "Pose")]
    pub pose: Pose,
    #[nbt(name = "TicksFrozen")]
    #[net_metadata(index = 7, kind = "VarInt")]
    pub ticks_frozen: i32,
    #[nbt(name = "HasVisualFire")]
    pub has_visual_fire: bool,
}

impl BaseEntity {
    /// `Some(text)` only when `custom_name` is the bare `TAG_String` form; `None` for
    /// the `TAG_Compound` form, any other tag shape, or `custom_name` itself being
    /// `None`. Every text this codebase actually carries anywhere is plain ASCII
    /// (`rc_protocol::wire::NbtTextComponent`'s own identical, already-established
    /// stance) — `Mutf8Str::to_str` returns a zero-copy `Cow::Borrowed` for exactly
    /// that case, which is the only case this accessor extracts a reference from; a
    /// non-ASCII name (requiring MUTF-8 re-decoding into an owned `String`) is a
    /// bounded limitation this borrowed-`&str` signature cannot express, restated
    /// here rather than silently mishandled.
    pub fn custom_name_text(&self) -> Option<&str> {
        self.custom_name
            .as_ref()
            .and_then(crate::entity::metadata::extract_custom_name_text)
    }
}
