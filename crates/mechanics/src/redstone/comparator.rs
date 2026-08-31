//! Comparator — compare/subtract modes, container-fullness analog input (Context §G).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;

use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ComparatorMode {
    Compare,
    Subtract,
}

/// The interface boundary M3-B06 implements (Context §G). This blueprint's own tests supply a
/// `HashMap`-backed fake — see Acceptance tests.
pub trait ContainerSignalSource: Send + Sync {
    /// The vanilla analog signal `0..=15` a comparator reading `pos` should see, per the
    /// container-fullness formula (Context §G), or `None` if `pos` holds no tier-1 container
    /// (comparator falls back to `base_diode_input_signal`) — distinct from `Some(0)`, which
    /// means "an empty container is present."
    fn container_signal(&self, pos: BlockPos) -> Option<u8>;
}

/// The trivial default: no position is ever a container (used when no block-entity blueprint
/// has landed yet — the composition root's own safe fallback, not a test-only type).
pub struct NoContainers;
impl ContainerSignalSource for NoContainers {
    fn container_signal(&self, _pos: BlockPos) -> Option<u8> {
        None
    }
}

#[derive(Copy, Clone, Debug)]
struct ComparatorState {
    powered: bool,
    output: u8,
    mode: ComparatorMode,
    /// `true` once `seed_powered_from_world` has run for this position at least once (M3
    /// field-report fix: own-state writeback's read-side companion, mirrors `RepeaterState::
    /// seeded`'s own identical rationale) -- `place()`'s own "placement is out of this
    /// blueprint's scope" gap means `powered` always starts seeded `false`, regardless of a
    /// fixture's real initial `POWERED` value.
    seeded: bool,
}

/// Own-state id arithmetic for `minecraft:comparator` (M3 field-report fix: own-state
/// writeback; WS-D15's generated per-property registry is future work, so this constant is
/// read directly off `datagen-output/26.2/generated/reports/blocks.json`'s own
/// `minecraft:comparator` entry, protocol 776, states 11263..=11278). `facing` (`signal::
/// diode_facing_index`, the same `[north,south,west,east]` order/stride repeater.rs's own id
/// arithmetic shares) is the slowest-varying property, stride 4; then `mode` (`[compare,
/// subtract]`), stride 2; then `powered` (`[true,false]`), stride 1. The analog `output` value
/// (comparator's own held `0..=15` signal strength) has no `BlockStateId` representation at all
/// -- blocks.json's own `minecraft:comparator` entry lists only `facing`/`mode`/`powered`; real
/// vanilla stores it in a separate `ComparatorBlockEntity`, out of this changeset's own scope
/// (Stage-7/block-entity wiring, per the M3 fix-agent brief's own container-case carve-out).
const COMPARATOR_BASE: u32 = 11263;

fn comparator_state_id(facing: Direction, mode: ComparatorMode, powered: bool) -> BlockStateId {
    let mode_idx = match mode {
        ComparatorMode::Compare => 0,
        ComparatorMode::Subtract => 1,
    };
    BlockStateId(
        COMPARATOR_BASE
            + signal::diode_facing_index(facing) * 4
            + mode_idx * 2
            + u32::from(!powered),
    )
}

/// Comparator (Context §G). One instance per region (Context §I).
pub struct ComparatorBehavior {
    /// M3 field-report fix (Task 2): interior-mutable, mirroring `RepeaterBehavior::facing`'s
    /// own identical rationale — `place` must be `&self`-callable so a comparator's own facing
    /// can be updated after this behavior has already been wrapped in an `Arc` and registered
    /// (`docs/findings-for-planning.md`'s own "diode re-placement" entry).
    facing: Mutex<HashMap<BlockPos, Direction>>,
    state: Mutex<HashMap<BlockPos, ComparatorState>>,
    containers: Arc<dyn ContainerSignalSource>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl ComparatorBehavior {
    pub fn new(containers: Arc<dyn ContainerSignalSource>) -> Self {
        Self {
            facing: Mutex::new(HashMap::new()),
            state: Mutex::new(HashMap::new()),
            containers,
            registry: OnceLock::new(),
        }
    }

    /// Establishes (or, called again for an already-registered position, *replaces*) a
    /// comparator's facing and mode — a full replace-on-replace, resetting every other
    /// per-position field to its own fresh-placement default, mirroring `RepeaterBehavior::
    /// place`'s identical M3 field-report fix (Task 2: placement-state seeding is now
    /// re-entrant).
    pub fn place(&self, pos: BlockPos, facing: Direction, mode: ComparatorMode) {
        self.facing.lock().unwrap().insert(pos, facing);
        self.state.lock().unwrap().insert(
            pos,
            ComparatorState {
                powered: false,
                output: 0,
                mode,
                seeded: false,
            },
        );
    }

    /// Test/composition-root-only mode toggle (Context §G — use-item mode cycling is out of
    /// scope, no item-use handling exists at M3).
    pub fn set_mode(&self, pos: BlockPos, mode: ComparatorMode) {
        self.state
            .lock()
            .unwrap()
            .entry(pos)
            .or_insert(ComparatorState {
                powered: false,
                output: 0,
                mode: ComparatorMode::Compare,
                seeded: false,
            })
            .mode = mode;
    }

    pub fn facing(&self, pos: BlockPos) -> Direction {
        *self
            .facing
            .lock()
            .unwrap()
            .get(&pos)
            .expect("ComparatorBehavior::facing: position was never placed")
    }

    pub fn mode(&self, pos: BlockPos) -> ComparatorMode {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.mode)
            .unwrap_or(ComparatorMode::Compare)
    }

    pub fn output(&self, pos: BlockPos) -> u8 {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.output)
            .unwrap_or(0)
    }

    pub fn powered(&self, pos: BlockPos) -> bool {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.powered)
            .unwrap_or(false)
    }

    fn set_output(&self, pos: BlockPos, output: u8) {
        let mut state = self.state.lock().unwrap();
        let entry = state.entry(pos).or_insert(ComparatorState {
            powered: false,
            output: 0,
            mode: ComparatorMode::Compare,
            seeded: false,
        });
        entry.output = output;
    }

    fn set_powered(&self, pos: BlockPos, powered: bool) {
        let mut state = self.state.lock().unwrap();
        let entry = state.entry(pos).or_insert(ComparatorState {
            powered: false,
            output: 0,
            mode: ComparatorMode::Compare,
            seeded: false,
        });
        entry.powered = powered;
    }

    /// Own-state writeback's read-side companion (M3 field-report fix) -- `ComparatorState::
    /// seeded`'s own doc comment, mirrors `RepeaterBehavior::seed_powered_from_world`'s
    /// identical rationale/shape. Reads `pos`'s own currently-stored raw id (if any) and
    /// decodes its `powered` bit into the side-table, exactly once.
    fn seed_powered_from_world(&self, world: &dyn BlockWorldAccess, pos: BlockPos) {
        let mut state = self.state.lock().unwrap();
        let entry = state.entry(pos).or_insert(ComparatorState {
            powered: false,
            output: 0,
            mode: ComparatorMode::Compare,
            seeded: false,
        });
        if entry.seeded {
            return;
        }
        entry.seeded = true;
        if let Some(current) = world.get_block(pos) {
            entry.powered = (current.0.wrapping_sub(COMPARATOR_BASE)) % 2 == 0;
        }
    }

    /// Sets this behavior's own registry handle (Context §I½). Called exactly once, by
    /// `Tier1RedstoneHandles::bind_registry` immediately after the composition root wraps the
    /// `register_tier1_redstone`-populated registry in an `Arc` (or directly, by a test that
    /// constructs this behavior standalone). Panics if called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>) {
        self.registry
            .set(registry)
            .unwrap_or_else(|_| panic!("ComparatorBehavior::bind_registry called more than once"));
    }

    fn registry(&self) -> &Arc<SignalSourceRegistry> {
        self.registry
            .get()
            .expect("ComparatorBehavior: bind_registry must run before dispatch")
    }

    /// `get_input_signal` (Context §G): the container-fullness analog reading at the block
    /// directly in front, if any, entirely replacing (never maxed with) the plain diode input.
    fn get_input_signal(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> u8 {
        let facing = self.facing(pos);
        let front = facing.apply(pos);
        match self.containers.container_signal(front) {
            Some(analog) => analog,
            None => signal::base_diode_input_signal(world, registry, pos, facing),
        }
    }

    /// The comparator's own side reading (Context §G): the plain signal from each perpendicular
    /// neighbor, `max`'d -- never diode-gated (unlike repeater's `alternate_signal`, which only
    /// feeds a boolean lock; a comparator reads a plain wire's power on its side directly).
    fn side_input_signal(
        &self,
        world: &dyn BlockWorldAccess,
        registry: &SignalSourceRegistry,
        pos: BlockPos,
    ) -> u8 {
        let facing = self.facing(pos);
        let (a, b) = signal::perpendicular_pair(facing);
        signal::signal_into(world, registry, pos, a)
            .max(signal::signal_into(world, registry, pos, b))
    }

    /// `calculate_output_signal` (Context §G) — a pure function, exposed directly for the
    /// acceptance tests' own hand-derived table (see Acceptance tests) without needing a full
    /// `UpdateContext` to exercise it.
    pub fn calculate_output_signal(input: u8, side: u8, mode: ComparatorMode) -> u8 {
        if input == 0 {
            return 0;
        }
        if side > input {
            return 0;
        }
        match mode {
            ComparatorMode::Compare => input,
            ComparatorMode::Subtract => input - side,
        }
    }

    pub fn should_turn_on(input: u8, side: u8, mode: ComparatorMode) -> bool {
        input > side || (input == side && mode == ComparatorMode::Compare)
    }
}

impl RedstoneSignalSource for ComparatorBehavior {
    fn weak_signal_toward(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        // Own-state writeback's read-side companion (M3 field-report fix, `ComparatorState::
        // seeded`'s own doc comment): mirrors `RepeaterBehavior::weak_signal_toward`'s identical
        // seed-on-read rationale -- a reader may reach this comparator's signal before its own
        // on_neighbor_changed/on_scheduled_tick has ever dispatched.
        self.seed_powered_from_world(world, pos);
        // `FACING` points toward this comparator's own INPUT side (Context §G, ASSET-D18(f)
        // research verdict); output flows out the opposite side, matching repeater's own
        // symmetric behavior.
        if towards == self.facing(pos).opposite() {
            self.output(pos)
        } else {
            0
        }
    }
    fn direct_signal_toward(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        self.weak_signal_toward(world, pos, towards)
    }
    fn is_signal_source(&self) -> bool {
        true
    }
    fn is_diode(&self) -> bool {
        true
    }
    fn connects_from(&self, _world: &dyn BlockWorldAccess, pos: BlockPos, from: Direction) -> bool {
        from == self.facing(pos) || from == self.facing(pos).opposite()
    }
    fn diode_facing(&self, pos: BlockPos) -> Option<Direction> {
        Some(self.facing(pos))
    }
}

impl BlockBehavior for ComparatorBehavior {
    /// `checkTickOnNeighbor` (Context §G): overridden to compare against the *stored analog
    /// value* in addition to the boolean -- otherwise the same shared `DiodeBlock`-base
    /// priority-selection logic as repeater's own override.
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        self.seed_powered_from_world(ctx.world, pos);
        let registry = Arc::clone(self.registry());
        let facing = self.facing(pos);
        let input = self.get_input_signal(ctx.world, &registry, pos);
        let side = self.side_input_signal(ctx.world, &registry, pos);
        let mode = self.mode(pos);
        let new_output = Self::calculate_output_signal(input, side, mode);
        let new_should = Self::should_turn_on(input, side, mode);
        let powered = self.powered(pos);
        let stored_output = self.output(pos);

        let mismatch = powered != new_should || new_output != stored_output;
        if mismatch && !ctx.scheduled.is_block_tick_pending(pos) {
            let priority = signal::diode_priority(ctx.world, &registry, pos, facing, powered);
            ctx.schedule_block_tick(pos, 2, priority);
        }
    }

    /// `refresh_output_state` (Context §G): the analog `output` is always stored; `powered` is
    /// only flipped and neighbors only notified if the analog value changed or the mode is
    /// `Compare`.
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let registry = Arc::clone(self.registry());
        let input = self.get_input_signal(ctx.world, &registry, pos);
        let side = self.side_input_signal(ctx.world, &registry, pos);
        let mode = self.mode(pos);
        let new_output = Self::calculate_output_signal(input, side, mode);
        let new_should = Self::should_turn_on(input, side, mode);
        let stored_output = self.output(pos);

        self.set_output(pos, new_output);
        if new_output != stored_output || mode == ComparatorMode::Compare {
            self.set_powered(pos, new_should);
            // Own-state writeback (M3 field-report fix): vanilla's `ComparatorBlock::
            // refreshOutputState` writes `POWERED` back via `level.setBlock(pos, .., 2)` (flag 2
            // = clients-only, no cascading neighbor/shape update of its own) whenever this same
            // branch flips it -- the real neighbor notify is `notify_neighbor_changed_only`
            // below, unchanged, so this never goes through `ctx.set_block`. The analog `output`
            // value has no `BlockStateId` representation (`comparator_state_id`'s own doc
            // comment) -- only `POWERED` is ever encoded here.
            let facing = self.facing(pos);
            let id = comparator_state_id(facing, mode, new_should);
            ctx.world.set_block(pos, id);
            signal::notify_neighbor_changed_only(ctx, pos);
        }
    }

    /// M3 field-report fix (Task 2): reseeds `facing`/`mode` (a full replace-on-replace via
    /// `place`, `place`'s own doc comment) directly off `pos`'s own current raw id — the exact
    /// inverse of `comparator_state_id`'s own arithmetic, mirroring `RepeaterBehavior::on_placed`
    /// and `seed_powered_from_world`'s established decode-from-raw-id pattern. A caller with its
    /// own freshly-written `BlockStateId` at `pos` (a real `/setblock`-shaped placement, never
    /// this behavior's own internal writeback) calls `on_placed` instead of a losing, ordering-
    /// sensitive direct `place()` call of its own.
    fn on_placed(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let Some(current) = ctx.get_block(pos) else {
            return;
        };
        let rel = current.0.wrapping_sub(COMPARATOR_BASE);
        if rel >= 16 {
            return; // not a real comparator id -- defensive only, dispatch never reaches here otherwise
        }
        let facing = signal::diode_facing_from_index(rel / 4);
        let mode = if (rel % 4) / 2 == 0 {
            ComparatorMode::Compare
        } else {
            ComparatorMode::Subtract
        };
        self.place(pos, facing, mode);
    }
}
