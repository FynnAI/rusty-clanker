//! M3-B07 — `check_state_id_consistency`, the one pure function of the capture
//! pipeline (blueprint Acceptance tests, `capture_pure_helpers.rs`). No real oracle,
//! no network, no locally installed Java, required to go green.
//!
//! M3.5-B03 follow-up (protocol-diff closure wave): `server_properties_text`/
//! `clear_world_dir` added here too — both pure or filesystem-local-only, no real
//! oracle/network/Java required either.

use rc_gametest::capture::{check_state_id_consistency, clear_world_dir, server_properties_text};
use rc_gametest::spec::PlacedBlock;

fn sample_block(state_id: u32) -> PlacedBlock {
    PlacedBlock {
        pos: (0, 1, 0),
        vanilla_state: "minecraft:redstone_torch[lit=true]".to_string(),
        state_id,
        has_analog_state: false,
    }
}

#[test]
fn check_state_id_consistency_passes_on_match() {
    let declared = sample_block(100);
    assert_eq!(check_state_id_consistency(&declared, 100), Ok(()));
}

#[test]
fn check_state_id_consistency_flags_mismatch() {
    let declared = sample_block(100);
    let result = check_state_id_consistency(&declared, 101);
    assert_eq!(result, Err((100, 101)));
}

/// M3.5-B03 follow-up: `server_properties_text` writes exactly one spawn-related
/// key (`spawn-monsters=false`) — verified against the pinned 26.2 reference's own
/// dedicated-server-properties class, which declares no `spawn-animals`/`spawn-npcs`
/// key at all in this version (`capture.rs`'s own doc comment has the full
/// citation). This test pins the exact written text so a future edit cannot
/// silently reintroduce either retired key without a visible diff here.
#[test]
fn server_properties_text_has_no_dead_spawn_keys() {
    let text = server_properties_text(25566);
    assert!(text.contains("spawn-monsters=false\n"));
    assert!(!text.contains("spawn-animals"));
    assert!(!text.contains("spawn-npcs"));
}

#[test]
fn server_properties_text_is_exact() {
    let text = server_properties_text(25566);
    assert_eq!(
        text,
        "online-mode=false\n\
         level-type=flat\n\
         generate-structures=false\n\
         spawn-protection=0\n\
         difficulty=peaceful\n\
         gamemode=creative\n\
         spawn-monsters=false\n\
         server-port=25566\n\
         view-distance=5\n"
    );
}

/// A unique-per-call scratch directory under the OS temp dir (mirrors the M3.5
/// governance "self-test fixture path unique per call" fix — parallel test-binary
/// threads must never share one fixture path).
fn unique_scratch_dir(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "rc-gametest-clear-world-dir-{tag}-{}-{n}",
        std::process::id()
    ))
}

#[test]
fn clear_world_dir_removes_an_existing_save() {
    let work_dir = unique_scratch_dir("existing");
    let world_dir = work_dir.join("world");
    std::fs::create_dir_all(world_dir.join("dimensions/minecraft/overworld/region")).unwrap();
    std::fs::write(world_dir.join("level.dat"), b"stale").unwrap();
    assert!(world_dir.exists());

    clear_world_dir(&work_dir).expect("clear_world_dir must succeed");

    assert!(!world_dir.exists(), "stale world save must be gone");
    let _ = std::fs::remove_dir_all(&work_dir);
}

#[test]
fn clear_world_dir_is_a_no_op_when_absent() {
    let work_dir = unique_scratch_dir("absent");
    // Deliberately never created — `work_dir` itself does not even exist yet.
    clear_world_dir(&work_dir).expect("clear_world_dir must not error on a missing world dir");
    assert!(!work_dir.exists());
}

#[test]
fn clear_world_dir_never_touches_sibling_files() {
    let work_dir = unique_scratch_dir("siblings");
    std::fs::create_dir_all(&work_dir).unwrap();
    std::fs::write(work_dir.join("eula.txt"), b"eula=true\n").unwrap();
    std::fs::create_dir_all(work_dir.join("world")).unwrap();

    clear_world_dir(&work_dir).expect("clear_world_dir must succeed");

    assert!(!work_dir.join("world").exists());
    assert!(
        work_dir.join("eula.txt").exists(),
        "sibling files (eula.txt, libraries/, versions/, ...) must survive"
    );
    let _ = std::fs::remove_dir_all(&work_dir);
}
