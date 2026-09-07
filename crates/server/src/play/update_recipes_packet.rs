//! PLAN-D12 NET-hardening changeset 1: the `ClientboundUpdateRecipesPacket` wire
//! packet itself -- `update_recipes_data.rs`'s own doc comment has the full content
//! provenance and wire-shape citation. Hand-implemented `RcPacket` (a
//! `Vec<(String,Vec<u32>)>`-then-`Vec<(u32,u32,u8)>` shape has no
//! `#[derive(RcPacket)]` support, mirroring `commands_packet.rs`'s own identical
//! precedent for this class of packet).

use bytes::BufMut;
use rc_protocol::{Bytes, BytesMut, PacketDecodeError, RcPacket, VarInt};

use super::update_recipes_data::{PROPERTY_SETS, STONECUTTER_RECIPES};

pub struct UpdateRecipes;

impl RcPacket for UpdateRecipes {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x85;

    fn encode_body(&self, buf: &mut BytesMut) {
        VarInt::new(PROPERTY_SETS.len() as i32).encode(buf);
        for (key, items) in PROPERTY_SETS {
            VarInt::new(key.len() as i32).encode(buf);
            buf.put_slice(key.as_bytes());
            VarInt::new(items.len() as i32).encode(buf);
            for item in *items {
                VarInt::new(*item as i32).encode(buf);
            }
        }

        VarInt::new(STONECUTTER_RECIPES.len() as i32).encode(buf);
        for (input_item, output_item, output_count) in STONECUTTER_RECIPES {
            // `Ingredient.CONTENTS_STREAM_CODEC` (`ByteBufCodecs.holderSet`): raw
            // VarInt `n = 2` means "one direct entry follows" (`n - 1 == 1`,
            // `update_recipes_data.rs`'s own doc comment: every real stonecutter
            // input is a single direct item, never a tag).
            VarInt::new(2).encode(buf);
            VarInt::new(*input_item as i32).encode(buf);
            // `SlotDisplay` type `4` = `ItemStack` (`rc_registries::generated_v776::registries::
            // slot_display::ITEM_STACK`), then `ItemStackTemplate{item, count,
            // components}` with an always-empty component patch.
            VarInt::new(
                rc_registries::generated_v776::registries::slot_display::ITEM_STACK.0 as i32,
            )
            .encode(buf);
            VarInt::new(*output_item as i32).encode(buf);
            VarInt::new(*output_count as i32).encode(buf);
            VarInt::new(0).encode(buf); // component patch: positive_count
            VarInt::new(0).encode(buf); // component patch: negative_count
        }
    }

    fn decode_body(_buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        Err(PacketDecodeError::UnexpectedEof)
    }
}
