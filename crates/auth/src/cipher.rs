//! AES-128/CFB8 stream setup (NET-D6, Context "AES-128/CFB8 stream setup — exact parameters
//! and the persistent-state requirement"): key = IV = the 16-byte shared secret, one persistent
//! stateful object per direction per connection, never reconstructed for the connection's
//! lifetime.

#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    #[error("AES-128/CFB8 shared secret must be exactly 16 bytes, got {0}")]
    InvalidSharedSecretLength(usize),
}

/// One direction (encrypt) of the AES-128/CFB8 stream a Login handshake establishes (NET-D6).
/// Construct once per connection from the 16-byte shared secret and never reconstruct for the
/// connection's lifetime (Context: reconstructing desynchronizes the feedback register from
/// the peer's).
pub struct Aes128Cfb8Encryptor {
    // fields are private; opaque to callers
}

impl Aes128Cfb8Encryptor {
    /// `shared_secret` must be exactly 16 bytes (`ServerKeyPair::decrypt_pkcs1v15`'s output on
    /// the client's Encryption Response `shared_secret` field). Used as both the AES-128 key
    /// and the CFB8 initialization vector (Context).
    pub fn new(shared_secret: &[u8]) -> Result<Self, CipherError> {
        todo!()
    }

    /// Enciphers `buf` in place, advancing this stream's internal feedback register by exactly
    /// `buf.len()` bytes. Call order across the connection's lifetime must exactly match wire
    /// send order — never re-encrypt, never skip, never reorder a call.
    pub fn encrypt_in_place(&mut self, buf: &mut [u8]) {
        todo!()
    }
}

/// The decrypt-direction counterpart of `Aes128Cfb8Encryptor` — same construction contract,
/// same persistent-state requirement, applied to inbound bytes in wire arrival order.
pub struct Aes128Cfb8Decryptor {
    // fields are private; opaque to callers
}

impl Aes128Cfb8Decryptor {
    pub fn new(shared_secret: &[u8]) -> Result<Self, CipherError> {
        todo!()
    }

    pub fn decrypt_in_place(&mut self, buf: &mut [u8]) {
        todo!()
    }
}
