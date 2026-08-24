//! `rc-scheduler` — RC-Executor, RC-WorkerPool, the 11-stage tick pipeline driver,
//! region lifecycle, the ARCH-D8 startup conflict graph, the Tokio<->RC-WorkerPool
//! boundary types (ARCH-D1-D9, D12, D18-D23). Depends on `dyn Transport` only, never
//! a concrete transport (`rc-messaging`'s `Transport` trait).

pub mod pool; // M0-B04 — not modified by this blueprint

mod access;
mod conflict_graph;
mod executor;
mod pipeline;
mod region;
mod registry;

pub use access::ComponentAccessSummary;
pub use conflict_graph::compute_waves;
pub use executor::{RcExecutor, TickReport};
pub use pipeline::{DomainGroup, Stage};
pub use region::RegionState;
pub use registry::{ExecutorBuildError, RcExecutorBuilder, SystemFactory, SystemId};
