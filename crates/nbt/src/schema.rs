use crate::{Mutf8Str, borrow, owned};

/// Locates a field inside a decoded NBT tree, for `SchemaError`'s diagnostics only
/// (Context: an original design inspired by, but not copied from, vanilla's own
/// documented `ValueInput`/`ValueOutput` problem-path concept). Cheap to clone.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NbtPath(Vec<PathSegment>);

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSegment {
    Field(&'static str),
    Index(usize),
}

impl NbtPath {
    /// The empty path — `Display`s as `<root>`.
    pub fn root() -> Self {
        Self(Vec::new())
    }
    /// A new path with one more named-field segment appended (`self` unchanged).
    pub fn field(&self, name: &'static str) -> Self {
        let mut segments = self.0.clone();
        segments.push(PathSegment::Field(name));
        Self(segments)
    }
    /// A new path with one more list-index segment appended (`self` unchanged).
    pub fn index(&self, i: usize) -> Self {
        let mut segments = self.0.clone();
        segments.push(PathSegment::Index(i));
        Self(segments)
    }
}

impl std::fmt::Display for NbtPath {
    /// e.g. `<root>.sections[3].block_states` — dot-joins `Field` segments, brackets
    /// `Index` segments onto the immediately preceding segment.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<root>")?;
        for segment in &self.0 {
            match segment {
                PathSegment::Field(name) => write!(f, ".{name}")?,
                PathSegment::Index(i) => write!(f, "[{i}]")?,
            }
        }
        Ok(())
    }
}

/// One typed struct <-> NBT compound conversion failure, always path-qualified.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchemaError {
    #[error("{path}: missing required field `{field}`")]
    MissingField { path: NbtPath, field: &'static str },
    #[error(
        "{path}: field `{field}` has the wrong tag type: expected {expected}, found tag id {actual_id}"
    )]
    WrongType {
        path: NbtPath,
        field: &'static str,
        expected: &'static str,
        actual_id: u8,
    },
    #[error("{path}: field `{field}` has an invalid value: {reason}")]
    InvalidValue {
        path: NbtPath,
        field: &'static str,
        reason: String,
    },
}

/// The write direction: `Self` -> a fresh, owned NBT compound. Hand-written per
/// vanilla schema type (WORLD-D11) — never `#[derive(...)]`d.
pub trait ToNbtCompound {
    fn to_nbt_compound(&self) -> owned::NbtCompound;
}

/// The read direction: a borrowed, zero-copy compound -> `Self`. Hand-written per
/// vanilla schema type, same rule as `ToNbtCompound`.
pub trait FromNbtCompound: Sized {
    fn from_nbt_compound<'a, 'tape>(
        compound: &borrow::NbtCompound<'a, 'tape>,
        path: &NbtPath,
    ) -> Result<Self, SchemaError>;
}

/// `SchemaError`-producing "require this field or fail with a precise path" accessors
/// over `borrow::NbtCompound`, layered on top of (never replacing) its existing
/// `Option`-returning accessors — use those directly for genuinely optional fields.
/// One `require_*` per NBT tag type, mirroring `NbtTag`'s own accessor completeness.
pub trait NbtCompoundExt<'a, 'tape> {
    fn require_byte(&self, path: &NbtPath, field: &'static str) -> Result<i8, SchemaError>;
    fn require_short(&self, path: &NbtPath, field: &'static str) -> Result<i16, SchemaError>;
    fn require_int(&self, path: &NbtPath, field: &'static str) -> Result<i32, SchemaError>;
    fn require_long(&self, path: &NbtPath, field: &'static str) -> Result<i64, SchemaError>;
    fn require_float(&self, path: &NbtPath, field: &'static str) -> Result<f32, SchemaError>;
    fn require_double(&self, path: &NbtPath, field: &'static str) -> Result<f64, SchemaError>;
    fn require_byte_array(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<&'a [u8], SchemaError>;
    fn require_string(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<&'a Mutf8Str, SchemaError>;
    fn require_list(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<borrow::NbtList<'a, 'tape>, SchemaError>;
    fn require_compound(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<borrow::NbtCompound<'a, 'tape>, SchemaError>;
    fn require_int_array(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<Vec<i32>, SchemaError>;
    fn require_long_array(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<Vec<i64>, SchemaError>;
}

/// Generates one `require_*` body: look up `field` in `self` (`MissingField` on
/// absence), then apply `$accessor` to the found tag (`WrongType { actual_id:
/// tag.id(), .. }` on the accessor's own `None`) — the single two-step shape every
/// `NbtCompoundExt` method follows (Deliverables' implementation note).
macro_rules! require_accessor {
    ($self:ident, $path:ident, $field:ident, $accessor:ident, $expected:literal) => {{
        let Some(tag) = $self.get($field) else {
            return Err(SchemaError::MissingField {
                path: $path.clone(),
                field: $field,
            });
        };
        tag.$accessor().ok_or_else(|| SchemaError::WrongType {
            path: $path.clone(),
            field: $field,
            expected: $expected,
            actual_id: tag.id(),
        })
    }};
}

impl<'a, 'tape> NbtCompoundExt<'a, 'tape> for borrow::NbtCompound<'a, 'tape> {
    // Implementation note (not part of the committed public surface): every method
    // follows the identical two-step shape — `self.get(field)` -> `MissingField` on
    // `None`; on `Some(tag)`, the matching `NbtTag` accessor (`tag.int()`, `tag.string()`,
    // ...) -> `WrongType { actual_id: tag.id(), .. }` on its own `None`. A private
    // `macro_rules!` generating all twelve bodies from this one shape is the expected,
    // but not mandated, implementation strategy (Implementation steps, below).

    fn require_byte(&self, path: &NbtPath, field: &'static str) -> Result<i8, SchemaError> {
        require_accessor!(self, path, field, byte, "Byte")
    }
    fn require_short(&self, path: &NbtPath, field: &'static str) -> Result<i16, SchemaError> {
        require_accessor!(self, path, field, short, "Short")
    }
    fn require_int(&self, path: &NbtPath, field: &'static str) -> Result<i32, SchemaError> {
        require_accessor!(self, path, field, int, "Int")
    }
    fn require_long(&self, path: &NbtPath, field: &'static str) -> Result<i64, SchemaError> {
        require_accessor!(self, path, field, long, "Long")
    }
    fn require_float(&self, path: &NbtPath, field: &'static str) -> Result<f32, SchemaError> {
        require_accessor!(self, path, field, float, "Float")
    }
    fn require_double(&self, path: &NbtPath, field: &'static str) -> Result<f64, SchemaError> {
        require_accessor!(self, path, field, double, "Double")
    }
    fn require_byte_array(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<&'a [u8], SchemaError> {
        require_accessor!(self, path, field, byte_array, "ByteArray")
    }
    fn require_string(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<&'a Mutf8Str, SchemaError> {
        require_accessor!(self, path, field, string, "String")
    }
    fn require_list(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<borrow::NbtList<'a, 'tape>, SchemaError> {
        require_accessor!(self, path, field, list, "List")
    }
    fn require_compound(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<borrow::NbtCompound<'a, 'tape>, SchemaError> {
        require_accessor!(self, path, field, compound, "Compound")
    }
    fn require_int_array(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<Vec<i32>, SchemaError> {
        require_accessor!(self, path, field, int_array, "IntArray")
    }
    fn require_long_array(
        &self,
        path: &NbtPath,
        field: &'static str,
    ) -> Result<Vec<i64>, SchemaError> {
        require_accessor!(self, path, field, long_array, "LongArray")
    }
}
