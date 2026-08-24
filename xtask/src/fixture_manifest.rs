//! TEST-D47's fixture integrity manifest, restated concretely for this blueprint's
//! generated artifacts (and reusable by any later blueprint that generates other
//! fixture kinds TEST-D47 also covers — golden data, `rc-gametest` structures, worldgen
//! seed-corpus entries — none of which this blueprint produces).

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct FixtureManifest {
    pub protocol_version: u32,
    pub mc_version: String,
    /// One row per fixture, per TEST-D47's exact wording: "relative path, SHA-256 of
    /// the fixture's own bytes, the generator/tool version that produced it, and the
    /// source vanilla-jar hash it was derived from."
    pub entries: Vec<FixtureEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct FixtureEntry {
    /// Relative to the manifest file's own directory.
    pub path: String,
    /// Lowercase hex, 64 characters.
    pub sha256: String,
    pub generator_tool_version: String,
    /// SHA-1 (lowercase hex) of the `server.jar` this fixture was derived from.
    pub source_jar_sha1: String,
}

/// One manifest-vs-disk discrepancy.
pub struct ManifestViolation {
    pub path: String,
    /// `"missing"` | `"hash_mismatch"`.
    pub kind: &'static str,
    pub message: String,
}

pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    let _ = bytes;
    todo!()
}

/// Builds a manifest whose every entry's `sha256` is `compute_sha256_hex` of that
/// file's own bytes; `generator_tool_version`/`source_jar_sha1` are copied verbatim
/// onto every entry from the corresponding arguments.
pub fn build_manifest(
    protocol_version: u32,
    mc_version: &str,
    files: &[(String, Vec<u8>)],
    generator_tool_version: &str,
    source_jar_sha1: &str,
) -> FixtureManifest {
    let _ = (
        protocol_version,
        mc_version,
        files,
        generator_tool_version,
        source_jar_sha1,
    );
    todo!()
}

/// Reads the manifest JSON at `manifest_path`, and for every listed entry reads
/// `base_dir.join(&entry.path)`, recomputes its SHA-256, and compares. Returns one
/// `ManifestViolation` per entry whose file is missing or whose hash does not match;
/// an empty result means every listed fixture verified. Does not flag files present on
/// disk but absent from the manifest (out of this blueprint's stated scope — see
/// Constraints).
pub fn verify_manifest(
    manifest_path: &std::path::Path,
    base_dir: &std::path::Path,
) -> Vec<ManifestViolation> {
    let _ = (manifest_path, base_dir);
    todo!()
}
