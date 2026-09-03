//! Light-section nibble-array helpers: index/coordinate math, single-nibble read/write,
//! and cross-region face extraction/injection (M4-B07 Context §10/§11). Light-specific
//! since `rc_chunk_storage::column::block_index`/`section_index_for_y` are asserted to
//! real block-section bounds only and would panic on a light section's own padding
//! range.

use rc_chunk_storage::LightNibbles;

use crate::direction::Direction;

pub const LIGHT_MIN_Y: i32 = rc_chunk_storage::WORLD_MIN_Y - 16; // -80
pub const LIGHT_HEIGHT: i32 = rc_chunk_storage::WORLD_HEIGHT + 32; // 416
pub const LIGHT_SECTION_COUNT: usize = rc_chunk_storage::LIGHT_SECTION_COUNT; // 26

/// This light section's `0..26` index for `world_y` (WORLD-D8's `+2`-padded
/// indexing). Panics (`assert!`) if `world_y` falls outside `LIGHT_MIN_Y ..
/// LIGHT_MIN_Y + LIGHT_HEIGHT`.
#[inline]
pub fn light_section_index_for_y(world_y: i32) -> usize {
    assert!(
        (LIGHT_MIN_Y..LIGHT_MIN_Y + LIGHT_HEIGHT).contains(&world_y),
        "world_y out of light-tracked range"
    );
    ((world_y - LIGHT_MIN_Y) / 16) as usize
}

/// `world_y`'s local Y (`0..16`) within its own light section.
#[inline]
pub fn light_local_y(world_y: i32) -> u8 {
    ((world_y - LIGHT_MIN_Y).rem_euclid(16)) as u8
}

/// Local nibble index within one `[u8; 2048]` light section array -- identical axis
/// order/formula to `rc_chunk_storage::column::block_index` (`(local_y << 8) |
/// (z << 4) | x`), restated locally since it must be callable for `local_y` values
/// belonging to padding sections that `block_index`'s own real-section-only
/// counterpart is never asked to handle.
#[inline]
pub fn light_nibble_index(x: u8, local_y: u8, z: u8) -> usize {
    ((local_y as usize) << 8) | ((z as usize) << 4) | (x as usize)
}

#[inline]
fn nibble_get_raw(bytes: &[u8], index: usize) -> u8 {
    let byte = bytes[index >> 1];
    if index & 1 == 0 {
        byte & 0x0F
    } else {
        (byte >> 4) & 0x0F
    }
}

#[inline]
fn nibble_set_raw(bytes: &mut [u8], index: usize, value: u8) {
    let byte_index = index >> 1;
    let byte = bytes[byte_index];
    bytes[byte_index] = if index & 1 == 0 {
        (byte & 0xF0) | (value & 0x0F)
    } else {
        (byte & 0x0F) | ((value & 0x0F) << 4)
    };
}

/// Reads one nibble (4 bits) at `index` (`0..4096`) from a 2048-byte nibble array.
#[inline]
pub fn get_nibble(data: &[u8; 2048], index: usize) -> u8 {
    nibble_get_raw(data, index)
}

/// Writes one nibble at `index`, touching only its own 4 bits of its containing byte.
#[inline]
pub fn set_nibble(data: &mut [u8; 2048], index: usize, value: u8) {
    nibble_set_raw(data, index, value);
}

/// Reads one nibble at `index` (`0..4096`) from a `LightNibbles` value (WORLD-D8),
/// regardless of which of its three representations is active.
pub fn nibble_at(nibbles: &LightNibbles, index: usize) -> u8 {
    match nibbles {
        LightNibbles::Uninitialized => 0,
        LightNibbles::Filled(v) => *v,
        LightNibbles::Data(arr) => get_nibble(arr, index),
    }
}

/// Packs `value` into every nibble of a fresh `[u8; 2048]` array (Context §12/§13).
pub fn uniform_array(value: u8) -> [u8; 2048] {
    [(value << 4) | (value & 0x0F); 2048]
}

/// Packs `value` into every nibble of a fresh `[u8; 128]` array (Context §10).
pub fn uniform_face(value: u8) -> [u8; 128] {
    [(value << 4) | (value & 0x0F); 128]
}

/// The fixed axis position for `face`'s own local-index layout (Context §10):
/// `perp` is `z` for `West`/`East` (fixed `x = 0`/`x = 15`), or `x` for
/// `North`/`South` (fixed `z = 0`/`z = 15`).
#[inline]
fn face_axis_position(face: Direction, perp: u8) -> (u8, u8) {
    match face {
        Direction::West => (0, perp),
        Direction::East => (15, perp),
        Direction::North => (perp, 0),
        Direction::South => (perp, 15),
        _ => unreachable!("extract_face/inject_face: face must be West/East/North/South"),
    }
}

/// Extracts one `LightSection` face (256 positions, 128 bytes) for cross-region
/// transmission (Context §10). `face` must be `West`, `East`, `North`, or `South`.
pub fn extract_face(section: &[u8; 2048], face: Direction) -> [u8; 128] {
    debug_assert!(matches!(
        face,
        Direction::West | Direction::East | Direction::North | Direction::South
    ));
    let mut out = [0u8; 128];
    for local_y in 0u8..16 {
        for perp in 0u8..16 {
            let (x, z) = face_axis_position(face, perp);
            let value = nibble_get_raw(section, light_nibble_index(x, local_y, z));
            let face_index = (local_y as usize) * 16 + perp as usize;
            nibble_set_raw(&mut out, face_index, value);
        }
    }
    out
}

/// Inverse of `extract_face` -- writes `face_data` into `section`'s own matching
/// 256 positions, leaving every other position in `section` untouched.
pub fn inject_face(section: &mut [u8; 2048], face: Direction, face_data: &[u8; 128]) {
    debug_assert!(matches!(
        face,
        Direction::West | Direction::East | Direction::North | Direction::South
    ));
    for local_y in 0u8..16 {
        for perp in 0u8..16 {
            let (x, z) = face_axis_position(face, perp);
            let face_index = (local_y as usize) * 16 + perp as usize;
            let value = nibble_get_raw(face_data, face_index);
            nibble_set_raw(section, light_nibble_index(x, local_y, z), value);
        }
    }
}

/// Extracts one channel's face slice (Context §10) directly from a `LightNibbles`
/// value, handling all three representations without ever forcing a `Filled`
/// section to materialize its full `[u8; 2048]` array first.
pub fn extract_face_from_nibbles(nibbles: &LightNibbles, face: Direction) -> Option<[u8; 128]> {
    match nibbles {
        LightNibbles::Uninitialized => None,
        LightNibbles::Filled(v) => Some(uniform_face(*v)),
        LightNibbles::Data(arr) => Some(extract_face(arr, face)),
    }
}
