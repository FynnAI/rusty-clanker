/// Deterministic, in-process-only content checksum for this crate's own round-trip
/// soak tests (Context — vanilla's Anvil format has no on-disk checksum of its own).
/// Backed by `std::collections::hash_map::DefaultHasher`; never written to disk, and
/// makes no cross-process/cross-Rust-version stability claim.
pub fn content_checksum(bytes: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}
