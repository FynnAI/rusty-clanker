//! Hopper — transfer semantics, restated exactly (Context: "Hopper — transfer semantics,
//! restated exactly"). Cross-region hopper chains, hopper minecarts, and item-entity
//! collection are explicitly out of scope (Constraints (g)).

use std::collections::HashSet;

use bevy_ecs::prelude::Component;
use rc_chunk_storage::ItemStackRecord;
use rc_core::BlockPos;
use rc_nbt::schema::{NbtCompoundExt, NbtPath, SchemaError};
use rc_nbt::{borrow, owned};

use crate::block_entity::BlockEntityWorldAccess;
use crate::container::ItemMaxStackSize;
use crate::direction::Direction;

pub const HOPPER_SLOT_COUNT: usize = 5;
pub const ALL_HOPPER_SLOTS: [usize; HOPPER_SLOT_COUNT] = [0, 1, 2, 3, 4];

#[derive(Component, Clone, Debug, PartialEq)]
pub struct HopperBlockEntity {
    pub slots: [Option<ItemStackRecord>; HOPPER_SLOT_COUNT],
    pub transfer_cooldown: u8,
    /// One of `{Down, North, South, East, West}` — never `Up` (Context; not structurally
    /// enforced, restated as a caller invariant).
    pub facing: Direction,
    pub custom_name: Option<String>,
    pub lock: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HopperTickOutcome {
    OnCooldown,
    Locked,
    Pushed,
    Pulled,
    Idle,
}

impl HopperBlockEntity {
    pub fn empty(facing: Direction) -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            transfer_cooldown: 0,
            facing,
            custom_name: None,
            lock: None,
        }
    }

    /// Context's own binding pseudocode, implemented exactly (cooldown gate, lock gate,
    /// push-then-pull, the 8/7-tick cooldown rule, furnace-face-aware insertion/extraction via
    /// `TierOneContainer`). `pos` is this hopper's own absolute position (needed to compute
    /// `facing.apply(pos)`/`Direction::Up.apply(pos)` and to query `world.is_locked_by_redstone`).
    ///
    /// **Field-report correction to the blueprint's own literal pseudocode:** `insertable_slots`'s
    /// `from_above` argument is computed here as `pos.y > push_target_pos.y` — the *hopper's*
    /// own Y is the greater one when it sits above the destination. The blueprint's own text
    /// gives the inverted `push_target_pos.y > self.pos.y`, which resolves to `false` for a
    /// hopper directly above a furnace (the exact "coal on the side, ore on top" auto-smelter
    /// case Context itself names), routing the push to the fuel slot instead of the input slot.
    ///
    /// **Field-report correction to the blueprint's own literal cooldown-gate pseudocode**
    /// (`docs/findings-for-planning.md`'s own hopper-cadence entry, verified against the real
    /// oracle via `redstone/clock/hopper_clock_basic`): vanilla decrements `cooldownTime`
    /// *unconditionally* every tick and re-checks the *post*-decrement value the very same
    /// call — a cooldown that reaches `0` this tick attempts its transfer this same tick, not
    /// the next one. The blueprint's own literal pseudocode instead gates the decrement itself
    /// on the *pre*-decrement value and returns immediately whenever it fired, never re-checking
    /// post-decrement within the same call — silently adding one whole extra idle tick after
    /// every cooldown, confirmed to reproduce `hopper_clock_basic`'s exact 21-mismatch pattern
    /// (periodic 7-tick windows) when combined with the still-modeled-here `7`/`8` push-into-
    /// empty split below.
    ///
    /// **Field-report correction, same entry:** "if either transfer succeeded → `setCooldown(8)`"
    /// sets the *acting* (pushing/pulling) hopper's own cooldown, always, regardless of whether
    /// the destination was empty — the blueprint's own pseudocode instead read the 7-tick
    /// "into empty" exception onto the *source's* own cooldown, which is not what it is: that
    /// exception is a distinct, destination-side effect (below), triggered only when the
    /// destination is itself a hopper.
    pub fn tick(
        &mut self,
        pos: BlockPos,
        world: &mut dyn BlockEntityWorldAccess,
        max_stack: &dyn ItemMaxStackSize,
        already_ticked_hoppers: &HashSet<BlockPos>,
    ) -> HopperTickOutcome {
        if self.transfer_cooldown > 0 {
            self.transfer_cooldown -= 1;
        }
        if self.transfer_cooldown > 0 {
            return HopperTickOutcome::OnCooldown;
        }
        if world.is_locked_by_redstone(pos) {
            return HopperTickOutcome::Locked;
        }

        // 1. PUSH -- attempted first.
        let push_target_pos = self.facing.apply(pos);
        let mut pushed = false;
        let mut pushed_into_empty = false;
        if let Some(destination) = world.container_at_mut(push_target_pos)
            && let Some(src_slot) =
                crate::container::find_leftmost_extract_slot(&self.slots, &ALL_HOPPER_SLOTS)
        {
            let item_id = self.slots[src_slot].as_ref().unwrap().id.clone();
            let cap = max_stack.max_stack_size(&item_id).min(64);
            let destination_was_empty = destination.slots().iter().all(Option::is_none);
            let insertable = destination.insertable_slots(pos.y > push_target_pos.y);
            if let Some(dst_slot) = crate::container::find_leftmost_insert_slot(
                destination.slots(),
                &item_id,
                cap,
                &insertable,
            ) {
                crate::container::move_one_item(
                    &mut self.slots,
                    src_slot,
                    destination.slots_mut(),
                    dst_slot,
                );
                pushed = true;
                pushed_into_empty = destination_was_empty;
            }
        }
        if pushed {
            // The acting hopper's own cooldown is always 8 on a successful transfer (Context's
            // own "if either transfer succeeded -> setCooldown(8)", unconditional).
            self.transfer_cooldown = 8;
            if pushed_into_empty {
                // Chained-hopper quirk (Context): pushing into an *empty* hopper additionally
                // seeds *that* hopper's own cooldown — 7 if it already ticked earlier this same
                // game tick (this push landing on it only after its own tick already ran this
                // pass), else 8. `container_at_mut`'s own borrow above has already ended by
                // this point, so this second, position-keyed lookup is a fresh one, not an
                // aliasing one — only a hopper destination carries a cooldown to seed at all;
                // `get_hopper_mut` returns `None` for a chest/furnace destination and this is a
                // no-op.
                if let Some(dest_hopper) = world.get_hopper_mut(push_target_pos) {
                    dest_hopper.transfer_cooldown =
                        if already_ticked_hoppers.contains(&push_target_pos) {
                            7
                        } else {
                            8
                        };
                }
            }
            return HopperTickOutcome::Pushed;
        }

        // 2. PULL -- only reached if push did not succeed.
        let above_pos = Direction::Up.apply(pos);
        if let Some(source) = world.container_at_mut(above_pos) {
            let extractable = source.extractable_slots();
            if let Some(src_slot) =
                crate::container::find_leftmost_extract_slot(source.slots(), &extractable)
            {
                let item_id = source.slots()[src_slot].as_ref().unwrap().id.clone();
                let cap = max_stack.max_stack_size(&item_id).min(64);
                if let Some(dst_slot) = crate::container::find_leftmost_insert_slot(
                    &self.slots,
                    &item_id,
                    cap,
                    &ALL_HOPPER_SLOTS,
                ) {
                    crate::container::move_one_item(
                        source.slots_mut(),
                        src_slot,
                        &mut self.slots,
                        dst_slot,
                    );
                    // Pulling never gets the 7-tick "into empty" exception -- documented as an
                    // ejection/pushing-side behavior only (Context).
                    self.transfer_cooldown = 8;
                    return HopperTickOutcome::Pulled;
                }
            }
        }

        // 3. item-entity collection -- out of scope (M4, Context).
        HopperTickOutcome::Idle
    }

    pub fn comparator_signal(&self, max_stack: &dyn ItemMaxStackSize) -> u8 {
        crate::container::comparator_signal_from_slots(&self.slots, max_stack)
    }

    pub fn to_nbt(&self, pos: BlockPos) -> owned::NbtCompound {
        let mut out = owned::NbtCompound::new();
        out.insert("id", "minecraft:hopper");
        out.insert("x", pos.x);
        out.insert("y", pos.y);
        out.insert("z", pos.z);
        out.insert("Items", crate::item_stack::slots_to_items_list(&self.slots));
        out.insert("TransferCooldown", self.transfer_cooldown as i32);
        out.insert("RCFacing", direction_to_byte(self.facing));
        if let Some(name) = &self.custom_name {
            out.insert("CustomName", name.as_str());
        }
        if let Some(lock) = &self.lock {
            out.insert("Lock", lock.as_str());
        }
        out
    }

    /// `facing` is not itself part of vanilla's own block-*entity* NBT (it is a block*state*
    /// property, per `07-blocks-blockstates.md`'s own state-vs-entity split) — this blueprint's
    /// own `to_nbt`/`from_nbt` write/read it as a convenience extra field (`RCFacing: Byte`,
    /// this blueprint's own non-vanilla tag, clearly namespaced so it never collides with a
    /// real vanilla tag name) purely so this blueprint's own hopper struct round-trips
    /// completely without needing the not-yet-existing real blockstate-property NBT
    /// integration a future blueprint supplies.
    pub fn from_nbt(
        compound: &borrow::NbtCompound<'_, '_>,
    ) -> Result<(BlockPos, Self), SchemaError> {
        let path = NbtPath::root();
        let x = compound.require_int(&path, "x")?;
        let y = compound.require_int(&path, "y")?;
        let z = compound.require_int(&path, "z")?;
        let pos = BlockPos::new(x, y, z);

        let mut slots: [Option<ItemStackRecord>; HOPPER_SLOT_COUNT] = std::array::from_fn(|_| None);
        crate::item_stack::items_list_from_nbt(compound, &path, &mut slots)?;

        let transfer_cooldown = compound.require_int(&path, "TransferCooldown")? as u8;
        let facing_byte = compound.require_byte(&path, "RCFacing")?;
        let facing = direction_from_byte(facing_byte, &path)?;
        let custom_name = compound
            .string("CustomName")
            .map(|s| s.to_str().into_owned());
        let lock = compound.string("Lock").map(|s| s.to_str().into_owned());

        Ok((
            pos,
            Self {
                slots,
                transfer_cooldown,
                facing,
                custom_name,
                lock,
            },
        ))
    }
}

fn direction_to_byte(d: Direction) -> i8 {
    match d {
        Direction::West => 0,
        Direction::East => 1,
        Direction::North => 2,
        Direction::South => 3,
        Direction::Down => 4,
        Direction::Up => 5,
    }
}

fn direction_from_byte(b: i8, path: &NbtPath) -> Result<Direction, SchemaError> {
    match b {
        0 => Ok(Direction::West),
        1 => Ok(Direction::East),
        2 => Ok(Direction::North),
        3 => Ok(Direction::South),
        4 => Ok(Direction::Down),
        5 => Ok(Direction::Up),
        other => Err(SchemaError::InvalidValue {
            path: path.clone(),
            field: "RCFacing",
            reason: format!("unrecognized direction byte {other}"),
        }),
    }
}

impl crate::container::TierOneContainer for HopperBlockEntity {
    fn slots(&self) -> &[Option<ItemStackRecord>] {
        &self.slots
    }
    fn slots_mut(&mut self) -> &mut [Option<ItemStackRecord>] {
        &mut self.slots
    }
}

/// M3.5-B05 (WORLD-D6): a thin wrapper over `to_nbt`/`from_nbt` above, exposing them
/// through `rc-chunk-storage`'s generic persistence contract.
impl rc_chunk_storage::BlockEntityCodec for HopperBlockEntity {
    fn to_record(&self, pos: BlockPos) -> rc_chunk_storage::BlockEntityRecord {
        rc_chunk_storage::BlockEntityRecord {
            pos,
            id: "minecraft:hopper".to_string(),
            data: self.to_nbt(pos),
        }
    }

    fn from_record(
        record: &rc_chunk_storage::BlockEntityRecord,
    ) -> Result<Self, rc_chunk_storage::BlockEntityCodecError> {
        let bytes = rc_nbt::write_owned(&rc_nbt::owned::BaseNbt::new("", record.data.clone()));
        let nbt = rc_nbt::read_borrowed_strict(&bytes)?;
        let base = match nbt {
            rc_nbt::borrow::Nbt::Some(base) => base,
            rc_nbt::borrow::Nbt::None => {
                return Err(rc_nbt::NbtError::from(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "BlockEntityRecord::data round-tripped to an empty NBT document",
                ))
                .into());
            }
        };
        let compound = base.as_compound();
        let (_pos, value) = Self::from_nbt(&compound)?;
        Ok(value)
    }
}
