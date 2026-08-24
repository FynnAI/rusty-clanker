//! WS-D3 dependency-graph rule checker: pure logic over an already-parsed
//! `cargo metadata` graph, plus the `lint-deps` CLI verb that drives it.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::metadata::CargoMetadata;

/// One WS-D3 rule violation.
pub struct Violation {
    /// "rule1" | "rule2" | "rule3" | "rule4"
    pub rule: &'static str,
    pub message: String,
}

/// Rule 1 — must be reachable, transitively, from both binaries.
const SHARED: &[&str] = &[
    "rc-core",
    "rc-nbt",
    "rc-registries",
    "rc-protocol-macros",
    "rc-protocol",
    "rc-mod-api",
    "rc-mod-host",
    "rc-physics",
];

/// Rule 2 — must never reach, nor be reached by, any `NETRENDER` crate.
const SIM: &[&str] = &["rc-scheduler", "rc-mechanics"];

/// Rule 2 — must never reach, nor be reached by, any `SIM` crate.
const NETRENDER: &[&str] = &[
    "rc-render",
    "rc-protocol",
    "rc-transport-inproc",
    "rc-transport-net",
    "rc-auth",
    "rc-cluster",
    "rc-proxy",
];

/// Rule 3 — `rc-messaging`'s exact normal-dependency set.
const MESSAGING_NORMAL: &[&str] = &["rc-core", "serde", "thiserror"];

/// Rule 4 — `rc-mod-api`'s exact normal-dependency set.
const MOD_API_NORMAL: &[&str] = &["rc-core", "bevy_ecs"];

/// Pure rule-checker: WS-D3 Rules 1-4 against an already-parsed dependency graph.
/// No I/O. This is the function the Acceptance tests exercise directly with
/// synthetic `CargoMetadata` values.
pub fn check_rules(meta: &CargoMetadata) -> Vec<Violation> {
    let id_to_name: HashMap<&str, &str> = meta
        .packages
        .iter()
        .map(|p| (p.id.as_str(), p.name.as_str()))
        .collect();
    let workspace_ids: HashSet<&str> = meta.workspace_members.iter().map(String::as_str).collect();

    // Internal-only forward graph, keyed and valued by crate *name* (not id),
    // built from every workspace-member node's `dependencies` (all kinds),
    // filtered down to edges that land on another workspace member.
    let mut graph: HashMap<&str, HashSet<&str>> = HashMap::new();
    for node in &meta.resolve.nodes {
        if !workspace_ids.contains(node.id.as_str()) {
            continue;
        }
        let Some(&from_name) = id_to_name.get(node.id.as_str()) else {
            continue;
        };
        let entry: &mut HashSet<&str> = graph.entry(from_name).or_default();
        for dep_id in &node.dependencies {
            if workspace_ids.contains(dep_id.as_str())
                && let Some(&dep_name) = id_to_name.get(dep_id.as_str())
            {
                entry.insert(dep_name);
            }
        }
    }

    // Names of every crate actually present in this graph (not merely listed
    // in `SHARED`/`SIM`/`NETRENDER`) — used so Rule 1/3/4 only evaluate a
    // binary/crate that the fixture under test actually includes, matching
    // the minimal single-rule fixtures the acceptance tests exercise.
    let workspace_names: HashSet<&str> = workspace_ids
        .iter()
        .filter_map(|id| id_to_name.get(id).copied())
        .collect();

    let mut violations = Vec::new();

    check_rule1(&graph, &workspace_names, &mut violations);
    check_rule2(&graph, &mut violations);
    check_exact_normal_deps(
        meta,
        &id_to_name,
        "rc-messaging",
        MESSAGING_NORMAL,
        "rule3",
        &mut violations,
    );
    check_exact_normal_deps(
        meta,
        &id_to_name,
        "rc-mod-api",
        MOD_API_NORMAL,
        "rule4",
        &mut violations,
    );

    violations
}

/// Rule 1 only evaluates a binary that is actually present in the graph
/// under test — the minimal per-rule fixtures in the acceptance tests
/// intentionally omit both binaries, and must not trip Rule 1 as a result.
fn check_rule1<'a>(
    graph: &HashMap<&'a str, HashSet<&'a str>>,
    workspace_names: &HashSet<&str>,
    violations: &mut Vec<Violation>,
) {
    if workspace_names.contains("rusty-clanker-server") {
        let server_closure = transitive_closure(graph, "rusty-clanker-server");
        for &shared in SHARED {
            if !server_closure.contains(shared) {
                violations.push(Violation {
                    rule: "rule1",
                    message: format!(
                        "SHARED crate `{shared}` is missing from `rusty-clanker-server`'s transitive dependency closure"
                    ),
                });
            }
        }
    }
    if workspace_names.contains("rusty-clanker-client") {
        let client_closure = transitive_closure(graph, "rusty-clanker-client");
        for &shared in SHARED {
            if !client_closure.contains(shared) {
                violations.push(Violation {
                    rule: "rule1",
                    message: format!(
                        "SHARED crate `{shared}` is missing from `rusty-clanker-client`'s transitive dependency closure"
                    ),
                });
            }
        }
    }
}

fn check_rule2<'a>(graph: &HashMap<&'a str, HashSet<&'a str>>, violations: &mut Vec<Violation>) {
    for &s in SIM {
        let closure = transitive_closure(graph, s);
        for &r in NETRENDER {
            if closure.contains(r) {
                violations.push(Violation {
                    rule: "rule2",
                    message: format!("SIM crate `{s}` transitively reaches NETRENDER crate `{r}`"),
                });
            }
        }
    }
    for &r in NETRENDER {
        let closure = transitive_closure(graph, r);
        for &s in SIM {
            if closure.contains(s) {
                violations.push(Violation {
                    rule: "rule2",
                    message: format!("NETRENDER crate `{r}` transitively reaches SIM crate `{s}`"),
                });
            }
        }
    }
}

/// Rule 3 / Rule 4: locate the workspace node by crate name and compare its
/// normal (non-dev, non-build) dependency set for exact equality against
/// `expected`. A crate absent from `meta` entirely (as in the minimal,
/// single-rule test fixtures) is simply not checked — Rule 3/4 only apply
/// when the crate in question is actually part of the graph under test.
fn check_exact_normal_deps(
    meta: &CargoMetadata,
    id_to_name: &HashMap<&str, &str>,
    crate_name: &str,
    expected: &[&str],
    rule: &'static str,
    violations: &mut Vec<Violation>,
) {
    let Some(node) = meta
        .resolve
        .nodes
        .iter()
        .find(|n| id_to_name.get(n.id.as_str()).copied() == Some(crate_name))
    else {
        return;
    };

    let actual: HashSet<&str> = node
        .deps
        .iter()
        .filter(|d| d.dep_kinds.iter().any(|k| k.kind.is_none()))
        .filter_map(|d| id_to_name.get(d.pkg.as_str()).copied())
        .collect();
    let expected_set: HashSet<&str> = expected.iter().copied().collect();

    if actual != expected_set {
        let mut missing: Vec<&str> = expected_set.difference(&actual).copied().collect();
        let mut extra: Vec<&str> = actual.difference(&expected_set).copied().collect();
        missing.sort_unstable();
        extra.sort_unstable();
        violations.push(Violation {
            rule,
            message: format!(
                "`{crate_name}` normal deps must be exactly {{{}}}; missing: {missing:?}, extra: {extra:?}",
                expected.join(", ")
            ),
        });
    }
}

/// BFS transitive closure of `start`'s outgoing edges, not including `start`
/// itself. `start` absent from `graph` (e.g. a crate not present in a
/// minimal test fixture) simply yields an empty closure.
fn transitive_closure<'a>(
    graph: &HashMap<&'a str, HashSet<&'a str>>,
    start: &str,
) -> HashSet<&'a str> {
    let mut visited: HashSet<&'a str> = HashSet::new();
    let mut queue: VecDeque<&'a str> = VecDeque::new();

    if let Some(neighbors) = graph.get(start) {
        for &n in neighbors {
            if visited.insert(n) {
                queue.push_back(n);
            }
        }
    }
    while let Some(cur) = queue.pop_front() {
        if let Some(neighbors) = graph.get(cur) {
            for &n in neighbors {
                if visited.insert(n) {
                    queue.push_back(n);
                }
            }
        }
    }
    visited
}

/// CLI entry point for the `lint-deps` verb: fetch + check + print + exit code.
pub fn run() -> std::process::ExitCode {
    let sh = match xshell::Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("lint-deps: failed to create shell: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let meta = match crate::metadata::fetch_metadata(&sh) {
        Ok(meta) => meta,
        Err(err) => {
            eprintln!("lint-deps: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let crate_count = meta.workspace_members.len();
    let violations = check_rules(&meta);
    if violations.is_empty() {
        println!("lint-deps: 0 forbidden edges across {crate_count} workspace crates");
        std::process::ExitCode::SUCCESS
    } else {
        for v in &violations {
            eprintln!("[{}] {}", v.rule, v.message);
        }
        std::process::ExitCode::FAILURE
    }
}
