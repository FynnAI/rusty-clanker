use xtask::lint_deps::check_rules;
use xtask::metadata::{CargoMetadata, Dep, DepKind, Node, Package, Resolve};

fn node(id: &str, all_deps: &[&str], normal_deps: &[&str]) -> Node {
    Node {
        id: id.to_string(),
        dependencies: all_deps.iter().map(|s| s.to_string()).collect(),
        deps: normal_deps
            .iter()
            .map(|s| Dep {
                pkg: s.to_string(),
                dep_kinds: vec![DepKind { kind: None }],
            })
            .collect(),
    }
}

fn pkg(id: &str, name: &str) -> Package {
    Package {
        id: id.to_string(),
        name: name.to_string(),
    }
}

/// The complete 25-crate workspace membership list, id == name for every
/// synthetic fixture in this file.
fn workspace_members() -> Vec<String> {
    [
        "rc-core",
        "rc-nbt",
        "rc-registries",
        "rc-protocol-macros",
        "rc-protocol",
        "rc-mod-api",
        "rc-mod-host",
        "rc-messaging",
        "rc-transport-inproc",
        "rc-transport-net",
        "rc-chunk-storage",
        "rc-worldgen",
        "rc-scheduler",
        "rc-mechanics",
        "rc-physics",
        "rc-entity-macros",
        "rc-brigadier",
        "rc-auth",
        "rc-cluster",
        "rc-proxy",
        "rc-assets",
        "rc-render",
        "rusty-clanker-server",
        "rusty-clanker-client",
        "xtask",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Every workspace crate plus the three external crates referenced by
/// Rule 3 (`rc-messaging`) / Rule 4 (`rc-mod-api`) checks in the clean graph.
fn clean_packages() -> Vec<Package> {
    let mut pkgs: Vec<Package> = workspace_members().iter().map(|id| pkg(id, id)).collect();
    pkgs.push(pkg("bevy_ecs", "bevy_ecs"));
    pkgs.push(pkg("serde", "serde"));
    pkgs.push(pkg("thiserror", "thiserror"));
    pkgs
}

/// The real, corrected M0-B01 edge table (see this blueprint's Deviations:
/// `rc-cluster` depends only on `rc-messaging` + `rc-transport-net`, never
/// `rc-scheduler` directly — required by WS-D3 Rule 2, which forbids any
/// `SIM` <-> `NETRENDER` edge in either direction; cross-partition
/// coordination between cluster orchestration and the tick scheduler flows
/// through the `rc-messaging` message substrate instead, never a direct
/// crate dependency).
fn clean_nodes() -> Vec<Node> {
    vec![
        node("rc-core", &[], &[]),
        node("rc-nbt", &["rc-core"], &["rc-core"]),
        node(
            "rc-registries",
            &["rc-core", "rc-nbt"],
            &["rc-core", "rc-nbt"],
        ),
        node("rc-protocol-macros", &[], &[]),
        node(
            "rc-protocol",
            &["rc-core", "rc-nbt", "rc-registries", "rc-protocol-macros"],
            &["rc-core", "rc-nbt", "rc-registries", "rc-protocol-macros"],
        ),
        node(
            "rc-mod-api",
            &["rc-core", "bevy_ecs"],
            &["rc-core", "bevy_ecs"],
        ),
        node(
            "rc-mod-host",
            &["rc-core", "rc-mod-api"],
            &["rc-core", "rc-mod-api"],
        ),
        node(
            "rc-messaging",
            &["rc-core", "serde", "thiserror"],
            &["rc-core", "serde", "thiserror"],
        ),
        node("rc-transport-inproc", &["rc-messaging"], &["rc-messaging"]),
        node("rc-transport-net", &["rc-messaging"], &["rc-messaging"]),
        node(
            "rc-chunk-storage",
            &["rc-core", "rc-nbt", "rc-registries"],
            &["rc-core", "rc-nbt", "rc-registries"],
        ),
        node(
            "rc-worldgen",
            &["rc-core", "rc-chunk-storage", "rc-registries"],
            &["rc-core", "rc-chunk-storage", "rc-registries"],
        ),
        node(
            "rc-scheduler",
            &["rc-core", "rc-messaging", "rc-mod-host"],
            &["rc-core", "rc-messaging", "rc-mod-host"],
        ),
        node(
            "rc-mechanics",
            &[
                "rc-core",
                "rc-registries",
                "rc-mod-api",
                "rc-physics",
                "rc-entity-macros",
                "rc-scheduler",
                "rc-chunk-storage",
                "rc-brigadier",
            ],
            &[
                "rc-core",
                "rc-registries",
                "rc-mod-api",
                "rc-physics",
                "rc-entity-macros",
                "rc-scheduler",
                "rc-chunk-storage",
                "rc-brigadier",
            ],
        ),
        node("rc-physics", &["rc-core"], &["rc-core"]),
        node("rc-entity-macros", &[], &[]),
        node("rc-brigadier", &["rc-core"], &["rc-core"]),
        node("rc-auth", &["rc-core"], &["rc-core"]),
        node(
            "rc-cluster",
            &["rc-messaging", "rc-transport-net"],
            &["rc-messaging", "rc-transport-net"],
        ),
        node(
            "rc-proxy",
            &["rc-cluster", "rc-transport-net", "rc-auth", "rc-protocol"],
            &["rc-cluster", "rc-transport-net", "rc-auth", "rc-protocol"],
        ),
        node(
            "rc-assets",
            &["rc-core", "rc-registries"],
            &["rc-core", "rc-registries"],
        ),
        node(
            "rc-render",
            &["rc-core", "rc-registries", "rc-assets", "rc-mod-host"],
            &["rc-core", "rc-registries", "rc-assets", "rc-mod-host"],
        ),
        node(
            "rusty-clanker-server",
            &[
                "rc-core",
                "rc-scheduler",
                "rc-mechanics",
                "rc-chunk-storage",
                "rc-worldgen",
                "rc-protocol",
                "rc-transport-inproc",
                "rc-auth",
                "rc-mod-host",
                "rc-cluster",
                "rc-transport-net",
                "rc-proxy",
            ],
            &[
                "rc-core",
                "rc-scheduler",
                "rc-mechanics",
                "rc-chunk-storage",
                "rc-worldgen",
                "rc-protocol",
                "rc-transport-inproc",
                "rc-auth",
                "rc-mod-host",
                "rc-cluster",
                "rc-transport-net",
                "rc-proxy",
            ],
        ),
        node(
            "rusty-clanker-client",
            &[
                "rc-core",
                "rc-protocol",
                "rc-registries",
                "rc-nbt",
                "rc-assets",
                "rc-render",
                "rc-physics",
                "rc-mod-host",
                "rc-mechanics",
            ],
            &[
                "rc-core",
                "rc-protocol",
                "rc-registries",
                "rc-nbt",
                "rc-assets",
                "rc-render",
                "rc-physics",
                "rc-mod-host",
                "rc-mechanics",
            ],
        ),
        node("xtask", &[], &[]),
    ]
}

fn clean_metadata() -> CargoMetadata {
    CargoMetadata {
        packages: clean_packages(),
        resolve: Resolve {
            nodes: clean_nodes(),
        },
        workspace_members: workspace_members(),
    }
}

#[test]
fn clean_graph_has_zero_violations() {
    let meta = clean_metadata();
    let violations = check_rules(&meta);
    assert!(
        violations.is_empty(),
        "expected zero violations, got: {}",
        violations
            .iter()
            .map(|v| format!("[{}] {}", v.rule, v.message))
            .collect::<Vec<_>>()
            .join("; ")
    );
}

#[test]
fn rule1_flags_missing_shared_crate() {
    let mut nodes = clean_nodes();
    let client = nodes
        .iter_mut()
        .find(|n| n.id == "rusty-clanker-client")
        .unwrap();
    client.dependencies.retain(|d| d != "rc-physics");
    client.deps.retain(|d| d.pkg != "rc-physics");

    let meta = CargoMetadata {
        packages: clean_packages(),
        resolve: Resolve { nodes },
        workspace_members: workspace_members(),
    };
    let violations = check_rules(&meta);
    assert_eq!(
        violations.len(),
        1,
        "violations: {:?}",
        violations_debug(&violations)
    );
    assert_eq!(violations[0].rule, "rule1");
    assert!(violations[0].message.contains("rc-physics"));
    assert!(violations[0].message.contains("rusty-clanker-client"));
}

#[test]
fn rule2_flags_scheduler_reaching_render() {
    let meta = CargoMetadata {
        packages: vec![
            pkg("rc-scheduler", "rc-scheduler"),
            pkg("rc-render", "rc-render"),
        ],
        resolve: Resolve {
            nodes: vec![
                node("rc-scheduler", &["rc-render"], &["rc-render"]),
                node("rc-render", &[], &[]),
            ],
        },
        workspace_members: vec!["rc-scheduler".to_string(), "rc-render".to_string()],
    };
    let violations = check_rules(&meta);
    assert_eq!(
        violations.len(),
        1,
        "violations: {:?}",
        violations_debug(&violations)
    );
    assert_eq!(violations[0].rule, "rule2");
}

#[test]
fn rule2_flags_transitive_violation() {
    let meta = CargoMetadata {
        packages: vec![
            pkg("rc-scheduler", "rc-scheduler"),
            pkg("rc-mod-host", "rc-mod-host"),
            pkg("rc-render", "rc-render"),
        ],
        resolve: Resolve {
            nodes: vec![
                node("rc-scheduler", &["rc-mod-host"], &["rc-mod-host"]),
                node("rc-mod-host", &["rc-render"], &["rc-render"]),
                node("rc-render", &[], &[]),
            ],
        },
        workspace_members: vec![
            "rc-scheduler".to_string(),
            "rc-mod-host".to_string(),
            "rc-render".to_string(),
        ],
    };
    let violations = check_rules(&meta);
    assert_eq!(
        violations.len(),
        1,
        "transitive violation must still fire: {:?}",
        violations_debug(&violations)
    );
    assert_eq!(violations[0].rule, "rule2");
}

#[test]
fn rule2_allows_scheduler_and_mechanics_depending_on_each_other() {
    let meta = CargoMetadata {
        packages: vec![
            pkg("rc-mechanics", "rc-mechanics"),
            pkg("rc-scheduler", "rc-scheduler"),
        ],
        resolve: Resolve {
            nodes: vec![
                node("rc-mechanics", &["rc-scheduler"], &["rc-scheduler"]),
                node("rc-scheduler", &[], &[]),
            ],
        },
        workspace_members: vec!["rc-mechanics".to_string(), "rc-scheduler".to_string()],
    };
    let violations = check_rules(&meta);
    assert!(
        violations.is_empty(),
        "SIM-to-SIM edge must not be flagged: {:?}",
        violations_debug(&violations)
    );
}

#[test]
fn rule3_flags_extra_normal_dep() {
    let meta = CargoMetadata {
        packages: vec![
            pkg("rc-messaging", "rc-messaging"),
            pkg("rc-core", "rc-core"),
            pkg("serde", "serde"),
            pkg("thiserror", "thiserror"),
            pkg("crossbeam-channel", "crossbeam-channel"),
        ],
        resolve: Resolve {
            nodes: vec![node(
                "rc-messaging",
                &["rc-core", "serde", "thiserror", "crossbeam-channel"],
                &["rc-core", "serde", "thiserror", "crossbeam-channel"],
            )],
        },
        workspace_members: vec!["rc-messaging".to_string()],
    };
    let violations = check_rules(&meta);
    assert_eq!(
        violations.len(),
        1,
        "violations: {:?}",
        violations_debug(&violations)
    );
    assert_eq!(violations[0].rule, "rule3");
}

#[test]
fn rule3_flags_missing_required_dep() {
    let meta = CargoMetadata {
        packages: vec![
            pkg("rc-messaging", "rc-messaging"),
            pkg("rc-core", "rc-core"),
            pkg("serde", "serde"),
        ],
        resolve: Resolve {
            nodes: vec![node(
                "rc-messaging",
                &["rc-core", "serde"],
                &["rc-core", "serde"],
            )],
        },
        workspace_members: vec!["rc-messaging".to_string()],
    };
    let violations = check_rules(&meta);
    assert_eq!(
        violations.len(),
        1,
        "violations: {:?}",
        violations_debug(&violations)
    );
    assert_eq!(violations[0].rule, "rule3");
}

#[test]
fn rule3_ignores_dev_dependency() {
    let mut messaging = node(
        "rc-messaging",
        &["rc-core", "serde", "thiserror", "some-test-crate"],
        &["rc-core", "serde", "thiserror"],
    );
    messaging.deps.push(Dep {
        pkg: "some-test-crate".to_string(),
        dep_kinds: vec![DepKind {
            kind: Some("dev".to_string()),
        }],
    });

    let meta = CargoMetadata {
        packages: vec![
            pkg("rc-messaging", "rc-messaging"),
            pkg("rc-core", "rc-core"),
            pkg("serde", "serde"),
            pkg("thiserror", "thiserror"),
            pkg("some-test-crate", "some-test-crate"),
        ],
        resolve: Resolve {
            nodes: vec![messaging],
        },
        workspace_members: vec!["rc-messaging".to_string()],
    };
    let violations = check_rules(&meta);
    assert!(
        violations.is_empty(),
        "dev-dependency must not count toward Rule 3: {:?}",
        violations_debug(&violations)
    );
}

#[test]
fn rule4_flags_extra_normal_dep() {
    let meta = CargoMetadata {
        packages: vec![
            pkg("rc-mod-api", "rc-mod-api"),
            pkg("rc-core", "rc-core"),
            pkg("bevy_ecs", "bevy_ecs"),
            pkg("rc-scheduler", "rc-scheduler"),
        ],
        resolve: Resolve {
            nodes: vec![node(
                "rc-mod-api",
                &["rc-core", "bevy_ecs", "rc-scheduler"],
                &["rc-core", "bevy_ecs", "rc-scheduler"],
            )],
        },
        workspace_members: vec!["rc-mod-api".to_string()],
    };
    let violations = check_rules(&meta);
    assert_eq!(
        violations.len(),
        1,
        "violations: {:?}",
        violations_debug(&violations)
    );
    assert_eq!(violations[0].rule, "rule4");
}

#[test]
fn multiple_violations_all_reported() {
    let meta = CargoMetadata {
        packages: vec![
            pkg("rc-scheduler", "rc-scheduler"),
            pkg("rc-render", "rc-render"),
            pkg("rc-messaging", "rc-messaging"),
            pkg("rc-core", "rc-core"),
            pkg("serde", "serde"),
            pkg("thiserror", "thiserror"),
            pkg("crossbeam-channel", "crossbeam-channel"),
        ],
        resolve: Resolve {
            nodes: vec![
                node("rc-scheduler", &["rc-render"], &["rc-render"]),
                node("rc-render", &[], &[]),
                node(
                    "rc-messaging",
                    &["rc-core", "serde", "thiserror", "crossbeam-channel"],
                    &["rc-core", "serde", "thiserror", "crossbeam-channel"],
                ),
            ],
        },
        workspace_members: vec![
            "rc-scheduler".to_string(),
            "rc-render".to_string(),
            "rc-messaging".to_string(),
        ],
    };
    let violations = check_rules(&meta);
    assert_eq!(
        violations.len(),
        2,
        "violations: {:?}",
        violations_debug(&violations)
    );
    let mut rules: Vec<&str> = violations.iter().map(|v| v.rule).collect();
    rules.sort_unstable();
    assert_eq!(rules, vec!["rule2", "rule3"]);
}

#[test]
fn real_workspace_has_zero_forbidden_edges() {
    let sh = xshell::Shell::new().unwrap();
    let meta = xtask::metadata::fetch_metadata(&sh).expect("cargo metadata failed");
    let violations = check_rules(&meta);
    assert!(
        violations.is_empty(),
        "violations: {:?}",
        violations.iter().map(|v| &v.message).collect::<Vec<_>>()
    );
}

fn violations_debug(violations: &[xtask::lint_deps::Violation]) -> Vec<String> {
    violations
        .iter()
        .map(|v| format!("[{}] {}", v.rule, v.message))
        .collect()
}
