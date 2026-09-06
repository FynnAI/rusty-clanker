//! Button — tier 2's manual redstone input, the direct sibling of the lever (PLAN-D10/MECH-D13,
//! M4-B10). Toggled through `BlockBehavior::on_use` (MECH-D82) exactly like the lever, but a
//! button's own press is transient: `on_use` powers it and schedules its own release
//! (`ticks_to_stay_pressed`, 20 for stone/polished-blackstone, 30 for every wooden variant) via
//! `on_scheduled_tick`. Pops to air when its own mount face stops being `Full`-sturdy
//! (MECH-D84), identical to the lever's own support-loss rule. One shared, stateless instance
//! covers one button block's whole id range (`registration.rs`'s own `register_tier2_inputs`
//! call site) -- `ticks_to_stay_pressed`/the two click-sound ids/`can_be_activated_by_arrows`
//! differ per block, so (unlike the lever) one instance is needed per *block*, not per family.
//!
//! Context §C's own private per-module decode helpers (`attach_face_from_str`/
//! `facing_from_str`/`mount_direction`) are duplicated here rather than shared with
//! `lever.rs`/`torch.rs`, matching each of those modules' own existing convention of carrying
//! its own private copies.

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_registries::block_state_properties::{properties, range_of, with_property};
use rc_registries::generated_v776::block_state_properties::{BlockId, block_id};
use rc_registries::generated_v776::block_states::{BlockStateId as GenStateId, default_state};
use rc_registries::generated_v776::registries::RegistryEntryId;
use rc_registries::generated_v776::registries::sound_event;

use crate::behavior::{BlockBehavior, UpdateContext, UseContext, UseOutcome, UseUpdateContext};
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;
use crate::sound_request::{SoundRequest, SoundSource};
use crate::world_access::BlockWorldAccess;

use super::signal::{self, RedstoneSignalSource};

/// `minecraft:*_button`'s own `face` block-state property (Context §C) -- byte-for-byte the
/// same shape as `lever.rs::AttachFace`, duplicated per that module's own established
/// per-file-private convention.
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
        other => panic!("attach_face_from_str: unrecognized button face value {other:?}"),
    }
}

fn facing_from_str(s: &str) -> Direction {
    match s {
        "north" => Direction::North,
        "south" => Direction::South,
        "west" => Direction::West,
        "east" => Direction::East,
        other => panic!("facing_from_str: unrecognized button facing value {other:?}"),
    }
}

fn powered_str(powered: bool) -> &'static str {
    if powered { "true" } else { "false" }
}

/// `getConnectedDirection(state).getOpposite()` (`FaceAttachedHorizontalDirectionalBlock`,
/// Context §C) -- byte-for-byte the same derivation `lever.rs::mount_direction` already
/// documents in full.
fn mount_direction(face: AttachFace, facing: Direction) -> Direction {
    match face {
        AttachFace::Floor => Direction::Down,
        AttachFace::Ceiling => Direction::Up,
        AttachFace::Wall => facing.opposite(),
    }
}

/// `air`'s own raw id (stable by protocol convention).
const AIR_ID: BlockStateId = BlockStateId(default_state::AIR.0);

/// One button block's own immutable per-block configuration (Context §C's table).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ButtonKindConfig {
    pub ticks_to_stay_pressed: u64,
    pub can_be_activated_by_arrows: bool,
    pub click_on: RegistryEntryId,
    pub click_off: RegistryEntryId,
}

/// Context §C's own complete 14-row table, block id + config -- every id read from the
/// generated registries, never a literal. `stone_button`/`polished_blackstone_button`: 20
/// ticks, no arrows, stone sounds. The eight plain wooden types: 30 ticks, arrow-capable,
/// the shared `wooden_button` sound (no per-species sound exists for them). Cherry, bamboo and
/// the two nether woods: 30 ticks, arrow-capable, their own distinct sound each.
pub const BUTTON_BLOCKS: &[(BlockId, ButtonKindConfig)] = &[
    (
        block_id::STONE_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 20,
            can_be_activated_by_arrows: false,
            click_on: sound_event::BLOCK_STONE_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_STONE_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::POLISHED_BLACKSTONE_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 20,
            can_be_activated_by_arrows: false,
            click_on: sound_event::BLOCK_STONE_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_STONE_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::OAK_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_WOODEN_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::SPRUCE_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_WOODEN_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::BIRCH_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_WOODEN_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::JUNGLE_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_WOODEN_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::ACACIA_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_WOODEN_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::DARK_OAK_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_WOODEN_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::PALE_OAK_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_WOODEN_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::MANGROVE_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_WOODEN_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::CHERRY_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_CHERRY_WOOD_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_CHERRY_WOOD_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::BAMBOO_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_BAMBOO_WOOD_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_BAMBOO_WOOD_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::CRIMSON_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_NETHER_WOOD_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_NETHER_WOOD_BUTTON_CLICK_OFF,
        },
    ),
    (
        block_id::WARPED_BUTTON,
        ButtonKindConfig {
            ticks_to_stay_pressed: 30,
            can_be_activated_by_arrows: true,
            click_on: sound_event::BLOCK_NETHER_WOOD_BUTTON_CLICK_ON,
            click_off: sound_event::BLOCK_NETHER_WOOD_BUTTON_CLICK_OFF,
        },
    ),
];

/// Stateless (Context §B) -- one instance per button *block* (not family, unlike the lever:
/// `ticks_to_stay_pressed`/the click sounds/`can_be_activated_by_arrows` differ per block).
/// Every read decodes directly off the world's own stored block-state id.
pub struct ButtonBehavior {
    config: ButtonKindConfig,
    block: BlockId,
}

impl ButtonBehavior {
    pub fn new(config: ButtonKindConfig, block: BlockId) -> Self {
        Self { config, block }
    }

    /// `true` iff `raw` falls inside this behavior's own registered block's real generated id
    /// range -- mirrors `lever.rs::is_lever_range`'s identical defensive convention, needed
    /// here specifically because one `ButtonBehavior` instance is constructed per *block*, so
    /// each instance's own range differs.
    fn is_own_range(&self, raw: u32) -> bool {
        let range = range_of(self.block);
        (range.first.0..=range.last.0).contains(&raw)
    }

    /// The sole place this module ever interprets a button's own generated property list.
    /// Panics if `raw` is missing any of the three properties (a config-time defect: every real
    /// button id carries all three).
    fn decode_raw(&self, raw: u32) -> (AttachFace, Direction, bool) {
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

    /// `decode_raw`, applied to whatever is currently stored at `pos` -- `None` if nothing is
    /// loaded there or the stored id falls outside this behavior's own registered range.
    fn decode(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
    ) -> Option<(AttachFace, Direction, bool)> {
        let raw = world.get_block(pos)?.0;
        if !self.is_own_range(raw) {
            return None;
        }
        Some(self.decode_raw(raw))
    }

    /// `updateNeighbours` (Context §C): fans out at the button's own cell (deliberately
    /// duplicating `set_block`'s/`write_block_state`'s own fan-out -- vanilla's own literal
    /// double-fire, `lever.rs::on_use`'s own identical citation) and, one hop further, at the
    /// mount cell.
    fn update_neighbours(
        ctx: &mut UpdateContext,
        pos: BlockPos,
        face: AttachFace,
        facing: Direction,
    ) {
        signal::notify_neighbor_changed_only(ctx, pos);
        let mount_pos = mount_direction(face, facing).apply(pos);
        signal::notify_neighbor_changed_only(ctx, mount_pos);
    }

    /// `check_pressed(ctx, pos, arrow_present)` (Context §C's own restated pseudocode): the one
    /// release/re-evaluate algorithm both `on_scheduled_tick` and the (deferred, §A) arrow entry
    /// point share. `arrow_present` is always `false` at every real call site this blueprint
    /// wires -- `can_be_activated_by_arrows` alone gates nothing here; the future projectile
    /// blueprint's whole remaining change is computing that one boolean honestly.
    fn check_pressed(&self, ctx: &mut UpdateContext, pos: BlockPos, arrow_present: bool) {
        let Some(current) = ctx.get_block(pos) else {
            return;
        };
        if !self.is_own_range(current.0) {
            return;
        }
        let (face, facing, was_pressed) = self.decode_raw(current.0);
        let should_be_pressed = self.config.can_be_activated_by_arrows && arrow_present;
        if should_be_pressed != was_pressed {
            let new_id = with_property(
                GenStateId(current.0),
                "powered",
                powered_str(should_be_pressed),
            )
            .expect("check_pressed: every button state has both powered values");
            ctx.set_block(pos, BlockStateId(new_id.0));
            Self::update_neighbours(ctx, pos, face, facing);
            let sound = if should_be_pressed {
                self.config.click_on
            } else {
                self.config.click_off
            };
            ctx.request_sound(SoundRequest {
                pos,
                sound,
                source: SoundSource::Blocks,
                volume: 1.0,
                pitch: 1.0,
                except_actor: false,
            });
        }
        if should_be_pressed {
            ctx.schedule_block_tick(pos, self.config.ticks_to_stay_pressed, TickPriority::Normal);
        }
    }
}

impl RedstoneSignalSource for ButtonBehavior {
    /// `ownSignal` (Context §C): unconditional `15` toward every one of the six neighbours
    /// while `POWERED`, `0` otherwise -- no direction exclusion at all, identical in shape to
    /// the lever's own `weak_signal_toward`.
    fn weak_signal_toward(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        _towards: Direction,
    ) -> u8 {
        match self.decode(world, pos) {
            Some((_, _, true)) => 15,
            _ => 0,
        }
    }

    /// `getDirectSignal` (Context §C): `15` only toward the button's own mount block --
    /// byte-for-byte the same `towards == mount_direction(face, facing)` composition
    /// `lever.rs::direct_signal_toward`'s own doc comment already spells out in full.
    fn direct_signal_toward(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        match self.decode(world, pos) {
            Some((face, facing, true)) if towards == mount_direction(face, facing) => 15,
            _ => 0,
        }
    }

    fn is_signal_source(&self) -> bool {
        true
    }
}

impl BlockBehavior for ButtonBehavior {
    /// Support-loss destruction (MECH-D84, Context §C): pops to air the instant a shape update
    /// arrives from the mount direction and the mount block is no longer `Full`-sturdy on the
    /// face toward the button -- `Full` for all three attach faces alike, never `Center`,
    /// structurally identical to `LeverBehavior::on_shape_update`.
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        let (face, facing, _) = self.decode(ctx.world, pos)?;
        let mount = mount_direction(face, facing);
        if from != mount {
            return None;
        }
        let mount_pos = mount.apply(pos);
        let face_toward_button = mount.opposite();
        if signal::is_face_sturdy(
            ctx.world,
            mount_pos,
            face_toward_button,
            rc_physics::SupportKind::Full,
        ) {
            None
        } else {
            Some(AIR_ID)
        }
    }

    /// `useWithoutItem` (Context §C): carries no `may_build` guard at all (the reference itself
    /// has none, unlike the repeater/comparator and unlike this engine's own lever -- §J item 2)
    /// -- returns `Consumed` without effect when already pressed, else runs `press`'s own exact
    /// five-step order: `set_block` (flag 3), `update_neighbours`, `schedule_block_tick`,
    /// `request_sound` (excluding the acting player), skipping the `BLOCK_ACTIVATE` game event
    /// (§A).
    fn on_use(
        &self,
        ctx: &mut UseUpdateContext,
        pos: BlockPos,
        _use_ctx: &UseContext,
    ) -> UseOutcome {
        let Some(current) = ctx.get_block(pos) else {
            return UseOutcome::Pass;
        };
        if !self.is_own_range(current.0) {
            return UseOutcome::Pass; // defensive only -- dispatch never reaches here otherwise
        }
        let (face, facing, powered) = self.decode_raw(current.0);
        if powered {
            return UseOutcome::Consumed;
        }
        let new_id = with_property(GenStateId(current.0), "powered", "true")
            .expect("on_use: every button state has a true powered sibling");
        ctx.set_block(pos, BlockStateId(new_id.0));
        Self::update_neighbours(&mut ctx.base, pos, face, facing);
        ctx.base
            .schedule_block_tick(pos, self.config.ticks_to_stay_pressed, TickPriority::Normal);
        ctx.request_sound(SoundRequest {
            pos,
            sound: self.config.click_on,
            source: SoundSource::Blocks,
            volume: 1.0,
            pitch: 1.0,
            except_actor: true,
        });
        UseOutcome::Consumed
    }

    /// `tick` (Context §C): `check_pressed` only when `POWERED` is currently `true`.
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let Some(current) = ctx.get_block(pos) else {
            return;
        };
        if !self.is_own_range(current.0) {
            return;
        }
        let (_, _, powered) = self.decode_raw(current.0);
        if powered {
            self.check_pressed(ctx, pos, false);
        }
    }
}
