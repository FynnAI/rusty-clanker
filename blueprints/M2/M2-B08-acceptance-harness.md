# M2-B08 — Acceptance Harness: Restart Round-Trip, 10,000-Chunk Soak, Save-Interval Cadence

| Field | Content |
|---|---|
| ID | M2-B08 |
| Milestone | M2 — Persistent World Storage |
| Prerequisites | M2-B01 (chunk representation — `PalettedContainer`, `BlockStateColumn`, `ChunkPersistenceState`, all reused unmodified; already merged). M2-B02 (`rc-nbt`'s `io`/`schema` layer — `read_borrowed`/`read_gzip_owned`, `FromNbtCompound`/`ToNbtCompound`/`NbtCompoundExt`, all reused unmodified; already merged). M2-B03 (assumed: Anvil region-file backend — `AnvilDiskBackend` implementing WORLD-D17's `ChunkStorageBackend` trait, plus a small write/read round-trip primitive this blueprint drives for the soak leg). M2-B04 (assumed: `level.dat` schema and the `ChunkColumn` NBT-bundling type WORLD-D11 names, `ChunkColumn::to_nbt`/`from_nbt`). M2-B05 (assumed: Stage-9 snapshot handoff wired to `RC-IoPool`, WORLD-D23's save pipeline, and its own configurable per-region save-interval knob firing off the tick thread, plus its CLI/diagnostic exposure). M2-B06 (assumed: player-data persistence — position/inventory/health via `rc-nbt`, WORLD-D15/`rc-mechanics`). M2-B07 (assumed: the minimal place/break protocol path `11-roadmap-milestones.md`'s M2 Scope commits to — "M2 implements only the minimal place/break path acceptance criterion 1 needs"). Also M1-B06 (the established harness architecture — `rc-test-harness`'s `process`/`fake_server` modules and `rc-paritybot`'s scenario-module pattern — extended here, never reinvented) and M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write_to, exit_code_for}` and `xtask::path_guard::{PROTECTED_PATHS, ChangesetType, glob_match, check_paths}`, both reused unmodified). **Derivation-time note on M2-B03/B04/B05/B06/B07:** none of these five blueprints exist as committed files at the time this blueprint was derived (verified by directory listing immediately before writing this document). This blueprint therefore fixes, precisely and narrowly, the external contract it needs from each of them — restated field-by-field in Context's "Assumed prerequisite API surface" subsection, in exactly the spirit M1-B06 already established for `rusty-clanker-server`'s not-yet-built `main.rs` CLI surface ("either already true by the time this blueprint is implemented or is this blueprint's own small, explicitly-scoped addition if not"). Where a since-merged B03/B04/B05/B06/B07 differs from an assumption below, that merged blueprint's actual shape is authoritative and this blueprint's integration points need the same small, scoped adaptation M1-B06's own precedent already accepts as normal — never a full rewrite, since every assumption below is deliberately narrow (one trait, one struct, one CLI flag at a time). |
| Implements | `11-roadmap-milestones.md`'s M2 Acceptance Criteria 1–3, verbatim, mapped 1:1 onto this blueprint's report cases (Context restates them exactly). TEST-D7 (differential-harness architecture, narrowed exactly as M1-B06 already narrowed it — no second server to diff against; M2's own content is still the superflat filler, per M2's own Boundaries). TEST-D10 (world-state content-hash canonicalization, applied here as a direct byte/value comparison rather than a hash, since the comparison set is small and precise mismatches are more useful than a hash collapse for this specific acceptance harness). TEST-D26 item (3)'s own named target — the Anvil round-trip property this blueprint's soak leg exercises operationally, at scale, distinct from TEST-D27's structural round-trip proptest (a future M2-B03 concern, not this blueprint's). TEST-D37/D40 (CI-tier placement and machine-readable JSON output, restated concretely below). TEST-D45/D46 (test-first changeset boundary, restated). WORLD-D16 (DataVersion 4903 assertion on every decoded artifact). WORLD-D23 (the save-interval knob this blueprint measures). PLAN-D5 (this blueprint is the mechanism that measures M2's own acceptance criteria, exactly as M1-B06 was for M1). |
| Crates touched | `crates/testing/test-harness/` (`rc-test-harness`, extended: `chunk_soak.rs`, `save_cadence.rs`, `fixtures/corrupting_backend.rs`, `process.rs` modified). `crates/testing/paritybot/` (`rc-paritybot`, extended: `restart_persistence.rs`). `xtask` (extended: `m2_report.rs`, `path_guard.rs`'s `PROTECTED_PATHS`, `main.rs`'s `Command` enum). `.github/workflows/ci.yml` (extended: one new nightly/manual job, `m2-acceptance`, plus the soak test's existing coverage under the already-existing `gates` job's `test` step — no new Tier-1 job needed). |
| Estimated scope | L |

## Goal & Done definition

Give M2 the same kind of real, agent-executable, per-criterion measurement M1-B06 gave M1: (1) a restart round-trip check — a real azalea bot (reusing M1-B06's `rc-paritybot` crate) places and breaks a defined block pattern against a real, freshly-spawned `rusty-clanker-server` subprocess (inventory editing is not exercised — an explicit, documented M2-scope gap, Context), the server is shut down cleanly and respawned pointed at the same world directory, the bot rejoins, and both a direct on-disk (region-file + player-data) comparison and an independent in-game (live-protocol) observation confirm every touched value survived byte-identical; (2) a 10,000-chunk write/read soak against a real `AnvilDiskBackend`, with deterministic, seed-logged pseudo-random chunk content covering every `PalettedContainer` strategy, asserting zero checksum mismatches; (3) a save-interval cadence measurement, using a small diagnostic CLI surface this blueprint adds to `rusty-clanker-server`, confirming every autosave fires within ±1 tick of its configured interval, in both a fast smoke variant and the literal 30-real-minute variant M2's own acceptance criterion states. Every leg's own analysis/comparison code is proven correct against deliberately-broken fakes before it is ever trusted against a real server or real disk.

Done when:

- [ ] `cargo build -p rc-test-harness -p rc-paritybot -p xtask --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset (all against fakes/fixtures — no real `rusty-clanker-server` build, no real `AnvilDiskBackend` I/O beyond a tempdir, required) passes under `cargo nextest run -p rc-test-harness -p rc-paritybot`.
- [ ] The real 10,000-chunk soak test (`crates/testing/test-harness/tests/chunk_soak_10000.rs`) passes under `cargo nextest run -p rc-test-harness`, reporting zero checksum mismatches, within its own stated wall-clock budget (Context).
- [ ] `cargo run -p xtask -- path-guard` still exits 0 against this blueprint's own governance changeset (labeled accordingly — Constraints).
- [ ] `cargo run -p xtask -- m2-report --help` prints usage with zero panics; a full `m2-report` run against a real `rusty-clanker-server` is **not** required for this blueprint's own Tier-1 Done state (mirrors M1-B06's identical precedent — see Context, "What this blueprint's own CI gate proves vs. what M2's nightly job proves").
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`, `lint-tests`, `verify-fixtures`) green on both `ubuntu-24.04` and `windows-2025`, on a clean checkout (TEST-D50) — the soak test runs here, inside the ordinary `test` step, via `cargo nextest run --workspace`. The new nightly `m2-acceptance` job is **not** part of this blueprint's own Tier-1 Done gate — its own first green run, once M2-B03 through M2-B07 have also landed, is what closes M2 itself (PLAN-D5), not this blueprint.

## Context (self-contained)

### M2 Acceptance Criteria, restated verbatim from `11-roadmap-milestones.md`

1. "A player places and breaks blocks, logs off, the server process restarts cleanly, the player rejoins: every block change and inventory item is present and byte-identical in block/item state to what was there before restart."
2. "An automated soak test performs 10,000 synthetic chunk write/read round trips with zero checksum mismatches."
3. "The configured save interval is measured, over a 30-minute run, to fire within ±1 tick of its configured cadence — this is the knob `13-cluster-architecture.md`'s CLUSTER-D17 will later override tighter for cluster mode, so its correctness here is a direct prerequisite for `M7`."

M2's own Boundaries section (restated): real worldgen is `M5` — M2's world content is the existing superflat filler, now actually persisted (M1-B05's placeholder layers, unchanged in content, now round-tripped through real storage). Full block mechanics are `M3` — M2 implements only the minimal place/break path AC1 needs. No manual/human-account step exists anywhere in M2 (unlike M1's AC3) — every leg of this blueprint is fully agent-executable end to end, so no `docs/MANUAL-VERIFICATION-M2.md` is produced by this blueprint.

### Relationship to M1-B06's harness architecture — reused, not reinvented

`rc_test_harness::process::{find_free_port, ManagedServerConfig, ManagedServer, spawn_server, SpawnError}` (M1-B06) is reused for spawning `rusty-clanker-server` subprocesses **twice** in this blueprint's own restart scenario — once before, once after the "restart." `rc_test_harness::fake_server` is **not** touched or extended by this blueprint: unlike M1-B06's own scenarios, a restart-round-trip check is fundamentally about real, on-disk persistence surviving a real process exit and re-launch — a fake in-process server that never touches disk cannot meaningfully stand in for the thing under test, so this blueprint's Tier-1 self-tests instead prove the harness's own *comparison/analysis* functions correct against hand-crafted fixtures (Acceptance tests, below), exactly mirroring what the task itself asks for ("a deliberately-corrupting storage fake must be caught by the checksum leg; a deliberately-late save timer must fail the cadence leg") — never a fake two-phase server. `rc_paritybot`'s existing `idle_stability` module and its `ScenarioConfig`/azalea-integration pattern (`ClientBuilder`, `Account::offline`, `Event::{Login, Spawn, Disconnect}`, the `tokio::time::timeout`-wrapping discipline around `start()`'s infinite-retry behavior) is the direct template this blueprint's new `restart_persistence` module follows — a second scenario module in the same crate, not a rewrite.

### CLI/diagnostic surface this blueprint assumes on `rusty-clanker-server` — extending M1-B06's `--bind`/`--offline` contract

M1-B06 already fixed `rusty-clanker-server --bind <ip:port> [--offline]` as the binary's external contract. This blueprint adds three more flags to that same contract, restated precisely so a since-merged M2-B05/B07 can verify or add them as a small, scoped touch if not already exactly this shape (identical discipline to M1-B06's own):

```
rusty-clanker-server --bind <ip:port> --offline --world-dir <path> [--save-interval-ticks <n>] [--save-event-log <path>]
```

- `--world-dir <path>`: the on-disk world save root (WORLD-D14's folder layout) this server instance reads from and writes to. **Required** for every invocation this blueprint makes (unlike M1, where no chunk persisted at all) — omitted, the binary's own default is never exercised by this blueprint, exactly matching M1-B06's identical framing for `--bind`. Passing the **same** `--world-dir` value across two separate `spawn_server` calls is the literal mechanism by which this blueprint's "restart" is real: the second process reads exactly what the first process wrote.
- `--save-interval-ticks <n>`: overrides WORLD-D23's own default autosave-interval knob (6000 ticks) for this process's lifetime — this blueprint always passes an explicit value (short for the smoke leg, the literal default-or-explicit value under real-time test for the full leg), never relying on the compiled-in default remaining exactly 6000 without this override existing.
- `--save-event-log <path>`: appends one newline-delimited JSON record to `path` **every time this process completes a region save** (i.e., every time WORLD-D23's Stage-9-triggered, `RC-IoPool`-executed write actually lands on disk) — `{"tick": u64, "region_id": string, "elapsed_ms": u64}`, `tick` being the tick count at which Stage 9 took the snapshot that led to this write (not the write's own completion tick — WORLD-D23's own snapshot-vs-async-write distinction), `elapsed_ms` the wall-clock milliseconds since process start. Omitted, no diagnostic log is written (harmless for the restart-round-trip leg, which does not need it). This is this blueprint's own small addition purely for measurability — it changes nothing about WORLD-D23's actual save *behavior*, only adds an observable record of when saves already-specified-to-happen actually happened, at the exact granularity ("±1 tick") AC3 needs, sidestepping filesystem-mtime polling's platform-dependent resolution noise entirely.

### Assumed prerequisite API surface

**From M2-B03 — `ChunkStorageBackend` and the round-trip soak primitive.** The trait itself is **not** an assumption — it is already committed, verbatim, by `03-world-chunks-persistence.md` itself (a planning document, not a blueprint under derivation):

```rust
// rc_chunk_storage — already fixed by 03-world-chunks-persistence.md, WORLD-D17
pub enum RegionFileKind { Terrain, Entities, Poi }

pub trait ChunkStorageBackend: Send + Sync + 'static {
    fn read_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, epoch: Option<u64>)
        -> Result<Option<Vec<u8>>, StorageError>;
    fn write_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, payload: &[u8], epoch: Option<u64>)
        -> Result<(), StorageError>;
    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError>;
    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>;
}
```

`StorageError` is treated as fully opaque by this blueprint — never matched on a specific variant, only `.is_err()`/`Display`, so this blueprint is insulated from M2-B03's own exact error taxonomy. `AnvilDiskBackend::new(world_dir: &std::path::Path) -> Self` (or an equivalent constructor taking the world root) is assumed to exist per WORLD-D12/D17's own text — this blueprint's soak test constructs one directly against a tempdir, no server process involved.

This blueprint additionally assumes M2-B03 exposes one small, generic round-trip primitive — a natural, minimal addition for the crate that owns both `ChunkStorageBackend` and its concrete `AnvilDiskBackend`:

```rust
// rc_chunk_storage — assumed addition, M2-B03
pub struct RoundTripOutcome {
    pub bytes_written: usize,
    /// `true` iff the bytes read back via `read_chunk` are byte-identical to the bytes
    /// just passed to `write_chunk` (a raw checksum comparison at the compressed-payload
    /// level — this primitive does not decode NBT at all, matching "soak primitive"'s own
    /// minimal, storage-layer-only scope).
    pub round_trip_identical: bool,
}

/// Writes `payload` via `backend.write_chunk(..)`, immediately reads it back via
/// `backend.read_chunk(..)`, and reports whether the two byte sequences match exactly.
/// `epoch: None` for every call this blueprint makes (monolithic-mode `AnvilDiskBackend`
/// never fences on epoch).
pub fn round_trip_write_read(
    backend: &dyn ChunkStorageBackend,
    dim: rc_core::DimensionId,
    kind: RegionFileKind,
    x: i32,
    z: i32,
    payload: &[u8],
) -> Result<RoundTripOutcome, StorageError>;
```

If M2-B03 does not name this function exactly this way, this blueprint's own `chunk_soak.rs` (Deliverables, below) may instead call `write_chunk` then `read_chunk` directly and perform the byte comparison itself inline — a purely mechanical substitution the implementer performs at that point, changing no test assertion in this blueprint's own Acceptance tests (which assert on `chunk_soak`'s own `SoakReport`, never on `round_trip_write_read` directly).

**From M2-B04 — `ChunkColumn`'s NBT bundling and its block-state accessor.** `03-world-chunks-persistence.md`'s own text already fixes `ChunkColumn::to_nbt(&self) -> simdnbt::owned::NbtCompound` / `ChunkColumn::from_nbt(tag: simdnbt::borrow::NbtCompound<'_>) -> Result<Self, ChunkNbtError>` (WORLD-D11). This blueprint additionally assumes `ChunkColumn` exposes the `BlockStateColumn` it bundles (M2-B01's already-committed type):

```rust
// rc_chunk_storage — WORLD-D11's own named type, this blueprint assumes one more accessor
impl ChunkColumn {
    pub fn block_states(&self) -> &rc_chunk_storage::BlockStateColumn;
}
```

— letting this blueprint call the already-committed `BlockStateColumn::get(x, world_y, z) -> BlockStateId` (M2-B01) on a freshly-decoded `ChunkColumn` to read back one specific block's value, without needing to know or reimplement the on-disk NBT section/palette tag-name schema itself.

**From M2-B05/M2-B06 — player-data persistence.** Assumed to live at `rc_mechanics::player_persistence` (`rc-mechanics`, the crate `12-workspace-structure.md`'s own manifest names as the eventual owner of player/entity state — this blueprint's own necessary early forward-reference, per WS-D2's precedent of a placeholder living ahead of its full owning milestone, e.g. M2-B01's own `ChunkGenStatus::Generating` placeholder ahead of `04`), built directly on M2-B02's already-committed `ToNbtCompound`/`FromNbtCompound` traits:

```rust
// rc_mechanics::player_persistence — assumed, M2-B06
#[derive(Debug, Clone, PartialEq)]
pub struct InventorySlotRecord {
    pub slot: i16,
    /// `None` = empty slot.
    pub item: Option<ItemStackRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemStackRecord { pub id: String, pub count: i32 }

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDataRecord {
    pub uuid: [u8; 16],
    pub x: f64, pub y: f64, pub z: f64,
    pub health: f32,
    pub inventory: Vec<InventorySlotRecord>,   // only occupied slots need appear
}

impl rc_nbt::ToNbtCompound for PlayerDataRecord { fn to_nbt_compound(&self) -> rc_nbt::owned::NbtCompound; }
impl rc_nbt::FromNbtCompound for PlayerDataRecord {
    fn from_nbt_compound<'a, 'tape>(compound: &rc_nbt::borrow::NbtCompound<'a, 'tape>, path: &rc_nbt::NbtPath)
        -> Result<Self, rc_nbt::SchemaError>;
}

/// The player-data-file counterpart to `ChunkStorageBackend` (WORLD-D17's own sibling
/// shape for a second on-disk artifact kind), assumed to live alongside `PlayerDataRecord`.
pub trait PlayerDataStore: Send + Sync + 'static {
    fn read_player_data(&self, uuid: [u8; 16]) -> std::io::Result<Option<Vec<u8>>>;
    fn write_player_data(&self, uuid: [u8; 16], payload: &[u8]) -> std::io::Result<()>;
}
```

This blueprint's own comparison code (`restart_persistence::compare_player_data`, Deliverables) is written against `PlayerDataRecord`'s four semantic fields (`x`/`y`/`z`, `health`, `inventory`) rather than against raw NBT bytes for this leg — a deliberate choice distinct from the block-state leg's raw-bytes-plus-decode approach, since WORLD-D15's own text records that vanilla's `level.dat` (and, by the same convention, player data) round-trips through a whole-tree, not partial/streaming, decode — comparing decoded semantic values is therefore both simpler and exactly as strong a check as comparing bytes would be, given the crate under test (M2-B02, already merged) already guarantees lossless byte-for-byte NBT round-tripping independently (M2-B02's own `roundtrip_proptest.rs`).

**Minimal place/break protocol path (M2-B07's own scope commitment).** `11-roadmap-milestones.md`'s own M2 Boundaries text commits another blueprint to building exactly this; the real M2-B07 (`Minimal Block Interaction: Place/Break, Reach Validation, Broadcast`) is that blueprint — not M2-B05, which owns Stage-9/`RC-IoPool`/the save-interval knob instead (corrected attribution, this blueprint's own Prerequisites field above). This blueprint assumes the server accepts, in Play state, Serverbound `Player Action` (`status = 0` start-digging treated as an immediate, unconditional creative-mode break for M2's minimal path — no mining-progress timing is required at M2, matching "minimal... path" framing) and Serverbound `Use Item On` — both well-known, stable vanilla packets whose exact field layout is **not** re-derived here (verify against `reports/packets.json`/`docs/research/mc-26.2/02-network-protocol.md` at implementation time, mirroring M1-B06's own identical "best-effort... `packets.json` is authoritative" hedge for exactly this class of not-yet-independently-verified packet). **`Use Item On`'s real M2-B07 behavior places a single, fixed `minecraft:stone` block on every successful placement, independent of which hand or hotbar slot a real client had selected** — no `ItemStack`/inventory model exists anywhere in M2 (MECH-D47 is M3/M4 scope), so "place using the client's currently-held hotbar item" is not something this milestone's server can do; this blueprint's own script (below) is written against that real, single-block-type behavior. This blueprint's bot driver issues these two packet kinds via **azalea's own high-level client API** (a real client library, TEST-D8) rather than hand-encoding them — mirroring M1-B06's own "the bot plays the client role, trust a real client library's encoder" precedent exactly (unlike `fake_server.rs`, which plays the *server* role and therefore must hand-encode); azalea's exact current method names for block interaction are **verified against azalea's own live documentation at implementation time** (identical verification discipline to M1-B06's own azalea research, restated as this blueprint's obligation rather than re-derived speculatively here).

**Inventory editing is explicitly out of M2's scope — no blueprint implements Serverbound `Set Creative Mode Slot`.** No M2 blueprint (M2-B05, M2-B06, or M2-B07) implements a handler for that packet: M2-B06 provides only a test-only backdoor (`PlayerSessionStore::with_record_mut`), explicitly documented there as a deliberate stand-in for the player's own action, noting that implementing that action itself is M2-B07's job; M2-B07's own Context explicitly states its "Inventory mutation stance at M2: none, in either direction," and its Constraints explicitly exclude any `ItemStack`/inventory mutation (MECH-D47, deferred to `M3`/`M4`). This blueprint's own restart-round-trip script (below) therefore performs **no** inventory edits and asserts no inventory content — a documented, explicit M2-scope gap, not a silently-scripted assertion against functionality that does not exist.

### Defined block pattern — exact, hand-specified

Every automated run of this blueprint's restart-round-trip leg uses **exactly** this script (dimension: overworld, `DimensionId::OVERWORLD`), relative to M1-B05's own established spawn point (`x=0.0, y=-59.0, z=0.0`). Every placement targets `minecraft:stone` — the one fixed block M2-B07's real `Use Item On` handler ever places (Context's corrected "Minimal place/break protocol path" subsection, above) — never a distinct block type per position, since M2 has no item-selection model to make a per-position choice meaningful:

| # | Action | Position | Value |
|---|---|---|---|
| 1 | Place | `BlockPos::new(2, -59, 0)` | `minecraft:stone` |
| 2 | Place | `BlockPos::new(2, -59, 1)` | `minecraft:stone` |
| 3 | Place | `BlockPos::new(3, -59, 0)` | `minecraft:stone` |
| 4 | Break | `BlockPos::new(0, -60, 0)` | → `minecraft:air` (whatever superflat filler block M1-B05's placeholder put there) |
| 5 | Break | `BlockPos::new(1, -60, 0)` | → `minecraft:air` |

No inventory-editing action is scripted (Context, above — no M2 blueprint implements `Set Creative Mode Slot`). Health is **not** actively modified (no damage-inducing mechanic exists at M2) — this blueprint's own health assertion is simply that whatever health value is observed immediately before disconnect (expected: full health, vanilla's `20.0f`, since nothing in this scenario can reduce it) is observed identically after restart+rejoin — a real, meaningful persistence check even though the value itself never changes, exactly as valid a regression guard as the block leg.

### Assertion methodology — two independent legs per artifact kind, restated precisely

**Direct on-disk comparison** (never through the live protocol): after the bot disconnects (post-actions 1–5) and the first server process is torn down cleanly (`ManagedServer`'s `Drop`, or an explicit graceful-shutdown request if M2-B05 adds one — this blueprint accepts either, since AC1's own wording is "restarts cleanly," not "restarts via a specific shutdown signal"), this blueprint opens a **fresh** `AnvilDiskBackend`/`PlayerDataStore` directly against the same `--world-dir` (library-level, no server process) and reads back: (a) for each of the 5 block positions, `ChunkStorageBackend::read_chunk` for that position's chunk, `ChunkColumn::from_nbt`, `.block_states().get(..)`, compared against the expected post-action `BlockStateId` (resolved via the generated registry's name-lookup, `M0-B07`'s own `crates/registries/generated/v776` output — exact lookup function verified at implementation time, mirroring M2-B01's own identical "registry API surface confirmed at implementation time" caveat); (b) `PlayerDataStore::read_player_data` + `PlayerDataRecord::from_nbt_compound`, compared field-by-field against the expected `x/y/z/health` — `inventory` is **not** compared here (Context, above: M2 performs no inventory mutation, so there is nothing this leg's inventory check could meaningfully distinguish from "server never touched it"). This leg runs **before** the second server process is spawned — proving persistence survived the shutdown independent of whether reload logic is also correct.

**In-game (live-protocol) observation**: the **second** `rusty-clanker-server` process (same `--world-dir`) is spawned, the bot reconnects, and this blueprint observes: (a) the `Level Chunk with Light` packet(s) covering the 5 test positions (decoded via `rc_protocol`, already-implemented per M1/M2's own protocol work — this blueprint reads the packet the server sends on chunk load exactly as a real client would, asserting the same 5 positions' block states via the wire-encoded `PalettedContainer`, an entirely independent decode path from the on-disk NBT leg above — a bug in "saved correctly but re-served incorrectly," or vice versa, is caught by exactly one of the two legs, never silently missed by both); (b) the player's observed health (vanilla's `Set Health`/equivalent, or read directly off azalea's own client-side player-state fields if azalea exposes health as ordinary client state, which is the expected, lower-friction path — verified at implementation time). No inventory-view packet is observed or asserted on (Context, above — there is no scripted inventory edit for it to confirm).

Both legs are required to pass independently — this blueprint's report (Deliverables) carries four separate case results specifically so a failure on only one leg is immediately diagnosable as a save-path bug vs. a load/re-serve-path bug, never collapsed into one pass/fail bit.

### Deterministic chunk-content generation for the 10,000-chunk soak

`chunk_soak::generate_chunk_payload(seed: u64, index: u32) -> Vec<u8>` produces one **already-NBT-encoded, already-compressed** chunk payload — exactly the byte shape `ChunkStorageBackend::write_chunk` expects — deterministically from `(seed, index)`, using a small in-crate PRNG (`rand_chacha`'s `ChaCha8Rng` — **not** newly pinned; if not already in `[workspace.dependencies]` by the time this blueprint is implemented, this blueprint's own governance changeset adds it as its one permitted addition, Constraints) seeded with `seed ^ (index as u64)`. Each generated chunk deliberately cycles its `BlockStateColumn`'s per-section palette strategy across all three `Palette<T>` states (WORLD-D2, M2-B01's own already-committed enum) so the soak corpus exercises every on-disk shape, not only one: `index % 3 == 0` → every section `SingleValue` (a uniform block, `index`-derived); `index % 3 == 1` → every section `Indirect` (the PRNG picks 2–200 distinct `BlockStateId`s per section, scattered across the section's 4096 entries); `index % 3 == 2` → at least one section forced `Direct` (the PRNG picks ≥257 distinct values in that section, forcing promotion past `PaletteThresholds::blocks(15)`'s 8-bit `Indirect` ceiling per M2-B01's own threshold rule). The chunk's `(x, z)` coordinates are `(index as i32, 0)` — spread across `index / 1024` distinct region files (1024 chunks/region, WORLD-D12's own 32×32 layout), so the soak also exercises multiple region files, not one pathologically large one. `seed` is logged into this test's own report (below) precisely so a failure is reproducible by re-running with the same logged seed — this blueprint's soak corpus is **never** a committed fixture (no `PROTECTED_PATHS` entry is added for it — Constraints), matching TEST-D47's own scope ("golden fixture... `rc-gametest` structure... worldgen seed-corpus entry" — a freshly-generated-every-run soak corpus is none of these three).

### CI tier placement

| Tier | What runs | Duration budget | Cadence |
|---|---|---|---|
| Tier 1 (PR-blocking, `gates`, unmodified job, extended coverage via `cargo nextest run --workspace`) | This blueprint's own self-tests (`chunk_soak` corruption/honesty/determinism fakes; `save_cadence` on-time/late-log analysis; `restart_persistence`'s pure comparison-function fixtures) — no real server, no real bot. **Plus** the real 10,000-chunk soak (`chunk_soak_10000.rs`) — a real `AnvilDiskBackend` against a tempdir, no server subprocess, so it fits the "no server process, no network" cheap-layer framing TEST-D5 already establishes for this kind of test | Self-tests: a few seconds total. Soak: budgeted ≤180s wall-clock on either OS leg (a seed default, same "calibrate once real hardware/implementation exists" status every other numeric threshold in this project's planning corpus already carries — Open Questions) | Every PR, both OS legs |
| Tier 2 (nightly, new `m2-acceptance` job) | `xtask m2-report --mode smoke`: one real restart-round-trip cycle (two real `rusty-clanker-server` subprocesses, real azalea bot) **plus** one save-cadence smoke run (`--save-interval-ticks 20`, real tick cadence, ~a few real seconds of wall time to observe several cycles — never an accelerated tick clock, mirroring M1-B06's own "compressed never means accelerated cadence" rule exactly, applied here to save cadence instead of keep-alive cadence) | A few minutes total | Nightly cron, both OS legs |
| Manual/on-demand (`workflow_dispatch` input `mode: full`, same job) | `xtask m2-report --mode full`: the same restart-round-trip cycle, **plus** the literal AC3 threshold — a 30-real-minute run at whatever `--save-interval-ticks` value is under test (default: 6000, WORLD-D23's own default, unless a shorter operator-relevant value is deliberately chosen — this blueprint's own default choice for the `full` mode is `1200` ticks/60s, short enough to observe ~30 real save cycles in 30 minutes while still being a "real," non-trivial interval; the literal default 6000-tick/5-minute interval is also exercisable via an explicit `--save-interval-ticks 6000` CLI override on the same `full` mode, left as an operator-invoked variant, not this blueprint's own default, since 30 minutes at a 5-minute cadence only yields 6 samples — statistically thin for a tight ±1-tick claim compared to `1200`'s ~30) | 30 real minutes | Triggered deliberately once a maintainer believes M2 is complete — mirrors M1-B06's identical "nightly signal, manual gate" split |

**What this blueprint's own CI gate proves vs. what M2's nightly job proves.** Identical framing to M1-B06's own precedent: this blueprint's Tier-1 Done state is satisfied entirely by its self-tests plus the real (but server-process-free) 10,000-chunk soak — it proves the harness's comparison/generation logic is correct and that `AnvilDiskBackend` itself round-trips at scale. It does **not** require a green `m2-acceptance` run, because that job's first meaningful green result can only happen once M2-B03 through M2-B07 (this blueprint's own prerequisites) are also merged — this blueprint's implementer cannot single-handedly guarantee that job passes, and gating this blueprint's own Done state on a job it cannot single-handedly control would repeat the exact design mistake M1-B06 already flagged and avoided.

## Deliverables

### `crates/testing/test-harness/src/lib.rs` (modify — two new `pub mod` lines)

```rust
pub mod chunk_soak;
pub mod save_cadence;
```

(`fake_server`, `probe`, `process` unchanged in module list — `process` itself gains new fields, below.)

### `crates/testing/test-harness/src/process.rs` (modify — extend `ManagedServerConfig`, additive only)

```rust
pub struct ManagedServerConfig {
    pub binary_path: PathBuf,
    pub offline: bool,
    pub startup_timeout: Duration,
    pub extra_args: Vec<String>,
    /// New (M2-B08): passed as `--world-dir <path>`. `None` is a programmer error for
    /// every call site this blueprint adds (Context: required for every M2-B08
    /// invocation) — `spawn_server` returns `SpawnError::MissingWorldDir` immediately,
    /// before ever spawning a process, when `None` is passed by an M2-B08 call site
    /// (existing M1-B06 call sites, which never set this field, get `None`'s prior
    /// behavior unchanged: no `--world-dir` flag emitted at all, `Option` staying
    /// backward-compatible via `#[derive(Default)]` on this struct, added by this
    /// blueprint alongside the two new fields).
    pub world_dir: Option<PathBuf>,
    /// New (M2-B08): passed as `--save-interval-ticks <n>` when `Some`.
    pub save_interval_ticks: Option<u64>,
    /// New (M2-B08): passed as `--save-event-log <path>` when `Some`.
    pub save_event_log: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("failed to reserve a free port: {0}")]
    PortReservation(io::Error),
    #[error("failed to spawn {path}: {source}")]
    Spawn { path: String, source: io::Error },
    #[error("server did not accept a connection on {addr} within {elapsed:?}")]
    StartupTimeout { addr: SocketAddr, elapsed: Duration },
    /// New (M2-B08): a `ManagedServerConfig::world_dir == None` reached a call site that
    /// requires it — this blueprint's own `restart_persistence`/`m2_report` call sites
    /// always set it; this variant exists to fail fast and clearly rather than silently
    /// exercising the binary's untested default world path.
    #[error("ManagedServerConfig::world_dir is required by this call site but was None")]
    MissingWorldDir,
}
```

`spawn_server`'s own body (M1-B06's, unmodified in shape) additionally appends `["--world-dir", <path>]` / `["--save-interval-ticks", <n>]` / `["--save-event-log", <path>]` to the child's argument list whenever each `Option` is `Some`, per Context's CLI-surface restatement — the polling-for-readiness loop and `Drop` behavior are untouched.

### `crates/testing/test-harness/src/chunk_soak.rs` (new)

```rust
use rc_chunk_storage::{ChunkStorageBackend, RegionFileKind};
use std::path::Path;

/// One soak trial's outcome.
#[derive(Debug, Clone)]
pub struct SoakCaseOutcome {
    pub index: u32,
    pub palette_shape: PaletteShape,
    pub round_trip_identical: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteShape { SingleValue, Indirect, Direct }

#[derive(Debug, Clone)]
pub struct SoakReport {
    pub seed: u64,
    pub total: u32,
    pub mismatches: Vec<SoakCaseOutcome>,   // only non-identical/errored cases
}

impl SoakReport {
    pub fn zero_mismatches(&self) -> bool { self.mismatches.is_empty() }
}

/// Deterministically generates one already-NBT-encoded, already-compressed chunk
/// payload from `(seed, index)` (Context's exact palette-cycling rule). Pure, no I/O.
pub fn generate_chunk_payload(seed: u64, index: u32) -> Vec<u8>;

/// Which of the three `Palette<T>` shapes `generate_chunk_payload` targeted for `index`
/// (`index % 3`, Context) — exposed so a caller/report can attribute a mismatch to a
/// specific palette shape rather than only an opaque index.
pub fn palette_shape_for(index: u32) -> PaletteShape;

/// Runs `count` soak trials against `backend` (constructed by the caller — this
/// blueprint's own soak test constructs a real `AnvilDiskBackend` over a tempdir; a
/// self-test constructs a fake, Acceptance tests below), each trial: generate payload
/// via `generate_chunk_payload(seed, i)`, round-trip it (via `rc_chunk_storage`'s
/// assumed `round_trip_write_read`, Context — or the inline write-then-read-then-
/// compare fallback if that exact function is not present, a mechanical substitution
/// that does not change this function's own signature or `SoakReport` shape), record a
/// `SoakCaseOutcome` only when `round_trip_identical == false` or the round trip
/// errored. `dim` is always `rc_core::DimensionId::OVERWORLD`; `kind` is always
/// `RegionFileKind::Terrain` (block-state content is what varies — Terrain is the one
/// `RegionFileKind` `PalettedContainer`-shaped content actually lives in, WORLD-D29).
pub fn run_soak(backend: &dyn ChunkStorageBackend, seed: u64, count: u32) -> SoakReport;
```

### `crates/testing/test-harness/src/fixtures/mod.rs`, `crates/testing/test-harness/src/fixtures/corrupting_backend.rs` (new)

```rust
// fixtures/mod.rs
pub mod corrupting_backend;
```

```rust
// fixtures/corrupting_backend.rs
use rc_chunk_storage::{ChunkStorageBackend, RegionFileKind, StorageError};
use std::sync::Mutex;
use std::collections::HashMap;

/// An in-memory, honest `ChunkStorageBackend` (a `HashMap`-backed store — no real disk
/// I/O). `write_chunk` stores exactly the bytes given; `read_chunk` returns exactly what
/// was last written for that key, `None` if never written.
#[derive(Default)]
pub struct InMemoryHonestBackend { store: Mutex<HashMap<(u16, u8, i32, i32), Vec<u8>>> }
impl InMemoryHonestBackend { pub fn new() -> Self; }
impl ChunkStorageBackend for InMemoryHonestBackend { /* as described above */ }

/// Wraps an `InMemoryHonestBackend`, but `read_chunk` flips the last byte of whatever
/// was stored before returning it — a deliberate, minimal, always-reproducible
/// corruption, used only by this blueprint's own self-test proving `run_soak` actually
/// catches a corrupted round trip rather than trivially always reporting success.
#[derive(Default)]
pub struct CorruptingBackend { inner: InMemoryHonestBackend }
impl CorruptingBackend { pub fn new() -> Self; }
impl ChunkStorageBackend for CorruptingBackend { /* write delegates to inner; read
    delegates to inner then flips the last byte of a non-empty result before returning */ }
```

(If M2-B03's actual `StorageError` type is not yet visible to `rc-test-harness` at the point this blueprint is implemented — e.g. M2-B03 has not yet merged — this fixture module's own `impl ChunkStorageBackend` block is written against whatever `StorageError` shape M2-B03 actually ships, per Context's "fully opaque" framing; no test in this blueprint ever constructs or matches a specific `StorageError` variant, only propagates `Ok`/`Err`.)

### `crates/testing/test-harness/src/save_cadence.rs` (new)

```rust
use std::path::Path;

/// One parsed line of a `--save-event-log` file (Context).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SaveEvent { pub tick: u64, pub region_id: String, pub elapsed_ms: u64 }

#[derive(Debug, Clone)]
pub struct CadenceViolation {
    /// Index into the parsed event sequence of the *later* of the two events whose
    /// gap violated tolerance.
    pub at_index: usize,
    pub expected_interval_ticks: u64,
    pub actual_interval_ticks: i64,   // signed: can be short or long
}

#[derive(Debug, Clone)]
pub struct CadenceReport {
    pub event_count: usize,
    pub violations: Vec<CadenceViolation>,
}
impl CadenceReport { pub fn within_tolerance(&self) -> bool { self.violations.is_empty() } }

/// Parses `path` as newline-delimited JSON `SaveEvent` records (one per line, per
/// Context's `--save-event-log` format). Malformed/empty lines are skipped, never a
/// hard error (a partially-flushed log at the moment of reading is expected, not
/// exceptional — this function is designed to be called against a log file that may
/// still be being appended to by a live server process).
pub fn parse_save_event_log(path: &Path) -> std::io::Result<Vec<SaveEvent>>;

/// Pure analysis (Acceptance tests exercise this directly against hand-crafted
/// `Vec<SaveEvent>`, no file I/O): for every consecutive pair of events **restricted to
/// the same `region_id`** (a log may interleave multiple regions' events), computes
/// `actual = events[i].tick as i64 - events[i-1].tick as i64` and records a
/// `CadenceViolation` whenever `(actual - expected_interval_ticks as i64).abs() > 1`
/// (AC3's own literal "±1 tick" tolerance). The very first event for a given
/// `region_id` never produces a violation (nothing precedes it to measure a gap
/// against).
pub fn analyze_cadence(events: &[SaveEvent], expected_interval_ticks: u64) -> CadenceReport;
```

### `crates/testing/paritybot/src/lib.rs` (modify — one new `pub mod` line)

```rust
pub mod restart_persistence;
```

### `crates/testing/paritybot/src/restart_persistence.rs` (new)

```rust
use std::time::Duration;

/// The 5-action script (Context's exact table), fixed — no per-call parameterization
/// needed by any test in this blueprint. No inventory-editing action exists in the
/// script (Context: no M2 blueprint implements `Set Creative Mode Slot`) — an explicit,
/// documented M2-scope gap, not an omission.
pub struct ActionScript;   // a unit type; `apply_actions` below is the only entry point,
                            // keeping the exact action list defined once, in Context and
                            // in this module's own implementation, never duplicated as
                            // caller-supplied data

#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedState {
    pub blocks: Vec<(rc_core::BlockPos, String)>,   // (position, expected block name);
                                                      // "minecraft:air" for the two
                                                      // broken positions, "minecraft:stone"
                                                      // for the three placed ones (M2-B07's
                                                      // single-fixed-block behavior, Context)
    pub health: f32,
}

/// The Context table's own expected end-state, as data — both the disk-comparison and
/// observation legs compare against this same fixed value.
pub fn expected_state() -> ExpectedState;

/// Connects (`azalea::Account::offline`), waits for `Event::Spawn` (bounded by
/// `login_timeout`, same infinite-retry-guarding discipline as `idle_stability`'s own
/// `run_idle_stability_scenario`), issues the 5 actions from Context's table via
/// azalea's own high-level block-interaction API (exact method names verified at
/// implementation time, Context), then performs a clean client-initiated disconnect.
/// Returns `Err` on any disconnect before the script completes, or a login timeout,
/// mirroring `idle_stability::ScenarioError`'s own shape for the two analogous failure
/// modes.
pub async fn apply_actions(host: &str, port: u16, username: &str, login_timeout: Duration)
    -> Result<(), ActionError>;

/// Connects, waits for `Event::Spawn`, then reads back the 5 test positions' block
/// state from the received `Level Chunk with Light` packet(s) and the player's own
/// observed health (Context's two observation targets — no inventory view is read,
/// Context's explicit M2-scope gap), returning them as an `ExpectedState`-shaped value
/// for direct comparison against `expected_state()`. Performs a clean disconnect
/// afterward.
pub async fn observe_state(host: &str, port: u16, username: &str, login_timeout: Duration)
    -> Result<ExpectedState, ActionError>;

#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("no Event::Login observed within the {0:?} login timeout")]
    LoginTimeout(Duration),
    #[error("disconnected before the script/observation completed: {reason:?}")]
    Disconnected { reason: Option<String> },
}

/// Pure comparison (Acceptance tests exercise this directly, no network/process
/// involved): every field of `actual` compared against `expected`; returns one
/// human-readable mismatch description per differing field (block position or health),
/// empty iff every field matches exactly.
pub fn compare_state(expected: &ExpectedState, actual: &ExpectedState) -> Vec<String>;
```

### `xtask/src/m2_report.rs` (new)

```rust
use crate::tier_result::TierResult;
use std::time::Duration;

#[derive(serde::Serialize)]
pub struct M2ReportResult {
    #[serde(flatten)]
    pub automated: TierResult,   // tier = "m2-acceptance"; cases named
                                  // "AC1a_block_state_disk_identical",
                                  // "AC1b_block_state_observed_identical",
                                  // "AC1c_player_position_health_disk_identical",
                                  // "AC1d_player_position_health_observed_identical",
                                  // "AC3_save_cadence_within_one_tick"
                                  // (no inventory case exists — M2 implements no
                                  // Set Creative Mode Slot handler anywhere, Context's
                                  // explicit, documented M2-scope gap)
    pub mode: String,             // "smoke" | "full"
    pub target: String,           // "<ip>:<port>" actually used (the second, post-restart spawn)
    pub save_interval_ticks_used: u64,
}

pub const OUT_PATH: &str = "target/verify/m2-acceptance.json";

/// CLI entry point (`xtask m2-report --server-bin <path> --mode {smoke|full}`):
/// reserves a fresh tempdir as `--world-dir`, resolves `save_interval_ticks`/
/// `cadence_run_duration` from `mode` (`smoke` → `20` ticks / a few real seconds;
/// `full` → `1200` ticks / 1800 real seconds, Context), runs the restart-round-trip
/// leg (spawn #1, `rc_paritybot::restart_persistence::apply_actions`, teardown #1,
/// direct-disk comparison via a fresh `AnvilDiskBackend`/`PlayerDataStore` against the
/// same `--world-dir`, spawn #2 with the same `--world-dir`,
/// `rc_paritybot::restart_persistence::observe_state`, teardown #2) producing cases
/// AC1a–AC1d, then the save-cadence leg (a **third**, separate spawn, fresh
/// `--world-dir`, `--save-interval-ticks`/`--save-event-log` set, held open for
/// `cadence_run_duration` real wall-clock time while some minimal periodic dirtying
/// keeps at least one region actually saving every cycle — Context's own place/break
/// path re-used here purely as a "keep this chunk dirty" mechanism, one extra
/// `apply_actions`-style single block toggle per cadence cycle is sufficient and is
/// this verb's own small addition, not a new blueprint-level scenario), parses the
/// resulting `--save-event-log` via `rc_test_harness::save_cadence`, producing case
/// AC3, inside one `tokio::runtime::Runtime::new()?.block_on(...)` (mirrors
/// `m1_report::run`'s identical isolation of the one async-touching verb). Writes and
/// returns per `M1ReportResult`'s own established pattern (`OUT_PATH`, `ExitCode` iff
/// `automated.status == Status::Pass`).
pub fn run(server_bin: std::path::PathBuf, mode: Mode) -> std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode { Smoke, Full }

impl Mode {
    /// `Smoke` → `(20, Duration::from_secs(30))`, `Full` → `(1200, Duration::from_secs(1800))`
    /// — `(save_interval_ticks, cadence_run_duration)`, Context's own exact pair.
    pub fn cadence_params(self) -> (u64, Duration);
}
```

### `xtask/src/path_guard.rs` (modify — no row correction needed; confirms coverage)

No new `ProtectedPath` row is added. Every file this blueprint's own governance changeset touches under `crates/testing/{test-harness,paritybot}/**` is already covered by the existing (M1-B06-corrected) rows `crates/testing/test-harness/**` and `crates/testing/paritybot/**`; `xtask/**` is already covered by the existing row #7. This blueprint introduces no committed fixture tree (Context — the soak corpus is generated fresh every run, never a file under version control), so no new `PROTECTED_PATHS` entry is warranted (TEST-D46's own scope is protecting *committed* artifacts).

### `xtask/src/main.rs` (modify — one new `Command` variant)

```rust
/// M2-B08: drives the M2 acceptance harness (restart round-trip + save-cadence legs)
/// against a real, freshly-spawned `rusty-clanker-server` and writes
/// `target/verify/m2-acceptance.json`.
M2Report {
    #[arg(long)]
    server_bin: std::path::PathBuf,
    #[arg(long, value_enum, default_value_t = m2_report::Mode::Smoke)]
    mode: m2_report::Mode,
},
```

One new `match` arm calling `m2_report::run(server_bin, mode)`.

### `.github/workflows/ci.yml` (modify — one new job appended; `gates`/`guardrails`/`soak`/`m1-acceptance` untouched)

```yaml
jobs:
  # ... existing gates/guardrails/soak/m1-acceptance jobs, byte-for-byte unchanged ...

  m2-acceptance:
    name: m2-acceptance (${{ matrix.os }})
    if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-24.04, windows-2025]
    steps:
      - uses: actions/checkout@v4
      - name: Install pinned toolchain (rust-toolchain.toml)
        run: rustup show
      - uses: Swatinem/rust-cache@v2
      - name: Build rusty-clanker-server (monolithic)
        run: cargo build --release -p rusty-clanker-server --no-default-features --features monolithic
      - name: m2-report
        shell: bash
        run: |
          MODE="${{ github.event_name == 'workflow_dispatch' && inputs.m2_report_mode || 'smoke' }}"
          cargo run -p xtask -- m2-report --server-bin target/release/rusty-clanker-server --mode "$MODE"
      - name: Upload m2-acceptance report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: m2-acceptance-${{ matrix.os }}
          path: target/verify/m2-acceptance.json
          if-no-files-found: warn
```

`workflow_dispatch.inputs` gains one new choice input, `m2_report_mode` (`[smoke, full]`, default `smoke`), added alongside M1-B06's existing `m1_report_mode` input in the same `on:` block (both inputs coexist — a maintainer triggers either milestone's full run independently).

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test-authoring changeset is every file under `crates/testing/test-harness/tests/` and `crates/testing/paritybot/tests/` listed below, plus every new `src/*.rs` module from Deliverables committed with every function body `todo!()`-stubbed (struct/enum shapes final). Per M1-B06's own established precedent (Context, "Relationship to M1-B06's harness architecture"), this test-authoring changeset is exempt from `path-guard`'s protected-path check regardless of `crates/testing/{test-harness,paritybot}/**` being fully protected, since `ChangesetType::TestAuthoring` and `ChangesetType::Governance` both bypass `check_paths` unconditionally (M0-B08). The governance changeset (Implementation steps, below; labeled `Changeset-Type: governance`, never `implementation` — Constraints) fills in real bodies only.

### `crates/testing/test-harness/tests/chunk_soak_self_tests.rs`

1. `honest_backend_reports_zero_mismatches` — `run_soak(&InMemoryHonestBackend::new(), seed: 42, count: 30)` → `.zero_mismatches() == true`, `.total == 30`.
2. `corrupting_backend_is_caught_by_the_checksum_leg` — `run_soak(&CorruptingBackend::new(), seed: 42, count: 30)` → `.zero_mismatches() == false`, `.mismatches.len() == 30` (every single trial corrupted, since `CorruptingBackend` corrupts unconditionally), and every entry's `round_trip_identical == false`.
3. `same_seed_generates_identical_payloads_across_two_calls` — `generate_chunk_payload(7, 3) == generate_chunk_payload(7, 3)` (byte-for-byte), proving determinism independent of any backend.
4. `different_indices_generate_different_payloads` — `generate_chunk_payload(7, 3) != generate_chunk_payload(7, 4)`.
5. `palette_shape_cycles_across_three_indices` — `palette_shape_for(0) == PaletteShape::SingleValue`, `palette_shape_for(1) == PaletteShape::Indirect`, `palette_shape_for(2) == PaletteShape::Direct`, `palette_shape_for(3) == PaletteShape::SingleValue` (the cycle repeats).

### `crates/testing/test-harness/tests/chunk_soak_10000.rs` (the real leg, Tier 1)

1. `soak_10000_chunks_zero_checksum_mismatches` — construct a real `rc_chunk_storage::AnvilDiskBackend` over a fresh `tempfile::tempdir()`; `run_soak(&backend, seed: 0xC0FFEE, count: 10_000)`; assert `.zero_mismatches() == true`; on failure, the test's own panic message includes the logged `seed` (`0xC0FFEE`) and every `SoakCaseOutcome` in `.mismatches` (index, palette shape, error if any) — never a bare "assertion failed," per TEST-D40's own "no tier's completion status may ever require reading prose to determine pass/fail" spirit applied to a single test's own diagnostic quality. Wall-clock budget: this test must complete within 180 real seconds (asserted via a `std::time::Instant` check inside the test itself, failing loudly and distinctly from a checksum mismatch if exceeded — Context's own stated Tier-1 budget).

### `crates/testing/test-harness/tests/save_cadence_self_tests.rs`

1. `on_time_events_report_no_violations` — a hand-built `Vec<SaveEvent>` for one `region_id`, ticks `[0, 1200, 2400, 3600, 4801]` (the last one exactly 1 tick late, still within tolerance); `analyze_cadence(&events, expected_interval_ticks: 1200)` → `.within_tolerance() == true`.
2. `late_save_timer_is_caught_by_the_cadence_leg` — ticks `[0, 1200, 2402]` (the second gap is `1202` ticks, `2` over tolerance); `analyze_cadence(&events, 1200)` → `.within_tolerance() == false`, exactly one `CadenceViolation` with `at_index == 2`, `expected_interval_ticks == 1200`, `actual_interval_ticks == 1202`.
3. `early_save_is_also_a_violation` — ticks `[0, 1197]` (gap `1197`, `3` under tolerance); one violation, `actual_interval_ticks == 1197`.
4. `multiple_regions_are_analyzed_independently` — events interleaving `region_id: "r1"` ticks `[0, 1200]` and `region_id: "r2"` ticks `[5, 1206]` (both individually on-time, `r2`'s absolute ticks merely offset from `r1`'s); `analyze_cadence(&events, 1200)` → `.within_tolerance() == true` (proves the analysis groups by `region_id` before computing gaps, never comparing across regions).
5. `single_event_per_region_produces_no_violation` — one event, one region; `.within_tolerance() == true`, `.violations.is_empty()`.
6. `parse_save_event_log_skips_malformed_lines` — a temp file with two valid JSON lines and one malformed line (`"not json"`) interleaved; `parse_save_event_log` returns exactly the 2 valid `SaveEvent`s, no error.

### `crates/testing/paritybot/tests/restart_persistence_self_tests.rs`

1. `matching_state_produces_no_mismatches` — `compare_state(&expected_state(), &expected_state())` (identical value both sides) → `vec![]`.
2. `wrong_block_state_is_reported` — `actual` built from `expected_state()` with one block entry's item string changed (e.g. position `(2,-59,0)` changed from `"minecraft:stone"` to `"minecraft:dirt"`); `compare_state(&expected_state(), &actual)` → exactly 1 entry, its text naming that position.
3. `wrong_health_is_reported` — `actual.health` changed from `20.0` to `19.0`; exactly 1 entry naming health.
4. `multiple_mismatches_are_all_reported_independently` — two block entries changed at once (e.g. positions `(2,-59,0)` and `(3,-59,0)`); exactly 2 entries, one per changed position — proves the comparison does not short-circuit on the first mismatch.

### `xtask/tests/m2_report_cli.rs`

1. `mode_cadence_params_smoke` — `Mode::Smoke.cadence_params() == (20, Duration::from_secs(30))`.
2. `mode_cadence_params_full` — `Mode::Full.cadence_params() == (1200, Duration::from_secs(1800))`.
3. `m2_report_result_serializes_with_flattened_tier_fields_and_new_fields` — build an `M2ReportResult` with a passing `TierResult` (`tier: "m2-acceptance"`), serialize to `serde_json::Value`, assert the top-level object has `tier`, `status`, `cases` (flattened) **and** `mode`, `target`, `save_interval_ticks_used` as sibling keys.
4. `path_guard_already_covers_m2_b08s_own_new_paths` — `path_guard::check_paths(ChangesetType::Implementation, &["crates/testing/test-harness/src/chunk_soak.rs".into(), "crates/testing/paritybot/src/restart_persistence.rs".into(), "xtask/src/m2_report.rs".into()])` → `assert_eq!(violations.len(), 3)` (all three paths already match an existing row — the two `crates/testing/**` rows and row #7's `xtask/**` — proving no gap exists for this blueprint's own new files without needing a table edit).

## Implementation steps

1. **`process.rs`.** Add the three new `ManagedServerConfig` fields (`#[derive(Default)]` added to the struct — a compatible, additive change to M1-B06's existing struct), the `MissingWorldDir` `SpawnError` variant, and `spawn_server`'s three new conditional `--world-dir`/`--save-interval-ticks`/`--save-event-log` argument pushes, checking `world_dir.is_none()` only at this blueprint's own new call sites (existing M1-B06 call sites are unaffected — they never require `world_dir`). Observable: `cargo build -p rc-test-harness` still succeeds.
2. **`fixtures/corrupting_backend.rs`.** Implement `InMemoryHonestBackend`/`CorruptingBackend` against M2-B03's actual `ChunkStorageBackend`/`StorageError` shape (by this point in a real implementation timeline, M2-B03 has landed — this blueprint's own implementer resolves any small shape mismatch against Context's assumption here, per this blueprint's own stated tolerance for that). Observable: compiles against `rc-chunk-storage`.
3. **`chunk_soak.rs`.** Implement `generate_chunk_payload` (PRNG-seeded `BlockStateColumn` construction per Context's exact palette-cycling rule, `ChunkColumn`-equivalent NBT encode via M2-B04's actual `to_nbt`, then Zlib-compress per WORLD-D13's own default), `palette_shape_for`, `run_soak` (calling M2-B03's `round_trip_write_read` or the inline fallback, Context). Observable: `chunk_soak_self_tests.rs` (cases 1–5) passes.
4. **`chunk_soak_10000.rs`.** Wire the real `AnvilDiskBackend` + tempdir + 10,000-count `run_soak` call, the wall-clock budget check, and the diagnostic panic-message assembly. Observable: `chunk_soak_10000.rs` passes within budget.
5. **`save_cadence.rs`.** Implement `parse_save_event_log` (line-by-line `serde_json::from_str`, skip on `Err`), `analyze_cadence` (group by `region_id` via a `HashMap<String, Vec<u64>>` of ticks in arrival order, then per-group consecutive-gap check against `expected_interval_ticks ± 1`). Observable: `save_cadence_self_tests.rs` passes.
6. **`restart_persistence.rs`.** Implement `expected_state()` (the Context table, literally), `compare_state` (field-by-field `Vec<String>` builder, never short-circuiting), then `apply_actions`/`observe_state` against azalea's real, verified-at-this-point API (Context's own deferred-verification note) plus `rc_protocol`'s existing `Level Chunk with Light` decoder for the observation leg's block-state check. Observable: `restart_persistence_self_tests.rs` (cases 1–4, which only exercise `expected_state`/`compare_state`) passes without needing `apply_actions`/`observe_state` to be complete or correct yet; those two async functions are exercised only by the real `m2-report` run (Tier 2/manual), never by this blueprint's own Tier-1 test changeset.
7. **`xtask/src/m2_report.rs`.** Implement `Mode::cadence_params`, `M2ReportResult`, and `run` per Deliverables' doc comment, reusing `tier_result::write_to`/`exit_code_for` unmodified. Observable: `m2_report_cli.rs` cases 1–3 pass.
8. **`xtask/src/main.rs`.** Add the `M2Report` variant and its `match` arm; add `rc-paritybot`'s new module to nothing extra (already a dependency of `xtask` via M1-B06). Observable: `cargo build -p xtask` succeeds; `cargo run -p xtask -- m2-report --help` prints usage; `m2_report_cli.rs` case 4 passes (confirming no `path_guard.rs` edit was needed).
9. **`.github/workflows/ci.yml`.** Append the `m2-acceptance` job and the `workflow_dispatch.inputs.m2_report_mode` addition exactly as specified; every other job's YAML untouched.
10. **Run the full acceptance suite.** `cargo nextest run -p rc-test-harness -p rc-paritybot -p xtask` — every test named in Acceptance tests now passes. Commit this blueprint's governance changeset with `Changeset-Type: governance` (Constraints).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, per Acceptance tests' own stated boundary; the governance changeset must not edit any of the listed test files or weaken/delete/`#[ignore]` any case in them.

(b) **This blueprint's implementation changeset is a governance changeset, not an implementation one** — identical framing and identical reason to M1-B06's own Constraint (b): it fills in real bodies inside `crates/testing/{test-harness,paritybot}/**`, both fully protected paths (TEST-D46, M0-B08's `PROTECTED_PATHS` rows 8/10 as corrected by M1-B06), plus touches `xtask/**` (row #7). Every commit carries `Changeset-Type: governance`.

(c) **No new external dependencies beyond the pinned set, with exactly one permitted addition:** `rand_chacha` (crates.io, deterministic PRNG for `generate_chunk_payload`) — added to `[workspace.dependencies]` only if not already present at implementation time (M2-B03's own soak-adjacent needs may have already pinned it; this blueprint does not duplicate a pin). `tempfile` (already a reasonable expectation as a dev-dependency for any crate testing real disk I/O — added to `rc-test-harness`'s `[dev-dependencies]` if not already present, no new workspace-level pin needed if it already exists from an earlier blueprint). No other crate is added anywhere in this blueprint's deliverables.

(d) **No Mojang or third-party reimplementation code.** The Player-Action/Use-Item-On packet *names* (not byte layouts, deliberately left to implementation-time verification per Context) are restated from public, commonly-known vanilla protocol conventions (minecraft.wiki-class public documentation, ASSET-D18(f)'s own primary-source hierarchy), never from decompiled source or any other reimplementation's code (ASSET-D30).

(e) **No `unsafe` code.** Nothing in this blueprint's deliverables requires it.

(f) **`rc-test-harness` stays free of any new async-runtime dependency** — `chunk_soak.rs`/`save_cadence.rs`/`fixtures/*` are synchronous, matching M1-B06's own "`rc-test-harness` stays synchronous" rule exactly; only `rc-paritybot`'s `restart_persistence.rs` (already `tokio`/`azalea`-dependent, unchanged from M1-B06) and `xtask`'s own `m2_report.rs` (isolated `block_on`, per M1-B06's precedent) touch async code.

(g) **No committed soak fixture.** `generate_chunk_payload`'s output is never written to a committed file anywhere in this blueprint's deliverables — reproducibility comes from the logged `seed`, never from a checked-in corpus (Context) — do not add a `PROTECTED_PATHS` row or a fixture-manifest entry for it.

(h) **Scope boundary.** This blueprint does not implement M2-B03/B04/B05/B06/B07's own actual persistence logic, the Anvil format, `level.dat`, player-data schemas, the save pipeline, or the minimal place/break protocol handlers themselves — it only consumes their (assumed, where not-yet-merged) public surface to measure the milestone's own acceptance criteria, exactly as M1-B06 measured M1's without implementing `rc-protocol`/`rusty-clanker-server` itself.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-test-harness -p rc-paritybot -p xtask --all-features
cargo nextest run -p rc-test-harness -p rc-paritybot -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- m2-report --help
```

Expected: every command exits 0, including `chunk_soak_10000.rs`'s real 10,000-round-trip soak (within its own 180s budget) as part of the `nextest run` above. `cargo test --doc -p rc-test-harness -p rc-paritybot` also exits 0. CI's `gates`/`guardrails` jobs green on both OS legs on a clean checkout (TEST-D50) is this blueprint's own authoritative Done signal. The new `m2-acceptance` job's own first green run (nightly `smoke`, then a manually-triggered `full`) is a separate, later signal — the one that closes `11-roadmap-milestones.md`'s M2 Acceptance Criteria 1 and 3 themselves (Criterion 2, the soak, is already closed by this blueprint's own Tier-1 `chunk_soak_10000.rs`), once M2-B03 through M2-B07 have also landed — not part of this blueprint's own Done state.

## Open Questions

- The 10,000-chunk soak's 180-second Tier-1 wall-clock budget (Context) is a seed default, consistent with every other numeric threshold this project's planning corpus carries at this stage — needs calibration once a real `AnvilDiskBackend` implementation and real CI hardware exist; if it proves too tight, the correct fix is raising the budget (a governance changeset, since it is this blueprint's own stated number, not a `PROTECTED_PATHS`-guarded SLO table entry), not silently loosening the checksum assertion itself.
- The `full`-mode save-cadence default of `1200` ticks (Context) is this blueprint's own choice, made to get a statistically meaningful sample count within AC3's literal 30-minute window — whether a future maintainer instead wants the literal vanilla-default 6000-tick interval exercised as the *default* `full` run (accepting only ~6 samples) rather than as an explicit override is left open, pending real operational experience.
- The exact azalea method names for block placement/breaking (Context) are deliberately left to implementation-time verification against azalea's own current documentation, mirroring M1-B06's identical treatment of azalea's evolving API surface — this is a standing, accepted category of "verify against a live external dependency at implementation time," not a gap specific to this blueprint.

**Resolved** (was previously open at derivation time): this blueprint's restart-round-trip script places only `minecraft:stone` (Context's "Defined block pattern" table) and performs no inventory edit — matching M2-B07's real, merged place/break behavior exactly (a single fixed block placed regardless of input, and no serverbound `Set Creative Mode Slot` handler anywhere in M2). Inventory persistence is out of M2's scope entirely, deferred to `M3`/`M4`'s real `ItemStack`/inventory model (MECH-D47) — AC1's own inventory half is therefore a documented, explicit M2-scope gap, not a scripted assertion against functionality that does not exist; this blueprint's own AC1 report cases (Deliverables) assert only block state and player position/health.
