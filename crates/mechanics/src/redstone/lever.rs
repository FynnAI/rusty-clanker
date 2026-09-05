//! Lever — tier 1's manual redstone input (PLAN-D10/MECH-D13, M3 field-report wave 3,
//! finding 2). Toggled through `BlockBehavior::on_use` (MECH-D82), pops to air when its own
//! mount face stops being `Full`-sturdy (MECH-D84), and answers every `RedstoneSignalSource`
//! query straight off its own stored `BlockStateId` — `face`/`facing`/`powered` are decoded on
//! every read via `rc_registries::block_state_properties::properties`/`with_property`, so this
//! behavior carries **no per-position side table at all**, unlike every other tier-1 component
//! (wire's connection digit, torch's `lit`/burnout state, repeater's `facing`/delay, comparator's
//! analog `output`): a lever's entire observable state — `powered` — already lives in its own
//! block state, with no placement-derived facing/delay/mode dimension and no analog value that
//! needs seeding separately. One shared, stateless instance covers the whole `minecraft:lever`
//! id range (`registration.rs`'s own call site).

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_registries::block_state_properties::{properties, range_of, with_property};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::{BlockStateId as GenStateId, default_state};
use rc_registries::generated_v776::registries::sound_event;

use crate::behavior::{BlockBehavior, UpdateContext, UseContext, UseOutcome, UseUpdateContext};
use crate::direction::Direction;
use crate::sound_request::{SoundRequest, SoundSource};
use crate::world_access::BlockWorldAccess;

use super::signal::{self, RedstoneSignalSource};

/// `minecraft:lever`'s own `face` block-state-property value (`AttachFace` in the reference,
/// `FaceAttachedHorizontalDirectionalBlock`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum AttachFace {
    Floor,
    Wall,
    Ceiling,
}

fn attach_face_from_str(s: &str) -> AttachFace {
    match s {
        "floor" => AttachFace::Floor,
        "wall" => AttachFace::Wall,
        "ceiling" => AttachFace::Ceiling,
        other => panic!("attach_face_from_str: unrecognized lever face value {other:?}"),
    }
}

fn facing_from_str(s: &str) -> Direction {
    match s {
        "north" => Direction::North,
        "south" => Direction::South,
        "west" => Direction::West,
        "east" => Direction::East,
        other => panic!("facing_from_str: unrecognized lever facing value {other:?}"),
    }
}

fn powered_str(powered: bool) -> &'static str {
    if powered { "true" } else { "false" }
}

/// `true` iff `raw` falls inside `minecraft:lever`'s own real generated id range — mirrors
/// `torch.rs`'s documented `is_wall_range` convention: keeps every decode-from-raw-id read path
/// below safe against a unit test's own small placeholder id, or a position with nothing
/// stored yet, without needing a real id.
fn is_lever_range(raw: u32) -> bool {
    let range = range_of(block_id::LEVER);
    (range.first.0..=range.last.0).contains(&raw)
}

/// `air`'s own raw id (`default_state::AIR.0`, stable by protocol convention).
const AIR_ID: BlockStateId = BlockStateId(default_state::AIR.0);

/// Decodes `raw`'s own `(face, facing, powered)` triple — the sole place this module ever
/// interprets a lever's own generated property list. Panics if `raw` is missing any of the
/// three properties (a config-time defect: every real lever id carries all three).
fn decode_raw(raw: u32) -> (AttachFace, Direction, bool) {
    let props = properties(GenStateId(raw));
    let mut face = None;
    let mut facing = None;
    let mut powered = None;
    for (name, value) in props {
        match *name {
            "face" => face = Some(attach_face_from_str(value)),
            "facing" => facing = Some(facing_from_str(value)),
            "powered" => powered = Some(*value == "true"),
            _ => {}
        }
    }
    (
        face.unwrap_or_else(|| panic!("decode_raw: raw id {raw} has no face property")),
        facing.unwrap_or_else(|| panic!("decode_raw: raw id {raw} has no facing property")),
        powered.unwrap_or_else(|| panic!("decode_raw: raw id {raw} has no powered property")),
    )
}

/// `decode_raw`, applied to whatever is currently stored at `pos` — `None` if nothing is
/// loaded there or the stored id does not fall inside the real lever range (`is_lever_range`'s
/// own doc comment).
fn decode(world: &dyn BlockWorldAccess, pos: BlockPos) -> Option<(AttachFace, Direction, bool)> {
    let raw = world.get_block(pos)?.0;
    if !is_lever_range(raw) {
        return None;
    }
    Some(decode_raw(raw))
}

/// `getConnectedDirection(state).getOpposite()` (`FaceAttachedHorizontalDirectionalBlock`,
/// verified against the ASSET-D18(f) reference): the direction from a lever's own cell to the
/// block it is mounted on. Floor mounts on the block below; ceiling mounts on the block above;
/// wall mounts on the block behind its own `facing` (`facing.opposite()` — `facing` itself
/// points AWAY from the wall, into the room, matching `TorchAttachment::Wall`'s own identical
/// convention for the wall torch).
fn mount_direction(face: AttachFace, facing: Direction) -> Direction {
    match face {
        AttachFace::Floor => Direction::Down,
        AttachFace::Ceiling => Direction::Up,
        AttachFace::Wall => facing.opposite(),
    }
}

/// Stateless (this module's own doc comment) — every read decodes directly off the world's
/// own stored block-state id.
#[derive(Default)]
pub struct LeverBehavior;

impl LeverBehavior {
    pub fn new() -> Self {
        LeverBehavior
    }
}

impl RedstoneSignalSource for LeverBehavior {
    /// `ownSignal` (verified against the reference): unconditional `15` toward every one of
    /// the six neighbours when powered — unlike the redstone torch (`TorchBehavior::
    /// weak_signal_toward`'s own direction exclusion toward its input side), a lever's weak
    /// signal carries no direction exclusion at all (PLAN-D10's own lever sentence: "weak 15
    /// toward all six neighbours when powered").
    fn weak_signal_toward(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        _towards: Direction,
    ) -> u8 {
        match decode(world, pos) {
            Some((_, _, true)) => 15,
            _ => 0,
        }
    }

    /// `getDirectSignal` (verified against the reference, `LeverBlock`/
    /// `FaceAttachedHorizontalDirectionalBlock.getConnectedDirection`): `15` only toward the
    /// lever's own mount block. Vanilla's own raw `direction` parameter there runs
    /// receiver -> lever (`SignalGetter.getDirectSignalTo`'s own call shape: `getDirectSignal
    /// (pos.below(), Direction.DOWN)` passes the direction FROM the querying receiver TO the
    /// neighbor being asked) — the opposite of this crate's own established source -> receiver
    /// `towards` convention (`direct_signal_to`'s own doc comment: it calls `direct_signal_
    /// toward(npos, d.opposite())`, translating vanilla's raw parameter by exactly one
    /// `.opposite()` at that one seam). Composing that translation with vanilla's own
    /// `getConnectedDirection(state) == direction` condition — and vanilla's own
    /// `getConnectedDirection` already answers "away from the mount," i.e.
    /// `mount_direction(..).opposite()`, for all three attachments alike (floor: `UP`,
    /// opposite of mount `DOWN`; ceiling: `DOWN`, opposite of mount `UP`; wall: `FACING`,
    /// opposite of mount `FACING.opposite()`) — yields exactly `towards == mount_direction`,
    /// never `towards == facing` as a literal, untranslated reading of `getConnectedDirection`
    /// would wrongly suggest.
    fn direct_signal_toward(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        match decode(world, pos) {
            Some((face, facing, true)) if towards == mount_direction(face, facing) => 15,
            _ => 0,
        }
    }

    fn is_signal_source(&self) -> bool {
        true
    }
}

impl BlockBehavior for LeverBehavior {
    /// Support-loss destruction (MECH-D84): pops to air the instant a shape-update arrives
    /// from the mount direction and the mount block is no longer sturdy on the face toward the
    /// lever — mirrors `TorchBehavior::on_shape_update`'s identical shape, `Full` (never
    /// `Center`) for every one of the lever's own three attachments alike (`canSurvive`'s own
    /// citation: `FaceAttachedHorizontalDirectionalBlock.canAttach` always checks
    /// `isFaceSturdy(.., FULL)`, regardless of `face`; `Center` is a floor-torch-only rule).
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        let (face, facing, _) = decode(ctx.world, pos)?;
        let mount = mount_direction(face, facing);
        if from != mount {
            return None;
        }
        let mount_pos = mount.apply(pos);
        let face_toward_lever = mount.opposite();
        if signal::is_face_sturdy(
            ctx.world,
            mount_pos,
            face_toward_lever,
            rc_physics::SupportKind::Full,
        ) {
            None
        } else {
            Some(AIR_ID)
        }
    }

    /// `pull` (MECH-D82): toggles `powered`, writes with fan-out (vanilla's own `setBlock(pos,
    /// state, 3)` — `ctx.set_block` already performs the matching automatic neighbor+shape
    /// dispatch to this lever's own six neighbours), then ALSO explicitly re-notifies both the
    /// lever's own cell and its mount cell (vanilla's own `updateNeighbours` helper — a literal,
    /// deliberately-redundant double-fire this port reproduces exactly, rather than keeping only
    /// the "new information" half: `updateNeighborsAt(pos, ..)` duplicates what `set_block`'s
    /// own fan-out already did; `updateNeighborsAt(pos.relative(front), ..)` is the genuinely
    /// new one-hop-further propagation into the mount cell's own neighbours), and queues
    /// `block.lever.click` (volume `0.3`, pitch `0.6` when now powered / `0.5` when now
    /// unpowered) excluding the acting player. `may_build: false` is a no-op (`Pass`), matching
    /// every other tier-1 `on_use` handler's identical guard.
    fn on_use(
        &self,
        ctx: &mut UseUpdateContext,
        pos: BlockPos,
        use_ctx: &UseContext,
    ) -> UseOutcome {
        if !use_ctx.may_build {
            return UseOutcome::Pass;
        }
        let Some(current) = ctx.get_block(pos) else {
            return UseOutcome::Pass;
        };
        if !is_lever_range(current.0) {
            return UseOutcome::Pass; // defensive only -- dispatch never reaches here otherwise
        }
        let (face, facing, powered) = decode_raw(current.0);
        let new_powered = !powered;
        let new_id = with_property(GenStateId(current.0), "powered", powered_str(new_powered))
            .expect("on_use: every lever (face,facing) combination has both powered values");
        let pitch = if new_powered { 0.6 } else { 0.5 };
        ctx.request_sound(SoundRequest {
            pos,
            sound: sound_event::BLOCK_LEVER_CLICK,
            source: SoundSource::Blocks,
            volume: 0.3,
            pitch,
            except_actor: true,
        });
        ctx.set_block(pos, BlockStateId(new_id.0));
        let mount_pos = mount_direction(face, facing).apply(pos);
        signal::notify_neighbor_changed_only(&mut ctx.base, pos);
        signal::notify_neighbor_changed_only(&mut ctx.base, mount_pos);
        UseOutcome::Consumed
    }
}
