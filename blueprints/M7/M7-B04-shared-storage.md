# M7-B04 — Cluster Shared Storage (`rc-chunk-storage::cluster_storage`)

| Field | Content |
|---|---|
| ID | M7-B04 |
| Milestone | M7 — Cluster Mode Activation |
| Prerequisites | **M7-B02** (`rc-cluster` — `Epoch(pub u64)`, `RegionLease{region,node,epoch}`/`is_current`, `DirectoryCache::lease_of` — restated exactly, §B; this blueprint is the literal "future shared-storage-backend blueprint" M7-B02 §F names as `RegionLease::is_current`'s storage-layer counterpart). **M2-B03** (`rc-chunk-storage`'s already-shipped `ChunkStorageBackend` trait, `StorageError`, `RegionFileKind`, `CompressionScheme` — restated exactly, §C; this blueprint implements the trait without modifying its signature). **M2-B05** (the save pipeline's `Arc<dyn ChunkStorageBackend>` injection point in `ChunkLifecycleManager` — restated, §C, confirming zero change needed there). **M5-B09** (confirms worldgen's own `ChunkGenerator` seam is equally backend-agnostic — cited only to confirm no third call site needs updating). **Not a formal prerequisite, but directly informing this blueprint's design** (found already committed at derivation time, exactly the class of "sibling blueprint discovered on disk" M2-B06 already established a precedent for handling): **M7-B03** (`rc-cluster`'s `MigrationStore` trait, §E.2/E.3 — an `async`, opaque-`Vec<u8>`, `(RegionId, Epoch)`-keyed staging seam that M7-B03 §e explicitly names this blueprint as the future WORLD-D20-owning implementation of: *"a real `MigrationStore` bridging to `rc-chunk-storage`'s not-yet-built `ObjectStoreBackend`/WORLD-D20 staging path (a future `03-world-chunks-persistence.md`-owned blueprint, per `M7-B02`'s own identical exclusion)"* — restated in full, §H). |
| Implements | WORLD-D17 (`ChunkStorageBackend`'s second implementation, `ObjectStoreBackend`, over `object_store` 0.14.1 — version and current API re-verified against crates.io/docs.rs, §D). WORLD-D18 (per-`(ChunkKey, RegionFileKind)`-object layout, conditional-write fencing — restated and made concrete, §E/§F). WORLD-D19 (`RegionManifest`, dirty-tracking generations — restated and made concrete, §G). WORLD-D20 (the `postcard` staging format's storage-side read/write/delete primitives — restated, §H). WORLD-D21 (sync-trait/async-store bridging via an injected `tokio::runtime::Handle`, still RC-IoPool-only — restated as a binding calling convention, §I). WORLD-D23 (autosave-interval knob reused unmodified by cluster mode — restated, §J). CLUSTER-D17 (durability bound, cluster save-interval recommendation — restated and asserted by a dedicated acceptance test, §J). CLUSTER-D18 (shared-reachability + single-writer requirement — satisfied by construction, §A/§E). CLUSTER-D19 (epoch/lease fencing — the storage-side conditional-write check itself, made concrete as this blueprint's own algorithm since CLUSTER-D19 only fixes the requirement, §F). TEST-D45/D46 (test-first changeset boundary). TEST-D50 (CI-is-authority). |
| Crates touched | `rc-chunk-storage` (`crates/chunk-storage/`) only — one new module tree, `src/cluster_storage/`; two modified existing files, `Cargo.toml` (one new normal dependency, `object_store`) and `src/anvil/error.rs` (two additive `StorageError` variants, flagged and justified as a minimal, non-breaking touch to an existing shared seam per PLAN-D3, §A). **Not** `rc-cluster`, not `rusty-clanker-server`, not `rc-transport-net` — every one of those is touched only through the already-fixed `ChunkStorageBackend` trait (unmodified) or M7-B03's already-fixed `MigrationStore` trait (unmodified); the actual bridge code between the two is a future composition-root-extension blueprint's job, named explicitly at each point below (§A, §H, Constraints). |
| Estimated scope | L, explicitly exceeding `00-blueprint-spec.md`'s ~800-line/~300-line-Context guidance — the same class of stated, deliberate exception `M6-B07`/`M7-B01`/`M7-B02`/`M7-B03` already established. This is the one blueprint that fixes `rc-chunk-storage`'s complete cluster-mode object layout, its epoch-fencing algorithm (CLUSTER-D19 fixes only the *requirement*; the concrete conditional-write mechanism is this blueprint's own derivation against the pinned `object_store` 0.14.1 API, verified live against crates.io/docs.rs rather than assumed from training data — §D), and the `RegionManifest`/staging primitives three sibling blueprints (M7-B02, M7-B03, and a still-future composition-root blueprint) already depend on by name. Splitting the key layout from the fencing algorithm from the manifest format would force each half to restate the other's shared vocabulary (the embedded-epoch object framing, §E, is load-bearing for all three) from scratch. |

## Goal & Done definition

Give `rc-chunk-storage` `ObjectStoreBackend` — WORLD-D17's second `ChunkStorageBackend` implementation, satisfying CLUSTER-D18's shared-reachability/single-writer requirement over the `object_store` crate (S3-compatible object stores and shared-POSIX volumes, one dependency, restated §D) — with a concrete, race-free, storage-layer realization of CLUSTER-D19's epoch fencing (§F), WORLD-D19's `RegionManifest` (§G), and WORLD-D20's migration-staging primitives (§H), while leaving `AnvilDiskBackend`, the `ChunkStorageBackend` trait's signature, `ChunkLifecycleManager`, `TicketManager`, and every other M0–M6 seam **byte-for-byte untouched** except the two additive `StorageError` variants named above — the concrete, literal fulfillment of PLAN-D3's "cluster mode is ONLY... the storage-backend swap behind existing traits" for the storage domain specifically.

**Out of scope, explicitly** (every one a future blueprint's job, named precisely so the boundary is never ambiguous, mirroring M7-B02/M7-B03's own identical discipline): parsing CLUSTER-D27's `shared_storage` TOML/URI field into a concrete `Arc<dyn object_store::ObjectStore>` (a composition-root concern, §D, matching M7-B02's own "TOML parsing itself is out of this crate's scope" precedent exactly); a real implementation of M7-B03's `MigrationStore` trait (a thin, future composition-root-extension-blueprint adapter over this blueprint's own staging primitives, §H); a real per-region cluster-mode save orchestrator that decides *which* chunks belong to *which* `RegionId` and calls this blueprint's manifest-write primitive accordingly (§G — `ChunkStorageBackend`'s own fixed trait signature carries no region-boundary information at all, a genuine, flagged limitation this blueprint works around rather than silently assumes away, §G); CLUSTER-D16's takeover-algorithm decision logic (which live node gets a failed node's regions — M7-B02 §A item 2's own already-flagged exclusion; this blueprint supplies the manifest-guided *read* primitives such an algorithm calls, §G.4, not the algorithm itself); `rc-proxy`/CLUSTER-D20-D24 connection-routing concerns (unrelated to storage).

Done when:

- [ ] `cargo build -p rc-chunk-storage --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-chunk-storage`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds exactly one new dependency edge, `rc-chunk-storage --> object_store` (already workspace-pinned, §D); it adds **no** edge to `rc-cluster`, `rc-messaging`, or `rc-transport-net` (§A explains why none is needed, mirroring `12-workspace-structure.md`'s own fixed Dependency Graph, which draws no `storage --> cluster`/`storage --> msg` edge).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-chunk-storage` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).
- [ ] `git diff --stat` between this blueprint's pre- and post-implementation trees shows **zero** changed lines under `crates/chunk-storage/src/anvil/{mod,backend,region_file,compression,checksum}.rs` and under any `crates/chunk-storage/tests/anvil_*.rs` file — the literal, mechanically-checkable form of "monolithic genuinely unaffected" (CLUSTER-D26/D27), verified by a dedicated acceptance test (`cluster_storage_monolithic_unaffected.rs`, Acceptance tests) in addition to this structural diff check.

## Context (self-contained)

### §A — Scope boundary and the dependency-graph resolution this blueprint commits to

`12-workspace-structure.md`'s fixed Dependency Graph (WS-D3, restated exactly) draws `storage --> core`, `storage --> nbt`, `storage --> reg` and **no other edge** out of `rc-chunk-storage` — no `storage --> msg` (which owns `RegionId`), no `storage --> cluster` (which owns `Epoch`/`RegionLease`). `ChunkStorageBackend`'s trait signature (M2-B03, restated verbatim §C) already anticipated this exactly: every method's fencing parameter is a bare `epoch: Option<u64>`, never an `rc_cluster::Epoch` — M7-B02 §F says so explicitly: *"a future shared-storage-backend blueprint (`ObjectStoreBackend`, CLUSTER-D18) passes `Some(lease.epoch.0)` and its own conditional-write check is exactly `RegionLease::is_current`'s logic, re-implemented at the storage layer against whatever epoch it independently reads back."* This blueprint is that future blueprint, and it honors the boundary precisely: **`rc-chunk-storage`'s `Cargo.toml` gains exactly one new dependency, `object_store` — never `rc-cluster`, never `rc-messaging`.** Every `u64` this blueprint's own new types carry that *corresponds* to an `rc_cluster`/`rc_messaging` concept (an epoch, a region identifier) is this crate's own plain newtype or bare integer, bridged by whichever future composition-root code already depends on both sides — the exact "thin, free, single-field-copy conversion... belongs in whichever future crate legitimately depends on both" pattern `M2-B01` established for `BlockStateId`/`BiomeId` and `M2-B05` reused for the same reason (Context, "Superflat filler").

**Two genuine, pre-existing planning-corpus gaps this blueprint resolves on its own authority, flagged precisely rather than silently patched over** (the same class of resolution M7-B02 §A item 2 and M7-B03 §A already modeled for their own analogous discrepancies):

1. **WORLD-D19's `RegionManifest.region_id: RegionId` field names a type this crate cannot import** (§A above). Resolution: this blueprint's own `RegionManifest.region_id` field is a bare `u64` — numerically identical to `rc_messaging::RegionId(pub u64)`'s own inner value (M7-B02's header restates that exact shape), converted at the one call site that legitimately holds both types (a future composition-root blueprint). Restated in full at §G.
2. **`ChunkStorageBackend`'s trait signature carries no region-boundary information at all** — `read_chunk`/`write_chunk` take `(dim, kind, x, z, epoch)`, never a `RegionId`. WORLD-D19's own text ("rewritten... at the end of every save cycle that touches that region") presumes a caller who already knows which chunks belong to which region — a fact `ChunkStorageBackend`'s fixed signature cannot carry through the trait itself. Resolution: `RegionManifest` maintenance is **not** an automatic side effect of ordinary `write_chunk` calls (it cannot be, without either changing the trait signature — forbidden by PLAN-D3 — or having `ObjectStoreBackend` independently reconstruct ARCH-D6's dynamic, splittable/mergeable grid-cell-to-region mapping, which is `rc-scheduler`'s own live, in-memory state and not something a storage crate outside `SimServer`'s dependency reach can observe). Instead, `ObjectStoreBackend` exposes an **additive, non-trait inherent method** (`write_region_manifest`, mirroring `AnvilDiskBackend::write_chunks_batch`'s own already-established "PERF-relevant primitive, not part of `ChunkStorageBackend`" pattern from M2-B03) that a caller who *does* know the region boundary — a per-region cluster-mode save orchestrator, itself out of this blueprint's own scope (Goal & Done, "Out of scope") — calls once per save cycle, supplying exactly the `(ChunkKey, RegionFileKind)` pairs it just wrote. Restated in full at §G.

### §B — `rc-cluster`'s fencing primitives (M7-B02), restated exactly

```rust
pub struct Epoch(pub u64);                    // per-region, monotonic, starts at Epoch(1)
pub struct RegionLease { pub region: rc_messaging::RegionId, pub node: NodeId, pub epoch: Epoch }
impl RegionLease {
    pub fn is_current(&self, presented_epoch: Epoch) -> bool { self.epoch == presented_epoch }
}
```

A caller (a future cluster-mode `ChunkLifecycleManager`-equivalent) obtains a `RegionLease` from `ClusterNode::directory().lease_of(region)` before a save cycle and passes `Some(lease.epoch.0)` as every `write_chunk`/`write_region_manifest` call's `epoch` parameter for that cycle. **This blueprint never calls `RegionLease::is_current` itself** (it has no `RegionLease` value — §A) — it re-implements the *identical* "is the presented value still the most-recent one" check independently, against whatever epoch value it can durably observe in shared storage, which is the storage-layer half of exactly the same safety property, closing the residual "zombie old owner" gap M7-B02 §E already names as the reason this second check exists at all: *"safety comes from epoch-fencing at the point of use, never from read-time freshness."*

### §C — `ChunkStorageBackend` and its ecosystem (M2-B03/M2-B05), restated exactly

```rust
pub enum RegionFileKind { Terrain, Entities, Poi }
pub enum CompressionScheme { Zlib, Lz4, Uncompressed }   // .tag()/.compress()/decompress_tagged()
pub trait ChunkStorageBackend: Send + Sync + 'static {
    fn read_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, epoch: Option<u64>) -> Result<Option<Vec<u8>>, StorageError>;
    fn write_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, payload: &[u8], epoch: Option<u64>) -> Result<(), StorageError>;
    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError>;
    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>;
}
```

Every method is **synchronous**, called only from `RC-IoPool` (WORLD-D21) — never `async`, never RC-WorkerPool, never the Tokio runtime's own worker threads. `write_chunk`'s `payload` is raw, **uncompressed** NBT bytes (the backend compresses); `read_chunk`'s returned bytes are already **decompressed**. `epoch: Option<u64>` exists on every method "for signature compatibility... `AnvilDiskBackend` ignores it entirely" (M2-B03) — `ObjectStoreBackend` is the implementation that finally gives it meaning. `ChunkLifecycleManager` (M2-B05) holds this trait only as `Arc<dyn ChunkStorageBackend>`, injected once at construction, and never matches on concrete type — the swap from `AnvilDiskBackend` to `ObjectStoreBackend` requires **zero** change anywhere in `crates/chunk-storage/src/{io_pool,superflat,lifecycle}.rs`, `crates/scheduler/src/chunk_ticket.rs`, or `rc-worldgen`'s `ChunkGenerator` consumer (M5-B09, confirmed by its own text: *"`rc-chunk-storage` never depends on `rc-scheduler` or `rc-worldgen`... `ChunkLifecycleManager` already receives `Arc<dyn ChunkStorageBackend>`"*) — this is the literal, load-bearing proof that CLUSTER-D26/D27's "monolithic genuinely unaffected" promise holds for the storage domain by construction, not by discipline alone.

### §D — `object_store` 0.14.1: verified current API surface

`12-workspace-structure.md` pins `object_store = "0.14.1"` (WORLD-D17). **Re-verified live against crates.io and docs.rs at derivation time** (not assumed from training data, per this blueprint's own task brief): `0.14.1` (published 2026-07-15) is confirmed the current, latest release as of this writing (`0.14.0` → 2026-06-22 → `0.14.1`; `0.13.x` line is now superseded). The following is this blueprint's own restatement of the exact API surface it depends on — flagged, per §K, wherever the fetched documentation left a signature-level detail unconfirmed:

- **`ObjectStore` trait** (async, object-safe, `Send + Sync`): `async fn get(&self, location: &Path) -> Result<GetResult>`, `async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult>`, `async fn put_opts(&self, location: &Path, payload: PutPayload, opts: PutOptions) -> Result<PutResult>`, `async fn delete(&self, location: &Path) -> Result<()>`, plus list/copy/rename methods this blueprint does not use.
- **`PutOptions { mode: PutMode, tags: TagSet, attributes: Attributes, extensions: Extensions }`**, all four `Default`-able; this blueprint only ever sets `mode` (`PutOptions { mode, ..Default::default() }`).
- **`PutMode`**: `Overwrite` (always succeeds, replacing any existing object — this blueprint never uses this mode, since every write this blueprint makes must be fenced), `Create` (atomic create-if-absent, fails `Error::AlreadyExists` if the object already exists), `Update(UpdateVersion)` (atomic compare-and-swap: succeeds only if the object's current version matches the given token, else `Error::Precondition`).
- **`UpdateVersion { pub e_tag: Option<String>, pub version: Option<String> }`** — the CAS token, obtained from a prior `GetResult.meta`/`PutResult`'s own `ObjectMeta`. "Stores use differing combinations of `e_tag`/`version`... applications should preserve both" (docs.rs) — this blueprint always carries both fields through unexamined, never assumes only one is populated.
- **`GetResult { payload, meta: ObjectMeta, range, attributes: Attributes, extensions }`** — confirmed to carry both the CAS-relevant `meta` **and** any custom `Attributes` in one call, which is why this blueprint's own fencing reads use `get` (§E/§F), never `head` (whose `ObjectMeta`-only return type is not confirmed to also expose `Attributes` — §K item 1).
- **`Error`**: relevant variants `NotFound { path, source }`, `AlreadyExists { path, source }`, `Precondition { path, source }` (a conditional-write mode's precondition failed — this is the CAS-loss signal, §F), `Generic { store, source }`, `NotSupported`/`NotImplemented` (a backend does not implement the requested capability at all — this blueprint treats this as a hard, fail-fast configuration error, never a silent fallback, §D "shared-POSIX" below).
- **`PutPayload`**: `impl From<Vec<u8>> for PutPayload` (confirmed) — this blueprint constructs every payload via `PutPayload::from(bytes: Vec<u8>)`.
- **`Path`**: `Path::from(&str)` percent-encodes unsafe characters and normalizes leading/trailing/empty segments; multi-segment, negative-number-containing, dot-containing strings such as `"world/chunks/terrain/-5/-3/c.rcc"` round-trip through it without special handling (confirmed against docs.rs's own worked example).
- **`S3ConditionalPut`** (`object_store::aws`): `ETagMatch` (standard HTTP `If-Match`/`If-None-Match` preconditions — confirmed supported natively by AWS S3 itself since AWS added native conditional-write support in August 2024, well before this crate's own pin date, and by Cloudflare R2/MinIO) or `Disabled`. This blueprint requires whichever S3-compatible backend it is pointed at to be configured with `ETagMatch` (or the crate's own default, if `ETagMatch` is already default for `AmazonS3Builder` as of 0.14.1 — §K item 4) — **construction of the `AmazonS3Builder` itself is a composition-root concern** (§A, Goal & Done), this blueprint only requires that whatever `Arc<dyn ObjectStore>` it is handed actually honors `PutMode::Create`/`PutMode::Update`, verified at `ObjectStoreBackend::open` time (§F, "capability probe").
- **`LocalFileSystem`** (`object_store::local`): `new()`/`new_with_prefix(prefix)`; internally rename-based atomic writes; directory `fsync` only on Unix (a no-op on Windows). **Its `PutMode::Create`/`PutMode::Update` conditional-write support is not confirmed as reliably present or race-free across all platforms/mount types by the fetched documentation** (§K item 3) — this blueprint does **not** silently assume `LocalFileSystem` gives real CAS guarantees. CLUSTER-D18 itself names "a self-hosted MinIO instance" as one of its own listed shared-storage shapes *alongside* "an equivalent shared POSIX volume" — this blueprint's own resolution: an operator whose shared-storage transport is a bare NFS/POSIX mount is expected to front it with a local MinIO instance (an S3-compatible server processes already recommend running colocated with the volume it serves) to get `ObjectStoreBackend`'s full epoch-fencing guarantee; `LocalFileSystem` remains usable directly only where the capability probe (§F) confirms real conditional-write support, and `ObjectStoreBackend::open` **fails fast**, loudly, rather than silently degrading to unfenced writes, if it does not (§F).
- **`object_store::memory::InMemory::new()`** — a full, in-process reference `ObjectStore` implementation; this blueprint's own fencing property tests and manifest tests use it for fast, deterministic, network-free CI runs (Acceptance tests) — its own conditional-put semantics are the crate's own reference behavior and are treated as trustworthy for testing the *algorithm* (§F), independent of the platform-specific `LocalFileSystem`/S3 questions above.

### §E — Object key layout (WORLD-D18, restated and made exact)

WORLD-D18's own illustrative key, restated verbatim: `world/<dim>/chunks/<x>/<z>/terrain.nbt.zz`. This blueprint's exact, binding scheme, one object per `(DimensionId, RegionFileKind, x, z)` — **never a literal `.mca` file** (WORLD-D18):

```
world/chunks/<dim_slug>/<kind_slug>/<x>/<z>/c.rcc
```

where `dim_slug` ∈ `{overworld, the_nether, the_end}` (`fn dim_slug(dim) -> Result<&'static str, StorageError>`, the exact same three built-in dimensions `AnvilDiskBackend::dimension_folder` already restricts itself to — anything else is `StorageError::UnsupportedDimension`, the identical variant `AnvilDiskBackend` already returns, reused unmodified) and `kind_slug` ∈ `{terrain, entities, poi}` (`RegionFileKind::folder_name()`, **already public** on the crate-root-exported type — reused directly, zero duplication). `c.rcc` (a made-up, unambiguously-ours extension — "Rusty Clanker Chunk") is a **fixed** filename regardless of compression scheme, because the compression tag and the fencing epoch both live in a small binary header **inside** the object body (below), not the key — this is a deliberate, cited divergence from WORLD-D18's own `.nbt.zz`-varies-by-scheme illustration, chosen because it removes any need to know an object's compression scheme before reading its key, and because embedding the epoch in the body (not as backend-specific `Attributes` metadata, §K item 1) is what makes this blueprint's fencing algorithm portable across every `ObjectStore` backend uniformly, including ones whose `Attributes` support is unconfirmed.

**Object body format**, every per-chunk object this blueprint writes:

```
[epoch: u64, big-endian, 8 bytes] [compression_tag: u8, 1 byte] [compressed NBT payload: remaining bytes]
```

The 9-byte header is this blueprint's own framing, analogous in spirit (never in bytes — ASSET-D18/D19/D30) to WORLD-D12's own per-record `[length][compression_tag]` sub-header — WORLD-D18's "the NBT payload inside each object is byte-identical to what a vanilla Anvil chunk tag would contain" refers to the **decompressed, de-framed** payload (exactly as WORLD-D12 itself never claims a raw Anvil *record's* bytes equal NBT bytes either, only its decompressed payload does) — `read_chunk` strips and validates this header before returning, so a caller never observes it.

`level.dat`: `world/level.dat` (a single, non-chunk, non-fenced-by-this-mechanism object per §J). `RegionManifest`: `world/manifests/<region_id>.postcard` (§G — **no** `<dim>` segment, a deliberate divergence from WORLD-D19's own illustrative `world/<dim>/manifests/<region_id>.postcard`, because M7-B03's own `MigrationStore`/manifest-adjacent call sites supply only a bare region identifier, never a `DimensionId`, and `RegionId` values are already globally unique across the whole world including every dimension per ARCH-D24's addressing scheme — including `<dim>` would require information this blueprint's own callers do not have and does not improve uniqueness). Staging: `world/staging/<region_id>-<epoch>.postcard` (§H, the identical reasoning for omitting `<dim>`).

### §F — Epoch fencing: the concrete, race-free conditional-write algorithm

CLUSTER-D19 fixes the *requirement* ("the storage backend rejects a write tagged with a stale epoch via a conditional-write check") without fixing the mechanism ("the exact mechanism per storage type... verify current S3 CAS support" — this blueprint's own assigned task). The algorithm below is this blueprint's own derivation, using exactly the primitives §D verified exist in `object_store` 0.14.1, applied uniformly to chunk objects, the region manifest, and (§J) an inherent, non-trait `level.dat` variant — one shared function, `fencing::write_fenced`, three call sites.

**Why a single-object CAS, not a separate "epoch marker" object.** An earlier design considered a dedicated per-region marker object, claimed once via CAS, with ordinary unconditional overwrites of the chunk body following it. That design has a real race: a marker-CAS success does not atomically extend to the *separate* chunk-body `put` that follows it — two legitimate writers (or a writer and a slow zombie) can interleave between the two calls, letting an older write silently land after a newer one. This blueprint instead performs the epoch comparison and the conditional write **on the same object, in the same `put_opts` call's precondition**, which is the only construction `object_store`'s own CAS primitive makes atomic.

**In-process version cache** (per `ObjectStoreBackend` instance): `HashMap<Path, CachedVersion { epoch: u64, update_version: object_store::UpdateVersion, confirmed_at: Instant }>`, guarded by `parking_lot::Mutex` (ARCH-D23's own already-established lock-usage convention, reused). This cache exists purely to skip a redundant `get` before a write this same process already knows is safe — it is **never** the sole source of truth for "is this epoch stale" (that would defeat fencing's whole purpose the moment two processes are involved), and every entry is revalidated against real storage no less often than `epoch_revalidation_interval` (default `30s`, deliberately matching CLUSTER-D17's own recommended cluster save-interval ceiling — reusing an already-established number rather than inventing a second one, §J) — bounding, to that same explicit, operator-visible, configurable duration, the window during which a zombie writer holding a warm cache entry for a key it already wrote once this process's lifetime could keep re-writing that *one* key without a fresh storage-level check. A zombie's cache can never contain an entry above its own frozen epoch value (§F "algorithm", below), so this bound is a **defense-in-depth** cap on an already-narrow, already-flagged residual gap, not the sole line of defense — the primary defense is that every *first* touch of a key in any given process's lifetime, and every touch past the revalidation window, always re-checks storage before writing.

**Algorithm** (`fencing::write_fenced(store, runtime, cache, key, presented_epoch, body) -> Result<UpdateVersion, StorageError>`, called with `body` already containing its own correct 8-byte epoch prefix per §E — the caller, `ObjectStoreBackend::write_chunk`/`write_region_manifest`/`write_level_dat_fenced`, builds `body` fresh with `presented_epoch` baked in before every call, including a retry):

```
fn write_fenced(store, cache, key, presented_epoch, body_builder):
    // body_builder: fn(header_epoch: u64) -> Vec<u8> — rebuilds the object body with the
    // correct epoch prefix on every attempt, since a retry may still use presented_epoch
    // unchanged (only the CAS token changes across retries).
    if let Some(cached) = cache.get(key), cached.epoch == presented_epoch,
            cached.confirmed_at.elapsed() < revalidation_interval:
        match store.put_opts(key, body_builder(presented_epoch),
                              PutOptions { mode: Update(cached.update_version.clone()), .. }):
            Ok(result) => { cache.insert(key, Confirmed{epoch: presented_epoch, update_version: result.into(), confirmed_at: now}); return Ok(result.into()) }
            Err(Precondition{..}) => {} // cache was stale relative to storage; fall through
            Err(other) => return Err(StorageError::Backend{path: key, source: other})

    for _attempt in 0..CAS_RETRY_LIMIT:        // default 8
        match store.get(key):
            Ok(existing) =>
                current_epoch = read_u64_be(existing.payload.bytes()[0..8])  // Corrupt if <8 bytes
                if presented_epoch < current_epoch:
                    return Err(StorageError::StaleEpoch{path: key, presented: presented_epoch, current: current_epoch})
                match store.put_opts(key, body_builder(presented_epoch),
                                      PutOptions { mode: Update(existing.meta.into()), .. }):
                    Ok(result) => { cache.insert(...); return Ok(result.into()) }
                    Err(Precondition{..}) => continue  // lost the race; loop re-reads fresh
                    Err(other) => return Err(StorageError::Backend{path: key, source: other})
            Err(NotFound{..}) =>
                match store.put_opts(key, body_builder(presented_epoch), PutOptions { mode: Create, .. }):
                    Ok(result) => { cache.insert(...); return Ok(result.into()) }
                    Err(AlreadyExists{..}) => continue  // someone created it concurrently; loop re-reads
                    Err(other) => return Err(StorageError::Backend{path: key, source: other})
    return Err(StorageError::Backend{path: key, source: <fencing retries exhausted, formatted>})
```

Every branch either durably succeeds (cache updated with the real, storage-confirmed `UpdateVersion`), rejects on a **provably** stale epoch (a fresh `get` observed a strictly newer epoch than the one presented — the exact CLUSTER-D19 "zombie" case), or retries on a benign lost race (two writers presenting the *same or compatible* epoch briefly overlapped — never silently drops or corrupts anything, since every `put_opts` call is itself atomic-or-nothing by `object_store`'s own contract). **No torn state is ever observable**: a rejected or retried write leaves the object's previously-committed bytes completely unchanged (a `put_opts` that returns `Err` never partially writes — this is `object_store`'s own atomicity guarantee, restated), and a successful write's result is exactly the bytes just sent, in full, or the call would not have returned `Ok`.

**Capability probe** (`ObjectStoreBackend::open`, §I): before returning, attempts one throwaway `put_opts(probe_key, empty_payload, PutMode::Create)` followed by one `put_opts(probe_key, empty_payload, PutMode::Update(that_version))`, then deletes the probe object. `Err(NotSupported | NotImplemented)` from either call is a **hard, fail-fast** `StorageError::Backend` returned from `open` itself — `ObjectStoreBackend` never silently starts up in an unfenced mode (§D, "shared-POSIX").

### §G — `RegionManifest` (WORLD-D19), restated and made concrete

```rust
pub struct ObjectVersion { pub e_tag: Option<String>, pub version: Option<String> }  // == object_store::UpdateVersion, field-for-field
pub struct RegionManifest {
    pub region_id: u64,          // §A item 1 — numerically == rc_messaging::RegionId.0
    pub epoch: u64,
    pub last_saved_tick: u64,
    pub chunk_object_versions: std::collections::HashMap<(rc_core::ChunkKey, RegionFileKind), ObjectVersion>,
}
```

**§G.1 — write cadence and merge semantics ("dirty-tracking accuracy").** `write_region_manifest(region_id, epoch, last_saved_tick, touched: &[(ChunkKey, RegionFileKind)])` is called once per save cycle by a region-scoped caller (§A item 2), listing only the chunks **that cycle actually wrote** — not the region's entire chunk set. Algorithm: (1) `read_region_manifest(region_id)` — `None` on a brand-new region; (2) start from its existing `chunk_object_versions` map (empty if none existed); (3) for every `(key, kind)` in `touched`, look up its just-confirmed `UpdateVersion` from `fencing`'s own in-process cache (warm, since the caller only ever lists keys it just wrote via `write_chunk` moments earlier in the same cycle — a cache miss falls back to one fresh `get`, handling a caller that lists an already-durable-but-not-freshly-written key) and **overwrite** that one map entry; (4) every other, previously-recorded entry is carried forward **unchanged**; (5) bump `epoch`/`last_saved_tick` to the call's own values; (6) `postcard::to_allocvec` the result and write it through `fencing::write_fenced` at `world/manifests/<region_id>.postcard`, with the SAME 8-byte-epoch-prefix framing §E defines (the manifest is itself an ordinary fenced object — CLUSTER-D18's single-writer requirement applies to it exactly as to chunk data). This merge-not-replace rule is what makes the manifest an accurate, incrementally-maintained "which chunks does this region durably have, and at what version" index without ever requiring a caller to enumerate a whole region's chunk set on every cycle — directly answering the "dirty-tracking accuracy" acceptance requirement.

**§G.2 — read path, generation-based cache-validity signal.** `read_region_manifest(region_id) -> Result<Option<RegionManifest>, StorageError>` — an ordinary, **unfenced** read (§F's algorithm is a write-time concern only; reading a manifest never needs an epoch, matching `read_chunk`'s own already-established epoch-agnostic read discipline, M2-B03). `RegionManifest.chunk_object_versions`'s `ObjectVersion` values **are** WORLD-D19's own "generation" concept made concrete: a consumer that separately caches a chunk's bytes alongside the `ObjectVersion` it was read at can detect staleness by comparing that remembered value against the manifest's current entry — a small (single-digit-KB) manifest fetch instead of re-fetching a chunk's full (potentially much larger) body just to check "did this change." This blueprint exposes the comparison data; it does not itself build a read-through cache consuming it (no such consumer exists yet in the corpus — an explicit, named future-work item, Open Issues).

**§G.3 — cluster-mode-only.** `write_region_manifest`/`read_region_manifest` exist **only** on `ObjectStoreBackend` (inherent, not part of `ChunkStorageBackend`) — `AnvilDiskBackend` has no manifest concept and gains none (monolithic mode has no takeover to guide, and `AnvilDiskBackend`'s own Anvil `.mca` files are themselves the durable record of "which chunks exist").

**§G.4 — manifest-guided takeover-resume, the exact sequence (restating "the exact resume sequence" this blueprint was assigned to fix).** CLUSTER-D16's takeover *algorithm* (which live node gets a failed node's regions) is explicitly a sibling, not-yet-written blueprint's job (M7-B02 §A item 2) — this is the **read-side sequence that algorithm's eventual implementation calls into**, using only primitives this blueprint already ships:

1. The newly-assigned node's `ClusterNode` observes a committed `AssignRegion{region, node=self}` (raft), yielding `RegionLease{region, node, epoch: E_new}` (M7-B02).
2. Composition-root code calls `ObjectStoreBackend::read_region_manifest(region_id)`. `None` — a brand-new, never-before-saved region; nothing to resume, chunks are generated on first ticket-driven demand exactly as WORLD-D22 already specifies for a fresh monolithic world. `Some(manifest)` — an existing region; `manifest.epoch` is always `<= E_new` by construction (epochs are strictly monotonic, and `E_new` was only just granted).
3. **No eager bulk load happens here.** The manifest's chunk list is not itself loaded — WORLD-D22's own ticket-driven, on-demand `read_chunk` path (unmodified, Context §C) is what actually loads a chunk, the first time some `Player`/other ticket needs it, exactly as in monolithic mode. The manifest's role at this step is purely informational/bookkeeping: confirming the region is non-empty, and (optionally) feeding CLUSTER-D24's pre-warm hint (M7-B03's own already-fixed text: pre-warm reads "the *ordinary*, already-durable canonical objects... not the migration-only staging path" — i.e., ordinary `read_chunk` calls seeded from the manifest's key list, ahead of actual player demand).
4. The new owner's **first write** to any chunk in this region (e.g., a player's block edit) goes through the ordinary `write_chunk(epoch=Some(E_new))` path. Because this process's own fencing cache starts cold, §F's algorithm takes its "slow path": a fresh `get` observes the chunk's embedded epoch left over from the *previous* owner's last write (some `E_old < E_new`), confirms `E_new >= E_old`, and wins the CAS — this is the literal moment the new owner becomes the storage-confirmed writer for that specific key, with zero additional machinery beyond what §F already provides.

This sequence requires no new method beyond `read_region_manifest` plus the ordinary trait methods — direct evidence this blueprint's primitive set is sufficient for the resume path CLUSTER-D16 needs, without this blueprint building the orchestration around it.

### §H — Migration-staging primitives (WORLD-D20), reconciled against M7-B03's already-fixed `MigrationStore`

M7-B03 (found already committed, §A "Prerequisites") already fixes the trait a future composition-root bridge implements:

```rust
// rc-cluster, M7-B03, restated exactly — NOT this blueprint's own type, shown for reconciliation only.
pub trait MigrationStore: Send + Sync + 'static {
    fn write_staging(&self, region: RegionId, epoch: Epoch, payload: &RegionSnapshotPayload) -> impl Future<Output = Result<(), MigrationError>> + Send;
    fn read_staging(&self, region: RegionId, epoch: Epoch) -> impl Future<Output = Result<Option<RegionSnapshotPayload>, MigrationError>> + Send;
    fn delete_staging(&self, region: RegionId, epoch: Epoch) -> impl Future<Output = Result<(), MigrationError>> + Send;
}
```

`MigrationStore` is `async`, keyed by `rc-cluster`'s own `RegionId`/`Epoch`/`RegionSnapshotPayload(Vec<u8>)` types, and lives in `rc-cluster` — this blueprint neither implements it (wrong crate, wrong async model, §A) nor depends on its types. Instead, `ObjectStoreBackend` ships the **storage-side primitive** a future composition-root adapter implements `MigrationStore` in terms of — synchronous (matching every other `ObjectStoreBackend` method, §C), opaque-bytes, bare-`u64`-keyed:

```rust
impl ObjectStoreBackend {
    pub fn write_staging(&self, region_id: u64, epoch: u64, payload: &[u8]) -> Result<(), StorageError>;
    pub fn read_staging(&self, region_id: u64, epoch: u64) -> Result<Option<Vec<u8>>, StorageError>;
    pub fn delete_staging(&self, region_id: u64, epoch: u64) -> Result<(), StorageError>;
}
```

at `world/staging/<region_id>-<epoch>.postcard` (§E). **Unlike chunk/manifest writes, staging writes are deliberately unfenced** (plain `PutMode::Overwrite`, no epoch-embedded-header, no `fencing::write_fenced` involvement) — this is a considered choice, not an oversight: a staging blob's safety already comes entirely from M7-B03's own six-phase `MigrationCoordinator` protocol ordering (freeze → serialize → **stage** → epoch-bump → destination-restore → source-cleanup, M7-B03 §E.3) — the SOURCE node is always the sole writer of a given `(region, epoch)` staging key (it is written *before* the epoch bump that would let a second node believe it owns the region), and the DESTINATION reads/deletes it only *after* observing that same epoch's commit, per M7-B03's own already-fixed ordering. Re-deriving a second fencing check here would duplicate a guarantee the calling protocol already provides, contradicting nothing this blueprint's own chunk/manifest fencing does (which protects a genuinely-concurrent, ongoing, multi-writer-candidate resource, unlike a single-produce/single-consume/delete staging blob). The future `MigrationStore`-bridge blueprint converts `region.0`/`epoch.0`/`payload.0`↔`Vec<u8>` at the one call site that holds both `rc_cluster` and `rc_chunk_storage` types — the identical bridging pattern §A already establishes for `RegionManifest.region_id`.

### §I — Sync/async bridging (WORLD-D21, restated as a binding calling convention)

`object_store::ObjectStore` is `async`; `ChunkStorageBackend`'s methods are `fn`, not `async fn` (WORLD-D17, unmodified). `ObjectStoreBackend::open` therefore takes a `tokio::runtime::Handle` (the identical pattern `M7-B01`'s `NetworkTransport::new` already established for the same reason) and every trait/inherent method body wraps its `object_store` calls in `self.runtime.block_on(async move { ... })`. This is safe **only** when called from a plain OS thread that is not itself a worker of the *same* Tokio runtime whose `Handle` was captured — exactly `RC-IoPool`'s own shape (WORLD-D21: `std::thread::spawn`-created workers, never Tokio-managed) — calling a `ChunkStorageBackend` method from within an `async fn` running on that same runtime would panic ("Cannot start a runtime from within a runtime"). This is not a new constraint this blueprint introduces — WORLD-D17 already restricts every `ChunkStorageBackend` call to `RC-IoPool` exclusively; this blueprint simply notes why that existing restriction is now load-bearing for correctness, not merely for tick-budget hygiene, restated as a binding Constraint below.

### §J — `level.dat`/player-data custody in cluster mode (restated, per `13-cluster-architecture.md`'s own request)

Neither `03-world-chunks-persistence.md` nor `13-cluster-architecture.md` names a custody rule for these two file classes in cluster mode — this blueprint supplies one, flagged as its own binding resolution rather than presumed pre-existing:

**`level.dat`.** `ChunkStorageBackend::write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>` (M2-B03, unmodified) carries **no epoch parameter at all** — a genuine, structural limitation of the already-fixed trait signature (not something this blueprint can retrofit without violating PLAN-D3). This blueprint's resolution: **custody is enforced by caller discipline, one level above the trait** — only composition-root code that has just confirmed (via `ClusterNode::raft().metrics()`, M7-B02) that this node is the *current* raft leader ever calls the trait's `write_level_dat`; raft's own single-leader property (verified, M7-B02 §H) is the sole exclusion mechanism for this one file, reusing an authority the cluster control plane already provides rather than inventing a second one keyed to a resource (`level.dat`) that has no natural `RegionId` of its own. For defense-in-depth beyond bare caller discipline, `ObjectStoreBackend` additionally exposes a **non-trait, inherent, fenced variant**, `write_level_dat_fenced(&self, payload: &[u8], cluster_config_epoch: u64) -> Result<(), StorageError>`, keyed by `rc_cluster::ClusterConfigEpoch` (M7-B02 §B: the one cluster-wide counter, incremented on every raft-applied entry including leadership changes) rather than a per-region `Epoch` — using the identical §F algorithm, at the fixed key `world/level.dat` with the same 8-byte-header framing. A future composition-root blueprint may call the fenced variant instead of the trait's own plain one wherever it already has a `ClusterConfigEpoch` in hand; both remain available, and the trait-conformant plain path is what any code written generically against `Arc<dyn ChunkStorageBackend>` (e.g., a future M2-B06-cluster-analog) actually gets.

**Player data.** `PlayerDataStore` (M2-B06) is a **separate trait**, deliberately independent of `ChunkStorageBackend` ("player files are not part of WORLD-D17's trait," M2-B06's own text) — this blueprint does not implement a cluster-mode `PlayerDataStore` (no such deliverable was assigned to this blueprint's crate/scope, and `FilesystemPlayerDataStore`'s own local-disk implementation is the only one that exists in the corpus today). This blueprint's own restated custody rule, for a **future** `ObjectStorePlayerDataStore` sibling to implement: a player's data is owned by whichever node currently owns the `RegionId` containing that player's entity (the same lease-holder-writes rule §F already establishes for chunk data, since a player is an ordinary ARCH-D24-addressed entity like any other) — a transient, moving custody exactly tracking the player's own region residency, requiring no new mechanism beyond reusing this blueprint's own `fencing::write_fenced` against a `players/<uuid>.dat`-shaped key. **Flagged explicitly as a gap this blueprint does not close** (Open Issues) — restating the rule without building the trait implementation, since `PlayerDataStore`'s own crate placement (`rc-chunk-storage`, per M2-B06's own header) means a future blueprint can add it as one more sibling module in this same crate without touching this blueprint's own deliverables.

### §K — Moderate-confidence flags and reconciliation steps (verify at implementation time)

Mirroring M7-B02 §I's own established convention — each independently low-risk (caught by `cargo build` or a failing acceptance test, never a silent correctness bug):

1. **Whether `ObjectStore::head`'s returned `ObjectMeta` also exposes custom `Attributes`.** Unconfirmed by the fetched documentation (§D). Irrelevant to this blueprint's own algorithm as specified (§F uses `get`, which is confirmed to return both `meta` and `attributes` in one call, and this blueprint does not use `Attributes` for the epoch at all — §E embeds it in the body instead, precisely to avoid depending on this uncertain detail) — listed here only so a future optimization pass (using a cheaper `head`-only check where the body needn't be re-read) is not attempted on a false assumption.
2. **The exact construction API for `object_store::Attributes`/`Attribute`.** Not needed by this blueprint's own design (§E) — listed only because it was investigated and found ambiguous during derivation; a future revision that wants to move the epoch into metadata instead of a body header should re-verify this first.
3. **`LocalFileSystem`'s exact `PutMode::Create`/`PutMode::Update` support level and cross-platform/NFS-mount atomicity.** Not confirmed reliably present by the fetched documentation, and third-party crates exist specifically to add conditional-write support `LocalFileSystem` itself may lack — this blueprint's own capability probe (§F) is the authoritative, implementation-time answer: `ObjectStoreBackend::open` fails fast if the configured store does not honor real `Create`/`Update` semantics, rather than this blueprint asserting an answer either way.
4. **Whether `S3ConditionalPut::ETagMatch` is `AmazonS3Builder`'s own default as of 0.14.1**, given AWS S3 itself has supported native `If-None-Match`/`If-Match` since August 2024. If it is not yet the crate's own default, the composition-root code that builds the `AmazonS3Builder` (§A, out of this blueprint's scope) must set it explicitly — this blueprint's own capability probe (§F) catches an unconfigured/misconfigured builder either way, since an unfenced `Overwrite`-only store would fail the probe's `PutMode::Create` step with `NotSupported`.
5. **`object_store::Error`'s exact variant field shapes** (`Precondition{path, source}` vs. a differently-shaped tuple/struct variant) — this blueprint's algorithm (§F) only ever matches on the variant's *discriminant* (`Precondition`/`AlreadyExists`/`NotFound`/other), never destructures field names beyond what every fetched signature already confirms (`path`), so a field-shape mismatch is a compile error, never a silent logic error.
6. **`object_store`'s exact `[features]` table** (Deliverables, `Cargo.toml`) names `"aws"` with high but not certain confidence as the feature gating the S3-compatible builder module — a one-line, build-time-caught verification against the pinned `0.14.1` manifest, not a design-level risk.
7. **`rc-chunk-storage` compiling `object_store` unconditionally**, never behind a Cargo feature the way `rc-cluster`/`rc-transport-net`/`rc-proxy` sit behind WS-D5(a)'s `cluster` gate — a genuinely minimal, from-source monolithic-only build (`--no-default-features --features monolithic`, WS-D5(a)) still links `object_store` even though it never constructs `ObjectStoreBackend`. This is not a defect this blueprint introduces (WORLD-D17 itself describes `ObjectStoreBackend` as runtime-selected, never naming a compile-time gate the way CLUSTER-D26 does for `NetworkTransport`) — whether `rc-chunk-storage` deserves its own optional feature mirroring WS-D5(d)'s already-established `io_uring` precedent is a question for a future revision of `12-workspace-structure.md`, not a change this blueprint makes unilaterally to a fixed planning document.

## Deliverables

### `crates/chunk-storage/Cargo.toml` (modify — add one new normal dependency; every existing line from M0-B01/M2-B01/M2-B03/M2-B04/M2-B05 unchanged)

```toml
[dependencies]
# ...every existing line unchanged (rc-core, rc-nbt, rc-registries, bevy_ecs, flate2,
# lz4_flex, parking_lot, thiserror, crossbeam-channel, postcard, serde, io-uring[optional])...
object_store = { workspace = true, features = ["aws"] }   # this blueprint — WORLD-D17/CLUSTER-D18
tokio = { workspace = true }                                # this blueprint — §I bridging (already
                                                              # workspace-pinned by ARCH-D21/NET-D7;
                                                              # first direct use inside this crate)
```

(`features = ["aws"]` enables the S3-compatible builder module `object_store::aws` this blueprint's own capability discussion, §D, refers to — covering AWS S3, Cloudflare R2, Backblaze B2, and MinIO alike, since all four speak the same S3 API; `LocalFileSystem` requires no feature flag. Verify this feature name against the pinned `0.14.1` manifest's own `[features]` table before finalizing — §K's own class of check, not separately numbered since it is a one-line, build-time-caught verification.)

### `crates/chunk-storage/src/anvil/error.rs` (modify — two additive `StorageError` variants; every existing line, including all nine current variants, unchanged)

```rust
// ... every existing variant (Io, Corrupt, SectorOutOfBounds, UnknownCompressionType,
// Decompress, InvalidNbtPayload, MissingExternalFile, WorldAlreadyOpen,
// UnsupportedDimension) unchanged ...

/// This blueprint's own addition (WORLD-D17/CLUSTER-D19) — a shared-storage write whose
/// presented epoch is strictly older than the epoch a fresh read observed already
/// durably recorded. `AnvilDiskBackend` never constructs this variant (it ignores
/// `epoch` entirely, M2-B03) — purely additive, confirmed non-breaking because nothing
/// in this workspace exhaustively matches `StorageError`'s variants (M2-B05's own text:
/// "propagated via `#[from]`, never matched on a specific variant").
#[error("shared-storage epoch fencing rejected a write at {path}: presented epoch {presented} is older than the currently recorded epoch {current}")]
StaleEpoch { path: String, presented: u64, current: u64 },

/// This blueprint's own addition — an `object_store` operation failed for a reason
/// other than a fencing rejection (network error, backend-reported `NotSupported`
/// during the capability probe, retry-limit exhaustion, malformed object body).
#[error("shared-storage backend error at {path}: {source}")]
Backend { path: String, #[source] source: Box<dyn std::error::Error + Send + Sync> },
```

(`path: String`, not `PathBuf` — `object_store::path::Path`'s own `Display`/`AsRef<str>` output is the natural fit here, unlike the existing nine variants' local-filesystem `PathBuf`s; `source: Box<dyn std::error::Error + Send + Sync>` rather than a direct `object_store::Error` field, so `crates/chunk-storage/src/anvil/error.rs` — a file `AnvilDiskBackend` also lives beside — does not need to import `object_store` types into a module conceptually shared by both backends; `ObjectStoreBackend`'s own code performs the `object_store::Error -> Box<dyn Error + Send + Sync>` conversion via ordinary `Box::new`/`.into()` at each call site.)

### `crates/chunk-storage/src/lib.rs` (modify — add one module declaration/re-export; every existing line from M0-B01 through M2-B06/M5-B09 unchanged)

```rust
pub mod cluster_storage;

pub use cluster_storage::{ObjectStoreBackend, ObjectVersion, RegionManifest};
```

### `crates/chunk-storage/src/cluster_storage/mod.rs` (new)

```rust
//! `ObjectStoreBackend` — WORLD-D17's cluster-mode `ChunkStorageBackend` implementation
//! (CLUSTER-D18), its epoch-fencing algorithm (CLUSTER-D19), `RegionManifest`
//! (WORLD-D19), and migration-staging primitives (WORLD-D20). See this crate's owning
//! blueprint, M7-B04, for the full design rationale and scope boundary — in particular,
//! this module never gains a dependency on `rc-cluster`/`rc-messaging` (M7-B04 §A).

mod backend;
mod fencing;
mod keys;
mod manifest;
mod staging;

pub use backend::{ObjectStoreBackend, ObjectStoreBackendConfig};
pub use manifest::{ObjectVersion, RegionManifest};
```

### `crates/chunk-storage/src/cluster_storage/keys.rs` (new)

```rust
use object_store::path::Path;
use rc_core::{ChunkKey, DimensionId};
use crate::{RegionFileKind, StorageError};

/// The three built-in dimensions' object-key slug (§E) — the identical restriction
/// `AnvilDiskBackend::dimension_folder` already applies, restated for this module's own
/// independent key scheme.
pub fn dim_slug(dim: DimensionId) -> Result<&'static str, StorageError>;

/// `world/chunks/<dim_slug>/<kind_slug>/<x>/<z>/c.rcc` (§E).
pub fn chunk_object_path(dim: DimensionId, kind: RegionFileKind, x: i32, z: i32) -> Result<Path, StorageError>;

/// `world/level.dat` (§J).
pub fn level_dat_path() -> Path;

/// `world/manifests/<region_id>.postcard` (§G, no `<dim>` segment — §E's rationale).
pub fn manifest_path(region_id: u64) -> Path;

/// `world/staging/<region_id>-<epoch>.postcard` (§H).
pub fn staging_path(region_id: u64, epoch: u64) -> Path;
```

### `crates/chunk-storage/src/cluster_storage/fencing.rs` (new)

```rust
use std::sync::Arc;
use std::time::{Duration, Instant};
use object_store::{path::Path, ObjectStore, PutMode, PutOptions, PutPayload, UpdateVersion};
use parking_lot::Mutex;
use std::collections::HashMap;
use crate::StorageError;

/// This blueprint's own conditional-write algorithm (§F) — the ONE place the
/// epoch-comparison-plus-CAS logic exists, shared by chunk writes, the region
/// manifest, and the fenced `level.dat` variant.
#[derive(Clone)]
struct CachedVersion { epoch: u64, update_version: UpdateVersion, confirmed_at: Instant }

/// Per-`ObjectStoreBackend`-instance, in-process only (§F) — never the sole fencing
/// authority, always subject to `revalidation_interval` re-checks against real storage.
pub struct VersionCache {
    entries: Mutex<HashMap<Path, CachedVersion>>,
    revalidation_interval: Duration,
}

impl VersionCache {
    /// `revalidation_interval` default `Duration::from_secs(30)` (§F — matches
    /// CLUSTER-D17's own recommended cluster save-interval ceiling).
    pub fn new(revalidation_interval: Duration) -> Self;
}

pub const CAS_RETRY_LIMIT: u32 = 8;

/// §F's exact algorithm. `runtime.block_on`s internally (§I) — callers never see an
/// `async fn`. `body_builder` rebuilds the full object body (§E's 9-byte header plus
/// payload) fresh on every attempt, since only the CAS token changes across a retry,
/// never the epoch a given call presents.
pub fn write_fenced(
    store: &Arc<dyn ObjectStore>,
    runtime: &tokio::runtime::Handle,
    cache: &VersionCache,
    key: &Path,
    presented_epoch: u64,
    body_builder: impl Fn(u64) -> Vec<u8>,
) -> Result<UpdateVersion, StorageError>;

/// The capability probe (§F, "Capability probe") — called once from
/// `ObjectStoreBackend::open`. `probe_key` is a fixed, reserved path
/// (`world/.rc-capability-probe`) this blueprint owns exclusively.
pub fn probe_conditional_write_support(
    store: &Arc<dyn ObjectStore>,
    runtime: &tokio::runtime::Handle,
) -> Result<(), StorageError>;
```

### `crates/chunk-storage/src/cluster_storage/manifest.rs` (new)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use object_store::ObjectStore;
use rc_core::ChunkKey;
use crate::{RegionFileKind, StorageError};
use super::fencing::VersionCache;

/// Field-for-field identical in shape to `object_store::UpdateVersion` (§D) — this
/// blueprint's own type so `manifest.rs` does not need `object_store` types in its own
/// public surface beyond what `backend.rs` already imports.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObjectVersion { pub e_tag: Option<String>, pub version: Option<String> }

/// WORLD-D19's manifest, restated exactly (§G). `region_id` is a bare `u64` — §A item 1
/// explains why this crate cannot name `rc_messaging::RegionId` directly.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RegionManifest {
    pub region_id: u64,
    pub epoch: u64,
    pub last_saved_tick: u64,
    pub chunk_object_versions: HashMap<(ChunkKey, RegionFileKind), ObjectVersion>,
}

/// §G.1's exact read-merge-write algorithm. Not `pub` at module level — re-exported only
/// through `ObjectStoreBackend`'s own inherent methods (Deliverables, `backend.rs`).
pub(super) fn write_region_manifest(
    store: &Arc<dyn ObjectStore>,
    runtime: &tokio::runtime::Handle,
    cache: &VersionCache,
    region_id: u64,
    epoch: u64,
    last_saved_tick: u64,
    touched: &[(ChunkKey, RegionFileKind)],
) -> Result<RegionManifest, StorageError>;

pub(super) fn read_region_manifest(
    store: &Arc<dyn ObjectStore>,
    runtime: &tokio::runtime::Handle,
    region_id: u64,
) -> Result<Option<RegionManifest>, StorageError>;
```

### `crates/chunk-storage/src/cluster_storage/staging.rs` (new)

```rust
use std::sync::Arc;
use object_store::ObjectStore;
use crate::StorageError;

/// §H's exact, deliberately-unfenced staging primitives — `pub(super)`, exposed publicly
/// only through `ObjectStoreBackend`'s own inherent methods.
pub(super) fn write_staging(store: &Arc<dyn ObjectStore>, runtime: &tokio::runtime::Handle, region_id: u64, epoch: u64, payload: &[u8]) -> Result<(), StorageError>;
pub(super) fn read_staging(store: &Arc<dyn ObjectStore>, runtime: &tokio::runtime::Handle, region_id: u64, epoch: u64) -> Result<Option<Vec<u8>>, StorageError>;
pub(super) fn delete_staging(store: &Arc<dyn ObjectStore>, runtime: &tokio::runtime::Handle, region_id: u64, epoch: u64) -> Result<(), StorageError>;
```

### `crates/chunk-storage/src/cluster_storage/backend.rs` (new)

```rust
use std::sync::Arc;
use std::time::Duration;
use object_store::ObjectStore;
use rc_core::{ChunkKey, DimensionId};
use crate::{ChunkStorageBackend, CompressionScheme, RegionFileKind, StorageError};
use super::{fencing::VersionCache, manifest::RegionManifest};

/// `ObjectStoreBackend`'s construction-time configuration. Building the concrete
/// `Arc<dyn ObjectStore>` (an `AmazonS3Builder`, a `LocalFileSystem`, or an
/// `object_store::memory::InMemory` for tests) is a **composition-root concern** (§A,
/// §D) — this blueprint accepts an already-constructed store, never parses a URI itself.
pub struct ObjectStoreBackendConfig {
    pub store: Arc<dyn ObjectStore>,
    /// Applied to every chunk this instance writes (WORLD-D13's own scheme enum, reused
    /// unmodified) — existing objects written under a different scheme remain correctly
    /// readable regardless (§E's header always names the scheme actually used).
    pub compression: CompressionScheme,
    /// §F — default `Duration::from_secs(30)`.
    pub epoch_revalidation_interval: Duration,
}

impl Default for ObjectStoreBackendConfig {
    /// `store` has NO default (a caller must always supply one) — this impl exists only
    /// to give `compression`/`epoch_revalidation_interval` their stated defaults when a
    /// caller uses struct-update syntax (`ObjectStoreBackendConfig { store, ..Default::default() }`
    /// is **not** directly expressible since `store` has no default value; callers
    /// construct the struct with every field named instead — this `Default` impl is
    /// provided for the two defaultable fields' own documented values to be
    /// programmatically inspectable, e.g. by this blueprint's own tests).
    fn default() -> Self;
}

/// WORLD-D17's cluster-mode `ChunkStorageBackend` implementation (CLUSTER-D18). Not
/// `Clone` — share via `Arc`, exactly `AnvilDiskBackend`'s own convention.
pub struct ObjectStoreBackend {
    // private: store: Arc<dyn ObjectStore>, compression: CompressionScheme,
    // runtime: tokio::runtime::Handle, cache: VersionCache
}

impl ObjectStoreBackend {
    /// §F's capability probe runs here, synchronously, before this returns — a
    /// misconfigured/unfenced-capable `config.store` is a hard `StorageError::Backend`
    /// from `open` itself, never a later silent failure (§D, §F).
    pub fn open(config: ObjectStoreBackendConfig, runtime: tokio::runtime::Handle) -> Result<Self, StorageError>;

    /// §G.1 — merge-and-write. `touched` lists exactly the chunks THIS call's caller
    /// just wrote via ordinary `write_chunk` (§A item 2 — this blueprint does not, and
    /// structurally cannot, discover this list on its own).
    pub fn write_region_manifest(&self, region_id: u64, epoch: u64, last_saved_tick: u64, touched: &[(ChunkKey, RegionFileKind)]) -> Result<RegionManifest, StorageError>;
    /// §G.2 — unfenced read.
    pub fn read_region_manifest(&self, region_id: u64) -> Result<Option<RegionManifest>, StorageError>;

    /// §H — the storage-side primitives a future `rc_cluster::MigrationStore` bridge
    /// adapts to. Deliberately unfenced (§H's own rationale).
    pub fn write_staging(&self, region_id: u64, epoch: u64, payload: &[u8]) -> Result<(), StorageError>;
    pub fn read_staging(&self, region_id: u64, epoch: u64) -> Result<Option<Vec<u8>>, StorageError>;
    pub fn delete_staging(&self, region_id: u64, epoch: u64) -> Result<(), StorageError>;

    /// §J — the fenced, non-trait `level.dat` variant, keyed by `ClusterConfigEpoch`
    /// (bare `u64` here, §A) rather than a per-region `Epoch`.
    pub fn write_level_dat_fenced(&self, payload: &[u8], cluster_config_epoch: u64) -> Result<(), StorageError>;

    /// PERF-relevant coalescing (mirrors `AnvilDiskBackend::write_chunks_batch`'s own
    /// already-established "additive, non-trait" shape, M2-B03) — dispatches every
    /// entry's underlying `write_fenced` call CONCURRENTLY within one `block_on`
    /// (`futures::future::join_all`-shaped), rather than sequentially, since an object
    /// store's bottleneck is per-call network round-trip latency, not a single shared
    /// file lock the way `AnvilDiskBackend`'s own batching primitive addresses — the
    /// same "batching," a genuinely different mechanism for a genuinely different
    /// resource. Every entry must share the same `epoch` (`debug_assert!`-checked,
    /// mirroring `AnvilDiskBackend::write_chunks_batch`'s identical same-group
    /// precondition) — collects and returns the FIRST error encountered, if any, after
    /// every dispatched write has been awaited (never leaves a write silently
    /// in-flight-and-unawaited on an early return).
    pub fn write_chunks_batch(&self, dim: DimensionId, kind: RegionFileKind, entries: &[(i32, i32, &[u8])], epoch: u64) -> Result<(), StorageError>;
}

impl ChunkStorageBackend for ObjectStoreBackend {
    /// Epoch-agnostic (§F, §G.4) — `epoch` is accepted for trait-signature compatibility
    /// and ignored, the same documented asymmetry `AnvilDiskBackend::read_chunk` already
    /// has toward its own (there, entirely inert) `epoch` parameter, restated here for a
    /// deliberate reason instead of an absent one: reads must succeed regardless of
    /// which epoch's data they observe, since a takeover-resuming node's very first read
    /// of a chunk necessarily predates its own first fenced write to it (§G.4 step 4).
    fn read_chunk(&self, dim: DimensionId, kind: RegionFileKind, x: i32, z: i32, epoch: Option<u64>) -> Result<Option<Vec<u8>>, StorageError>;
    /// `epoch: None` is a hard `StorageError::Backend` (a caller-error — cluster-mode
    /// writes MUST present an epoch; `epoch: None` has no meaning here the way it does
    /// for `AnvilDiskBackend`'s own single-process-local disk, which never has a
    /// concurrent-writer scenario an epoch could usefully fence against — restated,
    /// Constraints).
    fn write_chunk(&self, dim: DimensionId, kind: RegionFileKind, x: i32, z: i32, payload: &[u8], epoch: Option<u64>) -> Result<(), StorageError>;
    /// Unfenced, trait-conformant (§J) — custody enforced by caller discipline alone at
    /// this call path.
    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError>;
    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>;
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary**, restated exactly per TEST-D45/D46 and M2-B03's own identical precedent: every file under `crates/chunk-storage/tests/cluster_storage_*.rs` is committed first, alongside `todo!()`-stubbed `src/cluster_storage/{mod,backend,fencing,keys,manifest,staging}.rs` (full struct fields, trait impls, derives, and doc comments already final) and the `Cargo.toml`/`lib.rs`/`anvil/error.rs` edits (the two new `StorageError` variants are themselves part of the test-authoring changeset, since acceptance tests reference them directly). The implementation changeset fills in real bodies only — it must not edit any test file, must not add/remove/rename any test case, and (restated from Goal & Done) must leave `src/anvil/{mod,backend,region_file,compression,checksum}.rs` and every `tests/anvil_*.rs` file with **zero** changed lines.

### `crates/chunk-storage/tests/cluster_storage_backend_contract.rs`

A fresh, generic-shaped suite (not a modification of M2-B03's own `AnvilDiskBackend`-concrete tests, which stay untouched — Constraints) exercising the same semantic properties M2-B03 established for the trait, against `ObjectStoreBackend` built over `object_store::local::LocalFileSystem` pointed at a `TempWorldDir` (reusing M2-B03's own `tests/support/mod.rs` helper, imported via `mod support;` exactly as every `anvil_*.rs` file already does):

1. `chunk_round_trips_through_the_trait_with_real_compression` — write a small, valid, hand-built NBT compound's encoded bytes (via `rc_nbt::owned` + `write_owned`, mirroring M2-B03's own identical test) through `write_chunk(..., epoch: Some(1))`; `read_chunk` at the same key returns `Ok(Some(bytes))` equal to the original uncompressed bytes.
2. `read_chunk_on_a_never_written_key_returns_none_without_creating_an_object` — fresh backend; `read_chunk` at a key nothing has ever written; `Ok(None)`; a subsequent `store.list()` (via the same underlying `LocalFileSystem`, held by the test directly) shows no object exists at that key.
3. `unsupported_dimension_is_rejected` — `write_chunk(DimensionId(999), .., epoch: Some(1))` returns `Err(StorageError::UnsupportedDimension(_))`.
4. `write_chunk_with_epoch_none_is_a_hard_error` — `write_chunk(.., epoch: None)` returns `Err(StorageError::Backend { .. })` (Deliverables' own stated asymmetry vs. `AnvilDiskBackend`).
5. `level_dat_round_trips` — `write_level_dat(bytes)`; `read_level_dat()` returns `Ok(bytes)` exactly.
6. `read_level_dat_on_a_fresh_backend_errors` — no `write_level_dat` ever called; `read_level_dat()` returns `Err(_)` (either `NotFound`-shaped `Backend` or an equivalent — both acceptable, mirroring `AnvilDiskBackend`'s own case 9's "either acceptable" phrasing).
7. `open_fails_fast_against_a_store_with_conditional_writes_disabled` — construct `ObjectStoreBackend::open` against a small test-only `ObjectStore` wrapper (a fake implementing only `Overwrite`-mode puts, returning `Err(NotSupported)` for `Create`/`Update` — this test's own fixture, not a real backend) — `open` itself returns `Err(StorageError::Backend { .. })`, never `Ok`.

### `crates/chunk-storage/tests/cluster_storage_fencing.rs`

Against `object_store::memory::InMemory` (§D — deterministic, network-free, full CAS reference semantics):

1. `first_write_at_a_new_key_always_succeeds` — `write_chunk(.., epoch: Some(1))` on a never-before-touched key succeeds regardless of epoch value (even a large one) — there is nothing to be stale relative to.
2. `equal_or_increasing_epoch_writes_always_succeed_in_sequence` — a `proptest!` property test: for any strictly-non-decreasing sequence of epochs `e_1 <= e_2 <= ... <= e_n` (generated via `proptest`'s own sorted-vector strategy), issuing `write_chunk` with each epoch in order to the SAME key always returns `Ok(())`, and a final `read_chunk` returns exactly the last write's payload.
3. `strictly_decreasing_epoch_is_always_rejected` — a `proptest!` property test: for any sequence where at some point a write presents an epoch strictly less than the maximum epoch already successfully written to that key, that specific write returns `Err(StorageError::StaleEpoch { presented, current, .. })` with `presented` and `current` matching the test's own tracked expected values exactly, **and** a subsequent `read_chunk` shows the object's content is still exactly what the last successful (higher-epoch) write produced — never the rejected write's own payload, never a mix of the two ("no torn state").
4. `concurrent_writers_at_the_same_epoch_never_corrupt_the_object` — `std::thread::scope` spawns `8` threads, each sharing one `ObjectStoreBackend`, each attempting `write_chunk(.., epoch: Some(5))` with a distinct, thread-tagged payload to the SAME chunk key; every thread's call either succeeds or fails with a benign retry-exhaustion `Backend` error (never `StaleEpoch`, since all epochs are equal); after joining, a final `read_chunk` returns EXACTLY one of the 8 threads' own payloads in full, never a byte-level mix.
5. `revalidation_ttl_forces_a_fresh_check_past_the_configured_window` — construct a backend with `epoch_revalidation_interval: Duration::from_millis(10)`; write once at epoch `5` (cache now warm); directly manipulate the underlying `InMemory` store (bypassing `ObjectStoreBackend` entirely, simulating a second process's own successful write) to bump the stored epoch to `6`; sleep past the `10ms` window; attempt a THIRD-process-shaped write at the original backend's own stale epoch `5` — asserted `Err(StorageError::StaleEpoch { presented: 5, current: 6, .. })`, proving the cache's TTL correctly forced a fresh check rather than trusting its own now-stale `5`-confirmed cache entry.

### `crates/chunk-storage/tests/cluster_storage_manifest.rs`

Against `object_store::memory::InMemory`:

1. `manifest_round_trips_byte_exact_through_postcard` — construct a `RegionManifest` by hand (a handful of synthetic `ChunkKey`/`RegionFileKind`/`ObjectVersion` entries), `write_region_manifest` it (via the real chunk-writing path first, so the referenced keys' versions are genuine), `read_region_manifest` it back; every field matches exactly.
2. `read_region_manifest_on_a_never_written_region_returns_none` — fresh backend; `read_region_manifest(42)` is `Ok(None)`.
3. `manifest_merges_new_entries_without_dropping_prior_ones` (the "dirty-tracking accuracy" case) — write chunks A, B via `write_chunk`, call `write_region_manifest(region, epoch=1, tick=100, touched=[A,B])`; write a THIRD chunk C, call `write_region_manifest(region, epoch=1, tick=200, touched=[C])` (deliberately NOT re-listing A/B); `read_region_manifest` shows all three of A, B, C present, with A/B's entries **unchanged** from the first call and C's entry reflecting the second — proving the merge-not-replace rule.
4. `manifest_entry_version_changes_when_the_chunk_is_rewritten` — write chunk A, manifest it; rewrite A with different bytes at a higher epoch, manifest it again (`touched=[A]`); the manifest's A-entry's `ObjectVersion` differs from the first manifest's A-entry (proving version tracking actually reflects the latest write, not a stale snapshot).
5. `manifest_write_is_itself_epoch_fenced` — write a manifest at epoch `5`; attempt to write a manifest for the SAME region at epoch `3`; returns `Err(StorageError::StaleEpoch { .. })`, and `read_region_manifest` still shows the epoch-`5` manifest's own content unchanged.

### `crates/chunk-storage/tests/cluster_storage_takeover_resume.rs`

Simulating CLUSTER-D17's durability bound directly (against `object_store::memory::InMemory`):

1. `acknowledged_writes_survive_a_simulated_mid_batch_kill_with_zero_corruption` — write chunks A, B, C successfully (`Ok`) at epoch `1`; deliberately never attempt chunk D (simulating "the process died before this write was even issued"); call `write_region_manifest(region, epoch=1, tick=X, touched=[A,B,C])` (D is never listed, since it was never written); simulate "process restart" by constructing a FRESH `ObjectStoreBackend` handle over the SAME underlying `InMemory` store; `read_region_manifest` returns exactly A/B/C's entries (never D); `read_chunk` for each of A/B/C returns EXACTLY its own originally-written bytes, byte-for-byte; `read_chunk` for D returns `Ok(None)` (never a corrupt/partial value — it was simply never written, the exact, bounded, documented loss shape CLUSTER-D17 describes: "bounded by time since that region's last successful persisted save").
2. `resuming_writer_at_a_higher_epoch_wins_every_previously_written_key` (§G.4's exact resume sequence, end-to-end) — write chunks A, B at epoch `1` (simulating the old owner); construct a SECOND `ObjectStoreBackend` handle (simulating the new owner's own fresh process, cold cache) over the same store; `read_region_manifest` (step 2 of §G.4) succeeds and lists A, B; `write_chunk` A at epoch `2` through the new handle (step 4 of §G.4) succeeds; a further attempt to `write_chunk` B at epoch `1` through the ORIGINAL (old-owner-simulating) handle now fails `Err(StorageError::StaleEpoch { presented: 1, current: 1, .. })` for A already... (test asserts precisely: writing A at epoch 1 via the OLD handle fails since A is now at epoch 2; B is still readable at its original epoch-1 content since nothing rewrote it yet) — proving the exact "zombie old owner, first touch of any key it attempts, gets caught" property §F/§G.4 describe.
3. `bounded_loss_window_matches_cluster_d17s_recommended_interval` — construct a backend with the DEFAULT `epoch_revalidation_interval` (`Duration::from_secs(30)`, Deliverables); assert this equals `Duration::from_secs(30)` directly (a literal, load-bearing assertion that this blueprint's own chosen default is exactly CLUSTER-D17's own recommended cluster save-interval ceiling, restated and pinned by a test rather than left as prose alone).

### `crates/chunk-storage/tests/cluster_storage_monolithic_unaffected.rs`

1. `anvil_disk_backend_round_trip_is_identical_after_this_blueprint_lands` — a single, direct re-exercise of `AnvilDiskBackend`'s own already-proven round-trip (open a fresh backend in a `TempWorldDir`, write a chunk, read it back, assert equality) — imported via the SAME crate-root path M2-B03 already established (`rc_chunk_storage::{AnvilDiskBackend, ChunkStorageBackend, CompressionScheme}`), proving `ObjectStoreBackend`'s addition changes nothing observable about `AnvilDiskBackend`'s own behavior. This is a **new** test file (not a modification of any `tests/anvil_*.rs` file, Constraints) — its existence is a belt-and-suspenders behavioral check on top of the structural `git diff --stat` requirement (Goal & Done).

## Implementation steps

1. **`Cargo.toml`.** Add `object_store`/`tokio` lines exactly as Deliverables. Observable: `cargo build -p rc-chunk-storage` fails only on `todo!()`s.
2. **`anvil/error.rs`.** Append the two new variants (`StaleEpoch`, `Backend`) to the existing `StorageError` enum — no other line in this file changes. Observable: compiles standalone; `cargo nextest run -p rc-chunk-storage` still passes every existing `anvil_*.rs` test file unchanged (confirms the addition is non-breaking before any new code depends on it).
3. **`cluster_storage/keys.rs`.** Pure, allocation-free path-building functions per §E. Observable: exercised indirectly by every later test file.
4. **`cluster_storage/fencing.rs`.** `VersionCache` (a `parking_lot::Mutex<HashMap<..>>` wrapper), `write_fenced` (§F's exact algorithm — the cache-hit fast path, the `get`-then-`put_opts(Update)`/`put_opts(Create)` slow-path loop, `CAS_RETRY_LIMIT` bound), `probe_conditional_write_support` (one `Create` then one `Update` against a reserved probe key, both wrapped so any `NotSupported`/`NotImplemented` becomes `StorageError::Backend`). Every `object_store` call wrapped in `runtime.block_on` (§I). Observable: `cluster_storage_fencing.rs`'s full suite passes.
5. **`cluster_storage/manifest.rs`.** `RegionManifest`/`ObjectVersion` (plain postcard-derived structs), `write_region_manifest`/`read_region_manifest` per §G.1/§G.2's exact merge-then-write / plain-read algorithms, routed through `fencing::write_fenced` for the write half. Observable: `cluster_storage_manifest.rs`'s full suite passes.
6. **`cluster_storage/staging.rs`.** Plain, unfenced `get`/`put_opts(Overwrite)`/`delete` wrappers per §H. Observable: exercised by `cluster_storage_takeover_resume.rs` only indirectly (no dedicated staging test file — M7-B03's own protocol, not this blueprint's, is what a staging-specific test would exercise, and building that protocol driver is explicitly out of this blueprint's scope; this blueprint's own tests confirm the primitive's own round-trip behavior as part of the general contract suite's discipline, not a separately named file, since staging has no fencing/dirty-tracking complexity of its own to test beyond plain get/put/delete correctness already covered by `object_store`'s own upstream test suite).
7. **`cluster_storage/backend.rs`.** `ObjectStoreBackendConfig`/`ObjectStoreBackend` (holds `store`/`compression`/`runtime`/`cache`), `open` (capability probe first, §F), the `ChunkStorageBackend` impl (`read_chunk`/`write_chunk` build/parse the §E 9-byte header around `fencing::write_fenced`/a plain `get`; `read_level_dat`/`write_level_dat` plain `get`/`put_opts(Overwrite)` against the fixed `world/level.dat` key), the inherent manifest/staging re-exports (thin wrappers over `manifest.rs`/`staging.rs`), `write_level_dat_fenced` (routed through `fencing::write_fenced`, `ClusterConfigEpoch`-keyed per §J), `write_chunks_batch` (concurrent dispatch within one `block_on`, per its own doc comment). Observable: `cluster_storage_backend_contract.rs`, `cluster_storage_takeover_resume.rs`, `cluster_storage_monolithic_unaffected.rs` all pass.
8. **`cluster_storage/mod.rs`, crate-root `lib.rs`.** Module declarations and re-exports exactly as Deliverables. Observable: `use rc_chunk_storage::{ObjectStoreBackend, RegionManifest, ObjectVersion};` resolves from outside the crate.
9. **Full acceptance suite.** `cargo nextest run -p rc-chunk-storage` — every test in every file under Acceptance tests passes, alongside every pre-existing `M2-B0x`/`M5-B09` test file, unchanged.
10. **Structural monolithic-unaffected check.** `git diff --stat` against this blueprint's own starting commit shows zero changed lines under `crates/chunk-storage/src/anvil/{mod,backend,region_file,compression,checksum}.rs` and every `crates/chunk-storage/tests/anvil_*.rs` file (Goal & Done's own literal done-condition).
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0.
12. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, restated exactly per TEST-D45/D46: every file under `crates/chunk-storage/tests/cluster_storage_*.rs` is committed first, alongside `todo!()`-stubbed `src/cluster_storage/*.rs` (full signatures, derives, doc comments already final) and the `Cargo.toml`/`lib.rs`/`anvil/error.rs` edits. The implementation changeset fills in real bodies only.

(b) **No new external dependencies beyond `object_store` and (first direct use) `tokio`, both already `[workspace.dependencies]`-pinned.** Do not add a dedicated retry/backoff crate, a separate async-runtime-bridging crate, or any S3-SDK crate directly (`object_store`'s own `aws` feature is the sole S3 integration surface, per WORLD-D17's own binding choice).

(c) **PLAN-D3's hard constraint, restated and made concrete for this blueprint specifically.** This blueprint touches exactly two pre-existing files outside its own new module tree: `Cargo.toml` (one new dependency line) and `src/anvil/error.rs` (two new, purely additive enum variants) — both flagged, justified, and minimal per §A/Deliverables. `src/anvil/{mod,backend,region_file,compression,checksum}.rs`, `src/{io_pool,superflat,lifecycle}.rs`, `crates/scheduler/src/chunk_ticket.rs`, and every `M2-B0x`/`M5-B09` test file are **untouched** — zero lines changed, mechanically verified (Goal & Done, Implementation step 10). This blueprint never modifies `ChunkStorageBackend`'s trait signature, `RegionFileKind`, or `CompressionScheme`.

(d) **No Mojang or third-party reimplementation code.** Every byte layout and algorithm in this blueprint (the §E object-key/header framing, the §F fencing algorithm) is this blueprint's own original design, derived from `docs/planning/03-world-chunks-persistence.md`'s WORLD-D17–D20 and `docs/planning/13-cluster-architecture.md`'s CLUSTER-D17–D19 plus the `object_store` crate's own public, current documentation (verified live at derivation time, §D) — no Mojang source, no other reimplementation's cluster/storage code, consulted or copied (ASSET-D18/D19/D30).

(e) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: composition-root TOML-to-`Arc<dyn ObjectStore>` construction (§A, §D); a real `rc_cluster::MigrationStore` implementation (§H — a future composition-root-extension blueprint's own adapter over this blueprint's `write_staging`/`read_staging`/`delete_staging`); a cluster-mode `PlayerDataStore` implementation (§J — restated custody rule only, no code); CLUSTER-D16's takeover-algorithm decision logic (§G.4 supplies only the read-side resume primitives such an algorithm calls); CLUSTER-D2's rebalancer or any wiring into `rusty-clanker-server`'s config/role selection. Do not add placeholder implementations of any of these as a shortcut.

(f) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in safe Rust — `object_store`'s own async API plus `tokio::runtime::Handle::block_on` for the sync bridge (§I), no raw pointers, no manual memory management.

(g) **The §F fencing algorithm is binding, not an optimization opportunity.** Do not add an unconditional-`Overwrite` fast path for chunk/manifest/fenced-level.dat writes "since single-writer-per-partition should already prevent conflicts" — §F's own rationale explains precisely why the CAS check exists for the abnormal (zombie/partition) case specifically, which single-writer-per-partition's *steady-state* guarantee does not cover. A future performance-focused blueprint may revisit this deliberately, with its own equivalence argument; this blueprint's implementation changeset does not.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-chunk-storage --all-features
cargo nextest run -p rc-chunk-storage
cargo test --doc -p rc-chunk-storage
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
git diff --stat -- crates/chunk-storage/src/anvil crates/chunk-storage/tests/anvil_*.rs
```

Expected: every `cargo`/`xtask` command exits 0; the `git diff --stat` invocation's own output is empty (zero changed lines) against this blueprint's starting commit. `cargo nextest run -p rc-chunk-storage` runs `cluster_storage_backend_contract.rs` (7) + `cluster_storage_fencing.rs` (5, two of which are `proptest!` property tests each running its own configured case count) + `cluster_storage_manifest.rs` (5) + `cluster_storage_takeover_resume.rs` (3) + `cluster_storage_monolithic_unaffected.rs` (1) = 21 named test cases (Acceptance tests), plus every pre-existing `M0`–`M6`/`M5-B09` test in this crate, unchanged and still passing. CI (`.github/workflows/ci.yml`, `M0-B01`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
