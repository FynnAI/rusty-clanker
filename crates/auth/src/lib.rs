//! `rc-auth` — NET-D6's server-side encryption handshake (RSA-1024 keypair, PKCS#1 v1.5 key
//! exchange, AES-128/CFB8 stream setup, the Notchian server-hash algorithm) and Mojang
//! online-mode `hasJoined` session validation, plus the offline-mode UUID derivation NET-D6's
//! non-default offline stance needs. Server-only (`12-workspace-structure.md`); has no Cargo
//! dependency on `rc-protocol` (Context, "Why `rc-auth` never depends on `rc-protocol`") —
//! every type here operates on plain `&[u8]`/`String`/`bool` values, never a wire packet type.

pub mod cipher;
pub mod hash;
pub mod keypair;
pub mod offline;
pub mod session;

pub use cipher::{Aes128Cfb8Decryptor, Aes128Cfb8Encryptor, CipherError};
pub use hash::compute_server_hash;
pub use keypair::{KeyPairError, RSA_KEY_BITS, ServerKeyPair, generate_verify_token};
pub use offline::offline_uuid;
pub use session::{
    HasJoinedProfile, MojangSessionService, ProfileProperty, SessionService, SessionServiceConfig,
    SessionServiceError,
};
