# M4-B10 — Tier-2 Input Components: Button and Pressure Plate

| Field | Content |
|---|---|
| ID | M4-B10 |
| Milestone | M4 — Mechanics Tier 2: Entities, AI, Combat, Items |
| Prerequisites | M3-B01 (`rc-mechanics`: `BlockBehavior`/`BlockBehaviorRegistry`, `UpdateContext`, `ScheduledTickQueue`, `NeighborUpdateEngine`, `stage4::run_scheduled_phase`, `BlockWorldAccess`). M3-B04 (`rc-mechanics::redstone`: `RedstoneSignalSource`/`SignalSourceRegistry`, `signal::is_face_sturdy`, `signal::notify_neighbor_changed_only`, `register_tier1_redstone`, `dispatch_ranges`). M3-B05 (`redstone::piston::classify`'s own `PushClass::Destroy` range set). M3-B03 (`rusty-clanker-server::play::mining`: `PlaceableBlockKind`, `HeldItemStub`, `dig_properties`, `resolve_orientation`, `AttachFace`, `Orientation`, `tier1_oriented_entries`, `apply_placement_with_redstone`, `apply_block_use`). M3.5-B01/B02 (`rc_registries::block_state_properties::{state_id, with_property, properties, range_of}` and the generated `block_id`/`sound_event`/`item` tables — every id in this blueprint is derived through them, never hand-typed). M3 field-report wave 3 / PLAN-D10 (`redstone::lever::LeverBehavior`, `MECH-D82`'s `on_use`/`UseContext`/`UseOutcome`/`UseUpdateContext`, `sound_request::{SoundRequest, SoundSource}`, `MECH-D84`'s `rc_physics::SupportKind`, `ScheduledTickQueue::will_block_tick_this_tick` — the lever is this blueprint's direct structural precedent and is reused, never modified). M4-B01 (`rc_mechanics::entity::{BaseEntity, EntityKind}`). M4-B02 (`entity::physics` — the `ecs.rs` per-kind dimension table this blueprint promotes to a public function; item entities exist and can rest on a plate). |
| Implements | MECH-D13 (the tier-2 half of the input-component set the lever's own PLAN-D10 sentence completes: button and pressure plate as their own Stage-4 behaviours in `ARCH-D13`'s single-worker pass); MECH-D73 (both new behaviours dispatch through the existing per-target registry, never a bespoke path); MECH-D82 (the button's `on_use` press, the click sound and its exclusion rule); MECH-D84 (per-face support predicates: `Full` on the button's mount face, `Rigid`-or-`Center` on the block below a plate, and the `noCollision` support-shape rule that makes both blocks sturdy on no face themselves); MECH-D78 (unchanged, reused — the dual-cell resend already covers a consumed button use); ARCH-D13/ARCH-D14 (scheduled block ticks at `TickPriority::Normal`); ARCH-D10 (only entities this region owns count toward a plate's entity census); PLAN-D10 (the roadmap's own `M4-B10` assignment; closes the `docs/findings-for-planning.md` entry "Tier-2 input components have no blueprint"). No new decision ID is created by this blueprint. |
| Crates touched | `rc-mechanics` (`crates/mechanics/src/redstone/{button.rs, pressure_plate.rs, entity_presence.rs}`, new; `crates/mechanics/src/redstone/{mod.rs, registration.rs, dispatch_ranges.rs, piston.rs}`, `crates/mechanics/src/{behavior.rs, stage4.rs, stage4/ecs.rs, lib.rs}`, `crates/mechanics/src/entity/physics/{mod.rs, ecs.rs}`, modified). `rc-physics` (`crates/physics/src/shapes.rs`, modified — additive rows only). `rusty-clanker-server` (`crates/server/src/play/entity_presence.rs`, new; `crates/server/src/play/{mining.rs, world.rs, mod.rs}`, modified). `crates/testing/gametest/corpus/redstone/` (new fixtures + manifest rows, test changeset only). |
| Estimated scope | M |

## Goal & Done definition

Give the engine vanilla's two tier-2 redstone *input* components, built on exactly the seams the lever already established: the **button** (`ButtonBlock`) — face-attached placement identical to the lever, an `on_use` press that powers the block, schedules its own release tick 20 (stone/polished blackstone) or 30 (every wooden variant) ticks out, fans out at its own and its mount cell, and plays the block-set-type's click sound; weak 15 toward all six neighbours while pressed, strong 15 only into its mount block; pops when the mount face stops being `Full`-sturdy — and the **pressure plate** family (`PressurePlateBlock`, `WeightedPressurePlateBlock`) — an entity-presence trigger polled through a new, generic `on_entity_inside` dispatch and re-checked by a scheduled tick every 20 (plain) or 10 (weighted) ticks while pressed, boolean 15/0 for plain plates with `EVERYTHING`/`MOBS` sensitivity and analog `ceil(min(count, max_weight) / max_weight * 15)` for the gold (`max_weight` 15) and iron (`max_weight` 150) weighted plates; weak signal in every direction, strong only downward; pops when the block below stops being `Rigid`-or-`Center`-sturdy. Ships the two mechanisms this content needs and no more: a Stage-4-reachable **sound outbox** (`UpdateContext.sounds` + `TickSoundOutbox`, the exact shape `changed`/`TickChangedPositions` already established, absorbing `UseUpdateContext`'s own duplicate outbox) so a sound produced by a scheduled tick reaches a client at all, and an **entity-box census seam** (`EntityPresenceSource`) whose one production implementation lives in `rusty-clanker-server`, the only crate that can see both `PlayerMarker` players and `BaseEntity` mobs/items.

Done when:

- [ ] `cargo build -p rc-mechanics -p rc-physics -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mechanics -p rc-physics -p rusty-clanker-server` (default features).
- [ ] Every button release-timing case is exact: a stone/polished-blackstone button released on the 20th tick after its press and not before; a wooden button on the 30th.
- [ ] Every weighted-plate analog row of the hand-computed `(count, max_weight) -> power` table matches exactly, including both `count == 0` and `count > max_weight` saturation.
- [ ] The support/pop tests pass for both blocks: a button pops the tick its mount face stops being `Full`-sturdy and only when the shape update arrives from the mount direction; a plate pops when the block below is neither `Rigid`- nor `Center`-sturdy on its top face.
- [ ] `xtask parity-check redstone` is green with this blueprint's own new corpus fixtures included, and `xtask verify-fixtures` accepts the regenerated corpus manifest.
- [ ] The real-connection suite passes: a bot places, presses and hears/does-not-hear a button; a bystander connection receives the press *and* the release `Sound` packet; a bot walking onto a plate powers it within one tick and un-powers it within `getPressedTime` ticks of stepping off; N dropped item entities on a weighted plate produce the hand-computed analog power.
- [ ] `cargo run -p xtask -- lint-deps`, `fmt-check`, `lint`, `lint-tests` all exit 0 — **no new dependency edge of any kind**: every crate this blueprint touches keeps exactly the dependency set it has today (`rc-mechanics` → `rc-physics`/`rc-registries`/`rc-core`/`rc-chunk-storage`/`rc-messaging`/`rc-scheduler`/`bevy_ecs`; `rc-physics` → `rc-registries`/`rc-core`; `rusty-clanker-server` unchanged), and no `[workspace.dependencies]` entry is added or bumped.
- [ ] `cargo test --doc -p rc-mechanics -p rc-physics -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `lint-tests`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50), plus the scheduled `parity-check redstone` tier.

## Context (self-contained)

### A. Scope boundary, stated up front

**In scope:** all 14 button blocks (`stone`, `polished_blackstone`, and the twelve wooden variants `oak`/`spruce`/`birch`/`jungle`/`acacia`/`cherry`/`dark_oak`/`pale_oak`/`mangrove`/`bamboo`/`crimson`/`warped`) and all 16 pressure plates (the same 14 block-set types as plain `PressurePlateBlock`s, plus `light_weighted_pressure_plate` and `heavy_weighted_pressure_plate`), each as a real `BlockBehavior` + `RedstoneSignalSource` registered over its own generated state-id range; the press/release state machine, signal semantics, support/pop rule, sounds, piston `DESTROY` reaction, scheduled-tick cadence, and placement for a bounded six-kind representative subset (§H).

**Out of scope, explicitly** (do not add a placeholder implementation of any of these):

- **Arrow-triggered button presses — deferred, with a wired-in seam.** `ButtonBlock.entityInside`/`checkPressed` press a wooden button when an `AbstractArrow` is inside the button's outline box. No projectile entity exists anywhere in this engine: M4-B01's tier-2 `EntityKind` set is `Item`/`Zombie`/`Cow`/`Villager` with no arrow kind, and M4-B05's own Context scopes "projectiles/arrows/bows entirely" out of M4. This blueprint therefore implements `check_pressed(ctx, pos, arrow_present: bool)` in full and calls it with a literal `false` from both of its own call sites, carrying `can_be_activated_by_arrows` on every `ButtonBehavior` instance so that the future projectile blueprint's whole change is computing that one boolean. Nothing else about the arrow path is stubbed or approximated.
- **Explosion-triggered presses.** `ButtonBlock.onExplosionHit` presses an unpressed button, and `LeverBlock`/`BasePressurePlateBlock` have their own explosion interactions. No explosion mechanic exists (M4-B05's own Context re-defers MECH-D18 and explosions entirely), so no explosion entry point is added here either.
- **Game events / vibration (`GameEvent.BLOCK_ACTIVATE`/`BLOCK_DEACTIVATE`) and particles.** No `GameEvent` bus and no particle mechanic exists in this engine. Both are named at their exact position in the algorithm below and skipped there, never silently omitted.
- **Wind charge activation** (`BlockSetType.canOpenByWindCharge`) — no wind charge exists.
- **A general `entityInside` effect catalog.** This blueprint introduces the `on_entity_inside` *dispatch* (§E) because a pressure plate cannot work without it, with exactly one production implementor (the plate). It does not port any other vanilla `entityInside` effect (cactus damage, cobweb slowdown, fire, portals, hopper minecart pickup, `InsideBlockEffectApplier` as a type).
- **`isPrecise` and the swept multi-step form of `Entity.checkInsideBlocks`.** §E states the exact bounded simplification and why it is observationally equivalent at this engine's own entity speeds.
- **Every non-representative placeable button/plate variant.** All 30 blocks get real behaviour, shapes, signals and dispatch; only six get a `PlaceableBlockKind` placement row (§H) — the exact minimal set that covers every distinct behavioural class.

### B. Where this content lives, and how both registries are fed

Two new modules under `crates/mechanics/src/redstone/`, mirroring `lever.rs` exactly: each behaviour type implements **both** `BlockBehavior` (Stage-4 dispatch) and `RedstoneSignalSource` (power queries), holds no per-position side table at all, and decodes everything it needs from the world's own stored `BlockStateId` on every read (`properties`/`with_property`/`state_id` from `rc_registries::block_state_properties`). A button's entire observable state is `(face, facing, powered)`; a plate's is `powered` or `power` — all of it already lives in the block state, exactly like the lever and unlike wire/torch/repeater/comparator.

Unlike the lever, one shared instance per *block type* is needed rather than one for the whole family, because `ticks_to_stay_pressed`, the two click-sound registry ids, `can_be_activated_by_arrows`, `sensitivity` and `max_weight` differ per block. Both registries are range-based and reject overlaps, so registration is a loop over a `const` table of `(BlockId, …)` rows, one `register_range(range_of(block).first, range_of(block).last + 1, instance)` pair per row into `BlockBehaviorRegistry` and `SignalSourceRegistry`. Neither behaviour needs a `SignalSourceRegistry` back-reference (nothing they compute depends on a neighbour's signal), so — exactly like `LeverBehavior` — neither appears in `Tier1RedstoneHandles` and neither participates in the two-phase `bind_registry` step.

`register_tier2_inputs(behaviors, signals, entities: Arc<dyn EntityPresenceSource>)` is a **new, separate** composition-root entry point beside `register_tier1_redstone`/`register_redstone_block`/`register_piston`/`register_hopper`, callable in any order relative to them (it has no ordering dependency of its own), invoked once per region from `bootstrap_redstone_dispatch` and from `crates/testing/gametest/src/replay.rs`'s own registry construction.

### C. The button — `ButtonBlock`, restated exactly

Verified against the ASSET-D18(f) reference (`ButtonBlock`, `FaceAttachedHorizontalDirectionalBlock`, `BlockSetType`, `Blocks`).

**Properties and states.** `FACING` (four horizontals) + `POWERED` (boolean) + `FACE` (`floor`/`wall`/`ceiling`) — 24 states per button block, default `facing=north, powered=false, face=wall`. Identical property set to the lever, so the lever's own `AttachFace`/`facing`/`mount_direction` decode logic is duplicated in shape but not shared in code: `button.rs` carries its own private copies of `attach_face_from_str`/`facing_from_str`/`mount_direction`, matching the existing per-module convention (`lever.rs` and `torch.rs` already each carry their own) rather than extracting a shared helper mid-milestone.

**`mount_direction(face, facing)`** — `getConnectedDirection(state).getOpposite()`: `Floor -> Down`, `Ceiling -> Up`, `Wall -> facing.opposite()` (`facing` points away from the wall, into the room). Byte-for-byte the same derivation `lever.rs::mount_direction` already documents.

**Press (`on_use`, MECH-D82).** `useWithoutItem` returns without pressing when `POWERED` is already `true` (vanilla's `InteractionResult.CONSUME`, which still ends the dispatch — so this blueprint returns `UseOutcome::Consumed`, never `Pass`, and a fall-through placement must not happen). Otherwise `press` runs, in this exact order:

1. `ctx.set_block(pos, powered=true)` — vanilla's `setBlock(pos, state, 3)`, whose flag-3 fan-out `UpdateContext::set_block` already reproduces.
2. `update_neighbours` — `signal::notify_neighbor_changed_only` at `pos` **and** at `mount_direction(..).apply(pos)`. The first call deliberately duplicates what `set_block`'s own fan-out just did (vanilla's own literal double-fire, reproduced exactly as `lever.rs::on_use` already does); the second is the genuinely new one-hop propagation into the mount cell's neighbours.
3. `ctx.schedule_block_tick(pos, ticks_to_stay_pressed, TickPriority::Normal)`.
4. `ctx.request_sound(SoundRequest { pos, sound: click_on, source: Blocks, volume: 1.0, pitch: 1.0, except_actor: true })`.
5. *(skipped: the `BLOCK_ACTIVATE` game event — §A.)*

`ButtonBlock.useWithoutItem` carries **no `mayBuild` guard** (unlike `RepeaterBlock`/`ComparatorBlock`, and unlike this engine's own `LeverBehavior::on_use`, which added one). This blueprint reproduces the reference: no `may_build` check. The difference is unobservable today (`UseContext::may_build` is unconditionally `true` at every real dispatch site) and is recorded as a ledger item rather than "fixed" in the lever (§J).

**Volume and pitch.** `ButtonBlock.playSound` and `BasePressurePlateBlock.checkPressed` both call `LevelAccessor.playSound(entity, pos, sound, source)`, the four-argument overload, which defaults to **volume `1.0`, pitch `1.0`** — unlike `LeverBlock`, which passes an explicit `0.3` / `0.6`-on-`0.5`-off. There is no per-material pitch for buttons or plates.

**The excluded listener.** `playSound(pressed ? player : null, …)`: the *press* excludes the acting player (who predicts it client-side — `except_actor: true`); the *release*, and every `checkPressed`-driven transition, excludes nobody (`except_actor: false`).

**Release (`on_scheduled_tick`).** `tick` calls `check_pressed` only when `POWERED` is `true`. `check_pressed(ctx, pos, arrow_present)`:

```
should_be_pressed = can_be_activated_by_arrows && arrow_present      # always false today, §A
was_pressed       = decoded POWERED
if should_be_pressed != was_pressed:
    ctx.set_block(pos, with_property(current, "powered", should_be_pressed))   # flag 3
    update_neighbours(ctx, pos, face, facing)
    ctx.request_sound(click_on if should_be_pressed else click_off, except_actor: false)
    # skipped: the BLOCK_ACTIVATE / BLOCK_DEACTIVATE game event
if should_be_pressed:
    ctx.schedule_block_tick(pos, ticks_to_stay_pressed, TickPriority::Normal)
```

**Signals.** `ownSignal` → weak `15` toward every one of the six neighbours while `POWERED`, `0` otherwise, with no direction exclusion. `getDirectSignal` → `15` only when `POWERED` and the queried direction equals `getConnectedDirection(state)`; translated into this crate's own source→receiver `towards` convention (`signal::direct_signal_to` calls `direct_signal_toward(npos, d.opposite())`, the single translating seam) that is exactly `towards == mount_direction(face, facing)` — the identical composition `lever.rs::direct_signal_toward` already spells out in full. `is_signal_source` → `true`.

**Support and pop (`on_shape_update`, MECH-D84).** `canSurvive` = `canAttach(level, pos, getConnectedDirection(state).getOpposite())` = the mount block's face toward the button is sturdy for `SupportType.FULL` — `Full` for all three attach faces alike, never `Center`. `updateShape` returns air only when the update arrives **from the mount direction** and `canSurvive` is false; every other direction is ignored. Structurally identical to `LeverBehavior::on_shape_update`.

**Removal.** `affectNeighborsAfterRemoval` re-fans the neighbours when a `POWERED` button is removed by anything other than a piston. This engine has no `on_removed` hook and no blueprint has ever needed one; a button broken by a player is written to air through `UpdateContext::set_block`, whose own six-neighbour fan-out already delivers the same observable result at the button's own cell. The one cell vanilla additionally re-notifies — the mount cell — is **not** covered by that fan-out. This blueprint does not add an `on_removed` hook for it (the mount cell is one of the six neighbours of the button's own cell in every attachment, so `set_block`'s fan-out already notifies the mount block itself; only the mount block's *own further* neighbours are missed, and reaching them needs the strong-signal relay that `notify_neighbor_changed_only`'s conductor hop already performs from the button's cell). Recorded in §J as a bounded, cited gap rather than an unstated one.

**Per-block table** (all 14 rows; every id resolved through `range_of(block_id::X)` / `sound_event::Y`, never a literal):

| Block | `ticks_to_stay_pressed` | arrows | `click_on` / `click_off` |
|---|---|---|---|
| `stone_button`, `polished_blackstone_button` | 20 | no | `BLOCK_STONE_BUTTON_CLICK_ON` / `_OFF` |
| `oak_`, `spruce_`, `birch_`, `jungle_`, `acacia_`, `dark_oak_`, `pale_oak_`, `mangrove_button` | 30 | yes | `BLOCK_WOODEN_BUTTON_CLICK_ON` / `_OFF` |
| `cherry_button` | 30 | yes | `BLOCK_CHERRY_WOOD_BUTTON_CLICK_ON` / `_OFF` |
| `bamboo_button` | 30 | yes | `BLOCK_BAMBOO_WOOD_BUTTON_CLICK_ON` / `_OFF` |
| `crimson_button`, `warped_button` | 30 | yes | `BLOCK_NETHER_WOOD_BUTTON_CLICK_ON` / `_OFF` |

There is **no** per-wood-species button sound: the eight plain wooden types share `wooden_button`; only cherry, bamboo and the two nether woods have their own.

### D. The pressure plate family — `BasePressurePlateBlock` and its two subclasses

**Properties.** `PressurePlateBlock`: `POWERED` only (2 states, signal `15`/`0`). `WeightedPressurePlateBlock`: `POWER` only, integer `0..=15` (16 states, signal = the property value). Neither carries any orientation property — placement resolves `Orientation::None`.

**Signal strength.**

```
PressurePlateBlock.getSignalStrength(level, pos):
    class = Entity        if sensitivity == EVERYTHING
    class = LivingEntity  if sensitivity == MOBS
    return 15 if count(class, TOUCH_AABB + pos) > 0 else 0

WeightedPressurePlateBlock.getSignalStrength(level, pos):
    count = min(count(Entity, TOUCH_AABB + pos), max_weight)      # class is ALWAYS Entity here,
    if count == 0: return 0                                       # never gated on sensitivity
    percent = (min(max_weight, count) as f32) / max_weight as f32
    return ceil(percent * 15.0)
```

`getEntityCount` filters with `EntitySelector.NO_SPECTATORS` **and** "the entity does not ignore block triggers". At M4's own entity set — players (no spectator game mode exists; `GameModeState` carries only `instabuild`), zombies, cows, villagers, item entities — both predicates are structurally all-pass; they are modelled as two explicit `true`-returning fields on the census record (§E) so a future spectator/armour-stand-marker mechanic flips one value rather than discovering the filter is missing.

**Sensitivity.** `BlockSetType.PressurePlateSensitivity` has exactly two values: `EVERYTHING` and `MOBS`. `stone` and `polished_blackstone` are `MOBS` (living entities only — players included, since a player *is* a `LivingEntity`); every wooden type, plus `iron`/`gold`/`copper`, is `EVERYTHING`.

**`getPressedTime`.** `20` on `BasePressurePlateBlock`; **`10`** on `WeightedPressurePlateBlock` — the weighted plates re-check twice as often, not on the 20-tick cadence.

**`check_pressed(ctx, pos, old_signal)`** — the one algorithm both subclasses share:

```
signal      = get_signal_strength(entities, pos)
was_pressed = old_signal > 0
is_pressed  = signal > 0
if old_signal != signal:
    ctx.write_block_state(pos, set_signal_for_state(current, signal))   # vanilla flag 2:
    update_neighbours(ctx, pos)                                          # clients only, NO fan-out
if !is_pressed && was_pressed:
    ctx.request_sound(click_off, except_actor: false)     # + skipped BLOCK_DEACTIVATE game event
elif is_pressed && !was_pressed:
    ctx.request_sound(click_on,  except_actor: false)     # + skipped BLOCK_ACTIVATE game event
if is_pressed:
    ctx.schedule_block_tick(pos, get_pressed_time(), TickPriority::Normal)
```

Three details are load-bearing and deliberate:

1. **`write_block_state`, not `set_block`.** Vanilla writes with update flag `2` — a client update with **no** `updateNeighborsAt` call — and then performs the neighbour fan-out itself via `updateNeighbours`. `UpdateContext::write_block_state` is this project's already-established mapping of exactly that flag-2 write (its own doc comment names every tier-1 component that uses it for the same reason), so the plate follows the diode/torch writeback convention, **not** the lever/button `set_block` convention. Using `set_block` here would fan out at `pos` twice, an ordering divergence the button genuinely has and the plate genuinely does not.
2. **`update_neighbours` for a plate is `pos` and `pos.below()`** — not the mount-cell rule the button and lever use.
3. **A nonzero→nonzero power change plays no sound** (a weighted plate going 3→7 is neither an on- nor an off-transition) but still writes and still fans out.

**Trigger points.** `tick` calls `check_pressed` only when the stored signal is `> 0` (the release/re-evaluate path). `entityInside` calls it only when the stored signal is exactly `0` (the press path). So while a plate is pressed, its power is re-evaluated **only** by the scheduled tick — which is exactly why a weighted plate's output lags an entity-count change by up to `get_pressed_time()` ticks, and why that lag is correct rather than a bug to remove.

**Support and pop.** `canSurvive` = `canSupportRigidBlock(level, below) || canSupportCenter(level, below, UP)` — the block below is sturdy on its **top** face for `SupportType.RIGID` **or** for `SupportType.CENTER`. An OR of two kinds, not `Rigid` alone; a shape that covers the outer 2-pixel ring *or* the centred 2×2 square carries a plate. `updateShape` returns air only when the update arrives from `DOWN` and `canSurvive` is false.

**Signals.** `ownSignal` → the state's own signal toward every direction. `getDirectSignal` → the state's own signal only for the queried direction `UP`, i.e. only into the block **below** the plate once translated into this crate's `towards` convention (`towards == Direction::Down`). `is_signal_source` → `true`.

**Per-block sound table** (16 rows): `stone`/`polished_blackstone` → `BLOCK_STONE_PRESSURE_PLATE_CLICK_ON`/`_OFF`; the eight plain wooden types → `BLOCK_WOODEN_PRESSURE_PLATE_CLICK_ON`/`_OFF`; `cherry` → `BLOCK_CHERRY_WOOD_…`; `bamboo` → `BLOCK_BAMBOO_WOOD_…`; `crimson`/`warped` → `BLOCK_NETHER_WOOD_…`; `light_weighted` (gold) and `heavy_weighted` (iron) → `BLOCK_METAL_PRESSURE_PLATE_CLICK_ON`/`_OFF`.

### E. The entity-presence trigger: `on_entity_inside` dispatch and the census seam

Two separate mechanisms are needed, and vanilla uses two: a **trigger** (`Entity.checkInsideBlocks` → `BlockState.entityInside`) that notices an entity has entered the plate's cell, and a **census** (`Level.getEntitiesOfClass(class, TOUCH_AABB + pos, filter)`) that counts what is actually standing there.

**The trigger — `BlockBehavior::on_entity_inside`.** A new hook with a no-op default, additive and backward-compatible in exactly the way `on_random_tick` and `on_use` already were:

```rust
fn on_entity_inside(&self, _ctx: &mut UpdateContext, _pos: BlockPos, _entity: &EntityTouch) {}
```

Vanilla's intersection rule is unusually simple for these blocks: `getEntityInsideCollisionShape` returns the **full block cube** by default, and `checkInsideBlocks` short-circuits its per-shape test to `true` whenever the shape *is* that full cube — so an entity is "inside" a button or a plate whenever its own bounding box, deflated by `1.0E-5`, intersects the block's whole `1×1×1` cell. Each intersected position is visited at most once per movement.

**Bounded, cited simplification:** vanilla sweeps the *movement segment* (`forEachBlockIntersectedBetween(from, to, …)`) under a 16-iteration budget shared across the whole movement; once that budget is exhausted vanilla itself abandons the sweep and re-checks only the destination box, and a teleport — which resets the old position rather than producing a movement segment — enters no sweep at all. Vanilla's own truncation therefore already reduces to exactly what this blueprint's driver does: it enumerates the cells intersected by the entity's **post-tick** AABB deflated by `1.0E-5` only, matching vanilla's own fallback in both of the cases where a full sweep would otherwise diverge from it. Every entity kind M4 has moves well under one block per tick under gravity and default `MovementIntent` (M4-B02's own tick shapes), so a full sweep never exhausts the budget at these speeds either, and no teleport path drives a plate, so the two enumerations agree for every case this milestone can produce. Recorded in §J so the future fast-movement/projectile blueprint knows what to widen.

**The driver — `entity_inside_step`, a manual tick-loop step.** `rc-mechanics` structurally cannot see `PlayerMarker`/`PlayerMotion` (WS-D3 rule 2 — the same boundary M4-B02's own module doc comment already hit for item pickup), and a plate exists almost entirely to be stood on by a player, so the driver lives in `rusty-clanker-server::play::entity_presence` and follows `entity_pickup_step`'s established shape exactly. It runs once per tick, immediately after `entity_pickup_step`/`entity_resync_step` (so it observes this tick's own fresh Stage-6b physics output) and **before** the `TickChangedPositions`/`TickBlockEventOutbox` drains (so anything it changes is broadcast this same tick with no call-site change). Per call it:

1. Rebuilds the region's `RegionEntityPresence` census from the live ECS world (§ below).
2. Takes `NeighborUpdateEngine`/`ScheduledTickQueue`/`BlockEventQueue`/`BlockBehaviorRegistry`/`LightDirtyQueue`/`RegionOwnership` out of `region.world`, exactly as the direct-action phase already does.
3. For every censused entity, enumerates the block positions its deflated AABB intersects, deduplicated across that entity's own cells; for each, resolves the behaviour and calls `on_entity_inside`, then drains the neighbour-update engine to a fixed point after each dispatch — the same per-entry settle `stage4::run_scheduled_phase` performs for a scheduled tick.
4. Merges its `changed` collector into `TickChangedPositions` and its `sounds` collector into `TickSoundOutbox` (§F), then reinserts every resource.

**The census — `EntityPresenceSource`.** The plate behaviour cannot see the ECS, so it reads through an injected trait, mirroring `ContainerSignalSource`/`Tier1ContainerSignalSource` (M3-B06) in shape, ownership and locking rationale:

```rust
pub trait EntityPresenceSource: Send + Sync {
    /// Vanilla's `getEntityCount(level, box, class)` — the number of owned, non-spectator,
    /// non-block-trigger-ignoring entities whose own AABB intersects `region`.
    fn count_entities_in(&self, region: rc_physics::Aabb, filter: EntityClassFilter) -> usize;
}
pub enum EntityClassFilter { AnyEntity, LivingOnly }
```

`RegionEntityPresence` (in `rusty-clanker-server`) holds a `Mutex<Vec<EntityPresenceRecord>>` refreshed once per tick by `entity_inside_step` and read by the plate during Stage 4 and during the entity-inside dispatch; the `Mutex` is never contended for the same reason M3-B06 documents (the phases run strictly sequentially within a region's tick). A record carries the entity's AABB, `is_living`, `is_spectator` (always `false` today) and `ignores_block_triggers` (always `false` today).

**ARCH-D10 — only owned entities count.** The census is built from one region's own `bevy_ecs::World`, so it contains exactly the entities that region owns, by construction. An entity standing on a plate whose chunk this region does not own is neither censused nor dispatched; a plate itself is only ever ticked by its owning region. This is stated rather than merely true-by-accident because vanilla's `Level.getEntitiesOfClass` has no ownership notion at all, and a future cluster-mode reader must not "fix" the difference by widening the query across regions.

**Entity AABBs.** Players: `rc_physics::Aabb::from_position(motion.position, PLAYER_HALF_WIDTH, PLAYER_HEIGHT_SNEAKING or PLAYER_HEIGHT)` — the identical crouch-aware construction `world.rs`'s placement-obstruction path already builds. Non-player entities: `Aabb::from_position(base.pos, half_width, height)` with `(half_width, height)` from M4-B02's own per-kind table, which this blueprint promotes from the private `ecs.rs::living_dimensions` to a public `rc_mechanics::entity::physics::entity_dimensions(kind)` and redirects `ecs.rs`'s two call sites to (a behaviour-preserving move, no value changed).

**Why a plate detects a player standing on it.** Buttons and plates are `noCollision` (§G), so a player standing on a plate at cell `(x, y, z)` rests on the block below with feet at exactly `y`. `TOUCH_AABB + pos` spans `y .. y + 0.25`; the player's box spans `y .. y + 1.8`; vanilla's `AABB.intersects` is strict on both bounds (`min < other.max && max > other.min`), which this overlap satisfies. Horizontally, a `0.6`-wide player centred in the cell spans `0.2 .. 0.8`, inside `TOUCH_AABB`'s `0.0625 .. 0.9375`.

### F. Sounds produced inside Stage 4 — the `UpdateContext` sound outbox

The button's release click and every pressure-plate click are produced by a **scheduled tick** or by the entity-inside dispatch, not by a direct player action. Today the only sound path is `UseUpdateContext::request_sound`, which exists solely for `on_use` and is drained synchronously at the block-use call site — structurally unreachable from `on_scheduled_tick`, whose context is a bare `UpdateContext`.

This blueprint therefore moves the outbox down one level, which also removes a duplicate:

- `UpdateContext` gains `pub sounds: &'a mut Vec<SoundRequest>` and `pub fn request_sound(&mut self, request: SoundRequest)` — the tenth field, threaded exactly like `changed` and `light_dirty` before it.
- `UseUpdateContext` **loses** its own `sounds` field; its `request_sound` forwards to `self.base.request_sound(..)`. Its public method surface and `mining::apply_block_use`'s public signature (which still takes a caller-supplied `&mut Vec<SoundRequest>`) are unchanged, so every already-merged `crates/server/tests/` call site keeps compiling untouched.
- `stage4::ecs` gains `TickSoundOutbox(pub Vec<SoundRequest>)`, inserted by `bootstrap_default_stage4_resources`, merged from each Stage-4 system's own local collector exactly as `tick_changed.merge(changed)` already does, and drained once per tick in `world.rs` immediately after the `TickBlockEventOutbox` drain, broadcasting through the existing `broadcast_sound_request` with the actor id `-1` (no acting connection; every such request carries `except_actor: false`, so the id is never consulted — a sentinel, not a lookup).

This is the same coordinated, cited, single-changeset field addition M4-B07 performed for `light_dirty` (its own Constraint (e)), and it lands the same way: **in the test-authoring changeset**, which updates every one of the workspace's `UpdateContext { .. }` struct-literal construction sites (5 source files, 21 test files, plus `crates/testing/gametest/src/replay.rs`) with one mechanical `sounds: &mut …` line each. The implementation changeset never touches a test file.

### G. Shapes, collision and support — the `noCollision` rule

Every button and every pressure plate is registered with `noCollision()`. In the reference this has one precise consequence chain: `getCollisionShape` returns an empty shape whenever the block was built `noCollision`; `getBlockSupportShape` **defaults to the collision shape**; and all three `SupportType` variants evaluate against `getBlockSupportShape`. A button or a plate is therefore sturdy on **no** face for **any** support kind, and collides with nothing.

`rc_physics::ShapeTable` stores exactly one `VoxelShape` per state and serves all three consumers of that shape — movement collision (`BlockShapeSource::properties_at`), placement obstruction (`is_placement_obstructed`), and `is_face_sturdy` (MECH-D84). All three want collision-shape semantics. This blueprint therefore registers **`BlockPhysicsProperties::air()`** (empty shape, friction `0.6`, speed/jump factor `1.0`) for every one of the ~396 button and plate states, enumerated by iterating each block's own `range_of` span. An explicit row per state is mandatory, not optional: `ShapeTable::lookup`'s fallback for an unregistered id is `default_full_cube()`, which would make an unregistered button a solid, `Full`-sturdy cube — the worst possible wrong answer.

Consequences, all correct and all intended: nothing can stand on a pressure plate (no plate-on-plate stack, no floor torch on a plate); nothing attaches to a button; a button may be placed into the cell a player occupies (vanilla's `isUnobstructed` uses the same empty collision shape); and a player walking onto a plate is never lifted by it, which is what makes §E's overlap arithmetic work.

**The outline geometry is still derived and pinned as verified fact**, because the deferred arrow path needs it (`checkPressed` searches `state.getShape(level, pos).bounds().move(pos)`) and because losing the derivation would cost more than recording it. It is **not** shipped as a function in this blueprint. A button's outline is `Shapes.join(rotate_attach_face(Block.boxZ(6, 4, 8, 16))[face][facing], Block.cube(powered ? 14 : 12), ONLY_FIRST)`; the identity (`wall`/`north`) case evaluates to `x 5..11, y 6..10, z 8..16` minus the centred cube, i.e. **`x 5..11, y 6..10, z 14..16` unpressed and `z 15..16` pressed** (in sixteenths) — the pressed button protrudes one pixel where the unpressed one protrudes two. Every other `(face, facing)` pair is the same box under `Shapes.rotateAttachFace`'s rotation, following the identical rotation rules `rc_physics::shapes::lever_shape`'s own doc comment already spells out for the lever's `boxZ(6, 8, 10, 16)` base. A plate's outline is `Block.column(14, 0, 1)` unpressed — `x/z 1..15, y 0..1` — and `Block.column(14, 0, 0.5)` pressed, the same footprint at **half** a pixel high, not one pixel.

### H. Placement — a bounded six-kind representative subset

`PlaceableBlockKind` is a hand-authored enum backing `dig_properties`, `resolve_orientation`, `tier1_oriented_entries` and `placeable_kind_for_item_id`; every variant costs a row in four closed tables. Adding all 30 blocks would be 30 rows in each for zero additional behavioural coverage, so placement is exposed for exactly the six kinds that span every distinct behavioural class:

| New `PlaceableBlockKind` | Block | Class it covers |
|---|---|---|
| `StoneButton` | `minecraft:stone_button` | 20-tick, no arrows, stone sounds |
| `OakButton` | `minecraft:oak_button` | 30-tick, arrow-capable, wooden sounds |
| `StonePressurePlate` | `minecraft:stone_pressure_plate` | boolean plate, `MOBS` sensitivity |
| `OakPressurePlate` | `minecraft:oak_pressure_plate` | boolean plate, `EVERYTHING` sensitivity |
| `LightWeightedPressurePlate` | `minecraft:light_weighted_pressure_plate` | analog, `max_weight` 15, 10-tick cadence |
| `HeavyWeightedPressurePlate` | `minecraft:heavy_weighted_pressure_plate` | analog, `max_weight` 150 |

Every other button and plate still has full behaviour, signals, shapes and dispatch — only its creative-slot placement row is absent, which is a bounded interim exactly like `HeldItemStub`'s own 13-item universe.

**Orientation.** Buttons use `FaceAttachedHorizontalDirectionalBlock.getStateForPlacement`, which is the *same function* the lever uses. `resolve_orientation`'s existing `PlaceableBlockKind::Lever` arm is widened to `Lever | StoneButton | OakButton` verbatim — the six-candidate loop over `ordered_by_nearest(look)` with the clicked face's own opposite moved to the front; a vertical candidate resolves `face = Ceiling`/`Floor` with `facing =` the player's own horizontal direction (never its opposite); a horizontal candidate resolves `face = Wall, facing = dir.opposite()`; every candidate's own check is `is_sturdy_at(dir, SupportKind::Full)`; the first valid candidate wins; none valid returns `Err(RejectReason::InvalidLeverFace)` — reused rather than renamed, since the reject reason is a wire-invisible diagnostic and renaming it would touch already-merged tests. Plates resolve `Orientation::None` with no look input at all.

**Placement-time support check.** Plates need the `NoSolidSupportBelow` gate `apply_placement_with_redstone` already applies to wire/repeater/comparator, but with `canSurvive`'s two-kind OR: `is_face_sturdy(below, Up, Rigid) || is_face_sturdy(below, Up, Center)`. Added as a second, additive `if matches!(kind, <the four plate kinds>)` block beside the existing one, using the same `rc_mechanics::redstone::signal::is_face_sturdy` the engine-side pop check uses, so placement and pop can never disagree (MECH-D84's own requirement). Buttons need no such block — their refusal is already inside `resolve_orientation`'s candidate loop, exactly like the lever's and the torch's.

**Oriented state table.** `tier1_oriented_entries` gains 24 button rows (2 kinds × 3 faces × 4 facings, `powered=false` only — a freshly placed button is never pressed; `on_use` writes the pressed sibling through `with_property`) and 4 plate rows (`Orientation::None` → each plate's own generated default state: `powered=false` / `power=0`).

**Dig properties.** All six rows are `DigProperties { hardness: 0.5, effective_tool: ToolKind::None, min_tier_for_drops: None }` — `strength(0.5F)` for every button and plate, and **no** button or plate sets `requiresCorrectToolForDrops`, so a stone button and a stone pressure plate both drop from a bare hand. Identical in shape to the lever's own row.

**Item mapping.** `placeable_kind_for_item_id` gains six rows from the generated `item` table (`STONE_BUTTON`, `OAK_BUTTON`, `STONE_PRESSURE_PLATE`, `OAK_PRESSURE_PLATE`, `LIGHT_WEIGHTED_PRESSURE_PLATE`, `HEAVY_WEIGHTED_PRESSURE_PLATE`).

### I. Piston interaction

Every button and every plate is registered with `PushReaction.DESTROY`. `redstone::piston::classify` already handles exactly this for the lever, via a `range_of(block_id::LEVER)` containment test bolted on beside the `DESTROY_IDS` literal set. This blueprint generalizes that one check into a `DESTROY_RANGE_BLOCK_IDS: &[BlockId]` constant — the lever plus all 14 button and all 16 plate block ids — iterated in a loop, preserving the lever's own behaviour exactly and adding the other 30. `DESTROY_IDS` is untouched.

`affectNeighborsAfterRemoval`'s `!movedByPiston` guard means a piston-destroyed pressed button/plate does **not** re-notify. This engine's piston already writes the destroyed cell through its own path without invoking an `on_removed` hook (none exists), so that guard is satisfied structurally, not by a check.

### J. Reconciliation items for the planning role (ledger entries, not decisions)

Each is appended verbatim to `docs/findings-for-planning.md` by this blueprint's own governance/test-authoring changeset. None is decided here.

1. **`ShapeTable` conflates the collision shape with the outline shape.** MECH-D84 says the table "carries the true shape of every tier-1 state", but face sturdiness in the reference is computed from `getBlockSupportShape`, which defaults to the *collision* shape and is empty for every `noCollision` block. This blueprint registers empty shapes for buttons and plates (§G, the parity-exact answer for all three consumers). The already-merged rows for `redstone_wire`, `redstone_torch`/`redstone_wall_torch` and `lever` — all three `noCollision` in vanilla — instead store outline boxes, which makes each of them `Center`-sturdy on at least one face in this engine and sturdy on none in vanilla (a floor torch can stand on a lever here and cannot there). Planning: decide whether MECH-D84 gains a support-shape/outline split, and whether those three rows change.
2. **`LeverBehavior::on_use` guards on `may_build`; `LeverBlock.useWithoutItem` has no such guard.** Unobservable today (`may_build` is unconditionally `true`), but MECH-D82's decision text lists the guard as a repeater/comparator property, and the lever adopted it without a reference basis. This blueprint's button follows the reference and adds no guard.
3. **`ContraptionSpec`/`ScriptedAction` can only drive a `/setblock` state swap.** A button's press-and-release *timing* is therefore not corpus-verifiable at all: a `/setblock`-written `powered=true` button schedules nothing in the oracle either, so the fixture can pin the signal semantics but never the auto-off delay (§ Acceptance tests covers timing with unit and real-connection tests instead). Planning: decide whether the corpus schema grows a `use` action.
4. **No `on_removed`/`affectNeighborsAfterRemoval` hook exists.** §C states the exact bounded gap for a broken pressed button.
5. **`entity_inside` enumeration is post-tick-AABB-only, not swept.** §E states the exact bounded simplification and the condition under which it stops being equivalent (an entity moving more than one block per tick, i.e. projectiles or teleports).
6. **Arrow-triggered wooden-button presses are deferred** to whichever blueprint first ships a projectile entity; the seam is already wired (§A).

### Claims to verify (TEST-D57)

- ButtonBlock extends FaceAttachedHorizontalDirectionalBlock and declares exactly three block-state properties -- FACING over the four horizontals, POWERED as a boolean, and FACE over floor/wall/ceiling -- giving 24 states per button block.
- Every button block's registered default state is FACING=north, POWERED=false, FACE=wall.
- minecraft:stone_button and minecraft:polished_blackstone_button are both constructed with BlockSetType.STONE and a ticks_to_stay_pressed of 20.
- All twelve wooden button blocks -- oak, spruce, birch, jungle, acacia, cherry, dark_oak, pale_oak, mangrove, bamboo, crimson and warped -- are constructed with a ticks_to_stay_pressed of 30.
- Blocks.buttonProperties gives every button block noCollision.
- Blocks.buttonProperties gives every button block strength 0.5 for both hardness and blast resistance.
- Blocks.buttonProperties gives every button block PushReaction.DESTROY.
- No button block sets requiresCorrectToolForDrops.
- ButtonBlock.useWithoutItem returns a consuming result without pressing when POWERED is already true.
- ButtonBlock.useWithoutItem calls press and returns a success result when POWERED is false.
- ButtonBlock.useWithoutItem carries no mayBuild guard of any kind.
- ButtonBlock.press performs, in this exact order: setBlock with POWERED=true at update flags 3, then updateNeighbours, then scheduleTick with a delay of ticks_to_stay_pressed, then playSound, then the BLOCK_ACTIVATE game event.
- ButtonBlock.updateNeighbours computes front as getConnectedDirection(state).getOpposite() and calls updateNeighborsAt both at the button's own position and at pos.relative(front), the mount cell.
- ButtonBlock.playSound passes the acting player as the excluded listener when pressing and null when releasing, so the presser does not hear their own press while every listener hears the release.
- ButtonBlock.getSound returns the block set type's buttonClickOn when pressed and buttonClickOff when released.
- Button and pressure-plate sounds are emitted through LevelAccessor's four-argument playSound overload, whose defaults are volume 1.0 and pitch 1.0; neither block passes an explicit volume or pitch.
- ButtonBlock.ownSignal returns 15 while POWERED and 0 otherwise, with no direction exclusion of any kind.
- ButtonBlock.getDirectSignal returns 15 only when POWERED and the queried direction equals getConnectedDirection(state).
- ButtonBlock.isSignalSource returns true.
- FaceAttachedHorizontalDirectionalBlock.getConnectedDirection returns UP for FACE=floor, DOWN for FACE=ceiling, and the FACING value itself for FACE=wall.
- FaceAttachedHorizontalDirectionalBlock.canSurvive calls canAttach with getConnectedDirection(state).getOpposite(), and canAttach requires the mount block's own face toward the attached block to be sturdy for SupportType.FULL, for all three attach faces alike.
- FaceAttachedHorizontalDirectionalBlock.updateShape replaces the block with air only when the shape update arrives from the mount direction and canSurvive is false.
- ButtonBlock.tick calls checkPressed only when POWERED is currently true.
- ButtonBlock.checkPressed looks for the first AbstractArrow inside the button's own outline shape moved to its position, and only when the block set type's canButtonBeActivatedByArrows is true; otherwise no arrow is ever searched for or found.
- ButtonBlock.checkPressed writes the new POWERED value at update flags 3, calls updateNeighbours and plays the click sound with no excluded listener, but only when the computed pressed state differs from the stored one.
- ButtonBlock.checkPressed re-schedules a tick ticks_to_stay_pressed out whenever the computed pressed state is true, and schedules nothing when it is false.
- ButtonBlock.checkPressed fires GameEvent.BLOCK_ACTIVATE when the computed pressed state becomes true and GameEvent.BLOCK_DEACTIVATE when it becomes false, in addition to writing the block state, calling updateNeighbours and playing the click sound.
- ButtonBlock.entityInside triggers checkPressed only on the server, only when the block set type's canButtonBeActivatedByArrows is true, and only when POWERED is currently false.
- BlockSetType.canButtonBeActivatedByArrows is true for every wooden block set type and false for both the stone and the polished_blackstone block set types.
- The stone and polished_blackstone block set types use SoundEvents.STONE_BUTTON_CLICK_ON and STONE_BUTTON_CLICK_OFF.
- The oak, spruce, birch, jungle, acacia, dark_oak, pale_oak and mangrove block set types all use SoundEvents.WOODEN_BUTTON_CLICK_ON and WOODEN_BUTTON_CLICK_OFF; none of them has a per-wood-species button sound.
- The cherry block set type uses CHERRY_WOOD_BUTTON_CLICK_ON and OFF, bamboo uses BAMBOO_WOOD_BUTTON_CLICK_ON and OFF, and crimson and warped both use NETHER_WOOD_BUTTON_CLICK_ON and OFF.
- ButtonBlock.affectNeighborsAfterRemoval calls updateNeighbours when the removed state was POWERED and the removal was not caused by a piston.
- A button's outline shape is the ONLY_FIRST join of the attach-face-rotated base box Block.boxZ(6.0, 4.0, 8.0, 16.0) with a centred cube that is Block.cube(14.0) when POWERED and Block.cube(12.0) when not.
- Block.boxZ(6.0, 4.0, 8.0, 16.0) evaluates to the box spanning x 5 to 11, y 6 to 10 and z 8 to 16 in sixteenths.
- Shapes.rotateAttachFace's identity entry is the FACE=wall, FACING=north orientation.
- Subtracting the centred cube leaves a wall-north button outline of x 5 to 11, y 6 to 10, z 14 to 16 when unpressed and x 5 to 11, y 6 to 10, z 15 to 16 when pressed, so a pressed button protrudes one pixel where an unpressed one protrudes two.
- BasePressurePlateBlock's unpressed outline shape is Block.column(14.0, 0.0, 1.0) -- x and z from 1 to 15, y from 0 to 1 in sixteenths -- and its pressed shape is Block.column(14.0, 0.0, 0.5), the same footprint at half that height.
- BasePressurePlateBlock's entity-detection box TOUCH_AABB is Block.column(14.0, 0.0, 4.0), spanning x and z from 1 to 15 and y from 0 to 4 in sixteenths, and it is the only detection box the class defines.
- BasePressurePlateBlock.getPressedTime returns 20, and WeightedPressurePlateBlock overrides it to return 10.
- BasePressurePlateBlock.canSurvive is true when the block below is sturdy on its own top face for SupportType.RIGID or for SupportType.CENTER -- an OR of two support kinds, not RIGID alone.
- BasePressurePlateBlock.updateShape replaces the plate with air only when the shape update arrives from DOWN and canSurvive is false.
- BasePressurePlateBlock.tick calls checkPressed only when the stored state's own signal is greater than zero.
- BasePressurePlateBlock.entityInside calls checkPressed only on the server and only when the stored state's own signal is exactly zero.
- BasePressurePlateBlock.checkPressed writes the recomputed signal into the block state at update flag 2 -- a client update with no updateNeighborsAt fan-out -- and then calls updateNeighbours explicitly, and does both only when the recomputed signal differs from the stored one.
- BasePressurePlateBlock.updateNeighbours calls updateNeighborsAt at the plate's own position and at the position directly below it.
- BasePressurePlateBlock.checkPressed plays the block set type's pressurePlateClickOff when the plate goes from pressed to unpressed and pressurePlateClickOn when it goes from unpressed to pressed, both with no excluded listener, and plays nothing when a nonzero signal merely changes to a different nonzero value.
- BasePressurePlateBlock.checkPressed schedules a tick getPressedTime ticks out whenever the recomputed signal is greater than zero.
- BasePressurePlateBlock.checkPressed fires GameEvent.BLOCK_ACTIVATE when the recomputed signal becomes pressed and GameEvent.BLOCK_DEACTIVATE when it becomes unpressed, in addition to writing the block state and playing the click sound.
- BasePressurePlateBlock.ownSignal returns the state's own signal toward every direction.
- BasePressurePlateBlock.getDirectSignal returns the state's own signal only for the queried direction UP.
- BasePressurePlateBlock.isSignalSource returns true.
- BasePressurePlateBlock.getEntityCount counts entities of the requested class inside the given box filtered by EntitySelector.NO_SPECTATORS and by the entity not ignoring block triggers.
- PressurePlateBlock carries the single boolean property POWERED, whose signal is 15 when true and 0 when false.
- PressurePlateBlock.getSignalStrength counts entities of class Entity when the block set type's pressure-plate sensitivity is EVERYTHING and of class LivingEntity when it is MOBS, returning 15 when the count is greater than zero and 0 otherwise.
- BlockSetType.PressurePlateSensitivity has exactly two values, EVERYTHING and MOBS.
- The stone and polished_blackstone block set types have pressure-plate sensitivity MOBS, while every wooden block set type plus iron, gold and copper have sensitivity EVERYTHING.
- WeightedPressurePlateBlock carries the single integer property POWER over the range 0 to 15 and uses that value directly as its signal.
- WeightedPressurePlateBlock.getSignalStrength always counts entities of class Entity regardless of its own block set type's sensitivity.
- WeightedPressurePlateBlock.getSignalStrength clamps the entity count to max_weight, divides that by max_weight as a float, multiplies by 15.0 and takes the ceiling, returning 0 when the count is zero.
- minecraft:light_weighted_pressure_plate is a WeightedPressurePlateBlock with max_weight 15 and BlockSetType.GOLD, and minecraft:heavy_weighted_pressure_plate has max_weight 150 and BlockSetType.IRON.
- The gold and iron block set types use METAL_PRESSURE_PLATE_CLICK_ON and OFF; stone and polished_blackstone use STONE_PRESSURE_PLATE_CLICK_ON and OFF; the eight plain wooden types use WOODEN_PRESSURE_PLATE_CLICK_ON and OFF; cherry, bamboo, crimson and warped use their own CHERRY_WOOD, BAMBOO_WOOD and NETHER_WOOD variants.
- Every pressure plate block is registered with noCollision.
- Every pressure plate block has strength 0.5 for both hardness and blast resistance.
- Every pressure plate block is registered with PushReaction.DESTROY.
- None of the pressure plate blocks sets requiresCorrectToolForDrops.
- BasePressurePlateBlock.affectNeighborsAfterRemoval calls updateNeighbours when the removed state's own signal was greater than zero and the removal was not caused by a piston.
- Neither pressure plate class carries any orientation property: PressurePlateBlock's only property is POWERED and WeightedPressurePlateBlock's only property is POWER.
- BlockBehaviour.getBlockSupportShape defaults to the block's own collision shape, and BlockBehaviour.getCollisionShape returns an empty shape whenever the block was registered with noCollision, so a noCollision block is sturdy on no face for any SupportType.
- All three SupportType variants -- FULL, CENTER and RIGID -- evaluate against getBlockSupportShape, never against the block's outline or visual shape.
- Entity.checkInsideBlocks deflates the entity's own bounding box by 1.0E-5 before enumerating the block positions it intersects.
- BlockBehaviour.getEntityInsideCollisionShape returns the full block cube by default, and Entity.checkInsideBlocks short-circuits its intersection test to true whenever that shape is the full cube, so a button or plate counts an entity as inside whenever the entity's deflated box intersects the block's whole cell.
- Entity.checkInsideBlocks visits each intersected block position at most once per movement, tracked in a visited-position set.
- Entity.checkInsideBlocks sweeps the movement segment through forEachBlockIntersectedBetween, but the 16-iteration cap is a hard budget for the whole movement rather than a per-segment allowance: the sweep visits the cells along the path only up to that budget, after which the caller re-checks only the destination box, so a movement needing more than 16 traversal steps has its tail cells skipped; a teleport resets the old position and produces no Entity.Movement, so it produces no sweep at all.
- AABB.intersects tests strict inequality on both bounds -- the queried box's min less than the other box's max, and its max greater than the other box's min -- never inclusive equality at either bound.
- Level.setBlock with update flag bit 2 set sends a client block update but performs no updateNeighborsAt call, while the neighbour shape updates still run because they are gated on flag bit 16 being clear.
- Level.setBlock with update flags 3 performs both the client update and the six-neighbour updateNeighborsAt fan-out.
- The three-argument level.scheduleTick(pos, block, delay) used by both ButtonBlock and BasePressurePlateBlock schedules at TickPriority.NORMAL.
- ButtonBlock.onExplosionHit presses an unpressed button whenever the explosion can trigger blocks.

## Deliverables

### `crates/mechanics/src/behavior.rs` (modify)

```rust
pub struct UpdateContext<'a> {
    // ... nine existing fields, unchanged ...
    /// M4-B10 (Context §F): the Stage-4-reachable sound outbox. Threaded exactly like
    /// `changed`/`light_dirty`; merged into `stage4::ecs::TickSoundOutbox` by every Stage-4
    /// system and drained once per tick by `crates/server/src/play/world.rs`. Absorbs
    /// `UseUpdateContext`'s own former duplicate outbox — there is now exactly one.
    pub sounds: &'a mut Vec<SoundRequest>,
}

impl<'a> UpdateContext<'a> {
    /// Queues one clientbound `sound` packet request.
    pub fn request_sound(&mut self, request: SoundRequest);
}

/// One entity touching a block cell, as `on_entity_inside` sees it (Context §E). Carries no
/// ECS handle and no `rc-core` entity id: a `BlockBehavior` never addresses an entity, it only
/// learns that one is present, and the census (`EntityPresenceSource`) answers everything else.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EntityTouch {
    pub aabb: rc_physics::Aabb,
    pub is_living: bool,
}

pub trait BlockBehavior: Send + Sync {
    // ... existing hooks, unchanged ...
    /// M4-B10 (Context §E): `BlockState.entityInside`. Called once per (entity, intersected
    /// cell) pair per tick by `rusty-clanker-server::play::entity_presence::entity_inside_step`.
    /// Default no-op — additive and backward-compatible, mirroring `on_random_tick`/`on_use`.
    fn on_entity_inside(&self, _ctx: &mut UpdateContext, _pos: BlockPos, _entity: &EntityTouch) {}
}

pub struct UseUpdateContext<'a, 'b> {
    pub base: UpdateContext<'a>,
    // `sounds` field REMOVED (Context §F); the `'b` lifetime parameter is retained so the
    // type's own public shape and every existing `UseUpdateContext<'_, '_>` mention keep
    // compiling unchanged.
}
impl<'a, 'b> UseUpdateContext<'a, 'b> {
    /// Forwards to `self.base.request_sound` (Context §F).
    pub fn request_sound(&mut self, request: SoundRequest);
}
```

### `crates/mechanics/src/stage4.rs` and `crates/mechanics/src/stage4/ecs.rs` (modify)

```rust
// stage4.rs: `make_ctx`, `dispatch_scheduled_tick`, `drain_engine`, `run_scheduled_phase`,
// `run_block_event_subphase` each gain one `sounds: &mut Vec<SoundRequest>` parameter,
// threaded through unchanged — the identical mechanical widening `light_dirty` already got.

// stage4/ecs.rs:
/// M4-B10 (Context §F): tick-wide accumulation of every `SoundRequest` any Stage-4 system
/// queued this tick, merged from each system's own local collector — mirrors
/// `TickChangedPositions`/`TickBlockEventOutbox` exactly.
#[derive(Default, Resource)]
pub struct TickSoundOutbox(pub Vec<SoundRequest>);
impl TickSoundOutbox {
    pub fn merge(&mut self, incoming: Vec<SoundRequest>);
    pub fn drain(&mut self) -> Vec<SoundRequest>;
}
// `bootstrap_default_stage4_resources` also inserts `TickSoundOutbox::default()`.
```

### `crates/mechanics/src/redstone/entity_presence.rs` (new)

```rust
/// Which entity class a pressure plate counts (Context §D) — vanilla's `Entity.class` vs
/// `LivingEntity.class` switch on `BlockSetType.pressurePlateSensitivity`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EntityClassFilter { AnyEntity, LivingOnly }

/// The census seam a pressure plate reads through (Context §E). Mirrors
/// `ContainerSignalSource`'s shape, ownership and locking rationale exactly. The one
/// production implementation lives in `rusty-clanker-server` (the only crate that can see
/// both players and `BaseEntity` entities); `rc-mechanics` ships only this trait and the
/// empty default below.
pub trait EntityPresenceSource: Send + Sync {
    /// Vanilla's `getEntityCount(level, box, class)`: owned (ARCH-D10), non-spectator,
    /// non-block-trigger-ignoring entities whose own AABB strictly intersects `region`.
    fn count_entities_in(&self, region: rc_physics::Aabb, filter: EntityClassFilter) -> usize;
}

/// Always `0` — the composition-root default until a real census is wired, mirroring
/// `NoContainers`. A plate reading this never presses; it never panics.
pub struct NoEntities;
impl EntityPresenceSource for NoEntities { /* returns 0 */ }
```

### `crates/mechanics/src/redstone/button.rs` (new)

```rust
/// One button block's own immutable per-block configuration (Context §C's table).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ButtonKindConfig {
    pub ticks_to_stay_pressed: u64,
    pub can_be_activated_by_arrows: bool,
    pub click_on: RegistryEntryId,
    pub click_off: RegistryEntryId,
}

/// Context §C's own complete 14-row table, block id + config, every id read from the
/// generated registries.
pub const BUTTON_BLOCKS: &[(BlockId, ButtonKindConfig)];

/// Stateless (Context §B) — one instance per button block, every read decoded from the
/// world's own stored `BlockStateId`.
pub struct ButtonBehavior { /* config: ButtonKindConfig */ }
impl ButtonBehavior {
    pub fn new(config: ButtonKindConfig, block: BlockId) -> Self;
}
impl RedstoneSignalSource for ButtonBehavior { /* weak_signal_toward, direct_signal_toward, is_signal_source */ }
impl BlockBehavior for ButtonBehavior {
    /// `updateShape` — pops to air when the update arrives from the mount direction and the
    /// mount face is no longer `Full`-sturdy (Context §C).
    fn on_shape_update(&self, ..) -> Option<BlockStateId>;
    /// `useWithoutItem` — `Consumed` without effect when already pressed, else `press`
    /// (Context §C, exact five-step order).
    fn on_use(&self, ..) -> UseOutcome;
    /// `tick` — `check_pressed` only when POWERED (Context §C).
    fn on_scheduled_tick(&self, ..);
}
```

### `crates/mechanics/src/redstone/pressure_plate.rs` (new)

```rust
/// Which of the two `getSignalStrength` implementations a plate uses (Context §D).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlateSignalModel {
    /// `PressurePlateBlock`: boolean POWERED, 15 or 0, class chosen by `sensitivity`.
    Boolean { sensitivity: EntityClassFilter },
    /// `WeightedPressurePlateBlock`: integer POWER, always `AnyEntity`, ceil-scaled.
    Weighted { max_weight: u32 },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PlateKindConfig {
    pub model: PlateSignalModel,
    /// 20 for every `PressurePlateBlock`, 10 for both weighted plates (Context §D).
    pub pressed_time: u64,
    pub click_on: RegistryEntryId,
    pub click_off: RegistryEntryId,
}

/// Context §D's own complete 16-row table.
pub const PRESSURE_PLATE_BLOCKS: &[(BlockId, PlateKindConfig)];

/// `WeightedPressurePlateBlock.getSignalStrength`'s exact arithmetic, pure and directly
/// unit-testable: `0` for `count == 0`, else `ceil(min(count, max_weight) as f32 / max_weight
/// as f32 * 15.0)`.
pub fn weighted_plate_signal(count: usize, max_weight: u32) -> u8;

pub struct PressurePlateBehavior { /* config + Arc<dyn EntityPresenceSource> */ }
impl PressurePlateBehavior {
    pub fn new(config: PlateKindConfig, block: BlockId, entities: Arc<dyn EntityPresenceSource>) -> Self;
}
impl RedstoneSignalSource for PressurePlateBehavior { /* weak = signal all round; direct = signal only towards Down */ }
impl BlockBehavior for PressurePlateBehavior {
    /// `updateShape` — pops to air when the update arrives from `Down` and neither
    /// `Rigid`- nor `Center`-sturdiness holds on the block below's top face (Context §D).
    fn on_shape_update(&self, ..) -> Option<BlockStateId>;
    /// `tick` — `check_pressed` only when the stored signal is `> 0` (Context §D).
    fn on_scheduled_tick(&self, ..);
    /// `entityInside` — `check_pressed` only when the stored signal is `0` (Context §D).
    fn on_entity_inside(&self, ..);
}
```

### `crates/mechanics/src/redstone/registration.rs` (modify — additive)

```rust
/// Context §B: constructs one `ButtonBehavior` per `BUTTON_BLOCKS` row and one
/// `PressurePlateBehavior` per `PRESSURE_PLATE_BLOCKS` row, registering each into both
/// registries over that block's own `range_of` span. Call once per region, in any order
/// relative to `register_tier1_redstone`/`register_piston`/`register_hopper` — these
/// behaviours need no `SignalSourceRegistry` back-reference, so they take no part in
/// `Tier1RedstoneHandles::bind_registry` and return no handle.
pub fn register_tier2_inputs(
    behaviors: &mut BlockBehaviorRegistry,
    signals: &mut SignalSourceRegistry,
    entities: Arc<dyn EntityPresenceSource>,
);
```

### `crates/mechanics/src/redstone/piston.rs` (modify — one generalization)

```rust
/// Context §I: the lever plus all 14 button and all 16 plate block ids, each contributing its
/// whole generated `range_of` span to `classify`'s `PushClass::Destroy` set. Replaces the
/// single hard-coded lever range check; `DESTROY_IDS` is untouched.
const DESTROY_RANGE_BLOCK_IDS: &[BlockId];
```

### `crates/mechanics/src/entity/physics/mod.rs` and `ecs.rs` (modify — behaviour-preserving promotion)

```rust
/// M4-B02's own per-kind `(half_width, height)` table, promoted from `ecs.rs`'s private
/// `living_dimensions` so `rusty-clanker-server`'s entity census (M4-B10, Context §E) can
/// build the same AABBs. Values unchanged: Zombie/Villager `(0.3, 1.95)`, Cow `(0.45, 1.4)`,
/// Item `(ITEM_HALF_WIDTH, ITEM_HEIGHT)`.
pub fn entity_dimensions(kind: EntityKind) -> (f64, f64);
```

### `crates/physics/src/shapes.rs` (modify — additive rows only)

```rust
/// Context §G: every state of all 14 button and all 16 pressure-plate blocks, registered as
/// `BlockPhysicsProperties::air()` — an empty collision shape, which is also the support
/// shape, so both block families collide with nothing and are sturdy on no face. An explicit
/// row per state is mandatory: `lookup`'s fallback is `default_full_cube()`. Built by
/// iterating each block's own `range_of` span; ~396 rows, no per-state literal.
const NON_COLLIDING_INPUT_BLOCK_IDS: &[BlockId];
```

### `crates/server/src/play/entity_presence.rs` (new)

```rust
/// One censused entity (Context §E). `is_spectator`/`ignores_block_triggers` are always
/// `false` at M4's own entity set and exist so a future mechanic flips a value rather than
/// discovering the filter was never modelled.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EntityPresenceRecord {
    pub aabb: rc_physics::Aabb,
    pub is_living: bool,
    pub is_spectator: bool,
    pub ignores_block_triggers: bool,
}

/// The region's own `EntityPresenceSource` (Context §E). One instance per region, shared via
/// two `Arc` clones with `register_tier2_inputs` (read side) and `entity_inside_step` (write
/// side) — the identical shape `Tier1ContainerSignalSource` established. The `Mutex` is never
/// contended: refresh and Stage-4 reads run strictly sequentially within one region's tick.
pub struct RegionEntityPresence { /* Mutex<Vec<EntityPresenceRecord>> */ }
impl RegionEntityPresence {
    pub fn new() -> Self;
    /// Replaces the whole census. Called once per tick, first thing in `entity_inside_step`.
    pub fn refresh(&self, records: Vec<EntityPresenceRecord>);
    /// Read-only snapshot of the current census — the driver's own cell-enumeration input.
    pub fn snapshot(&self) -> Vec<EntityPresenceRecord>;
}
impl rc_mechanics::redstone::EntityPresenceSource for RegionEntityPresence { .. }

/// `Resource` wrapper, inserted by the composition root (mirrors `ContainerSignalsResource`).
#[derive(Resource, Clone)]
pub struct EntityPresenceResource(pub std::sync::Arc<RegionEntityPresence>);

/// Context §E's manual tick-loop step: refresh the census, then dispatch
/// `BlockBehavior::on_entity_inside` at every cell any censused entity's own
/// `1.0E-5`-deflated AABB intersects (deduplicated per entity), settling the neighbour-update
/// engine to a fixed point after each dispatch, and merging `changed`/`sounds` into
/// `TickChangedPositions`/`TickSoundOutbox`. Runs after `entity_pickup_step`/
/// `entity_resync_step` and before those two drains.
pub fn entity_inside_step(world: &mut bevy_ecs::world::World, current_tick: u64);
```

### `crates/server/src/play/mining.rs` (modify)

Six new `PlaceableBlockKind` variants (§H); their `dig_properties` row (all six identical: hardness `0.5`, `ToolKind::None`, `min_tier_for_drops: None`); `resolve_orientation`'s `Lever` arm widened to `Lever | StoneButton | OakButton`; a new plate arm returning `Orientation::None`; the additive plate support-check block in `apply_placement_with_redstone` (`Rigid || Center` on the block below's top face); 24 button + 4 plate rows in `tier1_oriented_entries`; six rows in `placeable_kind_for_item_id`.

### `crates/server/src/play/world.rs` (modify)

`bootstrap_redstone_dispatch` constructs `Arc<RegionEntityPresence>`, calls `register_tier2_inputs` with one clone and inserts `EntityPresenceResource` with the other; `bootstrap_default_stage4_resources`'s new `TickSoundOutbox` is drained once per tick right after `TickBlockEventOutbox`, each request broadcast through the existing `broadcast_sound_request` with actor id `-1`; `entity_inside_step` is called after `entity_resync_step`; three test/diagnostic accessors mirroring every prior `debug_*` precedent:

```rust
impl HardcodedWorld {
    /// Current `powered`/`power` value decoded from whatever is stored at `pos`; `None` if the
    /// stored id is not a button or plate.
    pub fn debug_query_input_signal(&self, pos: BlockPos) -> Option<u8>;
    /// Ticks until `pos`'s own queued block tick fires; `None` if none is queued.
    pub fn debug_pending_block_tick_delay(&self, pos: BlockPos) -> Option<u64>;
    /// Current entity census size intersecting `region` — the plate's own input, exposed so a
    /// test can assert the census, not only its consequence.
    pub fn debug_entity_census(&self, region: rc_physics::Aabb, filter: EntityClassFilter) -> usize;
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46), restated exactly:** every file below, plus every `src/*.rs` item Deliverables lists with each function body replaced by `todo!()` (fields, derives and doc comments unchanged), plus the mechanical `sounds:` field addition at every existing `UpdateContext { .. }` construction site (§F), is the **test-authoring changeset**, committed first with subject `M4-B10 test-authoring: …`. The implementation changeset (subject `M4-B10 implementation: …`) fills bodies only: it must not modify any file listed here, must not add, remove or rename a test case, and must not weaken any expected value.

Every new `crates/{server,mechanics}/tests/` file below matches a TEST-D55 mechanic-name pattern (`redstone_`, `play_redstone_`, `mining_`, `play_block_`) and therefore carries a `//! test-matrix: boundaries=<v> orientations=<v> self=<v> composition=<v> nondefault-state=<v>` header line within its first 40 lines, with the verdicts named per file below and a `yes` category backed by a test name containing that category's required token (`_boundary_`, `_orientation_`/`_facing_`, `_self_`, `_chain_`/`_composition_`, `_nondefault_`).

### `crates/mechanics/tests/redstone_button.rs` (pure, `FakeWorld` + `UpdateContext`, mirroring `redstone_lever.rs`)

*Header:* `boundaries=waived(fixed local test-world positions, never near the Y limits, see world_bounds_fan_out.rs) orientations=yes self=waived(no player/actor entity in this suite's own domain model) composition=waived(single component per case, no >=3-component chain, see redstone_wire.rs) nondefault-state=yes`

1. `press_powers_the_button_and_schedules_its_release` — a wall button on stone; `on_use` with `may_build: true`; assert the stored state's `powered` is now `true`, that exactly one block tick is queued at that position, and that its delay is 20 (stone).
2. `wooden_button_release_delay_is_thirty_ticks_nondefault_material` — the same with an oak button; assert the queued delay is 30.
3. `pressing_an_already_pressed_button_consumes_without_rescheduling` — `powered=true` already; `on_use`; assert `UseOutcome::Consumed`, no state change, no new scheduled tick, no sound request.
4. `scheduled_tick_releases_the_button_and_plays_click_off` — pressed button; run its scheduled tick; assert `powered=false`, exactly one queued `SoundRequest` carrying `click_off` with `except_actor == false`, volume `1.0`, pitch `1.0`, and that no further tick is scheduled.
5. `press_sound_excludes_the_actor_and_carries_the_material_sound` — assert the press request's `sound == BLOCK_STONE_BUTTON_CLICK_ON` (stone) / `BLOCK_WOODEN_BUTTON_CLICK_ON` (oak), `except_actor == true`, volume `1.0`, pitch `1.0`.
6. `pressed_button_emits_weak_fifteen_toward_every_neighbour` — assert `weak_signal_toward` returns 15 for all six directions while pressed and 0 while unpressed.
7. `pressed_button_emits_direct_fifteen_only_toward_its_mount_in_every_orientation` — sweep all three attach faces × four facings; assert `direct_signal_toward` is 15 exactly for `mount_direction(face, facing)` and 0 for the other five.
8. `button_pops_when_its_mount_face_stops_being_full_sturdy` — wall button on stone; replace the stone with a non-`Full` block; `on_shape_update` from the mount direction returns `Some(AIR)`.
9. `button_ignores_a_shape_update_from_any_non_mount_direction` — the same setup, update from each of the other five directions; returns `None` every time.
10. `button_release_fans_out_at_its_own_cell_and_its_mount_cell` — instrument the neighbour-update engine; assert both `pos` and `mount_direction.apply(pos)` were notified on press and on release.
11. `arrow_capable_flag_matches_the_material_table` — assert `can_be_activated_by_arrows` is `false` for stone/polished blackstone and `true` for all twelve wooden configs, and that `check_pressed(.., arrow_present = false)` on a pressed button always releases it regardless of the flag.

### `crates/mechanics/tests/redstone_pressure_plate.rs` (pure)

*Header:* `boundaries=waived(fixed local test-world positions, see world_bounds_fan_out.rs) orientations=waived(a pressure plate has no orientation property at all, Context §D) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single plate per case, see redstone_wire.rs) nondefault-state=yes`

1. `weighted_plate_analog_table_is_exact` — a hand-computed table over `weighted_plate_signal(count, max_weight)`: `(0, 15) -> 0`, `(1, 15) -> 1`, `(7, 15) -> 7`, `(8, 15) -> 8`, `(15, 15) -> 15`, `(20, 15) -> 15`, `(1, 150) -> 1`, `(10, 150) -> 1`, `(11, 150) -> 2`, `(150, 150) -> 15`, `(500, 150) -> 15`.
2. `plate_presses_when_an_entity_enters_its_cell` — a stub `EntityPresenceSource` returning 1; `on_entity_inside` on an unpressed plate; assert the stored state became `powered=true`, one `click_on` request with `except_actor == false`, and one queued block tick with delay 20.
3. `weighted_plate_recheck_cadence_is_ten_ticks_nondefault_material` — the same with a gold plate; assert the queued delay is 10.
4. `plate_releases_on_its_scheduled_tick_once_the_census_is_empty` — pressed plate, census now 0; run its scheduled tick; assert `powered=false`, one `click_off` request, no new scheduled tick.
5. `pressed_plate_ignores_entity_inside_entirely` — a pressed plate with census 5; `on_entity_inside`; assert nothing changed, nothing scheduled, no sound — the press path is gated on `signal == 0`.
6. `weighted_plate_power_change_while_pressed_fans_out_without_a_sound` — a gold plate at `power=3`, census now 7; run its scheduled tick; assert the stored `power` became 7, that both `pos` and `pos.below()` were notified, and that **no** `SoundRequest` was queued.
7. `mobs_sensitivity_ignores_a_non_living_entity` — a stone plate whose census reports 0 living / 3 non-living; `on_entity_inside`; assert it stays unpressed. The same census against an oak plate presses it.
8. `plate_writes_without_its_own_fan_out_then_notifies_explicitly` — assert the plate's write went through `write_block_state` semantics: exactly one notify pass at `pos` (not the double-fire the button produces) plus one at `pos.below()`.
9. `plate_emits_weak_signal_all_round_and_direct_signal_only_downward` — a `power=9` gold plate; assert `weak_signal_toward` is 9 for all six directions and `direct_signal_toward` is 9 only for `Direction::Down`, 0 for the other five.
10. `plate_pops_only_when_the_block_below_is_neither_rigid_nor_center_sturdy` — three cases: a full cube below (survives), a hopper below (`Rigid` only — survives), a shape that is `Center`-sturdy but not `Rigid` (survives), and a non-sturdy shape (`on_shape_update` from `Down` returns `Some(AIR)`); plus one case asserting an update from any non-`Down` direction returns `None`.

### `crates/mechanics/tests/redstone_input_registration.rs` (pure)

*Header:* `boundaries=waived(no world position involved, registry construction only) orientations=waived(registration is per block id, not per facing) self=waived(no actor entity) composition=waived(no multi-component chain) nondefault-state=waived(every state in a registered range is covered by construction)`

1. `every_button_and_plate_state_id_resolves_to_its_own_behavior` — build both registries via `register_tier1_redstone` + `register_piston` + `register_hopper` + `register_tier2_inputs`; for every id in every button/plate `range_of` span, assert `BlockBehaviorRegistry::resolve` is not the `NoOpBehavior` default and `SignalSourceRegistry::resolve` reports `is_signal_source() == true`.
2. `registering_tier2_inputs_never_overlaps_an_existing_range` — the combined registration completes without the overlap panic, in both orders (tier-2 before and after tier-1/piston/hopper).
3. `every_button_and_plate_state_is_a_destroy_class_push_target` — for every id in every span, `piston::classify` returns `PushClass::Destroy`; the lever's own span still does too.

### `crates/physics/tests/input_component_shapes.rs` (pure)

*Header:* not required — `crates/physics/tests/` is outside TEST-D55's own `crates/{server,mechanics}/tests/` trigger scope (§2.4).

1. `every_button_and_plate_state_has_an_explicit_empty_shape_row` — for every id in every span, `tier1_shape_table().lookup(id).shape.is_empty()` is `true`.
2. `buttons_and_plates_are_sturdy_on_no_face_for_any_support_kind` — the full cross product of six faces × three `SupportKind`s over one representative state of each of the 30 blocks; every answer is `false`.

### `crates/server/tests/play_redstone_input_components.rs` (integration, real loopback connections, mirroring `play_lever_field_report.rs`)

*Header:* `boundaries=waived(fixed local test-world positions, see world_bounds_fan_out.rs) orientations=yes self=waived(placement into the actor's own cell is covered by mining_placement_obstruction.rs; buttons and plates are noCollision and never obstruct) composition=yes nondefault-state=yes`

1. `button_placement_orientation_over_a_real_connection` — place a stone button against a wall, a floor and a ceiling; assert the broadcast `Block Update` carries the exact `state_id` for `face=wall/floor/ceiling` with the expected `facing` and `powered=false`, resolved through `state_id(block_id::STONE_BUTTON, ..)`.
2. `pressing_a_button_powers_a_wire_chain_and_releases_on_schedule_composition` — button → wire → wire → lamp-substitute probe; press over the wire with `Use Item On`; assert the wire's power rises within the same tick, stays for exactly 19 further ticks, and is gone on the 20th tick after the press.
3. `wooden_button_release_is_thirty_ticks_nondefault_material` — the same with an oak button; assert the release lands on tick 30, not 20.
4. `button_press_sound_reaches_a_bystander_and_not_the_presser` — two connections; assert the bystander receives a `Sound` packet with the `stone_button.click_on` registry id and the presser receives none; assert **both** receive the `click_off` packet on release.
5. `player_standing_on_a_plate_powers_it_within_one_tick_and_releases_after_twenty` — place an oak plate on stone; walk the bot onto it with real `SetPlayerPosition` packets; assert the plate's `powered` flips on the next tick and that stepping off releases it exactly `20` ticks later.
6. `stone_plate_is_pressed_by_a_player_since_a_player_is_living` — the same on a stone (`MOBS`) plate; asserts the sensitivity mapping end to end.
7. `weighted_plate_analog_output_from_dropped_item_entities` — spawn N item entities (M4-B02's own `debug_spawn_item_entity`) on a light-weighted (gold, `max_weight` 15) plate for N in `{1, 7, 8, 16}`; after at most one re-check cadence, assert `debug_query_input_signal` reports `1, 7, 8, 15` respectively, and that the neighbouring wire reads the same value.
8. `weighted_plate_output_lags_a_count_change_by_at_most_the_recheck_cadence` — with the plate already pressed at power 1, drop six more items; assert the power is still 1 on the next tick and has become 7 by the 10th tick after the drop.
9. `plate_placement_is_refused_without_a_rigid_or_center_sturdy_block_below` — attempt to place a plate over air and over a non-sturdy shape; assert `PlaceOutcome::Rejected { reason: NoSolidSupportBelow, .. }` and MECH-D78's dual-cell resend; then place it on a hopper (`Rigid`-only) and assert it is accepted.
10. `breaking_the_support_pops_a_pressed_button_and_a_pressed_plate` — break the mount block under each; assert both cells become air and the wire they were powering drops to 0.

### Corpus fixtures (`crates/testing/gametest/corpus/redstone/`, plus regenerated `manifest.json`)

Every fixture drives the oracle with `ScriptedAction` state swaps only — the schema has no "use" action, so a button's *auto-off timing* is deliberately **not** corpus-covered (§J item 3); these fixtures pin signal semantics, fan-out and the support rule, which a state swap does reach.

1. `button_pressed_powers_adjacent_wire.ron` — a wall stone button on stone with a wire tile beside it; swap to `powered=true` at tick 3 and back at tick 5; category `QcShowcase`; mirrors `lever_toggle_powers_adjacent_wire.ron`'s geometry exactly.
2. `button_strong_powers_mount_block_wire_on_top.ron` — a wall button whose mount block carries a wire on top; the swap must raise that wire through the mount block's strong-signal relay, and must not raise a wire resting on a *different* neighbour.
3. `pressure_plate_strong_powers_block_below.ron` — an oak plate on stone with a wire on the stone's own far side; swap `powered=false → true → false`; pins "direct signal only downward".
4. `weighted_plate_analog_steps_into_comparator.ron` — a light-weighted plate feeding a comparator's back input; swap `power` through `0 → 1 → 8 → 15 → 0`; pins the analog read path end to end.

## Implementation steps

1. **`behavior.rs`.** Add `UpdateContext.sounds` + `request_sound`; add `EntityTouch` and the `on_entity_inside` default hook; strip `UseUpdateContext.sounds` and forward its `request_sound`. Observable: `cargo build -p rc-mechanics` fails only at the not-yet-updated `UpdateContext` construction sites.
2. **`stage4.rs` / `stage4/ecs.rs`.** Thread `sounds` through every helper; add `TickSoundOutbox` and its `bootstrap_default_stage4_resources` insert; merge the local collector in both Stage-4 systems. Observable: `rc-mechanics` builds; every pre-existing `rc-mechanics` test still passes.
3. **`entity/physics/{mod.rs, ecs.rs}`.** Promote `living_dimensions` to `pub fn entity_dimensions` and redirect its two call sites. Observable: no value changes; M4-B02's suites stay green.
4. **`redstone/entity_presence.rs`.** `EntityClassFilter`, `EntityPresenceSource`, `NoEntities`.
5. **`redstone/button.rs`.** `ButtonKindConfig`, the 14-row `BUTTON_BLOCKS` table, decode helpers, `mount_direction`, both trait impls, `press`/`check_pressed`/`update_neighbours` per §C. Observable: `redstone_button.rs` passes.
6. **`redstone/pressure_plate.rs`.** `PlateSignalModel`, `PlateKindConfig`, the 16-row table, `weighted_plate_signal`, both trait impls, the shared `check_pressed` per §D. Observable: `redstone_pressure_plate.rs` passes.
7. **`redstone/{mod.rs, registration.rs, piston.rs}`.** Module declarations and re-exports; `register_tier2_inputs`; `DESTROY_RANGE_BLOCK_IDS`. Observable: `redstone_input_registration.rs` passes.
8. **`rc-physics::shapes.rs`.** `NON_COLLIDING_INPUT_BLOCK_IDS` and the range-iterating `air()` rows. Observable: `input_component_shapes.rs` passes.
9. **`server::play::mining.rs`.** The six kinds and their four table rows, the widened `resolve_orientation` arm, the plate placement support check, the oriented-state and item-map rows. Observable: `cargo build -p rusty-clanker-server` succeeds; `play_redstone_input_components.rs` tests 1 and 9 pass.
10. **`server::play::entity_presence.rs`.** `EntityPresenceRecord`, `RegionEntityPresence`, `EntityPresenceResource`, `entity_inside_step` per §E.
11. **`server::play::world.rs`.** Composition-root wiring (`register_tier2_inputs` + the two `Arc` clones + `EntityPresenceResource`), the `entity_inside_step` call site, the `TickSoundOutbox` drain, the three `debug_*` accessors. Observable: the whole `play_redstone_input_components.rs` suite passes.
12. **Corpus.** Author the four `.ron` fixtures, capture against the pinned oracle (`cargo run -p xtask -- fetch-corpus`), regenerate `manifest.json`. Observable: `xtask parity-check redstone` green with the new fixtures counted.
13. **Ledger.** Append §J's six entries to `docs/findings-for-planning.md` (governance/test-authoring changeset only — never the implementation changeset).
14. **Full workspace pass.** `fmt-check`, `lint`, `lint-deps`, `lint-tests`, `test` all exit 0; `cargo test --doc` for the three crates exits 0.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** (TEST-D45/D46). The implementation changeset touches no file under any `tests/` directory, no corpus fixture, no `manifest.json`, no `xtask` verification code and no budget table. The mechanical `sounds:` field addition at every existing `UpdateContext { .. }` site — including the 21 already-merged test files and `crates/testing/gametest/src/replay.rs` — belongs to the **test-authoring** changeset, exactly as M4-B07's own `light_dirty` addition did. Commit subjects are `M4-B10 test-authoring: …` and `M4-B10 implementation: …`, each with the `Changeset-Type` trailer (M0-B08). `blueprints/M4/M4-B10-CLAIMS.md` is written by the TEST-D57 research pass before any implementation changeset and is never authored, edited or self-certified by an implementation changeset (M3.5-B04's own path guard rejects it).

(b) **No new dependencies of any kind.** No `[workspace.dependencies]` entry is added, removed or bumped; no crate gains a new path dependency. `rc-mechanics`, `rc-physics` and `rusty-clanker-server` each keep exactly the dependency set they have today — every id this blueprint needs comes from `rc-registries`, already a dependency of all three.

(c) **`rc-mechanics` still must never depend on `rc-protocol`, `rc-transport-inproc`, `rc-transport-net`, `rc-auth`, `rc-cluster` or `rc-proxy`** (WS-D3 rule 2), and `rc-physics` must never depend on `rusty-clanker-server`. This is why `EntityPresenceSource` is a trait in `rc-mechanics` with its only real implementation in `rusty-clanker-server`, and why `entity_inside_step` is a server-side manual tick-loop step rather than an ECS system in `rc-mechanics`: only that crate can see both `PlayerMarker` players and `BaseEntity` entities.

(d) **No Mojang or third-party reimplementation code.** Every constant, ordering and formula above was derived by reading the ASSET-D18(f) reference (`ButtonBlock`, `BasePressurePlateBlock`, `PressurePlateBlock`, `WeightedPressurePlateBlock`, `FaceAttachedHorizontalDirectionalBlock`, `BlockSetType`, `Blocks`, `Block`, `SupportType`, `BlockBehaviour`, `Level`, `LevelAccessor`, `ScheduledTickAccess`, `ScheduledTick`, `Entity`, `Shapes`) and restating it in this project's own words; no method body is transcribed, and no other reimplementation's source was consulted. The implementer works from this blueprint alone and must not open the reference.

(e) **No algorithmic deviation from the pinned orders.** §C's five-step press order, §D's write-then-notify-then-sound-then-reschedule order, the plate's `write_block_state` (flag 2) versus the button's `set_block` (flag 3) split, the "sound only on a zero↔nonzero transition" rule, `TickPriority::Normal` for every schedule, and the `ceil`-based weighted formula (never `round`, never integer division) are all binding. Do not "simplify" the button's deliberately duplicated fan-out at its own cell, and do not merge the two `update_neighbours` calls.

(f) **Support checks go through `signal::is_face_sturdy` only.** Neither behaviour may re-derive sturdiness, call `is_conductor` as a stand-in, or compare shapes directly — MECH-D84 requires placement and the engine-side pop check to consult the identical predicate.

(g) **No `unsafe` code**, and no `unwrap`/`expect` on a world read: every decode path guards its raw id against the block's own `range_of` span first (the established `is_lever_range`/`is_wire_range` convention) so a unit test's placeholder id or an unloaded position can never panic.

(h) **Scope boundary, restated exhaustively.** This blueprint does not implement: arrow or projectile presses; explosion presses; game events, vibrations or particles; wind-charge activation; any other `entityInside` effect; an `on_removed` hook; swept multi-step inside-block enumeration; a `PlaceableBlockKind` row for any button or plate outside §H's six; a corpus "use" action; or any change to the already-merged wire/torch/lever shape rows (§J item 1 is a ledger entry for planning, not a change to make here).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-mechanics -p rc-physics -p rusty-clanker-server --all-features
cargo nextest run -p rc-mechanics -p rc-physics -p rusty-clanker-server
cargo test --doc -p rc-mechanics -p rc-physics -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- lint-tests
cargo run -p xtask -- test
cargo run -p xtask -- verify-fixtures
cargo run -p xtask -- parity-check redstone
```

Machine-readable artifacts: `target/verify/*.json` per verb (the shape every `xtask` verb already writes via `tier_result`), plus `parity-check redstone`'s own per-fixture pass/fail report including this blueprint's four new fixtures.
