//! Server-authoritative player movement (M3-B02, MECH-D62 supersession of M2-B07's fixed
//! `SPAWN_POSITION` reach-check input): `PlayerMotion`/`TeleportState` component state,
//! per-tick movement-report merging, and `evaluate_movement`'s replay-validation/speed-
//! check/teleport-correction pipeline (Context: "Server-side movement processing -- the
//! exact reactive model"). Supersedes the M2 field-report fix's own prior, minimal
//! decode-and-apply path (raw claimed position/rotation, basic finite-value clamp only, no
//! collision replay, no anti-cheat) that used to live in this file; `feet_block_pos` is the
//! one piece of that prior content still needed elsewhere in this crate (`connection.rs`'s
//! Play-entry chunk grid, `world.rs`'s per-tick chunk-crossing detection) and is kept,
//! unmodified in shape, below.

use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, RegistryId, WORLD_HEIGHT, WORLD_MIN_Y};
use rc_core::{BlockPos, DimensionId};
use rc_physics::collide::{collide_and_slide, has_new_collision, overlaps_any_solid};
use rc_physics::{
    BlockPhysicsProperties, BlockShapeSource, PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT,
    STEP_HEIGHT, Vec3, tier1_shape_table,
};

use super::block_action::ChunkIndex;

/// 14-physics-collision.md §5. See Context, "Server-side movement validation".
pub const SPEED_CHECK_THRESHOLD: f64 = 100.0;
pub const MISMATCH_TOLERANCE_SQ: f64 = 0.0625;
pub const POSITION_CLAMP_HORIZONTAL: f64 = 3.0e7;
pub const POSITION_CLAMP_VERTICAL: f64 = 2.0e7;

/// Per-player persistent physics state (Context: "Which pipeline stage" -- this crate's own,
/// deliberately not `rc-mechanics`, per Context's own architectural note). Spawned at join
/// with `velocity = Vec3::ZERO, on_ground = true` and `position`/`yaw`/`pitch` from the
/// player's own just-loaded (or freshly defaulted) persisted record -- `SPAWN_POSITION`'s
/// own value for a brand-new player, a real prior position for a rejoin (this crate's own
/// already-established M2 persistence path, `world.rs`'s join-drain step).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlayerMotion {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub fall_distance: f64,
}

/// Teleport/correction acknowledgment state (Context: "Teleport / position-sync protocol").
/// `next_teleport_id` starts at `2` (M1-B05's own join flow already consumed `1`).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct TeleportState {
    pub awaiting_teleport_id: Option<i32>,
    pub next_teleport_id: i32,
}

/// This tick's coalesced, per-field-"last write wins" decode of every movement packet a
/// player sent (Context: "Which pipeline stage", step 1). Cleared to `PendingMoveReport::
/// default()` after each tick's Stage-6b-equivalent step consumes it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PendingMoveReport {
    pub position: Option<Vec3>,
    pub rotation: Option<(f32, f32)>,
    pub on_ground: Option<bool>,
    pub confirm_teleport_id: Option<i32>,
}

/// One decoded, not-yet-applied movement packet -- queued by `enter_play`'s dispatch loop,
/// consumed by `HardcodedWorld`'s own manual drain step (mirrors `PendingBlockAction`,
/// M2-B07).
pub struct PendingMovementPacket {
    pub network_entity_id: i32,
    pub report: PendingMoveReport,
}

/// The outcome `evaluate_movement` produces (Context, full algorithm) -- consumed by the
/// tick-loop caller to decide whether to issue a `SynchronizePlayerPosition` correction or
/// close the connection.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MovementOutcome {
    NoPositionClaim,
    IgnoredAwaitingTeleport,
    Accepted,
    RejectSpeed,
    RejectMismatch,
    Disconnect,
}

/// Merges one packet's decoded fields into `report`, per-field "last write wins" (Context):
/// for each `Option` field on `incoming`, if `Some`, overwrites that same field on `report`
/// -- never touches a field `incoming` leaves `None`.
pub fn merge_move_report(report: &mut PendingMoveReport, incoming: &PendingMoveReport) {
    todo!()
}

/// `POSITION_CLAMP_HORIZONTAL`/`_VERTICAL` (Context: "Server-side movement validation").
pub fn clamp_position(pos: Vec3) -> Vec3 {
    todo!()
}

/// Allocates the next teleport id and marks it awaited (Context: "Issuing a correction").
/// `motion` itself is left untouched -- the player's own last-known-good position/velocity
/// stay put until the ack arrives; the caller (`respond_to_movement`, `world.rs`) sends the
/// matching `SynchronizePlayerPosition` using that still-unchanged `motion`.
fn issue_correction(teleport: &mut TeleportState) {
    todo!()
}

/// Context: "Server-side movement processing" -- the full algorithm. `motion`/`teleport` are
/// mutated in place; `report` is consumed (read-only).
pub fn evaluate_movement(
    motion: &mut PlayerMotion,
    teleport: &mut TeleportState,
    report: &PendingMoveReport,
    shapes: &dyn BlockShapeSource,
) -> MovementOutcome {
    todo!()
}

/// Bridges `rc_chunk_storage::BlockStateColumn` + `rc_physics::tier1_shape_table()` into a
/// `BlockShapeSource` (Context: "Unloaded-position policy" -- returns `BlockPhysicsProperties
/// ::air()` for any position outside `index`'s coverage, matching this policy's own "a player
/// who walks past the edge of the currently-loaded... grid falls through open air rather than
/// being blocked by an invisible wall" rule; extended here, for the same reason, to any
/// position outside the world's own vertical bounds). Borrows the region `World` and
/// `ChunkIndex` for the duration of one Stage-6b-equivalent step; never stored beyond that.
pub struct ChunkBlockShapeSource<'w> {
    pub world: &'w World,
    pub index: &'w ChunkIndex,
    pub dimension: DimensionId,
}

impl BlockShapeSource for ChunkBlockShapeSource<'_> {
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties {
        todo!()
    }
}

/// `eye_position(motion.position)` -- the player's real eye position (Context, MECH-D62
/// supersession note; Interfaces). `PLAYER_EYE_HEIGHT` reused unmodified from `M2-B07`.
pub fn eye_position(position: Vec3) -> Vec3 {
    todo!()
}

/// A live fractional world position's containing block coordinate -- floor (not
/// truncate-toward-zero) on every axis, so a negative-fractional coordinate (e.g.
/// `x == -0.5`) resolves to the correct block/chunk (`-1`, not `0`), matching vanilla's own
/// `Mth.floor` convention. Shared by `connection.rs`'s initial Play-entry chunk grid and
/// `world.rs`'s own per-tick chunk-crossing detection so the two can never disagree about
/// which chunk a given live position belongs to.
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
    fn merge_move_report_leaves_unset_fields_untouched() {
        let mut report = PendingMoveReport {
            position: Some(Vec3::new(1.0, 2.0, 3.0)),
            rotation: None,
            on_ground: Some(true),
            confirm_teleport_id: None,
        };
        merge_move_report(
            &mut report,
            &PendingMoveReport {
                position: None,
                rotation: Some((10.0, -5.0)),
                on_ground: None,
                confirm_teleport_id: None,
            },
        );
        assert_eq!(report.position, Some(Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(report.rotation, Some((10.0, -5.0)));
        assert_eq!(report.on_ground, Some(true));
    }

    #[test]
    fn merge_move_report_last_write_wins_when_both_set() {
        let mut report = PendingMoveReport {
            position: Some(Vec3::new(1.0, 2.0, 3.0)),
            ..Default::default()
        };
        merge_move_report(
            &mut report,
            &PendingMoveReport {
                position: Some(Vec3::new(9.0, 9.0, 9.0)),
                ..Default::default()
            },
        );
        assert_eq!(report.position, Some(Vec3::new(9.0, 9.0, 9.0)));
    }

    #[test]
    fn clamp_position_clamps_each_axis_independently() {
        let clamped = clamp_position(Vec3::new(-4.0e7, 3.0e7, 4.0e7));
        assert_eq!(clamped.x, -POSITION_CLAMP_HORIZONTAL);
        assert_eq!(clamped.y, POSITION_CLAMP_VERTICAL);
        assert_eq!(clamped.z, POSITION_CLAMP_HORIZONTAL);
    }

    struct EmptyShapes;
    impl BlockShapeSource for EmptyShapes {
        fn properties_at(&self, _pos: BlockPos) -> BlockPhysicsProperties {
            BlockPhysicsProperties::air()
        }
    }

    fn fresh_motion() -> PlayerMotion {
        PlayerMotion {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: true,
            fall_distance: 0.0,
        }
    }

    fn fresh_teleport() -> TeleportState {
        TeleportState {
            awaiting_teleport_id: None,
            next_teleport_id: 2,
        }
    }

    #[test]
    fn evaluate_movement_accepts_a_small_in_range_move() {
        let mut motion = fresh_motion();
        let mut teleport = fresh_teleport();
        let outcome = evaluate_movement(
            &mut motion,
            &mut teleport,
            &PendingMoveReport {
                position: Some(Vec3::new(0.1, 0.0, 0.0)),
                on_ground: Some(true),
                ..Default::default()
            },
            &EmptyShapes,
        );
        assert_eq!(outcome, MovementOutcome::Accepted);
        assert_eq!(motion.position, Vec3::new(0.1, 0.0, 0.0));
        assert_eq!(teleport.awaiting_teleport_id, None);
    }

    #[test]
    fn evaluate_movement_rejects_a_speed_violation_and_issues_a_correction() {
        let mut motion = fresh_motion();
        let mut teleport = fresh_teleport();
        let outcome = evaluate_movement(
            &mut motion,
            &mut teleport,
            &PendingMoveReport {
                position: Some(Vec3::new(5000.0, 0.0, 0.0)),
                on_ground: Some(true),
                ..Default::default()
            },
            &EmptyShapes,
        );
        assert_eq!(outcome, MovementOutcome::RejectSpeed);
        assert_eq!(motion.position, Vec3::ZERO, "rejected motion stays put");
        assert_eq!(teleport.awaiting_teleport_id, Some(2));
    }

    #[test]
    fn evaluate_movement_disconnects_on_a_non_finite_claim() {
        let mut motion = fresh_motion();
        let mut teleport = fresh_teleport();
        let outcome = evaluate_movement(
            &mut motion,
            &mut teleport,
            &PendingMoveReport {
                position: Some(Vec3::new(f64::NAN, 0.0, 0.0)),
                ..Default::default()
            },
            &EmptyShapes,
        );
        assert_eq!(outcome, MovementOutcome::Disconnect);
    }

    #[test]
    fn evaluate_movement_ignores_position_claims_while_awaiting_a_teleport_ack() {
        let mut motion = fresh_motion();
        let mut teleport = TeleportState {
            awaiting_teleport_id: Some(2),
            next_teleport_id: 3,
        };
        let outcome = evaluate_movement(
            &mut motion,
            &mut teleport,
            &PendingMoveReport {
                position: Some(Vec3::new(0.1, 0.0, 0.0)),
                ..Default::default()
            },
            &EmptyShapes,
        );
        assert_eq!(outcome, MovementOutcome::IgnoredAwaitingTeleport);
        assert_eq!(motion.position, Vec3::ZERO);
    }

    #[test]
    fn evaluate_movement_clears_the_awaiting_teleport_id_on_a_matching_confirmation() {
        let mut motion = fresh_motion();
        let mut teleport = TeleportState {
            awaiting_teleport_id: Some(2),
            next_teleport_id: 3,
        };
        let outcome = evaluate_movement(
            &mut motion,
            &mut teleport,
            &PendingMoveReport {
                confirm_teleport_id: Some(2),
                ..Default::default()
            },
            &EmptyShapes,
        );
        assert_eq!(outcome, MovementOutcome::NoPositionClaim);
        assert_eq!(teleport.awaiting_teleport_id, None);
    }

    #[test]
    fn eye_position_adds_player_eye_height() {
        let eye = eye_position(Vec3::new(1.0, -59.0, 2.0));
        assert_eq!(eye, Vec3::new(1.0, -59.0 + PLAYER_EYE_HEIGHT, 2.0));
    }
}
