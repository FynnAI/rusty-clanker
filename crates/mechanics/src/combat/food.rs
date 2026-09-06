//! Food, exhaustion, and natural regeneration (M4-B05 Context §3.18, exact — bounded to what
//! this blueprint's own systems produce).

use super::damage::Difficulty;

#[derive(bevy_ecs::prelude::Component, Clone, Debug, PartialEq)]
pub struct FoodStats {
    pub food_level: i32,
    pub saturation: f32,
    pub exhaustion: f32,
    pub regen_tick_timer: u32,
}

impl FoodStats {
    /// `food_level: 20, saturation: 5.0` (vanilla's own join default), `exhaustion: 0.0,
    /// regen_tick_timer: 0`.
    pub fn new_at_join() -> Self {
        Self {
            food_level: 20,
            saturation: 5.0,
            exhaustion: 0.0,
            regen_tick_timer: 0,
        }
    }

    pub fn add_exhaustion(&mut self, amount: f32) {
        self.exhaustion = (self.exhaustion + amount).min(40.0);
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FoodTickOutcome {
    NoChange,
    DecayedOnly,
    Healed { amount: f32 },
    Starved,
}

fn is_hurt(health: f32, max_health: f32) -> bool {
    health < max_health && health > 0.0
}

/// §3.18, exact (Context). `difficulty`/`is_creative` gate as Context describes -- skipped
/// entirely for a creative/`instabuild` player. `health`/`max_health` are read-only inputs;
/// the caller (`rusty-clanker-server::play::combat`) applies `Healed`'s own `amount` to the
/// real health field itself, since this function has no `&mut` access to it (mirrors
/// `apply_damage_pipeline`'s own "plain numeric fields, zero ECS coupling" pattern).
pub fn tick_food(
    stats: &mut FoodStats,
    health: f32,
    max_health: f32,
    difficulty: Difficulty,
    is_creative: bool,
) -> FoodTickOutcome {
    if is_creative {
        return FoodTickOutcome::NoChange;
    }

    let mut decayed = false;
    if stats.exhaustion > 4.0 {
        stats.exhaustion -= 4.0;
        decayed = true;
        if stats.saturation > 0.0 {
            stats.saturation = (stats.saturation - 1.0).max(0.0);
        } else if difficulty != Difficulty::Peaceful {
            stats.food_level = (stats.food_level - 1).max(0);
        }
    }

    let hurt = is_hurt(health, max_health);

    if stats.saturation > 0.0 && hurt && stats.food_level >= 20 {
        stats.regen_tick_timer += 1;
        if stats.regen_tick_timer >= 10 {
            let spend = stats.saturation.min(6.0);
            stats.regen_tick_timer = 0;
            stats.add_exhaustion(spend);
            return FoodTickOutcome::Healed {
                amount: spend / 6.0,
            };
        }
    } else if stats.food_level >= 18 && hurt {
        stats.regen_tick_timer += 1;
        if stats.regen_tick_timer >= 80 {
            stats.regen_tick_timer = 0;
            stats.add_exhaustion(6.0);
            return FoodTickOutcome::Healed { amount: 1.0 };
        }
    } else if stats.food_level <= 0 {
        stats.regen_tick_timer += 1;
        if stats.regen_tick_timer >= 80 {
            stats.regen_tick_timer = 0;
            let should_starve = health > 10.0
                || difficulty == Difficulty::Hard
                || (health > 1.0 && difficulty == Difficulty::Normal);
            if should_starve {
                return FoodTickOutcome::Starved;
            }
        }
    } else {
        stats.regen_tick_timer = 0;
    }

    if decayed {
        FoodTickOutcome::DecayedOnly
    } else {
        FoodTickOutcome::NoChange
    }
}
