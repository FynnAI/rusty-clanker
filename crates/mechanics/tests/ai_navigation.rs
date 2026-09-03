//! M4-B03 Acceptance tests: navigation execution -- `MoveControl`/`LookControl`
//! turn-rate clamping, `JumpControl` trigger conditions, the exact `MovementIntent`
//! values these produce for known inputs, and `PathNavigation`'s own recompute-throttle
//! + stuck-detection algorithm (Context §G).

use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::ai::navigation::{
    JumpControl, MoveControl, MoveControlOperation, PathNavigation, PendingMovementIntent,
    MAX_TURN_DEGREES_PER_TICK, MOVE_CONTROL_ARRIVAL_EPSILON_SQ,
};
use rc_mechanics::ai::pathfinding::node::WalkNodeEvaluator;
use rc_mechanics::ai::rotate_towards;
use rc_mechanics::world_access::BlockWorldAccess;
use rc_messaging::Address;
use rc_registries::generated_v776::block_states::default_state;

struct FlatWorld;
impl BlockWorldAccess for FlatWorld {
    fn get_block(&self, pos: BlockPos) -> Option<rc_chunk_storage::BlockStateId> {
        let id = if pos.y == 63 {
            default_state::STONE
        } else {
            default_state::AIR
        };
        Some(rc_chunk_storage::BlockStateId(id.0))
    }
    fn set_block(&mut self, _pos: BlockPos, _state: rc_chunk_storage::BlockStateId) -> bool {
        false
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        Address::Region(rc_messaging::RegionId(0))
    }
    fn local_identity(&self) -> Address {
        Address::Region(rc_messaging::RegionId(0))
    }
}

#[test]
fn move_control_wait_produces_zero_forward() {
    let mut control = MoveControl {
        operation: MoveControlOperation::Wait,
        wanted_pos: [10.0, 64.0, 0.0],
        speed_modifier: 1.0,
    };
    let (forward, yaw, jumping) = control.tick([0.0, 64.0, 0.0], 45.0, true, 0.6, 0.6);
    assert_eq!(forward, 0.0);
    assert_eq!(yaw, 45.0);
    assert!(!jumping);
}

#[test]
fn move_control_arrival_epsilon_switches_to_zero_forward() {
    let epsilon_offset = (MOVE_CONTROL_ARRIVAL_EPSILON_SQ / 4.0).sqrt();
    let mut control = MoveControl {
        operation: MoveControlOperation::MoveTo,
        wanted_pos: [epsilon_offset, 64.0, 0.0],
        speed_modifier: 1.0,
    };
    let (forward, _yaw, jumping) = control.tick([0.0, 64.0, 0.0], 0.0, true, 0.6, 0.6);
    assert_eq!(forward, 0.0);
    assert!(!jumping);
}

#[test]
fn move_control_moving_produces_full_forward_and_turns_toward_target() {
    let mut control = MoveControl {
        operation: MoveControlOperation::MoveTo,
        wanted_pos: [10.0, 64.0, 0.0],
        speed_modifier: 1.0,
    };
    // Facing north (yaw = 180 in this project's own convention, matching
    // `atan2(dz,dx) - 90`): moving due east (+x) is a hard turn.
    let (forward, new_yaw, jumping) = control.tick([0.0, 64.0, 0.0], 180.0, true, 0.6, 0.6);
    assert_eq!(forward, 1.0);
    assert!(!jumping);
    let desired_yaw = (0.0f64).atan2(10.0).to_degrees() as f32 - 90.0;
    // Moved toward the target yaw by at most MAX_TURN_DEGREES_PER_TICK, never past it.
    let moved = angle_delta(180.0, new_yaw).abs();
    assert!(moved <= MAX_TURN_DEGREES_PER_TICK + 1e-3);
    let remaining_before = angle_delta(180.0, desired_yaw).abs();
    let remaining_after = angle_delta(new_yaw, desired_yaw).abs();
    assert!(remaining_after <= remaining_before);
}

fn angle_delta(from: f32, to: f32) -> f32 {
    let mut delta = (to - from) % 360.0;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    delta
}

#[test]
fn rotate_towards_clamps_at_max_turn_and_never_overshoots() {
    let result = rotate_towards(0.0, 200.0, MAX_TURN_DEGREES_PER_TICK);
    // The short direction toward 200 from 0 is -160 (i.e. 200 - 360); clamped to
    // exactly -MAX_TURN_DEGREES_PER_TICK.
    assert!((result - (-MAX_TURN_DEGREES_PER_TICK)).abs() < 1e-4);
}

#[test]
fn rotate_towards_reaches_target_exactly_when_within_range() {
    let result = rotate_towards(0.0, 10.0, MAX_TURN_DEGREES_PER_TICK);
    assert!((result - 10.0).abs() < 1e-4);
}

#[test]
fn pending_movement_intent_default_fields_at_m4_scope() {
    let mut control = MoveControl {
        operation: MoveControlOperation::MoveTo,
        wanted_pos: [10.0, 64.0, 0.0],
        speed_modifier: 1.0,
    };
    let (forward, yaw, jumping) = control.tick([0.0, 64.0, 0.0], 0.0, true, 0.6, 0.6);
    let intent = PendingMovementIntent(rc_physics::MovementIntent {
        strafe: 0.0,
        forward,
        yaw_degrees: yaw,
        sprinting: false,
        sneaking: false,
        jumping,
        jump_boost_amplifier: 0,
    });
    assert_eq!(intent.0.strafe, 0.0);
    assert!(!intent.0.sprinting);
    assert!(!intent.0.sneaking);
    assert!(!intent.0.jumping);
    assert_eq!(intent.0.jump_boost_amplifier, 0);
}

#[test]
fn path_navigation_recompute_is_throttled_to_every_20_ticks() {
    let world = FlatWorld;
    let evaluator = WalkNodeEvaluator;
    let mut nav = PathNavigation::default();
    let goal = Some(BlockPos::new(5, 64, 0));

    let mut ran_ticks = Vec::new();
    for tick in 1..=25u32 {
        // A fresh search is only ever attempted when there is no current path to
        // follow (Context §G) -- clearing it every call isolates this test to the
        // `recompute_cooldown_ticks` countdown itself, decoupled from that separate
        // gate (which `path_navigation_stuck_detection_clears_the_path`, below,
        // exercises together with a real, persisting path instead).
        nav.current_path = None;
        let ran = nav.tick(
            [0.0, 64.0, 0.0],
            goal,
            0.5,
            &evaluator,
            &world,
            1.95,
            1000,
        );
        if ran.is_some() {
            ran_ticks.push(tick);
        }
    }
    assert_eq!(ran_ticks, vec![1, 21]);
}

#[test]
fn path_navigation_stuck_detection_clears_the_path() {
    let world = FlatWorld;
    let evaluator = WalkNodeEvaluator;
    let mut nav = PathNavigation::default();
    let goal = Some(BlockPos::new(5, 64, 0));

    // `stuck_check_countdown` starts at 0 (the `Default` derive Context §G's own
    // struct definition uses), so the very first call performs the first stuck
    // check -- with no prior position on record, it establishes the baseline and
    // resets the 100-tick countdown rather than flagging stuck immediately -- and
    // the *next* check (100 ticks later) is the one this test exercises. Stopping
    // the instant `is_stuck` flips also avoids a subsequent tick's own recompute
    // throttle happening to hit zero and silently refilling `current_path` again
    // (the fixture's entity never actually moves, so a fresh search would succeed
    // just as easily as the first one did).
    for _ in 0..110 {
        nav.tick([0.0, 64.0, 0.0], goal, 0.5, &evaluator, &world, 1.95, 1000);
        if nav.is_stuck {
            break;
        }
    }

    assert!(nav.is_stuck);
    assert!(nav.current_path.is_none());
}

#[test]
fn jump_control_fires_when_rise_exceeds_step_height_and_clears_on_ground() {
    let mut control = MoveControl {
        operation: MoveControlOperation::MoveTo,
        wanted_pos: [0.5, 65.0, 0.0],
        speed_modifier: 1.0,
    };
    let (_forward, _yaw, jumping) = control.tick([0.0, 64.0, 0.0], 0.0, true, 0.6, 0.6);
    assert!(jumping);
    assert_eq!(control.operation, MoveControlOperation::Jumping);

    let (_forward2, _yaw2, jumping2) = control.tick([0.0, 64.0, 0.0], 0.0, true, 0.6, 0.6);
    assert!(!jumping2);
    assert_ne!(control.operation, MoveControlOperation::Jumping);
}

#[test]
fn jump_control_does_not_fire_for_a_rise_within_step_height_or_too_far_horizontally() {
    assert!(!JumpControl::should_jump(0.5, 0.1, 0.6, 0.6));
    assert!(!JumpControl::should_jump(1.0, 4.0, 0.6, 0.6));
}
