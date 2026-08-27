# M3-B03 — Full Tier-1 Block Breaking & Placing

| Field | Content |
|---|---|
| ID | M3-B03 |
| Milestone | M3 — Mechanics Tier 1: Movement, Blocks, Redstone Core |
| Prerequisites | M3-B01 (`rc-mechanics`: `Direction`/`SHAPE_UPDATE_ORDER`/`NEIGHBOR_CHANGED_ORDER`, `BlockWorldAccess`, `NeighborUpdateEngine`/`PendingUpdate`, `ScheduledTickQueue`/`TickPriority`, `BlockEventQueue`, `BlockBehavior`/`BlockBehaviorRegistry`/`NoOpBehavior`/`UpdateContext`, `BorderHalo`/`RegionOwnership`, `border::fan_out_from_changed_block`, `stage4::ecs::{ChunkIndex, EcsBlockWorld, register_stage4, bootstrap_default_stage4_resources}`, `rc-scheduler`'s `BorderUpdateInbox`/`RegionMessageOutbox`/`CurrentTick` — reused unmodified); M3-B02 (`rc-physics`: `Vec3`/`Aabb`/`Axis`, `VoxelShape`/`BlockPhysicsProperties`/`BlockShapeSource`/`ShapeTable`/`tier1_shape_table()`, `mth_sin`/`mth_cos`, `SHAPE_EPSILON`/`PLAYER_HALF_WIDTH`/`PLAYER_HEIGHT`/`PLAYER_EYE_HEIGHT`; `rusty-clanker-server`'s `play::{PlayerMotion, eye_position, ChunkBlockShapeSource}` — reused unmodified). Also builds directly on M2-B01 (`rc-chunk-storage`'s `BlockStateColumn`/`ChunkKeyTag`/`ChunkPersistenceState`/`PaletteThresholds`/`BlockStateId`/`RegistryId`) and M2-B07 (`rusty-clanker-server`'s `play::block_action` module — **explicitly superseded in large part**, see Context) via already-merged content this blueprint restates rather than re-deriving. |
| Implements | MECH-D4 (Stage-3 placement of block-break/place actions, restated and mapped onto this milestone's manual tick loop, now bridged into M3-B01's engine for the first time in production); MECH-D9/D10/D15 (update emission via `UpdateContext::set_block`, restated and exercised end-to-end for the first time against real gameplay content); MECH-D61 (full survival dig-timing formula — hardness, tool effectiveness, Efficiency/Haste/Mining-Fatigue, water/airborne penalties — restated exactly with a flagged correction to `05`'s own Mining-Fatigue text); MECH-D62 (reach/interaction-range validation — superseded from M2-B07's fixed-position Euclidean check to a real per-player-position box-distance-from-eye predicate against the claimed block's own full unit cell, see "Reach validation" below); MECH-D63 (the client-allocated/server-echoed `sequence` contract, reused unmodified per M2-B07's own flagged correction); a bounded, explicitly-scoped interim reading of MECH-D47/D51 (no `ItemStack`/item-entity model exists — this blueprint's own held-item and drop stances, stated explicitly, not silently) |
| Crates touched | `rc-physics` (`crates/physics/`, additive: one new file, `raycast.rs`); `rusty-clanker-server` (`crates/server/`: `Cargo.toml` modified; `crates/server/src/play/mining.rs` new; `crates/server/src/play/{block_action.rs, packets.rs, world.rs, connection.rs, mod.rs}` modified) |
| Estimated scope | L |

## Goal & Done definition

Give the engine real, vanilla-parity tier-1 block breaking and placing: the full survival destroy-progress formula (hardness × tool-effectiveness × Efficiency/Haste/Mining-Fatigue × water/airborne penalties) with a server-side dig-packet state machine (`START`/`STOP`/`ABORT_DESTROY_BLOCK`, delayed-destroy, the 0.7 stop-threshold, per-tick crack-stage broadcast); creative-mode instant break (kept from M2-B07); placement-context-driven block-state selection and orientation for the tier-1 placeable set (redstone wire/torch, repeater, comparator, piston/sticky piston, chest, furnace/blast furnace/smoker, hopper) via a held-item stub (no real inventory exists yet — MECH-D47/M4); real per-player reach validation via a box-distance-from-eye predicate against the claimed block's own full unit cell (see "Reach validation" below), replacing M2-B07's fixed-position Euclidean placeholder; wiring every place/break mutation through M3-B01's `UpdateContext::set_block` (the first real production exerciser of that engine) with this blueprint's own settle-to-fixed-point driver, and wiring M3-B01's Stage-4 substrate into `HardcodedWorld`'s live tick loop for the first time; the corrected wire layout for `Player Action`/`Use Item On` plus two new packets (`Set Block Destroy Stage`, `Level Event`) for crack-overlay and break-effect broadcast.

Done when:

- [ ] `cargo build -p rc-physics -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-physics -p rusty-clanker-server`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rusty-clanker-server` gains exactly one new normal dependency, `rc-mechanics` (feature `server-systems`); `rc-physics` gains none (its own dependency set stays `{rc-core}`, WS-D3 rule 1).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-physics -p rusty-clanker-server` exits 0.
- [ ] Every dig-timing golden-table entry (Acceptance tests) matches its hand-computed expected tick count exactly (integer equality, not a tolerance band).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### What this blueprint supersedes from M2-B07, stated exactly

M2-B07 shipped a **minimal** place/break path: creative-only instant break (no survival timing at all), a single fixed placement block (`STONE`, no held-item concept), reach validated by straight-line Euclidean distance from the fixed `SPAWN_POSITION` (no real movement existed yet), and block mutation applied directly via `BlockStateColumn::set` with **zero** neighbor-update propagation (M3-B01 did not exist yet). This blueprint supersedes:

- **`apply_block_action`'s entire body and its `ApplyOutcome`/`RejectReason`/`BlockActionKind` shape** — replaced by this blueprint's own richer state machine (`mining.rs`, Deliverables). M2-B07's function name is retired; `mining::apply_mining_action` is its replacement, with a materially different signature (game mode, held item, dig-progress state, and an `UpdateContext` now flow through it instead of a bare `resolve_owner`/`bus` pair).
- **The reach check** — `within_reach`'s straight-line-Euclidean-from-a-fixed-position body is replaced by `mining::is_within_block_interaction_range` (real per-player `PlayerMotion.position` via `movement::eye_position`, box-distance against the claimed block's own full unit cell — see "Reach validation" below; this section's own original text specified an intermediate `mining::raycast_reach` voxel-raycast design here, itself later retired by an M3 field-report correction). `EYE_HEIGHT`/`eye_position(BlockPos)` (M2-B07) are replaced by M3-B02's own `eye_position(Vec3) -> Vec3` (already shipped, unused by any code path before this blueprint per M3-B02's own Interfaces note — this blueprint is the "future M3 block-placement/breaking blueprint" that note names; `eye_position` itself later gained a `crouching: bool` parameter, same field-report correction).
- **The fixed `STONE` placement block** — replaced by a held-item stub (Context, "Held-item stub — the pre-inventory placement/tool source").
- **`Player Action`'s `face` wire type and `Use Item On`'s packet ID** — corrected (Context, "Packet layout — corrected and new").

**Kept unchanged from M2-B07:** `Face` (vanilla `Direction` ordinal enum) and its `from_ordinal`/`offset`; `resolve_place_position`/`target_position`'s algorithm shape (inside-block-flag rule); `to_storage_id`/`to_storage_biome_id`; `seed_chunk_column`; `ChunkIndex`; `debug_query_block`; `pack_position`/`unpack_position`; the "replaceable == currently `AIR`" rule (no plants/fluids exist in the tier-1 superflat world, M2-B07's own scoping remains valid); `Block Update` (`0x08`)/`Acknowledge Block Change` (`0x04`) packets, byte-for-byte; the "broadcast to every connected player" interest-set simplification (M1-B05/M2-B07's own established stand-in, still valid — no per-player view distance exists before M5+); the sequence-ack contract (client-allocated, server-echoed, exactly once per received action packet, MECH-D63 as M2-B07 already corrected it).

### Packet layout — corrected and new (verified against a fresh, protocol-776-pinned `minecraft.wiki` fetch performed while deriving this blueprint, ASSET-D18(b)/(d)/(f))

M2-B07's own `Player Action`/`Use Item On` field-type/id table carried an explicit, self-flagged reconciliation caveat ("must be reconciled against a locally-generated `reports/packets.json`... before this blueprint is considered final"). This blueprint performs that reconciliation with the best available substitute (a live, version-pinned wiki fetch — the same class of source M2-B07/M3-B02 themselves used) and states the result as a binding correction, exactly mirroring this project's own established precedent for exactly this situation (M2-B07's own MECH-D63 correction, M2-B06's WORLD-D14 correction):

| Packet | Bound | ID (was, M2-B07) | ID (corrected) | Fields (wire order) |
|---|---|---|---|---|
| `Player Action` | server | `0x29` | **`0x29` (unchanged)** | `status: i32 #[rc(varint)]`, `location: i64` (packed Position), `direction: i32 #[rc(varint)]` (was `face: i8` — **corrected from Byte to VarInt Enum**; same 6-value vanilla `Direction` ordinal meaning as M2-B07's own `Face::from_ordinal`, only the wire width/kind changes), `sequence: i32 #[rc(varint)]` |
| `Use Item On` | server | `0x2A` | **`0x42`** (corrected — a large ID drift consistent with the many new 26.2 serverbound packets the research corpus's own `11-player-gameplay.md` §3.7/§3.14/§3.15 documents: the permission system, dialogs, and waypoints each add packet traffic in this direction) | `hand: i32 #[rc(varint)]`, `location: i64`, `direction: i32 #[rc(varint)]` (unchanged type, only the field name is restated from `face`), `cursor_x/y/z: f32`, `inside_block: bool`, `sequence: i32 #[rc(varint)]` |
| `Block Update` | client | `0x08` | `0x08` (unchanged) | `location: i64`, `block_state_id: i32 #[rc(varint)]` |
| `Acknowledge Block Change` | client | `0x04` | `0x04` (unchanged) | `sequence: i32 #[rc(varint)]` |
| `Set Block Destroy Stage` (new) | client | — | `0x05` | `entity_id: i32 #[rc(varint)]`, `location: i64`, `destroy_stage: i8` (`0..=9` = crack overlay stage; any other value, this blueprint always sends `-1`, clears the overlay) |
| `Level Event` (new) | client | — | `0x2E` | `event_id: i32` (**plain `Int`, not VarInt** — restated exactly, matching the fetch's own explicit distinction from every VarInt field above), `location: i64`, `data: i32` (plain `Int`) |

**Reconciliation caveat, restated exactly as every prior M1/M2/M3 blueprint's own identical caveat:** every id/type above must be reconciled against a real, locally-generated `reports/packets.json` for protocol 776 before this blueprint is considered final (Implementation steps) — this blueprint's own fetch is a best-effort substitute, not a claim of certainty equal to the real generator pipeline.

`Level Event`'s vanilla event id `2001` ("block break with sound + particles", `data` = the broken block's own raw pre-break state id) is this blueprint's only consumed value — restated as `pub const LEVEL_EVENT_BLOCK_BREAK: i32 = 2001;` (long-stable, well-known vanilla constant, unchanged across many version lines).

### Survival dig-timing formula — exact algorithm and constants (MECH-D61, pinned per the same live-fetch process)

MECH-D61 states the formula's *shape*: `hardness × harvest-multiplier-table(tool material, correct-tool check)⁻¹`, divided by Efficiency/Haste, multiplied by `0.2^(mining_fatigue_level)` under Mining Fatigue, divided by 5 for water-without-Aqua-Affinity and again for airborne; hardness-0 blocks instant, hardness-(−1) never break. This blueprint pins every numeric constant the shape leaves open, sourced from the same `minecraft.wiki` fetch used above (Breaking article, cross-checked against the well-established, version-stable public formula):

**Tool speed multiplier** (`ToolMaterial::speed_multiplier()`), applied only when the tool's `ToolKind` matches the block's effective-tool category (pickaxe/axe/shovel — Context's own per-block table below):

| Material | Multiplier | Mining tier (for correct-tool-for-drops) |
|---|---|---|
| None (bare hand) | 1 | — (never satisfies a tier requirement) |
| Wood | 2 | 0 |
| Gold | 12 | 0 (same tier as wood — fast but weak, a well-known vanilla quirk) |
| Stone | 4 | 1 |
| Iron | 6 | 2 |
| Diamond | 8 | 3 |
| Netherite | 9 | 3 (same tier as diamond) |

A tool's `ToolKind` (Pickaxe/Axe/Shovel/None) only matters for whether the multiplier applies at all (`effective = tool_kind == block's effective category`); the *tier* only matters for whether `has_correct_tool_for_drops` is true, which is a **separate** check (`tier >= block's minimum required tier`, or unconditionally `true` for a block with no tool requirement at all — Context's own per-block table marks this per row). A tool can be speed-effective (its kind matches) while still tier-insufficient for drops — the golden table below has a worked example (wood pickaxe on furnace: fast, but no drop).

**Efficiency** (enchantment level `L ∈ 1..=5` typical, uncapped by this formula itself): applied only when the base multiplier from the row above exceeds `1` (i.e. the tool is speed-effective at all — Efficiency does nothing for bare-hand or a mismatched tool kind): `speed += L² + 1`.

**Haste / Conduit Power** (status-effect amplifier `A`, level `= A + 1`): `speed *= 1.0 + 0.2 * level`.

**Mining Fatigue — a flagged correction to `05`'s own stated shape.** MECH-D61's text says "multiplied by `0.2^(mining_fatigue_level)`"; the verified per-level values (Mining Fatigue I: `×0.3`, II: `×0.09`) do **not** match a `0.2ⁿ` series (`0.2¹=0.2≠0.3`) but match a clean `0.3ⁿ` series exactly (`0.3¹=0.3`, `0.3²=0.09`). This blueprint's own binding correction, stated once with the same explicit-correction discipline M2-B07 already established for MECH-D63: `MINING_FATIGUE_MULTIPLIER(level) = 0.3^min(level, 4)`, `level = amplifier + 1` — levels III/IV (`0.027`/`0.0081`) follow the same clean power series; the raw fetch's own level-III/IV decimal transcription (`0.0027`/`0.00081`) disagreed with this series by exactly one decimal order of magnitude on both entries simultaneously, which this blueprint treats as a transcription artifact of the summarizing fetch rather than the true value — flagged, moderate confidence on levels III/IV specifically (levels I/II are high confidence, matching well-established, oft-cited vanilla trivia), for reconciliation against a real decompile-adjacent or black-box source at implementation time.

**Water/airborne penalties:** `speed /= 5.0` if the entity's eyes are in water without Aqua Affinity; `speed /= 5.0` again (independently, both can apply) if not on ground.

**Hardness == 0 / hardness < 0 — MECH-D61's own explicit special cases, not derived from the general formula (avoids a division by zero at hardness 0):** `hardness == 0.0` → breaks in exactly one tick, unconditionally (no tool, Efficiency, Haste, Mining Fatigue, water, or airborne term ever changes this — MECH-D61's own text states this plainly with no caveat, and this blueprint follows it exactly rather than second-guessing it with an unstated general-formula extrapolation). `hardness < 0.0` → never breaks in survival (creative's own separate instant-break path, kept from M2-B07, is the only way to remove such a block).

**Per-tick progress and tick-count** (general case, `0 < hardness`): `progress_per_tick = speed / (hardness × divisor)`, `divisor = 30.0` if `has_correct_tool_for_drops` else `100.0`; `ticks_to_break = ceil(1.0 / progress_per_tick)` (equivalently `ceil(hardness × divisor / speed)`).

**Tier-1 block table** (hardness, effective tool category, minimum tier for drops — same live-fetch source):

| Block(s) | Hardness | Effective tool | Correct-tool-for-drops rule |
|---|---|---|---|
| Stone | 1.5 | Pickaxe | tier ≥ 0 (any pickaxe, including wood) |
| Dirt | 0.5 | Shovel | none — any tool (incl. hand) always drops |
| Grass Block | 0.6 | Shovel | none — any tool (incl. hand) always drops |
| Bedrock | −1 | — | unbreakable in survival |
| Redstone Wire / Torch / Wall Torch / Repeater / Comparator | 0 | none | none — instant, always drops, any tool |
| Piston / Sticky Piston | 1.5 | Pickaxe | none — any tool (incl. hand) always drops |
| Chest | 2.5 | Axe | none — any tool (incl. hand) always drops |
| Furnace / Blast Furnace / Smoker | 3.5 | Pickaxe | tier ≥ 1 (stone or better) |
| Hopper | 3.0 | Pickaxe | tier ≥ 1 (stone or better) |

### Dig packet lifecycle — server-side state machine (restated exactly from `docs/research/mc-26.2/11-player-gameplay.md` §3.4)

Per-player `DestroyState`: `is_destroying: bool`, `destroy_pos: BlockPos`, `destroy_progress_start: u64` (tick), `has_delayed_destroy: bool`, `delayed_destroy_pos: BlockPos`, `delayed_tick_start: u64`, `last_sent_stage: i8` (`-1` initially).

**Packet-apply substep** (this tick's `START`/`STOP`/`ABORT_DESTROY_BLOCK` actions, applied first, in the same ascending-`network_entity_id` deterministic order MECH-D4/M2-B07 already establish):

- **`START_DESTROY_BLOCK`** (already reach-validated, target already resolved to `pos`): if `abilities.instabuild` (creative, Context "Creative vs. survival paths"): destroy immediately, skip every check below. Else: compute `progress_per_tick` (formula above) from the current held tool, effects, and environment; `progress = progress_per_tick × 1` (one tick elapsed, matching vanilla's own `(gameTicks - startTick + 1)` with `gameTicks == startTick`); if `progress >= 1.0`: destroy immediately (the "insta-mine" case: hardness-0 blocks, or a fast-enough tool/hardness ratio). Else: `is_destroying = true, destroy_pos = pos, destroy_progress_start = current_tick, last_sent_stage = -1` (any previously-active destroy at a *different* position is silently aborted — its true block state is resent to correct client-side prediction).
- **`STOP_DESTROY_BLOCK`**: only meaningful if `pos == destroy_pos`. Recompute `progress = progress_per_tick × (current_tick - destroy_progress_start + 1)` using the SAME tool/effect/environment snapshot taken at `START` time (this blueprint does not re-sample tool/effects mid-dig — a future blueprint that lets a player swap tools mid-dig, per vanilla's own real re-sampling behavior, is out of this blueprint's own scope, Constraints). If `progress >= 0.7`: finalize the destroy now. Else, if no delayed destroy is already queued: `has_delayed_destroy = true, delayed_destroy_pos = destroy_pos, delayed_tick_start = destroy_progress_start` (the **original** start tick — a delayed destroy does not restart the clock). `is_destroying` is cleared either way (unless the destroy was just finalized, in which case both flags are already clear from the destroy itself).
- **`ABORT_DESTROY_BLOCK`**: unconditionally clears `is_destroying`. Does **not** clear `has_delayed_destroy` (a delayed destroy, once queued, keeps accumulating independently of further `ABORT` packets — matches vanilla's own real behavior exactly, restated from the research corpus).

**Per-player `tick()` substep** (run once per player, per tick, immediately after the packet-apply substep above — mirroring vanilla's own real `tick()`-after-packets ordering):

- If `has_delayed_destroy`: recompute `progress` from `delayed_tick_start`; if the block at `delayed_destroy_pos` is no longer the state it was when queued (already air, or already replaced): cancel (clear both flags, no destroy). Else if `progress >= 1.0`: finalize the destroy now (clear both flags).
- Else if `is_destroying`: recompute `progress` from `destroy_progress_start`; if the target has already turned to air (e.g. removed by another path): cancel (`is_destroying = false`), no broadcast. Else: `stage = floor(progress × 10.0).clamp(0, 9)`; if `stage != last_sent_stage`: broadcast `Set Block Destroy Stage { entity_id: <this player's network entity id>, location: pack_position(destroy_pos), destroy_stage: stage as i8 }` to every **other** currently-connected player (excludes the digging player itself — matches vanilla's own real per-entity destroy-progress broadcast, which the digging player's own client already predicts locally without needing the packet); `last_sent_stage = stage`.

**Finalizing a destroy** (from any of the three paths above): apply the break via `mining::apply_mining_action`'s break path (Deliverables) — computes drop eligibility, calls `UpdateContext::set_block(pos, AIR)`, broadcasts `Block Update` + the `Level Event` break effect, clears `DestroyState`'s active/delayed flags for that position, and (if a crack overlay had been shown) implicitly clears it client-side via the `Block Update` itself (vanilla's own client behavior — no explicit clear packet needed once the block is actually gone; this blueprint does not send an extra `Set Block Destroy Stage{destroy_stage:-1}` on finalize for this reason, only on `ABORT`/cancel where the block survives).

### Held-item stub — the pre-inventory placement/tool source

No `ItemStack`/inventory model exists anywhere in this project (MECH-D47/M4). This blueprint's own concrete, explicitly-interim resolution — a direct extension of M2-B07's own "fixed placeholder" precedent, generalized just enough to exercise the full tier-1 placement/breaking surface and no further:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlaceableBlockKind {
    Stone, RedstoneWire, RedstoneTorch, Repeater, Comparator,
    Piston, StickyPiston, Chest, Furnace, BlastFurnace, Smoker, Hopper,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToolMaterial { None, Wood, Stone, Iron, Diamond, Netherite, Gold }
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToolKind { None, Pickaxe, Axe, Shovel }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HeldItemStub {
    /// Default (Context: preserves M2-B07's own default placement behavior exactly).
    Block(PlaceableBlockKind),
    Tool(ToolMaterial, ToolKind),
    EmptyHand,
}
```

`HeldItemStub` is a `Component` on the player entity, defaulting to `Block(PlaceableBlockKind::Stone)` at join (M2-B07's own exact prior fixed behavior, preserved as the default rather than silently changed). A test/diagnostic-only setter (mirroring `debug_query_block`'s own already-established precedent exactly) lets acceptance tests select any tier-1 block or tool: `HardcodedWorld::debug_set_held_item(network_entity_id, item: HeldItemStub)`. **A future blueprint that implements MECH-D47's real `ItemStack`/hotbar model replaces this stub's resolution (which slot's item is "held") with real hotbar-slot lookup — it does not touch this blueprint's dig-timing formula, placement-orientation logic, update-emission, reach validation, or packet handling, all of which are already item-content-agnostic** (the same framing M2-B07 already used for its own, narrower fixed-`STONE` stub).

`GameModeState { pub instabuild: bool }` — a second, minimal `Component`, defaulting to `true` (Creative, matching M1-B05's own hardcoded gamemode) — the smallest possible slice of MECH-D60's abilities model needed to make the creative-vs-survival **branch itself** real and independently testable, rather than only ever exercising the creative path (M2-B07's own limitation). A matching `debug_set_survival(network_entity_id, bool)` setter lets acceptance tests exercise the full survival formula end-to-end even though no code path in this milestone ever sets it to `false` in live production traffic (every M1-B05-spawned player is still Creative by default) — restated explicitly, not silently: **this blueprint does not implement a real gamemode-switch command or MECH-D60's full abilities derivation** — only the one boolean this blueprint's own branch needs, test-settable, exactly mirroring `HeldItemStub`'s own interim-stub framing.

### Reach validation — a box-distance-from-eye predicate, no raycast (MECH-D62, full text)

M3 field-report correction (supersedes this section's own prior text below, which specified a per-player voxel raycast against `rc-physics`'s own voxel shapes — a live-vanilla-client field report found that design rejects a legitimate edge-of-block aim, since the server's own DDA and the client's own picking algorithm can resolve a grazing ray to different neighboring cells; a designated research role's own authoritative verdict, against the ASSET-D18(f) reference, is that vanilla's real server performs **no raycast whatsoever** for block break/place reach): the reach check is `mining::is_within_block_interaction_range(eye, claimed_target, range) -> bool`. It builds the full `1x1x1` axis-aligned box of `claimed_target`'s own block cell — **always** the full unit cell, never the block's real collision/visual shape (a slab, stair, or fence uses the same box as stone) — and accepts iff the squared distance from `eye` to the **nearest point** on that box (per axis, `max(box_min − coord, coord − box_max, 0.0)`, summed as squares — nearest-point-of-box distance, never centre distance, never the client's own reported cursor hit location) is less than `(range + BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER)^2`. `BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER = 1.0` is a fixed slack both the break and the place path add on top of the raw `BLOCK_INTERACTION_RANGE_CREATIVE`/`_SURVIVAL` range before validating, so the server is never *stricter* than the client's own local reach check — absorbing latency, tick-boundary look/position staleness, and float drift. Effective accept thresholds: `5.5` survival, `6.0` creative. There is **no** line-of-sight/occlusion/directional component whatsoever: a claimed target behind another solid block, or approached from any look direction at all (or none), is accepted purely on this distance. `eye` is `movement::eye_position(motion.position, crouching)` — pose-aware (`PLAYER_EYE_HEIGHT` `1.62` standing / `PLAYER_EYE_HEIGHT_CROUCHING` `1.27` crouching, `crouching = shift_key_down && !flying`; this milestone tracks no flying state, so pose reduces to the `player_input` packet's own shift bit alone), not the fixed-height `eye_position(motion.position)` this section's own prior text described.

Placement additionally applies a loose sanity bound on the client-sent cursor hit location (`mining::apply_placement`'s own `cursor_within_sanity_bound`): reconstructing `location = claimed_block_pos + (cursor_x, cursor_y, cursor_z)`, every axis of `location − (claimed_block_pos + 0.5)` must stay under `1.0000001` in absolute value. A legitimate surface hit can only ever be `0.5` off-centre, so this is a generous anti-garbage-payload guard, never a precision or reach limiter — it must never reject a legitimate off-centre aim.

`rc_physics::cast_ray` (below) is unchanged and still shipped — a general-purpose, correct DDA raycast, just no longer the reach call site (`crates/testing/paritybot`'s own bot-aim self-tests are its real remaining caller):

```rust
// crates/physics/src/raycast.rs (new)
use crate::{Vec3, BlockShapeSource};
use rc_core::BlockPos;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RayHit {
    pub block_pos: BlockPos,
    pub hit_point: Vec3,
    pub distance: f64,
}

/// Voxel-grid ray traversal (Amanatidis–Woo style: visits every integer block cell the ray
/// passes through, in ascending-distance order, testing each visited cell's own `VoxelShape`
/// sub-boxes for an exact ray/AABB intersection) up to `max_distance`. Returns the *closest*
/// hit, or `None` if the ray exits `max_distance` without touching a non-empty shape.
/// This is this blueprint's own reasonable general-shape DDA implementation — it is **not**
/// claimed to reproduce vanilla's own `BlockGetter.clip`/`ClipContext` byte-for-byte (that
/// would need its own black-box-capture research pass, out of this blueprint's own budget) —
/// flagged as an M3 open item, mirroring `M3-B02`'s own `xtask extract-shapes` deferral.
pub fn cast_ray(
    origin: Vec3,
    direction: Vec3,
    max_distance: f64,
    shapes: &dyn BlockShapeSource,
) -> Option<RayHit>;
```

Algorithm, precisely: `direction` is assumed pre-normalized by the caller (undefined ordering otherwise — this function does not normalize). Standard 3D DDA: `step_i = direction_i.signum()` per axis; `t_delta_i = (1.0 / direction_i).abs()` (axes with `direction_i == 0.0` get `t_delta_i = f64::INFINITY`, never advance); `t_max_i` initialized to the distance-along-the-ray to the *first* grid line crossing on that axis from `origin`'s own fractional position within its starting cell. Loop: at the current cell, if `shapes.properties_at(cell).shape` is non-empty, ray-test every sub-box (`world_box = sub_box.offset_by(cell)`) via the standard slab method (`t_enter = max over axes of (box.min - origin)/direction` etc., `t_exit` symmetric; a hit exists iff `t_enter <= t_exit` and `t_enter` is within `[0, max_distance]`); among all sub-boxes hit at this cell, keep the smallest `t_enter`; if any sub-box was hit, return `RayHit { block_pos: cell, hit_point: origin + direction * t_enter, distance: t_enter }` immediately (cells are visited in strictly ascending distance order by DDA construction, so the first cell with any hit is the closest hit overall — no need to keep scanning further cells). Otherwise, advance to the next cell: pick the axis with the smallest `t_max_i`, step that axis's cell coordinate by `step_i`, add `t_delta_i` to that axis's `t_max_i`; if the new `t_max_i` for the axis just stepped now exceeds `max_distance`, stop and return `None`.

**Reach-check call site**: `mining::is_within_block_interaction_range(eye, claimed_target, range)`, per this section's own opening paragraph above — `cast_ray` is not part of this call site at all; `claimed_target` is still the packet's own raw, unresolved clicked position (`target_position`, M2-B07's algorithm, unchanged), and `range` is still `BLOCK_INTERACTION_RANGE_CREATIVE`/`_SURVIVAL` gamemode-selected via `GameModeState.instabuild` (unchanged constants, kept from M2-B07).

### Orientation from placement context — per-block rules (MECH-D62/MECH-D38's shared yaw convention)

Every look-direction-dependent placement rule below shares one horizontal-look-vector construction, reusing M3-B02's own already-established `mth_sin`/`mth_cos` yaw convention exactly (no new trig convention invented here): `look = Vec3::new(-mth_sin(yaw_rad) as f64 * cos(pitch_rad), -sin(pitch_rad), mth_cos(yaw_rad) as f64 * cos(pitch_rad))` where `yaw_rad = yaw as f64 * PI / 180.0`, `pitch_rad` likewise (`sin`/`cos` for the pitch term use ordinary `f64::sin`/`cos` — this look vector is a server-authoritative *placement-orientation* input, not a rendered/predicted quantity `18-float-determinism.md`'s trig-table rule binds; only the yaw-driven horizontal component reuses the table, matching M3-B02's own `get_input_vector` precedent exactly, restated here for the second call site that needs it. `look_vector` is no longer a reach-check input at all — MECH-D62's real predicate is direction-independent, "Reach validation" above).

`nearest_horizontal_direction4(yaw) -> Direction4` (`North|South|East|West`, `rc-mechanics`'s own `Direction` values minus `Up`/`Down`): the horizontal look vector's dominant axis, by magnitude (`|look.x|` vs `|look.z|`), signed (`look.x > 0` → `East`, `< 0` → `West`; `look.z > 0` → `South`, `< 0` → `North`).

`nearest_direction6(yaw, pitch) -> Direction`: the full look vector's dominant axis among all three (`|look.x|`, `|look.y|`, `|look.z|`), signed identically (`look.y > 0` → `Down` — a positive-pitch/looking-down convention matches this project's own already-established vanilla pitch sign, restated from M3-B02's own trig section — flagged moderate confidence on the exact `y`-sign mapping, reconciliation item).

| Block(s) | Orientation rule | Fixed defaults at placement (unaffected by orientation) |
|---|---|---|
| Stone | none — `Orientation::None`, no orientation-dependent property exists (M2-B07's own kept default target, restated here so `resolve_orientation`'s `match` is exhaustive over every `PlaceableBlockKind` variant, not just the orientable ones) | — |
| Redstone Wire | none — placed flat, `power=0`. Requires a solid top face on the block below (this blueprint's own simplified check: the block below's `tier1_shape_table()` lookup is the `FULL_CUBE` default row — Constraints); connection shape/power are **not** computed by this blueprint — the immediate shape-update fan-out `UpdateContext::set_block` performs (M3-B01) is what a sibling redstone blueprint's own `on_shape_update`/`on_neighbor_changed` behavior settles, same tick (MECH-D10) | — |
| Redstone Torch / Wall Torch | placed as standing `redstone_torch` if `clicked_face == Up` (on top of a block); as `redstone_wall_torch` with `facing = clicked_face` if `clicked_face` is one of the 4 horizontal directions (sticks out from that face); rejected (`RejectReason::InvalidTorchFace`) if `clicked_face == Down` | `lit = true` |
| Repeater | `facing = nearest_horizontal_direction4(yaw).opposite()` (points away from the placing player — matches every `HorizontalDirectionalBlock`'s own real vanilla convention, restated once here and reused by every other row that says "faces away from the player") | `delay = 1 tick`, `locked = false`, `powered = false` |
| Comparator | `facing = nearest_horizontal_direction4(yaw).opposite()` | `mode = compare`, `powered = false` |
| Piston / Sticky Piston | `facing = nearest_direction6(yaw, pitch).opposite()` (the 6-directional case — a player looking steeply up/down places a vertically-facing piston) | `extended = false` |
| Chest | `facing = nearest_horizontal_direction4(yaw).opposite()` | `type = single` (no double-chest merging — M3-B02's own already-stated scope boundary, unchanged), `waterlogged = false` |
| Furnace / Blast Furnace / Smoker | `facing = nearest_horizontal_direction4(yaw).opposite()` | `lit = false` |
| Hopper | `facing = clicked_face.opposite()`, **clamped**: if that would be `Up`, use `Down` instead (a hopper can never face up — this is a genuinely different rule from every row above: face-derived, not look-direction-derived, restated exactly from vanilla's own well-established `HopperBlock.getStateForPlacement`) | `enabled = true` |

**Raw block-state id resolution.** No generated per-property state-permutation table exists yet — `M0-B07`'s own codegen output (Context of that blueprint) emits exactly **one** named constant per block type (its default state's raw id), nothing for any non-default permutation. This blueprint does not extend `xtask`'s codegen (out of this blueprint's own crate scope) — it hand-authors a small, explicitly provisional lookup, mirroring `M3-B02`'s own `tier1_shape_table()` precedent exactly:

```rust
/// A closed, hand-authored (BlockKind, Orientation) -> raw BlockStateId table (Context:
/// "Raw block-state id resolution" — the production-table entries' literal `u32` values are
/// placeholders pending reconciliation against a real `reports/blocks.json` for protocol 776,
/// Implementation steps; the lookup *algorithm* and every call site using it are final).
pub struct OrientedStateTable { /* private: HashMap<(PlaceableBlockKind, Orientation), u32> */ }

pub enum Orientation { None, Horizontal(rc_mechanics::Direction), Full(rc_mechanics::Direction) }

impl OrientedStateTable {
    /// Test/production-shared constructor — acceptance tests build their own instance with
    /// arbitrary, internally-consistent placeholder ids (proving `select_block_state`'s own
    /// *routing* logic, independent of whether any literal matches real vanilla data);
    /// `tier1_oriented_state_table()` (below) is the one production instance, built with this
    /// blueprint's own best-effort literal placeholders.
    pub fn from_entries(entries: Vec<((PlaceableBlockKind, Orientation), u32)>) -> Self;
    /// Panics (a config-time bug, not a runtime-input bug) if `kind`/`orientation` has no
    /// entry — every tier-1 `(kind, orientation)` pair this blueprint's own placement logic
    /// can ever construct has a row; an unregistered pair reaching this call is this
    /// blueprint's own defect, not a malformed-packet case (which is rejected earlier).
    pub fn lookup(&self, kind: PlaceableBlockKind, orientation: Orientation) -> u32;
}

pub fn tier1_oriented_state_table() -> &'static OrientedStateTable;
```

### Update emission on place/break — into M3-B01's engine, exact order (MECH-D9/D10/D15, restated)

Every successful place or break calls exactly one `ctx.set_block(pos, new_state)` (M3-B01's `UpdateContext`, Deliverables above list its full field set). Per M3-B01's own `border::fan_out_from_changed_block` (already-shipped, unmodified): **the neighbor-changed pass runs to completion first** (6 directions, `NEIGHBOR_CHANGED_ORDER = [West, East, Down, Up, North, South]`, each notifying `dir.apply(pos)` with `from: dir.opposite()`), **then the shape-update pass runs** (6 directions, `SHAPE_UPDATE_ORDER = [West, East, North, South, Down, Up]`) — restated here verbatim from M3-B01's own Context since this blueprint's own acceptance tests assert this exact order end-to-end for the first time against real (non-synthetic) breaking/placing content.

**This blueprint's own settle-to-fixed-point driver**, since `UpdateContext::set_block` only *seeds* the engine's pending buffer (M3-B01's own documented behavior — it does not drain/dispatch) and this blueprint calls it from Stage 3 (before Stage 4's own `run_scheduled_phase` next runs), not from inside `run_scheduled_phase` itself:

```rust
/// Drains `ctx.engine` to a fixed point, dispatching each popped item to
/// `behaviors.resolve(state_at(item's own pos)).on_neighbor_changed`/`on_shape_update` —
/// mirroring the same per-entry settle pattern `stage4.rs`'s own internal implementation
/// uses (M3-B01's own Context/Implementation steps), restated here since M3-B01 exposes the
/// constituent pieces (`NeighborUpdateEngine::drain`, `BlockBehaviorRegistry::resolve`) but no
/// ready-made "settle" helper of its own. A `ShapeUpdate` item whose resolved behavior's
/// `on_shape_update` returns `Some(new_state)` is applied via a **recursive** `ctx.set_block`
/// call from *inside* the handler — safe by construction: `NeighborUpdateEngine::drain`'s own
/// signature (`handler: &mut dyn FnMut(&mut Self, PendingUpdate)`) hands the engine back to
/// the handler as its own `&mut Self` parameter on every call, so the handler's captured
/// `world`/`scheduled`/`events`/`outbound`/`ownership`/`current_tick` (all disjoint from the
/// engine reference it receives fresh each call) can be re-bundled into a fresh `UpdateContext`
/// inside the closure with no aliasing conflict — the same reason `drain`'s own signature is
/// shaped this way in the first place.
fn settle_neighbor_updates(
    world: &mut dyn rc_mechanics::BlockWorldAccess,
    engine: &mut rc_mechanics::NeighborUpdateEngine,
    scheduled: &mut rc_mechanics::ScheduledTickQueue,
    events: &mut rc_mechanics::BlockEventQueue,
    outbound: &mut Vec<(rc_messaging::Address, rc_messaging::RegionMessage)>,
    ownership: &rc_mechanics::RegionOwnership,
    behaviors: &rc_mechanics::BlockBehaviorRegistry,
    current_tick: u64,
);
```

No real `BlockBehavior` ships in this blueprint (every tier-1 block resolves to `NoOpBehavior` in this blueprint's own test suite, since the redstone-component blueprint that registers real wire/repeater/piston behaviors is a sibling, not a prerequisite) — this driver is written generically and correctly for whatever behaviors a future blueprint registers, but is only exercised against `NoOpBehavior` here (mirroring M3-B01's own Constraint (d) framing exactly).

### Wiring M3-B01's Stage-4 substrate into `HardcodedWorld` for the first time

Neither M2-B07 nor M3-B02 called `rc_mechanics::stage4::ecs::{bootstrap_default_stage4_resources, register_stage4}` — `HardcodedWorld`'s region `World` has never carried M3-B01's resources, and Stage 4 has run zero registered systems in every prior blueprint. This blueprint is the first to wire it in (necessary: `settle_neighbor_updates` above needs `NeighborUpdateEngine`/`ScheduledTickQueue`/`BlockEventQueue`/`BlockBehaviorRegistry`/`BorderHalo` to already be present as resources in `region.world`, and any scheduled tick or block event this blueprint's own future siblings register needs Stage 4 actually running to drain them):

- `HardcodedWorld::new()`'s `bootstrap: fn(&mut World)` closure additionally calls `rc_mechanics::stage4::ecs::bootstrap_default_stage4_resources(world)` (inserts the six `Default`-able resources).
- The `RcExecutorBuilder` construction additionally calls `rc_mechanics::stage4::ecs::register_stage4(&mut builder)` before `.build()` (registers M3-B01's two Stage-4 systems into `DomainGroup::BlockRedstone`).
- Immediately after `RcExecutor::spawn_region` returns (mirroring M3-B01's own documented pattern exactly, and M0-B06's `SyntheticLoadProfile` precedent it cites): `region.world.insert_resource(RegionOwnership { local: Address::Region(HARDCODED_REGION_ID), resolve: Box::new(|key| if LOCAL_CHUNK_KEYS.contains(&key) { Address::Region(HARDCODED_REGION_ID) } else { Address::Region(RegionId(u64::MAX)) }) })` — reusing M2-B07's own already-established `resolve_owner` closure shape verbatim, now as M3-B01's own `RegionOwnership` type instead of a bare closure parameter threaded per call site.

With this wiring in place, Stage 4 now runs for real every tick. Under this milestone's own tier-1 scope (no fluids, no leaf decay, no real redstone behavior registered), it is inert in the steady state (nothing scheduled, no inbound border events under single-region operation, `NoOpBehavior` everywhere) — this blueprint's own acceptance tests confirm exactly that (Acceptance tests, "stage4_is_inert_with_no_registered_behavior").

### Which pipeline stage — restated concretely for the manual tick loop

Mirrors M2-B07's/M3-B02's own already-established shape: two ordered steps inserted into `HardcodedWorld`'s tick loop, **after** the movement steps M3-B02 already inserted, **before** `executor.tick_region(...)`:

1. **Packet-apply substep** (Stage-3-equivalent): every queued `PlayerAction`/`UseItemOn`-derived action, drained and stable-sorted by ascending `network_entity_id` (MECH-D4's own determinism rule, unchanged from M2-B07), reach-validated (`mining::is_within_block_interaction_range`, "Reach validation" above), then dispatched into `mining::apply_mining_action` (break-status actions) or the placement path — each mutation immediately followed by this blueprint's own `settle_neighbor_updates` call (Context above), so every neighbor-visible effect of a place/break is fully settled *before* the next queued action in the same tick is processed, matching MECH-D10's same-tick-visible guarantee transitively through the whole ordered batch.
2. **Destroy-state tick substep**: for every player currently in the region, run the per-player `tick()` logic (Context, "Dig packet lifecycle") — recompute/rebroadcast crack-stage progress or finalize a delayed destroy, exactly mirroring vanilla's own real "tick() runs once per player, after that tick's packets" ordering.

### Creative vs. survival paths — restated concretely

Creative (`GameModeState.instabuild == true`, M1-B05's own live default for every M3 player): `START_DESTROY_BLOCK` destroys immediately, unconditionally — no dig-timing formula evaluated at all, no `DestroyState` entered, no drop computed (creative never drops, matching M2-B07's own kept precedent), `STOP`/`ABORT_DESTROY_BLOCK` are decoded, acked, and otherwise no-ops (the block is already gone by the time either could arrive). Placement is identical in both modes (no hotbar depletion in either — MECH-D47/M4, restated from M2-B07's own "zero inventory mutation in either direction" stance, still valid since no inventory exists).

Survival (`instabuild == false`, test-only-reachable via `debug_set_survival` at M3's own scope): the full dig-timing state machine above; placement is unchanged (still no hotbar depletion — a real inventory model, M4, is what makes placement consume the held stack; this blueprint's own held-item stub has no "count," so there is nothing to deplete even conceptually).

### Drops stance at M3 — explicit interim decision (MECH-D51, item entities are M4)

Per the M3 milestone's own boundary text ("item DROPS from breaking: follow 05's tiering; if drops need item entities, M3 stance must be explicit, not silent") and MECH-D51 (item entities, M4 scope): this blueprint **computes** `has_correct_tool_for_drops` at break-finalize time (Context's own per-block table) and records the outcome (`BreakOutcome::Applied{drop_eligible: bool, ..}` — Deliverables) for test-assertion purposes, but **spawns no item entity under any circumstance** — there is no `ItemEntity`/entity-spawning mechanism anywhere in this project before M4. This is the honest, direct extension of M2-B07's own "zero inventory mutation" stance to breaking's drop half specifically: a survival-mode break with an eligible tool is recorded as "would have dropped," not silently treated as if drops did not matter at all, but the item never materializes in the world at M3. **A future M4 blueprint that implements MECH-D51's real item entities extends `BreakOutcome`'s `Applied` arm to actually spawn one when `drop_eligible` is true — not this blueprint's dig-timing formula, tool-effectiveness computation, or any other part of this blueprint.**

## Deliverables

### `crates/physics/src/raycast.rs` (new)

Full signature already given in Context ("Reach validation"): `RayHit`, `cast_ray`.

### `crates/physics/src/lib.rs` (modify — add one module + re-export line; every existing line unchanged)

```rust
pub mod raycast;
pub use raycast::{cast_ray, RayHit};
```

### `crates/server/Cargo.toml` (modify — add one normal dependency)

```toml
[dependencies]
rc-mechanics = { path = "../mechanics", features = ["server-systems"] }
```

(Every other line unchanged from M1-B01/M2-B07/M3-B02. `rc-physics` is already a dependency since M3-B02 — this blueprint adds no line for it.)

### `crates/server/src/play/packets.rs` (modify — redefine two packets, add two new ones; every other line unchanged)

```rust
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x29)]
pub struct PlayerAction {
    #[rc(varint)] pub status: i32,
    pub location: i64,
    #[rc(varint)] pub direction: i32,
    #[rc(varint)] pub sequence: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x42)]
pub struct UseItemOn {
    #[rc(varint)] pub hand: i32,
    pub location: i64,
    #[rc(varint)] pub direction: i32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
    pub inside_block: bool,
    #[rc(varint)] pub sequence: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x05)]
pub struct SetBlockDestroyStage {
    #[rc(varint)] pub entity_id: i32,
    pub location: i64,
    pub destroy_stage: i8,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x2E)]
pub struct LevelEvent {
    pub event_id: i32,
    pub location: i64,
    pub data: i32,
}

pub const LEVEL_EVENT_BLOCK_BREAK: i32 = 2001;
```

(`BlockUpdate`/`AcknowledgeBlockChange`, M2-B07's own, are unchanged — not repeated here.)

### `crates/server/src/play/mining.rs` (new)

```rust
use rc_chunk_storage::BlockStateId as StorageBlockStateId;
use rc_core::BlockPos;
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, Direction, NeighborUpdateEngine,
    RegionOwnership, ScheduledTickQueue,
};
use rc_messaging::{Address, RegionMessage};
use rc_physics::{cast_ray, BlockShapeSource, Vec3};

use crate::play::block_action::Face;
use crate::play::movement::PlayerMotion;

// --- Held-item / gamemode stubs (Context) ---
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlaceableBlockKind { Stone, RedstoneWire, RedstoneTorch, Repeater, Comparator, Piston, StickyPiston, Chest, Furnace, BlastFurnace, Smoker, Hopper }
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToolMaterial { None, Wood, Stone, Iron, Diamond, Netherite, Gold }
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToolKind { None, Pickaxe, Axe, Shovel }
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HeldItemStub { Block(PlaceableBlockKind), Tool(ToolMaterial, ToolKind), EmptyHand }

impl ToolMaterial {
    /// Context's own table. `None` (bare hand) is `1`.
    pub const fn speed_multiplier(self) -> f64;
    /// Context's own tier table (`None`/`Wood`/`Gold` = 0, `Stone` = 1, `Iron` = 2,
    /// `Diamond`/`Netherite` = 3). `None` (bare hand) still returns `0` here — tier alone
    /// never grants `has_correct_tool_for_drops`; `ToolKind` must also match (see
    /// `has_correct_tool_for_drops`).
    pub const fn tier(self) -> u8;
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, bevy_ecs::prelude::Component)]
pub struct GameModeState { pub instabuild: bool }
// Default derives `instabuild: false`; the join-drain step (world.rs) explicitly
// constructs `GameModeState { instabuild: true }` instead (Context: matches M1-B05's own
// hardcoded Creative default) — `Default` above exists only to satisfy `#[derive(Default)]`
// ergonomics elsewhere, never relied on for the real spawn value.

#[derive(Copy, Clone, Debug, PartialEq, Eq, bevy_ecs::prelude::Component)]
pub struct HeldItem(pub HeldItemStub);
// Spawned as `HeldItem(HeldItemStub::Block(PlaceableBlockKind::Stone))` (Context).

/// Per-block-type physical properties this blueprint's own formula needs (Context's own
/// tier-1 table). `min_tier_for_drops: None` means "any tool, including bare hand, always
/// drops" (Context's own per-row rule); `Some(t)` means "tier >= t AND ToolKind matches
/// `effective_tool`."
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DigProperties {
    pub hardness: f64,
    pub effective_tool: ToolKind,
    pub min_tier_for_drops: Option<u8>,
}
pub fn dig_properties(kind: PlaceableBlockKind) -> DigProperties;

/// `Instant` for hardness == 0, `Unbreakable` for hardness < 0, `PerTick(progress)` otherwise
/// (Context: the div-by-zero-avoiding special cases, restated).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DestroySpeed { Instant, Unbreakable, PerTick(f64) }

/// The complete dig-timing formula (Context, full algorithm). `haste_level`/`fatigue_level`
/// are `0` for "effect not active" (both formulas below already no-op at level 0 without a
/// separate `Option`/bool gate: `speed *= 1.0 + 0.2*0` and `MINING_FATIGUE_MULTIPLIER` is
/// simply not applied when `fatigue_level == 0`).
pub fn destroy_speed(
    props: DigProperties,
    tool: (ToolMaterial, ToolKind),
    efficiency_level: u8,
    haste_level: u8,
    fatigue_level: u8,
    in_water_no_aqua_affinity: bool,
    airborne: bool,
) -> DestroySpeed;

/// `true` iff `tool`'s kind matches `props.effective_tool` AND (no minimum tier is required,
/// or `tool.0.tier() >= required`).
pub fn has_correct_tool_for_drops(props: DigProperties, tool: (ToolMaterial, ToolKind)) -> bool;

/// `ceil(1.0 / progress_per_tick)` for `DestroySpeed::PerTick`; `1` for `Instant`; panics for
/// `Unbreakable` (a caller must never reach the survival tick-count path for an unbreakable
/// block — MECH-D61's own "never breaks" rule is enforced earlier, at `START_DESTROY_BLOCK`
/// time, by refusing to enter `DestroyState` at all for a hardness-negative target).
pub fn ticks_to_break(speed: DestroySpeed) -> u64;

// --- Dig packet lifecycle (Context) ---
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, bevy_ecs::prelude::Component)]
pub struct DestroyState {
    pub is_destroying: bool,
    pub destroy_pos: BlockPos,
    pub destroy_progress_start: u64,
    pub has_delayed_destroy: bool,
    pub delayed_destroy_pos: BlockPos,
    pub delayed_tick_start: u64,
    pub last_sent_stage: i8, // -1 initial, via a `Default` override in the join-drain step
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DestroyOutcome {
    /// Creative-instant or the "insta-mine" survival case — finalize now.
    FinalizeNow,
    /// Survival, not yet complete — `DestroyState` now tracks an active destroy.
    Tracking,
}

/// `START_DESTROY_BLOCK`'s own logic (Context, full algorithm). `current_tick` is this
/// region's own `CurrentTick` resource value.
pub fn begin_destroy(
    state: &mut DestroyState,
    pos: BlockPos,
    instabuild: bool,
    speed: DestroySpeed,
    current_tick: u64,
) -> DestroyOutcome;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StopOutcome { FinalizeNow, DelayedQueued, NothingQueued }
/// `STOP_DESTROY_BLOCK`'s own logic (Context). `speed` is the SAME `DestroySpeed` snapshot
/// `begin_destroy` was called with (Context: "does not re-sample tool/effects mid-dig").
pub fn stop_destroy(state: &mut DestroyState, pos: BlockPos, speed: DestroySpeed, current_tick: u64) -> StopOutcome;

/// `ABORT_DESTROY_BLOCK`'s own logic (Context) — clears only `is_destroying`.
pub fn abort_destroy(state: &mut DestroyState);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TickOutcome {
    Idle,
    /// `(stage 0..=9)` — rebroadcast owed iff this differs from `state.last_sent_stage`
    /// (caller's own responsibility; `last_sent_stage` is updated by the caller after
    /// broadcasting, not by this function, so a caller can distinguish "should I send" from
    /// "did I already record having sent").
    ActiveProgress(u8),
    CancelledBlockChanged,
    FinalizeDelayedNow,
    CancelledDelayedBlockChanged,
}
/// Per-player `tick()`'s own logic (Context). `current_state_at_pos`/`current_state_at_delayed_pos`
/// are the caller's own already-fetched current block states at the relevant tracked position(s)
/// (this function never touches `BlockWorldAccess` itself, keeping it a plain, independently
/// testable state-machine step).
pub fn tick_destroy_state(
    state: &mut DestroyState,
    speed: DestroySpeed,
    current_tick: u64,
    current_state_at_pos: StorageBlockStateId,
    current_state_at_delayed_pos: StorageBlockStateId,
    air: StorageBlockStateId,
) -> TickOutcome;

// --- Reach (Context, "Reach validation" above — M3 field-report correction) ---
pub const BLOCK_INTERACTION_RANGE_SURVIVAL: f64 = 4.5;
pub const BLOCK_INTERACTION_RANGE_CREATIVE: f64 = 5.0;
pub const BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER: f64 = 1.0;

/// Context's own shared look-vector construction -- no longer a reach-check input (see
/// below); still reused by every placement-orientation rule and (a future blueprint's) any
/// other look-driven mechanic.
pub fn look_vector(yaw_degrees: f32, pitch_degrees: f32) -> Vec3;
pub fn nearest_horizontal_direction4(yaw_degrees: f32) -> Direction;
pub fn nearest_direction6(yaw_degrees: f32, pitch_degrees: f32) -> Direction;

/// Context's own full algorithm ("Reach validation" above): the squared distance from `eye`
/// to the NEAREST POINT of `claimed_target`'s own full unit-cell box, compared against
/// `(range + BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER)^2`. No raycast, no direction
/// input at all.
pub fn is_within_block_interaction_range(eye: Vec3, claimed_target: BlockPos, range: f64) -> bool;

// --- Placement orientation (Context) ---
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Orientation { None, Horizontal(Direction), Full(Direction) }

/// Context's own per-block-type table, dispatched by `kind`. `clicked_face`/`yaw`/`pitch` are
/// the inputs each row's own rule (Context) actually reads; unused inputs for a given `kind`
/// are simply ignored (e.g. torches ignore yaw/pitch entirely).
pub fn resolve_orientation(kind: PlaceableBlockKind, clicked_face: Face, yaw_degrees: f32, pitch_degrees: f32) -> Result<PlacementSelection, RejectReason>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PlacementSelection { pub kind: PlaceableBlockKind, pub orientation: Orientation, pub is_wall_variant: bool }

pub struct OrientedStateTable { /* Context, full shape already given */ }
pub fn tier1_oriented_state_table() -> &'static OrientedStateTable;

// --- Top-level action application ---
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    OutOfReach, TargetNotAir, TargetAlreadyAir, InvalidTorchFace, NoSolidSupportBelow,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BreakOutcome { Applied { pos: BlockPos, drop_eligible: bool }, Rejected { pos: BlockPos, reason: RejectReason, current_state: u32 } }
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlaceOutcome { Applied { pos: BlockPos, new_state: u32 }, Rejected { pos: BlockPos, reason: RejectReason, current_state: Option<u32> } }

/// Finalizes a break: reads current state at `pos`, rejects `TargetAlreadyAir` if already
/// air; else computes `drop_eligible` (`has_correct_tool_for_drops`, `false` unconditionally
/// if `instabuild` — creative never drops, Context), calls `ctx.set_block(pos, AIR)` then
/// this file's own `settle_neighbor_updates` (Context, full algorithm already given).
pub fn finalize_break(
    ctx_world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    ownership: &RegionOwnership,
    behaviors: &BlockBehaviorRegistry,
    current_tick: u64,
    pos: BlockPos,
    instabuild: bool,
    tool: (ToolMaterial, ToolKind),
) -> BreakOutcome;

/// Placement: resolves the target position (`block_action::target_position`, unchanged),
/// checks `TargetNotAir`, resolves orientation (`resolve_orientation`), resolves the raw
/// state via `tier1_oriented_state_table()`, calls `ctx.set_block` + `settle_neighbor_updates`.
/// Wire-connection blocks additionally check `NoSolidSupportBelow` (Context's own simplified
/// "block below is the `FULL_CUBE` default shape-table row" rule) before calling `set_block`.
pub fn apply_placement(
    ctx_world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    ownership: &RegionOwnership,
    behaviors: &BlockBehaviorRegistry,
    current_tick: u64,
    location: BlockPos,
    face: Face,
    inside_block: bool,
    held: HeldItemStub,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> PlaceOutcome;
```

### `crates/server/src/play/block_action.rs` (modify)

`apply_block_action`/`ApplyOutcome`/`BlockActionKind` (M2-B07) are **removed** — superseded by `mining.rs`'s own richer entry points (Context). `Face`/`from_ordinal`/`offset`, `resolve_place_position`, `target_position`, `to_storage_id`/`to_storage_biome_id`, `seed_chunk_column`, `ChunkIndex`, `debug_query_block`, `DebugBlockInfo`, `PendingBlockAction` (restructured, below), `RejectReason` (**moved** to `mining.rs`, Context's own expanded variant set — this file no longer defines it) are kept, per Context's own "kept unchanged" list. `PendingBlockAction`'s `kind` field is retyped:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockActionKind {
    StartDestroy { location: BlockPos },
    StopDestroy { location: BlockPos },
    AbortDestroy { location: BlockPos },
    Place { location: BlockPos, face: Face, inside_block: bool },
    Ignored,
}
```

(`PlayerAction.status` 0/2/1 map to `StartDestroy`/`StopDestroy`/`AbortDestroy` respectively — restated once here since M2-B07's own comment only named `Break`/`Ignored`; 3..=6 remain `Ignored`, unchanged.)

### `crates/server/src/play/world.rs` (modify)

Join-drain step additionally inserts `GameModeState { instabuild: true }`, `HeldItem(HeldItemStub::Block(PlaceableBlockKind::Stone))`, `DestroyState { last_sent_stage: -1, ..Default::default() }` on the newly-spawned player entity (alongside M3-B02's own `PlayerMotion`/`TeleportState`). `HardcodedWorld` gains:

```rust
impl HardcodedWorld {
    /// Test/diagnostic only (Context: mirrors `debug_query_block`'s precedent).
    pub fn debug_set_held_item(&self, network_entity_id: i32, item: HeldItemStub);
    pub fn debug_set_survival(&self, network_entity_id: i32, survival: bool);
    /// Test/diagnostic only — reads `NeighborUpdateEngine::is_idle()`,
    /// `ScheduledTickQueue::block_len()`/`fluid_len()`, `BlockEventQueue::pending_next_tick()`
    /// straight off `region.world`'s own M3-B01 resources (Acceptance tests,
    /// `mining_stage4_wiring.rs`). Awaits the next tick's drain, mirroring `debug_query_block`.
    pub fn debug_stage4_counters(&self) -> impl std::future::Future<Output = Stage4Counters>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Stage4Counters {
    pub neighbor_engine_idle: bool,
    pub block_ticks_pending: usize,
    pub fluid_ticks_pending: usize,
    pub block_events_pending_next_tick: usize,
}
```

`RcExecutorBuilder`/`bootstrap`/`spawn_region` wiring per Context's own "Wiring M3-B01's Stage-4 substrate" subsection (`bootstrap_default_stage4_resources`, `register_stage4`, per-region `RegionOwnership` insertion) — applied once, at `HardcodedWorld::new()`'s own construction, not per-tick.

Tick loop: the two new manual steps (Context, "Which pipeline stage") inserted after M3-B02's own movement steps, before `executor.tick_region(...)`. Packet-apply substep, per queued action (stable-sorted by `network_entity_id`): resolve `target_position`; `mining::is_within_block_interaction_range` against the acting player's own pose-aware eye position (`movement::eye_position(motion.position, crouching)`, "Reach validation" above) — reject `OutOfReach` (ack-only, no correction packet, matching M2-B07's own kept precedent) on failure; else dispatch to `mining::finalize_break`/`abort_destroy`/`stop_destroy`/`begin_destroy`/`apply_placement` per `BlockActionKind`, always sending exactly one `AcknowledgeBlockChange` first (MECH-D63, unchanged), broadcasting `BlockUpdate` + `LevelEvent{event_id: LEVEL_EVENT_BLOCK_BREAK, data: <pre-break state id>}` to every connected player on a finalized break, `BlockUpdate` only (no `LevelEvent`) on a successful placement, and a corrective `BlockUpdate` to the actor only on any `Rejected{current_state: Some(..)}` outcome (all unchanged broadcast shapes from M2-B07, restated). Destroy-state tick substep: for every player, `mining::tick_destroy_state`, dispatching `ActiveProgress`/`FinalizeDelayedNow`/`Cancelled*` per Context's own algorithm, broadcasting `SetBlockDestroyStage` to every *other* player on a changed `ActiveProgress` stage.

### `crates/server/src/play/connection.rs` (modify)

`0x29` (`PlayerAction`, corrected `direction` field): `Face::from_ordinal(packet.direction)`, `status` 0/1/2 map to `BlockActionKind::StartDestroy/AbortDestroy/StopDestroy`, else `Ignored`. `0x42` (`UseItemOn`, corrected id + `direction` field name): `BlockActionKind::Place{location, face, inside_block}`, unchanged shape otherwise.

### `crates/server/src/play/mod.rs` (modify — re-exports)

Adds `mod mining;` and a `pub use mining::{...}` block covering every public item Deliverables lists above; removes `apply_block_action`/`ApplyOutcome`/`BlockActionKind`'s **old** re-export line (M2-B07's), replaced by the new `BlockActionKind` (now defined in `block_action.rs`, per this blueprint's own restructuring) and `mining`'s own exports.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly per every prior M3 blueprint's own identical framing):** every file below, plus every `src/*.rs` file Deliverables lists with each function body replaced by `todo!()` (fields/derives/doc comments unchanged), is the test-authoring changeset, committed first. The implementation changeset fills in bodies only — it must not modify any file under `crates/physics/tests/raycast_*.rs` or `crates/server/tests/{mining_*.rs, play_block_*.rs, play_dig_*.rs}`, must not add/remove/rename a test case, and must not weaken or change any golden-table expected value.

### `crates/physics/tests/raycast_basic.rs`

1. `unobstructed_ray_returns_none_within_air` — no blocks at all (air everywhere), `cast_ray(Vec3::new(0.5,0.5,0.5), Vec3::new(0.0,0.0,1.0), 10.0, &EmptyWorld)` returns `None`.
2. `ray_hits_adjacent_full_cube_immediately` — `SingleBlock(BlockPos::new(0,0,1), VoxelShape::full_cube())`; ray from `(0.5,0.5,0.5)` direction `(0,0,1)`, `max_distance=10.0`; assert `Some(RayHit{block_pos: BlockPos::new(0,0,1), distance, ..})` with `(distance - 0.5).abs() < 1e-9` (the ray travels exactly `0.5` blocks before entering the cube's near face at `z=1.0`).
3. `ray_stops_at_first_non_empty_cell_not_a_farther_one` — two blocks, `SingleBlock`-style double: full cubes at `(0,0,1)` and `(0,0,2)`; same ray as test 2; assert the hit's `block_pos == BlockPos::new(0,0,1)` (the nearer one), never `(0,0,2)`.
4. `ray_exceeding_max_distance_returns_none` — full cube at `(0,0,5)`; ray direction `(0,0,1)`, `max_distance = 2.0`; returns `None` (the block is `4.5` blocks away, past the budget).
5. `diagonal_ray_visits_cells_in_correct_dda_order` — full cube at `(2,0,2)` only (air everywhere else, including at `(1,0,1)`, `(2,0,1)`, `(1,0,2)` — the cells a naive diagonal step might visit out of order); ray from `(0.5,0.5,0.5)` direction `(1,0,1).normalized()`, `max_distance=10.0`; asserts a hit at exactly `block_pos == BlockPos::new(2,0,2)` (proves the traversal doesn't skip past or misorder diagonal cells).

### `crates/server/tests/mining_dig_timing_golden_table.rs` (pure, no sockets — the "dig-timing golden table")

Table-driven test, one case per row, calling `destroy_speed`/`ticks_to_break` directly (no `HardcodedWorld`). Every row's `ticks` value is the hand-computed result per Context's own formula, restated here as the binding expected value:

| # | Block | Tool | Efficiency | Haste | Fatigue | Water(no AA) | Airborne | Expected ticks |
|---|---|---|---|---|---|---|---|---|
| 1 | Stone | bare hand | 0 | 0 | 0 | no | no | 150 |
| 2 | Stone | Wood pickaxe | 0 | 0 | 0 | no | no | 23 |
| 3 | Stone | Iron pickaxe | 0 | 0 | 0 | no | no | 8 |
| 4 | Stone | Diamond pickaxe | 0 | 0 | 0 | no | no | 6 |
| 5 | Stone | Diamond pickaxe | 5 | 0 | 0 | no | no | 2 |
| 6 | Dirt | bare hand | 0 | 0 | 0 | no | no | 15 |
| 7 | Dirt | Wood shovel | 0 | 0 | 0 | no | no | 8 |
| 8 | Grass Block | Wood shovel | 0 | 0 | 0 | no | no | 9 |
| 9 | Piston | bare hand | 0 | 0 | 0 | no | no | 45 |
| 10 | Piston | Iron pickaxe | 0 | 0 | 0 | no | no | 8 |
| 11 | Chest | bare hand | 0 | 0 | 0 | no | no | 75 |
| 12 | Chest | Wood axe | 0 | 0 | 0 | no | no | 38 |
| 13 | Furnace | bare hand | 0 | 0 | 0 | no | no | 350 (and `has_correct_tool_for_drops == false`) |
| 14 | Furnace | Wood pickaxe | 0 | 0 | 0 | no | no | 175 (and `has_correct_tool_for_drops == false` — tier 0 < required tier 1, despite the tool being speed-effective) |
| 15 | Furnace | Stone pickaxe | 0 | 0 | 0 | no | no | 27 (and `has_correct_tool_for_drops == true`) |
| 16 | Hopper | Iron pickaxe | 0 | 0 | 0 | no | no | 15 |
| 17 | Stone | Iron pickaxe | 0 | 0 | 2 (Mining Fatigue II) | no | no | 84 |
| 18 | Stone | Iron pickaxe | 0 | 2 (Haste II) | 0 | no | no | 6 |
| 19 | Stone | Iron pickaxe | 0 | 0 | 0 | yes | no | 38 |
| 20 | Stone | Iron pickaxe | 0 | 0 | 0 | yes | yes | 188 |

Two further, non-tabular cases in the same file: `redstone_wire_torch_repeater_comparator_are_always_instant` — `dig_properties`/`destroy_speed` for each of the four hardness-0 tier-1 blocks returns `DestroySpeed::Instant` regardless of tool/efficiency/fatigue arguments passed (assert with at least one deliberately "slow" combination, e.g. bare hand + Mining Fatigue IV, still `Instant`); `bedrock_is_unbreakable` — `dig_properties(Bedrock-equivalent)` (this file's own synthetic `DigProperties{hardness: -1.0, ..}`, since `Bedrock` is not itself a `PlaceableBlockKind` — it is breakable-in-survival-never but never *placeable*, so this test constructs the `DigProperties` value directly rather than going through `dig_properties(kind)`) `destroy_speed` returns `DestroySpeed::Unbreakable` for every input combination tried.

### `crates/server/tests/mining_destroy_state_machine.rs` (pure)

1. `start_destroy_enters_tracking_for_a_multi_tick_block` — `DestroySpeed::PerTick(1.0/23.0)` (Stone/Wood-pickaxe, from the golden table), `begin_destroy(&mut state, pos, false, speed, 100)` returns `Tracking`; `state.is_destroying == true`, `state.destroy_pos == pos`, `state.destroy_progress_start == 100`.
2. `start_destroy_finalizes_immediately_for_instant_blocks` — `DestroySpeed::Instant`, `begin_destroy(.., false, speed, 100)` returns `FinalizeNow`; `state.is_destroying == false` (never entered tracking at all).
3. `start_destroy_always_finalizes_in_creative_regardless_of_speed` — `DestroySpeed::PerTick(1.0/150.0)` (a very slow dig), `begin_destroy(.., true, speed, 100)` (instabuild) returns `FinalizeNow`.
4. `stop_before_threshold_queues_delayed_destroy` — from test 1's own tracking state, `current_tick = 105` — elapsed `= current_tick - start + 1 = 6` (the `+1` convention restated in Context, "insta-mine" paragraph), progress `6/23 ≈ 0.261`, below `0.7` — `stop_destroy(&mut state, pos, speed, 105)` returns `DelayedQueued`; `state.is_destroying == false`, `state.has_delayed_destroy == true`, `state.delayed_tick_start == 100` (the **original** start tick, not `105`).
5. `stop_at_or_above_threshold_finalizes_immediately` — from test 1's own tracking state, `current_tick = 116` (`17` ticks elapsed, `17/23 ≈ 0.739 ≥ 0.7`), `stop_destroy(.., 116)` returns `FinalizeNow`.
6. `abort_clears_active_but_not_delayed` — build a state with both `is_destroying = true` (at some position) and `has_delayed_destroy = true` (at a *different* position, hand-constructed) simultaneously; `abort_destroy(&mut state)` leaves `is_destroying == false` but `has_delayed_destroy == true`, unchanged.
7. `tick_reports_rising_stage_and_detects_cancellation` — from test 1's own tracking state (`speed = PerTick(1/23)`, `start=100`), `tick_destroy_state(.., current_tick=105, current_state_at_pos=<stone's raw id>, .., air)` returns `ActiveProgress(stage)` where `stage == floor((6.0/23.0)*10.0) as u8` (elapsed `= 105-100+1 = 6`; `6/23≈0.2609`, `×10≈2.609`, `floor=2`); a second call with `current_state_at_pos == air` (the block already gone) returns `CancelledBlockChanged`.
8. `delayed_destroy_finalizes_once_progress_reaches_one_via_tick` — from test 4's own delayed state (`delayed_tick_start=100`), `tick_destroy_state(.., current_tick=122, ..)` — elapsed `= 122-100+1 = 23` ticks at `1/23` per tick `== 1.0` exactly (the golden table's own predicted tick-count for this exact tool/block pair) — returns `FinalizeDelayedNow`.

### `crates/server/tests/mining_placement_orientation.rs` (pure)

One case per tier-1 orientable block, calling `resolve_orientation` directly:

1. `repeater_faces_away_from_player` — `resolve_orientation(Repeater, Face::Up /*unused*/, yaw_degrees=0.0 /* looking South, vanilla convention */, pitch=0.0)` returns `Orientation::Horizontal(Direction::North)` (faces away from a player looking South — hand-derived from Context's own `nearest_horizontal_direction4(0.0).opposite()`: yaw 0 → South per this blueprint's own `look_vector` construction → opposite → North).
2. `piston_faces_up_when_player_looks_steeply_down` — `resolve_orientation(Piston, Face::Up, yaw_degrees=0.0, pitch_degrees=80.0)` (a steep downward look, this project's own pitch-positive-is-down convention, Context) returns `Orientation::Full(Direction::Up)` (opposite of the dominant-downward look axis).
3. `torch_on_top_face_is_standing` — `resolve_orientation(RedstoneTorch, Face::Up, 0.0, 0.0)` returns `Ok(PlacementSelection{kind: RedstoneTorch, orientation: Orientation::None, is_wall_variant: false})`.
4. `torch_on_side_face_is_wall_variant_facing_that_side` — `resolve_orientation(RedstoneTorch, Face::North, 0.0, 0.0)` returns `Ok(PlacementSelection{orientation: Orientation::Horizontal(Direction::North), is_wall_variant: true, ..})`.
5. `torch_on_bottom_face_is_rejected` — `resolve_orientation(RedstoneTorch, Face::Down, 0.0, 0.0)` returns `Err(RejectReason::InvalidTorchFace)`.
6. `hopper_faces_opposite_the_clicked_side_face` — `resolve_orientation(Hopper, Face::North, 0.0, 0.0)` returns `Orientation::Horizontal(Direction::South)` (opposite of North).
7. `hopper_clicked_on_top_defaults_to_facing_down_never_up` — `resolve_orientation(Hopper, Face::Up, 0.0, 0.0)` returns `Orientation::Full(Direction::Down)` (the clamp — opposite of `Up` would be `Down` already in this specific case, so this test additionally covers `Face::Down` clicked, whose naive opposite is `Up`, clamped to `Down`: assert `resolve_orientation(Hopper, Face::Down, 0.0, 0.0) == Ok(PlacementSelection{orientation: Orientation::Full(Direction::Down), ..})` too).
8. `chest_and_furnace_share_the_same_horizontal_away_from_player_rule` — `resolve_orientation(Chest, Face::Up, 90.0, 0.0)` and `resolve_orientation(Furnace, Face::Up, 90.0, 0.0)` both return `Orientation::Horizontal(<the same Direction>)` (proves the shared rule, not a per-block special case, by construction rather than by asserting a specific literal — the literal value is exercised by test 1's own worked example).

### `crates/server/tests/play_block_break_place_full.rs` (sockets, mirrors `M2-B07`'s own `play_block_place_break.rs` shape, extended)

1. `creative_break_is_still_instant_and_broadcasts_level_event` — two connections `A`/`B`, `A` sends `PlayerAction{status:0, location: pack_position(BlockPos::new(0,-60,0)), direction:1, sequence:1}` (unchanged target from M2-B07's own test, now via the corrected `direction` field name and the real `PlayerMotion`-based reach check — `A`'s spawned position/eye still resolves in range identically to M2-B07's own fixed-position case, since M3-B02's own spawn-time `PlayerMotion` initializes to the same `SPAWN_POSITION`). `A` reads `AcknowledgeBlockChange{sequence:1}`, `BlockUpdate{location, block_state_id:<AIR>}`, `LevelEvent{event_id: LEVEL_EVENT_BLOCK_BREAK, location, data: <GRASS_BLOCK's raw id — the block's state *before* the break>}`, in that order. `B` reads the identical `BlockUpdate` then `LevelEvent` (broadcast to both, unlike the crack-overlay packet).
2. `survival_multi_tick_break_shows_rising_crack_stages_then_finalizes_on_stop` — `A` calls `world.debug_set_survival(A_id, true)`, `world.debug_set_held_item(A_id, HeldItemStub::Tool(ToolMaterial::Wood, ToolKind::Pickaxe))`, targets a *fresh* `Stone`-seeded test position (this test's own `HardcodedWorld` first has that one position placed via a creative `UseItemOn` with a `HeldItemStub::Block(Stone)` held item, to guarantee a known-`Stone` target independent of the fixed superflat layer table) — golden table row 2 (`23` ticks). `A` sends `PlayerAction{status:0, ..}` (`START_DESTROY_BLOCK`); `B` (a second, observing connection) reads a sequence of `SetBlockDestroyStage` packets over the next several server ticks with strictly non-decreasing `destroy_stage` values, reaching `9` no later than the tick at which elapsed ticks (`current_tick - start + 1`) first reaches `23`, **never** receiving one addressed with a stage that skips backward; `A` itself receives **no** `SetBlockDestroyStage` packets at all (Context: excludes the digging player). Once `B` has observed stage `9`, `A` sends `PlayerAction{status:2, location: <same pos>, ..}` (`STOP_DESTROY_BLOCK` — Context: the *active* tracking path never auto-finalizes purely from reaching progress `1.0`; only a client's own `STOP` packet, or an already-*delayed* destroy's own `tick()` check, ever finalizes — restated exactly from the research corpus's own `ServerPlayerGameMode.tick()`/`handleBlockBreakAction` split). Both `A` and `B` then receive `BlockUpdate{.., AIR}` + `LevelEvent{.., data:<STONE>}` in the **same** tick as that `STOP` packet's own `AcknowledgeBlockChange` (the `>= 0.7` threshold is already satisfied by then, per Context's own algorithm — `stop_destroy` returns `FinalizeNow` directly, no delayed-destroy queueing needed).
3. M3 field-report correction: this bullet originally specified `raycast_reach_rejects_an_occluded_target_even_within_euclidean_range`, asserting that a target occluded by another solid block was rejected — that assertion described the retired voxel-raycast design (a real strengthening over M2-B07 at the time) and is no longer true. The current test (`crates/server/tests/play_block_break_place_full.rs::distance_based_reach_ignores_occlusion_and_accepts_a_block_behind_another`) asserts the opposite: MECH-D62's real predicate ("Reach validation" above) has no occlusion component at all, so a target directly *behind* another solid block, within the box-distance-plus-buffer threshold, is correctly *accepted* and breaks normally.
4. `placement_selects_the_held_items_own_block_and_orientation` — `A` calls `world.debug_set_held_item(A_id, HeldItemStub::Block(PlaceableBlockKind::Repeater))`, sends `UseItemOn{hand:0, location: pack_position(<a grass-topped, in-range position>), direction:1 /*Up*/, cursor_x:0.5,cursor_y:0.0,cursor_z:0.5, inside_block:false, sequence:2}` while facing a known yaw (the test's own `A` connection is constructed with a fixed test-only yaw/pitch pair injected the same way `world.rs`'s `PlayerMotion` already stores rotation — via a preceding `SetPlayerRotation` packet, M3-B02, sent before this action). `A` reads `AcknowledgeBlockChange{sequence:2}` then `BlockUpdate{location, block_state_id: <tier1_oriented_state_table().lookup(Repeater, Horizontal(<the expected opposite-of-yaw direction>))>}` — proving the full held-item → orientation → raw-id pipeline end-to-end.

### `crates/server/tests/mining_stage4_wiring.rs` (sockets, one case)

`stage4_is_inert_with_no_registered_behavior`: fresh `HardcodedWorld::new()`; assert (via a new debug accessor, `HardcodedWorld::debug_stage4_counters()`, exposing `NeighborUpdateEngine::is_idle()`/`ScheduledTickQueue::block_len()`/`fluid_len()`/`BlockEventQueue::pending_next_tick()` read straight off `region.world`'s own resources) that after ten ordinary ticks with **no** block actions sent at all, every counter is still at its zero/idle default — proving Context's own "inert in the steady state" claim, not merely asserting it in prose. Then perform one ordinary break (as in `play_block_break_place_full.rs` test 1) and assert the same counters return to idle again within the *same* tick the break was processed (the neighbor-update engine settles fully via this blueprint's own `settle_neighbor_updates`, never leaving residual pending work for Stage 4 to discover later).

## Implementation steps

1. **`rc-physics` — `raycast.rs`.** `cast_ray` per Context's exact DDA algorithm. Observable: `raycast_basic.rs` passes.
2. **`rc-physics` — `lib.rs`.** Add the module/re-export line. Observable: `cargo build -p rc-physics` succeeds.
3. **`rusty-clanker-server` — `Cargo.toml`.** Add the `rc-mechanics` line. Observable: `cargo metadata` resolves.
4. **`mining.rs` — pure formula/state-machine pieces** (`dig_properties`, `destroy_speed`, `has_correct_tool_for_drops`, `ticks_to_break`, `begin_destroy`/`stop_destroy`/`abort_destroy`/`tick_destroy_state`). Observable: `mining_dig_timing_golden_table.rs` and `mining_destroy_state_machine.rs` pass.
5. **`mining.rs` — orientation** (`look_vector`, `nearest_horizontal_direction4`, `nearest_direction6`, `resolve_orientation`, `OrientedStateTable`/`tier1_oriented_state_table`). Populate `tier1_oriented_state_table()`'s literal `u32` values as clearly-commented placeholders (Context: pending reconciliation). Observable: `mining_placement_orientation.rs` passes.
6. **`mining.rs` — reach, `finalize_break`, `apply_placement`, `settle_neighbor_updates`.** Observable: compiles against `rc-mechanics`/`rc-physics`.
7. **`block_action.rs`.** Remove `apply_block_action`/`ApplyOutcome`; retype `BlockActionKind` per Deliverables; keep every other item.
8. **`packets.rs`.** Redefine `PlayerAction`/`UseItemOn`, add `SetBlockDestroyStage`/`LevelEvent`.
9. **`world.rs`.** Join-drain additions; Stage-4 wiring (`bootstrap_default_stage4_resources`, `register_stage4`, per-region `RegionOwnership`); the two new tick-loop steps; `debug_set_held_item`/`debug_set_survival`/`debug_stage4_counters`. Observable: `mining_stage4_wiring.rs` passes.
10. **`connection.rs`.** Updated `0x29` arm, new `0x42` arm (replacing the old `0x2A`).
11. **`mod.rs`.** Re-export wiring.
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`.
13. **Reconcile provisional facts.** Per Context's own caveats: (a) confirm `Player Action`'s `direction` wire type and `Use Item On`'s `0x42` id (and every other id in the corrected table) against a real `reports/packets.json` for protocol 776; (b) confirm `tier1_oriented_state_table()`'s literal raw ids against `reports/blocks.json`; (c) confirm Mining Fatigue's level-III/IV multipliers (`0.027`/`0.0081` per this blueprint's own `0.3ⁿ` correction) and the `nearest_direction6` pitch-sign convention against the same source or a black-box capture. Each is a one-line-per-finding edit, re-running step 12 afterward.
14. **Push and confirm CI.**

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** (TEST-D45/D46), exactly as restated in Acceptance tests.

(b) **No new external dependencies.** `rc-physics` gains no dependency (still exactly `{rc-core}`); `rusty-clanker-server` gains exactly one new line, `rc-mechanics` (already workspace-pinned, `server-systems` feature).

(c) **No Mojang or third-party reimplementation code.** Every fact this blueprint restates is sourced from `docs/planning/05-game-mechanics.md`'s own decisions, `docs/research/mc-26.2/11-player-gameplay.md` §3.4, and a live `minecraft.wiki` fetch performed while deriving this blueprint (ASSET-D18(b)/(d)/(f)) — no decompiled source, no other reimplementation's code, was consulted.

(d) **No algorithmic deviation from this blueprint's own pinned formulas.** The dig-timing formula's operation order (base multiplier → Efficiency → Haste → Mining Fatigue → water → airborne) is binding, not illustrative — matches Context's own restatement, which itself matches the live-fetch source's own stated order.

(e) **Scope boundary.** This blueprint does not implement: any real `BlockBehavior` for tier-1 blocks (wire power, repeater delay, comparator reading, torch burnout, piston extend/retract — MECH-D11–D13, a sibling M3 blueprint's own content, registered later into the exact `BlockBehaviorRegistry` this blueprint's own `settle_neighbor_updates` already dispatches through correctly); real `ItemStack`/inventory (`HeldItemStub`/`GameModeState` are explicit, flagged interim stubs, MECH-D47/M4); item entities/real drops (`BreakOutcome::Applied.drop_eligible` is computed but never spawns anything, MECH-D51/M4); gravity-block falling (MECH-D28, not named in M3's own roadmap scope); double-chest merging (kept out of scope from M3-B02); a real gamemode-switch command (`debug_set_survival` is test-only); tool durability loss on use; mid-dig tool/effect re-sampling (Context's own "does not re-sample" note); a placement-time support check for redstone torch/wall torch (only redstone wire gets `NoSolidSupportBelow` — an unsupported torch is accepted at placement time and left to a future sibling redstone blueprint's own `on_neighbor_changed` handler to remove reactively, matching vanilla's own eventual-consistency behavior rather than an immediate rejection); `xtask` codegen changes of any kind (the oriented-state table is hand-authored, explicitly flagged, per Context — no `xtask`/`codegen.rs` file is touched by this blueprint). Do not add placeholder implementations of any of these as a shortcut.

(f) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-physics -p rusty-clanker-server --all-features
cargo nextest run -p rc-physics -p rusty-clanker-server
cargo test --doc -p rc-physics -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-physics` runs `raycast_basic.rs` (5 cases); `cargo nextest run -p rusty-clanker-server` additionally runs `mining_dig_timing_golden_table.rs` (20 table rows + 2 non-tabular cases) + `mining_destroy_state_machine.rs` (8 cases) + `mining_placement_orientation.rs` (8 cases) + `play_block_break_place_full.rs` (4 cases) + `mining_stage4_wiring.rs` (1 case) = 43 new test cases, alongside every pre-existing `rusty-clanker-server`/`rc-physics` test this blueprint does not touch. CI green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Interfaces

**Provides to this milestone's redstone-component blueprint (wire/repeater/comparator/torch/piston behavior, MECH-D11–D13):** `settle_neighbor_updates`'s own dispatch-through-`BlockBehaviorRegistry` design (Context) is written generically and will correctly notify whatever real behaviors that blueprint registers via `BlockBehaviorRegistry::register_range` — no change to this blueprint's own code is needed once that registration happens. This blueprint's `OrientedStateTable`/`resolve_orientation` already select the correct *default* redstone-component states (delay=1, unpowered, unlocked, etc.) at placement time — the redstone blueprint's own behaviors are what make those states *reactive*, not this one's job.

**Provides to this milestone's block-entity-tick blueprint (chest/furnace/hopper tick behavior, ARCH-D17):** real, correctly-oriented `chest`/`furnace`/`blast_furnace`/`smoker`/`hopper` block states are now placeable and breakable, with `UpdateContext::set_block`'s own fan-out already notifying neighbors on placement — that blueprint's own block-entity spawn/tick logic is a separate concern this blueprint does not touch (no `BlockEntity`/block-entity-`Entity` is spawned by this blueprint on placing any of these — a documented, bounded gap: **placing a chest/furnace/hopper via this blueprint creates only the block *state*, not its accompanying block-entity data**, since M2-B01's own `BlockEntityIndex` component exists but this blueprint never populates it — flagged explicitly, not silently, as the natural hand-off point to that sibling blueprint).

**Needs from a future blueprint:** `xtask`'s codegen extended to emit a real per-property state-permutation table (Context: "Raw block-state id resolution"), replacing `tier1_oriented_state_table()`'s own hand-authored placeholder literals with generated, verified ones; `xtask extract-shapes` (M3-B02's own already-flagged deferral) to reconcile this blueprint's own `nearest_direction6` pitch-sign convention and the DDA raycast's fidelity against vanilla's real `BlockGetter.clip`.

## Open questions

- **`tier1_oriented_state_table()`'s literal raw ids are placeholders**, exactly like M3-B02's own `tier1_shape_table()` caveat — real reconciliation needs a `reports/blocks.json` run for protocol 776 (Implementation step 13).
- **Mining Fatigue's level-III/IV multipliers** (`0.027`/`0.0081` per this blueprint's own `0.3ⁿ` correction of both `05`'s stated shape and the raw fetch's own suspect decimal transcription) are flagged moderate confidence, pending a firmer source.
- **`nearest_direction6`'s exact pitch-sign-to-`Up`/`Down` mapping and threshold** is this blueprint's own reasonable inference from this project's already-established pitch convention, not independently black-box-confirmed for the specific case of piston vertical placement — flagged for reconciliation.
- **The DDA raycast (`rc_physics::raycast::cast_ray`) is this blueprint's own reasonable general algorithm, not a byte-exact reproduction of vanilla's `BlockGetter.clip`** — sufficient for reach *validation* (a boolean accept/reject), not claimed sufficient for any future mechanic that needs vanilla's exact hit-point/face semantics (e.g. precise cursor-based stair/slab placement, itself out of this blueprint's own tier-1 scope per Context's own orientation table, which never varies on cursor position).
- **A block placed via this blueprint that should own a block-entity (chest/furnace/hopper) does not yet get one** (Interfaces) — the block-entity-tick blueprint's own first responsibility should include backfilling this for any block placed before it lands, or this blueprint's own placement path should be revisited to spawn an empty block-entity directly; left unresolved here since it depends on that blueprint's own not-yet-written `BlockEntityIndex` population convention.
