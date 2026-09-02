//! The Stage-4 replay driver: drives one `ContraptionSpec` through Rusty Clanker's
//! own Stage-4 core (M3-B01's `stage4::{run_scheduled_phase, run_block_event_subphase}`,
//! unmodified) and produces a `RedstoneTrace` in exactly the schema/order the capture
//! pipeline produces (blueprint Deliverables, `replay.rs`).
//!
//! Placement (`ContraptionSpec::blocks`) and every scripted action are, per the
//! blueprint's own "Tick 0, precisely" Context section, settled *immediately*
//! (ARCH-D13/MECH-D10's same-tick fan-out) rather than deferred to the next Stage-4
//! pass — `stage4.rs`'s own equivalent dispatch loop (`drain_engine`/
//! `dispatch_pending_update`) is a private implementation detail of that module, so
//! `place_and_settle`/`dispatch_one` below are this crate's own necessarily-
//! duplicated re-statement of the identical algorithm, used only for this "outside
//! the tick loop" placement/action-application step (never for the tick loop
//! itself, which calls `stage4::run_scheduled_phase`/`run_block_event_subphase`
//! directly, unmodified).

use std::collections::HashMap;
use std::sync::Arc;

use rc_chunk_storage::{BlockStateId, ItemStackRecord};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::piston::PistonBehavior;
use rc_mechanics::redstone::{
    ComparatorBehavior, ComparatorMode, ContainerSignalSource, RedstoneSignalSource,
    RepeaterBehavior, SignalSourceRegistry, TorchAttachment, TorchBehavior, WireBehavior,
    notify_neighbor_changed_only, register_redstone_block,
};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEntityKind, BlockEntityWorldAccess, BlockEventQueue,
    BlockWorldAccess, BorderHalo, ChestBlockEntity, DefaultMaxStackSize, FuelTable,
    FurnaceBlockEntity, FurnaceLitStateResolver, HopperBlockEntity, NeighborUpdateEngine,
    PendingUpdate, RegionOwnership, ScheduledTickQueue, SmeltingRecipeTable,
    Tier1ContainerSignalSource, TierOneContainer, UpdateContext,
};
use rc_mechanics::{stage4, stage7};
use rc_messaging::{Address, RegionId, RegionMessage};
use rc_registries::block_state_properties::range_of;
use rc_registries::generated_v776::block_state_properties::{BlockId, block_id};

use crate::spec::{ContraptionSpec, bounding_box};
use crate::trace::{BlockObservation, RedstoneTrace, TRACE_FORMAT_VERSION, TickSnapshot};

/// A `HashMap`-backed `BlockWorldAccess` scoped to one contraption — the identical
/// in-memory test-double shape M3-B01's own `stage4_ordering.rs`/`cross_region_
/// border.rs` test files already establish (`FakeWorld`), reused here as this
/// blueprint's own production replay world, not merely a test fixture.
pub struct ReplayWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    dimension: DimensionId,
    local: Address,
}

impl ReplayWorld {
    pub fn new(dimension: DimensionId) -> Self {
        Self {
            blocks: HashMap::new(),
            dimension,
            // A fixed placeholder id — never observed outside this single-region
            // replay (Deliverables, `replay_contraption`'s own doc comment).
            local: Address::Region(RegionId(0)),
        }
    }
}

impl BlockWorldAccess for ReplayWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.blocks.get(&pos).copied()
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let changed = self.blocks.get(&pos) != Some(&state);
        self.blocks.insert(pos, state);
        changed
    }
    fn dimension(&self) -> DimensionId {
        self.dimension
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        self.local
    }
    fn local_identity(&self) -> Address {
        self.local
    }
}

/// A `HashMap`-backed `BlockEntityWorldAccess` scoped to one contraption replay (M3 fix-agent
/// brief, "bring the three container fixtures into the replay") — mirrors `ReplayWorld`'s own
/// "production replay world, not merely a test fixture" role, reusing `crates/mechanics/tests/
/// container_signal_source_wiring.rs`'s own `FakeContainerWorld` test-double shape as this
/// crate's own driver behind the tick loop rather than a test double. No furnace ever appears in
/// this corpus's own committed fixtures — `get_furnace_mut` always answers `None`, matching a
/// `world` with zero furnace block entities, never a special-cased gap.
///
/// `load_order` tracks each entity's own first-seeded position explicitly (a plain `Vec`, never
/// `HashMap` key iteration, which this project's own determinism requirements forbid relying on)
/// — `block_entities_in_chunk`'s one ordering guarantee that is itself vanilla-observable
/// (`BlockEntityIndex`'s own stored load order, `block_entity/mod.rs`'s own doc comment).
struct ReplayBlockEntityWorld {
    chests: HashMap<BlockPos, ChestBlockEntity>,
    hoppers: HashMap<BlockPos, HopperBlockEntity>,
    load_order: Vec<(BlockPos, BlockEntityKind)>,
    dimension: DimensionId,
}

impl ReplayBlockEntityWorld {
    fn new(dimension: DimensionId) -> Self {
        Self {
            chests: HashMap::new(),
            hoppers: HashMap::new(),
            load_order: Vec::new(),
            dimension,
        }
    }

    fn chunk_of(&self, pos: BlockPos) -> ChunkKey {
        pos.chunk_key(self.dimension)
    }

    /// Records `pos`'s own first appearance only — a later re-seed at an already-tracked
    /// position (a fixture that re-`/setblock`s the same chest/hopper via `actions:`) updates
    /// the stored entity's own content without moving its place in `load_order`, mirroring real
    /// vanilla's own "block entity stays at its original chunk-index slot across a data merge."
    fn note_load_order(&mut self, pos: BlockPos, kind: BlockEntityKind) {
        if !self.load_order.iter().any(|&(p, _)| p == pos) {
            self.load_order.push((pos, kind));
        }
    }

    fn insert_chest(&mut self, pos: BlockPos, chest: ChestBlockEntity) {
        self.note_load_order(pos, BlockEntityKind::Chest);
        self.chests.insert(pos, chest);
    }

    fn insert_hopper(&mut self, pos: BlockPos, hopper: HopperBlockEntity) {
        self.note_load_order(pos, BlockEntityKind::Hopper);
        self.hoppers.insert(pos, hopper);
    }
}

impl BlockEntityWorldAccess for ReplayBlockEntityWorld {
    fn region_chunks(&self) -> Vec<ChunkKey> {
        let mut keys: Vec<ChunkKey> = self
            .load_order
            .iter()
            .map(|&(pos, _)| self.chunk_of(pos))
            .collect();
        keys.sort_unstable_by_key(|k| (k.x, k.z));
        keys.dedup();
        keys
    }

    fn block_entities_in_chunk(&self, chunk: ChunkKey) -> Vec<(BlockPos, BlockEntityKind)> {
        self.load_order
            .iter()
            .copied()
            .filter(|&(pos, _)| self.chunk_of(pos) == chunk)
            .collect()
    }

    fn container_at_mut(&mut self, pos: BlockPos) -> Option<&mut dyn TierOneContainer> {
        if let Some(h) = self.hoppers.get_mut(&pos) {
            return Some(h);
        }
        if let Some(c) = self.chests.get_mut(&pos) {
            return Some(c);
        }
        None
    }

    fn get_hopper_mut(&mut self, pos: BlockPos) -> Option<&mut HopperBlockEntity> {
        self.hoppers.get_mut(&pos)
    }
    fn get_furnace_mut(&mut self, _pos: BlockPos) -> Option<&mut FurnaceBlockEntity> {
        None
    }
    fn get_chest_mut(&mut self, pos: BlockPos) -> Option<&mut ChestBlockEntity> {
        self.chests.get_mut(&pos)
    }
    /// Not implemented by the real production adapter either (`stage7::ecs::EcsBlockEntityWorld`'s
    /// own identical, already-documented gap: "no comparator/wire/redstone-signal-strength query
    /// exists ... outside Stage 4's own internal state") — mirrored here for the same reason,
    /// never a replay-only shortcut.
    fn is_locked_by_redstone(&self, _pos: BlockPos) -> bool {
        false
    }
    /// A no-op, exactly as the production adapter's own identical method — this corpus never
    /// places a furnace, so no lit-state swap is ever needed.
    fn swap_furnace_lit_state(
        &mut self,
        _pos: BlockPos,
        _now_lit: bool,
        _resolver: Option<&dyn FurnaceLitStateResolver>,
    ) {
    }
}

/// Drives `spec` through Rusty Clanker's own Stage-4 core for exactly
/// `spec.max_ticks` ticks, against a single-region `RegionOwnership::always_local`
/// (this contraption never spans a region), producing a `RedstoneTrace` in exactly
/// the same schema/order the capture pipeline produces.
pub fn replay_contraption(
    spec: &ContraptionSpec,
    behaviors: &BlockBehaviorRegistry,
    container_signals: &Tier1ContainerSignalSource,
    analog_reader: Option<&dyn Fn(BlockPos) -> Option<u8>>,
) -> RedstoneTrace {
    let mut world = ReplayWorld::new(DimensionId::OVERWORLD);
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    // A single-region replay receives no inbound border events — never populated
    // (Deliverables doc comment).
    let mut halo = BorderHalo::default();
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
    // Always empty at return — asserted below (a non-empty `outbound` after any
    // step is a hard bug, since a single, `always_local`-owned region can never
    // route a message cross-region, Deliverables doc comment).
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    // M3 field-report fix ("block-state changes made outside a direct player action never
    // reach any client"): the mechanical collector `UpdateContext` now always carries —
    // this replay harness only supplies it (every construction site below), it never reads
    // it back, matching this changeset's own "the replay harness/corpus stay byte-identical"
    // requirement (the recorded `RedstoneTrace` output never reflects `changed` at all).
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();

    // M3 fix-agent brief ("bring the three container fixtures into the replay"): the Stage-7
    // block-entity world this replay now drives alongside Stage 4, plus the minimal fixed tables
    // `run_block_entity_tick` requires (`DefaultMaxStackSize`/`SmeltingRecipeTable::minimal_
    // tier1`/`FuelTable::minimal_tier1` — the identical minimal tables `container_signal_source_
    // wiring.rs`'s own acceptance suite already uses, M3-B06's established harness pattern).
    let mut block_entities = ReplayBlockEntityWorld::new(DimensionId::OVERWORLD);
    let recipes = SmeltingRecipeTable::minimal_tier1();
    let fuels = FuelTable::minimal_tier1();
    let max_stack = DefaultMaxStackSize;

    let (bounds_min, bounds_max) = bounding_box(spec);

    // Step 2: place every `spec.blocks` entry in list order, each immediately
    // settled (module doc comment) at `current_tick: 0`.
    for block in &spec.blocks {
        let pos = BlockPos::new(block.pos.0, block.pos.1, block.pos.2);
        let state = BlockStateId(block.state_id);
        place_and_settle(
            &mut world,
            &mut engine,
            &mut scheduled,
            &mut events,
            &mut outbound,
            &mut changed,
            &ownership,
            0,
            behaviors,
            pos,
            state,
        );
        seed_container_if_present(&mut block_entities, pos, state, &block.vanilla_state);
    }

    let mut ticks = Vec::with_capacity(spec.max_ticks as usize + 1);
    ticks.push(TickSnapshot {
        tick: 0,
        blocks: snapshot_volume(&world, bounds_min, bounds_max, analog_reader),
    });

    for t in 1..=spec.max_ticks as u64 {
        for action in spec.actions.iter().filter(|a| a.tick == t) {
            let pos = BlockPos::new(action.pos.0, action.pos.1, action.pos.2);
            let state = BlockStateId(action.state_id);
            // M3 field-report governance fix (docs/findings-for-planning.md, the replay.rs
            // `current_tick` off-by-one entry): a scripted action is applied *before* this
            // tick's own advance (`ScriptedAction::tick`'s contract — "at the start of this
            // tick, before that tick's Stage-4 pass"), exactly like the capture pipeline's
            // own frozen-time `/setblock`, whose synchronous cascade sees the game-time
            // counter as it stood before the `tick step` that produces this snapshot. Its
            // `current_tick` is therefore `t - 1`, so a diode's `schedule_block_tick(delay)`
            // lands at `t - 1 + delay` — the oracle-verified schedule (repeater chain
            // 2/4/6/8 and both comparator-subtract fixtures match it hop-for-hop, while
            // every cascade fired from *inside* `run_scheduled_phase`'s own draining
            // correctly keeps the real `t`). Initial `blocks:` placement (tick-0 snapshot)
            // already uses `0` under the same convention.
            place_and_settle(
                &mut world,
                &mut engine,
                &mut scheduled,
                &mut events,
                &mut outbound,
                &mut changed,
                &ownership,
                t - 1,
                behaviors,
                pos,
                state,
            );
            seed_container_if_present(&mut block_entities, pos, state, &action.vanilla_state);
        }

        stage4::run_scheduled_phase(
            &mut world,
            &[],
            &mut halo,
            &ownership,
            &mut engine,
            &mut scheduled,
            &mut events,
            behaviors,
            &mut outbound,
            &mut changed,
            t,
        );
        stage4::run_block_event_subphase(
            &mut world,
            &ownership,
            &mut engine,
            &mut scheduled,
            &mut events,
            behaviors,
            &mut outbound,
            &mut changed,
            t,
        );

        // Stage 7 (M3-B06): block-entity ticking, strictly *after* Stage 4 within this same
        // tick — `DISPATCH_ORDER` (`crates/scheduler/src/executor.rs`) fixes `BlockRedstone`
        // (Stage 4) ahead of `BlockEntity` (Stage 7) precisely so scheduled-tick/block-event
        // redstone dispatch never misses that same tick's own block-entity state; this replay's
        // hand-driven tick loop reproduces that ordering by calling Stage 7 here, immediately
        // after both Stage-4 sub-phases above.
        stage7::run_block_entity_tick(
            &mut block_entities,
            &recipes,
            &fuels,
            &max_stack,
            None,
            container_signals,
        );

        // Section C (M3 fix-agent brief, "Stage7->Stage4 container notify"): rc-mechanics has
        // no production wiring yet for vanilla's own `BlockEntity.setChanged ->
        // updateNeighbourForOutputSignal` push (a real architectural gap, recorded in
        // docs/findings-for-planning.md rather than fixed at the production ECS level here --
        // Stage 7's own system has no access to `NeighborUpdateEngine`/`BlockWorldAccess` at
        // all, `crates/mechanics/src/stage7/ecs.rs`). This replay driver supplies the minimal
        // parity-faithful equivalent directly: every position `container_signals.take_changed()`
        // reports this tick gets a `notify_neighbor_changed_only` call (its own one-hop
        // conductor relay already covers both "a comparator reads straight off the container"
        // and "a comparator reads through a conductor the container also touches" identically),
        // then the resulting `NeighborChanged`/`ShapeUpdate` cascade is drained to a fixed point
        // exactly like every other trigger this loop settles.
        for pos in container_signals.take_changed() {
            let mut ctx = UpdateContext {
                world: &mut world,
                engine: &mut engine,
                scheduled: &mut scheduled,
                events: &mut events,
                outbound: &mut outbound,
                changed: &mut changed,
                ownership: &ownership,
                current_tick: t,
            };
            notify_neighbor_changed_only(&mut ctx, pos);
        }
        engine.drain(&mut |eng, item| {
            let mut ctx = UpdateContext {
                world: &mut world,
                engine: eng,
                scheduled: &mut scheduled,
                events: &mut events,
                outbound: &mut outbound,
                changed: &mut changed,
                ownership: &ownership,
                current_tick: t,
            };
            dispatch_one(&mut ctx, behaviors, item);
        });

        ticks.push(TickSnapshot {
            tick: t,
            blocks: snapshot_volume(&world, bounds_min, bounds_max, analog_reader),
        });
    }

    assert!(
        outbound.is_empty(),
        "replay_contraption: a single always_local region must never produce an outbound cross-region message, got {} entries",
        outbound.len()
    );

    RedstoneTrace {
        format_version: TRACE_FORMAT_VERSION,
        contraption_id: spec.id.clone(),
        // Replay has no jar provenance — only a captured trace's `source_jar_sha1`
        // is meaningful (Deliverables doc comment).
        source_jar_sha1: String::new(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        bounds_min,
        bounds_max,
        ticks,
    }
}

/// Reads every position in `[bounds_min, bounds_max]` from `world` (plus
/// `analog_reader`, if supplied, at every position), sorted per `TickSnapshot::
/// blocks`'s own documented `(y, z, x)` ascending order — the nested loop order
/// below (`y` outer, `z` middle, `x` inner) already produces exactly that order, so
/// no separate sort step is needed.
fn snapshot_volume(
    world: &dyn BlockWorldAccess,
    bounds_min: (i32, i32, i32),
    bounds_max: (i32, i32, i32),
    analog_reader: Option<&dyn Fn(BlockPos) -> Option<u8>>,
) -> Vec<BlockObservation> {
    let mut out = Vec::new();
    for y in bounds_min.1..=bounds_max.1 {
        for z in bounds_min.2..=bounds_max.2 {
            for x in bounds_min.0..=bounds_max.0 {
                let pos = BlockPos::new(x, y, z);
                // `BlockStateId(0)` is vanilla's own air default (M0-B07's
                // `block_states.rs` codegen) — an untouched position reads as air
                // exactly as it should (Implementation step 5).
                let state = world.get_block(pos).unwrap_or(BlockStateId(0));
                let analog = analog_reader.and_then(|read| read(pos));
                out.push(BlockObservation {
                    pos: (x, y, z),
                    state_id: state.0,
                    analog,
                });
            }
        }
    }
    out
}

/// The `[min, max_inclusive]` of every real reachable state id for each tier-1 block --
/// M3.5-B02 (WS-D15): every one of the eight former hand-authored `*_RANGE` constants this
/// section used to declare is retired against `rc-registries`' M3.5-B01-generated
/// per-block-state-property registry (`range_of`) instead; `in_range`/`exclusive` below now
/// take a `BlockId` directly and read the real range off that registry on every call, so there
/// is no longer a second, independently hand-derived range anywhere in this file to drift out
/// of sync with the generated source (the M3 field-report widening this doc comment used to
/// describe -- "narrowest window covering this corpus's own placements" versus "every id
/// blocks.json actually declares" -- is moot now that the range itself always comes from the
/// generated table). `minecraft:chest`/`minecraft:hopper` are not redstone tier-1 components
/// (no `BlockBehavior`/`RedstoneSignalSource` registration below) -- their own ranges are used
/// only to recognize a placement for Stage-7 block-entity seeding (`seed_container_if_present`).
fn in_range(id: u32, block: BlockId) -> bool {
    let range = range_of(block);
    id >= range.first.0 && id <= range.last.0
}

/// The `[start, end_exclusive)` `register_range` needs, from `block`'s own real generated
/// range (M3.5-B02, WS-D15).
fn exclusive(block: BlockId) -> (BlockStateId, BlockStateId) {
    let range = range_of(block);
    (BlockStateId(range.first.0), BlockStateId(range.last.0 + 1))
}

/// Extracts one `key=value` property out of a `PlacedBlock::vanilla_state`'s own bracket
/// syntax (e.g. `"minecraft:repeater[facing=east,delay=1,locked=false,powered=false]"`) — the
/// same legal `/setblock` grammar `spec.rs`'s own doc comment already describes this field as
/// carrying verbatim.
fn vanilla_property<'a>(vanilla_state: &'a str, key: &str) -> Option<&'a str> {
    let start = vanilla_state.find('[')?;
    let end = vanilla_state.rfind(']')?;
    vanilla_state[start + 1..end].split(',').find_map(|entry| {
        let (k, v) = entry.split_once('=')?;
        (k == key).then_some(v)
    })
}

fn facing_property(vanilla_state: &str) -> Direction {
    match vanilla_property(vanilla_state, "facing") {
        Some("north") => Direction::North,
        Some("south") => Direction::South,
        Some("east") => Direction::East,
        Some("west") => Direction::West,
        Some("up") => Direction::Up,
        Some("down") => Direction::Down,
        other => {
            panic!("tier1_registry: unrecognized/missing facing in {vanilla_state:?}: {other:?}")
        }
    }
}

/// One entry from a fixture's own inline `Items:[...]` block-entity NBT list (M3 fix-agent
/// brief, step 2: "Container contents") — the exact minimal subset this corpus's own committed
/// fixtures use: `Slot: Byte`, `id: String`, `Count: Byte`. Nothing else.
struct SeedItem {
    slot: u8,
    id: String,
    count: i32,
}

/// Seeds a fresh chest/hopper block entity at `pos` from `vanilla_state`'s own trailing `{...}`
/// block-entity NBT (M3 fix-agent brief, step 2) the instant `place_and_settle` places a
/// chest/hopper-ranged state — mirrors real placement, where a single `/setblock ...
/// {Items:[...]}}` command creates the block entity with that exact content already loaded. A
/// no-op for every other state id (every non-container block this corpus's own fixtures place).
fn seed_container_if_present(
    block_entities: &mut ReplayBlockEntityWorld,
    pos: BlockPos,
    state: BlockStateId,
    vanilla_state: &str,
) {
    if in_range(state.0, block_id::CHEST) {
        let mut chest = ChestBlockEntity::empty();
        for item in seed_items(vanilla_state) {
            place_seed_item(&mut chest.slots, item, vanilla_state);
        }
        block_entities.insert_chest(pos, chest);
    } else if in_range(state.0, block_id::HOPPER) {
        let facing = facing_property(vanilla_state);
        let mut hopper = HopperBlockEntity::empty(facing);
        for item in seed_items(vanilla_state) {
            place_seed_item(&mut hopper.slots, item, vanilla_state);
        }
        block_entities.insert_hopper(pos, hopper);
    }
}

fn place_seed_item(slots: &mut [Option<ItemStackRecord>], item: SeedItem, vanilla_state: &str) {
    let slot = item.slot as usize;
    assert!(
        slot < slots.len(),
        "seed_container_if_present: Slot {slot} out of range (container has {} slots) in {vanilla_state:?}",
        slots.len()
    );
    slots[slot] = Some(ItemStackRecord {
        id: item.id,
        count: item.count,
        components: None,
    });
}

/// Parses `vanilla_state`'s own trailing block-entity NBT compound (the `{...}` suffix after any
/// `[...]` blockstate-property bracket — legal `/setblock` grammar, `spec.rs`'s own doc comment)
/// into the `Items` list it carries, or an empty `Vec` if `vanilla_state` carries no `{...}`
/// suffix at all (an empty container — every non-hopper/chest fixture block, and any hopper/
/// chest this corpus ever places with nothing inside). Reject-loudly (`panic!`) on anything
/// outside the exact minimal subset this corpus's own committed fixtures actually use — a bare
/// `{Items:[{Slot:<n>b,id:"<id>",Count:<n>b}, ...]}` compound, nothing else (no `CustomName`/
/// `Lock`/`TransferCooldown`/item `components` tags) — never silently ignored (M3 fix-agent
/// brief, step 2's own explicit instruction).
fn seed_items(vanilla_state: &str) -> Vec<SeedItem> {
    let Some(brace_start) = vanilla_state.find('{') else {
        return Vec::new();
    };
    let nbt = &vanilla_state[brace_start..];
    assert!(
        nbt.starts_with('{') && nbt.ends_with('}'),
        "seed_items: malformed block-entity NBT suffix in {vanilla_state:?}"
    );
    let inner = &nbt[1..nbt.len() - 1];
    let list_inner = inner
        .strip_prefix("Items:[")
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or_else(|| {
            panic!(
                "seed_items: only a bare `Items:[...]` block-entity NBT compound is supported, \
                 got {inner:?} (vanilla_state {vanilla_state:?})"
            )
        });
    if list_inner.trim().is_empty() {
        return Vec::new();
    }
    split_top_level_compounds(list_inner)
        .into_iter()
        .map(|entry| parse_item_entry(entry, vanilla_state))
        .collect()
}

/// Splits `s` into its top-level `{...}` entries (depth-tracked — none of `Slot`/`id`/`Count`'s
/// own values ever nest a compound in this corpus's own committed subset, but the split is
/// depth-aware regardless rather than a naive comma-split, so a malformed/unsupported nested
/// value is at least captured whole for `parse_item_entry`'s own field-by-field rejection to
/// report clearly instead of being silently mis-split).
fn split_top_level_compounds(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = None;
    for (i, c) in s.char_indices() {
        match c {
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    out.push(&s[start.expect("split_top_level_compounds: unbalanced braces")..=i]);
                }
            }
            _ => {}
        }
    }
    out
}

fn parse_item_entry(entry: &str, vanilla_state: &str) -> SeedItem {
    let inner = entry
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or_else(|| {
            panic!("seed_items: malformed item entry {entry:?} in {vanilla_state:?}")
        });
    let mut slot = None;
    let mut id = None;
    let mut count = None;
    for field in inner.split(',') {
        let (key, value) = field.split_once(':').unwrap_or_else(|| {
            panic!("seed_items: malformed item field {field:?} in {vanilla_state:?}")
        });
        match key {
            "Slot" => slot = Some(parse_nbt_byte(value, vanilla_state)),
            "id" => id = Some(parse_nbt_quoted_string(value, vanilla_state)),
            "Count" => count = Some(parse_nbt_byte(value, vanilla_state) as i32),
            other => panic!(
                "seed_items: unsupported item NBT key {other:?} in {entry:?} (vanilla_state \
                 {vanilla_state:?}) — only Slot/id/Count are supported"
            ),
        }
    }
    SeedItem {
        slot: slot.unwrap_or_else(|| panic!("seed_items: item entry missing Slot in {entry:?}")),
        id: id.unwrap_or_else(|| panic!("seed_items: item entry missing id in {entry:?}")),
        count: count.unwrap_or_else(|| panic!("seed_items: item entry missing Count in {entry:?}")),
    }
}

fn parse_nbt_byte(value: &str, vanilla_state: &str) -> u8 {
    value
        .strip_suffix(['b', 'B'])
        .unwrap_or(value)
        .parse()
        .unwrap_or_else(|err| {
            panic!("seed_items: not a byte: {value:?} in {vanilla_state:?}: {err}")
        })
}

fn parse_nbt_quoted_string(value: &str, vanilla_state: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or_else(|| {
            panic!("seed_items: expected a quoted string, got {value:?} in {vanilla_state:?}")
        })
        .to_string()
}

/// Governance fix (M3 field-report): wires the same production composition — M3-B04's four
/// tier-1 components (wire, torch-floor, torch-wall, repeater, comparator) followed by M3-B05's
/// piston, piston strictly after the four components have fully populated `SignalSourceRegistry`
/// (`register_piston`'s own doc comment) — into this replay path, which until now left every
/// position resolving to the shared `NoOpBehavior` default (an honest, documented M3-B07
/// placeholder, "until the component blueprints land"; they all have).
///
/// Deliberately does **not** call `register_tier1_redstone`/`register_piston` themselves
/// (`registration.rs`/`piston.rs`): both wrap their constructed instances in an opaque handle
/// with no getter (`Tier1RedstoneHandles`'s own doc comment — "carries no public field or
/// getter"), but `RepeaterBehavior::place`/`ComparatorBehavior::place` (each block's own facing,
/// plus delay/mode) require `&mut self` access **before** the instance is ever shared behind an
/// `Arc` — a real fixture's repeater/comparator facing can only be recovered from its own
/// `vanilla_state` property string (still no generated per-property registry, same gap as
/// above), which only this spec-aware caller has. So this function reproduces
/// `register_tier1_redstone`+`Tier1RedstoneHandles::bind_registry`+`register_piston`'s own
/// exact construction/registration/bind order by hand, inserting the one additional seeding
/// step each of those two components needs, immediately before it is wrapped in its own `Arc`.
///
/// Seeding scans `spec.blocks` only (never `spec.actions`) — every repeater/comparator/piston
/// this corpus's own fixtures ever place first appears in `blocks:` (verified against every
/// committed `.ron` fixture); a handful of comparator fixtures *re-place* the same position
/// with a different facing/mode later via `actions:` (`comparator_facing_probe_all_four`'s own
/// four-facing rotation; `comparator_compare_vs_subtract`/`comparator_tie_no_turn_on`'s own
/// mid-run mode swap) — `ComparatorBehavior` exposes no way to update facing after construction
/// at all, and mode only via `set_mode`, which this generic, redstone-behavior-agnostic replay
/// driver deliberately never special-cases, so those three fixtures keep showing a real
/// mismatch from their own re-placement tick onward (an accepted, reported gap, not a bug in
/// this wiring — do not "fix" it here).
///
/// Also constructs the region's own `Tier1ContainerSignalSource` (M3 fix-agent brief, "bring
/// the three container fixtures into the replay") and wires it into the comparator exactly as
/// the real composition root would (`ComparatorBehavior::new`'s own read side) — returned
/// alongside the registry so the caller can hand the identical `Arc` to `replay_contraption`,
/// whose own Stage-7 pass is this same instance's write side (`Tier1ContainerSignalSource`'s own
/// doc comment: "two independent `Arc` clones... shared with both `ComparatorBehavior::new`...
/// and Stage 7's own driver").
pub fn tier1_registry(
    spec: &ContraptionSpec,
) -> (BlockBehaviorRegistry, Arc<Tier1ContainerSignalSource>) {
    let mut behaviors = BlockBehaviorRegistry::new();
    let mut signals = SignalSourceRegistry::new();
    let container_signals = Arc::new(Tier1ContainerSignalSource::new());

    // `minecraft:redstone_block` (M3 field-report fix, Task 1): a stateless always-on source,
    // no `BlockBehavior`/registry-self-reference concerns (`register_redstone_block`'s own doc
    // comment) — registered directly via the production function, unlike the four tier-1
    // components below (which this replay driver must hand-reconstruct for their own
    // pre-`Arc` placement-seeding needs, module doc comment).
    register_redstone_block(&mut signals);

    let wire = Arc::new(WireBehavior::new());
    let (lo, hi) = exclusive(block_id::REDSTONE_WIRE);
    behaviors.register_range(lo, hi, Arc::clone(&wire) as Arc<dyn BlockBehavior>);
    signals.register_range(lo, hi, Arc::clone(&wire) as Arc<dyn RedstoneSignalSource>);

    let torch_floor = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    let (lo, hi) = exclusive(block_id::REDSTONE_TORCH);
    behaviors.register_range(lo, hi, Arc::clone(&torch_floor) as Arc<dyn BlockBehavior>);
    signals.register_range(
        lo,
        hi,
        Arc::clone(&torch_floor) as Arc<dyn RedstoneSignalSource>,
    );

    // One representative `Wall(North)` orientation for the whole range (`registration.rs`'s
    // own identical, already-documented M3 scope limitation) — a wall torch actually facing a
    // different direction in a fixture dispatches with the wrong `input_direction` here, same
    // as it would through the real composition root today.
    let torch_wall = Arc::new(TorchBehavior::new(TorchAttachment::Wall(Direction::North)));
    let (lo, hi) = exclusive(block_id::REDSTONE_WALL_TORCH);
    behaviors.register_range(lo, hi, Arc::clone(&torch_wall) as Arc<dyn BlockBehavior>);
    signals.register_range(
        lo,
        hi,
        Arc::clone(&torch_wall) as Arc<dyn RedstoneSignalSource>,
    );

    let repeater = RepeaterBehavior::new();
    for block in &spec.blocks {
        if in_range(block.state_id, block_id::REPEATER) {
            let pos = BlockPos::new(block.pos.0, block.pos.1, block.pos.2);
            let facing = facing_property(&block.vanilla_state);
            let delay: u8 = vanilla_property(&block.vanilla_state, "delay")
                .unwrap_or_else(|| {
                    panic!(
                        "tier1_registry: repeater with no delay property: {}",
                        block.vanilla_state
                    )
                })
                .parse()
                .expect("tier1_registry: repeater delay property must be a small integer");
            repeater.place(pos, facing, delay);
        }
    }
    let repeater = Arc::new(repeater);
    let (lo, hi) = exclusive(block_id::REPEATER);
    behaviors.register_range(lo, hi, Arc::clone(&repeater) as Arc<dyn BlockBehavior>);
    signals.register_range(
        lo,
        hi,
        Arc::clone(&repeater) as Arc<dyn RedstoneSignalSource>,
    );

    // M3 fix-agent brief ("bring the three container fixtures into the replay"): the real
    // `Tier1ContainerSignalSource` this same function returns, replacing the former
    // `Arc::new(NoContainers)` placeholder now that `replay_contraption`'s own Stage-7 pass
    // (module doc comment) actually populates it every tick.
    let comparator =
        ComparatorBehavior::new(Arc::clone(&container_signals) as Arc<dyn ContainerSignalSource>);
    for block in &spec.blocks {
        if in_range(block.state_id, block_id::COMPARATOR) {
            let pos = BlockPos::new(block.pos.0, block.pos.1, block.pos.2);
            let facing = facing_property(&block.vanilla_state);
            let mode = match vanilla_property(&block.vanilla_state, "mode") {
                Some("compare") => ComparatorMode::Compare,
                Some("subtract") => ComparatorMode::Subtract,
                other => panic!(
                    "tier1_registry: unrecognized/missing mode in {:?}: {other:?}",
                    block.vanilla_state
                ),
            };
            comparator.place(pos, facing, mode);
        }
    }
    let comparator = Arc::new(comparator);
    let (lo, hi) = exclusive(block_id::COMPARATOR);
    behaviors.register_range(lo, hi, Arc::clone(&comparator) as Arc<dyn BlockBehavior>);
    signals.register_range(
        lo,
        hi,
        Arc::clone(&comparator) as Arc<dyn RedstoneSignalSource>,
    );

    // Two-phase registry self-reference (Context §I½, `Tier1RedstoneHandles::bind_registry`'s
    // own identical order: wire, torch-floor, torch-wall, repeater, comparator).
    let signals = Arc::new(signals);
    wire.bind_registry(Arc::clone(&signals));
    torch_floor.bind_registry(Arc::clone(&signals));
    torch_wall.bind_registry(Arc::clone(&signals));
    repeater.bind_registry(Arc::clone(&signals));
    comparator.bind_registry(Arc::clone(&signals));

    // Piston strictly after the four components (`register_piston`'s own doc comment).
    let piston = Arc::new(PistonBehavior::new(signals));
    for block in &spec.blocks {
        if in_range(block.state_id, block_id::PISTON)
            || in_range(block.state_id, block_id::STICKY_PISTON)
        {
            let pos = BlockPos::new(block.pos.0, block.pos.1, block.pos.2);
            let facing = facing_property(&block.vanilla_state);
            let sticky = block.vanilla_state.starts_with("minecraft:sticky_piston");
            // M3 field-report fix (phantom-extend-on-already-extended-placement defect):
            // `spec.blocks` settles in list order (this function's own module doc comment),
            // so a raw `blocks:` entry placing a piston already `extended=true` must seed
            // `PistonBehavior` with that same real property -- a later-listed signal source
            // (e.g. a `redstone_block`) fanning `on_neighbor_changed` to this already-placed
            // piston must see a state that already matches its own signal, not a spurious
            // `false -> true` transition (`piston.rs`'s own `place` doc comment has the full
            // citation).
            let extended = match vanilla_property(&block.vanilla_state, "extended") {
                Some("true") => true,
                Some("false") => false,
                other => panic!(
                    "tier1_registry: unrecognized/missing extended in {:?}: {other:?}",
                    block.vanilla_state
                ),
            };
            piston.place(pos, facing, sticky, extended);
        }
    }
    let (lo, hi) = exclusive(block_id::PISTON);
    behaviors.register_range(lo, hi, Arc::clone(&piston) as Arc<dyn BlockBehavior>);
    let (lo, hi) = exclusive(block_id::STICKY_PISTON);
    behaviors.register_range(lo, hi, Arc::clone(&piston) as Arc<dyn BlockBehavior>);

    (behaviors, container_signals)
}

/// Immediate-settle: writes `new_state` at `pos` (fanning out both signals, per
/// `UpdateContext::set_block`), then drains the resulting `NeighborUpdateEngine`
/// queue to a fixed point via `dispatch_one` — module doc comment.
#[allow(clippy::too_many_arguments)]
fn place_and_settle(
    world: &mut ReplayWorld,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    changed: &mut Vec<(BlockPos, BlockStateId)>,
    ownership: &RegionOwnership,
    current_tick: u64,
    behaviors: &BlockBehaviorRegistry,
    pos: BlockPos,
    state: BlockStateId,
) {
    {
        let mut ctx = UpdateContext {
            world,
            engine,
            scheduled,
            events,
            outbound,
            changed,
            ownership,
            current_tick,
        };
        ctx.set_block(pos, state);
        // M3 field-report fix (Task 2): every `place_and_settle` call is a real placement (both
        // `spec.blocks`' own initial setup and a later scripted `spec.actions` re-placement at
        // an already-occupied position) — `on_placed` lets a diode (or any future behavior with
        // its own placement-state side-table) reseed itself straight off the id just written,
        // closing the gap `RepeaterBehavior::place`/`ComparatorBehavior::place`'s own former
        // `&mut self`-only signature left (`docs/findings-for-planning.md`'s own "diode
        // re-placement" entry) without this generic, redstone-behavior-agnostic driver needing
        // to know which concrete behavior type it just placed. Called before `engine.drain`
        // below so every dispatch this same placement triggers already sees the reseeded state.
        behaviors.resolve(state).on_placed(&mut ctx, pos);
    }
    engine.drain(&mut |eng, item| {
        let mut ctx = UpdateContext {
            world,
            engine: eng,
            scheduled,
            events,
            outbound,
            changed,
            ownership,
            current_tick,
        };
        dispatch_one(&mut ctx, behaviors, item);
    });
}

/// One popped `PendingUpdate`'s dispatch — module doc comment (a necessary
/// restatement of `stage4.rs`'s own private `dispatch_pending_update`, which this
/// crate cannot call directly, including its identical `ShapeUpdate` handling: a
/// state-change request is written directly via `ctx.world.set_block` (never
/// `ctx.set_block`, which would restart a brand-new fan-out from this position), then
/// the cascade continues one hop further if depth remains).
fn dispatch_one(ctx: &mut UpdateContext, behaviors: &BlockBehaviorRegistry, item: PendingUpdate) {
    match item {
        PendingUpdate::NeighborChanged { pos, from } => {
            if let Some(state) = ctx.get_block(pos) {
                let behavior = behaviors.resolve(state);
                behavior.on_neighbor_changed(ctx, pos, from);
            }
        }
        PendingUpdate::ShapeUpdate {
            pos,
            from,
            remaining_depth,
        } => {
            let Some(state) = ctx.get_block(pos) else {
                return;
            };
            let Some(neighbor_state) = ctx.get_block(from.apply(pos)) else {
                return;
            };
            let behavior = behaviors.resolve(state);
            if let Some(new_state) = behavior.on_shape_update(ctx, pos, from, neighbor_state) {
                ctx.write_block_state(pos, new_state);
                if remaining_depth > 0 {
                    ctx.engine
                        .emit_shape_update_fanout_at_depth(pos, remaining_depth - 1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M3.5-B02 (WS-D15) governance changeset: pins the real generated range for every one of
    /// this module's own seven former hand-authored diode/wire/piston `*_RANGE` constants
    /// (retired against `rc-registries`' M3.5-B01-generated per-block-state-property registry
    /// earlier in this same governance changeset, `in_range`/`exclusive`'s own doc comment) --
    /// the exact literal values those constants used to declare, restated here since the
    /// constants themselves no longer exist to compare against.
    #[test]
    fn wire_torch_repeater_comparator_piston_ranges_match_generated_ranges() {
        let wire = range_of(block_id::REDSTONE_WIRE);
        assert_eq!((wire.first.0, wire.last.0), (4011, 5306));
        let torch_floor = range_of(block_id::REDSTONE_TORCH);
        assert_eq!((torch_floor.first.0, torch_floor.last.0), (6885, 6886));
        let torch_wall = range_of(block_id::REDSTONE_WALL_TORCH);
        assert_eq!((torch_wall.first.0, torch_wall.last.0), (6887, 6894));
        let repeater = range_of(block_id::REPEATER);
        assert_eq!((repeater.first.0, repeater.last.0), (7034, 7097));
        let comparator = range_of(block_id::COMPARATOR);
        assert_eq!((comparator.first.0, comparator.last.0), (11263, 11278));
        let piston = range_of(block_id::PISTON);
        assert_eq!((piston.first.0, piston.last.0), (2257, 2268));
        let sticky_piston = range_of(block_id::STICKY_PISTON);
        assert_eq!((sticky_piston.first.0, sticky_piston.last.0), (2235, 2246));
    }

    /// Pins that the generated range agrees with `replay.rs`'s own former `CHEST_RANGE`
    /// literal, not with `mining.rs`'s former default-anchored arithmetic (§3.7 of
    /// `blueprints/M3.5/M3.5-B02-retire-hand-authored-id-tables.md`) -- `CHEST_RANGE` was
    /// always correct here; `mining.rs`'s own `decode_chest_state` was the defect, already
    /// retired in the preceding Implementation changeset.
    #[test]
    fn chest_range_matches_generated_range() {
        let chest = range_of(block_id::CHEST);
        assert_eq!((chest.first.0, chest.last.0), (3987, 4010));
    }

    #[test]
    fn hopper_range_matches_generated_range() {
        let hopper = range_of(block_id::HOPPER);
        assert_eq!((hopper.first.0, hopper.last.0), (11313, 11322));
    }
}
