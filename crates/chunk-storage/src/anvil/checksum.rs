/// Deterministic, in-process-only content checksum for this crate's own round-trip
/// soak tests (Context — vanilla's Anvil format has no on-disk checksum of its own).
/// Backed by `std::collections::hash_map::DefaultHasher`; never written to disk, and
/// makes no cross-process/cross-Rust-version stability claim.
pub fn content_checksum(bytes: &[u8]) -> u64 {
    todo!()
}
