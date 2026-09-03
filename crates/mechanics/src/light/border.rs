//! Cross-region light propagation (M4-B07 Context §10, WORLD-D10): outbound
//! `LightBorderUpdate` construction and inbound application.
//!
//! Ledger note (recorded in `docs/findings-for-planning.md` section B):
//! Implementation step 12's own prose leaves `build_light_border_update`'s `face`
//! parameter and `apply_inbound_light_border_update`'s exact `directions`/
//! `increase_from_emission` values ambiguous ("restate and fix this convention
//! concretely... once written" -- its own words), and its own literal
//! `increase_from_emission: false` / `directions: all_except(edge_face.opposite())`
//! phrasing, worked through against `light_chunk_border.rs`'s own acceptance test 3
//! numbers (local `(0,0,0) == 13`, local `(1,0,0) == 12`), does not actually
//! reproduce them -- an inbound seed with `increase_from_emission: false` at the
//! position's own untouched (zero) stored level is immediately stale-discarded by
//! `propagate_increase_step`'s own first check, and `all_except(edge_face.opposite())`
//! forbids exactly the direction the test's own second assertion needs. This module
//! instead: (a) `build_light_border_update`'s own `face` parameter names the
//! *sending* chunk's own outward edge (the direction from sender toward receiver,
//! matching that same Implementation step's own opening framing); `edge_face` in the
//! built message is `direction_index(face.opposite())` -- the *receiving* chunk's own
//! edge, matching `light_chunk_border.rs` test 2's own explicit `edge_face == 0`
//! assertion and its own "the field names the receiving chunk's own edge" prose
//! exactly. (b) `apply_inbound_light_border_update` seeds each face position with
//! `increase_from_emission: true` and `directions: all_except(edge_dir)` (`edge_dir`
//! being this chunk's own edge the data arrived at, i.e. the direction back toward
//! the sender) -- mirroring `check_node_sky`'s own identical "the value is written
//! directly, not derived from an ordinary local hop" lazy-materialization pattern,
//! and the within-region cross-chunk deferral mechanism `stage8.rs`'s own merge step
//! uses for the identical purpose (Context §5), both of which this same file's
//! `propagator.rs` sibling already established need `increase_from_emission: true`
//! for a cross-boundary-received value to materialize correctly on the receiving
//! side's own next round. The seeded position's own absolute `BlockPos` is derived
//! from `ev.chunk`'s own `(x, z)` (block origin `chunk.x * 16`/`chunk.z * 16`) and
//! `ev.section_index` -- this function's own signature carries no `LocalChunkLight`,
//! so `ev.chunk` (the receiving chunk's own identity, already part of the message)
//! is the only available source for that origin.

use rc_chunk_storage::LightColumn;
use rc_core::{BlockPos, ChunkKey};
use rc_messaging::LightBorderUpdate;

use crate::direction::Direction;
use crate::light::properties::direction_index;
use crate::light::queue::{ChannelState, DirectionSet, QueueEntry, all_except};
use crate::light::section_ops::{self, LIGHT_MIN_Y, extract_face_from_nibbles};

/// Builds one outbound `LightBorderUpdate` for `column`'s own `section_index`/`face`
/// (Context §8 step 10's own caller). `face` is the *sending* chunk's own outward
/// edge -- the direction from the sender toward `receiving_chunk` -- used both to
/// select which of `column`'s own faces to extract and, via its own opposite, to
/// derive the receiving chunk's own `edge_face` (this module's own doc comment has
/// the full reasoning).
pub fn build_light_border_update(
    receiving_chunk: ChunkKey,
    section_index: u8,
    face: Direction,
    column: &LightColumn,
) -> LightBorderUpdate {
    let section = column.section(section_index as usize);
    LightBorderUpdate {
        chunk: receiving_chunk,
        section_index,
        edge_face: direction_index(face.opposite()) as u8,
        sky: extract_face_from_nibbles(&section.sky, face),
        block: extract_face_from_nibbles(&section.block, face),
    }
}

fn direction_from_index(index: u8) -> Direction {
    match index {
        0 => Direction::West,
        1 => Direction::East,
        2 => Direction::North,
        3 => Direction::South,
        4 => Direction::Down,
        5 => Direction::Up,
        other => panic!("direction_from_index: {other} is not a valid Direction index"),
    }
}

/// Applies one inbound `LightBorderUpdate` (Stage 8's own seeding step 3). See this
/// module's own doc comment for the exact `directions`/`increase_from_emission`
/// resolution and for how each seeded position's own absolute `BlockPos` is derived.
pub fn apply_inbound_light_border_update(
    state: &mut crate::light::queue::LightPropagatorState,
    ev: &LightBorderUpdate,
) {
    let edge_dir = direction_from_index(ev.edge_face);
    let directions = all_except(edge_dir);

    if let Some(sky_face) = &ev.sky {
        seed_face(
            &mut state.sky,
            ev.chunk,
            ev.section_index,
            edge_dir,
            directions,
            sky_face,
        );
    }
    if let Some(block_face) = &ev.block {
        seed_face(
            &mut state.block,
            ev.chunk,
            ev.section_index,
            edge_dir,
            directions,
            block_face,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn seed_face(
    channel: &mut ChannelState,
    chunk: ChunkKey,
    section_index: u8,
    edge_dir: Direction,
    directions: DirectionSet,
    face_bytes: &[u8; 128],
) {
    let chunk_origin_x = chunk.x * 16;
    let chunk_origin_z = chunk.z * 16;
    let expanded = expand_face(face_bytes);

    for local_y in 0u8..16 {
        for perp in 0u8..16 {
            let face_index = (local_y as usize) * 16 + perp as usize;
            let received = section_ops::get_nibble(&expanded, face_index);
            let from_level = received.saturating_sub(1);

            let (local_x, local_z) = match edge_dir {
                Direction::West => (0u8, perp),
                Direction::East => (15u8, perp),
                Direction::North => (perp, 0u8),
                Direction::South => (perp, 15u8),
                _ => unreachable!(
                    "apply_inbound_light_border_update: edge_face must be West/East/North/South"
                ),
            };
            let world_y = LIGHT_MIN_Y + (section_index as i32) * 16 + local_y as i32;
            let pos = BlockPos::new(
                chunk_origin_x + local_x as i32,
                world_y,
                chunk_origin_z + local_z as i32,
            );

            channel.increase.push_back(QueueEntry {
                pos,
                from_level,
                directions,
                increase_from_emission: true,
            });
        }
    }
}

/// Materializes a 128-byte face slice into a `[u8; 2048]`-shaped view so
/// `section_ops::get_nibble` (which operates on the full-size array) can read a
/// single face-local nibble -- the face array's own first 128 bytes carry every
/// nibble the caller needs, the rest is never read.
fn expand_face(face: &[u8; 128]) -> [u8; 2048] {
    let mut out = [0u8; 2048];
    out[..128].copy_from_slice(face);
    out
}
