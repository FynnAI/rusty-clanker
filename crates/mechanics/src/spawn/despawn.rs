//! Despawn rules — blueprint Context, "Despawn rules", restated exactly from
//! `docs/research/mc-26.2/23-spawning-math.md` §3.13 and cross-checked against
//! `M4-B04-CLAIMS.md` rows 45-48/61.

use crate::entity::EntityKind;
use crate::random::RcRandom;
use crate::spawn::category::MobCategory;

/// Per-mob despawn state (blueprint Context: "Despawn rules").
#[derive(bevy_ecs::prelude::Component, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DespawnTimer {
    pub no_action_ticks: u32,
}

/// Vanilla's own per-species `Mob.removeWhenFarAway` predicate (blueprint Context,
/// "Despawn rules" predicate table), resolved per `EntityKind` rather than per
/// `MobCategory` — vanilla's own override lives on the class hierarchy (`Animal`
/// overrides it for every animal species, M4-B04-CLAIMS.md row 45's own correction),
/// flattened here to a per-kind table since this blueprint ships a fixed, closed kind
/// set. Never called for `Item`/`Villager` — neither is naturally spawned by this
/// blueprint, so neither ever carries a `MobCategoryTag`/`DespawnTimer`.
pub fn remove_when_far_away_for_kind(kind: EntityKind) -> bool {
    let _ = kind;
    todo!()
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DespawnDecision {
    Keep,
    Despawn,
}

/// Blueprint Context's own restated algorithm. `persistence_required`/
/// `nearest_player_dist_sqr` are read from the caller's own already-fetched entity/world
/// state; `remove_when_far_away` is the mob's own per-species distance-despawn-
/// eligibility term (`true` for `Monster`/Zombie, `false` for `Creature`/Cow at this
/// blueprint's own two shipped kinds); `timer` is mutated in place (incremented by the
/// caller once per Stage-6b tick before this call).
pub fn check_despawn(
    persistence_required: bool,
    nearest_player_dist_sqr: Option<f64>,
    category: MobCategory,
    remove_when_far_away: bool,
    rng: &mut RcRandom,
    timer: &mut DespawnTimer,
) -> DespawnDecision {
    let _ = (
        persistence_required,
        nearest_player_dist_sqr,
        category,
        remove_when_far_away,
        rng,
        timer,
    );
    todo!()
}
