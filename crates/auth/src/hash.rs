//! The Notchian server hash (NET-D6, Context "The Notchian server hash — exact algorithm and
//! verified test vectors"): SHA-1 of `server_id ++ shared_secret ++ server_public_key_der`,
//! reinterpreted as a signed two's-complement big integer and hex-encoded.

/// Computes the Notchian server hash (NET-D6): SHA-1 of `server_id ++ shared_secret ++
/// server_public_key_der`, reinterpreted as a signed two's-complement big integer and
/// hex-encoded (Context, "The Notchian server hash — exact algorithm and verified test
/// vectors," which this function implements exactly).
pub fn compute_server_hash(
    server_id: &str,
    shared_secret: &[u8],
    server_public_key_der: &[u8],
) -> String {
    todo!()
}
