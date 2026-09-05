//! The push-model BFS propagator core (M4-B07 Context §2) -- shared by sky light and
//! block light, operating on exactly one chunk's own local data (`LocalChunkLight`).

use rc_chunk_storage::{BlockStateColumn, HeightmapSet, LightColumn, LightNibbles};
use rc_core::{BlockPos, ChunkKey, DimensionId};

use crate::direction::Direction;
use crate::light::properties::{LightProperties, LightPropertiesRegistry, shape_occludes};
use crate::light::queue::{ChannelState, QueueEntry, all_except, contains, only};
use crate::light::section_ops::{self, LIGHT_HEIGHT, LIGHT_MIN_Y};
use crate::light::sky_source::SkyLightSourceColumn;

/// Which of the two independent channels a propagator call operates on (Context §2).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LightChannel {
    Sky,
    Block,
}

/// Every reference one chunk's own local propagator step needs.
pub struct LocalChunkLight<'a> {
    pub light: &'a mut LightColumn,
    pub sky_sources: &'a mut SkyLightSourceColumn,
    pub blocks: &'a BlockStateColumn,
    pub heightmap: &'a HeightmapSet,
    pub properties: &'a LightPropertiesRegistry,
    pub chunk_origin_x: i32,
    pub chunk_origin_z: i32,
}

/// `Direction`'s own declaration order, restated as a plain array (Context §4's own
/// direction-set iteration needs a concrete list to walk).
const ALL_DIRECTION_VALUES: [Direction; 6] = [
    Direction::West,
    Direction::East,
    Direction::North,
    Direction::South,
    Direction::Down,
    Direction::Up,
];

/// `true` iff `pos`'s `x`/`z` fall inside this chunk's own 16x16 horizontal extent
/// (Context §5).
pub fn is_local(pos: BlockPos, chunk_origin_x: i32, chunk_origin_z: i32) -> bool {
    pos.x >= chunk_origin_x
        && pos.x < chunk_origin_x + 16
        && pos.z >= chunk_origin_z
        && pos.z < chunk_origin_z + 16
}

fn local_xz(local: &LocalChunkLight, pos: BlockPos) -> (u8, u8) {
    (
        (pos.x - local.chunk_origin_x) as u8,
        (pos.z - local.chunk_origin_z) as u8,
    )
}

/// Reads `properties.resolve(blocks.get(..))` for `pos`, treating a `y` outside the
/// real block-section range (a light-tracked padding section, WORLD-D8's own `+2`)
/// as `LightProperties::AIR` rather than reaching into `BlockStateColumn::get`'s own
/// out-of-range `assert!`.
fn resolve_properties(local: &LocalChunkLight, pos: BlockPos) -> LightProperties {
    if (rc_chunk_storage::WORLD_MIN_Y
        ..rc_chunk_storage::WORLD_MIN_Y + rc_chunk_storage::WORLD_HEIGHT)
        .contains(&pos.y)
    {
        let (lx, lz) = local_xz(local, pos);
        local.properties.resolve(local.blocks.get(lx, pos.y, lz))
    } else {
        LightProperties::AIR
    }
}

/// Reads `pos`'s current stored nibble for `channel` via `section_ops::nibble_at`.
pub fn get_stored(local: &LocalChunkLight, pos: BlockPos, channel: LightChannel) -> u8 {
    let section_index = section_ops::light_section_index_for_y(pos.y);
    let local_y = section_ops::light_local_y(pos.y);
    let (lx, lz) = local_xz(local, pos);
    let nibble_index = section_ops::light_nibble_index(lx, local_y, lz);
    let section = local.light.section(section_index);
    let nibbles = match channel {
        LightChannel::Sky => &section.sky,
        LightChannel::Block => &section.block,
    };
    section_ops::nibble_at(nibbles, nibble_index)
}

/// Writes `pos`'s stored nibble for `channel`, lazily materializing the containing
/// `LightSection`'s field into `LightNibbles::Data` on first write (PERF-D61,
/// Context §13).
pub fn set_stored(local: &mut LocalChunkLight, pos: BlockPos, channel: LightChannel, value: u8) {
    let section_index = section_ops::light_section_index_for_y(pos.y);
    let local_y = section_ops::light_local_y(pos.y);
    let (lx, lz) = local_xz(local, pos);
    let nibble_index = section_ops::light_nibble_index(lx, local_y, lz);
    let section = local.light.section_mut(section_index);
    let nibbles = match channel {
        LightChannel::Sky => &mut section.sky,
        LightChannel::Block => &mut section.block,
    };
    if let LightNibbles::Data(arr) = nibbles {
        section_ops::set_nibble(arr, nibble_index, value);
        return;
    }
    let mut arr = match nibbles {
        LightNibbles::Uninitialized => Box::new(section_ops::uniform_array(0)),
        LightNibbles::Filled(v) => Box::new(section_ops::uniform_array(*v)),
        LightNibbles::Data(_) => unreachable!("handled above"),
    };
    section_ops::set_nibble(&mut arr, nibble_index, value);
    *nibbles = LightNibbles::Data(arr);
}

/// Context §2's `check_node`, block-light channel.
pub fn check_node_block(
    local: &mut LocalChunkLight,
    pos: BlockPos,
    old_emission: u8,
    new_emission: u8,
    state: &mut ChannelState,
) {
    // `old_emission` names the pre-change block's own emission (Stage 8's own
    // seeding step, Context §8 step 1, computes and passes it) -- the corrected
    // branch condition (CLAIMS row 16) compares `new_emission` against the
    // position's own *currently stored light level*, never `old_emission` itself.
    let _ = old_emission;
    let current = get_stored(local, pos, LightChannel::Block);
    if new_emission < current {
        set_stored(local, pos, LightChannel::Block, 0);
        state.decrease.push_back(QueueEntry {
            pos,
            from_level: current,
            directions: crate::light::queue::ALL_DIRECTIONS,
            increase_from_emission: false,
            foreign_origin: None,
        });
    } else {
        state.decrease.push_back(QueueEntry {
            pos,
            from_level: 1,
            directions: crate::light::queue::ALL_DIRECTIONS,
            increase_from_emission: false,
            foreign_origin: None,
        });
    }
    if new_emission > 0 {
        state.increase.push_back(QueueEntry {
            pos,
            from_level: new_emission,
            directions: crate::light::queue::ALL_DIRECTIONS,
            increase_from_emission: true,
            foreign_origin: None,
        });
    }
}

/// Context §2's `check_node`, sky channel -- `is_source`: `true` iff `pos` is
/// *currently* a sky source per `SkyLightSourceColumn::is_source` (Context §6).
pub fn check_node_sky(
    local: &mut LocalChunkLight,
    pos: BlockPos,
    is_source: bool,
    state: &mut ChannelState,
) {
    if is_source {
        // The level 15 is written directly by this column-maintenance pass itself,
        // not lazily materialized on propagation (Context §2).
        set_stored(local, pos, LightChannel::Sky, 15);
        let directions = all_except(Direction::Up);
        state.decrease.push_back(QueueEntry {
            pos,
            from_level: 15,
            directions,
            increase_from_emission: false,
            foreign_origin: None,
        });
        state.increase.push_back(QueueEntry {
            pos,
            from_level: 15,
            directions,
            increase_from_emission: false,
            foreign_origin: None,
        });
    } else {
        let current = get_stored(local, pos, LightChannel::Sky);
        if current > 0 {
            set_stored(local, pos, LightChannel::Sky, 0);
            state.decrease.push_back(QueueEntry {
                pos,
                from_level: current,
                directions: crate::light::queue::ALL_DIRECTIONS,
                increase_from_emission: false,
                foreign_origin: None,
            });
        } else {
            state.decrease.push_back(QueueEntry {
                pos,
                from_level: 1,
                directions: crate::light::queue::ALL_DIRECTIONS,
                increase_from_emission: false,
                foreign_origin: None,
            });
        }
    }
}

/// `true` iff `neighbor.y` has permanently left the tracked light range -- a hard
/// stop, never deferred (Context §5).
fn outside_light_range(world_y: i32) -> bool {
    !(LIGHT_MIN_Y..LIGHT_MIN_Y + LIGHT_HEIGHT).contains(&world_y)
}

fn defer_chunk_key(pos: BlockPos) -> ChunkKey {
    // `LocalChunkLight` carries no `dimension` field of its own (Context §5's own
    // struct definition) -- a region never spans dimensions (ARCH-D5/D6), so every
    // chunk `run_stage8_lighting` processes in one invocation already shares one
    // dimension; `DimensionId::OVERWORLD` stands in here as this blueprint's own
    // fixed simplification (recorded in `docs/findings-for-planning.md`) until a
    // real multi-dimension composition root threads the true value through.
    ChunkKey::new(DimensionId::OVERWORLD, pos.chunk_x(), pos.chunk_z())
}

/// Context §2's `propagate_increase_step`, one dequeued entry. A cross-boundary
/// target is pushed onto `state.outgoing` instead of being applied locally.
///
/// M4-B07 field-report fix ("light bounce", `docs/findings-for-planning.md`'s own
/// entry on this changeset has the full writeup): `entry.foreign_origin.is_some()`
/// marks a cross-chunk-boundary deferral -- `entry.pos` then names the *origin*
/// node, in the *sending* chunk, never local to `local` -- so every read/write this
/// function would otherwise perform directly against `entry.pos` (the stale check,
/// the emission re-materialize write, and `from_props`'s own resolve) is skipped in
/// favor of the sender-supplied `from_level`/`foreign_origin` instead; the single
/// set bit in `entry.directions` then yields exactly the one local neighbour cell
/// the sender deferred, and every neighbour-side check below (level comparison,
/// opacity, shape occlusion, `set_stored`, re-enqueue) runs bit-for-bit identical to
/// the ordinary local path -- restoring the missing occlusion/opacity check a naive
/// direct write into the neighbour cell used to skip entirely.
pub fn propagate_increase_step(
    local: &mut LocalChunkLight,
    entry: QueueEntry,
    channel: LightChannel,
    state: &mut ChannelState,
) {
    let from_level = match entry.foreign_origin {
        Some(_) => entry.from_level,
        None => {
            let mut from_level = get_stored(local, entry.pos, channel);
            if entry.increase_from_emission && from_level < entry.from_level {
                set_stored(local, entry.pos, channel, entry.from_level);
                from_level = entry.from_level;
            }
            if from_level != entry.from_level {
                return; // Stale -- superseded by a larger increase queued after it.
            }
            from_level
        }
    };

    for &dir in &ALL_DIRECTION_VALUES {
        if !contains(entry.directions, dir) {
            continue;
        }
        let max_possible = from_level.saturating_sub(1);
        if max_possible == 0 {
            continue;
        }
        let neighbor_pos = dir.apply(entry.pos);

        if outside_light_range(neighbor_pos.y) {
            continue; // Hard stop -- never deferred (Context §5).
        }

        if !is_local(neighbor_pos, local.chunk_origin_x, local.chunk_origin_z) {
            if entry.foreign_origin.is_some() {
                // A foreign-origin entry's own single direction always resolves to a
                // local neighbour by construction (the sender deferred exactly this
                // one edge into this chunk, `defer_chunk_key`'s own routing already
                // guarantees it) -- never actually reachable; skip rather than
                // attempt a second, doubly-deferred hop through another chunk's data
                // this one cannot resolve.
                debug_assert!(
                    false,
                    "foreign-origin QueueEntry's own single direction produced a non-local neighbour"
                );
                continue;
            }
            // `entry.pos` is genuinely local here (`foreign_origin` is `None`) --
            // the deferred edge itself is "origin node (here, at its own established
            // `from_level`) -> the neighbour cell in the chunk `neighbor_pos` falls
            // in", carried at `entry.pos` (not `neighbor_pos`) with a single
            // direction so the *receiving* chunk's own next round runs this same
            // neighbour-side check itself, instead of trusting a value computed
            // against this chunk's own opacity data.
            state.outgoing.push((
                defer_chunk_key(neighbor_pos),
                QueueEntry {
                    pos: entry.pos,
                    from_level,
                    directions: only(dir),
                    increase_from_emission: true,
                    foreign_origin: Some(resolve_properties(local, entry.pos)),
                },
            ));
            continue;
        }

        let current_neighbor = get_stored(local, neighbor_pos, channel);
        if max_possible <= current_neighbor {
            continue;
        }

        let from_props = match entry.foreign_origin {
            Some(props) => props,
            None => resolve_properties(local, entry.pos),
        };
        let neighbor_props = resolve_properties(local, neighbor_pos);
        if shape_occludes(from_props, neighbor_props, dir) {
            continue;
        }

        let new_level = from_level.saturating_sub(neighbor_props.get_opacity());
        if new_level <= current_neighbor {
            continue;
        }

        set_stored(local, neighbor_pos, channel, new_level);
        if new_level > 1 {
            state.increase.push_back(QueueEntry {
                pos: neighbor_pos,
                from_level: new_level,
                directions: all_except(dir.opposite()),
                increase_from_emission: false,
                foreign_origin: None,
            });
        }
    }
}

/// Context §2's `propagate_decrease_step`, one dequeued entry.
///
/// M4-B07 field-report fix ("light bounce"): this function never reads `entry.pos`
/// itself (only `dir.apply(entry.pos)`, pure coordinate arithmetic, safe regardless
/// of whether `entry.pos` is local), so a `foreign_origin.is_some()` entry needs no
/// special-cased body here -- the fix is entirely in the deferred entry's own
/// *shape*, below: `pos` carries the true origin (never an already-crossed
/// neighbour position masquerading as one), and `directions` carries only the one
/// direction that actually crosses the boundary, so the receiving chunk's own next
/// round updates exactly the one real border cell the origin's decrease reaches --
/// never fans out to that cell's *own* other neighbours before that cell's own
/// value has even been reconciled (the previous shape's own bug: a decrease
/// deferral used to carry `neighbor_pos` in `pos` and `all_except(dir.opposite())`
/// in `directions`, running the loop below against neighbours of a still-unupdated
/// node instead of against the node itself).
pub fn propagate_decrease_step(
    local: &mut LocalChunkLight,
    entry: QueueEntry,
    channel: LightChannel,
    state: &mut ChannelState,
) {
    for &dir in &ALL_DIRECTION_VALUES {
        if !contains(entry.directions, dir) {
            continue;
        }
        let neighbor_pos = dir.apply(entry.pos);

        if outside_light_range(neighbor_pos.y) {
            continue; // Hard stop -- never deferred (Context §5).
        }

        if !is_local(neighbor_pos, local.chunk_origin_x, local.chunk_origin_z) {
            if entry.foreign_origin.is_some() {
                // See `propagate_increase_step`'s own identical assertion -- never
                // actually reachable by construction.
                debug_assert!(
                    false,
                    "foreign-origin QueueEntry's own single direction produced a non-local neighbour"
                );
                continue;
            }
            state.outgoing.push((
                defer_chunk_key(neighbor_pos),
                QueueEntry {
                    pos: entry.pos,
                    from_level: entry.from_level,
                    directions: only(dir),
                    increase_from_emission: false,
                    foreign_origin: Some(resolve_properties(local, entry.pos)),
                },
            ));
            continue;
        }

        let current = get_stored(local, neighbor_pos, channel);
        if current == 0 {
            continue;
        }
        if current <= entry.from_level.saturating_sub(1) {
            set_stored(local, neighbor_pos, channel, 0);
            let own_source = own_source_strength(local, neighbor_pos, channel);
            if own_source < current {
                state.decrease.push_back(QueueEntry {
                    pos: neighbor_pos,
                    from_level: current,
                    directions: all_except(dir.opposite()),
                    increase_from_emission: false,
                    foreign_origin: None,
                });
            }
            if own_source > 0 {
                state.increase.push_back(QueueEntry {
                    pos: neighbor_pos,
                    from_level: own_source,
                    directions: crate::light::queue::ALL_DIRECTIONS,
                    increase_from_emission: true,
                    foreign_origin: None,
                });
            }
        } else {
            state.increase.push_back(QueueEntry {
                pos: neighbor_pos,
                from_level: current,
                directions: crate::light::queue::only(dir.opposite()),
                increase_from_emission: false,
                foreign_origin: None,
            });
        }
    }
}

/// `channel`'s own baseline glow at `pos` -- block: `properties.resolve(blocks.get(..)).
/// block_emission`; sky: `15` if `local.sky_sources.is_source(..)` else `0` (Context
/// §2's decrease-cascade "own source" check).
pub fn own_source_strength(local: &LocalChunkLight, pos: BlockPos, channel: LightChannel) -> u8 {
    match channel {
        LightChannel::Block => resolve_properties(local, pos).block_emission,
        LightChannel::Sky => {
            let (lx, lz) = local_xz(local, pos);
            if local.sky_sources.is_source(lx, pos.y, lz) {
                15
            } else {
                0
            }
        }
    }
}
