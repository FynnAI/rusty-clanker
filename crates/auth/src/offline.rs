//! Offline-mode UUID derivation (NET-D6's non-default offline stance, Context "Offline-mode
//! stance and UUID derivation"): Java's `UUID.nameUUIDFromBytes` applied directly to
//! `"OfflinePlayer:" + username` (no namespace prefix).

/// Derives the deterministic offline-mode player UUID (NET-D6's offline-mode stance, Context
/// "Offline-mode stance and UUID derivation," which this function implements exactly): an
/// RFC 4122 version-3 (name-based, MD5) UUID computed directly over `"OfflinePlayer:" +
/// username`, no namespace prefix.
pub fn offline_uuid(username: &str) -> uuid::Uuid {
    todo!()
}
