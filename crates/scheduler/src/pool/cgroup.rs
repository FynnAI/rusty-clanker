//! cgroup-aware `baseline` clamping (PERF-D57), resolving `01`'s own recorded
//! Open Question on `available_parallelism()` under containers. Pure parsers
//! are not `cfg`-gated (unit-testable identically on both OS legs); only the
//! filesystem-reading `read_cgroup_cores` is Linux-only.

/// Pure parser for cgroup v2's `cpu.max` file content (`"$QUOTA $PERIOD"` or
/// `"max $PERIOD"`). `None` for an unlimited quota or malformed content.
/// `ceil(quota / period)` on a finite quota (PERF-D57's exact formula).
pub fn parse_cgroup_v2_max(content: &str) -> Option<u64> {
    let _ = content;
    todo!()
}

/// Pure parser for cgroup v1's split quota/period files. `quota <= 0` is the
/// documented "unlimited" sentinel and returns `None`.
pub fn parse_cgroup_v1(quota_us: &str, period_us: &str) -> Option<u64> {
    let _ = (quota_us, period_us);
    todo!()
}

/// Reads `/sys/fs/cgroup/cpu.max` (v2), falling back to
/// `/sys/fs/cgroup/cpu/{cpu.cfs_quota_us,cpu.cfs_period_us}` (v1) if absent.
/// Linux-only (`#[cfg(target_os = "linux")]`); `None` on any read/parse
/// failure or an unlimited quota.
#[cfg(target_os = "linux")]
pub fn read_cgroup_cores() -> Option<usize> {
    todo!()
}
