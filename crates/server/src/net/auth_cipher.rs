use rc_auth::cipher::{Aes128Cfb8Decryptor, Aes128Cfb8Encryptor, CipherError};
use rc_protocol::ConnectionCipher;

/// Wraps `rc-auth`'s plain, `rc-protocol`-free AES-128/CFB8 primitives to satisfy
/// `rc_protocol::ConnectionCipher` (M1-B01's seam) — the one file in this blueprint that
/// imports both `rc_auth` and `rc_protocol` types together, exactly because
/// `rusty-clanker-server` is the only crate depending on both (Context, "Why `rc-auth` never
/// depends on `rc-protocol`").
pub struct AuthConnectionCipher {
    encryptor: Aes128Cfb8Encryptor,
    decryptor: Aes128Cfb8Decryptor,
}

impl AuthConnectionCipher {
    /// `shared_secret` must be exactly 16 bytes — the value `rc_auth::ServerKeyPair::
    /// decrypt_pkcs1v15` produces from the client's Encryption Response (Context). Both
    /// directions are constructed from the same shared secret (Context: key = IV = shared
    /// secret, both directions).
    pub fn new(shared_secret: &[u8]) -> Result<Self, CipherError> {
        Ok(Self {
            encryptor: Aes128Cfb8Encryptor::new(shared_secret)?,
            decryptor: Aes128Cfb8Decryptor::new(shared_secret)?,
        })
    }
}

impl ConnectionCipher for AuthConnectionCipher {
    fn decrypt(&mut self, buf: &mut [u8]) {
        self.decryptor.decrypt_in_place(buf);
    }

    fn encrypt(&mut self, buf: &mut [u8]) {
        self.encryptor.encrypt_in_place(buf);
    }
}
