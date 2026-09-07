//! M4-B09 Acceptance test (Context Part I): proves the three real
//! `DomainGroup::EntityPhysicsIntegration` registrants — M4-B02's `register_stage6b`,
//! M4-B04's `register_mob_despawn`, and M4-B05's `register_mob_combat_system` — co-register
//! against one `RcExecutorBuilder` without triggering `ExecutorBuildError::
//! AmbiguousMutationAuthority`, and receive the documented `order_tag` values `0`, `1`, `2`
//! respectively, in the exact order `HardcodedWorld`'s own composition root
//! (`crates/server/src/play/world.rs`) already calls them in.
//!
//! **Forced deviation from this blueprint's own literal Deliverables path**
//! (`crates/mechanics/tests/entity_physics_integration_group_registration.rs`): the third
//! registrant, `register_mob_combat_system`, is not an `rc-mechanics` item at all — it is
//! defined in `rusty_clanker_server::play::combat` (`crates/server/src/play/combat.rs`),
//! reusing `rc-mechanics`' pure combat formulas but registering its own ECS system directly
//! in the server crate. A test exercising all three registrants together therefore cannot
//! live under `crates/mechanics/tests/` at all (that crate has no dependency edge onto
//! `rusty-clanker-server`, and WS-D3's own crate-graph rules would forbid one) — this file
//! lives under `crates/server/tests/` instead, the one crate that can actually reach all
//! three registration functions. Recorded as a deviation in this blueprint's own final
//! report; still "pure — real `RcExecutorBuilder`, no `HardcodedWorld`/network" exactly as
//! the Deliverables' own text otherwise specifies.

use bevy_ecs::world::World;
use rc_scheduler::{DomainGroup, ExecutorBuildError, RcExecutorBuilder};

fn empty_bootstrap(_world: &mut World) {}

#[test]
fn three_real_registrants_co_register_without_ambiguous_mutation_authority() {
    let mut builder = RcExecutorBuilder::new(empty_bootstrap);
    rc_mechanics::entity::physics::register_stage6b(&mut builder);
    rc_mechanics::spawn::register_mob_despawn(&mut builder);
    rusty_clanker_server::play::combat::register_mob_combat_system(&mut builder);

    let result = builder.build();
    match &result {
        Err(err @ ExecutorBuildError::AmbiguousMutationAuthority { .. }) => {
            panic!("expected no AmbiguousMutationAuthority error, got {err}")
        }
        Err(err) => panic!("expected Ok(_), got Err({err})"),
        Ok(_) => {}
    }
}

#[test]
fn three_real_registrants_receive_the_documented_order_tags() {
    let mut builder = RcExecutorBuilder::new(empty_bootstrap);
    let stage6b_id = rc_mechanics::entity::physics::register_stage6b(&mut builder);
    let despawn_id = rc_mechanics::spawn::register_mob_despawn(&mut builder);
    let combat_id = rusty_clanker_server::play::combat::register_mob_combat_system(&mut builder);

    assert_eq!(stage6b_id.group, DomainGroup::EntityPhysicsIntegration);
    assert_eq!(stage6b_id.order_tag, 0);
    assert_eq!(despawn_id.group, DomainGroup::EntityPhysicsIntegration);
    assert_eq!(despawn_id.order_tag, 1);
    assert_eq!(combat_id.group, DomainGroup::EntityPhysicsIntegration);
    assert_eq!(combat_id.order_tag, 2);

    match builder.build() {
        Ok(_) => {}
        Err(err) => panic!("expected Ok(_), got Err({err})"),
    }
}
