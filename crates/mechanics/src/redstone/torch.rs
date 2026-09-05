//! Redstone torch — inverter, quasi-connectivity input, burnout (Context §E).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_registries::block_state_properties::{properties, range_of, state_id, with_property};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::{BlockStateId as GenStateId, default_state};

use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;
use crate::world_access::BlockWorldAccess;

use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

fn torch_facing_from_str(s: &str) -> Direction {
    match s {
        "north" => Direction::North,
        "south" => Direction::South,
        "west" => Direction::West,
        "east" => Direction::East,
        other => panic!("torch_facing_from_str: unrecognized wall-torch facing value {other:?}"),
    }
}

fn lit_str(lit: bool) -> &'static str {
    if lit { "true" } else { "false" }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TorchAttachment {
    Floor,
    Wall(Direction),
}

impl TorchAttachment {
    /// The direction this torch reads its input from (Context §E).
    pub fn input_direction(self) -> Direction {
        match self {
            TorchAttachment::Floor => Direction::Down,
            TorchAttachment::Wall(facing) => facing.opposite(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct TorchState {
    lit: bool,
    burnt_out: bool,
}

/// M3.5-B02 (WS-D15): `true` iff `raw` falls inside `minecraft:redstone_wall_torch`'s own real
/// generated id range -- `TorchBehavior::attachment_at`'s own guard, mirroring `wire.rs`'s
/// documented `is_wire_range` convention: keeps a `Wall`-constructed behavior safe against a
/// unit test's own small placeholder id (`wall_torch_reads_from_its_attach_direction`'s own
/// `TORCH_ID = BlockStateId(1)`, standing in for "some wall torch" without needing a real id) as
/// well as a position with nothing stored yet, falling back to the constructor's own `attachment`
/// in both cases rather than attempting arithmetic that assumes a real one.
fn is_wall_range(raw: u32) -> bool {
    let range = range_of(block_id::REDSTONE_WALL_TORCH);
    (range.first.0..=range.last.0).contains(&raw)
}

/// `air`'s own raw id (M3 field-report fix, Task 1) — M3.5-B02: read off `rc-registries`' own
/// generated `default_state::AIR` constant (value unchanged, `0`) now that this crate already
/// depends on `rc-registries` normally.
const AIR_ID: BlockStateId = BlockStateId(default_state::AIR.0);

/// Redstone torch (Context §E). One instance per region (Context §I).
pub struct TorchBehavior {
    /// The constructor-supplied attachment (`Floor`, or a representative `Wall` orientation --
    /// `registration.rs`'s own single shared wall-torch instance covers every registered id with
    /// one constructed value, since there is only ever one `TorchBehavior` per variant per
    /// region, Context §I). For `Floor`, this is authoritative -- a floor torch has no `facing`
    /// property, so there is nothing to derive per-position. For `Wall`, this is only the
    /// *fallback*: `attachment_at` derives each individual wall torch's own real per-position
    /// `facing` straight off its own stored `BlockStateId` instead (M3 field-report fix, closing
    /// the last 17 corpus mismatches -- a shared field cannot track per-position facing, exactly
    /// the same shape as `RepeaterBehavior`'s own `facing: Mutex<HashMap<BlockPos, Direction>>`
    /// side-table, except a wall torch's facing is never actually mutable state worth
    /// side-tabling -- it is already fully recoverable, on every read, from the id already
    /// sitting in the world). This field is used as `Wall`'s fallback only when the position's
    /// own id does not (yet) fall in the real wall-torch range (`is_wall_range`'s own doc
    /// comment: a unit test's small placeholder id, or nothing stored yet).
    attachment: TorchAttachment,
    state: Mutex<HashMap<BlockPos, TorchState>>,
    recent_toggles: Mutex<HashMap<BlockPos, VecDeque<u64>>>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl TorchBehavior {
    pub const RECENT_TOGGLE_TIMER: u64 = 60;
    pub const MAX_RECENT_TOGGLES: usize = 8;
    pub const RESTART_DELAY: u64 = 160;
    pub const REEVAL_DELAY: u64 = 2;

    pub fn new(attachment: TorchAttachment) -> Self {
        Self {
            attachment,
            state: Mutex::new(HashMap::new()),
            recent_toggles: Mutex::new(HashMap::new()),
            registry: OnceLock::new(),
        }
    }

    /// `true` if never observed (matches vanilla's own freshly-placed-lit default, Context §E).
    pub fn lit(&self, pos: BlockPos) -> bool {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.lit)
            .unwrap_or(true)
    }

    /// This position's own real attachment (M3 field-report fix, closing the last 17 corpus
    /// mismatches -- `attachment` field's own doc comment). For a `Floor`-constructed behavior,
    /// short-circuits straight to `self.attachment` without ever touching the world -- a floor
    /// torch's attachment never varies. For a `Wall`-constructed behavior, decodes the real
    /// `facing` off `pos`'s own currently-stored raw id when it falls in the wall-torch range
    /// (mirroring `RepeaterBehavior::on_placed`'s/`ComparatorBehavior::on_placed`'s established
    /// "recover facing from the placed id" pattern), otherwise falls back to `self.attachment`
    /// (`is_wall_range`'s own doc comment: a unit test's small placeholder id, or a position with
    /// nothing stored yet). Every `RedstoneSignalSource`/`BlockBehavior` read path below calls
    /// this instead of reading `self.attachment` directly.
    fn attachment_at(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> TorchAttachment {
        let TorchAttachment::Wall(_) = self.attachment else {
            return self.attachment;
        };
        match world.get_block(pos) {
            Some(current) if is_wall_range(current.0) => {
                let facing_str = properties(GenStateId(current.0))
                    .iter()
                    .find(|(name, _)| *name == "facing")
                    .map(|(_, v)| *v)
                    .unwrap_or_else(|| {
                        panic!("attachment_at: raw id {} has no facing property", current.0)
                    });
                TorchAttachment::Wall(torch_facing_from_str(facing_str))
            }
            _ => self.attachment,
        }
    }

    /// Pure query, no mutation (Context §E's "out of scope, flagged" support-loss note) —
    /// `true` iff this torch's own support block (per `attachment_at`'s per-position derivation)
    /// is currently not a conductor.
    pub fn should_pop(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        let support = self.attachment_at(world, pos).input_direction().apply(pos);
        !signal::is_conductor(world, support)
    }

    /// Sets this behavior's own registry handle (Context §I½). Called exactly once, by
    /// `Tier1RedstoneHandles::bind_registry` immediately after the composition root wraps the
    /// `register_tier1_redstone`-populated registry in an `Arc` (or directly, by a test that
    /// constructs this behavior standalone). Panics if called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>) {
        self.registry
            .set(registry)
            .unwrap_or_else(|_| panic!("TorchBehavior::bind_registry called more than once"));
    }

    fn registry(&self) -> &Arc<SignalSourceRegistry> {
        self.registry
            .get()
            .expect("TorchBehavior: bind_registry must run before dispatch")
    }

    /// This own-state writeback's new `BlockStateId` for `lit` (M3 field-report fix). Floor: a
    /// pure `lit` encoding, no facing dimension. Wall: `with_property` leaves every other
    /// property of `current_raw` -- in particular `facing` -- exactly as it already has it
    /// (M3.5-B02, WS-D15: this is a direct match for `with_property`'s own contract), so only
    /// the `lit` bits are ever replaced here, exactly mirroring vanilla's own
    /// `state.setValue(LIT, val)` (leaves `FACING` untouched).
    fn new_state_id(&self, current_raw: u32, lit: bool) -> BlockStateId {
        match self.attachment {
            TorchAttachment::Floor => {
                let id = state_id(block_id::REDSTONE_TORCH, &[("lit", lit_str(lit))])
                    .expect("new_state_id: lit is always a legal minecraft:redstone_torch value");
                BlockStateId(id.0)
            }
            TorchAttachment::Wall(_) => {
                let id = with_property(GenStateId(current_raw), "lit", lit_str(lit)).expect(
                    "new_state_id: lit is always a legal minecraft:redstone_wall_torch value",
                );
                BlockStateId(id.0)
            }
        }
    }

    fn has_neighbor_signal(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        signal::has_signal(
            world,
            self.registry(),
            pos,
            self.attachment_at(world, pos).input_direction(),
        )
    }

    fn is_burnt_out(&self, pos: BlockPos) -> bool {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.burnt_out)
            .unwrap_or(false)
    }

    fn set_burnt_out(&self, pos: BlockPos, value: bool) {
        self.state
            .lock()
            .unwrap()
            .entry(pos)
            .or_insert(TorchState {
                lit: true,
                burnt_out: false,
            })
            .burnt_out = value;
    }

    fn set_lit(&self, pos: BlockPos, value: bool) {
        self.state
            .lock()
            .unwrap()
            .entry(pos)
            .or_insert(TorchState {
                lit: true,
                burnt_out: false,
            })
            .lit = value;
    }

    /// Prunes entries older than `RECENT_TOGGLE_TIMER`, pushes `current_tick`, returns the
    /// resulting count (Context §E burnout paragraph).
    fn record_and_prune_toggle(&self, pos: BlockPos, current_tick: u64) -> usize {
        let mut toggles = self.recent_toggles.lock().unwrap();
        let entries = toggles.entry(pos).or_default();
        entries.retain(|&t| current_tick.saturating_sub(t) <= Self::RECENT_TOGGLE_TIMER);
        entries.push_back(current_tick);
        entries.len()
    }

    /// The reeval logic shared by both the ordinary 2-tick re-check and the burnout-restart
    /// tick (Context §E: "that tick itself still respects the ordinary ... flip logic").
    fn reeval_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let target_lit = !self.has_neighbor_signal(ctx.world, pos);
        let current_lit = self.lit(pos);
        if current_lit == target_lit {
            return;
        }
        self.set_lit(pos, target_lit);
        // M3 field-report fix: own-state writeback -- vanilla's `RedstoneTorchBlock::tick`
        // flips `LIT` via `level.setBlock(pos, state.setValue(LIT, val), 2)` (flag 2 =
        // clients-only, no cascading neighbor/shape update of its own -- the actual neighbor
        // notify is `notify_neighbor_changed_only` below, unchanged), so this write goes
        // through the raw world accessor, never `ctx.set_block`.
        if let Some(current) = ctx.get_block(pos) {
            let new_id = self.new_state_id(current.0, target_lit);
            ctx.write_block_state(pos, new_id);
        }
        if !target_lit {
            let count = self.record_and_prune_toggle(pos, ctx.current_tick);
            if count >= Self::MAX_RECENT_TOGGLES {
                self.set_burnt_out(pos, true);
                ctx.schedule_block_tick(pos, Self::RESTART_DELAY, TickPriority::Normal);
            }
        }
        signal::notify_neighbor_changed_only(ctx, pos);
    }
}

impl RedstoneSignalSource for TorchBehavior {
    fn weak_signal_toward(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        if !self.lit(pos) {
            return 0;
        }
        if towards == self.attachment_at(world, pos).input_direction() {
            0
        } else {
            15
        }
    }
    /// M3 field-report fix (finding 4): vanilla's `RedstoneTorchBlock::getDirectSignal` is
    /// hard-coded to fire only when the querying block sits directly ABOVE the torch, for
    /// both the floor and wall variants (`RedstoneWallTorchBlock` never overrides it) — the
    /// former attachment-derived axis (`input_direction().opposite()`) was only correct for a
    /// floor torch by coincidence (`input_direction() == Down`, whose `.opposite()` is `Up`);
    /// for a wall torch it fired sideways, toward the torch's own `facing`, instead. This
    /// project's own `towards` runs source -> receiver (`signal.rs`'s `direct_signal_to` calls
    /// `direct_signal_toward(npos, d.opposite())`), so the fix is simply attachment-
    /// independent: fire only toward `Up`.
    fn direct_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        if self.lit(pos) && towards == Direction::Up {
            15
        } else {
            0
        }
    }
    fn is_signal_source(&self) -> bool {
        true
    }
}

impl BlockBehavior for TorchBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        if self.is_burnt_out(pos) {
            return;
        }
        let target_lit = !self.has_neighbor_signal(ctx.world, pos);
        let current_lit = self.lit(pos);
        if current_lit != target_lit && !ctx.scheduled.is_block_tick_pending(pos) {
            ctx.schedule_block_tick(pos, Self::REEVAL_DELAY, TickPriority::Normal);
        }
    }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        // A pending restart tick (burnout) always clears the flag before the ordinary reeval
        // logic runs -- a no-op when not currently burnt out (Context §E: "that tick itself
        // still respects the ordinary ... flip logic").
        self.set_burnt_out(pos, false);
        self.reeval_tick(ctx, pos);
    }
    /// M3 field-report fix (Task 1): support-loss destruction — `BaseTorchBlock` survives only
    /// if its support (floor torch: the block below; wall torch: its own attached block, both
    /// `TorchAttachment::input_direction`) remains solid; a shape update arriving from that same
    /// direction destroys the torch (returns air) if it is now gone
    /// (`08-redstone-ticking.md` §3.7/Notes: "or `Blocks.AIR.defaultBlockState()` to
    /// self-destruct, e.g. a torch losing its supporting wall"). `should_pop`'s own pre-existing
    /// "no mutation" query already implements the exact support check this now actually wires
    /// up; every other trigger direction is unaffected (Context §E: detection only elsewhere).
    /// Also clears this position's own side-table state, so a future re-placement here starts
    /// fresh rather than inheriting the destroyed torch's last `lit`/`burnt_out` values.
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        if from == self.attachment_at(ctx.world, pos).input_direction()
            && self.should_pop(ctx.world, pos)
        {
            self.state.lock().unwrap().remove(&pos);
            self.recent_toggles.lock().unwrap().remove(&pos);
            return Some(AIR_ID);
        }
        None
    }
}
