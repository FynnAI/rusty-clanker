/// Byte-stream cipher seam NET-D6's real AES/CFB8 implementation (in `rc-auth`, a future
/// blueprint) plugs into. No implementation exists in this crate or this blueprint.
pub trait ConnectionCipher: Send {
    /// Decrypts `buf` in place. Called by the reader task on exactly the newly-read byte
    /// range, in socket-arrival order, once installed — every byte after installation is
    /// enciphered, matching the reference's own placement.
    fn decrypt(&mut self, buf: &mut [u8]);
    /// Encrypts `buf` in place, called by the writer task on a fully-framed outbound chunk.
    fn encrypt(&mut self, buf: &mut [u8]);
}
