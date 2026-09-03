//! `rc-scheduler`-facing adapter for Stage 8 (feature `server-systems`), mirroring
//! `stage4/ecs.rs`'s exact role.

use rc_scheduler::pool::RcWorkerPool;

use crate::light::stage8::ParallelDispatch;

/// Local trait, foreign type -- legal under Rust's orphan rules.
impl ParallelDispatch for RcWorkerPool {
    fn run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>) {
        self.run_batch(tasks)
    }
}

/// The `LightingStageDriver` `rc-scheduler::RcExecutorBuilder::with_lighting_driver`
/// expects (Context §8).
pub fn lighting_stage_driver(world: &mut bevy_ecs::world::World, pool: &RcWorkerPool) {
    let _report = crate::light::stage8::run_stage8_lighting(world, pool);
}
