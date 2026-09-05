//! TEST-D59's own "packet name must resolve" validation source for
//! `known_divergences::load_register` — the clientbound packet resource names for
//! every protocol state this harness's own `SESSION_STEPS`/redstone-corpus-over-the-
//! wire pass ever observes (login, configuration, play), copied verbatim (namespaced
//! resource-name keys only — no other `packets.json` content, e.g. `protocol_id`, is
//! reproduced) from the ASSET-D18(f) reference's own datagen `reports/packets.json`
//! for the pinned 26.2 / protocol 776 target (NET-D1). This is data, not Mojang's own
//! expression (ASSET-D18/D19's own "behavior, structures, constants... reimplemented"
//! boundary) — the same "generated ID catalog, never copied code" status this
//! project's own `blocks.json`-derived reports and `crates/registries/generated/
//! v776/` already carry. Sorted, deduplicated; regenerating this table for a future
//! pinned-version bump is a governance changeset (TEST-D46), never something an
//! implementation changeset may do on its own initiative.

/// `packets.json["login"]["clientbound"]` — every step this harness names
/// `session/login` resolves register entries against this table
/// (`known_divergences::state_catalog`).
pub const LOGIN_CLIENTBOUND_PACKET_NAMES: &[&str] = &[
    "minecraft:cookie_request",
    "minecraft:custom_query",
    "minecraft:hello",
    "minecraft:login_compression",
    "minecraft:login_disconnect",
    "minecraft:login_finished",
];

/// `packets.json["configuration"]["clientbound"]` — every step this harness names
/// `session/configuration` resolves register entries against this table.
pub const CONFIGURATION_CLIENTBOUND_PACKET_NAMES: &[&str] = &[
    "minecraft:clear_dialog",
    "minecraft:code_of_conduct",
    "minecraft:cookie_request",
    "minecraft:custom_payload",
    "minecraft:custom_report_details",
    "minecraft:disconnect",
    "minecraft:finish_configuration",
    "minecraft:keep_alive",
    "minecraft:ping",
    "minecraft:registry_data",
    "minecraft:reset_chat",
    "minecraft:resource_pack_pop",
    "minecraft:resource_pack_push",
    "minecraft:select_known_packs",
    "minecraft:server_links",
    "minecraft:show_dialog",
    "minecraft:store_cookie",
    "minecraft:transfer",
    "minecraft:update_enabled_features",
    "minecraft:update_tags",
];

/// `packets.json["play"]["clientbound"]` — every step this harness other than
/// `session/login`/`session/configuration` (every other scripted-session step, and
/// every redstone-corpus contraption id) resolves register entries against this
/// table.
pub const PLAY_CLIENTBOUND_PACKET_NAMES: &[&str] = &[
    "minecraft:add_entity",
    "minecraft:animate",
    "minecraft:award_stats",
    "minecraft:block_changed_ack",
    "minecraft:block_destruction",
    "minecraft:block_entity_data",
    "minecraft:block_event",
    "minecraft:block_update",
    "minecraft:boss_event",
    "minecraft:bundle_delimiter",
    "minecraft:change_difficulty",
    "minecraft:chunk_batch_finished",
    "minecraft:chunk_batch_start",
    "minecraft:chunks_biomes",
    "minecraft:clear_dialog",
    "minecraft:clear_titles",
    "minecraft:command_suggestions",
    "minecraft:commands",
    "minecraft:container_close",
    "minecraft:container_set_content",
    "minecraft:container_set_data",
    "minecraft:container_set_slot",
    "minecraft:cookie_request",
    "minecraft:cooldown",
    "minecraft:custom_chat_completions",
    "minecraft:custom_payload",
    "minecraft:custom_report_details",
    "minecraft:damage_event",
    "minecraft:debug_sample",
    "minecraft:delete_chat",
    "minecraft:disconnect",
    "minecraft:disguised_chat",
    "minecraft:entity_event",
    "minecraft:entity_position_sync",
    "minecraft:explode",
    "minecraft:forget_level_chunk",
    "minecraft:game_event",
    "minecraft:game_rule_values",
    "minecraft:game_test_highlight_pos",
    "minecraft:hurt_animation",
    "minecraft:initialize_border",
    "minecraft:keep_alive",
    "minecraft:level_chunk_with_light",
    "minecraft:level_event",
    "minecraft:level_particles",
    "minecraft:light_update",
    "minecraft:login",
    "minecraft:low_disk_space_warning",
    "minecraft:map_item_data",
    "minecraft:merchant_offers",
    "minecraft:mount_screen_open",
    "minecraft:move_entity_pos",
    "minecraft:move_entity_pos_rot",
    "minecraft:move_entity_rot",
    "minecraft:move_minecart_along_track",
    "minecraft:move_vehicle",
    "minecraft:open_book",
    "minecraft:open_screen",
    "minecraft:open_sign_editor",
    "minecraft:ping",
    "minecraft:place_ghost_recipe",
    "minecraft:player_abilities",
    "minecraft:player_chat",
    "minecraft:player_combat_end",
    "minecraft:player_combat_enter",
    "minecraft:player_combat_kill",
    "minecraft:player_info_remove",
    "minecraft:player_info_update",
    "minecraft:player_look_at",
    "minecraft:player_position",
    "minecraft:player_rotation",
    "minecraft:pong_response",
    "minecraft:projectile_power",
    "minecraft:recipe_book_add",
    "minecraft:recipe_book_remove",
    "minecraft:recipe_book_settings",
    "minecraft:remove_entities",
    "minecraft:remove_mob_effect",
    "minecraft:reset_score",
    "minecraft:resource_pack_pop",
    "minecraft:resource_pack_push",
    "minecraft:respawn",
    "minecraft:rotate_head",
    "minecraft:section_blocks_update",
    "minecraft:select_advancements_tab",
    "minecraft:server_data",
    "minecraft:server_links",
    "minecraft:set_action_bar_text",
    "minecraft:set_border_center",
    "minecraft:set_border_lerp_size",
    "minecraft:set_border_size",
    "minecraft:set_border_warning_delay",
    "minecraft:set_border_warning_distance",
    "minecraft:set_camera",
    "minecraft:set_chunk_cache_center",
    "minecraft:set_chunk_cache_radius",
    "minecraft:set_cursor_item",
    "minecraft:set_default_spawn_position",
    "minecraft:set_display_objective",
    "minecraft:set_entity_data",
    "minecraft:set_entity_link",
    "minecraft:set_entity_motion",
    "minecraft:set_equipment",
    "minecraft:set_experience",
    "minecraft:set_health",
    "minecraft:set_held_slot",
    "minecraft:set_objective",
    "minecraft:set_passengers",
    "minecraft:set_player_inventory",
    "minecraft:set_player_team",
    "minecraft:set_score",
    "minecraft:set_simulation_distance",
    "minecraft:set_subtitle_text",
    "minecraft:set_time",
    "minecraft:set_title_text",
    "minecraft:set_titles_animation",
    "minecraft:show_dialog",
    "minecraft:sound",
    "minecraft:sound_entity",
    "minecraft:start_configuration",
    "minecraft:stop_sound",
    "minecraft:store_cookie",
    "minecraft:system_chat",
    "minecraft:tab_list",
    "minecraft:tag_query",
    "minecraft:take_item_entity",
    "minecraft:teleport_entity",
    "minecraft:test_instance_block_status",
    "minecraft:ticking_state",
    "minecraft:ticking_step",
    "minecraft:transfer",
    "minecraft:update_advancements",
    "minecraft:update_attributes",
    "minecraft:update_mob_effect",
    "minecraft:update_recipes",
    "minecraft:update_tags",
    "minecraft:waypoint",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The register's own state-aware validation (`known_divergences::
    /// packet_name_known`) only means anything if these tables are themselves
    /// sorted and duplicate-free — a stray unsorted or repeated entry is a
    /// transcription slip from the ASSET-D18(f) reference this test would catch
    /// immediately.
    #[test]
    fn every_table_is_sorted_and_deduplicated() {
        for table in [
            LOGIN_CLIENTBOUND_PACKET_NAMES,
            CONFIGURATION_CLIENTBOUND_PACKET_NAMES,
            PLAY_CLIENTBOUND_PACKET_NAMES,
        ] {
            let mut sorted: Vec<&str> = table.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted, table, "table is not sorted/deduplicated: {table:?}");
        }
    }

    /// Every packet name this harness's own `NORMALIZATION_RULES` table masks
    /// (`protocol_capture.rs`) is a real clientbound packet in at least one of the
    /// three protocol states this harness ever observes — a stale or mistyped
    /// `packet_name` row there would otherwise mask nothing. Most rows are Play-state
    /// (the harness's own overwhelming majority of traffic), but `NORMALIZATION_RULES`
    /// is matched purely by name (`normalize_body`'s own dispatch), never by state, so
    /// a genuinely Login-only or Configuration-only packet (`login_finished`,
    /// M3.5-B03 governance fix) belongs in this table exactly the same way — checked
    /// against the union of all three tables, never `PLAY_CLIENTBOUND_PACKET_NAMES`
    /// alone.
    #[test]
    fn every_normalization_rule_packet_name_is_a_known_clientbound_packet() {
        for rule in crate::protocol_capture::NORMALIZATION_RULES {
            let namespaced = format!("minecraft:{}", rule.packet_name);
            assert!(
                LOGIN_CLIENTBOUND_PACKET_NAMES.contains(&namespaced.as_str())
                    || CONFIGURATION_CLIENTBOUND_PACKET_NAMES.contains(&namespaced.as_str())
                    || PLAY_CLIENTBOUND_PACKET_NAMES.contains(&namespaced.as_str()),
                "NORMALIZATION_RULES packet_name {:?} not found in any of the Login/\
                 Configuration/Play clientbound packet-name tables",
                rule.packet_name
            );
        }
    }
}
