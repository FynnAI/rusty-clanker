//! Offline-mode UUID derivation (NET-D6's non-default offline stance, Context "Offline-mode
//! stance and UUID derivation"): Java's `UUID.nameUUIDFromBytes` applied directly to
//! `"OfflinePlayer:" + username` (no namespace prefix).

use md5::{Digest, Md5};

/// Derives the deterministic offline-mode player UUID (NET-D6's offline-mode stance, Context
/// "Offline-mode stance and UUID derivation," which this function implements exactly): an
/// RFC 4122 version-3 (name-based, MD5) UUID computed directly over `"OfflinePlayer:" +
/// username`, no namespace prefix.
pub fn offline_uuid(username: &str) -> uuid::Uuid {
    let mut hasher = Md5::new();
    hasher.update(b"OfflinePlayer:");
    hasher.update(username.as_bytes());
    let mut bytes: [u8; 16] = hasher.finalize().into();

    bytes[6] = (bytes[6] & 0x0f) | 0x30; // RFC 4122 version 3
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC 4122 variant (10xx)

    uuid::Uuid::from_bytes(bytes)
}
