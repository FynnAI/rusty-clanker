//! M4-B09 Acceptance tests (pure, no scenario execution): `ai_scenario`'s own RON-fixture
//! loading + manifest verification, and `ScenarioWorld::spawn_mob`'s own per-kind
//! component-set defaulting (Context Part D).

use rc_gametest::ai_scenario::{ScenarioWorld, load_ai_scenario};
use rc_mechanics::entity::EntityKind;
use rc_registries::generated_v776::registries::attribute;

fn corpus_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/ai_combat")
}

#[test]
fn load_ai_scenario_parses_the_two_shipped_ron_files() {
    for name in ["wall_with_gap", "four_block_pit"] {
        let path = corpus_dir().join(format!("{name}.ron"));
        let spec = load_ai_scenario(&path).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert_eq!(spec.id, name);
        assert!(
            !spec.blocks.is_empty(),
            "{name}: expected a nonempty blocks list"
        );
    }

    let manifest_path = corpus_dir().join("manifest.json");
    let violations = xtask::fixture_manifest::verify_manifest(&manifest_path, &corpus_dir());
    assert!(
        violations.is_empty(),
        "manifest verification failed: {}",
        violations
            .iter()
            .map(|v| v.message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    );
}

#[test]
fn scenario_world_spawns_a_zombie_with_a_full_component_set() {
    let mut world = ScenarioWorld::new();
    let id = world.spawn_mob(EntityKind::Zombie, [0.0, 64.0, 0.0]);
    let mob = world
        .mobs
        .get_mut(&id)
        .expect("zombie present in world.mobs");
    assert!(
        !mob.target_selector.is_empty(),
        "expected the zombie's own target_selector to carry at least HurtByTargetGoal"
    );
    assert_eq!(mob.attributes.value_or(attribute::ATTACK_DAMAGE, -1.0), 3.0);
}

#[test]
fn scenario_world_spawns_a_cow_with_an_empty_target_selector() {
    let mut world = ScenarioWorld::new();
    let id = world.spawn_mob(EntityKind::Cow, [0.0, 64.0, 0.0]);
    let mob = world.mobs.get(&id).expect("cow present in world.mobs");
    assert!(mob.target_selector.is_empty());
}
