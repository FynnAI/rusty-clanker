//! Parsed `cargo metadata --format-version 1` shape (only the fields this
//! blueprint's rule-checker needs; every field name matches cargo's real
//! JSON schema).

/// Top-level `cargo metadata --format-version 1` output.
#[derive(serde::Deserialize)]
pub struct CargoMetadata {
    pub packages: Vec<Package>,
    pub resolve: Resolve,
    pub workspace_members: Vec<String>,
}

#[derive(serde::Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct Resolve {
    pub nodes: Vec<Node>,
}

#[derive(serde::Deserialize)]
pub struct Node {
    pub id: String,
    /// All resolved dependency edges from this node, any kind, as PackageIds.
    pub dependencies: Vec<String>,
    /// Same edges, individually kind-tagged.
    pub deps: Vec<Dep>,
}

#[derive(serde::Deserialize)]
pub struct Dep {
    pub pkg: String,
    pub dep_kinds: Vec<DepKind>,
}

#[derive(serde::Deserialize)]
pub struct DepKind {
    /// `None` = normal dependency; `Some("dev")` / `Some("build")` otherwise.
    pub kind: Option<String>,
}

/// Runs `cargo metadata --format-version 1 --all-features` via `sh` and parses stdout.
/// Returns `Err(<process/parse error message>)` on any failure.
pub fn fetch_metadata(sh: &xshell::Shell) -> Result<CargoMetadata, String> {
    let stdout = xshell::cmd!(sh, "cargo metadata --format-version 1 --all-features")
        .read()
        .map_err(|err| format!("`cargo metadata` failed: {err}"))?;
    serde_json::from_str::<CargoMetadata>(&stdout)
        .map_err(|err| format!("failed to parse `cargo metadata` output: {err}"))
}
