//! `minecraft:redstone_block` — a constant, always-on redstone signal source (M3 field-report
//! fix: "nearly every remaining failing contraption is triggered by a redstone_block that
//! currently emits nothing — it is registered as a `RedstoneSignalSource` nowhere").
//!
//! blocks.json's own `minecraft:redstone_block` entry (protocol 776): `definition.type:
//! "minecraft:powered"`, no properties, exactly one state id (`11311`, also `default: true`) —
//! this block is never rewritten and never varies, so there is nothing per-position to track.
//! Real vanilla `RedStoneBlock` overrides `isSignalSource` to unconditionally `true` and both
//! `getSignal`/`getDirectSignal` to unconditionally `15`, regardless of the queried direction or
//! any world state — a constant strength-15 source on all six faces, both weak (what a
//! non-conductor neighbor, e.g. adjacent wire, reads) and strong/direct (what a conductor
//! resting against it reads, letting quasi-connectivity carry the signal one hop further) —
//! unlike torch/wire/diode, this output never depends on `pos`, `towards`, or any internal
//! state, so this is a stateless zero-sized `RedstoneSignalSource`, not a `BlockBehavior` (it
//! never reacts to a neighbor change, shape update, or scheduled tick — nothing ever varies).

use rc_core::BlockPos;

use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;

use super::signal::RedstoneSignalSource;

/// The always-on power source `minecraft:redstone_block` is (module doc comment). Zero-sized —
/// every method ignores its arguments except where the trait signature requires them.
pub struct RedstoneBlockSource;

impl RedstoneSignalSource for RedstoneBlockSource {
    fn weak_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        _pos: BlockPos,
        _towards: Direction,
    ) -> u8 {
        15
    }
    fn direct_signal_toward(
        &self,
        _world: &dyn BlockWorldAccess,
        _pos: BlockPos,
        _towards: Direction,
    ) -> u8 {
        15
    }
    fn is_signal_source(&self) -> bool {
        true
    }
    // `connects_from`/`is_diode`/`raw_wire_power`/`diode_facing` all keep their shared
    // defaults: a redstone block is a plain signal source for wire-connectivity purposes
    // (`connects_from`'s own default already answers `true` for any `is_signal_source`, correct
    // here — a wire tile visually connects to an adjacent redstone block exactly as it does to
    // an adjacent torch), never a diode, and never itself wire.
}
