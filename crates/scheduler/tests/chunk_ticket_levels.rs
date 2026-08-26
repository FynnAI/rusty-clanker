//! M2-B05 acceptance tests: WORLD-D24's ticket/level system (`rc_scheduler::chunk_ticket`).
//! Pure, no I/O, no `bevy_ecs`.

use std::collections::HashSet;

use rc_core::{ChunkKey, DimensionId};
use rc_scheduler::chunk_ticket::{ChunkLoadState, PlayerTicketId, TicketManager};

fn key(x: i32, z: i32) -> ChunkKey {
    ChunkKey {
        dimension: DimensionId::OVERWORLD,
        x,
        z,
    }
}

#[test]
fn single_player_ticket_produces_a_uniform_disc_then_a_ring() {
    let mut mgr = TicketManager::new();
    mgr.register_player(PlayerTicketId(1), key(0, 0), 2);
    mgr.step();

    assert_eq!(mgr.level(key(0, 0)), Some(31));
    assert_eq!(mgr.level(key(2, 2)), Some(31));
    assert_eq!(mgr.level(key(3, 0)), Some(32));
    assert_eq!(mgr.level(key(15, 0)), Some(44));
    assert_eq!(mgr.level(key(16, 0)), None);
}

#[test]
fn two_overlapping_tickets_take_the_minimum_level() {
    let mut mgr = TicketManager::new();
    mgr.register_player(PlayerTicketId(1), key(0, 0), 0);
    mgr.register_player(PlayerTicketId(2), key(1, 0), 0);
    mgr.step();

    assert_eq!(mgr.level(key(0, 0)), Some(31));
    assert_eq!(mgr.level(key(1, 0)), Some(31));
}

#[test]
fn dimension_isolation() {
    let mut mgr = TicketManager::new();
    mgr.register_player(PlayerTicketId(1), key(0, 0), 4);
    mgr.step();

    let nether_key = ChunkKey {
        dimension: DimensionId::THE_NETHER,
        x: 0,
        z: 0,
    };
    assert_eq!(mgr.level(nether_key), None);
}

#[test]
fn first_step_after_registration_reports_needs_load_for_the_whole_reachable_set() {
    let mut mgr = TicketManager::new();
    mgr.register_player(PlayerTicketId(1), key(0, 0), 1);
    let churn = mgr.step();

    // `level <= BORDER_LEVEL (33)` holds for every Chebyshev distance `<= radius + 2`
    // (Context's `contribution` formula: level = 31 + (d - radius) for d > radius) --
    // radius 1 therefore reaches every chunk within distance 3, a 7x7 = 49 chunk disc,
    // not merely the 3x3 = 9 chunk disc a literal point-source flood from an occupied
    // chunk alone would suggest (Context explicitly rejects that reading -- "an entire
    // simulation-distance disc of chunks all tick uniformly, not a single point-source
    // flood-fill").
    let expected: HashSet<ChunkKey> = (-3..=3)
        .flat_map(|x| (-3..=3).map(move |z| key(x, z)))
        .collect();
    let actual: HashSet<ChunkKey> = churn.needs_load.into_iter().collect();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 49);
}

#[test]
fn unregister_requires_two_consecutive_over_threshold_steps_before_unload() {
    let mut mgr = TicketManager::new();
    mgr.register_player(PlayerTicketId(1), key(0, 0), 0);
    mgr.step();

    mgr.unregister_player(PlayerTicketId(1));

    let churn2 = mgr.step();
    assert!(
        churn2.needs_unload.is_empty(),
        "only one consecutive over-threshold step so far"
    );

    let churn3 = mgr.step();
    assert!(churn3.needs_unload.contains(&key(0, 0)));
}

#[test]
fn memory_pressure_skips_the_second_consecutive_check() {
    let mut mgr = TicketManager::new();
    mgr.register_player(PlayerTicketId(1), key(0, 0), 0);
    let far = key(10_000, 10_000);
    mgr.register_player(PlayerTicketId(2), far, 0);
    mgr.step();

    mgr.unregister_player(PlayerTicketId(1));
    mgr.set_memory_pressure(true);

    let churn2 = mgr.step();
    assert!(churn2.needs_unload.contains(&key(0, 0)));
    assert!(!churn2.needs_unload.contains(&far));
}

#[test]
fn move_player_recenters_and_produces_load_then_unload_churn() {
    let mut mgr = TicketManager::new();
    mgr.register_player(PlayerTicketId(1), key(0, 0), 0);
    mgr.step();

    mgr.move_player(PlayerTicketId(1), key(5, 0));

    let churn2 = mgr.step();
    assert!(churn2.needs_load.contains(&key(5, 0)));
    assert!(!churn2.needs_unload.contains(&key(0, 0)));

    let churn3 = mgr.step();
    assert!(churn3.needs_unload.contains(&key(0, 0)));
}

#[test]
fn load_state_matches_the_worldd24_table() {
    assert_eq!(
        ChunkLoadState::from_level(Some(31)),
        ChunkLoadState::EntityTicking
    );
    assert_eq!(
        ChunkLoadState::from_level(Some(32)),
        ChunkLoadState::Ticking
    );
    assert_eq!(ChunkLoadState::from_level(Some(33)), ChunkLoadState::Border);
    assert_eq!(
        ChunkLoadState::from_level(None),
        ChunkLoadState::Inaccessible
    );
}
