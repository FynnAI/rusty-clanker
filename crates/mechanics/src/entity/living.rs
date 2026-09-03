//! `LivingEntity`'s own rung (MECH-D29): adds health and sleeping bed position (both
//! NBT + metadata) and two genuinely metadata-only fields with no independent
//! `LivingEntity`-rung NBT key at all (Context, "`LivingEntity` NBT field set").

/// `LivingEntity`'s own rung (MECH-D29): adds health and sleeping bed position (both
/// NBT + metadata) and two genuinely metadata-only fields with no independent
/// `LivingEntity`-rung NBT key at all (Context, "`LivingEntity` NBT field set").
/// Exactly two fields, `health` and `sleeping_bed_pos`, carry `#[nbt(...)]` —
/// `hand_states`/`arrow_count`/`stinger_count` are metadata-only (no `#[nbt(...)]`
/// attribute at all), each defaulted via `Default::default()` on `read_nbt_fields` per
/// `EntityNbtFields` rule 2.
#[derive(
    Clone,
    Debug,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    rc_entity_macros::EntityNbtFields,
    rc_entity_macros::EntityMetadataFields,
)]
pub struct LivingEntity {
    #[net_metadata(index = 8, kind = "Byte")]
    pub hand_states: u8,
    #[nbt(name = "Health")]
    #[net_metadata(index = 9, kind = "Float")]
    pub health: f32,
    #[net_metadata(index = 12, kind = "VarInt")]
    pub arrow_count: i32,
    #[net_metadata(index = 13, kind = "VarInt")]
    pub stinger_count: i32,
    #[nbt(name = "sleeping_pos")]
    #[net_metadata(index = 14, kind = "OptionalPosition")]
    pub sleeping_bed_pos: Option<rc_core::BlockPos>,
}
