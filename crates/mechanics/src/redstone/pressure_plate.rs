//! Pressure plate family -- `BasePressurePlateBlock`'s two subclasses, `PressurePlateBlock`
//! (boolean) and `WeightedPressurePlateBlock` (analog), tier 2's entity-presence input
//! (PLAN-D10/MECH-D13, M4-B10). Unlike the button, a plate never dispatches through `on_use`
//! at all -- its trigger is `BlockBehavior::on_entity_inside` (Context §E), and its release/
//! re-evaluate path is the same `check_pressed` algorithm the button's own release shares in
//! spirit but differs from in every load-bearing detail (Context §D, restated in full on
//! `check_pressed`'s own doc comment below): a `write_block_state` (flag 2) writeback, not
//! `set_block`; `update_neighbours` targets `pos`/`pos.below()`, not the mount cell; a
//! nonzero-to-nonzero power change fans out silently.

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_physics::{Aabb, SupportKind, Vec3};
use rc_registries::block_state_properties::{properties, range_of, with_property};
use rc_registries::generated_v776::block_state_properties::{BlockId, block_id};
use rc_registries::generated_v776::block_states::BlockStateId as GenStateId;
use rc_registries::generated_v776::registries::RegistryEntryId;
use rc_registries::generated_v776::registries::sound_event;

use crate::behavior::{BlockBehavior, EntityTouch, UpdateContext};
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;
use crate::sound_request::{SoundRequest, SoundSource};
use crate::world_access::BlockWorldAccess;

/// `air`'s own raw id (stable by protocol convention) -- mirrors `button.rs`'s own identical
/// per-module constant.
const AIR_ID: BlockStateId =
    BlockStateId(rc_registries::generated_v776::block_states::default_state::AIR.0);

use super::entity_presence::{EntityClassFilter, EntityPresenceSource};
use super::signal::{self, RedstoneSignalSource};

/// Which of the two `getSignalStrength` implementations a plate uses (Context §D).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlateSignalModel {
    /// `PressurePlateBlock`: boolean `POWERED`, `15` or `0`, class chosen by `sensitivity`.
    Boolean { sensitivity: EntityClassFilter },
    /// `WeightedPressurePlateBlock`: integer `POWER`, always `AnyEntity` regardless of the
    /// block-set type's own `sensitivity` (Context §D: "class is ALWAYS Entity here, never
    /// gated on sensitivity"), ceil-scaled by `max_weight`.
    Weighted { max_weight: u32 },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PlateKindConfig {
    pub model: PlateSignalModel,
    /// `20` for every `PressurePlateBlock`, `10` for both weighted plates (Context §D).
    pub pressed_time: u64,
    pub click_on: RegistryEntryId,
    pub click_off: RegistryEntryId,
}

/// Context §D's own complete 16-row table: the 14 plain block-set types as boolean plates
/// (`stone`/`polished_blackstone` sensing only living entities; every wooden type sensing any
/// entity), plus `light_weighted` (gold, `max_weight` 15) and `heavy_weighted` (iron,
/// `max_weight` 150).
pub const PRESSURE_PLATE_BLOCKS: &[(BlockId, PlateKindConfig)] = &[
    (
        block_id::STONE_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::LivingOnly,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_STONE_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_STONE_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::POLISHED_BLACKSTONE_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::LivingOnly,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_STONE_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_STONE_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::OAK_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::SPRUCE_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::BIRCH_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::JUNGLE_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::ACACIA_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::DARK_OAK_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::PALE_OAK_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::MANGROVE_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::CHERRY_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_CHERRY_WOOD_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_CHERRY_WOOD_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::BAMBOO_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_BAMBOO_WOOD_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_BAMBOO_WOOD_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::CRIMSON_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_NETHER_WOOD_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_NETHER_WOOD_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::WARPED_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Boolean {
                sensitivity: EntityClassFilter::AnyEntity,
            },
            pressed_time: 20,
            click_on: sound_event::BLOCK_NETHER_WOOD_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_NETHER_WOOD_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::LIGHT_WEIGHTED_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Weighted { max_weight: 15 },
            pressed_time: 10,
            click_on: sound_event::BLOCK_METAL_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_METAL_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
    (
        block_id::HEAVY_WEIGHTED_PRESSURE_PLATE,
        PlateKindConfig {
            model: PlateSignalModel::Weighted { max_weight: 150 },
            pressed_time: 10,
            click_on: sound_event::BLOCK_METAL_PRESSURE_PLATE_CLICK_ON,
            click_off: sound_event::BLOCK_METAL_PRESSURE_PLATE_CLICK_OFF,
        },
    ),
];

/// `WeightedPressurePlateBlock.getSignalStrength`'s exact arithmetic (Context §D), pure and
/// directly unit-testable: `0` for `count == 0`, else `ceil(min(count, max_weight) as f32 /
/// max_weight as f32 * 15.0)` -- never `round`, never integer division.
pub fn weighted_plate_signal(count: usize, max_weight: u32) -> u8 {
    if count == 0 {
        return 0;
    }
    let clamped = count.min(max_weight as usize) as f32;
    let percent = clamped / max_weight as f32;
    (percent * 15.0).ceil() as u8
}

/// Context §D's own `TOUCH_AABB` (`Block.column(14.0, 0.0, 4.0)`): `x`/`z` from `1/16` to
/// `15/16`, `y` from `0` to `4/16`, positioned at `pos`.
fn touch_aabb(pos: BlockPos) -> Aabb {
    let x = pos.x as f64;
    let y = pos.y as f64;
    let z = pos.z as f64;
    Aabb {
        min: Vec3::new(x + 1.0 / 16.0, y, z + 1.0 / 16.0),
        max: Vec3::new(x + 15.0 / 16.0, y + 4.0 / 16.0, z + 15.0 / 16.0),
    }
}

/// Stateless per-block-type configuration plus the injected entity census (Context §B/§E) --
/// one instance per plate *block* (`registration.rs`'s own `register_tier2_inputs` loop), the
/// census shared by every one of them via `Arc` clones of the same `EntityPresenceSource`.
pub struct PressurePlateBehavior {
    config: PlateKindConfig,
    block: BlockId,
    entities: Arc<dyn EntityPresenceSource>,
}

impl PressurePlateBehavior {
    pub fn new(
        config: PlateKindConfig,
        block: BlockId,
        entities: Arc<dyn EntityPresenceSource>,
    ) -> Self {
        Self {
            config,
            block,
            entities,
        }
    }

    fn is_own_range(&self, raw: u32) -> bool {
        let range = range_of(self.block);
        (range.first.0..=range.last.0).contains(&raw)
    }

    /// Decodes the stored signal (`POWERED` as `15`/`0`, or `POWER` directly) off `raw`'s own
    /// generated property list -- the sole place this module ever interprets either property.
    fn decode_signal(&self, raw: u32) -> u8 {
        let props = properties(GenStateId(raw));
        match self.config.model {
            PlateSignalModel::Boolean { .. } => {
                let powered = props
                    .iter()
                    .find(|(name, _)| *name == "powered")
                    .map(|(_, value)| *value == "true")
                    .unwrap_or_else(|| {
                        panic!("decode_signal: raw id {raw} has no powered property")
                    });
                if powered { 15 } else { 0 }
            }
            PlateSignalModel::Weighted { .. } => props
                .iter()
                .find(|(name, _)| *name == "power")
                .and_then(|(_, value)| value.parse::<u8>().ok())
                .unwrap_or_else(|| panic!("decode_signal: raw id {raw} has no power property")),
        }
    }

    /// The new raw id `set_signal_for_state` (Context §D pseudocode) resolves to, writing only
    /// this plate's own varying property and leaving every other property of `current_raw`
    /// untouched (`with_property`'s own established contract).
    fn state_for_signal(&self, current_raw: u32, signal: u8) -> BlockStateId {
        let new_id = match self.config.model {
            PlateSignalModel::Boolean { .. } => {
                let value = if signal > 0 { "true" } else { "false" };
                with_property(GenStateId(current_raw), "powered", value)
            }
            PlateSignalModel::Weighted { .. } => {
                let value = signal.to_string();
                with_property(GenStateId(current_raw), "power", &value)
            }
        };
        BlockStateId(
            new_id
                .expect("state_for_signal: signal is always a legal property value")
                .0,
        )
    }

    /// `decode_signal`, applied to whatever is currently stored at `pos` -- `None` if nothing
    /// is loaded there or the stored id falls outside this behavior's own registered range.
    fn decode(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> Option<u8> {
        let raw = world.get_block(pos)?.0;
        if !self.is_own_range(raw) {
            return None;
        }
        Some(self.decode_signal(raw))
    }

    /// `getSignalStrength` (Context §D): boolean plates read `15`/`0` off the census, gated by
    /// `sensitivity`'s own class filter; weighted plates always query `AnyEntity` and scale by
    /// `weighted_plate_signal`, regardless of the block-set type's own sensitivity.
    fn compute_signal_strength(&self, region: Aabb) -> u8 {
        match self.config.model {
            PlateSignalModel::Boolean { sensitivity } => {
                let count = self.entities.count_entities_in(region, sensitivity);
                if count > 0 { 15 } else { 0 }
            }
            PlateSignalModel::Weighted { max_weight } => {
                let count = self
                    .entities
                    .count_entities_in(region, EntityClassFilter::AnyEntity);
                weighted_plate_signal(count, max_weight)
            }
        }
    }

    /// `check_pressed(ctx, pos, old_signal)` (Context §D's own restated pseudocode), the one
    /// algorithm both subclasses share -- three load-bearing, deliberate details:
    ///
    /// 1. `write_block_state`, not `set_block`: vanilla writes with update flag `2` (a client
    ///    update with no `updateNeighborsAt` call) and performs the neighbour fan-out itself
    ///    via its own `updateNeighbours`, so the plate follows the diode/torch writeback
    ///    convention, never the lever/button `set_block` convention (which would fan out at
    ///    `pos` twice).
    /// 2. `update_neighbours` for a plate is `pos` and `pos.below()` -- not the mount-cell rule
    ///    the button and lever use.
    /// 3. A nonzero-to-nonzero power change plays no sound (a weighted plate going 3->7 is
    ///    neither an on- nor an off-transition) but still writes and still fans out.
    fn check_pressed(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let Some(current) = ctx.get_block(pos) else {
            return;
        };
        if !self.is_own_range(current.0) {
            return;
        }
        let old_signal = self.decode_signal(current.0);
        let signal = self.compute_signal_strength(touch_aabb(pos));
        let was_pressed = old_signal > 0;
        let is_pressed = signal > 0;

        if old_signal != signal {
            let new_state = self.state_for_signal(current.0, signal);
            ctx.write_block_state(pos, new_state);
            signal::notify_neighbor_changed_only(ctx, pos);
            signal::notify_neighbor_changed_only(ctx, Direction::Down.apply(pos));
        }

        if !is_pressed && was_pressed {
            ctx.request_sound(SoundRequest {
                pos,
                sound: self.config.click_off,
                source: SoundSource::Blocks,
                volume: 1.0,
                pitch: 1.0,
                except_actor: false,
            });
        } else if is_pressed && !was_pressed {
            ctx.request_sound(SoundRequest {
                pos,
                sound: self.config.click_on,
                source: SoundSource::Blocks,
                volume: 1.0,
                pitch: 1.0,
                except_actor: false,
            });
        }

        if is_pressed {
            ctx.schedule_block_tick(pos, self.config.pressed_time, TickPriority::Normal);
        }
    }
}

impl RedstoneSignalSource for PressurePlateBehavior {
    /// `ownSignal` (Context §D): the state's own signal toward every direction.
    fn weak_signal_toward(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        _towards: Direction,
    ) -> u8 {
        self.decode(world, pos).unwrap_or(0)
    }

    /// `getDirectSignal` (Context §D): the state's own signal only for the queried direction
    /// `UP`, i.e. only into the block below the plate once translated into this crate's own
    /// `towards` convention (`towards == Direction::Down`).
    fn direct_signal_toward(
        &self,
        world: &dyn BlockWorldAccess,
        pos: BlockPos,
        towards: Direction,
    ) -> u8 {
        if towards == Direction::Down {
            self.decode(world, pos).unwrap_or(0)
        } else {
            0
        }
    }

    fn is_signal_source(&self) -> bool {
        true
    }
}

impl BlockBehavior for PressurePlateBehavior {
    /// Support-loss destruction (MECH-D84, Context §D): pops to air the instant a shape update
    /// arrives from `Down` and the block below is neither `Rigid`- nor `Center`-sturdy on its
    /// own top face -- an OR of two support kinds, not `Rigid` alone.
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        self.decode(ctx.world, pos)?;
        if from != Direction::Down {
            return None;
        }
        let below = Direction::Down.apply(pos);
        let rigid = signal::is_face_sturdy(ctx.world, below, Direction::Up, SupportKind::Rigid);
        let center = signal::is_face_sturdy(ctx.world, below, Direction::Up, SupportKind::Center);
        if rigid || center { None } else { Some(AIR_ID) }
    }

    /// `tick` (Context §D): `check_pressed` only when the stored signal is `> 0` -- the
    /// release/re-evaluate path.
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let Some(current) = ctx.get_block(pos) else {
            return;
        };
        if !self.is_own_range(current.0) {
            return;
        }
        if self.decode_signal(current.0) > 0 {
            self.check_pressed(ctx, pos);
        }
    }

    /// `entityInside` (Context §D/§E): `check_pressed` only when the stored signal is exactly
    /// `0` -- the press path. `_entity`'s own touch details are irrelevant to a plate: its
    /// signal is always recomputed fresh off the whole census, never off the one entity whose
    /// movement triggered this particular dispatch (so multiple entities entering the same
    /// tick redundantly, harmlessly, re-run this same check while the stored signal is still
    /// `0`).
    fn on_entity_inside(&self, ctx: &mut UpdateContext, pos: BlockPos, _entity: &EntityTouch) {
        let Some(current) = ctx.get_block(pos) else {
            return;
        };
        if !self.is_own_range(current.0) {
            return;
        }
        if self.decode_signal(current.0) == 0 {
            self.check_pressed(ctx, pos);
        }
    }
}
