# M2-B06 — Player Data Persistence (`playerdata`, minimal `level.dat` schema, join/disconnect wiring)

| Field | Content |
|---|---|
| ID | M2-B06 |
| Milestone | M2 — Persistent World Storage |
| Prerequisites | M2-B02 (`rc-nbt`: `read_gzip_owned`/`write_gzip_owned`/`read_borrowed_strict`/`NbtError`, the `borrow`/`owned` module split, `NbtCompoundExt`'s `require_*` accessors, `SchemaError`/`NbtPath` — this blueprint is `rc-nbt`'s first player-data schema consumer and reuses every one of these primitives unmodified). M1-B04 (`rusty_clanker_server::net::{PlayerSession, PlayerSessionSink, ResolvedProfile}`, restated in full below). M1-B05 (`rusty_clanker_server::play::{enter_play, PlayerProfile, SPAWN_POSITION}` — this blueprint's own composition-root wiring is stated as an integration recipe against whichever shape `play::world`'s `HardcodedWorld` currently has, Context's own "Sibling M2 blueprints" subsection explains why a literal full-file diff against M1-B05's original shape is not this blueprint's own Deliverable). **Not formal prerequisites, but directly informing this blueprint's design** (discovered already present on disk at derivation time, alongside this blueprint's own assigned task — Context's "Sibling M2 blueprints" subsection restates exactly what each one changes and how this blueprint reconciles against it): M2-B03 (`rc-chunk-storage::anvil`'s `AnvilDiskBackend`/`ChunkStorageBackend`, whose `read_level_dat`/`write_level_dat` this blueprint's own `level_dat` module produces/consumes bytes for, never reimplementing file-level `level.dat` safety itself); M2-B05 (extends `HardcodedWorld`/adds `crate::config::WorldConfig` in `rusty-clanker-server`); M2-B07 (also extends `HardcodedWorld`/`PlayerMarker`/`PendingJoin`, spawns real chunk entities); M2-B08 (the milestone's own acceptance-harness blueprint, which owns the *end-to-end* "real bot places/breaks blocks, restarts, rejoins, compares" scenario this blueprint's own roadmap criterion partly feeds — this blueprint does not duplicate that harness). |
| Implements | The M2 roadmap's "player data persistence (position, inventory, health) via `rc-nbt`" scope item (`11-roadmap-milestones.md`), and the player-save-cadence half of "configurable per-region save interval, firing off the tick thread" (the chunk half is M2-B05's own, already-delivered scope — Context); the player-data and minimal-`level.dat` portions of `03-world-chunks-persistence.md`'s WORLD-D14 (folder layout — this blueprint corrects one field of it, Context's Resolved discrepancy) and WORLD-D15 (`level.dat`'s schema-ownership pattern, applied here at M2's own minimal-field scope, storage mechanics delegated to M2-B03); consumes `05-game-mechanics.md`'s already-binding MECH-D30 (base entity NBT field set, reused unmodified for `Pos`/`Motion`/`Rotation`) and MECH-D47 (post-1.20.5 item data-component model, restated field-shape-only — every component payload is stored opaquely, MECH-D47's own 90 concrete semantics stay `05`'s job) without redefining either. |
| Crates touched | `rc-chunk-storage` (`crates/chunk-storage/`) — three new modules, `player`, `level_dat`, and their shared error type; `rusty-clanker-server` (`crates/server/`) — adds `crates/server/src/play/persistence.rs` (a fully self-contained, independently testable module); Context's own "Composition-root integration" subsection gives the small, signature-agnostic wiring recipe into `play::world`/`play::connection` that a since- or later-merged M2-B05/M2-B07 needs, rather than a literal full-file diff against a contested, still-settling shape. |
| Estimated scope | L |

## Goal & Done definition

Give a player's position, rotation, motion, health, food stats, experience, inventory (main + hotbar), selected hotbar slot, dimension, game mode, and abilities a real, `rc-nbt`-backed on-disk representation at `<world root>/players/data/<uuid>.dat` (Context's Resolved discrepancy explains the exact folder name), behind a small, fake-friendly `PlayerDataStore` trait mirroring `ChunkStorageBackend`'s own already-established shape (M2-B03/M2-B05's precedent); plus a minimal `LevelDat` schema producing/consuming the exact GZip-compressed byte payload `AnvilDiskBackend::write_level_dat`/`read_level_dat` (M2-B03) already expect and return, carrying just the fields M2 itself needs (`DataVersion`, `LevelName`, world `Time`, default spawn) and an opaque, round-tripped rest-of-`Data` bag M2 never interprets. Give `rusty-clanker-server` a fully self-contained, independently-testable `PlayerSessionStore` (load-on-join, save-on-disconnect, periodic save) that composes with whichever `PlayerDataStore` implementation the composition root wires it to — real (`FilesystemPlayerDataStore`) or fake (a test's own in-memory `HashMap`). Every real vanilla player-entity field this blueprint does not itself model (equipment, `EnderItems`, `recipeBook`, stats/advancement-adjacent fields, `foodTickTimer`, `XpSeed`, and the rest of MECH-D30's base-entity field set beyond `Pos`/`Motion`/`Rotation`) survives an unmodified load-then-save cycle byte-for-byte, via the patch-over-original design in Context.

Done when:

- [ ] `cargo build -p rc-chunk-storage -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-chunk-storage -p rusty-clanker-server` (default features).
- [ ] `cargo test --doc -p rc-chunk-storage -p rusty-clanker-server` exits 0.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0. `lint-deps` still reports zero violations: this blueprint's new dependency edges on `rc-chunk-storage` (`uuid`, already workspace-pinned since M1-B04's CROSS-D12 addition; `thiserror` and `flate2`, both already workspace-pinned and both **also** independently added by the sibling M2-B03 — the identical "union of additions, same line either way" shared-crate-landing pattern M2-B01/M2-B03 already established for this crate's `Cargo.toml`) touch no `SIM`/`NETRENDER` boundary; `rc-chunk-storage` gains no new dependency on `rc-protocol` or any `NETRENDER`/`SIM` crate — item ids are plain on-disk strings, never `rc_registries::generated_v776`'s registry-id types, exactly the reasoning M2-B01 already established for block states.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Sibling M2 blueprints discovered during this blueprint's own derivation — reconciliation stance

At the time this blueprint was written, `blueprints/M2/` already contained M2-B01 through M2-B08 (not only this blueprint's formally assigned prerequisite, M2-B02) — several derived in parallel by other agents, without mutual awareness, each restating its own assumed contract for not-yet-existing siblings exactly as this project's own governance already sanctions (`blueprints/00-blueprint-spec.md`'s "restate everything the implementer needs" rule, applied by every one of M2-B05's/M2-B08's own "Derivation-time note" paragraphs: "where a since-merged sibling differs from an assumption below, that merged blueprint's actual shape is authoritative... a small, scoped adaptation... never a full rewrite"). This blueprint adopts the identical discipline for the three siblings whose own Deliverables materially touch the same files this blueprint needs to reach:

- **M2-B03** (`rc-chunk-storage::anvil`) already implements `AnvilDiskBackend`/`ChunkStorageBackend` with `read_level_dat(&self) -> Result<Vec<u8>, StorageError>`/`write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>`, and states explicitly that `payload` is **already GZip-compressed by the caller** — `AnvilDiskBackend` itself only owns the file-level atomic-write-with-backup mechanics (`level.dat_new`/`level.dat_old`) and the world-level `session.lock`. This blueprint's own `level_dat` module is therefore designed as a **pure, storage-agnostic** producer/consumer of exactly that byte shape (`LevelDat::to_gzip_bytes`/`from_gzip_bytes`, Deliverables) — it performs no file I/O of its own and does not reimplement M2-B03's already-correct safe-write scheme. A future composition-root blueprint wires the two together with one line each way (`backend.write_level_dat(&level.to_gzip_bytes()?)`, `LevelDat::from_gzip_bytes(&backend.read_level_dat()?)`) — not this blueprint's own job, since M2-B03's `AnvilDiskBackend` construction (world directory, compression scheme, session lock) is that blueprint's own concern, not restated here.
- **M2-B05** (`rusty-clanker-server`) extends `HardcodedWorld` with a `crate::config::WorldConfig { save_interval_secs, simulation_distance_chunks, world_dir: PathBuf }`, changes `HardcodedWorld::new()`'s own signature to `new(config: WorldConfig)`, and adds a `HardcodedWorld::shutdown()` flushing chunk saves via `ChunkLifecycleManager::shutdown` before the region thread exits. **M2-B07** independently extends the *same* `HardcodedWorld`/`PlayerMarker`/`PendingJoin` types (spawning nine real chunk entities at bootstrap, adding a `connection: ConnectionHandle` field to `PlayerMarker`/`PendingJoin` for its own block-update broadcast). Because these two blueprints' own additions to `play::world.rs` were derived independently of each other and of this blueprint, this blueprint does **not** attempt a literal, full-file `world.rs`/`connection.rs` diff against either one's presumed shape (doing so would silently conflict with whichever one, or both, actually land) — Context's own "Composition-root integration" subsection instead gives the small, self-contained, signature-agnostic set of calls this blueprint's own `PlayerSessionStore` (Deliverables, fully self-contained and independently unit-tested with **zero** dependency on `HardcodedWorld`'s exact shape) needs wired in, wherever `HardcodedWorld` construction, its tick loop, and `enter_play` actually end up living once every M2 blueprint has landed and been reconciled.
- **M2-B08** (the milestone's own acceptance-harness blueprint) independently guessed, ahead of this blueprint existing, that player persistence would live at `rc_mechanics::player_persistence` with a `PlayerDataStore` trait keyed by `uuid: [u8; 16]` and a four-field `PlayerDataRecord` (`x`/`y`/`z`, `health`, `inventory`) built on M2-B02's closed-schema `ToNbtCompound`/`FromNbtCompound` traits. Per M2-B08's own stated reconciliation policy ("this blueprint's own actual shape is authoritative... the same small, scoped adaptation"), **this blueprint's real design supersedes that guess**: player persistence lives in `rc-chunk-storage` (the crate `03-world-chunks-persistence.md` — this blueprint's own owning document — already anchors every other piece of M2's persistence work in, and the crate this blueprint's own task was assigned against), keyed by `uuid::Uuid` (already workspace-pinned, more ergonomic than a bare byte array), with the fuller field set Context's own schema table gives (position/rotation/motion/health/food/XP/inventory/selected-slot/dimension/game-mode/abilities, not just four). This blueprint *does* adopt M2-B08's one genuinely good idea independent of where it was guessed to live — a small, fake-friendly storage trait mirroring `ChunkStorageBackend`'s own shape (`PlayerDataStore`, Deliverables) — since it is straightforwardly better design, not merely alignment for its own sake. M2-B08's own implementer needs exactly one adaptation once this blueprint lands: `use rc_mechanics::player_persistence::{PlayerDataStore, PlayerDataRecord}` becomes `use rc_chunk_storage::{PlayerDataStore, LoadedPlayerRecord}`, and its own four-field comparison (`x`/`y`/`z`/`health`/`inventory`) reads those same values off `LoadedPlayerRecord::data.{pos, health, inventory}` instead — a signature-only fix, never a redesign, exactly M2-B08's own accepted category.

### What already exists, restated exactly (no already-merged blueprint's own files are modified by this blueprint beyond Context's own "Composition-root integration" recipe)

`rc-nbt` (M2-B02) exports, and this blueprint reuses verbatim: `rc_nbt::{borrow, owned}` (thin re-exports of `simdnbt::{borrow, owned}`'s tree types), `rc_nbt::{read_gzip_owned, write_gzip_owned, read_borrowed_strict, NbtError}`, and the schema layer `rc_nbt::schema::{NbtPath, SchemaError, NbtCompoundExt}` — the last of these giving `borrow::NbtCompound` twelve `require_*(path, field) -> Result<T, SchemaError>` accessors, used throughout this blueprint's own read-side code exactly as M2-B02's own `ExamplePoint` test fixture already demonstrates. This blueprint does **not** implement the generic `ToNbtCompound`/`FromNbtCompound` traits from M2-B02 for `LoadedPlayerRecord`/`LevelDat` themselves — Context's "Unknown-field preservation" subsection explains why a different, complementary pattern is needed instead; `NbtCompoundExt`/`SchemaError`/`NbtPath` remain fully reused either way.

`rusty_clanker_server::net::{PlayerSession, PlayerSessionSink, ResolvedProfile}` (M1-B04, restated): `PlayerSession { profile: ResolvedProfile, entity_id: rc_core::RcEntityId, connection: ConnectionHandle, inbound: mpsc::Receiver<RawPacket> }`, `ResolvedProfile { id: uuid::Uuid, name: String, properties: Vec<rc_auth::ProfileProperty> }`. `uuid` is already workspace-pinned at `1.24.0` with features `["v4", "v5"]` (M1-B04's own addition) and already a normal dependency of `rusty-clanker-server`.

### Additional `simdnbt` surface this blueprint needs, beyond what M2-B02 already restated

M2-B02's own Context restates `simdnbt::owned::NbtTag`'s full variant list (including `List(NbtList)`) but does not enumerate `NbtList` itself, since nothing M2-B02 built needed a list-of-primitives. `Pos`/`Motion`/`Rotation` are exactly that (`List<Double>`/`List<Float>`), so this blueprint restates the two corners of `simdnbt` 0.10.0's `NbtList` surface it actually uses, verified against a live docs.rs fetch performed while deriving this blueprint (2026-08-21): `owned::NbtList` is a plain enum with one variant per element type holding a `Vec<T>` (`Double(Vec<f64>)`, `Float(Vec<f32>)`, `Compound(Vec<NbtCompound>)`, and so on for every other primitive/tag-tree type) — this blueprint constructs `owned::NbtList::Double(self.data.pos.to_vec())` etc. directly, and `owned::NbtCompound::insert`'s own `impl simdnbt::ToNbtTag` parameter bound (M2-B02's own restated signature) already accepts a plain `owned::NbtTag` value, so `out.insert("Pos", owned::NbtTag::List(owned::NbtList::Double(...)))` type-checks without any further conversion. `borrow::NbtList<'a,'tape>` (the read-side counterpart) exposes `doubles(&self) -> Option<Vec<f64>>` / `floats(&self) -> Option<Vec<f32>>` / `compounds(&self) -> Option<NbtCompoundList<'a,'tape>>` accessors, each `None` if the list's element type does not match — this blueprint's own `from_nbt` treats a `None` here as `SchemaError::WrongType`, and treats a `Some(vec)` whose length is not exactly the expected arity (`3` for `Pos`/`Motion`, `2` for `Rotation`) as `SchemaError::InvalidValue`. **Implementer verification note**, mirroring M2-B01/M0-B05's own identical practice for their own third-party API surfaces: confirm `ToNbtTag`'s exact blanket-impl set against the actually-installed `simdnbt` 0.10.0 docs at implementation time — this blueprint's own confidence in the *shape* of every call above is high, but `insert`'s exact bound was restated, not re-verified a second time, by M2-B02 itself.

### Resolved discrepancy, binding for this blueprint: `players/data/`, not `playerdata/`

`03-world-chunks-persistence.md`'s WORLD-D14 states the current (26.2) save-folder layout includes a `playerdata/` directory, citing a minecraft.wiki verification pass dated 2026-08-20 — a claim M2-B03's own restated WORLD-D14 diagram repeats verbatim (though M2-B03 itself never creates or touches that directory, calling it explicitly "future blueprints' responsibility"). `docs/research/mc-26.2/04-persistence-nbt.md` §3.13 — produced under the ASSET-D18(f) decompiled-reference process, a primary source for implementation-level facts this project's own reference-source policy ranks alongside minecraft.wiki, not beneath it — documents a **more specific, code-level fact WORLD-D14's wiki pass did not surface**: a new file-layout migration mechanism (`FileFixerUpper`, §3.16) was introduced at data version **4772** and renames the on-disk player-data directory from the historical `playerdata/` to `players/data/` (`LevelResource.PLAYER_DATA_DIR = "players/data"`). The pinned target's DataVersion is **4903** (WORLD-D16) — strictly greater than 4772 — so every world at the pinned target, including a freshly created one, already uses `players/data/`. A live fetch of `minecraft.wiki/w/Player.dat_format` performed while deriving this blueprint (2026-08-21) independently confirms the same folder-name history is current, real, and non-experimental as of today.

**Resolution:** this blueprint's `FilesystemPlayerDataStore::player_data_path` (Deliverables) resolves to `<world root>/players/data/<uuid>.dat`, not `<world root>/playerdata/<uuid>.dat`. Flagged here, and in M2-B03's own restated diagram, as a correction the next revision of `03-world-chunks-persistence.md`'s WORLD-D14 should incorporate — not a silent divergence. `level.dat`, `session.lock`, and the `region/`/`entities/`/`poi/` family are unaffected by this specific migration.

### Player NBT schema — the fields this blueprint actively models, field-by-field, DataVersion 4903

Base entity fields `Motion`/`Rotation` are MECH-D30's own already-binding names, reused verbatim. Every other MECH-D30 base-entity field (`FallDistance`, `Fire`, `Air`, `OnGround`, entity-level `Invulnerable`, `PortalCooldown`, `UUID`, `CustomName`, `CustomNameVisible`, `Silent`, `NoGravity`, `Glowing`, `TicksFrozen`, `HasVisualFire`, `Tags`, `Passengers`) is **not** modeled by this blueprint — it round-trips opaquely via the unknown-field-preservation mechanism below, exactly like every player-specific field this blueprint also does not model (`foodTickTimer`, `XpSeed`, `equipment`, `EnderItems`, `recipeBook`, `Score`, `SleepTimer`, `LastDeathLocation`, `RootVehicle`, `seenCredits`, `entered_nether_pos`, `warden_spawn_tracker`, and any field a future vanilla version adds). A future `05-game-mechanics.md`-owned blueprint extends this schema field-by-field as each of those becomes load-bearing — exactly WORLD-D6's own established "storage contract now, concrete semantics later" pattern.

Every field name/type below is restated from a live fetch of `minecraft.wiki/w/Player.dat_format` and `minecraft.wiki/w/Entity_format` performed while deriving this blueprint (2026-08-21) — public, long-documented structural facts, not Mojang creative expression, consistent with ASSET-D18(b)/(c)'s allowed-source categories:

| NBT key | NBT type | Rust field | Notes |
|---|---|---|---|
| `Pos` | `List<Double>`, 3 elements | `pos: [f64; 3]` | `[x, y, z]`, world-absolute |
| `Motion` | `List<Double>`, 3 elements | `motion: [f64; 3]` | `[dx, dy, dz]`, blocks/tick |
| `Rotation` | `List<Float>`, 2 elements | `rotation: [f32; 2]` | `[yaw, pitch]` |
| `Health` | `Float` | `health: f32` | |
| `foodLevel` | `Int` | `food_level: i32` | |
| `foodSaturationLevel` | `Float` | `food_saturation_level: f32` | |
| `foodExhaustionLevel` | `Float` | `food_exhaustion_level: f32` | |
| `XpLevel` | `Int` | `xp_level: i32` | |
| `XpP` | `Float` | `xp_p: f32` | progress toward next level, `0.0..1.0` |
| `XpTotal` | `Int` | `xp_total: i32` | |
| `Inventory` | `List<Compound>` | `inventory: Vec<InventorySlotEntry>` | see below |
| `SelectedItemSlot` | `Int` | `selected_item_slot: i32` | hotbar index, `0..=8` |
| `Dimension` | `String` | `dimension: rc_core::DimensionId` | `"minecraft:overworld"`/`"minecraft:the_nether"`/`"minecraft:the_end"` ↔ `DimensionId::{OVERWORLD, THE_NETHER, THE_END}`; any other string is a load-time `SchemaError::InvalidValue` |
| `playerGameType` | `Int` | `player_game_type: i32` | `0`=survival,`1`=creative,`2`=adventure,`3`=spectator (M1-B05's own `LoginPlay.game_mode` convention) |
| `previousPlayerGameType` | `Int` | `previous_player_game_type: i32` | `-1` = none (M1-B05's own `LoginPlay.previous_game_mode` convention) |
| `abilities` | `Compound` | `abilities: PlayerAbilities` | see below |

**`Inventory` entry schema** — one compound per occupied slot: `{ Slot: Byte, id: String, count: Int, components: Compound (omitted entirely when there is nothing to store) }`. `Slot` range is **`0..=35`** only (hotbar `0..=8`, main inventory `9..=35`) — Context's "Equipment scope exclusion" explains why armor/offhand are deliberately out of this blueprint's range.

**`abilities` compound schema:**

| NBT key | NBT type | Rust field |
|---|---|---|
| `flying` | `Byte` (0/1) | `flying: bool` |
| `flySpeed` | `Float` | `fly_speed: f32` |
| `instabuild` | `Byte` | `instabuild: bool` |
| `invulnerable` | `Byte` | `invulnerable: bool` |
| `mayBuild` | `Byte` | `may_build: bool` |
| `mayfly` | `Byte` | `may_fly: bool` |
| `walkSpeed` | `Float` | `walk_speed: f32` |

Default (survival) values, MECH-D60's own already-binding baseline: `walk_speed = 0.1`, `fly_speed = 0.05`, every `bool` `false` except `may_build = true`.

### Item-stack (data-component) format — restated from `docs/research/mc-26.2/{04-persistence-nbt.md, 10-items-recipes-loot.md}` plus a live web verification (2026-08-21), and the equipment scope exclusion

MECH-D47 (binding) already fixes the pinned target's item model as the post-1.20.5 **data-component** format — a `(ItemId, count, ComponentMap)` value. On disk, per live verification and `10-items-recipes-loot.md` §3.2's own description of `PatchedDataComponentMap`: an item stack's compound has exactly three possible members — `id` (`String`, namespaced, e.g. `"minecraft:diamond_sword"`), `count` (`Int`, `1..=99`), and `components` (`Compound`, **present only when the patch is non-empty**).

**This blueprint stores `components` as a fully opaque, byte-preserved `rc_nbt::owned::NbtCompound`** — it never inspects, validates, or interprets a single one of the 90 concrete `DataComponentType`s the research doc catalogues. Implementing any of that is squarely `05-game-mechanics.md`'s MECH-D47 scope. `ItemStackRecord::components: Option<rc_nbt::owned::NbtCompound>` (`None` ⇔ the tag is entirely absent, matching the omit-if-empty rule byte-for-byte) is the concrete embodiment of this design.

**Documented, bounded simplification — equipment is out of scope.** Both the research corpus and a live web verification confirm that armor (head/chest/legs/feet) and the off-hand slot are, as of the current format, stored in a separate per-entity `equipment` compound (keys `head`/`chest`/`legs`/`feet`/`offhand`) — **not** inside the flat `Inventory` list at the legacy slot numbers `100`–`103`/`-106` older Minecraft history used. This blueprint does not model `equipment` at all: neither block placement/breaking nor the byte-identity criterion depends on armor. An `equipment` compound present in a loaded file survives untouched via the unknown-field-preservation mechanism, exactly like any other field this blueprint does not actively model.

### Unknown-field preservation — a patch-over-original design, not the generic `ToNbtCompound`/`FromNbtCompound` traits

M2-B02's `ToNbtCompound`/`FromNbtCompound` traits assume a **closed** schema. Neither `PlayerSaveData` nor `LevelDat` has a closed shape at M2 — both are a *strict, evolving subset* of a much larger real schema, and this blueprint's own task requires that everything outside its subset survive a load-then-resave cycle unchanged.

The mechanism, applied identically to both `LoadedPlayerRecord` and `LevelDat`: on load, the **entire** decoded root compound is kept (`.to_owned()`'d into an `owned::NbtCompound`) as a `base` field, alongside this blueprint's own typed field extraction (via `NbtCompoundExt`'s `require_*` accessors). On save, a **fresh clone of `base`** is the starting point, and only this blueprint's own modeled top-level keys are inserted/overwritten on top of it (`owned::NbtCompound::insert`) — every key `base` already carried that this blueprint's own field list does not name is left byte-for-byte as it was read. A freshly-created record (never loaded from disk) starts from an **empty** `base` — its very first save contains exactly this blueprint's own modeled fields and nothing else.

### `level.dat` — the minimal compound M2 needs, produced/consumed as bytes for M2-B03's `AnvilDiskBackend`

WORLD-D15 (binding) fixes `level.dat`'s full field ownership; this blueprint implements only the subset M2's own roadmap scope needs. Root shape (WORLD-D15, `docs/research/mc-26.2/04-persistence-nbt.md` §3.12): GZip-compressed NBT, an unnamed root `Compound` containing exactly one child, `Data` (`Compound`).

| NBT key (inside `Data`) | NBT type | Rust field |
|---|---|---|
| `DataVersion` | `Int` | `data_version: i32` — always `4903` on write (WORLD-D16) |
| `LevelName` | `String` | `level_name: String` |
| `Time` | `Long` | `time: i64` — total world age in ticks |
| `LastPlayed` | `Long` | `last_played: i64` — epoch millis |
| `spawn.X`/`.Y`/`.Z` | `Int` each | `spawn_x/spawn_y/spawn_z: i32` — flattened rather than modeling the nested `spawn` compound's own full internal shape, since M2 needs only the coordinates |
| `spawn.Angle` | `Float` | `spawn_angle: f32` |
| `Version.Name` | `String` | `version_name: String` — e.g. `"26.2"` |
| `Version.Id` | `Int` | (derived: always mirrors `data_version`, not a separate field) |
| `Version.Snapshot` | `Byte` | `version_snapshot: bool` — `false` for every record this blueprint writes |
| `Version.Series` | `String` | `version_series: String` — `"main"` |

`GameRules` and every other real `level.dat` field are **not** modeled — round-tripped via the identical patch-over-`base` mechanism as player records. A fresh `LevelDat::fresh_default` has an empty `base`.

### Load-on-join, save-on-disconnect, periodic save — a fully self-contained `PlayerSessionStore`

`PlayerSessionStore` (Deliverables) owns an in-memory `sessions: HashMap<uuid::Uuid, LoadedPlayerRecord>` and a `store: Arc<dyn PlayerDataStore>` — it is a plain, synchronous, `HardcodedWorld`-agnostic type, fully testable with either `FilesystemPlayerDataStore` (real disk) or a test's own in-memory fake (Acceptance tests). `load_or_create(uuid, dimension, default_pos)` reads (or freshly defaults) a record, stashes it, and returns the `(pos, rotation)` pair a caller needs for its own Play-entry packets. `save_and_remove(uuid)` synchronously persists and evicts one record. `snapshot_all()`/`save_all()` support a periodic sweep. None of these methods know anything about `HardcodedWorld`, Tokio, or TCP — they are the exact, minimal seam Context's "Composition-root integration" subsection wires into whichever shape `play::world`/`play::connection` actually has.

### Composition-root integration — a signature-agnostic recipe, not a full-file diff

Because `play::world.rs`'s `HardcodedWorld` (and `play::connection.rs`'s `enter_play`) are simultaneously targeted by M2-B05 and M2-B07 in ways this blueprint cannot predict the final merged shape of (Context's "Sibling M2 blueprints" subsection), this blueprint states its own integration requirement as a precise recipe applicable regardless of which shape lands first, rather than claiming to know the exact resulting file contents:

1. Wherever `HardcodedWorld` (or its eventual successor) is constructed, add one field holding a `PlayerSessionStore` (Deliverables), initialized via `PlayerSessionStore::new(Arc::new(FilesystemPlayerDataStore::new(<the composition root's own world-directory value — M2-B05's `WorldConfig.world_dir` if present, or an explicit caller-supplied path otherwise>)))`.
2. Inside `enter_play` (or wherever the Play-entry packet sequence is built), immediately before constructing the position-carrying packets, call `sessions.load_or_create(uuid, rc_core::DimensionId::OVERWORLD, <the world's own default spawn, converted to `[f64;3]`>)` and use the returned `(pos, rotation)` in place of any hardcoded literal — `SetDefaultSpawnPosition` (the world's own compass point) stays independent of this, unchanged.
3. At every exit path of the connection's own driving loop (disconnect, timeout, error), call `sessions.save_and_remove(uuid)`, logging (never panicking) on `Err`.
4. Inside the region's own tick loop, add a plain `u64` counter incremented once per tick; when it reaches the configured save interval (M2-B05's own `WorldConfig::save_interval_ticks()` if present, else `PlayerPersistenceConfig::default().save_interval_ticks`, Deliverables — reusing one shared interval for both chunk and player saves is preferred over two independently-configured ones), reset it and spawn one `std::thread::spawn` closure calling `sessions.save_all()` (Context's own bounded-simplification note below explains why a plain thread, not `RC-IoPool`, is this blueprint's own choice at M2's current scope).
5. If a `shutdown()` method already exists (M2-B05's own addition, flushing chunk saves), extend it to also call `sessions.save_all()` — giving every still-connected player's data the same clean-restart guarantee WORLD-D25 already gives chunk data.

**Documented, bounded simplification:** step 4's periodic sweep is a plain, uncoordinated `std::thread::spawn`, not `RC-IoPool` (WORLD-D21) — that dedicated pool does not exist in any merged blueprint yet. A future `rc-chunk-storage`-owned blueprint that builds `RC-IoPool` should retarget this call site onto it; nothing about `PlayerSaveData`'s own shape or `PlayerDataStore`'s own trait signature needs to change when that happens, since both are already plain, synchronous, `RC-IoPool`-callable-shaped.

## Deliverables

### `crates/chunk-storage/Cargo.toml` (modify — add three normal dependencies; union with M2-B01's and M2-B03's own additions, same "shared-crate landing order" convention those two blueprints already established for this file)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-nbt = { path = "../nbt" }
rc-registries = { path = "../registries" }
bevy_ecs = { workspace = true }          # M2-B01's own addition
flate2 = { workspace = true }             # this blueprint AND M2-B03 both add this line — identical either way
lz4_flex = { workspace = true }           # M2-B03's own addition, unused by this blueprint
parking_lot = { workspace = true }        # M2-B03's own addition, unused by this blueprint
thiserror = { workspace = true }          # this blueprint AND M2-B03 both add this line — identical either way
uuid = { workspace = true }               # this blueprint's own addition — already pinned, features ["v4","v5"], since M1-B04
io-uring = { workspace = true, optional = true }

[dev-dependencies]
proptest = { workspace = true }           # M2-B01's own addition, unused by this blueprint's own tests

[features]
io_uring = ["dep:io-uring"]
```

(If M2-B01/M2-B03 have not yet landed when this blueprint is implemented, this blueprint's own three lines — `flate2`, `thiserror`, `uuid` — are the only ones it actually needs to add; the rest is shown for a complete, unambiguous file per this crate's own established multi-blueprint convention.)

### `crates/chunk-storage/src/lib.rs` (modify — add two module declarations and their re-exports; union with every other blueprint's own additions to this file)

```rust
mod player;
mod level_dat;

pub use player::{
    FilesystemPlayerDataStore, InventorySlotEntry, ItemStackRecord, LoadedPlayerRecord,
    PlayerAbilities, PlayerDataStore, PlayerPersistenceError, PlayerSaveData,
};
pub use level_dat::LevelDat;
```

### `crates/chunk-storage/src/player.rs` (new)

```rust
use std::path::{Path, PathBuf};
use rc_nbt::{borrow, owned, schema::{NbtCompoundExt, NbtPath, SchemaError}, NbtError};

#[derive(Debug, thiserror::Error)]
pub enum PlayerPersistenceError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Nbt(#[from] NbtError),
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("player data root NBT document must be a non-empty compound, found an empty document")]
    EmptyDocument,
}

/// The player-data-file storage seam (mirrors `rc_chunk_storage::anvil::ChunkStorageBackend`'s
/// own already-established shape, M2-B03 — Context's own "Sibling M2 blueprints" subsection).
/// Deliberately independent of `ChunkStorageBackend` itself: player files are not part of
/// WORLD-D17's trait (M2-B03 explicitly never touches `players/data/`), so this is its own,
/// narrower seam — one real implementation (`FilesystemPlayerDataStore`) plus, in this
/// blueprint's own tests, a fake in-memory one, exactly the pattern M2-B03/M2-B05 already use
/// for their own storage-backend tests.
pub trait PlayerDataStore: Send + Sync + 'static {
    /// `Ok(None)` — not an error — if no file/entry exists yet for `uuid` (never joined
    /// before). The returned bytes, when `Some`, are the raw on-disk bytes: already
    /// GZip-compressed (this blueprint's own `save_player`/`load_player`, Deliverables,
    /// perform the (de)compression on top of this trait, mirroring M2-B03's own
    /// caller-compresses convention for `write_level_dat`).
    fn read_player_data(&self, uuid: uuid::Uuid) -> Result<Option<Vec<u8>>, PlayerPersistenceError>;
    fn write_player_data(&self, uuid: uuid::Uuid, payload: &[u8]) -> Result<(), PlayerPersistenceError>;
}

/// The real, local-disk `PlayerDataStore` (Context's Resolved discrepancy for the exact path).
#[derive(Clone, Debug)]
pub struct FilesystemPlayerDataStore {
    root: PathBuf,
}

impl FilesystemPlayerDataStore {
    /// `root` is the world save directory — the same value a composition root passes to
    /// `AnvilDiskBackend::open` (M2-B03), kept as an independent path here rather than a
    /// hard type dependency on that blueprint's own struct (Context's "Composition-root
    /// integration").
    pub fn new(root: impl Into<PathBuf>) -> Self;

    /// `<root>/players/data/<uuid>.dat` (Context's Resolved discrepancy — NOT
    /// `<root>/playerdata/<uuid>.dat`).
    pub fn player_data_path(&self, uuid: uuid::Uuid) -> PathBuf;

    /// `<root>/players/data/` — created via `std::fs::create_dir_all` on first write if it
    /// does not yet exist.
    pub fn player_data_dir(&self) -> PathBuf;
}

impl PlayerDataStore for FilesystemPlayerDataStore {
    /// `Ok(None)` specifically on `std::io::ErrorKind::NotFound`; `Err(Io(..))` otherwise.
    fn read_player_data(&self, uuid: uuid::Uuid) -> Result<Option<Vec<u8>>, PlayerPersistenceError>;
    /// `std::fs::create_dir_all(player_data_dir())` then `std::fs::write`. Overwrites any
    /// existing file at that path — no `.dat_new`/`.dat_old` safety scheme (unlike M2-B03's
    /// own `level.dat` handling): a future blueprint may add one to this store specifically
    /// if player-file corruption-on-crash is ever observed, not built here (Constraints).
    fn write_player_data(&self, uuid: uuid::Uuid, payload: &[u8]) -> Result<(), PlayerPersistenceError>;
}

/// One occupied `Inventory` slot (Context's schema table). `slot` is `0..=35`.
#[derive(Clone, Debug, PartialEq)]
pub struct InventorySlotEntry {
    pub slot: i8,
    pub item: ItemStackRecord,
}

/// The post-1.20.5 data-component item-stack shape (Context), `components` stored
/// fully opaque.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemStackRecord {
    pub id: String,
    pub count: i32,
    pub components: Option<owned::NbtCompound>,
}

/// The `abilities` sub-compound (Context's schema table).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerAbilities {
    pub flying: bool,
    pub fly_speed: f32,
    pub instabuild: bool,
    pub invulnerable: bool,
    pub may_build: bool,
    pub may_fly: bool,
    pub walk_speed: f32,
}

impl Default for PlayerAbilities {
    /// MECH-D60's survival baseline (Context).
    fn default() -> Self;
}

/// This blueprint's own actively-modeled field set (Context's schema table).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSaveData {
    pub pos: [f64; 3],
    pub motion: [f64; 3],
    pub rotation: [f32; 2],
    pub health: f32,
    pub food_level: i32,
    pub food_saturation_level: f32,
    pub food_exhaustion_level: f32,
    pub xp_level: i32,
    pub xp_p: f32,
    pub xp_total: i32,
    pub inventory: Vec<InventorySlotEntry>,
    pub selected_item_slot: i32,
    pub dimension: rc_core::DimensionId,
    pub player_game_type: i32,
    pub previous_player_game_type: i32,
    pub abilities: PlayerAbilities,
}

/// A player record together with everything this blueprint does not itself model,
/// preserved for a lossless round trip (Context's "Unknown-field preservation").
#[derive(Clone, Debug)]
pub struct LoadedPlayerRecord {
    pub data: PlayerSaveData,
    base: owned::NbtCompound,
}

impl LoadedPlayerRecord {
    /// A brand-new player: `data` a fresh default (`pos`/`rotation`/`dimension` from the
    /// caller; every other field its own natural default: zero motion, `Health = 20.0`,
    /// `foodLevel = 20`, `foodSaturationLevel = 5.0`, everything else `0`, empty inventory,
    /// `selected_item_slot = 0`, `player_game_type = 0`, `previous_player_game_type = -1`,
    /// default `PlayerAbilities`), `base` empty.
    pub fn fresh_default(dimension: rc_core::DimensionId, pos: [f64; 3]) -> Self;

    /// Decodes `compound` into `data`, keeping a full `.to_owned()` copy as `base`. Every
    /// field in Context's schema table is required except `Inventory` entries' own
    /// `components`, which is genuinely optional.
    pub fn from_nbt(compound: &borrow::NbtCompound<'_, '_>) -> Result<Self, SchemaError>;

    /// `base.clone()` patched with this blueprint's own fields (Context's mechanism), in
    /// the fixed order Implementation steps gives.
    pub fn to_nbt(&self) -> owned::NbtCompound;
}

/// GZip-decompresses `store.read_player_data(uuid)`'s bytes (if any) and decodes via
/// `LoadedPlayerRecord::from_nbt`. `Ok(None)` if `store` returns `None`.
pub fn load_player(
    store: &dyn PlayerDataStore,
    uuid: uuid::Uuid,
) -> Result<Option<LoadedPlayerRecord>, PlayerPersistenceError>;

/// GZip-compresses `record.to_nbt()` (via `rc_nbt::write_gzip_owned`) and hands the bytes
/// to `store.write_player_data`.
pub fn save_player(
    store: &dyn PlayerDataStore,
    uuid: uuid::Uuid,
    record: &LoadedPlayerRecord,
) -> Result<(), PlayerPersistenceError>;
```

### `crates/chunk-storage/src/level_dat.rs` (new)

```rust
use rc_nbt::{borrow, owned, schema::SchemaError};
use crate::player::PlayerPersistenceError;

/// The minimal `level.dat` `Data` compound M2 needs (Context's schema table); pure —
/// produces/consumes exactly the byte shape `AnvilDiskBackend::write_level_dat`/
/// `read_level_dat` (M2-B03) already expect and return (Context's "Sibling M2
/// blueprints"). Every other real field round-trips opaquely via `base`.
#[derive(Clone, Debug)]
pub struct LevelDat {
    pub data_version: i32,
    pub level_name: String,
    pub time: i64,
    pub last_played: i64,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub spawn_z: i32,
    pub spawn_angle: f32,
    pub version_name: String,
    pub version_snapshot: bool,
    pub version_series: String,
    base: owned::NbtCompound,
}

impl LevelDat {
    /// A brand-new world: `data_version = 4903` always, `time = 0`, `last_played`/
    /// `spawn`/`version_name` the caller-supplied values, `version_snapshot = false`,
    /// `base` empty.
    pub fn fresh_default(
        level_name: impl Into<String>,
        last_played_millis: i64,
        spawn: (i32, i32, i32, f32),
        version_name: impl Into<String>,
    ) -> Self;

    /// Decodes the **`Data`** sub-compound (the caller has already unwrapped the root's
    /// single `Data` child) into this blueprint's own fields, keeping the full `Data`
    /// compound as `base`.
    pub fn from_data_compound(data: &borrow::NbtCompound<'_, '_>) -> Result<Self, SchemaError>;

    /// `base.clone()` patched with this blueprint's own fields — returns the **`Data`**
    /// compound only, not the root.
    pub fn to_data_compound(&self) -> owned::NbtCompound;

    /// GZip-decompresses `bytes` and decodes the root's one `Data` child via
    /// `from_data_compound` — the exact inverse of `to_gzip_bytes`, and the exact shape
    /// `AnvilDiskBackend::read_level_dat`'s own return value should be run through.
    /// `PlayerPersistenceError::EmptyDocument` if the decompressed bytes decode to an
    /// empty NBT document; `SchemaError` (wrapped) if no `Data` child is present.
    pub fn from_gzip_bytes(bytes: &[u8]) -> Result<Self, PlayerPersistenceError>;

    /// Wraps `to_data_compound()` as the root's one `Data` child, GZip-compresses via
    /// `rc_nbt::write_gzip_owned` — the exact shape `AnvilDiskBackend::write_level_dat`'s
    /// own `payload` parameter expects.
    pub fn to_gzip_bytes(&self) -> Result<Vec<u8>, PlayerPersistenceError>;
}
```

### `crates/server/src/play/persistence.rs` (new — fully self-contained; does not import `super::world`/`super::connection`)

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use rc_chunk_storage::{
    load_player, save_player, LoadedPlayerRecord, PlayerDataStore, PlayerPersistenceError,
};

/// Default per-player save interval: `6000` ticks / 5 minutes (WORLD-D23's own
/// already-established default, Context) — used only when no `WorldConfig::
/// save_interval_ticks()` (M2-B05) is already present to reuse instead (Context's
/// "Composition-root integration").
pub const DEFAULT_SAVE_INTERVAL_TICKS: u64 = 6000;

#[derive(Clone, Copy, Debug)]
pub struct PlayerPersistenceConfig {
    pub save_interval_ticks: u64,
}

impl Default for PlayerPersistenceConfig {
    fn default() -> Self;
}

/// The live, currently-connected-players' working set (Context's "Load-on-join..."
/// subsection). `Clone`, cheap (`Arc`-backed). Fully self-contained: takes any
/// `Arc<dyn PlayerDataStore>` (real or fake), never `HardcodedWorld` or any
/// `rusty-clanker-server`-internal type — independently constructible and testable.
#[derive(Clone)]
pub struct PlayerSessionStore {
    store: Arc<dyn PlayerDataStore>,
    sessions: Arc<Mutex<HashMap<uuid::Uuid, LoadedPlayerRecord>>>,
}

impl PlayerSessionStore {
    pub fn new(store: Arc<dyn PlayerDataStore>) -> Self;

    /// Loads (or freshly defaults, via `LoadedPlayerRecord::fresh_default(dimension,
    /// default_pos)`) `uuid`'s record, inserts it into the live set, and returns a
    /// clone of its current `pos`/`rotation`.
    pub fn load_or_create(
        &self,
        uuid: uuid::Uuid,
        dimension: rc_core::DimensionId,
        default_pos: [f64; 3],
    ) -> Result<([f64; 3], [f32; 2]), PlayerPersistenceError>;

    /// Synchronously saves `uuid`'s current record and removes it from the live set. A
    /// no-op (`Ok(())`) if `uuid` is not currently present.
    pub fn save_and_remove(&self, uuid: uuid::Uuid) -> Result<(), PlayerPersistenceError>;

    /// Clones every currently-connected player's `(Uuid, LoadedPlayerRecord)` pair
    /// (a short-held lock) without removing anything from the live set.
    pub fn snapshot_all(&self) -> Vec<(uuid::Uuid, LoadedPlayerRecord)>;

    /// Saves every entry `snapshot_all` would return, logging (`tracing::warn!`, never
    /// panicking) on any individual failure.
    pub fn save_all(&self);

    /// Direct mutable access to one live record — this blueprint's own test-only
    /// stand-in for "the player's own action (block break, item pickup) changed their
    /// state" (Context/Constraints — implementing that action itself is `B07`'s job).
    /// `None` if `uuid` is not currently connected. Not `#[cfg(test)]`-gated (unlike an
    /// earlier draft of this blueprint): a real future mechanics system legitimately
    /// needs the identical direct-mutation access once it exists, so this is this
    /// blueprint's own small, intentionally-permanent public surface, not scaffolding
    /// to delete later.
    pub fn with_record_mut<R>(&self, uuid: uuid::Uuid, f: impl FnOnce(&mut LoadedPlayerRecord) -> R) -> Option<R>;
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/chunk-storage/src/{player.rs, level_dat.rs, lib.rs}` and `crates/server/src/play/persistence.rs` with every function body replaced with `todo!()` (fields/derives/doc comments unchanged), plus the two `Cargo.toml`/`lib.rs` edits and the one new `crates/server/src/play/mod.rs` line (`mod persistence; pub use persistence::{PlayerSessionStore, PlayerPersistenceConfig, DEFAULT_SAVE_INTERVAL_TICKS};` — purely additive, appended to whatever `mod.rs` already contains from M1-B05/M2-B05/M2-B07, none of whose own lines this blueprint touches). The implementation changeset (Implementation steps) fills in real bodies only; it must not modify any file under `crates/chunk-storage/tests/player_*.rs`, `crates/chunk-storage/tests/level_dat_*.rs`, or `crates/server/tests/play_persistence_*.rs`. This blueprint's own tests touch no file M2-B05/M2-B07 own (`play::world`/`play::connection`/`play::chunk`/`play::packets`/`play::keepalive`) at all — Context's own "Composition-root integration" recipe is guidance for a later reconciliation step, not something this blueprint's own test changeset exercises directly.

### `crates/chunk-storage/tests/player_nbt_known_answer.rs` — hand-derived byte vectors

1. `abilities_compound_known_bytes` — construct `PlayerAbilities { flying: true, fly_speed: 0.5, instabuild: false, invulnerable: true, may_build: false, may_fly: true, walk_speed: 1.0 }` (deliberately **not** MECH-D60's real survival defaults — `0.5`/`1.0` are chosen solely because they are exactly representable in IEEE-754 binary32 with no rounding, making this vector hand-derivable with full confidence; the real-defaults case is exercised separately by `player_data_roundtrip.rs`'s equality-based tests), hand-build the equivalent `owned::NbtCompound` via its seven fields in the exact key order Context's schema table gives (`flying, flySpeed, instabuild, invulnerable, mayBuild, mayfly, walkSpeed`), wrap as an unnamed root, `rc_nbt::write_owned`; assert the resulting bytes equal exactly this 97-byte vector (`0.5f32 = 0x3F000000`, `1.0f32 = 0x3F800000`, both well-known, rounding-free binary32 bit patterns):
```
0x0A,0x00,0x00,
0x01,0x00,0x06,0x66,0x6C,0x79,0x69,0x6E,0x67,0x01,
0x05,0x00,0x08,0x66,0x6C,0x79,0x53,0x70,0x65,0x65,0x64,0x3F,0x00,0x00,0x00,
0x01,0x00,0x0A,0x69,0x6E,0x73,0x74,0x61,0x62,0x75,0x69,0x6C,0x64,0x00,
0x01,0x00,0x0C,0x69,0x6E,0x76,0x75,0x6C,0x6E,0x65,0x72,0x61,0x62,0x6C,0x65,0x01,
0x01,0x00,0x08,0x6D,0x61,0x79,0x42,0x75,0x69,0x6C,0x64,0x00,
0x01,0x00,0x06,0x6D,0x61,0x79,0x66,0x6C,0x79,0x01,
0x05,0x00,0x09,0x77,0x61,0x6C,0x6B,0x53,0x70,0x65,0x65,0x64,0x3F,0x80,0x00,0x00,
0x00
```
(97 bytes total: 3-byte root header, then per key `[tag_id][name_len: u16 BE][name bytes][payload]` — `Byte` payloads 1 byte, `Float` payloads 4 big-endian bytes — terminated by one `0x00`.) Asserted with `assert_eq!` on the full `Vec<u8>`.
2. `item_stack_no_components_known_bytes` — `ItemStackRecord { id: "minecraft:stick".into(), count: 3, components: None }`; hand-build `{id: String("minecraft:stick"), count: Int(3)}` (no `components` key at all) as an unnamed root, `write_owned`; assert the resulting bytes equal exactly this 38-byte vector:
```
0x0A,0x00,0x00,
0x08,0x00,0x02,0x69,0x64,0x00,0x0F,0x6D,0x69,0x6E,0x65,0x63,0x72,0x61,0x66,0x74,0x3A,0x73,0x74,0x69,0x63,0x6B,
0x03,0x00,0x05,0x63,0x6F,0x75,0x6E,0x74,0x00,0x00,0x00,0x03,
0x00
```
(root header; `id` — `String` tag `0x08`, payload `[u16 BE MUTF-8 length = 15][15 ASCII bytes "minecraft:stick"]`; `count` — `Int` tag `0x03`, payload `Int(3)`; terminator.) This is the concrete byte-level proof that `components: None` never emits an empty `Compound` tag.

### `crates/chunk-storage/tests/player_data_roundtrip.rs`

1. `fresh_default_round_trips_through_save_then_load` — `LoadedPlayerRecord::fresh_default(DimensionId::OVERWORLD, [8.5, -59.0, -3.25])`; `to_nbt()` then `from_nbt()` on the result (via `write_owned`/`read_borrowed`/`.as_compound()`, mirroring M2-B02's own `ExamplePoint` round-trip shape); assert `decoded.data == original.data` field-by-field.
2. `every_modeled_field_survives_a_hand_populated_round_trip` — construct a `PlayerSaveData` with every field set to a distinctive, non-default value (`pos: [123.5, 64.0, -77.25]`, `motion: [0.1,-0.2,0.3]`, `rotation: [45.0, -12.5]`, `health: 13.5`, `food_level: 17`, `food_saturation_level: 2.5`, `food_exhaustion_level: 1.75`, `xp_level: 9`, `xp_p: 0.42`, `xp_total: 315`, `inventory: vec![InventorySlotEntry{slot:0, item: ItemStackRecord{id:"minecraft:diamond_sword".into(), count:1, components: Some(<hand-built compound with one Int "damage" entry = 5>)}}, InventorySlotEntry{slot:35, item: ItemStackRecord{id:"minecraft:cobblestone".into(), count:64, components: None}}]`, `selected_item_slot: 4`, `dimension: DimensionId::THE_NETHER`, `player_game_type: 2`, `previous_player_game_type: 0`, `abilities: PlayerAbilities{flying:true, fly_speed:0.1, instabuild:true, invulnerable:true, may_build:true, may_fly:true, walk_speed:0.2}`), wrap in a `LoadedPlayerRecord` with an empty `base`, round-trip through `to_nbt`/`from_nbt`; assert full equality, including that the `"minecraft:diamond_sword"` entry's `components` compound decodes back equal to the hand-built one and the `"minecraft:cobblestone"` entry's `components` is still exactly `None`.
3. `dimension_round_trips_for_all_three_vanilla_values` (table-driven) — `DimensionId::OVERWORLD` ↔ `"minecraft:overworld"`, `THE_NETHER` ↔ `"minecraft:the_nether"`, `THE_END` ↔ `"minecraft:the_end"`.
4. `unrecognized_dimension_string_is_a_schema_error` — hand-build a compound identical to a valid fresh-default record's own `to_nbt()` output except `Dimension` overwritten to `"minecraft:my_custom_dim"`; `from_nbt` returns `Err(SchemaError::InvalidValue { field: "Dimension", .. })`.
5. `unknown_field_preservation_survives_a_full_load_then_save_cycle` — hand-build a compound: every one of `fresh_default`'s own emitted fields, **plus** three fields this blueprint never models: `"foodTickTimer": Int(12)`, `"recipeBook": Compound({})`, `"Fire": Short(-20)`; `from_nbt` then `to_nbt`; assert the re-serialized compound still contains all three, unchanged.
6. `byte_level_idempotency_on_an_untouched_reload` — `to_nbt()`/`write_owned` a fresh-default; `from_nbt` the decoded result, then `to_nbt`/`write_owned` again with **no field mutated**; assert the second byte vector equals the first exactly.

### `crates/chunk-storage/tests/player_data_store_roundtrip.rs` — exercises the `PlayerDataStore` trait, both a fake and the real filesystem impl

`#[derive(Clone, Default)] struct FakeStore { entries: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<uuid::Uuid, Vec<u8>>>> }` implementing `PlayerDataStore` over a plain in-memory map — this test file's own dev-only helper, mirroring the `FakeBackend`/`MockTransport` convention M2-B03/M0-B05 already established for test-local trait fakes. `Arc`-wrapped specifically so `fake.clone()` shares the same underlying map (a shallow clone), matching `play_persistence_store.rs`'s own "simulate a restart by dropping and rebuilding a `PlayerSessionStore` over the same underlying data" usage of this exact type.

1. `load_player_returns_none_for_an_unknown_uuid` — empty `FakeStore`; `load_player(&fake, Uuid::new_v4())` returns `Ok(None)`.
2. `save_then_load_round_trips_through_the_fake_store` — `save_player(&fake, uuid, &record)` then `load_player(&fake, uuid)`; assert the loaded record's `data` equals the original's.
3. `filesystem_store_creates_the_players_data_directory_and_round_trips` — `FilesystemPlayerDataStore::new(<unique temp dir>)`; `save_player` then `load_player`; assert equality; assert `store.player_data_path(uuid)` exists on disk at exactly `<temp dir>/players/data/<uuid>.dat` (Context's Resolved discrepancy, verified at the actual path-string level, not just "the file exists somewhere").

### `crates/chunk-storage/tests/level_dat_roundtrip.rs`

1. `fresh_default_round_trips_through_gzip_bytes` — `LevelDat::fresh_default("New World", 1_700_000_000_000, (0, -59, 0, 0.0), "26.2")`; `to_gzip_bytes()` then `from_gzip_bytes()`; assert every field equals the original, and `data_version == 4903` unconditionally.
2. `unknown_top_level_data_fields_survive_round_trip` — hand-add a synthetic `"GameRules": Compound({"doDaylightCycle": String("true")})` entry to a fresh-default's own `Data` compound before decoding via `from_data_compound`; assert it survives a `from_data_compound`/`to_data_compound` cycle unchanged.
3. `gzip_bytes_are_actually_gzip_compressed` — `to_gzip_bytes()`'s first two bytes equal the GZip magic `[0x1F, 0x8B]` (a cheap, direct proof this blueprint's own bytes are exactly the shape `AnvilDiskBackend::write_level_dat`'s doc comment promises its caller supplies).

### `crates/chunk-storage/tests/item_stack_component_fixtures.rs` — synthetic golden fixture

1. `diamond_sword_with_enchantments_and_damage_round_trips` — a hand-built `components` compound shaped like a real 26.2 `diamond_sword`'s non-default patch would look (per `docs/research/mc-26.2/10-items-recipes-loot.md` §7's own cited data-generator example): one `Int` `"minecraft:damage" -> 3`, one nested `Compound` `"minecraft:enchantments"` containing a `Compound` `"levels"` with one `Int` entry `"minecraft:sharpness" -> 2` — wrapped in an `ItemStackRecord{id:"minecraft:diamond_sword", count:1, components: Some(..)}`; round-trip through a full `LoadedPlayerRecord` (one `InventorySlotEntry`) exactly as `player_data_roundtrip.rs`'s test 2, asserting the decoded `components` compound is `owned::NbtCompound`-equal to the original, structure and all — this crate never needs to know these keys mean "damage" or "sharpness 2" for the test to be a meaningful golden fixture.
2. `oracle_compatibility.rs`-style placeholder, `#[ignore]`d exactly per M2-B02's own established pattern:
```rust
#[ignore = "requires a vanilla-produced players/data/<uuid>.dat sample from rc-test-harness (TEST-D7), not yet implemented — see issue #<TRACKING_ISSUE, opened by the implementer at commit time>"]
#[test]
fn decodes_real_vanilla_player_dat_without_error() {
    let path = std::path::Path::new("oracle/26.2/harness/samples/players/data/sample.dat");
    let bytes = std::fs::read(path).expect("sample not present — see #[ignore] reason");
    let nbt = rc_nbt::read_gzip_owned(&bytes).expect("must decode a real vanilla player record cleanly");
    // Further field-level assertions deferred to whichever future blueprint first
    // wires rc-test-harness — this test's own job, today, is to exist and be
    // honestly skipped, exactly M2-B02's own precedent.
}
```

### `crates/server/tests/play_persistence_store.rs` — fully real, no `HardcodedWorld`, no TCP, no dependency on M2-B05/M2-B07's own contested shape

1. `join_disconnect_restart_round_trips_position_and_inventory` — construct a `FakeStore` (test-local, as `player_data_store_roundtrip.rs`'s own — reused via a small shared test helper crate-internal module, or duplicated verbatim, implementer's choice, both satisfy this test's own needs) shared behind one `Arc`; `let store1 = PlayerSessionStore::new(Arc::new(fake.clone()));` (`FakeStore` is `Clone`-shallow, sharing the same underlying `Mutex<HashMap<..>>` so a second `PlayerSessionStore` instance sees the first's writes — simulating "the same on-disk directory survives a process restart" without any real filesystem I/O). `store1.load_or_create(uuid, DimensionId::OVERWORLD, [0.0,-59.0,0.0])` (first-ever join — returns the default position); `store1.with_record_mut(uuid, |r| { r.data.inventory.push(InventorySlotEntry{slot:0, item: ItemStackRecord{id:"minecraft:diamond".into(), count:5, components:None}}); r.data.health = 7.5; r.data.pos = [42.0, 70.0, -13.0]; })` (this blueprint's own stand-in for "the player's own action changed their state" — Context/Constraints: real block-place/break is `B07`'s job); `store1.save_and_remove(uuid)` (the "logs off" half). **Simulate a server restart**: drop `store1`, construct `let store2 = PlayerSessionStore::new(Arc::new(fake));` (a fresh `PlayerSessionStore`, same underlying data). `store2.load_or_create(uuid, ..)` (the "rejoins" half); assert, via `store2.with_record_mut(uuid, |r| r.data.clone())`, that `pos == [42.0, 70.0, -13.0]`, `health == 7.5`, `inventory == vec![InventorySlotEntry{slot:0, item: ItemStackRecord{id:"minecraft:diamond".into(), count:5, components:None}}]` — byte-identical (field-equal) survival of both the position and inventory halves of M2's roadmap criterion 1, driven entirely by this blueprint's own code.
2. `same_scenario_against_a_real_filesystem_store` — as test 1, but `FilesystemPlayerDataStore::new(<unique temp dir>)` in place of `FakeStore` for both `store1`/`store2` — the identical scenario proven against real disk I/O, not only the in-memory fake, closing the gap between "the logic is right" and "the logic is right when real files are involved."

### `crates/server/tests/play_persistence_periodic_save.rs`

1. `save_all_persists_every_currently_connected_player` — `PlayerSessionStore` over a `FakeStore`; `load_or_create` for two distinct UUIDs; mutate each via `with_record_mut` to a distinctive `health` value; `save_all()` (without calling `save_and_remove` — both players stay "connected"); construct a **second** `PlayerSessionStore` over the same underlying `FakeStore` data and `load_or_create` each UUID again; assert both loaded `health` values match what was set, proving `save_all` persisted without requiring a disconnect.
2. `with_record_mut_returns_none_for_a_disconnected_uuid` — `with_record_mut` on a UUID never `load_or_create`d returns `None`, and does not panic.

## Implementation steps

1. **`rc-chunk-storage/Cargo.toml`.** Add the `flate2`/`thiserror`/`uuid` lines (or confirm M2-B03/M2-B01 have already added the first two, in which case only `uuid` is genuinely new). Observable: `cargo metadata` resolves.
2. **`player.rs` — `PlayerPersistenceError`, `PlayerDataStore`, `FilesystemPlayerDataStore`.** Trivial path-joining (`root.join("players").join("data").join(format!("{uuid}.dat"))`) and `thiserror`-derived bodies; `read_player_data`/`write_player_data` are the only two functions in this blueprint's own Deliverables that touch `std::fs` directly. Observable: compiles; `player_data_store_roundtrip.rs` test 3 passes.
3. **`player.rs` — `PlayerAbilities::default`, `LoadedPlayerRecord::fresh_default`.** Literal field-literal construction per Context's defaults table. Observable: `player_data_roundtrip.rs` test 1 compiles against real types.
4. **`player.rs` — `LoadedPlayerRecord::from_nbt`.** Use `NbtCompoundExt`'s `require_double`/`require_int`/`require_float`/`require_string`/`require_list`/`require_compound` per Context's schema table; `Pos`/`Motion` (`require_list` then `.doubles().ok_or(WrongType{..})?`, asserting exactly 3 elements, else `SchemaError::InvalidValue`); `Rotation` the same shape via `.floats()`, 2 elements; `Dimension` via `require_string` then the three-way match (Context) producing `SchemaError::InvalidValue { field: "Dimension", .. }` on no match; `Inventory` via `require_list` then `.compounds()`, one `InventorySlotEntry` per element (`require_byte` for `Slot`, `require_string` for `id`, `require_int` for `count`, `compound.compound("components")` — the plain `Option`-returning accessor, since this field is genuinely optional — `.map(|c| c.to_owned())` for `components`); `abilities` via `require_compound` then the same accessor pattern one level down. `base = compound.to_owned()`. Observable: `player_data_roundtrip.rs` tests 1-4 pass.
5. **`player.rs` — `LoadedPlayerRecord::to_nbt`.** `let mut out = self.base.clone();` then, in Context's own fixed field order, `out.insert("Pos", owned::NbtTag::List(owned::NbtList::Double(self.data.pos.to_vec())))` (and the analogous `Motion`/`Rotation`), `out.insert("Health", owned::NbtTag::Float(self.data.health))`, ... through `abilities` (a nested `owned::NbtCompound::from_values`), and `Inventory` (`owned::NbtTag::List(owned::NbtList::Compound(...))`, each entry's own `Slot`/`id`/`count`/optional-`components` inserted the same way). Observable: `player_data_roundtrip.rs` tests 5-6 and `player_nbt_known_answer.rs` pass.
6. **`player.rs` — `load_player`/`save_player`.** `load_player`: `store.read_player_data(uuid)?`; `None` → `Ok(None)`; `Some(bytes)` → GZip-decompress via `flate2::read::GzDecoder` + `read_to_end` (this crate's own direct `flate2` dependency, Deliverables), then `rc_nbt::read_borrowed_strict` on the decompressed bytes (this is the identical "decompress myself, then borrow-read" pattern this blueprint's own `LevelDat::from_gzip_bytes` also needs — Constraints notes this as a deliberate, minimal, twice-duplicated six-or-so lines rather than a request to extend M2-B02's own already-committed, protected public surface with a new `read_gzip_borrowed` entry point) → `.as_compound()` → `LoadedPlayerRecord::from_nbt`. `save_player`: `rc_nbt::write_gzip_owned(&owned::BaseNbt::new("", record.to_nbt()))` then `store.write_player_data(uuid, &bytes)`. Observable: `player_data_store_roundtrip.rs` tests 1-2 pass.
7. **`level_dat.rs`.** `from_data_compound`/`to_data_compound` directly analogous to `player.rs` steps 4-5, one level shallower; `fresh_default` per Context; `from_gzip_bytes`/`to_gzip_bytes` reuse the identical decompress-then-borrow-read / owned-then-compress pattern step 6 established, wrapping/unwrapping the root's one `Data` child. `data_version` always hardcoded `4903` on write regardless of `self.data_version`'s stored value. Observable: `level_dat_roundtrip.rs` passes.
8. **`rc-chunk-storage/src/lib.rs`.** Wire the two new `mod`/`pub use` blocks exactly as Deliverables. Observable: `cargo build -p rc-chunk-storage` succeeds with zero `todo!()` remaining.
9. **`crates/server/src/play/persistence.rs`.** `PlayerSessionStore::new`/`load_or_create`/`save_and_remove`/`snapshot_all`/`save_all`/`with_record_mut` per Deliverables' doc comments, each a short-held `Mutex::lock()` around a `HashMap` operation, disk/store I/O performed **outside** the lock wherever the two can be separated (`load_or_create`'s own read happens before the map insert; `save_and_remove`'s own write happens after a `remove` that returns an owned `LoadedPlayerRecord`, releasing the lock before the write starts). Observable: `play_persistence_store.rs` and `play_persistence_periodic_save.rs` both pass.
10. **`crates/server/src/play/mod.rs`.** Add the `mod persistence;` declaration and its `pub use` line — a two-line, purely additive edit appended to whatever this file already contains (M1-B05's own lines, plus whatever M2-B05/M2-B07 have already added). Observable: `cargo build -p rusty-clanker-server` succeeds; `play_persistence_*.rs` compile and pass without needing anything from `play::world`/`play::connection` at all.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
12. **Composition-root integration (not gated by this blueprint's own Tier-1 Done state, Context's own recipe restated as an action item for whoever performs the M2-wide reconciliation pass — M2-B05's or M2-B07's own implementer, or a dedicated later integration blueprint):** wire `PlayerSessionStore`/`FilesystemPlayerDataStore` into `play::world`/`play::connection` exactly per Context's "Composition-root integration" five-step recipe, once `HardcodedWorld`'s own final, reconciled shape (after M2-B05 and M2-B07 have both landed) is known.
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50) for this blueprint's own Tier-1 test suite (steps 1-11) — step 12 is explicitly out of this blueprint's own CI gate, per Context's own reconciliation stance.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/chunk-storage/tests/player_*.rs`/`level_dat_*.rs`/`item_stack_*.rs` and `crates/server/tests/play_persistence_*.rs` is committed first, alongside `todo!()`-stubbed `src/*.rs` files and the `Cargo.toml`/`lib.rs`/`mod.rs` edits. The implementation changeset fills in real bodies only — it must not edit any test file, must not weaken any assertion (in particular, every hand-derived byte vector in `player_nbt_known_answer.rs` must survive unchanged), and must not touch any file `play::world`/`play::connection`/`play::chunk`/`play::packets`/`play::keepalive` own.

(b) **No new external dependencies beyond `uuid`/`flate2`/`thiserror`** (all already workspace-pinned, Context) for `rc-chunk-storage`. `rusty-clanker-server`'s own `Cargo.toml` gains **no new line at all** from this blueprint (`PlayerSessionStore` depends only on `rc-chunk-storage`, already reachable) — a deliberate, notable difference from an earlier draft of this blueprint, which incorrectly assumed a new `rc-chunk-storage` dependency line was needed; it is not, since `rc-chunk-storage` is already a normal dependency of `rusty-clanker-server` via M2-B05/M2-B07's own already-landed additions, and if neither has landed yet, adding that one line is Implementation step 12's own reconciliation-time concern, not this blueprint's Tier-1-gated Deliverables. This blueprint's own tests build unique temp directories via plain `std::env::temp_dir()` + `uuid::Uuid::new_v4()` string formatting specifically to avoid needing `tempfile`.

(c) **No Mojang or third-party reimplementation code.** Every NBT key name, tag type, and folder-path fact this blueprint restates is sourced from `docs/research/mc-26.2/{04-persistence-nbt.md, 10-items-recipes-loot.md}` (produced under the ASSET-D18(f) reference-source policy), `docs/planning/{03-world-chunks-persistence.md, 05-game-mechanics.md}`'s own WORLD-D14/D15/D16 and MECH-D30/D47/D60, and live `minecraft.wiki`/docs.rs fetches performed while deriving this blueprint (2026-08-21, ASSET-D18(b)/(c)) — no decompiled source is quoted, and no third-party reimplementation's code is consulted or copied (ASSET-D18/D19/D30).

(d) **Item components stay opaque — no exception.** No special-cased handling for any specific `DataComponentType`, in any test fixture — every component payload is stored and compared as an undifferentiated `owned::NbtCompound`. Modeling any concrete component's semantics is `05-game-mechanics.md`'s MECH-D47 scope.

(e) **Scope boundary — do not implement beyond this blueprint's own stated Deliverables.** This blueprint does not implement: `equipment` (armor/offhand) modeling (Context's documented, bounded simplification); any real block place/break packet handling (`B07`'s scope — this blueprint's own round-trip tests use `PlayerSessionStore::with_record_mut` as a deliberate stand-in, and this blueprint does not attempt to duplicate `M2-B08`'s own real, end-to-end azalea-bot restart scenario, Context's "Sibling M2 blueprints" subsection); the chunk half of "configurable per-region save interval" (M2-B05's own, already-delivered scope); a literal `play::world.rs`/`play::connection.rs` file diff (Context's own reconciliation stance — Implementation step 12 states the integration recipe, but performing it against a specific, possibly-still-unsettled file is explicitly **not** part of this blueprint's own Tier-1 Done state); `RC-IoPool` (WORLD-D21) — this blueprint's own periodic-save sweep description (Context) uses a plain `std::thread::spawn`, explicitly flagged as a bounded simplification; the Anvil `.mca` region-file format, `level.dat`'s own file-level safe-write scheme (both M2-B03's, reused not reimplemented), `rc-test-harness`, or any part of the oracle-compatibility placeholder test's actual execution; any of MECH-D30's base-entity fields beyond `Pos`/`Motion`/`Rotation`, or any player-specific field beyond Context's own schema table. Do not add placeholder implementations of any of these as a shortcut.

(f) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

(g) **The duplicated gzip-decompress-then-borrow-read pattern (Implementation steps 6-7) is deliberate and bounded** — it exists only because `LoadedPlayerRecord::from_nbt`/`LevelDat::from_data_compound` are typed against `borrow::NbtCompound` (the zero-copy hot path WORLD-D11 mandates for reads) while `rc_nbt`'s own committed `read_gzip_owned` returns `owned::Nbt`. Do not "fix" this by adding a new `read_gzip_borrowed` entry point to `rc-nbt` itself in this blueprint's own changeset — `rc-nbt`'s public surface is M2-B02's protected file set; a future blueprint that wants that entry point genuinely shared is a small, separate, `rc-nbt`-owned addition.

(h) **Reconciliation stance is binding, not optional.** This blueprint's own Deliverables/Acceptance tests deliberately avoid claiming a specific, exact shape for `play::world.rs`/`play::connection.rs`/`HardcodedWorld` (Context) — do not "helpfully" write a literal diff against either M1-B05's original shape or a guessed M2-B05/M2-B07-merged shape as part of this blueprint's own implementation changeset; Implementation step 12 is explicitly a separate, later, non-Tier-1-gated action.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-chunk-storage -p rusty-clanker-server --all-features
cargo nextest run -p rc-chunk-storage
cargo nextest run -p rusty-clanker-server
cargo test --doc -p rc-chunk-storage -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-chunk-storage` reports `player_nbt_known_answer.rs` (2) + `player_data_roundtrip.rs` (6) + `player_data_store_roundtrip.rs` (3) + `level_dat_roundtrip.rs` (3) + `item_stack_component_fixtures.rs` (1 run + 1 skipped) = 15 run cases, 1 skipped, never silently passed. `cargo nextest run -p rusty-clanker-server` includes `play_persistence_store.rs` (2) and `play_persistence_periodic_save.rs` (2) — 4 new run cases, no new skips, and zero dependency on any file `M2-B05`/`M2-B07` own (so this blueprint's own Tier-1 gate is genuinely independent of whether either has landed yet). CI (`.github/workflows/ci.yml`, unmodified) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint. Implementation step 12's composition-root integration is confirmed separately, once `M2-B05`/`M2-B07` have both landed and been reconciled — not part of this blueprint's own CI gate (Constraints (h)).
