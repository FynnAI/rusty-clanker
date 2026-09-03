//! Entity-side fluid interaction — the AABB submersion scan, the eye-submersion query
//! (drowning), and fluid push (`14-physics-collision.md` §3.8, Context §E, closing M4-B06's
//! own explicitly-reserved "this blueprint does not implement the AABB submersion scan" gap).

use rc_core::BlockPos;
use rc_physics::{Aabb, Vec3};

use crate::fluid::{FluidKind, FluidTables, fluid_state_at, get_flow, get_height};
use crate::world_access::BlockWorldAccess;

pub const FLUID_PROBE_INSET: f64 = 0.001;
pub const WATER_PUSH_SCALE: f64 = 0.014;
pub const LAVA_PUSH_SCALE_FAST: f64 = 0.007;
pub const LAVA_PUSH_SCALE_SLOW: f64 = 0.0023333333333333335;
pub const PUSH_FLOOR_MAGNITUDE: f64 = 0.0045;
/// Reused, for internal consistency, by both the swim-threshold check (§ physics ecs) and the
/// lava shallow/deep branch selection — not an independently-sourced vanilla constant
/// (Context §E, flagged moderate-confidence).
pub const SUBMERSION_SWIM_THRESHOLD: f64 = 0.4;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct FluidInteraction {
    /// Height (blocks) the entity's own lowest point is submerged below the fluid's own
    /// surface at the highest-submersion touched cell; `0.0` if not touching this `kind` at
    /// all.
    pub submersion: f64,
    /// Accumulated, height-scaled horizontal flow vector across every touched cell of `kind`
    /// (`y` is always `0.0`).
    pub flow: Vec3,
}

fn deflated(aabb: Aabb, inset: f64) -> Aabb {
    Aabb {
        min: Vec3::new(aabb.min.x + inset, aabb.min.y + inset, aabb.min.z + inset),
        max: Vec3::new(aabb.max.x - inset, aabb.max.y - inset, aabb.max.z - inset),
    }
}

/// Context §E's own scan algorithm — pure, `bevy_ecs`-free, matching `rc-physics`'s own
/// established "world access via a trait object" boundary.
pub fn scan_fluid_interaction(
    aabb: Aabb,
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    kind: FluidKind,
) -> FluidInteraction {
    let probe = deflated(aabb, FLUID_PROBE_INSET);
    let mut max_submersion: f64 = 0.0;
    let mut flow_x: f64 = 0.0;
    let mut flow_z: f64 = 0.0;

    for block_pos in probe.overlapped_block_positions() {
        let Some(state) = fluid_state_at(world, tables, block_pos) else {
            continue;
        };
        if state.kind != kind {
            continue;
        }
        let fluid_top = block_pos.y as f64 + get_height(world, tables, block_pos, state) as f64;
        let submersion = fluid_top - probe.min.y;
        if submersion > 0.0 {
            max_submersion = max_submersion.max(submersion);
            let flow = get_flow(world, tables, block_pos, state);
            let scale = if max_submersion < SUBMERSION_SWIM_THRESHOLD {
                max_submersion
            } else {
                1.0
            };
            flow_x += flow.x * scale;
            flow_z += flow.z * scale;
        }
    }

    FluidInteraction {
        submersion: max_submersion,
        flow: Vec3::new(flow_x, 0.0, flow_z),
    }
}

fn floor_block_pos(v: Vec3) -> BlockPos {
    BlockPos::new(v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32)
}

/// `true` iff the entity's eye position sits inside a fluid cell of `kind` — a single-point
/// query, used only by drowning (only meaningful for living tier-2 kinds; item entities never
/// need it).
pub fn eyes_in_fluid(
    eye_position: Vec3,
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    kind: FluidKind,
) -> bool {
    let block_pos = floor_block_pos(eye_position);
    fluid_state_at(world, tables, block_pos).is_some_and(|state| {
        state.kind == kind
            && get_height(world, tables, block_pos, state) as f64 + block_pos.y as f64
                > eye_position.y
    })
}

/// Context §E's own push-vector application (normalize, scale, floor-renormalize). Called by
/// `system_entity_physics_integration` (`ecs.rs`) at a per-kind position, never uniformly:
/// before `step_living_entity_tick` for a living tier-2 kind, after `step_item_entity_tick`
/// for an item entity.
pub fn apply_fluid_push(velocity: Vec3, interaction: &FluidInteraction, push_scale: f64) -> Vec3 {
    let flow = interaction.flow;
    let flow_len = (flow.x * flow.x + flow.z * flow.z).sqrt();
    if flow_len == 0.0 {
        return velocity;
    }

    let normalized = Vec3::new(flow.x / flow_len, 0.0, flow.z / flow_len);
    let mut impulse = normalized * push_scale;
    let impulse_mag = (impulse.x * impulse.x + impulse.z * impulse.z).sqrt();

    let horizontal_vel_mag = (velocity.x * velocity.x + velocity.z * velocity.z).sqrt();
    if horizontal_vel_mag < 1e-3 && impulse_mag > 0.0 && impulse_mag < PUSH_FLOOR_MAGNITUDE {
        let scale_up = PUSH_FLOOR_MAGNITUDE / impulse_mag;
        impulse = impulse * scale_up;
    }

    velocity + impulse
}
