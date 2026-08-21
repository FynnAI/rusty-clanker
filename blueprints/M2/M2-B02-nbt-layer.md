# M2-B02 — NBT Layer (`rc-nbt`)

| Field | Content |
|---|---|
| ID | M2-B02 |
| Milestone | M2 — Persistent World Storage |
| Prerequisites | M0 complete (in particular M0-B01: `crates/nbt/` already exists as an empty-shell crate, `rc-core` path-dependency already declared in its `Cargo.toml`, `simdnbt = "0.10.0"` already pinned in the workspace root's `[workspace.dependencies]`). Parallel-safe with M2-B01 — this blueprint does not read or depend on any type M2-B01 introduces. |
| Implements | WORLD-D11 (NBT crate choice, `simdnbt` 0.10.0, `borrow`/`owned` split, hand-written — never derived — schema conversion); WORLD-D15's compression clause (GZip for `level.dat`/player data, restated as this crate's convenience entry points; the schemas themselves are B04/B06's job, not this blueprint's); the parts of TEST-D25–D28, TEST-D39, TEST-D45–D47 this crate's own test/property/fuzz suite must satisfy |
| Crates touched | `rc-nbt` (`crates/nbt/`) — filled in from M0-B01's empty shell; a new, deliberately **non-member** crate `crates/nbt/fuzz/` (cargo-fuzz convention, excluded from the root workspace via its own `[workspace]` table) |
| Estimated scope | L |

## Goal & Done definition

Turn `rc-nbt` from M0-B01's empty-shell placeholder into the engine's one NBT implementation boundary: a thin, correctly-scoped wrapper over `simdnbt` 0.10.0 giving every future blueprint (starting with B04's `level.dat` schema and B06's player-data schema) a typed read/write entry point, a stable error taxonomy, and a hand-written (never `#[derive]`-based, per WORLD-D11) schema-conversion helper layer to build vanilla NBT schemas on top of. No vanilla schema (chunk, `level.dat`, player, entity, block-entity) is implemented here — this blueprint is pure infrastructure, exercised in its own tests only via small synthetic examples.

Done when:

- [ ] `cargo build -p rc-nbt --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-nbt`.
- [ ] `cargo test --doc -p rc-nbt` exits 0.
- [ ] `cargo run -p xtask -- fmt-check`, `cargo run -p xtask -- lint`, `cargo run -p xtask -- lint-deps` all still exit 0 (no `xtask lint-deps` rule names `rc-nbt`; this blueprint touches no protected path, TEST-D46).
- [ ] `crates/nbt/fuzz/` type-checks: `cargo +nightly fuzz build` (or, absent a local nightly toolchain, at minimum `cargo check --manifest-path crates/nbt/fuzz/Cargo.toml` against a `--cfg fuzzing`-free compile — see Constraints for exactly what is and is not required here) succeeds locally at least once; this step is **not** part of this blueprint's own CI-required verification commands (nightly wiring is out of scope — see Constraints).
- [ ] Every known-answer byte vector in Acceptance tests decodes to, and every corresponding value re-encodes to, the exact bytes given — byte-for-byte, asserted with `assert_eq!` on `Vec<u8>`/slices, never approximately.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, plus M0-B08's `path-guard`/`lint-tests`/`verify-fixtures` — all already-existing gates, none newly added by this blueprint) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### What this crate is, and is not

`rc-nbt` (`crates/nbt/`) is the *only* place any other Rusty Clanker crate touches `simdnbt` directly for **vanilla-schema** data (chunk NBT, `level.dat`, player data, entities, block entities, POI records). `rc-protocol` is a separate, independent consumer of `simdnbt` for wire-level NBT (chat components, NET-D5) and is not this blueprint's concern — WORLD-D11's own rationale explicitly frames `simdnbt` as "a general-purpose, MC-protocol-agnostic NBT library," so two crates depending on it directly creates no coupling between them. This blueprint does **not** implement: the chunk-column schema (`ChunkColumn::to_nbt`/`from_nbt`, WORLD-D11's own named example — a later `rc-chunk-storage` blueprint), the Anvil `.mca` region-file container (WORLD-D12, M2-B01 or a sibling blueprint), `level.dat` (a later B04-numbered blueprint), player data (a later B06-numbered blueprint), or SNBT (see its own subsection below — explicitly deferred). What it delivers is the shared foundation every one of those needs: typed read/write entry points, an error taxonomy, and a schema-conversion helper layer.

### Vanilla NBT binary format — restated field-precise (source: `docs/research/mc-26.2/04-persistence-nbt.md` §3.1, §5)

**Tag type IDs** (also each concrete tag's `id()`/discriminant byte):

| ID | Type | ID | Type | ID | Type |
|---|---|---|---|---|---|
| 0 | End | 5 | Float | 10 | Compound |
| 1 | Byte | 6 | Double | 11 | Int Array |
| 2 | Short | 7 | Byte Array | 12 | Long Array |
| 3 | Int | 8 | String | | |
| 4 | Long | 9 | List | | |

**Root framing.** An unnamed/named root document is `[tag_id: u8][name: modified-UTF-8 string][payload]` — the outermost tag is always a Compound (`tag_id = 10`) in every persisted artifact this project writes (WORLD-D14's folder layout, WORLD-D15's `level.dat`, WORLD-D29's entity/POI files); a bare non-Compound root is a format vanilla itself never produces for on-disk files and this crate does not specially support at the entry-point level (`simdnbt::{borrow,owned}::read` themselves already enforce "Compound or nothing," returning `Error::InvalidRootType` otherwise — see Error taxonomy below).

**Compound payload.** `[tag_id][name][payload]` repeated per entry, terminated by a single `TAG_End` byte (`0x00`) with no trailing name/payload of its own. Entry order is **preserved** (not resorted, not hash-bucketed) — `simdnbt`'s own `owned::NbtCompound` doc text says exactly this ("the order of the tags is preserved"), unlike vanilla's own Java `HashMap`-backed `CompoundTag`, which does *not* preserve insertion order (research doc §8: "do not build any Rusty Clanker test or hashing scheme that assumes stable raw bytes for unmodified data"). This crate inherits `simdnbt`'s deterministic, insertion-order behavior — a documented, harmless deviation from vanilla's own unspecified `HashMap` order, exactly the kind of safe deviation the research doc's own §8 note calls out as worth deciding explicitly rather than leaving implicit.

**List payload.** `[element_tag_id: u8][count: i32 BE][elements...]` — homogeneous, **no per-element type byte**; an empty list is written with `element_tag_id = 0` (`TAG_End`), matching vanilla's own convention for a zero-length list.

**Numeric encoding.** All multi-byte numeric tags are **big-endian**, fixed-width: Byte = 1 byte (signed `i8`), Short = 2 bytes (`i16`), Int = 4 bytes (`i32`), Long = 8 bytes (`i64`), Float = 4 bytes (IEEE-754 `f32`), Double = 8 bytes (IEEE-754 `f64`). Array tags (`ByteArray`/`IntArray`/`LongArray`) are `[count: i32 BE][elements, each in that same big-endian fixed width, back-to-back, no padding]`.

**String encoding — the modified-UTF-8 (MUTF-8) caveat.** A String tag's payload is `[length: u16 BE][length bytes of MUTF-8-encoded text]` — the length field counts **encoded bytes**, not characters, matching Java's `DataOutput.writeUTF` convention the research doc names directly. MUTF-8 differs from standard UTF-8 in exactly two ways, both restated here field-precise because they are the two cases this blueprint's Acceptance tests hand-derive byte-for-byte:
  1. **U+0000 (NUL)** is never encoded as a literal `0x00` byte (which would collide with C-string/vanilla-internal null-termination conventions) — it is instead encoded as the **overlong two-byte sequence `0xC0 0x80`**.
  2. **Supplementary-plane code points** (above U+FFFF) are not encoded as 4-byte UTF-8 — they are instead encoded as a **CESU-8-style UTF-16 surrogate pair**, where each of the two 16-bit surrogate halves is independently run back through ordinary 3-byte UTF-8 encoding, producing **6 bytes total** instead of standard UTF-8's 4. Worked example (used verbatim in Acceptance tests): U+10000 (the first supplementary-plane code point) has UTF-16 surrogate pair `0xD800` (high) / `0xDC00` (low); each surrogate half, encoded as an ordinary 3-byte UTF-8 sequence, gives `0xED 0xA0 0x80` and `0xED 0xB0 0x80` respectively — MUTF-8 bytes `ED A0 80 ED B0 80` (6 bytes total), versus standard UTF-8's `F0 90 80 80` (4 bytes) for the same code point.

  A string's encoded length is capped at 65535 bytes (the `u16` length field's range) by construction in vanilla's own writer, which the research doc notes silently substitutes an empty string on overflow rather than erroring (`StringFallbackDataOutput`). This crate does **not** replicate that fallback — see Known limitation below.

### `simdnbt` 0.10.0's actual public API — verified against the live docs.rs page for the pinned version on 2026-08-21, resolving 03's own flagged Open Question ("simdnbt's exact owned/borrow API surface... needs re-verification... once implementation starts")

Two parallel modules, `simdnbt::borrow` (zero-copy, lifetime-tied to the caller's buffer) and `simdnbt::owned` (self-contained, heap-owned):

```rust
// simdnbt::borrow — every type additionally carries a second lifetime 'tape (the
// internal parse-index structure simdnbt builds once per document) alongside 'a
// (the original byte buffer's lifetime).
pub enum Nbt<'a> { Some(BaseNbt<'a>), None }
pub struct BaseNbt<'a> { /* private */ }
impl<'a> BaseNbt<'a> {
    pub fn name(&self) -> &'a Mutf8Str;
    pub fn as_compound<'tape>(&'a self) -> NbtCompound<'a, 'tape> where 'a: 'tape;
    pub fn to_owned(&self) -> simdnbt::owned::BaseNbt;
    pub fn write(&self, data: &mut Vec<u8>);
    // plus name()-scoped convenience accessors (byte/short/int/.../compound) mirroring
    // NbtCompound's own, via an internal Deref<Target = NbtCompound<'a, 'tape>>-shaped
    // relationship — this blueprint's own code always goes through `.as_compound()`
    // explicitly rather than relying on that implicit surface, for clarity.
}
pub struct NbtCompound<'a, 'tape> { /* private */ }
impl<'a, 'tape> NbtCompound<'a, 'tape> {
    pub fn get(&self, name: &str) -> Option<NbtTag<'a, 'tape>>;
    pub fn contains(&self, name: &str) -> bool;
    pub fn byte(&self, name: &str) -> Option<i8>;
    pub fn short(&self, name: &str) -> Option<i16>;
    pub fn int(&self, name: &str) -> Option<i32>;
    pub fn long(&self, name: &str) -> Option<i64>;
    pub fn float(&self, name: &str) -> Option<f32>;
    pub fn double(&self, name: &str) -> Option<f64>;
    pub fn byte_array(&self, name: &str) -> Option<&'a [u8]>;
    pub fn string(&self, name: &str) -> Option<&'a Mutf8Str>;
    pub fn list(&self, name: &str) -> Option<NbtList<'a, 'tape>>;
    pub fn compound(&self, name: &str) -> Option<NbtCompound<'a, 'tape>>;
    pub fn int_array(&self, name: &str) -> Option<Vec<i32>>;
    pub fn long_array(&self, name: &str) -> Option<Vec<i64>>;
    pub fn iter(&self) -> NbtCompoundIter<'a, 'tape>;
    pub fn keys(&self) -> impl Iterator<Item = &'a Mutf8Str>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn to_owned(&self) -> simdnbt::owned::NbtCompound;
    pub fn write(&self, data: &mut [Vec<u8>]);
}
pub struct NbtTag<'a, 'tape> { /* private */ }
impl<'a, 'tape> NbtTag<'a, 'tape> {
    pub fn id(&self) -> u8; // the Tag-ID table above
    pub fn byte(&self) -> Option<i8>;
    pub fn short(&self) -> Option<i16>;
    pub fn int(&self) -> Option<i32>;
    pub fn long(&self) -> Option<i64>;
    pub fn float(&self) -> Option<f32>;
    pub fn double(&self) -> Option<f64>;
    pub fn byte_array(&self) -> Option<&'a [u8]>;
    pub fn string(&self) -> Option<&'a Mutf8Str>;
    pub fn list(&self) -> Option<NbtList<'a, 'tape>>;
    pub fn compound(&self) -> Option<NbtCompound<'a, 'tape>>;
    pub fn int_array(&self) -> Option<Vec<i32>>;
    pub fn long_array(&self) -> Option<Vec<i64>>;
    pub fn to_owned(&self) -> simdnbt::owned::NbtTag;
}
pub fn read<'a>(data: &mut std::io::Cursor<&'a [u8]>) -> Result<Nbt<'a>, simdnbt::Error>;
```

```rust
// simdnbt::owned — heap-owned, no lifetime parameter.
pub enum Nbt { Some(BaseNbt), None }
pub struct BaseNbt { /* private */ }
impl BaseNbt {
    pub fn new(name: impl Into<Mutf8String>, tag: NbtCompound) -> Self;
    pub fn name(&self) -> &Mutf8Str;
    pub fn as_compound(self) -> NbtCompound; // consumes self — owned variant, no re-borrow needed
    pub fn write(&self, data: &mut Vec<u8>);
    pub fn write_unnamed(&self, data: &mut Vec<u8>);
    // Deref<Target = NbtCompound> — get/byte/short/.../compound/iter/len/is_empty all
    // available directly on a `&BaseNbt`, exactly as on `NbtCompound` below.
}
pub struct NbtCompound { /* private, ordered Vec<(Mutf8String, NbtTag)>-shaped */ }
impl NbtCompound {
    pub fn new() -> Self;
    pub fn from_values(values: Vec<(Mutf8String, NbtTag)>) -> Self;
    pub fn get(&self, name: &str) -> Option<&NbtTag>;
    // byte/short/int/long/float/double/byte_array/string/list/compound/int_array/long_array,
    // each `(&self, name: &str) -> Option<...>`, identical shape to the borrow variant above
    pub fn insert(&mut self, name: impl Into<Mutf8String>, tag: impl simdnbt::ToNbtTag);
    pub fn take(&mut self, name: &str) -> Option<NbtTag>;
    pub fn remove(&mut self, name: &str) -> Option<NbtTag>;
    pub fn contains(&self, name: &str) -> bool;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = (&Mutf8Str, &NbtTag)>;
    pub fn write(&self, data: &mut Vec<u8>);
}
pub enum NbtTag {
    Byte(i8), Short(i16), Int(i32), Long(i64), Float(f32), Double(f64),
    ByteArray(Vec<u8>), String(Mutf8String), List(NbtList), Compound(NbtCompound),
    IntArray(Vec<i32>), LongArray(Vec<i64>),
}
impl NbtTag {
    pub fn id(&self) -> u8;
    pub fn write(&self, data: &mut Vec<u8>);
    // byte()/short()/.../compound()/int_array()/long_array(), each `(&self) -> Option<...>`
}
pub fn read(data: &mut std::io::Cursor<&[u8]>) -> Result<Nbt, simdnbt::Error>;
```

```rust
// simdnbt crate root
pub struct Mutf8Str { /* unsized, str-like */ }
impl Mutf8Str {
    pub fn from_slice(slice: &[u8]) -> &Mutf8Str;      // zero validation — arbitrary bytes accepted
    pub fn from_str(s: &str) -> std::borrow::Cow<'_, Mutf8Str>; // the actual MUTF-8 *encoder*
    pub fn to_str(&self) -> std::borrow::Cow<'_, str>; // lossy: malformed input yields "" per crate docs, never panics
    pub fn as_bytes(&self) -> &[u8];
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
pub struct Mutf8String { /* owned, Vec<u8>-backed */ } // Deref<Target = Mutf8Str>
impl Mutf8String {
    pub fn new() -> Self;
    pub fn from_vec(vec: Vec<u8>) -> Self;             // zero validation — arbitrary bytes accepted
    pub fn from_string(s: String) -> Self;
    pub fn into_string(self) -> String;                // lossy, same "" fallback as to_str()
    pub fn try_into_string(self) -> Result<String, simdnbt::DeserializeError>; // strict escape hatch, owned-only
}
// From<&str>/From<String> for Mutf8String also exist — used throughout this blueprint's
// Deliverables/tests wherever a literal name/value is passed as `impl Into<Mutf8String>`.

pub enum Error { InvalidRootType(u8), UnknownTagId(u8), UnexpectedEof, MaxDepthExceeded } // implements std::error::Error
```

Two crate default features, both **disabled** by this blueprint's `Cargo.toml` (Deliverables below): `derive` (pulls in `simdnbt-derive`, the exact derive-based mapping WORLD-D11 forbids using for vanilla schemas) and `serde` (gives `simdnbt`'s own types `serde` impls this crate has no use for — `rc-nbt`'s own error/path types get their own, separate, hand-written `Display`/`Debug` via `thiserror`, no `serde` needed anywhere in this crate).

### Zero-copy read-path policy (WORLD-D11)

`borrow` is the default for every read this crate exposes over an **already-in-memory, already-decompressed** byte slice — this is the hot path relative to `RC-IoPool` (WORLD-D21) that WORLD-D11 names explicitly. `owned` is used only where a caller genuinely needs a value outliving the source buffer, or where the source buffer does not exist yet as a plain slice because it must first be decompressed (see next subsection — `level.dat`'s GZip wrapping is exactly this case, since decompression itself must produce a fresh, owned `Vec<u8>`; there is nothing to zero-copy *into*, only *out of*, and the "out of" step still uses `borrow` internally where this blueprint's own `read_gzip_owned` chooses to decode via `owned::read` for API-shape symmetry with `level.dat`'s own single-round-trip, non-hot-path nature per WORLD-D15). **What this blueprint explicitly does not decide**: whether the Anvil chunk-payload zlib/LZ4 decompression itself happens before or as part of handing bytes to this crate — that is WORLD-D13/WORLD-D17's `rc-chunk-storage` concern (a sibling blueprint); this crate's `read_borrowed`/`read_borrowed_strict` simply accept an already-decompressed `&[u8]` regardless of who produced it or how.

### Compression stance — GZip convenience only, chunk-payload compression stays out of scope

WORLD-D15 fixes `level.dat` (and, by WORLD-D29/§3.13 of the research doc, player data) as **always** GZip-compressed, non-configurable — unlike WORLD-D13's chunk-payload compression, which is operator-selectable among Zlib/LZ4/none and is entirely `rc-chunk-storage`'s own concern (a sibling blueprint choosing and applying that scheme, then handing this crate already-decompressed bytes, as the previous subsection states). Because GZip is the *fixed, non-negotiable* framing for exactly two future schema kinds this crate's own future consumers (B04, B06) will need on day one, this blueprint provides `read_gzip_owned`/`write_gzip_owned` convenience wrappers using `flate2` (already workspace-pinned at `1.1.9` with the `zlib-ng` backend, NET-D5's own pin, reused here — zero new dependency). This crate never gains Zlib- or LZ4-specific wrapper functions — those belong to whichever crate implements WORLD-D13's compression-scheme selection.

### SNBT stance at M2 — not implemented, module path reserved only

`12-workspace-structure.md`'s crate-manifest text names an eventual "SNBT (stringified-NBT) text reader/writer for command arguments and data-tag literals" as part of `rc-nbt`'s long-run responsibility. That surface is **not** built by this blueprint. Two independent reasons, both already on record: (1) M2's own roadmap scope is world storage, not commands — SNBT's actual consumers (`/data` command arguments, `rc-brigadier`) belong to `05-game-mechanics.md`'s milestone, not M2; (2) the research doc's own §8 note flags that "a minimal string-in-string-out SNBT implementation is not sufficient if command-argument parity... is ever a goal" — vanilla's real SNBT is a full packrat grammar with source-position-precise error reporting (research doc §3.3), a scope decision that document explicitly says belongs to `06-modding-api.md`/`02-protocol-networking.md`, not this one. This blueprint's only concrete action here is a **reservation**: no `rc_nbt::snbt` module exists after this blueprint lands, and no other module claims that name, so the blueprint that eventually implements SNBT adds `crates/nbt/src/snbt.rs` without moving or renaming anything this blueprint creates.

### Error taxonomy

Two independent error enums, at two independent layers:

**`NbtError`** — byte-level read/write failures (decode, decompress, this crate's own trailing-bytes strictness check). Wraps `simdnbt::Error` verbatim via `#[from]` (that type already implements `std::error::Error`/`Debug`/`Display`/`PartialEq`/`Copy`/`Clone`, confirmed against the live docs.rs page above) rather than re-deriving its four variants (`InvalidRootType`, `UnknownTagId`, `UnexpectedEof`, `MaxDepthExceeded`) a second time.

**`SchemaError`** — typed struct ↔ NBT compound conversion failures from the schema-conversion helper layer (`FromNbtCompound`/`NbtCompoundExt`, below), always carrying an `NbtPath` pinpointing exactly which field failed and why. This design is an original, from-scratch adaptation of the *concept* the research doc documents at §3.11 for vanilla's own `ValueInput`/`ValueOutput` layer — "a decode/encode failure on one field... recorded against a hierarchical path... e.g. `chunk[3,-1].entities[2].Item`" — reused here as an architecture-lesson only (no vanilla code consulted, matching every other concept-only adoption already on record in this project, e.g. WORLD-D7's identical treatment of Starlight). Unlike vanilla's `ProblemReporter`, this blueprint's `SchemaError` is a single bail-on-first-failure `Result`, not an accumulating best-effort collector — a deliberately simpler starting point; a future blueprint may layer an accumulating variant on top of `NbtPath` if B04/B06 ever need vanilla's own "keep loading, just report the field that failed" tolerance, but nothing in this blueprint requires it yet.

### Schema-conversion helper layer — what it is and is not

`ToNbtCompound`/`FromNbtCompound` are the two hand-written-per-type traits B04 (`level.dat`)/B06 (player data) implement — never a `#[derive(...)]`, per WORLD-D11's own explicit ruling that vanilla's schema shapes do not map cleanly enough onto idiomatic Rust struct layout for a derive to reproduce byte-exact vanilla schema. `NbtCompoundExt` is this blueprint's own value-add: `Result`-returning, `SchemaError`-producing "require a field or fail with a precise path" accessors layered over `simdnbt::borrow::NbtCompound`'s existing `Option`-returning accessors (which remain directly usable, unwrapped, for genuinely optional fields — `NbtCompoundExt` does not replace them, only adds the "this field is required" case `simdnbt` itself has no opinion on).

### Test/property/fuzz toolchain — what is newly pinned, and where

`proptest` is already workspace-pinned at `1.11.0` (added by M0-B02, TEST-D27's exact version) — reused directly as a `rc-nbt` dev-dependency, no new pin needed, for the round-trip property tests over arbitrary tag trees this blueprint's Acceptance tests specify.

`cargo-fuzz`/`libfuzzer-sys`/`arbitrary` are **not** added to the root workspace's `[workspace.dependencies]` — per standard `cargo-fuzz` project shape (what `cargo fuzz init` itself generates), the fuzz target lives in its own crate, `crates/nbt/fuzz/`, carrying its own `[workspace]` table so Cargo never tries to unify it into the root workspace's dependency resolution (mirroring, for a non-member crate, the same "pin directly, cite the source decision, do not touch `[workspace.dependencies]`" pattern WS-D7 already establishes for `xtask`'s `clap`/`xshell`). Versions are TEST-D25's own exact pins, cited directly in that crate's `Cargo.toml`: `libfuzzer-sys = "0.4.13"`, `arbitrary = { version = "1.4.2", features = ["derive"] }`. `cargo-fuzz` **0.13.2** itself is an installed CLI tool (`cargo install cargo-fuzz --locked --version 0.13.2`), never a `Cargo.toml` dependency — the same category as `cargo-nextest`, WS-D10.

TEST-D26 item (2) ("NBT decode... `simdnbt`'s zero-copy borrowed-buffer decode entry point... since it parses untrusted network bytes directly") names `NET-D5` in its own parenthetical, which could be read as scoping that fuzz target to `rc-protocol`. This blueprint resolves that reading concretely: **the actual call site of `simdnbt::borrow::read` lives in `rc-nbt`**, per this crate's own "thin wrapper, one boundary" architecture (`12`'s crate-manifest text) — `rc-protocol` (once it exists) calls through `rc-nbt`'s own `read_borrowed`, never `simdnbt` directly for this purpose. The fuzz target this blueprint ships therefore satisfies TEST-D26 item (2) at the crate that structurally owns the decode entry point, regardless of which downstream crate's traffic (network-received, per NET-D5, or disk-read, per WORLD-D11) eventually drives it — one fuzz target, one call site, both consumers covered.

### Compatibility-check strategy against vanilla-produced NBT samples (TEST-D41/D47, restated)

No Mojang-derived byte is ever committed to this repository (ASSET-D18/D19, TEST-D38). A real compatibility check therefore needs a **freshly, locally, or CI-produced** vanilla sample — never a checked-in fixture. `xtask setup-oracle` (TEST-D41, already implemented by M0-B08) already reserves the on-disk location this blueprint's strategy targets — `oracle/<version>/harness/` — but, as it stands today, only downloads the pinned `server.jar` and runs its `--reports` data generator; it does **not** launch the server itself, so it produces no `level.dat` or any other save-format sample yet. Actually running the vanilla server long enough to produce a fresh world save is `rc-test-harness`'s job (TEST-D7), a crate that does not exist before a later milestone. This blueprint's strategy is therefore: (1) fix the exact, git-ignored path a real sample would live at (`oracle/26.2/harness/samples/level.dat`, inside `setup-oracle`'s already-`.gitignore`d `oracle/` root); (2) ship one concrete, `#[ignore]`d test asserting exactly what a real sample must decode to once the harness exists (Acceptance tests, below) — satisfying TEST-D49's linked-issue rule for `#[ignore]` by naming a tracking issue the implementer opens at commit time; (3) leave the actual harness-building work — and therefore actually running this test — to whichever future blueprint first implements `rc-test-harness`. This is an honest, structurally-correct placeholder, not a claim of working oracle-backed verification this blueprint does not actually deliver.

### Known limitation, not solved by this blueprint

`simdnbt`'s own writer performs no length validation on a `Mutf8String`'s encoded byte length before writing its `u16` length prefix (the crate's own docs describe it as deliberately skipping "string/integer validation" for speed) — unlike vanilla, which silently substitutes an empty string on a >65535-byte overflow (research doc §3.1's `StringFallbackDataOutput`) rather than corrupting the length field. This crate does not add that guard either at M2: no vanilla field this project's pinned target actually persists is anywhere near that size, so the risk is theoretical at this milestone's scope. A future blueprint should add an explicit oversized-string check to `write_owned`/`write_gzip_owned` if this ever proves reachable (e.g. a maliciously long player-set sign/book text funneled all the way to a raw NBT string without prior length clamping elsewhere in the stack).

## Deliverables

### `crates/nbt/Cargo.toml` (modify — M0-B01 already declares `rc-core`; this blueprint adds three lines)

```toml
[package]
name = "rc-nbt"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
simdnbt = { workspace = true, default-features = false }
flate2 = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

`rc-core` stays declared, unused by any `use` statement this blueprint writes — the fixed dependency-graph edge `12-workspace-structure.md`'s Dependency Graph diagram already draws (`nbt --> core`) and M0-B01 already scaffolded; this blueprint does not remove it, and a later blueprint (SNBT command-literal integration, or a schema helper needing `BlockPos`/`ChunkKey` directly) is expected to be its first real consumer. `simdnbt`'s two default features (`derive`, `serde`) are explicitly disabled — see Context.

### `crates/nbt/src/lib.rs`

```rust
//! `rc-nbt` — the engine's one boundary onto `simdnbt` 0.10.0 for vanilla-schema NBT
//! (WORLD-D11): typed read/write entry points (`io`), a byte-level/schema-level error
//! taxonomy (`error`), and a hand-written schema-conversion helper layer (`schema`)
//! future blueprints build vanilla `level.dat`/player/chunk/entity schemas on top of.
//! No vanilla schema is implemented in this crate.

mod io;
pub mod schema;
mod error;

/// Re-exported unmodified — this crate's read/write entry points return these types
/// directly rather than wrapping them a second time (WORLD-D11: "thin wrapper").
pub use simdnbt::{Mutf8Str, Mutf8String};

/// Zero-copy, lifetime-tied tree types — the default read path (see Context's
/// "Zero-copy read-path policy").
pub mod borrow {
    pub use simdnbt::borrow::{
        BaseNbt, Nbt, NbtCompound, NbtCompoundIter, NbtList, NbtTag,
    };
}

/// Heap-owned tree types — used for `level.dat`/player-data (always GZip, see
/// Context's "Compression stance") and anywhere a value must outlive its source buffer.
pub mod owned {
    pub use simdnbt::owned::{BaseNbt, Nbt, NbtCompound, NbtList, NbtTag};
}

pub use error::NbtError;
pub use io::{read_borrowed, read_borrowed_strict, read_gzip_owned, read_owned, write_gzip_owned, write_owned};
pub use schema::{FromNbtCompound, NbtCompoundExt, NbtPath, SchemaError, ToNbtCompound};
```

### `crates/nbt/src/error.rs`

```rust
/// Byte-level read/write failure (decode, decompress, this crate's own trailing-bytes
/// strictness check). Wraps `simdnbt::Error` verbatim (Context: that type already
/// implements `std::error::Error`) rather than re-deriving its four variants.
#[derive(Debug, thiserror::Error)]
pub enum NbtError {
    #[error("malformed NBT: {0}")]
    Decode(#[from] simdnbt::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// `read_borrowed_strict`/`read_owned_strict`-only: bytes remained after the root
    /// document ended. Never produced by the non-`_strict` read functions.
    #[error("trailing bytes after root NBT document: consumed {consumed} of {total} bytes")]
    TrailingBytes { consumed: usize, total: usize },
}
```

### `crates/nbt/src/io.rs`

```rust
use crate::{NbtError, borrow, owned};

/// Zero-copy read (WORLD-D11's hot path) of an already-decompressed byte slice
/// containing one root NBT document. `Ok(borrow::Nbt::None)` means "valid, empty
/// document" (e.g. a not-yet-written chunk slot) — not an error.
pub fn read_borrowed(data: &[u8]) -> Result<borrow::Nbt<'_>, NbtError>;

/// As `read_borrowed`, additionally erroring with `NbtError::TrailingBytes` if `data`
/// contains any byte after the root document ends — a stricter, rc-nbt-specific
/// corruption check `simdnbt` itself does not perform (this crate's own choice, not a
/// vanilla behavior).
pub fn read_borrowed_strict(data: &[u8]) -> Result<borrow::Nbt<'_>, NbtError>;

/// Owned read of an already-decompressed byte slice — used where the decoded value
/// must outlive `data`, or where `data` was itself just produced by decompression
/// (see `read_gzip_owned`).
pub fn read_owned(data: &[u8]) -> Result<owned::Nbt, NbtError>;

/// GZip-decompresses `data` (via `flate2`), then `read_owned`s the result. The only
/// entry point this crate offers for `level.dat`/player-data's fixed GZip framing
/// (WORLD-D15) — see Context's "Compression stance" for why chunk-payload
/// compression (Zlib/LZ4/none, WORLD-D13) has no equivalent wrapper here.
pub fn read_gzip_owned(data: &[u8]) -> Result<owned::Nbt, NbtError>;

/// Serializes `nbt` (named root, per `owned::BaseNbt::write`) to a fresh `Vec<u8>`.
pub fn write_owned(nbt: &owned::BaseNbt) -> Vec<u8>;

/// As `write_owned`, then GZip-compresses the result (`flate2`, default compression
/// level) — the write-side counterpart to `read_gzip_owned`.
pub fn write_gzip_owned(nbt: &owned::BaseNbt) -> Result<Vec<u8>, NbtError>;
```

### `crates/nbt/src/schema.rs`

```rust
use crate::{Mutf8Str, borrow, owned};

/// Locates a field inside a decoded NBT tree, for `SchemaError`'s diagnostics only
/// (Context: an original design inspired by, but not copied from, vanilla's own
/// documented `ValueInput`/`ValueOutput` problem-path concept). Cheap to clone.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NbtPath(Vec<PathSegment>);

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSegment {
    Field(&'static str),
    Index(usize),
}

impl NbtPath {
    /// The empty path — `Display`s as `<root>`.
    pub fn root() -> Self;
    /// A new path with one more named-field segment appended (`self` unchanged).
    pub fn field(&self, name: &'static str) -> Self;
    /// A new path with one more list-index segment appended (`self` unchanged).
    pub fn index(&self, i: usize) -> Self;
}

impl std::fmt::Display for NbtPath {
    /// e.g. `<root>.sections[3].block_states` — dot-joins `Field` segments, brackets
    /// `Index` segments onto the immediately preceding segment.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

/// One typed struct <-> NBT compound conversion failure, always path-qualified.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchemaError {
    #[error("{path}: missing required field `{field}`")]
    MissingField { path: NbtPath, field: &'static str },
    #[error("{path}: field `{field}` has the wrong tag type: expected {expected}, found tag id {actual_id}")]
    WrongType { path: NbtPath, field: &'static str, expected: &'static str, actual_id: u8 },
    #[error("{path}: field `{field}` has an invalid value: {reason}")]
    InvalidValue { path: NbtPath, field: &'static str, reason: String },
}

/// The write direction: `Self` -> a fresh, owned NBT compound. Hand-written per
/// vanilla schema type (WORLD-D11) — never `#[derive(...)]`d.
pub trait ToNbtCompound {
    fn to_nbt_compound(&self) -> owned::NbtCompound;
}

/// The read direction: a borrowed, zero-copy compound -> `Self`. Hand-written per
/// vanilla schema type, same rule as `ToNbtCompound`.
pub trait FromNbtCompound: Sized {
    fn from_nbt_compound<'a, 'tape>(
        compound: &borrow::NbtCompound<'a, 'tape>,
        path: &NbtPath,
    ) -> Result<Self, SchemaError>;
}

/// `SchemaError`-producing "require this field or fail with a precise path" accessors
/// over `borrow::NbtCompound`, layered on top of (never replacing) its existing
/// `Option`-returning accessors — use those directly for genuinely optional fields.
/// One `require_*` per NBT tag type, mirroring `NbtTag`'s own accessor completeness.
pub trait NbtCompoundExt<'a, 'tape> {
    fn require_byte(&self, path: &NbtPath, field: &'static str) -> Result<i8, SchemaError>;
    fn require_short(&self, path: &NbtPath, field: &'static str) -> Result<i16, SchemaError>;
    fn require_int(&self, path: &NbtPath, field: &'static str) -> Result<i32, SchemaError>;
    fn require_long(&self, path: &NbtPath, field: &'static str) -> Result<i64, SchemaError>;
    fn require_float(&self, path: &NbtPath, field: &'static str) -> Result<f32, SchemaError>;
    fn require_double(&self, path: &NbtPath, field: &'static str) -> Result<f64, SchemaError>;
    fn require_byte_array(&self, path: &NbtPath, field: &'static str) -> Result<&'a [u8], SchemaError>;
    fn require_string(&self, path: &NbtPath, field: &'static str) -> Result<&'a Mutf8Str, SchemaError>;
    fn require_list(&self, path: &NbtPath, field: &'static str) -> Result<borrow::NbtList<'a, 'tape>, SchemaError>;
    fn require_compound(&self, path: &NbtPath, field: &'static str) -> Result<borrow::NbtCompound<'a, 'tape>, SchemaError>;
    fn require_int_array(&self, path: &NbtPath, field: &'static str) -> Result<Vec<i32>, SchemaError>;
    fn require_long_array(&self, path: &NbtPath, field: &'static str) -> Result<Vec<i64>, SchemaError>;
}

impl<'a, 'tape> NbtCompoundExt<'a, 'tape> for borrow::NbtCompound<'a, 'tape> {
    // Implementation note (not part of the committed public surface): every method
    // follows the identical two-step shape — `self.get(field)` -> `MissingField` on
    // `None`; on `Some(tag)`, the matching `NbtTag` accessor (`tag.int()`, `tag.string()`,
    // ...) -> `WrongType { actual_id: tag.id(), .. }` on its own `None`. A private
    // `macro_rules!` generating all twelve bodies from this one shape is the expected,
    // but not mandated, implementation strategy (Implementation steps, below).
}
```

### `crates/nbt/fuzz/Cargo.toml` (new — deliberately **not** a root-workspace member)

```toml
[package]
name = "rc-nbt-fuzz"
version = "0.0.0"
edition = "2024"
publish = false

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4.13"
arbitrary = { version = "1.4.2", features = ["derive"] }
rc-nbt = { path = ".." }

[workspace]
# Empty on purpose — excludes this crate from the root workspace's member/dependency
# resolution, the standard `cargo fuzz init` shape (TEST-D25). Mirrors, for a
# non-member crate, WS-D7's own "pin directly, do not touch [workspace.dependencies]"
# treatment of xtask's clap/xshell.

[[bin]]
name = "nbt_decode"
path = "fuzz_targets/nbt_decode.rs"
test = false
doc = false
bench = false
```

### `crates/nbt/fuzz/fuzz_targets/nbt_decode.rs` (new)

```rust
//! TEST-D26 item (2): "NBT decode — `simdnbt`'s zero-copy borrowed-buffer decode
//! entry point... must never panic, regardless of input." Raw-byte input (not a
//! derived `Arbitrary` struct) — the type under test *is* a byte buffer, per TEST-D26's
//! own "entry point = raw... bytes" framing for this exact fuzz-target class.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Must never panic and must never hang, for any input whatsoever. A successful
    // decode's further round-trip property (decode(encode(x)) == x) is exercised by
    // this crate's proptest suite instead (TEST-D27's own division of labor: raw-byte
    // fuzzing for "never panics on garbage," structured proptest for "round-trips on
    // valid values") — not duplicated here.
    let _ = rc_nbt::read_borrowed(data);
});
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/nbt/src/{error.rs, io.rs, schema.rs}` and `crates/nbt/src/lib.rs` with every function body from the Deliverables signatures replaced with `todo!()` (struct/enum field lists, derives, trait definitions, and doc comments stay exactly as specified — only executable bodies are stubbed), plus the `Cargo.toml` edit, plus the complete `crates/nbt/fuzz/` crate (its one fuzz target has no body to stub — it is not exercised by any Tier-1 test, per Constraints, so it is committed complete in the test changeset). The implementation changeset (Implementation steps, below) fills in real bodies only — it must not modify any file under `crates/nbt/tests/`, must not weaken any assertion, and must not change any type's field list, derive list, trait definition, or public function signature from what the test changeset already compiled against.

### `crates/nbt/tests/known_answer_vectors.rs`

Thirteen cases — one root-document byte vector per tag type plus the empty-compound case — each asserting **both** directions: (a) hand-constructing the value via `owned::NbtCompound::from_values`/`owned::BaseNbt::new` and calling `rc_nbt::write_owned` produces exactly the given bytes; (b) calling `rc_nbt::read_borrowed` (and separately `rc_nbt::read_owned`) on those exact bytes decodes back to a document whose single field matches the original value via the appropriate typed accessor. All names/values below are exact, hand-derivable from the binary format restated in Context — no case may be altered.

1. `byte_tag` — root compound, one entry `"b"` = `NbtTag::Byte(-1)`. Bytes: `[0x0A,0x00,0x00, 0x01,0x00,0x01,0x62,0xFF, 0x00]` (9 bytes).
2. `short_tag` — `"s"` = `NbtTag::Short(-12345)`. Bytes: `[0x0A,0x00,0x00, 0x02,0x00,0x01,0x73,0xCF,0xC7, 0x00]` (10 bytes).
3. `int_tag` — `"i"` = `NbtTag::Int(-1)`. Bytes: `[0x0A,0x00,0x00, 0x03,0x00,0x01,0x69,0xFF,0xFF,0xFF,0xFF, 0x00]` (12 bytes).
4. `long_tag` — `"l"` = `NbtTag::Long(1)`. Bytes: `[0x0A,0x00,0x00, 0x04,0x00,0x01,0x6C,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x01, 0x00]` (16 bytes).
5. `float_tag` — `"f"` = `NbtTag::Float(1.0)`. Bytes: `[0x0A,0x00,0x00, 0x05,0x00,0x01,0x66,0x3F,0x80,0x00,0x00, 0x00]` (12 bytes).
6. `double_tag` — `"d"` = `NbtTag::Double(1.0)`. Bytes: `[0x0A,0x00,0x00, 0x06,0x00,0x01,0x64,0x3F,0xF0,0x00,0x00,0x00,0x00,0x00,0x00, 0x00]` (16 bytes).
7. `byte_array_tag` — `"ba"` = `NbtTag::ByteArray(vec![1,2,3])`. Bytes: `[0x0A,0x00,0x00, 0x07,0x00,0x02,0x62,0x61,0x00,0x00,0x00,0x03,0x01,0x02,0x03, 0x00]` (16 bytes).
8. `string_tag` — `"st"` = `NbtTag::String(Mutf8String::from("hi"))`. Bytes: `[0x0A,0x00,0x00, 0x08,0x00,0x02,0x73,0x74,0x00,0x02,0x68,0x69, 0x00]` (13 bytes).
9. `list_tag` — `"li"` = a `NbtList::Byte(vec![7,8])`-shaped list tag, i.e. `NbtTag::List(NbtList::Byte(vec![7,8]))`. Bytes: `[0x0A,0x00,0x00, 0x09,0x00,0x02,0x6C,0x69,0x01,0x00,0x00,0x00,0x02,0x07,0x08, 0x00]` (16 bytes).
10. `compound_tag` — `"c"` = a nested compound containing one entry `"x"` = `NbtTag::Byte(9)`. Bytes: `[0x0A,0x00,0x00, 0x0A,0x00,0x01,0x63, 0x01,0x00,0x01,0x78,0x09, 0x00, 0x00]` (14 bytes).
11. `int_array_tag` — `"ia"` = `NbtTag::IntArray(vec![1,2])`. Bytes: `[0x0A,0x00,0x00, 0x0B,0x00,0x02,0x69,0x61,0x00,0x00,0x00,0x02,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x02, 0x00]` (21 bytes).
12. `long_array_tag` — `"la"` = `NbtTag::LongArray(vec![1])`. Bytes: `[0x0A,0x00,0x00, 0x0C,0x00,0x02,0x6C,0x61,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x01, 0x00]` (21 bytes).
13. `empty_compound` — root compound with zero entries. Bytes: `[0x0A,0x00,0x00, 0x00]` (4 bytes). Decoding must yield `owned::Nbt::Some(base)` with `base.len() == 0` (an empty compound is a valid, non-`None` document — distinct from `Nbt::None`, which only ever arises from a **zero-length** input buffer per `simdnbt`'s own `read` contract).

### `crates/nbt/tests/mutf8_edge_cases.rs`

1. `nul_encodes_as_overlong_two_byte_sequence` — `Mutf8Str::from_str("\u{0}").as_bytes() == &[0xC0, 0x80]` (not `&[0x00]`).
2. `supplementary_plane_encodes_as_surrogate_pair` — `Mutf8Str::from_str("\u{10000}").as_bytes() == &[0xED,0xA0,0x80,0xED,0xB0,0x80]` (6 bytes, not standard UTF-8's 4-byte `[0xF0,0x90,0x80,0x80]`); additionally `Mutf8Str::from_slice(&[0xED,0xA0,0x80,0xED,0xB0,0x80]).to_str().as_ref() == "\u{10000}"` (decodes back to the original code point).
3. `nul_string_round_trips_through_full_write_read_cycle` — a root compound with one entry `"n"` = `NbtTag::String(Mutf8String::from("\u{0}"))`; `write_owned` then `read_owned`; assert the decoded compound's `.string("n")` returns bytes `[0xC0, 0x80]` (`.as_bytes()`), proving the encode-time MUTF-8 rule survives this crate's own write/read wrapper, not just `simdnbt`'s raw encoder.
4. `malformed_string_bytes_round_trip_without_corruption` — construct `Mutf8String::from_vec(vec![0xFF, 0xFE])` (not valid MUTF-8 by construction) as a compound entry's value; `write_owned` then `read_owned`; assert the decoded string's `.as_bytes() == &[0xFF, 0xFE]` exactly (byte-for-byte preserved, not silently replaced or corrupted) — malformed already-on-disk data must round-trip losslessly through this layer even though it can never be losslessly *displayed*.
5. `malformed_string_to_str_never_panics` — on that same malformed value, `.to_str()` returns without panicking (assert `.to_str().is_empty()`, matching `simdnbt`'s own documented fallback — this test pins that documented behavior as a regression guard, not as a design choice this blueprint made).

### `crates/nbt/tests/malformed_input_rejection.rs`

Each case asserts `.is_err()` on `rc_nbt::read_borrowed` (never a panic, never a hang — every case must return within the test's ordinary timeout):

1. `truncated_immediately_after_root_tag_id` — input `&[0x0A]` (root tag id only, nothing else).
2. `invalid_root_tag_id` — input `&[0xFF, 0x00, 0x00, 0x00]` (`0xFF` is not a valid tag id).
3. `invalid_list_element_type_id` — a hand-built root compound whose one list entry's element-type byte is `0xFF` (everything else well-formed).
4. `truncated_byte_array_length_claim` — a hand-built root compound whose one `ByteArray` entry declares `count = i32::MAX` via its 4-byte length field but supplies zero trailing payload bytes.
5. `excessively_nested_compound_is_rejected_not_stack_overflowed` — programmatically build (via a loop, not by hand) a root document nesting an empty compound inside itself 100,000 levels deep; assert `read_borrowed` returns `Err(_)` (the specific `simdnbt::Error::MaxDepthExceeded` variant is expected but this test asserts only `is_err()`, since the exact configured depth cap is `simdnbt`'s own internal, unconfirmed-by-this-blueprint constant).

### `crates/nbt/tests/roundtrip_proptest.rs`

A hand-written `proptest::strategy::Strategy` generating arbitrary `owned::NbtCompound` trees, bounded to max depth 4 and max 6 entries per compound (this blueprint's own testing-scope choice, unrelated to `simdnbt`'s 512-level parse-time depth cap) — covering all twelve tag types including nested `Compound` (up to the depth bound) and every `List` element type. Two properties:

1. `compound_round_trips_through_owned_write_then_owned_read` — for arbitrary generated `owned::NbtCompound` `c`: `let root = owned::BaseNbt::new("", c.clone()); let bytes = rc_nbt::write_owned(&root); let decoded = rc_nbt::read_owned(&bytes).unwrap();` — match `decoded` as `owned::Nbt::Some(base)` (panic the property on `None`) and assert `base == root`.
2. `compound_round_trips_through_owned_write_then_borrowed_read` — as above, but via `rc_nbt::read_borrowed(&bytes)`, matching `borrow::Nbt::Some(base)` and asserting `base.to_owned() == root` (using `borrow::BaseNbt::to_owned`'s direct owned-conversion, confirmed against the live docs.rs page in Context).

### `crates/nbt/tests/schema_helpers.rs`

A test-local (not a deliverable) example type:

```rust
struct ExamplePoint { x: i32, y: i32, label: String }

impl rc_nbt::ToNbtCompound for ExamplePoint {
    fn to_nbt_compound(&self) -> rc_nbt::owned::NbtCompound {
        rc_nbt::owned::NbtCompound::from_values(vec![
            ("x".into(), rc_nbt::owned::NbtTag::Int(self.x)),
            ("y".into(), rc_nbt::owned::NbtTag::Int(self.y)),
            ("label".into(), rc_nbt::owned::NbtTag::String(self.label.as_str().into())),
        ])
    }
}

impl rc_nbt::FromNbtCompound for ExamplePoint {
    fn from_nbt_compound<'a, 'tape>(
        compound: &rc_nbt::borrow::NbtCompound<'a, 'tape>,
        path: &rc_nbt::NbtPath,
    ) -> Result<Self, rc_nbt::SchemaError> {
        use rc_nbt::NbtCompoundExt;
        Ok(ExamplePoint {
            x: compound.require_int(path, "x")?,
            y: compound.require_int(path, "y")?,
            label: compound.require_string(path, "label")?.to_str().into_owned(),
        })
    }
}
```

1. `round_trips_through_to_and_from_nbt_compound` — `ExamplePoint { x: 3, y: -5, label: "hi".into() }`; `to_nbt_compound()`, wrap in `owned::BaseNbt::new("", ..)`, `write_owned`, `read_borrowed`, `.as_compound()`, `ExamplePoint::from_nbt_compound(&compound, &NbtPath::root())`; assert the result equals the original field-by-field.
2. `missing_field_reports_exact_path_and_field_name` — encode a compound missing `"y"` entirely (only `"x"` and `"label"` present); `from_nbt_compound` must return `Err(SchemaError::MissingField { field: "y", .. })` — assert on the `field` value specifically, not just `is_err()`.
3. `wrong_type_reports_expected_and_actual_tag_id` — encode `"x"` as `NbtTag::String(...)` instead of `NbtTag::Int(...)`; assert `Err(SchemaError::WrongType { field: "x", expected: "Int", actual_id: 8, .. })` (`8` = String's tag id, per the Tag-ID table).

### `crates/nbt/tests/oracle_compatibility.rs`

The one strategy test from Context's "Compatibility-check strategy" subsection, `#[ignore]`d pending `rc-test-harness` (TEST-D7):

```rust
#[ignore = "requires a vanilla-produced level.dat sample from rc-test-harness (TEST-D7), not yet implemented — see issue #<TRACKING_ISSUE, opened by the implementer at commit time>"]
#[test]
fn decodes_real_vanilla_level_dat_without_error() {
    let path = std::path::Path::new("oracle/26.2/harness/samples/level.dat");
    let bytes = std::fs::read(path).expect("sample not present — see #[ignore] reason");
    let nbt = rc_nbt::read_gzip_owned(&bytes).expect("must decode a real vanilla level.dat cleanly");
    let root = match nbt {
        rc_nbt::owned::Nbt::Some(base) => base,
        rc_nbt::owned::Nbt::None => panic!("level.dat must not be an empty document"),
    };
    let data = root.compound("Data").expect("level.dat root must contain a Data compound");
    assert!(data.contains("DataVersion"), "Data compound must contain DataVersion");
    assert_eq!(data.int("DataVersion"), Some(4903), "sample must be the pinned DataVersion (WORLD-D16)");
}
```

### `crates/nbt/tests/fuzz_regressions/.gitkeep`

Empty, reserved (TEST-D28: every crash `cargo-fuzz` ever finds against `nbt_decode` is minimized and committed here as a permanent regression test). Zero entries at M2 — nothing has been fuzzed yet.

## Implementation steps

1. **`Cargo.toml`.** Add the three dependency lines (`simdnbt`, `flate2`, `thiserror`) and the `proptest` dev-dependency exactly as Deliverables specifies. Observable: `cargo metadata` still resolves; `cargo build -p rc-nbt` fails only on the still-`todo!()`d source files, not on manifest issues.
2. **`error.rs`.** Fill in `NbtError`'s derive-driven bodies (all three variants are `thiserror`-generated — no hand-written `fmt`/`From` bodies beyond the derive itself). Observable: compiles standalone.
3. **`io.rs`.** Implement each function per its doc comment: `read_borrowed`/`read_owned` wrap `std::io::Cursor::new(data)` + the matching `simdnbt::{borrow,owned}::read`, propagating errors via `?` (the `#[from]` impl handles the conversion). `read_borrowed_strict` additionally checks `cursor.position() as usize == data.len()` after the read succeeds, returning `NbtError::TrailingBytes` otherwise. `read_gzip_owned` decompresses via `flate2::read::GzDecoder` into a fresh `Vec<u8>` (`std::io::Read::read_to_end`), then calls `read_owned` on it. `write_owned` allocates a `Vec::new()`, calls `nbt.write(&mut buf)`, returns it. `write_gzip_owned` calls `write_owned`, pipes the result through `flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default())` + `write_all` + `finish()`. Observable: `cargo build -p rc-nbt` succeeds except for `schema.rs`'s remaining `todo!()`s.
4. **`schema.rs`.** `NbtPath`: `Vec<PathSegment>`-backed, `root()` is `Self(vec![])`, `field`/`index` each clone `self.0`, push one segment, wrap in `Self`. `Display` joins `Field` segments with `.` and appends `Index` segments as `[i]` onto the running string, starting from the literal `"<root>"` when the segment list is empty. `SchemaError`: pure `thiserror` derive, no hand-written bodies beyond the attribute. `NbtCompoundExt` impl: write the private `macro_rules!` (or twelve individual near-identical bodies, implementer's choice — both satisfy the committed trait signatures identically) following the exact two-step shape Deliverables' implementation-note doc comment specifies. Observable: `cargo build -p rc-nbt` succeeds with zero `todo!()` remaining.
5. **`crates/nbt/fuzz/`.** Already complete from the test changeset (Acceptance tests' changeset-boundary note) — no implementation-changeset edit needed here at all.
6. **Run the full acceptance suite.** `cargo nextest run -p rc-nbt` — every test in every file under Acceptance tests passes, **except** `oracle_compatibility.rs`'s one `#[ignore]`d test (expected to be skipped, not run).
7. **Doctests.** `cargo test --doc -p rc-nbt` passes (no runnable doc examples are required by this blueprint's Deliverables; guards against accidentally introducing a broken one).
8. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `cargo run -p xtask -- lint`, `cargo run -p xtask -- lint-deps` all still exit 0.
9. **One-time local fuzz sanity check (not a CI gate — see Constraints).** With a nightly toolchain and `cargo-fuzz` 0.13.2 installed, run `cargo +nightly fuzz run nbt_decode -- -max_total_time=30` from `crates/nbt/fuzz/` once, confirming it builds and executes without an immediate crash. Not required to pass any particular case count — this is a build/link sanity check, not a corpus run.
10. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs of the existing `.github/workflows/ci.yml` (unmodified by this blueprint) go green on a clean checkout — the authoritative done-signal (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/nbt/tests/` and the complete `crates/nbt/fuzz/` crate are committed first, alongside `todo!()`-stubbed `crates/nbt/src/{error.rs,io.rs,schema.rs,lib.rs}` (field lists, derives, and trait definitions already final) and the `Cargo.toml` edit. The implementation changeset (steps 1–10 above) fills in real bodies only — it must not edit any test file, must not add/remove/rename any test case listed in Acceptance tests, and must not weaken any assertion (in particular, every hand-derived byte vector in `known_answer_vectors.rs` and every MUTF-8 byte sequence in `mutf8_edge_cases.rs` must survive unchanged — these are hand-verifiable against Context's restated binary format, not values to "fix" if a first implementation attempt disagrees with them).

(b) **No new external dependencies beyond the pinned set, with the fuzz crate's own cited exception.** `rc-nbt` itself uses only `simdnbt`, `flate2`, `thiserror` (all already in `[workspace.dependencies]`) plus the already-pinned dev-dependency `proptest` — do not add `serde`, `bincode`, `anyhow`, or enable `simdnbt`'s `derive`/`serde` features. `crates/nbt/fuzz/` is the one place this blueprint adds dependencies not present in the root workspace table (`libfuzzer-sys`, `arbitrary`) — exactly TEST-D25's pinned versions, cited directly in that crate's own `Cargo.toml` per the standard cargo-fuzz non-member-crate shape (Context), never added to the root `[workspace.dependencies]` table.

(c) **No Mojang or third-party reimplementation code.** Every byte layout and encoding rule in this blueprint is restated from `docs/research/mc-26.2/04-persistence-nbt.md` (itself sourced under the ASSET-D18(f) reference-source policy) in this blueprint's own words; `SchemaError`'s `NbtPath` design is an original adaptation of a documented *concept* only (Context), no vanilla source consulted or copied (ASSET-D18/D19/D30).

(d) **No `unsafe` code.** Every type and function in this blueprint's Deliverables is implementable in 100% safe Rust — `simdnbt` itself may use `unsafe` internally (SIMD endianness-swapping, per its own crate docs), but nothing this blueprint writes does.

(e) **Scope boundary — do not implement beyond this blueprint's own crate.** This blueprint does not implement: SNBT (see Context — module path reserved, nothing else); any concrete vanilla schema (`level.dat`, player data, chunk NBT, entities, block entities, POI records — B04/B06/`rc-chunk-storage`'s jobs); the Anvil `.mca` region-file container (WORLD-D12, M2-B01 or a sibling); chunk-payload Zlib/LZ4/none compression selection (WORLD-D13, `rc-chunk-storage`'s job — this crate's GZip wrappers are for `level.dat`/player data only, per Context); `rc-test-harness` or any part of the oracle-compatibility test's actual execution (Context's own honest boundary statement); wiring `nbt_decode` into any nightly/Tier-2 CI job or into `xtask`'s verb surface (no `xtask fuzz run` exists yet — a future test-infrastructure blueprint's job, matching the same "verb doesn't exist yet, don't invent it here" discipline M0-B01/M0-B08 already established for `bench`/`parity-check`). Do not add placeholder implementations of any of these as a shortcut.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-nbt --all-features
cargo nextest run -p rc-nbt
cargo test --doc -p rc-nbt
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
```

Expected: every command exits 0. `cargo nextest run -p rc-nbt` reports 13 (`known_answer_vectors.rs`) + 5 (`mutf8_edge_cases.rs`) + 5 (`malformed_input_rejection.rs`) + 2 (`roundtrip_proptest.rs`, each a property test counted as one case regardless of internal generated-input count) + 3 (`schema_helpers.rs`) = 28 run cases, plus exactly 1 skipped case (`oracle_compatibility.rs`'s `#[ignore]`d test) — never a silent pass, always reported as skipped. `crates/nbt/fuzz/`'s own build/run (Implementation step 9) is a one-time local sanity check, not part of this list — it requires a nightly toolchain this project's own `rust-toolchain.toml` does not pin, and is not gated by this blueprint's CI. CI (`.github/workflows/ci.yml`, unmodified) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
