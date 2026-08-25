//! AES-128/CFB8 stream setup (NET-D6, Context "AES-128/CFB8 stream setup — exact parameters
//! and the persistent-state requirement"): key = IV = the 16-byte shared secret, one persistent
//! stateful object per direction per connection, never reconstructed for the connection's
//! lifetime.
//!
//! Implementation note (M1-B03 reconciliation): the installed `cfb8` 0.9.1's own public API
//! (verified against its source, `cargo doc -p cfb8`) is simpler than this blueprint's own
//! guessed shape — `Encryptor<C>`/`Decryptor<C>` expose inherent, `&mut self`-taking
//! `encrypt(&mut self, buf: &mut [u8])`/`decrypt(&mut self, buf: &mut [u8])` methods directly
//! (not the `BlockModeEncrypt`/`BlockModeDecrypt` trait's per-block `encrypt_block` the
//! blueprint's Context speculated), and neither consumes `self`. The binding *shape* — one
//! persistent stateful object per direction, its internal feedback register advancing across
//! every call, never reconstructed mid-connection — is exactly what the blueprint requires;
//! this is simply the real, verified spelling of it.

use aes::Aes128;
use cfb8::cipher::KeyIvInit;
use cfb8::{Decryptor as Cfb8Decryptor, Encryptor as Cfb8Encryptor};

const SHARED_SECRET_LEN: usize = 16;

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
    cipher: Cfb8Encryptor<Aes128>,
}

impl Aes128Cfb8Encryptor {
    /// `shared_secret` must be exactly 16 bytes (`ServerKeyPair::decrypt_pkcs1v15`'s output on
    /// the client's Encryption Response `shared_secret` field). Used as both the AES-128 key
    /// and the CFB8 initialization vector (Context).
    pub fn new(shared_secret: &[u8]) -> Result<Self, CipherError> {
        if shared_secret.len() != SHARED_SECRET_LEN {
            return Err(CipherError::InvalidSharedSecretLength(shared_secret.len()));
        }
        let cipher = Cfb8Encryptor::<Aes128>::new_from_slices(shared_secret, shared_secret)
            .expect("length already checked above");
        Ok(Self { cipher })
    }

    /// Enciphers `buf` in place, advancing this stream's internal feedback register by exactly
    /// `buf.len()` bytes. Call order across the connection's lifetime must exactly match wire
    /// send order — never re-encrypt, never skip, never reorder a call.
    pub fn encrypt_in_place(&mut self, buf: &mut [u8]) {
        self.cipher.encrypt(buf);
    }
}

/// The decrypt-direction counterpart of `Aes128Cfb8Encryptor` — same construction contract,
/// same persistent-state requirement, applied to inbound bytes in wire arrival order.
pub struct Aes128Cfb8Decryptor {
    cipher: Cfb8Decryptor<Aes128>,
}

impl Aes128Cfb8Decryptor {
    pub fn new(shared_secret: &[u8]) -> Result<Self, CipherError> {
        if shared_secret.len() != SHARED_SECRET_LEN {
            return Err(CipherError::InvalidSharedSecretLength(shared_secret.len()));
        }
        let cipher = Cfb8Decryptor::<Aes128>::new_from_slices(shared_secret, shared_secret)
            .expect("length already checked above");
        Ok(Self { cipher })
    }

    pub fn decrypt_in_place(&mut self, buf: &mut [u8]) {
        self.cipher.decrypt(buf);
    }
}
