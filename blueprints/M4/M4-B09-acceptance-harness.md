# M4-B09 — Acceptance Harness: Region-Boundary Delta, Hopper Cadence, AI/Combat Scenario Suite, M4 Completion Report

| Field | Content |
|---|---|
| ID | M4-B09 |
| Milestone | M4 — Mechanics Tier 2: Entities, AI, Combat, Items |
| Prerequisites | M4-B03 (`rc-mechanics::ai` — `Goal`/`GoalSelector`/`AiContext`, `Brain`/`BrainProgram`/`Sensor`/`Behavior`/`Activity`, `WalkNodeEvaluator`/`find_path`/`Path`/`PathType`/`PathTypeTable`, `PathNavigation`/`MoveControl`/`LookControl`, `Sensing`/`nearest_within_range`/`raycast_line_of_sight`, `ai::attributes::{AttributeMap, AttributeInstance, AttributeModifier, AttributeModifierOperation}`, `should_full_tick`, the tier-2 Zombie/Cow/Villager goal/activity tables — read in full, reused unmodified below except for two small, cited, additive corrections this blueprint makes and justifies, Context Parts B/C). M4-B05 (`rc-mechanics::combat` — `apply_damage_pipeline`/`DamageOutcome`/`DamageSource`/`DamageTypeKind`/`Difficulty`/`GlobalDifficulty`, `assemble_player_melee_damage`/`assemble_mob_melee_damage`/`attack_cooldown_charge_scale`/`can_critical_attack`/`CombatRuntimeState`, `apply_knockback_impulse`/`get_knockback`, `calculate_fall_damage`, `PendingMeleeAttack`, `EntityLootProvider`/`FixedTierTwoLoot`, `HardcodedWorld::{debug_spawn_mob, debug_deal_damage, debug_override_attribute, debug_query_entity, debug_set_difficulty}`, the `Interact`/`Damage Event`/`Set Health`/`Update Attributes`/`Entity Event` packets — read in full, reused unmodified below except for the same two corrections, Context Parts B/C). M4-B08 (`rusty-clanker-server::play::{TwoRegionWorld, REGION_WEST_ID, REGION_EAST_ID, BOUNDARY_CHUNK_X}` and its own `debug_query_player_position`/`queue_join`/`debug_spawn_mob`/`debug_move_mob`/`debug_query_mob`; `crates/mechanics/tests/hopper_cross_chunk_border.rs`'s and `crates/server/tests/play_region_transfer_player_walk.rs`'s own already-complete, already-passing acceptance tests — read in full, reused and integrated unmodified below, never re-derived). Also M4-B01 (`rc-mechanics::entity` — `BaseEntity`, `LivingEntity`, `EntityKind{Item,Zombie,Villager,Cow}`, `MobMarker`/`AiSystemKind`, `RcEntityId` — restated only to the field level this blueprint's own scenario harness needs). Also M4-B02 (`rc-mechanics::entity::physics::ecs::register_stage6b`) and M4-B04 (`rc-mechanics::entity::ecs::register_mob_despawn`) — read only for their own `register_*(builder: &mut RcExecutorBuilder)` signatures and each one's own already-stated `order_tag` claim, the two other real registrants into `DomainGroup::EntityPhysicsIntegration` this blueprint's own Context Part I reconciles alongside M4-B05's `register_mob_combat_system`. Also the established harness architecture this blueprint extends without reinventing: M1-B06 (`rc_test_harness::process`, the `xtask m<n>-report`/`target/verify/m<n>-acceptance.json` shape, `TierResult`/`CaseResult`/`Status` from `xtask::tier_result`, `ChangesetType`/`path_guard::check_paths`), M2-B08 (the `Mode`-free precedent is this blueprint's own departure, justified in Context Part A — every other structural convention reused), M3-B07 (`rc_gametest`'s own established "pure core + lightweight non-`bevy_ecs` replay world" pattern — `ReplayWorld`, `RegionOwnership::always_local`, `xtask::fixture_manifest::{build_manifest, verify_manifest}` — this blueprint's own `ai_scenario` module is a direct, sibling extension of that exact pattern, restated in full below since M3-B07's own file is not itself a dependency any crate here needs at compile time), M3-B08 (the fullest prior expression of this harness architecture — `Mode`/self-test-against-synthetic-bad-input discipline, `build_report`'s own aggregation-proven-by-perturbation pattern — restated and adapted, not copied verbatim, since M4's own criteria need no live oracle, Context Part A). |
| Implements | `11-roadmap-milestones.md`'s M4 Acceptance Criteria 1–3, verbatim, restated in Context Part A and mapped 1:1 onto this blueprint's report cases. ARCH-D10 (cross-region transfer — the criterion-1 integration, unmodified from M4-B08). ARCH-D17 (cross-chunk-same-region hopper collapse — the criterion-2 integration, unmodified from M4-B08). MECH-D31/D32/D33 (AI/pathfinding — exercised end-to-end for the first time across multiple entities and many ticks, criterion 3). MECH-D40/D43–D46 (combat/damage — exercised end-to-end through a real AI-selected target for the first time, criterion 3). TEST-D37/D40 (CI-tier placement and machine-readable tier output, restated concretely — with one structural simplification over every prior harness blueprint, Context Part A). TEST-D42 (RON-authored gametest structures — this blueprint's own `ai_combat` corpus). TEST-D45/D46 (test-first changeset split, restated). TEST-D47 (fixture integrity manifest — this blueprint's own `corpus/ai_combat/manifest.json`). TEST-D50 (CI-is-authority). |
| Crates touched | `crates/testing/test-harness/` (`rc-test-harness`, additive: `position_delta.rs`). `crates/testing/gametest/` (`rc-gametest`, additive: a new `ai_scenario` module — `spec.rs`, `world.rs`, `assertions.rs`; new `corpus/ai_combat/*.ron` + `manifest.json`; new `tests/ai_combat_scenarios.rs` (scenarios 1–9), `tests/ai_scenario_harness_self_tests.rs`). `crates/mechanics/` (`rc-mechanics`, additive-with-two-cited-corrections: `src/ai/goal.rs` (`AiContext` gains three fields), `src/ai/attributes.rs` (four new registry rows), `src/ai/mob_config.rs` (concrete `ZombieAttackGoal`/`HurtByTargetGoal`/Cow-`PanicGoal`/Villager sensor+behavior bodies — Context Part C), `src/combat/death.rs` (`PendingMeleeAttack` reshaped — its own already-established home file, M4-B05's own `combat/mod.rs` re-exports it from `death`, not `melee`), `src/combat/ai_bridge.rs` (new — `RecentDamage`), `src/combat/attributes.rs`/`src/combat/mod.rs`/`src/combat/damage.rs`/`src/combat/knockback.rs`/`src/combat/fall.rs` (the `AttributeKind`→registry-constant reconciliation, Context Part B)). `crates/server/` (`rusty-clanker-server`, additive-with-cited-fixes: `src/play/world.rs` (`HardcodedWorld::debug_override_attribute`'s second parameter retyped, Context Part B; the executor-build step's three `register_*` calls ordered per Context Part I); `src/play/attribute_packets.rs` (gutted to a re-export, Context Part B); `src/play/combat_packets.rs` (one test's attribute-key reference fixed, Context Part B); new `tests/ai_combat_melee_scenarios.rs` (scenarios 10–11)). `crates/mechanics/tests/entity_physics_integration_group_registration.rs` (new, Context Part I). `xtask` (`src/m4_report.rs`, new; `src/main.rs`'s `Command` enum; `src/path_guard.rs`, one new row). `.github/workflows/ci.yml` (one new job, `m4-acceptance` — PR-blocking, Context Part A). |
| Estimated scope | L |

## Goal & Done definition

Close the two real, load-bearing gaps M4-B03 and M4-B05 each independently, explicitly left open for "whichever blueprint wires the milestone together" (M4-B03's own words) — an accidental duplicate `AttributeMap` type and an unclosed AI-decision-to-combat-action production seam — and then use the now-coherent substrate to give M4 the same kind of real, agent-executable, per-criterion measurement M1-B06 gave M1, M2-B08 gave M2, and M3-B08 gave M3: (1) integrate M4-B08's own already-complete, already-passing region-boundary position-delta test into a unified report, plus a standalone, reusable, independently-tested restatement of the exact "no discontinuity beyond the one-tick budget" formula; (2) integrate M4-B08's own already-complete, already-passing cross-chunk hopper-cadence test into the same report; (3) an eleven-scenario, qualitative AI/combat behavioral-envelope suite — pathfinding routing around obstacles and refusing an over-deep drop, sensing range/line-of-sight gating, passive-vs-hostile target-acquisition boundaries, damage-triggered aggro and its expiry, a live proof that the pure `GoalSelector` priority-eviction algorithm still holds once driven by a real, multi-entity, many-tick harness, and two exact combat-envelope scenarios (cooldown-timed hits on an armored target; charged-critical vs. uncharged damage) — each scenario stating its own setup, script, and explicit tolerance band, honestly framed as behavioral parity, never bit-exactness, per the milestone's own text; (4) one unified `xtask m4-report` and `target/verify/m4-acceptance.json`, aggregating all three criteria; (5) harness self-tests proving the report's own analysis/aggregation logic — not merely its plumbing — actually catches a broken input: a synthetic teleport-glitch position log, a synthetic hopper-transfer log faster than the legal cooldown, and a synthetic "mob never moved" scenario trace; (6) fix the third real load-bearing gap M4-B03/M4-B05 left open for whichever blueprint wires the milestone together — M4-B02, M4-B04, and M4-B05 each independently register a real system into `DomainGroup::EntityPhysicsIntegration` with no stated call order relative to the other two — by fixing that composition-root order and proving, with a real acceptance test, that the three co-register without an `AmbiguousMutationAuthority`-class error (Context Part I).

**Structural simplification over every prior harness blueprint, stated up front.** M1/M2/M3's own harnesses each needed a live vanilla oracle process (differential comparison, redstone-trace replay) and therefore a `Mode::{Smoke, Full}` duration split, an oracle-bootstrap step, and a Tier-2/manual CI placement for their own real end-to-end run. **No criterion this blueprint measures has a vanilla oracle at all** — M4-B08's own Context already established this for criteria 1/2 ("every mechanism in this blueprint is server-internal bookkeeping... that has no vanilla analog at all"), and the milestone's own acceptance-criteria text establishes it for criterion 3 directly ("entity AI has no equivalent public bit-exact reference to compare against"). Every test this blueprint's own report runs is therefore a plain, hermetic `cargo nextest` run — no Java, no downloaded jar, no spawned `rusty-clanker-server` subprocess anywhere in this blueprint's own pipeline (M4-B08's own criterion-1 test does open a real loopback socket, but against its own in-process `TwoRegionWorld`, never an external process). `xtask m4-report` therefore takes **no `--mode` flag** (a deliberate simplification, not an oversight — every criterion's own cost is already small enough that "smoke" and "full" would be identical, Context Part A) and this blueprint's own new CI job runs on **every PR**, not gated to a nightly/`workflow_dispatch` cadence the way `m1-acceptance`/`m2-acceptance`/`m3-acceptance` all are.

Done when:

- [ ] `cargo build -p rc-test-harness -p rc-gametest -p rc-mechanics -p rusty-clanker-server -p xtask --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-test-harness -p rc-gametest -p rc-mechanics -p rusty-clanker-server -p xtask`.
- [ ] `cargo nextest run -p rusty-clanker-server -p rc-mechanics -p rc-gametest` continues to pass every test M4-B01/M4-B03/M4-B05/M4-B08 already shipped, unmodified — the two cited corrections (Context Parts B/C) touch only files those blueprints' own Deliverables named, never a test file.
- [ ] `entity_physics_integration_group_registration.rs`'s test passes: `register_stage6b` (M4-B02), `register_mob_despawn` (M4-B04), `register_mob_combat_system` (M4-B05) called in that order against one `RcExecutorBuilder`, `.build()` returns `Ok(_)` with each system's `order_tag` equal to `0`/`1`/`2` respectively (Context Part I).
- [ ] `cargo run -p xtask -- m4-report` exits 0 against a from-scratch build and writes `target/verify/m4-acceptance.json` with `status: "pass"` — unlike M1-B06/M2-B08/M3-B08, this **is** part of this blueprint's own Done state (Context Part A: no oracle gate exists to defer it behind).
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changesets (labeled per Constraints).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-test-harness -p rc-gametest -p rc-mechanics -p rusty-clanker-server` exits 0.
- [ ] CI tier: the new `m4-acceptance` job green on both `ubuntu-24.04` and `windows-2025`, on every PR, on a clean checkout (TEST-D34/D37/D50) — this blueprint's own Done state, not a later milestone-signal-only event (contrast M1-B06/M2-B08/M3-B08's own `m<n>-acceptance` jobs, each deferred behind an oracle/subprocess gate).

## Context (self-contained)

### Part A — No oracle, no `Mode`, Tier-1 placement: the concrete CI shape

Every case this blueprint's report produces resolves to one of exactly two mechanisms, both hermetic:

1. **Subprocess-exit-code integration** (criteria 1 and 2): `xtask m4-report` shells out to `cargo nextest run -p <crate> <test-name-substring> --no-tests fail` (nextest's positional substring filter, restricted to one already-shipped, already-passing M4-B08 test per case; `--no-tests fail` — moderate confidence on this exact flag spelling against the pinned nextest 0.9.137, TEST-D2 — makes "the filter matched zero tests" a hard failure rather than nextest's own default silent-pass, closing the exact "a fake CI run where the delta leg silently never ran" gap TEST-D49's own forbidden-pattern-lints spirit warns against) and records the child's exit code as that case's `Status`. This is **integration**, restated precisely: this blueprint never re-implements M4-B08's own position-delta or hopper-cadence assertions, it runs the real, already-committed test and trusts nextest's own per-process exit code (TEST-D2's own process-isolation guarantee) exactly as `m1_report`/`m2_report`/`m3_report` already trust `parity_check::run`'s own `TierResult`.
2. **In-process scenario execution** (criterion 3): `xtask m4-report` runs `rc_gametest::ai_scenario`'s own scenario suite directly, in-process, against `rc-gametest`'s own lightweight `ScenarioWorld` (Part D) — no subprocess at all, mirroring `parity_check::run`'s own "call the pure comparison function directly" shape more than `m3_report`'s own "spawn `rusty-clanker-server`" shape, since nothing here needs a live server.

Both mechanisms complete in low single-digit seconds total (eleven scenarios of a few hundred simulated — never wall-clock — ticks each, plus two nextest subprocess spawns) — comfortably inside Tier 1's `< 10 min` budget (TEST-D37), which is exactly why this blueprint places its own `m4-acceptance` job there, PR-blocking, rather than replicating M1/M2/M3's own nightly/`workflow_dispatch` placement. **Runtime budget, stated as this blueprint's own concrete number**: the full `m4-report` run (both integration cases plus all eleven scenarios) completes in **≤ 20 seconds** on the reference hardware TEST-D32 already names, with generous headroom against CI-runner variance — restated as a real, checkable number per this project's own "concrete numbers matter" convention (TEST-D32's own rationale, applied here), verified by this blueprint's own acceptance test (`m4_report_completes_within_the_stated_budget`, below).

**M4's own three acceptance criteria, restated verbatim** (`11-roadmap-milestones.md`):

> 1. A player walks across a live region boundary (two independently-ticking regions, still monolithic — no cluster mode) with position-delta logging on the client showing no observable discontinuity beyond ARCH-D10's documented one-tick transfer budget.
> 2. An automated test confirms a hopper chain crossing a chunk border within one region transfers items at vanilla's correct tick cadence.
> 3. A scripted scenario suite confirms mob AI pathfinding routes around obstacles and engages in combat consistent with vanilla behavioral expectations — this criterion is qualitative/behavioral parity, explicitly distinguished from `M3`'s bit-exact redstone-trace standard, since entity AI has no equivalent public bit-exact reference to compare against.

"On the client" (criterion 1): no Phase-2 client exists yet (`CLAUDE.md`'s own phase framing — Phase 1 server only). M4-B08's own Context already resolved this honestly: `debug_query_player_position`, sampled once per tick from a real, network-connected `TwoRegionWorld` session, is the observable-from-the-client's-own-perspective position (the server's own authoritative value for exactly what a real client would be told), the correct interim reading of "on the client" until Phase 2 exists — reused here unmodified, not re-litigated.

### Part B — Reconciling M4-B03's and M4-B05's independently-invented `AttributeMap` types

M4-B03 and M4-B05 were derived in parallel and, per this project's own established "each blueprint binds only to its own named prerequisites, never a sibling wave-2 blueprint" discipline (M4-B03's own Context: "M4-B02... is not read or bound to"), neither reads the other. Each independently built a complete attribute system for the same entities:

- M4-B03's `crate::ai::attributes::AttributeMap` — keyed by `rc_registries::generated_v776::registries::RegistryEntryId` (the real `minecraft:attribute` registry, data-driven, wire-packet-ready), twelve constants (`MAX_HEALTH, MOVEMENT_SPEED, FOLLOW_RANGE, ATTACK_DAMAGE, ATTACK_KNOCKBACK, KNOCKBACK_RESISTANCE, ARMOR, ARMOR_TOUGHNESS, STEP_HEIGHT, JUMP_STRENGTH, BLOCK_INTERACTION_RANGE, ENTITY_INTERACTION_RANGE`), with a per-tier-2-kind default table already populated for every combat-relevant attribute — M4-B03's own text names this explicitly: *"`ATTACK_DAMAGE`/`ATTACK_KNOCKBACK`/`KNOCKBACK_RESISTANCE`/`ARMOR`/`ARMOR_TOUGHNESS` exist in every tier-2 kind's `AttributeMap`... purely so that map is complete per-kind"* — i.e. M4-B03 already anticipated exactly this reconciliation.
- M4-B05's `crate::combat::attributes::AttributeMap` — keyed by a hand-rolled `AttributeKind` enum (`AttackDamage, AttackKnockback, AttackSpeed, Armor, ArmorToughness, KnockbackResistance, MaxHealth, SafeFallDistance, FallDamageMultiplier, SweepingDamageRatio`), a smaller, combat-only, non-registry-backed type.

Two entity attribute systems on the same entity, disagreeing on both the key type and the module path for six overlapping concepts (`ATTACK_DAMAGE`/`AttackDamage`, `ARMOR`/`Armor`, etc.), is not a documentation inconsistency — it is a genuine compile-time/design conflict a mob spawned by this blueprint's own scenario harness would otherwise sit directly on top of. **Binding resolution, cited and justified exactly as M4-B08 cited its own correction to M4-B01's per-region `NetworkEntityIdAllocator` scope**: `crate::ai::attributes::AttributeMap` (M4-B03's, registry-keyed) is the **one** attribute system for every entity in the engine, for the concrete reason M4-B03's own text already gives — it is the strictly more general, wire-packet-ready, already-extensible design, and it already reserves space for exactly this. `crate::combat::attributes::{AttributeKind, AttributeInstance, AttributeMap, ModifierOperation}` (M4-B05's own file, `crates/mechanics/src/combat/attributes.rs`) are **retired** — this blueprint's own governance changeset deletes that file's type definitions and replaces `crates/mechanics/src/combat/mod.rs`'s own `pub mod attributes; pub use attributes::{...}` lines with `pub use crate::ai::attributes::AttributeMap;` (re-exported under `combat`'s own namespace, so any call site that already wrote `rc_mechanics::combat::AttributeMap` keeps compiling unchanged).

Four attribute concepts M4-B05 needs and M4-B03's own twelve-constant list does not yet declare are added, additively, to M4-B03's own registry-attribute table (`crates/mechanics/src/ai/attributes.rs`) and its per-kind default table:

| New registry constant | Default `[min, max]` | Zombie | Villager | Cow | Consumed by (M4-B05, unmodified formula) |
|---|---|---|---|---|---|
| `ATTACK_SPEED` | `4.0 [0.0, 1024.0]` | `4.0` | `4.0` | `4.0` | player-only cooldown-charge curve (§3.9a); mob melee path never reads it (§3.13, no charge curve) |
| `SAFE_FALL_DISTANCE` | `3.0 [-1024.0, 1024.0]` | `3.0` | `3.0` | `3.0` | fall damage — players only |
| `FALL_DAMAGE_MULTIPLIER` | `1.0 [0.0, 100.0]` | `1.0` | `1.0` | `1.0` | fall damage — players only |
| `SWEEPING_DAMAGE_RATIO` | `0.0 [0.0, 1.0]` | `0.0` | `0.0` | `0.0` | vestigial — M4-B05's own sweep formula reads `sweeping_edge_level` from its `EnchantLevels` parameter, never this attribute; retained only for `Update Attributes` registry completeness, matching M4-B05's own already-stated reason for carrying it at all |

Every one of these four rows' own numeric values is copied verbatim from M4-B05's own already-cited table (`19-combat-damage.md` §4) — no new number is invented here, only the type/module each lives in changes.

**Key-mapping table** (the mechanical substitution this blueprint's own governance changeset applies to every already-merged M4-B05 file that reads an attribute — `damage.rs`, `melee.rs`, `knockback.rs`, `fall.rs`, and (below) `combat_packets.rs`'s own `update_attributes_encodes_empty_modifier_arrays` test; every formula those files already implement, per M4-B05's own Context, is unchanged byte-for-byte — only the attribute-lookup key type changes):

| M4-B05's retired `AttributeKind` variant | Binding replacement |
|---|---|
| `AttackDamage` | `ai::attributes::registry::ATTACK_DAMAGE` (already existed, M4-B03) |
| `AttackKnockback` | `ATTACK_KNOCKBACK` (already existed) |
| `AttackSpeed` | `ATTACK_SPEED` (new, above) |
| `Armor` | `ARMOR` (already existed) |
| `ArmorToughness` | `ARMOR_TOUGHNESS` (already existed) |
| `KnockbackResistance` | `KNOCKBACK_RESISTANCE` (already existed) |
| `MaxHealth` | `MAX_HEALTH` (already existed) |
| `SafeFallDistance` | `SAFE_FALL_DISTANCE` (new) |
| `FallDamageMultiplier` | `FALL_DAMAGE_MULTIPLIER` (new) |
| `SweepingDamageRatio` | `SWEEPING_DAMAGE_RATIO` (new) |

`AttributeInstance`/`ModifierOperation` (M4-B05) are likewise retired in favor of M4-B03's own `AttributeInstance`/`AttributeModifierOperation` (field-for-field equivalent shape; only the name `ModifierOperation` → `AttributeModifierOperation` and `AttributeMap::get(kind) -> f64` → `AttributeMap::get(attr).map(|i| i.value())`/`value_or(attr, default)` change at call sites). `default_attributes_for(kind)` (M4-B05) is retired; `mob_config::default_attribute_map(kind)` (M4-B03), extended with the four new rows above, is the one binding constructor — every production/test call site that constructed a mob's starting attributes via M4-B05's own function is redirected to this one.

**One more cited call-site consequence.** `HardcodedWorld::debug_override_attribute` (M4-B05, `rusty-clanker-server::play::world`) takes `kind: rc_mechanics::combat::AttributeKind` as its second parameter — a direct reference to the now-retired type. This blueprint's own governance changeset retypes that one parameter to `kind: rc_registries::generated_v776::registries::RegistryEntryId` (`crates/server/src/play/world.rs`, one signature edit; the function body's own `NetworkEntityIndex` lookup and `AttributeMap` mutation are otherwise unchanged). Every other `HardcodedWorld::debug_*` method M4-B05/M4-B08 defined is unaffected — this is the only call site anywhere in either prerequisite blueprint's own Deliverables that names `AttributeKind` outside `rc-mechanics` itself.

**Retiring the duplicate `UpdateAttributes` wire packet.** M4-B03 and M4-B05 each independently ship a complete, hand-implemented `UpdateAttributes` clientbound packet at the identical protocol id `0x83` — `crates/server/src/play/attribute_packets.rs::UpdateAttributes` (M4-B03, a full per-modifier `Identifier`/`amount`/`operation` array) and `crates/server/src/play/combat_packets.rs::UpdateAttributes` (M4-B05, always `modifier_count = 0`, a numeric registry-id convention) — the identical shape of conflict as the two `AttributeMap` types above, and left unaddressed by an earlier draft of this Part. **Binding resolution**: `combat_packets::UpdateAttributes` (M4-B05's) is the one packet definition in the engine; `attribute_packets.rs`'s own struct/`impl RcPacket`/`build_update_attributes` are retired — this blueprint's own governance changeset deletes those three items and replaces the file with a single re-export, `pub use crate::play::combat_packets::UpdateAttributes;`, so `play/mod.rs`'s own already-established `mod attribute_packets; pub use attribute_packets::UpdateAttributes;` lines keep compiling unchanged (mirroring the re-export shape already used for `combat::attributes`, above). `combat_packets::UpdateAttributes` is chosen over `attribute_packets::UpdateAttributes` because a production call site (`combat.rs`'s own future health/attribute-sync wiring, M4-B05's own established pattern) is the more likely real sender at M4's own scope — M4-B03 never wires its own tracking/AI systems into `HardcodedWorld`'s live tick loop (Constraint (e), restated), so `attribute_packets::UpdateAttributes` was never actually constructed by any production code path either way. The retired packet's own fuller per-modifier wire shape (real `AttributeModifier` values, not just base values) is not lost as a *capability* — it is simply not implemented by either surviving packet definition at M4's own scope, restated as an explicit, bounded simplification (identical in kind to M4-B05's own already-cited "never sends a live `AttributeModifier` over the wire" scope note) rather than silently dropped; a future blueprint that needs to send real modifiers extends `combat_packets::UpdateAttributes`'s own field shape, the one surviving definition, rather than reviving the retired one.

**What this reconciliation does *not* touch**: every one of M4-B05's own already-merged, already-passing *pure-formula* unit tests (`combat_damage_pipeline.rs`, `combat_melee_assembly.rs`, `combat_knockback.rs`, `combat_fall_food.rs`) calls its formulas with **plain `f64`/`f32` parameters** (`armor_effective_damage(damage, total_armor, toughness)`, never an `AttributeMap` directly) — per M4-B05's own Deliverables, the pipeline math is a pure function of plain numbers, with attribute *lookup* happening only in the adapter layer (`crates/server/src/play/combat.rs`) that reads a live entity's attributes to produce those numbers. This reconciliation touches only that adapter layer and the `AttributeMap` type itself — not one already-merged test file, satisfying the changeset-boundary rule (Constraints) without needing an exception.

### Part C — Closing the AI-decision → combat-action production seam

M4-B03's own Context names this gap directly and predicts this exact blueprint closes it: *"deciding when a mob attacks... is Stage-6a AI/goal-selector content... M4-B05 defines the exact, minimal contract that future blueprint's Stage-6a system must produce, and implements the Stage-6b consumer against it now, so the two blueprints can land independently."* Reading both blueprints' own texts together shows the contract is real but was never actually wired: `PendingMeleeAttack { target: RcEntityId }` (M4-B05) is described as *"written by a Stage-6a system... consumed and cleared by this blueprint's own Stage-6b `apply_mob_melee_attacks`"* — but M4-B03's own Stage-6a discipline (its own Context, restated: a Stage-6a system's `Commands` are silently discarded, MECH-D32) means a Stage-6a `Goal` genuinely cannot *add* a new component via `Commands`, only mutate one it already owns via `Query<&mut T>`. The fix is small and mechanical, not a redesign:

1. **`PendingMeleeAttack` is reshaped from a Commands-added marker to an always-attached, `Option`-valued field** (`crates/mechanics/src/combat/death.rs` — its own already-established home file per M4-B05's own `combat/mod.rs` re-export line, `pub use death::{EntityLootProvider, FixedTierTwoLoot, PendingMeleeAttack};`, restated so this is never mistaken for `melee.rs`; cited correction to M4-B05):
   ```rust
   #[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
   pub struct PendingMeleeAttack(pub Option<rc_core::RcEntityId>);
   ```
   Attached (as `PendingMeleeAttack::default()`, i.e. `None`) at every mob-spawn call site alongside `AttributeMap`/`CombatRuntimeState` (M4-B05's own `HardcodedWorld::debug_spawn_mob`, and this blueprint's own scenario-spawn helper, Part D). M4-B05's own Stage-6b `apply_mob_melee_attacks` system's query gains `&mut PendingMeleeAttack` in place of an add/remove `Commands` pair; its own already-specified logic (Context §"Mob melee attacks") is unchanged except its entry condition becomes `if let Some(target) = attack.0.take() { ... }` (a direct field read-and-clear, not a structural removal) — every downstream line of that already-specified algorithm (`assemble_mob_melee_damage`, `apply_damage_pipeline`, `apply_knockback_impulse_2`) is untouched.
2. **`RecentDamage`, new** (`crates/mechanics/src/combat/ai_bridge.rs`, new file): the symmetric input signal M4-B03's own `AiContext.hurt_by` needs (its own Context already assumes this exists: *"an `AiContext`-supplied `hurt_by: Option<RcEntityId>` this blueprint does not itself populate, an explicit, bounded seam"* — restated here as the concrete component backing that assumption).
   ```rust
   //! The Stage-6b(damage) -> Stage-6a(next tick, AI) bridge M4-B03's own `AiContext.hurt_by`
   //! field assumes exists (M4-B03 Context, §D/§E) and M4-B05's own `apply_damage_pipeline`
   //! is the natural producer for (Context, Part C).

   /// One tick's "this entity was just damaged, by whom" pulse — set by
   /// `apply_damage_pipeline`'s own adapter whenever a `LivingEntity`+`MobMarker` target
   /// takes nonzero net damage this tick from a source with a resolvable attacking
   /// `RcEntityId`; read and cleared exactly once, at the start of the *next* Stage-6a
   /// tick, by whichever adapter constructs that tick's `AiContext` — matching
   /// `HurtByTargetGoal`'s/`HurtBySensor`'s own "this entity was just damaged this tick"
   /// framing (M4-B03) precisely: a genuine one-tick pulse, never a sticky flag.
   #[derive(bevy_ecs::prelude::Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
   pub struct RecentDamage(pub Option<rc_core::RcEntityId>);
   ```
   Attached (`RecentDamage::default()`) alongside `PendingMeleeAttack` at every mob-spawn call site. `apply_damage_pipeline`'s own adapter (`crates/server/src/play/combat.rs`, M4-B05, additive one-line edit) sets `target.recent_damage.0 = Some(attacker_id)` on any `DamageOutcome::{Dealt, Died}` outcome against a mob target whose damage source carries a resolvable attacking `RcEntityId` (both this blueprint's own `DamageTypeKind::PlayerAttack`/`MobAttack` sources always do; `Fall`/`Starve` never do and never set it).
3. **`AiContext` gains three fields** (`crates/mechanics/src/ai/goal.rs`, cited additive correction to M4-B03):
   ```rust
   pub hurt_by: Option<rc_core::RcEntityId>,                    // input — Part C.2, above
   pub melee_attack_signal: &'a mut Option<rc_core::RcEntityId>, // output — backs PendingMeleeAttack.0 directly
   pub current_target: Option<rc_core::RcEntityId>,              // input — this tick's target_selector output,
                                                                   // populated by the adapter (a future production
                                                                   // Stage-6a system, or this blueprint's own
                                                                   // ScenarioWorld, Part D) before goal_selector
                                                                   // ticks, since target_selector ticks first
                                                                   // (Part D's own fixed ordering) and a goal like
                                                                   // ZombieAttackGoal needs to read its result
                                                                   // without re-deriving it from target_selector's
                                                                   // own internal state directly
   ```
   All three are plain-Rust-struct fields — `AiContext` was already constructed by borrowing owned state directly (`navigation: &'a mut PathNavigation` is the identical shape already), so this is a pure, additive field addition with no bevy_ecs/Commands implication either way; whichever adapter constructs `AiContext` each tick (a future production Stage-6a system, *or* this blueprint's own scenario harness, Part D) supplies both by direct field borrow.

4. **Concrete goal/sensor/behavior bodies** M4-B03 left to "implementer's freedom" (its own Deliverables state only the `Goal`/`Sensor`/`Behavior` trait surface and a priority *table*, never a concrete struct body) are pinned here, precisely enough to drive and assert against, reusing M4-B03's own already-given `can_use` conditions verbatim:

   ```
   pub const MELEE_ATTACK_RANGE: f64 = 1.5;   // this blueprint's own moderate-confidence
                                                // melee-adjacency constant (vanilla's real
                                                // per-mob attack reach varies by hitbox and
                                                // is not pinned by any merged blueprint;
                                                // flagged for reconciliation)
   pub const HURT_BY_MEMORY_TTL_TICKS: u32 = 100;  // this blueprint's own moderate-confidence
                                                     // Villager `HurtBy` memory expiry — no
                                                     // merged blueprint pins vanilla's real
                                                     // value; flagged for reconciliation

   struct ZombieAttackGoal;
   impl Goal for ZombieAttackGoal {
       fn flags(&self) -> u8 { FLAG_MOVE | FLAG_LOOK }
       fn can_use(&self, ctx: &AiContext) -> bool {
           // ctx.current_target is Some(t) and horizontal+vertical distance to t's own
           // tracked position <= MELEE_ATTACK_RANGE.
       }
       fn tick(&mut self, ctx: &mut AiContext) {
           // if still in range: *ctx.melee_attack_signal = ctx.current_target; else None.
       }
   }

   struct HurtByTargetGoal;
   impl Goal for HurtByTargetGoal {
       fn flags(&self) -> u8 { FLAG_TARGET }
       fn can_use(&self, ctx: &AiContext) -> bool { ctx.hurt_by.is_some() }
       fn start(&mut self, ctx: &mut AiContext) { /* sets this entity's own current_target output to ctx.hurt_by */ }
   }
   ```
   Cow's `PanicGoal` (`can_use: ctx.hurt_by.is_some()`, flees at a `2.0` navigation-speed modifier away from `hurt_by`'s own last-known position — the Cow's own panic speed modifier; the Villager has no `FleeFromHostile` goal at all, since the villager is brain-driven, not goal-driven, for this behavior — its own panic activity package computes a runaway speed of `1.5×` the *villager's own registered speed modifier* (`0.5`, giving an effective `0.75`), so the two kinds' flee speeds are independent numbers, not a shared multiplier) and Villager's `HurtBySensor` (`tick`: `if let Some(id) = ctx.hurt_by { brain.set(MemoryModuleType::HurtByEntity, id, Some(HURT_BY_MEMORY_TTL_TICKS)); }` — vanilla's own `HurtBy` module holds the damage source itself, not an attacker id, and this blueprint's own `AiContext.hurt_by` carries only a resolvable attacker id, so this blueprint's own sensor sets only `HurtByEntity`, with this blueprint's own explicit `HURT_BY_MEMORY_TTL_TICKS` standing in for vanilla's own damage-source-driven, TTL-argument-free expiry) follow the identical `hurt_by`-gated shape — restated once, not per-kind, since both are direct instantiations of the same pattern M4-B03's own priority/activity tables already name.

### Part D — The scenario harness: a lightweight, `bevy_ecs`-free replay world, mirroring M3-B07's own established pattern

M3-B07's own `rc_gametest` crate already establishes, for redstone, the exact right shape for this: *not* a real `bevy_ecs::World`/`RcExecutor` (which would require standing up production Stage-scheduling and, for Stage 6a specifically, would collide with the "Commands are silently discarded" discipline Part C above exists to route around at the *type* level, not the *dispatch* level) — but a small, direct, pure-function-driven **replay world** (M3-B07's own `ReplayWorld`, driving `stage4::run_scheduled_phase` directly). This blueprint's own `ai_scenario` module is a sibling extension of that exact pattern, for Stage 6a/6b instead of Stage 4:

```rust
// crates/testing/gametest/src/ai_scenario/world.rs — new

use std::collections::HashMap;
use rc_core::{BlockPos, RcEntityId};
use rc_chunk_storage::BlockStateId;
use rc_mechanics::ai::{AiContext, GoalSelector, Sensing, PathNavigation, PendingMovementIntent};
use rc_mechanics::ai::brain::{Brain, BrainProgram};
use rc_mechanics::ai::attributes::AttributeMap;
use rc_mechanics::combat::{PendingMeleeAttack, ai_bridge::RecentDamage};
use rc_mechanics::entity::{BaseEntity, LivingEntity, EntityKind, kinds::MobMarker};

/// A trivial, `HashMap`-backed `BlockWorldAccess` — every block this suite's own scenarios
/// need (a wall, a pit, open air) fits a hand-populated map; no real chunk/Anvil storage
/// is exercised (M4's own roadmap boundary: "world content remains superflat filler until
/// M5" — restated). Unlisted positions default to air (`BlockStateId::AIR`, `rc_registries`'
/// own generated constant). `owner_of`/`local_identity` are both fixed to one local
/// placeholder region — every scenario in this suite is single-region by construction, no
/// cross-region traffic is ever exercised here (that is M4-B08's own, already-integrated,
/// job — Context Part A).
pub struct ScenarioWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    pub mobs: HashMap<RcEntityId, ScenarioMob>,
    pub players: HashMap<RcEntityId, ScenarioPlayerProxy>,
    pub tick_count: u64,
}

/// A minimal, generic target stand-in — deliberately *not* `PlayerMarker` (`rc-mechanics`
/// must never depend on `rusty-clanker-server`-only types, WS-D3 rule 2, restated) and
/// deliberately just an `(RcEntityId, position)` pair, matching `nearest_within_range`'s
/// own already-generic candidate-list signature (M4-B03) exactly — this scenario harness's
/// own concrete resolution of the "how does a Stage-6a adapter learn where players are"
/// question M4-B03's own text leaves unanswered (a real production adapter's own answer,
/// e.g. a shared resource a composition root populates, is out of this blueprint's scope
/// — restated, Constraints).
pub struct ScenarioPlayerProxy {
    pub id: RcEntityId,
    pub pos: [f64; 3],
}

pub struct ScenarioMob {
    pub id: RcEntityId,
    pub kind: EntityKind,
    pub base: BaseEntity,
    pub living: LivingEntity,
    pub mob_marker: MobMarker,
    pub attributes: AttributeMap,
    pub sensing: Sensing,
    pub navigation: PathNavigation,
    pub movement_intent: PendingMovementIntent,
    pub goal_selector: GoalSelector,
    pub target_selector: GoalSelector,
    pub current_target: Option<RcEntityId>,       // this tick's target_selector output
    pub brain: Option<(Brain, BrainProgram)>,      // `Some` only for Villager
    pub pending_melee_attack: PendingMeleeAttack,
    pub recent_damage: RecentDamage,
}

impl ScenarioWorld {
    pub fn new() -> Self;
    /// Populates one block (this blueprint's own scenario-spec loader, `spec.rs`, calls
    /// this once per RON-declared block entry).
    pub fn set_block(&mut self, pos: BlockPos, state: BlockStateId);
    /// Spawns one tier-2 mob with a full, correctly-defaulted component set — the "union
    /// of M4-B03's AI components and M4-B05's combat components" helper neither prerequisite
    /// blueprint provides alone (Context, Part C's own framing — this is that same closure,
    /// applied to spawn-time construction rather than the tick-time signal path).
    /// `goal_selector`/`target_selector`/`brain` are populated per `mob_config`'s own
    /// per-kind table (M4-B03, unmodified); `attributes` via `mob_config::default_attribute_map`
    /// (extended, Part B).
    pub fn spawn_mob(&mut self, kind: EntityKind, pos: [f64; 3]) -> RcEntityId;
    pub fn spawn_player_proxy(&mut self, pos: [f64; 3]) -> RcEntityId;
    /// One full simulated tick, driving every live mob through the exact order M4-B03's
    /// own `systems.rs` doc comment already specifies, with this blueprint's own explicit,
    /// cited sub-ordering for step 2's own two selectors: (1) sensing, (2a) `target_selector.tick`
    /// then (2b) `goal_selector.tick` — `target_selector` first, so a same-tick goal like
    /// `ZombieAttackGoal` can read `ctx.current_target` already reflecting this tick's own
    /// target choice, never last tick's stale value (this blueprint's own reasonable,
    /// stated resolution of an ordering M4-B03's own text leaves unpinned), (3) navigation,
    /// (4) `brain` (any mob whose own AI hook populates one — this suite's own Villager is
    /// the only kind that does, never a Villager-exclusive step in general), (5)
    /// movement-intent application — then this blueprint's own bounded,
    /// explicitly-approximate position-integration step (below) — never real `rc_physics`
    /// integration, which is a different blueprint's scope entirely (out of scope,
    /// Constraints) and not needed for this suite's own envelope-level assertions.
    pub fn tick(&mut self);
    /// Test/debug-only, mirrors every `debug_query_*` precedent in this project: current
    /// position of a live mob, `None` if despawned/unknown.
    pub fn mob_pos(&self, id: RcEntityId) -> Option<[f64; 3]>;
}

impl rc_mechanics::world_access::BlockWorldAccess for ScenarioWorld { /* HashMap-backed, Context above */ }
```

**Movement realization — a bounded, explicitly-flagged scenario-local approximation, not a physics claim.** `ScenarioWorld::tick`'s own final step, after `PendingMovementIntent` is produced (M4-B03's own Stage 6a output, unchanged): `if intent.0.forward > 0.0 { pos.x += cos(yaw_radians) * STEP_BLOCKS_PER_TICK; pos.z += sin(yaw_radians) * STEP_BLOCKS_PER_TICK; }` with `STEP_BLOCKS_PER_TICK = 0.2` — this blueprint's own fixed, hand-picked, moderate-confidence constant, deliberately **not** derived from `MOVEMENT_SPEED`/vanilla's own real per-tick movement formula (that derivation is `rc_physics`'s own scope, a different blueprint's job — Constraints). This scenario harness needs only "a mob with an active `MoveControl::MoveTo` visibly, monotonically approaches its target over many ticks," never an exact vanilla speed — restated as an honest, bounded simplification exactly like M4-B03's own `MoveControl`/`Path::advance_if_reached` epsilon constants are already flagged.

**`AiScenarioSpec`, RON-authored, per TEST-D42** — the block-layout half of each scenario (mob/player placement and per-scenario scripting stay in Rust, in the test file itself, since TEST-D42 licenses RON specifically for "position + block-state... per entry" structures, not for behavioral scripts):

```rust
// crates/testing/gametest/src/ai_scenario/spec.rs — new

#[derive(serde::Deserialize)]
pub struct AiScenarioSpec {
    pub id: String,
    pub blocks: Vec<BlockPlacement>,   // TEST-D42's own literal shape: position + block-state
}

#[derive(serde::Deserialize)]
pub struct BlockPlacement { pub pos: [i32; 3], pub state_id: u32 }

/// Parses one `.ron` file under `corpus/ai_combat/`. Mirrors `rc_gametest::spec::load_spec`'s
/// own validation discipline (M3-B07) at this blueprint's own smaller scope (no
/// `max_ticks`/`ScriptedAction` fields — this corpus's own scripts are Rust, not RON,
/// Context above).
pub fn load_ai_scenario(path: &std::path::Path) -> Result<AiScenarioSpec, SpecError>;

#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    #[error("failed to read {path}: {source}")]
    Io { path: String, #[source] source: std::io::Error },
    #[error("failed to parse {path} as RON: {source}")]
    Parse { path: String, #[source] source: ron::error::SpanError },
}
```

**Assertion evaluators — pure, reusable, independently self-tested** (`crates/testing/gametest/src/ai_scenario/assertions.rs`, new):

```rust
/// One tick's recorded observation — every scenario test appends one of these per
/// `ScenarioWorld::tick()` call, building a trace this module's own pure evaluators check
/// after the fact (never inline in the test body) — exactly the separation
/// `rc_test_harness::position_delta` (Part E) applies to criterion 1, applied here to
/// criterion 3, and for the identical reason: a pure, reusable evaluator is what a
/// "wall-stuck-mob fake" self-test can be aimed at directly.
#[derive(Clone, Copy, Debug)]
pub struct MobTick { pub tick: u64, pub pos: [f64; 3], pub current_target: Option<rc_core::RcEntityId> }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressReport { pub reached_within_budget: bool, pub ticks_to_reach: Option<u64>, pub net_displacement: f64 }

/// `true` iff `trace`'s own final position is within `arrival_radius` blocks (horizontal)
/// of `target_pos` at or before `trace`'s own last entry's `tick <= max_ticks`, **and**
/// `trace`'s own position strictly, monotonically decreased its distance to `target_pos`
/// at least once every `stall_window_ticks` ticks somewhere in the trace (the concrete,
/// checkable form of "did not get stuck" — a mob genuinely wedged against an obstacle
/// produces a `net_displacement` near zero across some window, which this check catches
/// independently of whether it also happens to end up "close enough" by luck).
pub fn analyze_approach(trace: &[MobTick], target_pos: [f64; 3], arrival_radius: f64, max_ticks: u64, stall_window_ticks: u64) -> ProgressReport;
```

### Part E — Criterion 1: integration + the reusable position-delta formula

Integration (Part A, mechanism 1): `xtask m4-report`'s `AC1` case runs `cargo nextest run -p rusty-clanker-server play_region_transfer_player_walk::player_walks_across_a_live_region_boundary_with_bounded_position_delta --no-tests fail` and records its exit code — the exact, already-complete, already-passing M4-B08 test, never re-derived.

**The reusable formula, restated as its own pure, independently-self-tested module** (`crates/testing/test-harness/src/position_delta.rs`, new — the direct sibling of M3-B08's own `tick_cadence.rs` extraction, same justification: a formula worth restating once as a checkable artifact, not left only as inline assertions inside one test body):

```rust
#[derive(Debug, Clone, Copy)]
pub struct PositionSample { pub tick: u64, pub region: Option<rc_messaging::RegionId>, pub pos: Option<[f64; 3]> }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionDeltaReport {
    pub none_count: u32,
    pub max_step_deviation: f64,       // largest |observed_delta - expected_delta| across all consecutive Some/Some pairs
    pub discontinuity_detected: bool,  // Context's exact rule, below
}

/// M4-B08's own required exact definition (Context, Part 1.5), restated as one pure
/// function: **pass** iff (a) `none_count <= 1` (at most one gap tick — the one-tick
/// transfer budget, ARCH-D10) and (b) every consecutive pair of `Some` entries differs by
/// exactly `expected_step` (component-wise, within `tolerance`) **when they are adjacent
/// ticks**, and — for the one allowed gap, if present — the position immediately before
/// the gap and immediately after differ by exactly `2 * expected_step` component-wise
/// (within `tolerance`): the gap tick's own "missing" delta is exactly one tick's worth,
/// never two or more. `discontinuity_detected = !(a && b)`.
pub fn analyze_position_delta(samples: &[PositionSample], expected_step: [f64; 3], tolerance: f64) -> PositionDeltaReport;
```

### Part F — Criterion 2: integration

`xtask m4-report`'s `AC2` case runs `cargo nextest run -p rc-mechanics hopper_cross_chunk_border::hand_derived_three_hopper_chain_tick_table --no-tests fail` and records its exit code — the exact, already-complete, already-passing M4-B08 test (its own hand-derived ten-tick table, Context of that blueprint), never re-derived, never re-implemented. No new pure-analysis module is introduced for this criterion: M4-B08's own test is already an exact, byte-for-byte tick-table replay (not a tolerance-banded rate), so there is no separate "formula" left to extract — the harness self-test for this leg (below) operates directly on `build_report`'s own aggregation layer instead (mirroring M3-B08's own `perturbed_redstone_replay_is_caught_by_the_parity_leg` precedent exactly, for the identical reason: the leg being integrated is itself already an exact comparison, so the self-test proves the *aggregation*, not a re-derived formula, is honest).

### Part G — Criterion 3: the eleven-scenario suite

Every scenario runs against a fresh `ScenarioWorld`, ticks synchronously (no wall-clock sleep — `tick()` advances simulated state only), and is evaluated by `assertions.rs`'s own pure functions against a recorded trace. Tolerance/envelope framing, restated once per the milestone's own text: our own engine's internal logic (`find_path`, `GoalSelector::tick`, `apply_damage_pipeline`) is fully deterministic and reproducible — most of these scenarios' own pass/fail lines are therefore *exact* against our own engine, not "banded" against it; the qualitative/behavioral framing applies to the comparison against **vanilla's own real behavior**, which no scenario here claims bit-exact agreement with (no oracle exists to compare against, Context Part A) — each scenario's own "envelope" is stated as the bound this project's own design intends to hold, not a measured vanilla value.

| # | Scenario id | Mob(s) | Setup | Assertion | Tolerance / envelope |
|---|---|---|---|---|---|
| 1 | `zombie_routes_around_wall_gap` | Zombie | RON: a straight 1-block-thick, 3-block-tall wall between spawn and a player-proxy 12 blocks away, with exactly one 1-block gap | `find_path`'s own returned path (read via `PathNavigation.current_path.nodes()` once a path exists) passes through the gap's own `(x,z)` column at some node; **never** through any wall block's own position | Exact (deterministic pathfinding) |
| 2 | `zombie_refuses_a_four_block_drop` | Zombie | RON: a 4-block-deep pit directly on the shortest straight line to a waypoint 12 blocks away, with a walk-around route available; the zombie carries no live AI target for this scenario (`current_target == None` throughout — `find_path` is invoked directly against the waypoint, not through target-selector combat targeting), so the mob's own max-fall-distance stays at its no-target value | No two consecutive nodes in the returned path differ by more than 3 in `y` — the descent scan's own stop condition is the mob's own max-fall-distance, which is exactly 3 only in the no-target case this scenario constructs (a mob with a live AI target is allowed a far larger, health- and difficulty-dependent drop) — the honest, reframed form of "refuses an over-deep drop," matching what this engine's pathfinding actually implements rather than an unbuilt "avoid-fall" flag (Context, this blueprint's own scope correction) | Exact |
| 3 | `zombie_ignores_target_outside_follow_range` | Zombie | Player proxy at `40.0` blocks (`FOLLOW_RANGE = 35.0`, M4-B03 §I) | `current_target == None` for the whole run | Exact |
| 4 | `zombie_loses_target_behind_opaque_wall` | Zombie | Player proxy `10.0` blocks away, within range, behind a full solid wall from the zombie's own eye line | `current_target == None` for the whole run (`has_line_of_sight` gates `NearestAttackableTargetGoal`, M4-B03 §H) | Exact |
| 5 | `zombie_engages_melee_within_range` | Zombie | Player proxy placed directly adjacent (`0.5` blocks) | `pending_melee_attack.0 == Some(player_id)` within `5` ticks | Exact, bounded-latency (the half-tick throttle, M4-B03 §D, can delay by at most 1 extra tick — 5 ticks is generous headroom) |
| 6 | `cow_never_acquires_a_target` | Cow | Player proxy adjacent, `200` ticks run | `current_target == None` for every tick — Cow's own `target_selector` is constructed empty (M4-B03 §J) | Exact — the passive-mob envelope contrast against scenario 5 |
| 7 | `zombie_aggros_the_entity_that_hurt_it` | Zombie | Player proxy far outside `FOLLOW_RANGE` (`100.0` blocks — normally never targeted, scenario 3); `recent_damage.0 = Some(player_id)` injected once, simulating a hit landing | `current_target == Some(player_id)` within 2 ticks after the pulse (`HurtByTargetGoal`, Context Part C) — "mob aggro... on damage," including the vanilla-consistent "aggro overrides ordinary range-based targeting" bound; acquisition is not guaranteed on the immediately-next tick, since `canUse`/`start` are evaluated only on a full selector tick and this scenario's own half-tick throttle (scenarios 5/9) already bounds a full tick to at most every other one | Exact, within a 2-tick latency |
| 8 | `villager_flees_then_deaggroes` | Villager | Attacker proxy adjacent; `recent_damage.0 = Some(attacker_id)` injected once | (a) within `2` ticks, `brain.active_activities == {Core, Panic}` and `movement_intent.0.forward > 0.0` with a yaw pointing away from the attacker; (b) at tick `HURT_BY_MEMORY_TTL_TICKS + 5` (past the memory's own expiry, Context Part C), `active_activities == {Core, Idle}` | Exact state transitions; the yaw-away check tolerates `±10°` of the exact opposite bearing (an explicit, stated tolerance band — the one genuinely angle-continuous quantity in this suite) |
| 9 | `goal_selector_evicts_lower_priority_under_real_ticking` | Zombie | No target initially (stroll goal, priority 7, becomes `running`); after `50` ticks, a player proxy is introduced adjacent | Before tick 50: the running goal holding `FLAG_MOVE` is the stroll goal (checked via a test-only "which goal instance currently holds this flag" introspection hook, `GoalSelector::running_goal_holding(flag) -> Option<usize>` — test/debug-only, mirrors every `debug_*` precedent). After tick 50 (+ the half-tick-throttle's own bounded latency, 2 ticks headroom): the flag-holder is `ZombieAttackGoal`'s own priority slot, and the stroll goal's own `stop()` was observed to fire exactly once (a call-counting fixture wrapper around the stroll goal, this test's own instrumentation) | Exact — this is the scenario proving M4-B03's own already-unit-tested pure `GoalSelector::tick` algorithm (`ai_goal_selector.rs`) still holds once driven by this harness's real, multi-tick, multi-goal loop, not merely in isolation |
| 10 | `cooldown_timed_hits_on_an_armored_target` | Zombie (real loopback connection, B05's own established pattern — `HardcodedWorld`, not `ScenarioWorld`) | `world.debug_spawn_mob(Zombie, ..)`; `world.debug_override_attribute(zombie_net_id, rc_registries::generated_v776::registries::attribute::ARMOR, 10.0)` (the retyped signature, Context Part B) (armor toughness left at `0.0`, so `real_armor = clamp(10.0 - dmg/2.0, 2.0, 20.0)`); three `Interact{Attack}` sends at simulated ticks `T0` (full charge, `ticker >= 5`), `T0+3` (undercharged, still within the `10`-tick top-up window that gates on `invulnerable_time > 10`, itself decrementing from the `20`-tick bookkeeping window hit 1 sets), `T0+8` (full charge again, `invulnerable_time` has decremented only to `12`, so still inside that same `10`-tick top-up window) | Hand-derived per M4-B05's own exact formulas, using the player's own base `AttackDamage` of `1.0` (Context, restated once here): hit 1 — `charge=1.0`, `raw=1.0*1.0=1.0` (unenchanted, bare-handed `AttackDamage=1.0`, the player's own base value — not the registry-wide `2.0` default), `real_armor=clamp(10.0-1.0/2.0,2.0,20.0)=9.5`, `armor_fraction=9.5/25.0=0.38`, `dealt=1.0*0.62=0.62`, `last_hurt=1.0`, `invulnerable_time=20`; hit 2 (`T0+3`, `invulnerable_time=17>10`, the top-up branch): `charge=clamp(3.5/5,0,1)=0.7`, `raw=(0.2+0.7²*0.8)*1.0=0.592`; since the comparison is raw-incoming against raw-previous (`0.592 <= last_hurt 1.0`): `NoOp`, health unchanged; hit 3 (`T0+8`, `invulnerable_time=12>10`, the top-up branch is still active): `charge=1.0`, `raw=1.0*1.0=1.0`; since `1.0 <= last_hurt 1.0`: `NoOp` again, health unchanged. Health sequence asserted to `1e-4`: `20.0 -> 19.38 -> 19.38 (unchanged) -> 19.38 (unchanged)` | Exact (all inputs deterministic, formulas fully pinned by M4-B05) |
| 11 | `charged_critical_exceeds_uncharged_by_the_documented_envelope` | Zombie (real loopback connection) | Hit A: thrown at `ticker=0` (`charge=0.2² ... ` — restated exactly: `charge_scale=clamp(0.5/5,0,1)=0.1`, `base_damage_scale_factor=0.2+0.01*0.8=0.208`); Hit B: thrown after waiting `5` ticks idle then jumping (airborne and critical-eligible per M4-B05's own `can_critical_attack`: `fall_distance>0`, `!on_ground`, `!on_climbable`, `!in_water`, `!mobility_restricted`, `!passenger`, target is a `LivingEntity`, `!sprinting`), full charge (`charge_scale=1.0`, satisfying the caller's own full-strength gate `charge_scale>0.9`, `factor=1.0`, `×1.5` critical) | `damage(B) / damage(A) >= 7.0` — hand-derived floor: `(1.0 * 1.5) / 0.208 ≈ 7.212`, this blueprint's own explicit envelope threshold set at `7.0` for headroom against the two hits' own independent armor-formula rounding | **Genuine tolerance band** (`>=`, not `==`) — the one scenario in this suite stated as a ratio bound rather than an exact value, matching the task's own explicit "explicit tolerance bands" requirement most directly |

**Harness self-test — the "wall-stuck-mob fake"** (`crates/testing/gametest/tests/ai_scenario_harness_self_tests.rs`): a synthetic `Vec<MobTick>` where every entry's `pos` is bit-identical (a mob that never moves, despite a nonzero `current_target` present throughout — the literal "stuck" fake, no real `ScenarioWorld` involved) fed to `analyze_approach`; asserts `reached_within_budget == false` **and** the stall condition is what triggers it (distinguished from a synthetic trace that *does* eventually arrive but slowly, which must pass) — proving `analyze_approach` itself, not merely scenario 1's own already-passing run, is the thing actually capable of catching a regression.

### Part H — The M4 completion report

```json
{
  "tier": "m4-acceptance",
  "status": "pass",
  "cases": [
    { "name": "AC1_region_boundary_position_delta", "status": "pass" },
    { "name": "AC2_hopper_cross_chunk_cadence", "status": "pass" },
    { "name": "AC3_scenario_01_zombie_routes_around_wall_gap", "status": "pass" },
    { "...": "one case per scenario, 11 total" }
  ],
  "scenario_count": 11,
  "runtime_ms": 0
}
```
`M4ReportResult` (below) wraps `xtask::tier_result::TierResult` exactly as `M1ReportResult`/`M2ReportResult`/`M3ReportResult` already do — `status: Fail` the instant any one case is `Fail` (fail-on-any, `tier_result::TierResult::finalize`'s own already-established rule, unmodified).

### Part I — Composition-root registration order for `DomainGroup::EntityPhysicsIntegration`'s three real registrants

M4-B02, M4-B04, and M4-B05 were each derived in parallel against only M4-B01 as their own named prerequisite, and each independently registers one real system into `DomainGroup::EntityPhysicsIntegration` (Stage 6b) — `register_stage6b` (M4-B02's `system_entity_physics_integration`), `register_mob_despawn` (M4-B04's `system_mob_despawn`), and `register_mob_combat_system` (M4-B05's mob-melee/death system) — all three wired, by each blueprint's own Deliverables, into the identical `HardcodedWorld` composition root (`crates/server/src/play/world.rs`'s own executor-build step). None of the three names either of the other two, so none states a required call order, and none proves the three co-register without `RcExecutorBuilder::build()` returning `Err(ExecutorBuildError::AmbiguousMutationAuthority { .. })` (M0-B05's own binding check: a system's declared `structural_writes` must never overlap that same system's own mutable `Query` access). **M4-B08's own two `DomainGroup::EntityPhysicsIntegration` registrants (`register_mob_crossing_detection` and its own player crossing-detection system) are not part of this ordering problem** — both are scoped to `TwoRegionWorld`'s own separate, isolated `RcExecutorBuilder` (Part D's sibling scenario-harness precedent: an additive, parallel composition, never `HardcodedWorld`'s own executor), which carries its own independent `[CompiledGroup; 8]` array and its own independent `order_tag` sequence; M4-B08's own text already states that harness's own required two-system call order completely and correctly, so this Part does not touch it.

**Binding composition-root call order, `HardcodedWorld`'s own executor build** (`crates/server/src/play/world.rs`): `register_stage6b` (M4-B02) first, `order_tag = 0`; `register_mob_despawn` (M4-B04) second, `order_tag = 1`; `register_mob_combat_system` (M4-B05) third, `order_tag = 2`. This order is fixed for determinism and documentation, not because a correctness-critical dependency forces it: at M4's own milestone boundary, no production Stage-6a system ever runs inside `HardcodedWorld` (M4-B03's own AI wiring is explicitly deferred to a future blueprint, Constraint (e) above), so `PendingMeleeAttack` never holds `Some` outside a debug-injected test — `register_mob_combat_system`'s own per-tick body is therefore an observationally-inert no-op walk over an all-`None` component in every current production tick, making the relative order of the three registrants inert for M4's own observable behavior. The order is still pinned explicitly here so a future blueprint that wires real Stage-6a AI into `HardcodedWorld` (closing that same deferred boundary) inherits one documented, tested call order instead of needing to re-derive it, exactly mirroring this project's own "reserve the seam, do not leave it ambiguous" convention.

**Conflict-freedom.** All three systems declare `structural_writes` naming components each system's own `Query` never holds a live mutable borrow against simultaneously (M4-B02's `system_entity_physics_integration` mutates `BaseEntity`/`LivingEntity` via its own `Query`, with `structural_writes` naming only despawn-adjacent bookkeeping components no other system in this group touches; M4-B04's `system_mob_despawn` and M4-B05's mob-combat system follow the identical shape M0-B05's own acceptance tests already prove safe per-system) — this satisfies `AmbiguousMutationAuthority`'s own binding rule (a strictly per-system self-consistency check, M0-B05 Context, restated: it fires only when one system's own `structural_writes` overlaps that same system's own declared mutable access, never when two *different* systems both touch the same component, which `RcExecutorBuilder::build()`'s own conflict-graph step instead serializes via the ordinary Kahn's-algorithm topological sort, ties broken by `order_tag`, ARCH-D8). The three systems are therefore guaranteed, by construction, never to trigger `AmbiguousMutationAuthority`, and this blueprint's own new acceptance test (below) proves it against the real, merged types rather than leaving the claim unverified.

**Correcting the three prerequisite blueprints' own stale doc-comment claims.** M4-B02's Context §A and Deliverables, M4-B04's Context and Deliverables, and M4-B08's Deliverables each originally described their own registrant as the group's sole or first member — each has already been corrected in place (M4-B02/M4-B04/M4-B08's own current text) to name this Part as the governance changeset fixing the real, multi-registrant call order; this Part is the one place that order is authoritatively fixed.

### Part J — Open items, restated (folded from a standalone section to keep this blueprint's structure spec-compliant)

`00-blueprint-spec.md`'s "Mandatory blueprint structure" fixes exactly eight top-level sections in order; the three items below are restated here, inside Context, rather than as a ninth top-level section, so this blueprint's own structure matches every other M4 blueprint's:

- `MELEE_ATTACK_RANGE = 1.5`, `HURT_BY_MEMORY_TTL_TICKS = 100`, and `STEP_BLOCKS_PER_TICK = 0.2` are this blueprint's own seed choices (Part C.4/D). None is pinned by any merged planning document or prerequisite blueprint. A future blueprint that builds real per-mob hitbox-derived reach or real `rc_physics`-integrated mob movement should reconcile these against that real value; until then, this suite's own scenarios 5/8/1–2/9 are correct relative to these stated constants, not yet vanilla-calibrated.
- The `AttributeMap` reconciliation (Part B) is written against M4-B03's and M4-B05's own *blueprint text*, not their merged code — if either blueprint's actual implementation deviated from its own Deliverables in a way this reconciliation does not anticipate (e.g., a different module path chosen at implementation time), this blueprint's own governance changeset must adapt the mechanical substitution accordingly; the mapping table and the four new registry rows (the *values*) are the binding, stable part regardless of exact file layout.
- Real production wiring of Stage-6a AI systems into a live `RcExecutor`/`HardcodedWorld` (as opposed to this blueprint's own lightweight scenario replay) remains unimplemented after this blueprint — flagged here as the same explicitly-open scope boundary M4-B03's own text already states, not newly introduced by this blueprint. (Stage-6b combat wiring is *not* in this same unimplemented category — Part I above states plainly that M4-B05's own combat registration is real, already-landed `HardcodedWorld` content this blueprint only reorders.)

### Claims to verify (TEST-D57)

- Registry attribute `ATTACK_SPEED` defaults to `4.0` within range `[0.0, 1024.0]`, and Zombie, Villager, and Cow each use `4.0` (Context Part B, attribute reconciliation table).
- Registry attribute `SAFE_FALL_DISTANCE` defaults to `3.0` within range `[-1024.0, 1024.0]`, used by Zombie, Villager, and Cow (Context Part B, attribute reconciliation table).
- Registry attribute `FALL_DAMAGE_MULTIPLIER` defaults to `1.0` within range `[0.0, 100.0]`, used by Zombie, Villager, and Cow (Context Part B, attribute reconciliation table).
- Registry attribute `SWEEPING_DAMAGE_RATIO` defaults to `0.0` within range `[0.0, 1.0]`, used by Zombie, Villager, and Cow (Context Part B, attribute reconciliation table).
- The clientbound Update Attributes packet has protocol id `0x83` (Context Part B, "Retiring the duplicate UpdateAttributes wire packet").
- A Zombie's default `ATTACK_DAMAGE` attribute value is `3.0` (Deliverables, `ai_scenario_layout.rs` acceptance-test spec).
- A Zombie's `FOLLOW_RANGE` attribute is `35.0` blocks; a target at `40.0` blocks is never acquired (Context Part G, scenario 3).
- A Zombie's `NearestAttackableTargetGoal` is gated by `has_line_of_sight`, so a target within range but behind an opaque wall is never acquired (Context Part G, scenario 4).
- A Cow's `target_selector` is constructed with zero goal entries, so a Cow never acquires an attack target (Context Part G, scenario 6).
- A mob's `HurtByTargetGoal` sets its current target to the attacker within 2 ticks of taking damage, overriding ordinary range-based targeting (Context Part G, scenario 7).
- A Cow's flee-on-damage behavior (`PanicGoal`) moves away from the attacker's last-known position at a `2.0` navigation-speed modifier; the Villager has no `FleeFromHostile` goal, and its own brain-driven panic package computes a runaway speed of `1.5x` the villager's own speed modifier instead (Context Part C.4, citing M4-B03).
- A Villager's brain, on taking damage, sets the `HurtBy` memory module type to the damage source itself and the `HurtByEntity` memory module type to the attacking entity only when that source has a living attacker, both written by a sensor with no TTL argument (Context Part C.4).
- A mob's pathfinding never returns a path where two consecutive nodes differ by more than `3` in y, when the mob has no current AI target — the descent scan is bounded by the mob's own max-fall-distance, which is `3` only in the no-target case and far larger, health- and difficulty-dependent, once a target is set (Context Part G, scenario 2, citing M4-B03 section F).
- A player's default, unenchanted `AttackDamage` attribute value is `1.0`; `2.0` is only the attribute's registry-wide default, which the player's own attribute supplier overrides (Context Part G, scenario 10).
- Against a target with `10.0` armor and `0.0` armor toughness, a `2.0`-damage hit computes real_armor=clamp(10.0-2.0/2.0,2.0,20.0)=9.0, armor_fraction=9.0/25.0=0.36, and deals dealt=2.0*0.64=1.28 damage — the clamp applies to real_armor within [total_armor*0.2,20.0], not to the fraction within [0,1] (Context Part G, scenario 10).
- At attack-cooldown ticker `3` of a 5-tick charge window, the charge fraction is charge=clamp(3.5/5,0,1)=0.7 (Context Part G, scenario 10).
- When a target's invulnerable_time exceeds `10` ticks, a second hit within that window computes raw damage as raw=(0.2+charge^2*0.8)*base_damage, which for charge=0.7 and base_damage=2.0 gives raw=(0.2+0.7^2*0.8)*2.0=1.184 (Context Part G, scenario 10).
- If a top-up hit's raw, pre-armor incoming damage does not exceed the previous hit's own raw, pre-armor incoming damage (`lastHurt`), the damage pipeline is a no-op and the target's health is unchanged (Context Part G, scenario 10).
- Across the three hits of scenario 10, a `20.0`-health target's health sequence is 20.0 -> 19.38 -> 19.38 (unchanged) -> 19.38 (unchanged) (Context Part G, scenario 10).
- A target's post-hit invulnerability bookkeeping window is `20` ticks, but the damage top-up branch applies only while that counter exceeds `10`; a hit landing after it has decremented to `10` or below instead deals independent fresh damage and resets the counter (Context Part G, scenario 10 setup).
- A critical hit multiplies damage by `1.5x` (Context Part G, scenario 11, citing M4-B05's can_critical_attack).
- A critical hit is eligible only when fall distance is nonzero, the attacker is not on the ground, not on a climbable block, not in water, not mobility-restricted, not a passenger, the target is a LivingEntity, and the attacker is not sprinting — plus the caller's own full-strength gate, attack-cooldown charge scale > 0.9 (Context Part G, scenario 11, citing M4-B05's can_critical_attack).
- At attack-cooldown ticker `0`, the charge scale is charge_scale=clamp(0.5/5,0,1)=0.1, giving a base-damage scale factor of base_damage_scale_factor=0.2+0.01*0.8=0.208 (Context Part G, scenario 11).
- A fully-charged critical hit's damage-scale factor is `1.0`; after the 1.5x critical multiplier, the ratio of a fully-charged critical hit's damage to an uncharged hit's damage is (1.0*1.5)/0.208 approximately equal to 7.212 (Context Part G, scenario 11).
- A mob's AI goal/target-selector evaluation is throttled to run only every other tick (the half-tick throttle, `should_full_tick`, M4-B03 §D), bounding any AI reaction to a newly eligible target or attack to at most one extra tick of latency beyond the triggering event (Context Part G, scenarios 5 and 9).
- A Zombie's random-stroll goal is registered in its goal selector at priority `7` (Context Part G, scenario 9 setup).
- A mob's per-tick AI evaluation order is: sensing, then target-selector evaluation, then goal-selector evaluation, then navigation, then brain evaluation (not Villager-only — any mob whose own AI hook populates one), then movement-intent application via the move/look/jump controls last (Context Part D, citing M4-B03's `systems.rs` doc comment).
- Each modifier entry in the Update Attributes packet's per-attribute modifier array carries an identifier, an amount, and an operation (Context Part B, "Retiring the duplicate UpdateAttributes wire packet").

## Deliverables

### `crates/testing/test-harness/src/lib.rs` (modify — one new `pub mod` line)

```rust
pub mod position_delta;
```

### `crates/testing/test-harness/src/position_delta.rs` (new)

Exactly the types/function specified in Context Part E.

### `crates/testing/gametest/src/lib.rs` (modify — one new `pub mod` line; every existing line from M3-B07 unchanged)

```rust
pub mod ai_scenario;
```

### `crates/testing/gametest/src/ai_scenario/mod.rs` (new)

```rust
pub mod assertions;
pub mod spec;
pub mod world;

pub use assertions::{analyze_approach, MobTick, ProgressReport};
pub use spec::{load_ai_scenario, AiScenarioSpec, BlockPlacement, SpecError};
pub use world::{ScenarioMob, ScenarioPlayerProxy, ScenarioWorld};

// `MELEE_ATTACK_RANGE`/`HURT_BY_MEMORY_TTL_TICKS` live in `rc_mechanics::ai::mob_config`
// (Context Part C.4, their one production home — `ZombieAttackGoal`/`HurtBySensor` read
// them directly) and are re-exported here, not redefined, so this crate never carries a
// second, driftable copy of either value:
pub use rc_mechanics::ai::mob_config::{MELEE_ATTACK_RANGE, HURT_BY_MEMORY_TTL_TICKS};

/// Scenario-harness-local only (Context Part D's own "movement realization" — never a
/// vanilla speed claim, never read by any `rc-mechanics` production code).
pub const STEP_BLOCKS_PER_TICK: f64 = 0.2;
```

### `crates/testing/gametest/src/ai_scenario/{spec.rs, world.rs, assertions.rs}` (new)

Exactly as specified in Context Part D.

### `crates/testing/gametest/corpus/ai_combat/*.ron` (new — two files needing a nontrivial block layout)

`wall_with_gap.ron` (scenario 1) and `four_block_pit.ron` (scenario 2) — every other scenario needs no blocks (`blocks: []`) and is constructed directly in its own test body without a RON file, per TEST-D42's own "equally valid... not a replacement" framing (a scenario with nothing to place has nothing a RON file would add). Each file:

```ron
AiScenarioSpec(
    id: "wall_with_gap",
    blocks: [
        // one entry per wall block: BlockPlacement(pos: (x,y,z), state_id: <resolved
        // against a real reports/blocks.json at implementation time, per every prior
        // blueprint's own identical reconciliation discipline>)
    ],
)
```

### `crates/testing/gametest/corpus/ai_combat/manifest.json` (new)

Built via `xtask::fixture_manifest::build_manifest(0, "26.2", &<two files as bytes>, "manual/M4-B09", "n/a")` (M0-B07's own already-shipped function, reused unmodified — `source_jar_sha1: "n/a"` exactly mirrors M3-B07's own identical precedent for hand-authored, non-jar-derived RON data).

### `crates/mechanics/src/combat/death.rs` (modify — `PendingMeleeAttack` reshaped, Context Part C.1)

### `crates/mechanics/src/combat/ai_bridge.rs` (new — `RecentDamage`, Context Part C.2)

### `crates/mechanics/src/combat/mod.rs` (modify — add `pub mod ai_bridge; pub use ai_bridge::RecentDamage;`; retire the `attributes` module per Part B)

### `crates/mechanics/src/combat/attributes.rs` (modify — gutted to a re-export, Context Part B)

```rust
//! Retired (M4-B09, Context Part B): `crate::ai::attributes::AttributeMap` (M4-B03) is the
//! one attribute system for every entity. Re-exported here so `rc_mechanics::combat::AttributeMap`
//! keeps resolving for any call site written against that path.
pub use crate::ai::attributes::AttributeMap;
```

### `crates/mechanics/src/combat/{damage.rs, knockback.rs, fall.rs}` (modify — mechanical key-type substitution per Part B's mapping table; every formula body unchanged)

### `crates/server/src/play/attribute_packets.rs` (modify — gutted to a re-export, Context Part B)

```rust
//! Retired (M4-B09, Context Part B): `crate::play::combat_packets::UpdateAttributes`
//! (M4-B05) is the one `Update Attributes` (`0x83`) packet definition in the engine.
//! Re-exported here so `rusty_clanker_server::play::attribute_packets::UpdateAttributes`
//! keeps resolving for any call site written against that path; `play/mod.rs`'s own
//! `pub use attribute_packets::UpdateAttributes;` line is unchanged.
pub use crate::play::combat_packets::UpdateAttributes;
```

### `crates/server/src/play/combat_packets.rs` (modify — one test fixed, Context Part B's own key-mapping table)

`update_attributes_encodes_empty_modifier_arrays` (M4-B05's own already-specified test) is rewritten to construct its `UpdateAttributes` value via `rc_registries::generated_v776::registries::attribute::ARMOR` (the reconciled registry constant, Part B's mapping table) in place of the now-retired `AttributeKind::Armor.registry_ordinal()` — the asserted byte sequence is unchanged, since `ARMOR`'s own numeric id is the same value `AttributeKind::Armor.registry_ordinal()` already resolved to.

### `crates/mechanics/src/ai/attributes.rs` (modify — four new registry rows, Context Part B)

### `crates/mechanics/src/ai/goal.rs` (modify — `AiContext` gains `hurt_by`, `melee_attack_signal`, `current_target`, Context Part C.3/C.4; plus one new test/debug-only method, `GoalSelector::running_goal_holding(&self, flag: u8) -> Option<usize>` — the `entries` index currently holding `flag` in `locked_flags`, `None` if unheld, mirroring every `debug_*` precedent in this project; scenario 9's own introspection, Context Part G)

### `crates/mechanics/src/ai/mob_config.rs` (modify — concrete `ZombieAttackGoal`/`HurtByTargetGoal`/Cow `PanicGoal`/Villager `HurtBySensor` bodies, Context Part C.4)

### `crates/server/src/play/world.rs` (modify — composition-root registration order, Context Part I)

`HardcodedWorld`'s own executor-build step calls the three `DomainGroup::EntityPhysicsIntegration` registration functions in this exact order: `rc_mechanics::entity::physics::ecs::register_stage6b(&mut builder)` (M4-B02); `rc_mechanics::entity::ecs::register_mob_despawn(&mut builder)` (M4-B04); `rc_mechanics::combat::register_mob_combat_system(&mut builder)?` (M4-B05, propagating its own `Result`). No other line of `world.rs`'s own executor-build step changes.

### `xtask/src/m4_report.rs` (new)

```rust
use crate::tier_result::TierResult;

#[derive(serde::Serialize)]
pub struct M4ReportResult {
    #[serde(flatten)]
    pub automated: TierResult,   // tier = "m4-acceptance"
    pub scenario_count: usize,
    pub runtime_ms: u64,
}

pub const OUT_PATH: &str = "target/verify/m4-acceptance.json";

/// Runs `cargo nextest run -p <crate> <test_name_substring> --no-tests fail` as a child
/// process (Context, Part A) and returns `true` iff it exits `0`.
pub fn run_nextest_filtered(krate: &str, test_name_substring: &str) -> std::io::Result<bool>;

/// Pure aggregation (Acceptance tests exercise this directly against synthetic inputs —
/// the harness self-tests, below). Builds one `CaseResult` for `AC1`/`AC2` from their own
/// `bool`, and one per scenario from `rc_gametest::ai_scenario`'s own suite-runner result
/// (Context Part G), and `finalize`s the wrapped `TierResult`.
pub fn build_report(ac1_passed: bool, ac2_passed: bool, scenario_results: &[(String, bool)]) -> M4ReportResult;

/// CLI entry point (`xtask m4-report`, no flags — Context Part A): calls
/// `run_nextest_filtered` twice (AC1/AC2), runs every scenario in
/// `rc_gametest::ai_scenario`'s own suite in-process, calls `build_report`, writes `OUT_PATH`.
pub fn run() -> std::process::ExitCode;
```

### `xtask/src/main.rs` (modify — one new `Command` variant, `M4Report` — no fields)

### `xtask/src/path_guard.rs` (modify — one new row)

```rust
ProtectedPath { pattern: "crates/testing/gametest/corpus/ai_combat/**", reason: "AI/combat scenario RON structures + manifest (M4-B09, TEST-D42/D47)" },
```

### `.github/workflows/ci.yml` (modify — one new job, `m4-acceptance`, PR-blocking, no `if:` gate)

```yaml
jobs:
  # ... existing gates/guardrails/soak/m1-acceptance/m2-acceptance/m3-acceptance jobs unchanged ...
  m4-acceptance:
    name: m4-acceptance (${{ matrix.os }})
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
      - name: m4-report
        run: cargo run -p xtask -- m4-report
      - name: Upload m4-acceptance report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: m4-acceptance-${{ matrix.os }}
          path: target/verify/m4-acceptance.json
          if-no-files-found: warn
```
No `workflow_dispatch` input is added (Context, Part A: nothing here has a smoke/full distinction to select).

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary**, restated per TEST-D45/D46 exactly as every prior blueprint states it: every file below, plus every new `src/*.rs` file from Deliverables with function bodies `todo!()`-stubbed, plus the cited additive edits to `death.rs`/`goal.rs`/`attributes.rs` (both), `damage.rs`/`melee.rs`/`knockback.rs`/`fall.rs`, `combat/mod.rs`, `main.rs`, `path_guard.rs`, `ci.yml` present but stubbed/unmodified-beyond-declaration where applicable, is the test-authoring changeset. **No file under `crates/mechanics/tests/`, `crates/server/tests/`, or `crates/testing/gametest/tests/` already merged by M4-B01/M4-B03/M4-B05/M4-B08 is touched at all** — this blueprint's own Part B/C reconciliation is proven not to have broken any of them by the Done-definition's own first two checkboxes (a full, unmodified `cargo nextest run` across every affected crate), not by editing them.

### `crates/testing/test-harness/tests/position_delta_self_tests.rs`

1. `clean_walk_within_tolerance_passes` — 8 samples, `expected_step=[0.5,0,0]`, positions stepping exactly `0.5` per tick, no `None` — `discontinuity_detected == false`.
2. `one_gap_tick_with_correct_double_step_passes` — 8 samples with entry index 4 `None`, entry 3 and entry 5 differing by exactly `1.0` (two steps' worth) in `x` — `discontinuity_detected == false`.
3. `teleport_glitch_is_caught` — the harness self-test required by the task: 8 samples, no `None` entries, but entry index 4's `x` jumps by `5.0` instead of `0.5` (a fake, instantaneous teleport, not a resolvable region-boundary event) — `discontinuity_detected == true`, `max_step_deviation >= 4.5`.
4. `two_gap_ticks_is_a_discontinuity` — two consecutive `None` entries — `discontinuity_detected == true` (`none_count == 2 > 1`), regardless of the surrounding deltas.
5. `gap_with_wrong_total_delta_is_caught` — one `None` entry, but the surrounding `Some` pair differs by `3.0×` the expected step (not `2×`) — `discontinuity_detected == true`.

### `crates/testing/gametest/tests/ai_scenario_layout.rs` (pure, no scenario execution)

1. `load_ai_scenario_parses_the_two_shipped_ron_files` — both `corpus/ai_combat/*.ron` files load without error, `blocks.len() > 0` for `wall_with_gap`/`four_block_pit`, manifest verifies clean (`xtask::fixture_manifest::verify_manifest`, reused).
2. `scenario_world_spawns_a_zombie_with_a_full_component_set` — `spawn_mob(EntityKind::Zombie, ..)` then assert the returned id resolves in `world.mobs` with a non-empty `target_selector` (has at least the `HurtByTargetGoal`/`NearestAttackableTargetGoal` entries) and a populated `attributes` map (`attributes.get(ATTACK_DAMAGE).value() == 3.0`, the Zombie default).
3. `scenario_world_spawns_a_cow_with_an_empty_target_selector` — `target_selector`'s own entry count is `0` for a spawned Cow (Context Part J's own table).

### `crates/testing/gametest/tests/ai_combat_scenarios.rs` (scenarios 1–9 of Context Part G's table)

Nine test functions, named exactly per the `id` column of Context Part G's table (rows 1–9), each asserting exactly the "Assertion" column against a fresh `ScenarioWorld`.

### `crates/server/tests/ai_combat_melee_scenarios.rs` (scenarios 10–11 of Context Part G's table — new file, `play_combat_melee_flow.rs` itself untouched)

Two test functions (`cooldown_timed_hits_on_an_armored_target`, `charged_critical_exceeds_uncharged_by_the_documented_envelope`), each against a fresh `HardcodedWorld` real-loopback session, mirroring M4-B05's own `play_combat_melee_flow.rs` connection-setup pattern (`ClientBuilder`/offline account/join-flow, M1-B05 precedent) exactly, asserting exactly the "Assertion" column of Context Part G's table rows 10–11.

### `crates/testing/gametest/tests/ai_scenario_harness_self_tests.rs`

1. `wall_stuck_mob_fake_fails_the_approach_analysis` — the synthetic never-moved trace, Context Part G's own "harness self-test" paragraph — `analyze_approach(..).reached_within_budget == false`.
2. `genuinely_slow_but_arriving_mob_still_passes` — a synthetic trace whose position changes by a small nonzero amount every tick and does arrive by the budget's own last tick — `reached_within_budget == true` (proves case 1 is catching *stuckness*, not merely *slowness*).

### `crates/mechanics/tests/entity_physics_integration_group_registration.rs` (pure — real `RcExecutorBuilder`, no `HardcodedWorld`/network)

1. `three_real_registrants_co_register_without_ambiguous_mutation_authority` — a fresh `RcExecutorBuilder`; call `register_stage6b`, then `register_mob_despawn`, then `register_mob_combat_system` (propagating its own `Result`, `assert!(result.is_ok())` immediately) — `builder.build()` returns `Ok(_)`, never `Err(ExecutorBuildError::AmbiguousMutationAuthority { .. })`.
2. `three_real_registrants_receive_the_documented_order_tags` — same setup; the built `RcExecutor`'s own `DomainGroup::EntityPhysicsIntegration` group has exactly three compiled systems, in `order_tag` `0, 1, 2` matching `system_entity_physics_integration`, `system_mob_despawn`, `register_mob_combat_system`'s own system respectively (introspected via whichever test-only accessor `RcExecutor`/`CompiledGroup` already exposes for M0-B05's own `pipeline_ordering.rs` tests, reused unmodified here).

### `xtask/tests/m4_report_cli.rs`

1. `build_report_all_passing_yields_pass` — `ac1=true, ac2=true`, 11 scenario results all `true` — `automated.status == Status::Pass`, `cases.len() == 13`.
2. `ac1_failure_is_attributed_to_the_correct_case` — `ac1=false`, everything else `true` — `automated.status == Status::Fail`, `AC1_region_boundary_position_delta` is the sole `Fail` case.
3. `ac2_failure_is_attributed_to_the_correct_case` — mirror of case 2 for `ac2`.
4. `one_failing_scenario_fails_the_whole_report_but_is_individually_named` — scenario 7 (`zombie_aggros_the_entity_that_hurt_it`) marked `false`, every other input `true` — `automated.status == Status::Fail`, exactly one case named `AC3_scenario_07_...` is `Fail`.
5. `m4_report_result_serializes_with_the_documented_shape` — matches Context Part H's own JSON shape (top-level `tier`/`status`/`cases` flattened, plus `scenario_count`/`runtime_ms` as siblings).
6. `path_guard_already_covers_m4_b09s_own_new_paths` — `path_guard::check_paths(ChangesetType::Implementation, &["crates/testing/gametest/src/ai_scenario/world.rs".into(), "crates/mechanics/src/combat/ai_bridge.rs".into(), "xtask/src/m4_report.rs".into()])` → the first is already covered by the corpus-manifest row this blueprint itself adds (a `ChangesetType::Implementation` touching `src/`, not `corpus/`, is unaffected by that row and passes through clean, as every prior blueprint's own `src/` edits do) — `assert_eq!(violations.len(), 0)`.
7. `m4_report_completes_within_the_stated_budget` — the one real, end-to-end integration case in this file (not synthetic `build_report` inputs): a timed call to `m4_report::run()` itself (both real `run_nextest_filtered` subprocess spawns plus the full in-process scenario suite) — asserts wall time `<= 20_000` ms (Context Part A's own stated budget) **and** `ExitCode::SUCCESS`. Requires a workspace build to already exist (the same precondition every prior harness blueprint's own real end-to-end case carries) — this is the one case in this file that is not reachable from a bare `cargo nextest run` on a completely unbuilt tree, mirrored by this blueprint's own Verification commands ordering `cargo build` before `cargo nextest run`.

## Implementation steps

1. **`rc-test-harness`.** `position_delta.rs` per Context Part E. Observable: `position_delta_self_tests.rs` passes.
2. **`rc-mechanics`/`rusty-clanker-server` — Part B, the `AttributeMap` reconciliation and the duplicate `UpdateAttributes` packet retirement.** Delete `combat/attributes.rs`'s own type definitions, replace with the one re-export line; add the four registry rows to `ai/attributes.rs`; mechanically substitute every `AttributeKind::X` reference in `damage.rs`/`melee.rs`/`knockback.rs`/`fall.rs`/`combat_packets.rs`'s own test per Part B's mapping table — no formula logic changes; delete `attribute_packets.rs`'s own `UpdateAttributes` struct/`impl`/`build_update_attributes`, replace with the one re-export line. Observable: `cargo build -p rc-mechanics -p rusty-clanker-server` succeeds; every pre-existing M4-B03/M4-B05 test (`ai_attributes.rs`, `combat_damage_pipeline.rs`, `play_combat_packets.rs`, etc.) still passes unmodified.
3. **`rc-mechanics`/`rusty-clanker-server` — Part I, the `EntityPhysicsIntegration` registration order.** Order `crates/server/src/play/world.rs`'s own executor-build step's three `register_*` calls per Context Part I; no `rc-mechanics` source file changes (the three registration functions themselves are unmodified — only the order they are called in is fixed). Observable: `entity_physics_integration_group_registration.rs`'s two cases pass.
4. **`rc-mechanics` — Part C, the AI→combat bridge.** Reshape `PendingMeleeAttack` (`combat/death.rs`); add `RecentDamage` (`combat/ai_bridge.rs`); add the three `AiContext` fields plus `GoalSelector::running_goal_holding` (`ai/goal.rs`); implement `ZombieAttackGoal`/`HurtByTargetGoal`/Cow's `PanicGoal`/Villager's `HurtBySensor` concrete bodies (`ai/mob_config.rs`) per Part C.4. Observable: `cargo build -p rc-mechanics` succeeds; every pre-existing M4-B05 combat test still passes (the `apply_mob_melee_attacks` query-shape change is internal, no test constructed `PendingMeleeAttack` via `Commands` directly per M4-B05's own Deliverables).
5. **`rc-gametest` — `ai_scenario` module.** `world.rs` (`ScenarioWorld`/`ScenarioMob`/`ScenarioPlayerProxy`, the 4-step tick order + bounded movement-realization step), `spec.rs` (`load_ai_scenario`), `assertions.rs` (`analyze_approach`). Observable: `ai_scenario_layout.rs` passes.
6. **The two RON fixtures + manifest.** Author `wall_with_gap.ron`/`four_block_pit.ron`, resolve `state_id` placeholders against a real, locally-run `reports/blocks.json` (the one legal-jar-dependent step, identical in kind to every prior corpus blueprint's own equivalent step), build `manifest.json` via `xtask::fixture_manifest::build_manifest`. Observable: `ai_scenario_layout.rs`'s manifest case passes.
7. **The eleven scenario tests + two harness self-tests.** `ai_combat_scenarios.rs` (scenarios 1–9), `crates/server/tests/ai_combat_melee_scenarios.rs` (scenarios 10–11), `ai_scenario_harness_self_tests.rs`. Observable: all thirteen tests pass.
8. **`xtask/src/m4_report.rs`.** `run_nextest_filtered`, `build_report`, `run`. Observable: `m4_report_cli.rs`'s six cases pass (all against `build_report`/synthetic inputs, no real nextest subprocess needed for these).
9. **`xtask/src/main.rs`, `path_guard.rs`, `.github/workflows/ci.yml`.** The `M4Report` command, the one new `ProtectedPath` row, the `m4-acceptance` job. Observable: `cargo run -p xtask -- m4-report` runs for real (this blueprint's own first end-to-end exercise) — every case passes, `target/verify/m4-acceptance.json` is written with `status: "pass"`.
10. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- path-guard`. Commit with `Changeset-Type: governance`.
11. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs of the new `m4-acceptance` job green on a clean checkout (TEST-D50) — this blueprint's own Done state (Context Part A: no oracle gate defers this further).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, per Acceptance tests' own stated boundary. No file under any crate's already-merged `tests/` directory is touched by this blueprint at all.

(b) **This blueprint's implementation changeset is a governance changeset**, identical framing to every prior harness blueprint's own Constraint (b) — it touches `crates/mechanics/src/{combat,ai}/**` (protected), `crates/testing/{test-harness,gametest}/**` (protected), and `xtask/**`/`.github/workflows/ci.yml`. Every commit carries `Changeset-Type: governance`.

(c) **No new external dependencies.** `ron`/`serde`/`serde_json`/`thiserror` are already present in `rc-gametest`'s `Cargo.toml` since M3-B07; `rc-mechanics` is already a dependency of `rc-gametest` since M3-B07 (`BlockBehaviorRegistry`); no `[dependencies]` table anywhere gains a new line. `xtask`'s `m4_report::run_nextest_filtered` shells out to the already-required `cargo-nextest` binary (TEST-D2) via `std::process::Command` — not a new library dependency.

(d) **No Mojang or third-party reimplementation code.** Every formula this blueprint's own scenario 10/11 hand-derives is copied from M4-B05's own already-committed Context (ASSET-D18/D19/D30) — this blueprint derives no new combat/pathfinding algorithm, only drives and reconciles already-specified ones.

(e) **Scope boundary.** This blueprint does not implement: real Stage-6a `bevy_ecs` production registration/dispatch for AI (M4-B03's own already-stated "wiring into `HardcodedWorld`'s live tick loop" remains deferred to a future blueprint — this blueprint's own `ScenarioWorld` is a lightweight, `bevy_ecs`-free replay harness, mirroring `rc_gametest`'s own established M3-B07 precedent, never a production composition root); real `rc_physics`-integrated mob movement (`ScenarioWorld`'s own `STEP_BLOCKS_PER_TICK` is an explicit, bounded scenario-only approximation, Context Part D); mob spawning/despawning algorithms (MECH-D34/35 — every mob in this blueprint's own tests is placed directly, `debug_spawn_mob`/`spawn_mob`, never naturally spawned); any new gameplay mechanic beyond the two cited, narrow, additive corrections (Parts B/C) needed to make M4-B03's and M4-B05's own already-specified content mutually coherent and end-to-end exercisable. **Not an exception to this boundary:** Part I's own `world.rs` edit orders three `register_*` calls each already specified, unmodified, by M4-B02/M4-B04/M4-B05's own Deliverables — it adds no new system, no new dispatch behavior, and no AI wiring. M4-B05's own Stage-6b combat registration was already real, production `HardcodedWorld` wiring before this blueprint touches it (only Stage-6a AI wiring is the deferred item named above); fixing that registration's call order relative to two siblings is call-order governance, not new production content. Do not add placeholder implementations of any of the deferred items above as a shortcut.

(f) **No `unsafe` code.**

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-test-harness -p rc-gametest -p rc-mechanics -p rusty-clanker-server -p xtask --all-features
cargo nextest run -p rc-test-harness -p rc-gametest -p rc-mechanics -p rusty-clanker-server -p xtask
cargo nextest run -p rc-mechanics -p rusty-clanker-server -p rc-gametest
cargo test --doc -p rc-test-harness -p rc-gametest -p rc-mechanics -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- m4-report
```

Expected: every command exits 0, including `m4-report`'s own first real, end-to-end run — writing `target/verify/m4-acceptance.json` with `status: "pass"` and thirteen `Pass` cases. CI's new `m4-acceptance` job green on both `ubuntu-24.04` and `windows-2025`, on a clean checkout (TEST-D50), is this blueprint's own authoritative Done signal — and, per Context Part A's own structural simplification, also closes `11-roadmap-milestones.md`'s M4 Acceptance Criteria 1–3 themselves the moment it first goes green, with no further oracle-gated run required.
