# M3-B05 — Piston

| Field | Content |
|---|---|
| ID | M3-B05 |
| Milestone | M3 — Mechanics Tier 1: Movement, Blocks, Redstone Core |
| Prerequisites | M3-B01 (`rc-mechanics`: `Direction`/`SHAPE_UPDATE_ORDER`/`NEIGHBOR_CHANGED_ORDER`, `BlockWorldAccess`, `NeighborUpdateEngine`/`PendingUpdate`, `ScheduledTickQueue`/`TickPriority`, `BlockEventQueue`/`BlockEvent`, `BlockBehavior`/`BlockBehaviorRegistry`/`NoOpBehavior`/`UpdateContext` — every field of `UpdateContext` (`world`, `engine`, `scheduled`, `events`, `outbound`, `ownership`, `current_tick`) used exactly as B01 shipped it, no new field added — `BorderHalo`/`RegionOwnership`, `border::fan_out_from_changed_block` — reused unmodified); M3-B04 (`rc-mechanics::redstone`: `RedstoneSignalSource`, `SignalSourceRegistry`, the free functions `is_conductor`/`emitted_toward`/`signal_into`/`best_neighbor_signal`/`has_signal`/`notify_neighbor_changed_only` — this blueprint is the "clean power-query API" consumer B04's own Context names by name, consumed exactly as shipped, no signature change; `rc_physics::tier1_shape_table()`/`ShapeTable`/`VoxelShape`/`BlockPhysicsProperties` — reused via the same transitive `rc-mechanics --> rc-physics` edge B04 already added, this blueprint adds no new crate edge). Transitively builds on M3-B02 (`rc-physics`'s shape-table extension point, which M3-B02's own Open Questions explicitly reserved for "this milestone's redstone blueprint" — restated in Context §D) and M3-B03 (`rusty-clanker-server`'s tier-1 placeable-block table already lists `piston`/`sticky_piston`; this blueprint does not touch `crates/server/` — placement-to-behavior wiring is explicitly future integration work, Context §I, mirroring B04's own identical, already-established precedent for repeater/comparator). |
| Implements | MECH-D13 (piston: extend/retract, push/pull, max 12-block push chain, sticky-piston pull, entity displacement — the entity-displacement half explicitly deferred to M4, Context §H — as its own Stage-4 system in vanilla's own tick-priority-queue order); MECH-D8 (quasi-connectivity, piston's own `getNeighborSignal`-equivalent, restated exactly); MECH-D7 (bug-for-bug parity, including the "does not re-check until it receives an update" staleness property and the 0-tick-pulse-as-emergent-consequence stance, Context §F); MECH-D14 (cross-partition block-push obstruction — reused unmodified, this blueprint's structure resolver treats a non-local destination exactly as an ordinary blocked/obstructed push, no new mechanism); MECH-D9/D10 (block-event-driven extend/retract, Stage-4 inline mutation — exercised for the first time against a real multi-block, batched mutation); MECH-D15 (piston activation is a neighbor-changed-only signal, never a shape-update signal — restated); MECH-D51 (piston-caused block destruction's drop stance — extends M3-B03's own already-established "compute eligibility, spawn no item entity" interim decision to a second, new destruction pathway, stated explicitly, not silently) |
| Crates touched | `rc-mechanics` (`crates/mechanics/`) — one new file (`src/redstone/piston.rs`), one modified file (`src/redstone/mod.rs`, additive: one `pub mod` line + re-exports); `rc-physics` (`crates/physics/`) — `src/shapes.rs` modified, additive only (six new `piston_head` shape-table entries, the exact extension point M3-B02's own Open Questions reserved for this blueprint) |
| Estimated scope | L |

## Goal & Done definition

Give `rc-mechanics::redstone` its fifth tier-1 component: the piston (`PistonBehavior`, both plain and sticky variants sharing one struct per the research corpus's own "`isSticky: boolean` field" framing) as a `BlockBehavior`-only registration (piston emits no redstone signal of its own, so it is never registered into B04's `SignalSourceRegistry`) that consumes B04's power-query API for its own quasi-connectivity activation check, fires vanilla's exact three block-event codes through B01's block-event sub-phase, resolves its own push/pull structure with vanilla's exact 12-block cap and push/destroy/block classification for the M3 tier-1 block set, and commits the whole batch atomically two ticks after activation — reproducing the piston's own real "decouple signal-says-extend from extension-actually-happening" timing without a genuine animated intermediate block state (a documented, bounded M3 simplification, Context §E). Six new `rc-physics` shape-table entries give a settled `piston_head` its real, non-full collision shape; a settled *extended* piston base needs no new entry (Context §D). Entity displacement, slime/honey-block adhesion, and continuous mid-animation collision are explicitly out of scope (Context §H) — this blueprint states each boundary rather than leaving it silent.

Done when:

- [ ] `cargo build -p rc-mechanics -p rc-physics --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mechanics -p rc-physics`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds **zero** new crate dependencies to either crate (both `rc-mechanics --> rc-physics` and every other edge already exist as of B04).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-mechanics -p rc-physics` exits 0.
- [ ] Determinism: every ordering-sensitive test (structure-resolver golden cases, tick-table cases, the QC-staleness test) passes identically across repeated runs — no flakiness, no `sleep`-based synchronization anywhere in this blueprint's suite.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Activation check — quasi-connectivity, restated exactly, and the "does not re-check until an update arrives" property

**The exact positions checked** (`08-redstone-ticking.md` §3.9's own English restatement, converted here into this project's own `signal::has_signal(world, registry, pos, from)` primitive — B04's own `has_signal(pos, from) = signal_into(pos, from) > 0`, "what `pos` receives from its neighbor in `from`"):

```
fn piston_neighbor_signal(world, registry, piston_pos, push_direction) -> bool:
    // 1. Every one of the piston's own 6 faces except DOWN (checked separately, step 2)
    //    and except the push direction itself (the piston never reads the face it is
    //    about to push into — that position may not even exist yet).
    for d in [West, East, North, South, Up]:
        if d != push_direction and has_signal(world, registry, piston_pos, from: d):
            return true
    // 2. The piston's own DOWN face, always checked regardless of push_direction (this is
    //    what makes a piston resting on an ordinary lever/wire-topped block extend).
    if has_signal(world, registry, piston_pos, from: Down):
        return true
    // 3. Quasi-connectivity through the block directly above the piston: its 4 HORIZONTAL
    //    faces only (not its own Up or Down face) — "a piston can be triggered by a signal
    //    touching the top-adjacent block from the side" (research doc, verbatim).
    let above = Up.apply(piston_pos)
    for d in [West, East, North, South]:
        if has_signal(world, registry, above, from: d):
            return true
    return false
```

Step 1 checks exactly 4 positions whenever `push_direction` is itself one of `{West, East, North, South, Up}` (the common, test-covered case: a horizontally- or upward-facing piston) — matching the research doc's own "4 side faces except the push direction" count precisely. When `push_direction == Down` (a downward-facing piston), step 1's own exclusion has nothing to remove from its 5-element candidate set, so step 1 checks all 5 and step 2 additionally re-checks `Down` (the same direction the piston is about to push into) — a harmless, order-independent duplicate check (an `OR` of booleans checked twice is not observably different from checked once). This one case — a downward-facing piston's own activation-check count — is **flagged for reconciliation**: the research corpus's own English gloss does not resolve this specific edge unambiguously from prose alone, and this blueprint's own acceptance-test corpus (Acceptance tests) exercises only horizontal- and upward-facing pistons (matching every one of the milestone's own canonical contraption names — extension, QC double-piston, piston door), leaving a downward-facing-piston-specific test to whichever future audit finds a contraption sensitive to this exact edge. `is_conductor` (B04, reused unmodified) already resolves `true` for a retracted piston base (its shape is `FULL_CUBE`, `07-blocks-blockstates.md`/M3-B02's own table) — so a retracted piston base already participates in QC as an ordinary conductor for *other* components reading power through it (e.g. a torch resting on top of a retracted piston base), entirely for free from B04's existing `is_conductor`/`direct_signal_to` machinery, with zero piston-specific code needed for that direction.

**"Does not re-check until an update arrives" — the exact, testable staleness rule.** `piston_neighbor_signal` above is recomputed fresh on **every** call, but it is only ever *called* from `PistonBehavior::on_neighbor_changed` — never on a periodic re-poll, and never transitively through an intermediate block whose own `BlockBehavior::on_neighbor_changed` does nothing further (`NoOpBehavior`'s every method is a true no-op, B01). Concretely: if a wire's power changes two positions away from a piston, and that wire's own 7-cell-plus notify (B04 Context §D) happens to include the *conductor block* the piston is resting against, but **not** the piston's own position directly, and that conductor is an ordinary, unregistered block (`NoOpBehavior`), the piston's own `on_neighbor_changed` is **never invoked** by that particular power change — the piston's cached `should_be_extended` flag (Context §B) stays exactly where it last was, even though a fresh `piston_neighbor_signal` query (if anyone bothered to run one) would now return a different answer. This is not a bug this blueprint introduces — it is the literal, direct consequence of B01's own event-driven dispatch model (a `BlockBehavior` callback fires only when the update-propagation engine's own fan-out targets that exact position) applied honestly to the piston, exactly as it already applies to every other tier-1 component. Acceptance test `piston_stays_stale_until_directly_notified` (below) exercises this precisely.

### B. Per-position piston state — the store, and reaching B04's `SignalSourceRegistry` from inside `BlockBehavior`

Mirroring B04's own established pattern (Context §I of that blueprint: "no generated block-state-property registry exists yet... this blueprint extends that pattern uniformly to all four components' full runtime state"), `PistonBehavior` holds its own per-position state, **one instance per region** (never shared across regions, identical rationale to B04's):

```
struct PistonState {
    facing: Direction,        // the push direction
    sticky: bool,
    extended: bool,           // mirrors the real, currently-committed BlockStateId's own
                               // EXTENDED property — updated only at commit time (Context §E)
    should_be_extended: bool, // the piston's own cached activation target (Context §A) —
                               // updated eagerly on every on_neighbor_changed call
}
```

stored in `Mutex<HashMap<BlockPos, PistonState>>` inside `PistonBehavior` (a `Mutex`, not `RefCell`, for the identical reason B04 already states: `BlockBehavior: Send + Sync`'s trait bound, never actually contended under `ARCH-D13`'s single-worker-per-region collapse). `PistonBehavior::place(pos, facing, sticky)` is a test/composition-root-only setter (`extended`/`should_be_extended` both start `false`) — mirroring B04's own `RepeaterBehavior::place`/`ComparatorBehavior::place` precedent exactly: real placement-pipeline integration (wiring a `Use Item On` action, via M3-B03's already-shipped placeable-block table for `piston`/`sticky_piston`, into a call to `PistonBehavior::place`) is explicitly **not** this blueprint's job — it is future integration work, following the identical scope boundary B04 already drew for repeater/comparator placement (that blueprint's own Constraints (d): "No block placement/removal machinery is implemented or modified... this blueprint's components only ever react to updates on already-placed blocks").

**Reaching B04's `SignalSourceRegistry`.** B04's own free functions (`signal::has_signal` et al.) take `registry: &SignalSourceRegistry` as an explicit parameter — but `BlockBehavior::on_neighbor_changed(&self, ctx: &mut UpdateContext, ...)` receives only `ctx` (B01's `UpdateContext` has no `SignalSourceRegistry` field). B04's own Context §I½ resolves this for its own four components with an explicit two-phase construction (each behavior holds a `OnceLock<Arc<SignalSourceRegistry>>`, bound via the `Tier1RedstoneHandles` value `register_tier1_redstone` returns) — piston sidesteps the same gap even more simply, since piston is registered strictly *after* B04's four components and never itself needs to be inserted *into* `SignalSourceRegistry` (no chicken-and-egg construction-order problem to begin with): `PistonBehavior::new(registry: Arc<SignalSourceRegistry>) -> Self` takes an already-fully-populated `Arc<SignalSourceRegistry>` at construction and stores it as a private field, used by every `signal::`-calling method. The composition root's own required sequencing (Implementation steps, Deliverables' `register_piston`), extending B04's own already-specified sequence by one step: call `register_tier1_redstone` (B04), keeping its returned `Tier1RedstoneHandles`; wrap the resulting `SignalSourceRegistry` value in `Arc::new(..)`; call `handles.bind_registry(Arc::clone(&registry))` (B04's own Context §I½ step, completing that blueprint's own four components' registry self-reference); *then* construct `PistonBehavior::new(Arc::clone(&registry))` and register it into `BlockBehaviorRegistry` only (never into `SignalSourceRegistry` — piston has no `RedstoneSignalSource` impl at all, since it emits no signal of its own).

### C. Push/pull structure resolution — 12-block cap, push/destroy/block classes for the tier-1 set

**`PistonStructureResolver.MAX_PUSH_DEPTH = 12`** (`08-redstone-ticking.md` §5, confirmed independently by `07-blocks-blockstates.md` §3.10's identical constant), restated as this blueprint's own exact algorithm — walking outward from the position immediately in front of the piston, in the push direction, one block at a time (`chunk_of(pos) = pos.chunk_key(world.dimension())` throughout, B01's own already-established convention, `border.rs`'s identical doc-comment phrasing, restated here for a caller that has `world: &dyn BlockWorldAccess` directly rather than a full `UpdateContext`):

```
fn resolve_extend(world, ownership, piston_pos, push_direction) -> Result<PushPlan, Abort>:
    to_push: Vec<BlockPos> = []
    pos = push_direction.apply(piston_pos)
    loop:
        local = ownership.resolve(chunk_of(pos)) == ownership.local   // MECH-D14, below
        match classify(world, pos, ownership_local: local):
            Air | Unloaded  -> break                          // empty landing spot — success
            Immovable       -> return Err(Abort::Blocked)      // whole push refused
            Destroy         -> to_destroy = Some(pos); break   // destroyed in place, chain ends here
            Normal          -> {
                to_push.push(pos)
                if to_push.len() > 12 { return Err(Abort::TooManyBlocks) }
                pos = push_direction.apply(pos)
            }
    Ok(PushPlan { to_push, to_destroy, head_pos: pos })
```

A `Destroy`-class block **always terminates the walk** — it is never itself added to `to_push`, and nothing "behind" it (further from the piston) is ever examined, matching `08-redstone-ticking.md`'s own "`DESTROY` is allowed only... at the very front of the line" wording read as its most direct, unambiguous consequence: since a destroyed block vacates its position entirely, there is never anything left for the chain to continue past. The depth check (`to_push.len() > 12`) mirrors vanilla's own `blockCount + toPush.size() > 12` exactly (aborting once a 13th pushable block would be added) — this blueprint's own `Abort::TooManyBlocks` and `Abort::Blocked` are both a plain "the whole push fails, nothing moves, nothing is written" outcome; this blueprint does not distinguish them observably beyond a diagnostic reason code. **World-border/Y-bound abort** (the third vanilla abort case named in the research corpus) is a documented no-op at M3: `HardcodedWorld` (M1-B05) has no configured world border and superflat Y bounds (`-64..320`) are never reachable by any 12-block push chain in this milestone's own acceptance corpus; a future blueprint implementing real world-border/height-limit enforcement extends `classify`'s own `Immovable` branch without changing this function's signature.

**`classify` — the tier-1 push/destroy/block table**, restated from `08-redstone-ticking.md` §3.9's own general rule ("`isPushable` — false for... anything with `getDestroySpeed == -1`... gated by `PushReaction` otherwise") applied to exactly the block set M3-B02/M3-B03 already place in this milestone's world:

| Block(s) | Class | Rationale |
|---|---|---|
| Air | *(terminator, not classified)* | empty landing space |
| Stone, Dirt, Grass Block | Normal | ordinary solid terrain, no special reaction |
| Bedrock | Immovable | hardness `-1` / unbreakable — `getDestroySpeed == -1` unconditionally blocks a push regardless of any other property, restated exactly from the research doc's own "false for anything with `getDestroySpeed == -1`" clause |
| Redstone Wire, Torch, Wall Torch, Repeater, Comparator | Destroy | non-full, attaching decorations — the well-established vanilla convention (this blueprint's own class of block that "breaks instead of being pushed") applied to exactly this milestone's tier-1 redstone-component set |
| Piston, Sticky Piston (**retracted**, `EXTENDED = false`) | Normal | an ordinary full-cube block when retracted — pushable like any other solid terrain block (vanilla does allow a piston to push another retracted piston) |
| Piston/Sticky Piston (**extended**, `EXTENDED = true`), Piston Head | Immovable *(deliberate, bounded M3 deviation — flagged)* | vanilla's real resolver moves an extended-piston/piston-head *pair* together as a linked unit (mirroring a door's two-half linkage); this blueprint does not implement that pair-aware special case (no canonical contraption in this milestone's own acceptance corpus requires pushing an already-extended piston) — treating the pair as an obstruction is the safe, conservative interim choice: it never silently corrupts a linked pair by moving only half of it. A future blueprint that needs this case extends `classify` alone. |
| Chest, Furnace/Blast Furnace/Smoker, Hopper | Immovable | vanilla's own real default for every block-entity-bearing block (a block-entity's own `PushReaction` defaults to blocking a piston push unless a specific block overrides it — none of this milestone's tier-1 block-entity set does) — restated from well-established, long-stable vanilla behavior; no tier-1 block-entity blueprint (M3-B06) overrides this default |

**Sticky retraction — the M3 interim scope, checked against `05` and stated explicitly (slime/honey not tiered in).** Neither `05-game-mechanics.md` nor `11-roadmap-milestones.md`'s M3 scope line names `slime_block`/`honey_block` anywhere in the tier-1 placeable/breakable set (checked against M3-B02's shape table and M3-B03's dig-timing table — neither lists either block; no `slime_block`/`honey_block` can exist anywhere in an M3-tier-1 world). MECH-D13's own text names "honey-block adhesion" as part of piston's *eventual, full* decision content, not as a claim that M3 tier-1 must implement it — this blueprint follows the milestone's own actually-placeable block set, not MECH-D13's aspirational full list. **Consequence, stated explicitly:** this blueprint's sticky-piston retraction pulls **at most one block** — whatever sits immediately in front of where the head currently rests, and only if that block classifies `Normal` (an `Immovable` or `Destroy`-class — or absent — candidate is simply not pulled, a bare retraction):

```
fn resolve_retract(world, ownership, piston_pos, push_direction, sticky) -> PullPlan:
    old_head = push_direction.apply(piston_pos)
    if sticky:
        candidate = push_direction.apply(old_head)
        local = ownership.resolve(chunk_of(candidate)) == ownership.local
        if classify(world, candidate, ownership_local: local) == Normal:
            return PullPlan { pulled: Some(candidate) }
    PullPlan { pulled: None }
```

No multi-block "stuck cluster" branching (`addBranchingBlocks`, vanilla's own slime/honey adjacency-propagation walk) is implemented — there is nothing in an M3-tier-1 world it could ever apply to. A future blueprint that tiers `slime_block`/`honey_block` into the world extends `resolve_retract` (and `resolve_extend`'s own `classify`) without changing either function's signature.

**Cross-partition obstruction (MECH-D14, reused unmodified).** `classify`'s own `ownership_local: bool` parameter (Deliverables) is exactly this check's result, computed once per candidate position by `resolve_extend`/`resolve_retract` themselves (`ownership.resolve(chunk_of(pos)) == ownership.local`, via `RegionOwnership`, B01) before ever calling `classify` — `classify` itself never reaches into `BlockWorldAccess::owner_of` directly, keeping the ownership check in exactly one place per candidate. Whenever `ownership_local` is `false`, `classify` returns `Immovable` unconditionally, regardless of whatever block actually occupies that (non-local) position — an ordinary "blocked" obstruction, identical in kind to any other `Immovable` case, exactly matching MECH-D14's own text ("treated by the push algorithm as an impassable obstruction... an ordinary 'blocked, cannot extend' failure") and the Cross-Border Mechanic Contract Summary's own row ("Piston push/pull — blocks | Not supported cross-border | Treated as blocked/obstruction | N/A — push fails"). No new `RegionMessage` variant, no cross-region write, is ever attempted for a block-push.

### D. `rc-physics` shape-table extension — `piston_head`, and why an extended base needs no new entry

M3-B02's own Open Questions reserved exactly this extension point: *"piston_head, piston (`extended = true`)... this milestone's *redstone* blueprint's content (MECH-D13), not movement/collision's; this blueprint's registry is a plain, open `BlockStateId -> BlockPhysicsProperties` map any future blueprint may add entries to without changing `rc-physics`'s own API."*

**An extended piston base needs no new literal entry at all.** `ShapeTable::lookup` (M3-B02) already returns `BlockPhysicsProperties::default_full_cube()` for any `block_state_id` with no explicit table entry — and an extended piston base's own collision shape is, in fact, an unchanged full cube (only its *texture* changes between retracted/extended in real vanilla; the base block itself never shrinks). Since M3-B02's own table lists only the *retracted* piston/sticky-piston states explicitly (an intentional, narrower entry than "every piston state"), an *extended* base's raw id simply falls through to the default fallback, which already produces the exactly correct answer — no action needed.

**`piston_head` — one non-full shape per facing** (`PLATFORM_THICKNESS = 4` sixteenths, `07-blocks-blockstates.md` §5, confirmed by `08-redstone-ticking.md` §5's identical constant — restated here as the face-plate's own thickness along the facing axis; the arm's cross-section width, sourced from minecraft.wiki's Piston article's long-stable, widely-documented geometry, **flagged for reconciliation** exactly as M3-B02's own `chest`/`hopper` entries already are, pending `xtask extract-shapes`): a piston head's `VoxelShape` is the union of two boxes — a **face plate**, `4/16` (`0.25`) thick along the facing axis, full `16/16` footprint on the other two axes, positioned at the *far* end of the block (the side the head visually points toward); and a centered **arm**, `4/16 × 4/16` cross-section (`0.375..0.625` on the two non-facing axes), spanning the *remaining* `12/16` of the facing axis, connecting the face plate back toward the block's near face (where the base sits). Worked reference case, `facing = Up`: face plate `[0,1]×[0.75,1]×[0,1]`; arm `[0.375,0.625]×[0,0.75]×[0.375,0.625]`. The other five facing values are the identical construction rotated onto the matching axis (the face plate always at the far/facing-pointing end, the arm always spanning the near `12/16`) — six literal `BlockPhysicsProperties` entries (`friction: 0.6, speed_factor: 1.0, jump_factor: 1.0`, matching every other tier-1 entry's own defaults), one per facing value's own raw `BlockStateId` range, added to `tier1_shape_table()`'s hand-authored table exactly as B04's own four new entries were (placeholder literal ids — Constraints (b)). `SHORT` (Context §E) has no effect on the shape this blueprint ships — this blueprint's own design never produces a `SHORT = true` piston head (Context §E explains why).

### E. Extension/retraction sequence — block-event codes, the 2-tick commit, and why there is no genuine intermediate block state

**Block-event codes** (`08-redstone-ticking.md` §3.9, restated exactly): `level.blockEvent(pos, this, actionId, direction.get3DDataValue())` — `actionId` `0 = TRIGGER_EXTEND`, `1 = TRIGGER_CONTRACT`, `2 = TRIGGER_DROP`; the event's own `event_param` byte is vanilla's own real `Direction` wire ordinal (restated exactly from M2-B07's own already-established table: `Down=0, Up=1, North=2, South=3, West=4, East=5` — **not** this project's own `Direction` enum's declaration order, `West, East, North, South, Down, Up`). This blueprint's own `Direction::vanilla_ordinal(self) -> u8` (Deliverables) is the conversion. This blueprint's own event-code selection rule, resolved concretely from the research doc's own three named codes: `TRIGGER_EXTEND` (0) whenever activating; `TRIGGER_CONTRACT` (1) for an ordinary retraction that **does** move a block back (non-sticky bare retraction, or sticky retraction whose `resolve_retract` found a `Normal`-classified block to pull); `TRIGGER_DROP` (2) for a **sticky** retraction whose `resolve_retract` found nothing to pull (a non-sticky piston never fires `TRIGGER_DROP` — it never attempts a pull in the first place). Every one of these three codes drives an identical structure-resolution-then-commit path server-side; this blueprint has no client to render a visual/audio distinction between them, so the code's only server-observable role at M3 is a diagnostic/wire-fidelity one (Acceptance tests assert the exact code selected per scenario, since a future client depends on getting this right).

**Emission timing — same-tick, not next-tick.** `PistonBehavior::on_neighbor_changed` runs during Stage 4's **scheduled phase** (B01's `system_scheduled_phase`, registered `order_tag` 0), *before* the block-event sub-phase (`order_tag` 1) runs in that same tick. `BlockEventQueue::emit` (B01) always appends to its internal `next` buffer; `begin_subphase` (called once, at the very end of Stage 4, by the block-event sub-phase itself) takes whatever is currently in `next` — which, since the scheduled phase already ran and already called `emit` this same tick, includes this piston's own just-emitted event. **A block event emitted from `on_neighbor_changed` is therefore processed in the block-event sub-phase of the *same* tick it was emitted, not deferred** — MECH-D9's own deferred-refire rule applies only to an event emitted *by a block-event handler, during that handler's own processing* (i.e. from inside `PistonBehavior::on_block_event` itself), which this blueprint's own design never does (Context, "zero-tick" paragraph below, addresses the one case where this matters).

**`on_block_event` — structure resolution, re-validated fresh, and scheduling the 2-tick commit.** Real vanilla re-resolves the whole push/pull structure at *execution* time (inside `triggerEvent`), not at the earlier `checkIfExtend` decision time — because block-event delivery can, in general, be delayed or coalesced, acting on a stale structure snapshot would be wrong. This blueprint's `on_block_event` reproduces that re-validation exactly: it calls `resolve_extend`/`resolve_retract` fresh, against the world state *as observed right now*, never reusing anything computed earlier. If resolution fails (`Abort::Blocked`/`Abort::TooManyBlocks`, extend only — retraction never fails to at least retract the bare head), nothing further happens this cycle — no commit is scheduled, no state changes; the piston's own `should_be_extended` flag (set independently, by `on_neighbor_changed`, Context §A/§B) is *not* rolled back, so a signal that stays "on" while the structure remains blocked does not re-trigger a fresh block event on every subsequent unrelated neighbor-changed call (no mismatch exists between `should_be_extended` and the *already-attempted* activation — this blueprint's own `on_neighbor_changed` only re-fires when `should_be_extended`'s own value *changes*, Context §A). On success, this blueprint records a `MovingPistonState { plan: PushPlan | PullPlan, extending: bool }` (a second per-position map, same `Mutex`-guarded pattern as `PistonState`) and calls `ctx.schedule_block_tick(piston_pos, delay_ticks: 2, priority: TickPriority::Normal)` — `TICKS_TO_EXTEND = 2` (`08-redstone-ticking.md` §5, confirmed by §3.9's own "`progress` animates... in steps of `0.5F` per tick... matches: two ticks × `0.5` = full extension"), `TickPriority::Normal` this blueprint's own reasonable default exactly where the research corpus pins no piston-specific priority (mirroring B04's identical choice for torch's re-eval tick, Context §E of that blueprint).

**No genuine intermediate `MOVING_PISTON` block-state — a documented, bounded M3 simplification, stated explicitly.** Real vanilla replaces every affected position with a temporary `MovingPistonBlock` placeholder *immediately* (same tick as the block event), carrying an interpolating `PistonMovingBlockEntity` that only converts back to the real, final `BlockState` — and only *then* fires the real neighbor/shape-update cascade — once `progress` reaches `1.0`, two ticks later. This blueprint reproduces the **externally observable half** of that behavior exactly (nothing at any affected position becomes its final state, and **no neighbor/shape-update fan-out fires**, until the full two ticks have elapsed) without reproducing the intermediate placeholder itself: between the trigger tick and the commit tick, every affected position's `BlockStateColumn` entry is simply left **unchanged** (whatever it held immediately before the block event fired) — the in-flight move exists *only* as this blueprint's own `MovingPistonState` bookkeeping, never as a real chunk-storage write. This is sufficient for every acceptance criterion this blueprint is responsible for (final-state correctness, push-limit/structure-resolution correctness, and neighbor-update *timing*, none of which depend on a real mid-transit block state or its own collision shape) and has two direct, positive consequences stated explicitly rather than left implicit: (a) **no continuous mid-animation collision is modeled at all** (Context §H — entity pushing is M4 scope, so nothing at M3 ever needs to collide against a moving piston mid-flight); (b) **mid-movement state is trivially safe across a save/load boundary** (Context §J) — since nothing partial is ever written to persisted chunk state, a restart or chunk unload mid-animation simply and safely loses the in-flight move (as if it had never been triggered), never corrupting or half-applying anything.

**The atomic commit** (`on_scheduled_tick`, firing at `piston_pos` two ticks after the triggering block event): first, re-validate that `piston_pos` still holds a registered piston state consistent with the pending `MovingPistonState` (Context §G handles the "not" case — a broken piston). If still valid: compute every affected position's **final** state (Extend: `piston_pos` itself flips `EXTENDED = true`; each `to_push[i]` moves to `push_direction.apply(to_push[i])`; `to_destroy`'s position, if any, becomes `AIR`; the front-most vacated slot becomes `piston_head` with `facing = push_direction`, `SHORT = false` — this blueprint's own design never produces a mid-animation `SHORT = true` head, since there is no mid-animation state to represent one; the previously-occupied positions that are now empty because their contents moved forward become `AIR`. Retract: mirror — `piston_pos` flips `EXTENDED = false`; the old head position becomes `AIR` (or the pulled block, if any, moves into it — `pulled.map_or(AIR, |p| state_at(p))`); the pulled block's own *old* position, if any, becomes `AIR`); then writes every one of these positions via the **raw** `ctx.world.set_block(pos, state)` (bypassing `UpdateContext::set_block`'s own fan-out — this is the direct `BlockWorldAccess::set_block` trait method, B01, which performs no propagation of its own); then, **only after every position in the batch has already been written**, calls `border::fan_out_from_changed_block(ctx, pos, state)` once per affected position (Extend order: `piston_pos` first, then `to_push` in original near-to-far walk order, then the new head position last; Retract order: mirrored, head/pulled-block positions first, `piston_pos` last) — reproducing the research doc's own explicit "all real neighbor notifications fire only after every block in the batch has already been converted... not interleaved per-block" ordering guarantee using B01's own already-shipped, unmodified public API (no new fan-out mechanism is built by this blueprint). This blueprint's own choice of *intra-batch* notify order (base-first-then-outward for extend, outward-first-then-base for retract) is a reasonable, self-consistent one where the research corpus does not independently pin an exact intra-batch order — flagged for reconciliation exactly as B04's own wire 7-cell notify order already is. Finally, `PistonState.extended` and `should_be_extended` (Context §B) are updated to match, and the `MovingPistonState` entry is cleared.

**Drops from piston-destroyed blocks (`to_destroy`) — the explicit interim stance (MECH-D51, extending M3-B03's own precedent).** When the commit converts a `Destroy`-classified block to `AIR`, this blueprint computes nothing beyond the fact that a destruction happened — no drop-eligibility formula is evaluated (unlike M3-B03's player-caused breaks, a piston-caused destruction has no "tool" concept at all to gate eligibility against) and, per this project's own already-established M3-wide stance (M3-B03's own Context, "Drops stance at M3"), **no item entity is ever spawned**, since no `ItemEntity`/entity-spawning mechanism exists anywhere in this project before M4 (MECH-D51). This is the direct, honest extension of M3-B03's own precedent to a second, new destruction pathway — stated explicitly here rather than left silent, exactly as the milestone's own task boundary requires.

### F. Zero-tick pulse stance — an emergent consequence of MECH-D7/D11, not a special case

MECH-D7/MECH-D11 already commit this project, project-wide, to reproducing the classic redstone evaluator's exact update-order/count behavior bug-for-bug, explicitly declining to "fix" the staleness that produces exactly this class of quirk (MECH-D11's own rationale: "any 'fix' to the staleness... changes observable update count and timing and therefore fails the parity bar for the default backend"). This blueprint does not attempt to independently verify, or special-case, whatever real vanilla 26.2 does with a "0-tick" pulse — it states the one binding rule that determines the outcome and lets the outcome fall out of that rule mechanically, exactly as the project's own parity model requires: **`PistonBehavior::on_neighbor_changed` may be invoked more than once at the same position within a single tick** (B01's `NeighborUpdateEngine::drain` can dispatch a chain of updates that revisits the same position more than once in one tick, e.g. via a redstone wire's own unconditional 7-cell-plus notify firing twice from two converging changes) — **each invocation independently recomputes `piston_neighbor_signal` and compares it against the piston's own `should_be_extended`'s current, live value**; if it differs, a fresh block event is emitted immediately (Context §E's "same-tick, not next-tick" timing) and `should_be_extended` is updated eagerly, in place, before the call returns. Two such invocations within one tick, with the second reversing the first's own conclusion, therefore emit **two** block events into the *same* tick's block-event `next` buffer (extend, then contract, or vice versa) — both processed, in that order, within that same tick's block-event sub-phase (`begin_subphase` returns both). This blueprint's own **commit-collision rule** resolves the natural consequence explicitly: `on_block_event`'s own `MovingPistonState` write (Context §E) is an ordinary map insert — a *second* block event arriving for a position that *already* has a `MovingPistonState` in flight (its 2-tick commit not yet reached) **overwrites** the existing entry with the new one and re-schedules a fresh 2-tick countdown for the *new* target, rather than queuing two separate sequential animations. Applied to the extend-then-contract case above: the pending "commit as extended" is superseded, before it ever fires, by "commit as retracted" — the piston never visibly reaches the extended state at all, and the whole two-tick window ends with the piston settled back at its starting state, having emitted no observable neighbor/shape-update fan-out for the superseded extension (only the final, surviving commit's own fan-out fires, at its own two-tick mark from whichever event scheduled it last). This is this blueprint's own considered, mechanically-derived interpretation of "what MECH-D7/D11's binding commitment produces here," not an independently-verified fact about real vanilla 26.2's own piston implementation — flagged accordingly, and directly exercised by Acceptance test `pulse_shorter_than_commit_window_is_absorbed` below.

### G. Interaction with M3-B03 — breaking a moving piston

Since Context §E's own design writes no intermediate block state, "breaking a moving piston" reduces to two clean, independently-testable cases, both resolved by the commit's own re-validation step (Context §E, "first, re-validate"): **(1) the piston base itself is broken mid-flight** (M3-B03's `mining::apply_mining_action` runs against `piston_pos` while a `MovingPistonState` entry exists for it — since the base's own `BlockStateColumn` entry is unchanged until commit, the break applies normally, exactly as if no move were pending, and produces its own ordinary drop-eligibility result per M3-B03's own ordinary rules) — when the pending commit's own scheduled tick later fires, its re-validation step finds `piston_pos` no longer holds a registered piston state consistent with the pending move, and **aborts the whole commit silently**: no position in the batch is written, the `MovingPistonState` entry is simply cleared, and no block event/neighbor-update ever fires for the abandoned move. **(2) one of the *other* affected positions is broken/changed mid-flight** (a player mines the block that was about to be pushed, or places a different block into the space the head was about to occupy) — since nothing at those positions has been silently pre-empted by this blueprint's own design (Context §E), the intervening break/place simply applies normally against whatever real state is there; the commit's own final-state computation reads each position's *live* state fresh at commit time (not a cached snapshot from the trigger tick) for every position **except** ones it is about to overwrite outright — a per-position re-validation narrower than case (1)'s whole-move abort: if a specific `to_push[i]` position no longer holds the exact state it held at resolution time, that single position's own write is skipped (its live, changed content is left alone) while every other still-consistent position in the same batch still commits normally. This is the closest honest single-position analog to vanilla's own real "re-verify, something changed in between" defensive pattern, applied without needing a second, separate abort-the-whole-batch mechanism for a case narrower than case (1)'s.

### H. Explicit boundaries — entity displacement (M4) and continuous mid-animation collision

**Entity pushing is M4 scope, stated as an explicit interim behavior, not a silent gap.** MECH-D13's own full decision text names "entity displacement" as part of piston's eventual complete behavior; the milestone's own task boundary is explicit ("entities/AI/combat/items-as-entities are M4"). This blueprint's own interim behavior, stated concretely: **a piston's push/pull structure resolution (Context §C) and its final commit (Context §E) never query, move, or otherwise interact with any entity** — no collision check against a player or any other entity is performed anywhere in this blueprint's own algorithm, at any point in the 2-tick window or at commit. A player standing in a position a piston pushes into simply has that position's block state silently changed under them server-side, with no push-out-of-the-way physics applied — an honest, bounded gap, not a crash or an undefined-behavior risk (M3-B02's own `collide_and_slide` is never invoked by anything in this blueprint). A future M4 blueprint that implements entity displacement extends the commit step (Context §E) with an entity-query-and-push pass, using `rc-physics`'s already-shipped `collide_and_slide`/`overlaps_any_solid` (M3-B02) — no change to this blueprint's own structure-resolution or timing model is anticipated to be required.

**Continuous mid-animation collision is not modeled**, restated from Context §E's own design: since no intermediate block state is ever written, there is nothing for `rc-physics` to collide against differently during the 2-tick window than before it — the six new `piston_head` shape entries (Context §D) describe only the *settled*, post-commit head shape.

### I. Headless/technical states — restated from the generated registry (best-effort, flagged)

No generated block-state-property registry exists for this project to consult (M2-B01's own still-empty `rc-registries::generated/`, unchanged as of this blueprint — identical gap B04 already restates). Restated from `07-blocks-blockstates.md` §3.10 (piston base, independently confirmed) and minecraft.wiki's own long-stable, widely-documented block-state convention (piston head — **flagged for reconciliation**, since the research corpus's own digest covers the base's properties but not the head's independently):

| Block | Properties |
|---|---|
| `piston` / `sticky_piston` | `FACING` (6-way), `EXTENDED` (bool) — `07-blocks-blockstates.md` §3.10, confirmed: "`PistonBaseBlock`... has 2 properties (`FACING`, `EXTENDED`)" |
| `piston_head` | `FACING` (6-way), `TYPE` (`normal`/`sticky` — so a settled head renders/behaves consistent with which base variant produced it), `SHORT` (bool — an animation-only property; Context §E's own design never produces `SHORT = true`, since this blueprint writes only the fully-settled head, never a mid-animation one) |

Neither this blueprint nor B01/B04 registers a real generated `moving_piston` block-state id anywhere — Context §E's own explicit design choice (no genuine intermediate `MOVING_PISTON` block-state) means this project has no need for one at M3; a future blueprint that reintroduces a real animated intermediate state (should continuous mid-animation collision, Context §H, ever become in-scope) would be the first to need it.

### J. Persistence of mid-movement state across save/load

Resolved directly by Context §E's own design, restated here as this blueprint's own explicit M3 decision (per the task's own requirement to restate, not silently inherit): **mid-movement piston state does not persist across a save/load boundary at M3.** `MovingPistonState` and `PistonState` both live entirely in `PistonBehavior`'s own in-memory `Mutex<HashMap<...>>` (Context §B), never touching any M2 NBT/chunk-serialization schema. A server restart, or a chunk unload/reload cycle, occurring during the 2-tick animation window simply loses the in-flight `MovingPistonState` entry (the whole `PistonBehavior` instance — one per region, Context §B — is reconstructed fresh on the next region bootstrap) — since Context §E's own design never writes a partial/intermediate `BlockStateColumn` value anywhere, nothing on disk or in a freshly-loaded chunk is ever inconsistent: every affected position simply reads back exactly whatever it held *before* the move was triggered, as if the move had never started. `PistonState`'s own steady-state fields (`facing`, `sticky`, `extended`) are, like B04's own wire/torch/repeater/comparator state, **not** independently persisted either — a piston's real, settled `EXTENDED`/`FACING` properties are recoverable directly from its own real `BlockStateId` in `BlockStateColumn` (M2's already-existing chunk persistence, unmodified), which *is* saved/loaded normally; only the *transient*, in-flight-move bookkeeping is ever lost, and only when a save/load genuinely interrupts mid-animation. A future blueprint that migrates B04's own `BlockState`-representable subset (wire `POWER`, torch `LIT`, etc.) into real generated `BlockStateId` transitions could, at the same time, choose to persist `should_be_extended` similarly if a future audit finds this gap observable — not needed for any M3 acceptance criterion.

## Deliverables

### `crates/mechanics/src/redstone/mod.rs` (modify — additive: one `pub mod` line, extend the re-export list)

```rust
pub mod piston;

pub use piston::{register_piston, PistonBehavior};
```

(Every other line in this file — B04's own `signal`/`wire`/`torch`/`repeater`/`comparator`/`registration` modules and their re-exports — unchanged.)

### `crates/mechanics/src/redstone/piston.rs` (new)

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

use crate::behavior::{BlockBehavior, BlockBehaviorRegistry, UpdateContext};
use crate::block_event::BlockEvent;
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;
use crate::world_access::BlockWorldAccess;

use super::signal::{self, SignalSourceRegistry};

/// Vanilla's own action-id constants (Context §E), `level.blockEvent`'s second argument.
pub const TRIGGER_EXTEND: u8 = 0;
pub const TRIGGER_CONTRACT: u8 = 1;
pub const TRIGGER_DROP: u8 = 2;

/// `PistonMovingBlockEntity.TICKS_TO_EXTEND` (Context §E) — the fixed commit delay every
/// extend/retract uses, regardless of push length or sticky-ness.
pub const COMMIT_DELAY_TICKS: u64 = 2;

/// `PistonStructureResolver.MAX_PUSH_DEPTH` (Context §C).
pub const MAX_PUSH_DEPTH: usize = 12;

/// One block's role in a resolved push/pull (Context §C).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PushClass { Normal, Destroy, Immovable }

/// `resolve_extend`'s pure classification step (Context §C's own table) — a free function so
/// this blueprint's own acceptance tests can exercise it directly against a `FakeWorld`
/// without needing a full `PistonBehavior` instance.
pub fn classify(world: &dyn BlockWorldAccess, pos: BlockPos, ownership_local: bool) -> PushClass;

/// Resolution failure reasons (Context §C) — both are a plain "the whole push fails" outcome;
/// this blueprint distinguishes them only for diagnostics.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExtendAbort { Blocked, TooManyBlocks }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushPlan {
    pub to_push: Vec<BlockPos>,
    pub to_destroy: Option<BlockPos>,
    pub head_pos: BlockPos,
}

/// Context §C's exact walk algorithm.
pub fn resolve_extend(
    world: &dyn BlockWorldAccess,
    ownership: &crate::border::RegionOwnership,
    piston_pos: BlockPos,
    push_direction: Direction,
) -> Result<PushPlan, ExtendAbort>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullPlan {
    pub pulled: Option<BlockPos>,
}

/// Context §C's exact one-block sticky-pull algorithm (M3's own interim scope — no slime/
/// honey adjacency walk, Context §C).
pub fn resolve_retract(
    world: &dyn BlockWorldAccess,
    ownership: &crate::border::RegionOwnership,
    piston_pos: BlockPos,
    push_direction: Direction,
    sticky: bool,
) -> PullPlan;

/// Context §A's exact quasi-connectivity activation check.
pub fn piston_neighbor_signal(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    piston_pos: BlockPos,
    push_direction: Direction,
) -> bool;

/// Per-position steady-state (Context §B). `extended`/`should_be_extended` both start `false`
/// from `place`.
#[derive(Copy, Clone, Debug)]
struct PistonState {
    facing: Direction,
    sticky: bool,
    extended: bool,
    should_be_extended: bool,
}

/// One in-flight extend or retract (Context §E) — cleared on commit or on a superseding event
/// (Context §F).
#[derive(Clone, Debug)]
enum MovingPlan {
    Extending(PushPlan),
    Retracting(PullPlan),
}

#[derive(Clone, Debug)]
struct MovingPistonState {
    plan: MovingPlan,
    direction: Direction,
}

/// Piston / sticky piston (Context, whole document). One instance per region (Context §B) —
/// never share across regions. Implements `BlockBehavior` only — a piston emits no redstone
/// signal of its own, so it is never registered into `SignalSourceRegistry` (Context §B).
pub struct PistonBehavior {
    registry: Arc<SignalSourceRegistry>,
    state: Mutex<HashMap<BlockPos, PistonState>>,
    moving: Mutex<HashMap<BlockPos, MovingPistonState>>,
}

impl PistonBehavior {
    /// `registry` must already be fully populated (Context §B — construct after
    /// `register_tier1_redstone` completes).
    pub fn new(registry: Arc<SignalSourceRegistry>) -> Self;

    /// Test/composition-root-only placement setter (Context §B) — mirrors B04's
    /// `RepeaterBehavior::place`/`ComparatorBehavior::place` precedent exactly. Real
    /// placement-pipeline integration is future work, not this blueprint's.
    pub fn place(&self, pos: BlockPos, facing: Direction, sticky: bool);

    pub fn facing(&self, pos: BlockPos) -> Direction;
    pub fn is_sticky(&self, pos: BlockPos) -> bool;
    pub fn is_extended(&self, pos: BlockPos) -> bool;
    /// The piston's own cached activation target (Context §A) — exposed for acceptance tests
    /// exercising the "does not re-check until notified" staleness property directly.
    pub fn should_be_extended(&self, pos: BlockPos) -> bool;
    /// `true` iff a `MovingPistonState` entry currently exists for `pos` (a commit has been
    /// scheduled but has not yet fired or been superseded).
    pub fn has_pending_move(&self, pos: BlockPos) -> bool;
}

impl BlockBehavior for PistonBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        /* Context §A/§E/§F: recompute piston_neighbor_signal fresh; if it differs from the
           cached should_be_extended, update should_be_extended eagerly and emit exactly one
           block event (TRIGGER_EXTEND, or TRIGGER_CONTRACT/TRIGGER_DROP per resolve_retract's
           own outcome — Context §E) via ctx.emit_block_event. May fire more than once per tick
           at the same position (Context §F) — no dedup beyond the should_be_extended
           mismatch check itself. */
    }

    fn on_block_event(&self, ctx: &mut UpdateContext, pos: BlockPos, event: &BlockEvent) {
        /* Context §E: re-resolve resolve_extend/resolve_retract fresh against live world
           state; on success, insert/overwrite this position's MovingPistonState (Context §F's
           own overwrite-supersedes rule) and ctx.schedule_block_tick(pos, COMMIT_DELAY_TICKS,
           TickPriority::Normal); on failure (extend only), do nothing further this cycle. */
    }

    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        /* Context §E's own atomic commit: re-validate (Context §G case 1); compute every
           affected position's final state; write each via the raw ctx.world.set_block (no
           fan-out); then call crate::border::fan_out_from_changed_block(ctx, p, state) once
           per affected position in this blueprint's own defined order; update PistonState;
           clear the MovingPistonState entry. Per-position re-validation for case (2) of
           Context §G is applied during the "compute every affected position's final state"
           step — a position whose live state no longer matches what resolution originally
           observed there is skipped, not overwritten. */
    }
}

/// The two raw `BlockStateId` ranges `BlockBehaviorRegistry` dispatch needs (Context §I — no
/// generated registry exists yet, mirroring B04's own `Tier1RedstoneStateIds` convention
/// exactly): `piston`/`sticky_piston`, each range covering **both** retracted and extended
/// states — `EXTENDED` does not change which behavior a state resolves to, only
/// `PistonState.extended`'s own runtime value (Context §B). `piston_head` is deliberately
/// **not** a field here and is never registered into `BlockBehaviorRegistry` at all by this
/// blueprint: a piston head is a pure, inert placeholder for redstone-behavior purposes
/// (`NoOpBehavior`-equivalent) — it needs only the six separate `rc-physics` shape-table
/// entries (Context §D, `crates/physics/src/shapes.rs`, a wholly different id space with its
/// own composition-root-supplied literals), never a `BlockBehavior` registration.
pub struct PistonStateIds {
    pub piston: (BlockStateId, BlockStateId),
    pub sticky_piston: (BlockStateId, BlockStateId),
}

/// Constructs one fresh `PistonBehavior` and registers it into `behaviors` at both of `ids`'
/// ranges. Call once per region, after `register_tier1_redstone` (B04) has fully populated
/// `registry` (Context §B). Never registers anything into `SignalSourceRegistry` (Context §B).
pub fn register_piston(
    behaviors: &mut BlockBehaviorRegistry,
    registry: Arc<SignalSourceRegistry>,
    ids: &PistonStateIds,
) -> Arc<PistonBehavior>;
```

### `crates/mechanics/src/direction.rs` (modify — additive: one new method on the already-shipped `Direction`, no existing line changes)

```rust
impl Direction {
    /// Vanilla's own real wire `Direction` ordinal (Context §E), restated exactly from
    /// M2-B07's own already-established table: `Down=0, Up=1, North=2, South=3, West=4,
    /// East=5` — **not** this enum's own declaration order.
    pub const fn vanilla_ordinal(self) -> u8;
}
```

### `crates/physics/src/shapes.rs` (modify — additive only, per M3-B02's own reserved extension point, Context §D)

Six new literal entries added to `tier1_shape_table()`'s hand-authored table (Context §D's exact box construction), keyed by whichever raw `BlockStateId` per-facing ranges the composition root supplies for `piston_head` — no other line in `shapes.rs` changes, following B04's own identical "placeholder literal ids, composition root confirms real literals later" convention exactly.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly as B01/B04's own).** Every file below, plus `crates/mechanics/src/redstone/piston.rs` and the `direction.rs` addition with every function body replaced by `todo!()` (fields/derives/doc comments unchanged), is the test-authoring changeset, committed and independently verifier-reviewed before any real implementation body exists. The implementation changeset fills in bodies only and must not touch `crates/mechanics/tests/`, must not add/remove/rename a test case below, and must not weaken any assertion — in particular `piston_structure_resolver.rs`'s exact push-limit/classification cases, `piston_tick_tables.rs`'s exact tick numbers, and `piston_zero_tick.rs`'s exact commit-supersession outcome.

All test files reuse the shared `support::FakeWorld`/`TestSignalSource` test doubles B04 already established (`crates/mechanics/tests/support/mod.rs`).

### `crates/mechanics/tests/piston_structure_resolver.rs`

1. `classify_matches_tier1_table` — one sub-case per row of Context §C's own table (`stone` → `Normal`, `bedrock` → `Immovable`, `redstone_wire`/`torch`/`repeater`/`comparator` → `Destroy`, retracted `piston` → `Normal`, extended `piston`/`piston_head` → `Immovable`, `chest`/`furnace`/`hopper` → `Immovable`).
2. `push_stops_at_the_first_destroy_class_block` — a line of 3 stone blocks then a redstone torch then air; `resolve_extend` returns `to_push = [stone, stone, stone]`, `to_destroy = Some(torch_pos)`, and the torch's own position is **not** included in `to_push`.
3. `push_refuses_entirely_on_an_immovable_block` — 2 stone blocks then bedrock; `resolve_extend` returns `Err(ExtendAbort::Blocked)`.
4. `push_refuses_at_exactly_thirteen_blocks` — a line of exactly 13 consecutive stone blocks (no terminator within reach); `resolve_extend` returns `Err(ExtendAbort::TooManyBlocks)`. A line of exactly 12 stone blocks then air succeeds with `to_push.len() == 12`.
5. `sticky_retract_pulls_the_one_directly_adjacent_normal_block` — sticky piston, a single stone block directly in front of the (already-extended) head position; `resolve_retract(.., sticky: true)` returns `PullPlan { pulled: Some(that_stone_pos) }`.
6. `sticky_retract_does_not_pull_an_immovable_or_destroy_class_block` — two sub-cases (bedrock, redstone wire) directly in front of the head; both return `PullPlan { pulled: None }`.
7. `non_sticky_retract_never_pulls_regardless_of_what_is_in_front` — a plain (non-sticky) piston with a stone block directly in front of its head; `resolve_retract(.., sticky: false)` returns `PullPlan { pulled: None }` unconditionally (never even evaluates `classify` on the candidate — asserted via `FakeWorld`'s own call-count instrumentation).
8. `non_local_neighbor_is_treated_as_blocked` — a `RegionOwnership` whose `resolve` maps the second position in the push line to a non-local `Address::Region`; `resolve_extend` returns `Err(ExtendAbort::Blocked)` at exactly that position (MECH-D14).

### `crates/mechanics/tests/piston_activation_qc.rs`

1. `piston_extends_from_a_direct_side_signal` — `TestSignalSource` fixed at power 15 directly touching one of the piston's own non-push, non-down faces; `piston_neighbor_signal` returns `true`.
2. `piston_extends_from_below` — signal touching the piston's own `Down` face; `true`.
3. `qc_torch_powers_piston_two_above` — the Context §A worked case: a lit floor torch `T`, a solid conductor block `B = T.up()`, piston `P` resting on top of `B` (i.e. `P = B.up()`) facing horizontally; `piston_neighbor_signal(P, facing)` is `true` via step 3's QC-through-the-block-above check.
4. `piston_never_reads_its_own_push_direction` — a `TestSignalSource` touching only the piston's own push-direction face (no other face powered); `piston_neighbor_signal` returns `false`.
5. `piston_stays_stale_until_directly_notified` (the "does not re-check" acceptance test, Context §A) — a piston `P` resting beside a plain, unregistered conductor block `C` (which the piston reads via QC through `C`'s Down face — mirroring `qc_torch_powers_piston_two_above`'s own shape but with `C` itself, not `C.up()`, adjacent to `P` on `P`'s own non-push side); a `TestSignalSource` at `C.down()` starts at power 0. Directly mutate the `TestSignalSource`'s power to 15 (bypassing any notify — simulating "the source changed but nothing propagated a notify to `P`"). Assert `P.should_be_extended(pos) == false` still (unchanged — no `on_neighbor_changed` call has reached `P`), even though a fresh `piston_neighbor_signal` query (called directly, bypassing the behavior) would now return `true`. Then call `P`'s own `on_neighbor_changed` directly (simulating the notify finally arriving); assert `should_be_extended` becomes `true` **now**.

### `crates/mechanics/tests/piston_tick_tables.rs` (the "hand-derived tick tables for canonical contraptions" acceptance tests)

Each case is a full `PistonBehavior` + `BlockBehaviorRegistry` + a `LoggingBehavior`-style instrumentation wrapper (B01's own established pattern) driven through `stage4::run_scheduled_phase`/`run_block_event_subphase` (B01) tick-by-tick, `current_tick` starting at `0`.

1. `simple_extension` — a lone piston, `TestSignalSource` flips `0 -> 15` at tick 0 touching the piston's `Down` face. Tick table: tick 0, `on_neighbor_changed` fires, `TRIGGER_EXTEND` block event queued and processed same tick (Context §E), commit scheduled at `trigger_tick = 2`; ticks 0–1, `is_extended(piston_pos) == false`, no fan-out observed at any affected position; tick 2, `on_scheduled_tick` fires, `is_extended(piston_pos) == true`, `piston_head` written at the front position, and the instrumented neighbor at that front position observes exactly one `on_neighbor_changed`/`on_shape_update` pair (B01's own `fan_out_from_changed_block`, exercised via this blueprint's per-position calls).
2. `qc_double_piston` — two pistons, `P1`/`P2`, both QC-activated by the **same** single power-source change in one tick (arranged so both receive `on_neighbor_changed` within the same tick's `NeighborUpdateEngine::drain` pass). Assert both schedule their own `TRIGGER_EXTEND` commit at `trigger_tick = 2` from the same trigger tick, and both commit (independently, at their own positions) at tick 2 — proving two independently-registered `PistonBehavior` instances sharing one `SignalSourceRegistry` handle (Context §B) do not interfere with each other's own `Mutex`-guarded per-position state.
3. `piston_door_element` — a sticky piston `SP` with a single stone "door" block directly in front of its (already-extended, pre-seeded via `place`+direct state manipulation) head; input flips `15 -> 0` at tick 0. Tick table: tick 0, `on_neighbor_changed` fires, `resolve_retract` finds the stone block, `TRIGGER_CONTRACT` (not `TRIGGER_DROP`) queued and processed same tick, commit at tick 2; ticks 0–1, the door position still holds `stone` (unchanged, Context §E); tick 2, the door position becomes `AIR`, the position directly in front of the (now-retracted) base holds `stone` (the pulled block), `is_extended(SP) == false`.
4. `sticky_retract_with_nothing_to_pull_fires_drop` — identical to case 3 but the door position starts as `AIR` (nothing to pull); assert the queued event's `event_id == TRIGGER_DROP` (not `TRIGGER_CONTRACT`), and at tick 2 only the base's own `EXTENDED` flag changes — no other position is written.
5. `commit_reads_live_state_and_skips_a_changed_position` (Context §G case 2) — a `simple_extension`-shaped setup (one block being pushed), but between tick 0 (trigger) and tick 2 (commit), test code directly overwrites the to-be-pushed position's own live state to a *different* value than what `resolve_extend` observed at trigger time; assert the tick-2 commit does **not** overwrite that position (it retains the test-injected value), while the piston base's own `EXTENDED` flip still commits normally.
6. `breaking_the_base_mid_flight_aborts_the_whole_commit` (Context §G case 1, the "interaction with B03" acceptance test) — a `simple_extension`-shaped setup; between tick 0 and tick 2, test code directly overwrites the piston base's own position to `AIR` (simulating M3-B03's own break path, which this test does not itself invoke — only its net effect, a changed base state, is exercised here per this blueprint's own test-boundary discipline). Assert the tick-2 `on_scheduled_tick` call writes **nothing** at any affected position (not even the base, already air) and produces **zero** fan-out calls.

### `crates/mechanics/tests/piston_zero_tick.rs` (the "zero-tick stance" acceptance test, Context §F)

1. `pulse_shorter_than_commit_window_is_absorbed` — a single piston; within **one** tick's `NeighborUpdateEngine::drain` pass, test code drives two sequential `on_neighbor_changed` calls at the piston's own position with the underlying signal read as `true` on the first call and `false` on the second (simulating a same-tick reversal, Context §F). Assert: exactly two block events are queued this tick (`TRIGGER_EXTEND` then `TRIGGER_CONTRACT`/`TRIGGER_DROP`, in that call order); both are processed in that same tick's block-event sub-phase; the resulting `MovingPistonState` reflects only the **second** (retracting) plan, not the first; at the eventual commit tick (`trigger_tick = 2`, counted from the *second* event's own emission tick, which is the same tick as the first's — Context §F: "re-schedule a fresh 2-tick countdown"), the piston's own `EXTENDED` flag is unchanged from its starting value, and **no** fan-out fires for the superseded extension at any point.
2. `two_events_in_different_ticks_do_not_supersede` — the same reversal, but the second `on_neighbor_changed` call happens on a **later** tick (after the first event's own commit already fired at tick 2); assert both commits happen independently, in full, each with its own fan-out — proving the overwrite-supersedes rule applies only to a genuinely still-in-flight `MovingPistonState`, never to an already-completed one.

### `crates/mechanics/tests/piston_shape_table.rs` (pure, `rc-physics`)

1. `piston_head_shape_is_non_full_per_facing` — for each of the six facing values, `tier1_shape_table().lookup(piston_head_id_for(facing)).shape` is **not** a single `(0,0,0)..(1,1,1)` box (i.e. `signal::is_conductor`-equivalent would resolve `false` for it, matching real vanilla — a piston head is not a redstone conductor).
2. `piston_head_face_plate_thickness_is_platform_thickness` — for `facing = Up` (Context §D's own worked reference case), the face-plate sub-box's own thickness along the `Y` axis is exactly `0.25` (`4/16`, `PLATFORM_THICKNESS`).
3. `extended_piston_base_falls_through_to_default_full_cube` — an `extended = true` piston `BlockStateId` with **no** explicit `tier1_shape_table()` entry (this blueprint adds none, Context §D) resolves via the table's own default fallback to `BlockPhysicsProperties::default_full_cube()`.

### `crates/mechanics/tests/direction_vanilla_ordinal.rs` (pure)

1. `vanilla_ordinal_matches_m2b07s_table` — `Direction::vanilla_ordinal` for all six values equals `Down=0, Up=1, North=2, South=3, West=4, East=5` exactly (Context §E).

## Implementation steps

1. **`direction.rs` — `vanilla_ordinal`.** Pure, no dependency on any other new content. Observable: `direction_vanilla_ordinal.rs` passes.
2. **`rc-physics` — `shapes.rs`.** Add the six `piston_head` literal entries (Context §D). Observable: `cargo nextest run -p rc-physics` still green (purely additive); `piston_shape_table.rs` passes.
3. **`redstone/piston.rs` — pure resolution functions.** `classify`, `resolve_extend`, `resolve_retract`, `piston_neighbor_signal` — every one a free function against `&dyn BlockWorldAccess`/`&SignalSourceRegistry`, no `PistonBehavior` state needed. Observable: `piston_structure_resolver.rs` and `piston_activation_qc.rs` pass.
4. **`redstone/piston.rs` — `PistonBehavior`.** `on_neighbor_changed`/`on_block_event`/`on_scheduled_tick` per Context §A/§E/§F/§G exactly, built on step 3's pure functions plus B01's `UpdateContext`/`border::fan_out_from_changed_block`. Observable: `piston_tick_tables.rs` and `piston_zero_tick.rs` pass.
5. **`redstone/piston.rs` — `PistonStateIds`/`register_piston`.** Wires one fresh `PistonBehavior` into `behaviors` at both supplied ranges. Observable: `cargo build -p rc-mechanics --all-features` succeeds.
6. **`redstone/mod.rs`.** Add the module declaration and re-export list. Observable: every re-exported symbol resolves.
7. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
8. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, exactly per TEST-D45/D46 as restated in Acceptance tests above: the test-authoring changeset (every `tests/*.rs` file plus `redstone/piston.rs`/the `direction.rs` addition stubbed with `todo!()` bodies) is committed and independently verifier-reviewed before any real implementation body exists. The implementation changeset fills in bodies only and must not touch `crates/mechanics/tests/` — in particular the exact push-limit/classification cases, tick numbers, and the zero-tick commit-supersession outcome must survive unchanged.

(b) **No new external dependencies, no new crate edges.** This blueprint adds zero new crate dependencies to either `rc-mechanics` or `rc-physics` — both crates' dependency sets are exactly as B04 already established. `tier1_shape_table()`'s six new entries use placeholder `BlockStateId` literals, exactly as B04's own four entries did; the composition root is responsible for confirming those literals against a real generated registry once one exists (M3-B02's own established convention, restated).

(c) **No Mojang or third-party reimplementation code.** Every algorithm in this blueprint is derived from this blueprint's own restatement of `05-game-mechanics.md` (MECH-D7, D8, D9, D10, D13, D14, D15, D51), `docs/research/mc-26.2/07-blocks-blockstates.md` §3.10, `docs/research/mc-26.2/08-redstone-ticking.md` §3.9 — and, for the small number of items the research corpus does not pin exactly (the piston-head arm/plate cross-section geometry beyond `PLATFORM_THICKNESS`, `piston_head`'s own `TYPE`/`SHORT` properties, the intra-batch commit notify order, the downward-facing-piston activation-check edge count, and the zero-tick commit-supersession outcome), minecraft.wiki's public documentation or this blueprint's own explicitly-flagged, mechanically-derived reasoning from MECH-D7/D11's binding commitment — each such item explicitly flagged for reconciliation in Context, mirroring this project's own established convention (B04's own identical flagging discipline for wire's 7-cell notify order, torch's wall-facing output symmetry, and the comparator container-fullness formula). No decompiled Mojang source, no other reimplementation's code, is consulted at any point.

(d) **Scope boundary, restated.** Entity displacement is explicitly M4 scope (Context §H) — this blueprint's own commit step never queries or moves any entity. Slime/honey-block adhesion is explicitly not tiered into M3 (Context §C) — no `slime_block`/`honey_block` state exists anywhere in an M3-tier-1 world, so sticky retraction is bounded to a single directly-adjacent block. Continuous mid-animation collision is not modeled (Context §E/§H) — no genuine intermediate block state is ever written. No item entity is ever spawned for a piston-destroyed block (Context §E, MECH-D51) — a future M4 blueprint extends the commit step to do so. Real placement-pipeline integration (wiring M3-B03's `Use Item On` handling to `PistonBehavior::place`) is not performed by this blueprint (Context §B) — `place` remains a test/composition-root-only setter, mirroring B04's own identical, already-established boundary for repeater/comparator.

(e) **Determinism, no unsafe code.** Every algorithm in this blueprint is single-threaded by construction (Stage 4's sequential collapse, `ARCH-D13`, reused unmodified from B01) and implementable in 100% safe Rust — the two `Mutex`es per `PistonBehavior` instance (Context §B/§E) exist only to satisfy `BlockBehavior: Send + Sync`'s trait bound and are never actually contended (single-worker-per-region), not a concurrency primitive doing real work. No `unsafe` block appears anywhere in this blueprint's deliverables.

(f) **Per-region state isolation is binding.** `PistonBehavior`'s own two internal state stores are constructed fresh per region by `register_piston` (Context §B) — sharing one `Arc<PistonBehavior>` across two regions' `BlockBehaviorRegistry` instances is a correctness bug this blueprint's own Implementation steps must not introduce, identical to B04's own restated rule for its own four components.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-mechanics -p rc-physics --all-features
cargo nextest run -p rc-mechanics -p rc-physics
cargo test --doc -p rc-mechanics -p rc-physics
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-mechanics -p rc-physics` runs every test case named in Acceptance tests above — 8 (`piston_structure_resolver.rs`) + 5 (`piston_activation_qc.rs`) + 6 (`piston_tick_tables.rs`) + 2 (`piston_zero_tick.rs`) + 3 (`piston_shape_table.rs`) + 1 (`direction_vanilla_ordinal.rs`) = 25 test cases — all pass, with zero flakiness (no `sleep`-based synchronization anywhere in this suite). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
