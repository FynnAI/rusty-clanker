//! `rc-scheduler` — RC-Executor, RC-WorkerPool, the 11-stage tick pipeline driver,
//! region lifecycle, the ARCH-D8 startup conflict graph, the Tokio<->RC-WorkerPool
//! boundary types (ARCH-D1-D9, D12, D18-D23). Depends on `dyn Transport` only, never
//! a concrete transport (`rc-messaging`'s `Transport` trait).

pub mod pool; // M0-B04 — not modified by this blueprint

mod access;
mod conflict_graph;
mod directory;
mod executor;
mod grid;
mod lifecycle;
mod managed_region;
mod measurement;
mod pipeline;
mod region;
mod region_manager;
mod registry;
mod synthetic_load;

pub use access::ComponentAccessSummary;
pub use conflict_graph::compute_waves;
pub use directory::{RegionDirectory, RegionIdAllocator};
pub use executor::{RcExecutor, TickReport};
pub use grid::GridCell;
pub use lifecycle::{LifecycleOutcome, largest_connectivity_cut};
pub use managed_region::ManagedRegion;
pub use measurement::{RegionTickHistogram, SoakReport, SoakStatus};
pub use pipeline::{DomainGroup, Stage};
pub use region::RegionState;
pub use region_manager::RegionManager;
pub use registry::{ExecutorBuildError, RcExecutorBuilder, SystemFactory, SystemId};
pub use synthetic_load::{
    SyntheticLoadProfile, bootstrap_default_profile, busy_spin, synthetic_busy_work_system,
    synthetic_system_factory,
};
