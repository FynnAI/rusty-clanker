//! The Notchian server hash (NET-D6, Context "The Notchian server hash — exact algorithm and
//! verified test vectors"): SHA-1 of `server_id ++ shared_secret ++ server_public_key_der`,
//! reinterpreted as a signed two's-complement big integer and hex-encoded.

use sha1::{Digest, Sha1};

/// Computes the Notchian server hash (NET-D6): SHA-1 of `server_id ++ shared_secret ++
/// server_public_key_der`, reinterpreted as a signed two's-complement big integer and
/// hex-encoded (Context, "The Notchian server hash — exact algorithm and verified test
/// vectors," which this function implements exactly).
pub fn compute_server_hash(
    server_id: &str,
    shared_secret: &[u8],
    server_public_key_der: &[u8],
) -> String {
    let mut hasher = Sha1::new();
    hasher.update(server_id.as_bytes());
    hasher.update(shared_secret);
    hasher.update(server_public_key_der);
    let mut magnitude: [u8; 20] = hasher.finalize().into();

    let negative = (magnitude[0] & 0x80) != 0;
    if negative {
        // Two's-complement negate the 20-byte big-endian value: invert every bit, then add 1.
        for byte in magnitude.iter_mut() {
            *byte = !*byte;
        }
        let mut carry: u16 = 1;
        for byte in magnitude.iter_mut().rev() {
            let sum = *byte as u16 + carry;
            *byte = sum as u8;
            carry = sum >> 8;
        }
    }

    let hex: String = magnitude.iter().map(|b| format!("{b:02x}")).collect();
    let trimmed = hex.trim_start_matches('0');
    let trimmed = if trimmed.is_empty() { "0" } else { trimmed };

    if negative {
        format!("-{trimmed}")
    } else {
        trimmed.to_string()
    }
}
