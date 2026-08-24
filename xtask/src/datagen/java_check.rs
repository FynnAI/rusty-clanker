//! Local Java-runtime detection (NET-D9's "runs java ... locally" precondition).

/// Documented, well-known Java major-version requirement for Minecraft 26.2's own
/// `server.jar` (docs/research/mc-26.2/00-source-overview.md §5, sourced from the
/// pinned version's own `version.json`, component `java-runtime-epsilon`). Used only as
/// a fallback when `--offline` (Deliverables, `fetch.rs`) prevents constructing a
/// `fetch_data::FetchedJar` with a real, manifest-sourced `min_java_major` — every
/// other code path reads the requirement dynamically off that field, never this
/// constant.
pub const FALLBACK_MIN_JAVA_MAJOR: u32 = 25;

/// Parses the major version number out of `java -version`'s combined stdout+stderr
/// text (the JVM historically writes this to **stderr**; some wrapped/managed JDK
/// launchers redirect to stdout instead — callers concatenate both streams before
/// calling this so stream choice never matters). Handles the modern scheme
/// (`"25"`, `"25.0.1"`) and the legacy `"1.MAJOR.MINOR_PATCH"` scheme used through
/// Java 8 (`"1.8.0_301"` -> `8`). Returns `None` if no quoted version string is found
/// or its leading component does not parse as an integer.
pub fn parse_java_major(version_output: &str) -> Option<u32> {
    let first_quote = version_output.find('"')?;
    let rest = &version_output[first_quote + 1..];
    let end_quote = rest.find('"')?;
    let version_str = &rest[..end_quote];

    let mut parts = version_str.split(['.', '-']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        // Legacy "1.MAJOR.MINOR_PATCH" scheme (through Java 8).
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Runs `java -version`, parses its output, and compares against `min_major`.
/// `Ok(detected_major)` if `detected_major >= min_major`. `Err(<actionable message>)`
/// naming exactly one of: `java` not found on `PATH`; output did not contain a
/// parseable version string; or a detected major version below `min_major` (message
/// names both the detected and required values and links to
/// <https://adoptium.net> as a concrete "how do I get one" pointer).
pub fn check_java(min_major: u32) -> Result<u32, String> {
    let output = std::process::Command::new("java")
        .arg("-version")
        .output()
        .map_err(|_| {
            format!(
                "`java` not found on PATH. Minecraft's data generator requires a local Java \
                 {min_major}+ runtime — install one (e.g. https://adoptium.net) and ensure \
                 `java -version` succeeds before retrying."
            )
        })?;

    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));

    match parse_java_major(&combined) {
        None => Err(format!(
            "could not parse a Java version string out of `java -version`'s output. Raw output \
             was:\n{combined}"
        )),
        Some(major) if major < min_major => Err(format!(
            "detected Java {major}, but Minecraft's data generator requires Java {min_major}+ \
             — install a newer runtime (e.g. https://adoptium.net)."
        )),
        Some(major) => Ok(major),
    }
}
