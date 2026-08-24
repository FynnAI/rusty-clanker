# M10-B07 — Mod Developer Guide: The Client-Side Chapter Lands

| Field | Content |
|---|---|
| ID | M10-B07 |
| Milestone | M10 — Client Feature Parity: Entities, UI, Isomorphic Mods |
| Prerequisites | M10-B01 (entity rendering — `entity::renderer::{EntityRenderer, EntityRendererRegistry}`, and `ClientRegistryBuildContext::register_entity_renderer`'s own registration-only bar, restated below exactly as that blueprint fixes it). M10-B02 (UI/HUD — `gui::widget::{Widget, Screen, HudOverlay}`, `hud::elements::DefaultHudOverlay`, consulted as the framework the mod-facing GUI/HUD extension points ultimately compose into — restated, never re-implemented). M10-B05 (client mod-host integration — the concrete subject of this chapter: `ClientRegistryBuildContext`'s six extension points at their real, current bar; `provide_model_geometry`/`provide_block_material`/`provide_hud_text_line`/`provide_static_screen`; `ClientModEntry::on_client_tick`/`on_channel_message`; `ClientTickContext`; and `mods/example-ores/client/src/lib.rs`'s own completed client render hook plus its two proof tests, `mods/example-ores/client/tests/pulse_material_isomorphism.rs` and `crates/render/tests/gpu_smoke/mod_block_render.rs` — every name and signature below is restated exactly as that blueprint ships it, not re-derived). M8-B01 (`rc-mod-api`'s original five client extension-point declarations — `register_model_provider`, `register_block_renderer`, `register_gui_screen`, `register_hud_overlay`, `register_input_binding`, all on `ClientRegistryBuildContext` — restated below exactly as shipped). M8-B07 (the mdBook infrastructure this blueprint extends: `docs/mod-guide/`'s `book.toml`/`SUMMARY.md`, `xtask/src/doc_guide/{manifest,anchors,build}.rs`, `xtask/tests/doc_guide_manifest.rs`, and Chapter 11's own short, honest, `<!-- STATUS: deferred -->`-marked stub page this blueprint replaces — every file this blueprint touches is a file M8-B07 already created; this blueprint's own edits to `xtask/src/doc_guide/manifest.rs` and `xtask/tests/doc_guide_manifest.rs` are additive, mirroring the exact "later blueprint additively extends an earlier blueprint's own file" pattern M10-B05 §2 already establishes for `rc-mod-api`'s `entrypoint.rs`). |
| Implements | MOD-D50 (the fixed 15-chapter curriculum — Chapter 11's own fixed position, title, and MOD-D18 citation, already bound; this blueprint fills the one chapter M8 itself could not). MOD-D52 (**"Client-Side (chapter 11) lands with `M10`, unconditionally"** — this blueprint is that landing, in full: MOD-D52's own binding definition-of-done rule, "a capability is not done until its chapter and its tested `examples/` entry both exist and pass Tier 1 CI," realized here via `Backing::Cited` rather than a new `examples/` entry, per the next citation). MOD-D51 (**the flagship-example-link convention** — "`mods/example-ores` is linked explicitly... from every chapter whose capability it happens to exercise... while `examples/`'s own per-chapter crates stay each chapter's primary, minimal, single-concept teaching example" — Chapter 11 is the first chapter whose *primary* worked example is the flagship reference mod itself, not a per-chapter `examples/` crate, because M10-B05 completed `mods/example-ores`'s own client entry against exactly two of MOD-D18's six extension points before this blueprint was written — mirroring MOD-D50's own Chapter 7 precedent, "links... directly — no separate `examples/` entry, per MOD-D50's own worked-case reuse," applied here to a second chapter for an identical reason: a real, CI-proven end-to-end example already exists and inventing a second, parallel, unverified one would violate this corpus's own "cite the gap, don't restate a proof that already exists" discipline). MOD-D18 (the six client extension points — this chapter is their first developer-facing documentation, restated below at each one's own real, current, honestly-bounded bar). TEST-D45/D46 (test-first changeset boundary, restated; `xtask/tests/doc_guide_manifest.rs` is a protected path under TEST-D46's general "any crate's `tests/` directory" rule — MOD-D51's own protected-path carve-out names only `examples/` crate source and `docs/mod-guide/`'s own chapter *prose* as ordinary, implementation-changeset-editable content; it does not extend to `xtask/tests/doc_guide_manifest.rs` itself, restated in Constraints). TEST-D50 (CI is the sole authority on completion). |
| Crates touched | `docs/mod-guide/src/11-client-side.md` (modify — full replacement of M8-B07's own deferred stub with real content; no other file under `docs/mod-guide/` changes — `SUMMARY.md` already lists Chapter 11 at its fixed position and needs no edit). `xtask/src/doc_guide/manifest.rs` (modify, additive only — one new `Landing` variant, one updated `CHAPTER_MANIFEST` row). `xtask/tests/doc_guide_manifest.rs` (modify — one updated expected-table assertion, one new self-test). No `Cargo.toml` edit (this blueprint adds no new crate and no new dependency — Chapter 11's own worked example already exists as a real, compiling, tested part of `mods/example-ores`, per MOD-D51 above). |
| Estimated scope | S |

## Goal & Done definition

Close the one M10-scope gap the roadmap's own text names explicitly and MOD-D52 binds unconditionally: give Chapter 11 (Client-Side: Models, Renderers, GUI, HUD, Input) real, honest, mechanically-verified content, replacing M8-B07's own deliberately short deferred stub. The chapter documents all six of `ClientRegistryBuildContext`'s client extension points (MOD-D18) at their real, current bar — two proven end to end by `mods/example-ores`'s own completed client render hook (M10-B05), two more real and tested but not yet exercised by the reference mod, and two still registration-only — never overstating any of them. `xtask/src/doc_guide/manifest.rs`'s `CHAPTER_MANIFEST` row for Chapter 11 moves from `Landing::Deferred { until: "M10" }` to a landed, enforced entry citing `mods/example-ores/client/tests/pulse_material_isomorphism.rs` as its backing proof (`Backing::Cited`), per MOD-D51's flagship-example convention — no new `examples/` crate is added, since the reference mod already is the real, working, CI-proven worked case for this chapter's own two live-payload extension points.

Done when:

- [ ] `docs/mod-guide/src/11-client-side.md` contains the real content below — non-empty, no `<!-- STATUS: deferred -->` marker anywhere in it, `MOD-D18` cited as a literal substring.
- [ ] `xtask/src/doc_guide/manifest.rs`'s `Landing` enum gains a new `M10` unit variant with identical enforcement semantics to the existing `M8` variant; `CHAPTER_MANIFEST`'s Chapter 11 row reads `landing: Landing::M10, backing: Backing::Cited { test_path: "mods/example-ores/client/tests/pulse_material_isomorphism.rs" }, decisions: &["MOD-D18"]` — every other one of the 16 rows unchanged.
- [ ] `cargo run -p xtask -- doc-guide verify-manifest` exits 0 against the real, committed tree.
- [ ] `cargo run -p xtask -- doc-guide verify-anchors` still exits 0 (this blueprint adds zero `{{#include}}`/`{{#rustdoc_include}}` directives — Chapter 11 links to real files by path, mirroring Chapter 7's own zero-anchor precedent exactly, Context below).
- [ ] `cargo run -p xtask -- doc-guide build` and `cargo run -p xtask -- doc-guide test` both exit 0.
- [ ] `cargo nextest run -p xtask` passes in full, including `xtask/tests/doc_guide_manifest.rs`'s updated and newly-added cases, with every other `xtask` test file this blueprint does not touch still passing unmodified.
- [ ] `cargo run -p xtask -- lint` and `cargo run -p xtask -- fmt-check` both exit 0.
- [ ] Tier 1 CI green on both `ubuntu-24.04` and `windows-2025` (MOD-D51 places every doc-guide check in Tier 1; TEST-D43) is the authoritative done-signal — no code beyond `xtask` is touched, so no other crate's own CI tier is affected.

## Context (self-contained)

**Why this blueprint exists.** `11-roadmap-milestones.md`'s M10 Scope text states plainly: "The Mod Developer Guide's Client-Side chapter (mdBook chapter 11: Models, Renderers, GUI, HUD, Input — MOD-D50/D52) lands here... it cannot land earlier, since `M10` is the milestone that first proves the hook renders correctly at all." `06-modding-api.md`'s MOD-D52 states the same requirement even more bindingly: "Client-Side (Chapter 11) lands with `M10`, unconditionally." M8-B07 (which built every other chapter and the whole mdBook infrastructure) deliberately shipped Chapter 11 only as a short, honest, `<!-- STATUS: deferred -->`-marked stub, naming `M10` as the milestone that must supply real content — restated in that blueprint's own words: "This chapter documents the full, working client-side rendering/GUI/input story once Phase 2's client renderer exists and can prove `mods/example-ores`'s own block actually renders — the milestone that unblocks this is `M10`." M10-B05 is that unblocking blueprint: it completes `mods/example-ores`'s client render hook for real (closing the M10 roadmap's own acceptance criterion 2) and adds a live payload-completion method to four more of the six extension points. This blueprint is the one M10 blueprint whose entire job is writing that now-possible chapter — nothing here changes engine behavior.

**The six extension points, restated at their real, current bar (the chapter's own subject).** `rc_mod_api::ClientRegistryBuildContext` (MOD-D18) exposes:

1. `register_model_provider(&mut self, id: Identifier)` (M8-B01) — declaration-only at `M8`. Completed at `M10` by `provide_model_geometry(&mut self, model: Identifier, state_properties: stabby::string::String, faces: ModBakedModel)` (M10-B05): supplies an explicit face list (`ModBakedModel` — quads, each with position/UV/normal) for a block needing a shape other than a plain cube. Real and Tier-1-tested via a small, synthetic, hand-authored fixture mod M10-B05's own Acceptance tests name — no shipped reference-mod content exercises it, since `pulse_crystal` (the reference mod's one block) stays a plain cube.
2. `register_block_renderer(&mut self, block: Identifier)` (M8-B01) — declaration-only at `M8`. Completed at `M10` by `provide_block_material(&mut self, block: Identifier, state_properties: stabby::string::String, color: rc_mod_api::ModColor, emissive: bool)` (M10-B05): supplies a material — a color and an emissive flag — for one block state, resolved client-side into a real texture-atlas entry. This is the extension point `mods/example-ores`'s own client entry exercises for real (below).
3. `register_gui_screen(&mut self, id: Identifier)` (M8-B01) — declaration-only at `M8`. Completed at `M10` by `provide_static_screen(&mut self, screen: Identifier, open_binding: Identifier, title: stabby::string::String, lines: stabby::vec::Vec<stabby::string::String>)` (M10-B05): a read-only screen — one title, an ordered list of text lines, no click handling, no nested widgets — opened when the named `open_binding` action (extension point 5, below) transitions to "just pressed." Real and Tier-1-tested by M10-B05's own registry-recording test; not exercised by the shipped reference mod.
4. `register_hud_overlay(&mut self, id: Identifier)` (M8-B01) — declaration-only at `M8`. Completed at `M10` by `provide_hud_text_line(&mut self, overlay: Identifier, anchor: rc_mod_api::ClientHudAnchor, initial_text: stabby::string::String)` (M10-B05), `ClientHudAnchor` being one of `{TopLeft, TopRight, BottomLeft, BottomRight}`: registers one labeled HUD text line at a fixed screen corner, updatable every tick via `ClientModEntry::on_client_tick(&mut self, ctx: &mut ClientTickContext)` calling `ClientTickContext::set_hud_text_line(&mut self, overlay: Identifier, text: stabby::string::String)`. This is the second extension point `mods/example-ores`'s own client entry exercises for real (below).
5. `register_input_binding(&mut self, id: Identifier)` (M8-B01) — declaration-only at `M8`, unchanged at `M10`: records an action `Identifier` a mod wants to bind. No `KeyCode`/keybinding-UI surface is wired to it yet — a real, named gap, not an oversight.
6. `register_entity_renderer(&mut self, entity_type: Identifier)` — new at `M10` (M10-B01 fixes this exact signature, "mirroring `register_block_renderer`'s exact registration-only shape"; M10-B05 adds it to `ClientRegistryBuildContext`). Declaration-only, headlessly verified — no live, ABI-safe payload-completion method exists, because `EntityRenderer` (M10-B01's own trait) uses non-ABI-safe types throughout (`glam::Vec3`/`Mat4`, an unbounded `Vec<EntityVertex>`) and the shipped reference mod has no entity content at all to validate a bridge against (M10-B05 §6's own stated reasoning, restated here for the chapter's own honesty). A future blueprint's job, named as such (Interfaces, below).

**`mods/example-ores`'s client entry is this chapter's own worked example — cited, never duplicated.** M10-B05 additively extends `mods/example-ores/client/src/lib.rs`'s already-shipped `on_client_registry_build`/`on_client_init` with two real, working, CI-proven client-side behaviors: two `provide_block_material` calls (one per `pulse_crystal` block state, `"lit=false"`/`"lit=true"`, distinct `color`/`emissive`), and one `provide_hud_text_line` call plus a real `on_client_tick` implementation toggling that line's text on `example_ores_shared::PULSE_PERIOD_TICKS`' own shared cadence — the identical constant the server-side `PulseCrystalBehavior` already uses, proving the client's own toggle is not an independently-guessed timing. Two real tests prove this: `mods/example-ores/client/tests/pulse_material_isomorphism.rs` (Tier 1, PR-blocking — asserts the two materials are distinct and the HUD toggle cadence matches the shared constant exactly) and `crates/render/tests/gpu_smoke/mod_block_render.rs` (Tier 2, nightly, TEST-D53 — a real offscreen render via a software rasterizer, asserting the sampled pixel color is visibly distinct between the two states). Per MOD-D51's own flagship-example-link convention and MOD-D50's own already-established Chapter 7 precedent ("links... directly — no separate `examples/` entry, per MOD-D50's own worked-case reuse"), this chapter cites these two real files directly rather than building a parallel, redundant `examples/11-client-side` crate that would only re-demonstrate what `mods/example-ores` already proves for real. This is a deliberate, cited choice, not a shortcut: a new example crate would need its own registry-build/HUD content to prove anything, and `mods/example-ores` already carries exactly that content, already Tier-1-and-Tier-2 proven.

**The still-open gap this chapter must state plainly, not gloss over.** M10-B05 §12 names it precisely: no live, network-connected client can render a mod's block correctly end to end against a real server yet, because no runtime, mod-extensible `BlockStateId` space exists on either side of the wire at any milestone through `M10` — the material/HUD proofs above run against a synthetic, test-reserved block-state slot and synthetic tick data, never a real join. This corpus's own "state the gap, don't gloss" discipline (already applied by Chapters 4/6/8/13 in M8-B07) applies here identically — the chapter says so, in its own words, rather than letting a reader assume more is proven than actually is.

**Why `Landing::M8` gains an `M10` sibling rather than being generalized.** M8-B07's own `Landing` enum (`xtask/src/doc_guide/manifest.rs`) has exactly two variants: `M8` (full enforcement) and `Deferred { until: &'static str }` (stub-only enforcement). Chapter 11 moving from the second to a full-enforcement state needs a variant carrying that same full-enforcement meaning at a different milestone. The minimal, additive, current-state-accurate edit is a new sibling unit variant, `M10`, with `verify()`'s existing "ships in full — backing and decisions enforced" check logic extended to match `Landing::M8 | Landing::M10` from this point forward — never renaming or restructuring `M8`'s own already-shipped, already-tested meaning, mirroring this corpus's own "additive-only" discipline for every other later blueprint's edit to an earlier blueprint's own shared file (M10-B05 §2's `ClientRegistration`/`ClientRegistryBuildContext` extension is the direct precedent for the same file class).

## Deliverables

### `docs/mod-guide/src/11-client-side.md` (modify — full replacement, verbatim)

````markdown
# Client-Side: Models, Renderers, GUI, HUD, Input

The client exposes six extension points on `rc_mod_api::ClientRegistryBuildContext`
(MOD-D18): a model provider, a block renderer, a GUI screen, a HUD overlay, an
input binding, and an entity renderer. All six are real today — every one
records a mod's declaration and is proven, headlessly, to have been called —
and two of them go further: they carry a genuine, working payload all the way
to pixels (or HUD text) on screen, proven end to end by this project's own
reference mod.

## See it running: `mods/example-ores`

`mods/example-ores`'s client entry (`mods/example-ores/client/src/lib.rs`) is
this chapter's own worked example — linked directly here rather than
duplicated into a separate `examples/` crate, since it already demonstrates
two of the six extension points for real, compiling and passing today.

- **Block renderer + material.** The entry calls
  `ctx.register_block_renderer(...)` for `example_ores:pulse_crystal`, then
  completes that declaration with two
  `ctx.provide_block_material(block, state_properties, color, emissive)`
  calls — one for `"lit=false"`, one for `"lit=true"` — each giving the
  client a distinct, resolvable color and emissive flag for that block
  state. This is a **material**, not a shape: the block stays a plain cube,
  only its color and glow differ between states. A block that needs a
  genuinely different shape uses `provide_model_geometry` instead (below).
- **HUD overlay + text line.** The entry also registers one labeled HUD text
  line, anchored to a screen corner (`ClientHudAnchor::TopLeft`, one of four
  corners). Every simulation tick, `ClientModEntry::on_client_tick` reads the
  mod's own last-known pulse state and, on the shared toggle cadence
  (`example_ores_shared::PULSE_PERIOD_TICKS` — the identical constant the
  server-side behavior already uses), calls
  `ClientTickContext::set_hud_text_line` to update the line's text. This is
  the same shared-constant discipline every isomorphic mod should follow:
  the client never guesses its own timing independently of the server.

Two real tests prove this end to end:
`mods/example-ores/client/tests/pulse_material_isomorphism.rs` (Tier 1 — the
two materials are distinct, and the HUD toggle cadence matches the shared
constant exactly) and `crates/render/tests/gpu_smoke/mod_block_render.rs`
(Tier 2, nightly — a real offscreen GPU render of both block states, asserting
the sampled pixel color is visibly different between them).

## The other four extension points, honestly

- **Model provider + geometry.** `provide_model_geometry` supplies an
  explicit face list (a `ModBakedModel` — quads with position, UV, and
  normal) for a block that needs a shape other than a plain cube. Real and
  tested — a synthetic fixture mod's own registry-build test proves the
  geometry path independently — but no shipped reference-mod content needs
  it yet, since `pulse_crystal` only needs a material, not a shape.
- **GUI screen + static content.** `provide_static_screen` registers a
  read-only screen — a title plus an ordered list of text lines — opened
  when a companion input-binding action transitions to "just pressed." No
  click handling, no nested widgets. Real and tested, but not exercised by
  the shipped reference mod. A genuinely interactive, mod-supplied screen
  (real widget trees, real click round-tripping) does not exist yet — a
  future capability, not a current one.
- **Input binding.** `register_input_binding` declares an action
  `Identifier` a mod wants to bind — recorded and headlessly verified, with
  no `KeyCode`/keybinding-UI surface wired to it yet.
- **Entity renderer.** `register_entity_renderer` declares that a mod owns
  rendering for a given entity type `Identifier` — recorded and headlessly
  verified, mirroring `register_block_renderer`'s own original
  declaration-only bar. No live, ABI-safe payload-completion method exists
  yet (the shipped reference mod has no entity content to validate one
  against), so a mod cannot yet make a custom entity actually render — a
  genuine, named gap for a future capability to close.

## What isn't there yet

Nothing above renders against a **live, network-connected client talking to
a real server** end to end — the material and HUD proofs above run against
synthetic, test-reserved block-state and tick data, not a real connection,
because no runtime, mod-extensible block-state-id space exists on either side
of the wire yet. A mod's block or HUD text is real and provably correct in
isolation; seeing it live, in a real joined world, is a future capability's
job.

## API reference

Every type and method named above lives on `rc_mod_api::{ClientRegistryBuildContext,
ClientModEntry, ClientTickContext, ClientHudAnchor}` and is documented in full
in the rustdoc reference (`cargo doc -p rc-mod-api --open`).
````

### `xtask/src/doc_guide/manifest.rs` (modify — additive)

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Landing {
    /// Ships in full at `M8` — this row's `backing` and `decisions` are enforced.
    M8,
    /// Ships in full at `M10` — identical enforcement to `M8`, a different milestone.
    /// New in this blueprint (M10-B07); every pre-existing `M8`-landing row is unaffected.
    M10,
    /// Present as a short, honest stub page; full content is a future milestone's job.
    /// Enforced only to exist and to carry the literal marker `<!-- STATUS: deferred -->`.
    Deferred { until: &'static str },
}
```

`verify`'s own existing "ships in full" check branch (M8-B07 Context, "six check classes, one per `ManifestViolation` variant") now matches `Landing::M8 | Landing::M10` — both land with backing and decisions enforced; only `Deferred` gets the reduced stub-only check. No other function signature in `manifest.rs` changes.

`CHAPTER_MANIFEST`'s Chapter 11 row (the one row this blueprint changes; every other row unchanged):

```rust
ChapterEntry {
    number: 11,
    title: "Client-Side",
    file: "11-client-side.md",
    landing: Landing::M10,
    backing: Backing::Cited { test_path: "mods/example-ores/client/tests/pulse_material_isomorphism.rs" },
    decisions: &["MOD-D18"],
},
```

### Curriculum table, restated in full (M8-B07's own table, Chapter 11's row updated — the literal fixture `chapter_manifest_matches_the_binding_curriculum_table` checks against)

| # | Title | File | Lands | Backing | Primary decisions cited |
|---|---|---|---|---|---|
| — | Introduction | `00-introduction.md` | M8 | Prose | — |
| 1 | Getting Started | `01-getting-started.md` | M8 | `examples/01-getting-started` | MOD-D27 |
| 2 | Core Concepts | `02-core-concepts.md` | M8 | Prose | MOD-D8, MOD-D9, MOD-D33 |
| 3 | Blocks & Behaviors | `03-blocks-and-behaviors.md` | M8 | `examples/03-blocks` | MOD-D6, MOD-D8 |
| 4 | Items | `04-items.md` | M8 | `examples/04-items` | MOD-D6 |
| 5 | Custom Systems & Ordering Anchors | `05-custom-systems-and-ordering-anchors.md` | M8 | `examples/05-systems` | MOD-D8, MOD-D10 |
| 6 | Events | `06-events.md` | M8 | `examples/06-events` | MOD-D39 |
| 7 | Override & Wrap Vanilla | `07-override-and-wrap-vanilla.md` | M8 | Cited: `crates/mechanics/tests/water_override_replace.rs` | MOD-D33, MOD-D35, MOD-D38 |
| 8 | Components on Vanilla Entities & Persistence | `08-components-and-persistence.md` | M8 | `examples/08-components` | MOD-D41, MOD-D42 |
| 9 | Custom World/Chunk Data | `09-custom-world-chunk-data.md` | Deferred (post-M8) | Prose (deferred stub) | MOD-D43, MOD-D44 |
| 10 | Mod Networking Channels | `10-mod-networking-channels.md` | M8 | `examples/10-networking` | MOD-D20 |
| **11** | **Client-Side** | `11-client-side.md` | **M10** | **Cited: `mods/example-ores/client/tests/pulse_material_isomorphism.rs`** | MOD-D18 |
| 12 | Isomorphic Packaging & the One-Crate-Two-Targets Build | `12-isomorphic-packaging.md` | M8 | `examples/12-isomorphic/server` (+ sibling `shared`/`client`) | MOD-D4, MOD-D5 |
| 13 | Testing Your Mod | `13-testing-your-mod.md` | M8 | `examples/13-testing` | MOD-D29 |
| 14 | Publishing, Versioning & ABI Compatibility | `14-publishing-versioning-and-abi-compatibility.md` | M8 | Cited: `crates/mod-api/tests/abi_handshake.rs` | MOD-D21, MOD-D22, MOD-D23, MOD-D26 |
| 15 | Migration Notes Policy | `15-migration-notes-policy.md` | M8 | Prose | MOD-D23 |

## Acceptance tests (write these FIRST — own changeset)

### `xtask/tests/doc_guide_manifest.rs` (modify — two of its existing eight cases affected, one new case added)

1. `chapter_manifest_matches_the_binding_curriculum_table` (existing, M8-B07) — its own expected-table fixture updates to match this blueprint's own "Curriculum table, restated in full" above: row 11 now reads `landing: Landing::M10`, `backing: Backing::Cited { test_path: "mods/example-ores/client/tests/pulse_material_isomorphism.rs" }`; every other row's expected value is unchanged from M8-B07's own original fixture.
2. `real_committed_tree_passes_every_check` (existing, M8-B07) — unchanged in shape; now also implicitly covers Chapter 11's own new landed content (the real, committed `docs/mod-guide/src/11-client-side.md` and the real, committed `mods/example-ores/client/tests/pulse_material_isomorphism.rs` both exist), so this case's own pass is a genuine, new proof this blueprint's own content is complete and correctly wired — no code change to the test itself.
3. **`m10_landing_chapter_is_enforced_identically_to_m8_landing_chapter`** (new, mandatory self-test — proves `Landing::M10`'s own enforcement branch actually fires, not merely that adding the variant compiles) — a scratch copy of the repo tree (never the real checkout, mirroring every other mandatory self-test's own established construction) with `docs/mod-guide/src/11-client-side.md`'s own `MOD-D18` citation string removed (the file otherwise left non-empty and marker-free); `verify(scratch_root)` returns `Err` containing `ManifestViolation::DecisionNotCited { number: 11, title: "Client-Side", decision: "MOD-D18", .. }` — proving `Landing::M10`'s row is checked at the identical `DecisionNotCited` bar `Landing::M8` rows already carry, not silently skipped as if it were still `Deferred`. A second sub-case in the same test: the same scratch copy with `mods/example-ores/client/tests/pulse_material_isomorphism.rs` deleted instead; `Err` containing `ManifestViolation::CitedTestMissing { number: 11, .. }` — proving Chapter 11's own `Backing::Cited` path is checked identically to Chapter 7's and Chapter 14's already-established `Cited` rows (test 7, `cited_test_path_must_exist`, unchanged, continues covering Chapter 7's own target independently).

Every other existing case in this file (`missing_chapter_file_is_caught`, `missing_backing_example_directory_is_caught`, `missing_decision_citation_is_caught`, `deferred_chapters_require_only_the_status_marker`, `cited_test_path_must_exist`, `summary_order_mismatch_is_caught`) is unchanged — none of them names Chapter 11 as its own scratch target, and each continues to pass unmodified against the updated `CHAPTER_MANIFEST` data.

No other test file is added or modified — this blueprint's own content (the chapter prose) is not itself a test target beyond the manifest checks above, matching MOD-D51's own explicit carve-out ("`docs/mod-guide/`'s own chapter prose [is] ordinary, implementation-changeset-editable content, never a `TEST-D46` protected path").

## Implementation steps

1. **`xtask/src/doc_guide/manifest.rs`: add `Landing::M10`.** Add the new unit variant exactly as specified (Deliverables); extend `verify`'s own existing "ships in full" match arm from `Landing::M8 => { .. }` to `Landing::M8 | Landing::M10 => { .. }` — the check body itself is unchanged, only the pattern widens. Observable: `cargo build -p xtask` succeeds; every pre-existing `xtask/tests/doc_guide_manifest.rs` case except test 1 (next step) still passes against the not-yet-updated `CHAPTER_MANIFEST` data, since `Landing::M8`'s own behavior is untouched.
2. **`xtask/src/doc_guide/manifest.rs`: update `CHAPTER_MANIFEST`'s Chapter 11 row.** Apply exactly the row given in Deliverables — `landing: Landing::M10`, `backing: Backing::Cited { test_path: "mods/example-ores/client/tests/pulse_material_isomorphism.rs" }`, `decisions` unchanged (`&["MOD-D18"]`). Every other one of the 16 rows in the array is untouched, byte-for-byte. Observable: `chapter_manifest_matches_the_binding_curriculum_table` now passes against the updated fixture (Acceptance tests, case 1).
3. **`docs/mod-guide/src/11-client-side.md`: replace the stub.** Overwrite the file with the exact content given in Deliverables. Observable: `real_committed_tree_passes_every_check` now returns `Ok(())` (previously it already returned `Ok(())` against the old `Deferred`-landing row's own reduced stub-only check; it continues to return `Ok(())` now against the new, fuller `Landing::M10` check — a genuine, new proof, not a coincidental pass).
4. **Run `cargo run -p xtask -- doc-guide verify-manifest`, `verify-anchors`, `build`, `test` against the real, committed tree.** All four exit 0. `verify-anchors` finds zero new directives (Context: this chapter adds none), so its own already-passing behavior is unaffected by this blueprint entirely.
5. **Full local test pass.** `cargo nextest run -p xtask`, confirming every case in `xtask/tests/doc_guide_manifest.rs` (the updated case, the new case, and all six untouched cases) passes, and every other `xtask/tests/*.rs` file this blueprint does not touch (`doc_check.rs`, `doc_guide_anchors.rs`, `doc_guide_build.rs`, and every non-doc-guide `xtask` test) still passes unmodified.

## Constraints & forbidden actions

- **Test-first changeset boundary is binding (TEST-D45/D46).** `xtask/tests/doc_guide_manifest.rs`'s own updated and new cases (Acceptance tests) are written and committed first; the implementation changeset (`xtask/src/doc_guide/manifest.rs`'s `Landing`/`CHAPTER_MANIFEST` edit, and `docs/mod-guide/src/11-client-side.md`'s own content) follows, touching nothing else. `xtask/tests/doc_guide_manifest.rs` is itself a protected path under TEST-D46's general "any crate's `tests/` directory" rule — MOD-D51's own protected-path carve-out applies only to `examples/` crate source and to `docs/mod-guide/`'s own chapter *prose*, never to `xtask/tests/` itself; this blueprint respects that boundary exactly, changing the test file only in its own dedicated test-authoring changeset.
- **No new external dependency.** This blueprint adds no `[workspace.dependencies]` entry and no new crate — `mods/example-ores` and its own tests already exist, shipped by M10-B05.
- **No Mojang or third-party reimplementation source is consulted.** Every fact this blueprint's own chapter states is drawn from this corpus's own already-committed blueprints (M10-B01/B02/B05, M8-B01/B07) and real, already-shipped source paths — never a decompiled jar, never another reimplementation's code (ASSET-D18/D19/D30).
- **Additive-only edit to `xtask/src/doc_guide/manifest.rs` and `xtask/tests/doc_guide_manifest.rs`.** Every pre-existing `ChapterEntry`, every pre-existing test case's own assertions besides the two named in Acceptance tests, and the `Landing::M8`/`Deferred` variants' own meaning are unchanged.
- **This blueprint modifies no engine, render, or mod-host production code.** `crates/mod-api/`, `crates/mod-host/`, `crates/client/`, `crates/render/`, and `mods/example-ores/` are read-only references for this blueprint — every signature and file path it cites is restated exactly as M10-B01/B02/B05 and M8-B01 already ship it, never altered here.

## Verification commands

```
cargo run -p xtask -- doc-guide verify-manifest
cargo run -p xtask -- doc-guide verify-anchors
cargo run -p xtask -- doc-guide build
cargo run -p xtask -- doc-guide test
cargo nextest run -p xtask
cargo run -p xtask -- lint
cargo run -p xtask -- fmt-check
```

All seven run headless on both `ubuntu-24.04` and `windows-2025` (TEST-D43), matching every other `xtask`-scoped verification command already established by M8-B07.

## Interfaces

**Needs from M10-B01/M10-B02/M10-B05 (all already merged before this blueprint starts, per its own Prerequisites):** every type, method signature, and file path this blueprint's own chapter content cites — `ClientRegistryBuildContext`'s six methods, `ClientHudAnchor`, `ClientTickContext`, `mods/example-ores/client/src/lib.rs`'s own completed content, and its two proof tests. This blueprint adds nothing to any of those three blueprints' own surfaces and requests no further edit from any of them.

**Provides to future work:** the concrete example, for any future blueprint that completes `register_entity_renderer`'s or `register_gui_screen`'s own live ABI payload (both named as open gaps above), of exactly which section of Chapter 11 that future blueprint's own documentation task must update — "The other four extension points, honestly" is the one section of this chapter a future capability-completing blueprint should expect to revisit, moving the relevant bullet out of that section and into "See it running" once a second reference-mod proof exists for it. This is a forward pointer, not a task this blueprint itself defers incompletely — every claim this blueprint's own chapter makes is true as of this blueprint's own drafting.

## Open Questions

- Whether a future revision of `docs/mod-guide/src/00-introduction.md`'s own "flagship reference mod" paragraph (M8-B07, currently reads "Several chapters below link to it directly" without enumerating which) should name Chapter 11 explicitly alongside whichever other chapters already link to `mods/example-ores` — left to that page's own next revision, since this blueprint's own Chapter 11 content already satisfies MOD-D51's linking requirement without the introduction page needing an edit to be *correct*, only to be more discoverable.
- Whether `register_gui_screen`/`provide_static_screen` and `register_input_binding` should gain a real reference-mod worked example (a companion action opening `pulse_crystal`'s own status as a static screen, say) in a future, small follow-on blueprint, giving Chapter 11's own "real and tested but not exercised by the shipped reference mod" bullets a second live proof the same way the material/HUD bullets already have one — left, explicitly, as a future capability, not required for this blueprint's own MOD-D52 done-bar (which needs one landed chapter with one passing backing citation, both of which this blueprint already supplies).
