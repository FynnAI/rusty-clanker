//! M0-B04 acceptance: pure-function tests for the cgroup-aware baseline
//! sizing helpers (PERF-D57). No threads; run identically on both OS legs —
//! `cgroup.rs`'s parse functions are not `cfg`-gated, only
//! `read_cgroup_cores` is.

use rc_scheduler::pool::cgroup::{parse_cgroup_v1, parse_cgroup_v2_max};
use rc_scheduler::pool::{compute_baseline, compute_hard_cap};

#[test]
fn cgroup_v2_parses_finite_quota() {
    assert_eq!(parse_cgroup_v2_max("150000 100000"), Some(2));
}

#[test]
fn cgroup_v2_unlimited_is_none() {
    assert_eq!(parse_cgroup_v2_max("max 100000"), None);
}

#[test]
fn cgroup_v2_malformed_is_none() {
    assert_eq!(parse_cgroup_v2_max("garbage"), None);
    assert_eq!(parse_cgroup_v2_max(""), None);
}

#[test]
fn cgroup_v1_parses_finite_quota() {
    assert_eq!(parse_cgroup_v1("150000", "100000"), Some(2));
}

#[test]
fn cgroup_v1_unlimited_sentinel_is_none() {
    assert_eq!(parse_cgroup_v1("-1", "100000"), None);
}

#[test]
fn compute_hard_cap_doubles_baseline() {
    assert_eq!(compute_hard_cap(4), 8);
}

#[test]
fn compute_baseline_is_at_least_one() {
    assert!(compute_baseline() >= 1);
}
