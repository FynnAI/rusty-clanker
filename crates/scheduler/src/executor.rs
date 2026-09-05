//! RC-Executor: the built, immutable conflict graph plus the per-region tick driver.

use std::collections::HashSet;

use bevy_ecs::component::ComponentId;
use bevy_ecs::system::System;
use bevy_ecs::world::World;

use crate::access::ComponentAccessSummary;
use crate::messaging_bridge::{
    BorderUpdateInbox, CurrentTick, LightBorderInbox, RegionMessageOutbox, RegionTransferInbox,
};
use crate::pipeline::DomainGroup;
use crate::pool::RcWorkerPool;
use crate::region::RegionState;
use crate::registry::{EntityArrivalDriver, LightingStageDriver, SystemFactory};
use rc_messaging::{RegionId, RegionMessage, Transport};

pub(crate) struct CompiledSystem {
    pub(crate) factory: SystemFactory,
    pub(crate) access: ComponentAccessSummary,
    pub(crate) structural_writes: HashSet<ComponentId>,
}

pub(crate) struct CompiledGroup {
    pub(crate) systems: Vec<CompiledSystem>, // index == order_tag
    pub(crate) waves: Vec<Vec<usize>>,       // from compute_waves; ignored by Stage 4's dispatch
}

/// M3-B06 field-report fix, extended by M4-B01: `DomainGroup::ALL`'s own declaration
/// order stays exactly what M0-B05 fixed it as (`BlockRedstone, ..., ChunkSerialize,
/// NetCodec`) with M3-B06's two groups appended (`RandomTick, BlockEntity`) and
/// M4-B01's `AiPhysics` replacement (`EntityAiSelection, EntityPhysicsIntegration`)
/// substituted in place — required, since `DomainGroup::index()`'s return values (and
/// `spawn_region`'s own per-group slot fill, which is order-independent) are pinned by
/// already-committed acceptance tests. But `tick_region`'s *dispatch* order must track
/// ascending `Stage` value, not `ALL`'s raw array order (M3-B06's own field-report fix,
/// restated): appending `RandomTick`/`BlockEntity` to the end of `ALL` and iterating
/// `ALL` directly would run Stage 5's and Stage 8's own content *after* Stage 12's
/// `NetCodec` pass, on every tick — random-tick and block-entity state changes would
/// consistently miss that same tick's own network snapshot, an undocumented, unbounded,
/// silent one-tick-late deviation forbidden by this project's own "vanilla parity is
/// bit-identical by default" binding principle. `DISPATCH_ORDER` is `ALL`'s own eight
/// members reordered to ascending `Stage` value once, by hand, at compile time
/// (`RandomTick` = Stage 5 slots between `BlockRedstone` = Stage 4 and
/// `EntityAiSelection` = Stage 6; `EntityPhysicsIntegration` = Stage 7 slots between
/// `EntityAiSelection` = Stage 6 and `BlockEntity` = Stage 8; `BlockEntity` = Stage 8
/// slots between `EntityPhysicsIntegration` = Stage 7 and `Lighting` = Stage 9) —
/// `tick_region` iterates this, never `DomainGroup::ALL` directly.
const DISPATCH_ORDER: [DomainGroup; 8] = [
    DomainGroup::BlockRedstone,
    DomainGroup::RandomTick,
    DomainGroup::EntityAiSelection,
    DomainGroup::EntityPhysicsIntegration,
    DomainGroup::BlockEntity,
    DomainGroup::Lighting,
    DomainGroup::ChunkSerialize,
    DomainGroup::NetCodec,
];

/// The built, immutable RC-Executor (ARCH-D8: conflict graph computed once,
/// "reused for every tick of every region"). `Send + Sync` — safe to share
/// (`&RcExecutor`) across multiple regions' ticks running concurrently on
/// different threads, a later blueprint's use case, not exercised here.
pub struct RcExecutor {
    bootstrap: fn(&mut bevy_ecs::world::World),
    groups: [CompiledGroup; 8],
    /// M4-B07: Stage 8's own additive dispatch path (Context §8) — `None` unless a
    /// caller registered one via `RcExecutorBuilder::with_lighting_driver`.
    lighting_driver: Option<LightingStageDriver>,
    /// M4-B08 (Context, Part 1.2): Stage 1's own additive arrival-application hook —
    /// `None` unless a caller registered one via
    /// `RcExecutorBuilder::with_entity_arrival_driver`.
    entity_arrival_driver: Option<EntityArrivalDriver>,
}

/// Minimal per-tick result. Extended by later blueprints as needed (e.g. per-stage
/// timing for ARCH-D19's hotness EWMA) — not this blueprint's scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickReport {
    pub tick_counter: u64,
}

impl RcExecutor {
    /// Crate-private constructor -- `RcExecutorBuilder::build` (`registry.rs`) is
    /// the only caller; the conflict graph is computed there, once.
    pub(crate) fn new(
        bootstrap: fn(&mut World),
        groups: [CompiledGroup; 8],
        lighting_driver: Option<LightingStageDriver>,
        entity_arrival_driver: Option<EntityArrivalDriver>,
    ) -> Self {
        Self {
            bootstrap,
            groups,
            lighting_driver,
            entity_arrival_driver,
        }
    }

    /// Creates a fresh region: a new `World` (bootstrapped identically to the
    /// prototype `World` used at build time), one freshly-`.initialize`d instance
    /// of every registered system, zeroed tick counter, empty `RegionMessageState`.
    pub fn spawn_region(&self, id: RegionId) -> RegionState {
        let mut world = World::new();
        (self.bootstrap)(&mut world);

        let mut system_instances: [Vec<Box<dyn System<In = (), Out = ()>>>; 8] = [
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];

        for group in DomainGroup::ALL {
            let compiled = &self.groups[group.index()];
            let mut instances = Vec::with_capacity(compiled.systems.len());
            for compiled_system in &compiled.systems {
                let mut system = (compiled_system.factory)();
                let access_set = system.initialize(&mut world);
                // Cross-checks the "`ComponentId` consistency across regions"
                // invariant (Context) this region's own `World` must uphold: this
                // system's access, re-derived against *this* freshly-bootstrapped
                // `World`, must still be self-consistent with its own
                // `structural_writes` -- exactly the check `RcExecutorBuilder::build`
                // already performed once against the prototype `World`. A mismatch
                // here would mean this region's `World` registered components in a
                // different order than the prototype did, silently invalidating the
                // conflict graph computed against the prototype.
                let summary =
                    ComponentAccessSummary::from_bevy_access(access_set.combined_access());
                debug_assert!(
                    summary
                        .writes
                        .is_disjoint(&compiled_system.structural_writes),
                    "region World's component registration diverged from the prototype \
                     World used at RcExecutorBuilder::build time"
                );
                instances.push(system);
            }
            system_instances[group.index()] = instances;
        }

        // M3-B01's `RegionMessageBus`-in-a-system bridge (Context: "Cross-region border
        // updates"): every region gets these three resources, unconditionally, at zero
        // cost to a region that registers nothing into Stage 4 — order relative to the
        // `.initialize` calls above does not matter (resources and components live in
        // disjoint id spaces).
        world.insert_resource(BorderUpdateInbox::default());
        world.insert_resource(RegionMessageOutbox::default());
        world.insert_resource(CurrentTick::default());
        // M4-B07: mirrors the three resources above exactly (Context §8).
        world.insert_resource(LightBorderInbox::default());
        // M4-B08 (Context, Part 1.2): mirrors the resources above exactly.
        world.insert_resource(RegionTransferInbox::default());

        RegionState {
            id,
            world,
            tick_counter: 0,
            message_state: rc_messaging::RegionMessageState::new(),
            system_instances,
        }
    }

    /// Advances `region` through the fixed 11-stage pipeline exactly once
    /// (ARCH-D12), dispatching each domain group's waves onto `pool`, applying the
    /// two ARCH-D9 sync points with Stage 4's inline exception, and fulfilling
    /// M0-B02's exact Stage-1/Stage-10 driver contract against `transport`.
    /// Synchronous — this is the "synchronous test-mode tick driver" shape
    /// `09-testing-quality.md`'s TEST-D14 describes, bypassing real-time EDF
    /// admission entirely; a later blueprint wraps this in the wall-clock-paced,
    /// multi-region 20 TPS loop (out of scope here).
    pub fn tick_region(
        &self,
        region: &mut RegionState,
        pool: &RcWorkerPool,
        transport: &dyn Transport,
    ) -> TickReport {
        // --- Stage 1: pre-tick sync (M0-B02's Stage-1 contract) ---
        let mut inbound = Vec::new();
        while let Some(msg) = transport.try_recv(region.id) {
            inbound.push(msg.payload);
        }

        // M3-B01's bridge (Context): mirror `tick_counter`'s pre-increment value and
        // filter this same drained batch's `BorderUpdateEvent` payloads into the
        // `World`-reachable resources a Stage-4 system reads — computed from `inbound`
        // by reference before `set_inbox` below takes ownership of it (same batch, same
        // observable effect as computing this after that call; reordered only because
        // `set_inbox` consumes its argument by value).
        region.world.resource_mut::<CurrentTick>().0 = region.tick_counter;
        region.world.resource_mut::<BorderUpdateInbox>().0 = inbound
            .iter()
            .filter_map(|m| match m {
                RegionMessage::BorderUpdateEvent(ev) => Some(*ev),
                _ => None,
            })
            .collect();
        // M4-B07: the same already-drained `inbound` batch, no second drain call
        // (Context §8).
        region.world.resource_mut::<LightBorderInbox>().0 = inbound
            .iter()
            .filter_map(|m| match m {
                RegionMessage::LightBorderUpdate(ev) => Some((**ev).clone()),
                _ => None,
            })
            .collect();

        // M4-B08 (Context, Part 1.1/1.2): the same already-drained `inbound` batch, no
        // second drain call. Populated *before* the entity-arrival driver below runs, and
        // stays readable afterward.
        let arrivals: Vec<rc_messaging::EntitySnapshot> = inbound
            .iter()
            .filter_map(|m| match m {
                RegionMessage::RegionTransferRequest(snap) => Some((**snap).clone()),
                _ => None,
            })
            .collect();
        region.world.resource_mut::<RegionTransferInbox>().0 = arrivals.clone();

        region.message_state.set_inbox(inbound);

        // M4-B08 (Context, Part 1.1: "Arrive-tick"): applies this tick's drained arrivals
        // to `world`, strictly before any registered `DomainGroup` system starts (Stage 1's
        // own internal step) — the third legal structural-mutation call site alongside
        // ARCH-D9's two sync points, since no system holds a live `Query`/`QueryState`
        // borrow into `world` at this point in `tick_region`.
        if let Some(driver) = self.entity_arrival_driver {
            driver(&mut region.world, arrivals);
        }

        // Stages 2, 3, 5, 7: content-less no-ops at M0 (no `DomainGroup` maps to
        // them; Context's "no mechanics content exists").

        let RegionState {
            world,
            system_instances,
            ..
        } = region;

        // (stage as u8, order_tag, group index, system index) for every system
        // whose deferred `Commands` state must be applied at the Stage-10 sync
        // point (ARCH-D9) -- every group except Stage 4 (applies inline, the
        // ARCH-D9 exception) and Stage 11 (read-only, never applied -- Constraint f).
        let mut deferred_targets: Vec<(u8, u32, usize, usize)> = Vec::new();

        for group in DISPATCH_ORDER {
            let compiled = &self.groups[group.index()];
            let instances = &mut system_instances[group.index()];

            match group {
                DomainGroup::BlockRedstone => {
                    // Stage 4 (ARCH-D13): mandatory sequential collapse regardless
                    // of the group's declared waves; `System::run` applies each
                    // system's own deferred commands immediately, before the next
                    // Stage-4 system starts (ARCH-D9's exception).
                    for system in instances.iter_mut() {
                        system.run((), world).expect("Stage 4 system failed to run");
                    }
                }
                DomainGroup::NetCodec | DomainGroup::EntityAiSelection => {
                    // Stage 12 (`NetCodec`) and Stage 6 (`EntityAiSelection`, M4-B01):
                    // both read-only -- run, but never apply. Reusing this exact code
                    // path for `EntityAiSelection` is deliberate and load-bearing
                    // (Context: "Stage-6a/6b system registration model"): it makes
                    // MECH-D32's "never mutates World state from within Stage 6a" rule
                    // structural, not conventional -- a `Commands`-misusing Stage-6a
                    // system's deferred state is silently retained, never flushed,
                    // the identical documented limitation Constraint (f) already
                    // accepts for Stage 12.
                    run_group_waves(compiled, instances, world, pool);
                }
                DomainGroup::Lighting => {
                    // Stage 8: conflict-graph-batched, deferred until Stage 10 --
                    // identical to the catch-all arm below, plus (M4-B07) this
                    // group's own additive dispatch path: `RcExecutorBuilder::
                    // with_lighting_driver`'s registered chunk-parallel BSP round
                    // driver, run after the ordinary wave dispatch so a future,
                    // unrelated `DomainGroup::Lighting`-registered system (none
                    // exists at M4) still executes normally first (Context §8).
                    run_group_waves(compiled, instances, world, pool);
                    let stage = group.stage() as u8;
                    for order_tag in 0..instances.len() {
                        deferred_targets.push((stage, order_tag as u32, group.index(), order_tag));
                    }
                    if let Some(driver) = self.lighting_driver {
                        driver(world, pool);
                    }
                }
                _ => {
                    // Stages 7, 8, 9, 10: conflict-graph-batched, deferred until
                    // Stage 11.
                    run_group_waves(compiled, instances, world, pool);
                    let stage = group.stage() as u8;
                    for order_tag in 0..instances.len() {
                        deferred_targets.push((stage, order_tag as u32, group.index(), order_tag));
                    }
                }
            }
        }

        // --- Stage 10: post-tick flush (ARCH-D9) ---
        // Primary key = originating stage number, secondary key = order_tag
        // ascending (Context: "Stage 10's '(stage, emission order)'").
        deferred_targets.sort_unstable_by_key(|&(stage, order_tag, _, _)| (stage, order_tag));
        for (_, _, group_index, system_index) in deferred_targets {
            system_instances[group_index][system_index].apply_deferred(world);
        }

        // M3-B01's bridge (Context): fold this tick's `RegionMessageOutbox` resource
        // (any registered system's `.send` calls) into `message_state`'s own outbox
        // before it is drained below, so a send from any system this tick is flushed
        // to `dyn Transport` within the same tick it was emitted (`world` here is the
        // same `&mut World` as `region.world` — bound via the destructure above).
        let bridged = world.resource_mut::<RegionMessageOutbox>().take();
        region.message_state.merge(bridged);

        let outgoing = region
            .message_state
            .drain_outbox(region.id, region.tick_counter);
        for msg in outgoing {
            // `TransportError::Backpressure` is dropped -- ARCH-D29's own
            // retry-policy Open Question is unresolved in the planning corpus and
            // not decided by this blueprint (Context: "M0-B02's Stage-1/Stage-10
            // contract").
            let _ = transport.send(msg);
        }

        let report = TickReport {
            tick_counter: region.tick_counter,
        };
        region.tick_counter += 1;
        report
    }
}

/// Runs one domain group's already-computed waves against `world`, dispatching
/// each multi-member wave onto `pool`. Never applies any system's deferred
/// `Commands` state -- callers decide separately whether (Stage 10) or never
/// (Stage 11) to call `apply_deferred` afterward. Stage 4 never calls this
/// function (its own sequential-collapse dispatch in `tick_region` ignores
/// `compiled.waves` entirely, per `CompiledGroup::waves`'s own doc comment).
fn run_group_waves(
    compiled: &CompiledGroup,
    instances: &mut [Box<dyn System<In = (), Out = ()>>],
    world: &mut World,
    pool: &RcWorkerPool,
) {
    for wave in &compiled.waves {
        match wave.as_slice() {
            [] => {}
            [only] => {
                instances[*only]
                    .run_without_applying_deferred((), world)
                    .expect("system failed to run");
            }
            many => {
                let world_cell = world.as_unsafe_world_cell();
                let members = borrow_disjoint(instances, many);
                let mut tasks: Vec<Box<dyn FnOnce() + Send + '_>> =
                    Vec::with_capacity(members.len());
                for system in members {
                    tasks.push(Box::new(move || {
                        // SAFETY: `wave` is one wave `compute_waves` produced; by
                        // that function's own doc-comment invariant, every member of
                        // one wave is pairwise access-compatible with every other
                        // member (Constraint (d)). Concurrent `run_unsafe` calls
                        // against the same `world_cell`, one per wave member, are
                        // therefore free of conflicting simultaneous World access.
                        unsafe {
                            system
                                .run_unsafe((), world_cell)
                                .expect("system failed to run");
                        }
                    }));
                }
                pool.run_batch(tasks);
            }
        }
    }
}

/// Splits `slice` into one `&mut T` per entry of `indices`, which must be
/// ascending and pairwise distinct (guaranteed by `compute_waves`'s own
/// "within a wave, indices are ascending" contract) -- entirely safe code, no
/// `unsafe` needed: each successive `split_at_mut` call only ever narrows the
/// remaining slice, so no two returned references can ever alias.
fn borrow_disjoint<'a, T>(slice: &'a mut [T], indices: &[usize]) -> Vec<&'a mut T> {
    let mut result = Vec::with_capacity(indices.len());
    let mut remaining = slice;
    let mut base = 0usize;
    for &idx in indices {
        let local = idx - base;
        let (_, rest) = remaining.split_at_mut(local);
        let (first, rest) = rest.split_at_mut(1);
        result.push(&mut first[0]);
        remaining = rest;
        base = idx + 1;
    }
    result
}
