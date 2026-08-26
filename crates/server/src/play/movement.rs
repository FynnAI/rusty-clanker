//! M2 field-report fix -- the minimal serverbound-movement application path M2-B05's own
//! chunk-streaming seam (`rc_scheduler::chunk_ticket::TicketManager::move_player`, that
//! function's own doc comment: "no production call site exists at M2... exposed for a
//! future mechanics blueprint") and M2-B07's own reach-check gate were both built to
//! consume, but that M1-B05's own Play-entry sequence never wired up: real-client manual
//! testing found `SetPlayerPosition`/`SetPlayerPositionAndRotation`/`SetPlayerRotation`
//! (`packets.rs`'s own doc comment on those three structs) decoded -- or, before this fix,
//! not even decoded at all -- but never applied to any authoritative player state, so every
//! downstream consumer that keyed off a player's position (reach validation, chunk
//! streaming, position persistence) stayed permanently pinned to whatever fixed/loaded
//! value it started at.
//!
//! Applies each tick, batched exactly like `PendingBlockAction` (`block_action.rs`'s own
//! established pattern): the raw client-claimed position/rotation is trusted as-is, with
//! only a basic finite-value clamp -- M3-B02's own full replay-validation/speed-check/
//! teleport-correction anti-cheat pipeline is deliberately out of scope here (M3's own job,
//! see that blueprint's Context for the real thing this stands in for).

use super::packets::{SetPlayerPosition, SetPlayerPositionAndRotation, SetPlayerRotation};

/// One decoded, not-yet-applied movement claim -- constructed by `connection.rs`'s dispatch
/// loop, consumed by `HardcodedWorld`'s own per-tick drain step. Each field is `Some` only
/// when the originating packet actually carries it (`SetPlayerRotation` carries no
/// position, for instance) and it passed its own finite-value clamp; `None` fields are left
/// untouched by the applying step, never zeroed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PendingMovementUpdate {
    pub network_entity_id: i32,
    pub position: Option<[f64; 3]>,
    pub rotation: Option<[f32; 2]>,
    pub on_ground: Option<bool>,
}

/// `Some(pos)` iff every component is finite (not NaN, not +/-infinity) -- the "basic
/// NaN/finite clamp" this M2-scope fix owes. A non-finite claim is silently discarded
/// (the field stays at its previous applied value) rather than applied or used to
/// disconnect the connection -- M3-B02's own `nan_position_disconnects_the_connection`
/// acceptance test is that blueprint's own job, not this one's.
pub fn finite_position(pos: [f64; 3]) -> Option<[f64; 3]> {
    pos.iter().all(|c| c.is_finite()).then_some(pos)
}

/// As `finite_position`, for a `[yaw, pitch]` rotation pair.
pub fn finite_rotation(rot: [f32; 2]) -> Option<[f32; 2]> {
    rot.iter().all(|c| c.is_finite()).then_some(rot)
}

impl PendingMovementUpdate {
    pub fn from_position(network_entity_id: i32, packet: SetPlayerPosition) -> Self {
        Self {
            network_entity_id,
            position: finite_position([packet.x, packet.y, packet.z]),
            rotation: None,
            on_ground: Some(packet.on_ground),
        }
    }

    pub fn from_position_and_rotation(
        network_entity_id: i32,
        packet: SetPlayerPositionAndRotation,
    ) -> Self {
        Self {
            network_entity_id,
            position: finite_position([packet.x, packet.y, packet.z]),
            rotation: finite_rotation([packet.yaw, packet.pitch]),
            on_ground: Some(packet.on_ground),
        }
    }

    pub fn from_rotation(network_entity_id: i32, packet: SetPlayerRotation) -> Self {
        Self {
            network_entity_id,
            position: None,
            rotation: finite_rotation([packet.yaw, packet.pitch]),
            on_ground: Some(packet.on_ground),
        }
    }
}

/// A live fractional world position's containing block coordinate -- floor (not
/// truncate-toward-zero) on every axis, so a negative-fractional coordinate (e.g.
/// `x == -0.5`) resolves to the correct block/chunk (`-1`, not `0`), matching vanilla's own
/// `Mth.floor` convention. Shared by `connection.rs`'s initial Play-entry chunk grid and
/// `world.rs`'s own per-tick chunk-crossing detection so the two can never disagree about
/// which chunk a given live position belongs to -- before this fix, `connection.rs`'s own
/// join-time computation used a plain `as i32` truncation, harmless only because every
/// position that ever reached it was an exact integer value (the hardcoded `SPAWN_POSITION`
/// or an as-yet-never-updated persisted default); now that a rejoining player's persisted
/// position can carry a genuine fractional part (this same fix's own persistence
/// consumer), the two call sites must agree exactly.
pub fn feet_block_pos(pos: [f64; 3]) -> rc_core::BlockPos {
    rc_core::BlockPos::new(
        pos[0].floor() as i32,
        pos[1].floor() as i32,
        pos[2].floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_position_accepts_ordinary_values() {
        assert_eq!(finite_position([1.0, -59.0, 2.5]), Some([1.0, -59.0, 2.5]));
    }

    #[test]
    fn finite_position_rejects_nan_and_infinity_in_any_component() {
        assert_eq!(finite_position([f64::NAN, 0.0, 0.0]), None);
        assert_eq!(finite_position([0.0, f64::INFINITY, 0.0]), None);
        assert_eq!(finite_position([0.0, 0.0, f64::NEG_INFINITY]), None);
    }

    #[test]
    fn finite_rotation_rejects_nan_and_infinity() {
        assert_eq!(finite_rotation([f32::NAN, 0.0]), None);
        assert_eq!(finite_rotation([0.0, f32::INFINITY]), None);
        assert_eq!(finite_rotation([12.0, -45.0]), Some([12.0, -45.0]));
    }

    #[test]
    fn feet_block_pos_floors_negative_fractional_coordinates() {
        assert_eq!(
            feet_block_pos([-0.5, -59.0, -0.001]),
            rc_core::BlockPos::new(-1, -59, -1)
        );
        assert_eq!(
            feet_block_pos([15.999, 0.0, 16.0]),
            rc_core::BlockPos::new(15, 0, 16)
        );
    }

    #[test]
    fn from_position_carries_on_ground_and_no_rotation() {
        let update = PendingMovementUpdate::from_position(
            7,
            SetPlayerPosition {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                on_ground: true,
            },
        );
        assert_eq!(update.network_entity_id, 7);
        assert_eq!(update.position, Some([1.0, 2.0, 3.0]));
        assert_eq!(update.rotation, None);
        assert_eq!(update.on_ground, Some(true));
    }

    #[test]
    fn from_position_discards_a_non_finite_claim_but_keeps_on_ground() {
        let update = PendingMovementUpdate::from_position(
            7,
            SetPlayerPosition {
                x: f64::NAN,
                y: 2.0,
                z: 3.0,
                on_ground: false,
            },
        );
        assert_eq!(update.position, None);
        assert_eq!(update.on_ground, Some(false));
    }

    #[test]
    fn from_rotation_carries_no_position() {
        let update = PendingMovementUpdate::from_rotation(
            9,
            SetPlayerRotation {
                yaw: 10.0,
                pitch: -5.0,
                on_ground: true,
            },
        );
        assert_eq!(update.position, None);
        assert_eq!(update.rotation, Some([10.0, -5.0]));
    }
}
