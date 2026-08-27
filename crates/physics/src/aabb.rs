//! Axis-aligned bounding box, world-space or block-local `[0,1]^3` depending on context
//! (Context: "VoxelShape representation"), and the `Axis` enum `sweep_axis`/`clip_distance`
//! stay generic across.

use crate::Vec3;
use rc_core::BlockPos;

/// Axis-aligned bounding box, world-space or block-local `[0,1]^3` depending on context.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

/// Which spatial axis; used by `sweep_axis`/`clip_distance` to stay generic across all three.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    /// The two axes other than `self`, in a fixed `(a, b)` order -- used only for the
    /// symmetric "do these two axes' extents overlap" check, so the order itself carries no
    /// meaning.
    pub const fn other_two(self) -> (Axis, Axis) {
        match self {
            Axis::X => (Axis::Y, Axis::Z),
            Axis::Y => (Axis::X, Axis::Z),
            Axis::Z => (Axis::X, Axis::Y),
        }
    }
}

/// The lowest/highest integer coordinate whose unit cell overlaps `[min, max)` -- floor of
/// `min`, and `ceil(max) - 1` (equal to `floor(max)` unless `max` sits exactly on an integer
/// boundary, in which case the cell beginning at that boundary is correctly excluded: a box
/// that merely touches a cell's near face does not overlap it). Clamped so the range is
/// never inverted for a degenerate (`min == max`) box.
fn axis_bounds(min: f64, max: f64) -> (i32, i32) {
    let lo = min.floor() as i32;
    let hi = ((max.ceil() as i32) - 1).max(lo);
    (lo, hi)
}

impl Aabb {
    /// Centered horizontally on `(position.x, position.z)`, feet at `position.y`, per the
    /// given half-width/height -- the standard entity-hitbox construction (Context: "Player
    /// dimensions").
    pub fn from_position(position: Vec3, half_width: f64, height: f64) -> Self {
        Aabb {
            min: Vec3::new(position.x - half_width, position.y, position.z - half_width),
            max: Vec3::new(
                position.x + half_width,
                position.y + height,
                position.z + half_width,
            ),
        }
    }

    pub fn translated(self, dx: f64, dy: f64, dz: f64) -> Self {
        Aabb {
            min: Vec3::new(self.min.x + dx, self.min.y + dy, self.min.z + dz),
            max: Vec3::new(self.max.x + dx, self.max.y + dy, self.max.z + dz),
        }
    }

    /// Extends this box's own extent on `axis` by `delta` (Context: "Collide-and-slide
    /// sweep" -- the motion-swept broad-phase box): grows `max` for a positive `delta`,
    /// `min` for a negative one, leaving the box's opposite face untouched either way.
    pub fn extended_along(self, axis: Axis, delta: f64) -> Self {
        let mut out = self;
        let (lo, hi): (&mut f64, &mut f64) = match axis {
            Axis::X => (&mut out.min.x, &mut out.max.x),
            Axis::Y => (&mut out.min.y, &mut out.max.y),
            Axis::Z => (&mut out.min.z, &mut out.max.z),
        };
        if delta >= 0.0 {
            *hi += delta;
        } else {
            *lo += delta;
        }
        out
    }

    pub fn min(self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.min.x,
            Axis::Y => self.min.y,
            Axis::Z => self.min.z,
        }
    }

    pub fn max(self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.max.x,
            Axis::Y => self.max.y,
            Axis::Z => self.max.z,
        }
    }

    /// `true` iff this box's extent on `axis` overlaps `other`'s, within `epsilon` -- the
    /// overlap must exceed `epsilon` on both sides to count, so two boxes merely touching
    /// (zero-width overlap) never register as overlapping (Context: `SHAPE_EPSILON`).
    pub fn overlaps_on(self, axis: Axis, other: Aabb, epsilon: f64) -> bool {
        self.max(axis) - other.min(axis) > epsilon && other.max(axis) - self.min(axis) > epsilon
    }

    /// Every integer block position whose unit cell overlaps this box (inclusive floor/ceil
    /// over all three axes) -- the broad-phase candidate set `sweep_axis` iterates.
    pub fn overlapped_block_positions(self) -> Vec<BlockPos> {
        let (x0, x1) = axis_bounds(self.min.x, self.max.x);
        let (y0, y1) = axis_bounds(self.min.y, self.max.y);
        let (z0, z1) = axis_bounds(self.min.z, self.max.z);
        let mut out = Vec::new();
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    out.push(BlockPos::new(x, y, z));
                }
            }
        }
        out
    }

    /// Translates a block-local `[0,1]^3` sub-box into world space at `pos` (adds `pos`'s
    /// integer coordinates to `self.min`/`self.max`).
    pub fn offset_by(self, pos: BlockPos) -> Aabb {
        Aabb {
            min: Vec3::new(
                self.min.x + pos.x as f64,
                self.min.y + pos.y as f64,
                self.min.z + pos.z as f64,
            ),
            max: Vec3::new(
                self.max.x + pos.x as f64,
                self.max.y + pos.y as f64,
                self.max.z + pos.z as f64,
            ),
        }
    }
}
