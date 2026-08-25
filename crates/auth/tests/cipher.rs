//! M1-B03 acceptance tests: AES-128/CFB8 known-answer vectors and the persistent-state
//! (never-reconstructed) requirement (Context, "AES-128/CFB8 stream setup — exact parameters
//! and the persistent-state requirement"). All ciphertext columns are independently computed
//! via `openssl enc -aes-128-cfb8`, not this project's own code.

use rc_auth::{Aes128Cfb8Decryptor, Aes128Cfb8Encryptor, CipherError};

struct Vector {
    key: &'static [u8],
    plaintext: &'static [u8],
    ciphertext: &'static [u8],
}

fn vectors() -> Vec<Vector> {
    vec![
        Vector {
            key: &[
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f,
            ],
            // NOTE (M1-B03 reconciliation): the blueprint's own transcription of this vector's
            // ciphertext hex string is 63 hex characters (odd length, one nibble short — not a
            // representable byte sequence). Independently re-derived byte-for-byte via
            // `openssl enc -aes-128-cfb8 -K <key> -iv <key> -nopad` against this exact
            // plaintext/key pair (the same oracle command the blueprint's Context cites) during
            // implementation; the corrected 32-byte value below is what that oracle actually
            // produces.
            plaintext: b"Hello, Rusty Clanker! 0123456789",
            ciphertext: &[
                0x42, 0xea, 0x5e, 0xd4, 0xda, 0xf8, 0x64, 0xf5, 0x13, 0xc3, 0x54, 0x96, 0x1a, 0xd8,
                0x2a, 0x29, 0x90, 0xa2, 0x6e, 0x64, 0xe0, 0x53, 0x4a, 0x92, 0x0b, 0x3a, 0xfa, 0x4c,
                0x71, 0x96, 0x9c, 0xfa,
            ],
        },
        Vector {
            key: &[
                0xa1, 0xf0, 0x3c, 0x77, 0x9b, 0x22, 0x04, 0xe8, 0x5d, 0x61, 0xaa, 0x10, 0xf3, 0x4b,
                0x88, 0x02,
            ],
            plaintext: b"Rusty",
            ciphertext: &[0x39, 0x84, 0x78, 0xa5, 0x5c],
        },
        Vector {
            key: &[
                0xa1, 0xf0, 0x3c, 0x77, 0x9b, 0x22, 0x04, 0xe8, 0x5d, 0x61, 0xaa, 0x10, 0xf3, 0x4b,
                0x88, 0x02,
            ],
            plaintext: b"",
            ciphertext: &[],
        },
        Vector {
            key: &[
                0xa1, 0xf0, 0x3c, 0x77, 0x9b, 0x22, 0x04, 0xe8, 0x5d, 0x61, 0xaa, 0x10, 0xf3, 0x4b,
                0x88, 0x02,
            ],
            plaintext: b"X",
            ciphertext: &[0x33],
        },
    ]
}

#[test]
fn known_answer_encrypt_vectors() {
    for v in vectors() {
        let mut encryptor = Aes128Cfb8Encryptor::new(v.key).unwrap();
        let mut buf = v.plaintext.to_vec();
        encryptor.encrypt_in_place(&mut buf);
        assert_eq!(buf, v.ciphertext);
    }
}

#[test]
fn known_answer_decrypt_vectors() {
    for v in vectors() {
        let mut decryptor = Aes128Cfb8Decryptor::new(v.key).unwrap();
        let mut buf = v.ciphertext.to_vec();
        decryptor.decrypt_in_place(&mut buf);
        assert_eq!(buf, v.plaintext);
    }
}

#[test]
fn cipher_split_calls_match_single_call() {
    let key: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f,
    ];
    let plaintext: [u8; 30] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30,
    ];

    let mut single_shot = Aes128Cfb8Encryptor::new(&key).unwrap();
    let mut single_buf = plaintext;
    single_shot.encrypt_in_place(&mut single_buf);

    let mut split = Aes128Cfb8Encryptor::new(&key).unwrap();
    let mut split_buf = plaintext;
    split.encrypt_in_place(&mut split_buf[0..7]);
    split.encrypt_in_place(&mut split_buf[7..19]);
    split.encrypt_in_place(&mut split_buf[19..30]);

    assert_eq!(single_buf, split_buf);
}

#[test]
fn new_rejects_wrong_length_shared_secret() {
    assert!(matches!(
        Aes128Cfb8Encryptor::new(&[0u8; 15]),
        Err(CipherError::InvalidSharedSecretLength(15))
    ));
    assert!(matches!(
        Aes128Cfb8Encryptor::new(&[0u8; 17]),
        Err(CipherError::InvalidSharedSecretLength(17))
    ));
    assert!(matches!(
        Aes128Cfb8Decryptor::new(&[0u8; 15]),
        Err(CipherError::InvalidSharedSecretLength(15))
    ));
    assert!(matches!(
        Aes128Cfb8Decryptor::new(&[0u8; 17]),
        Err(CipherError::InvalidSharedSecretLength(17))
    ));
}

proptest::proptest! {
    #[test]
    fn proptest_round_trip_arbitrary_buffer(
        data in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..=2048),
        key in proptest::prelude::any::<[u8; 16]>(),
    ) {
        let mut encryptor = Aes128Cfb8Encryptor::new(&key).unwrap();
        let mut buf = data.clone();
        encryptor.encrypt_in_place(&mut buf);

        let mut decryptor = Aes128Cfb8Decryptor::new(&key).unwrap();
        decryptor.decrypt_in_place(&mut buf);

        proptest::prop_assert_eq!(buf, data);
    }
}
