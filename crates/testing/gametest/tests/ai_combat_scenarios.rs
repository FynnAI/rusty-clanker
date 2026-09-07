//! M4-B09 Acceptance tests (Context Part G, scenarios 1-9): the AI/combat
//! behavioral-envelope suite, run against `ScenarioWorld` (Part D). Tolerance/envelope
//! framing per Part G's own text: this engine's own internal logic is fully
//! deterministic — most assertions below are *exact* against our own engine, never
//! "banded" against it; the qualitative/behavioral framing applies only to the
//! (unmeasured, no oracle exists) comparison against vanilla's own real behavior.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rc_core::BlockPos;
use rc_gametest::ai_scenario::{HURT_BY_MEMORY_TTL_TICKS, ScenarioWorld, load_ai_scenario};
use rc_mechanics::ai::brain::Activity;
use rc_mechanics::ai::goal::AiContext;
use rc_mechanics::ai::mob_config::ZombieAttackGoal;
use rc_mechanics::ai::{FLAG_MOVE, Goal, GoalSelector};
use rc_mechanics::entity::EntityKind;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/ai_combat")
}

fn load_blocks_into(world: &mut ScenarioWorld, ron_id: &str) {
    let spec = load_ai_scenario(&corpus_dir().join(format!("{ron_id}.ron")))
        .unwrap_or_else(|err| panic!("{ron_id}: {err}"));
    for block in spec.blocks {
        world.set_block(
            BlockPos::new(block.pos[0], block.pos[1], block.pos[2]),
            rc_chunk_storage::BlockStateId(block.state_id),
        );
    }
}

#[test]
fn zombie_routes_around_wall_gap() {
    let mut world = ScenarioWorld::new();
    load_blocks_into(&mut world, "wall_with_gap");

    let start = BlockPos::new(0, 64, 0);
    let target = BlockPos::new(0, 64, 12);
    let outcome = rc_mechanics::ai::find_path(
        start,
        &[target],
        0.0,
        &rc_mechanics::ai::WalkNodeEvaluator,
        &world,
        1.95,
        &HashMap::new(),
        4096,
    );
    let path = outcome.path.expect("expected a path to be found");
    let nodes = path.nodes();

    assert!(
        nodes.iter().any(|n| n.x == 0 && n.z == 6),
        "expected the path to pass through the gap's own (x,z) column at some node, got {nodes:?}"
    );
    for wall_x in [-2, -1, 1, 2] {
        for wall_y in [64, 65, 66] {
            assert!(
                !nodes
                    .iter()
                    .any(|n| n.x == wall_x && n.y == wall_y && n.z == 6),
                "expected the path to never pass through a wall block, but it did at \
                 ({wall_x},{wall_y},6): {nodes:?}"
            );
        }
    }
}

#[test]
fn zombie_refuses_a_four_block_drop() {
    let mut world = ScenarioWorld::new();
    load_blocks_into(&mut world, "four_block_pit");

    let start = BlockPos::new(0, 64, 0);
    let target = BlockPos::new(0, 64, 12);
    let outcome = rc_mechanics::ai::find_path(
        start,
        &[target],
        0.0,
        &rc_mechanics::ai::WalkNodeEvaluator,
        &world,
        1.95,
        &HashMap::new(),
        4096,
    );
    let path = outcome.path.expect("expected a path to be found");
    let nodes = path.nodes();
    assert!(
        nodes.len() >= 2,
        "expected a nontrivial path, got {nodes:?}"
    );
    for pair in nodes.windows(2) {
        let dy = (pair[1].y - pair[0].y).abs();
        assert!(
            dy <= 3,
            "expected no two consecutive nodes to differ by more than 3 in y, got \
             {:?} -> {:?} (dy={dy})",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn zombie_ignores_target_outside_follow_range() {
    let mut world = ScenarioWorld::new();
    let zombie = world.spawn_mob(EntityKind::Zombie, [0.0, 64.0, 0.0]);
    // Zombie's own FOLLOW_RANGE is 35.0 (M4-B09-CLAIMS.md CONFIRMED) -- 40.0 blocks away
    // is always outside it.
    world.spawn_player_proxy([0.0, 64.0, 40.0]);

    for _ in 0..10 {
        world.tick();
        assert_eq!(
            world.mobs[&zombie].current_target, None,
            "tick {}: expected no target beyond FOLLOW_RANGE",
            world.tick_count
        );
    }
}

#[test]
fn zombie_loses_target_behind_opaque_wall() {
    let mut world = ScenarioWorld::new();
    let zombie = world.spawn_mob(EntityKind::Zombie, [0.0, 64.0, 0.0]);
    world.spawn_player_proxy([0.0, 64.0, 10.0]);
    // A full solid wall between the zombie and the player-proxy, spanning well above and
    // below both entities' own eye lines.
    for y in 62..=68 {
        for x in -1..=1 {
            world.set_block(BlockPos::new(x, y, 5), rc_chunk_storage::BlockStateId(1));
        }
    }

    for _ in 0..10 {
        world.tick();
        assert_eq!(
            world.mobs[&zombie].current_target, None,
            "tick {}: expected no target behind an opaque wall",
            world.tick_count
        );
    }
}

#[test]
fn zombie_engages_melee_within_range() {
    let mut world = ScenarioWorld::new();
    let zombie = world.spawn_mob(EntityKind::Zombie, [0.0, 64.0, 0.0]);
    let player = world.spawn_player_proxy([0.5, 64.0, 0.0]);

    let mut engaged = false;
    for _ in 0..5 {
        world.tick();
        if world.mobs[&zombie].pending_melee_attack.0 == Some(player) {
            engaged = true;
            break;
        }
    }
    assert!(
        engaged,
        "expected pending_melee_attack.0 == Some(player) within 5 ticks"
    );
}

#[test]
fn cow_never_acquires_a_target() {
    let mut world = ScenarioWorld::new();
    let cow = world.spawn_mob(EntityKind::Cow, [0.0, 64.0, 0.0]);
    world.spawn_player_proxy([0.5, 64.0, 0.0]);

    for _ in 0..200 {
        world.tick();
        assert_eq!(
            world.mobs[&cow].current_target, None,
            "tick {}: a Cow's own target_selector is empty by construction",
            world.tick_count
        );
    }
}

#[test]
fn zombie_aggros_the_entity_that_hurt_it() {
    let mut world = ScenarioWorld::new();
    let zombie = world.spawn_mob(EntityKind::Zombie, [0.0, 64.0, 0.0]);
    // Far outside FOLLOW_RANGE (35.0) -- normally never targeted (scenario 3's own
    // envelope).
    let player = world.spawn_player_proxy([0.0, 64.0, 100.0]);
    world.mobs.get_mut(&zombie).unwrap().recent_damage.0 = Some(player);

    let mut aggroed = false;
    for _ in 0..2 {
        world.tick();
        if world.mobs[&zombie].current_target == Some(player) {
            aggroed = true;
            break;
        }
    }
    assert!(
        aggroed,
        "expected current_target == Some(player) within 2 ticks after the damage pulse"
    );
}

fn angle_diff_degrees(a: f32, b: f32) -> f32 {
    let mut delta = (a - b) % 360.0;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    delta.abs()
}

#[test]
fn villager_flees_then_deaggroes() {
    let mut world = ScenarioWorld::new();
    let villager = world.spawn_mob(EntityKind::Villager, [0.0, 64.0, 0.0]);
    let attacker = world.spawn_player_proxy([0.5, 64.0, 0.0]);
    world.mobs.get_mut(&villager).unwrap().recent_damage.0 = Some(attacker);

    world.tick();
    world.tick();
    {
        let mob = &world.mobs[&villager];
        let (brain, _) = mob.brain.as_ref().expect("villager carries a brain");
        assert_eq!(
            brain.active_activities,
            [Activity::Core, Activity::Panic].into_iter().collect(),
            "expected {{Core, Panic}} within 2 ticks of the damage pulse"
        );
        assert!(
            mob.movement_intent.0.forward > 0.0,
            "expected forward movement while fleeing"
        );
        let attacker_pos = world.players[&attacker].pos;
        let self_pos = mob.base.pos;
        let dx = self_pos[0] - attacker_pos[0];
        let dz = self_pos[2] - attacker_pos[2];
        let expected_yaw = (dz.atan2(dx)).to_degrees() as f32 - 90.0;
        let diff = angle_diff_degrees(mob.movement_intent.0.yaw_degrees, expected_yaw);
        assert!(
            diff <= 10.0,
            "expected yaw within 10 degrees of {expected_yaw}, got {} (diff {diff})",
            mob.movement_intent.0.yaw_degrees
        );
    }

    for _ in 0..(HURT_BY_MEMORY_TTL_TICKS as u64 + 5 - 2) {
        world.tick();
    }
    let mob = &world.mobs[&villager];
    let (brain, _) = mob.brain.as_ref().expect("villager carries a brain");
    assert_eq!(
        brain.active_activities,
        [Activity::Core, Activity::Idle].into_iter().collect(),
        "expected {{Core, Idle}} at tick HURT_BY_MEMORY_TTL_TICKS + 5"
    );
}

/// A test-local, call-counting wrapper `Goal` standing in for the real Zombie stroll
/// goal (Context Part G, scenario 9's own "a call-counting fixture wrapper around the
/// stroll goal, this test's own instrumentation"). Always eligible (`can_use: true`),
/// `FLAG_MOVE` only, exactly like `WaterAvoidingRandomStrollGoal`'s own flag set --
/// `mob_config`'s real stroll goal is private to that module and cannot be wrapped
/// directly from this crate, so this is a behaviorally-equivalent (for eviction-timing
/// purposes) stand-in.
struct CountingStrollGoal {
    stops: Arc<Mutex<u32>>,
}
impl Goal for CountingStrollGoal {
    fn flags(&self) -> u8 {
        FLAG_MOVE
    }
    fn can_use(&self, _ctx: &AiContext) -> bool {
        true
    }
    fn stop(&mut self, _ctx: &mut AiContext) {
        *self.stops.lock().unwrap() += 1;
    }
}

#[test]
fn goal_selector_evicts_lower_priority_under_real_ticking() {
    let mut world = ScenarioWorld::new();
    let zombie = world.spawn_mob(EntityKind::Zombie, [0.0, 64.0, 0.0]);

    let stops = Arc::new(Mutex::new(0u32));
    let mut selector = GoalSelector::new();
    selector.add_goal(3, Box::new(ZombieAttackGoal)); // index 0
    selector.add_goal(
        7,
        Box::new(CountingStrollGoal {
            stops: Arc::clone(&stops),
        }),
    ); // index 1
    world.mobs.get_mut(&zombie).unwrap().goal_selector = selector;

    for _ in 0..50 {
        world.tick();
    }
    assert_eq!(
        world.mobs[&zombie]
            .goal_selector
            .running_goal_holding(FLAG_MOVE),
        Some(1),
        "expected the stroll goal (index 1) to hold FLAG_MOVE before tick 50"
    );
    assert_eq!(*stops.lock().unwrap(), 0);

    world.spawn_player_proxy([0.5, 64.0, 0.0]);

    let mut evicted = false;
    for _ in 0..4 {
        world.tick();
        if world.mobs[&zombie]
            .goal_selector
            .running_goal_holding(FLAG_MOVE)
            == Some(0)
        {
            evicted = true;
            break;
        }
    }
    assert!(
        evicted,
        "expected ZombieAttackGoal (index 0) to hold FLAG_MOVE within a few ticks of tick 50"
    );
    assert_eq!(
        *stops.lock().unwrap(),
        1,
        "expected the stroll goal's own stop() to have fired exactly once"
    );
}
