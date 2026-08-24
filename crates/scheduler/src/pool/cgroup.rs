//! cgroup-aware `baseline` clamping (PERF-D57), resolving `01`'s own recorded
//! Open Question on `available_parallelism()` under containers. Pure parsers
//! are not `cfg`-gated (unit-testable identically on both OS legs); only the
//! filesystem-reading `read_cgroup_cores` is Linux-only.

/// Pure parser for cgroup v2's `cpu.max` file content (`"$QUOTA $PERIOD"` or
/// `"max $PERIOD"`). `None` for an unlimited quota or malformed content.
/// `ceil(quota / period)` on a finite quota (PERF-D57's exact formula).
pub fn parse_cgroup_v2_max(content: &str) -> Option<u64> {
    let mut parts = content.split_whitespace();
    let quota_str = parts.next()?;
    let period_str = parts.next()?;
    if quota_str == "max" {
        return None;
    }
    let quota: u64 = quota_str.parse().ok()?;
    let period: u64 = period_str.parse().ok()?;
    if period == 0 {
        return None;
    }
    Some(quota.div_ceil(period))
}

/// Pure parser for cgroup v1's split quota/period files. `quota <= 0` is the
/// documented "unlimited" sentinel and returns `None`.
pub fn parse_cgroup_v1(quota_us: &str, period_us: &str) -> Option<u64> {
    let quota: i64 = quota_us.trim().parse().ok()?;
    if quota <= 0 {
        return None;
    }
    let period: u64 = period_us.trim().parse().ok()?;
    if period == 0 {
        return None;
    }
    Some((quota as u64).div_ceil(period))
}

/// Reads `/sys/fs/cgroup/cpu.max` (v2), falling back to
/// `/sys/fs/cgroup/cpu/{cpu.cfs_quota_us,cpu.cfs_period_us}` (v1) if absent.
/// Linux-only (`#[cfg(target_os = "linux")]`); `None` on any read/parse
/// failure or an unlimited quota.
#[cfg(target_os = "linux")]
pub fn read_cgroup_cores() -> Option<usize> {
    if let Ok(content) = std::fs::read_to_string("/sys/fs/cgroup/cpu.max") {
        return parse_cgroup_v2_max(content.trim()).map(|cores| cores as usize);
    }
    let quota = std::fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_quota_us").ok()?;
    let period = std::fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_period_us").ok()?;
    parse_cgroup_v1(&quota, &period).map(|cores| cores as usize)
}
