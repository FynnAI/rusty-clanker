//! `load_scenario::{block_grid_cell, plan_bot_layout}` self-tests (Acceptance
//! tests) — pure geometry, no network. Never exercises `run_one_load_bot`/
//! `run_load_scenario` (those are azalea-driven and only ever run against a real
//! `rusty-clanker-server`, module doc comment).

use rc_paritybot::load_scenario::{
    ARENA_MAX, ARENA_MIN, BASE_Y, COLS, ROWS, block_grid_cell, plan_bot_layout,
};

fn plans() -> Vec<rc_paritybot::load_scenario::BotPlan> {
    plan_bot_layout(COLS, ROWS, ARENA_MIN, ARENA_MAX, BASE_Y)
}

#[test]
fn plan_bot_layout_produces_cols_times_rows_entries() {
    assert_eq!(plans().len(), 20);
}

#[test]
fn every_username_is_unique_and_zero_padded() {
    let plans = plans();
    let usernames: Vec<&str> = plans.iter().map(|p| p.username.as_str()).collect();

    let expected: Vec<String> = (0..20).map(|i| format!("rc-load-bot-{i:02}")).collect();
    let expected_refs: Vec<&str> = expected.iter().map(String::as_str).collect();
    assert_eq!(usernames, expected_refs);

    let mut sorted = usernames.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        usernames.len(),
        "usernames must all be distinct"
    );
}

#[test]
fn every_waypoint_and_interaction_post_stays_in_one_grid_cell() {
    let spawn_cell = block_grid_cell(0, 0);
    for plan in plans() {
        for wp in &plan.waypoints {
            assert_eq!(
                block_grid_cell(wp.x, wp.z),
                spawn_cell,
                "waypoint {wp:?} of {} left the spawn's own grid cell",
                plan.username
            );
        }
        assert_eq!(
            block_grid_cell(plan.interaction_post.x, plan.interaction_post.z),
            spawn_cell,
            "interaction post {:?} of {} left the spawn's own grid cell",
            plan.interaction_post,
            plan.username
        );
    }
}

#[test]
fn interaction_post_sits_outside_its_own_patrol_square() {
    for plan in plans() {
        let min_waypoint_z = plan.waypoints.iter().map(|wp| wp.z).min().unwrap();
        assert!(
            plan.interaction_post.z < min_waypoint_z,
            "{}'s interaction post z={} should be strictly south of every waypoint's own minimum z={min_waypoint_z}",
            plan.username,
            plan.interaction_post.z
        );
    }
}

#[test]
fn start_offset_ticks_are_distinct_and_ascending_by_index() {
    let plans = plans();
    for (index, plan) in plans.iter().enumerate() {
        assert_eq!(plan.start_offset_ticks, index as u32 * 2);
    }
}

#[test]
fn arena_bounds_stay_at_least_30_blocks_inside_the_cell_edge() {
    for plan in plans() {
        for wp in &plan.waypoints {
            assert!(
                wp.x >= 30 && wp.x <= 225,
                "waypoint x {} out of bounds",
                wp.x
            );
            assert!(
                wp.z >= 30 && wp.z <= 225,
                "waypoint z {} out of bounds",
                wp.z
            );
        }
        assert!(
            plan.interaction_post.x >= 30 && plan.interaction_post.x <= 225,
            "interaction post x {} out of bounds",
            plan.interaction_post.x
        );
        assert!(
            plan.interaction_post.z >= 30 && plan.interaction_post.z <= 225,
            "interaction post z {} out of bounds",
            plan.interaction_post.z
        );
    }
}
