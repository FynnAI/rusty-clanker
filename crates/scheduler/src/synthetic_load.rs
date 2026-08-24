//! This blueprint's own synthetic-load generator: a real, registered `bevy_ecs` system
//! doing tunable busy-work — no game mechanics (Context: "Synthetic-load generator").

use bevy_ecs::prelude::*;

use crate::SystemFactory; // M0-B05, re-exported at this crate's root

/// Per-region tunable synthetic busy-work cost (a `bevy_ecs::Resource`). Not a real
/// mechanic (ARCH-D8) — this blueprint's own synthetic-load knob only.
#[derive(Resource, Copy, Clone, Debug)]
pub struct SyntheticLoadProfile {
    /// Approximate CPU time `synthetic_busy_work_system` spends per tick.
    pub busy_work_micros: u64,
}

/// `RcExecutorBuilder::new`'s `bootstrap` argument: inserts a zero-cost default profile
/// so every freshly-`spawn_region`'d `World` has one before this blueprint's own harness
/// overrides it per-region (Context: "Synthetic-load generator").
pub fn bootstrap_default_profile(world: &mut bevy_ecs::world::World) {
    world.insert_resource(SyntheticLoadProfile {
        busy_work_micros: 0,
    });
}

/// The one system this blueprint registers (into `DomainGroup::AiPhysics`, M0-B05's
/// Stage 6 mapping): reads `Res<SyntheticLoadProfile>` (its only declared access — no
/// `Query`, no `Commands`) and busy-spins for approximately `busy_work_micros`.
pub fn synthetic_busy_work_system(profile: Res<SyntheticLoadProfile>) {
    busy_spin(profile.busy_work_micros);
}

/// `RcExecutorBuilder::register_system`'s `factory` argument for
/// `synthetic_busy_work_system`.
pub fn synthetic_system_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(bevy_ecs::system::IntoSystem::into_system(
            synthetic_busy_work_system,
        )) as Box<dyn bevy_ecs::system::System<In = (), Out = ()>>
    })
}

/// Spins (never sleeps) for approximately `micros`, polling `Instant::now()` every 256
/// iterations of a `std::hint::black_box`-guarded wrapping-multiply accumulator (the
/// computed value is discarded; `black_box` only prevents the optimizer from eliding the
/// loop). CPU-bound by design (Context).
pub fn busy_spin(micros: u64) {
    let start = std::time::Instant::now();
    let target = std::time::Duration::from_micros(micros);
    let mut acc: u64 = std::hint::black_box(1);
    loop {
        for _ in 0..256 {
            acc = std::hint::black_box(acc.wrapping_mul(2654435761));
        }
        if start.elapsed() >= target {
            break;
        }
    }
    std::hint::black_box(acc);
}
