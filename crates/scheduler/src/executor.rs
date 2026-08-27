//! RC-Executor: the built, immutable conflict graph plus the per-region tick driver.

use std::collections::HashSet;

use bevy_ecs::component::ComponentId;
use bevy_ecs::system::System;
use bevy_ecs::world::World;

use crate::access::ComponentAccessSummary;
use crate::messaging_bridge::{BorderUpdateInbox, CurrentTick, RegionMessageOutbox};
use crate::pipeline::DomainGroup;
use crate::pool::RcWorkerPool;
use crate::region::RegionState;
use crate::registry::SystemFactory;
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

/// The built, immutable RC-Executor (ARCH-D8: conflict graph computed once,
/// "reused for every tick of every region"). `Send + Sync` — safe to share
/// (`&RcExecutor`) across multiple regions' ticks running concurrently on
/// different threads, a later blueprint's use case, not exercised here.
pub struct RcExecutor {
    bootstrap: fn(&mut bevy_ecs::world::World),
    groups: [CompiledGroup; 5],
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
    pub(crate) fn new(bootstrap: fn(&mut World), groups: [CompiledGroup; 5]) -> Self {
        Self { bootstrap, groups }
    }

    /// Creates a fresh region: a new `World` (bootstrapped identically to the
    /// prototype `World` used at build time), one freshly-`.initialize`d instance
    /// of every registered system, zeroed tick counter, empty `RegionMessageState`.
    pub fn spawn_region(&self, id: RegionId) -> RegionState {
        let mut world = World::new();
        (self.bootstrap)(&mut world);

        let mut system_instances: [Vec<Box<dyn System<In = (), Out = ()>>>; 5] =
            [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];

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

        region.message_state.set_inbox(inbound);

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

        for group in DomainGroup::ALL {
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
                DomainGroup::NetCodec => {
                    // Stage 11: read-only -- run, but never apply (Constraint f's
                    // documented limitation: a `Commands`-misusing Stage-11 system's
                    // state is silently retained, never flushed).
                    run_group_waves(compiled, instances, world, pool);
                }
                _ => {
                    // Stages 6, 8, 9: conflict-graph-batched, deferred until Stage 10.
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
