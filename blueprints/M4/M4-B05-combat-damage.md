# M4-B05 — Combat & Damage

| Field | Content |
|---|---|
| ID | M4-B05 |
| Milestone | M4 — Mechanics Tier 2: Entities, AI, Combat, Items |
| Prerequisites | M4-B01 (`rc-mechanics::entity`: `BaseEntity`/`LivingEntity`/`EntityKind`/`EntityPayload`/`ZombieBundle`/`CowBundle`/`VillagerBundle`/`ItemBundle`, `EntityUuid`/`NetworkEntityIdAllocator`, the metadata wire helpers `encode_metadata_entries`/`decode_metadata_entries`/`MetadataValue`, the `Spawn Entity`/`Set Entity Data`/`Remove Entities`/`Set Entity Velocity`/`Teleport Entity` packets in `crates/server/src/play/entity_packets.rs`, the `EntityAiSelection`(Stage 6, read-only)/`EntityPhysicsIntegration`(Stage 7) `DomainGroup`/`Stage` split, the tracking system `compute_tracking_delta` — all reused unmodified; restated below only where this blueprint's own systems consume them). M3-B01 (`rc-mechanics::random::RcRandom`, bit-exact LCG, reused unmodified for the one rare RNG call this blueprint needs). M3-B02 (`rc-physics`: `Vec3`/`Aabb`, `mth_sin`/`mth_cos`, `PLAYER_EYE_HEIGHT`; `rusty-clanker-server::play::movement`: `PlayerMotion`, `eye_position`, `GameModeState` — reused, with one cited additive field added to `PlayerMotion`, Context). M3-B03 (`rc_physics::raycast::{cast_ray, RayHit}`, `rusty-clanker-server::play::mining`: `HeldItemStub`, reused unmodified as the item this blueprint's enchantment-level stub reads from). M2-B07/M3-B03 (the manual, non-`DomainGroup` "Stage-3-equivalent" tick-loop-step pattern, restated and reused for this blueprint's own packet-apply step). |
| Implements | MECH-D40/D43–D46 (knockback, 1.9+ attack cooldown, damage order of operations, armor formula, status-effect placement — restated with exact constants); MECH-D51 (item-entity despawn/merge — reused unmodified from M4-B01, referenced only for the loot-drop seam); MECH-D62/D63 (reach/interaction-range validation and the `sequence`-adjacent per-action contract, extended to entity targets); MECH-D4 (Stage-3 placement for the `Attack`/`Interact` packets); MECH-D32 (Stage 6a read-only AI selection / Stage 6b action integration, exercised for the first time with real content — the mob-melee-attack seam); MECH-D64/D65 (a minimal, real `GlobalDifficulty` resource, first exercised here); ARCH-D8/D12/D15 (system registration into `EntityPhysicsIntegration`, the first real Stage-6b content); NET-D3 (six new hand-written packet types: `Attack`, `Interact`, `Set Health`, `Update Attributes`, `Damage Event`, `Entity Event`, plus reuse of M4-B01's `Set Entity Velocity`/`Set Entity Data`). MECH-D18's border-halo widening is explicitly **not** implemented (Context, "Explosions — out of scope"). |
| Crates touched | `rc-mechanics` (`crates/mechanics/src/combat/`, new module; `crates/mechanics/src/entity/{living.rs, ids.rs}`, modified) — extends M4-B01's own first-real-content precedent; `rc-physics` (`crates/physics/src/motion.rs`, modified — one cited additive field/branch); `rusty-clanker-server` (`crates/server/src/play/{combat_packets.rs, combat.rs}`, new; `crates/server/src/play/{movement.rs, mining.rs, world.rs, connection.rs, mod.rs}`, modified). |
| Estimated scope | L (exceeds the ~800-line guideline, flagged explicitly per `blueprints/M3/M3-B06-random-ticks-block-entities.md`'s own precedent for a coherent, non-splittable task: the melee damage order of operations, the attack-cooldown/critical/sweep model, the two-impulse knockback model, fall damage, attributes, death/loot, and food/exhaustion are one interlocking damage pipeline — splitting any one piece into its own blueprint would leave it consuming a not-yet-specified upstream stage of the same formula chain). |

## Goal & Done definition

Give the engine a complete, vanilla-parity melee combat and damage pipeline exercising real content in Stage 3 (player-caused) and Stage 6b (simulation-caused) for the first time: the full damage order-of-operations (invulnerability top-up window, armor/toughness, enchantment-protection factor, absorption hearts), the 1.9+ attack-cooldown charge curve with critical hits and sweep attacks, the two-impulse knockback model, fall damage for network-connected players, a minimal real attribute system (`AttributeMap`, the vanilla 3-stage `ADD_VALUE → ADD_MULTIPLIED_BASE → ADD_MULTIPLIED_TOTAL` calculation) backing every attribute the formulas above read, a bounded player-health stand-in (players are not yet migrated onto M4-B01's composition model — Context explains why and how a future migration reconciles this), death detection and a loot-drop seam a future items/loot blueprint fulfills, a bounded seam for mob-melee-attack timing a future AI blueprint fulfills, a minimal real food/exhaustion/natural-regeneration system, a minimal real `GlobalDifficulty` resource, and the six new packets (plus reuse of M4-B01's `Set Entity Velocity`/`Set Entity Data`) this all requires. This blueprint spawns real `Zombie`/`Cow`/`Villager` mobs into `HardcodedWorld`'s live tick loop for the first time (a debug/test-only entry point, Context) — the first real production exerciser of M4-B01's `NetworkEntityIdAllocator`/entity-persistence/tracking machinery beyond its own unit tests.

Done when:

- [ ] `cargo build -p rc-mechanics -p rc-physics -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mechanics -p rc-physics -p rusty-clanker-server` (default features).
- [ ] Every damage-formula golden-table row (weapon × armor × enchant matrices, hand-computed) matches to `f32` bit-for-bit or a stated `1e-6` tolerance where the formula is genuinely float-order-sensitive.
- [ ] Every attack-cooldown charge-curve golden vector matches to `1e-9`.
- [ ] Every knockback velocity golden vector (both impulses, in order) matches to `1e-9`.
- [ ] The invulnerability-window sequence tests (top-up vs. fresh-hit branches, the `> 10` not `> 0` boundary) pass.
- [ ] The death/drop integration test passes: a mob reduced to 0 health despawns, drops its configured loot via the seam trait, and is removed from every viewer's `tracked_entities`.
- [ ] Packet conformance + validation-rejection tests (`Attack`/`Interact` decode/encode round trip, `OutOfReach`/occluded-target rejection, out-of-angle rejection) pass.
- [ ] `cargo run -p xtask -- lint-deps`, `fmt-check`, `lint` all exit 0 — no new dependency edges beyond `rc-mechanics`→`rc-physics` (already present since M4-B01) and ordinary intra-crate additions.
- [ ] `cargo test --doc -p rc-mechanics -p rc-physics -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Scope boundary, stated up front

**In scope**, restated with exact constants below: the full player-damage order of operations (shield block excluded, see below); invulnerability window; armor/toughness formula; enchantment-protection-factor formula (fed by a bounded-zero enchantment-level stub, see below); absorption hearts (both the player and the mob code path, including their documented asymmetry); attack-cooldown charge curve; critical hits; sweep attacks (05 carries no distinct sweep decision ID — included anyway, justification below); knockback (both impulses); fall damage for **players only**; mob melee attacks (`Mob.doHurtTarget`'s own deterministic path); a minimal `GlobalDifficulty` resource and incoming-player-damage scaling; death detection, mob despawn, and a loot-drop seam; a minimal food/exhaustion/regen system; the `Interact`/`Set Health`/`Update Attributes`/`Damage Event`/`Entity Event` packets.

**Out of scope, explicitly** (do not add placeholder implementations of any of these): shield blocking (`BlocksAttacks`, needs an active-use-item/held-item state no `HeldItemStub` models); Resistance status-effect reduction and every other status effect (MECH-D46 — no `StatusEffects` component or potion-application mechanic exists; the pipeline step is skipped, not silently omitted — restated at its exact pipeline position below); Mace smash attacks and piercing/"stab" weapons (§3.11/§3.12 of the research corpus — no Mace/spear item type exists); projectiles/arrows/bows entirely (no ranged-combat item type exists, and M4-B01 itself already scopes projectiles out of M4); XP orbs (below); explosions and MECH-D18's border-halo widening (below); mob fall damage (below); full player respawn/keep-inventory-drop cycle (MECH-D59, Section J — not named anywhere in `11-roadmap-milestones.md`'s M4 scope or acceptance criteria, unlike combat/damage/item-entities, which are; a player's death **state** is fully modeled — Context, "Death" — but the respawn packet round-trip and inventory-drop rules are a future player-lifecycle blueprint's job); real mob spawning (MECH-D34 — a debug-only spawn entry point is provided for this blueprint's own testing, natural spawning is not).

**XP orbs — excluded, checked against 05/11 explicitly.** `11-roadmap-milestones.md`'s M4 scope and acceptance-criteria text never names XP or experience orbs (unlike "combat/damage," "item entities and pickup," which it names explicitly). `05-game-mechanics.md`'s only XP-adjacent decision, MECH-D59, is scoped to *player* death/respawn (Section J, Player Lifecycle — itself out of scope, above), not mob-kill XP reward. M4-B01's own tier-2 entity-kind list has no `ExperienceOrb` kind, and its own Context explicitly documents the pattern for a future blueprint to add a fifth kind if one is needed — this blueprint does not exercise that pattern for XP orbs. No player XP/level component exists anywhere in the merged codebase (`PlayerMarker` carries none). Conclusion: mob-kill XP-orb spawning is deferred to a future blueprint that also builds the player XP/level model this reward would feed.

**Explosions — out of scope, checked against 05/11 explicitly.** `11-roadmap-milestones.md`'s M4 scope/acceptance-criteria text never names TNT, creepers, or explosions (M5 is worldgen; no later-tiered explosion mention exists in M4's own section). `05-game-mechanics.md`'s MECH-D18 states the mechanism (wide-radius `RegionMessage::ExplosionEffect`, one-chunk border-halo widening) but assigns it to no milestone. M3-B01's own `BorderHalo` (Context, "The border halo and `resolve_owner`") is explicitly the lazy, minimal, outermost-row-only halo, with MECH-D18's full one-chunk-slice widening "explicitly deferred to whichever blueprint first implements MECH-D18." Since no explosion-producing mechanic (TNT, creeper charge, bed/respawn-anchor) is in M4's scope, this blueprint is not that blueprint. `BorderHalo` is left exactly as M3-B01 shipped it; MECH-D18's widening remains deferred to whichever future blueprint first implements an explosion.

**Mob fall damage — deferred.** Player fall damage (Context, "Fall damage") consumes `PlayerMotion`'s own gravity/collision integration, which M3-B02 built for network-connected players specifically. No blueprint has yet registered a Stage-6b system that runs `rc_physics::motion::step_living_entity_tick` (gravity/collision integration for AI/simulation-driven entities) against the mobs this blueprint spawns — M4-B01 ships zero AI/physics content by its own stated scope, and general mob physics integration is not this blueprint's stated task. A mob this blueprint's own debug spawn entry point creates therefore never falls and never accrues fall distance; fall damage for mobs is deferred to whichever future blueprint first wires Stage-6b physics integration for `LivingEntity`-bundle entities.

### Tick-pipeline placement (MECH-D2/D4/D32, restated concretely)

Player-caused combat (the `Attack`/`Interact` packets, Stage 3 per MECH-D4's own explicit naming of "Combat" under Stage 3) is applied via a **manual, non-`DomainGroup` tick-loop step**, identical in kind to M2-B07's block-action step, M3-B02/B03's movement/mining steps, and M4-B01's own entity-tracking step — Stage 3 (`NetworkInboundApply`) accepts no `DomainGroup` registration in this project as of M4-B01 (M0-B05's own table), and every prior blueprint that needed Stage-3-equivalent behavior used this same hand-rolled pattern rather than inventing a first real Stage-3 `DomainGroup`. This blueprint's own combat packet-apply step is inserted into `HardcodedWorld`'s tick loop **after** M4-B01's entity-tracking step and **before** `executor.tick_region(...)`, mirroring the exact insertion-point convention every prior manual step already establishes.

Mob melee attacks and fall damage are simulation-driven, not packet-driven, and are real Stage-6b (`DomainGroup::EntityPhysicsIntegration`) content — the first ever registered into that group, per MECH-D32's own text ("[Stage 6a] producing a chosen-action command consumed by Stage 6b's movement/action integration"). This blueprint registers exactly one system into `EntityPhysicsIntegration` (Context, "Mob melee attacks — the AI seam"); it does **not** register anything into `EntityAiSelection` (Stage 6a) — deciding *when* a mob wants to attack is AI/goal-selector content this blueprint does not own (below).

### Attribute system (09-entities-ai.md §3.4/§5, cross-referenced by 19-combat-damage.md throughout) — new, real content

No attribute system exists anywhere in the merged codebase before this blueprint. `AttributeInstance.getValue()`'s exact 3-stage calculation, restated verbatim from the research corpus:
1. `base = base_value`; add every `AddValue` modifier's `amount`.
2. `result = base`; for every `AddMultipliedBase` modifier, add `base * amount` to `result` — multiple such modifiers are **additive against each other** (against the original `base`, not against the running result).
3. For every `AddMultipliedTotal` modifier, multiply `result` by `(1 + amount)` — these **do** compound with each other since they apply sequentially to the running total.
4. Clamp to `[min, max]`.

Vanilla's own modifier application order for step 2/3 is hash-table slot order within each operation bucket, not registration or insertion order: vanilla stores each operation's modifiers in an open-addressing hash map keyed by the modifier's own identifier and iterates that map's value view — only the attribute instance's separate by-id and permanent-modifier lookups are insertion-ordered, and neither participates in the value calculation.

**This project's own bounded exception, stated explicitly, not silently substituted.** This blueprint's own `AttributeInstance::compute_value` (Deliverables) iterates `self.modifiers` — a plain `Vec`, not a hash map — in `Vec` insertion (push) order, for every operation bucket, rather than reproducing vanilla's own hash-slot order. This is a deliberate, bounded, cited deviation, not an oversight, and it holds only under one precise condition: **it holds exactly as long as no computation in M4 ever combines two modifiers of the same `ModifierOperation` (two `AddValue` modifiers, two `AddMultipliedBase` modifiers, or two `AddMultipliedTotal` modifiers) on the same attribute.** Iteration order is observable at all only when two same-operation modifiers on one attribute exist together and could disagree with vanilla's real hash-slot order; with at most one modifier per `(AttributeKind, ModifierOperation)` pair — which is what every one of this blueprint's own production call sites produces, since `default_attributes_for` attaches no modifiers at all (Context table above; every per-kind value is a bare `AttributeInstance::constant`) — `Vec` order and vanilla's hash-slot order can never diverge, so the exception costs nothing observable today. For `AddMultipliedTotal`, `result *= (1 + amount)` is mathematically commutative across modifiers in exact arithmetic; only IEEE-754 double rounding makes iteration order observable, not the running-total structure itself — the same rounding-only sensitivity holds for `AddMultipliedBase`'s plain summation — so even a future same-operation collision this exception has not yet been asked to handle would only ever be a last-bit float divergence from vanilla, never a structural one.

The precondition is enforced, not merely asserted in prose: `AttributeMap::add_modifier` (Deliverables) carries a debug-assertion-style runtime check that, before appending a new modifier to an attribute's own `Vec`, scans that attribute's existing modifiers for one already sharing the new modifier's `ModifierOperation`, and panics immediately — naming the attribute and the operation — if it finds one. A second same-operation modifier landing on one attribute is exactly the situation this exception depends on never occurring in M4, so this check is the single enforcement point that keeps the exception's precondition from ever being silently violated without the project noticing.

`AttributeInstance` lazily caches its computed value in vanilla, invalidated on any base/modifier mutation — this blueprint's own `compute_value` is a pure, uncached function; a caller wanting caching wraps it, this blueprint does not build a dirty-flag cache (out of scope, not load-bearing for correctness).

**Attribute registry and defaults**, restated verbatim from `19-combat-damage.md` §4 (high confidence — every row below is that document's own constants table) plus this blueprint's own per-mob-type overrides (moderate confidence, well-established but unverified against a real `--reports` dump — flagged for reconciliation, Implementation steps):

| `AttributeKind` | Default | `[min, max]` | Zombie | Cow | Villager |
|---|---|---|---|---|---|
| `AttackDamage` | 2.0 | `[0.0, 2048.0]` | 3.0 | *(unused)* | *(unused)* |
| `AttackKnockback` | 0.0 | `[0.0, 5.0]` | 0.0 | — | — |
| `AttackSpeed` | 4.0 | `[0.0, 1024.0]` | *(unused — §3.13, mobs have no cooldown curve)* | — | — |
| `Armor` | 0.0 | `[0.0, 30.0]` | 2.0 | 0.0 | 0.0 |
| `ArmorToughness` | 0.0 | `[0.0, 20.0]` | 0.0 | 0.0 | 0.0 |
| `KnockbackResistance` | 0.0 | `[-2.0, 1.0]` | 0.0 | 0.0 | 0.0 |
| `MaxHealth` | 20.0 | `[1.0, 1024.0]` (moderate confidence on bounds) | 20.0 | 10.0 (moderate confidence) | 20.0 (moderate confidence) |
| `SafeFallDistance` | 3.0 | `[-1024.0, 1024.0]` | 3.0 | 3.0 | 3.0 |
| `FallDamageMultiplier` | 1.0 | `[0.0, 100.0]` | 1.0 | 1.0 | 1.0 |
| `SweepingDamageRatio` | 0.0 | `[0.0, 1.0]` | — | — | — |

`Gravity` is deliberately **not** an `AttributeKind` here — it is owned by `rc-physics`'s own `GRAVITY_LIVING` constant (MECH-D37, M3-B02), not duplicated. `Luck` is deliberately not modeled — no loot-luck consumer exists in this blueprint's own scope.

### Player health — a bounded interim stand-in, not a composition-model migration

`05-game-mechanics.md`'s Entity Composition Model names a `PlayerBundle` sibling to `MobBundle` under `LivingEntity`'s rung (MECH-D29), but M4-B01 explicitly does not build it ("No other vanilla entity type is named or given a bundle by this blueprint"). `PlayerMarker` (M1-B05/M2-B07/M3-B02/M3-B03) remains the player's own component set, with no `health` field anywhere. Migrating players onto the full composition model is a real redesign this blueprint does not attempt (it would touch tracking, persistence, and movement systems this blueprint does not own). Instead, mirroring M3-B03's own identical `GameModeState`-as-"smallest possible slice" precedent, this blueprint adds one new, minimal, player-only component:

```rust
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlayerCombatState {
    pub health: f32,
    pub absorption: f32,
    pub hurt_time: i16,
    pub death_time: i16,
    pub is_dead: bool,
    pub attributes: crate::entity::combat::attributes::AttributeMap,
}
```

Its four health-adjacent fields (`health`, `absorption`, `hurt_time`, `death_time`, `is_dead`) are declared **field-for-field identical in name and type** to `LivingEntity`'s own equivalents (this blueprint's own addition, below) so that a future player-composition-model-migration blueprint can delete `PlayerCombatState` and fold the player entity onto real `BaseEntity`+`LivingEntity` components with a pure field-rename, not a redesign — stated here as a deliberate, cited, bounded duplication, not an oversight.

### Enchantment level source — a bounded-zero stub

No item/inventory model carries enchantment data anywhere in the merged codebase (MECH-D47, still M4+ unbuilt scope; `HeldItemStub`, M3-B03, is `Block(PlaceableBlockKind)|Tool(ToolMaterial, ToolKind)` — no enchantment field at all). Every enchantment-level input this blueprint's damage-pipeline functions need is threaded as an explicit, plain parameter (`EnchantLevels`, Deliverables) — the pipeline math itself is implemented **exactly and fully**, unit-tested against hand-fed nonzero levels (Acceptance tests' golden matrices). The one production call site that must supply a value, `rusty-clanker-server::play::combat::resolve_enchant_levels(item: &HeldItemStub) -> EnchantLevels`, is a bounded stub that **always returns `EnchantLevels::default()`** (every field `0`) — explicitly flagged in its own doc comment as the seam a future items/enchantment blueprint replaces, not this blueprint's own pipeline math (identical framing to `HeldItemStub`/`GameModeState`'s own established precedent).

### Damage sources — a bounded subset of the registry

MECH-D52's data-driven `damage_type` registry (51 vanilla entries) does not exist — no `xtask codegen` output for it has been specified by any blueprint yet. This blueprint hand-declares exactly the four damage-type entries its own systems ever construct, restated from `19-combat-damage.md` §3.17's own grouped table (high confidence, that table's own literal rows):

```rust
pub enum DamageTypeKind { PlayerAttack, MobAttack, Fall, Starve }
```

| `DamageTypeKind` | `exhaustion` | `scaling` | `bypasses_armor` | `no_knockback` |
|---|---|---|---|---|
| `PlayerAttack` | 0.1 | `WhenCausedByLivingNonPlayer` | false | false |
| `MobAttack` | 0.1 | `WhenCausedByLivingNonPlayer` | false | false |
| `Fall` | 0.0 | `WhenCausedByLivingNonPlayer` | true | true |
| `Starve` | 0.0 | `WhenCausedByLivingNonPlayer` | true | true |

A future blueprint extending this enum for a new damage-causing mechanic (fire, drowning, potions, explosions) adds a variant and a matching table row here, at that mechanic's own point of implementation — this blueprint's own `apply_damage_pipeline` (below) is written generically over `DamageTypeKind`, not hardcoded to these four beyond their table rows.

### The damage pipeline — order of operations, exact (19-combat-damage.md §3.1–§3.7, §3.10)

Per hit, in this exact order (steps this blueprint excludes are listed and skipped explicitly, never silently omitted):

1. ~~Shield block~~ — excluded (Context, Scope boundary).
2. Freezing ×5.0 / damaged-helmet ×0.75 multipliers — **excluded**: no freeze/ice mechanic and no armor-durability model exist; this step is a no-op multiply-by-1.0 in this blueprint's own pipeline, restated as a literal identity step so a future blueprint inserts real logic at the correct position rather than at the end of the function.
3. **Invulnerability top-up gate** (§3.2, exact):
   ```
   if invulnerable_time > 10 (not > 0):
       if damage <= last_hurt: return NoOp                    # fully absorbed
       actually_hurt(damage - last_hurt)                       # delta only
       last_hurt = damage                                      # invulnerable_time NOT reset
   else:
       last_hurt = damage
       invulnerable_time = 20
       actually_hurt(damage)                                   # full damage
       hurt_time = 10
   ```
   `invulnerable_time` decrements by 1 every player/mob tick (this blueprint's own Stage-6b system, below, for mobs; a manual per-tick decrement for players inside the same combat tick-loop step). `BYPASSES_COOLDOWN` is an empty vanilla tag at 26.2 — no `DamageTypeKind` this blueprint declares bypasses the window; the check is a fixed `false` per damage type, restated rather than hardcoded away, so a future variant can flip it.
4. **Armor absorption** (`actually_hurt`'s own first sub-step, MECH-D45, exact, only if `!bypasses_armor`):
   ```
   toughness      = 2.0 + armor_toughness / 4.0
   real_armor     = clamp(total_armor - damage / toughness, total_armor * 0.2, 20.0)
   armor_fraction = clamp(real_armor / 25.0, 0.0, 1.0)          # weapon armor-effectiveness enchant hook (Breach) excluded — no Mace
   damage         = damage * (1.0 - armor_fraction)
   ```
   `total_armor = floor(AttributeMap[Armor])` (int-floored before use, exactly as source does, even though the rest of the formula is float).
5. Resistance status effect — **excluded** (Context, Scope boundary; MECH-D46 out of scope). Skipped, not silently omitted.
6. **Enchantment protection (EPF)** (§3.6, exact, only if damage type is not the (currently empty) `bypasses_enchantments` set):
   ```
   epf_sum      = protection_epf(protection_lvl) + fire_protection_epf(...) + blast_protection_epf(...)
                + projectile_protection_epf(...) + feather_falling_epf(...)     # summed, uncapped
   real_epf     = clamp(epf_sum, 0.0, 20.0)
   damage       = damage * (1.0 - real_epf / 25.0)
   ```
   EPF-per-level table (§4, high confidence): Protection `1.0`/level (max level 4 → max EPF 4); Fire/Blast/Projectile Protection `2.0`/level (max 4 → max EPF 8 each); Feather Falling `3.0`/level (max 4 → max EPF 12). Feather Falling applies to `Fall` damage specifically (its own `is_fall` gate) — restated because `Fall`'s own `bypasses_armor = true` does **not** also bypass this step (research corpus hazard #7 — a common reimplementation bug this blueprint's own step ordering avoids by construction).
7. **Absorption hearts** (§3.7, exact — the player/mob asymmetry, restated verbatim, reproduced exactly, not "corrected"):
   - Mob path (target has no `PlayerCombatState`, only `LivingEntity.absorption`):
     ```
     original = damage
     damage   = max(damage - absorption, 0.0)
     absorption -= (original - damage)                # first subtraction: the absorbed portion
     if damage != 0.0:
         health -= damage
         absorption -= damage                          # SECOND subtraction — mob path only, reproduced exactly
     ```
   - Player path (`PlayerCombatState`):
     ```
     original = damage
     damage   = max(damage - absorption, 0.0)
     absorption -= (original - damage)                 # only ONE subtraction
     if damage != 0.0:
         causeFoodExhaustion(damage_type.exhaustion)    # Context, "Food & exhaustion" — only here, only if net damage nonzero
         health -= damage
     ```
   Both paths clamp `absorption` to `>= 0.0` after subtraction (never negative).

Fired on the fresh-hit branch of the invulnerability gate only (`tookFullDamage` per §3.1 — never on the top-up-delta branch, even when that branch deals damage): **knockback impulse #1** (Context, "Knockback") if the damage type is not `no_knockback`-tagged; hurt/death packets (Context, "Packets").

### Attack-cooldown charge curve (§3.9a, exact — players only)

```
attack_strength_delay(ticks) = 1.0 / attack_speed_attribute * 20.0        # default ATTACK_SPEED=4.0 -> 5 ticks
charge_scale(ticker, offset) = clamp((ticker + offset) / attack_strength_delay, 0.0, 1.0)
base_damage_scale_factor()   = 0.2 + charge_scale(ticker, 0.5)^2 * 0.8
```
`attack_strength_ticker` (this blueprint's own field on `PlayerCombatState`... **correction**: per the Player Health section above, `PlayerCombatState` does not declare it — restated here as this blueprint's own concrete resolution: `attack_strength_ticker: u32` lives on the new `CombatRuntimeState` component, below, attached to both players and mobs (mobs never advance or read it — §3.13 has no cooldown curve) so the two entity shapes share one runtime-timer component instead of two near-duplicate ones) increments by 1 every player tick and resets to 0 on `on_attack()` (fired at the start of every successful `Attack` packet dispatch, Context "Packets," regardless of whether the swing dealt damage).

### Player melee assembly (§3.9, exact order)

```
base_damage  = AttributeMap[AttackDamage]                                  # no weapon model -> attribute value directly
charge_scale = charge_scale(ticker, 0.5)
magic_boost  = charge_scale * (enchanted_damage(base_damage, enchants) - base_damage)   # against UNSCALED base_damage
base_damage *= base_damage_scale_factor()                                  # charge curve applied HERE
full_strength    = charge_scale > 0.9                                       # exact threshold, §3.9b
knockback_attack = is_sprinting && full_strength                            # +0.5 knockback, KNOCKBACK sound
                                                                              # Mace bonus excluded — no Mace
critical = full_strength && can_critical_attack(fall_distance, on_ground, in_water, on_climbable, target_is_living)
if critical: base_damage *= 1.5
total_damage = base_damage + magic_boost
sweep = full_strength && !critical && !knockback_attack && on_ground
        && horizontal_speed_sq < (movement_speed * 2.5)^2 && main_hand_is_sword
```
`can_critical_attack`: `fall_distance > 0.0 && !on_ground && !on_climbable && !in_water && target_is_living && !is_sprinting` (mobility-restriction/passenger checks excluded — no such state exists yet; `target_is_living` is the attacked entity being a `LivingEntity` — an attack against a non-living target, e.g. M4-B01's `Item` kind, can never crit). Deterministic, no RNG (§3.9b, exact). `enchanted_damage(base, enchants)`: `base + sharpness_bonus(enchants.sharpness) + (target_is_undead ? smite_bonus(enchants.smite) : 0.0) + (target_is_arthropod ? bane_bonus(enchants.bane_of_arthropods) : 0.0)`.

**Enchant-bonus formulas** (§4, high confidence, all `linear(base, per_level_above_first)` evaluated as `base + per_level * (level - 1)` for `level >= 1`, else `0.0`):

| Enchant | base | per-level |
|---|---|---|
| Sharpness | 1.0 | 0.5 |
| Smite | 2.5 | 2.5 |
| Bane of Arthropods | 2.5 | 2.5 |

**Sweep** (§3.9c/d, included — 05's MECH-D43/D44 carry no distinct sweep decision ID, but `Player.attack`'s own sweep branch shares the identical `full_strength`/`knockback_attack`/`critical` context this blueprint already computes in full for the cooldown/crit formulas; implementing critical without sweep would leave the one function both live in incompletely restated):
```
sweep_ratio = sweeping_edge_level > 0 ? sweeping_edge_level / (sweeping_edge_level + 1) as f32 : 0.0   # SWEEPING_DAMAGE_RATIO
sweep_base  = 1.0 + sweep_ratio * base_damage                                    # base_damage = post-charge, post-crit
for each LivingEntity within primary_target_aabb.inflate(1.0, 0.25, 1.0), distance_sq (attacker-to-nearby) < 9.0,
        excluding self, primary target, entities allied to the attacker, and marker ArmorStands:
    per_target = enchanted_damage(sweep_base, enchants) * charge_scale           # re-invokes enchant formula independently per target
    if apply_damage_pipeline(nearby, PlayerAttack_source, per_target).dealt:
        apply_knockback_impulse_2(nearby, 0.4, (sin(attacker_yaw), -cos(attacker_yaw)))   # attacker-yaw-directed, flat 0.4, no enchant term
```

### Mob melee attacks — the AI seam (§3.13, exact — Stage 6b, `EntityPhysicsIntegration`)

Fully deterministic, no crit/sweep/charge curve, no RNG:
```
damage = AttributeMap[AttackDamage] + enchant_bonus(target)         # enchant_bonus always 0 in production (bounded stub)
outcome = apply_damage_pipeline(target, MobAttack_source, damage)
if outcome.dealt:
    apply_knockback_impulse_2(target, get_knockback(attacker, target), (sin(attacker_yaw), -cos(attacker_yaw)))
```
**The seam this blueprint owns and exposes, not fulfills:** *deciding when* a mob attacks — target selection, approach, attack-range/cooldown timing (vanilla's own melee-attack goal, e.g. `MeleeAttackGoal`) — is Stage-6a AI/goal-selector content (MECH-D31/D32) that a future AI blueprint (referred to by this project's own task assignment as "B03") owns. This blueprint defines the exact, minimal contract that future blueprint's Stage-6a system must produce, and implements the Stage-6b consumer against it now, so the two blueprints can land independently:

```rust
/// Written by a Stage-6a system (a future AI blueprint), consumed and cleared by this
/// blueprint's own Stage-6b `apply_mob_melee_attacks` system. One component per attacker
/// entity; presence for one tick means "attack `target` this tick," matching MECH-D32's own
/// "chosen-action command consumed by Stage 6b" framing exactly. Never read or written by
/// this blueprint's own Stage-6a (this blueprint registers nothing into `EntityAiSelection`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingMeleeAttack {
    pub target: rc_core::RcEntityId,
}
```
This blueprint's own acceptance tests attach `PendingMeleeAttack` directly (bypassing any real goal selector, which does not exist yet) to prove the Stage-6b consumer's own damage/knockback math end-to-end — restated explicitly as this blueprint's own test-only production of the signal a future AI blueprint will produce for real.

### Knockback — the two-impulse model (§3.10, exact)

**Impulse #1** (`deal_default_knockback`, fired by `apply_damage_pipeline` on the fresh-hit branch of the invulnerability gate whenever the damage source's `DamageTypeKind::no_knockback()` is `false` — Context, "Damage pipeline" step 3; never fired on the invulnerability top-up branch even when that branch deals damage, and it does fire on a fresh hit whose damage is reduced to zero by armor): direction = `(source_pos.x - victim_pos.x, source_pos.z - victim_pos.z)` normalized in the XZ plane — i.e. from the victim toward the source; magnitude flat `0.4`. `Fall`/`Starve` are `no_knockback = true` in the table above, modeled explicitly as a per-`DamageTypeKind` flag rather than inferred from an absent source position — impulse #1 never fires for them, and their `source_position` is simply never consulted.

**Impulse #2** (attacker-directed, fired strictly *after* impulse #1 has already mutated velocity — Context above shows both call sites): direction = `(sin(attacker_yaw_rad), -cos(attacker_yaw_rad))`; magnitude = `get_knockback(attacker, target) + (knockback_attack ? 0.5 : 0.0)` (player path) or `get_knockback(attacker, target)` (mob path, `knockback_attack` always `false`).

```
get_knockback(attacker, target) = (attacker.AttributeMap[AttackKnockback] + knockback_enchant_bonus(level)) / 2.0
knockback_enchant_bonus(level) = level > 0 ? 1.0 + 1.0 * (level - 1) : 0.0     # linear(base=1, per_level=1)
```

**The impulse itself** (`apply_knockback_impulse`, exact — called twice per hit with different `(power, dir_xz)` inputs, never merged into one call — reproducing vanilla's own velocity-halving-per-call behavior):
```
power *= (1.0 - target.AttributeMap[KnockbackResistance])
if power <= 0.0: return velocity unchanged
(xd, zd) = dir_xz
while xd*xd + zd*zd < KNOCKBACK_DEGENERATE_THRESHOLD:        # degenerate direction — rare, re-drawn until the sample clears the threshold
    xd = (ambient_rng.next_double() - ambient_rng.next_double()) * 0.01
    zd = (ambient_rng.next_double() - ambient_rng.next_double()) * 0.01
# KNOCKBACK_DEGENERATE_THRESHOLD = f64::from(1.0e-5_f32) = 9.999999747378752e-6, NOT the double
# literal 1e-5 — the reference compares against the float 1.0E-5F widened to double, so the
# threshold must be derived via an f32->f64 cast, never transcribed as a decimal literal.
# RNG consumption is therefore 4 draws per loop iteration and unbounded, not a fixed 4.
(dx, dz) = normalize(xd, zd) * power
new_vx = old_vx / 2.0 - dx
new_vz = old_vz / 2.0 - dz
new_vy = on_ground ? min(0.4, old_vy / 2.0 + power) : old_vy
```
`ambient_rng` is one `rc_mechanics::random::RcRandom` instance, held in a new `AmbientCombatRandom(RcRandom)` resource seeded once at world init (Context, "Ambient combat RNG" below) — this rare fallback is not part of MECH-D5's seed-determinism contract (19-combat-damage.md §5's own RNG map: "combat RNG is not part of seed-determinism... call count and order still matter," not "must be world-seed-reproducible") but still uses `RcRandom`'s bit-exact LCG per MECH-D5's blanket mandate that every vanilla-observable random draw goes through it.

### Fall damage (§3.15, exact — players only, Context "Mob fall damage — deferred")

```
fall_power  = fall_distance + 1e-6 - AttributeMap[SafeFallDistance]
fall_damage = floor(fall_power * damage_modifier * AttributeMap[FallDamageMultiplier])   # int result; damage_modifier fixed 1.0 (Context)
```
Per-block landing modifiers (Bed halves the fall distance itself before the default modifier applies; Hay Bale sets `damage_modifier = 0.2` while passing the fall distance through unscaled, so `SafeFallDistance` still subtracts from the full distance first; Slime Block sets `damage_modifier = 0.0` in both the bouncing and the sneaking branches — sneaking, `isSuppressingBounce`, instead skips the fall-damage call entirely, suppressing the bounce rather than restoring fall damage; Powder Snow full immunity) are **not** implemented — `damage_modifier` is always vanilla's own default `1.0`, deferred to a future block-behavior blueprint that extends `BlockBehavior` with an `on_fall` hook (M3-B01's registry, not touched here). Fall damage is skipped entirely (not computed) if `fall_damage <= 0` or the falling player's `GameModeState.instabuild` is `true` (creative — Context, "Damage invulnerability gate," below); the resulting `fall_damage` (if positive) re-enters `apply_damage_pipeline` as ordinary `Fall`-typed damage (armor-bypassing, EPF/Feather-Falling-subject, per the pipeline table above).

**Cited additive modification to M3-B02's `crates/server/src/play/movement.rs`.** M3-B02's own `evaluate_movement` resets `player.motion.fall_distance = 0.0` the instant `on_ground` becomes true (both its client-reported-`on_ground` branch and its server-replayed-fallback branch), and its own Context explicitly flags fall damage as "M4" — but a reset-before-my-own-later-manual-step reads it would leave nothing to consume. This blueprint adds one field to `PlayerMotion` and one capture line at each of the two existing reset sites (both already-cited call sites, no new branch structure):
```rust
// PlayerMotion (movement.rs, MODIFY): add one field.
pub landed_fall_distance: Option<f64>,   // Some(d) for exactly one tick after a landing with d > 0.0, else None
```
Immediately before each existing `player.motion.fall_distance = 0.0` line (both the `if let Some(on_ground) = report.on_ground { ... }` branch and its `else` replay-fallback branch, M3-B02 Context): `if player.motion.fall_distance > 0.0 { player.motion.landed_fall_distance = Some(player.motion.fall_distance); }`. This blueprint's own combat tick-loop step, run immediately after M3-B02/M3-B03's own movement/mining steps (Context, "Which pipeline stage"), calls `player.motion.landed_fall_distance.take()` (consuming and clearing it) for every player, applying fall damage if `Some(d)` and `d > 0.0`.

### Damage invulnerability gate (creative/`instabuild`)

`apply_damage_pipeline`'s very first check, before step 1 of the Context "Damage pipeline" list: if the target is a player and `GameModeState.instabuild == true` (M1-B05 hardcodes every player's gamemode to Creative by default, M3-B03's own `debug_set_survival` is the only production-reachable way to flip it), the pipeline returns `DamageOutcome::Invulnerable` immediately — no invulnerability-window consumption, no animation, no packets. Matches vanilla's own `abilities.invulnerable` short-circuit; reuses `GameModeState` unmodified rather than inventing a second invulnerability flag.

### Difficulty scaling (§3.8, MECH-D64/D65 — minimal, real content)

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Difficulty { Peaceful, Easy, #[default] Normal, Hard }

pub struct GlobalDifficulty(pub Difficulty);   // Resource, one per world, default Normal (vanilla's own default)
```
Applied to *incoming player damage only*, strictly before the Context "Damage pipeline" steps (§3.8's own text: "before any of §3.1's item-blocking/freeze/helmet logic"), and only for damage types whose `scaling == WhenCausedByLivingNonPlayer` when the attacker is a non-player living entity, or `Always` unconditionally (this blueprint's own four `DamageTypeKind`s are all `WhenCausedByLivingNonPlayer` — `MobAttack` is the only one ever scaled, since `PlayerAttack`'s causing entity is always a player):
```
Peaceful: damage = 0.0
Easy:     damage = min(damage / 2.0 + 1.0, damage)
Hard:     damage = damage * 3.0 / 2.0   # multiply then divide, NOT a single `damage * 1.5` (differ only above f32::MAX/3, where this form overflows to infinity and a single x1.5 stays finite)
Normal:   unchanged
```
If the scaled result is exactly `0.0`, the whole hit short-circuits (`DamageOutcome::NoOp`, no invulnerability consumption) exactly as §3.8 specifies. `GlobalDifficulty` is test/debug-settable (`HardcodedWorld::debug_set_difficulty`, mirroring every prior `debug_*` precedent) — no `/difficulty` command exists yet (MECH-D70's Tier-1 command list is a future blueprint's scope).

### Death (MECH-D29/D51, generic across players and mobs)

Health reaching `<= 0.0` inside `apply_damage_pipeline`'s absorption step (Context, "Damage pipeline" step 7) sets `is_dead = true` on the target (`LivingEntity.is_dead` for mobs, `PlayerCombatState.is_dead` for players) and returns `DamageOutcome::Died` instead of `Dealt`. `is_dead` gates every other system this blueprint registers (Stage-6b mob-attack, the manual combat packet-apply step) from acting on that entity again — a dead entity neither attacks nor is attacked further this tick or any later one.

**Mob death**: this blueprint's own Stage-6b system, on observing a newly-`is_dead` mob, (a) broadcasts `Entity Event{event_id: 3}` (death animation, Context "Packets") to every tracking viewer, (b) rolls loot via the seam below, spawning the resulting item entities (M4-B01's own `ItemBundle`/tracking/spawn machinery, reused unmodified) at the mob's death position with a small vanilla-documented randomized velocity spread (`vx, vy, vz = rng.next_double()*0.2 - 0.1, 0.2, rng.next_double()*0.2 - 0.1` — `vy` is the constant `0.2`, not a random draw, so each item spawn consumes exactly two `next_double()` calls, x then z; moderate confidence on the exact spread constants, restated from long-stable community knowledge, flagged for reconciliation), and (c) despawns the mob entity (removes it from the ECS `World`, `EntityIndex`, `NetworkEntityIndex`, and every viewer's `tracked_entities` via a synthetic `Remove Entities` broadcast, reusing M4-B01's own despawn-packet shape unmodified).

**Player death**: broadcasts `Player Combat Kill` (Context "Packets") and `Set Health{health: 0.0, ...}` to the dying player, and sets `is_dead = true` on `PlayerCombatState` — the player entity is **not** removed from the world (unlike a mob) and no further combat/movement/interaction packet from that connection is processed while `is_dead` (this blueprint's own combat step and M3-B02/B03's own movement/mining steps each gain one `if player_combat_state.is_dead { continue; }` guard at the top of their per-player loop — a minimal, additive, cited change to each). **Explicitly out of scope**: the actual respawn packet round-trip, keepInventory-gated inventory drop, and bed/anchor spawn-point resolution (MECH-D59) — a future player-lifecycle blueprint clears `is_dead` and completes the cycle; this blueprint leaves a dead player connected, visibly dead, indefinitely, which is an accepted, documented interim gap, not a silent one.

**The loot-drop seam** (MECH-D55's data-driven interpreter is not built by any blueprint yet — the roadmap's own "M4-B02" task is expected to own it): this blueprint defines the exact trait contract its own death system consumes, and ships one bounded, minimal implementation sufficient for its own tests and for real production use of the four tier-2 mob kinds — not the general interpreter:

```rust
/// The seam a future data-driven loot-table blueprint (MECH-D55: pools -> entries ->
/// functions/conditions, interpreted against `rc_mechanics::random::RcRandom`) is expected
/// to implement in place of `FixedTierTwoLoot`. This blueprint's own death system depends
/// only on this trait, never on a concrete implementation, so landing the real interpreter
/// requires zero change to `death.rs`'s own call site — only a new `impl` and a swapped
/// resource insertion at the composition root.
pub trait EntityLootProvider: Send + Sync {
    /// Rolls this one death's drop table. `rng` is the region's own ambient RcRandom
    /// instance (Context, "Ambient combat RNG") — MECH-D5 governs: every roll this call
    /// makes must come from `rng`, never a fresh/ambient source of its own.
    fn roll_death_loot(
        &self,
        kind: crate::entity::EntityKind,
        rng: &mut crate::random::RcRandom,
    ) -> Vec<crate::entity::kinds::ItemStackRecord>;
}

/// This blueprint's own bounded, hand-authored implementation — a fixed, non-random-count
/// item per tier-2 kind, deliberately not attempting MECH-D55's pools/functions/conditions
/// shape (`ore_drops`-style Fortune scaling, `looting_enchant` count bonuses, etc. are all
/// future-blueprint content). `Item`'s own kind never appears here (item entities do not
/// drop loot on "death" — they merge/despawn per MECH-D51, M4-B01, unmodified).
pub struct FixedTierTwoLoot;

impl EntityLootProvider for FixedTierTwoLoot {
    fn roll_death_loot(&self, kind: EntityKind, rng: &mut RcRandom) -> Vec<ItemStackRecord>;
    // Zombie -> 0..=2 rotten_flesh (uniform via rng.next_int_bounded(3)); Cow -> 1..=3 beef
    // + 0..=2 leather (two independent rng.next_int_bounded calls, in that field order);
    // Villager -> empty Vec (vanilla villagers drop nothing on death). Registry ids for
    // rotten_flesh/beef/leather are hand-typed RegistryEntryId constants mirroring M4-B01's
    // own villager_type/profession constant-transcription convention — reconciled against a
    // real `xtask codegen` run at Implementation-steps time, one-line-per-constant.
}
```

### Ambient combat RNG

`AmbientCombatRandom(pub RcRandom)`, a `Resource`, inserted once per region at the same bootstrap point every other Stage-4-adjacent resource is inserted (mirroring M3-B01's `bootstrap_default_stage4_resources` precedent), seeded `RcRandom::new(0x5EED_C0BA)` (an arbitrary fixed constant — Context, "Knockback," already establishes this stream is not part of MECH-D5's seed-determinism contract, so a fixed literal is a legitimate, deterministic-for-tests, non-parity-relevant seed choice, not a shortcut around a real requirement). Consumed by the knockback degenerate-direction fallback and by `FixedTierTwoLoot::roll_death_loot`.

### Food, exhaustion, and natural regeneration (§3.18, exact — bounded to what this blueprint's own systems produce)

**In scope**: exhaustion cost on landing a melee attack (flat `0.1`, `EXHAUSTION_ATTACK`) and on taking net damage (per `DamageTypeKind.exhaustion`, Context table — charged only when net health damage after absorption is nonzero, per §3.7's own player-path ordering, restated at its exact pipeline position above); the shared natural-regen/starvation tick. **Out of scope**: sprint/swim/walk/crouch/jump exhaustion (movement-system territory, M3-B02's own scope, not extended here) and mining exhaustion (M3-B03's own scope, not extended here) — a future revision of either blueprint adds its own `add_exhaustion` call against the `FoodStats` component this blueprint defines; this blueprint does not call into movement or mining code.

```rust
#[derive(Component, Clone, Debug, PartialEq)]
pub struct FoodStats {
    pub food_level: i32,          // 0..=20, starts 20
    pub saturation: f32,          // starts 5.0 (vanilla's own join default)
    pub exhaustion: f32,          // starts 0.0, ceiling 40.0
    pub regen_tick_timer: u32,    // internal, starts 0
}
```
Attached to every player alongside `PlayerCombatState` at join. `add_exhaustion(&mut self, amount: f32)`: `self.exhaustion = (self.exhaustion + amount).min(40.0)`.

**Per-player-tick algorithm** (§3.18, exact — this blueprint's own combat tick-loop step runs it once per player per tick, after the packet-apply half):
```
if exhaustion > 4.0:
    exhaustion -= 4.0
    if saturation > 0.0: saturation = max(saturation - 1.0, 0.0)
    elif difficulty != Peaceful: food_level = max(food_level - 1, 0)

if saturation > 0.0 && is_hurt(health, max_health) && food_level >= 20:
    regen_tick_timer += 1
    if regen_tick_timer >= 10:
        spend = min(saturation, 6.0)
        health = min(health + spend / 6.0, max_health)
        add_exhaustion(spend)
        regen_tick_timer = 0
elif food_level >= 18 && is_hurt(health, max_health):
    regen_tick_timer += 1
    if regen_tick_timer >= 80:
        health = min(health + 1.0, max_health)
        add_exhaustion(6.0)
        regen_tick_timer = 0
elif food_level <= 0:
    regen_tick_timer += 1
    if regen_tick_timer >= 80:
        if health > 10.0 || difficulty == Hard || (health > 1.0 && difficulty == Normal):
            apply_damage_pipeline(player, Starve_source, 1.0)
        regen_tick_timer = 0
else:
    regen_tick_timer = 0
```
`is_hurt(health, max_health) = health < max_health && health > 0.0`. This entire block is skipped for a creative/`instabuild` player (Context, "Damage invulnerability gate" — vanilla's own natural-regen gamerule/creative interaction; simplified here to the same `instabuild` gate this blueprint already uses everywhere else, rather than modeling the separate `naturalRegeneration` gamerule, which does not exist yet). Health/food/saturation changes trigger one `Set Health` packet to the owning player (Context, "Packets") — coalesced to at most one per tick per player even if both the decay and regen branches both fired.

### Packets (NET-D3, restated — moderate confidence on every numeric id, per this project's own established live-fetch-and-flag convention, M1-B05/M2-B07/M3-B03/M4-B01)

Restated from a live fetch of `minecraft.wiki/w/Java_Edition_protocol/Packets` performed while deriving this blueprint (2026-08-21), cross-checked against this project's own long-stable prior knowledge of these packets' shapes where the fetch's own summarization looked incomplete (flagged per-row below) — every id and field needs the same one-line reconciliation against a real `reports/packets.json` every prior blueprint's own hand-typed table already carries.

| Packet | Bound | ID | Fields (wire order) |
|---|---|---|---|
| `Attack` | server | `0x01` | `entity_id: i32 #[rc(varint)]` — a single-field record, ordinary `#[derive(RcPacket)]` shape (no conditional fields, no custom codec). This blueprint decodes, reach/angle-validates, and dispatches every `Attack` through the melee pipeline (Context, "Player melee assembly") |
| `Interact` | server | `0x1A` | `entity_id: i32 #[rc(varint)]`, `hand: i32 #[rc(varint)]` (0=main, 1=off), `location: LpVec3` (a quantized, variable-length position codec this blueprint introduces: a single `0x00` byte for a near-zero vector, else a fixed 6-byte payload of three 15-bit quantized components plus a 2-bit scale and continuation flag, followed by one trailing VarInt scale field when the continuation flag is set — moderate confidence on the exact bit layout, flagged for reconciliation against a real packet capture), `using_secondary_action: bool` — a flat, unconditional four-field layout with **no discriminator and no conditional field groups**. **Hand-implemented `RcPacket`** (not `#[derive(RcPacket)]`) only because `location`'s `LpVec3` codec is bespoke, not because of any conditional shape — mirroring M4-B01's own `Set Entity Data` precedent for a different reason. This blueprint decodes and validates the packet but never acts on it: right-click interaction (trading/feeding/mounting) has no modeled mechanic in this blueprint's own scope, so `Interact` is always a silent no-op accept; only round-trip codec correctness is asserted, never the decoded `location` value |
| `Set Health` | client | `0x68` | `health: f32`, `food: i32 #[rc(varint)]`, `saturation: f32` |
| `Update Attributes` | client | `0x83` | `entity_id: i32 #[rc(varint)]`, then a raw, `VarInt`-count-prefixed sequence of `{attribute_id: i32 #[rc(varint)] (this project's own numeric registry-id convention, matching every other post-M4-B01 registry reference — not an `Identifier` string), base_value: f64, modifier_count: i32 #[rc(varint)] (always `0` — this blueprint never sends a live `AttributeModifier` over the wire, Context "Attribute system" scopes modifiers to server-internal use only at M4)}` — **hand-implemented `RcPacket`**, mirroring `Set Entity Data`, since the nested variable-count struct array is the same "derive shape doesn't fit" case |
| `Damage Event` | client | `0x19` | `entity_id: i32 #[rc(varint)]`, `source_type_id: i32 #[rc(varint)]` (this blueprint's own `DamageTypeKind` ordinal, Context table), `source_cause_id: i32 #[rc(varint)]` (network entity id + 1, `0` = none), `source_direct_id: i32 #[rc(varint)]` (network entity id + 1, `0` = none — always equal to `source_cause_id` for this blueprint's own melee-only sources, since no indirect-damage mechanic like a thrown potion exists), `has_source_position: bool`, `[if true: source_x: f64, source_y: f64, source_z: f64]` (this blueprint always sends `false`/omits — fall/starve have no attacker position and melee already carries `source_cause_id`, redundant for this blueprint's own scope; the field exists in the wire shape for a future mechanic that needs it, e.g. a future explosion blueprint) |
| `Entity Event` | client | `0x22` | `entity_id: i32` (plain `Int`, **not** VarInt — a genuine, cited asymmetry with every other entity-id field in this blueprint's own packets, restated exactly per the live fetch, mirroring M4-B01's own precedent of flagging real per-packet field-encoding asymmetries rather than "normalizing" them away), `event_id: u8` — this blueprint constructs only `3` (death animation); `event_id 2` is vanilla's `KINETIC_HIT` (a weapon-hit-sound cue this blueprint's own scope never triggers, no `KineticWeapon`-shaped item component exists), not a generic hurt animation — hurt/hit feedback for a live target is carried entirely by the `Damage Event` packet (`0x19`) this blueprint already sends, moderate confidence on the death ordinal, well-established and long-stable |
| `Player Combat Kill` | client | `0x44` | `player_id: i32 #[rc(varint)]`, `message: String #[rc(prefixed_string)]` (a plain, length-prefixed string payload — a deliberate, real-client-visible wire-shape divergence from vanilla, which encodes this field as an NBT-backed chat `Component` (`ComponentSerialization.TRUSTED_STREAM_CODEC`), not a length-prefixed string; this blueprint's own stand-in exists because no real text-component `TextComponent` encoding exists anywhere in this project yet — M4-B01's own `OptionalTextComponent` metadata variant carries the identical, already-accepted "plain string, not real JSON-text-component" simplification; reused here rather than inventing a second one) — this blueprint always sends a fixed, hand-authored message (`"<killed>"`-shaped placeholder text, Implementation steps gives the exact literal), real death-message composition (attacker name, weapon name) is a future blueprint's scope |

`Set Entity Velocity` (M4-B01, reused unmodified) broadcasts every knockback-impulse result to every tracking viewer (both impulses collapsed into one packet per hit — only the *final* post-both-impulses velocity is ever sent, matching vanilla's own single velocity-update-packet-per-hit behavior even though the *server-side* math applies two separate impulses). `Set Entity Data` (M4-B01, reused unmodified) broadcasts a health-metadata-index (index 9, per M4-B01's own table) update to every viewer other than the entity's own owning player (who receives `Set Health` instead, for players) whenever health changes.

### Reach and angle validation for entity targets (MECH-D62, extended from block targets)

`entity_interaction_range` attribute does not exist in this blueprint's own `AttributeKind` table (Context, "Attribute system") — restated here as a **fixed constant** per MECH-D62's own text ("default ~3.0... exact per-gamemode defaults to be pinned... at blueprint time"), pinned now: `ENTITY_INTERACTION_RANGE: f64 = 3.0` (moderate confidence, matches this blueprint's own pinning discipline for every other numeric constant this section restates). A per-player-position, per-target-entity raycast (reusing M3-B03's own `cast_ray`-established DDA/slab-intersection style, applied against the target entity's own bounding box rather than a block's `VoxelShape`):

```rust
/// Per-kind hitbox width/height, moderate confidence (well-established, unverified against
/// a real entity-dimensions data dump — reconciliation item, Implementation steps).
/// Zombie/Villager: 0.6 x 1.95. Cow: 0.9 x 1.4. Item: 0.25 x 0.25 (never a combat target
/// in practice — included only for `EntityDimensions`'s own match-exhaustiveness).
pub fn entity_dimensions(kind: EntityKind) -> (f64, f64);

/// `origin`/`direction` per M3-B03's own shared look-vector construction (`mth_sin`/
/// `mth_cos`-based yaw term, `f64::sin`/`cos` pitch term, reused unmodified — Context,
/// M3-B03's own "Orientation from placement context"). Builds the target's AABB from
/// `entity_dimensions` centered on `target_pos`, tests via the standard slab method
/// (min/max `t` per axis), accepts only if the ray enters the box within
/// `[0, ENTITY_INTERACTION_RANGE]` **and** the straight-line Euclidean distance from
/// `origin` to the target's own center is also `<= ENTITY_INTERACTION_RANGE` (belt-and-
/// suspenders — the two checks agree for every non-degenerate case; kept both, since the
/// slab test alone would accept a graze along a box edge at extreme range if `direction`
/// were not exactly normalized, and this function does not re-normalize its input).
pub fn raycast_entity_reach(
    origin: rc_physics::Vec3,
    direction: rc_physics::Vec3,
    target_pos: [f64; 3],
    target_kind: EntityKind,
) -> bool;
```
**Reach-check call site**: `origin = eye_position(motion.position)` (M3-B02, reused), `direction` from the attacker's own `(yaw, pitch)` via the shared look-vector formula. A miss (`raycast_entity_reach` returns `false`) rejects the `Attack` as `OutOfReach` — ack-only (no `Attack` packet carries a `sequence` field to acknowledge in the first place, unlike block actions; the client silently receives no combat-effect packets at all on rejection, which is itself the vanilla-matching "nothing happened" signal).

### Mob spawning — a debug-only entry point, first real production wiring of M4-B01's allocator

M4-B01 explicitly deferred wiring `NetworkEntityIdAllocator` into `HardcodedWorld`'s live tick loop ("left to whichever future M4 blueprint first spawns a real mob into `HardcodedWorld`'s live tick loop"). This blueprint is that blueprint, but only to the bounded extent its own acceptance tests need — no real mob-spawning algorithm (MECH-D34) is implemented.

**Cited fix, necessary, not optional: unify the two network-id counters.** `HardcodedWorld` already allocates player network entity ids from its own separate `alloc_network_entity_id` counter (M1-B05), independent of M4-B01's `NetworkEntityIdAllocator`. Both counters start at `1`. Once this blueprint spawns a mob through `NetworkEntityIdAllocator` into the *same* region a real player (allocated through the *other* counter) already occupies, the two numeric spaces can collide (a player and a mob both holding network entity id `1`), which breaks `Interact`'s own target resolution (Context, "Packets") the moment such a collision occurs — silently attacking the wrong entity, or resolving ambiguously. This blueprint therefore replaces `HardcodedWorld`'s player-join call site (wherever `alloc_network_entity_id` is currently called, M1-B05's own join-drain step) with a call through the **same shared `NetworkEntityIdAllocator` instance** this blueprint's own `debug_spawn_mob` uses — the exact migration M4-B01's own Context predicted a future blueprint would perform, cited there by name. `alloc_network_entity_id` itself is deleted; nothing else in the merged codebase calls it.

```rust
impl HardcodedWorld {
    /// Test/diagnostic-only, mirroring `debug_set_held_item`/`debug_spawn`-style precedent
    /// established by every prior M2/M3 blueprint. Allocates `RcEntityId` (`rc_core`,
    /// unmodified) and a network entity id (the now-shared `NetworkEntityIdAllocator`),
    /// spawns a `bevy_ecs` entity carrying `BaseEntity`+`LivingEntity`(+ kind bundle, per
    /// M4-B01) + this blueprint's own `AttributeMap`(defaulted per-kind, Context) +
    /// `CombatRuntimeState::default()`, inserts it into `EntityIndex`/`NetworkEntityIndex`
    /// (both new, below), and returns the allocated `RcEntityId`. `Item` is a valid `kind`
    /// (no `LivingEntity`/`AttributeMap`/`CombatRuntimeState` attached, matching M4-B01's own
    /// composition rule) but is never a combat target in this blueprint's own tests. Returns
    /// both identities — `RcEntityId` for internal/loot-seam use, and the allocated network
    /// entity id for test call sites that need to address the spawned mob over the wire
    /// (`Attack`'s own target field, `debug_deal_damage`/`debug_override_attribute`'s own
    /// `network_entity_id` argument) — so no acceptance test needs to guess or hard-code an
    /// allocation-order-dependent network id.
    pub fn debug_spawn_mob(&mut self, kind: EntityKind, pos: [f64; 3]) -> (rc_core::RcEntityId, i32);

    /// Test/diagnostic-only. Overrides `GlobalDifficulty`'s current value.
    pub fn debug_set_difficulty(&mut self, difficulty: rc_mechanics::combat::Difficulty);

    /// Test/diagnostic-only. Resolves `network_entity_id` via `NetworkEntityIndex` and
    /// applies `apply_damage_pipeline` against it directly with a synthetic, positionless
    /// `Starve`-typed `DamageSource` (chosen because it is the one declared `DamageTypeKind`
    /// that is armor/EPF-bypassing and carries no knockback — the cleanest "just subtract
    /// health" source for test setup), broadcasting the same packets a real hit would. A
    /// no-op (returns `false`) if `network_entity_id` does not resolve to a living target.
    pub fn debug_deal_damage(&mut self, network_entity_id: i32, amount: f32) -> bool;

    /// Test/diagnostic-only. Resolves `network_entity_id` via `NetworkEntityIndex` and
    /// applies `f` to that entity's own `AttributeMap` in place. A no-op if the id does not
    /// resolve or the resolved entity carries no `AttributeMap` (e.g. an `Item`).
    pub fn debug_override_attribute(&mut self, network_entity_id: i32, kind: rc_mechanics::combat::AttributeKind, value: f64);

    /// Test/diagnostic-only. `None` if `network_entity_id` no longer resolves (despawned or
    /// never spawned) — used to assert full despawn after death.
    pub fn debug_query_entity(&self, network_entity_id: i32) -> Option<DebugEntityInfo>;
}

/// Minimal read-only snapshot for test assertions (mirrors `debug_query_block`'s own
/// `DebugBlockInfo`-shaped precedent, M2-B07).
pub struct DebugEntityInfo {
    /// `Some` for a mob/item (Context, `EntityIndex`), `None` for a player (Context,
    /// "Player health" — players are never given an `RcEntityId` by this blueprint).
    pub rc_entity_id: Option<rc_core::RcEntityId>,
    pub health: f32,
    pub is_dead: bool,
}
```

Two new, small resources this entry point (and every combat system) depends on:

```rust
/// `RcEntityId -> bevy_ecs::Entity`, the ECS-coupled lookup M4-B01 needed but never built
/// (its own tracking core is deliberately `bevy_ecs`-free — Context, M4-B01's "pure,
/// bevy_ecs-free" pattern). Lives in `rusty-clanker-server`, not `rc-mechanics`, mirroring
/// that exact split. Maintained by `debug_spawn_mob` (insert) and mob despawn (remove) —
/// this blueprint's own first real maintainer; a future real-spawning blueprint reuses it.
/// Covers non-player entities only (mobs, items) — players are never given an `RcEntityId`
/// by this blueprint (Context, "Player health" — no composition-model migration).
pub struct EntityIndex(std::collections::HashMap<rc_core::RcEntityId, bevy_ecs::entity::Entity>);

/// `i32` network entity id -> `bevy_ecs::Entity`, resolving an `Attack` packet's target
/// field (Context, "Packets") or a `debug_*` accessor's own id argument straight to the ECS
/// entity to mutate — **not** to `RcEntityId`, deliberately: a player has no `RcEntityId`
/// (Context, "Player health"), so keying this index by the identifier both players and mobs
/// already share (their now-unified network entity id, above) is what lets one index and
/// one resolution path serve both `Attack`'s target (which may be a player or a
/// mob) without a second, parallel lookup. Maintained at player-join (insert), mob spawn
/// (insert, `debug_spawn_mob`), and mob despawn (remove) — never removed for a player (a
/// dead player stays resolvable, Context "Death — Player death").
pub struct NetworkEntityIndex(std::collections::HashMap<i32, bevy_ecs::entity::Entity>);
```

### Claims to verify (TEST-D57)

- AttributeInstance's value calculation stage 1: base = base_value, then add every AddValue modifier's amount to base.
- AttributeInstance's value calculation stage 2: result = base; for every AddMultipliedBase modifier, add base * amount to result; multiple AddMultipliedBase modifiers are additive against each other, against the original base, not the running result.
- AttributeInstance's value calculation stage 3: for every AddMultipliedTotal modifier, multiply result by (1 + amount) sequentially, so multiple AddMultipliedTotal modifiers compound with each other since each applies to the already-updated running total.
- AttributeInstance's value calculation stage 4: clamp the result to [min, max].
- Modifier application order within the AddMultipliedBase/AddMultipliedTotal buckets is hash-table slot order keyed by each modifier's own identifier, not registration or insertion order; AddMultipliedTotal's result *= (1 + amount) is mathematically commutative across modifiers, so only IEEE-754 double rounding, not the running-total structure, makes iteration order observable.
- AttackDamage attribute default is 2.0, range [0.0, 2048.0]; the Zombie override is 3.0.
- AttackKnockback attribute default is 0.0, range [0.0, 5.0]; the Zombie override is also 0.0.
- AttackSpeed attribute default is 4.0, range [0.0, 1024.0]; unused by mobs, which have no attack-cooldown charge curve.
- Armor attribute default is 0.0, range [0.0, 30.0]; Zombie's own override is 2.0, Cow and Villager keep the 0.0 default.
- ArmorToughness attribute default is 0.0, range [0.0, 20.0], identical across Zombie, Cow, and Villager.
- KnockbackResistance attribute default is 0.0, range [-2.0, 1.0], identical across Zombie, Cow, and Villager.
- MaxHealth attribute default is 20.0, range [1.0, 1024.0]; Zombie 20.0, Cow 10.0, Villager 20.0.
- SafeFallDistance attribute default is 3.0, range [-1024.0, 1024.0]; Zombie, Cow, and Villager are all 3.0.
- FallDamageMultiplier attribute default is 1.0, range [0.0, 100.0]; Zombie, Cow, and Villager are all 1.0.
- SweepingDamageRatio attribute default is 0.0, range [0.0, 1.0].
- The per-hit damage pipeline runs, in exact order: shield block, freezing/damaged-helmet multipliers, the invulnerability top-up gate, armor absorption, the Resistance status-effect reduction, the enchantment-protection-factor reduction, then absorption hearts.
- The freezing damage multiplier is x5.0.
- The damaged-helmet damage multiplier is x0.75.
- If invulnerable_time > 10, not > 0, and the new damage is <= last_hurt, the hit is fully absorbed and produces no effect.
- If invulnerable_time > 10 and the new damage is > last_hurt, only the delta (damage - last_hurt) is actually applied, last_hurt is updated to the new damage value, and invulnerable_time is NOT reset.
- If invulnerable_time <= 10, the hit applies its full damage, sets last_hurt to that damage, resets invulnerable_time to 20, and sets hurt_time to 10.
- invulnerable_time decrements by 1 every tick.
- The BYPASSES_COOLDOWN damage-type tag is empty in vanilla at version 26.2, so no damage type bypasses the invulnerability window.
- Armor-absorption formula: toughness = 2.0 + armor_toughness/4.0; real_armor = clamp(total_armor - damage/toughness, total_armor*0.2, 20.0); armor_fraction = clamp(real_armor/25.0, 0.0, 1.0); damage = damage * (1.0 - armor_fraction).
- total_armor used in the armor-absorption formula is floor(AttributeMap[Armor]), the armor value is int-floored before use even though the rest of the formula is float.
- Enchantment-protection-factor formula: epf_sum is the uncapped sum of protection_epf, fire_protection_epf, blast_protection_epf, projectile_protection_epf, and feather_falling_epf; real_epf = clamp(epf_sum, 0.0, 20.0); damage = damage * (1.0 - real_epf/25.0).
- Protection's EPF is 1.0 per level, max level 4, max EPF 4.0.
- Fire Protection's, Blast Protection's, and Projectile Protection's EPF are each 2.0 per level, max level 4, max EPF 8.0 each.
- Feather Falling's EPF is 3.0 per level, max level 4, max EPF 12.0.
- Feather Falling's EPF applies specifically to Fall damage, and this still applies even though Fall damage otherwise bypasses the armor-absorption step entirely.
- Mob absorption-hearts path: original = damage; damage = max(damage - absorption, 0.0); absorption -= (original - damage); then if damage != 0.0, health -= damage AND absorption -= damage again, a second subtraction, mob path only.
- Player absorption-hearts path: original = damage; damage = max(damage - absorption, 0.0); absorption -= (original - damage), only one subtraction; then if damage != 0.0, trigger food exhaustion for the damage type and health -= damage.
- In both the mob and player absorption-hearts paths, absorption is clamped to >= 0.0, never negative, after the subtraction(s).
- attack_strength_delay(ticks) = 1.0 / attack_speed_attribute * 20.0, which for the default AttackSpeed of 4.0 yields 5 ticks.
- charge_scale(ticker, offset) = clamp((ticker + offset) / attack_strength_delay, 0.0, 1.0).
- base_damage_scale_factor() = 0.2 + charge_scale(ticker, 0.5)^2 * 0.8.
- attack_strength_ticker increments by 1 every player tick and resets to 0 whenever on_attack() fires, which happens at the start of every successful Attack packet dispatch regardless of whether the swing dealt damage.
- Player melee damage assembly order: base_damage = AttributeMap[AttackDamage]; charge_scale = charge_scale(ticker, 0.5); magic_boost = charge_scale * (enchanted_damage(base_damage, enchants) - base_damage), computed against the UNSCALED base_damage; base_damage is then multiplied by base_damage_scale_factor(), the charge curve is applied after magic_boost is computed; total_damage = base_damage + magic_boost.
- full_strength = charge_scale > 0.9, the exact threshold.
- knockback_attack = is_sprinting && full_strength, which adds +0.5 knockback and triggers the KNOCKBACK sound.
- critical = full_strength && can_critical_attack(...); if critical, base_damage is multiplied by 1.5.
- can_critical_attack = fall_distance > 0.0 && !on_ground && !on_climbable && !in_water && target_is_living && !is_sprinting; this check is deterministic and consumes no RNG.
- sweep = full_strength && !critical && !knockback_attack && on_ground && horizontal_speed_sq < (movement_speed * 2.5)^2 && main_hand_is_sword.
- enchanted_damage(base, enchants) = base + sharpness_bonus(sharpness_level) + smite_bonus(smite_level) if target is undead else 0.0, + bane_bonus(bane_of_arthropods_level) if target is arthropod else 0.0.
- Sharpness's enchant-bonus formula is linear as base + per_level*(level-1) for level >= 1 else 0.0, with base 1.0 and per-level 0.5.
- Smite's enchant-bonus formula is linear as base + per_level*(level-1) for level >= 1 else 0.0, with base 2.5 and per-level 2.5.
- Bane of Arthropods's enchant-bonus formula is linear as base + per_level*(level-1) for level >= 1 else 0.0, with base 2.5 and per-level 2.5.
- sweep_ratio = sweeping_edge_level / (sweeping_edge_level + 1) if sweeping_edge_level > 0, else 0.0.
- sweep_base = 1.0 + sweep_ratio * base_damage, where base_damage is the post-charge, post-critical value.
- A sweep attack hits every LivingEntity within the primary target's own AABB inflated by (1.0, 0.25, 1.0) whose attacker-to-candidate squared distance is < 9.0, excluding the attacker itself, the primary target, entities allied to the attacker, and marker ArmorStands.
- Each sweep-hit nearby target takes enchanted_damage(sweep_base, enchants) * charge_scale.
- On a successful sweep hit, the nearby target receives a knockback impulse directed by the attacker's yaw with a flat magnitude of 0.4 and no enchant term.
- Mob melee damage = AttributeMap[AttackDamage] + enchant_bonus(target); this is fully deterministic with no critical hits, no sweep, no charge curve, and no RNG.
- On a mob attack that deals damage, the target receives a knockback impulse with magnitude get_knockback(attacker, target), directed by the attacker's yaw.
- Knockback impulse #1 fires on the fresh-hit branch of the invulnerability gate only (never the top-up branch, even when it deals damage) whenever the damage type is not no_knockback-tagged; its direction is the normalized XZ-plane vector from the victim's position to the source's position, and its magnitude is a flat 0.4.
- Fall and Starve are both members of the no_knockback set, modeled explicitly as a per-DamageTypeKind flag, so knockback impulse #1 never fires for them; their self-inflicted, positionless source is incidental, not the mechanism.
- Knockback impulse #2 fires strictly after impulse #1 has already mutated velocity; its direction is (sin(attacker_yaw_rad), -cos(attacker_yaw_rad)); its magnitude is get_knockback(attacker, target) + 0.5 if knockback_attack is true, else get_knockback(attacker, target), for players, mobs never have knockback_attack true.
- get_knockback(attacker, target) = (attacker.AttributeMap[AttackKnockback] + knockback_enchant_bonus(level)) / 2.0.
- knockback_enchant_bonus(level) = 1.0 + 1.0*(level-1) if level > 0, else 0.0.
- In the knockback-impulse-application algorithm, power *= (1.0 - target.AttributeMap[KnockbackResistance]) is applied first, and if the resulting power <= 0.0, velocity is returned unchanged before any halving is applied.
- In the knockback-impulse-application algorithm, while the direction vector's squared XZ magnitude is below the double-widened float threshold 9.999999747378752e-6 (not the literal 1e-5), a fallback direction is re-drawn as xd = (rng.next_double() - rng.next_double()) * 0.01 and zd = (rng.next_double() - rng.next_double()) * 0.01, consuming 4 RNG draws per iteration, unbounded, until the sample clears the threshold.
- In the knockback-impulse-application algorithm, the normalized direction scaled by power gives (dx, dz), and new_vx = old_vx/2.0 - dx; new_vz = old_vz/2.0 - dz; new_vy = min(0.4, old_vy/2.0 + power) if on_ground, else old_vy unchanged.
- The knockback impulse function is called exactly twice per hit with two different (power, direction) inputs and its results are never merged into a single call, reproducing vanilla's own velocity-halving-per-call behavior.
- fall_power = fall_distance + 1e-6 - AttributeMap[SafeFallDistance].
- fall_damage = floor(fall_power * damage_modifier * AttributeMap[FallDamageMultiplier]), an integer result, with damage_modifier fixed at vanilla's own default of 1.0.
- Fall damage is skipped entirely, not computed or applied at all, when the resulting fall_damage is <= 0, rather than being applied as a zero-value hit.
- In vanilla, landing on a Bed halves fall distance.
- In vanilla, landing on a Hay Bale sets the fall-damage modifier to 0.2 while passing the fall distance through unscaled, so SafeFallDistance still subtracts from the full distance before the 0.2 multiplies.
- In vanilla, landing on a Slime Block sets the fall-damage modifier to 0.0 in both branches; sneaking (isSuppressingBounce) instead skips the fall-damage call entirely, suppressing the bounce rather than restoring fall damage.
- In vanilla, landing in Powder Snow grants full fall-damage immunity.
- When the target is a player whose GameModeState.instabuild is true, the damage pipeline returns Invulnerable immediately, with no invulnerability-window consumption, no animation, and no packets sent, matching vanilla's own abilities.invulnerable short-circuit.
- Difficulty scaling is applied to incoming player damage only, strictly before any of the pipeline's item-blocking/freeze/helmet-multiplier logic, and only for damage types whose scaling is WhenCausedByLivingNonPlayer when the attacker is a non-player living entity, or Always unconditionally.
- Peaceful difficulty sets incoming player damage to 0.0.
- Easy difficulty sets incoming player damage to min(damage/2.0 + 1.0, damage).
- Hard difficulty sets incoming player damage to (damage * 3.0) / 2.0 — a multiply followed by a divide, not a single multiply by 1.5; the two forms agree everywhere except above f32::MAX/3, where the multiply-then-divide form overflows to infinity while a single ×1.5 stays finite.
- Normal difficulty leaves incoming player damage unchanged.
- If difficulty scaling reduces the damage to exactly 0.0, the whole hit short-circuits with no invulnerability-window consumption.
- Vanilla's own default world difficulty is Normal.
- On mob death, vanilla broadcasts an Entity Event with event_id 3, death animation, to every viewer tracking that mob.
- Health reaching <= 0.0 marks the target as dead and the pipeline outcome is Died rather than Dealt.
- A dying mob's dropped item entities receive vx = rng.next_double()*0.2 - 0.1, vy = the constant 0.2 (not a random draw), vz = rng.next_double()*0.2 - 0.1, consuming exactly two next_double() calls per item, x then z.
- Vanilla villagers drop no items on death.
- A vanilla zombie drops 0 to 2 rotten flesh on death, a uniform random count.
- A vanilla cow drops 1 to 3 beef and, independently, 0 to 2 leather on death.
- The default player entity-interaction range is approximately 3.0 blocks.
- Zombie's and Villager's hitbox dimensions are 0.6 wide x 1.95 tall.
- Cow's hitbox dimensions are 0.9 wide x 1.4 tall.
- Item's hitbox dimensions are 0.25 wide x 0.25 tall.
- Landing a melee attack costs a flat 0.1 exhaustion, EXHAUSTION_ATTACK.
- PlayerAttack damage costs 0.1 exhaustion, scales WhenCausedByLivingNonPlayer, and does not bypass armor.
- MobAttack damage costs 0.1 exhaustion, scales WhenCausedByLivingNonPlayer, and does not bypass armor.
- Fall damage costs 0.0 exhaustion, scales WhenCausedByLivingNonPlayer, and bypasses armor.
- Starve damage costs 0.0 exhaustion, scales WhenCausedByLivingNonPlayer, and bypasses armor.
- A joining player's food stats start at food_level 20, range 0..=20, saturation 5.0, vanilla's own join default, and exhaustion 0.0 with a ceiling of 40.0.
- add_exhaustion adds the given amount to exhaustion, clamped to a ceiling of 40.0.
- Exhaustion-decay branch: if exhaustion > 4.0, subtract 4.0 from exhaustion, then reduce saturation by 1.0, floored at 0.0, if saturation > 0.0, else reduce food_level by 1, floored at 0, if difficulty is not Peaceful.
- Fast, saturation-driven, natural regeneration: if saturation > 0.0 and the entity is hurt and food_level >= 20, a regen timer increments each tick and at 10 ticks heals health by min(saturation, 6.0)/6.0, capped at max_health, spends that same amount as exhaustion, and resets the timer.
- Slow, food-driven, natural regeneration: if food_level >= 18 and the entity is hurt, and the fast-regen branch did not fire, a regen timer increments each tick and at 80 ticks heals 1.0 health, capped at max_health, adds 6.0 exhaustion, and resets the timer.
- Starvation: if food_level <= 0, and neither regen branch fired, a regen timer increments each tick and at 80 ticks applies 1.0 Starve damage if health > 10.0, or difficulty is Hard, or health > 1.0 and difficulty is Normal; the timer then resets regardless.
- is_hurt(health, max_health) is defined as health < max_health && health > 0.0.
- The Interact packet's payload has no interaction_type discriminator: Attack is its own server-bound packet, ID 0x01, a single entity_id (VarInt) field; Interact is server-bound ID 0x1A with fields, in wire order, entity_id (VarInt), hand (VarInt: 0=main hand, 1=off hand), location (LpVec3), using_secondary_action (bool) — a flat, unconditional four-field layout with no conditional field groups.
- The Set Health packet is client-bound with packet ID 0x68 and fields, in wire order: health (f32), food (VarInt), saturation (f32).
- The Update Attributes packet is client-bound with packet ID 0x83 and fields, in wire order: entity_id (VarInt), then a VarInt-count-prefixed sequence of entries each shaped attribute_id (VarInt), base_value (f64), modifier_count (VarInt).
- The Damage Event packet is client-bound with packet ID 0x19 and fields, in wire order: entity_id (VarInt), source_type_id (VarInt), source_cause_id (VarInt, network entity id + 1, 0 = none), source_direct_id (VarInt, network entity id + 1, 0 = none), has_source_position (bool), then source_x/source_y/source_z (each f64) only if has_source_position is true.
- The Entity Event packet is client-bound with packet ID 0x22 and fields, in wire order: entity_id as a plain Int, not a VarInt, unlike every other entity-id field in this packet set, then event_id (u8); event_id 2 is KINETIC_HIT, a weapon-hit-sound cue this blueprint never constructs, and event_id 3 is the death-animation event, the only value this blueprint ever sends.
- The Player Combat Kill packet is client-bound with packet ID 0x44 and fields, in wire order: player_id (VarInt), then message. Vanilla encodes message as an NBT-backed chat Component, not a length-prefixed string; this blueprint deliberately sends a plain length-prefixed string as a documented stand-in, a real, client-visible wire-shape divergence from vanilla, not a parity bug to fix.
- After a hit applies both knockback impulses, only the single final post-both-impulses velocity is broadcast via one Set Entity Velocity packet, matching vanilla's own single velocity-update-packet-per-hit behavior even though the server-side math applies two separate impulses.
- Health is broadcast via the entity metadata field at index 9, Set Entity Data, to viewers other than the entity's own owning player, who instead receive Set Health directly.

## Deliverables

### `crates/mechanics/src/entity/living.rs` (modify — add fields, declaration order matters only for the pre-existing `#[net_metadata(...)]` fields, which are unchanged and untouched)

```rust
// Added at the end of the existing field list (Context, "Damage pipeline" step 7 and
// "Death"). None carry #[net_metadata(...)] (Context, "Player health" — no client-render
// need at M4 scope); `is_dead` carries neither #[nbt(...)] nor #[net_metadata(...)]
// (transient runtime-only, defaulted via `Default::default()` on load per M4-B01's own
// `EntityNbtFields` rule 2).
#[nbt(name = "AbsorptionAmount")]
pub absorption: f32,
#[nbt(name = "HurtTime")]
pub hurt_time: i16,
#[nbt(name = "DeathTime")]
pub death_time: i16,
pub is_dead: bool,
```

### `crates/mechanics/src/entity/ids.rs` (modify — no new resource here, Context already places `EntityIndex`/`NetworkEntityIndex` in `rusty-clanker-server`; this file gains nothing new. Listed for completeness of the "files touched" set — no diff.)

### `crates/mechanics/src/combat/mod.rs`

```rust
//! Combat/damage pipeline (MECH-D40/D43-D46, ARCH-D15 Stage-6b content). Pure, ECS-free
//! math + plain data types; `rusty-clanker-server` supplies the ECS/packet adapter layer,
//! mirroring M3-B01's `BlockWorldAccess`/M4-B01's tracking-core split exactly.

pub mod attributes;
pub mod damage;
pub mod melee;
pub mod knockback;
pub mod fall;
pub mod death;
pub mod food;

pub use attributes::{AttributeInstance, AttributeKind, AttributeMap, AttributeModifier, ModifierOperation, default_attributes_for};
pub use damage::{DamageOutcome, DamageScaling, DamageSource, DamageTypeKind, Difficulty, EnchantLevels, apply_damage_pipeline, difficulty_scale_incoming};
pub use melee::{CombatRuntimeState, MeleeAssemblyResult, assemble_mob_melee_damage, assemble_player_melee_damage, attack_cooldown_charge_scale, can_critical_attack};
pub use knockback::{apply_knockback_impulse, get_knockback};
pub use fall::calculate_fall_damage;
pub use death::{EntityLootProvider, FixedTierTwoLoot, PendingMeleeAttack};
pub use food::FoodStats;
```

### `crates/mechanics/src/combat/attributes.rs`

```rust
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AttributeKind {
    AttackDamage, AttackKnockback, AttackSpeed, Armor, ArmorToughness,
    KnockbackResistance, MaxHealth, SafeFallDistance, FallDamageMultiplier, SweepingDamageRatio,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ModifierOperation { AddValue, AddMultipliedBase, AddMultipliedTotal }

#[derive(Clone, Debug, PartialEq)]
pub struct AttributeModifier {
    pub id: u64,
    pub amount: f64,
    pub operation: ModifierOperation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttributeInstance {
    pub base: f64,
    pub min: f64,
    pub max: f64,
    /// Iterated in `Vec` insertion (push) order per operation bucket, not vanilla's own
    /// hash-table slot order — Context's own "Attribute system" section states the exact
    /// bounded condition (at most one modifier per operation per attribute) this deviation
    /// holds under, and the runtime check (`AttributeMap::add_modifier`, below) that guards
    /// it.
    pub modifiers: Vec<AttributeModifier>,
}

impl AttributeInstance {
    pub fn constant(value: f64, min: f64, max: f64) -> Self;
    /// Exact vanilla 3-stage calc (Context) + clamp, folding `self.modifiers` in `Vec`
    /// insertion order per operation bucket (Context's own cited, bounded ordering
    /// exception) rather than vanilla's own hash-table slot order. Pure, no caching.
    pub fn compute_value(&self) -> f64;
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttributeMap(HashMap<AttributeKind, AttributeInstance>);

impl AttributeMap {
    pub fn get(&self, kind: AttributeKind) -> f64;
    /// Debug/test-only mutator (mirrors the project's own `debug_*` precedent) — replaces
    /// `kind`'s own `base` in place, keeping `min`/`max`/`modifiers` unchanged.
    pub fn set_base(&mut self, kind: AttributeKind, value: f64);
    /// Appends `modifier` to `kind`'s own modifier `Vec` (Context's own bounded
    /// `Vec`-insertion-order exception). Enforces that exception's precondition with a
    /// debug-assertion-style runtime check: before appending, scans `kind`'s existing
    /// modifiers for one already sharing `modifier.operation`, and panics immediately,
    /// naming `kind` and the operation, if it finds one — two same-operation modifiers on
    /// one attribute is exactly the case where `Vec` insertion order could disagree with
    /// vanilla's real hash-slot order, so this call is the single point that keeps the
    /// exception from being silently violated.
    pub fn add_modifier(&mut self, kind: AttributeKind, modifier: AttributeModifier);
}

/// Context's own per-`EntityKind` default table. `EntityKind::Item` returns an empty map
/// (never consulted — Item is never a `LivingEntity`).
pub fn default_attributes_for(kind: crate::entity::EntityKind) -> AttributeMap;
```

### `crates/mechanics/src/combat/damage.rs`

```rust
use super::attributes::AttributeMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DamageTypeKind { PlayerAttack, MobAttack, Fall, Starve }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DamageScaling { Never, WhenCausedByLivingNonPlayer, Always }

impl DamageTypeKind {
    /// Context's own table, verbatim.
    pub fn exhaustion(self) -> f32;
    pub fn scaling(self) -> DamageScaling;
    pub fn bypasses_armor(self) -> bool;
    /// Context's own table, verbatim — gates knockback impulse #1 (Context, "Knockback").
    pub fn no_knockback(self) -> bool;
}

#[derive(Clone, Debug, PartialEq)]
pub struct DamageSource {
    pub kind: DamageTypeKind,
    pub causing_entity: Option<rc_core::RcEntityId>,
    /// XZ-plane position, used to compute impulse #1's direction on hits whose
    /// `DamageTypeKind::no_knockback()` is `false` (Context, "Knockback") — impulse #1's
    /// firing gate is that flag, not the presence of this field. `None` for `Fall`/`Starve`
    /// (self-inflicted, `no_knockback = true`, so this field is never consulted for them).
    pub source_position: Option<[f64; 2]>,
    pub causing_entity_is_living_non_player: bool,
}

/// Every field defaults `0`; see Context, "Enchantment level source."
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct EnchantLevels {
    pub sharpness: u8,
    pub smite: u8,
    pub bane_of_arthropods: u8,
    pub knockback: u8,
    pub sweeping_edge: u8,
    pub protection: u8,
    pub fire_protection: u8,
    pub blast_protection: u8,
    pub projectile_protection: u8,
    pub feather_falling: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DamageOutcome {
    Invulnerable,
    NoOp,
    /// Damage was dealt but did not kill; carries the final `f32` health-delta actually
    /// subtracted (post-absorption), for exhaustion/stat bookkeeping call sites.
    Dealt { health_delta: f32 },
    Died,
}

impl DamageOutcome {
    /// `true` for `Dealt`/`Died`, `false` for `Invulnerable`/`NoOp` — the exact condition
    /// vanilla's own `tookFullDamage` check gates knockback/sound/animation on (§3.1). Every
    /// Context pseudocode block's own informal `.dealt`/`outcome.dealt` shorthand refers to
    /// this method.
    pub fn dealt_damage(&self) -> bool;
}

/// The target's own mutable health-bearing state — implemented once per call-site shape
/// (`PlayerCombatState`, `LivingEntity`) by the caller; this function reads/writes through
/// plain `&mut` numeric fields, never a component type, so it has zero ECS coupling.
pub struct DamageTarget<'a> {
    pub health: &'a mut f32,
    pub max_health: f64,
    pub absorption: &'a mut f32,
    pub invulnerable_time: &'a mut i32,
    pub last_hurt: &'a mut f32,
    pub is_player: bool,
    pub is_creative: bool,
    pub attributes: &'a AttributeMap,
}

/// The full pipeline (Context, "Damage pipeline"), steps 1-7 in exact order, the
/// invulnerability gate (Context "Damage invulnerability gate") first. `enchants` are the
/// **attacker's** weapon enchant levels (armor-effectiveness/EPF are the **target's** own
/// worn-armor enchants — Context's own bounded-zero stub means both are always `0` in
/// production regardless of whose enchants they represent; kept as two separate parameters,
/// `attacker_enchants`/`target_epf_enchants`, so a future items blueprint wires each to its
/// own correct side without a signature change).
pub fn apply_damage_pipeline(
    target: DamageTarget<'_>,
    source: &DamageSource,
    raw_damage: f32,
    attacker_enchants: EnchantLevels,
    target_epf_enchants: EnchantLevels,
) -> DamageOutcome;

/// MECH-D45, exact (Context step 4).
pub fn armor_effective_damage(damage: f32, total_armor: f64, armor_toughness: f64) -> f32;

/// §3.6, exact (Context step 6). `epf_sum` is the caller's own sum of the five
/// `*_protection_epf` functions below.
pub fn epf_reduction(damage: f32, epf_sum: f32) -> f32;

pub fn protection_epf(level: u8) -> f32;         // 1.0 base, 1.0/level
pub fn fire_protection_epf(level: u8) -> f32;    // 2.0 base, 2.0/level
pub fn blast_protection_epf(level: u8) -> f32;   // 2.0 base, 2.0/level
pub fn projectile_protection_epf(level: u8) -> f32; // 2.0 base, 2.0/level
pub fn feather_falling_epf(level: u8) -> f32;    // 3.0 base, 3.0/level

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Difficulty { Peaceful, Easy, #[default] Normal, Hard }

pub struct GlobalDifficulty(pub Difficulty);

/// §3.8, exact (Context). Returns the possibly-scaled damage; `0.0` means "short-circuit,
/// no hit at all" per §3.8's own text.
pub fn difficulty_scale_incoming(damage: f32, difficulty: Difficulty, source: &DamageSource) -> f32;
```

### `crates/mechanics/src/combat/melee.rs`

```rust
use super::{attributes::AttributeMap, damage::EnchantLevels};

#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct CombatRuntimeState {
    pub invulnerable_time: i32,
    pub last_hurt: f32,
    pub attack_strength_ticker: u32,   // unused by mobs (§3.13) — always 0 there
}

/// §3.9a, exact (Context).
pub fn attack_cooldown_charge_scale(ticker: u32, attack_speed: f64, offset: f32) -> f32;

/// §3.9b, exact (Context) — deterministic, no RNG.
pub fn can_critical_attack(fall_distance: f64, on_ground: bool, in_water: bool, on_climbable: bool, target_is_living: bool, is_sprinting: bool) -> bool;

pub fn sharpness_bonus(level: u8) -> f32;   // 1.0 base, 0.5/level
pub fn smite_bonus(level: u8) -> f32;       // 2.5 base, 2.5/level
pub fn bane_bonus(level: u8) -> f32;        // 2.5 base, 2.5/level
pub fn sweeping_edge_ratio(level: u8) -> f32; // level/(level+1), 0.0 at level 0

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MeleeAssemblyResult {
    pub total_damage: f32,
    pub is_critical: bool,
    pub is_sweep: bool,
    pub extra_knockback_bonus: f32,   // 0.5 if knockback_attack, else 0.0
}

/// §3.9, exact order (Context, "Player melee assembly"). `is_undead`/`is_arthropod`
/// classify the primary target for Smite/Bane; `horizontal_speed_sq` is the attacker's own
/// last-known-movement horizontal speed squared (§3.9c's own "known movement," not raw
/// velocity — `PlayerMotion.velocity`'s XZ magnitude squared is this blueprint's own
/// sufficient stand-in, no separate "known movement" tracking exists).
pub fn assemble_player_melee_damage(
    attributes: &AttributeMap,
    ticker: u32,
    fall_distance: f64,
    on_ground: bool,
    in_water: bool,
    on_climbable: bool,
    target_is_living: bool,
    is_sprinting: bool,
    horizontal_speed_sq: f64,
    is_undead: bool,
    is_arthropod: bool,
    enchants: EnchantLevels,
) -> MeleeAssemblyResult;

/// §3.13, exact (Context, "Mob melee attacks") — fully deterministic.
pub fn assemble_mob_melee_damage(attacker_attributes: &AttributeMap, enchants: EnchantLevels) -> f32;
```

### `crates/mechanics/src/combat/knockback.rs`

```rust
use rc_physics::Vec3;
use crate::random::RcRandom;

/// (attacker.AttackKnockback + enchant bonus) / 2.0 — Context, exact.
pub fn get_knockback(attacker_attack_knockback: f64, knockback_enchant_level: u8) -> f64;

/// One impulse application (Context, "Knockback" — called twice per hit, never merged).
/// `rng` is only consulted on the rare degenerate-direction branch.
pub fn apply_knockback_impulse(
    velocity: Vec3,
    power: f64,
    dir_xz: (f64, f64),
    on_ground: bool,
    knockback_resistance: f64,
    rng: &mut RcRandom,
) -> Vec3;
```

### `crates/mechanics/src/combat/fall.rs`

```rust
/// §3.15, exact (Context, "Fall damage"). Returns the floored `i32` damage (never negative
/// — caller discards non-positive results without applying any damage).
pub fn calculate_fall_damage(fall_distance: f64, damage_modifier: f32, safe_fall_distance: f64, fall_damage_multiplier: f64) -> i32;
```

### `crates/mechanics/src/combat/death.rs`

```rust
use crate::{entity::EntityKind, entity::kinds::ItemStackRecord, random::RcRandom};

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq)]
pub struct PendingMeleeAttack {
    pub target: rc_core::RcEntityId,
}

pub trait EntityLootProvider: Send + Sync {
    fn roll_death_loot(&self, kind: EntityKind, rng: &mut RcRandom) -> Vec<ItemStackRecord>;
}

pub struct FixedTierTwoLoot;

impl EntityLootProvider for FixedTierTwoLoot {
    fn roll_death_loot(&self, kind: EntityKind, rng: &mut RcRandom) -> Vec<ItemStackRecord>;
}
```

### `crates/mechanics/src/combat/food.rs`

```rust
#[derive(Component, Clone, Debug, PartialEq)]
pub struct FoodStats {
    pub food_level: i32,
    pub saturation: f32,
    pub exhaustion: f32,
    pub regen_tick_timer: u32,
}

impl FoodStats {
    pub fn new_at_join() -> Self;   // food_level: 20, saturation: 5.0, exhaustion: 0.0, regen_tick_timer: 0
    pub fn add_exhaustion(&mut self, amount: f32);
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FoodTickOutcome { NoChange, DecayedOnly, Healed { amount: f32 }, Starved }

/// §3.18, exact (Context). `difficulty`/`is_creative` gate as Context describes.
pub fn tick_food(
    stats: &mut FoodStats,
    health: f32,
    max_health: f32,
    difficulty: crate::combat::damage::Difficulty,
    is_creative: bool,
) -> FoodTickOutcome;
```

### `crates/physics/src/motion.rs` (modify — one cited additive field/branch, for the future Stage-6b mob-physics consumer this blueprint does not itself register)

```rust
// LivingMotionState (MODIFY): add one field, mirroring PlayerMotion's own identical
// addition (Context, "Fall damage" — kept symmetric across both fall-tracking call sites
// even though this blueprint's own production systems only ever read the PlayerMotion
// copy, Context "Mob fall damage — deferred").
pub landed_fall_distance: Option<f64>,
```
`step_living_entity_tick`'s own body (Context step 8, M3-B02): immediately before its existing `if new_on_ground { fall_distance = 0.0 }` line, add `if fall_distance > 0.0 { landed_fall_distance = Some(fall_distance); } else { landed_fall_distance = None; }` — additive, does not change any existing golden-vector test's asserted `position`/`velocity`/`on_ground` fields (M3-B02's own three acceptance tests never assert on `fall_distance`/`landed_fall_distance` at all, so this addition changes zero existing assertions).

### `crates/server/src/play/combat_packets.rs` (new)

Packet structs per the Context "Packets" table (`Attack`, `Interact`, `SetHealth`, `UpdateAttributes`, `DamageEvent`, `EntityEvent`, `PlayerCombatKill`), `#[derive(RcPacket)]` for `Attack`/`SetHealth`/`EntityEvent`/`PlayerCombatKill` (fixed shape, no conditional fields, no custom codecs), hand-implemented `RcPacket` for `Interact` (its bespoke `LpVec3` `location` codec) and `UpdateAttributes` (its nested variable-count list) per Context — neither packet has a conditional field shape. Exact field types/order as the Context table.

### `crates/server/src/play/combat.rs` (new)

```rust
use rc_mechanics::combat::*;

/// The bounded stub (Context, "Enchantment level source") — always `EnchantLevels::default()`.
pub fn resolve_enchant_levels(item: &crate::play::mining::HeldItemStub) -> EnchantLevels;

/// Context, "Reach and angle validation" — thin wrapper binding `PlayerMotion`'s real
/// position/rotation into `raycast_entity_reach`'s plain-`Vec3` signature.
pub fn entity_reach_check(motion: &crate::play::movement::PlayerMotion, target_pos: [f64; 3], target_kind: rc_mechanics::entity::EntityKind) -> bool;

/// The manual Stage-3-equivalent combat step (Context, "Tick-pipeline placement"): drains
/// queued `Attack` actions (stable-sorted by ascending `network_entity_id`,
/// MECH-D4's own determinism rule, unchanged convention), resolves the target via
/// `NetworkEntityIndex`, reach/angle-validates, dispatches to
/// `assemble_player_melee_damage` + `apply_damage_pipeline` + both knockback impulses,
/// broadcasts `Damage Event`/`Set Entity Velocity`/`Set Entity Data`/`Entity Event` per
/// outcome, then runs the per-player fall-damage consumption (`landed_fall_distance.take()`)
/// and `tick_food` passes. Skips any player whose `PlayerCombatState.is_dead` is `true`
/// (Context, "Death").
pub fn apply_combat_step(world: &mut HardcodedWorld);

/// The Stage-6b `EntityPhysicsIntegration` system (Context, "Mob melee attacks"):
/// registered via `RcExecutorBuilder`'s ordinary `register_system` path (not the
/// no-apply-deferred Stage-6a path M4-B01 reused for `EntityAiSelection`), consuming and
/// removing every `PendingMeleeAttack`, applying `assemble_mob_melee_damage` +
/// `apply_damage_pipeline` + one knockback impulse against its `target`. Also runs death
/// handling (Context, "Death — Mob death") for every entity whose `LivingEntity.is_dead`
/// became true this tick, and decrements every combat-capable entity's own
/// `CombatRuntimeState.invulnerable_time` (floored at `0`).
pub fn register_mob_combat_system(builder: &mut rc_scheduler::RcExecutorBuilder) -> Result<(), rc_scheduler::ExecutorBuildError>;
```

### `crates/server/src/play/movement.rs`, `mining.rs`, `world.rs`, `connection.rs`, `mod.rs` (modify)

- `movement.rs`: `PlayerMotion` gains `landed_fall_distance: Option<f64>` (Context); its two existing reset-on-landing sites each gain the one capture line (Context, exact).
- `mining.rs`/`movement.rs`'s own per-player packet-loop entry points each gain one `if player_combat_state.is_dead { continue; }` guard (Context, "Death — Player death").
- `world.rs`: `HardcodedWorld` gains `EntityIndex`, `NetworkEntityIndex`, `AmbientCombatRandom`, `GlobalDifficulty` (all `Default`-initialized at construction except `AmbientCombatRandom`'s fixed seed, Context), `debug_spawn_mob`, `debug_set_difficulty`; the join-drain step additionally inserts `PlayerCombatState::default()`-equivalent (health = `MaxHealth` default `20.0`, everything else zeroed) and `FoodStats::new_at_join()` and `CombatRuntimeState::default()` onto the newly-spawned player entity, alongside every prior blueprint's own already-established insertions. Tick loop gains `apply_combat_step` (Context, insertion point) and `register_mob_combat_system`'s own one-time builder registration (composition-root wiring, alongside every other `register_stageN`-style call this project's own composition root already makes).
- `connection.rs`: dispatch table gains two new inbound-packet-id match arms — `Attack`, routed to a per-connection queue mirroring `queue_block_action`/`queue_movement_packet`'s own established shape exactly, and `Interact`, decoded and silently discarded per Context's own out-of-scope framing.
- `mod.rs`: `pub mod combat; pub mod combat_packets;`.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46, restated exactly per every prior blueprint's own identical framing): the test changeset is every file below plus every `crates/mechanics/src/combat/*.rs`/`crates/server/src/play/combat*.rs` file with every function body from Deliverables replaced with `todo!()` (fields/derives/doc comments unchanged), plus the cited `living.rs`/`motion.rs`/`movement.rs`/`world.rs`/`connection.rs`/`mining.rs`/`mod.rs` edits present but `todo!()`-bodied where a new function is introduced. The implementation changeset fills in real bodies only — it must not edit any test file, weaken any assertion, or change any golden value this section fixes.

### `crates/mechanics/tests/combat_damage_pipeline.rs`

1. `armor_toughness_golden_table` — hand-computed matrix: `(damage, total_armor, toughness)` in `{(10.0,0,0), (10.0,10,0), (10.0,20,0), (10.0,15,8), (100.0,20,20)}`, asserting `armor_effective_damage` against independently hand-derived expected `f32` values (shown in the test's own comments per the exact formula in Context step 4), tolerance `1e-6`.
2. `epf_reduction_golden_table` — `epf_sum` in `{0.0, 4.0 (Protection IV alone), 16.0 (4x Protection IV), 24.0 (Protection IV + Blast Protection IV on the matching type, exceeds the cap)}` against `damage=20.0`, asserting the `1.0 - clamp(epf_sum,0,20)/25.0` formula exactly, including the saturation case (`epf_sum=24` and `epf_sum=20` must produce identical output).
3. `invulnerability_top_up_sequence` — a `DamageTarget` with `health=20.0, invulnerable_time=0, last_hurt=0.0`: hit 1 (`raw_damage=6.0`) deals full damage, sets `invulnerable_time=20, last_hurt=6.0`; hit 2 at the same simulated tick (`invulnerable_time` still `20 > 10`) with `raw_damage=4.0` (`<= last_hurt`) returns `NoOp`, health unchanged; hit 3, `invulnerable_time` manually decremented to `10` (boundary, not `> 10`), `raw_damage=4.0` deals **full** fresh damage (not a delta) — asserts the `> 10` not `> 0` boundary precisely; hit 4, `invulnerable_time` at `15` (`> 10`, top-up branch), `raw_damage=9.0` (`> last_hurt=4.0` from hit 3's own reset) deals exactly the **delta** `9.0 - 4.0 = 5.0`, not the full `9.0`.
4. `absorption_asymmetry_mob_vs_player` — mob path: `health=20.0, absorption=3.0`, `raw_damage=5.0` (post-armor/EPF, both `0`) — assert final `absorption == 0.0` (not `-3.0` clamped, and not `2.0` — the double-subtraction: `3.0` absorbed then `2.0` more from the health-damage amount, clamped at `0`) and `health == 18.0` (`20 - 2.0`, only the un-absorbed remainder). Player path: identical inputs via `PlayerCombatState`-shaped `DamageTarget` (`is_player: true`) — assert final `absorption == 0.0` (single subtraction: `3.0` absorbed, nothing more) and `health == 18.0` (same health result, different absorption bookkeeping — the documented asymmetry, §3.7).
5. `epf_and_armor_bypass_are_independent_for_fall_damage` — `Fall`-typed source (`bypasses_armor=true`), `total_armor=20.0` (should have zero effect), `feather_falling_epf(4)=12.0` fed as `target_epf_enchants.feather_falling=4` — assert the output damage reflects the EPF reduction but **not** the armor reduction (armor step entirely skipped for this damage type, EPF step still runs) — the exact hazard #7 regression test.
6. `difficulty_scaling_three_branches` — `damage=10.0`, `MobAttack` source (`causing_entity_is_living_non_player: true`): `Peaceful -> 0.0`; `Easy -> 6.0` (`min(10/2+1,10)`); `Hard -> 15.0`; `Normal -> 10.0` unchanged. A `PlayerAttack` source with `Hard` difficulty is asserted **unchanged** (`causing_entity_is_living_non_player: false` — scaling never applies to player-vs-player).
7. `creative_player_is_fully_invulnerable` — `DamageTarget{is_creative: true, ..}`, any nonzero `raw_damage` — assert `DamageOutcome::Invulnerable`, health/absorption/invulnerable_time all unchanged.

### `crates/mechanics/tests/combat_melee_assembly.rs`

1. `attack_cooldown_charge_curve_golden_vectors` — `attack_speed=4.0` (`delay=5.0` ticks): `attack_cooldown_charge_scale(ticker, 4.0, 0.5)` for `ticker in [0,1,2,3,4,5,10]`, asserting exact `clamp((t+0.5)/5.0, 0, 1)` values (`0.1, 0.3, 0.5, 0.7, 0.9, 1.0, 1.0`) to `1e-9`, and `base_damage_scale_factor = 0.2 + charge^2*0.8` derived from each (`0.208, 0.272, 0.4, 0.592, 0.848, 1.0, 1.0`) to `1e-6`.
2. `critical_hit_requires_all_five_conditions` — five sub-cases, each flipping exactly one of `fall_distance>0`/`!on_ground`/`!in_water`/`target_is_living`/`!is_sprinting` to false while the other four hold true — assert `can_critical_attack` is `false` for each, and `true` when all five hold.
3. `sharpness_smite_bane_golden_table` — `sharpness_bonus(0..=5)` against hand-computed `[0.0, 1.0, 1.5, 2.0, 2.5, 3.0]`; `smite_bonus`/`bane_bonus(0..=3)` against `[0.0, 2.5, 5.0, 7.5]` each.
4. `sweeping_edge_ratio_golden_table` — `sweeping_edge_ratio(0..=3)` against `[0.0, 0.5, 0.6666667, 0.75]`, tolerance `1e-6`.
5. `player_melee_full_assembly_undead_target_with_smite` — full-charge (`ticker=5`), `AttributeMap[AttackDamage]=7.0` (a diamond-sword-equivalent base), `is_undead=true`, `enchants.smite=2`, on-ground/not-sprinting/not-critical-eligible (`fall_distance=0.0`) — hand-derive `magic_boost = 1.0 * (7.0 + smite_bonus(2) - 7.0) = 5.0` (Smite II = `2.5+2.5=5.0`), `base_damage = 7.0 * 1.0 = 7.0` (full charge), no crit, `total_damage = 12.0` — assert `assemble_player_melee_damage`'s `total_damage == 12.0` (tolerance `1e-5`), `is_critical == false`.
6. `mob_melee_deterministic_no_rng_no_charge` — `assemble_mob_melee_damage` called twice with identical inputs (`attacker_attributes[AttackDamage]=3.0`, `enchants=default`) — assert bit-identical output both times and equal to `3.0` exactly (no floating variance from any hidden RNG/charge term).

### `crates/mechanics/tests/combat_knockback.rs`

1. `zero_power_impulse_is_a_true_no_op_not_a_silent_halving` — starting `velocity=Vec3::ZERO`, `on_ground=true`, `knockback_resistance=0.0`: impulse #1 (`power=0.4`, `dir_xz=(1.0,0.0)`, source-relative) — assert `velocity.x == -0.4` after impulse #1 (dir normalized `(1,0)`, `new_vx = 0/2 - 1*0.4 = -0.4`). Then impulse #2 with `power=get_knockback(0.0,0)=0.0` (bare attributes, zero enchant, no sprint bonus) — Context's own algorithm's `power <= 0.0: return velocity unchanged` line fires *before* the `old/2.0` halving term is ever evaluated, so the resulting velocity after impulse #2 must equal impulse #1's own result **exactly unchanged** (`x == -0.4` still, not `-0.2`) — this is the regression test for a reimplementation that moves the early return after the halving lines, which would silently halve velocity on every zero-power hit (a flat-`ATTACK_KNOCKBACK`/no-enchant swing, i.e. every unenchanted weapon).
2. `two_impulse_sequence_with_nonzero_second_impulse_is_not_equivalent_to_one_merged_call` — impulse #1 `power=0.4, dir_xz=(1.0,0.0)` then impulse #2 `power=0.5, dir_xz=(0.0,1.0)` (perpendicular, simulating attacker-yaw != source-direction): hand-derive `v1 = (-0.4, 0, 0)`; impulse #2: `new_vx = -0.4/2 - 0 = -0.2`, `new_vz = 0/2 - 1*0.5 = -0.5` — assert final `velocity == Vec3::new(-0.2, ?, -0.5)` (Y per the `on_ground` branch, `min(0.4, 0/2+0.5)=0.4`) — and explicitly assert this **differs** from a single hypothetical merged-impulse calculation (`(1.0,0.0)*0.4 + (0.0,1.0)*0.5` applied once), proving the two-call halving is load-bearing, not simplifiable.
3. `knockback_resistance_scales_power_before_the_early_return` — `knockback_resistance=1.0` (full resistance), any nonzero input `power` — assert velocity unchanged (power becomes `<= 0` after the `*(1.0 - resistance)` scale, early-return path).
4. `degenerate_direction_uses_rng_fallback_deterministically` — `dir_xz=(0.0,0.0)` (exactly degenerate), `rng = RcRandom::new(42)` — assert the function consumes a nonzero multiple of 4 `next_double()` calls (the while loop keeps re-drawing until the sample clears `KNOCKBACK_DEGENERATE_THRESHOLD`, so the exact count depends on the RNG stream; checked by running the identical while-loop redraw against a second, independently-advanced `RcRandom::new(42)` instance and asserting both the resulting call count and final `(xd, zd)` match) and produces a non-`NaN`, finite result.
5. `get_knockback_formula` — `get_knockback(0.0, 0) == 0.0`; `get_knockback(0.0, 2) == 1.5` (`(0 + (1.0+1.0*(2-1)))/2.0 = 2.0/2.0`).

### `crates/mechanics/tests/combat_fall_food.rs`

1. `fall_damage_golden_table` — `(fall_distance, safe_fall_distance, multiplier, expected)`: `(3.0, 3.0, 1.0, 0)` (at the safe threshold, `floor(1e-6*1)=0`); `(10.0, 3.0, 1.0, 7)` (`floor(7.000001)`); `(5.5, 3.0, 1.0, 2)` (`floor(2.500001)`); `(10.0, 3.0, 0.0, 0)` (multiplier zeroes it).
2. `food_tick_fast_regen_branch` — `FoodStats{food_level:20, saturation:6.0, exhaustion:0.0, regen_tick_timer:9}`, `health=15.0, max_health=20.0`, `Normal`, not creative — one `tick_food` call reaches `regen_tick_timer==10` internally and fires: assert `health == 16.0` (`15 + min(6,6)/6.0`), `saturation` reduced by the `add_exhaustion(6.0)` call's own downstream first-branch interaction is **not** re-triggered same-tick (single `tick_food` call = one pass only, not a fixed-point loop) — assert `FoodTickOutcome::Healed{amount: 1.0}`.
3. `food_tick_starvation_gated_by_difficulty` — `food_level=0, regen_tick_timer=79`, `health=10.0` (boundary — `Normal` requires `health>1.0`, `10.0>1.0` true) — `Normal` difficulty starves (`FoodTickOutcome::Starved`); identical setup with `health=1.0` on `Normal` does **not** starve (`1.0>1.0` false, and `1.0>10.0` false) — `FoodTickOutcome::NoChange` for the starve branch specifically (timer still resets).
4. `food_tick_skips_entirely_for_creative` — any decaying/starving input, `is_creative=true` — assert `FoodTickOutcome::NoChange` and every field of `stats` unchanged.

### `crates/mechanics/tests/combat_death_loot.rs`

1. `fixed_tier_two_loot_zombie_bounded_range` — `FixedTierTwoLoot.roll_death_loot(EntityKind::Zombie, &mut RcRandom::new(1))`, run 200 times with independent seeds `1..=200` — assert every result has `0..=2` rotten-flesh-id entries (never more, never a different item id).
2. `fixed_tier_two_loot_villager_is_empty` — `roll_death_loot(EntityKind::Villager, ..)` returns an empty `Vec` for any seed.
3. `fixed_tier_two_loot_is_deterministic_given_seed` — two calls with two freshly-constructed `RcRandom::new(7)` instances produce bit-identical results.

### `crates/server/tests/play_combat_melee_flow.rs`

1. `player_attacks_zombie_full_pipeline_packet_sequence` — `HardcodedWorld::new()`, connection `A` (spawns, joins, network entity id `A_net_id` read from its own already-established join-flow response, M1-B05 precedent), `(_, zombie_net_id) = world.debug_spawn_mob(EntityKind::Zombie, [3.0, -60.0, 0.0])` — within `A`'s own spawn eye's reach and look direction (`A`'s fixed test yaw/pitch set via a preceding `SetPlayerRotation` packet, M3-B02 precedent, aimed directly at the zombie). `A` sends `Attack{entity_id: zombie_net_id}`. `A` reads, in order: `Damage Event{entity_id: zombie_net_id, source_type_id: <PlayerAttack's own ordinal>, ...}`, `Set Entity Velocity{entity_id: zombie_net_id, ...}` (nonzero, the flat `0.4` impulse #1's own resulting velocity, since bare attack-knockback/enchant are both `0`), `Set Entity Data{entity_id: zombie_net_id, ...}` (health metadata index 9, reduced from `20.0` by the full-charge unarmed `AttackDamage=2.0` default). `world.debug_query_entity(zombie_net_id).unwrap().health == 18.08` (Zombie's own `Armor=2.0` absorbs part of the hit: `toughness=2.0`, `real_armor=clamp(2-2.0/2.0, 0.4, 20)=1.0`, `armor_fraction=0.04`, `damage=2.0*0.96=1.92`, `health=20.0-1.92`).
2. `attack_out_of_reach_produces_no_packets` — `A` and `(_, zombie_net_id) = world.debug_spawn_mob(..)` at a position `10.0` blocks away (beyond `ENTITY_INTERACTION_RANGE=3.0`) — `A` sends `Attack{entity_id: zombie_net_id}` — `A` reads **nothing** within a bounded timeout; `world.debug_query_entity(zombie_net_id).unwrap().health` unchanged.
3. `attack_occluded_target_is_rejected` — `(_, zombie_net_id)` spawned within Euclidean range but behind a solid block from `A`'s own look direction (mirroring M3-B03's own `raycast_reach_rejects_an_occluded_target` test shape, applied to an entity target) — `A` reads nothing; health unchanged.
4. `interact_packet_is_a_silent_no_op` — `A` sends `Interact{entity_id: zombie_net_id, hand: 0, location: <arbitrary in-range point>, using_secondary_action: false}` (`zombie_net_id` from a freshly spawned, in-range, unoccluded zombie) — `A` reads nothing; zombie health unchanged (Context, "Packets" — `Interact` is accepted, not rejected, and produces zero effect).
5. `repeated_attacks_within_ten_ticks_apply_only_the_delta` — `A` attacks the same fresh zombie (`health=20.0`, `zombie_net_id`) twice within the same simulated 10-tick window (`invulnerable_time` still `>10` after hit 1) with the second attack's own effective damage reduced via `world.debug_override_attribute(A_net_id, AttributeKind::AttackDamage, <a smaller value>)` between the two `Attack` sends — the second `Damage Event`/`Set Entity Data` pair reflects only the delta, matching `combat_damage_pipeline.rs`'s own unit-level assertion end-to-end through real packets.

### `crates/server/tests/play_combat_death.rs`

1. `zombie_death_drops_loot_and_despawns` — `(_, zombie_net_id) = world.debug_spawn_mob(EntityKind::Zombie, ..)`, `world.debug_override_attribute(zombie_net_id, AttributeKind::MaxHealth, 2.0)` then `world.debug_deal_damage(zombie_net_id, 2.0)` (killing it in one call, exercising the same `apply_damage_pipeline` death branch a real hit would) — `A` (an observing connection, already tracking the zombie) reads `Damage Event`, then `Entity Event{entity_id: zombie_net_id, event_id: 3}`, then one or more `Spawn Entity` packets for the dropped rotten-flesh item entity/entities (asserted `entity_type == EntityKind::Item.registry_id().0 as i32`, mirroring M4-B01's own identical assertion style), then `Remove Entities{entity_ids: [zombie_net_id]}`, in that order. A second observer connection `B` (also already tracking the zombie) reads the identical sequence. Post-test: `world.debug_query_entity(zombie_net_id)` returns `None` (fully despawned, `EntityIndex`/`NetworkEntityIndex` both no longer resolve it).
2. `player_death_sends_combat_kill_and_marks_dead_without_despawn` — connection `A` (`A_net_id` from its own join flow), `world.debug_deal_damage(A_net_id, 25.0)` — `A` reads `Set Health{health: 0.0, ...}` then `Player Combat Kill{player_id: A_net_id, ..}`. A subsequent `Attack` packet sent by `A` (targeting anything) produces **no** further combat packets (the `is_dead` guard) — `A`'s own connection remains open (not disconnected), matching Context's explicit "left connected, visibly dead, indefinitely" scope statement. `world.debug_query_entity(A_net_id)` still returns `Some(..)` with `is_dead == true` (a player is never removed from `NetworkEntityIndex` on death, Context).

### `crates/server/tests/play_combat_packets.rs`

1. `attack_and_interact_packets_round_trip` — `Attack{entity_id}` encodes then decodes to an identical struct (a single VarInt, ordinary derive); `Interact{entity_id, hand, location, using_secondary_action}` encodes then decodes to an identical struct across a representative set of `location` values (near-zero, an ordinary vector, and one requiring the trailing scale VarInt), proving the flat, unconditional four-field layout round-trips and the hand-implemented `LpVec3` codec handles all three encodings.
2. `update_attributes_encodes_empty_modifier_arrays` — `UpdateAttributes{entity_id: 5, attributes: vec![(AttributeKind::Armor.registry_ordinal(), 12.0)]}` encodes to exactly the expected byte sequence (entity_id VarInt, count-prefix `1`, attribute_id VarInt, `f64` base_value, modifier_count VarInt `0`) — hand-computed byte vector asserted exactly.
3. `set_health_and_damage_event_are_derive_generated_and_symmetric` — ordinary round-trip encode/decode for both.

## Implementation steps

1. **`crates/mechanics/src/entity/living.rs`.** Add the four new fields per Deliverables, in declaration order at the end of the struct (does not disturb the existing `#[net_metadata(...)]` ascending-index compile check — none of the four carry that attribute). Observable: `cargo build -p rc-mechanics` succeeds; M4-B01's own `zombie_round_trips`-style NBT test (unaffected, since the new fields default via `EntityNbtFields` rule 2 when absent from a loaded compound) still passes unmodified.
2. **`crates/mechanics/src/combat/attributes.rs`.** `AttributeInstance::compute_value` implements the exact 3-stage calc (Context) via three sequential folds over `self.modifiers` filtered by `operation`, then `.clamp(self.min, self.max)` — the three folds iterate `self.modifiers` in `Vec` insertion order, Context's own cited, bounded ordering exception, not vanilla's own hash-table slot order. `AttributeMap::add_modifier` implements the exception's own debug-assertion-style precondition check (Deliverables), panicking if a second modifier of the same `ModifierOperation` is ever added to the same attribute. `default_attributes_for` is one `match` per `EntityKind`, each arm building an `AttributeMap` via ten `AttributeInstance::constant(...)` calls per the Context table (Item's arm returns an empty map) — every arm attaches zero modifiers, so the exception's precondition holds trivially for every production call site this blueprint ships. Observable: `combat_damage_pipeline.rs` compiles against this file.
3. **`crates/mechanics/src/combat/damage.rs`.** `DamageTypeKind::{exhaustion,scaling,bypasses_armor,no_knockback}` are four `match` statements per the Context table. `armor_effective_damage`/`epf_reduction`/the five `*_epf` functions/`difficulty_scale_incoming` are direct, mechanical translations of Context's own pseudocode — no algorithmic freedom. `apply_damage_pipeline` composes them in Context's exact order, including the creative short-circuit, the invulnerability gate (mutating `target.invulnerable_time`/`last_hurt` in place), and the absorption asymmetry (an `if target.is_player` branch selecting between the two documented sub-algorithms). Observable: `combat_damage_pipeline.rs`'s seven cases pass.
4. **`crates/mechanics/src/combat/melee.rs`.** Direct translation of Context's own pseudocode for every function; `assemble_player_melee_damage` composes `attack_cooldown_charge_scale`/`can_critical_attack`/the three enchant-bonus functions/the sweep-gate boolean expression in Context's exact order (charge scale computed once, reused for both the `magic_boost` scale and the `base_damage_scale_factor` call). Observable: `combat_melee_assembly.rs`'s six cases pass.
5. **`crates/mechanics/src/combat/knockback.rs`.** `apply_knockback_impulse` is a direct translation of Context's own pseudocode, including the unconditional `power *= (1-resistance)` before the `power <= 0` early return (test 3's own asserted behavior), the degenerate-direction `rng.next_double()` fallback (a `while` loop re-drawing in groups of 4 calls until the sample clears `KNOCKBACK_DEGENERATE_THRESHOLD`, test 4), and the `on_ground`-branched Y formula. Observable: `combat_knockback.rs`'s five cases pass.
6. **`crates/mechanics/src/combat/fall.rs`, `death.rs`, `food.rs`.** Direct translations of their own Context pseudocode; `FixedTierTwoLoot`'s per-kind item-id constants are hand-typed placeholders (reconciled against a real `xtask codegen` run per its own doc comment, one line each, mirroring M4-B01's own `ITEM`/`ZOMBIE`/`VILLAGER`/`COW` constant-transcription convention exactly). Observable: `combat_fall_food.rs`/`combat_death_loot.rs` pass.
7. **`crates/physics/src/motion.rs`.** Add `landed_fall_distance: Option<f64>` to `LivingMotionState`'s struct literal and the one capture line into `step_living_entity_tick`'s existing body, per Deliverables' exact insertion point. Observable: `cargo build -p rc-physics` succeeds; M3-B02's own three golden-vector tests still pass unmodified (they construct `LivingMotionState` positionally/by-name at call sites this blueprint does not touch, or via a `..Default::default()`-shaped literal if M3-B02's own tests already use one — either way the new field defaults to `None` and is never asserted on by M3-B02's own test file).
8. **`crates/server/src/play/combat_packets.rs`.** The packet structs per Deliverables/Context table; `Interact`'s hand-implemented `WireRead`/`WireWrite` codes its bespoke `LpVec3` `location` field, `UpdateAttributes`'s iterates the attribute list, mirroring M4-B01's own `Set Entity Data` hand-roll precedent structurally; `Attack`'s single-VarInt shape derives normally. Observable: `play_combat_packets.rs`'s three cases pass.
9. **`crates/server/src/play/movement.rs`.** Add `landed_fall_distance` to `PlayerMotion` and the two capture-site edits per Deliverables (exact, cited, additive). Observable: compiles; M3-B02's own movement tests unaffected (same reasoning as step 7).
10. **`crates/server/src/play/combat.rs`.** `resolve_enchant_levels` returns `EnchantLevels::default()` unconditionally (one line, doc-commented per Context). `entity_reach_check`/`raycast_entity_reach` (this file or `rc-mechanics`, per Deliverables' own placement — `raycast_entity_reach` lives in `rc-mechanics::combat` since it needs no `rusty-clanker-server`-only type, `entity_reach_check` is the thin `PlayerMotion`-binding wrapper here) implement the slab-method ray/AABB test per Context's own algorithm description. `apply_combat_step`/`register_mob_combat_system` wire every prior step's functions together per Context's own exact call order, sending packets per the Context "Packets" section's own broadcast rules (mirroring M2-B07/M3-B03's own `respond_to_action`-shaped broadcast-then-ack pattern). Observable: `play_combat_melee_flow.rs`/`play_combat_death.rs` pass once wired into `world.rs`/`connection.rs` (step 11).
11. **`crates/server/src/play/{world.rs, connection.rs, mining.rs, mod.rs}`.** Per Deliverables' own exact description: new resources/fields, `debug_spawn_mob`/`debug_set_difficulty`/`debug_deal_damage` diagnostic methods, join-drain insertions, the `is_dead` guards on the movement/mining packet loops, the two new tick-loop steps (`apply_combat_step` inserted after M4-B01's own entity-tracking step; `register_mob_combat_system`'s builder registration performed once, alongside every other composition-root `register_*` call), the `Interact` packet's connection-dispatch match arm. Observable: every `crates/server/tests/play_combat_*.rs` file passes; the full `rusty-clanker-server` test suite (every prior blueprint's own tests, unmodified) remains green.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** (TEST-D45/D46). Every file under `crates/mechanics/tests/combat_*.rs` and `crates/server/tests/play_combat_*.rs` is committed first, alongside `todo!()`-stubbed `src/` files and the already-shaped-but-`todo!()`-bodied edits to every modified file this blueprint names. The implementation changeset fills in real bodies only — it must not edit any test file, weaken any assertion, or change any golden value this blueprint's Acceptance tests section already fixes.

(b) **No new external dependencies.** Every crate this blueprint uses (`rc-core`, `rc-physics`, `rc-scheduler`, `bevy_ecs`) is already a dependency of `rc-mechanics`/`rusty-clanker-server` per M4-B01/M3-B02/M3-B03's own already-merged `Cargo.toml` edits. This blueprint adds zero new `[workspace.dependencies]` entries and zero new per-crate dependency lines.

(c) **No Mojang or third-party reimplementation code.** Every formula this blueprint restates is sourced from `docs/research/mc-26.2/{19-combat-damage.md, 20-enchant-xp-loot-math.md, 09-entities-ai.md}` (original prose/pseudocode per ASSET-D18(f)'s policy — no verbatim method body reproduced) and from a live `minecraft.wiki` fetch performed while deriving this blueprint (packet field shapes, ASSET-D18(f)). No decompiled source and no third-party reimplementation's code (Azalea, Pumpkin, MCProtocolLib, or any other, per ASSET-D30's firewall) was consulted.

(d) **Parity rule.** Every formula in Context is implemented exactly as stated, including the documented player/mob absorption asymmetry (§3.7) and the two-impulse, non-mergeable knockback model (§3.10) — neither may be "corrected," unified, or simplified during implementation. Where a constant is flagged moderate-confidence (per-mob attribute defaults, packet ids, entity hitbox dimensions, item-drop-velocity spread), the implementer records the reconciliation as a one-line follow-up per constant, never silently guesses a different value without the flag.

(e) **Determinism rule.** The one RNG consumer this blueprint has (knockback's degenerate-direction fallback, `FixedTierTwoLoot`'s item-count rolls) uses `rc_mechanics::random::RcRandom` exclusively (MECH-D5) — never `rand`, `std::random`, or any other source, even though neither consumer's own output needs to be world-seed-reproducible (Context, "Ambient combat RNG").

(f) **Scope boundary**, restated from Context's own "Scope boundary" section verbatim: this blueprint does not implement shield blocking, Resistance/status effects, Mace/piercing weapons, projectiles, XP orbs, explosions or MECH-D18's halo widening, mob fall damage, or the full player respawn/keepInventory cycle. Do not add placeholder implementations of any of these as a shortcut — a `todo!()` or an obviously-wrong stand-in is worse than an honestly absent feature, since it invites a later blueprint to assume real behavior exists where it does not.

(g) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

## Verification commands

```
cargo build -p rc-mechanics -p rc-physics -p rusty-clanker-server --all-features
cargo nextest run -p rc-mechanics -p rc-physics -p rusty-clanker-server --profile ci
cargo test --doc -p rc-mechanics -p rc-physics -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
```

Expected: every command exits 0. `cargo nextest run` additionally runs `combat_damage_pipeline.rs` (7 cases) + `combat_melee_assembly.rs` (6 cases) + `combat_knockback.rs` (5 cases) + `combat_fall_food.rs` (4 cases) + `combat_death_loot.rs` (3 cases) + `play_combat_melee_flow.rs` (5 cases) + `play_combat_death.rs` (2 cases) + `play_combat_packets.rs` (3 cases) = 35 new test cases, alongside every pre-existing test this blueprint does not touch. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
