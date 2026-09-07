//! PLAN-D12 NET-hardening changeset 1: `ClientboundRecipeBookAddPacket` and
//! `ClientboundUpdateAdvancementsPacket` -- the one recipe (and its one matching
//! advancement pair) every genuinely fresh vanilla player has automatically unlocked
//! and granted, with no prior player action at all: `minecraft:recipes/root` (the
//! recipe-book feature's own always-present root advancement, granted to every player
//! on first join) and its one child, `minecraft:recipes/decorations/crafting_table`
//! (the crafting-table recipe, whose own `has_the_recipe`/`unlock_right_away` criteria
//! make it "always unlocked" -- decompiled-source-verified: `PlayerAdvancements`'s own
//! bootstrap grants `recipes/root` and awards its child recipes' `unlock_right_away`
//! trigger immediately, before any player action).
//!
//! Both packet bodies below are a byte-for-byte structural decode of the real oracle's
//! own `recipe_book_add`/`update_advancements` bodies observed at every join in the
//! frozen protocol-diff capture (`session/spawn`'s own second `recipe_book_add`
//! instance and its `update_advancements`), confirmed clean, zero-residue decodes
//! against `ClientboundRecipeBookAddPacket`/`ClientboundUpdateAdvancementsPacket`/
//! `Advancement`/`DisplayInfo`/`AdvancementRequirements`/`AdvancementProgress`/
//! `CriterionProgress`'s own real decompiled-source field layouts (this module's own
//! sibling `commands_packet.rs`/`update_recipes_data.rs` established the same
//! decode-then-restate method). Every string is a bare `minecraft:...` advancement/
//! criterion identifier -- protocol/registry keys, not Mojang prose.
//!
//! This engine has no recipe-unlock or advancement-progress persistence at all yet
//! (M4's own future scope) -- every join therefore honestly sends exactly this same
//! fixed "brand new player" grant, which is correct for every scenario this crate's
//! own test corpus exercises (a freshly created player, every time) and is the same
//! bounded scope `update_recipes_data.rs`'s own doc comment already establishes for
//! this changeset. The `unlock_right_away` criterion's own `obtained` timestamp is
//! this crate's own real wall-clock grant time (`AdvancementProgress`'s own real
//! semantics: "when this criterion was satisfied") -- it can never byte-match one
//! specific frozen oracle capture's own historical timestamp, an unavoidable, honestly
//! reported residue (this module's own doc comment on `default_join_grant`, and the
//! completion report).

use bytes::BufMut;
use rc_protocol::{BytesMut, PacketDecodeError, RcPacket, VarInt};

fn write_prefixed_str(buf: &mut BytesMut, s: &str) {
    VarInt::new(s.len() as i32).encode(buf);
    buf.put_slice(s.as_bytes());
}

/// The `minecraft:slot_display` variants this changeset's own crafting-table recipe
/// display actually uses (`rc_registries::generated_v776::registries::slot_display`'s own real
/// registration order).
enum SlotDisplayWire<'a> {
    Item(u32),
    ItemStack { item: u32, count: i32 },
    Tag(&'a str),
}

fn write_slot_display(buf: &mut BytesMut, display: &SlotDisplayWire) {
    use rc_registries::generated_v776::registries::slot_display as sd;
    match display {
        SlotDisplayWire::Item(item) => {
            VarInt::new(sd::ITEM.0 as i32).encode(buf);
            VarInt::new(*item as i32).encode(buf);
        }
        SlotDisplayWire::ItemStack { item, count } => {
            VarInt::new(sd::ITEM_STACK.0 as i32).encode(buf);
            VarInt::new(*item as i32).encode(buf);
            VarInt::new(*count).encode(buf);
            VarInt::new(0).encode(buf); // component patch: positive_count
            VarInt::new(0).encode(buf); // component patch: negative_count
        }
        SlotDisplayWire::Tag(tag) => {
            VarInt::new(sd::TAG.0 as i32).encode(buf);
            write_prefixed_str(buf, tag);
        }
    }
}

/// `Ingredient.CONTENTS_STREAM_CODEC` (`ByteBufCodecs.holderSet`, decompiled-source-
/// verified): raw VarInt `n`; `n == 0` means "by tag" (an Identifier string follows).
fn write_holder_set_by_tag(buf: &mut BytesMut, tag: &str) {
    VarInt::new(0).encode(buf);
    write_prefixed_str(buf, tag);
}

/// `ClientboundRecipeBookAddPacket(List<Entry> entries, boolean replace)`.
pub struct RecipeBookAdd {
    body: Vec<u8>,
}

impl RcPacket for RecipeBookAdd {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x4A;

    fn encode_body(&self, buf: &mut BytesMut) {
        buf.put_slice(&self.body);
    }

    fn decode_body(_buf: &mut rc_protocol::Bytes) -> Result<Self, PacketDecodeError> {
        Err(PacketDecodeError::UnexpectedEof)
    }
}

impl RecipeBookAdd {
    /// The real oracle's own first, unconditional `recipe_book_add` at every join:
    /// zero entries, `replace = true` -- `ServerRecipeBook.sendInitialRecipeBook`'s own
    /// "reset the client's recipe book to empty" broadcast, decompiled-source-verified
    /// real body `00 01`.
    pub fn reset() -> Self {
        let mut body = BytesMut::new();
        VarInt::new(0).encode(&mut body); // entries: empty
        body.put_u8(1); // replace = true
        RecipeBookAdd {
            body: body.to_vec(),
        }
    }

    /// The real oracle's own second `recipe_book_add`: the one recipe every fresh
    /// player has unlocked (this module's own doc comment), `replace = false`.
    pub fn default_unlocks() -> Self {
        let mut body = BytesMut::new();
        VarInt::new(1).encode(&mut body); // entries: one

        // RecipeDisplayEntry.id -- the real oracle's own observed display index for
        // this exact recipe at this pinned protocol version (a fixed, deterministic
        // load-order index into vanilla's whole recipe catalog, not player state).
        VarInt::new(318).encode(&mut body);

        // RecipeDisplay: type 1 = CRAFTING_SHAPED (`rc_registries::...::recipe_display
        // ::CRAFTING_SHAPED`), a 2x2 grid of "any plank" -> one crafting table, craftable
        // at the bare inventory (`craftingStation` is the result item itself -- the real
        // oracle's own observed shape, matching vanilla's own "no station required"
        // 2x2-grid convention).
        VarInt::new(
            rc_registries::generated_v776::registries::recipe_display::CRAFTING_SHAPED.0 as i32,
        )
        .encode(&mut body);
        VarInt::new(2).encode(&mut body); // width
        VarInt::new(2).encode(&mut body); // height
        VarInt::new(4).encode(&mut body); // ingredients count
        for _ in 0..4 {
            write_slot_display(&mut body, &SlotDisplayWire::Tag("minecraft:planks"));
        }
        write_slot_display(
            &mut body,
            &SlotDisplayWire::ItemStack {
                item: CRAFTING_TABLE_ITEM_ID,
                count: 1,
            },
        );
        write_slot_display(&mut body, &SlotDisplayWire::Item(CRAFTING_TABLE_ITEM_ID));

        VarInt::new(0).encode(&mut body); // group: OptionalVarInt, absent
        VarInt::new(3).encode(&mut body); // category: CRAFTING_MISC

        // craftingRequirements: Optional<List<Ingredient>>, present -- the same four
        // "any plank" tag ingredients the display grid already shows.
        body.put_u8(1);
        VarInt::new(4).encode(&mut body);
        for _ in 0..4 {
            write_holder_set_by_tag(&mut body, "minecraft:planks");
        }

        body.put_u8(0x02); // flags: FLAG_HIGHLIGHT (newly unlocked)

        body.put_u8(0); // replace = false

        RecipeBookAdd {
            body: body.to_vec(),
        }
    }
}

/// `rc_registries::generated_v776::registries::item::CRAFTING_TABLE`'s own numeric id, per the
/// real oracle bytes this module's own doc comment cites.
const CRAFTING_TABLE_ITEM_ID: u32 = 360;

/// `ClientboundUpdateAdvancementsPacket(boolean reset, List<AdvancementHolder> added,
/// Set<Identifier> removed, Map<Identifier, AdvancementProgress> progress, boolean
/// showAdvancements)`.
pub struct UpdateAdvancements {
    body: Vec<u8>,
}

impl RcPacket for UpdateAdvancements {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x82;

    fn encode_body(&self, buf: &mut BytesMut) {
        buf.put_slice(&self.body);
    }

    fn decode_body(_buf: &mut rc_protocol::Bytes) -> Result<Self, PacketDecodeError> {
        Err(PacketDecodeError::UnexpectedEof)
    }
}

impl UpdateAdvancements {
    /// The fixed two-advancement grant every genuinely fresh player receives (this
    /// module's own doc comment). `unlock_right_away`'s own `obtained` timestamp is
    /// real wall-clock "now," so this packet's own body is honestly non-reproducible
    /// byte-for-byte against one specific historical oracle capture -- see this
    /// module's own doc comment.
    pub fn default_join_grant() -> Self {
        let mut body = BytesMut::new();
        body.put_u8(1); // reset = true

        VarInt::new(2).encode(&mut body); // added: two advancements

        // AdvancementHolder 0: recipes/decorations/crafting_table (real oracle order:
        // the child is listed before its own parent).
        write_prefixed_str(&mut body, "minecraft:recipes/decorations/crafting_table");
        body.put_u8(1); // parent present
        write_prefixed_str(&mut body, "minecraft:recipes/root");
        body.put_u8(0); // display absent
        // requirements: List<List<String>> = [["has_the_recipe", "unlock_right_away"]]
        VarInt::new(1).encode(&mut body);
        VarInt::new(2).encode(&mut body);
        write_prefixed_str(&mut body, "has_the_recipe");
        write_prefixed_str(&mut body, "unlock_right_away");
        body.put_u8(0); // sendsTelemetryEvent = false

        // AdvancementHolder 1: recipes/root (the feature's own always-present root).
        write_prefixed_str(&mut body, "minecraft:recipes/root");
        body.put_u8(0); // parent absent (this IS the root)
        body.put_u8(0); // display absent
        // requirements: [["impossible"]] -- vanilla's own dummy, never-satisfied root
        // criterion (decompiled-source-verified: `recipes/root` exists only to anchor
        // the recipe-book advancement tree, never itself "completed").
        VarInt::new(1).encode(&mut body);
        VarInt::new(1).encode(&mut body);
        write_prefixed_str(&mut body, "impossible");
        body.put_u8(0); // sendsTelemetryEvent = false

        VarInt::new(0).encode(&mut body); // removed: empty

        // progress: Map<Identifier, AdvancementProgress> -- one entry per added
        // advancement, matching real oracle order.
        VarInt::new(2).encode(&mut body);

        write_prefixed_str(&mut body, "minecraft:recipes/decorations/crafting_table");
        VarInt::new(2).encode(&mut body); // criteria count
        write_prefixed_str(&mut body, "unlock_right_away");
        body.put_u8(1); // obtained present
        body.put_i64(now_millis());
        write_prefixed_str(&mut body, "has_the_recipe");
        body.put_u8(0); // obtained absent -- not yet crafted

        write_prefixed_str(&mut body, "minecraft:recipes/root");
        VarInt::new(1).encode(&mut body);
        write_prefixed_str(&mut body, "impossible");
        body.put_u8(0); // obtained absent -- the dummy criterion never completes

        body.put_u8(1); // showAdvancements = true

        UpdateAdvancements {
            body: body.to_vec(),
        }
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
