# M8-B01 — `rc-mod-api`: The Isomorphic Mod API Surface

| Field | Content |
|---|---|
| ID | M8-B01 |
| Milestone | M8 — Mod API Alpha |
| Prerequisites | M0-B02 (`rc-messaging`'s `Address`/`RegionId`/`RegionMessage` shape — this blueprint's `ModMessage` mirror types must stay wire-compatible with MOD-D14's `RegionMessage::ModMessage` variant, though `rc-mod-api` never depends on `rc-messaging` itself, see Context); M0-B05 (`RcExecutorBuilder`/`SystemFactory`/`DomainGroup`/`ComponentAccessSummary` — the exact registration surface this blueprint's own `DomainGroup`/access-declaration mirror types must convert into, cleanly, from a later `rc-scheduler` blueprint); M3-B01 (`BlockBehavior`/`BlockBehaviorRegistry`/`UpdateContext`/`TickPriority`/`Direction` — the exact seam this blueprint's `ModBlockBehavior` wraps); M4-B01 (`RegistryEntryId`/`EntityKind`/`ItemStackRecord` — the exact seam this blueprint's item-registration surface wraps). **None of these four is a literal Cargo dependency of `rc-mod-api`** (WS-D3 rule 4 — `rc-mod-api` is a leaf; the "prerequisite" relationship here is *shape-mirroring consistency*, not a `use` edge: every mirror type this blueprint defines must convert losslessly, in a single free function, in whichever future blueprint owns both sides of the conversion). M2-B01 (`PalettedContainer<T>`'s Direct-palette bit-width rule) and M6-B07 (the composition-root's future mod-loading slot) are consulted context, not build prerequisites. |
| Implements | MOD-D1–D32 (06-modding-api.md, in full — this blueprint is the concrete Rust/WIT realization of every decision whose *contract* is fixed at the API-surface level; decisions whose *mechanism* lives engine-side — MOD-D2's wasmtime embedding, MOD-D25's fuel/epoch limits, MOD-D26's hash-allowlist check, MOD-D28's filesystem-watch reload, MOD-D30's precompile cache — are restated as context this crate's types must stay compatible with, not implemented here, since they belong to `rc-mod-host`, a separate M8 blueprint); ARCH-D4 (dynamic component registration, wrapped by `component.rs`); ARCH-D8 (five domain groups, mirrored by `access.rs`'s `DomainGroup`); ARCH-D13 (Stage 4 sequential collapse, restated as `TickPriority`'s binding constraint); WS-D3 rule 4 (leaf-crate dependency ceiling, restated and reconciled in Context) |
| Crates touched | `rc-mod-api` (`crates/mod-api/`) only |
| Estimated scope | L (this blueprint is a deliberate exception to the ~800-line sizing guideline — it is the single, complete, public-API-defining blueprint every other M8 blueprint compiles against per the milestone's own framing; splitting it would leave later M8 blueprints without a coherent, load-bearing contract to build on) |

## Goal & Done definition

Implement `rc-mod-api` end to end: the `.rcmod`/`manifest.toml` schema and its parser/validator (MOD-D4/D6/D8/D10–D12/D24/D31), the native-tier ABI-stability mechanism (`stabby`, MOD-D3) including the version handshake (MOD-D22), the WASM-tier canonical-ABI interface package (WIT, MOD-D2) with guest-side bindings generated via `wit-bindgen`, the isomorphic entrypoint contract (shared/server/client, MOD-D5), the `ComponentDescriptor` builder wrapping ARCH-D4 (MOD-D13), the declared-access-set types RC-Executor's conflict graph will consume (MOD-D8/D9/D10/D11/D12), the hook/event catalog for M8 alpha (lifecycle, the five ARCH-D8 tick-domain hooks, block-behavior registration wrapping M3-B01, item/registry insertion wrapping M4-B01's seam, networking, client-render-hook *registration*), the manifest-declared capability model (MOD-D24), and every diagnostic/error type. Nothing in this blueprint loads a dylib, spins up a `wasmtime::Engine`, or touches a real `bevy_ecs::World` — `rc-mod-api` is pure, dependency-minimal, ABI-and-schema-defining code; `rc-mod-host` (a separate, later M8 blueprint) is the only crate that ever instantiates any of it against a running engine.

Done when:

- [ ] `cargo build -p rc-mod-api --all-features` succeeds with zero warnings, and `cargo build -p rc-mod-api --no-default-features --features wasm-tier` / `--no-default-features --features native-tier` both succeed independently (the two tiers' code paths never accidentally depend on each other's feature-gated types).
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mod-api --all-features`.
- [ ] `cargo run -p xtask -- lint-deps` exits 0 against this blueprint's own restated, expanded Rule 4 set (Context: "Reconciling WS-D3 rule 4"), the binding CI-enforced dependency ceiling for `rc-mod-api` this blueprint fixes.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-mod-api` exits 0.
- [ ] The `wit/rc-mod-api.wit` package validates as syntactically well-formed WIT — asserted indirectly by `wit_guest.rs`'s `wit_bindgen::generate!` invocation compiling successfully under the `wasm-tier` feature (a malformed `.wit` file fails that macro at compile time).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test` under both feature combinations) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Reconciling WS-D3 rule 4 — this blueprint's binding, expanded dependency ceiling

`12-workspace-structure.md`'s prose states rule 4 as "`rc-mod-api` depends only on `rc-core` (plus `bevy_ecs` for the `ComponentDescriptor` types ARCH-D4 requires)." That prose undercounts what `12`'s own Workspace Dependency Versions table already attributes to this crate: the `wit-bindgen` line is commented `# rc-mod-host/rc-mod-api, MOD-D2` — naming `rc-mod-api` explicitly as a consumer, which the prose omits. Separately, MOD-D3 names a crate, `rc-mod-abi-native`, for the native tier's `#[stabby::stabby]`-annotated boundary types — a crate that does not exist anywhere in `12`'s closed 28-crate manifest (WS-D2: "No crate outside this list may be added without revising this document first"). This blueprint resolves both gaps the same way `12`'s own WS-D2 already resolved an analogous situation for `07-client-architecture.md`'s proposed `rc-world-model`/`rc-entity-state` crates ("deliberately not separate crates... folds into this document's existing crate... under a different name"): `rc-mod-abi-native`'s described role — the stable native-tier ABI boundary types a mod's `cdylib` compiles against — folds into `rc-mod-api` itself, because `rc-mod-api` is the *only* crate `12`'s own Crate Manifest table labels "Used by: server, client, **mod authors**." A native-tier mod author needs a leaf to compile against; `rc-mod-api` is that leaf.

**Binding, expanded Rule 4 set for `xtask lint-deps`, fixed by this blueprint:** `rc-mod-api`'s complete normal-dependency set is `{rc-core, serde, toml, thiserror}` unconditionally, plus `{bevy_ecs, stabby}` under the `native-tier` feature and `{wit-bindgen}` under the `wasm-tier` feature (Cargo `optional = true` dependencies, gated — Deliverables' `Cargo.toml`). `serde`/`toml`/`thiserror` are the minimum needed to own "the mod manifest schema" (12's own Crate Manifest text for this crate) at all — a schema crate that cannot parse or report errors on its own schema is not a schema owner. This is the authoritative CI-enforced set this blueprint's own `lint-deps` done-condition checks against; `12-workspace-structure.md` should be revised to match at its own next update (not this blueprint's job).

### MOD-D1/D2/D3 — the two tiers, restated exactly, and what each needs from this crate

**Tier 1, WASM (default, MOD-D1).** `wasmtime` 36.0.13 LTS + `wasmtime-wasi` 36.0.13 + `wit-bindgen` 0.60.0 (MOD-D2, all pinned in `12`'s `[workspace.dependencies]`). Mods compile to `wasm32-wasip2` (Rust Tier-2 target since 1.82, emits a Component-Model component directly). In-tick hook calls run under wasmtime's **synchronous** embedding (`Config::async_support(false)`); this crate has no wasmtime dependency at all (that is `rc-mod-host`'s alone) — its only WASM-tier responsibility is **shipping the canonical `.wit` interface package** (Deliverables' `wit/rc-mod-api.wit`) and **generating + re-exporting the guest-side Rust bindings** from it via `wit_bindgen::generate!`, so a WASM-tier mod's own crate never re-invokes `generate!` against its own copy of the interface — it depends on `rc-mod-api` (default features, `wasm-tier` only) and uses `rc_mod_api::guest::*` directly.

**Tier 2, native (opt-in, MOD-D1).** `stabby` 72.1.16 (MOD-D3, dual `EPL-2.0 OR Apache-2.0`, project takes `Apache-2.0`), verified (WebFetch against `docs.rs/stabby/72.1.16`, this document's own research pass) to provide: `#[stabby::stabby]` (converts an item to `extern "C"`, requires every exchanged type implement `stabby::abi::IStable`), `#[stabby::export]` (adds `#[no_mangle]` plus a generated `<fn>_stabbied` type-report verification function), ABI-stable `string`/`vec`/`option`/`result`/`boxed` modules as drop-in replacements for their `std`/`alloc` counterparts, `dynptr!`/`vtable!` macros for stable trait-object references (`&'a dyn Trait` → `DynRef<'a, vtable!(Trait)>`; `Box<dyn Trait>` → `dynptr!(Box<dyn Trait>)`), and `stabby::closure::{Call0..Call9, CallMut0..CallMut9, CallOnce0..CallOnce9}` for ABI-stable closures. **No raw `bevy_ecs` type ever crosses the dylib boundary directly** (MOD-D3's own binding text) — every native-tier public type in this blueprint that must cross that boundary is either a plain `Copy`/`#[repr(C)]`-safe scalar newtype or explicitly `#[stabby::stabby]`-annotated; nothing from `bevy_ecs` appears in any `#[stabby::stabby]` item's field list anywhere in this blueprint.

**Moderate-confidence flag, re-verify at implementation time:** `stabby::closure::Call*`/`CallMut*`'s exact generic-parameter order/arity-naming convention (this blueprint assumes `CallMut2<'a, Arg1, Arg2, Ret>`-shaped generics matching the family's documented `0..9`-arity naming) and `stabby::result`/`stabby::option`'s exact variant names (assumed `Result::Ok`/`Result::Err`, `Option::Some`/`Option::None`, mirroring `std`) should be confirmed against the installed `stabby` 72.1.16 crate's actual generated docs before `component.rs`/`entrypoint.rs`/`block_behavior.rs`'s bodies are written — no Deliverable signature's *shape* changes if a detail differs, only exact generic-parameter spelling.

### MOD-D4 — the manifest schema, exact and complete

`.rcmod` is a zip (MOD-D4, `rc-mod-host`'s concern — this crate never touches the zip container, only the `manifest.toml` text once extracted). This blueprint's own concrete, binding schema (06 names the field groups; the exact nested shape below is this blueprint's resolution of what 06 leaves open):

```toml
[mod]
id = "example_ores"                 # Identifier namespace (this blueprint's own Identifier type)
version = "0.1.0"                   # this mod's own SemVer — validated as syntactically SemVer-shaped only (see "No semver crate" below)
display_name = "Example Ores"
authors = ["Jane Modder"]
license = "MIT"

[api]
requires = "^0.1"                   # SemVer range against rc-mod-api's own MOD_API_VERSION (MOD-D21) — syntax-validated only, see below
unstable_features = []              # opts into @unstable-gated WIT imports/exports (MOD-D22)

[compat]
engine = ">=0.1.0, <0.2.0"          # optional
mc_parity = "26.2"                  # optional

[dependencies]
some_lib_mod = "^1.0"               # mod_id -> semver range (MOD-D31); resolution itself is rc-mod-host's job

[entrypoints]
tier = "wasm"                       # "wasm" | "native" — MOD-D1's mutual exclusivity
shared = "shared"                   # optional: WASM-tier only, a source-module name (MOD-D4: "not itself a runtime artifact")
server = "rc-mod-server"            # WASM tier: the WIT world name this artifact exports; native tier: the exported native symbol name
client = "rc-mod-client"            # same, client side

[entrypoints.native."x86_64-pc-windows-msvc"]
server = true                        # this triple's one dylib exports the server-entrypoint symbol
client = false

[capabilities]
filesystem = false
network = false
network_channels = ["example_ores:sync"]   # MOD-D20/D24

[[capabilities.components]]
hook = "example_ores:block_redstone"        # which [[hooks]] entry this access belongs to (this blueprint's own required field — 06's {name, access, group} shape is necessary but not sufficient once a manifest may declare more than one hook; see MOD-D8 restatement below)
name = "minecraft:block_state"
access = "read"
group = "block_redstone"

[[capabilities.components]]
hook = "example_ores:block_redstone"
name = "example_ores:ore_charge"
access = "write"
group = "block_redstone"

[[hooks]]
id = "example_ores:block_redstone"          # this mod's own hook id (namespace MUST equal [mod].id, validated)
group = "block_redstone"                     # one of the five ARCH-D8 groups — at most one [[hooks]] entry per group per mod (Context: "why one hook per group")
priority = "normal"                          # Stage-4 (block_redstone) only, MOD-D11 — TickPriority; ignored/rejected for other groups
before = []
after = ["native:block_redstone"]
exclusive_world_access = false               # MOD-D12
```

**Schema notes, each restating a decision by ID:**
- `[mod].id` and every `Identifier`'s namespace anywhere in this manifest that names *this* mod's own content must equal `[mod].id` exactly (MOD-D6's own registry-ownership implication) — checked by `validate_manifest`, not `parse_manifest` (Context: "parse vs. validate").
- `[[capabilities.components]]` is 06's own literal TOML path (MOD-D8: "carries a `[[capabilities.components]]` manifest entry: `{ name, access, group }`"); this blueprint adds the `hook` field because 06's own text names *the hook* as the thing that "carries" the entry, and one manifest may declare more than one hook — an entry without a `hook` field would be ambiguous the moment a second hook exists. This is the minimal necessary extension of 06's own stated shape, cited and not silent.
- **Why at most one `[[hooks]]` entry per `group` per mod (M8-alpha constraint):** the WIT `tick-hooks` interface (Deliverables' `wit/rc-mod-api.wit`) exports exactly one function per ARCH-D8 group (`on-block-redstone-tick`, etc., MOD-D8's "hooks are namespaced by `01`'s five domain groups" — API Surface section). A WASM-tier component's compiled shape has exactly one implementation slot per named export; native tier's single `dyn ServerModEntry::on_tick_hook(hook_id, ctx)` dispatch method (Deliverables' `entrypoint.rs`) is likewise one call per declared hook. `validate_manifest` rejects a manifest declaring two `[[hooks]]` entries with the same `group`.
- **Why `[mod].id` is `ModId`, not `Identifier`.** `[mod].id`'s worked value (`"example_ores"`) has no `path` half — a mod's own identity is a bare namespace-charset string, never a `namespace:path` pair. Typing it as `Identifier` would force every manifest to either invent a meaningless path segment or make `Identifier::parse` special-case a colonless input everywhere it is used (including inside `capabilities.components[].name`, where a colonless value must stay a hard error). This blueprint's binding resolution: `[mod].id: ModId` (Deliverables' `identifier.rs`) is its own distinct type, reusing `Identifier`'s namespace charset unchanged; every other `Identifier` in the manifest whose namespace names *this* mod (a hook's own `id`, per MOD-D6's ownership rule) is validated against this `ModId`'s string value directly, never round-tripped through `Identifier::parse`.
- **No `semver` crate — syntax validation only, not range satisfaction.** `12`'s `[workspace.dependencies]` table pins no `semver` crate anywhere; this blueprint adds none (Constraints (b)). `[api].requires`, `[compat].engine`, and every `[dependencies]` value are validated only for a minimal, hand-rolled syntactic shape (Deliverables' `manifest.rs`'s `DependencyRange::parse`: every character must be in `[0-9.^~<>=, *]`, non-empty — an optional leading `^`/`~`/`>=`/`<`/`=` comparator, one or more `.`-separated numeric or `*` components, optionally comma-chained) — real range-*satisfaction* logic (needed by MOD-D31's actual dependency resolution and MOD-D21's actual API-compatibility check) is deferred to whichever future `rc-mod-host` blueprint implements the resolver, flagged here rather than silently assumed solved.

### MOD-D6/D8/D9/D10/D11/D12 — declared access, ordering, and the exclusive escape hatch

Restated exactly: every hook a mod registers into one of `01`'s five domain groups carries a manifest-declared `{ name, access: read|write, group }` entry (MOD-D8); RC-Executor resolves each declared component *name* to a real `ComponentId` at RegistryBuild and builds one host-side `ModSystemShim` per hook, registered into `RC-Executor`'s startup conflict graph identically to a native `SystemHandle` (M0-B05's own `RcExecutorBuilder::register_system(group, factory, structural_writes) -> SystemId` is the exact mechanism a future `rc-scheduler`/`rc-mod-host` blueprint calls — this blueprint does not call it, having no dependency on `rc-scheduler`). Declared-access enforcement is host-performed and access-denying for the WASM tier (a genuine sandboxing property) and honesty-based for the native tier (MOD-D9) — this crate's types carry the *declaration*, never an enforcement mechanism (that lives entirely in `rc-mod-host`/`rc-scheduler`). Ordering (`before`/`after`, resolving into `order_tag`) and an unresolvable conflict being a hard boot-time error reuse `ARCH-D8`'s own policy verbatim (MOD-D10). A Stage-4 (`block_redstone`) hook inherits `ARCH-D13`'s mandatory full sequentiality unconditionally, with no declared-access-based parallelism ever offered (MOD-D11) — restated as this crate's own binding rule: `TickPriority` selection is meaningful, and required, only for `group = BlockRedstone`; `validate_manifest` rejects a non-`BlockRedstone` hook that sets `priority`. Exclusive world access (MOD-D12) is the explicit, discouraged, logged, metriced opt-in this crate's `exclusive_world_access: bool` field carries verbatim — the full-drain-barrier mechanism itself is `rc-scheduler`'s.

**`TickPriority`'s seven variants, reconciling MOD-D11 against M3-B01.** MOD-D11's own text describes "vanilla's four tick-priority levels (`EXTREMELY_HIGH`..`NORMAL`) or a fifth, mod-reserved tier ordered strictly after `NORMAL`" — that four-level framing traces to `01-server-architecture.md`'s ARCH-D13, which itself only lists four. `M3-B01` (a prerequisite of this blueprint, already committed), restating `05-game-mechanics.md`'s own scheduled-tick engine, fixed vanilla's **real** seven-level `TickPriority` enum (`ExtremelyHigh, VeryHigh, High, Normal, Low, VeryLow, ExtremelyLow`, in that declaration/ordinal order) — matching real vanilla behavior, which already has three levels strictly after `Normal` (`Low`/`VeryLow`/`ExtremelyLow`). This blueprint's binding resolution: no synthetic "mod-reserved" eighth tier is invented — a mod hook wanting strictly-after-`NORMAL` FIFO ordering simply selects `TickPriority::ExtremelyLow` (or any of the three below-`Normal` levels, matching whatever native-block-like behavior it is replicating), reusing M3-B01's already-real seven-level enum one-for-one. `rc-mod-api`'s own `TickPriority` (Deliverables' `access.rs`) is declared in the **identical** seven-variant, identical declaration order as `rc_mechanics::scheduled_tick::TickPriority` specifically so a future `rc-mechanics` blueprint's `From<mod_api::TickPriority> for rc_mechanics::TickPriority` conversion (which must live in `rc-mechanics`, the only crate depending on both types per the dependency graph's `mech --> modapi` edge) is a trivial one-to-one match, never a lossy remap.

**`DomainGroup`'s five variants** mirror `rc_scheduler::DomainGroup` (M0-B05) one-for-one, same names, same conceptual mapping to the fixed 11-stage pipeline (`BlockRedstone`→Stage 4, `AiPhysics`→Stage 6, `Lighting`→Stage 8, `ChunkSerialize`→Stage 9, `NetCodec`→Stage 11). A future `rc-scheduler` blueprint (which depends on `rc-mod-host`, hence transitively on this crate) supplies the trivial `From<mod_api::DomainGroup> for rc_scheduler::DomainGroup` conversion — `rc-mod-host` itself cannot, since it does not depend on `rc-scheduler` (the dependency graph's `sched --> modhost` edge runs the other way).

### MOD-D6 — registry insertion, id-space partitioning, and M2-B01's palette-sizing implication

Registries are `Identifier`-keyed, populated only during RegistryBuild, frozen immutable thereafter (MOD-D6). This blueprint's `RegistryKind` covers **only `Block`/`Item`** for M8 alpha — the milestone's acceptance criteria name exactly "one new block type... one new item"; every other MOD-D6-listed kind (biomes, dimension types, enchantments, entity types, recipes, commands, worldgen types) is a later milestone's registration contract, owned by its respective domain doc (04/05), not fixed here.

**Id allocation must be dense and sequential, never a sparse reserved range — restated from M2-B01's own palette-sizing rule.** `rc-chunk-storage`'s `PalettedContainer<T>`'s `Direct` palette state is sized at a **fixed bit width for the whole target registry** — `bits = ceil(log2(registry_size))`, computed once over the *entire* frozen registry, "not a function of this container's own distinct-value count" (M2-B01's own restatement of WORLD-D2). If a mod's first new block state were assigned an id from a large, sparsely-reserved address space far above vanilla's own highest id (an approach this blueprint's own early drafting considered and rejected), `registry_size` for bit-width purposes would jump to that sparse id's own magnitude the instant *any* mod block existed — inflating every `Direct`-palette section's bit width for the rest of the process's life, even for a world with a single mod block installed. This blueprint's binding resolution instead: mod-contributed block/item ids are assigned **densely, immediately continuing after the pinned vanilla registry's own highest generated id** — the first mod's first new entry gets `vanilla_max_id + 1`, the next gets `+ 2`, and so on, in MOD-D31's resolved dependency load order, then manifest declaration order within one mod. `Direct`-palette bit width therefore only grows once total *installed* mod content approaches vanilla's own registry scale, exactly the efficient behavior WORLD-D2's formula rewards.

**Named, binding limitation (Open Questions):** because allocation order depends on the installed mod set's load order, adding, removing, or reordering a mod changes every *subsequently*-loaded mod's ids from that point on. This blueprint does not solve world-persistence stability across a changing mod set — a future blueprint (most plausibly whichever one first needs to load a saved world against a different mod set than created it) must add an explicit id-remapping-on-load step; this crate's own `DenseIdAllocator` (Deliverables' `registry.rs`) is a pure, mod-set-order-agnostic primitive that such a remap layer can reuse unchanged.

### MOD-D13 — `ComponentDescriptor`, POD constraint, and why this crate does not expose `bevy_ecs::component::ComponentDescriptor` directly

ARCH-D4's own summary ("size, alignment, drop fn — no static Rust type required") undersells the installed `bevy_ecs` 0.19.1 API's actual shape (verified, WebFetch against `docs.rs/bevy_ecs/0.19.1`, this document's own research pass): `ComponentDescriptor::new_with_layout` is `unsafe`, takes seven parameters (`name`, `storage_type: StorageType`, `layout: std::alloc::Layout`, `drop: Option<for<'a> unsafe fn(OwningPtr<'a>)>`, `mutable: bool`, `clone_behavior: ComponentCloneBehavior`, `relationship_accessor: Option<RelationshipAccessorInitializer>`), several of which are `bevy_ecs`-internal-shaped types with no ABI-stability guarantee of their own and no obvious mapping for a WASM-tier mod (which never sees `bevy_ecs` at all). This blueprint's binding resolution: `rc-mod-api` defines its own stable `ModComponentDescriptor` value type (Deliverables' `component.rs`) built through a validating `ComponentDescriptorBuilder`; translating one into a real `bevy_ecs::component::ComponentDescriptor` via `new_with_layout` is `rc-mod-host`'s job alone (the only crate with both `bevy_ecs` and this crate's types in scope with engine-internal knowledge of `StorageType`/`ComponentCloneBehavior`/relationship handling) — exactly the kind of internal-detail absorption WS-D3 rule 4's "nothing about the engine's... internals leaks into it" already intends. **Moderate-confidence flag:** `new_with_layout`'s exact seven-parameter shape should be re-verified against the installed `bevy_ecs` 0.19.1 docs before `rc-mod-host`'s own future blueprint writes the translation — it does not change anything in *this* blueprint's Deliverables, which never call it.

MOD-D13's POD/self-contained constraint (no raw pointer, engine handle, or `NodeId`/host-identity field; WASM-tier payloads additionally must be WIT-record-representable) is a **binding authorial discipline this crate documents and structurally nudges toward** (the builder accepts only a `size`/`align`/optional-`drop-fn`, giving a mod author no API surface through which to accidentally embed a live reference) but does **not** mechanically scan or enforce at the byte level — it cannot: a layout-only descriptor carries no type information to inspect. This is stated plainly, not glossed over, matching MOD-D9's own "declared access... honesty-based" precedent for exactly this class of trust boundary.

**`ComponentId`-across-ABI rule (MOD-D8's own phrase), restated exactly.** `ARCH-D4`'s rationale: "a `ComponentId` obtained from a descriptor crosses the dylib ABI boundary safely where a monomorphized generic type cannot" — because `bevy_ecs::component::ComponentId` is a small, `Copy`, non-generic newtype over an integer index. `bevy_ecs::component::ComponentId` itself is not `#[stabby::stabby]`-annotated (it is a third-party type outside this project's control) and therefore cannot cross the native FFI boundary directly without an ABI-stability guarantee this project does not own. This blueprint's `ModComponentId(pub u64)` (Deliverables' `abi.rs`) is the ABI-safe mirror that *does* cross; `rc-mod-host` converts between the two (`bevy_ecs::component::ComponentId`'s own public index accessor/constructor pair — moderate-confidence, re-verify at implementation time) — never this crate, which has no way to construct a real `ComponentId` in the first place (it never touches a `World`). The WASM tier carries the identical value as a plain WIT `u64` (Deliverables' `wit/rc-mod-api.wit`), so both tiers agree on the wire representation without either depending on the other's mechanism.

### MOD-D5/D18/D19/D20 — the entrypoint contract and the hook catalog for M8 alpha

**Isomorphic split (MOD-D5), restated exactly.** The server process loads only `manifest.toml` + the server entry; the client process loads only the client entry; neither ever opens the other side's artifact — "not loading-then-ignoring." This crate's `ServerModEntry`/`ClientModEntry` traits are therefore two **separate** traits, never one trait with optional methods a loader "skips" — a purely-visual mod simply never implements `ServerModEntry` at all, and its `.rcmod`'s `[entrypoints]` table has no `server` key, so the server-side loader (`rc-mod-host`) never even attempts to resolve one.

**Hook catalog structure (06's API Surface section, restated), fixed concretely for M8 alpha:**

| Category | This crate's mechanism | Wraps |
|---|---|---|
| Lifecycle | `SharedModInit`/`ServerModEntry::on_registry_build`/`on_server_init`/`on_server_shutdown`, `ClientModEntry::on_client_registry_build`/`on_client_init` | MOD-D6's RegistryBuild timing |
| Tick-domain (5 ARCH-D8 groups) | `ServerModEntry::on_tick_hook(hook_id, ctx)` (native, one dispatch method, id-routed); `wit/rc-mod-api.wit`'s `tick-hooks` interface (WASM, one export per group — sparse, MOD-D8's "zero per-tick cost for unused hooks") | MOD-D8's `ModSystemShim` mechanism (a future `rc-scheduler`/`rc-mod-host` blueprint builds the shim itself; this crate only fixes the mod-facing call shape) |
| Block-behavior registration | `ModBlockBehavior` trait (four methods, mirroring `BlockBehavior` one-for-one) + `RegistryBuildContext::register_block_behavior` | M3-B01's `BlockBehaviorRegistry`/`BlockBehavior`/`UpdateContext` |
| Item registration | `RegistryBuildContext::register_item` | M4-B01's `ItemStackRecord`/`RegistryEntryId` seam |
| Networking | `ServerModEntry::on_channel_message`/`on_mod_message`, `RegistryBuildContext::register_channel` | MOD-D20 (Custom Payload channel dispatch), MOD-D14 (`ModMessage`) |
| Client-only | `ClientRegistryBuildContext`'s five `register_*` methods | MOD-D18, **registration only** at M8 (no renderer exists — PLAN-D2; M8's own acceptance criterion 3: "client-side render hook visual verification explicitly deferred to M10... registered + headless-verified only") |

**Why block-behavior registration is a *separate* mechanism from a generic tick-domain hook, not an instance of one.** A mod that gives one new block custom `on_scheduled_tick`/`on_neighbor_changed` behavior does not need its own `ModSystemShim`-style Stage-4 hook with a declared `Access<ComponentId>` set at all — it needs its `ModBlockBehavior` implementation registered into the **same** `BlockBehaviorRegistry` every native block already dispatches through (M3-B01: `BlockBehaviorRegistry::register_range`/`register_one`, resolved by `resolve(state) -> &Arc<dyn BlockBehavior>` inside Stage 4's already-existing, already-sequential dispatch loop). This is exactly the task's own framing ("block-behavior registration — M3-B01's registry seam wrapped — a modded block's custom tick via scheduled ticks") and is strictly cheaper than a generic hook: no new conflict-graph entry, no new `[[hooks]]` manifest declaration, Stage 4's existing single-worker sequential collapse (ARCH-D13) already covers it for free.

**`ModBlockBehavior`/`ModUpdateContext`, the ABI-safe mirror of M3-B01's `BlockBehavior`/`UpdateContext<'a>`.** M3-B01's real `UpdateContext<'a>` holds live Rust references (`&'a mut dyn BlockWorldAccess`, `&'a mut NeighborUpdateEngine`, etc.) that cannot cross a native dylib boundary or a WASM canonical-ABI boundary at all. This blueprint's `ModUpdateContext` instead bundles `stabby`-safe closures (native tier) — one per `UpdateContext` method (`get_block`, `set_block`, `schedule_block_tick`, `schedule_fluid_tick`, `emit_block_event`) — constructed host-side by `rc-mod-host` (which alone has a real `UpdateContext<'a>` in scope) for the duration of exactly one callback, and torn down immediately after. `set_block` remains "the only way a behavior mutates block state" (M3-B01's own rule, preserved verbatim through the mirror) — `ModUpdateContext` exposes no other block-mutation path. The WASM tier's equivalent is the canonical-ABI import functions in `wit/rc-mod-api.wit`'s `world-query`/`registry-build` interfaces, called directly by the guest (no closure bundle needed — WIT imports are already host functions).

### MOD-D14/D15/D16/D17 — cluster compatibility, restated as this crate's own binding constraints (no new mechanism)

This crate adds **zero** new cross-partition mechanism — every cluster-facing rule here is a direct, unmodified consequence of `01`/`13`'s already-decided machinery, restated so a mod author reading only this crate's docs (never `06` itself) still gets the complete rule: (MOD-D15) no type in this crate's public surface anywhere carries a `NodeId`, hostname, or PID — the only addressable values a mod ever sees are `Identifier`s and this crate's own opaque `ModAddress` string wrapper (mirroring `rc_messaging::Address` structurally without depending on it — Deliverables' `entrypoint.rs`); (MOD-D16) `on_mod_message`/`on_channel_message` are the only way a mod ever observes a cross-partition effect, always as a later, asynchronous callback, never a blocking call — no method anywhere in this crate's traits returns a value obtained by waiting on another partition; (MOD-D17) nothing in `ModBlockBehavior`/`ServerModEntry` holds authoritative state across calls beyond what the engine re-supplies each time (every context type is constructed fresh, per-call, by the host) — a mod author who wants state that survives a tick must register it as an ECS component via `register_component`, never as a field on their own entrypoint struct that the engine cannot see.

`ModMessage`'s wire shape mirrors `rc_messaging::RegionMessage::ModMessage { mod_id, channel, payload }` (MOD-D14) exactly, field-for-field, without a dependency edge: `mod_id: Identifier`, `channel: String`, `payload: Vec<u8>` (native tier, plain owned types — the message crosses `rc-mod-host`'s own boundary into `rc-messaging`, never this crate's); WASM tier carries the identical three fields as WIT `string`/`string`/`list<u8>` params on `on-mod-message`.

### MOD-D24 — capabilities, M8-alpha enforcement scope stated honestly

`[capabilities]`'s `filesystem`/`network`/`network_channels` fields (Deliverables' `capabilities.rs`) are **declaration and manifest-validation only** at this crate's own scope: `validate_manifest` checks that every channel a hook references via `register_channel`/`on_channel_message` appears in `network_channels`, and that a declared-but-unused capability is not itself an error (an author may over-declare). The actual **enforcement** mechanisms — WASI's no-ambient-authority instantiation-time capability grant for the WASM tier, the honesty-based (non-)enforcement for the native tier, and the operator-approval gate for `network` specifically (MOD-D24's own text: "requires an explicit server-operator approval step at mod-install time") — are entirely `rc-mod-host`'s runtime job, not implemented, simulated, or even stubbed by this blueprint. Stated plainly so no reader of this blueprint alone mistakes manifest validation for a security boundary.

### Parse vs. validate — the two-phase contract every consumer of this crate relies on

`parse_manifest(toml_text: &str) -> Result<ModManifest, ManifestError>` performs pure syntactic deserialization plus every field-local check that needs no *cross*-field knowledge (a malformed `Identifier`, an unparseable `toml`, an out-of-enum `access`/`group`/`tier` string). `validate_manifest(&ModManifest) -> Result<(), Vec<ManifestValidationError>>` performs every check that needs the *whole* manifest at once (namespace-matches-`[mod].id`, at-most-one-hook-per-group, every `capabilities.components` entry's `hook` field resolves to a declared `[[hooks]]` id, every `before`/`after` reference resolves to a declared hook id or a valid `native:<domain>` marker, `priority` set only for `group = block_redstone`, every `network_channels`-adjacent consistency check). `rc-mod-host` calls both, in order, always — this crate does not fuse them into one function, because a caller that wants to report *every* validation problem at once (rather than stopping at the first parse error) needs the two phases separable, and `validate_manifest`'s `Vec`-returning signature (collect-all, not fail-fast) is deliberate for exactly that reason.

## Deliverables

### `crates/mod-api/Cargo.toml`

```toml
[package]
name = "rc-mod-api"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
serde = { workspace = true }
toml = { workspace = true }
thiserror = { workspace = true }
bevy_ecs = { workspace = true, optional = true }
stabby = { workspace = true, optional = true }
wit-bindgen = { workspace = true, optional = true }

[dev-dependencies]
proptest = { workspace = true }

[features]
default = ["wasm-tier"]
wasm-tier = ["dep:wit-bindgen"]
native-tier = ["dep:bevy_ecs", "dep:stabby"]
```

### `crates/mod-api/src/lib.rs`

```rust
//! `rc-mod-api` — the isomorphic mod API contract: the `.rcmod`/`manifest.toml`
//! schema and parser/validator, the native-tier `stabby` ABI-stable boundary types
//! and version handshake, the WASM-tier canonical-ABI WIT interface package and its
//! generated guest bindings, the isomorphic entrypoint contract, the
//! `ComponentDescriptor` builder wrapping ARCH-D4, the declared-access-set types
//! RC-Executor's conflict graph consumes, the hook/event catalog for M8 alpha, the
//! registry-insertion surface, and the capability-declaration types (06-modding-
//! api.md's MOD-D1-D32, restated in full — see this file's own blueprint's Context
//! section for every decision-ID-cited resolution). A leaf: `{rc-core, serde, toml,
//! thiserror}` unconditionally, plus `{bevy_ecs, stabby}` under `native-tier` and
//! `{wit-bindgen}` under `wasm-tier` (this blueprint's own expanded, CI-enforced
//! Rule 4 set — see Context).

mod identifier;
mod manifest;
mod access;
mod capabilities;
mod geometry;

pub use identifier::{Identifier, IdentifierError, ModId};
pub use manifest::{
    ApiSection, CompatSection, DependencyRange, EntrypointsSection,
    ManifestError, ManifestValidationError, ModManifest, ModSection, ModTier,
    NativeTripleSupport, parse_manifest, validate_manifest,
};
pub use access::{
    AccessKind, ComponentAccessDecl, DomainGroup, HookDecl, HookOrderRef, NativeDomainMarker,
    TickPriority,
};
pub use capabilities::CapabilityDecl;
pub use geometry::{ModBlockPos, ModDirection};

#[cfg(feature = "native-tier")]
mod abi;
#[cfg(feature = "native-tier")]
mod component;
#[cfg(feature = "native-tier")]
mod registry;
#[cfg(feature = "native-tier")]
mod block_behavior;
#[cfg(feature = "native-tier")]
mod entrypoint;

#[cfg(feature = "native-tier")]
pub use abi::{ABI_HANDSHAKE_SYMBOL, AbiHandshakeFn, MOD_API_VERSION, ModAbiVersion, ModComponentId};
#[cfg(feature = "native-tier")]
pub use component::{ComponentDescriptorBuilder, ComponentDescriptorError, ModComponentDescriptor};
#[cfg(feature = "native-tier")]
pub use registry::{
    BlockRegistration, DenseIdAllocator, ItemRegistration, ModBlockStateId, ModItemId,
    RegistryKind,
};
#[cfg(feature = "native-tier")]
pub use block_behavior::{ModBlockBehavior, ModUpdateContext};
#[cfg(feature = "native-tier")]
pub use entrypoint::{
    ClientInitContext, ClientModEntry, ClientRegistryBuildContext, ModAddress, ModHookError,
    ModInitError, RegistryBuildContext, ServerInitContext, ServerModEntry, ServerShutdownContext,
    SharedInitContext, SharedModInit, TickHookContext,
};

#[cfg(feature = "wasm-tier")]
pub mod guest;
```

### `crates/mod-api/src/identifier.rs`

```rust
/// `namespace:path` resource-location identifier (MOD-D6), matching vanilla's own
/// convention. Charset restated from `minecraft.wiki`'s publicly documented resource-
/// location rule (ASSET-D18(b), a functional-fact source, no Mojang source
/// consulted): `namespace` matches `[a-z0-9_.-]+`, `path` matches `[a-z0-9_./-]+`.
/// This type performs no ownership check (that a namespace equals a particular mod's
/// `[mod].id`) — that is `validate_manifest`'s job (Context: "parse vs. validate"),
/// since `Identifier` alone has no notion of "which mod is asking."
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Identifier {
    namespace: String,
    path: String,
}

impl Identifier {
    pub fn new(namespace: impl Into<String>, path: impl Into<String>) -> Result<Self, IdentifierError>;
    /// Parses `"namespace:path"`. Exactly one `:` is expected; the first is the
    /// separator (a `path` may itself never contain `:`, so this is unambiguous).
    pub fn parse(s: &str) -> Result<Self, IdentifierError>;
    pub fn namespace(&self) -> &str;
    pub fn path(&self) -> &str;
}

impl std::fmt::Display for Identifier {
    /// `"namespace:path"` — the exact inverse of `parse`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl std::str::FromStr for Identifier {
    type Err = IdentifierError;
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

/// A bare mod identity string — `[mod].id`'s own type. **Not** an `Identifier`
/// (namespace:path): a mod id has no `path` half, and requiring one would be
/// meaningless for `[mod].id` specifically (Context: "why `[mod].id` is `ModId`, not
/// `Identifier`"). Reuses `Identifier`'s own namespace charset (`[a-z0-9_.-]+`)
/// unchanged — every other `Identifier` elsewhere in this manifest whose namespace
/// names *this* mod is validated against this value's own string, not re-parsed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ModId(String);

impl ModId {
    pub fn new(s: impl Into<String>) -> Result<Self, IdentifierError>;
    pub fn as_str(&self) -> &str;
}

impl std::fmt::Display for ModId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdentifierError {
    #[error("identifier {0:?} is missing the ':' namespace separator")]
    MissingSeparator(String),
    #[error("identifier namespace {0:?} contains a character outside [a-z0-9_.-]")]
    InvalidNamespaceChar(String),
    #[error("identifier path {0:?} contains a character outside [a-z0-9_./-]")]
    InvalidPathChar(String),
    #[error("identifier namespace or path is empty in {0:?}")]
    EmptyComponent(String),
}
```

### `crates/mod-api/src/access.rs`

```rust
use crate::Identifier;

/// Mirrors `rc_scheduler::DomainGroup` (M0-B05) one-for-one — same five variants,
/// same conceptual stage mapping. A future `rc-scheduler` blueprint (the only crate
/// depending on both this type and `rc_scheduler::DomainGroup`) supplies
/// `impl From<DomainGroup> for rc_scheduler::DomainGroup`, a trivial match — not
/// implemented here (Context: "DomainGroup's five variants").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainGroup { BlockRedstone, AiPhysics, Lighting, ChunkSerialize, NetCodec }

/// Mirrors `rc_mechanics::scheduled_tick::TickPriority` (M3-B01) one-for-one, same
/// seven variants in the same ascending-priority declaration order (so
/// `#[derive(Ord)]` already matches). Meaningful only for a hook declared against
/// `DomainGroup::BlockRedstone` (MOD-D11) — `validate_manifest` rejects any other
/// group declaring a `priority`. Context: "TickPriority's seven variants,
/// reconciling MOD-D11 against M3-B01."
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TickPriority { ExtremelyHigh, VeryHigh, High, Normal, Low, VeryLow, ExtremelyLow }

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessKind { Read, Write }

/// One `[[capabilities.components]]` manifest entry (MOD-D8), extended with the
/// `hook` field this blueprint's own schema resolution requires (Context: schema
/// notes).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ComponentAccessDecl {
    pub hook: Identifier,
    pub name: Identifier,
    pub access: AccessKind,
    pub group: DomainGroup,
}

/// A `before`/`after` ordering target (MOD-D10): another mod's hook id, or a
/// reserved `native:<domain>` marker for the corresponding native engine stage.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum HookOrderRef {
    Hook(Identifier),
    NativeDomain(NativeDomainMarker),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NativeDomainMarker { BlockRedstone, AiPhysics, Lighting, ChunkSerialize, NetCodec }

/// One `[[hooks]]` manifest entry — this blueprint's own concrete schema
/// resolution of MOD-D8/D10/D11/D12 (Context: "schema notes"). `id`'s namespace
/// must equal the owning mod's `[mod].id` (validated, not parsed). At most one
/// entry per `group` per mod (validated — Context: "why one hook per group").
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct HookDecl {
    pub id: Identifier,
    pub group: DomainGroup,
    #[serde(default)]
    pub priority: Option<TickPriority>,
    #[serde(default)]
    pub before: Vec<HookOrderRef>,
    #[serde(default)]
    pub after: Vec<HookOrderRef>,
    #[serde(default)]
    pub exclusive_world_access: bool,
}
```

### `crates/mod-api/src/capabilities.rs`

```rust
use crate::Identifier;

/// `[capabilities]` (MOD-D24). Declaration/validation only at this crate's own
/// scope — no enforcement mechanism lives here (Context: "capabilities, M8-alpha
/// enforcement scope stated honestly").
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct CapabilityDecl {
    #[serde(default)]
    pub filesystem: bool,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub network_channels: Vec<Identifier>,
    #[serde(default, rename = "components")]
    pub components: Vec<crate::access::ComponentAccessDecl>,
}
```

### `crates/mod-api/src/geometry.rs`

```rust
/// ABI-safe mirror of `rc_core::BlockPos` (Context: "ModBlockBehavior/
/// ModUpdateContext"). `rc-mod-api` depends on `rc-core` unconditionally (WS-D3 rule
/// 4), so the conversions below are free, direct, and defined here rather than
/// deferred to a future crate.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "native-tier", stabby::stabby)]
pub struct ModBlockPos { pub x: i32, pub y: i32, pub z: i32 }

impl From<rc_core::BlockPos> for ModBlockPos {
    fn from(p: rc_core::BlockPos) -> Self;
}
impl From<ModBlockPos> for rc_core::BlockPos {
    fn from(p: ModBlockPos) -> Self;
}

/// Mirrors M3-B01's `Direction` one-for-one (west, east, north, south, down, up —
/// vanilla's own post-placement neighbor-fan-out order, ARCH-D13).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "native-tier", stabby::stabby)]
pub enum ModDirection { West, East, North, South, Down, Up }
```

### `crates/mod-api/src/manifest.rs`

```rust
use crate::access::{ComponentAccessDecl, DomainGroup, HookDecl, HookOrderRef, NativeDomainMarker};
use crate::capabilities::CapabilityDecl;
use crate::{Identifier, ModId};

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct ModSection {
    pub id: ModId,
    pub version: String,
    pub display_name: String,
    #[serde(default)]
    pub authors: Vec<String>,
    pub license: String,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct ApiSection {
    pub requires: DependencyRange,
    #[serde(default)]
    pub unstable_features: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct CompatSection {
    #[serde(default)]
    pub engine: Option<DependencyRange>,
    #[serde(default)]
    pub mc_parity: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModTier { Wasm, Native }

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct NativeTripleSupport { #[serde(default)] pub server: bool, #[serde(default)] pub client: bool }

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct EntrypointsSection {
    pub tier: ModTier,
    #[serde(default)]
    pub shared: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub native: std::collections::BTreeMap<String, NativeTripleSupport>,
}

/// A syntax-validated (never range-satisfaction-validated — Context: "No `semver`
/// crate") SemVer-range-shaped string: `[mod].version`, `[api].requires`,
/// `[compat].engine`, and every `[dependencies]` value all parse through this type.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DependencyRange(String);

impl DependencyRange {
    /// `Err` iff the string contains a character outside `[0-9.^~<>=, *]` or is
    /// empty — a minimal, deliberately permissive syntax gate, not a full SemVer
    /// grammar (no `semver` crate is pinned in the workspace, Constraints (b)).
    pub fn parse(s: &str) -> Result<Self, ManifestError>;
    pub fn as_str(&self) -> &str;
}

/// The fully parsed manifest (post `parse_manifest`, pre `validate_manifest` —
/// Context: "parse vs. validate"). Field names match the TOML table names 1:1
/// except `mod_` (`mod` is a Rust keyword) and `hooks` (top-level array-of-tables,
/// not nested under any of the above sections).
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct ModManifest {
    #[serde(rename = "mod")]
    pub mod_: ModSection,
    pub api: ApiSection,
    #[serde(default)]
    pub compat: CompatSection,
    #[serde(default)]
    pub dependencies: std::collections::BTreeMap<String, DependencyRange>,
    pub entrypoints: EntrypointsSection,
    #[serde(default)]
    pub capabilities: CapabilityDecl,
    #[serde(default)]
    pub hooks: Vec<HookDecl>,
}

/// Parses `manifest.toml`'s text into a `ModManifest`. Syntactic-only (Context:
/// "parse vs. validate") — every `Identifier`/`DependencyRange`/enum field is
/// checked for well-formedness by `serde`'s own deserialization path (their
/// `Deserialize` impls reject a malformed value), but no cross-field consistency
/// check runs here.
pub fn parse_manifest(toml_text: &str) -> Result<ModManifest, ManifestError>;

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("failed to parse manifest.toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("dependency range {0:?} contains a character outside the syntactic SemVer-range charset")]
    InvalidDependencyRange(String),
}

/// Every cross-field consistency problem found, collected rather than fail-fast
/// (Context: "parse vs. validate" — a caller wants every problem reported at once).
/// `Ok(())` iff the vector `validate_manifest` would otherwise have returned is
/// empty.
pub fn validate_manifest(manifest: &ModManifest) -> Result<(), Vec<ManifestValidationError>>;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ManifestValidationError {
    #[error("hook {hook} declares a component under namespace {found_namespace:?}, which is neither {mod_id:?} (this mod's own namespace) nor a vanilla `minecraft:` component")]
    ComponentNamespaceNotOwned { hook: Identifier, found_namespace: String, mod_id: String },
    #[error("hook id {0} does not carry the mod's own [mod].id ({1}) as its namespace")]
    HookNamespaceMismatch(Identifier, ModId),
    #[error("duplicate [[hooks]] entry for group {0:?} — at most one hook per domain group per mod (M8-alpha, see this blueprint's Context)")]
    DuplicateHookGroup(DomainGroup),
    #[error("hook {0} sets `priority`, but priority is meaningful only for group = block_redstone (MOD-D11)")]
    PriorityOnNonBlockRedstoneGroup(Identifier),
    #[error("hook {0}, group = block_redstone, declares no `priority` — required for Stage-4 hooks (MOD-D11)")]
    MissingPriorityOnBlockRedstone(Identifier),
    #[error("[[capabilities.components]] entry references hook {0}, which has no matching [[hooks]] entry")]
    ComponentAccessUnknownHook(Identifier),
    #[error("[[capabilities.components]] entry's group {found:?} does not match its referenced hook {hook}'s own declared group {expected:?}")]
    ComponentAccessGroupMismatch { hook: Identifier, expected: DomainGroup, found: DomainGroup },
    #[error("`before`/`after` reference {0:?} on hook {1} does not resolve to any declared [[hooks]] id in this manifest and is not a recognized native:<domain> marker")]
    UnresolvedOrderingReference(String, Identifier),
    #[error("network_channels does not declare channel {0} referenced elsewhere")]
    UndeclaredNetworkChannel(Identifier),
}
```

### `crates/mod-api/src/abi.rs` (`native-tier` feature)

```rust
/// This crate's own compiled release, embedded in `MOD_API_VERSION` and checked by
/// every native-tier mod's exported handshake symbol before any other symbol is
/// looked up (Context: "MOD-D3's version-handshake"). Starts at `0.1.0` — WS-D12's
/// own "engine SemVer starts at 0.1.0, independent of any other version line"
/// precedent, applied here to `rc-mod-api`'s own SemVer line (MOD-D21); 06's own WIT
/// sketch's illustrative `@3.2.0` package version is explicitly marked "illustrative,
/// not exhaustive" and is not a value this blueprint reproduces (Context).
#[stabby::stabby]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ModAbiVersion { pub major: u32, pub minor: u32, pub patch: u32 }

pub const MOD_API_VERSION: ModAbiVersion = ModAbiVersion { major: 0, minor: 1, patch: 0 };

impl ModAbiVersion {
    /// SemVer compatibility per MOD-D21: same `major`; the mod's own `minor` must be
    /// `<= engine`'s `minor` (an older-minor mod stays supported for MOD-D23's
    /// deprecation window; a newer-minor mod than the running engine supports is
    /// rejected, since it may reference an import the engine does not yet provide).
    pub const fn is_compatible_with(self, engine: ModAbiVersion) -> bool;
}

/// The exact exported-symbol contract every native-tier `.{dll,so,dylib}` must
/// provide (Context: "MOD-D3's version-handshake"). `rc-mod-host` looks this symbol
/// up via `libloading::Symbol` immediately after `libloading::Library::new` and
/// before resolving any other symbol — a hash mismatch or missing/mismatched
/// handshake is a hard load failure, never a silent fallback (MOD-D31's own policy,
/// restated here for the ABI-handshake case).
pub type AbiHandshakeFn = unsafe extern "C" fn() -> ModAbiVersion;
pub const ABI_HANDSHAKE_SYMBOL: &str = "rc_mod_abi_handshake";

/// ABI-safe mirror of `bevy_ecs::component::ComponentId` (Context:
/// "ComponentId-across-ABI rule"). `rc-mod-host` alone converts between the two —
/// this crate never constructs a real `ComponentId`, having no `World` access.
#[stabby::stabby]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModComponentId(pub u64);
```

### `crates/mod-api/src/component.rs` (`native-tier` feature)

```rust
/// The stable, layout-only value type a native-tier mod builds and hands to
/// `RegistryBuildContext::register_component` (Context: "why this crate does not
/// expose `bevy_ecs::component::ComponentDescriptor` directly"). MOD-D13's POD
/// constraint is a documented authorial discipline, not mechanically enforced here
/// (Context).
#[stabby::stabby]
#[derive(Clone, Debug)]
pub struct ModComponentDescriptor {
    pub name: stabby::string::String,
    pub size: usize,
    pub align: usize,
    /// `None` for a component with no drop glue (plain-old-data, the common case).
    /// The function receives a raw pointer to exactly `size` bytes, aligned to
    /// `align`, and must not read or write outside that range.
    pub drop_fn: stabby::option::Option<unsafe extern "C" fn(*mut u8)>,
    pub mutable: bool,
}

pub struct ComponentDescriptorBuilder {
    name: String,
    size: usize,
    align: usize,
    drop_fn: Option<unsafe extern "C" fn(*mut u8)>,
    mutable: bool,
}

impl ComponentDescriptorBuilder {
    /// `size`/`align` must both be nonzero and `align` must be a power of two
    /// (`std::alloc::Layout::from_size_align`'s own validity rule — this builder
    /// performs the identical check so `build()` can never hand `rc-mod-host` a
    /// layout that would panic when that crate later constructs the real
    /// `std::alloc::Layout`).
    pub fn new(name: impl Into<String>, size: usize, align: usize) -> Result<Self, ComponentDescriptorError>;
    pub fn with_drop(self, drop_fn: unsafe extern "C" fn(*mut u8)) -> Self;
    /// Default `true` (matches `bevy_ecs`'s own default for a component with no
    /// special immutability marker).
    pub fn mutable(self, mutable: bool) -> Self;
    pub fn build(self) -> Result<ModComponentDescriptor, ComponentDescriptorError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ComponentDescriptorError {
    #[error("component size must be nonzero")]
    ZeroSize,
    #[error("component alignment {0} is not a power of two")]
    AlignNotPowerOfTwo(usize),
    #[error("component name must not be empty")]
    EmptyName,
}
```

### `crates/mod-api/src/registry.rs` (`native-tier` feature)

```rust
use crate::Identifier;

/// Which generic registry a mod may insert new entries into at RegistryBuild
/// (MOD-D6). M8-alpha scope: `Block`/`Item` only (Context: "registry insertion,
/// id-space partitioning").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RegistryKind { Block, Item }

/// ABI-safe, numerically-vanilla-compatible mirror of `rc_chunk_storage::BlockStateId`
/// — same "textually distinct, numerically identical" precedent M2-B01 itself already
/// established for exactly this kind of cross-crate id bridging.
#[stabby::stabby]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModBlockStateId(pub u32);

#[stabby::stabby]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModItemId(pub u32);

/// One `register_block` request (MOD-D6). Each call to
/// `RegistryBuildContext::register_block` allocates **exactly one**
/// `ModBlockStateId`, regardless of `default_state_component_count`'s value — a
/// mod with several state variants (e.g. a directional block with 6 facings) calls
/// `register_block` once per variant, each call returning its own single id
/// (`rc-mod-host`'s `DenseIdAllocator` below advances by exactly 1 per call).
/// `default_state_component_count` is recorded, informational metadata only — the
/// number of distinct block-state property combinations this block's *total* state
/// space is expected to have (e.g. a directional block with 6 facings reports 6) —
/// intended to size a future mod-registered-component/state translation layer; it
/// has no effect on how many ids this call reserves.
#[stabby::stabby]
#[derive(Clone, Debug)]
pub struct BlockRegistration { pub id: stabby::string::String, pub default_state_component_count: u16 }

#[stabby::stabby]
#[derive(Clone, Debug)]
pub struct ItemRegistration { pub id: stabby::string::String, pub max_stack_size: u8 }

/// Pure, dense/sequential id allocator (Context: "id allocation must be dense and
/// sequential"). `rc-mod-host` is the only caller — constructed once per
/// `RegistryKind` per RegistryBuild pass, seeded with the pinned vanilla registry's
/// own highest generated id + 1.
pub struct DenseIdAllocator { next: u32 }

impl DenseIdAllocator {
    pub fn starting_at(next: u32) -> Self;
    /// Returns the next id and advances the internal counter by exactly 1. Never
    /// returns a value twice for the same instance.
    pub fn allocate(&mut self) -> u32;
    pub fn peek_next(&self) -> u32;
}
```

### `crates/mod-api/src/block_behavior.rs` (`native-tier` feature)

```rust
use crate::geometry::{ModBlockPos, ModDirection};
use crate::registry::ModBlockStateId;
use crate::access::TickPriority;

/// ABI-safe mirror of M3-B01's `UpdateContext<'a>`, bundling one `stabby`-safe
/// closure per method that crate's real type exposes (Context: "ModBlockBehavior/
/// ModUpdateContext"). Constructed host-side by `rc-mod-host` for the duration of
/// exactly one callback. `set_block` remains the only block-mutation path, matching
/// M3-B01's own rule verbatim.
#[stabby::stabby]
pub struct ModUpdateContext<'a> {
    get_block: stabby::closure::CallMut1<'a, ModBlockPos, stabby::option::Option<ModBlockStateId>>,
    set_block: stabby::closure::CallMut2<'a, ModBlockPos, ModBlockStateId, bool>,
    schedule_block_tick: stabby::closure::CallMut3<'a, ModBlockPos, u64, TickPriority, ()>,
    schedule_fluid_tick: stabby::closure::CallMut3<'a, ModBlockPos, u64, TickPriority, ()>,
    emit_block_event: stabby::closure::CallMut4<'a, ModBlockPos, u8, u8, ModBlockStateId, ()>,
    pub current_tick: u64,
}

impl<'a> ModUpdateContext<'a> {
    pub fn get_block(&mut self, pos: ModBlockPos) -> Option<ModBlockStateId>;
    pub fn set_block(&mut self, pos: ModBlockPos, new_state: ModBlockStateId) -> bool;
    pub fn schedule_block_tick(&mut self, pos: ModBlockPos, delay_ticks: u64, priority: TickPriority);
    pub fn schedule_fluid_tick(&mut self, pos: ModBlockPos, delay_ticks: u64, priority: TickPriority);
    pub fn emit_block_event(&mut self, pos: ModBlockPos, event_id: u8, event_param: u8, block_state: ModBlockStateId);
}

/// ABI-safe mirror of M3-B01's `BlockBehavior` trait, method-for-method. Every
/// method has the identical no-op default M3-B01's own `NoOpBehavior` establishes.
#[stabby::stabby]
pub trait ModBlockBehavior: Send + Sync {
    fn on_neighbor_changed(&self, ctx: &mut ModUpdateContext, pos: ModBlockPos, from: ModDirection) {}
    fn on_shape_update(&self, ctx: &mut ModUpdateContext, pos: ModBlockPos, from: ModDirection, neighbor_state: ModBlockStateId) -> stabby::option::Option<ModBlockStateId> { stabby::option::Option::None }
    fn on_scheduled_tick(&self, ctx: &mut ModUpdateContext, pos: ModBlockPos) {}
    fn on_block_event(&self, ctx: &mut ModUpdateContext, pos: ModBlockPos, event_id: u8, event_param: u8, block_state: ModBlockStateId) {}
}
```

### `crates/mod-api/src/entrypoint.rs` (`native-tier` feature)

```rust
use crate::abi::ModComponentId;
use crate::block_behavior::ModBlockBehavior;
use crate::component::ModComponentDescriptor;
use crate::registry::{BlockRegistration, ItemRegistration, ModBlockStateId, ModItemId};
use crate::Identifier;

/// Mirrors `rc_messaging::Address` structurally (MOD-D15: never a `NodeId`/
/// hostname/PID) without a dependency edge — this crate cannot depend on
/// `rc-messaging` (a leaf, WS-D3 rule 4). `rc-mod-host` converts to/from the real
/// `Address` when bridging a `send_mod_message` call into `RegionMessageBus`.
#[stabby::stabby]
#[derive(Clone, Debug)]
pub enum ModAddress {
    Region(stabby::string::String),
    Entity(u64),
    Chunk { dimension: u16, x: i32, z: i32 },
}

/// A mod's shared-side (`shared/`) initialization, if any (MOD-D4).
#[stabby::stabby]
pub trait SharedModInit: Send + Sync {
    fn on_shared_init(&mut self, ctx: &mut SharedInitContext) -> stabby::result::Result<(), ModInitError> { stabby::result::Result::Ok(()) }
}
pub struct SharedInitContext { /* opaque, engine-owned; no fields a mod author constructs directly */ }

/// RegistryBuild-time context (MOD-D6): the only surface through which a
/// native-tier mod inserts new registry entries, components, hooks-adjacent
/// resources, or block behaviors. Never exposes a `bevy_ecs::World` reference
/// (ARCH-D4's boundary).
pub struct RegistryBuildContext { /* opaque, engine-owned */ }
impl RegistryBuildContext {
    pub fn register_block(&mut self, reg: BlockRegistration) -> ModBlockStateId;
    pub fn register_item(&mut self, reg: ItemRegistration) -> ModItemId;
    pub fn register_component(&mut self, descriptor: ModComponentDescriptor) -> ModComponentId;
    pub fn register_block_behavior(&mut self, state: ModBlockStateId, behavior: stabby::dynptr!(stabby::boxed::Box<dyn ModBlockBehavior>));
    /// MOD-D20: must appear in this mod's `[capabilities].network_channels`, checked
    /// by `rc-mod-host` at call time, not by this type.
    pub fn register_channel(&mut self, id: Identifier);
}

pub struct ServerInitContext { /* opaque, engine-owned */ }
pub struct ServerShutdownContext { /* opaque, engine-owned */ }

/// One tick-domain hook invocation (Context: "hook catalog structure"). `hook_id`
/// matches the `id` of exactly one of this mod's own `[[hooks]]` manifest entries —
/// `rc-mod-host` only ever calls `on_tick_hook` with an id this mod itself declared,
/// so a mod's own dispatch `match` inside its `on_tick_hook` body is exhaustive over
/// a known-small, self-declared set.
pub struct TickHookContext { /* opaque, engine-owned; access is exactly the declared ComponentAccessDecl set for this hook_id */ }

#[stabby::stabby]
pub trait ServerModEntry: Send + Sync {
    fn on_registry_build(&mut self, ctx: &mut RegistryBuildContext) -> stabby::result::Result<(), ModInitError>;
    fn on_server_init(&mut self, ctx: &mut ServerInitContext) -> stabby::result::Result<(), ModInitError> { stabby::result::Result::Ok(()) }
    fn on_server_shutdown(&mut self, ctx: &mut ServerShutdownContext) {}
    fn on_tick_hook(&mut self, hook_id: &stabby::string::String, ctx: &mut TickHookContext) -> stabby::result::Result<(), ModHookError>;
    fn on_channel_message(&mut self, channel: &stabby::string::String, sender_entity: u64, payload: &stabby::vec::Vec<u8>) {}
    fn on_mod_message(&mut self, channel: &stabby::string::String, sender: &ModAddress, payload: &stabby::vec::Vec<u8>) {}
}

pub struct ClientRegistryBuildContext { /* opaque, engine-owned */ }
impl ClientRegistryBuildContext {
    /// MOD-D18, registration-only at M8 (Context: "hook catalog structure" table).
    /// Every method below records the extension point's intended use; none has a
    /// functioning visual effect until `07-client-architecture.md`'s renderer exists
    /// (M10, PLAN-D2).
    pub fn register_model_provider(&mut self, id: Identifier);
    pub fn register_block_renderer(&mut self, block: Identifier);
    pub fn register_gui_screen(&mut self, id: Identifier);
    pub fn register_hud_overlay(&mut self, id: Identifier);
    pub fn register_input_binding(&mut self, id: Identifier);
}
pub struct ClientInitContext { /* opaque, engine-owned */ }

#[stabby::stabby]
pub trait ClientModEntry: Send + Sync {
    fn on_client_registry_build(&mut self, ctx: &mut ClientRegistryBuildContext) -> stabby::result::Result<(), ModInitError>;
    fn on_client_init(&mut self, ctx: &mut ClientInitContext) -> stabby::result::Result<(), ModInitError> { stabby::result::Result::Ok(()) }
}

#[derive(Debug, Clone, thiserror::Error)]
#[stabby::stabby]
pub enum ModInitError {
    #[error("{0}")]
    Message(stabby::string::String),
}

#[derive(Debug, Clone, thiserror::Error)]
#[stabby::stabby]
pub enum ModHookError {
    #[error("{0}")]
    Message(stabby::string::String),
}
```

### `crates/mod-api/wit/rc-mod-api.wit`

```wit
package rc:mod-api@0.1.0;

interface types {
  record block-pos { x: s32, y: s32, z: s32 }
  enum direction { west, east, north, south, down, up }
  type block-state-id = u32;
  type item-id = u32;
  type component-id = u64;
}

interface world-query {
  use types.{component-id};

  @since(version = "0.1.0")
  get-component: func(entity: u64, component: string) -> option<list<u8>>;

  @since(version = "0.1.0")
  set-component: func(entity: u64, component: string, value: list<u8>);
}

interface messaging {
  @since(version = "0.1.0")
  send-mod-message: func(address: string, channel: string, payload: list<u8>) -> result<_, string>;
}

interface registry-build {
  use types.{block-state-id, item-id, component-id};

  @since(version = "0.1.0")
  register-block: func(id: string, default-state-component-count: u16) -> block-state-id;

  @since(version = "0.1.0")
  register-item: func(id: string, max-stack-size: u8) -> item-id;

  @since(version = "0.1.0")
  register-component: func(name: string, size: u32, align: u32) -> component-id;

  @since(version = "0.1.0")
  register-channel: func(id: string);
}

interface lifecycle {
  @since(version = "0.1.0")
  on-registry-build: func() -> result<_, string>;

  @since(version = "0.1.0")
  on-init: func() -> result<_, string>;

  @since(version = "0.1.0")
  on-shutdown: func();
}

/// One export per ARCH-D8 domain group (MOD-D8's sparse-export shape: a mod
/// implements only the groups it declared a [[hooks]] entry for).
interface tick-hooks {
  @since(version = "0.1.0")
  on-block-redstone-tick: func();
  @since(version = "0.1.0")
  on-ai-physics-tick: func();
  @since(version = "0.1.0")
  on-lighting-tick: func();
  @since(version = "0.1.0")
  on-chunk-serialize-tick: func();
  @since(version = "0.1.0")
  on-net-codec-tick: func();
}

/// Mirrors M3-B01's `BlockBehavior` trait one-for-one (Context: "block-behavior
/// registration"). `state` identifies which of this mod's own registered block
/// states the call targets, letting one component's fixed exports serve any number
/// of block types this mod registered.
interface block-behavior {
  use types.{block-pos, block-state-id, direction};

  @since(version = "0.1.0")
  on-neighbor-changed: func(state: block-state-id, pos: block-pos, from: direction);

  @since(version = "0.1.0")
  on-shape-update: func(state: block-state-id, pos: block-pos, from: direction, neighbor-state: block-state-id) -> option<block-state-id>;

  @since(version = "0.1.0")
  on-scheduled-tick: func(state: block-state-id, pos: block-pos);

  @since(version = "0.1.0")
  on-block-event: func(state: block-state-id, pos: block-pos, event-id: u8, event-param: u8);
}

interface networking {
  @since(version = "0.1.0")
  on-channel-message: func(channel: string, sender-entity: u64, payload: list<u8>);

  @since(version = "0.1.0")
  on-mod-message: func(channel: string, sender: string, payload: list<u8>);
}

/// MOD-D18, registration-only at M8 (Context: "hook catalog structure").
interface client-registration {
  @since(version = "0.1.0")
  register-model-provider: func(id: string);
  @since(version = "0.1.0")
  register-block-renderer: func(block: string);
  @since(version = "0.1.0")
  register-gui-screen: func(id: string);
  @since(version = "0.1.0")
  register-hud-overlay: func(id: string);
  @since(version = "0.1.0")
  register-input-binding: func(id: string);
}

world rc-mod-server {
  import world-query;
  import messaging;
  import registry-build;
  export lifecycle;
  export tick-hooks;
  export block-behavior;
  export networking;
}

world rc-mod-client {
  import registry-build;
  export lifecycle;
  export client-registration;
}
```

### `crates/mod-api/src/guest.rs` (`wasm-tier` feature)

```rust
//! Guest-side WASM-tier bindings, generated once here from `wit/rc-mod-api.wit` and
//! re-exported so a WASM-tier mod's own crate never re-invokes `wit_bindgen::generate!`
//! against its own copy of the interface (Context: "Tier 1, WASM"). A mod depends on
//! `rc-mod-api` with default features (`wasm-tier` only) and uses this module
//! directly: `use rc_mod_api::guest::*;`.
//!
//! Moderate-confidence flag (Context): the exact `generate!` invocation shape
//! (inline `world` selection, `generate_unused_types`, feature-gating for
//! `@unstable`) should be re-verified against the installed `wit-bindgen` 0.60.0
//! macro docs at implementation time — no Deliverable signature elsewhere in this
//! blueprint depends on this macro's exact invocation syntax.

wit_bindgen::generate!({
    path: "wit/rc-mod-api.wit",
    world: "rc-mod-server", // a mod's own build additionally generates "rc-mod-client" as needed; both worlds' bindings are available from this one macro call
});
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below plus every `src/*.rs` file from Deliverables with every function body replaced by `todo!()` (field lists, derives, doc comments, and every `#[stabby::stabby]`/`#[cfg(feature = ...)]` attribute stay exactly as specified), plus `Cargo.toml`, `wit/rc-mod-api.wit`, and `guest.rs`'s `wit_bindgen::generate!` call (which has no function body to stub — it either compiles or it doesn't, so it ships complete in the test changeset). The implementation changeset (Implementation steps) fills in bodies only; it must not modify any file under `crates/mod-api/tests/`, must not change any type's field list/derive list/public signature, and must not weaken any assertion below.

### `crates/mod-api/tests/identifier.rs`

1. `parse_valid_identifier` — `Identifier::parse("example_ores:ore_charge")` succeeds; `.namespace() == "example_ores"`, `.path() == "ore_charge"`.
2. `parse_missing_separator` — `Identifier::parse("no_colon_here")` returns `Err(IdentifierError::MissingSeparator(_))`.
3. `parse_invalid_namespace_char` — `Identifier::parse("Example:ore")` (uppercase) returns `Err(IdentifierError::InvalidNamespaceChar(_))`.
4. `parse_invalid_path_char` — `Identifier::parse("example:ore charge")` (space) returns `Err(IdentifierError::InvalidPathChar(_))`.
5. `parse_empty_component` — `Identifier::parse(":path")` and `Identifier::parse("ns:")` both return `Err(IdentifierError::EmptyComponent(_))`.
6. `display_round_trips_parse` — for 20 proptest-generated valid `(namespace, path)` pairs restricted to the allowed charset, `Identifier::new(ns, path).unwrap().to_string().parse::<Identifier>().unwrap() == Identifier::new(ns, path).unwrap()`.
7. `identifier_is_ord_and_hashable` — construct a `BTreeSet<Identifier>` and a `HashSet<Identifier>` from 5 distinct identifiers plus one duplicate; both collections' final length is 5.
8. `serde_round_trips_through_toml_string` — `toml::to_string(&Identifier::new("a", "b").unwrap())` then `toml::from_str::<Identifier>(...)` (exercising the `try_from = "String"`/`into = "String"` serde attributes) round-trips equal.

### `crates/mod-api/tests/manifest_parsing.rs`

Uses a shared `const MINIMAL_MANIFEST: &str` (the worked example from Context's schema, trimmed to only the required fields: `[mod]` id/version/display_name/license, `[api].requires`, `[entrypoints].tier`) plus per-test variations built by string-editing that base.

1. `minimal_manifest_parses` — `parse_manifest(MINIMAL_MANIFEST)` is `Ok`; every required field round-trips to its literal source value; every `#[serde(default)]` field (`authors`, `unstable_features`, `dependencies`, `capabilities`, `hooks`) is empty/default.
2. `full_manifest_parses` — the complete worked example from Context (every optional section present, one `[[hooks]]` entry, two `[[capabilities.components]]` entries, one `[entrypoints.native."x86_64-pc-windows-msvc"]` entry) parses `Ok`, and every field matches the source exactly (field-by-field assertions on `ModManifest`).
3. `missing_required_field_is_a_toml_error` — `MINIMAL_MANIFEST` with the `license` line deleted returns `Err(ManifestError::Toml(_))`.
4. `unknown_tier_value_is_a_toml_error` — `tier = "bogus"` (not `"wasm"`/`"native"`) returns `Err(ManifestError::Toml(_))` (serde's own enum-variant rejection).
5. `invalid_dependency_range_syntax_is_rejected` — `[dependencies] foo = "not a version at all!"` returns `Err(ManifestError::Toml(_))` (via `DependencyRange`'s `try_from` failing during deserialization, surfacing as a `toml::de::Error`).
6. `hook_component_access_kind_must_be_read_or_write` — a `[[capabilities.components]]` entry with `access = "readwrite"` returns `Err(ManifestError::Toml(_))`.
7. `native_triple_table_parses_into_btreemap` — the full-manifest fixture's `entrypoints.native` map has exactly one key, `"x86_64-pc-windows-msvc"`, with `NativeTripleSupport { server: true, client: false }`.

### `crates/mod-api/tests/manifest_validation.rs`

Each test builds a `ModManifest` value directly (not through `parse_manifest` — these test `validate_manifest`'s own logic in isolation) via a shared `tests/common/mod.rs` helper `fn base_manifest(mod_id: &str) -> ModManifest` (a minimal, always-valid starting point every test mutates).

1. `minimal_valid_manifest_passes` — `validate_manifest(&base_manifest("example_ores"))` is `Ok(())`.
2. `hook_namespace_mismatch_is_rejected` — add one `HookDecl { id: Identifier::parse("other_mod:block_redstone").unwrap(), group: BlockRedstone, priority: Some(Normal), .. }` to a manifest whose `[mod].id == "example_ores"`; `validate_manifest` returns `Err` containing exactly one `ManifestValidationError::HookNamespaceMismatch`.
3. `duplicate_hook_group_is_rejected` — two `HookDecl`s both with `group: DomainGroup::Lighting` (distinct valid ids, both correctly namespaced); `Err` containing `ManifestValidationError::DuplicateHookGroup(DomainGroup::Lighting)`.
4. `priority_required_on_block_redstone` — one `HookDecl { group: BlockRedstone, priority: None, .. }`; `Err` containing `ManifestValidationError::MissingPriorityOnBlockRedstone`.
5. `priority_forbidden_off_block_redstone` — one `HookDecl { group: Lighting, priority: Some(TickPriority::Normal), .. }`; `Err` containing `ManifestValidationError::PriorityOnNonBlockRedstoneGroup`.
6. `component_access_unknown_hook_is_rejected` — `capabilities.components` contains one entry whose `hook` references an id with no matching `[[hooks]]` entry; `Err` containing `ManifestValidationError::ComponentAccessUnknownHook`.
7. `component_access_group_mismatch_is_rejected` — one valid `HookDecl { group: AiPhysics, .. }` plus one `ComponentAccessDecl { hook: <that hook's id>, group: Lighting, .. }` (mismatched group); `Err` containing `ManifestValidationError::ComponentAccessGroupMismatch`.
8. `unresolved_ordering_reference_is_rejected` — one `HookDecl` whose `after` contains `HookOrderRef::Hook(Identifier::parse("example_ores:nonexistent").unwrap())`; `Err` containing `ManifestValidationError::UnresolvedOrderingReference`.
9. `native_domain_marker_in_ordering_always_resolves` — one `HookDecl` whose `after` contains `HookOrderRef::NativeDomain(NativeDomainMarker::BlockRedstone)`; validation of *this* field produces no error (a `native:*` marker never needs an in-manifest match).
10. `multiple_problems_are_all_collected_not_fail_fast` — a manifest constructed to trigger both `DuplicateHookGroup` and `MissingPriorityOnBlockRedstone` simultaneously; the returned `Vec` has length exactly 2 and contains both variants (proves `validate_manifest` does not stop at the first problem).
11. `undeclared_network_channel_is_rejected` — `capabilities.network_channels` is empty but a `HookDecl`'s presence alone does not trigger this check (this check is exercised at the `RegistryBuildContext::register_channel`/`on_channel_message` call level, which this crate's own manifest-only validation cannot see — so this test instead asserts the *documented* boundary directly: constructing a manifest with `network_channels: vec![]` and no other capability-related field passes `validate_manifest` cleanly, confirming this crate does not attempt an enforcement check it cannot actually perform, matching Context's "M8-alpha enforcement scope stated honestly").

### `crates/mod-api/tests/abi_handshake.rs` (`native-tier` feature)

1. `same_version_is_compatible` — `ModAbiVersion { 0, 1, 0 }.is_compatible_with(ModAbiVersion { 0, 1, 0 })` is `true`.
2. `mod_api_version_constant_matches_itself` — `MOD_API_VERSION.is_compatible_with(MOD_API_VERSION)` is `true`.
3. `older_minor_is_compatible` — mod `{0, 0, 5}` vs. engine `{0, 1, 0}` (same major, mod's minor `<` engine's) is `true`.
4. `newer_minor_is_incompatible` — mod `{0, 2, 0}` vs. engine `{0, 1, 0}` is `false`.
5. `different_major_is_always_incompatible` — mod `{1, 0, 0}` vs. engine `{0, 1, 0}`, and the reverse, both `false` — a matrix of 4 cases: `(0,x,y)` vs `(1,x,y)` in both directions, plus `(1,5,0)` vs `(2,0,0)` in both directions.
6. `patch_never_affects_compatibility` — mod `{0, 1, 999}` vs. engine `{0, 1, 0}` is `true` (patch is compared nowhere in `is_compatible_with`).
7. `handshake_symbol_name_is_stable` — `assert_eq!(ABI_HANDSHAKE_SYMBOL, "rc_mod_abi_handshake")` (a literal regression guard — a future edit accidentally renaming this constant silently breaks every already-compiled native mod's handshake, so its exact string value is pinned by a test, not just by convention).

### `crates/mod-api/tests/component_descriptor.rs` (`native-tier` feature)

1. `valid_descriptor_builds` — `ComponentDescriptorBuilder::new("example_ores:ore_charge", 8, 4).unwrap().build()` is `Ok`; the resulting `ModComponentDescriptor`'s `size == 8`, `align == 4`, `mutable == true` (default), `drop_fn` is `None`.
2. `zero_size_is_rejected` — `ComponentDescriptorBuilder::new("x", 0, 4)` returns `Err(ComponentDescriptorError::ZeroSize)`.
3. `non_power_of_two_align_is_rejected` — `ComponentDescriptorBuilder::new("x", 8, 3)` returns `Err(ComponentDescriptorError::AlignNotPowerOfTwo(3))`.
4. `empty_name_is_rejected` — `ComponentDescriptorBuilder::new("", 8, 4)` returns `Err(ComponentDescriptorError::EmptyName)`.
5. `with_drop_is_recorded_and_invoked_on_call` — a `static CALLED: AtomicBool` plus an `unsafe extern "C" fn recording_drop(_: *mut u8) { CALLED.store(true, Relaxed) }`; build a descriptor `.with_drop(recording_drop)`; assert `descriptor.drop_fn` is `Some(f)` where `f as usize == recording_drop as usize` (function-pointer identity); then, inside an `unsafe` block, call `f(std::ptr::null_mut())` directly and assert `CALLED.load(Relaxed) == true` (proves the stored pointer is genuinely callable and reaches the same function, not merely present).
6. `mutable_false_is_recorded` — `.mutable(false).build().unwrap().mutable == false`.
7. `descriptor_layout_matches_a_real_rust_type` — build a descriptor from `(std::mem::size_of::<[u64; 3]>(), std::mem::align_of::<[u64; 3]>())` (`24`, `8`); assert the built descriptor's `size`/`align` equal those exact values — a sanity check that this builder's raw `usize` parameters are not silently reinterpreted or rounded anywhere in `build()`.

### `crates/mod-api/tests/access_roundtrip.rs`

1. `tick_priority_declaration_order_matches_ordinal_order` — `TickPriority::ExtremelyHigh < TickPriority::VeryHigh < TickPriority::High < TickPriority::Normal < TickPriority::Low < TickPriority::VeryLow < TickPriority::ExtremelyLow` (six chained comparisons via `assert!`), proving the `#[derive(Ord)]` declaration order matches vanilla's real ordinal order — the exact property a future `rc-mechanics` conversion's correctness depends on.
2. `domain_group_serde_round_trips_snake_case` — for each of the five `DomainGroup` variants, `toml::to_string` then `toml::from_str` round-trips equal, and the serialized TOML string value is the exact expected `snake_case` token (`"block_redstone"`, `"ai_physics"`, `"lighting"`, `"chunk_serialize"`, `"net_codec"`) — a literal regression guard, since a future `rc-scheduler` blueprint's `From` conversion and this crate's own manifest schema both depend on these exact string tokens never silently changing case/spelling.
3. `component_access_decl_round_trips` — a `ComponentAccessDecl { hook: ..., name: ..., access: AccessKind::Write, group: DomainGroup::Lighting }` serializes through `toml` and deserializes back equal.
4. `hook_order_ref_parses_both_forms` — `HookOrderRef` deserialized from the TOML string `"native:block_redstone"` equals `HookOrderRef::NativeDomain(NativeDomainMarker::BlockRedstone)`; deserialized from `"example_ores:some_hook"` equals `HookOrderRef::Hook(Identifier::parse("example_ores:some_hook").unwrap())`; an invalid string (e.g. `"native:not_a_real_group"`) is `Err`.
5. `hook_decl_round_trips_through_full_manifest_toml` — the `HookDecl` embedded in `manifest_parsing.rs`'s full-manifest fixture, re-serialized and re-parsed standalone (outside the full manifest), equals the original.
6. `access_kind_serde_tokens_are_read_write` — `AccessKind::Read`/`Write` serialize to the literal strings `"read"`/`"write"` (regression guard, same rationale as test 2).

### `crates/mod-api/tests/registry_ids.rs` (`native-tier` feature)

1. `allocator_starting_point_is_first_returned_value` — `DenseIdAllocator::starting_at(1000).allocate() == 1000`.
2. `allocator_is_sequential_with_no_gaps` — 100 sequential `.allocate()` calls on one instance starting at `0` produce exactly `0, 1, 2, ..., 99` (assert the collected `Vec<u32>` equals `(0..100).collect::<Vec<_>>()`).
3. `allocator_never_returns_a_value_twice` — 500 sequential `.allocate()` calls collected into a `HashSet<u32>`; the set's length is exactly 500.
4. `peek_next_does_not_advance` — `let mut a = DenseIdAllocator::starting_at(5); assert_eq!(a.peek_next(), 5); assert_eq!(a.peek_next(), 5);` (two peeks, same value, no state change) `assert_eq!(a.allocate(), 5);` (the peeked value is exactly what `allocate` then returns).
5. `two_allocators_are_independent` — one `DenseIdAllocator::starting_at(0)` for blocks and one `DenseIdAllocator::starting_at(0)` for items, interleaved `.allocate()` calls on both; each instance's own sequence is independently gapless and zero-based, proving no shared/global state leaks between separately-constructed instances (Context: "Block and Item ids do not share one id-space").
6. `mod_block_state_id_and_mod_item_id_are_distinct_types` — `ModBlockStateId(5) == ModBlockStateId(5)` compiles and is `true`; `ModBlockStateId(5)` and `ModItemId(5)` are not comparable (a compile-time-only assertion: this test's presence, with no `PartialEq<ModItemId> for ModBlockStateId` impl existing anywhere in the crate, is itself the proof — the test body simply constructs one of each and asserts each equals itself).

### `crates/mod-api/tests/guest_bindings_compile.rs` (`wasm-tier` feature, compiled for the **host** target, not `wasm32-wasip2` — this test only proves the macro invocation itself is well-formed against the shipped `.wit` file; it does not exercise a real component)

1. `guest_module_is_reachable` — `let _: fn() = || { let _ = rc_mod_api::guest::exports::rc::mod_api::lifecycle::Guest::on_init; };` referencing at least one generated item by its expected generated path (exact generated module path is a `wit-bindgen`-version-dependent detail — Implementation step 9 fixes the exact path once the macro is run once and its output inspected via `cargo expand` or equivalent; this test's assertion is only that *some* generated path compiles, proving `wit/rc-mod-api.wit` parsed successfully).

## Implementation steps

1. **`Cargo.toml`.** Add the dependency/feature block exactly as specified in Deliverables. Observable: `cargo metadata -p rc-mod-api` succeeds for every feature combination (`--no-default-features`, `--features wasm-tier`, `--features native-tier`, `--all-features`).
2. **`identifier.rs`.** Implement `Identifier::new`/`parse`/accessors, `Display`, `FromStr`, and the `serde(try_from/into)` glue (`impl TryFrom<String> for Identifier` calling `parse`; `impl From<Identifier> for String` calling `to_string`). Charset check: iterate `namespace.chars()` against `is_ascii_lowercase() || is_ascii_digit() || matches!(c, '_' | '.' | '-')`; `path` additionally allows `/`. Observable: `identifier.rs`'s test file passes in full.
3. **`geometry.rs`.** Trivial field-copy `From` impls both directions. Observable: compiles; no dedicated test file beyond what `block_behavior.rs`'s tests exercise indirectly (Constraints (a) still requires this file be present and correct in the test changeset's `todo!()`-stubbed form first).
4. **`access.rs`, `capabilities.rs`.** Every type here is plain data with derive-generated behavere except `HookOrderRef`'s `serde(try_from/into = "String")` glue: `TryFrom<String>` splits on the literal prefix `"native:"` (if present, parse the remainder against `NativeDomainMarker`'s five snake_case tokens via a manual match, `Err` on no match) else falls through to `Identifier::parse` wrapped in `HookOrderRef::Hook`. Observable: `access_roundtrip.rs` passes in full.
5. **`manifest.rs` — parsing.** Implement `DependencyRange::parse` (a single `chars().all(...)` charset check against `[0-9.^~<>=, *]`, `Err(ManifestError::InvalidDependencyRange)` on any other character or empty string) and its `serde(try_from/into = "String")` glue. Implement `parse_manifest` as `toml::from_str::<ModManifest>(toml_text).map_err(ManifestError::Toml)`. Observable: `manifest_parsing.rs` passes in full.
6. **`manifest.rs` — validation.** Implement `validate_manifest` as a sequence of independent checks, each appending to one `Vec<ManifestValidationError>`, returning `Err(vec)` if non-empty else `Ok(())`: (a) for each `hooks` entry, `Err(HookNamespaceMismatch)` if `hook.id.namespace() != manifest.mod_.id.as_str()` (a hook's own `Identifier` namespace must equal the mod's own `ModId` string exactly — the type split from Context's "why `[mod].id` is `ModId`, not `Identifier`" note is what makes this a plain string comparison with no parsing ambiguity). (b) duplicate group detection via a `HashSet<DomainGroup>` scan over `hooks`. (c) priority-required/forbidden checks per hook's `group`. (d) for each `capabilities.components` entry, look up `hook` in `hooks` by id; `Err(ComponentAccessUnknownHook)` if absent, `Err(ComponentAccessGroupMismatch)` if present but `group` differs. (e) for each hook's `before`/`after` entries, `HookOrderRef::NativeDomain(_)` always resolves; `HookOrderRef::Hook(id)` must match some `hooks[].id` (any mod's, not just this one — this M8-alpha implementation only has one manifest in scope at validation time, so it checks against *this manifest's own* `hooks` list only, documented as a known, narrower-than-ideal scope: cross-mod ordering-reference validation needs the full installed mod set and is `rc-mod-host`'s job at RegistryBuild, not this crate's per-manifest validator). Observable: `manifest_validation.rs` passes in full.
7. **`abi.rs`.** `ModAbiVersion::is_compatible_with`: `self.major == engine.major && self.minor <= engine.minor`. Observable: `abi_handshake.rs` passes in full.
8. **`component.rs`.** `ComponentDescriptorBuilder::new` validates `size != 0` and `align.is_power_of_two()` (`usize::is_power_of_two`, also implicitly rejects `align == 0`) and `!name.is_empty()`, in that order (first failure wins — matches the test file's ordering, though only one failure is triggered per test case so ordering is not itself asserted). `build()` constructs `ModComponentDescriptor` from the accumulated builder state, converting `String`→`stabby::string::String` and `Option<fn>`→`stabby::option::Option`. Observable: `component_descriptor.rs` passes in full.
9. **`registry.rs`.** `DenseIdAllocator::allocate` is `let id = self.next; self.next += 1; id`. `peek_next` is `self.next`. Observable: `registry_ids.rs` passes in full.
10. **`block_behavior.rs`, `entrypoint.rs`.** These files' non-trivial bodies are exactly the `ModUpdateContext` accessor methods (`self.get_block.call(pos)`-shaped one-line forwards to the bundled `stabby` closures — exact `stabby::closure` call-method name, e.g. `.call(...)` vs. `.call_mut(...)`, is this blueprint's flagged moderate-confidence point, Context — confirm against the installed `stabby` 72.1.16 docs before writing these five bodies) — every other item in both files is a plain struct/trait declaration with no logic to implement (the opaque `*Context` structs' fields are engine-owned and populated by `rc-mod-host`, out of this blueprint's scope; their `impl` blocks here declare only the signatures a mod calls, with bodies that forward into the (not-yet-existent, `rc-mod-host`-owned) private fields — **this blueprint stops at the public signature**; a stub body returning a default/no-op value, clearly marked, is acceptable here specifically because no `rc-mod-host` exists yet to wire a real implementation against, unlike every other file in this blueprint where the full real behavior is implementable today). Observable: `cargo build -p rc-mod-api --features native-tier` succeeds with zero `todo!()` remaining in the files this blueprint's tests actually exercise (`abi.rs`, `component.rs`, `registry.rs`, `access.rs`, `manifest.rs`, `identifier.rs`, `capabilities.rs`, `geometry.rs`) — `block_behavior.rs`/`entrypoint.rs`'s untestable-until-`rc-mod-host`-exists methods may retain a documented `unimplemented!("wired by rc-mod-host, a later M8 blueprint")` body, which is not a `todo!()` and does not violate this blueprint's own done-condition (no acceptance test in this blueprint's own changeset calls any such method — Constraints (f) restates this explicitly as a named, bounded exception).
11. **`wit/rc-mod-api.wit`.** Write the file exactly as specified in Deliverables — no logic, pure WIT source.
12. **`guest.rs`.** Write the `wit_bindgen::generate!` invocation. Run `cargo build -p rc-mod-api --features wasm-tier` once and inspect the generated module tree (`cargo expand -p rc-mod-api --features wasm-tier guest` or equivalent) to fix `guest_bindings_compile.rs`'s exact referenced path, then update that test file's own single assertion to match (this is the one place this blueprint's own test file content is discovered rather than fully pre-specified — Constraints (a) permits this single named exception, since the exact generated path is mechanically determined by `wit-bindgen` 0.60.0's own codegen convention, not an authorial choice). Observable: `guest_bindings_compile.rs` passes.
13. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` (against this blueprint's own expanded Rule 4 set, Context), `-- test` (workspace default features, then again with each of `rc-mod-api`'s own feature combinations) — all exit 0.
14. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, with exactly one named exception: `guest_bindings_compile.rs`'s single assertion's exact referenced path is discovered during Implementation step 12 (Constraints note there explains why — it is mechanically determined by `wit-bindgen`'s own codegen, not an authorial choice) and may be corrected in the implementation changeset; no other test file, test case, or assertion anywhere in Acceptance tests may be added, removed, renamed, or weakened by the implementation changeset.

(b) **No new external dependencies beyond the pinned `[workspace.dependencies]` set.** Every crate this blueprint's Deliverables use — `rc-core`, `serde`, `toml`, `thiserror`, `bevy_ecs`, `stabby`, `wit-bindgen`, plus the `proptest` dev-dependency — is already pinned in `12-workspace-structure.md`'s `[workspace.dependencies]` table. In particular, **no `semver` crate is added**, even though one would make `DependencyRange`'s validation more rigorous (Context: "No `semver` crate — syntax validation only") — this is a deliberate scope boundary, not an oversight, and must not be silently "fixed" by adding one.

(c) **No Mojang or third-party reimplementation code.** Every algorithm, schema, and type in this blueprint is derived solely from `docs/planning/06-modding-api.md`'s MOD-D1–D32 (plus the cited restatements from `01`, `12`, `14`, and this blueprint's own prerequisite blueprints M0-B02/B05, M3-B01, M4-B01) and this blueprint's own concrete, cited resolutions of what those decisions leave open (ASSET-D18/D19/D30). The `Identifier` charset is sourced from `minecraft.wiki`'s public documentation of vanilla's own resource-location convention (ASSET-D18(b)) — a functional-fact charset rule, not any copied text or code.

(d) **`unsafe` code is permitted only where `stabby`'s own API requires it, never elsewhere.** `ModComponentDescriptor::drop_fn`'s function-pointer type (`unsafe extern "C" fn(*mut u8)`) and `component.rs`'s `with_drop` accepting one are the only places this blueprint's Deliverables touch `unsafe` signatures directly — this blueprint calls no such function itself (that is `rc-mod-host`'s job, holding a real allocation to drop); the one test that does call a drop function directly (`component_descriptor.rs`'s `with_drop_is_recorded_and_invoked_on_call`) passes a null pointer specifically because the recording function never dereferences it, and must say so in a `// SAFETY:` comment.

(e) **Feature-gate discipline.** `native-tier`-gated code must never reference a `wasm-tier`-only item and vice versa; `cargo build -p rc-mod-api --features native-tier` (no `wasm-tier`, since it is not the default when `native-tier` alone is requested via `--no-default-features --features native-tier`) and `cargo build -p rc-mod-api --features wasm-tier` (the default) must each succeed independently, proven by this blueprint's own Done-definition checkbox, not merely `--all-features`.

(f) **Scope boundary — do not implement beyond this blueprint's stated Deliverables.** This blueprint does not implement: `rc-mod-host`'s dylib loading, `catch_unwind` crash isolation, or the `libloading`/`wasmtime` embedding itself (MOD-D2/D25/D26/D32 — a separate, later M8 blueprint); the translation from `ModComponentDescriptor`/`ModBlockStateId`/etc. into real `bevy_ecs`/`rc-registries`/`rc-chunk-storage` values (`rc-mod-host`'s job, per Context's repeated "never this crate" notes); the `ModSystemShim`/RC-Executor conflict-graph integration itself (`rc-scheduler`, building on M0-B05's already-existing `RcExecutorBuilder::register_system`); real WASI capability enforcement, fuel/epoch metering, or the operator-approval gate for `network` (MOD-D24/D25, `rc-mod-host`); the `cargo generate` template (MOD-D27, a separate, later M8 or dev-experience blueprint); `rc-mod-test`'s mocked-host harness (MOD-D29, likewise separate). Every `*Context` struct's fields in `entrypoint.rs`/`block_behavior.rs`'s Deliverables are intentionally left `/* opaque, engine-owned */` — populating them is `rc-mod-host`'s job, and a small number of their methods' bodies may stay `unimplemented!()`-stubbed per Implementation step 10's named, bounded exception, never silently faked with a plausible-looking but fabricated return value.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-mod-api --all-features
cargo build -p rc-mod-api --no-default-features --features wasm-tier
cargo build -p rc-mod-api --no-default-features --features native-tier
cargo nextest run -p rc-mod-api --all-features
cargo test --doc -p rc-mod-api --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-mod-api --all-features` runs all 8 (`identifier.rs`) + 7 (`manifest_parsing.rs`) + 11 (`manifest_validation.rs`) + 7 (`abi_handshake.rs`) + 7 (`component_descriptor.rs`) + 6 (`access_roundtrip.rs`) + 6 (`registry_ids.rs`) + 1 (`guest_bindings_compile.rs`) = 53 test cases named in Acceptance tests — all pass. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
