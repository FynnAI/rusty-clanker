//! M4-B04 acceptance tests: the mob-cap census (`LocalCapCounts`, `RegionCensusState`,
//! `GlobalMobCensus`, `global_cap`) — blueprint Context, "Mob-cap formula" / "MECH-D35
//! cluster-safe census".

use rc_mechanics::spawn::{
    GlobalMobCensus, LocalCapCounts, MobCategory, RegionCensusState, global_cap,
};
use rc_messaging::RegionId;

const NEAR: [f64; 3] = [0.0, 0.0, 0.0];
const CHUNK_CENTER: [f64; 3] = [8.0, 0.0, 8.0];

#[test]
fn local_cap_allows_when_any_nearby_player_has_room() {
    let mut counts = LocalCapCounts::new();
    // Player 1 (id 1) is bumped to cap for `Creature` (10 instances); player 2 (id 2)
    // stays under.
    for _ in 0..10 {
        counts.bump_near(MobCategory::Creature, NEAR, &[(1, NEAR)]);
    }
    assert!(counts.allows(MobCategory::Creature, CHUNK_CENTER, &[(1, NEAR), (2, NEAR)]));
}

#[test]
fn local_cap_denies_when_every_nearby_player_is_at_cap() {
    let mut counts = LocalCapCounts::new();
    for _ in 0..10 {
        counts.bump_near(MobCategory::Creature, NEAR, &[(1, NEAR), (2, NEAR)]);
    }
    assert!(!counts.allows(MobCategory::Creature, CHUNK_CENTER, &[(1, NEAR), (2, NEAR)]));
}

#[test]
fn global_cap_scales_with_eligible_chunk_count() {
    assert_eq!(global_cap(MobCategory::Monster, 289), 70);
    assert_eq!(global_cap(MobCategory::Monster, 578), 140);
    assert_eq!(global_cap(MobCategory::Monster, 0), 0);
    // Floor-division case: 70 * 288 / 289 = 20160 / 289 = 69.75.. -> 69.
    assert_eq!(global_cap(MobCategory::Monster, 288), 69);
}

#[test]
fn global_mob_census_aggregates_own_plus_every_peer() {
    let region_a = RegionId(1);
    let region_b = RegionId(2);
    let region_c = RegionId(3);

    let mut census = GlobalMobCensus::new(region_a);
    let mut own = rc_mechanics::spawn::MobCategoryCounts::default();
    own.bump(MobCategory::Monster);
    own.bump(MobCategory::Monster);
    own.bump(MobCategory::Monster);
    census.set_own_counts(own);

    let mut peer_b = rc_mechanics::spawn::MobCategoryCounts::default();
    peer_b.bump(MobCategory::Monster);
    peer_b.bump(MobCategory::Monster);
    census.record_peer_report(region_b, peer_b);

    let mut peer_c = rc_mechanics::spawn::MobCategoryCounts::default();
    for _ in 0..5 {
        peer_c.bump(MobCategory::Monster);
    }
    census.record_peer_report(region_c, peer_c);

    assert_eq!(census.aggregate(MobCategory::Monster), 10);
}

#[test]
fn global_mob_census_peer_report_overwrites_not_accumulates() {
    let region_b = RegionId(2);
    let mut census = GlobalMobCensus::new(RegionId(1));

    let mut first = rc_mechanics::spawn::MobCategoryCounts::default();
    first.bump(MobCategory::Monster);
    first.bump(MobCategory::Monster);
    census.record_peer_report(region_b, first);

    let mut second = rc_mechanics::spawn::MobCategoryCounts::default();
    second.bump(MobCategory::Monster);
    census.record_peer_report(region_b, second);

    assert_eq!(census.aggregate(MobCategory::Monster), 1);
}

#[test]
fn global_mob_census_unreporting_peer_has_zero_contribution() {
    let census = GlobalMobCensus::new(RegionId(1));
    // `region_d` has never sent a report -- `aggregate` is unaffected by its existence.
    assert_eq!(census.aggregate(MobCategory::Monster), 0);
    assert_eq!(census.known_peer_count(), 0);
}

#[test]
fn region_census_state_persistence_locked_mobs_excluded() {
    let players: Vec<(i32, [f64; 3])> = vec![(1, NEAR)];
    let live_mobs: Vec<(MobCategory, [f64; 3])> = vec![(MobCategory::Creature, NEAR)];
    // The caller filters `persistence_required` mobs BEFORE calling `build` (blueprint
    // Context, "Persistence exemption") -- this test pins that `build` itself never adds
    // anything the caller didn't already include.
    let census = RegionCensusState::build(live_mobs, &players);
    assert_eq!(census.global.get(MobCategory::Creature), 1);

    let empty_census = RegionCensusState::build(Vec::<(MobCategory, [f64; 3])>::new(), &players);
    assert_eq!(empty_census.global.get(MobCategory::Creature), 0);
}
