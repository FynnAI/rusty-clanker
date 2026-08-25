//! M1 registry-sync-fix follow-up acceptance tests: `ClientboundUpdateTags` (Configuration-
//! state clientbound id `0x0D`) — docs/research/mc-26.2/26-registry-sync-configuration.md
//! §5.1/§5.2's wire structure. Decode/round-trip coverage plus a payload-content test built
//! from `rc-registries`' own real, committed generated tag tables (never the external local
//! data-generator output — that path exists only on the machine this fix was developed on).

use bytes::BytesMut;
use rc_protocol::{
    ClientboundUpdateTags, Identifier, RcPacket, TagEntry, TagRegistryEntry, VarInt, WireRead,
    WireWrite, decode_one,
};

#[test]
fn update_tags_packet_id_and_state() {
    assert_eq!(ClientboundUpdateTags::ID, 0x0D);
}

#[test]
fn tag_entry_roundtrip() {
    let entry = TagEntry {
        tag_id: Identifier::new("minecraft:infiniburn_overworld"),
        entries: vec![VarInt::new(285), VarInt::new(671)],
    };
    let mut buf = BytesMut::new();
    entry.write_wire(&mut buf);
    let mut bytes = buf.freeze();
    let decoded = TagEntry::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, entry);
}

#[test]
fn tag_entry_with_empty_members_is_a_legal_bound_empty_tag() {
    // §5.4: "bound empty" -- the tag id is present, `entries` is empty, this is a legal,
    // meaningful payload (real vanilla's own `dialog/pause_screen_additions`/`quick_actions`
    // shape), never omitted just because it has no members.
    let entry = TagEntry {
        tag_id: Identifier::new("minecraft:pause_screen_additions"),
        entries: vec![],
    };
    let mut buf = BytesMut::new();
    entry.write_wire(&mut buf);

    let mut expected = BytesMut::new();
    Identifier::new("minecraft:pause_screen_additions").write_wire(&mut expected);
    VarInt::new(0).write_wire(&mut expected); // member-count VarInt = 0
    assert_eq!(buf, expected);

    let mut bytes = buf.freeze();
    let decoded = TagEntry::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, entry);
}

#[test]
fn tag_registry_entry_roundtrip() {
    let entry = TagRegistryEntry {
        registry_id: Identifier::new("minecraft:block"),
        tags: vec![
            TagEntry {
                tag_id: Identifier::new("minecraft:infiniburn_overworld"),
                entries: vec![VarInt::new(285), VarInt::new(671)],
            },
            TagEntry {
                tag_id: Identifier::new("minecraft:infiniburn_end"),
                entries: vec![VarInt::new(34), VarInt::new(285), VarInt::new(671)],
            },
        ],
    };
    let mut buf = BytesMut::new();
    entry.write_wire(&mut buf);
    let mut bytes = buf.freeze();
    let decoded = TagRegistryEntry::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, entry);
}

#[test]
fn update_tags_packet_roundtrip_multiple_registries() {
    let packet = ClientboundUpdateTags {
        registries: vec![
            TagRegistryEntry {
                registry_id: Identifier::new("minecraft:block"),
                tags: vec![TagEntry {
                    tag_id: Identifier::new("minecraft:infiniburn_overworld"),
                    entries: vec![VarInt::new(285), VarInt::new(671)],
                }],
            },
            TagRegistryEntry {
                registry_id: Identifier::new("minecraft:dialog"),
                tags: vec![
                    TagEntry {
                        tag_id: Identifier::new("minecraft:pause_screen_additions"),
                        entries: vec![],
                    },
                    TagEntry {
                        tag_id: Identifier::new("minecraft:quick_actions"),
                        entries: vec![],
                    },
                ],
            },
        ],
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    let decoded = decode_one::<ClientboundUpdateTags>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn update_tags_packet_with_zero_registries_is_a_single_zero_varint() {
    let packet = ClientboundUpdateTags { registries: vec![] };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    assert_eq!(buf.as_ref(), &[0x00]);
    let decoded = decode_one::<ClientboundUpdateTags>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn update_tags_packet_exact_wire_bytes_for_one_registry_one_tag() {
    let packet = ClientboundUpdateTags {
        registries: vec![TagRegistryEntry {
            registry_id: Identifier::new("minecraft:timeline"),
            tags: vec![TagEntry {
                tag_id: Identifier::new("minecraft:in_overworld"),
                entries: vec![VarInt::new(0), VarInt::new(1)],
            }],
        }],
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);

    // Hand-built per §5.2's own wire shape: outer VarInt count (1), then registry_id
    // Identifier, inner VarInt tag-count (1), tag_id Identifier, VarInt member-count (2),
    // then two VarInt member ids.
    let mut expected = BytesMut::new();
    VarInt::new(1).write_wire(&mut expected); // outer registries count
    Identifier::new("minecraft:timeline").write_wire(&mut expected);
    VarInt::new(1).write_wire(&mut expected); // inner tags count
    Identifier::new("minecraft:in_overworld").write_wire(&mut expected);
    VarInt::new(2).write_wire(&mut expected); // member count
    VarInt::new(0).write_wire(&mut expected);
    VarInt::new(1).write_wire(&mut expected);

    assert_eq!(buf, expected);
}

// --- Payload-content test against `rc-registries`' own real, committed generated tables. ---

#[test]
fn generated_tag_tables_cover_the_docs_5_3_minimal_registry_set() {
    use rc_registries::generated_v776::tags::REGISTRIES;

    let registry_ids: Vec<&str> = REGISTRIES.iter().map(|(id, _)| *id).collect();
    for expected in [
        "minecraft:block",
        "minecraft:item",
        "minecraft:enchantment",
        "minecraft:dialog",
        "minecraft:timeline",
    ] {
        assert!(
            registry_ids.contains(&expected),
            "missing {expected} from the generated tag tables: {registry_ids:?}"
        );
    }
}

/// Round-3 real-client evidence (crash report `disconnect-2026-08-25_20.21.13-client.txt`)
/// proved the earlier 5-registry cherry-pick incomplete: 8 remaining `enchantment` decode
/// failures whose own codecs reference tags in registries the cherry-pick never sent. This
/// asserts the *complete* set — every registry directory actually present under the local
/// tags tree that resolves to a real, network-safe registry, not a hand-picked subset.
#[test]
fn generated_tag_tables_cover_the_complete_discovered_registry_set() {
    use rc_registries::generated_v776::tags::REGISTRIES;

    let registry_ids: Vec<&str> = REGISTRIES.iter().map(|(id, _)| *id).collect();
    for expected in [
        "minecraft:banner_pattern",
        "minecraft:block",
        "minecraft:damage_type",
        "minecraft:dialog",
        "minecraft:enchantment",
        "minecraft:entity_type",
        "minecraft:fluid",
        "minecraft:game_event",
        "minecraft:instrument",
        "minecraft:item",
        "minecraft:painting_variant",
        "minecraft:point_of_interest_type",
        "minecraft:potion",
        "minecraft:timeline",
        "minecraft:worldgen/biome",
    ] {
        assert!(
            registry_ids.contains(&expected),
            "missing {expected} from the generated tag tables: {registry_ids:?}"
        );
    }
    assert_eq!(
        registry_ids.len(),
        15,
        "expected exactly the 15 registries the real local datapack's tags tree resolves to \
         a network-safe registry, got: {registry_ids:?}"
    );

    // Never a server-only / not-network-safe registry (§3.2's own "absent from
    // SYNCHRONIZED_REGISTRIES" list, plus `villager_trade`, confirmed absent from both the
    // static report and SYNCHRONIZED_REGISTRIES).
    for forbidden in ["minecraft:villager_trade", "minecraft:worldgen/structure"] {
        assert!(
            !registry_ids.contains(&forbidden),
            "{forbidden} is not network-safe and must never appear: {registry_ids:?}"
        );
    }
}

/// The specific gap round 3 found and closed: `bane_of_arthropods`/`impaling`/`smite`'s own
/// `entity_effects` fields reference `entity_type` tags this server never sent before.
#[test]
fn generated_entity_type_tags_cover_the_round_3_enchantment_dependencies() {
    use rc_registries::generated_v776::tags::entity_type;

    assert!(
        !entity_type::SENSITIVE_TO_BANE_OF_ARTHROPODS
            .entries
            .is_empty()
    );
    assert!(!entity_type::SENSITIVE_TO_IMPALING.entries.is_empty());
    assert!(!entity_type::SENSITIVE_TO_SMITE.entries.is_empty());
}

/// The specific gap round 3 found and closed: `soul_speed`'s own `location_based_effects`
/// field references a `block` tag this server's earlier 3-tag cherry-pick never sent.
#[test]
fn generated_block_tags_cover_the_round_3_soul_speed_dependency() {
    use rc_registries::generated_v776::tags::block;

    assert!(!block::SOUL_SPEED_BLOCKS.entries.is_empty());
}

#[test]
fn generated_infiniburn_tags_carry_the_real_vanilla_block_ids() {
    use rc_registries::generated_v776::tags::block;

    // §4.3: real vanilla `infiniburn_overworld`/`infiniburn_nether` = {netherrack,
    // magma_block}; `infiniburn_end` additionally includes bedrock.
    assert_eq!(block::INFINIBURN_OVERWORLD.entries.len(), 2);
    assert_eq!(block::INFINIBURN_NETHER.entries.len(), 2);
    assert_eq!(
        block::INFINIBURN_OVERWORLD.entries,
        block::INFINIBURN_NETHER.entries
    );
    assert_eq!(block::INFINIBURN_END.entries.len(), 3);
    assert!(
        block::INFINIBURN_END
            .entries
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .is_superset(&block::INFINIBURN_OVERWORLD.entries.iter().collect()),
        "infiniburn_end must be a superset of infiniburn_overworld's own member ids"
    );
}

#[test]
fn generated_dialog_tags_are_present_but_legally_empty() {
    use rc_registries::generated_v776::tags::dialog;

    // Real vanilla data: both required §5.3 dialog tags currently have zero members --
    // "bound empty" is enough to fix the crash (§5.4), the tag id being present is what
    // matters.
    assert_eq!(dialog::PAUSE_SCREEN_ADDITIONS.entries, &[] as &[u32]);
    assert_eq!(dialog::QUICK_ACTIONS.entries, &[] as &[u32]);
    assert_eq!(dialog::TAGS.len(), 2);
}

#[test]
fn generated_enchantment_exclusive_sets_resolve_to_synchronized_registries_indices() {
    use rc_registries::generated_v776::tags::enchantment;

    // exclusive_set/armor = {protection, blast_protection, fire_protection,
    // projectile_protection}, whose indices in play::world::SYNCHRONIZED_REGISTRIES's own
    // `minecraft:enchantment` entry list are 28, 3, 11, 27 respectively.
    assert_eq!(enchantment::EXCLUSIVE_SET_ARMOR.entries, &[3, 11, 27, 28]);
}

#[test]
fn build_update_tags_payload_from_generated_tables_round_trips() {
    use rc_registries::generated_v776::tags::REGISTRIES as GENERATED;

    // Mirrors `rusty_clanker_server::net::configuration_flow::build_update_tags_packet`'s own
    // construction (never imported directly -- `rc-protocol` does not depend on
    // `rusty-clanker-server` -- so this restates the same mapping over the same real data to
    // prove the wire shape actually round-trips end to end).
    let registries: Vec<TagRegistryEntry> = GENERATED
        .iter()
        .filter(|(_, tags)| !tags.is_empty())
        .map(|(registry_id, tags)| TagRegistryEntry {
            registry_id: Identifier::new(*registry_id),
            tags: tags
                .iter()
                .map(|table| TagEntry {
                    tag_id: Identifier::new(table.tag_id),
                    entries: table
                        .entries
                        .iter()
                        .map(|id| VarInt::new(*id as i32))
                        .collect(),
                })
                .collect(),
        })
        .collect();
    let packet = ClientboundUpdateTags { registries };

    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    let decoded = decode_one::<ClientboundUpdateTags>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
    assert_eq!(decoded.registries.len(), GENERATED.len());
}
