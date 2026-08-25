//! M1-B03 acceptance tests: `ServerKeyPair`'s generation, DER export, and PKCS#1 v1.5
//! decryption round-trip (Context, "RSA keypair — lifecycle, size, and DER export").

use rc_auth::{KeyPairError, ServerKeyPair};

#[test]
fn generate_produces_der_encoded_public_key_of_expected_length() {
    let keys = ServerKeyPair::generate().unwrap();
    assert_eq!(keys.public_key_der().len(), 162);
}

#[test]
fn two_generated_keypairs_have_different_public_keys() {
    let a = ServerKeyPair::generate().unwrap();
    let b = ServerKeyPair::generate().unwrap();
    assert_ne!(a.public_key_der(), b.public_key_der());
}

#[test]
fn pkcs1v15_round_trip_via_reconstructed_public_key() {
    let keys = ServerKeyPair::generate().unwrap();

    let public_key: rsa::RsaPublicKey =
        rsa::pkcs8::DecodePublicKey::from_public_key_der(keys.public_key_der())
            .expect("public_key_der must round-trip through DecodePublicKey");

    let plaintext: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F,
    ];
    let ciphertext = public_key
        .encrypt(&mut rsa::rand_core::OsRng, rsa::Pkcs1v15Encrypt, &plaintext)
        .expect("encryption against the reconstructed public key must succeed");

    let decrypted = keys.decrypt_pkcs1v15(&ciphertext).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn decrypt_pkcs1v15_rejects_non_matching_ciphertext() {
    let keys = ServerKeyPair::generate().unwrap();
    let result = keys.decrypt_pkcs1v15(&[0u8; 128]);
    assert!(matches!(result, Err(KeyPairError::Decryption(_))));
}

#[test]
fn generate_verify_token_produces_distinct_tokens_across_calls() {
    let a = rc_auth::generate_verify_token();
    let b = rc_auth::generate_verify_token();
    assert_ne!(a, b);
}
