//! M3.5-B03 acceptance tests (test-authoring changeset, TEST-D45) for the
//! `--debug-hooks` server flag: `crates/server/src/main.rs`'s own doc comment names
//! the exact contract this file pins — `debug-setblock`/`debug-gamemode` stdin lines
//! must be completely inert without the flag, and must actually apply with it.
//!
//! Spawns a real, freshly-built `rusty-clanker-server` release binary
//! (`ManagedServerConfig::debug_hooks`) and drives a real azalea bot against it —
//! this crate's own nightly toolchain, mirrors `chunk_decode_diagnostic.rs`'s own
//! established "spawn a real release binary, panic loudly if missing, drive one real
//! azalea client inside a `LocalSet`" pattern. Each test gets its own fresh, unique
//! temp `world_dir` (never `ManagedServerConfig::new`'s own relative default) so
//! concurrently-running tests never race the same on-disk world (`HardcodedWorld::
//! new`'s own doc comment has the full "shared relative world dir" hazard this
//! avoids).

use std::path::PathBuf;
use std::time::Duration;

use azalea::BlockPos as AzBlockPos;
use azalea::core::direction::Direction as AzDirection;
use azalea::protocol::packets::game::s_player_action::{
    Action as PlayerActionKind, ServerboundPlayerAction,
};
use rc_paritybot::packet_capture::{BlockSnapshotView, connect_and_observe};
use rc_test_harness::process::{ManagedServer, ManagedServerConfig, spawn_server};

const LOGIN_TIMEOUT: Duration = Duration::from_secs(30);
/// Bot usernames — `[a-zA-Z0-9_]`, well under 16 characters (`corpus_capture.rs`'s
/// own `CORPUS_BOT_NAME` doc comment has the full "silently over-ran vanilla's own
/// limit" field report this convention avoids).
const BOT_NAME: &str = "rc_dbg_hook_bot";
/// A world position well above any terrain this project's own placeholder/superflat
/// world ever generates — guaranteed air, the known starting state every
/// `debug-setblock` case here needs.
const SKY_POS: (i32, i32, i32) = (0, 40, 0);
/// `minecraft:stone`'s default state id (`corpus_capture.rs`'s own established
/// `BARRIER_STATE_STONE` constant, restated here).
const STONE_STATE_ID: u32 = 1;
/// How long a `debug-setblock`/`debug-gamemode` line — and, for the gamemode cases,
/// the one break attempt that follows — is given to land before this file's own
/// assertions read the result. Comfortably longer than one tick-loop drain step
/// (§4.6's own mpsc-plus-oneshot-ack wiring resolves within a tick or two) and
/// comfortably shorter than any real "correct tool" survival break time, so a break
/// still standing at the end of this window is a genuine "still survival" signal, not
/// merely "hasn't finished yet" (blueprint §5's own "creative-speed break completing
/// under the timing window... is the observable signal a gamemode switch never
/// landed").
const SETTLE: Duration = Duration::from_millis(800);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/testing
        .expect("paritybot always lives at <repo>/crates/testing/paritybot")
        .parent() // crates
        .expect("paritybot always lives at <repo>/crates/testing/paritybot")
        .parent() // <repo>
        .expect("paritybot always lives at <repo>/crates/testing/paritybot")
        .to_path_buf()
}

fn release_binary_path() -> PathBuf {
    let name = if cfg!(windows) {
        "rusty-clanker-server.exe"
    } else {
        "rusty-clanker-server"
    };
    repo_root().join("target").join("release").join(name)
}

/// A fresh, uniquely named temp directory, removed best-effort on `Drop` — mirrors
/// `xtask::corpus::placement_diff::TempWorldDir`'s own identical convention, restated
/// locally here for the identical reason that module's own doc comment gives (this
/// integration-test crate has no shared test-only "temp dir" helper to import).
struct TempWorldDir {
    path: PathBuf,
}
impl TempWorldDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rc-debug-hooks-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).expect("create temp world dir");
        Self { path }
    }
}
impl Drop for TempWorldDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn spawn(label: &str, debug_hooks: bool) -> (ManagedServer, TempWorldDir) {
    let binary_path = release_binary_path();
    assert!(
        binary_path.exists(),
        "no release binary at {binary_path:?} -- run `cargo build --release -p \
         rusty-clanker-server --no-default-features --features monolithic` from the \
         repository root first"
    );
    let world = TempWorldDir::new(label);
    let mut config = ManagedServerConfig::new(binary_path);
    config.world_dir = Some(world.path.clone());
    config.debug_hooks = debug_hooks;
    config.startup_timeout = Duration::from_secs(60);
    let managed =
        spawn_server(config).expect("rusty-clanker-server should start and accept a connection");
    (managed, world)
}

async fn wait_for_state(
    view: &BlockSnapshotView,
    pos: (i32, i32, i32),
    timeout: Duration,
) -> Option<u32> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some(state) = view.state_id_at(pos) {
            return Some(state);
        }
        if tokio::time::Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Scans downward from a generous ceiling at the bot's own current column for the
/// first non-air state — mirrors `placement_capture.rs::discover_floor_y`'s own
/// identical algorithm (private to that module; restated here rather than exposed,
/// since this integration-test crate can only ever see `rc_paritybot`'s `pub`
/// surface).
async fn discover_floor(
    client: &azalea::Client,
    view: &BlockSnapshotView,
    timeout: Duration,
) -> (i32, i32, i32) {
    const CEILING: i32 = 24;
    const FLOOR_SCAN_BOTTOM: i32 = -64;
    let pos = client
        .position()
        .expect("bot position readable after spawn");
    let x = pos.x.floor() as i32;
    let z = pos.z.floor() as i32;

    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        for y in (FLOOR_SCAN_BOTTOM..=CEILING).rev() {
            if let Some(state_id) = view.state_id_at((x, y, z))
                && state_id != 0
            {
                return (x, y, z);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("no natural floor ever observed at column ({x}, {z}) within {timeout:?}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn instant_break(client: &azalea::Client, pos: (i32, i32, i32)) {
    client.write_packet(ServerboundPlayerAction {
        action: PlayerActionKind::StartDestroyBlock,
        pos: AzBlockPos::new(pos.0, pos.1, pos.2),
        direction: AzDirection::Up,
        seq: 1,
    });
}

fn current_thread_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("building a current-thread tokio runtime should never fail here")
}

#[test]
fn debug_setblock_is_inert_without_the_flag() {
    let (mut managed, _world) = spawn("setblock-off", false);
    let runtime = current_thread_runtime();
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async {
        let (view, _observer) =
            connect_and_observe("127.0.0.1", managed.addr.port(), BOT_NAME, LOGIN_TIMEOUT)
                .await
                .expect("bot should connect and reach Event::Spawn");

        let before = wait_for_state(&view, SKY_POS, Duration::from_secs(10)).await;
        assert_eq!(
            before,
            Some(0),
            "expected air at the sky test cell before any debug hook"
        );

        managed.send_stdin_line(&format!(
            "debug-setblock {} {} {} {STONE_STATE_ID}",
            SKY_POS.0, SKY_POS.1, SKY_POS.2
        ));
        tokio::time::sleep(SETTLE).await;

        assert_eq!(
            view.state_id_at(SKY_POS),
            Some(0),
            "debug-setblock must be completely inert without --debug-hooks"
        );
    });
    drop(managed);
}

#[test]
fn debug_setblock_applies_the_state_when_the_flag_is_set() {
    let (mut managed, _world) = spawn("setblock-on", true);
    let runtime = current_thread_runtime();
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async {
        let (view, _observer) =
            connect_and_observe("127.0.0.1", managed.addr.port(), BOT_NAME, LOGIN_TIMEOUT)
                .await
                .expect("bot should connect and reach Event::Spawn");

        let before = wait_for_state(&view, SKY_POS, Duration::from_secs(10)).await;
        assert_eq!(
            before,
            Some(0),
            "expected air at the sky test cell before any debug hook"
        );

        managed.send_stdin_line(&format!(
            "debug-setblock {} {} {} {STONE_STATE_ID}",
            SKY_POS.0, SKY_POS.1, SKY_POS.2
        ));

        // Polls for the exact expected value, not merely for any `Some` — the sky
        // cell already reported `Some(0)` (air) above, so this closes the "stale
        // first observation" race the same way `packet_capture.rs`'s own module doc
        // comment establishes for every other caller of `state_id_at`.
        let after =
            wait_for_state_matching(&view, SKY_POS, STONE_STATE_ID, Duration::from_secs(5)).await;
        assert_eq!(
            after,
            Some(STONE_STATE_ID),
            "debug-setblock must apply the requested state id when --debug-hooks is set"
        );
    });
    drop(managed);
}

async fn wait_for_state_matching(
    view: &BlockSnapshotView,
    pos: (i32, i32, i32),
    expected: u32,
    timeout: Duration,
) -> Option<u32> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let observed = view.state_id_at(pos);
        if observed == Some(expected) {
            return observed;
        }
        if tokio::time::Instant::now() >= deadline {
            return observed;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[test]
fn debug_gamemode_is_inert_without_the_flag() {
    let (mut managed, _world) = spawn("gamemode-off", false);
    let runtime = current_thread_runtime();
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async {
        let (view, _observer) =
            connect_and_observe("127.0.0.1", managed.addr.port(), BOT_NAME, LOGIN_TIMEOUT)
                .await
                .expect("bot should connect and reach Event::Spawn");
        let client = view.client().expect("client available after Event::Spawn");
        let floor = discover_floor(&client, &view, LOGIN_TIMEOUT).await;

        // The first-ever joined player always gets network_entity_id 1
        // (`HardcodedWorld::alloc_network_entity_id`'s own monotonic-from-1 counter;
        // `play_block_break_place_full.rs` and every other in-process M3 test relies
        // on this identical convention for a single-bot session).
        managed.send_stdin_line("debug-gamemode 1 survival");
        tokio::time::sleep(SETTLE).await;

        instant_break(&client, floor);
        tokio::time::sleep(SETTLE).await;

        // A fresh join defaults to creative (`GameModeState { instabuild: true }`,
        // `world.rs`'s own established default) — with the flag off, the
        // `debug-gamemode` line above never lands, so the break above still runs at
        // creative (instant) speed and the floor cell is gone.
        assert_eq!(
            view.state_id_at(floor),
            Some(0),
            "debug-gamemode must be completely inert without --debug-hooks — the break \
             should still have completed at creative (instant) speed"
        );
    });
    drop(managed);
}

#[test]
fn debug_gamemode_switches_survival_when_the_flag_is_set() {
    let (mut managed, _world) = spawn("gamemode-on", true);
    let runtime = current_thread_runtime();
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async {
        let (view, _observer) =
            connect_and_observe("127.0.0.1", managed.addr.port(), BOT_NAME, LOGIN_TIMEOUT)
                .await
                .expect("bot should connect and reach Event::Spawn");
        let client = view.client().expect("client available after Event::Spawn");
        let floor = discover_floor(&client, &view, LOGIN_TIMEOUT).await;
        let original_state = view
            .state_id_at(floor)
            .expect("floor cell already observed by discover_floor");

        managed.send_stdin_line("debug-gamemode 1 survival");
        tokio::time::sleep(SETTLE).await;

        instant_break(&client, floor);
        tokio::time::sleep(SETTLE).await;

        // A bare-hand break on a real block in genuine survival mode does not
        // complete within this short window (blueprint §5's own "creative-speed
        // break completing... is the observable signal a gamemode switch never
        // landed", read in the negative: still standing IS the positive signal the
        // switch landed).
        assert_eq!(
            view.state_id_at(floor),
            Some(original_state),
            "debug-gamemode must switch the player to survival when --debug-hooks is \
             set — the break should not have completed within the settle window"
        );
    });
    drop(managed);
}
