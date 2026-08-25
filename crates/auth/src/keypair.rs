//! Per-process-boot RSA-1024 keypair (NET-D6, Context "RSA keypair — lifecycle, size, and DER
//! export"): X.509 `SubjectPublicKeyInfo` DER export for the wire's `public_key` field, and
//! PKCS#1 v1.5 decryption for the client's Encryption Response fields.

/// RSA key size in bits (NET-D6: "server generates one RSA-1024 keypair per process boot").
/// Fixed, never configurable — matches the pinned protocol's own well-established convention
/// (Context, "RSA keypair — lifecycle, size, and DER export").
pub const RSA_KEY_BITS: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum KeyPairError {
    #[error("RSA-{RSA_KEY_BITS} key generation failed: {0}")]
    Generation(String),
    #[error("X.509 SubjectPublicKeyInfo DER encoding of the public key failed: {0}")]
    DerEncoding(String),
    #[error("PKCS#1 v1.5 decryption failed: {0}")]
    Decryption(String),
}

/// One process-boot-lifetime RSA keypair (Context). Share via `Arc<ServerKeyPair>` across
/// every connection — every method here takes `&self`.
pub struct ServerKeyPair {
    // fields are private; opaque to callers
}

impl ServerKeyPair {
    /// Generates a fresh RSA-`RSA_KEY_BITS`-bit keypair using the OS CSPRNG. Call exactly once
    /// per server process boot (Context) — never per-connection.
    pub fn generate() -> Result<Self, KeyPairError> {
        todo!()
    }

    /// The public key, X.509 `SubjectPublicKeyInfo` DER-encoded — exactly the bytes a future
    /// Login-packet-catalog blueprint's Encryption Request `public_key` field carries (Context,
    /// "Encryption Request / Encryption Response — exact wire layout"). Deterministically
    /// `162` bytes for `RSA_KEY_BITS = 1024` (Context, empirically verified).
    pub fn public_key_der(&self) -> &[u8] {
        todo!()
    }

    /// Decrypts a PKCS#1 v1.5-encrypted byte array — the client's Encryption Response
    /// `shared_secret` or `verify_token` field — using this keypair's private key. Both fields
    /// are always exactly 128 bytes on input for an RSA-1024 modulus (Context).
    pub fn decrypt_pkcs1v15(&self, ciphertext: &[u8]) -> Result<Vec<u8>, KeyPairError> {
        todo!()
    }
}

/// Generates a fresh, cryptographically random 4-byte verify token (NET-D6's "challenge") —
/// one call per connection's login attempt, never reused across connections.
pub fn generate_verify_token() -> [u8; 4] {
    todo!()
}
