//! `restart_persistence`'s own pure-function self-tests (Acceptance tests) — no
//! network/process involved; `apply_actions`/`observe_state` are exercised only by a
//! real `m2-report` run (Context).
//!
//! M3 field-report fix (DEFECT 3): `aim_geometry` below hand-verifies `click_aim_point`'s own
//! aim-point geometry against a real `rc_physics::cast_ray` — the exact algorithm the server's
//! own MECH-D62 raycast (`crates/server/src/play/mining.rs::raycast_reach`) runs — instead of
//! trusting the module's own doc-comment derivation alone.

use rc_paritybot::restart_persistence::{ActionError, compare_state, expected_state};

#[test]
fn matching_state_produces_no_mismatches() {
    assert!(compare_state(&expected_state(), &expected_state()).is_empty());
}

#[test]
fn wrong_block_state_is_reported() {
    let mut actual = expected_state();
    actual.blocks[0].1 = actual.blocks[0].1.wrapping_add(1);
    let mismatches = compare_state(&expected_state(), &actual);
    assert_eq!(mismatches.len(), 1, "got {mismatches:?}");
}

#[test]
fn wrong_health_is_reported() {
    let mut actual = expected_state();
    actual.health = 19.0;
    let mismatches = compare_state(&expected_state(), &actual);
    assert_eq!(mismatches.len(), 1, "got {mismatches:?}");
    assert!(mismatches[0].contains("health"));
}

#[test]
fn multiple_mismatches_are_all_reported_independently() {
    let mut actual = expected_state();
    actual.blocks[0].1 = actual.blocks[0].1.wrapping_add(1);
    actual.blocks[2].1 = actual.blocks[2].1.wrapping_add(1);
    let mismatches = compare_state(&expected_state(), &actual);
    assert_eq!(mismatches.len(), 2, "got {mismatches:?}");
}

/// DEFECT 3's own second half, "close that hole": `ActionError::ActionRejected`'s own
/// `Display` message must actually name the failed action and position — this is the message
/// `restart_persistence_runner`'s `RESULT=ERROR`/`MESSAGE=` lines (and, downstream,
/// `xtask::m2_report`'s own `Fail` case detail) surface, so a silent/uninformative message
/// here would defeat the whole point of failing loudly.
#[test]
fn action_rejected_error_names_the_action_position_and_ids() {
    let err = ActionError::ActionRejected {
        action: "place",
        pos: rc_core::BlockPos::new(3, -59, 0),
        expected: 4321,
        observed: 0,
    };
    let message = err.to_string();
    assert!(message.contains("place"), "{message}");
    assert!(message.contains("4321"), "{message}");
    assert!(message.contains("BlockPos"), "{message}");
}

/// Hand-verifies `click_aim_point`'s own aim-point geometry (DEFECT 3) against a *real*
/// `rc_physics::cast_ray` — `crates/physics/src/raycast.rs`'s own DDA implementation, the
/// exact algorithm `crates/server/src/play/mining.rs::raycast_reach` runs server-side — rather
/// than trusting the hand derivation in `restart_persistence.rs`'s own doc comments alone.
mod aim_geometry {
    use std::collections::HashMap;

    use rc_core::BlockPos;
    use rc_paritybot::restart_persistence::{CLICK_AIM_INSET, click_aim_point};
    use rc_physics::{
        BlockPhysicsProperties, BlockShapeSource, PLAYER_EYE_HEIGHT, Vec3, cast_ray, mth_cos,
        mth_sin, tier1_shape_table,
    };
    use rc_registries::generated_v776::block_states::default_state::{AIR, GRASS_BLOCK, STONE};

    /// `crates/server/src/play/mining.rs::BLOCK_INTERACTION_RANGE_CREATIVE` — restated (that
    /// constant is `mod`-private to `rusty-clanker-server`) since every scripted action in
    /// this script runs under M1-B05's own hardcoded Creative default (`GameModeState {
    /// instabuild: true }`, `world.rs`'s join-drain step).
    const CREATIVE_RANGE: f64 = 5.0;

    /// The 5 scripted clicked positions, in `apply_actions_inner`'s own script order —
    /// restated here purely as data (this crate cannot reuse `xtask::m2_report::
    /// EXPECTED_BLOCKS`, the reverse direction of the identical restatement that module's own
    /// doc comment already explains; `xtask` never depends on `rc-paritybot` either).
    const SCRIPTED_CLICKS: [(i32, i32, i32); 5] = [
        (3, -60, 0),
        (2, -60, 0),
        (2, -60, 1),
        (0, -60, 0),
        (1, -60, 0),
    ];

    /// A minimal `BlockShapeSource`: M1-B05's superflat layer table restricted to the one
    /// layer (`y == -60`, solid grass) any ray in this test could ever actually reach — every
    /// ray here originates well above it and stops at its first hit, so the deeper bedrock/
    /// dirt layers this test never exercises are intentionally omitted — plus explicit
    /// per-position overrides for this script's own place/break effects, applied as the test
    /// progresses through `SCRIPTED_CLICKS` exactly like `apply_actions_inner`'s real script
    /// does.
    struct SuperflatWithOverrides {
        overrides: HashMap<(i32, i32, i32), u32>,
    }

    impl SuperflatWithOverrides {
        fn new() -> Self {
            Self {
                overrides: HashMap::new(),
            }
        }

        fn set(&mut self, pos: BlockPos, raw: u32) {
            self.overrides.insert((pos.x, pos.y, pos.z), raw);
        }
    }

    impl BlockShapeSource for SuperflatWithOverrides {
        fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties {
            let raw = self
                .overrides
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or(if pos.y == -60 { GRASS_BLOCK.0 } else { AIR.0 });
            tier1_shape_table().lookup(raw)
        }
    }

    /// Mirrors `crates/server/src/play/mining.rs::look_vector` exactly — that function is
    /// `mod`-private to `rusty-clanker-server`, unreachable from this crate, so its formula is
    /// restated here, using the real `rc_physics::mth_sin`/`mth_cos` table (not a second,
    /// independently-implemented trig approximation) so this test reconstructs exactly the
    /// direction vector the server's own raycast computes from a reported yaw/pitch.
    fn server_look_vector(yaw_degrees: f32, pitch_degrees: f32) -> Vec3 {
        let yaw_rad = yaw_degrees as f64 * std::f64::consts::PI / 180.0;
        let pitch_rad = pitch_degrees as f64 * std::f64::consts::PI / 180.0;
        let yaw_sin = mth_sin(yaw_rad) as f64;
        let yaw_cos = mth_cos(yaw_rad) as f64;
        Vec3::new(
            -yaw_sin * pitch_rad.cos(),
            -pitch_rad.sin(),
            yaw_cos * pitch_rad.cos(),
        )
    }

    /// The yaw/pitch `Client::look_at`'s own listener would compute for looking from `eye`
    /// toward `aim` — `azalea::bot::direction_looking_at` itself (public API), not a
    /// reimplementation, so this test feeds `server_look_vector` exactly what the real bot
    /// would send over the wire.
    fn yaw_pitch_for(eye: azalea::Vec3, aim: azalea::Vec3) -> (f32, f32) {
        let look = azalea::bot::direction_looking_at(eye, aim);
        (look.y_rot(), look.x_rot())
    }

    /// Reproduces DEFECT 3's own root cause as a standing regression guard, independent of
    /// `click_aim_point`: a level look (yaw/pitch `0.0`/`0.0`, a brand-new player's own
    /// persisted rotation, `connection.rs`'s join flow) from the literal spawn corner resolves
    /// none of the 5 scripted positions — this world's own content sits entirely below spawn
    /// height. Documents the bug `look_at_click`/`recenter_in_spawn_block` actually had to
    /// solve, not merely a claim in a doc comment.
    #[test]
    fn a_level_look_from_the_spawn_corner_hits_none_of_the_5_scripted_positions() {
        let eye = Vec3::new(0.0, -59.0 + PLAYER_EYE_HEIGHT, 0.0);
        let direction = server_look_vector(0.0, 0.0);
        let world = SuperflatWithOverrides::new();

        for &(x, y, z) in &SCRIPTED_CLICKS {
            let hit = cast_ray(eye, direction, CREATIVE_RANGE, &world);
            assert_ne!(
                hit.map(|h| h.block_pos),
                Some(BlockPos::new(x, y, z)),
                "a level look accidentally hit ({x}, {y}, {z}) — this guard is meant to \
                 document DEFECT 3's own root cause, not a coincidental hit"
            );
        }
    }

    /// The actual DEFECT 3 regression test: for every one of the 5 scripted clicked
    /// positions, in script order, with the exact cumulative world mutations
    /// `apply_actions_inner`'s own script produces along the way, `click_aim_point` (aimed
    /// from the bot's own recentered spawn-block position, `recenter_in_spawn_block`) resolves
    /// to a real `cast_ray` hit on that exact position — never a neighboring column, never a
    /// miss. Fails to even compile before this fix (`click_aim_point`/`CLICK_AIM_INSET` did
    /// not exist), and would fail its assertions against the pre-fix aim strategy (`BlockPos::
    /// center`) or the pre-fix script order (`(2,-60,0)` before `(3,-60,0)`) — verified by
    /// hand while developing this fix, restated in this crate's own completion report.
    #[test]
    fn click_aim_point_resolves_every_scripted_position_in_script_order() {
        let eye = Vec3::new(0.5, -59.0 + PLAYER_EYE_HEIGHT, 0.5); // recentered spawn block
        let azalea_eye = azalea::Vec3::new(eye.x, eye.y, eye.z);
        let mut world = SuperflatWithOverrides::new();

        for &(x, y, z) in &SCRIPTED_CLICKS {
            let click = azalea::BlockPos::new(x, y, z);
            let aim = click_aim_point(click);
            let (yaw, pitch) = yaw_pitch_for(azalea_eye, aim);
            let direction = server_look_vector(yaw, pitch);

            let hit = cast_ray(eye, direction, CREATIVE_RANGE, &world).unwrap_or_else(|| {
                panic!(
                    "no hit at all for scripted click ({x}, {y}, {z}) with inset \
                     {CLICK_AIM_INSET}"
                )
            });
            assert_eq!(
                hit.block_pos,
                BlockPos::new(x, y, z),
                "aiming at ({x}, {y}, {z}) with inset {CLICK_AIM_INSET} resolved to {:?} \
                 instead",
                hit.block_pos
            );

            match (x, y, z) {
                (3, -60, 0) => world.set(BlockPos::new(3, -59, 0), STONE.0),
                (2, -60, 0) => world.set(BlockPos::new(2, -59, 0), STONE.0),
                (2, -60, 1) => world.set(BlockPos::new(2, -59, 1), STONE.0),
                (0, -60, 0) | (1, -60, 0) => world.set(BlockPos::new(x, y, z), AIR.0),
                other => panic!("unexpected scripted click {other:?}"),
            }
        }
    }
}
