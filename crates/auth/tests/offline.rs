//! M1-B03 acceptance tests: offline-mode UUID derivation known-answer vectors (Context,
//! "Offline-mode stance and UUID derivation").

use rc_auth::offline_uuid;

#[test]
fn known_answer_vectors() {
    assert_eq!(
        offline_uuid("Notch").to_string(),
        "b50ad385-829d-3141-a216-7e7d7539ba7f"
    );
    assert_eq!(
        offline_uuid("Rusty").to_string(),
        "43b8bb75-73b2-363f-a76e-efaccf040b2e"
    );
    assert_eq!(
        offline_uuid("jeb_").to_string(),
        "a762f560-4fce-3236-812a-b80efff0b62b"
    );
}

#[test]
fn offline_uuid_is_deterministic() {
    assert_eq!(offline_uuid("Notch"), offline_uuid("Notch"));
}

#[test]
fn offline_uuid_differs_by_username() {
    assert_ne!(offline_uuid("Notch"), offline_uuid("notch"));
}
