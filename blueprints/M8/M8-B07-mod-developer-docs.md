# M8-B07 — Mod Developer Documentation: Infrastructure, Rot-Proofing, and the M8 Curriculum

| Field | Content |
|---|---|
| ID | M8-B07 |
| Milestone | M8 — Mod API Alpha |
| Prerequisites | M8-B01 (`rc-mod-api`'s complete public surface — every type/trait/function this blueprint's chapters and `examples/` crates cite is restated below exactly as M8-B01 shipped it: `Identifier`/`ModId`, the manifest schema (`ModManifest`/`parse_manifest`/`validate_manifest`), `DomainGroup`/`TickPriority`/`AccessKind`/`ComponentAccessDecl`/`HookDecl`/`HookOrderRef`/`NativeDomainMarker`, `ModAbiVersion`/`MOD_API_VERSION`/`ABI_HANDSHAKE_SYMBOL`, `ComponentDescriptorBuilder`/`ModComponentDescriptor`, `DenseIdAllocator`, `BlockRegistration`/`ItemRegistration`/`ModBlockStateId`/`ModItemId`, `ModBlockBehavior`/`ModUpdateContext`, `ServerModEntry`/`ClientModEntry`/`RegistryBuildContext`/`ClientRegistryBuildContext`/every marker `*Context`). M8-B02 (`rc-mod-host`'s public surface, consulted as explanatory context only — this blueprint's own `examples/` crates are never dylib-loaded through `ServerModHost` by this blueprint's own tests; restated where a chapter's prose needs to describe what loading *would* do). M8-B04 (`mods/example-ores`'s own already-resolved `stabby::dynptr!`/entry-factory construction syntax — every `examples/` crate in this blueprint mirrors that already-settled pattern verbatim rather than re-opening it as a moderate-confidence flag; `crates/mechanics/tests/water_override_replace.rs`'s prerequisite chain is unaffected, cited only by M8-B06a below). M8-B06a (`rc-mechanics`'s `crates/mechanics/tests/water_override_replace.rs` — Chapter 7's own worked example, linked directly, never duplicated, per MOD-D50's own binding curriculum; `resolve_override_order`/`OverrideMode`/`OverrideOrder`/`ModBlockBehaviorWrap`/`ModOriginalBlockBehavior`/`RegistryBuildContext::override_block_behavior_replace`/`_wrap`/`BlockBehaviorRegistry::register_named_range`/`resolve_named`/`override_named_range`/`active_overrides`/`RcExecutorBuilder::register_named_system`/`disable_named_system`/`replace_named_system`/`RcExecutor::active_system_overrides`, restated below). M8-B06b (`rc-mod-api`'s `EventPriority`/`BlockBreakAttempt`/`EventDispatcher<E>`/`ModEventListener`/`RegistryBuildContext::register_block_break_attempt_listener`, `ModComponentEntry`/`ModComponentsTag`/`encode_mod_components`/`decode_mod_components`, `ModEntityId`/`ModChunkKey`, and `rc-mechanics`'s `resolve_chunk_entity`/`resolve_block_entity`, restated below). |
| Implements | MOD-D47 (documentation ships as three mechanically-verified parts — rustdoc reference, mdBook guide, `examples/` crates — realized in full by this blueprint); MOD-D48 (the anchor-based rot-proofing mechanism — `{{#include}}`/`{{#rustdoc_include}}` bound to `// ANCHOR:`/`// ANCHOR_END:` pairs in real, CI-built `examples/` source, plus the `trybuild` negative-example mechanism — implemented, with its own dedicated checker); MOD-D49 (mdBook 0.5.4 pinned as an external CLI tool, `docs/mod-guide/` layout, the `-L`-flagged `xtask doc-guide build`/`test` pair, repo-served static-artifact publishing); MOD-D50 (the fixed 15-chapter curriculum — every M8-landing chapter authored); MOD-D51 (Tier-1 CI placement for all four verification steps; the honest protected-path scoping — `examples/` source and `docs/mod-guide/` prose are ordinary, implementation-changeset-editable content, never `TEST-D46`-protected; the flagship-example link to `mods/example-ores`); MOD-D52 (M8's own documentation scope split — infrastructure plus every chapter whose native-tier mechanism M8 itself ships; the binding definition-of-done rule, realized as a machine-checkable manifest checker). TEST-D37 (Tier-1 membership, restated verbatim below — already names this blueprint's own four verification steps by name in `09-testing-quality.md`'s current text); TEST-D40 (uniform machine-readable tier output — this blueprint's own `xtask` verbs follow the identical `CaseResult`/exit-code contract every other tier already uses); TEST-D43 (Windows/Linux parity — every verb below is proven on both CI legs); TEST-D45/D46 (test-first changeset boundary, restated and applied to a docs-shaped deliverable for the first time in this milestone — Context explains the one necessary adaptation); TEST-D50 (CI is the sole authority on completion). |
| Crates touched | `docs/mod-guide/` (new — mdBook source corpus: `book.toml`, `src/*.md`). `examples/01-getting-started/`, `examples/03-blocks/`, `examples/04-items/`, `examples/05-systems/`, `examples/06-events/`, `examples/08-components/`, `examples/10-networking/`, `examples/12-isomorphic/{shared,server,client}/`, `examples/13-testing/` (new — 11 real Cargo crates, every one a `[workspace]` member of the **main** Rusty Clanker workspace, per `12`'s own Interfaces-section extension). `crates/mod-api/` (`rc-mod-api`, modify: `src/lib.rs` gains `#![deny(missing_docs)]`; every public item gains a `///` summary and, where a standalone value is constructible, a runnable doctest; `Cargo.toml` gains one new `[dev-dependencies]` line, `trybuild`; new `tests/ui/wrong_native_entrypoint_signature.rs` + generated `.stderr`; new `tests/examples_manifest_conformance.rs`). `xtask/` (new: `src/doc_check.rs`, `src/doc_guide/mod.rs`, `src/doc_guide/anchors.rs`, `src/doc_guide/manifest.rs`, `src/doc_guide/build.rs`; modify `src/lib.rs`, `src/main.rs`, both additive; new `tests/doc_check.rs`, `tests/doc_guide_anchors.rs`, `tests/doc_guide_manifest.rs`, `tests/doc_guide_build.rs`). `Cargo.toml` (repo root, modify — additive `[workspace] members` entries for the 11 new crates, one additive `[workspace.dependencies]` line for `trybuild`). `.gitignore` (modify — one additive line, `/docs/mod-guide/book/`). |
| Estimated scope | L — a deliberate, cited sizing exception matching M8-B01/B02/B04/B06a's own established precedent. This is the blueprint that turns MOD-D47–D52's on-paper "documentation cannot rot" mandate into an actually-enforced CI gate: the mdBook/rustdoc/`examples/` infrastructure, the anchor and definition-of-done checkers, and the M8-scope curriculum content are one coherent, cross-referencing unit — splitting the infrastructure from the content it verifies would leave either half unable to prove anything on its own. |

## Goal & Done definition

Ship the mod developer documentation as a real, CI-gated deliverable, never a discretionary afterthought: the mdBook 0.5.4 book at `docs/mod-guide/` (MOD-D49) with its full 15-chapter navigation structure; the anchor-based rot-proofing mechanism (MOD-D48) as a real, self-tested `xtask doc-guide verify-anchors` checker binding every guide code block to a live, named anchor in a real `examples/` crate; the `examples/` workspace (MOD-D48) with one small, complete, `.rcmod`-shaped, compiling-and-tested mod crate per M8-landing capability chapter; `#![deny(missing_docs)]` plus `RUSTDOCFLAGS="-D warnings"` and a runnable-doctest audit over `rc-mod-api`'s complete public surface (MOD-D47); and the binding definition-of-done manifest (MOD-D52) as a fourth, real `xtask doc-guide verify-manifest` checker proving every M8-shipped capability has both a chapter and a passing, tested `examples/` entry. Every M8-landing chapter (MOD-D50's own fixed curriculum, filtered to what M8 itself ships per MOD-D52) is written in full; Chapters 1 and 2 are specified verbatim in this blueprint's own Deliverables for direct transcription; every other M8-landing chapter is specified as a binding outline plus exact example anchors plus the exact `rc-mod-api`/`rc-mechanics` item each section must cite, precise enough that an implementer writes a correct chapter without inventing an API claim. Chapters 9 (Custom World/Chunk Data, MOD-D43/D44's mechanism does not exist yet) and 11 (Client-Side, lands at `M10`, PLAN-D2) ship as short, honest, explicitly-dated stub pages — present in the fixed navigation MOD-D50 requires, never silently omitted, never claiming content that is not yet true.

Done when:

- [ ] `cargo build --workspace --all-features` succeeds with zero warnings, including all 11 new `examples/` crates as ordinary workspace members.
- [ ] `cargo nextest run --workspace` passes in full, including every new `examples/*/tests/*.rs` file, `crates/mod-api/tests/examples_manifest_conformance.rs`, and every new `xtask/tests/*.rs` file.
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc -p rc-mod-api --all-features --no-deps` exits 0 — `#![deny(missing_docs)]` finds nothing undocumented, and no intra-doc link is broken.
- [ ] `cargo test --doc -p rc-mod-api --all-features` exits 0 — every doctest added by this blueprint (and every one already shipped) runs and passes; every `no_run` doctest is exactly one this blueprint's own Context names as unable to run standalone, never a silently-broken one.
- [ ] `cargo test --test ui -p rc-mod-api` (the `trybuild` harness) exits 0 — `tests/ui/wrong_native_entrypoint_signature.rs`'s committed `.stderr` matches the real compiler diagnostic exactly.
- [ ] `cargo run -p xtask -- doc-guide build` produces `docs/mod-guide/book/index.html` with zero errors.
- [ ] `cargo run -p xtask -- doc-guide test` exits 0 — every `{{#rustdoc_include}}`-anchored slice in every chapter compiles and runs as a real doctest against the built `examples/` crates.
- [ ] `cargo run -p xtask -- doc-guide verify-anchors` exits 0 against the real, committed book, and its own self-tests (Acceptance tests) prove a guide block referencing a moved/deleted/renamed anchor is caught, named precisely, and fails the command.
- [ ] `cargo run -p xtask -- doc-guide verify-manifest` exits 0 against the real, committed book and `examples/` tree, and its own self-tests prove a missing chapter file, a missing backing `examples/` entry, and a missing decision-ID citation are each caught and named precisely — the definition-of-done rule (MOD-D52) mechanically enforced.
- [ ] `cargo run -p xtask -- doc-check` exits 0, wrapping the `RUSTDOCFLAGS`/`cargo doc`/`cargo test --doc` gates above into one uniform, `CaseResult`-shaped tier verb (TEST-D40); its own self-test proves the mechanism (a scratch, throwaway crate with one undocumented public item) is genuinely caught by `#![deny(missing_docs)]`.
- [ ] A dedicated self-test proves an `examples/` crate whose source fails to compile fails the docs gate — realized as an ordinary `cargo build --manifest-path` failure against a scratch copy, mirroring M8-B02/M8-B04's own established fixture-mutation-testing technique.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo run -p xtask -- lint-deps` exits 0 — the one new dependency this blueprint adds, `trybuild` (already named and version-pinned by MOD-D48 itself), is the only addition; no crate gains an unpinned or undeclared dependency.
- [ ] CI tier: Tier 1 (`09-testing-quality.md`'s own current TEST-D37 text already names every one of this blueprint's four doc-verification steps as Tier-1 members by name — this blueprint is what makes that text true rather than aspirational) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37/D43), on a clean checkout (TEST-D50).

## Context (self-contained)

### MOD-D47–D52, restated exactly enough to build against without opening `06` itself

**MOD-D47 — three mechanically-verified parts.** (a) A complete rustdoc reference over every public `rc-mod-api` item, gated by `#![deny(missing_docs)]` on the crate root plus `RUSTDOCFLAGS="-D warnings"` in CI; every public item carries at least a one-line summary, and every item whose correct use isn't obvious from its signature alone carries at least one runnable doctest wherever a value of the documented type is standalone-constructible; a doctest for the small remainder of items with no standalone-constructible value is `no_run`, documented as such, never silently absent. (b) The Mod Developer Guide, an mdBook book. (c) `examples/`, a set of complete, buildable, workspace-member example mod crates every guide code block is mechanically extracted from, never hand-copied.

**MOD-D48 — the rot-proofing mechanism, restated as this blueprint's own binding contract.** `examples/<NN>-<slug>/` holds one small, focused, real mod crate per curriculum chapter, each a genuine `manifest.toml`+`Cargo.toml` workspace member built by the standard `cargo build --workspace`/`cargo test --workspace` Tier-1 run — never a snippet living only inside a Markdown fence. A guide chapter's `.md` file never pastes example code directly; every shown code block is pulled from the real file via mdBook's `{{#include}}`/`{{#rustdoc_include}}` directives, anchored to named `// ANCHOR: <name>` / `// ANCHOR_END: <name>` comment pairs in the example's own source — a moved, renamed, or deleted anchor fails the `mdbook build`/`mdbook test` invocation immediately, never silently rendering stale text. `{{#rustdoc_include}}` is used for every anchor meant to also compile-check under `mdbook test`: it shows only the pedagogically-relevant slice in the rendered page while feeding the *whole* surrounding file to rustdoc's own doctest runner, every non-shown line auto-prefixed `#` exactly as an ordinary rustdoc example already hides setup lines — the binding authoring rule is that every anchor boundary aligns with a complete syntactic unit (a whole function or impl block), never a mid-expression cut. `trybuild` 1.0.120 (dtolnay's crate, crates.io, dual `MIT OR Apache-2.0`) is scoped narrowly to one negative worked example: a native-tier entrypoint function whose signature does not match the ABI-stable shape `#[stabby::export]` requires is, today, a genuine `rustc`/`stabby` compile error with no new mechanism needed — a `crates/mod-api/tests/ui/*.rs` + matching `*.stderr` pair pins that exact diagnostic so the guide's "here is what happens when you get the signature wrong" callout quotes real, CI-verified compiler output rather than a hand-typed approximation. `trybuild` is never used for manifest/TOML-level validation failures — those are `validate_manifest`'s own runtime diagnostic class, illustrated in the guide as documented error tables and direct `parse_manifest`/`validate_manifest` test output instead.

**MOD-D49 — the toolchain, layout, and publishing.** mdBook `0.5.4`, installed as a pinned **external CLI tool** (`cargo install mdbook --locked --version 0.5.4`) — never a `[workspace.dependencies]` entry, so its MPL-2.0 license never enters `cargo-deny`'s license gate over the shipped dependency graph. Guide source lives at `docs/mod-guide/` (`book.toml` + `src/*.md`, sibling to `docs/planning/`/`docs/research/`); its build output (`docs/mod-guide/book/`) is git-ignored, regenerated by `xtask doc-guide build`/`xtask doc-guide test` wrapping `mdbook build`/`mdbook test -L <path>`, never committed. Hosting: a repo-served static artifact — the built book is a CI-produced artifact, versioned per tagged engine release; docs.rs is explicitly not applicable (this project's current phase publishes no crate to crates.io).

**MOD-D50 — the fixed 15-chapter curriculum, restated as the binding table below (Deliverables' own "Curriculum table").** Short conceptual pseudocode is permitted only for explaining a concept with no working code shape yet; every code block demonstrating something the engine actually does is real, CI-verified source, never the reverse.

**MOD-D51 — quality gates.** All four doc-verification steps (`cargo doc --no-deps` under `#![deny(missing_docs)]`+`RUSTDOCFLAGS`, `cargo test --doc -p rc-mod-api`, `examples/`'s own `cargo build --workspace`/`cargo test --workspace` membership, `xtask doc-guide test`) run in Tier 1 (PR-blocking). `examples/`'s own crate source and `docs/mod-guide/`'s own chapter prose are ordinary, implementation-changeset-editable content, never `TEST-D46`-protected — only `crates/mod-api/tests/ui/*.rs`/`*.stderr` (the `trybuild` fixtures) fall under `TEST-D46`'s existing "any crate's `tests/` directory" clause. The published guide is rebuilt and republished on every tagged engine release. `mods/example-ores` (M8-B04) is the flagship worked example, linked from the guide's own landing page and every chapter whose capability it exercises; `examples/`'s own per-chapter crates stay each chapter's primary, minimal, single-concept teaching example — the two never duplicate each other's role.

**MOD-D52 — milestone placement and the binding definition-of-done rule.** `M8` alpha ships the full documentation infrastructure plus Getting Started, Core Concepts, and every per-capability chapter whose underlying mechanism `M8` itself ships natively. Custom World/Chunk Data lands its own chapter once MOD-D43/D44's mechanism exists to write a real example against — explicitly out of `M8`'s own shipped scope. **Binding rule: an API capability is not done until its chapter and its tested `examples/` entry both exist and pass Tier 1 CI.** Client-Side (Chapter 11) lands with `M10` unconditionally.

### The one necessary adaptation to TEST-D45's test-first rule, named explicitly

TEST-D45 requires acceptance tests "authored and committed in their own dedicated test-authoring changeset before the corresponding implementation task starts." Every prior M8 blueprint applies this to *engine mechanism* under test, with fixture/content crates (`mods/example-ores`, M8-B02's dylib fixtures) shipping **complete, real, working source in the test-authoring changeset itself** — "these are test *inputs*, not implementation" (M8-B02's own phrase, reused verbatim by M8-B04). This blueprint is the first whose entire deliverable *is* content (chapter prose, example crates) rather than an engine mechanism a content fixture merely exercises. The binding resolution, extending the identical precedent rather than inventing a new one: **every `examples/` crate's source, every `manifest.toml`, and every chapter `.md` file ships complete in the test-authoring changeset** (Acceptance tests, below) — exactly as `mods/example-ores` did — because a chapter or an example crate has no "body to stub"; only the four `xtask` checker functions (`doc_check::run`, `doc_guide::anchors::verify`, `doc_guide::manifest::verify`, `doc_guide::build::{build,test}`) have logic to `todo!()`-stub, and only those function bodies are the implementation changeset's own content.

### Directory layout, fixed by this blueprint

```
docs/
  mod-guide/
    book.toml
    src/
      00-introduction.md
      01-getting-started.md
      02-core-concepts.md
      03-blocks-and-behaviors.md
      04-items.md
      05-custom-systems-and-ordering-anchors.md
      06-events.md
      07-override-and-wrap-vanilla.md
      08-components-and-persistence.md
      09-custom-world-chunk-data.md
      10-mod-networking-channels.md
      11-client-side.md
      12-isomorphic-packaging.md
      13-testing-your-mod.md
      14-publishing-versioning-and-abi-compatibility.md
      15-migration-notes-policy.md
      SUMMARY.md
    book/                          # git-ignored, xtask doc-guide build output
examples/
  01-getting-started/   {Cargo.toml, manifest.toml, src/lib.rs, tests/glow_pebble.rs}
  03-blocks/             {Cargo.toml, manifest.toml, src/lib.rs, tests/mirror_pane.rs}
  04-items/              {Cargo.toml, manifest.toml, src/lib.rs, tests/registry_content.rs}
  05-systems/            {Cargo.toml, manifest.toml, src/lib.rs, tests/audit_tick.rs}
  06-events/             {Cargo.toml, manifest.toml, src/lib.rs, tests/cancellation.rs}
  08-components/         {Cargo.toml, manifest.toml, src/lib.rs, tests/persistence.rs}
  10-networking/         {Cargo.toml, manifest.toml, src/lib.rs, tests/channel.rs}
  12-isomorphic/
    shared/              {Cargo.toml, src/lib.rs}
    server/              {Cargo.toml, manifest.toml, src/lib.rs}
    client/              {Cargo.toml, manifest.toml, src/lib.rs}
  13-testing/            {Cargo.toml, manifest.toml, src/lib.rs, tests/block_behavior_test.rs, tests/registry_content_test.rs, tests/manifest_test.rs}
```

Every `examples/<NN>-<slug>/` crate is added as its own flat entry in the **repo-root** `Cargo.toml`'s `[workspace] members` array (`12`'s own Interfaces-section text already names this extension as required; this blueprint realizes it) — never a nested virtual-manifest workspace like `mods/example-ores` (M8-B04), which deliberately sits *outside* the main workspace via Cargo's nearest-ancestor-`Cargo.toml` discovery. `examples/12-isomorphic/` is this blueprint's one three-crate example (Context, "Chapter 12," below) — since a directory containing a `[workspace]`-table `Cargo.toml` cannot itself be listed as a main-workspace member path (Cargo rejects "added as a member, but it's a virtual manifest, not a package"), this blueprint's own binding resolution, extending MOD-D48's per-crate framing to the one case it did not spell out, is three flat member entries — `examples/12-isomorphic/shared`, `examples/12-isomorphic/server`, `examples/12-isomorphic/client` — with no `Cargo.toml` at `examples/12-isomorphic/` itself at all.

### Every `examples/` crate's common shape

Native tier only (M8's own native-first scoping, restated identically by every prior M8 blueprint). Each crate: `crate-type = ["cdylib", "rlib"]` (`cdylib` is the real `.rcmod`-packageable artifact MOD-D4 names; `rlib` lets `cargo test -p <crate>` link the crate directly for its own unit tests — M8-B04's own established rationale, reused verbatim); depends on `rc-mod-api = { path = "../../crates/mod-api", default-features = false, features = ["native-tier"] }` and `stabby = { workspace = true }`; a sibling `manifest.toml` that is a real, `parse_manifest`/`validate_manifest`-passing `.rcmod` manifest (native tier, one or two committed triples — `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`, matching this project's own CI matrix, TEST-D34, exactly mirroring `mods/example-ores`'s own convention); an `unsafe extern "C" fn rc_mod_abi_handshake() -> ModAbiVersion { MOD_API_VERSION }` handshake export; one `#[stabby::export] extern "C" fn <mod_id>_server_entry() -> stabby::dynptr!(stabby::boxed::Box<dyn ServerModEntry>)` entry-factory export. **Every `dynptr!`/`stabby::closure` construction in this blueprint's own Deliverables mirrors `mods/example-ores/server/src/lib.rs`'s own already-resolved, already-compiling pattern exactly** (M8-B04's own Prerequisites) — this blueprint opens no new moderate-confidence flag for a question M8-B04 already settled by the time this blueprint is implemented.

Every `examples/` crate's own tests construct `RegistryBuildContext::new(0, 0)` / `ModUpdateContext::new(...)` / marker `*Context::new()` values **directly** — no dylib load, no `ServerModHost`, mirroring `mods/example-ores/server/tests/*.rs`'s own already-established, already-proven convention exactly. This is a deliberate, honest scope boundary (Context, "Chapter 13," below): this blueprint proves every example's own logic correct and every manifest well-formed, never that a real running server loads it — that remains `M8-B05`'s own already-named, still-open composition-root gap, restated by every M8 blueprint including this one.

### The M8 example set, table form

| `examples/` dir | Mod id | Chapter | Demonstrates |
|---|---|---|---|
| `01-getting-started` | `hello_block` | 1 | One block + `on_scheduled_tick`, the minimal registry-build-to-proof loop |
| `03-blocks` | `blocks_demo` | 3 | `on_neighbor_changed`/`on_shape_update`, `get_block`/`set_block` |
| `04-items` | `items_demo` | 4 | `register_item`, `ItemRegistration` |
| `05-systems` | `systems_demo` | 5 | `[[hooks]]`, `native:<domain>` ordering anchor, `[[capabilities.components]]` |
| `06-events` | `events_demo` | 6 | `EventDispatcher`, `ModEventListener`, cancel/observe |
| `08-components` | `components_demo` | 8 | `register_component`, `ModComponentsTag` round-trip |
| `10-networking` | `networking_demo` | 10 | `network_channels`, `register_channel`, `on_channel_message`/`on_mod_message` |
| `12-isomorphic/{shared,server,client}` | `isomorphic_demo` | 12 | The shared/server/client split, the ABI handshake, the entry-factory export, the `trybuild` negative example |
| `13-testing` | `testing_demo` | 13 | The hand-built-context testing technique itself, as this blueprint's own worked cookbook |

Chapter 7 (Override & Wrap Vanilla) has **no** `examples/` entry — it links `crates/mechanics/tests/water_override_replace.rs` (M8-B06a) directly, per MOD-D50's own binding curriculum text naming this reuse explicitly ("since it is already a real, CI-proven end-to-end example rather than one the guide would have to invent"). Chapters 2, 14, 15 are prose-only (no backing example — Chapter 14 links `crates/mod-api/tests/abi_handshake.rs`, already real and already passing). Chapters 9 and 11 are honest stub pages (Context, below).

### The anchor mechanism, restated concretely — how a guide code block is bound to a CI-built example

A chapter `.md` file embeds a directive line of the shape `{{#rustdoc_include ../../../examples/<dir>/src/lib.rs:<anchor>}}` (relative from `docs/mod-guide/src/`, three levels up to the repo root, then into `examples/`) or `{{#include ...}}` for non-Rust content (manifest TOML, which cannot be doctested — Context, "trybuild," above). The referenced file carries a matching `// ANCHOR: <anchor>` / `// ANCHOR_END: <anchor>` comment pair around a complete syntactic unit. Two independent things check this binding: (1) `mdbook build`/`mdbook test` itself fails immediately if the referenced file or anchor does not exist — MOD-D48's own baseline guarantee; (2) this blueprint's own `xtask doc-guide verify-anchors` is a **second, purpose-built, faster, more specific** checker: it scans every `docs/mod-guide/src/*.md` file for every `{{#include}}`/`{{#rustdoc_include}}` directive whose target has a *named* (non-numeric) anchor suffix, resolves the referenced path relative to the including file, and verifies the target file contains both a `// ANCHOR: <name>` and a later `// ANCHOR_END: <name>` line for that exact name — reporting every violation found (collect-all, never fail-fast, matching `validate_manifest`'s own established discipline) with the exact chapter file, line, directive text, and reason (file missing / `ANCHOR` missing / `ANCHOR_END` missing / mismatched nesting). Running this checker standalone (no `mdbook`/rustdoc invocation, pure filesystem + string scanning) is what makes "a guide block without a live anchor fails" a fast, precisely-diagnosed, CI-native property rather than only an indirect consequence of a slower `mdbook build` failure.

```rust
// xtask/src/doc_guide/anchors.rs (new)

/// One `{{#include}}`/`{{#rustdoc_include}}` directive found in a chapter file,
/// with a named (non-numeric) anchor suffix — numeric line-range includes
/// (`file.rs:10:20`) and whole-file includes (no `:suffix` at all) are out of this
/// checker's own scope (Context) and are skipped, never flagged.
pub struct AnchorDirective {
    pub chapter_file: std::path::PathBuf,
    pub directive_line: usize,
    pub target_path: std::path::PathBuf,
    pub anchor: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AnchorViolation {
    #[error("{chapter_file}:{directive_line}: {{{{#include {target_path}:{anchor}}}}} references a file that does not exist")]
    TargetFileMissing { chapter_file: std::path::PathBuf, directive_line: usize, target_path: std::path::PathBuf, anchor: String },
    #[error("{chapter_file}:{directive_line}: {{{{#include {target_path}:{anchor}}}}} — no `// ANCHOR: {anchor}` line found in {target_path}")]
    AnchorStartMissing { chapter_file: std::path::PathBuf, directive_line: usize, target_path: std::path::PathBuf, anchor: String },
    #[error("{chapter_file}:{directive_line}: {{{{#include {target_path}:{anchor}}}}} — `// ANCHOR: {anchor}` found but no matching `// ANCHOR_END: {anchor}` after it")]
    AnchorEndMissing { chapter_file: std::path::PathBuf, directive_line: usize, target_path: std::path::PathBuf, anchor: String },
}

/// Scans every `*.md` file directly under `book_src_dir` (non-recursive — every
/// chapter file lives flat in `docs/mod-guide/src/`, Context) for anchor-suffixed
/// `{{#include}}`/`{{#rustdoc_include}}` directives.
pub fn find_directives(book_src_dir: &std::path::Path) -> std::io::Result<Vec<AnchorDirective>>;

/// Resolves and checks every directive found; returns every violation, never
/// fail-fast (Context). `Ok(())` iff the returned `Vec` would otherwise be empty.
pub fn verify(book_src_dir: &std::path::Path) -> Result<(), Vec<AnchorViolation>>;
```

### The definition-of-done manifest — the machine-checkable chapter↔example↔decision mapping

MOD-D52's binding rule ("a capability is not done until its chapter and its tested `examples/` entry both exist and pass Tier 1 CI") is realized as a hard-coded table (the curriculum is "the curriculum, not a suggestion," MOD-D50) plus a checker that verifies every M8-landing row against the real, committed filesystem and chapter text.

```rust
// xtask/src/doc_guide/manifest.rs (new)

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Landing {
    /// Ships in full at M8 — this row's `backing` and `decisions` are enforced.
    M8,
    /// Present as a short, honest stub page at M8; full content is a future
    /// milestone's job. Enforced only to exist and to carry the literal marker
    /// `<!-- STATUS: deferred -->` (Context, "Chapters 9 and 11").
    Deferred { until: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Backing {
    /// A real `examples/<dir>/` crate must exist with `Cargo.toml`, `manifest.toml`,
    /// and `src/lib.rs` all present. For Chapter 12 (the one multi-crate
    /// example, Context: "Chapter 12's `trybuild` negative example" and
    /// "Directory layout"), `dir` names the one sub-crate that actually carries
    /// a `manifest.toml` (`"12-isomorphic/server"`) — sibling `shared`/`client`
    /// crates are real workspace members covered unconditionally by the
    /// ordinary `cargo build --workspace` gate, not by this per-chapter check.
    Example { dir: &'static str },
    /// A specific, already-real, already-passing test file this chapter links
    /// directly rather than duplicating (Chapter 7, Chapter 14).
    Cited { test_path: &'static str },
    /// No backing example needed — pure conceptual/prose content (Chapter 2,
    /// Chapter 15).
    Prose,
}

pub struct ChapterEntry {
    pub number: u8,
    pub title: &'static str,
    pub file: &'static str,          // relative to docs/mod-guide/src/
    pub landing: Landing,
    pub backing: Backing,
    /// MOD-D IDs this chapter must cite as a literal substring somewhere in its
    /// own rendered text (a citation-*presence* check — Context: "every API
    /// statement in a chapter must cite the rc-mod-api item it documents" is
    /// enforced at decision-ID granularity, the coarsest mechanically-checkable
    /// proxy for that rule this blueprint can verify without a full semantic
    /// read of the prose).
    pub decisions: &'static [&'static str],
}

/// The fixed, binding 15-chapter table (MOD-D50/D52), hard-coded here exactly as
/// Deliverables' own "Curriculum table" fixes it.
pub const CHAPTER_MANIFEST: &[ChapterEntry] = &[ /* Deliverables, verbatim */ ];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestViolation {
    #[error("chapter {number} ({title}): required file {file} does not exist or is empty")]
    ChapterFileMissingOrEmpty { number: u8, title: &'static str, file: &'static str },
    #[error("chapter {number} ({title}): deferred chapter file {file} is missing the required `<!-- STATUS: deferred -->` marker")]
    DeferredMarkerMissing { number: u8, title: &'static str, file: &'static str },
    #[error("chapter {number} ({title}): backing examples/{dir}/ is missing Cargo.toml, manifest.toml, or src/lib.rs")]
    BackingExampleIncomplete { number: u8, title: &'static str, dir: &'static str },
    #[error("chapter {number} ({title}): cited test path {test_path} does not exist")]
    CitedTestMissing { number: u8, title: &'static str, test_path: &'static str },
    #[error("chapter {number} ({title}): does not cite required decision {decision} anywhere in its own text")]
    DecisionNotCited { number: u8, title: &'static str, decision: &'static str },
    #[error("SUMMARY.md does not reference chapter {number} ({title})'s file {file} in the expected order")]
    SummaryOrderMismatch { number: u8, title: &'static str, file: &'static str },
}

/// Runs every check in Context above against `repo_root`; collects every
/// violation, never fail-fast.
pub fn verify(repo_root: &std::path::Path) -> Result<(), Vec<ManifestViolation>>;
```

### `missing_docs`/doctest gating, and the honest `rc-mod-test`/MOD-D29 gap

MOD-D47's own text names `rc-mod-test`'s (MOD-D29) mocked-host fixture as the intended backing for a doctest that needs a constructed host — but `rc-mod-test` is not shipped by any of this blueprint's own prerequisites (M8-B01/B02/B04/B06a/B06b) and remains, in every one of those blueprints' own Constraints, "a separate, later blueprint." **This blueprint does not add the `rc-mod-api → rc-mod-test` dev-dependency cycle MOD-D47 names** — the crate it would point at does not exist yet. This is not a blocker: every concrete type this blueprint's own audit finds in `rc-mod-api`'s public surface already carries a real, public, standalone constructor (`Identifier::new`/`parse`, `ModId::new`, `ModAbiVersion` as a plain struct literal, `ComponentDescriptorBuilder::new`, `DenseIdAllocator::starting_at`, `RegistryBuildContext::new`/`ClientRegistryBuildContext::new` (M8-B02's own completion), every marker `*Context::new()` (M8-B02), `ModUpdateContext::new` (M8-B04's own completion), `EventDispatcher::new`, `BlockBreakAttempt::new`, `ModComponentsTag::new`, `OverrideOrder::default` — M8-B06a/B06b's own additions) — so every one of these gets a real, running doctest with no host needed at all. Only the WIT-generated guest bindings (`rc_mod_api::guest`, `wasm-tier` feature — meant to be implemented as a Component-Model export, not called from ordinary host-side Rust) and the five mod-facing traits' own `impl` blocks where a full ABI-crossing call would be needed to observe anything, are `no_run` — documented as such per MOD-D47's own text, never a silent gap.

### Chapters 9 and 11 — the honest stub pages

Both are present in `SUMMARY.md`, in their fixed curriculum position (MOD-D50's own binding numbering — the navigation does not skip a number), each a short page: an `<h1>` matching the chapter's own title, immediately followed by the literal HTML comment `<!-- STATUS: deferred -->` (the manifest checker's own machine-readable marker, Context above), then two or three sentences stating precisely what mechanism is missing and which decision/milestone will supply it. Neither page claims example code, an anchor, or a capability that does not exist — Deliverables gives each verbatim.

### Chapter 12's `trybuild` negative example

`crates/mod-api/tests/ui/wrong_native_entrypoint_signature.rs` declares a `#[stabby::export]`-annotated `extern "C"` entry-factory function whose return type is a plain `std::boxed::Box<dyn rc_mod_api::ServerModEntry>` — the ordinary standard-library `Box`, not `stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ServerModEntry>)` — the one MOD-D3's own ABI-stable-boundary rule requires. `#[stabby::export]`'s own generated type-report verification (M8-B01's own cited rationale, "adds... a generated `<fn>_stabbied` type-report verification function") requires every exchanged type implement `stabby::abi::IStable`, which `std::boxed::Box`'s own plain form does not — this is a genuine, already-true `rustc`/`stabby` compile failure needing no new mechanism, exactly as MOD-D48 states. This blueprint does not pin the exact diagnostic text itself (trybuild's own real, standard workflow: run once against the installed toolchain, `trybuild` prints the actual compiler output and a suggested `.stderr` to commit) — Implementation steps name this explicitly as a "run once, commit what it produces" step, never a hand-typed guess.

### Curriculum table (MOD-D50/D52, restated as the binding source `CHAPTER_MANIFEST` mirrors byte-for-byte)

| # | Title | File | Lands | Backing | Primary decisions cited |
|---|---|---|---|---|---|
| — | Introduction | `00-introduction.md` | M8 | Prose | — |
| 1 | Getting Started | `01-getting-started.md` | M8 | `examples/01-getting-started` | MOD-D27 |
| 2 | Core Concepts | `02-core-concepts.md` | M8 | Prose | MOD-D8, MOD-D9, MOD-D33 |
| 3 | Blocks & Behaviors | `03-blocks-and-behaviors.md` | M8 | `examples/03-blocks` | MOD-D6, MOD-D8 |
| 4 | Items | `04-items.md` | M8 | `examples/04-items` | MOD-D6 |
| 5 | Custom Systems & Ordering Anchors | `05-custom-systems-and-ordering-anchors.md` | M8 | `examples/05-systems` | MOD-D8, MOD-D10 |
| 6 | Events | `06-events.md` | M8 | `examples/06-events` | MOD-D39 |
| 7 | Override & Wrap Vanilla | `07-override-and-wrap-vanilla.md` | M8 | Cited: `crates/mechanics/tests/water_override_replace.rs` | MOD-D33, MOD-D34, MOD-D35, MOD-D36, MOD-D37, MOD-D38 |
| 8 | Components on Vanilla Entities & Persistence | `08-components-and-persistence.md` | M8 | `examples/08-components` | MOD-D41, MOD-D42 |
| 9 | Custom World/Chunk Data | `09-custom-world-chunk-data.md` | Deferred (post-M8) | Prose (deferred stub) | MOD-D43, MOD-D44 |
| 10 | Mod Networking Channels | `10-mod-networking-channels.md` | M8 | `examples/10-networking` | MOD-D20 |
| 11 | Client-Side | `11-client-side.md` | Deferred (M10) | Prose (deferred stub) | MOD-D18 |
| 12 | Isomorphic Packaging & the One-Crate-Two-Targets Build | `12-isomorphic-packaging.md` | M8 | `examples/12-isomorphic/server` (+ sibling `shared`/`client`, Context) | MOD-D4, MOD-D5 |
| 13 | Testing Your Mod | `13-testing-your-mod.md` | M8 | `examples/13-testing` | MOD-D29 |
| 14 | Publishing, Versioning & ABI Compatibility | `14-publishing-versioning-and-abi-compatibility.md` | M8 | Cited: `crates/mod-api/tests/abi_handshake.rs` | MOD-D21, MOD-D22, MOD-D23, MOD-D26 |
| 15 | Migration Notes Policy | `15-migration-notes-policy.md` | M8 | Prose | MOD-D23 |

## Deliverables

### `Cargo.toml` (repo root, modify — additive only)

```toml
[workspace]
members = [
    "crates/*", "xtask",
    "examples/01-getting-started", "examples/03-blocks", "examples/04-items",
    "examples/05-systems", "examples/06-events", "examples/08-components",
    "examples/10-networking",
    "examples/12-isomorphic/shared", "examples/12-isomorphic/server", "examples/12-isomorphic/client",
    "examples/13-testing",
]

[workspace.dependencies]
# ...every pre-existing line unchanged, plus:
trybuild = "1.0.120"
```

### `.gitignore` (modify — one additive line)

Add `/docs/mod-guide/book/` to the existing pattern list; every existing line unchanged.

### `docs/mod-guide/book.toml` (new)

```toml
[book]
title = "Rusty Clanker Mod Developer Guide"
authors = ["Rusty Clanker Project"]
description = "Step-by-step guides, worked examples, and the API reference for writing Rusty Clanker mods."
language = "en"
src = "src"

[build]
build-dir = "book"
# A SUMMARY.md entry naming a chapter file that does not exist is a hard build
# error, never a silently-generated placeholder page (matches this whole
# corpus's "never silently render stale/missing text" discipline, extended here).
create-missing = false
```

### `docs/mod-guide/src/SUMMARY.md` (new)

```markdown
# Summary

[Introduction](./00-introduction.md)

- [Getting Started](./01-getting-started.md)
- [Core Concepts](./02-core-concepts.md)
- [Blocks & Behaviors](./03-blocks-and-behaviors.md)
- [Items](./04-items.md)
- [Custom Systems & Ordering Anchors](./05-custom-systems-and-ordering-anchors.md)
- [Events](./06-events.md)
- [Override & Wrap Vanilla](./07-override-and-wrap-vanilla.md)
- [Components on Vanilla Entities & Persistence](./08-components-and-persistence.md)
- [Custom World/Chunk Data](./09-custom-world-chunk-data.md)
- [Mod Networking Channels](./10-mod-networking-channels.md)
- [Client-Side: Models, Renderers, GUI, HUD, Input](./11-client-side.md)
- [Isomorphic Packaging & the One-Crate-Two-Targets Build](./12-isomorphic-packaging.md)
- [Testing Your Mod](./13-testing-your-mod.md)
- [Publishing, Versioning & ABI Compatibility](./14-publishing-versioning-and-abi-compatibility.md)
- [Migration Notes Policy](./15-migration-notes-policy.md)
```

### `docs/mod-guide/src/00-introduction.md` (new, verbatim)

````markdown
# Rusty Clanker Mod Developer Guide

Rusty Clanker is a from-scratch Rust reimplementation of the Minecraft: Java
Edition server, with an isomorphic modding API as one of its core pillars: you
write one mod crate, and the engine loads whichever side (server, client, or
both) actually applies.

This guide is one of three mechanically-verified parts of the mod API's own
documentation (MOD-D47):

- **This book** — step-by-step guides and worked examples, chapter by chapter.
- **The rustdoc reference** — generated from `rc-mod-api`'s own doc comments
  (`cargo doc -p rc-mod-api --open`), covering every public type, trait, and
  function precisely.
- **`examples/`** — a real, compiling, CI-tested Cargo workspace, one small
  crate per chapter. Every code block in this book is pulled directly from a
  file under `examples/` — never hand-copied — so a broken example breaks the
  build, not just the prose.

At `M8` alpha, the mod API's **native tier** is the only one hosted end to end
(a WASM/Component-Model tier is fully specified but not yet host-implemented —
every chapter below says so wherever it matters). Every worked example in this
book targets the native tier.

`mods/example-ores` is this project's own flagship reference mod — a real,
permanent, git-tracked mod exercising a broader slice of the API than any one
chapter's own small example does. Several chapters below link to it directly.

Start with [Getting Started](./01-getting-started.md).
````

### Chapter 1 — Getting Started (new, verbatim — transcribe directly)

````markdown
# Getting Started

By the end of this chapter you will have a real, compiling mod — one block
with real tick behavior — proven correct by a real, passing test. This is the
`examples/01-getting-started` crate; every code block below is pulled directly
from its own committed source.

## Scaffolding a mod crate

A `cargo generate` template (`rusty-clanker/mod-template`, MOD-D27) that
automates everything in this section is planned but not yet published — the
steps below are exactly what that template will eventually produce, and are
fully working today.

A native-tier mod is an ordinary Cargo crate:

```toml
# Cargo.toml
[package]
name = "hello-block"
version = "0.1.0"
edition = "2021"

[lib]
name = "hello_block"
crate-type = ["cdylib", "rlib"]

[dependencies]
rc-mod-api = { path = "../../crates/mod-api", default-features = false, features = ["native-tier"] }
stabby = "72.1.16"
```

`crate-type` carries `rlib` alongside `cdylib`: `cdylib` is the real
`.{dll,so,dylib}` binary the engine's mod loader (`rc-mod-host`) will
eventually open; `rlib` is what lets `cargo test` link your crate directly, so
your own tests never need to load a compiled dylib at all (Chapter 13 covers
this testing technique in full).

Every mod also carries a `manifest.toml` — the `.rcmod` manifest MOD-D4 fixes
the shape of:

```toml
{{#include ../../../examples/01-getting-started/manifest.toml}}
```

`[mod].id` is your mod's own bare identity (`hello_block` — no `:`, unlike
every other identifier in this manifest, which is always `namespace:path`).
`[entrypoints].tier = "native"` selects the native tier (the only tier hosted
at `M8`); `server = "hello_block_server_entry"` names the exported Rust
symbol the engine will call to obtain your mod's own `ServerModEntry`
instance. `[entrypoints.native."<triple>"]` tables name which platforms your
compiled binary actually supports — one entry per platform you ship for; this
project's own CI matrix covers exactly the two triples shown.

This manifest is real: `crates/mod-api/tests/examples_manifest_conformance.rs`
parses and validates it on every commit, exactly as it validates every other
`examples/` crate's own manifest.

## One block, one behavior

```rust
{{#rustdoc_include ../../../examples/01-getting-started/src/lib.rs:behavior}}
```

`ModBlockBehavior` mirrors the engine's own real, internal `BlockBehavior`
trait one-for-one (`rc_mod_api::ModBlockBehavior`) — every method has a
no-op default, so you implement only what your block actually needs.
`on_scheduled_tick` is called whenever the engine's Stage-4 scheduled-tick
pipeline reaches a block registered with this behavior. `ModUpdateContext`
(`rc_mod_api::ModUpdateContext`) is your only way to read or mutate block
state from inside a behavior callback — `set_block` is the *only* way a
behavior mutates block state, matching the engine's own real rule for native
block behaviors exactly.

## Registering it

```rust
{{#rustdoc_include ../../../examples/01-getting-started/src/lib.rs:entry}}
```

`RegistryBuildContext::register_block` (`rc_mod_api::RegistryBuildContext`)
allocates one new, dense `ModBlockStateId` and records your block's own id
(`"hello_block:glow_pebble"`) and its declared property-combination count.
`register_block_behavior` attaches your `GlowPebbleBehavior` to that exact
state id. Both calls happen inside `on_registry_build` — the one, one-shot,
boot-time phase every mod's registrations run inside (Chapter 2 explains why
this timing matters).

Every native mod also exports two fixed symbols the engine looks for by name:
the ABI version handshake (`rc_mod_abi_handshake`, checked before anything
else is trusted) and your entry-factory function (the name your manifest's
`[entrypoints].server` field names):

```rust
{{#rustdoc_include ../../../examples/01-getting-started/src/lib.rs:exports}}
```

## Proving it works

```rust
{{#rustdoc_include ../../../examples/01-getting-started/tests/glow_pebble.rs:test}}
```

`ModUpdateContext::new` gives you a real context backed by plain closures you
supply — no compiled dylib, no running server, no `rc-mod-host` involved at
all. This test calls `on_scheduled_tick` directly and asserts the fixture-log
side effect happened — the exact proof technique the reference mod
(`mods/example-ores`) and every other `examples/` crate in this guide use.

**What this chapter does *not* yet prove.** Loading `hello-block` into a real,
running `rusty-clanker-server` via a `--mods-dir` flag is not wired end to end
yet — that is a still-open, explicitly tracked composition-root gap (see
`M8-B05`'s own binding contract). What you *have* proven, with a real test
against real engine types, is that your block's behavior is correct — the
same proof the engine's own crash-isolation and registration-content tests
rely on for the reference mod itself.

Next: [Core Concepts](./02-core-concepts.md) — what actually happens between
`on_registry_build` and your tick hook firing.
````

### Chapter 2 — Core Concepts (new, verbatim — transcribe directly)

````markdown
# Core Concepts

This chapter has no code of its own — it explains the model every other
chapter's example already runs inside. Come back to it once something in a
later chapter feels surprising.

## You never manage threads

Rusty Clanker's server runs on a custom, multithreaded scheduler
(`RC-Executor`) that decides, once per boot, exactly which systems may run
concurrently, based on which ECS components each one declares it reads or
writes. This is `ARCH-D8`'s startup conflict graph, and — this is the whole
point of MOD-D33's "no special position for vanilla" rule — **your mod's own
tick hooks and block behaviors are ordinary participants in that identical
graph.** You never spawn a thread, take a lock, or decide when your code runs
relative to anyone else's; you declare *what data* you touch, and the
scheduler works out a safe, correct order for you, exactly as it already does
for every native engine system.

## Two ways your code runs

**Block behaviors** (Chapter 3) are the cheap, common case: a block's
`on_scheduled_tick`/`on_neighbor_changed`/etc. run inside Stage 4's
already-existing, already-sequential dispatch loop — registering one costs no
new scheduler entry at all.

**Generic tick hooks** (Chapter 5) are for logic that isn't tied to one block:
you declare, in your manifest, exactly which components you read and which
you write, and which of the engine's tick-domain groups you want to run in.
The engine turns that declaration into a real conflict-graph participant at
boot — this is why the declaration lives in `manifest.toml`, not in your Rust
code: the scheduler needs to know your access set *before* any of your code
ever runs, so it can compute a safe schedule once, up front, for the whole
run.

## Domains and stages

The engine's tick pipeline is divided into a fixed sequence of stages, grouped
into a small number of **domain groups** a mod may target:
`rc_mod_api::DomainGroup` currently has five variants —
`BlockRedstone`, `AiPhysics`, `Lighting`, `ChunkSerialize`, and `NetCodec` —
each mapping onto one point in the engine's own pipeline. `BlockRedstone`
carries one binding rule every mod hook inherits unconditionally: it is
always fully sequential, single-worker, for every mod and every native system
alike — redstone timing has no parallel axis anyone is allowed to opt into,
mods included.

## Declared access, and what it actually buys you (MOD-D8, MOD-D9)

Every `[[hooks]]` entry's `[[capabilities.components]]` declarations
(`{ hook, name, access, group }`) are what the scheduler resolves into a real
conflict-graph node. What that declaration *enforces* differs by tier — an
important, honestly-stated asymmetry: at the native tier (the only tier
hosted at `M8`), enforcement is **honesty-based** — your mod shares the
engine's own process, so a native mod that lies about its declared access can
corrupt engine state exactly as badly as a bug in engine code could. A future
WASM tier gets a stronger guarantee for free (the host, not the guest, ever
touches memory directly), but that tier is not hosted yet.

## Determinism duties are inherited, never opted out of

Three rules apply to your mod's code exactly as they apply to every native
engine system, with no smaller version for mods (MOD-D33's own three
invariants, restated for a mod-author audience): (1) whatever stage/domain
your code runs in, it keeps that stage's own concurrency contract — a
Stage-4 block behavior is always sequential, a chunk-parallel hook stays
chunk-parallel; (2) the scheduler alone ever decides what runs when — nothing
in this API hands you a raw thread or a manual lock; (3) every component you
register must be plain, self-contained data — no raw pointer, no engine
handle, nothing that would be invalid the instant its bytes are copied
elsewhere. This last rule (MOD-D13) is what lets your mod's state travel
automatically through region snapshots, saves, and (eventually) cluster
migration with zero mod-specific code on any of those paths — a component
that broke this rule would silently corrupt the moment any of them touched
it.

## Two entry traits, never one

A mod's server-side and client-side code are two **separate** Rust traits —
`rc_mod_api::ServerModEntry` and `rc_mod_api::ClientModEntry` — never one
trait with optional methods a loader "skips." The server process never even
opens your `manifest.toml`'s client entrypoint; a purely server-side mod
simply never implements `ClientModEntry` at all (Chapter 12 covers the full
shared/server/client split).

Next: pick the chapter matching what you want your mod to do —
[Blocks & Behaviors](./03-blocks-and-behaviors.md),
[Items](./04-items.md), or
[Custom Systems & Ordering Anchors](./05-custom-systems-and-ordering-anchors.md)
are the natural next steps after Getting Started.
````

### Chapters 3–15, specified as binding outlines

Each table row below is binding: every listed anchor must exist, verbatim-named, in the cited `examples/` source; every listed decision ID must be cited as a literal substring somewhere in the chapter's own rendered text (`xtask doc-guide verify-manifest` checks this mechanically); every listed API item is the *only* vocabulary the chapter may claim exists — an implementer must not invent a method, field, or manifest key beyond what this outline (cross-referenced against this blueprint's own Prerequisites restatement, Header) names.

**Chapter 3 — Blocks & Behaviors** (`examples/03-blocks`, mod id `blocks_demo`). Sections: "A block with two states," "Reacting to neighbors (`on_neighbor_changed`)," "Suggesting a shape update (`on_shape_update`)," "What `set_block` guarantees." Anchors: `examples/03-blocks/src/lib.rs:behavior` (the `MirrorPaneBehavior` impl), `:entry` (registration), `tests/mirror_pane.rs:test`. Cites: MOD-D6 (registry insertion — two `BlockRegistration` calls sharing one `id`, distinct `ModBlockStateId`s, exactly `hello_block`'s own one-state case widened), MOD-D8 (block-behavior registration is *not* a generic hook — no `[[hooks]]` entry, no conflict-graph node — restate M8-B01's own "strictly cheaper" rationale by name). API items: `ModBlockBehavior::on_neighbor_changed(&self, ctx, pos, from: ModDirection)`, `::on_shape_update(&self, ctx, pos, from, neighbor_state: ModBlockStateId) -> Option<ModBlockStateId>`, `ModUpdateContext::get_block`/`set_block`, `ModDirection` (six variants: `West, East, North, South, Down, Up`).

**Chapter 4 — Items** (`examples/04-items`, mod id `items_demo`). Sections: "Registering an item," "What you can't do yet." Anchors: `examples/04-items/src/lib.rs:entry`, `tests/registry_content.rs:test`. Cites: MOD-D6. API items: `RegistryBuildContext::register_item`, `ItemRegistration { id, max_stack_size }`, `ModItemId`. Binding honesty note the chapter must state: an item registered this way is **not obtainable by any in-game means at `M8` alpha** — no crafting, loot, or creative-inventory system exists yet for a mod to plug into (mirrors `mods/example-ores`'s own `pulse_shard` scoping exactly, cited by name).

**Chapter 5 — Custom Systems & Ordering Anchors** (`examples/05-systems`, mod id `systems_demo`). Sections: "Declaring a hook," "`before`/`after` and the `native:<domain>` anchor," "Declared component access," "Dispatch." Anchors: `examples/05-systems/manifest.toml` (whole-file `{{#include}}`, no anchor), `src/lib.rs:entry` (the `on_tick_hook` match), `tests/audit_tick.rs:test`. Cites: MOD-D8, MOD-D10. API items: `[[hooks]]`'s full field set (`id, group, priority, before, after, exclusive_world_access` — state plainly that `priority` is meaningful, and required, only for `group = "block_redstone"`, per M8-B01's own reconciliation of MOD-D11 against the real seven-level `TickPriority`), `[[capabilities.components]]`'s `{hook, name, access, group}` shape, `HookOrderRef`'s two forms (`"native:<domain>"` and another mod's own hook id, shown via the manifest's own literal `after = ["native:lighting"]` line), `ServerModEntry::on_tick_hook(&mut self, hook_id, ctx)`. The chapter must state `exclusive_world_access` exists and is a discouraged, logged, metriced opt-in (MOD-D12) without the example itself using it.

**Chapter 6 — Events** (`examples/06-events`, mod id `events_demo`). Sections: "The event catalog at `M8`," "Priority tiers and cancellation," "The monitor tier can observe but never mutate," "Registering a listener." Anchors: `examples/06-events/src/lib.rs:canceller`, `:observer`, `:entry` (the `register_block_break_attempt_listener` call), `tests/cancellation.rs:test` (the direct `EventDispatcher` wiring). Cites: MOD-D39. API items: `EventPriority` (six variants, `Highest` through `Lowest` plus `Monitor`, in that order), `BlockBreakAttempt::{new, pos, player_entity, block_state, cancel, uncancel, is_cancelled}`, `EventDispatcher::<E>::{new, register, fire, listener_count}`, `ModEventListener::on_block_break_attempt`, `RegistryBuildContext::register_block_break_attempt_listener`. Binding honesty note: `BlockBreakAttempt` is real and fully dispatchable, but is not yet wired to a real block-breaking call site anywhere in the engine (M8-B06b's own named, still-future integration point) — state this plainly, the same way Chapter 1 states the composition-root gap.

**Chapter 7 — Override & Wrap Vanilla** (no `examples/` entry — cites `crates/mechanics/tests/water_override_replace.rs` directly). Sections: "No special position for vanilla," "`Wrap` vs. `Replace`," "Call-original," "Cross-mod ordering and the double-`Replace` diagnostic," "Discovering active overrides," "The worked example: replacing `minecraft:water`." Cites: MOD-D33, MOD-D34, MOD-D35, MOD-D36, MOD-D37, MOD-D38. API items: `OverrideMode::{Wrap, Replace}`, `OverrideOrder { before, after }`, `RegistryBuildContext::override_block_behavior_replace`/`_wrap`, `ModOriginalBlockBehavior` (the call-original handle, one method per `ModBlockBehavior` method), `ModBlockBehaviorWrap`, `resolve_override_order` (state its return shape — `chain`/`truncated`/`rejected` — and MOD-D38's own rule for each), `BlockBehaviorRegistry::{register_named_range, resolve_named, override_named_range, active_overrides}` (named as `rc-mechanics`-internal, never called directly by mod code — a mod only ever calls `RegistryBuildContext::override_block_behavior_*`; these are the engine-side seam a future composition-root drains recorded overrides into), `RcExecutorBuilder::{register_named_system, disable_named_system, replace_named_system}` and `RcExecutor::active_system_overrides()` for the system-level tier (likewise `rc-scheduler`-internal, not a manifest field or a call a mod's own `on_registry_build` makes — a future composition-root/orchestrator blueprint is what reads a loaded mod's recorded override requests and calls these on the mod's behalf, mirroring M8-B03's own already-established "manifest declares, orchestrator translates" split for generic hooks; `active_system_overrides()` is the system-level counterpart to `active_overrides()` above, satisfying MOD-D34's discoverability requirement at the system-level tier exactly as `active_overrides()` satisfies it at the behavior-level tier; state this plainly, the same honesty register as every other engine-internal-seam note in this outline). Last section walks `water_override_replace.rs`'s own four test cases in prose (never re-pasting their code — link the file, name each test by its own function name and one-sentence purpose).

**Chapter 8 — Components on Vanilla Entities & Persistence** (`examples/08-components`, mod id `components_demo`). Sections: "Registering a component," "How it's saved: `ModComponents`," "What's not reachable yet." Anchors: `examples/08-components/src/lib.rs:entry`, `tests/persistence.rs:test`. Cites: MOD-D41, MOD-D42. API items: `ComponentDescriptorBuilder::{new, with_drop, mutable, build}`, `RegistryBuildContext::register_component`, `ModComponentId`, `ModComponentEntry { component, schema_version, raw_bytes }`, `ModComponentsTag::{new, set, get, live_entries}`, `encode_mod_components`/`decode_mod_components`. Binding honesty note (Context, "MOD-D42's engine-internal scope"): `rc_mechanics::mod_world_query::resolve_chunk_entity`/`resolve_block_entity` are real and tested but are `rc-mechanics`-internal — no native-tier mod can call into `rc-mechanics` directly, and `TickHookContext` remains a fieldless marker at `M8` with no live component-attach call of its own yet. State this as a named, tracked gap, not a working feature.

**Chapter 9 — Custom World/Chunk Data** (deferred stub, verbatim in Deliverables below).

**Chapter 10 — Mod Networking Channels** (`examples/10-networking`, mod id `networking_demo`). Sections: "Declaring a channel," "Sending and receiving," "What rides underneath." Anchors: `examples/10-networking/manifest.toml` (whole-file include), `src/lib.rs:entry` (`register_channel` plus both handler methods), `tests/channel.rs:test`. Cites: MOD-D20. API items: `[capabilities].network_channels`, `RegistryBuildContext::register_channel`, `ServerModEntry::{on_channel_message, on_mod_message}`, `ModAddress` (three variants: `Region(String)`, `Entity(u64)`, `Chunk { dimension, x, z }`). State plainly that this rides vanilla's own real Custom Payload packet — no new transport.

**Chapter 11 — Client-Side** (deferred stub, verbatim in Deliverables below).

**Chapter 12 — Isomorphic Packaging & the One-Crate-Two-Targets Build** (`examples/12-isomorphic/{shared,server,client}`, mod id `isomorphic_demo`). Sections: "The three-crate split," "Shared logic, proven identical on both sides," "The ABI handshake," "The entry-factory export," "What happens when the ABI shape is wrong." Anchors: `examples/12-isomorphic/shared/src/lib.rs:greeting`, `server/src/lib.rs:entry`, `client/src/lib.rs:entry`. Cites: MOD-D4, MOD-D5. API items: `[entrypoints]`'s full field set (`tier, shared, server, client, native.<triple>`), `ModAbiVersion { major, minor, patch }`, `MOD_API_VERSION`, `ModAbiVersion::is_compatible_with`, `ABI_HANDSHAKE_SYMBOL`. Last section walks `crates/mod-api/tests/ui/wrong_native_entrypoint_signature.rs`'s own committed `.stderr` in prose — quote the real, committed diagnostic text directly (`{{#include}}` the `.stderr` file itself, whole-file, no anchor needed — a `.stderr` file has no Rust syntax to anchor a slice of).

**Chapter 13 — Testing Your Mod** (`examples/13-testing`, mod id `testing_demo`). Sections: "Three things worth testing," "Testing a block behavior," "Testing registry-build content," "Testing your manifest," "What `rc-mod-test` will eventually add." Anchors: `examples/13-testing/tests/block_behavior_test.rs:test`, `tests/registry_content_test.rs:test`, `tests/manifest_test.rs:test`. Cites: MOD-D29. Binding honesty note (Context, "the honest `rc-mod-test`/MOD-D29 gap"): state plainly that `rc-mod-test`'s own mocked-host convenience harness (MOD-D29) is planned but not yet built, and that the hand-built-context technique this chapter teaches is not a workaround — it is the real, complete testing story at `M8`, and the exact mechanism `rc-mod-test` will eventually wrap for ergonomics, not replace.

**Chapter 14 — Publishing, Versioning & ABI Compatibility** (no `examples/` entry — cites `crates/mod-api/tests/abi_handshake.rs` directly). Sections: "SemVer, and how it's checked," "Deprecation windows," "Unstable features," "What's enforced today vs. planned." Cites: MOD-D21, MOD-D22, MOD-D23, MOD-D26. API items: `ModAbiVersion::is_compatible_with`'s own exact rule (same major; mod's minor `<=` engine's minor), `MOD_API_VERSION`, `[api].unstable_features`. Binding honesty note: `[api].unstable_features` is a real, validated manifest field today; the WIT `@since`/`@unstable`/`@deprecated` gates and the native `#[rc_mod_api::unstable(feature = "...")]` attribute macro MOD-D22 describes are the planned enforcement mechanism and are **not yet implemented** by any merged blueprint — state this as a named, tracked gap.

**Chapter 15 — Migration Notes Policy** (prose, a living index — verbatim starter content in Deliverables below).

### Chapter 9 stub (new, verbatim)

````markdown
# Custom World/Chunk Data

<!-- STATUS: deferred -->

This chapter documents region-scoped mod data (MOD-D43) and world-scoped mod
data (MOD-D44) — a reserved singleton entity per region, and a single
authoritative global value mirrored per-region, respectively. Neither
mechanism is implemented yet: MOD-D43 needs a per-region bootstrap singleton
spawn no merged blueprint has built, and MOD-D44 needs `05-game-mechanics.md`'s
own GameRules mechanism (`MECH-D64`), which is itself still unbuilt.

This chapter will be written, with a real worked example under `examples/`,
once a future blueprint ships one of these two mechanisms. Until then, see
[Components on Vanilla Entities & Persistence](./08-components-and-persistence.md)
for the persistence story that *is* real today.
````

### Chapter 11 stub (new, verbatim)

````markdown
# Client-Side: Models, Renderers, GUI, HUD, Input

<!-- STATUS: deferred -->

The client-side extension points (MOD-D18: `register_model_provider`,
`register_block_renderer`, `register_gui_screen`, `register_hud_overlay`,
`register_input_binding` — `rc_mod_api::ClientRegistryBuildContext`) are real
and already **registration-and-headless-verified**: `mods/example-ores`'s own
client entry calls `register_block_renderer` today, and its registration is
proven by a real, passing test. What does not exist yet is a renderer to draw
anything — Rusty Clanker's Phase 2 client has not been built.

This chapter documents the full, working client-side rendering/GUI/input
story once Phase 2's client renderer exists and can prove `mods/example-ores`'s
own block actually renders — the milestone that unblocks this is `M10`.
Until then, every client-side extension point's own contract is documented in
the rustdoc reference (`ClientRegistryBuildContext`, `ClientModEntry`) even
though this book's own worked walkthrough is not yet written.
````

### Chapter 15 starter content (new, verbatim)

````markdown
# Migration Notes Policy

This page is a living index of every mod-API-visible change, in chronological
order — never a changelog of the engine itself, and never rewritten to erase
history (this project's own "current-state-only" documentation rule applies
to every *other* page in this corpus; this one page is the deliberate,
named exception, because a mod author upgrading across versions needs the
history this page exists to preserve).

An entry marked `@deprecated(since = X.Y.Z)` remains fully functional for at
least two mod-API minor versions or six months, whichever is longer, before it
may be removed — and removal only ever happens at the next mod-API **major**
version bump (MOD-D23).

## `0.1.0` — initial release

The complete `M8`-alpha native-tier surface: manifest schema and
parser/validator, the `stabby` ABI boundary and version handshake, registry
insertion (blocks, items, components, channels), block-behavior registration,
generic tick hooks with declared access and ordering anchors, the cancellable
event layer, behavior- and system-level override/replace/disable, and the
`ModComponents` persistence encoding. No item in this release is yet marked
`@deprecated`.
````

### `examples/01-getting-started/manifest.toml` (new)

```toml
[mod]
id = "hello_block"
version = "0.1.0"
display_name = "Hello Block"
authors = ["Rusty Clanker Mod Guide"]
license = "MIT OR Apache-2.0"

[api]
requires = "^0.1"

[entrypoints]
tier = "native"
server = "hello_block_server_entry"

[entrypoints.native."x86_64-pc-windows-msvc"]
server = true

[entrypoints.native."x86_64-unknown-linux-gnu"]
server = true

[capabilities]
```

### `examples/01-getting-started/Cargo.toml`, `src/lib.rs`, `tests/glow_pebble.rs` (new)

`Cargo.toml`: exactly Chapter 1's own shown TOML (Deliverables, above), package `hello-block`, lib `hello_block`.

```rust
// examples/01-getting-started/src/lib.rs
//! `hello_block` — Chapter 1's own worked example: one block, one behavior,
//! proven with a real unit test.

use rc_mod_api::{
    BlockRegistration, ModAbiVersion, ModBlockBehavior, ModBlockPos, ModBlockStateId,
    ModHookError, ModInitError, ModUpdateContext, RegistryBuildContext, ServerModEntry,
    TickHookContext, MOD_API_VERSION,
};

// ANCHOR: behavior
/// `hello_block:glow_pebble`'s own tick behavior: on every scheduled tick, it
/// writes one line to the fixture log path named by
/// `HELLO_BLOCK_FIXTURE_LOG_PATH`, if set.
struct GlowPebbleBehavior;

impl ModBlockBehavior for GlowPebbleBehavior {
    fn on_scheduled_tick(&self, _ctx: &mut ModUpdateContext, pos: ModBlockPos) {
        if let Ok(path) = std::env::var("HELLO_BLOCK_FIXTURE_LOG_PATH") {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(f, "glow_pebble ticked at {pos:?}");
            }
        }
    }
}
// ANCHOR_END: behavior

// ANCHOR: entry
#[derive(Default)]
struct HelloBlockServerEntry;

impl ServerModEntry for HelloBlockServerEntry {
    fn on_registry_build(&mut self, ctx: &mut RegistryBuildContext) -> stabby::result::Result<(), ModInitError> {
        let state = ctx.register_block(BlockRegistration {
            id: "hello_block:glow_pebble".into(),
            default_state_component_count: 1,
        });
        ctx.register_block_behavior(state, /* mirrors mods/example-ores/server/src/lib.rs's own already-resolved dynptr!(Box::new(GlowPebbleBehavior)) construction */);
        stabby::result::Result::Ok(())
    }

    fn on_tick_hook(&mut self, _hook_id: &stabby::string::String, _ctx: &mut TickHookContext) -> stabby::result::Result<(), ModHookError> {
        stabby::result::Result::Ok(())
    }
}
// ANCHOR_END: entry

// ANCHOR: exports
#[stabby::export]
extern "C" fn rc_mod_abi_handshake() -> ModAbiVersion { MOD_API_VERSION }

#[stabby::export]
extern "C" fn hello_block_server_entry() -> stabby::dynptr!(stabby::boxed::Box<dyn ServerModEntry>) {
    /* mirrors mods/example-ores/server/src/lib.rs's own already-resolved entry-factory construction */
}
// ANCHOR_END: exports
```

```rust
// examples/01-getting-started/tests/glow_pebble.rs
//! Proves `GlowPebbleBehavior::on_scheduled_tick` fires — no dylib, no
//! `ServerModHost`, mirroring `mods/example-ores/server/tests/pulse_crystal_behavior.rs`'s
//! own already-established convention exactly.

// ANCHOR: test
#[test]
fn on_scheduled_tick_logs_a_line() {
    // Constructs a `ModUpdateContext` via its own public `new` constructor
    // (M8-B04), backed by simple recording closures. `get_block`/`set_block`/
    // `schedule_block_tick`/`schedule_fluid_tick`/`emit_block_event` are unused
    // by this behavior, so every closure here is a no-op stand-in.
    let mut ctx = rc_mod_api::ModUpdateContext::new(/* five no-op closures + current_tick: 0 */);
    let tmp = std::env::temp_dir().join(format!("hello_block_test_{}.log", std::process::id()));
    std::env::set_var("HELLO_BLOCK_FIXTURE_LOG_PATH", &tmp);
    let behavior = /* the crate's own GlowPebbleBehavior, constructed directly — this test lives in the same crate's `tests/` tree and links it via the crate's own `rlib` output */;
    rc_mod_api::ModBlockBehavior::on_scheduled_tick(&behavior, &mut ctx, rc_mod_api::ModBlockPos { x: 0, y: 64, z: 0 });
    let contents = std::fs::read_to_string(&tmp).unwrap();
    assert!(contents.contains("glow_pebble ticked"));
    let _ = std::fs::remove_file(&tmp);
}
// ANCHOR_END: test
```

### `examples/03-blocks/…` through `examples/13-testing/…` (new)

Every crate below shares `examples/01-getting-started`'s own `Cargo.toml`/manifest-table shape exactly (Context, "Every `examples/` crate's common shape") — only the package/lib name, mod id, and the interesting logic differ; each is listed here as the delta from that established pattern.

**`examples/03-blocks`** (package `blocks-demo`, lib `blocks_demo`, mod id `blocks_demo`). `BlockRegistration` twice under one id (`"blocks_demo:mirror_pane"`, `default_state_component_count: 2`) for `off`/`on` states, mirroring `mods/example-ores`'s own `pulse_crystal` two-call convention exactly.

```rust
// examples/03-blocks/src/lib.rs (relevant excerpt)
// ANCHOR: behavior
struct MirrorPaneBehavior { off: ModBlockStateId, on: ModBlockStateId }

impl ModBlockBehavior for MirrorPaneBehavior {
    fn on_neighbor_changed(&self, ctx: &mut ModUpdateContext, pos: ModBlockPos, _from: ModDirection) {
        // A minimal, illustrative reaction: flip to "on" whenever any neighbor changes.
        ctx.set_block(pos, self.on);
    }

    fn on_shape_update(&self, _ctx: &mut ModUpdateContext, _pos: ModBlockPos, _from: ModDirection, neighbor_state: ModBlockStateId) -> stabby::option::Option<ModBlockStateId> {
        // Suggests staying "on" only if the neighbor's own state is this
        // block's own "on" state too — an illustrative connecting-block shape.
        if neighbor_state == self.on {
            stabby::option::Option::Some(self.on)
        } else {
            stabby::option::Option::Some(self.off)
        }
    }
}
// ANCHOR_END: behavior
```

`tests/mirror_pane.rs` proves both methods via `ModUpdateContext::new` exactly as Chapter 1's own test does — one case per method, asserting the returned/mutated state matches the two branches above.

**`examples/04-items`** (package `items-demo`, lib `items_demo`, mod id `items_demo`). `on_registry_build` calls only `ctx.register_item(ItemRegistration { id: "items_demo:polished_shard".into(), max_stack_size: 16 })`. `tests/registry_content.rs` mirrors `mods/example-ores/server/tests/registry_build_recording.rs`'s own convention: construct `RegistryBuildContext::new(0, 0)` directly, call `on_registry_build`, assert `into_recorded().items` has exactly one entry with the expected id/stack size.

**`examples/05-systems`** (package `systems-demo`, lib `systems_demo`, mod id `systems_demo`). `manifest.toml` gains:

```toml
[[capabilities.components]]
hook = "systems_demo:audit_tick"
name = "rc_engine_test:pulse_flag"
access = "read"
group = "lighting"

[[hooks]]
id = "systems_demo:audit_tick"
group = "lighting"
before = []
after = ["native:lighting"]
exclusive_world_access = false
```

`on_tick_hook` checks `hook_id.as_str() == "systems_demo:audit_tick"` and logs, mirroring `mods/example-ores`'s own `pulse_survey` body exactly. `tests/audit_tick.rs` calls `on_tick_hook` directly with both the matching and a non-matching `hook_id`, asserting the log gains a line only for the match — mirroring `crates/mod-host/tests/entry_loading_and_dispatch.rs`'s own dispatch-proof convention structurally, without any dylib.

**`examples/06-events`** (package `events-demo`, lib `events_demo`, mod id `events_demo`).

```rust
// examples/06-events/src/lib.rs (relevant excerpt)
// ANCHOR: canceller
struct Canceller;
impl rc_mod_api::ModEventListener for Canceller {
    fn on_block_break_attempt(&self, event: &mut rc_mod_api::BlockBreakAttempt) {
        event.cancel();
    }
}
// ANCHOR_END: canceller

// ANCHOR: observer
struct Observer { seen_cancelled: std::sync::Arc<std::sync::atomic::AtomicBool> }
impl rc_mod_api::ModEventListener for Observer {
    fn on_block_break_attempt(&self, event: &mut rc_mod_api::BlockBreakAttempt) {
        self.seen_cancelled.store(event.is_cancelled(), std::sync::atomic::Ordering::SeqCst);
    }
}
// ANCHOR_END: observer

// ANCHOR: entry
// inside on_registry_build:
ctx.register_block_break_attempt_listener(rc_mod_api::EventPriority::Highest, /* dynptr!(Box::new(Canceller)) */);
// ANCHOR_END: entry
```

`tests/cancellation.rs` builds a real `rc_mod_api::EventDispatcher<BlockBreakAttempt>` directly (never through the `dynptr!`-boxed registration path — `EventDispatcher::register` takes a plain `Box<dyn Fn(&mut E) + Send + Sync>`, so the test wraps `Canceller`/`Observer`'s own trait methods in ordinary closures), registers `Canceller` at `Highest` and `Observer` at `Monitor`, fires one constructed `BlockBreakAttempt`, and asserts `seen_cancelled` reads `true` — the cancellation-visible-to-later-tiers property, at this example's own small scale. A second test proves registration recording separately: `RegistryBuildContext::new(0,0)` + `on_registry_build` + `into_recorded().event_listeners.len() == 1`.

**`examples/08-components`** (package `components-demo`, lib `components_demo`, mod id `components_demo`). `on_registry_build`:

```rust
let descriptor = rc_mod_api::ComponentDescriptorBuilder::new("components_demo:visit_counter", 4, 4).unwrap().build().unwrap();
let _id = ctx.register_component(descriptor);
```

`tests/persistence.rs` has two cases: (1) registration recording (mirrors `examples/04-items`'s own convention); (2) a standalone `ModComponentsTag` round-trip — `ModComponentEntry { component: Identifier::parse("components_demo:visit_counter").unwrap(), schema_version: 1, raw_bytes: 7u32.to_le_bytes().to_vec() }`, `set` onto a fresh `ModComponentsTag`, `encode_mod_components` then `decode_mod_components`, asserting the round trip is byte-exact — the persistence story made concrete for this exact component.

**`examples/10-networking`** (package `networking-demo`, lib `networking_demo`, mod id `networking_demo`). `manifest.toml` gains `network_channels = ["networking_demo:chat_relay"]`. `on_registry_build` calls `ctx.register_channel(Identifier::parse("networking_demo:chat_relay").unwrap())`. `on_channel_message`/`on_mod_message` both log their received `payload` via the fixture-log convention. `tests/channel.rs` calls both handler methods directly on a bare `NetworkingDemoServerEntry` value with distinct payload bytes, asserting both log lines appear with the exact bytes round-tripped — mirroring `crates/mod-host/tests/entry_loading_and_dispatch.rs` test 3's own assertion shape, without a dylib.

**`examples/12-isomorphic/shared`** (package `isomorphic-demo-shared`, lib `isomorphic_demo_shared`, plain lib crate, no `rc-mod-api` dependency, no manifest.toml — mirrors `mods/example-ores/shared`'s own role exactly):

```rust
// examples/12-isomorphic/shared/src/lib.rs
// ANCHOR: greeting
/// The one piece of logic both sides call — proving isomorphism means proving
/// the *same compiled function* runs on both sides, not merely that both
/// crates happen to depend on identical source.
pub fn greeting() -> &'static str {
    "hello from isomorphic_demo"
}
// ANCHOR_END: greeting
```

**`examples/12-isomorphic/server`** (mod id `isomorphic_demo`, manifest `[entrypoints] tier = "native", shared = "shared", server = "isomorphic_demo_server_entry", client = "isomorphic_demo_client_entry"`, depends on `isomorphic-demo-shared = { path = "../shared" }` alongside `rc-mod-api`). `on_server_init` calls `isomorphic_demo_shared::greeting()` and logs it via the fixture-log convention.

**`examples/12-isomorphic/client`** (mirrors `server`'s own Cargo.toml shape, no manifest.toml of its own — `mods/example-ores`'s own precedent: one manifest, shared by both sides, lives only under the server crate's directory in this simplified layout; this blueprint's own binding simplification, since `examples/`'s own crates are never packaged into a real `.rcmod` archive by this blueprint's own tests, unlike `mods/example-ores`). `on_client_init` calls the identical `isomorphic_demo_shared::greeting()` and logs it under the same fixture-log convention.

`examples/12-isomorphic/server`'s own `tests/` (or a workspace-level integration test under `server/tests/isomorphism.rs`) asserts: calling `ExampleServerEntry::on_server_init` and, separately, `ExampleClientEntry::on_client_init` (from `client`'s own crate — reached via a `[dev-dependencies]` path edge from `server`'s own `Cargo.toml` back onto the `client` crate, the one exception to "examples/ crates depend only on rc-mod-api/stabby," justified exactly as `mods/example-ores`'s own cross-crate proof needs it) both log the identical string — proving the *same compiled logic* ran on both sides, mirroring `mods/example-ores`'s own `next_pulse_event` cross-check test exactly.

**`examples/13-testing`** (package `testing-demo`, lib `testing_demo`, mod id `testing_demo`). A minimal mod: one block (`testing_demo:sample_block`, one state, a trivial `on_scheduled_tick` logging a line) plus one item (`testing_demo:sample_item`). Its own `tests/` directory *is* this chapter's real content:

- `tests/block_behavior_test.rs` — `ModUpdateContext::new(...)` direct construction, testing the block's own tick logic (mirrors `pulse_crystal_behavior.rs`). Its own top-of-file doc comment states: *"This is the complete testing story for a block behavior at M8: construct a `ModUpdateContext` with recording closures, call the method directly, assert on what was recorded. No dylib, no running server, no mocked-host crate needed."*
- `tests/registry_content_test.rs` — `RegistryBuildContext::new(0, 0)` direct construction, testing registered content (mirrors `registry_build_recording.rs`).
- `tests/manifest_test.rs` — reads this crate's own committed `manifest.toml` text and asserts `parse_manifest`/`validate_manifest` both succeed (mirrors `mod_reference_template_conformance.rs` test 1).

### `crates/mod-api/tests/examples_manifest_conformance.rs` (new)

```rust
//! Every `examples/` crate's own `manifest.toml` is a real, validating `.rcmod`
//! manifest — proven once, here, for every example, rather than once per example
//! crate's own test tree (avoiding nine near-identical copies of the same check).

const EXAMPLE_MANIFESTS: &[(&str, &str)] = &[
    ("01-getting-started", include_str!("../../../examples/01-getting-started/manifest.toml")),
    ("03-blocks", include_str!("../../../examples/03-blocks/manifest.toml")),
    ("04-items", include_str!("../../../examples/04-items/manifest.toml")),
    ("05-systems", include_str!("../../../examples/05-systems/manifest.toml")),
    ("06-events", include_str!("../../../examples/06-events/manifest.toml")),
    ("08-components", include_str!("../../../examples/08-components/manifest.toml")),
    ("10-networking", include_str!("../../../examples/10-networking/manifest.toml")),
    ("12-isomorphic/server", include_str!("../../../examples/12-isomorphic/server/manifest.toml")),
    ("13-testing", include_str!("../../../examples/13-testing/manifest.toml")),
];

#[test]
fn every_example_manifest_parses_and_validates() {
    for (name, text) in EXAMPLE_MANIFESTS {
        let manifest = rc_mod_api::parse_manifest(text)
            .unwrap_or_else(|e| panic!("examples/{name}/manifest.toml failed to parse: {e}"));
        rc_mod_api::validate_manifest(&manifest)
            .unwrap_or_else(|errs| panic!("examples/{name}/manifest.toml failed to validate: {errs:?}"));
    }
}
```

### `crates/mod-api/Cargo.toml` (modify — one additive line)

```toml
[dev-dependencies]
proptest = { workspace = true }
trybuild = { workspace = true }
```

### `crates/mod-api/tests/ui/wrong_native_entrypoint_signature.rs` (new)

```rust
// A native-tier entry-factory function using a plain std::boxed::Box instead of
// stabby::dynptr!(stabby::boxed::Box<dyn ...>) — MOD-D3's ABI-stable-boundary
// rule requires the latter; #[stabby::export]'s own generated type-report
// verification is what actually rejects this (Context, "Chapter 12's trybuild
// negative example").
#[stabby::export]
extern "C" fn bad_entry() -> Box<dyn rc_mod_api::ServerModEntry> {
    unimplemented!()
}

fn main() {}
```

### `crates/mod-api/tests/ui.rs` (new — the `trybuild` harness entry point)

```rust
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/wrong_native_entrypoint_signature.rs");
}
```

`tests/ui/wrong_native_entrypoint_signature.stderr` is **generated, not hand-authored** (Implementation steps) — `trybuild`'s own standard workflow: run once with `TRYBUILD=overwrite cargo test --test ui -p rc-mod-api`, inspect the produced `.stderr`, commit it.

### `crates/mod-api/src/lib.rs` (modify — one additive attribute)

```rust
#![deny(missing_docs)]
//! ...existing crate-root doc comment, unchanged...
```

### `xtask/src/doc_check.rs` (new)

```rust
use crate::tier_result::CaseResult;

/// Runs `RUSTDOCFLAGS="-D warnings" cargo doc -p rc-mod-api --all-features --no-deps`
/// (the `#![deny(missing_docs)]` + intra-doc-link gate, MOD-D47) and
/// `cargo test --doc -p rc-mod-api --all-features` (every doctest, including
/// every `no_run` one — a `no_run` doctest still compile-checks). Returns one
/// `CaseResult` per sub-check, never conflating the two failure classes.
pub fn run() -> Vec<CaseResult>;
```

### `xtask/src/doc_guide/mod.rs`, `anchors.rs`, `manifest.rs`, `build.rs` (new)

`anchors.rs`/`manifest.rs`: exactly as specified in Context above. `build.rs`:

```rust
use crate::tier_result::CaseResult;

/// Checks `mdbook --version` reports `0.5.4` on PATH before doing anything else
/// — a missing or wrong-version `mdbook` fails loudly with an actionable
/// install command (`cargo install mdbook --locked --version 0.5.4`), never a
/// confusing "command not found."
pub fn check_mdbook_version() -> Result<(), String>;

/// `mdbook build docs/mod-guide`.
pub fn build() -> CaseResult;

/// `cargo build --workspace` (ensuring every `examples/` crate's rlib exists),
/// then `mdbook test -L <target-dir>/debug/deps docs/mod-guide` (MOD-D49's own
/// already-fixed `-L` contract, restated).
///
/// Moderate-confidence flag, re-verify at implementation time: mdBook 0.5.4's
/// own exact `-L`/`--extern` resolution behavior for a 2021-edition
/// `use rc_mod_api::...;` path (as opposed to a legacy `extern crate
/// rc_mod_api;` declaration) should be confirmed against the installed
/// `mdbook` binary before this function is finalized. If bare `-L` proves
/// insufficient, every `{{#rustdoc_include}}`-anchored example file adds one
/// `extern crate <crate_name>;` line inside its own `// ANCHOR:` region as a
/// fallback (Rust permits a redundant `extern crate` under any edition) — no
/// anchor name or blueprint signature changes.
pub fn test() -> CaseResult;
```

`mod.rs`:

```rust
pub mod anchors;
pub mod build;
pub mod manifest;

pub use anchors::{verify as verify_anchors, AnchorViolation};
pub use build::{build as mdbook_build, test as mdbook_test};
pub use manifest::{verify as verify_manifest, ManifestViolation, CHAPTER_MANIFEST};
```

### `xtask/src/lib.rs` (modify — two additive lines)

```rust
pub mod doc_check;
pub mod doc_guide;
```

### `xtask/src/main.rs` (modify — two additive `Command` variants)

```rust
DocCheck,
DocGuide {
    #[command(subcommand)]
    verb: DocGuideVerb,
},
```

```rust
#[derive(clap::Subcommand)]
enum DocGuideVerb {
    Build,
    Test,
    VerifyAnchors,
    VerifyManifest,
}
```

`Command::DocCheck` dispatches to `doc_check::run`; `Command::DocGuide { verb }` dispatches to the matching `doc_guide::*` function per `verb`, each writing its own `CaseResult`(s) to a fixed, documented path under `target/verify/` and using its own exit code as the authoritative pass/fail signal (TEST-D40).

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46), adapted per Context's own "one necessary adaptation" note.** The test-authoring changeset is: every file under `docs/mod-guide/` in full (real, final chapter prose — content, not implementation to stub, mirroring `mods/example-ores`'s own precedent exactly); every file under every `examples/` crate in full, including every `manifest.toml`, `Cargo.toml`, `src/*.rs`, and `tests/*.rs` (real, complete, working source and real, complete, working tests — content, never `todo!()`-stubbed, for the identical reason); `crates/mod-api/tests/examples_manifest_conformance.rs`, `tests/ui/wrong_native_entrypoint_signature.rs`, `tests/ui.rs` in full; `crates/mod-api/src/lib.rs`'s `#![deny(missing_docs)]` line and every doc-comment/doctest addition across `rc-mod-api`'s existing files (documentation is content, not implementation logic — it ships complete, matching every other content class in this list); `Cargo.toml` (root) and `.gitignore`'s additive edits; and every `xtask/src/doc_check.rs`/`doc_guide/*.rs` file from Deliverables **with every function body replaced by `todo!()`** (signatures, doc comments, the `CHAPTER_MANIFEST` constant's own data, and every error-enum variant stay exactly as specified — only the four checking/building *functions'* bodies are stubbed). The implementation changeset fills those `todo!()` bodies only; it must not modify any `docs/mod-guide/` file, any `examples/` file, `crates/mod-api/tests/examples_manifest_conformance.rs`, `tests/ui/*`, or any chapter/example content, and must not weaken any assertion below. `tests/ui/wrong_native_entrypoint_signature.stderr` is the one named exception (Implementation steps): generated once, at implementation time, by `trybuild` itself, then committed — mirroring M8-B01's own single named "discovered path" exception for `guest_bindings_compile.rs`.

### `xtask/tests/doc_check.rs` (new)

1. `missing_docs_lint_mechanism_actually_rejects_an_undocumented_item` **(mandatory self-test)** — writes a scratch, standalone single-file crate (own `Cargo.toml` with an empty `[workspace]` table, mirroring M8-B02's own fixture-crate opt-out idiom) containing `#![deny(missing_docs)] pub fn undocumented() {}`; runs `cargo build --manifest-path <scratch>/Cargo.toml`; asserts non-zero exit — proves the enforcement mechanism itself is sound, decoupled from `rc-mod-api`'s own current documentation completeness (which the real `run()` gate checks directly).
2. `doc_check_run_reports_two_distinct_cases` — `doc_check::run()` against the real, already-implemented `rc-mod-api` (this blueprint's own implementation changeset having already documented every item) returns exactly two `CaseResult`s, both `pass`.

### `xtask/tests/doc_guide_anchors.rs` (new)

1. `valid_directive_with_a_real_anchor_passes` — a scratch `book_src_dir` with one `.md` file containing `{{#rustdoc_include target.rs:demo}}` and a scratch `target.rs` containing `// ANCHOR: demo\nfn x() {}\n// ANCHOR_END: demo`; `verify` returns `Ok(())`.
2. `directive_referencing_a_missing_anchor_fails` **(mandatory self-test)** — the identical scratch `.md`, but `target.rs` contains no `// ANCHOR: demo` line at all; `verify` returns `Err(vec)` containing exactly one `AnchorViolation::AnchorStartMissing` naming the exact chapter file, directive line, and anchor name.
3. `directive_with_start_but_no_end_fails` — `target.rs` contains `// ANCHOR: demo` with no matching `// ANCHOR_END: demo` anywhere after it; `Err` containing `AnchorViolation::AnchorEndMissing`.
4. `directive_referencing_a_missing_file_fails` — the directive names a `target.rs` that does not exist on disk at all; `Err` containing `AnchorViolation::TargetFileMissing`.
5. `numeric_line_range_directives_are_skipped_not_flagged` — `{{#include target.rs:10:20}}`; `find_directives` returns zero entries for this line (out of this checker's own scope, Context).
6. `multiple_violations_across_files_are_all_collected` — two `.md` files, each with one broken directive; `verify` returns exactly two violations, not one (collect-all, never fail-fast).
7. `real_committed_book_passes` — `verify(docs/mod-guide/src/)` against the real, final, committed chapter files this blueprint ships returns `Ok(())` — the actual gate, proven against real content.

### `xtask/tests/doc_guide_manifest.rs` (new)

1. `chapter_manifest_matches_the_binding_curriculum_table` — `CHAPTER_MANIFEST.len() == 16` (15 numbered chapters + the Introduction row), and each entry's `number`/`title`/`landing`/`backing`/`decisions` fields match Context's own "Curriculum table" exactly (a literal, field-by-field regression guard).
2. `real_committed_tree_passes_every_check` — `verify(repo_root)` against the real, final, committed `docs/mod-guide/`/`examples/` trees this blueprint ships returns `Ok(())`.
3. `missing_chapter_file_is_caught` **(mandatory self-test)** — a scratch copy of the repo tree (never the real checkout) with one M8-landing chapter's `.md` file deleted; `verify` returns `Err` containing `ManifestViolation::ChapterFileMissingOrEmpty` naming that exact chapter.
4. `missing_backing_example_directory_is_caught` **(mandatory self-test)** — a scratch copy with one M8-landing chapter's backing `examples/<dir>/manifest.toml` deleted; `Err` containing `ManifestViolation::BackingExampleIncomplete`.
5. `missing_decision_citation_is_caught` **(mandatory self-test)** — a scratch copy with one chapter's `.md` text edited to remove its own required MOD-D citation substring; `Err` containing `ManifestViolation::DecisionNotCited` naming the missing decision.
6. `deferred_chapters_require_only_the_status_marker` — a scratch copy where Chapter 9's own stub file is missing its `<!-- STATUS: deferred -->` marker; `Err` containing `ManifestViolation::DeferredMarkerMissing`; restoring the marker (with no other content) makes that one violation disappear.
7. `cited_test_path_must_exist` — a scratch copy with `crates/mechanics/tests/water_override_replace.rs` (Chapter 7's own citation target) deleted; `Err` containing `ManifestViolation::CitedTestMissing`.
8. `summary_order_mismatch_is_caught` **(mandatory self-test)** — a scratch copy of `SUMMARY.md` with two chapter links swapped; `Err` containing `ManifestViolation::SummaryOrderMismatch` naming one of the two swapped chapters.

### `xtask/tests/doc_guide_build.rs` (new)

1. `check_mdbook_version_gives_an_actionable_message_when_absent` — a scratch `PATH` environment with no `mdbook` binary on it; `check_mdbook_version()` returns `Err` whose message contains the literal install command `cargo install mdbook --locked --version 0.5.4`.
2. `build_produces_index_html` — `mdbook_build()` against the real, committed `docs/mod-guide/` returns a `pass` `CaseResult`, and `docs/mod-guide/book/index.html` exists afterward.
3. `test_runs_every_anchored_slice_as_a_real_doctest` — `mdbook_test()` against the real, committed book (after `cargo build --workspace` has produced every `examples/` crate's rlib) returns a `pass` `CaseResult`.
4. `a_broken_example_build_fails_the_docs_gate` **(mandatory self-test)** — copies `examples/13-testing/` (chosen for its small size) into a scratch directory *outside* the main workspace (an empty `[workspace]` table added to its own `Cargo.toml`, mirroring M8-B02's/M8-B04's own established fixture-mutation technique exactly), introduces a syntax error into its `src/lib.rs`, and runs `cargo build --manifest-path <scratch>/Cargo.toml` as a child process; asserts non-zero exit — proving concretely that a broken example fails the same ordinary `cargo build` that Tier 1's own `--workspace` invocation already runs against every real `examples/` crate.

## Implementation steps

1. **`Cargo.toml` (root), `.gitignore`.** Apply both additive edits exactly as specified. Observable: `cargo metadata` still resolves; `git status` shows `docs/mod-guide/book/` ignored once it exists.
2. **`crates/mod-api/Cargo.toml`, `tests/ui/wrong_native_entrypoint_signature.rs`, `tests/ui.rs`.** Add the `trybuild` dev-dependency and the fixture file exactly as specified. Run `TRYBUILD=overwrite cargo test --test ui -p rc-mod-api` once, inspect the produced `tests/ui/wrong_native_entrypoint_signature.stderr`, and commit it verbatim (Constraints (a)'s one named exception). Observable: `cargo test --test ui -p rc-mod-api` passes on a clean run (no `TRYBUILD=overwrite`).
3. **`crates/mod-api/src/lib.rs` and every other file in the crate: `#![deny(missing_docs)]` + doctests.** Add the crate-root attribute; run `cargo doc -p rc-mod-api --all-features --no-deps` once, collect every "missing documentation for..." warning, and add one one-line `///` summary per flagged item — mechanical, no behavior change, touches no test. Then, per Context's audit ("`missing_docs`/doctest gating"), add one runnable doctest to every public item with a standalone constructor, and one explicitly-commented `no_run` doctest to the WIT guest-bindings module and the five mod-facing traits' own top-level doc comments. Observable: `RUSTDOCFLAGS="-D warnings" cargo doc -p rc-mod-api --all-features --no-deps` and `cargo test --doc -p rc-mod-api --all-features` both exit 0; every pre-existing test in `rc-mod-api` still passes unmodified.
4. **`crates/mod-api/tests/examples_manifest_conformance.rs`.** Write exactly as specified — this will not compile/pass until step 6 creates the `examples/` manifests it `include_str!`s, so this file's own presence is committed first (test-authoring) and its assertions become green once step 6 lands (implementation-adjacent content, per Context's own changeset-boundary note).
5. **`docs/mod-guide/book.toml`, `SUMMARY.md`, every chapter `.md` file (00 through 15).** Transcribe Chapters 1 and 2 verbatim from Deliverables; write every other M8-landing chapter following its own binding outline (Deliverables, "Chapters 3–15") — every anchor referenced must exist once step 6 lands; write Chapters 9 and 11's stub pages verbatim; write Chapter 15's starter content verbatim. Observable: `mdbook build docs/mod-guide` succeeds once step 6's anchors exist (deferred to step 7's own observable).
6. **Every `examples/` crate.** Write all 11 crates' `Cargo.toml`/`manifest.toml`/`src/*.rs`/`tests/*.rs` exactly per Deliverables (resolving every `dynptr!`/entry-factory construction by copying `mods/example-ores`'s own already-resolved pattern verbatim, per Context — no new moderate-confidence flag). Observable: `cargo build --workspace --all-features` and `cargo nextest run --workspace` both succeed, including every new crate; `examples_manifest_conformance.rs` (step 4) now passes.
7. **`xtask/src/doc_guide/anchors.rs`.** Implement `find_directives`/`verify` per Context's own algorithm (regex/string-scan every `.md` file for `{{#(include|rustdoc_include)\s+([^:}]+):([^0-9][^}]*)}}`-shaped lines with a non-numeric anchor suffix; resolve the path relative to the including file's own directory; scan the target file's lines for `// ANCHOR: <name>` then a later `// ANCHOR_END: <name>`). Observable: `xtask/tests/doc_guide_anchors.rs`'s 7 cases pass, including against the real, now-complete book (steps 5–6).
8. **`xtask/src/doc_guide/manifest.rs`.** Fill in `CHAPTER_MANIFEST`'s own data exactly per Context's "Curriculum table"; implement `verify` per its own six check classes, one per `ManifestViolation` variant (Deliverables) — `SummaryOrderMismatch` specifically: read `SUMMARY.md`'s own text and assert each `CHAPTER_MANIFEST` entry's `file` appears, as a Markdown link target (`(./<file>)`), in the identical order the table itself declares. Observable: `xtask/tests/doc_guide_manifest.rs`'s 8 cases pass.
9. **`xtask/src/doc_guide/build.rs`.** Implement `check_mdbook_version` (shell `mdbook --version`, parse for `0.5.4`, actionable `Err` on any mismatch/absence), `build` (shell `mdbook build docs/mod-guide`), `test` (shell `cargo build --workspace` then `mdbook test -L <target>/debug/deps docs/mod-guide`, resolving the `-L`/`--extern` moderate-confidence flag against the installed `mdbook` 0.5.4 binary first). Observable: `xtask/tests/doc_guide_build.rs`'s 4 cases pass.
10. **`xtask/src/doc_check.rs`.** Implement `run` (two `Command::new("cargo")` invocations, the first with `RUSTDOCFLAGS` set via `.env(...)` never shell syntax, both mapped to `CaseResult`). Observable: `xtask/tests/doc_check.rs`'s 2 cases pass.
11. **`xtask/src/lib.rs`, `xtask/src/main.rs`.** Add the two module lines and the two `Command` variants (`DocCheck`, `DocGuide { verb: DocGuideVerb }`) exactly as specified, dispatching to the functions above. Observable: `cargo run -p xtask -- doc-check`, `-- doc-guide build`, `-- doc-guide test`, `-- doc-guide verify-anchors`, `-- doc-guide verify-manifest` all run and exit 0 against the real, final, committed content.
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all exit 0.
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50), including a preceding `cargo install mdbook --locked --version 0.5.4` step added beside CI's existing pinned-CLI-tool installs (TEST-D25/D33/D35's own established precedent — this blueprint's own one-line, additive CI-setup edit; the exact `.github/workflows/ci.yml` file this line lands in is not reproduced in full here, since it is not among this blueprint's own read prerequisites — mirroring how every other M8 blueprint treats "CI green" as the authoritative done-signal without claiming a byte-for-byte `ci.yml` diff).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, per Context's own stated adaptation: every chapter file, every `examples/` crate's full source, and `crates/mod-api/tests/examples_manifest_conformance.rs`/`tests/ui/*.rs` ship complete, as content, in the test-authoring changeset — never `todo!()`-stubbed. The **one** named exception is `tests/ui/wrong_native_entrypoint_signature.stderr`, generated by `trybuild` itself at implementation time and committed (Implementation step 2) — the same class of exception M8-B01's `guest_bindings_compile.rs` and M8-B02's fixture crates already establish. Only `xtask/src/doc_check.rs`'s and `doc_guide/*.rs`'s four function bodies are `todo!()`-stubbed in the test-authoring changeset; the implementation changeset fills those bodies only and must not modify any other file this blueprint's own Acceptance tests name.

(b) **No new external dependencies beyond the pinned `[workspace.dependencies]` set, with one deliberate, named exception.** `trybuild` 1.0.120, exactly as MOD-D48 itself already names and version-pins it, is the one new `[workspace.dependencies]` line this blueprint adds. `mdbook` 0.5.4 is a pinned **external CLI tool** (MOD-D49), never a `[workspace.dependencies]` entry — no crate anywhere in this blueprint's Deliverables `use`s an `mdbook` crate. Every other crate used (`rc-mod-api`, `stabby`) is already pinned by an earlier M8 blueprint.

(c) **No Mojang or third-party reimplementation code.** Every chapter's prose, every example's content, and the anchor/manifest-checker algorithms are derived solely from `docs/planning/06-modding-api.md`'s MOD-D47–D52, this blueprint's own prerequisite blueprints (M8-B01/B02/B04/B06a/B06b), and this blueprint's own concrete, cited resolutions of what those decisions leave open (ASSET-D18/D19/D30). No content from any other engine's modding documentation is consulted.

(d) **`unsafe` code is permitted only where `stabby`'s own API already requires it**, identical scope to every prior M8 blueprint — every `examples/` crate's own `unsafe extern "C" fn rc_mod_abi_handshake`/entry-factory export mirrors `mods/example-ores`'s own already-audited pattern exactly; this blueprint's own `xtask`/chapter-prose content introduces no new `unsafe` block anywhere.

(e) **Protected-path scoping, restated exactly per MOD-D51.** `examples/`'s own crate source — including every `examples/<NN>-<slug>/tests/*.rs` file this blueprint adds — and `docs/mod-guide/`'s own chapter prose are ordinary, implementation-changeset-editable content, never `TEST-D46`-protected (an explicit, named carve-out `TEST-D46`'s own decision text now states directly) — a future `rc-mod-api` signature change belongs in the *same* changeset as the `examples/`/chapter-anchor fix it requires, including the matching fix to that example's own tests. `crates/mod-api/tests/ui/*.rs`/`*.stderr` already fall under `TEST-D46`'s existing "any crate's `tests/` directory" clause with no extension needed — this blueprint's own new files there are protected by that pre-existing rule, not by any new rule this blueprint invents.

(f) **Scope boundary — do not implement beyond this blueprint's stated Deliverables.** This blueprint does not implement: `rc-mod-test` (MOD-D29, still a separate, later blueprint — Chapter 13's own honest framing, Context); the `cargo generate` template itself (MOD-D27's own tooling half, still deferred — Chapter 1's own honest framing); any wiring of a `ServerModHost` into a real `rusty-clanker-server` composition root (`M8-B05`'s own still-open, named gap, restated by Chapter 1); MOD-D43/D44's region-/world-scoped mod data mechanism (Chapter 9's own honest stub); any client-side renderer (Chapter 11's own honest stub, deferred to `M10`); the WIT `@since`/`@unstable`/`@deprecated` gate enforcement or the native `#[rc_mod_api::unstable]` attribute macro (MOD-D22's own still-unimplemented enforcement half, Chapter 14's own honest framing); public hosting of the built book beyond a CI-produced, repo-served static artifact (MOD-D49's own scope, no external host wired). Every honest "not yet" statement embedded in this blueprint's own chapter content (Deliverables) is binding text, not a placeholder an implementer should "complete" by inventing an unbuilt mechanism.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build --workspace --all-features
cargo nextest run --workspace
cargo test --doc -p rc-mod-api --all-features
cargo test --test ui -p rc-mod-api
cargo run -p xtask -- doc-check
cargo run -p xtask -- doc-guide build
cargo run -p xtask -- doc-guide test
cargo run -p xtask -- doc-guide verify-anchors
cargo run -p xtask -- doc-guide verify-manifest
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo run -p xtask -- doc-guide verify-manifest`'s own output names every one of the 13 M8-landing chapters as `pass` (chapter file present, backing example or cited test present, every required decision cited) and both deferred chapters (9, 11) as `pass` under the relaxed "status marker only" rule — this is the mechanical, CI-enforced realization of MOD-D52's own binding definition-of-done rule. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
