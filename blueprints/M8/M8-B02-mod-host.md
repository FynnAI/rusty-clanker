# M8-B02 — `rc-mod-host`: Discovery, Loading, Lifecycle & Crash Isolation

| Field | Content |
|---|---|
| ID | M8-B02 |
| Milestone | M8 — Mod API Alpha |
| Prerequisites | M8-B01 (`rc-mod-api`'s complete public surface — this blueprint builds against it exactly, restated field-for-field below where load-bearing; the one place this blueprint edits `rc-mod-api` is a purely **additive** completion of the `RegistryBuildContext`/`ClientRegistryBuildContext`/marker-`*Context` constructors M8-B01's own Implementation step 10 explicitly deferred to "`rc-mod-host`, a later M8 blueprint" — no already-declared public signature changes shape); M6-B07 (`rusty-clanker-server`'s composition root — read as consulted context only: as of M6-B07, `crates/server/src/main.rs`/`run_embedded`'s startup sequence (§C) never mentions `rc-mod-host` anywhere, confirming that wiring a `ServerModHost` into the real startup sequence, `RcExecutor`'s conflict graph, and `BlockBehaviorRegistry` is genuinely still-unbuilt future work — not something this blueprint can lean on or must avoid breaking, since nothing there touches mods yet). M0-B05 (`rc-scheduler`'s `RcExecutorBuilder`/conflict-graph — consulted only to establish that `rc-mod-host` is *not* the crate that calls it; restated in Context, "Where this blueprint's boundary actually falls"). M3-B01 (`rc-mechanics`'s `BlockBehaviorRegistry`/`BlockBehavior`/`UpdateContext` — consulted only to establish the same boundary point for block-behavior registration; this blueprint never depends on `rc-mechanics`). |
| Implements | MOD-D2 (WASM-tier deferral, stated and owned honestly — restated, not implemented); MOD-D4 (`.rcmod` zip container, manifest-driven per-side/per-triple entrypoint resolution — the concrete discovery/extraction mechanism); MOD-D5 (per-side load selection, automatic and exclusive — the server process never opens a client entry and vice versa); MOD-D9 (honesty-based native-tier trust boundary — restated as this crate's own binding non-enforcement scope); MOD-D21/D22 (the ABI version handshake's actual execution, consuming M8-B01's `ModAbiVersion`/`ABI_HANDSHAKE_SYMBOL`); MOD-D26 (native-tier SHA-256 content-hash allowlist, hand-rolled per M0-B08's own established precedent); MOD-D28 (no native-tier hot reload — restated as this crate's binding unload stance); MOD-D31 (dependency-order topological resolution, hard per-mod load failure, cascading); MOD-D32 (native-tier `catch_unwind`-at-the-FFI-boundary crash isolation, auto-disable-on-panic, `"halt"` escape hatch); PERF-D46 (workspace-wide `panic = "unwind"`, restated as the correctness precondition this whole blueprint depends on and verifies rather than assumes). |
| Crates touched | `rc-mod-host` (`crates/mod-host/`, new content — the crate itself already exists as M0-B01's empty scaffold placeholder); `rc-mod-api` (`crates/mod-api/`, additive-only: `src/entrypoint.rs` modified to complete four context types M8-B01 itself left as `unimplemented!()`-stubbed opaque placeholders, `src/lib.rs` modified to re-export the two new types this completion introduces — no other file in `rc-mod-api` is touched, and no existing public signature named in M8-B01's Deliverables changes shape). |
| Estimated scope | L — a deliberate exception to the ~800-line guideline, the same class of exception `00-blueprint-spec.md` already grants M8-B01/M6-B07/M5-B02-adjacent blueprints: this is the crate that turns MOD-D1–D32's on-paper security model into an actually-loadable, actually-isolated dylib boundary, and splitting discovery from loading from isolation would scatter one coherent trust boundary's reasoning across files an implementer would have to re-derive the seams between. |

## Goal & Done definition

Implement `rc-mod-host`'s native-tier mod lifecycle end to end: `mods/` directory discovery of `.rcmod` zip archives, manifest extraction and `rc-mod-api`'s `parse_manifest`/`validate_manifest` gate, MOD-D31's dependency-order topological resolution with per-mod (never whole-boot) hard failure and cascading, current-platform native-triple resolution and MOD-D4's exact `<mod_id>.{dll,so,dylib}` filename convention, MOD-D26's SHA-256 trust-allowlist gate, the ABI version handshake, a newly-defined entrypoint-factory symbol contract (the one genuine gap M8-B01 left unnamed — restated in Context), `libloading`-based dylib loading, and a `catch_unwind`-at-every-boundary dispatch layer around every `ServerModEntry`/`ClientModEntry` method with auto-disable-on-panic, an operator-configurable halt escape hatch, and an honestly-scoped double-fault analysis. Per-side loading is implemented symmetrically and *both* sides are exercised by this blueprint's own headless test suite — the server path against real discovery/loading, and the client path (M10's future consumer) proven to work identically today with no renderer in existence. Nothing in this blueprint touches `bevy_ecs`, `rc-scheduler`'s conflict graph, or `rc-mechanics`'s `BlockBehaviorRegistry` — that translation is out of `rc-mod-host`'s own dependency reach by construction (Context: "Where this blueprint's boundary actually falls") and is a later blueprint's job.

Done when:

- [ ] `cargo build -p rc-mod-host -p rc-mod-api --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mod-host -p rc-mod-api`.
- [ ] `cargo run -p xtask -- lint-deps` exits 0 against `rc-mod-host`'s binding, restated dependency set (Context: "Reconciling `rc-mod-host`'s dependency set") — `rc-mod-host` gains no dependency on `bevy_ecs`, `rc-scheduler`, `rc-mechanics`, `rc-chunk-storage`, or `rc-registries`, confirming this blueprint stayed inside its own stated boundary.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-mod-host -p rc-mod-api` exits 0.
- [ ] `crash_isolation.rs`'s tests prove, against a *real*, separately-compiled, `libloading`-loaded native dylib: a deliberately panicking hook is caught, the host process survives, the offending mod is marked disabled, and every other loaded mod (and the host itself) remains fully callable afterward.
- [ ] `double_fault_subprocess.rs` proves, via a dedicated child-process harness, that the one residual failure mode this blueprint cannot prevent (a panic inside a mod's own `Drop` glue running during another panic's unwind) aborts the process exactly as documented — not silently, not differently.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50). Fixture dylibs are compiled by the test suite itself at test-run time (Context: "Fixture dylibs — build mechanism") — no pre-built binary is ever committed (ASSET-D19-adjacent discipline: no opaque binary artifact enters the repository).

## Context (self-contained)

### Where this blueprint's boundary actually falls — restated from `12`'s own Crate Manifest text

`12-workspace-structure.md`'s Crate Manifest row for `rc-mod-host` reads: "Engine-side mod loader: `libloading`-based dylib loading, the ABI boundary, `catch_unwind`-based crash isolation, **generic hook-slot registration that `rc-scheduler` (server-side domain groups) and `rc-render` (client-side frame hooks) each pull from independently**." The dependency graph (`12`'s own Dependency Graph section) gives `rc-mod-host` exactly two edges — `modhost --> core`, `modhost --> modapi` — and gives `rc-scheduler` the edge `sched --> modhost` (the reverse direction), confirming `rc-scheduler` is a *consumer* of whatever generic surface this crate exposes, never the other way. No edge anywhere connects `rc-mod-host` to `bevy_ecs`, `rc-mechanics`, `rc-chunk-storage`, or `rc-registries`. This is not an oversight this blueprint quietly works around — it is the literal, binding shape WS-D3's four hard rules and `12`'s own crate-responsibility text already commit to, and this blueprint's own `xtask lint-deps` done-condition is the mechanical proof it stayed inside it.

The practical consequence: **this blueprint never translates a mod's registration request into a real `bevy_ecs::component::ComponentDescriptor`, a real `rc_chunk_storage::BlockStateId`, or a real `rc_mechanics::BlockBehaviorRegistry` entry.** M3-B01's `BlockBehaviorRegistry::register_range`/`register_one` (the seam a modded block's tick behavior ultimately reaches) and M0-B05's `RcExecutorBuilder::register_system` (the seam a mod's declared-access hook ultimately reaches) are both real, already-shipped APIs — but reaching them from a loaded mod requires `bevy_ecs` and, for block behaviors, `rc-mechanics`, neither of which this crate may depend on. That translation is `rc-scheduler`'s job, in a future blueprint this one's own Deliverables are deliberately shaped to make easy: every hook this blueprint dispatches returns plain, ABI-safe `rc-mod-api` types (never an opaque handle only this crate understands), and `RegistryBuildContext`'s completion below (Context: "The opaque `*Context` constructibility gap") is a self-contained *recording* structure a future `rc-scheduler` blueprint drains and translates, not a live callback into a `World` this crate has no way to construct in the first place.

### Reconciling `rc-mod-host`'s dependency set

M0-B01's scaffold gave `rc-mod-host`'s `Cargo.toml` exactly `{rc-core, rc-mod-api}` (the mechanical per-crate template applied to the edge-table row `rc-mod-host | rc-core, rc-mod-api`). This blueprint's own binding, expanded set, fixed by this blueprint and checked by its own `lint-deps` done-condition: `{rc-core, rc-mod-api (native-tier only, no default features), libloading, zip, stabby, parking_lot, thiserror, tracing}` — every one of these seven already sits in `12`'s `[workspace.dependencies]` table (`libloading = "0.9.0"` explicitly commented "rc-mod-host dylib loading"; `zip = "8.6.0"` currently commented "rc-assets resource-pack archive reading" — this blueprint is `zip`'s second consumer, not a new pin; `stabby`, `parking_lot`, `thiserror`, `tracing` all already pinned for other crates' use) — so Constraints (b)'s "no new external dependencies beyond the pinned set" holds exactly, this blueprint only *activates* seven already-approved pins for a crate that previously used none of them. `rc-mod-api` is pulled `default-features = false, features = ["native-tier"]` — this crate never needs `wasm-tier`'s `wit-bindgen`-generated guest bindings, since it never runs guest-side code.

`parking_lot::{Mutex, RwLock}` guards this crate's own per-mod bookkeeping (loaded-mod table, status flags) — squarely `01`'s ARCH-D23 "cold-path bookkeeping" category (a mod-hook dispatch, even once wired into a real tick loop by a future blueprint, fires at most a handful of times per region-tick, nothing like RC-WorkerPool's own steal/execute hot path) — restated here because ARCH-D23's own text scopes this exact class of use case. `parking_lot::Mutex` additionally never poisons on a panicking guard-holder (unlike `std::sync::Mutex`) — a genuinely load-bearing property for this specific crate, not an incidental one: Context's "Double-fault analysis" section below depends on it directly.

### MOD-D4's zip container and manifest extraction

`.rcmod` is a plain zip (MOD-D4, via the `zip` crate). Discovery scans `config.mods_dir` (a plain `std::path::Path`, supplied by the caller — this blueprint owns no server-config-file schema of its own) non-recursively for `*.rcmod` entries; a missing `mods_dir` is **not** an error (the common "no mods installed" case, matching M6-B07's own `--region-layout`-absent "dynamic bootstrap, nothing pre-exists" precedent for an absent-but-optional input) — only a genuine I/O failure reading an *existing* directory (permission denied, not-a-directory) is a boot-level `ModHostBootError`. For each archive, `manifest.toml`'s bytes are read out of the zip (`zip::ZipArchive::by_name("manifest.toml")`; a missing entry is a per-archive `ModLoadError::ManifestEntryMissing`, never a boot failure) and handed, as a UTF-8 string, to `rc_mod_api::parse_manifest` then `rc_mod_api::validate_manifest` — this crate never re-implements either check, per M8-B01's own two-phase contract ("Context: parse vs. validate"). A parse or validation failure is a per-mod hard load failure (MOD-D31's own "reported by name and reason, never a partial/best-effort load" policy, extended here from dependency failures to manifest failures for the identical reason).

### MOD-D31's dependency-order resolution — Kahn's algorithm again, restated for a second graph

Exactly the topological-layering technique M0-B05's `compute_waves` already established for the ARCH-D8 conflict graph, applied to a different graph here: nodes are successfully-parsed-and-validated mods, edges are `[dependencies]` entries (mod_id → range, MOD-D31), direction `dependency -> dependent`. This blueprint's own binding, restated resolution of what MOD-D31 leaves open, matching M8-B01's own explicit deferral verbatim ("Context: 'No `semver` crate — syntax validation only'"): dependency resolution at M8 alpha checks only that every declared `mod_id` names a mod that itself successfully parsed and validated, and that the graph is acyclic — it does **not** check whether that dependency's actual `[mod].version` satisfies the declaring mod's `DependencyRange` (real range-satisfaction logic remains M8-B01's own named, deferred future work; this blueprint does not silently "fix" that gap). A missing-dependency or a cycle is a hard load failure for exactly the mod(s) it touches, cascading forward through the dependency graph (a mod depending on a failed mod is *itself* failed, transitively) — computed and reported all at once (every failure in the whole discovered set, not fail-fast on the first), mirroring `validate_manifest`'s own collect-then-report discipline. Cycle detection reuses Kahn's algorithm's own natural byproduct: any node still unprocessed once every in-degree-0 node has been drained is part of a cycle.

### Platform-triple resolution and MOD-D4's exact filename convention

`[entrypoints.native."<triple>"]` (MOD-D4's schema) is keyed by Rust's own target-triple strings (e.g. `x86_64-pc-windows-msvc`). This crate's own compile-time knowledge of *its own* running triple is obtained the standard way a build-script-free crate cannot get from `cfg!` alone reliably across every triple variant: a `build.rs` reads Cargo's own `TARGET` environment variable (set for every build script invocation, and — critically — correct even under cross-compilation, unlike a hand-matched `cfg(target_os = ..., target_arch = ...)` table that would need updating for every new triple) and forwards it via `cargo:rustc-env` into a compile-time constant:

```rust
// crates/mod-host/build.rs
fn main() {
    let target = std::env::var("TARGET").expect("Cargo always sets TARGET for a build script invocation");
    println!("cargo:rustc-env=RC_MOD_HOST_TARGET_TRIPLE={target}");
}
```

A mod with no `[entrypoints.native.<CURRENT_TARGET_TRIPLE>]` table at all, or one present but with `server = false` (server-side loading) / `client = false` (client-side loading), is **silently a no-op on this platform for that side** — never a hard load failure — directly extending MOD-D5's own "a mod with only a `client` entrypoint is silently a no-op on a server" precedent from *which side* to *which platform*, the same shape of gap MOD-D5 already names but does not itself resolve for the multi-triple case.

**MOD-D4's own literal naming, restated exactly — deliberately *not* `libloading::library_filename`'s convention.** MOD-D4's schema comment states the on-disk shape plainly: `native/<target-triple>/<mod_id>.{dll,so,dylib}` — no `lib` prefix on the Unix variants. `libloading` 0.9.0 ships a `library_filename(name) -> OsString` helper that would produce `libmymod.so`/`mymod.dll` (verified, `docs.rs/libloading/0.9.0`, this blueprint's own research pass) — a **mismatch** with MOD-D4's own stated convention on every Unix-like target. This blueprint's binding resolution: never call `library_filename`; construct the expected in-archive path directly per MOD-D4's literal pattern:

```rust
#[cfg(target_os = "windows")]
pub fn native_binary_filename(mod_id: &rc_mod_api::ModId) -> String { format!("{}.dll", mod_id.as_str()) }
#[cfg(target_os = "macos")]
pub fn native_binary_filename(mod_id: &rc_mod_api::ModId) -> String { format!("{}.dylib", mod_id.as_str()) }
#[cfg(all(unix, not(target_os = "macos")))]
pub fn native_binary_filename(mod_id: &rc_mod_api::ModId) -> String { format!("{}.so", mod_id.as_str()) }
```

The zip-internal path looked up is therefore `native/{CURRENT_TARGET_TRIPLE}/{native_binary_filename(mod_id)}`. Because `libloading::Library::new` needs a real filesystem path (it cannot load from an in-memory byte buffer), this blueprint extracts that one zip entry's bytes to a per-process cache directory (`{mods_dir}/.rc-mod-cache/{mod_id}-{sha256_prefix}.{ext}`, created once at discovery time) before ever calling `Library::new` — the cache key includes a hash prefix specifically so a rebuilt/updated mod binary (different bytes, same `mod_id`) never collides with a stale cached copy from a previous run.

### MOD-D26's SHA-256 trust allowlist — hand-rolled, no crate, matching M0-B08's own established precedent

MOD-D26 requires SHA-**256** (not SHA-1) hashing the extracted native binary's bytes against operator-supplied `[mods.native.trusted]` entries (`hash -> mod id`, supplied to this crate as a pre-parsed `Vec<NativeTrustEntry>` — this crate owns no server-config-file schema, exactly as it owns no `mods_dir`-discovery config schema beyond the plain path). **No SHA-256 crate is pinned anywhere in `12`'s `[workspace.dependencies]` table** — `sha1 = "0.11.0"` (NET-D6) only computes SHA-1, and `md-5` only computes MD5. `M0-B08-verification-wiring.md` (already committed) hit this identical gap for TEST-D47's fixture-integrity manifest and resolved it by hand-rolling SHA-256 in pure safe Rust rather than adding a new pinned dependency ("no SHA-256 crate is pinned anywhere in the workspace... Implement SHA-256 by hand"). This blueprint reuses that same resolution, independently, for MOD-D26's own SHA-256 requirement: `crates/mod-host/src/sha256.rs` implements the published FIPS 180-4 SHA-256 algorithm in safe Rust (no `unsafe`, matching M0-B08's own Constraints (f) precedent for its hand-rolled hash), producing a lowercase-hex `String` a caller compares against a `NativeTrustEntry.sha256_hex` value case-insensitively. A native binary whose hash matches no trusted entry is a hard load failure (`ModLoadError::UntrustedNativeBinary`), never a silent fallback — MOD-D26's own "a hash mismatch... refuses to load with a clear error, never a silent fallback" restated exactly. `xtask`'s own hand-rolled SHA-256 (M0-B08, inside the `xtask` dev-tooling binary) is not reused directly — `xtask` is dev-tooling-only and cannot be a dependency of any production crate (WS-D1/D8's own binding "never shipped" framing for `xtask`), so this blueprint's copy is necessarily this crate's own, independent implementation of the identical, publicly-standardized algorithm — not a second design, the same published spec, restated where it is actually needed.

### The ABI handshake — consuming M8-B01's types exactly

`rc_mod_api::abi::{ModAbiVersion, MOD_API_VERSION, ABI_HANDSHAKE_SYMBOL, AbiHandshakeFn}` are used exactly as M8-B01 fixed them, restated: after `unsafe { libloading::Library::new(cached_path) }` succeeds, this blueprint looks up `ABI_HANDSHAKE_SYMBOL` (`"rc_mod_abi_handshake"`) via `unsafe { library.get::<AbiHandshakeFn>(ABI_HANDSHAKE_SYMBOL.as_bytes()) }` — a missing or wrong-signature symbol surfaces as `libloading::Error` (verified variant set, `docs.rs/libloading/0.9.0`: `DlSym`/`DlSymUnknown` on Unix, `GetProcAddress`/`GetProcAddressUnknown` on Windows, both already carrying the OS-level diagnostic via their own `Display` impl), wrapped by this crate's `ModLoadError::HandshakeSymbolMissing` (`#[from]`-derived, `thiserror`, so the OS-level detail is never lost). If the symbol resolves, this blueprint calls it (`unsafe`, `// SAFETY:` citing the ABI contract MOD-D3/D21 fix) and checks `mod_version.is_compatible_with(rc_mod_api::MOD_API_VERSION)` — `false` is `ModLoadError::AbiIncompatible { mod_version, engine_version: MOD_API_VERSION }`, a hard per-mod load failure, before any other symbol is ever looked up (MOD-D3's own binding ordering, restated: "before any other symbol is looked up").

### The entrypoint-factory symbol contract — the one genuine gap M8-B01 left unnamed, resolved here

M8-B01 fixes `ServerModEntry`/`ClientModEntry` as traits and fixes `[entrypoints].server`/`.client`'s manifest field as "the exported native symbol name" — but never states what that symbol's own function *signature* must be, since constructing a value of a `#[stabby::stabby]` trait object across an FFI boundary is exactly the kind of dylib-loading mechanism M8-B01 itself named as out of its own scope ("Constraints (f): this blueprint does not implement... the `libloading`/`wasmtime` embedding itself"). This blueprint fixes it, restating `stabby`'s own `Box<dyn Traits + 'a>` → `dynptr!(Box<dyn Traits + 'a>)` mapping (verified, `docs.rs/stabby/72.1.16`, this blueprint's own research pass — confirming M8-B01's identical usage in `RegistryBuildContext::register_block_behavior`'s parameter type is exactly this same mapping, applied to a return position here instead of a parameter):

```rust
/// The exact exported-symbol contract every `.rcmod`'s `[entrypoints].server` string
/// must name (native tier). Called exactly once per mod, immediately after a
/// successful ABI handshake, to obtain the mod's one, process-lifetime
/// `ServerModEntry` instance. `#[stabby::export]`-annotated on the mod-author side
/// (MOD-D3's own binding convention for a stable-ABI exported function).
pub type ServerEntryFactoryFn =
    unsafe extern "C" fn() -> stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ServerModEntry>);

/// As `ServerEntryFactoryFn`, for `[entrypoints].client`.
pub type ClientEntryFactoryFn =
    unsafe extern "C" fn() -> stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ClientModEntry>);
```

Symbol resolution follows the handshake exactly (`library.get::<ServerEntryFactoryFn>(manifest.entrypoints.server.as_bytes())`), and the call itself — unlike the handshake's own trusted, pre-any-mod-code-runs call — is wrapped in this blueprint's own `catch_unwind` guard (Context: "Crash isolation") from the very first call onward, since a factory function is already mod-authored code capable of panicking during construction (e.g. a `Default::default()`-style initializer that validates something and panics on failure) — MOD-D32's "wraps every native-mod hook invocation" is read literally here to include the very first one. `[entrypoints].shared` is never resolved as a runtime symbol at all — MOD-D4's own text fixes `shared/` as "not itself a runtime artifact," already compiled into the loaded dylib by the mod author's own build; a mod wanting `SharedModInit`'s hook composes and calls it from inside its own `ServerModEntry`/`ClientModEntry` implementation (this blueprint's own resolution of an otherwise-unspecified wiring point — `rc-mod-host` never constructs a `SharedInitContext` or calls `SharedModInit::on_shared_init` itself, since no manifest field names a runtime symbol for it to resolve).

**Moderate-confidence flag, re-verify at implementation time (elevated — see the next subsection for why).** `dynptr!`'s exact construction syntax on the mod-author side (how a real `Box<T: ServerModEntry>` becomes a `dynptr!(Box<dyn ServerModEntry>)` value — an explicit `From`/`Into` conversion or a named constructor) should be confirmed against the installed `stabby` 72.1.16 docs before this blueprint's fixture crates (Context: "Fixture dylibs") are written; it affects only fixture-crate authoring, not any signature above.

### The unwind-across-the-vtable-boundary question — elevated moderate-confidence flag, verified early by design

MOD-D32's entire premise is that a panic thrown *inside* a mod's own `ServerModEntry` method — reached, per M8-B01's own `#[stabby::stabby]` annotation on that trait, through an ABI-stable **vtable call** — is catchable by `std::panic::catch_unwind` on this crate's side. `#[stabby::stabby]`'s own documentation (verified, `docs.rs/stabby/72.1.16`) confirms it "makes [an annotated item] `extern "C"`" for standalone functions, but does **not** state, anywhere this blueprint's own research pass could confirm, whether the vtable *method calls* it generates for a trait are `extern "C"` (which would make an unwinding panic crossing that call **undefined behavior / an unconditional abort**, per Rust's own `extern "C"`-vs-`extern "C-unwind"` ABI distinction, stable since Rust 1.71) or `extern "C-unwind"` (which would make it exactly the sound, catchable boundary MOD-D32 assumes). PERF-D46's binding `panic = "unwind"` requirement is necessary but **not sufficient** on its own if the vtable dispatch itself uses the non-unwinding `extern "C"` convention — the panic strategy and the individual call's ABI-unwind-ability are two independent axes.

This is not glossed over: **Implementation step 1 of this blueprint is a minimal, standalone smoke test that proves or disproves this before a single other line of production code is written against the assumption.** A tiny fixture crate (Context: "Fixture dylibs") exporting one `#[stabby::stabby]`-vtable-dispatched trait method that unconditionally panics is compiled and loaded, and the smoke test asserts `catch_unwind` around the call returns `Err(..)` rather than the test process aborting. If it aborts instead, this is a hard blocker on this blueprint's entire design as specified by MOD-D32 and must be escalated (a governance-level question for `06-modding-api.md` itself — whether `stabby`'s trait-dispatch mechanism is fit for MOD-D32's stated purpose at all — not a detail this blueprint can silently route around by, say, wrapping every trait method in an additional hand-written `extern "C-unwind"` shim, since `rc-mod-api`'s trait definitions are already fixed by M8-B01 and such a shim would need to live on the mod-author's own side of a boundary this blueprint does not control). This blueprint's own Deliverables and every subsequent Implementation step assume the smoke test passes, exactly as MOD-D32 itself assumes; the smoke test exists specifically so that assumption is proven, not merely inherited.

### Crash isolation — the generic wrapper every hook dispatch reuses

One private helper, `call_guarded`, is the entire mechanism (`crates/mod-host/src/isolation.rs`):

```rust
fn call_guarded<R>(f: impl FnOnce() -> R + std::panic::UnwindSafe) -> Result<R, CaughtPanic>;
```

Every public dispatch method on `ServerModHost`/`ClientModHost` (Deliverables' `host.rs`) is a thin, one-line wrapper: acquire this mod's own per-entry `parking_lot::Mutex` (Context: "one instance, one mutex — why"), check `status` under a fast-path read lock first (skip entirely, no call attempted, if already `Disabled` — `HookOutcome::Skipped`), then call `call_guarded(|| unsafe { AssertUnwindSafe(...).0.method(...) })`, mapping `Ok(v)` to `HookOutcome::Ran(v)` and `Err(caught)` to: log (`tracing::error!`, structured fields: `mod_id`, `hook_name`, `caught.message`), mark this mod's status `Disabled { reason: DisableReason::Panic { hook: hook_name, message: caught.message }, at_tick: None }` under the write lock, increment a per-mod panic-count metric field (a plain `u32` counter this blueprint exposes via `ModStatus`, consumed by a future metrics blueprint — not itself wired to `06-modding-api.md`'s `MetricsRegistry`, M6-B02's crate, which `rc-mod-host` does not depend on), and return `HookOutcome::Panicked { message: caught.message }`. `AssertUnwindSafe` is required and used deliberately, not incidentally: `&mut dyn ServerModEntry` is not `UnwindSafe` by `std`'s own default rule (a mutable reference may have been left pointing at a torn invariant mid-panic) — asserting it anyway is the direct, honest expression of this blueprint's own "assumed poisoned after a panic" policy (next subsection), not a claim that the mod's state is actually fine.

**One instance, one mutex — why.** Every `ServerModEntry`/`ClientModEntry` method takes `&mut self` (M8-B01's own signatures) — a mod's own internal state has no concurrency story of its own unless the mod author adds interior mutability themselves. This blueprint therefore serializes every call into one loaded mod's entry behind one `parking_lot::Mutex<stabby::dynptr!(...)>` per mod, for every hook, unconditionally — a deliberate simplification appropriate for M8 alpha (MOD-D11 already mandates full sequentiality for any Stage-4/`BlockRedstone`-group hook regardless, and no other M8-alpha hook is latency-sensitive enough that cross-region parallel dispatch into the *same* mod instance is a real requirement yet); a future blueprint that needs true concurrent multi-region dispatch into one mod's hooks (if `06` or a future revision ever requires it) extends this mutex's granularity, not its existence.

### What "poisoned" means after a caught panic — restated plainly, per 06's own honesty discipline

A caught panic proves nothing about the panicking mod's own remaining internal state beyond "it did not finish the call it was making." This blueprint's binding policy, matching MOD-D9's own "honesty-based... exactly as badly as a bug in engine code could" framing extended to the post-panic case: **the panicking mod's own boxed `ServerModEntry`/`ClientModEntry` value is never called again** (the `Disabled` status check, enforced before every subsequent dispatch, is the entire mechanism — there is no attempt to inspect, repair, or partially trust its remaining state). Every **other** loaded mod's own state is provably untouched — each lives behind its own independent `Box`/`Mutex`, sharing no memory with the panicking mod's instance — and the host's own bookkeeping (the loaded-mod table, every other mod's status) is likewise untouched, since `call_guarded`'s own catch-and-mark sequence runs entirely in this crate's own simple, non-panicking safe Rust (a `parking_lot::Mutex` write-lock plus an enum assignment — no `unwrap`, no fallible arithmetic, nothing that could itself panic) — restated explicitly because it is exactly the property Context's "Double-fault analysis" section next depends on holding.

### Disable-on-panic policy — registered content's fate, restated from 06

**Mirroring MOD-D25's WASM-tier default exactly (MOD-D32's own text): `mod_fault_policy` is `Disable` by default, `Halt` an operator-configurable escape hatch.** This crate never calls `std::process::exit`/`abort` itself under either policy — under `Disable`, disabling is exactly the bookkeeping above; under `Halt`, this crate surfaces `HookOutcome::Panicked { .. }` and a `ModFaultPolicy::Halt`-tagged flag exactly as it would under `Disable`, and it is the **caller's** job (a future composition-root blueprint, the only crate with the authority to shut down the whole process) to observe that flag and act — this crate never unilaterally kills the server process, since doing so from inside a library crate several layers below the actual process owner would be exactly the kind of "structural isolation, not discipline" violation `01`'s ARCH-D22 already warns against for a different boundary. **Registered content's fate (06's own decision, restated verbatim for the case this blueprint's own dependency scope can actually speak to):** MOD-D6 freezes every registry after RegistryBuild — nothing in this engine's design ever reopens one. A mod disabled *after* RegistryBuild completed (the overwhelmingly common case — a tick-hook panic, a channel-message panic) therefore keeps every block/item/component it already registered fully present in the frozen registry and in the world; only its own **behavior going forward** stops running. This blueprint exposes exactly the query a future scheduler-integration blueprint's own `ModBlockBehavior`-wrapping adapter needs to honor that: `ServerModHost::is_disabled(&self, mod_id: &ModId) -> bool` — the seam this blueprint defines and a later blueprint's block-behavior dispatch adapter checks before every call, falling back to `NoOpBehavior`-equivalent (M3-B01's own default) semantics once `true`, exactly mirroring how `NoOpBehavior` already covers "no behavior registered at all." A mod disabled **during** its own `on_registry_build` call (a panic before that mod's own content ever registered) never contributes anything to the recorded set at all (Context: "The opaque `*Context` constructibility gap" — its `RegistryBuildContext`'s recorded `Vec`s simply reflect whatever was pushed before the panic, which a future scheduler blueprint discards entirely for a `Disabled` mod rather than partially applying) — and, per MOD-D31's own dependency-cascade policy extended here, any mod declaring a dependency on it is itself a per-mod hard load failure, computed at the same discovery pass (a registry-build-time panic is discovered only later, at RegistryBuild dispatch time, not at discovery time — so this cascade applies only to *discovery-time* failures; a registry-build panic's cascade to dependents is explicitly out of this blueprint's own scope, flagged in Open Questions, since resolving it needs the future scheduler blueprint's own RegistryBuild-ordering machinery this crate does not have).

### Double-fault analysis — the one residual, honestly-unpreventable failure mode

Two distinct things could be meant by "double fault," both addressed, one preventable and proven, one not:

1. **A second panic occurring in this crate's own catch-and-disable bookkeeping.** Proven not to happen by construction (previous subsection: the bookkeeping is simple, non-panicking safe Rust, and `parking_lot::Mutex` never poisons on a panicking guard-holder — the one realistic way `std::sync::Mutex`-based bookkeeping *could* have turned an ordinary caught panic into a second, unrelated panic on the very next lock acquisition). `disable_bookkeeping_never_panics_even_when_invoked_repeatedly` (Acceptance tests) proves this directly: the same panicking mod's same hook is called several times in a row (simulating a caller bug that forgets to check `is_disabled` first — a defensive proof, since this crate's own dispatch methods always check first) and every call after the first returns `HookOutcome::Skipped` cleanly, with the host's own state never corrupted.

2. **A panic occurring inside a mod's own `Drop` implementation, itself running as part of unwinding the *first* panic.** Rust's own runtime behavior here is unconditional and **cannot** be caught by any `catch_unwind` placement, on either side of this or any FFI boundary: a panic while a panic is already unwinding always aborts the process, full stop, regardless of `panic = "unwind"`. This blueprint does not, and cannot, prevent this — it is named here as a structural fact, not a bug this crate owns, and is verified rather than merely asserted: `double_fault_subprocess.rs` spawns this exact scenario (Context: "Fixture dylibs" — `double_fault_mod`, whose `ServerModEntry` implementer's `Drop` impl itself panics) inside a dedicated child process (`std::process::Command::new(std::env::current_exe())` re-invoked with a scenario-selecting argument, the standard technique for testing an expected-abort code path without taking the whole test binary down with it — `cargo-nextest`'s own per-test-process isolation, `12`'s WS-D10 rationale, is exactly what makes this technique safe to run inside the normal test suite rather than needing a separate, hand-run tool) and asserts the child's exit status reports an abort (a non-zero, non-ordinary-panic exit code — `!status.success()` at minimum, with a documented, non-binding note that the *exact* exit-code/signal shape is platform-dependent and this assertion only needs to distinguish "aborted" from "exited 0" or "exited via an ordinary caught-panic path," never a precise numeric code).

### Resource limits — stated honestly, per MOD-D9/D24's own native-tier trust-boundary framing

**Nothing beyond MOD-D26's SHA-256 allowlist is enforced by this crate for native-tier mods, and nothing else is even attempted.** `06`'s own Security Model section states this plainly for the tier this blueprint implements: "The native tier trades all of this away for performance: full process trust, honesty-based access declarations, no resource caps possible." No CPU budget, no memory cap, no filesystem/network capability gate exists anywhere in this blueprint's Deliverables for the native tier — a loaded, trusted native mod has the same process-wide authority the engine binary itself has, by design, the moment its hash matches an operator-approved entry. This is restated explicitly, not left to be inferred from an absence, because a reader of only this blueprint should never mistake "no enforcement code here" for an oversight rather than `06`'s own deliberate, disclosed trade-off.

### WASM-tier deferral — stated honestly, with an owner

Per `11-roadmap-milestones.md`'s own M8 scope text ("`rc-mod-host`'s dylib loader (`libloading`)") and this blueprint's own task framing ("follow `06`'s delivery-model tiering exactly for what M8 alpha ships... the roadmap scope names the dylib path for M8"), **this blueprint implements the native tier only.** The WASM tier's host-side embedding — a real `wasmtime::Engine`/`Store`, Component-Model instantiation against `rc-mod-api`'s already-shipped `wit/rc-mod-api.wit`/`guest.rs` bindings (M8-B01), `Config::consume_fuel`/`Config::epoch_interruption`/`Store::limiter` (MOD-D25's resource-limit mechanisms), and WASI's no-ambient-authority capability grant (MOD-D24) — is **not implemented here**. Owner: a future, not-yet-numbered `rc-mod-host` blueprint (this crate's own name and crate boundary do not change — `12`'s Crate Manifest already scopes `rc-mod-host` to cover both tiers; only the *implementation order* is native-first, matching the roadmap's own explicit M8-alpha scoping). This blueprint's own `ModTier` handling (from `rc_mod_api::manifest::ModTier`) treats a `tier = "wasm"` manifest as a clean, diagnosed skip (`ModLoadError::WasmTierNotYetSupported`, a per-mod, non-fatal-to-the-boot diagnostic) rather than a crash or a silent no-op — a WASM-tier mod's absence from the loaded set is always visible in `discover_and_load`'s own diagnostics.

### The opaque `*Context` constructibility gap — the one additive edit to `rc-mod-api`

M8-B01's own Implementation step 10 leaves `RegistryBuildContext`, `ServerInitContext`, `ServerShutdownContext`, `TickHookContext`, `ClientRegistryBuildContext`, `ClientInitContext` declared with **no fields** ("`/* opaque, engine-owned */`") and several method bodies `unimplemented!("wired by rc-mod-host, a later M8 blueprint")` — a deliberate, named exception M8-B01's own Constraints (a) and Implementation step 10 both explicitly sanction, not an oversight this blueprint silently discovers. **This blueprint is that later blueprint**, and completes exactly the four types this crate's own dispatch surface actually needs to construct and pass by `&mut` reference — no public method signature already declared by M8-B01 changes shape; every edit below is a private-field addition plus a new `pub fn` constructor/accessor, invisible to any code that only calls the methods M8-B01 already fixed.

`ServerInitContext`, `ServerShutdownContext`, `TickHookContext`, `ClientInitContext` carry **zero methods** in M8-B01's own Deliverables (no `impl` block is shown for any of them) — at M8 alpha they are, and remain, pure markers with nothing for a mod to call on them yet. This blueprint gives each a trivial `pub fn new() -> Self` (a bare, fieldless struct literal) — nothing more is needed or added, and nothing about their eventual real content (a future blueprint's job, once `TickHookContext` gains its own declared-access-scoped query methods, say) is anticipated or guessed at here.

`RegistryBuildContext` and `ClientRegistryBuildContext` **do** carry real methods in M8-B01 (`register_block`/`register_item`/`register_component`/`register_block_behavior`/`register_channel`, and the five `register_*` client extension points respectively) — for these, this blueprint's binding resolution is a **recording** structure, not a live callback into a `World` this crate cannot construct: each `register_*` call appends to an owned `Vec`, using M8-B01's own already-shipped `DenseIdAllocator` (`registry.rs`) to hand back a self-consistent, real-shaped id a mod can immediately reuse (e.g. passing a just-returned `ModBlockStateId` straight into `register_block_behavior`) without this crate needing any live registry to allocate from — a future `rc-scheduler` blueprint drains the finished recording (`RegistryBuildContext::into_recorded() -> RecordedRegistrations`) once `on_registry_build` returns and performs the *real* translation into `bevy_ecs`/`rc-chunk-storage`/`rc-mechanics` types at that point, never during the mod's own call. This is a materially simpler design than a per-call `stabby`-closure bundle (`ModUpdateContext`'s own pattern, Context: this blueprint never needs that pattern at all, since it never calls `ModBlockBehavior` — Context: "Where this blueprint's boundary actually falls") and is sufficient because RegistryBuild is a one-shot, boot-time, batch phase (MOD-D6), not a per-tick live-callback one.

```rust
// crates/mod-api/src/entrypoint.rs (modify — additive only; every method's PUBLIC
// signature below is copied verbatim from M8-B01's own Deliverables, unchanged)

use crate::registry::DenseIdAllocator;

pub struct RegistryBuildContext {
    block_ids: DenseIdAllocator,
    item_ids: DenseIdAllocator,
    component_ids: DenseIdAllocator,
    blocks: Vec<(ModBlockStateId, BlockRegistration)>,
    items: Vec<(ModItemId, ItemRegistration)>,
    components: Vec<(ModComponentId, ModComponentDescriptor)>,
    behaviors: Vec<(ModBlockStateId, stabby::dynptr!(stabby::boxed::Box<dyn ModBlockBehavior>))>,
    channels: Vec<Identifier>,
}
impl RegistryBuildContext {
    /// `block_id_start`/`item_id_start` seed the two `DenseIdAllocator`s (Context:
    /// M2-B01's own dense/sequential allocation rule) — a future `rc-scheduler`
    /// blueprint seeds these from the real pinned vanilla registry's own highest id
    /// + 1; this crate's own tests seed both at `0`.
    pub fn new(block_id_start: u32, item_id_start: u32) -> Self;
    pub fn register_block(&mut self, reg: BlockRegistration) -> ModBlockStateId;
    pub fn register_item(&mut self, reg: ItemRegistration) -> ModItemId;
    pub fn register_component(&mut self, descriptor: ModComponentDescriptor) -> ModComponentId;
    pub fn register_block_behavior(&mut self, state: ModBlockStateId, behavior: stabby::dynptr!(stabby::boxed::Box<dyn ModBlockBehavior>));
    /// MOD-D20: whether `id` appears in this mod's own declared `network_channels`
    /// is checked by `rc-mod-host` immediately after `on_registry_build` returns
    /// (Context's own resolution of M8-B01's "checked by `rc-mod-host` at call
    /// time" doc comment — functionally equivalent for a single-threaded,
    /// one-shot RegistryBuild call, and simpler), never inside this method.
    pub fn register_channel(&mut self, id: Identifier);
    /// Consumes `self`, returning everything recorded — the seam a future
    /// `rc-scheduler` blueprint drains and translates (Context).
    pub fn into_recorded(self) -> RecordedRegistrations;
}

/// Everything one mod's `on_registry_build` call recorded, in call order per
/// `Vec` (never reordered) — the complete, self-contained handoff to whichever
/// future blueprint performs the real `bevy_ecs`/registry translation.
pub struct RecordedRegistrations {
    pub blocks: Vec<(ModBlockStateId, BlockRegistration)>,
    pub items: Vec<(ModItemId, ItemRegistration)>,
    pub components: Vec<(ModComponentId, ModComponentDescriptor)>,
    pub behaviors: Vec<(ModBlockStateId, stabby::dynptr!(stabby::boxed::Box<dyn ModBlockBehavior>))>,
    pub channels: Vec<Identifier>,
}

pub struct ClientRegistryBuildContext {
    registrations: Vec<ClientRegistration>,
}
/// One recorded `ClientRegistryBuildContext::register_*` call — headless
/// verification's own observable unit (M8's own acceptance criterion 3:
/// "registered + headless-verified only").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientRegistration {
    ModelProvider(Identifier),
    BlockRenderer(Identifier),
    GuiScreen(Identifier),
    HudOverlay(Identifier),
    InputBinding(Identifier),
}
impl ClientRegistryBuildContext {
    pub fn new() -> Self;
    pub fn register_model_provider(&mut self, id: Identifier);
    pub fn register_block_renderer(&mut self, block: Identifier);
    pub fn register_gui_screen(&mut self, id: Identifier);
    pub fn register_hud_overlay(&mut self, id: Identifier);
    pub fn register_input_binding(&mut self, id: Identifier);
    /// Everything recorded so far, in call order — this blueprint's own headless
    /// verification reads this directly; a future `07-client-architecture.md`
    /// consumer drains it identically once a renderer exists (M10).
    pub fn registrations(&self) -> &[ClientRegistration];
}

impl ServerInitContext { pub fn new() -> Self; }
impl ServerShutdownContext { pub fn new() -> Self; }
impl TickHookContext { pub fn new() -> Self; }
impl ClientInitContext { pub fn new() -> Self; }
```

`crates/mod-api/src/lib.rs`'s existing `pub use entrypoint::{...}` line gains exactly two new names, `ClientRegistration` and `RecordedRegistrations`, appended to the already-present list — every name already exported by M8-B01 stays exported unchanged.

### Fixture dylibs — build mechanism

Every dylib-loading acceptance test needs a *real*, separately-compiled `cdylib` to load — a committed pre-built binary is never acceptable (no repository-committed compiled artifact of any kind, matching this project's own no-opaque-binary discipline). Each fixture lives at `crates/mod-host/tests/fixtures/<name>/` as a **standalone** Cargo package — its own `Cargo.toml` carrying an empty `[workspace]` table specifically to opt it *out* of the parent Rusty Clanker workspace (a mechanical, well-known Cargo idiom for "this package is a sibling package on disk, never a workspace member") — depending on `rc-mod-api` via a relative `path = "../../../mod-api"` with `default-features = false, features = ["native-tier"]`, plus `stabby`. This sidesteps `12`'s WS-D2 closed-manifest concern entirely: these six crates are never workspace members, never appear in `cargo metadata`'s workspace graph, and are invisible to `xtask lint-deps`.

`crates/mod-host/tests/common/mod.rs` provides the one shared helper every dylib test uses:

```rust
/// Builds `tests/fixtures/{fixture_name}`'s `cdylib` via a `cargo build` child
/// process (target dir: `{CARGO_TARGET_TMPDIR}/fixture-builds/{fixture_name}` — the
/// standard, dependency-free Cargo-provided per-test-binary scratch directory, no
/// `tempfile` crate needed), locates the produced artifact using
/// `std::env::consts::{DLL_PREFIX, DLL_SUFFIX}` (Cargo's own default `cdylib`
/// output naming, which — unlike this blueprint's own MOD-D4-shaped
/// `native_binary_filename` — DOES carry the `lib` prefix on Unix; this helper
/// locates cargo's own output under cargo's own convention, then renames it into
/// `.rcmod`'s MOD-D4-shaped convention on the way into the archive, exactly the
/// rename step a real mod-packaging pipeline would also need), and packs it plus
/// the given `manifest_toml` text into a fresh `.rcmod` zip archive under
/// `native/{current_target_triple}/{mod_id}.{ext}`. Returns the archive's path.
/// Cached per-process (built once, reused by every test in the same binary that
/// asks for the same fixture name) since a `cargo build` child process is not free.
pub fn build_fixture_archive(fixture_name: &str, mod_id: &str, manifest_toml: &str) -> std::path::PathBuf;
```

Six fixtures, each a minimal `ServerModEntry`/`ClientModEntry` implementer plus one exported `#[stabby::export]`-annotated factory function per side, plus the mandatory `#[stabby::export] extern "C" fn rc_mod_abi_handshake() -> ModAbiVersion` handshake symbol (Context):

| Fixture | Behavior |
|---|---|
| `good_mod` | Correct handshake (`MOD_API_VERSION` exactly); `on_registry_build` registers one block, one item, one component via `RegistryBuildContext`, returns `Ok`; `on_server_init`/`on_client_init` return `Ok`; `on_tick_hook` for `hook_id == "good_mod:demo"` appends a line to the file named by env var `RC_MOD_HOST_FIXTURE_LOG_PATH` (read at call time — the dylib and the test process share one OS process's environment once loaded, so the test sets this var via `std::env::set_var` before dispatching, then reads the file afterward — this blueprint's own simple, portable, no-extra-FFI signaling mechanism, chosen deliberately over a raw C-string-returning diagnostic symbol) and returns `Ok`; `on_channel_message`/`on_mod_message` likewise append a log line. |
| `panicking_mod` | Correct handshake/entry; `on_tick_hook` for `hook_id == "panicking_mod:boom"` unconditionally `panic!("panicking_mod: deliberate test panic")`; every other hook behaves like `good_mod`'s. |
| `wrong_abi_mod` | Exports `rc_mod_abi_handshake` returning `ModAbiVersion { major: 99, minor: 0, patch: 0 }`. |
| `no_handshake_mod` | A structurally valid `cdylib` exporting *no* symbol named `rc_mod_abi_handshake` at all. |
| `no_entry_factory_mod` | Correct handshake; exports no symbol matching whatever name its own accompanying test-supplied manifest names in `[entrypoints].server`. |
| `double_fault_mod` | Correct handshake/entry; its `ServerModEntry`-implementing struct has a hand-written `impl Drop` that itself unconditionally panics; its `on_tick_hook` unconditionally panics — used only by `double_fault_subprocess.rs`'s dedicated child-process scenario (Context: "Double-fault analysis"), never by any test running in the main suite's own process. |

## Deliverables

### `crates/mod-host/Cargo.toml` (modify — full expected content)

```toml
[package]
name = "rc-mod-host"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
rc-mod-api = { path = "../mod-api", default-features = false, features = ["native-tier"] }
libloading = { workspace = true }
zip = { workspace = true }
stabby = { workspace = true }
parking_lot = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

(No `[build-dependencies]` beyond `std`; no new `[dev-dependencies]` — fixture dylibs are built via a `cargo build` child-process call from ordinary test code, needing no additional crate.)

### `crates/mod-host/build.rs` (new)

```rust
fn main() {
    let target = std::env::var("TARGET").expect("Cargo always sets TARGET for a build script invocation");
    println!("cargo:rustc-env=RC_MOD_HOST_TARGET_TRIPLE={target}");
}
```

### `crates/mod-host/src/lib.rs`

```rust
//! `rc-mod-host` — engine-side native-tier mod loader: `mods/` directory discovery,
//! `.rcmod` zip extraction, MOD-D31 dependency-order resolution, MOD-D4 platform-
//! triple/filename resolution, MOD-D26 SHA-256 trust-allowlist enforcement, the
//! MOD-D21/D22 ABI version handshake, `libloading`-based dylib loading, and
//! `catch_unwind`-at-every-boundary crash isolation with auto-disable-on-panic
//! (MOD-D32). Depends only on `{rc-core, rc-mod-api}` beyond its own leaf-level
//! utility crates (`libloading`, `zip`, `stabby`, `parking_lot`, `thiserror`,
//! `tracing`) — never `bevy_ecs`, `rc-scheduler`, `rc-mechanics`, `rc-chunk-storage`,
//! or `rc-registries` (Context: "Where this blueprint's boundary actually falls").
//! WASM-tier host embedding is deliberately not implemented here (Context:
//! "WASM-tier deferral").

mod sha256;
mod platform;
mod discovery;
mod dependency_order;
mod handshake;
mod entry_factory;
mod isolation;
mod trust;
mod config;
mod error;
mod host;

pub use sha256::sha256_hex;
pub use platform::{native_binary_filename, CURRENT_TARGET_TRIPLE};
pub use config::{ModHostConfig, ModFaultPolicy, NativeTrustEntry};
pub use error::{ModHostBootError, ModLoadError};
pub use isolation::{CaughtPanic, HookOutcome};
pub use host::{
    ClientModHost, DisableReason, LoadOutcome, ModLoadDiagnostic, ModStatus, ServerModHost,
};
pub use entry_factory::{ClientEntryFactoryFn, ServerEntryFactoryFn};
```

### `crates/mod-host/src/sha256.rs`

```rust
/// Hand-rolled FIPS 180-4 SHA-256 (Context: "MOD-D26's SHA-256 trust allowlist" —
/// no SHA-256 crate is pinned anywhere in the workspace; this restates M0-B08's own
/// identical resolution for TEST-D47, independently, for this crate's own
/// production use). Pure safe Rust, no `unsafe`.
pub fn sha256_hex(bytes: &[u8]) -> String;
```

### `crates/mod-host/src/platform.rs`

```rust
/// This crate's own build-time-resolved Rust target triple (`build.rs`, Context:
/// "Platform-triple resolution"). Correct even under cross-compilation, unlike a
/// hand-matched `cfg!` table.
pub const CURRENT_TARGET_TRIPLE: &str = env!("RC_MOD_HOST_TARGET_TRIPLE");

/// MOD-D4's literal `<mod_id>.{dll,so,dylib}` convention — deliberately not
/// `libloading::library_filename`'s `lib`-prefixed convention (Context).
#[cfg(target_os = "windows")]
pub fn native_binary_filename(mod_id: &rc_mod_api::ModId) -> String;
#[cfg(target_os = "macos")]
pub fn native_binary_filename(mod_id: &rc_mod_api::ModId) -> String;
#[cfg(all(unix, not(target_os = "macos")))]
pub fn native_binary_filename(mod_id: &rc_mod_api::ModId) -> String;
```

### `crates/mod-host/src/config.rs`

```rust
use rc_mod_api::ModId;

#[derive(Clone, Debug)]
pub struct ModHostConfig {
    pub mods_dir: std::path::PathBuf,
    /// MOD-D26's operator-maintained allowlist — pre-parsed by the caller; this
    /// crate owns no server-config-file schema of its own.
    pub native_trust: Vec<NativeTrustEntry>,
    pub fault_policy: ModFaultPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeTrustEntry {
    /// Lowercase hex, compared case-insensitively (Context).
    pub sha256_hex: String,
    pub mod_id: ModId,
}

/// MOD-D32's own two policies, restated exactly ("mirroring MOD-D25's WASM-tier
/// `mod_fault_policy` default, with the same operator-configurable `\"halt\"`
/// escape hatch").
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ModFaultPolicy {
    #[default]
    Disable,
    Halt,
}
```

### `crates/mod-host/src/error.rs`

```rust
use rc_mod_api::{IdentifierError, ManifestError, ManifestValidationError, ModAbiVersion, ModId};

/// Fails the *whole* discovery pass — reserved for genuine I/O failure reading an
/// *existing* `mods_dir` (Context: a missing `mods_dir` is not an error at all).
#[derive(Debug, thiserror::Error)]
pub enum ModHostBootError {
    #[error("failed to read mods directory {path:?}: {source}")]
    ModsDirIo { path: std::path::PathBuf, source: std::io::Error },
}

/// Fails exactly one mod (MOD-D31's own "reported by name and reason" policy,
/// extended to every failure class this crate can produce). Never aborts
/// `discover_and_load` as a whole — collected into `ModLoadDiagnostic`.
#[derive(Debug, thiserror::Error)]
pub enum ModLoadError {
    #[error("failed to open {path:?} as a zip archive: {0}")]
    ArchiveOpen(std::path::PathBuf, #[source] zip::result::ZipError),
    #[error("{path:?} contains no manifest.toml entry")]
    ManifestEntryMissing { path: std::path::PathBuf },
    #[error("manifest.toml in {path:?} is not valid UTF-8")]
    ManifestNotUtf8 { path: std::path::PathBuf },
    #[error(transparent)]
    ManifestParse(#[from] ManifestError),
    #[error("manifest validation failed with {} problem(s): {0:?}", .0.len())]
    ManifestInvalid(Vec<ManifestValidationError>),
    #[error("mod id {namespace:?} is not a valid mod identifier: {source}")]
    InvalidModId { namespace: String, #[source] source: IdentifierError },
    #[error("mod {mod_id} declares a dependency on unknown mod {missing}")]
    MissingDependency { mod_id: ModId, missing: ModId },
    #[error("dependency cycle detected among: {0:?}")]
    DependencyCycle(Vec<ModId>),
    #[error("mod {mod_id} depends on {failed}, which itself failed to load")]
    DependencyFailed { mod_id: ModId, failed: ModId },
    #[error("mod {mod_id} declares tier = \"wasm\" — the WASM tier is not yet implemented by this engine build (Context: WASM-tier deferral)")]
    WasmTierNotYetSupported { mod_id: ModId },
    #[error("mod {mod_id} has no [entrypoints.native.{triple}] table naming this platform — skipped, not an error")]
    PlatformNotDeclared { mod_id: ModId, triple: String },
    #[error("{path:?} contains no {expected:?} entry named by [entrypoints.native.{triple}]")]
    NativeBinaryEntryMissing { path: std::path::PathBuf, expected: String, triple: String },
    #[error("mod {mod_id}'s native binary (sha256 {actual_hash}) matches no entry in [mods.native.trusted] (MOD-D26)")]
    UntrustedNativeBinary { mod_id: ModId, actual_hash: String },
    #[error("failed to load {path:?}: {source}")]
    DylibOpen { mod_id: ModId, path: std::path::PathBuf, #[source] source: libloading::Error },
    #[error("mod {mod_id}'s ABI handshake symbol ({symbol}) could not be resolved: {source}")]
    HandshakeSymbolMissing { mod_id: ModId, symbol: &'static str, #[source] source: libloading::Error },
    #[error("mod {mod_id} reports ABI version {mod_version:?}, incompatible with this engine's {engine_version:?}")]
    AbiIncompatible { mod_id: ModId, mod_version: ModAbiVersion, engine_version: ModAbiVersion },
    #[error("mod {mod_id}'s entrypoint factory symbol {symbol:?} could not be resolved: {source}")]
    EntryFactorySymbolMissing { mod_id: ModId, symbol: String, #[source] source: libloading::Error },
    #[error("mod {mod_id}'s entrypoint factory panicked during construction")]
    EntryFactoryPanicked { mod_id: ModId },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

### `crates/mod-host/src/discovery.rs`

```rust
use rc_mod_api::ModManifest;

pub struct DiscoveredMod {
    pub archive_path: std::path::PathBuf,
    pub manifest: ModManifest,
}

/// Scans `mods_dir` non-recursively for `*.rcmod` entries, extracts and
/// parses+validates each `manifest.toml` (Context). Returns one entry per archive
/// that parsed and validated successfully, plus a diagnostic for every archive that
/// did not (collected, never fail-fast). A nonexistent `mods_dir` yields
/// `(vec![], vec![])`, not an error.
pub fn discover(mods_dir: &std::path::Path) -> Result<(Vec<DiscoveredMod>, Vec<(std::path::PathBuf, crate::ModLoadError)>), crate::ModHostBootError>;
```

### `crates/mod-host/src/dependency_order.rs`

```rust
use rc_mod_api::{ModId, ModManifest};

/// MOD-D31's Kahn's-algorithm resolution (Context — restated as a second
/// application of M0-B05's own technique). Returns mods in a valid load order
/// (dependency before dependent); every entry in `failed` names a mod excluded by
/// a missing dependency, a cycle, or (transitively) a failed dependency, alongside
/// the specific `ModLoadError` variant naming why.
pub fn resolve_load_order(
    discovered: Vec<(ModId, ModManifest)>,
) -> (Vec<(ModId, ModManifest)>, Vec<(ModId, crate::ModLoadError)>);
```

### `crates/mod-host/src/handshake.rs`

```rust
use rc_mod_api::ModAbiVersion;

/// Looks up and calls `ABI_HANDSHAKE_SYMBOL` (Context), returning the mod's own
/// reported `ModAbiVersion`. Does not itself check compatibility — the caller
/// compares against `rc_mod_api::MOD_API_VERSION`.
pub fn run_handshake(library: &libloading::Library) -> Result<ModAbiVersion, libloading::Error>;
```

### `crates/mod-host/src/entry_factory.rs`

```rust
/// Context: "The entrypoint-factory symbol contract."
pub type ServerEntryFactoryFn =
    unsafe extern "C" fn() -> stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ServerModEntry>);
pub type ClientEntryFactoryFn =
    unsafe extern "C" fn() -> stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ClientModEntry>);

/// Resolves `symbol_name` as a `ServerEntryFactoryFn` and calls it, itself wrapped
/// in `catch_unwind` (Context — even the very first call is guarded).
pub fn load_server_entry(
    library: &libloading::Library,
    symbol_name: &str,
) -> Result<stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ServerModEntry>), EntryLoadError>;
pub fn load_client_entry(
    library: &libloading::Library,
    symbol_name: &str,
) -> Result<stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ClientModEntry>), EntryLoadError>;

#[derive(Debug, thiserror::Error)]
pub enum EntryLoadError {
    #[error("entrypoint factory symbol {0:?} could not be resolved: {1}")]
    SymbolMissing(String, #[source] libloading::Error),
    #[error("entrypoint factory panicked during construction")]
    Panicked,
}
```

### `crates/mod-host/src/isolation.rs`

```rust
/// A panic caught by `call_guarded`, reduced to a `String` message (Context:
/// downcasts `&str`/`String` payloads; any other payload type becomes a fixed
/// `"<non-string panic payload>"` placeholder).
#[derive(Debug, Clone)]
pub struct CaughtPanic {
    pub message: String,
}

/// Every hook-dispatch method's own return shape (Context: "Crash isolation").
#[derive(Debug)]
pub enum HookOutcome<T> {
    /// The call ran to completion; `T` is the hook's own real return value.
    Ran(T),
    /// The call panicked; caught, logged, and the owning mod is now `Disabled`.
    Panicked { message: String },
    /// The owning mod was already `Disabled` before this call was attempted — no
    /// call was made at all.
    Skipped,
}

/// The one, generic catch_unwind wrapper every hook-dispatch method reuses
/// (Context). Never used directly outside this crate — `host.rs`'s own dispatch
/// methods are the only callers.
pub(crate) fn call_guarded<R>(f: impl FnOnce() -> R + std::panic::UnwindSafe) -> Result<R, CaughtPanic>;
```

### `crates/mod-host/src/trust.rs`

```rust
use crate::config::NativeTrustEntry;
use rc_mod_api::ModId;

/// MOD-D26: `bytes`'s SHA-256 (Context, `sha256.rs`) must match some `entries`
/// row's `sha256_hex` (case-insensitive) for `mod_id` specifically — a hash
/// matching a *different* mod's trusted entry does not count. Returns `Ok(())` or
/// `Err(actual_hash)` for the caller to build `ModLoadError::UntrustedNativeBinary`.
pub fn check_trusted(bytes: &[u8], mod_id: &ModId, entries: &[NativeTrustEntry]) -> Result<(), String>;
```

### `crates/mod-host/src/host.rs`

```rust
use parking_lot::{Mutex, RwLock};
use rc_mod_api::{ClientModEntry, ModAddress, ModId, ModManifest, ServerModEntry};
use crate::config::ModHostConfig;
use crate::isolation::HookOutcome;

#[derive(Clone, Debug)]
pub enum ModStatus {
    Active,
    Disabled { reason: DisableReason, panic_count: u32 },
}

#[derive(Clone, Debug)]
pub enum DisableReason {
    Panic { hook: String, message: String },
}

struct LoadedServerMod {
    manifest: ModManifest,
    /// Declared before `library` so it drops first (Rust's own struct-field
    /// declaration-order drop guarantee) — the boxed `dyn ServerModEntry`'s vtable
    /// pointers must never outlive the `Library` they point into.
    /// SAFETY: relies on this declared field order; do not reorder.
    entry: Mutex<stabby::dynptr!(stabby::boxed::Box<dyn ServerModEntry>)>,
    status: RwLock<ModStatus>,
    library: libloading::Library,
}

pub struct ServerModHost {
    loaded: std::collections::HashMap<ModId, LoadedServerMod>,
    config: ModHostConfig,
}

/// One discovered `.rcmod` archive's own final outcome (Context: "collect-all,
/// never fail-fast").
#[derive(Debug)]
pub enum LoadOutcome {
    Loaded,
    /// Not an error — MOD-D5's own "silently a no-op" precedent (no server/client
    /// entrypoint declared for this side, or `PlatformNotDeclared`).
    NotApplicable { reason: String },
    Failed(crate::ModLoadError),
}

#[derive(Debug)]
pub struct ModLoadDiagnostic {
    pub archive_path: std::path::PathBuf,
    pub mod_id: Option<ModId>,
    pub outcome: LoadOutcome,
}

impl ServerModHost {
    /// The full pipeline (Context, every subsection in order): discovery ->
    /// dependency-order resolution -> per-mod (platform check -> extraction ->
    /// trust check -> dylib load -> handshake -> entry factory) -> insertion into
    /// `loaded`. Every per-mod failure becomes exactly one `ModLoadDiagnostic`;
    /// `discover_and_load` itself only fails for `ModHostBootError`'s narrow
    /// I/O-on-an-existing-directory case.
    pub fn discover_and_load(config: ModHostConfig) -> Result<(Self, Vec<ModLoadDiagnostic>), crate::ModHostBootError>;

    pub fn loaded_mod_ids(&self) -> Vec<ModId>;
    pub fn status(&self, mod_id: &ModId) -> Option<ModStatus>;
    pub fn is_disabled(&self, mod_id: &ModId) -> bool;
    /// Force-disables `mod_id` for a reason this crate did not itself observe (a
    /// future resource-limit or scheduler-integration blueprint's own escalation
    /// path) — exposed so that seam exists without this crate needing to know
    /// what triggers it.
    pub fn disable(&self, mod_id: &ModId, reason: DisableReason);

    pub fn call_on_registry_build(&self, mod_id: &ModId, ctx: &mut rc_mod_api::RegistryBuildContext) -> HookOutcome<stabby::result::Result<(), rc_mod_api::ModInitError>>;
    pub fn call_on_server_init(&self, mod_id: &ModId, ctx: &mut rc_mod_api::ServerInitContext) -> HookOutcome<stabby::result::Result<(), rc_mod_api::ModInitError>>;
    pub fn call_on_server_shutdown(&self, mod_id: &ModId, ctx: &mut rc_mod_api::ServerShutdownContext) -> HookOutcome<()>;
    pub fn call_on_tick_hook(&self, mod_id: &ModId, hook_id: &str, ctx: &mut rc_mod_api::TickHookContext) -> HookOutcome<stabby::result::Result<(), rc_mod_api::ModHookError>>;
    pub fn call_on_channel_message(&self, mod_id: &ModId, channel: &str, sender_entity: u64, payload: &[u8]) -> HookOutcome<()>;
    pub fn call_on_mod_message(&self, mod_id: &ModId, channel: &str, sender: &ModAddress, payload: &[u8]) -> HookOutcome<()>;
}

struct LoadedClientMod {
    manifest: ModManifest,
    entry: Mutex<stabby::dynptr!(stabby::boxed::Box<dyn ClientModEntry>)>,
    status: RwLock<ModStatus>,
    library: libloading::Library,
}

/// Mirrors `ServerModHost` exactly, resolving `[entrypoints].client`/native-triple
/// client support instead of server (MOD-D5's exclusive per-side load). Headless-
/// testable today (Context: "Fixture dylibs") — no renderer exists until M10.
pub struct ClientModHost {
    loaded: std::collections::HashMap<ModId, LoadedClientMod>,
    config: ModHostConfig,
}
impl ClientModHost {
    pub fn discover_and_load(config: ModHostConfig) -> Result<(Self, Vec<ModLoadDiagnostic>), crate::ModHostBootError>;
    pub fn loaded_mod_ids(&self) -> Vec<ModId>;
    pub fn status(&self, mod_id: &ModId) -> Option<ModStatus>;
    pub fn is_disabled(&self, mod_id: &ModId) -> bool;
    pub fn disable(&self, mod_id: &ModId, reason: DisableReason);
    pub fn call_on_client_registry_build(&self, mod_id: &ModId, ctx: &mut rc_mod_api::ClientRegistryBuildContext) -> HookOutcome<stabby::result::Result<(), rc_mod_api::ModInitError>>;
    pub fn call_on_client_init(&self, mod_id: &ModId, ctx: &mut rc_mod_api::ClientInitContext) -> HookOutcome<stabby::result::Result<(), rc_mod_api::ModInitError>>;
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file below, plus every `crates/mod-host/src/*.rs` file from Deliverables with every function body replaced by `todo!()` (field lists, derives, doc comments stay exactly as specified), plus `Cargo.toml`, `build.rs` (which has no `todo!()`-able body — it either compiles or it doesn't, ships complete in the test changeset, mirroring M8-B01's own `guest.rs` precedent), and the six fixture crates under `tests/fixtures/` (real, complete, working source — these are test *inputs*, not implementation, and ship complete from the start, exactly as M0-B05's `tests/common/mod.rs` marker components do). The additive `rc-mod-api` edit (`entrypoint.rs`) ships with every new struct/field/derive exactly as specified and every new method's body `todo!()`-stubbed, plus its own new test file below. The implementation changeset fills in bodies only; it must not modify any file under either crate's `tests/` directory, must not change any type's field list/derive list/public signature, and must not weaken any assertion below.

### `crates/mod-api/tests/entrypoint_context_construction.rs` (new — `rc-mod-api`'s own test changeset)

1. `registry_build_context_records_blocks_in_call_order` — `RegistryBuildContext::new(0, 0)`; register three blocks with distinct `default_state_component_count`s; `into_recorded().blocks` has length 3, in call order, each `ModBlockStateId` distinct and ascending from `0`.
2. `registry_build_context_item_and_block_ids_are_independent` — register 2 blocks then 2 items; block ids are `[0, 1]`, item ids are `[0, 1]` (independent counters, matching `DenseIdAllocator`'s own already-proven independence, M8-B01's `registry_ids.rs` test 5).
3. `registry_build_context_component_ids_are_sequential` — register 3 components; recorded `ModComponentId`s are `0, 1, 2` (widened from the internal `u32` allocator).
4. `register_block_behavior_is_recorded_with_its_state_id` — register one block (id `0`), then `register_block_behavior(ModBlockStateId(0), <a dynptr-boxed dummy ModBlockBehavior>)`; `into_recorded().behaviors` has length 1, keyed `ModBlockStateId(0)`.
5. `register_channel_is_recorded_in_call_order` — three `register_channel` calls with distinct `Identifier`s; `into_recorded().channels` equals them in order.
6. `client_registry_build_context_records_every_kind_in_call_order` — call each of the five `register_*` methods once, in a fixed order, with distinct `Identifier`s; `.registrations()` returns exactly five entries, in that exact order, each the correct `ClientRegistration` variant.
7. `marker_contexts_are_constructible` — `ServerInitContext::new()`, `ServerShutdownContext::new()`, `TickHookContext::new()`, `ClientInitContext::new()` all compile and construct (a compile-time-and-construction-only assertion — nothing else to check on a fieldless marker).

### `crates/mod-host/tests/sha256_vectors.rs`

1. `empty_input` — `sha256_hex(b"")` equals the well-known published SHA-256 empty-string digest, `"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"` (a fixed, published test vector, not derived from any Mojang or third-party source — ASSET-D18/D19).
2. `abc` — `sha256_hex(b"abc")` equals the well-known published vector `"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"`.
3. `long_input_spanning_multiple_blocks` — `sha256_hex` of the published NIST test vector `"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"` equals its published digest `"248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"`.

### `crates/mod-host/tests/platform_naming.rs`

1. `native_binary_filename_has_no_lib_prefix_on_unix` (cfg'd to `unix`) — `native_binary_filename(&ModId::new("example_ores").unwrap())` equals `"example_ores.so"` on Linux or `"example_ores.dylib"` on macOS — never `"libexample_ores.so"`.
2. `native_binary_filename_uses_dll_extension_on_windows` (cfg'd to `windows`) — equals `"example_ores.dll"`.
3. `current_target_triple_is_nonempty_and_well_formed` — `CURRENT_TARGET_TRIPLE` is non-empty and contains at least two `-`-separated components (a coarse sanity check, not a full triple-grammar validator).

### `crates/mod-host/tests/manifest_discovery.rs`

Uses `build_fixture_archive` (Context, `tests/common/mod.rs`) with `good_mod`.

1. `discovers_a_well_formed_archive` — one `good_mod` archive dropped into a fresh `CARGO_TARGET_TMPDIR`-scoped `mods_dir`; `discover(mods_dir)` returns one `DiscoveredMod` whose `manifest.mod_.id.as_str() == "good_mod"`, zero diagnostics.
2. `missing_mods_dir_is_not_an_error` — `discover(&nonexistent_path)` returns `Ok((vec![], vec![]))`.
3. `non_rcmod_files_are_ignored` — a `mods_dir` containing one `.txt` file and one real `.rcmod`; only the `.rcmod` is discovered.
4. `archive_with_no_manifest_entry_is_diagnosed_not_fatal` — a hand-built zip with no `manifest.toml` entry at all; `discover` returns `Ok((vec![], vec![(path, ModLoadError::ManifestEntryMissing { .. })]))` — the *pass* succeeds, the *one archive* is diagnosed.
5. `invalid_toml_is_diagnosed_not_fatal` — a hand-built zip whose `manifest.toml` is `"not valid toml {{{"`; diagnosed as `ModLoadError::ManifestParse(_)`, discovery pass still succeeds overall.

### `crates/mod-host/tests/dependency_order.rs` (pure — operates on hand-built `(ModId, ModManifest)` pairs, no dylib needed)

1. `linear_chain_resolves_in_dependency_first_order` — `a` depends on `b` depends on `c` (no cycle); `resolve_load_order` returns `[c, b, a]` (or any order respecting "dependency before dependent" — assert index(`c`) < index(`b`) < index(`a`), not a single fixed permutation, since Kahn's algorithm's own tie-break among equally-ready nodes is this blueprint's own implementation freedom exactly as M0-B05's `compute_waves` already established for a different graph).
2. `missing_dependency_fails_only_the_declaring_mod` — `a` depends on `b`, `b` is simply absent from the discovered set; `a` is in `failed` (`MissingDependency`), and if a third mod `c` has no dependencies, `c` is still in the resolved order.
3. `dependency_cycle_fails_every_mod_in_the_cycle` — `a` depends on `b`, `b` depends on `a`; both `a` and `b` are in `failed` (`DependencyCycle`), and an unrelated, dependency-free `c` still resolves.
4. `failure_cascades_transitively` — `a` depends on `b`, `b` depends on `missing`; both `a` (`DependencyFailed`) and `b` (`MissingDependency`) end up in `failed`.
5. `no_dependencies_resolves_trivially` — three mods, no `[dependencies]` entries anywhere; all three resolve, `failed` is empty.

### `crates/mod-host/tests/trust_allowlist.rs`

1. `matching_hash_and_mod_id_is_trusted` — `check_trusted(bytes, &mod_id, &[NativeTrustEntry { sha256_hex: sha256_hex(bytes), mod_id: mod_id.clone() }])` is `Ok(())`.
2. `hash_matching_a_different_mod_id_is_untrusted` — the same hash, but the trust entry's `mod_id` names a different mod: `Err(_)`.
3. `hash_case_insensitivity` — the trust entry's `sha256_hex` is uppercase; still `Ok(())`.
4. `no_matching_entry_is_untrusted` — an empty `entries` list: `Err(_)`, `Err`'s payload equals `sha256_hex(bytes)`.

### `crates/mod-host/tests/handshake_matrix.rs`

Uses `build_fixture_archive` with each of `good_mod`, `wrong_abi_mod`, `no_handshake_mod`, `no_entry_factory_mod`, and a hand-built archive with a syntactically-broken manifest (reusing `manifest_discovery.rs`'s technique) — the full matrix named by this blueprint's own Done-definition.

1. `good_mod_loads_and_handshakes_successfully` — full `ServerModHost::discover_and_load` against a `mods_dir` containing only `good_mod`; one `ModLoadDiagnostic` with `outcome: LoadOutcome::Loaded`; `host.status(&mod_id) == Some(ModStatus::Active)`.
2. `wrong_abi_version_is_rejected_before_entry_factory_is_ever_resolved` — `wrong_abi_mod`; diagnostic is `Failed(ModLoadError::AbiIncompatible { .. })`; the mod is absent from `loaded_mod_ids()`.
3. `missing_handshake_symbol_is_rejected` — `no_handshake_mod`; diagnostic is `Failed(ModLoadError::HandshakeSymbolMissing { .. })`.
4. `missing_entry_factory_symbol_is_rejected_after_a_successful_handshake` — `no_entry_factory_mod` (correct handshake, wrong/absent factory symbol); diagnostic is `Failed(ModLoadError::EntryFactorySymbolMissing { .. })` — proving the handshake genuinely ran first and succeeded before this later failure was even reached (distinguishing this case from test 3's earlier failure).
5. `bad_manifest_is_rejected_before_any_dylib_is_ever_opened` — the syntactically-broken-manifest archive; diagnostic is `Failed(ModLoadError::ManifestParse(_))`; assert (via a `good_mod` sibling in the same `mods_dir`) that a bad manifest in one archive never prevents a well-formed sibling archive from loading successfully in the same `discover_and_load` call.

### `crates/mod-host/tests/entry_loading_and_dispatch.rs`

Uses `good_mod`.

1. `on_registry_build_dispatches_with_correct_data` — load `good_mod`; construct `RegistryBuildContext::new(0, 0)`; `host.call_on_registry_build(&mod_id, &mut ctx)` is `HookOutcome::Ran(stabby::result::Result::Ok(()))`; `ctx.into_recorded()` shows exactly the one block, one item, one component `good_mod`'s own fixture registers (Context's fixture table) — this is the concrete "hook fires... with correct data" proof at this crate's own scope (the full milestone-level, scheduler-integrated version of this proof is a later blueprint's job, Context: "Where this blueprint's boundary actually falls").
2. `on_tick_hook_dispatches_and_the_mod_observably_ran` — set `RC_MOD_HOST_FIXTURE_LOG_PATH` to a fresh temp file path (`CARGO_TARGET_TMPDIR`-scoped); `host.call_on_tick_hook(&mod_id, "good_mod:demo", &mut TickHookContext::new())` is `HookOutcome::Ran(stabby::result::Result::Ok(()))`; the log file now contains the expected marker line.
3. `on_channel_message_and_on_mod_message_both_dispatch_with_plain_args` — call both with distinct `channel`/`payload` values; both return `HookOutcome::Ran(())`; the log file shows both, with the exact `channel`/`sender_entity`/`payload` bytes round-tripped.
4. `client_side_dispatches_identically` — `ClientModHost::discover_and_load` against the same `good_mod` archive; `call_on_client_registry_build` returns `Ran(Ok(()))`; the passed `ClientRegistryBuildContext`'s `.registrations()` reflects `good_mod`'s own client-side registration calls (this fixture's client entry additionally calls `register_block_renderer` once, for this test's own assertion).
5. `unloaded_mod_id_returns_none_not_a_panic` — `host.status(&ModId::new("nonexistent").unwrap())` is `None`; calling any `call_on_*` method with an unloaded `mod_id` returns `HookOutcome::Skipped` (never panics, never a `Result::Err` — a caller-side bug, handled gracefully).

### `crates/mod-host/tests/crash_isolation.rs`

Uses `panicking_mod` alongside `good_mod` (both loaded into the same `ServerModHost`, proving isolation *between* mods, not merely that one mod's own panic is caught).

1. **`vtable_dispatched_panic_is_caught_smoke_test`** — Implementation step 1's own standalone proof (Context: "The unwind-across-the-vtable-boundary question"), repeated here as a committed, permanent regression guard, not only a one-time manual check: load `panicking_mod`, call `call_on_tick_hook(&mod_id, "panicking_mod:boom", &mut TickHookContext::new())`; assert the call returns (does not abort the test process) with `HookOutcome::Panicked { message }` where `message` contains the fixture's own known panic string.
2. `panicking_mod_is_disabled_after_the_panic` — immediately after test 1's call, `host.status(&mod_id)` is `Some(ModStatus::Disabled { reason: DisableReason::Panic { hook, message }, panic_count: 1 })` where `hook == "panicking_mod:boom"`.
3. `disabled_mod_skips_every_subsequent_call_without_attempting_it` — after test 2's disable, call `call_on_tick_hook` again (any `hook_id`, including a *different* one than the one that originally panicked) — returns `HookOutcome::Skipped`, and (via the fixture's own file-log mechanism) no new log line appears, proving the call was never actually attempted, not merely that its result was discarded.
4. `every_other_loaded_mod_remains_fully_callable` — with `panicking_mod` now disabled (tests 1–3), `good_mod`'s own `call_on_tick_hook(&mod_id_of_good_mod, "good_mod:demo", ..)` still returns `HookOutcome::Ran(Ok(()))` — the host process, and every *other* mod, survived and remain unaffected.
5. `disable_bookkeeping_never_panics_even_when_invoked_repeatedly` — call `panicking_mod`'s panicking hook three times in a row *before* checking `is_disabled` between calls (bypassing the normal skip-check path deliberately, to stress the bookkeeping itself, Context: "Double-fault analysis" item 1); every one of the three calls returns cleanly (`Panicked` the first time, `Skipped` for the second and third once the status write from the first call has landed — a data race between "check status" and "the status write from an in-flight first call" is not possible here since this test's three calls are sequential on one thread, not concurrent), and the host's own internal state is never corrupted (assert `loaded_mod_ids()` still returns both mods afterward).

### `crates/mod-host/tests/double_fault_subprocess.rs`

1. `panic_during_drop_during_unwind_aborts_the_process_not_silently` — the parent test process spawns `Command::new(std::env::current_exe()).arg("--double-fault-child-scenario")` (this blueprint's own dedicated internal re-invocation convention — the test binary's own `main`-equivalent recognizes this one argument and, instead of running the normal test harness, loads `double_fault_mod`, calls its panicking `on_tick_hook` once, and returns/exits normally if — contrary to Context's own analysis — it somehow survives); asserts the child process's `ExitStatus` reports failure (`!status.success()`), distinguishing this from both a clean `exit(0)` and from an *ordinary* caught-and-continued run (which this same harness, invoked without the special argument, already proves happens for `panicking_mod` in `crash_isolation.rs` — this test's own value is specifically that `double_fault_mod`'s scenario is *worse* than that, not merely present).

### `crates/mod-host/tests/wasm_tier_deferral.rs`

1. `wasm_tier_manifest_is_a_clean_diagnosed_skip_not_a_crash` — a hand-built archive whose manifest declares `tier = "wasm"`; `discover_and_load` succeeds overall, the diagnostic for this one archive is `Failed(ModLoadError::WasmTierNotYetSupported { .. })`, and (alongside a `good_mod` sibling in the same `mods_dir`) the sibling still loads successfully.

## Implementation steps

1. **The vtable-unwind smoke test, first, before anything else.** Build the smallest possible fixture (a trivial `#[stabby::stabby]` one-method trait, one panicking implementation, one `#[stabby::export]` factory function) and a throwaway test proving `catch_unwind` around a call reached through its `dynptr!`-boxed vtable genuinely catches the panic rather than aborting (Context: "The unwind-across-the-vtable-boundary question"). Observable: this smoke test passes. **If it does not, stop and escalate** (Context) — every step below assumes it passes. Fold this proof into `crash_isolation.rs`'s own committed `vtable_dispatched_panic_is_caught_smoke_test` (Acceptance tests) rather than discarding the throwaway harness.
2. **`sha256.rs`.** Implement FIPS 180-4 SHA-256 by hand (safe Rust, no `unsafe`) per the published algorithm. Observable: `sha256_vectors.rs` passes.
3. **`platform.rs`.** Implement the three `#[cfg]`-gated `native_binary_filename` bodies. Observable: `platform_naming.rs` passes for the host's own OS.
4. **`trust.rs`.** Implement `check_trusted` (linear scan, case-insensitive `eq_ignore_ascii_case` comparison, `mod_id` must also match). Observable: `trust_allowlist.rs` passes.
5. **`discovery.rs`.** Implement `discover`: `std::fs::read_dir` (return `Ok((vec![], vec![]))` on `ErrorKind::NotFound`, propagate any other `io::Error` as `ModHostBootError::ModsDirIo`), filter `*.rcmod`, open each via `zip::ZipArchive::new(std::fs::File::open(..)?)`, read `manifest.toml`'s bytes, `String::from_utf8`, `rc_mod_api::parse_manifest` then `validate_manifest`. Observable: `manifest_discovery.rs` passes.
6. **`dependency_order.rs`.** Implement `resolve_load_order` via Kahn's algorithm exactly as M0-B05's `compute_waves` already establishes the technique (build the incompatibility — here, dependency — edge set; repeatedly drain in-degree-0 nodes; anything left unprocessed once no more in-degree-0 nodes exist is part of a cycle, reported as `DependencyCycle`), plus a `MissingDependency`/`DependencyFailed` cascade pass before the topological step (any mod whose declared dependency is not in the successfully-discovered set is excluded and reported before the graph is even built, and every mod transitively depending on an excluded one is excluded too, fixed-point iterated until no further exclusion occurs). Observable: `dependency_order.rs` passes.
7. **`handshake.rs`.** Implement `run_handshake`: `unsafe { library.get::<rc_mod_api::AbiHandshakeFn>(rc_mod_api::ABI_HANDSHAKE_SYMBOL.as_bytes()) }`, map `Err` through unchanged, else call the resolved function (`unsafe`, `// SAFETY:` citing MOD-D3/D21's ABI contract). Observable: exercised end-to-end by step 9's `entry_factory.rs`/`host.rs` work; no standalone test file of its own beyond what `handshake_matrix.rs` already covers through the full pipeline.
8. **`entry_factory.rs`.** Implement `load_server_entry`/`load_client_entry`: resolve the named symbol via `library.get::<ServerEntryFactoryFn>(..)`/`ClientEntryFactoryFn`, then call it wrapped in `crate::isolation::call_guarded` (step 9), mapping a caught panic to `EntryLoadError::Panicked`. Observable: compiles; exercised by step 10.
9. **`isolation.rs`.** Implement `call_guarded` (`std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))`, mapping `Err(payload)` to `CaughtPanic` via a small `downcast_ref::<&str>`/`downcast_ref::<String>` helper, falling back to the fixed placeholder string for any other payload type). Observable: unit-testable in isolation (a trivial always-panics closure), though this blueprint names no dedicated test file for it beyond what `crash_isolation.rs` already proves end-to-end.
10. **`host.rs`.** Implement `ServerModHost::discover_and_load` as the full pipeline (Context, Deliverables' own doc comment: discovery -> dependency order -> per-mod platform/extraction/trust/load/handshake/entry), building one `ModLoadDiagnostic` per discovered archive regardless of outcome; implement every `call_on_*` method as the `status`-check-then-`call_guarded`-then-status-update pattern (Context: "Crash isolation"); implement `ClientModHost` identically, resolving `[entrypoints].client`/the current triple's `client` flag instead of `server`. Observable: `handshake_matrix.rs`, `entry_loading_and_dispatch.rs`, `crash_isolation.rs` all pass.
11. **`crates/mod-api/src/entrypoint.rs` (the additive `rc-mod-api` edit).** Add the private fields/derives/constructors exactly as specified in Context/Deliverables to `RegistryBuildContext`, `RecordedRegistrations`, `ClientRegistryBuildContext`, `ClientRegistration`, and the four marker contexts' trivial constructors; update `crates/mod-api/src/lib.rs`'s `pub use entrypoint::{...}` line to add `ClientRegistration`, `RecordedRegistrations`. Observable: `entrypoint_context_construction.rs` passes; `cargo build -p rc-mod-api --all-features` still succeeds with zero `todo!()` remaining anywhere in this crate.
12. **`double_fault_subprocess.rs`'s child-process harness.** Add the `--double-fault-child-scenario` argument-recognition branch to the test binary's own entry (a `#[test]`-adjacent, harness-level addition — Rust integration-test binaries do not have a conventional `main` an implementer overrides directly; the standard technique is a `#[ctor]`-free, top-of-file `fn main()` override via `harness = false` in `Cargo.toml`'s `[[test]]` table for this one test binary specifically, or an equivalent early-argument-check inside the test's own `#[test]` function that re-execs itself — implementer's choice of mechanism, the *observable behavior* in Acceptance tests is what is binding, not the mechanism). Observable: `double_fault_subprocess.rs` passes.
13. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` (against this blueprint's own restated dependency set, Context), `-- test` — all exit 0.
14. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50) — `double_fault_subprocess.rs` and every dylib-loading test must pass identically on both, since MOD-D26/D32's own trust and isolation model makes no platform exception.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, with the same named, bounded exception class M8-B01 already established: the six fixture crates under `tests/fixtures/` and the `--double-fault-child-scenario` re-exec mechanism's own exact wiring detail (step 12's "implementer's choice of mechanism") are the only places this blueprint's own test-authoring changeset content is partly mechanical/discovered rather than fully pre-specified prose — no other test file, test case, or assertion anywhere in Acceptance tests may be added, removed, renamed, or weakened by the implementation changeset. The one additive `rc-mod-api` edit is itself governed by *that* crate's own Constraints (a) exactly as M8-B01 fixed them — this blueprint's edit is a permitted, named continuation of a deliberately-left-open completion, not a violation of M8-B01's own test-first boundary, since it adds new fields/methods `entrypoint_context_construction.rs`'s own new test file governs, never touching any of M8-B01's already-passing test files.

(b) **No new external dependencies beyond the pinned `[workspace.dependencies]` set.** `libloading`, `zip`, `stabby`, `parking_lot`, `thiserror`, `tracing` are all already pinned; `rc-mod-host` is simply their new consumer (Context: "Reconciling `rc-mod-host`'s dependency set"). In particular, **no SHA-256 crate is added** — SHA-256 is hand-rolled (Context, matching M0-B08's own established precedent) — and **no `tempfile` crate is added** — `CARGO_TARGET_TMPDIR` (a standard Cargo-provided environment variable for integration-test binaries, no dependency required) is this blueprint's own scratch-directory mechanism throughout.

(c) **No Mojang or third-party reimplementation code.** Every algorithm here (SHA-256 — a public, standardized, non-Mojang cryptographic algorithm published as FIPS 180-4, not a copied implementation; Kahn's-algorithm dependency ordering; the catch_unwind isolation design) is derived solely from `docs/planning/06-modding-api.md`'s MOD-D1–D32, this blueprint's own prerequisite M8-B01, and this blueprint's own concrete, cited resolutions of what those decisions leave open (ASSET-D18/D19/D30). No dylib-loading crate other than `libloading` (already pinned, MOD-D2's own text framing it as the mechanism) is consulted or considered.

(d) **`unsafe` code is permitted only where `libloading`'s and `stabby`'s own APIs require it.** `Library::new`, `Library::get::<T>`, and calling a resolved function pointer are all `unsafe` by `libloading`'s own design (Context, verified) — every such call site carries a `// SAFETY:` comment citing the specific invariant relied on (the ABI handshake having already succeeded before any other symbol is trusted; PERF-D46's workspace-wide `panic = "unwind"` requirement; the `LoadedServerMod`/`LoadedClientMod` field-declaration-order drop invariant, Deliverables' own `host.rs` doc comment). No other use of `unsafe` is permitted anywhere in this blueprint's Deliverables.

(e) **Feature-gate discipline.** `rc-mod-api` is pulled with `default-features = false, features = ["native-tier"]` only — this crate never references any `wasm-tier`-only item (`rc_mod_api::guest::*`), proven by `cargo build -p rc-mod-host` succeeding with that exact feature selection (not `--all-features`, which would silently permit an accidental `wasm-tier` reference to compile unnoticed).

(f) **Scope boundary — do not implement beyond this blueprint's stated Deliverables.** This blueprint does not implement: the WASM tier's `wasmtime` embedding in any form (Context: "WASM-tier deferral" — owner: a future, not-yet-numbered `rc-mod-host` blueprint); any translation of a mod's recorded registrations, block behaviors, or declared hook-access sets into real `bevy_ecs`/`rc-mechanics`/`rc-chunk-storage` types, or any integration with `rc-scheduler`'s `RcExecutorBuilder`/conflict graph (Context: "Where this blueprint's boundary actually falls" — owner: a future `rc-scheduler` blueprint, the only crate with the necessary dependency reach); any resource-limit enforcement for native-tier mods beyond MOD-D26's hash allowlist (Context: "Resource limits" — 06's own deliberate, disclosed non-enforcement, not a gap this blueprint fills); hot reload in any form (MOD-D28's own binding "no native-tier hot reload" — this crate never calls `Library`'s own unload path during normal operation, for any mod, disabled or active, until process exit); a server-config-file schema for `mods_dir`/`[mods.native.trusted]`/`mod_fault_policy` (`ModHostConfig` is a plain, already-parsed Rust value this crate consumes — owning a TOML/config-file schema for it is a future composition-root blueprint's job); real dependency-range *satisfaction* checking (Context, MOD-D31 — this crate checks only that a named dependency exists and that no cycle exists, exactly matching M8-B01's own already-named deferral, never silently "fixed" here). Every `*Context` type this blueprint completes in `rc-mod-api` stays exactly as functionally minimal as Context's own "recording, not live" design states — no field, method, or behavior beyond what is specified above is added under any circumstance.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-mod-host -p rc-mod-api --all-features
cargo build -p rc-mod-host --no-default-features
cargo nextest run -p rc-mod-host -p rc-mod-api
cargo test --doc -p rc-mod-host -p rc-mod-api
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-mod-host -p rc-mod-api` runs all 7 (`entrypoint_context_construction.rs`) + 3 (`sha256_vectors.rs`) + 3 (`platform_naming.rs`) + 5 (`manifest_discovery.rs`) + 5 (`dependency_order.rs`) + 4 (`trust_allowlist.rs`) + 5 (`handshake_matrix.rs`) + 5 (`entry_loading_and_dispatch.rs`) + 5 (`crash_isolation.rs`) + 1 (`double_fault_subprocess.rs`) + 1 (`wasm_tier_deferral.rs`) = 44 test cases named in Acceptance tests — all pass, with zero flakiness (fixture dylibs are built once per test binary and cached, Context — no test rebuilds a fixture it has already built in the same process). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
