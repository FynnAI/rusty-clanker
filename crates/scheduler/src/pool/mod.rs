//! RC-WorkerPool (ARCH-D18-D20, D23) and the region tick clock (ARCH-D7).
//!
//! Neither type knows anything about regions, chunks, or messages:
//! [`RcWorkerPool`] executes arbitrary closures, and one [`TickClock`] tracks
//! one caller-chosen entity's own deadline schedule. Composing many
//! `TickClock`s into a real, admission-controlled, multi-region 20 TPS loop,
//! and dispatching real domain-system waves onto `RcWorkerPool`, are both
//! later blueprints' jobs (M0-B04's own Constraints (d)).

pub mod cgroup;
mod os;
mod tick_clock;
mod worker_pool;

pub use tick_clock::{SERVER_TICK_PERIOD, SystemTickWaiter, TickClock, TickTiming, TickWaiter};
pub use worker_pool::{
    BACKLOG_EWMA_ALPHA, BACKLOG_GROW_MULTIPLIER, GROW_STREAK_THRESHOLD,
    POOL_RESIZE_SAMPLE_INTERVAL, PoolMode, RcWorkerPool, RcWorkerPoolConfig, RealtimeConfig,
    SHRINK_IDLE_STREAK_THRESHOLD, WORKER_IDLE_POLL_INTERVAL, compute_baseline, compute_hard_cap,
};
