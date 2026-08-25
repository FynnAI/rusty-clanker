//! M1-B03 acceptance tests: the Notchian server-hash algorithm's known-answer vectors
//! (Context, "The Notchian server hash — exact algorithm and verified test vectors").

use rc_auth::compute_server_hash;

#[test]
fn known_answer_vectors() {
    let cases: &[(&str, &str)] = &[
        ("Notch", "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48"),
        ("jeb_", "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1"),
        ("simon", "88e16a1019277b15d58faf0541e11910eb756f6"),
        ("", "-25c65c11a194b4f2cdaa40106a9fe76f5027f8f7"),
    ];

    for (server_id, expected) in cases {
        let actual = compute_server_hash(server_id, b"", b"");
        assert_eq!(&actual, expected, "server_id={server_id:?}");
    }
}

#[test]
fn hash_changes_when_any_input_changes() {
    let base = compute_server_hash("a", b"s1", b"k1");
    assert_ne!(base, compute_server_hash("b", b"s1", b"k1"));
    assert_ne!(base, compute_server_hash("a", b"s2", b"k1"));
    assert_ne!(base, compute_server_hash("a", b"s1", b"k2"));
}
