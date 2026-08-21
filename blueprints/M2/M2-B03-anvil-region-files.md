# M2-B03 — Anvil Region-File Reader/Writer (`rc-chunk-storage::anvil`)

| Field | Content |
|---|---|
| ID | M2-B03 |
| Milestone | M2 — Persistent World Storage |
| Prerequisites | M2-B02 (`rc-nbt`'s `read_borrowed_strict`/`read_gzip_owned`/`write_gzip_owned`/`NbtError`, used here as a corruption-detection and `level.dat` primitive). Does **not** depend on M2-B01 — this blueprint operates entirely on opaque `&[u8]` payloads (pre-encoded NBT bytes handed in by a future caller), never on M2-B01's `PalettedContainer`/`BlockStateColumn`/component types, so the two are order-independent despite sharing one crate directory (see Context's "Shared-crate landing order" note). |
| Implements | WORLD-D12 (Anvil `.mca` byte layout, hand-rolled, no `mca` crate dependency), WORLD-D13 (Zlib default / LZ4 operator-selectable chunk compression, exact crate versions), WORLD-D14 (world save folder layout, restated in full), WORLD-D15 (`level.dat` GZip framing and the `level.dat_new`/`level.dat_old` safe-write scheme — storage mechanics only, no schema), WORLD-D17 (`ChunkStorageBackend` trait, restated exactly; `AnvilDiskBackend` is this blueprint's implementation; `ObjectStoreBackend` is explicitly out of scope, deferred to a later milestone), WORLD-D21 (this crate's calls are synchronous, RC-IoPool-only — restated as a calling-convention constraint, RC-IoPool's own thread-pool machinery is a `rc-scheduler` blueprint's job, not built here), PERF-D28 (region-file save batching + `fdatasync`-not-`fsync` durability, restated as an additive inherent method plus a universal per-write durability rule), PERF-D29 (open-region-file-handle LRU cache, exact sizing: 256 handles, 60 s idle eviction), the Anvil-round-trip portion of TEST-D26 item (3) (fuzz targets: `decode(encode(x)) == x`, and "decode never panics on arbitrary bytes claiming to be a region file") |
| Crates touched | `rc-chunk-storage` (`crates/chunk-storage/`) — new `src/anvil/` module tree, `Cargo.toml` dependency additions; new, deliberately **non-member** crate `crates/chunk-storage/fuzz/` (mirrors M2-B02's `crates/nbt/fuzz/` shape) |
| Estimated scope | L |

## Goal & Done definition

Implement the hand-rolled Anvil `.mca` region-file container: exact byte layout (8 KiB header, sector allocation, per-chunk record framing, oversized-chunk `.mcc` overflow), Zlib/LZ4/uncompressed chunk compression, crash-safe sector (re)allocation, corruption detection at every structural boundary vanilla's own format leaves unchecked (there is no on-disk checksum in real Anvil files — Context explains why and states this crate's own substitute), the `ChunkStorageBackend` trait and its `AnvilDiskBackend` implementation over WORLD-D14's real save-folder layout including `level.dat`'s atomic-write-with-backup scheme and a world-level single-writer file lock, an open-handle LRU cache, and the low-level `RegionFile` primitive PERF-D28's future batched-save caller will build on. No `ObjectStoreBackend`, no `IoUringAnvilDiskBackend`, no Stage-9 scheduling wiring, and no real `ChunkColumn` NBT schema exist after this blueprint — all four are later work (Context's Scope boundary).

Done when:

- [ ] `cargo build -p rc-chunk-storage --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-chunk-storage`.
- [ ] `crates/chunk-storage/fuzz/` type-checks (`cargo check --manifest-path crates/chunk-storage/fuzz/Cargo.toml`, or `cargo +nightly fuzz build` with a local nightly toolchain) — not a CI-required verification command (Constraints), mirroring M2-B02's identical fuzz-crate treatment.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds `flate2`, `lz4_flex`, `parking_lot`, `thiserror` (all already workspace-pinned) as new dependency edges of `rc-chunk-storage`; `rc-chunk-storage` is in neither `SIM`'s forbidden-neighbor set nor `NETRENDER`, and none of these four additions creates an edge into `NETRENDER`.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-chunk-storage` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Shared-crate landing order

`rc-chunk-storage` already exists as an empty-shell crate (M0-B01: `rc-core`/`rc-nbt`/`rc-registries` normal dependencies, the off-by-default `io_uring` feature already wired). M2-B01 (in-memory chunk representation) and this blueprint both add content to that same crate but touch **disjoint** files: M2-B01 owns `src/{bits,registry_id,palette,column,light,heightmap,block_entity,status,persistence,chunk_key}.rs`; this blueprint owns a new `src/anvil/` subtree. Both append their own lines to the shared `Cargo.toml` and `src/lib.rs` — Deliverables below show the union of both blueprints' additions so the result is identical regardless of landing order; if M2-B01 has already landed when this blueprint is implemented, merge these additions into its version of those two files instead of overwriting.

### Resolved discrepancy: no separate `rc-anvil` crate

WORLD-D12's own text names "`rc-anvil` crate" as the Anvil implementation's home, and TEST-D26 item (3) repeats that name. `12-workspace-structure.md`'s Crate Manifest — the file-owning decision for crate layout (WS-D2) — was never revised to add such a crate: `rc-chunk-storage`'s own manifest row already reads "on-disk region-file format... storage-backend abstraction," and the dependency graph places it once, in the `SIM` server-simulation subgraph. This blueprint follows the manifest that actually ships: everything WORLD-D12 describes as "`rc-anvil`" is implemented as a new `anvil` module inside `rc-chunk-storage` (`crates/chunk-storage/src/anvil/`), never a second crate. This is the identical class of resolution M2-B01 already made for `BlockStateId`/`BiomeId`'s planned-but-never-scaffolded `crates/world/` home — restated here as this blueprint's own binding resolution, not a new pattern.

### Vanilla Anvil `.mca` byte layout — exact, field-precise (WORLD-D12, cross-checked against `docs/research/mc-26.2/04-persistence-nbt.md` §3)

**8 KiB header**, two 4096-byte tables at the very start of the file:
- **Location table** (file offset `0..4096`): 1024 big-endian 4-byte entries, one per chunk slot in the region's 32×32 grid, indexed by `local_x + 32 * local_z` where `local_x = chunk_x.rem_euclid(32)`, `local_z = chunk_z.rem_euclid(32)` (both `0..32`). Each entry packs `(sector_offset: 24 bits) << 8 | (sector_count: 8 bits)` — sector granularity is 4096 bytes, sector index counted from the start of the file (so sector 0 is the location table itself, sector 1 the timestamp table — both permanently reserved, real chunk data never starts before sector 2). An all-zero entry means "no chunk stored at this slot."
- **Timestamp table** (file offset `4096..8192`): 1024 big-endian 4-byte Unix-epoch-seconds entries, same indexing, purely informational (last-write time per chunk); this crate writes the current time on every successful `write_record` and exposes it via `RegionFile::timestamp`, but nothing in this blueprint reads it back to make a decision.

**Per-chunk record**, stored at the sector range its location entry points to, sector-aligned (unused trailing bytes in the record's last sector are left as whatever was previously there — never zeroed, matching vanilla's own behavior of not bothering to clear padding, since the `length` field alone bounds the real payload): `[length: u32 BE][compression_tag: u8][payload: (length - 1) bytes]`. `length` counts the compression-tag byte plus the payload that follows it (so a record with zero payload bytes has `length == 1`). `compression_tag`'s low 7 bits select the scheme (`1` = GZip — historical, McRegion-era, **read-only**, this crate never writes it; `2` = Zlib; `3` = uncompressed; `4` = LZ4; `127` = a custom scheme identified by a following length-prefixed namespaced string — this crate never writes `127` and treats it as `StorageError::UnknownCompressionType(127)` on read, since no custom scheme is defined anywhere in this project's planning corpus). The top bit (`0x80`) means "this chunk's real data lives in an external `.mcc` file" (see below) — when set, the in-region record is a fixed 5-byte stub (`length = 1`, no payload bytes at all), always occupying exactly one sector.

**Oversized-chunk `.mcc` overflow.** The location entry's sector-count field is one byte, capping any non-external record at `255` sectors (`1,044,480` bytes total, `~1 MiB`). A record whose real total size (`4 + 1 + compressed_len`) would need more than 255 sectors is instead written as: the full compressed payload, verbatim, as the entire contents of a sibling file `c.<chunk_x>.<chunk_z>.mcc` (absolute chunk coordinates, not region-local — this file sits in the **same directory** as the `.mca` file itself, e.g. `region/c.130.-9.mcc` next to `region/r.4.-1.mca`), and the in-region record becomes the 5-byte external stub described above with `compression_tag | 0x80`. `.mcc`'s own bytes carry no length prefix or framing of their own — the file's own size on disk is the length.

**Sector-allocation algorithm is this crate's own implementation choice, not part of the wire format.** Nothing about which physical sectors a chunk's bytes land in is observable by any Anvil-reading tool — only the header's own recorded `(offset, count)` per chunk is. This blueprint is therefore free to choose the simplest *correct* allocator rather than needing to reproduce vanilla's own internal free-list bookkeeping, subject to one binding correctness property (next subsection).

### Crash-safety: write-then-repoint, never overwrite-then-free

`docs/research/mc-26.2/04-persistence-nbt.md` flags, as a correctness-critical property, that vanilla's own `RegionFile.write` durably writes the *new* sector data and the *new* header pointer **before** the *old* sector range becomes available for reuse — reordering this (freeing first, for "simplicity") reintroduces a real historical Mojang bug class: a chunk truncated or corrupted by a power loss mid-save. This blueprint's algorithm satisfies that property by construction, using a single, uniform rule with no special-casing: **every write allocates a fresh sector range that never includes the chunk's own current range**, so a mid-write crash always leaves one of two recoverable states — either the header still points at the old (fully intact, untouched) location, or it has been durably repointed at the new (fully intact, already-synced) location; the old range's bytes become logically free automatically the moment nothing in the header (rescanned fresh from disk on every `RegionFile::open`) points at them any more. No separate free-list is ever persisted to disk — it is always *recomputed* from the 1024 location entries plus the file's own current length, which is what makes this property hold across a crash for free: recomputation only ever notices sectors that a *currently valid* header entry claims, so a header that was never durably updated still protects its old data, and a header that *was* durably updated protects the new data instead.

**`RegionFile::write_record`'s exact algorithm:**
1. Compute the record's total sector-aligned size (`sectors_needed`) from the compressed payload length (or `1` if this write must go external — see below).
2. Compute the file's current free-sector ranges by scanning `[sector 2, file_sectors)` for every position not covered by any of the 1024 location entries' claimed `[offset, offset+count)` range (out-of-bounds claims are clipped at `file_sectors`, never causing a panic or an out-of-range write). This scan uses the *unmodified* current location table — the chunk being written still owns its old range at this point, so that range is correctly excluded from consideration and a fresh range is always chosen.
3. First-fit: the first free range at least `sectors_needed` sectors long; if none exists, allocate `sectors_needed` fresh sectors appended at the end of the file (extending it).
4. Build the sector-aligned record buffer (`length` + `compression_tag` + payload, zero-padded to the sector boundary) and write it at the chosen offset.
5. `File::sync_data()` (durably persists the new record before the header is touched — PERF-D28's "`fdatasync`, not `fsync`": chunk data durability, not inode metadata, is all crash recovery needs, since WORLD-D19's `RegionManifest` — a later milestone's concern — is content-keyed, never mtime-keyed).
6. Update the in-memory location and timestamp entries for this chunk's slot, write those two 4-byte fields to their fixed header offsets (`index*4` and `4096+index*4`), and `sync_data()` again.
7. If this write is **not** external and the chunk previously had an on-disk `.mcc` file (there is no cheap way to know this without tracking it — instead, unconditionally attempt `fs::remove_file` on the chunk's `.mcc` path whenever writing a non-external record, ignoring a `NotFound` error; this is a no-op when no such file exists and correctly cleans up a chunk that has shrunk below the external threshold).

No separate "free the old range" step exists — step 2's fresh scan on the *next* write is what makes the old range available again, automatically, with no bookkeeping to keep consistent across a crash.

### Compression (WORLD-D13) — exact schemes, crate versions, and this crate's own LZ4 framing choice

Three writer-selectable schemes, chosen once per `AnvilDiskBackend` instance (an operator-level default, not a per-chunk choice — matches vanilla's own `server.properties` `region-file-compression` single setting):

- **Zlib (default, tag `2`)** — `flate2` `1.1.9` (already workspace-pinned, `zlib-ng` backend, NET-D5/WORLD-D13's shared pin), `Compression::default()` (zlib's own default level, matching vanilla's `Deflater.DEFAULT_COMPRESSION`).
- **LZ4 (tag `4`)** — `lz4_flex` `0.14.0` (already workspace-pinned, WORLD-D13, pure Rust, no `unsafe` by default). This crate uses `lz4_flex::block::compress_prepend_size`/`decompress_size_prepended` — the plain block API already included in `lz4_flex`'s default feature set (no `frame` feature needed), which prepends its own 4-byte little-endian uncompressed-length header ahead of the raw LZ4 block. **This is this crate's own on-disk sub-encoding choice for the LZ4 payload's bytes**, not a claim of bit-identical compatibility with vanilla's own LZ4 implementation — the Anvil container format only requires that *this crate's own reader* can decompress *this crate's own writer's* output correctly (WORLD-D13 itself frames LZ4 support as "configurability parity," never bit-exactness); whether vanilla's own LZ4 sub-framing happens to match is unconfirmed and flagged in Open Questions, not assumed.
- **Uncompressed (tag `3`)** — identity, no crate needed.

**GZip (tag `1`) is decode-only**, supported in the read path via `flate2::read::GzDecoder` for defense against an old `.mca` file that predates Zlib ever becoming default (McRegion-era vanilla files) — this crate's writer never selects it, and `CompressionScheme` (the writer-facing type) has no `Gzip` variant; only the internal tag-dispatch decompression function recognizes tag `1`.

### Corruption handling — there is no on-disk checksum in real Anvil files

Unlike some container formats, vanilla's own Anvil region file carries **no CRC, hash, or checksum field anywhere** — not in the header, not in the per-chunk record. Corruption is detected purely structurally: a location entry pointing outside the file's current sector range, a record's declared `length` exceeding its allocated sectors, a compressed payload that fails to decompress, or (this crate's own addition, since we already depend on `rc-nbt`) decompressed bytes that fail to parse as a well-formed NBT document via `rc_nbt::read_borrowed_strict` (M2-B02) — the strict variant specifically, so trailing garbage after a structurally-valid-looking NBT document is also caught, not just outright decode failure. This NBT-well-formedness check runs only at `AnvilDiskBackend::read_chunk`'s level (the trait boundary, where "payload" is understood to be NBT bytes) — the lower-level `RegionFile::read_record`/`write_record` primitive is deliberately NBT-agnostic (it stores and retrieves opaque `(compression_tag, bytes)` pairs) so this blueprint's own byte-level container tests can exercise the format directly without needing every hand-constructed test fixture to also be valid NBT.

Every corruption case is scoped to the smallest unit it can be: a bad location entry, an unreadable `.mcc` file, or a failed decompression/NBT-validation affects only the one chunk slot it belongs to — reading or writing any *other* chunk in the same region file, or any chunk in any *other* region file, is unaffected. A structurally-unreadable region file itself (wrong length, non-sector-aligned size) fails only that region's `RegionFile::open` call — `AnvilDiskBackend` opens region files lazily, per access, so one bad region file never prevents any other region from being read or written.

### The `ChunkStorageBackend` trait — restated exactly (WORLD-D17)

```rust
pub trait ChunkStorageBackend: Send + Sync + 'static {
    fn read_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, epoch: Option<u64>) -> Result<Option<Vec<u8>>, StorageError>;
    fn write_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, payload: &[u8], epoch: Option<u64>) -> Result<(), StorageError>;
    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError>;
    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>;
}
```

`AnvilDiskBackend` (this blueprint) is one of the trait's two intended implementations; `ObjectStoreBackend` (WORLD-D17/D18, `object_store` 0.14.1) is the other, explicitly **out of scope here** — a later milestone's blueprint, since it needs no `.mca`/sector concept at all (WORLD-D18: cluster storage is object-per-chunk, never literal `.mca` files). `epoch: Option<u64>` exists for `ObjectStoreBackend`'s conditional-put fencing (CLUSTER-D19); `AnvilDiskBackend` **ignores it entirely** (accepted for trait-signature compatibility only) — a local single-process disk backend, guarded by this blueprint's own world-level file lock (below), has no concurrent-writer scenario an epoch token could usefully fence against.

**`write_chunk`'s `payload` is raw, uncompressed NBT bytes** — `AnvilDiskBackend` applies WORLD-D13's compression itself before writing, and `read_chunk` returns already-decompressed bytes. `write_level_dat`'s `payload` is, by contrast, **already GZip-compressed** by the caller (a future blueprint owning `level.dat`'s actual schema, via `rc_nbt::write_gzip_owned`) — `AnvilDiskBackend` performs only the atomic-write-with-backup file mechanics for `level.dat`, no compression, since GZip is `level.dat`'s one fixed, non-configurable scheme (WORLD-D15) and is simplest left entirely to the schema-owning caller that already has an `rc-nbt` dependency for exactly this purpose. This asymmetry is a deliberate, restated design choice — WORLD-D15/D17 do not pin it explicitly, and this blueprint's resolution is binding for every future caller of this trait.

### World save folder layout — restated exactly (WORLD-D14)

```
<world_root>/
├── level.dat              # + level.dat_new / level.dat_old (safe-write scheme, below)
├── session.lock            # this blueprint's single-writer advisory lock (below)
├── region/                 # Overworld terrain   — RegionFileKind::Terrain
├── entities/                # Overworld entities  — RegionFileKind::Entities
├── poi/                     # Overworld POI        — RegionFileKind::Poi
├── DIM-1/
│   ├── region/               # Nether terrain
│   ├── entities/
│   └── poi/
├── DIM1/
│   ├── region/               # The End terrain
│   ├── entities/
│   └── poi/
├── playerdata/, stats/, advancements/, data/, datapacks/, resourcepacks/, icon.png
```

This blueprint creates and manages exactly the entries a `ChunkStorageBackend` touches: `level.dat`(`_new`/`_old`), `session.lock`, and the three `{region,entities,poi}/` directories per dimension. The remaining entries (`playerdata/`, `stats/`, etc.) are future blueprints' responsibility — `AnvilDiskBackend::open` never creates them. **Dimension-to-folder mapping is fixed to exactly the three built-in dimensions** `rc_core::DimensionId` currently defines (`OVERWORLD` → world root itself, `THE_NETHER` → `DIM-1/`, `THE_END` → `DIM1/`); any other `DimensionId` value is `StorageError::UnsupportedDimension` — custom-dimension folder naming is unspecified anywhere in this project's planning corpus and out of this blueprint's scope. The Overworld's own three directories (`region/`, `entities/`, `poi/`) are created eagerly by `AnvilDiskBackend::open` (every world has an Overworld); `DIM-1/`/`DIM1/`'s directories are created lazily, only when a Nether/End chunk is actually written, so a world that never touches those dimensions never gets empty placeholder folders.

**`level.dat` safe-write scheme** (WORLD-D14/D15, matching the research corpus's cited `PlayerDataStorage` "corrupt-file backup + `.dat_old` fallback, same atomic-replace pattern"): `write_level_dat` writes to `level.dat_new` first, `sync_data`s it, removes any stale `level.dat_old` (best-effort), renames the current `level.dat` to `level.dat_old` (skipped if no `level.dat` exists yet — first save of a brand-new world), then renames `level.dat_new` to `level.dat`. `read_level_dat` reads `level.dat`, validates it decodes via `rc_nbt::read_gzip_owned` (a decode-only probe — the raw bytes, not a re-encoded copy, are what gets returned), and falls back to `level.dat_old` (validated the same way) if the primary is missing or fails to decode; if both fail, `StorageError::Corrupt`.

### World-level single-writer lock (`session.lock`)

`AnvilDiskBackend::open` acquires a non-blocking, exclusive advisory lock on `<world_root>/session.lock` (created if absent) via `std::fs::File::try_lock` — stabilized in Rust 1.89.0 (this project's toolchain pin, `1.97.0`, is comfortably past it; the implementer confirms the exact method/error-kind shape against the installed toolchain's own `std::fs::File` documentation before writing this code, mirroring M2-B01/M2-B02's identical "verify against installed docs" notes for `bevy_ecs`/`simdnbt` — the intended behavior is unambiguous regardless of the exact signature: a second `open` call against the same `world_root` while the first is still alive must fail with `StorageError::WorldAlreadyOpen`, and the lock must release automatically when the owning `AnvilDiskBackend` (and the `File` handle it holds open for the lock's entire lifetime) is dropped). This is advisory and local-machine-only (irrelevant to `AnvilDiskBackend`'s local-disk-only scope) and is this blueprint's entire answer to "file locking/single-writer rules" at the *world* level; per-region-file concurrency within one already-open backend is a separate, second mechanism (below).

### Concurrency model: one `parking_lot::Mutex` per open region-file handle

Per WORLD-D12's own text ("sector table maintained in-place, `parking_lot::Mutex`-guarded per open region-file handle — a cold-relative-to-the-tick-path lock, consistent with `01`'s ARCH-D23 lock-usage philosophy since this all runs on RC-IoPool, never the tick"): every open `RegionFile` is wrapped in its own `parking_lot::Mutex` (already workspace-pinned, `0.12.5`, ARCH-D23). **Both reads and writes** acquire this same mutex for the full duration of their `RegionFile` call — this is deliberately the simplest correct rule (not merely a writer-exclusion lock): two concurrent reads of the same region file serialize through it too, trading a small amount of read-read parallelism (never on the hot tick path, per WORLD-D21) for a total absence of read/write or write/write races on that file's in-memory location-table cache and its underlying `File` handle's cursor position. Two threads touching *different* region files never contend at all — each open handle's mutex is independent. This is the entirety of this blueprint's "concurrent-read safety" guarantee, verified directly by an acceptance test spawning multiple threads against one `AnvilDiskBackend`.

### Open-region-file-handle LRU cache (PERF-D29, exact sizing)

`AnvilDiskBackend` caches up to **256** concurrently-open `RegionFile` handles (one per `(DimensionId, RegionFileKind, region_x, region_z)` — up to 3 kinds per region, comfortably under typical `RLIMIT_NOFILE`), evicting the least-recently-touched entry once the cap is reached, and — this blueprint's own concrete interpretation of PERF-D29's "proactively closing any handle idle more than 60 s even below the cap," since no background sweep thread is specified anywhere in `03`/`14` and spinning one up inside a library crate raises ownership/shutdown questions those documents don't answer — checking every open handle's idle time **opportunistically on every cache access** (not via a dedicated timer thread) and evicting any handle idle past 60 s at that point, in addition to cap-triggered eviction. A dedicated background sweep is left as a documented possible future refinement, not built here (Constraints).

### Region-file save batching (PERF-D28) — an additive primitive, not Stage-9 wiring

PERF-D28 extends this blueprint's own region-file writer with a batched-write tactic: when several chunks belong to the same `(dim, kind, region)` file, group them under one handle-lock hold, compute all their sector (re)allocations together, and rewrite the file's header/`sync_data` **once** for the whole group instead of once per chunk. This blueprint implements that tactic as one additional **inherent** method on `AnvilDiskBackend`, `write_chunks_batch` — **not** part of the `ChunkStorageBackend` trait itself, since `ObjectStoreBackend` has no equivalent "shared file" concept to batch by. Deciding *which* dirty chunks to group into one batch call, and *when* (tied to Stage 9's save cadence, WORLD-D23), is explicitly **out of scope** — that is a future scheduler-integration blueprint's job; this blueprint only builds the primitive it will call.

### Soak-test checksum method (feeds M2's milestone acceptance criterion 2)

Vanilla's Anvil format has no on-disk checksum (above), so there is nothing to reuse for round-trip verification — this blueprint defines its own, purely in-process integrity check: `content_checksum(bytes: &[u8]) -> u64`, backed by `std::collections::hash_map::DefaultHasher` (SipHash-1-3 with fixed, non-randomized keys — deterministic for the lifetime of one test process, which is all a same-process pre-write-vs-post-read-back comparison needs; this is never written to disk and makes no cross-process or cross-Rust-version stability claim). The milestone's own 10,000-round-trip soak test (Acceptance tests, below) hashes each synthetic chunk's NBT-encoded bytes before `write_chunk` and again after the matching `read_chunk`, asserting equality every time.

### Fuzz target (TEST-D26 item 3)

Mirroring M2-B02's `crates/nbt/fuzz/` shape exactly (own non-member `[workspace]` crate, `libfuzzer-sys 0.4.13`/`arbitrary 1.4.2` per TEST-D25's pins, `cargo-fuzz` 0.13.2 as an installed CLI tool, never a manifest dependency): two targets, since TEST-D26 item (3) names two distinct properties. Both operate against real temporary files (`RegionFile` is filesystem-based by construction, matching WORLD-D12's "hand-rolled... in-repo" reality — there is no in-memory abstraction to fuzz against instead), which is slower per-iteration than a pure in-memory fuzz target but is the same acceptable tradeoff M2-B02's own fuzz-crate Constraints already establish: not a CI-gated Tier-1 requirement, exercised only in Tier 2's time-boxed nightly run and a one-time local sanity build.

### Scope boundary — explicitly not built by this blueprint

`ObjectStoreBackend` (WORLD-D17/D18, `object_store` 0.14.1) — ChunkStorageBackend's cluster-mode implementation, a later milestone. `IoUringAnvilDiskBackend` (PERF-D23) — the optional, Linux-only, `io_uring`-backed alternate backend; `rc-chunk-storage`'s `io_uring` Cargo feature stays wired-but-unimplemented exactly as M0-B01 left it (this blueprint adds no code behind it). Stage-9 snapshot scheduling, the dirty-chunk save-interval timer, and any wiring into `RC-IoPool`'s actual thread pool (WORLD-D21/D23, ARCH-adjacent) — a future scheduler-integration blueprint calls `write_chunk`/`write_chunks_batch` from wherever that pool ends up living; none of it is built here. Real `ChunkColumn` NBT (de)serialization (WORLD-D11's own named `to_nbt`/`from_nbt` example) — this blueprint's tests use small, synthetic, hand-built NBT compounds (via `rc-nbt`'s own `owned` API) standing in for real chunk payloads; wiring actual chunk data through this trait is a future blueprint's job. Entity/POI *schema* content (WORLD-D29 — this blueprint's `RegionFileKind::{Entities,Poi}` variants only select the correct folder/file-naming convention; no entity or POI record shape is interpreted here, matching WORLD-D29's own "this document owns only the container/section framing" boundary). `RegionManifest`/`ChunkSnapshot`/cluster migration (WORLD-D19/D20) — cluster-only, later milestone.

## Deliverables

### `crates/chunk-storage/Cargo.toml` (modify — full resulting file, union of M2-B01's and this blueprint's additions)

```toml
[package]
name = "rc-chunk-storage"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
rc-nbt = { path = "../nbt" }
rc-registries = { path = "../registries" }
bevy_ecs = { workspace = true }          # M2-B01's own addition; present once M2-B01 lands, unused by this blueprint
flate2 = { workspace = true }             # this blueprint — WORLD-D13 Zlib
lz4_flex = { workspace = true }           # this blueprint — WORLD-D13 LZ4
parking_lot = { workspace = true }        # this blueprint — WORLD-D12 per-handle lock
thiserror = { workspace = true }          # this blueprint — StorageError
io-uring = { workspace = true, optional = true }

[dev-dependencies]
proptest = { workspace = true }           # M2-B01's own addition, unused by this blueprint's own tests

[features]
io_uring = ["dep:io-uring"]
```

### `crates/chunk-storage/src/lib.rs` (modify — append if M2-B01's module list is already present, else this is the whole file plus M2-B01's own eventual additions)

```rust
mod anvil;

pub use anvil::{
    content_checksum, AnvilDiskBackend, ChunkStorageBackend, CompressionScheme, RegionFile,
    RegionFileKind, StorageError,
};
```

### `crates/chunk-storage/src/anvil/mod.rs`

```rust
//! Anvil `.mca` region-file container (WORLD-D12/D13/D17), the `ChunkStorageBackend`
//! trait and its `AnvilDiskBackend` implementation over WORLD-D14's save-folder layout.
//! `ObjectStoreBackend`, `IoUringAnvilDiskBackend`, Stage-9 scheduling wiring, and real
//! `ChunkColumn` NBT schemas are explicitly out of scope — see this crate's own
//! module-level docs in the owning blueprint (M2-B03) for the full boundary.

mod backend;
mod checksum;
mod compression;
mod error;
mod region_file;

pub use backend::{AnvilDiskBackend, ChunkStorageBackend, RegionFileKind};
pub use checksum::content_checksum;
pub use compression::CompressionScheme;
pub use error::StorageError;
pub use region_file::RegionFile;
```

### `crates/chunk-storage/src/anvil/error.rs`

```rust
use std::path::PathBuf;

/// `ChunkStorageBackend`'s one error type (WORLD-D17), shared by every module in this
/// tree. Every variant that names a path carries it for diagnostics — no variant is
/// ever constructed with a placeholder/empty path (Constraints).
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io error at {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },

    /// A region file's own structure is invalid independent of any one chunk's record
    /// (wrong overall length, non-sector-aligned size, an unreadable header).
    #[error("region file {path} structurally corrupt: {reason}")]
    Corrupt { path: PathBuf, reason: String },

    /// One chunk's location-table entry claims sectors outside the file's own current
    /// extent, or a record's declared `length` exceeds the sectors it was allocated.
    #[error("chunk ({local_x},{local_z}) sector pointer out of bounds: offset {offset} count {count}, file has {file_sectors} sectors")]
    SectorOutOfBounds { local_x: u8, local_z: u8, offset: u32, count: u8, file_sectors: u32 },

    #[error("unknown chunk compression scheme id {0}")]
    UnknownCompressionType(u8),

    #[error("decompression failed: {0}")]
    Decompress(String),

    /// This crate's own defense-in-depth corruption check (Context) — decompressed
    /// bytes that do not parse as a well-formed NBT document via
    /// `rc_nbt::read_borrowed_strict`.
    #[error("chunk payload failed NBT well-formedness validation: {0}")]
    InvalidNbtPayload(String),

    #[error("an external chunk record at {path} points to a `.mcc` file that does not exist")]
    MissingExternalFile { path: PathBuf },

    #[error("world at {path} is already open (held by another process via session.lock)")]
    WorldAlreadyOpen { path: PathBuf },

    #[error("unsupported dimension {0:?} — only the built-in Overworld/Nether/End are mapped to a save folder at this milestone's scope")]
    UnsupportedDimension(rc_core::DimensionId),
}
```

### `crates/chunk-storage/src/anvil/compression.rs`

```rust
use crate::anvil::error::StorageError;

/// The three writer-selectable chunk-compression schemes (WORLD-D13) — one chosen per
/// `AnvilDiskBackend` instance, applied to every chunk it writes. GZip (on-disk tag `1`,
/// McRegion-era) is intentionally **not** a variant here — it is decode-only (Context)
/// and never selected for writing.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompressionScheme {
    Zlib,
    Lz4,
    Uncompressed,
}

impl CompressionScheme {
    /// The on-disk compression-tag byte's low 7 bits for this scheme (WORLD-D12: `2`
    /// Zlib, `3` uncompressed, `4` LZ4).
    pub const fn tag(self) -> u8 {
        match self {
            CompressionScheme::Zlib => 2,
            CompressionScheme::Uncompressed => 3,
            CompressionScheme::Lz4 => 4,
        }
    }

    /// Compresses `raw` per this scheme. `Lz4`'s exact on-disk sub-encoding is this
    /// crate's own choice (Context) — `lz4_flex::block::compress_prepend_size`.
    pub fn compress(self, raw: &[u8]) -> Vec<u8>;

    /// Decompresses `data`, dispatching on the raw on-disk `tag` byte's low 7 bits (the
    /// caller strips the external-file `0x80` bit before calling this). Recognizes tag
    /// `1` (GZip, read-only, Context) in addition to this enum's own three writable
    /// schemes; any other value is `StorageError::UnknownCompressionType`.
    pub fn decompress_tagged(tag: u8, data: &[u8]) -> Result<Vec<u8>, StorageError>;
}
```

### `crates/chunk-storage/src/anvil/checksum.rs`

```rust
/// Deterministic, in-process-only content checksum for this crate's own round-trip
/// soak tests (Context — vanilla's Anvil format has no on-disk checksum of its own).
/// Backed by `std::collections::hash_map::DefaultHasher`; never written to disk, and
/// makes no cross-process/cross-Rust-version stability claim.
pub fn content_checksum(bytes: &[u8]) -> u64;
```

### `crates/chunk-storage/src/anvil/region_file.rs`

```rust
use std::path::PathBuf;
use crate::anvil::error::StorageError;

/// One open `.mca` file: the 8 KiB header (decoded into two 1024-entry in-memory
/// tables) plus the underlying `std::fs::File`. NBT-agnostic by design (Context) — reads
/// and writes opaque `(compression_tag, bytes)` records; `AnvilDiskBackend` owns
/// compression selection and NBT validation. Not internally synchronized — callers
/// (`AnvilDiskBackend`) are responsible for the one-`parking_lot::Mutex`-per-handle
/// discipline (Context's Concurrency model).
pub struct RegionFile {
    // private: File, dir: PathBuf, region_x: i32, region_z: i32,
    // locations: Box<[u32; 1024]>, timestamps: Box<[u32; 1024]>, file_sectors: u32
}

impl RegionFile {
    /// Opens the `.mca` file at `path`, creating it (with an immediately-written, fresh
    /// all-zero 8 KiB header) if it does not already exist. `region_x`/`region_z` are
    /// the region's own grid coordinates (`chunk_x.div_euclid(32)` etc.) — supplied by
    /// the caller, not parsed from `path`'s filename, so this type never depends on any
    /// particular file-naming convention. Structural validity rule (Context): a
    /// pre-existing file of length `0` is treated as "not yet written" (same as
    /// freshly-created); length `1..8192` is `StorageError::Corrupt` ("shorter than the
    /// mandatory header"); length `>= 8192` not a multiple of `4096` is
    /// `StorageError::Corrupt` ("not sector-aligned"); anything else parses normally.
    pub fn open(path: PathBuf, region_x: i32, region_z: i32) -> Result<Self, StorageError>;

    /// Reads the record at local slot `(local_x, local_z)` (each `0..32`, already
    /// reduced modulo 32 by the caller). `Ok(None)` = empty slot (all-zero location
    /// entry) — never an error. `Ok(Some((tag, bytes)))` on success: `tag` is the raw
    /// on-disk compression-tag byte **including** the `0x80` external bit if it was set
    /// (the caller strips it before passing to `CompressionScheme::decompress_tagged`);
    /// `bytes` is the still-compressed payload (from the in-region sectors, or read
    /// whole from the paired `.mcc` file when external). Returns
    /// `StorageError::SectorOutOfBounds`/`Corrupt`/`MissingExternalFile` per Context's
    /// corruption-handling rules — a bad record at this one slot never affects any
    /// other slot's readability.
    pub fn read_record(&mut self, local_x: u8, local_z: u8) -> Result<Option<(u8, Vec<u8>)>, StorageError>;

    /// Writes `data` (already compressed by the caller) under `compression_tag`'s low 7
    /// bits (**without** the `0x80` bit — this method decides internally, from `data`'s
    /// own length against the 255-sector cap, whether the record must go external, sets
    /// the bit itself, and writes `data` verbatim to the paired `.mcc` file when it
    /// does). Implements the crash-safe always-fresh-allocation algorithm exactly
    /// (Context). Cleans up (best-effort) a stale `.mcc` file when this write is
    /// non-external (Context, step 7).
    pub fn write_record(&mut self, local_x: u8, local_z: u8, compression_tag: u8, data: &[u8]) -> Result<(), StorageError>;

    /// This slot's last-write Unix timestamp (seconds), or `None` if never written.
    pub fn timestamp(&self, local_x: u8, local_z: u8) -> Option<u32>;

    /// `(free_range_count, total_free_sectors)` — exposed for this blueprint's own
    /// sector-reuse/fragmentation acceptance tests; recomputed fresh on every call
    /// (Context: no persisted free-list exists to introspect).
    pub fn free_sector_summary(&self) -> (usize, u32);
}
```

### `crates/chunk-storage/src/anvil/backend.rs`

```rust
use std::path::{Path, PathBuf};
use crate::anvil::{compression::CompressionScheme, error::StorageError, region_file::RegionFile};

/// Which of the three per-dimension region-file kinds WORLD-D14's layout defines
/// (folder names: `region`/`entities`/`poi`). Only the container/naming convention is
/// this crate's concern — no entity or POI record shape is interpreted here (WORLD-D29,
/// Context's Scope boundary).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RegionFileKind {
    Terrain,
    Entities,
    Poi,
}

impl RegionFileKind {
    pub const fn folder_name(self) -> &'static str {
        match self {
            RegionFileKind::Terrain => "region",
            RegionFileKind::Entities => "entities",
            RegionFileKind::Poi => "poi",
        }
    }
}

/// WORLD-D17's storage-backend abstraction, restated exactly (Context). `epoch` is
/// accepted on every method for signature compatibility with `ObjectStoreBackend`
/// (a later milestone) but is meaningless to `AnvilDiskBackend`, which ignores it.
pub trait ChunkStorageBackend: Send + Sync + 'static {
    fn read_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, epoch: Option<u64>) -> Result<Option<Vec<u8>>, StorageError>;
    fn write_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, payload: &[u8], epoch: Option<u64>) -> Result<(), StorageError>;
    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError>;
    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>;
}

/// WORLD-D17's monolithic-mode implementation: real local `.mca`/`level.dat` files
/// under WORLD-D14's save-folder layout, an open-handle LRU cache (PERF-D29), and a
/// world-level single-writer advisory lock (Context). Not `Clone` — share via `Arc`.
pub struct AnvilDiskBackend {
    // private: world_root: PathBuf, compression: CompressionScheme,
    // handles: parking_lot::Mutex<HandleCache>, _world_lock: std::fs::File
}

impl AnvilDiskBackend {
    /// Opens (creating if absent) `world_root` as a world save directory: creates the
    /// Overworld's `region/`/`entities/`/`poi/` directories eagerly (`DIM-1`/`DIM1`'s
    /// equivalents lazily, on first write to that dimension — Context); acquires
    /// `session.lock` (Context's World-level single-writer lock), returning
    /// `StorageError::WorldAlreadyOpen` if another live `AnvilDiskBackend` (in this or
    /// another process) already holds it. `compression` is the scheme applied to every
    /// chunk this instance writes (WORLD-D13) — existing chunks written under a
    /// different scheme by an earlier config remain correctly readable regardless (the
    /// on-disk tag byte is always authoritative for reads).
    pub fn open(world_root: PathBuf, compression: CompressionScheme) -> Result<Self, StorageError>;

    pub fn world_root(&self) -> &Path;

    /// PERF-D28's batched-write primitive (Context) — **not** part of
    /// `ChunkStorageBackend`. Every entry in `entries` must belong to the same `(dim,
    /// kind)` pair (mixed dimensions/kinds within one call is a programmer error,
    /// `debug_assert!`-checked, not a recoverable `Result` case); entries destined for
    /// the same region file are grouped internally under one handle-lock hold. `epoch`
    /// is ignored exactly as elsewhere.
    pub fn write_chunks_batch(&self, dim: rc_core::DimensionId, kind: RegionFileKind, entries: &[(i32, i32, &[u8])], epoch: Option<u64>) -> Result<(), StorageError>;

    /// Current open-handle count — introspection for this blueprint's own LRU-cache
    /// acceptance tests, not otherwise used.
    pub fn open_handle_count(&self) -> usize;
}

impl ChunkStorageBackend for AnvilDiskBackend {
    fn read_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, epoch: Option<u64>) -> Result<Option<Vec<u8>>, StorageError>;
    fn write_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, payload: &[u8], epoch: Option<u64>) -> Result<(), StorageError>;
    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError>;
    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>;
}
```

### `crates/chunk-storage/fuzz/Cargo.toml` (new — deliberately **not** a root-workspace member)

```toml
[package]
name = "rc-chunk-storage-fuzz"
version = "0.0.0"
edition = "2024"
publish = false

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4.13"
arbitrary = { version = "1.4.2", features = ["derive"] }
rc-chunk-storage = { path = ".." }

[workspace]
# Empty on purpose — mirrors crates/nbt/fuzz/'s identical M2-B02 precedent exactly.

[[bin]]
name = "anvil_roundtrip"
path = "fuzz_targets/anvil_roundtrip.rs"
test = false
doc = false
bench = false

[[bin]]
name = "anvil_decode_never_panics"
path = "fuzz_targets/anvil_decode_never_panics.rs"
test = false
doc = false
bench = false
```

### `crates/chunk-storage/fuzz/fuzz_targets/anvil_roundtrip.rs` (new)

```rust
//! TEST-D26 item (3), round-trip half: `decode(encode(x)) == x` for arbitrary valid
//! in-memory chunk values.
#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rc_chunk_storage::{CompressionScheme, RegionFile};

#[derive(Arbitrary, Debug)]
struct RoundtripInput {
    scheme: u8,          // reduced mod 3 to {Zlib, Lz4, Uncompressed}
    local_x: u8,         // reduced mod 32
    local_z: u8,          // reduced mod 32
    payload: Vec<u8>,
}

fuzz_target!(|input: RoundtripInput| {
    let scheme = match input.scheme % 3 {
        0 => CompressionScheme::Zlib,
        1 => CompressionScheme::Lz4,
        _ => CompressionScheme::Uncompressed,
    };
    let local_x = input.local_x % 32;
    let local_z = input.local_z % 32;
    let path = std::env::temp_dir().join(format!("rc-anvil-fuzz-{}-{:?}.mca", std::process::id(), std::thread::current().id()));
    let compressed = scheme.compress(&input.payload);
    if let Ok(mut rf) = RegionFile::open(path.clone(), 0, 0) {
        if rf.write_record(local_x, local_z, scheme.tag(), &compressed).is_ok() {
            if let Ok(Some((tag, bytes))) = rf.read_record(local_x, local_z) {
                if let Ok(decompressed) = CompressionScheme::decompress_tagged(tag, &bytes) {
                    assert_eq!(decompressed, input.payload);
                }
            }
        }
    }
    let _ = std::fs::remove_file(&path);
});
```

### `crates/chunk-storage/fuzz/fuzz_targets/anvil_decode_never_panics.rs` (new)

```rust
//! TEST-D26 item (3), never-panics half: "decode never panics on arbitrary bytes
//! claiming to be a region file."
#![no_main]
use libfuzzer_sys::fuzz_target;
use rc_chunk_storage::RegionFile;

fuzz_target!(|data: &[u8]| {
    let path = std::env::temp_dir().join(format!("rc-anvil-fuzz-decode-{}-{:?}.mca", std::process::id(), std::thread::current().id()));
    if std::fs::write(&path, data).is_ok() {
        if let Ok(mut rf) = RegionFile::open(path.clone(), 0, 0) {
            for local_z in 0..32u8 {
                for local_x in 0..32u8 {
                    let _ = rf.read_record(local_x, local_z); // must never panic, Ok/Err both fine
                }
            }
        }
    }
    let _ = std::fs::remove_file(&path);
});
```

### `crates/chunk-storage/tests/fuzz_regressions/.gitkeep` (new)

Empty, reserved (TEST-D28) — zero entries at M2, mirroring M2-B02's identical precedent.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/chunk-storage/src/anvil/{mod,error,compression,checksum,region_file,backend}.rs` with every function body from Deliverables replaced with `todo!()` (struct fields, derives, and doc comments stay exactly as specified), plus the `Cargo.toml`/`lib.rs` edits, plus the complete `crates/chunk-storage/fuzz/` crate (both targets have no body to stub, matching M2-B02's precedent). The implementation changeset (Implementation steps, below) fills in real bodies only — it must not modify any file under `crates/chunk-storage/tests/`, must not add/remove/rename any test case listed below, and must not weaken any assertion.

### `crates/chunk-storage/tests/support/mod.rs` (shared test helper, not a deliverable — part of the test-authoring changeset)

A small `TempWorldDir` RAII guard: `TempWorldDir::new(test_name: &str) -> Self` creates `std::env::temp_dir().join(format!("rc-chunk-storage-test-{test_name}-{}-{}", std::process::id(), <a file-local `AtomicU64` counter>.fetch_add(1, Ordering::Relaxed)))`, creates the directory, exposes `.path() -> &Path`, and removes the directory tree (`fs::remove_dir_all`, ignoring errors) on `Drop`. Used by every test file below that touches the filesystem, via `mod support;` (Rust's standard `tests/support/mod.rs` shared-helper convention).

### `crates/chunk-storage/tests/anvil_header_and_indexing.rs`

1. `location_table_index_matches_x_plus_32z` — hand-verify the slot index formula: chunk `(local_x=5, local_z=3)` → index `3*32+5 = 101`; `(31,31)` → `1023`; `(0,0)` → `0`.
2. `fresh_region_file_header_is_all_zero` — `RegionFile::open` on a nonexistent path; assert the created file is exactly `8192` bytes, every byte `0`; assert `free_sector_summary() == (0, 0)` — a header-only, two-sector file has no sectors past the header yet, so there is nothing to report as free (a free range only exists once the file has been extended by at least one write and then partially vacated).
3. `read_record_on_empty_slot_returns_none` — fresh `RegionFile`; `read_record(0, 0)` is `Ok(None)`.
4. `zero_length_existing_file_is_treated_as_fresh` — create a `0`-byte file at a path first (`std::fs::File::create` then drop), then `RegionFile::open` on that same path; behaves identically to a nonexistent path (case 2's assertions hold).
5. `truncated_header_is_corrupt` — write a `100`-byte file (shorter than `8192`) at a path, `RegionFile::open` returns `Err(StorageError::Corrupt { .. })`.
6. `non_sector_aligned_length_is_corrupt` — write a `9000`-byte file (`>= 8192`, not a multiple of `4096`), `RegionFile::open` returns `Err(StorageError::Corrupt { .. })`.

### `crates/chunk-storage/tests/anvil_write_read_roundtrip.rs`

1. `single_chunk_write_then_read_round_trips_exactly` — `RegionFile::open` fresh; `write_record(3, 4, 2 /* Zlib tag */, b"hello anvil")`; `read_record(3, 4)` returns `Ok(Some((2, bytes)))` with `bytes == b"hello anvil"`.
2. `write_record_updates_timestamp` — before any write, `timestamp(3,4) == None`; after `write_record`, `timestamp(3,4)` is `Some(t)` with `t` within a few seconds of `SystemTime::now()`.
3. `write_is_sector_aligned_and_minimal` — `write_record(0, 0, 3, &[0u8; 10])` (uncompressed, tiny payload: total record bytes `4+1+10=15`); after the write, `free_sector_summary()` reports the file has grown to exactly `3` sectors total (`2` header + `1` for this record) and `0` free ranges (the one sector is fully claimed).
4. `two_chunks_in_different_slots_do_not_alias` — write distinct payloads at `(0,0)` and `(31,31)`; read both back; each matches its own payload, neither the other's.
5. `rewrite_same_chunk_larger_moves_to_a_fresh_allocation` — write a small payload at `(1,1)`, note the file's sector count via `free_sector_summary`'s implied growth; rewrite the same slot with a payload requiring more sectors; read back and confirm the NEW content; assert the file's total sector count grew by (at least) the new record's own size (proving the old, now-orphaned range was never reused in place for this larger write, matching Context's always-fresh-allocation rule) — concretely: writing a `9000`-byte uncompressed payload after a `10`-byte one causes total file sectors to increase from `3` to `6` (2 header + 1 old orphaned + 3 new: `ceil((4+1+9000)/4096)=3`), not merely from `3` to `5` (which would imply in-place reuse).

### `crates/chunk-storage/tests/anvil_sector_reuse_and_fragmentation.rs`

1. `shrinking_a_chunk_frees_its_excess_sectors_for_reuse` — write chunk A at `(0,0)` with a payload needing `3` sectors; write chunk B at `(1,0)` with a payload needing `1` sector (lands past A, file now `2+3+1=6` sectors); rewrite chunk A with a payload needing only `1` sector (moves to a fresh location — per the always-fresh rule this is NOT the old 3-sector range in place, but note the old 3-sector range becomes free); write a NEW chunk C at `(2,0)` needing `2` sectors; assert (via `free_sector_summary` and/or by re-reading C successfully) that C's `2`-sector allocation was satisfied from the free range A's shrink left behind rather than by extending the file further — concretely: assert the file's total sector count after writing C is **not** larger than it was immediately after A's shrink-rewrite (proving reuse, not append-only growth).
2. `free_sector_summary_reports_correct_range_count_and_total` — three sequential `4091`-byte-uncompressed writes (each exactly `1` sector: `4+1+4091=4096`) at `(0,0)`, `(1,0)`, `(2,0)` (landing at offsets `2`, `3`, `4`, file grows to `5`); rewrite `(1,0)` (the middle one) with a `10`-byte payload (still `1` sector needed, but per the always-fresh rule it moves to a new offset, `5`, growing the file to `6`); assert `free_sector_summary() == (1, 1)` — exactly one free range (the vacated offset-`3` sector) totaling `1` free sector.
3. `fragmentation_first_fit_reuses_the_earlier_gap_even_when_a_later_gap_fits_more_tightly` — hand-verified sector arithmetic, exact throughout: write `(0,0)` needing `5` sectors (offset `2`, file→`7`), `(1,0)` needing `2` sectors (offset `7`, file→`9`), `(2,0)` needing `1` sector (offset `9`, file→`10`). Rewrite `(1,0)` first, with a payload needing `3` sectors (no free range exists yet, so it must extend: offset `10`, file→`13`) — this frees a `2`-sector gap at offset `7` (the **later**, **smaller** gap). Rewrite `(0,0)` next, with a payload needing `6` sectors (the only free range, `(7,2)`, is too small to hold `6`, so this too must extend: offset `13`, file→`19`) — this frees a `5`-sector gap at offset `2` (the **earlier**, **larger** gap). At this point exactly two free ranges coexist: `(offset 2, count 5)` and `(offset 7, count 2)`. Write a brand-new chunk at `(3,0)` needing exactly `2` sectors: first-fit (scanning from the lowest offset) must consume `2` sectors out of the **earlier, larger** range at offset `2` — leaving it shrunk to `(4, 3)` — rather than the later range at offset `7`, which exactly matches the request and a best-fit strategy would have chosen instead. Assert `free_sector_summary() == (2, 5)` (two ranges — the shrunk `(4,3)` plus the untouched `(7,2)` — five free sectors total); a best-fit allocator would instead have fully consumed the offset-`7` range, leaving `free_sector_summary() == (1, 5)` (one range, the untouched `(2,5)`) — the range-count field alone distinguishes the two strategies unambiguously.

### `crates/chunk-storage/tests/anvil_mcc_overflow.rs`

1. `oversized_payload_goes_external` — `write_record` with an uncompressed payload of `260 * 4096` bytes (`> 255` sectors' worth); assert the in-region allocation is exactly `1` sector (`free_sector_summary` shows the file grew by only `1` sector for this write, not `~260`); assert a file named `c.<abs_x>.<abs_z>.mcc` (computed from the `RegionFile`'s own `region_x`/`region_z` plus the write's `local_x`/`local_z`) now exists alongside the `.mca` file, with byte-for-byte the compressed payload as its entire content.
2. `external_record_reads_back_correctly` — following case 1, `read_record` on the same slot returns `Ok(Some((tag, bytes)))` with `tag`'s `0x80` bit set and `bytes` exactly equal to the original oversized payload.
3. `missing_mcc_file_is_a_distinct_corruption_error` — as case 1, then delete the `.mcc` file directly via `std::fs::remove_file`; `read_record` on that slot returns `Err(StorageError::MissingExternalFile { .. })`, not a generic `Corrupt` or an `Io` error indistinguishable from an unrelated failure.
4. `shrinking_below_threshold_removes_the_stale_mcc_file` — following case 1, rewrite the same slot with a small (`10`-byte) payload; assert the `.mcc` file no longer exists on disk; assert `read_record` returns the new small payload with the `0x80` bit clear.

### `crates/chunk-storage/tests/anvil_compression_schemes.rs`

1. `zlib_round_trips` / `lz4_round_trips` / `uncompressed_round_trips` (three cases) — for each `CompressionScheme`, `decompress_tagged(scheme.tag(), &scheme.compress(payload)) == Ok(payload)` for a representative payload (a mix of highly-compressible repeated bytes and pseudo-random bytes).
2. `gzip_tag_decodes_but_is_never_produced_by_compress` — hand-construct GZip-compressed bytes of a known payload via `flate2::write::GzEncoder` directly in the test; `CompressionScheme::decompress_tagged(1, &those_bytes) == Ok(payload)`; separately, assert no `CompressionScheme` variant's `.tag()` ever equals `1`.
3. `unknown_compression_tag_is_rejected` — `decompress_tagged(200, &[1,2,3])` is `Err(StorageError::UnknownCompressionType(200))`.
4. `corrupted_compressed_bytes_fail_decompression_not_panic` — `decompress_tagged(2 /* Zlib */, &[0xFF, 0xFE, 0xFD])` (not valid zlib data) returns `Err(StorageError::Decompress(_))`, does not panic.

### `crates/chunk-storage/tests/anvil_corruption_recovery.rs`

1. `bad_location_offset_below_header_is_rejected` — hand-write a `.mca` file whose header's slot-`0` location entry claims `offset=1` (inside the reserved header, illegal); `read_record(0,0)` returns `Err(StorageError::SectorOutOfBounds { .. })`.
2. `location_offset_past_file_end_is_rejected` — hand-write a file whose slot-`0` entry claims `offset=50, count=1` but the file itself is only `3` sectors long; `read_record(0,0)` returns `Err(StorageError::SectorOutOfBounds { .. })`.
3. `declared_length_exceeding_allocated_sectors_is_rejected` — hand-write a valid `1`-sector allocation whose record's `length` field claims `5000` (far exceeding the `4091` payload bytes actually available in one sector after the 5-byte sub-header); `read_record` returns `Err(StorageError::Corrupt { .. })`.
4. `one_corrupt_chunk_does_not_affect_a_sibling_chunk_in_the_same_file` — a `.mca` file with slot `(0,0)` corrupt (per case 1's construction) and slot `(1,0)` holding a valid, distinct record; `read_record(0,0)` errors, `read_record(1,0)` succeeds and returns the correct sibling payload.
5. `nbt_validation_rejects_non_nbt_payload_at_the_backend_level` — (this test operates at `AnvilDiskBackend`, not raw `RegionFile`, since `RegionFile` itself is NBT-agnostic per Context) a fresh `AnvilDiskBackend` in a temp world dir; directly write a non-NBT byte payload through the LOW-LEVEL `RegionFile`/`CompressionScheme` primitives at the exact slot `AnvilDiskBackend::read_chunk` would look up for `(DimensionId::OVERWORLD, RegionFileKind::Terrain, 0, 0)`; calling `backend.read_chunk(DimensionId::OVERWORLD, RegionFileKind::Terrain, 0, 0, None)` returns `Err(StorageError::InvalidNbtPayload(_))`.

### `crates/chunk-storage/tests/anvil_backend_directory_and_level_dat.rs`

1. `open_creates_overworld_directories_eagerly` — fresh temp dir; `AnvilDiskBackend::open`; assert `region/`, `entities/`, `poi/` all exist as directories under the world root; assert `DIM-1/` and `DIM1/` do **not** exist yet.
2. `writing_a_nether_chunk_lazily_creates_dim_minus_1` — following case 1, `write_chunk(DimensionId::THE_NETHER, RegionFileKind::Terrain, 0, 0, <valid nbt bytes>, None)`; assert `DIM-1/region/` now exists; assert `DIM1/` still does not.
3. `unsupported_dimension_id_is_rejected` — `write_chunk(DimensionId(999), ..)` returns `Err(StorageError::UnsupportedDimension(_))`.
4. `chunk_round_trips_through_the_trait_with_real_compression` — write a small, valid, hand-built NBT compound's encoded bytes (via `rc_nbt::owned::{BaseNbt,NbtCompound,NbtTag}` + `rc_nbt::write_owned`) through `write_chunk` at `(Overworld, Terrain, 5, -3)`; `read_chunk` at the same key returns `Ok(Some(bytes))` equal to the original uncompressed bytes (proving the backend's own compress-on-write/decompress-on-read round trip, independent of `RegionFile`'s own already-covered container correctness).
5. `read_chunk_on_a_never_written_region_returns_none_without_creating_a_file` — fresh backend; `read_chunk` at a region no chunk has ever been written to; returns `Ok(None)`; assert the corresponding `.mca` path does **not** exist on disk afterward (Context's "reads never litter the filesystem" rule) and `open_handle_count() == 0`.
6. `level_dat_first_write_has_no_backup_rename` — fresh backend; `write_level_dat(b"...")`; assert `level.dat` exists with the given bytes, `level.dat_old` does **not** exist, `level.dat_new` does not exist (renamed away).
7. `level_dat_second_write_creates_dat_old_backup` — following case 6, `write_level_dat(b"...second version...")`; assert `level.dat` holds the second version, `level.dat_old` holds the first version's exact bytes.
8. `read_level_dat_falls_back_to_dat_old_when_primary_is_corrupt` — after case 7's two writes (using real GZip-compressed valid NBT bytes for both, via `rc_nbt::write_gzip_owned` on a trivial compound, so the "corrupt" case is unambiguous), overwrite `level.dat` directly with garbage bytes; `read_level_dat()` returns `Ok(bytes)` equal to the *first* write's original bytes (the `.dat_old` fallback), not an error.
9. `read_level_dat_errors_when_both_primary_and_backup_are_corrupt_or_missing` — fresh backend, no `write_level_dat` ever called; `read_level_dat()` returns `Err(StorageError::Corrupt { .. })` or `Err(StorageError::Io { .. })` (either acceptable — both `level.dat` and `level.dat_old` are simply absent).
10. `second_open_on_the_same_world_root_fails_with_world_already_open` — `AnvilDiskBackend::open(path.clone(), ..)` succeeds and is kept alive; a second `AnvilDiskBackend::open(path, ..)` in the same process returns `Err(StorageError::WorldAlreadyOpen { .. })`.
11. `dropping_the_backend_releases_the_lock_for_a_subsequent_open` — open, then explicitly `drop` the backend, then open again on the same `world_root`; the second `open` succeeds.

### `crates/chunk-storage/tests/anvil_lru_handle_cache.rs`

1. `handle_count_grows_as_distinct_regions_are_touched` — write one chunk each into `5` distinct regions (far enough apart in `x`/`z` to guarantee `5` distinct `.mca` files); `open_handle_count() == 5`.
2. `revisiting_the_same_region_does_not_grow_the_count` — following case 1, write a second chunk into one of the same `5` regions; `open_handle_count()` is still `5`.
3. `cache_evicts_least_recently_touched_past_256_handles` — write one chunk each into `257` distinct regions, touching them in a fixed, known order; assert `open_handle_count() <= 256`; assert the very FIRST region touched (least-recently-used by the time all 257 have been written) is no longer counted among open handles (re-reading a chunk from it still succeeds correctly, via a fresh re-open — proving eviction is transparent to correctness, only handle-cache residency changes).

### `crates/chunk-storage/tests/anvil_concurrent_access.rs`

1. `concurrent_writes_to_disjoint_chunks_all_succeed_and_are_correct` — a shared `AnvilDiskBackend` (behind an `Arc`); `std::thread::scope` spawns `16` threads, each writing a distinct chunk (`(thread_index, 0)`) with a payload encoding its own thread index; after joining, every chunk is read back via the same backend and matches its writer's own payload exactly — no cross-thread aliasing.
2. `concurrent_reads_and_writes_to_the_same_chunk_never_panic_and_converge` — `8` threads, each alternately writing (with a payload tagged by an increasing generation counter) and reading the SAME chunk key in a loop of `50` iterations; no thread panics; after all threads join, a final read returns a payload matching one of the actually-written generations (proving the per-handle mutex fully serializes access — no torn read ever observed, verified by asserting every successful read's bytes exactly match one of the generation payloads the test tracked, never a mix of two).

### `crates/chunk-storage/tests/anvil_batch_write.rs`

1. `batch_write_places_every_entry_correctly` — `write_chunks_batch` with `20` entries, all in the same `(dim, kind)` but spanning `3` distinct region files; after the call, every one of the `20` entries reads back correctly via ordinary `read_chunk`.
2. `batch_write_with_entries_for_a_never_before_seen_region_creates_it` — a batch whose entries land in a region no prior write ever touched; the call succeeds and the resulting `.mca` file exists with correct content.

### `crates/chunk-storage/tests/anvil_soak_roundtrip.rs`

`ten_thousand_chunk_write_read_round_trips_have_zero_checksum_mismatches` — fresh `AnvilDiskBackend`; loop `i` in `0..10_000`: build a small synthetic NBT compound via `rc_nbt::owned::{BaseNbt, NbtCompound, NbtTag}` (one `Int` field set to `i as i32`, one `String` field `"soak"`), encode via `rc_nbt::write_owned`; compute `pre = content_checksum(&encoded)`; pick `(dim, kind, x, z)` deterministically from `i` (cycling through all three `RegionFileKind`s and the `Overworld`/`THE_NETHER`/`THE_END` dimensions, `x = i as i32 % 4096`, `z = (i / 4096) as i32`, guaranteeing good spread across many region files); `write_chunk(..., &encoded, None)`; `read_chunk(...)` must return `Ok(Some(bytes))`; `post = content_checksum(&bytes)`; assert `pre == post` AND `bytes == encoded`. Track a running mismatch count, asserted `== 0` at the end (in addition to the per-iteration assertion, matching M2's own milestone acceptance criterion 2's exact phrasing).

## Implementation steps

1. **`Cargo.toml`, `lib.rs`.** Add the dependency/module lines exactly as Deliverables. Observable: `cargo build -p rc-chunk-storage` fails only on `todo!()`s.
2. **`error.rs`.** Pure `thiserror` derive bodies, no hand-written logic. Observable: compiles standalone.
3. **`compression.rs`.** `tag`/`compress`/`decompress_tagged` per Context's exact scheme table: Zlib via `flate2::write::ZlibEncoder`/`read::ZlibDecoder` at `Compression::default()`; Lz4 via `lz4_flex::block::{compress_prepend_size, decompress_size_prepended}`; Uncompressed is identity; tag `1` decodes via `flate2::read::GzDecoder`; any other tag is `UnknownCompressionType`. Observable: `anvil_compression_schemes.rs` passes.
4. **`checksum.rs`.** `content_checksum` per its doc comment (`DefaultHasher` + `Hash::hash`). Observable: trivial, exercised indirectly by the soak test.
5. **`region_file.rs`.** `open`: `OpenOptions::new().read(true).write(true).create(true).open(path)`, `metadata()?.len()` drives the three-way structural-validity branch (Context); on the fresh/zero-length path, write the 8192-byte zero header immediately and set `file_sectors = 2`; on the normal path, read the full 8192-byte header, decode 1024+1024 big-endian `u32` entries into `locations`/`timestamps`, set `file_sectors = file_len / 4096`. `read_record`: decode the slot's location entry (`offset = entry >> 8`, `count = (entry & 0xFF) as u8`); `Ok(None)` if `entry == 0`; bounds-check `offset >= 2 && offset as u64 + count as u64 <= file_sectors as u64`, else `SectorOutOfBounds`; seek+`read_exact` the `count*4096`-byte block; parse `length`/`compression_tag` from its first 5 bytes (`Corrupt` if the block is shorter than 5 bytes or `length == 0`); if the tag's `0x80` bit is set, compute the `.mcc` path from `region_x*32+local_x`/`region_z*32+local_z` and `std::fs::read` it (`MissingExternalFile` on any read error); else slice out `length - 1` payload bytes starting at offset `5` (`Corrupt` if that would read past the allocated block). `write_record`: compute whether this write must go external (`4 + 1 + data.len() > 255 * 4096`); if so, write `data` verbatim to the `.mcc` path (create/truncate/write_all/`sync_data`) and set the in-region payload to empty with the tag's `0x80` bit set; else remove any stale `.mcc` file for this slot (best-effort) and use `data` as the in-region payload directly. Compute `sectors_needed`, call a private `compute_free_ranges(&self) -> Vec<(u32,u32)>` helper (Context's exact bitmap-scan algorithm: build a `Vec<bool>` of length `file_sectors`, mark every currently-claimed sector `true` from every non-zero location entry — clipped at `file_sectors`, never indexing out of bounds — then collect maximal `false` runs starting from index `2`), first-fit-select a range (or append at end-of-file), build the sector-aligned buffer, seek+`write_all`+`sync_data`, update `locations[idx]`/`timestamps[idx]` in memory and on disk (two small seek+`write_all` calls at their fixed header offsets), `sync_data()` again. `timestamp`/`free_sector_summary` are thin wrappers over the same decode/`compute_free_ranges` logic. Observable: `anvil_header_and_indexing.rs`, `anvil_write_read_roundtrip.rs`, `anvil_sector_reuse_and_fragmentation.rs`, `anvil_mcc_overflow.rs`, `anvil_corruption_recovery.rs` (cases 1-4) all pass.
6. **`backend.rs`.** `RegionFileKind::folder_name` is the trivial match. `AnvilDiskBackend::open`: validate/create `world_root`, `create_dir_all` the Overworld's three directories, open-or-create `session.lock` and call `try_lock()` (verify the exact stable-`std` signature against the installed 1.97.0 toolchain's docs first, per Context), map a contended lock to `WorldAlreadyOpen`, hold the `File` for the backend's own lifetime. Path-resolution helpers: `dimension_folder(dim) -> Result<&'static str, StorageError>` (Context's three-arm match plus `UnsupportedDimension`), `region_file_path(world_root, dim, kind, region_x, region_z) -> Result<PathBuf, StorageError>` joining `world_root`/`dimension_folder`/`kind.folder_name()`/`format!("r.{region_x}.{region_z}.mca")`, `create_dir_all`ing the kind directory lazily on first use (per-dimension, tracked or simply called unconditionally — `create_dir_all` on an already-existing directory is a harmless no-op, so no separate "already created" tracking is required). `get_or_open_handle(dim, kind, region_x, region_z, create: bool) -> Result<Option<Arc<Mutex<RegionFile>>>, StorageError>`: look up in the `handles` map under its own lock; on a hit, update the entry's last-touch time, return `Ok(Some(..))`; on a miss with `create == false`, check the target path's existence via `Path::exists()` — if absent, return `Ok(None)` without opening or caching anything (Context's "reads never litter the filesystem" rule); on a miss with `create == true` (or the path already exists), evict (cap-based, `>= 256`, least-recently-touched by linear scan — trivially cheap at this size; plus opportunistic idle-`>60s` eviction checked on every call, Context) before inserting, then `RegionFile::open` the path and insert. `read_chunk`: resolve the path via `create=false`; on `Ok(None)` return `Ok(None)`; else lock the handle, `read_record`, on `Some((tag,bytes))` decompress via `CompressionScheme::decompress_tagged(tag & 0x7F, &bytes)`, then validate via `rc_nbt::read_borrowed_strict(&raw).map_err(|e| InvalidNbtPayload(e.to_string()))`, return `Ok(Some(raw))`. `write_chunk`: compress `payload` via `self.compression`, resolve the path via `create=true`, lock the handle, `write_record(local_x, local_z, self.compression.tag(), &compressed)`. `read_level_dat`/`write_level_dat`: exactly Context's atomic-with-backup algorithm. `write_chunks_batch`: `debug_assert!` every entry shares `(dim,kind)`'s implied region grouping is computed correctly; group entries by `(region_x,region_z)`, for each group acquire that region's handle lock ONCE and call `write_record` for every entry in the group under that single lock hold (the syscall-count win PERF-D28 describes is the single lock acquisition plus each `write_record`'s own already-minimal two-`sync_data`-call shape — no further special batching of the `sync_data` calls themselves is required by this blueprint's own scope, see Constraints). `open_handle_count`: `self.handles.lock().len()`. Observable: `anvil_corruption_recovery.rs` case 5, `anvil_backend_directory_and_level_dat.rs`, `anvil_lru_handle_cache.rs`, `anvil_concurrent_access.rs`, `anvil_batch_write.rs`, `anvil_soak_roundtrip.rs` all pass.
7. **`crates/chunk-storage/fuzz/`.** Already complete from the test changeset — no implementation-changeset edit.
8. **Run the full acceptance suite.** `cargo nextest run -p rc-chunk-storage` — every test in every file under Acceptance tests passes.
9. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0.
10. **One-time local fuzz sanity check (not a CI gate).** `cargo check --manifest-path crates/chunk-storage/fuzz/Cargo.toml` succeeds; if a local nightly toolchain is available, `cargo +nightly fuzz run anvil_roundtrip -- -max_total_time=30` and `-- -max_total_time=30` for `anvil_decode_never_panics` both build and run without an immediate crash.
11. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/chunk-storage/tests/` (including `tests/support/`) and the complete `crates/chunk-storage/fuzz/` crate are committed first, alongside `todo!()`-stubbed `src/anvil/*.rs` (full struct fields, derives, doc comments already final) and the `Cargo.toml`/`lib.rs` edits. The implementation changeset fills in real bodies only — it must not edit any test file, must not add/remove/rename any test case, and must not weaken any assertion (in particular, every hand-derived sector-count/free-range expectation in `anvil_write_read_roundtrip.rs`/`anvil_sector_reuse_and_fragmentation.rs`/`anvil_mcc_overflow.rs` must survive unchanged).

(b) **No new external dependencies beyond the pinned set, with the fuzz crate's own cited exception.** `rc-chunk-storage` itself uses only `flate2`, `lz4_flex`, `parking_lot`, `thiserror` (all already in `[workspace.dependencies]`) as this blueprint's new edges, plus `rc-nbt` (M2-B02, already an existing dependency edge, merely unused until now). Do not add a dedicated LRU-cache crate, a file-locking crate (`fs2`/`fs4`), a checksum/CRC crate, or the `object_store`/`io-uring` crates' actual *usage* (both stay unused at this milestone, Context's Scope boundary) — every one of those capabilities is built from `std` plus the four newly-added pinned crates exactly as Deliverables/Implementation steps specify. `crates/chunk-storage/fuzz/` is the one place this blueprint adds dependencies not present in the root workspace table (`libfuzzer-sys`, `arbitrary`), exactly TEST-D25's pinned versions, mirroring M2-B02's identical precedent.

(c) **No Mojang or third-party reimplementation code.** Every byte layout, algorithm, and threshold in this blueprint is restated from `docs/research/mc-26.2/04-persistence-nbt.md` and `03-world-chunks-persistence.md`'s own WORLD-D12/D13/D14 (themselves produced under the ASSET-D18(f)/D30 research process) in this blueprint's own words; the `mca` crate (`VilleOlof/mca`) is cited by WORLD-D12 only as independent confirmation that this understanding of the format is correct, never consulted or copied as code (ASSET-D18/D19/D30).

(d) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: `ObjectStoreBackend` (WORLD-D17/D18, a later milestone); `IoUringAnvilDiskBackend` or any other use of the already-wired-but-inert `io_uring` Cargo feature (PERF-D23); Stage-9 dirty-chunk save scheduling, the autosave-interval timer, or any wiring into an actual `RC-IoPool` thread pool (WORLD-D20/D21/D23 — a future `rc-scheduler`-adjacent blueprint calls this crate's already-synchronous `write_chunk`/`write_chunks_batch` from wherever that pool ends up living); a dedicated background thread for PERF-D29's idle-handle sweep (this blueprint's own opportunistic-check-on-access interpretation stands as shipped, Context); any real `ChunkColumn`/entity/POI NBT schema (WORLD-D6/D11/D29 — this blueprint's own tests use small synthetic NBT compounds standing in for real chunk payloads); `RegionManifest`/`ChunkSnapshot`/any cluster-migration format (WORLD-D19/D20). Do not add placeholder implementations of any of these as a shortcut.

(e) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust (sector/header arithmetic is ordinary integer math over `std::fs::File` via `Seek`/`Read`/`Write`; no positional-I/O platform trait is used — Context's `region_file.rs` steps use plain `seek` + `read_exact`/`write_all`, which is fully portable and needs no `std::os::{unix,windows}::fs::FileExt` split).

(f) **`RegionFile`'s always-fresh-allocation rule is binding, not an optimization opportunity.** Do not add an in-place-reuse fast path for the case where a rewrite's new size fits within the chunk's own old sector range, even though it would save I/O in the common case — Context explains exactly why the uniform rule is what makes the crash-safety property hold without any special-casing; a future performance-focused blueprint may revisit this deliberately, with its own equivalence argument, but this blueprint's implementation changeset does not.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-chunk-storage --all-features
cargo nextest run -p rc-chunk-storage
cargo test --doc -p rc-chunk-storage
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
```

Expected: every command exits 0. `cargo nextest run -p rc-chunk-storage` runs `anvil_header_and_indexing.rs` (6) + `anvil_write_read_roundtrip.rs` (5) + `anvil_sector_reuse_and_fragmentation.rs` (3) + `anvil_mcc_overflow.rs` (4) + `anvil_compression_schemes.rs` (6, counting each of the three round-trip cases separately) + `anvil_corruption_recovery.rs` (5) + `anvil_backend_directory_and_level_dat.rs` (11) + `anvil_lru_handle_cache.rs` (3) + `anvil_concurrent_access.rs` (2) + `anvil_batch_write.rs` (2) + `anvil_soak_roundtrip.rs` (1) = 48 test cases named in Acceptance tests — all pass (plus M2-B01's own suite, if landed, unaffected by this blueprint's disjoint files). CI (`.github/workflows/ci.yml`, `M0-B01`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
