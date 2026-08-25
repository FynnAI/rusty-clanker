//! M1-B03's own scoped manual-verification test: proves `MojangSessionService::has_joined`
//! against Mojang's real, live session server. Never run by `cargo nextest run`/CI by default
//! (`#[ignore]`d) — see the blueprint's "Manual verification procedure" for the exact steps.

#[tokio::test]
#[ignore = "requires a real Mojang session and network access — see this blueprint's Manual verification procedure"]
async fn real_hasjoined_call_against_a_genuine_session() {
    let username = std::env::var("RC_AUTH_MANUAL_USERNAME")
        .expect("set RC_AUTH_MANUAL_USERNAME — see Manual verification procedure");
    let server_hash = std::env::var("RC_AUTH_MANUAL_SERVER_HASH")
        .expect("set RC_AUTH_MANUAL_SERVER_HASH — see Manual verification procedure");
    let service = rc_auth::MojangSessionService::new(rc_auth::SessionServiceConfig::default());
    let result = rc_auth::SessionService::has_joined(&service, &username, &server_hash, None).await;
    match result {
        Ok(Some(profile)) => {
            assert_eq!(profile.name, username);
            println!(
                "hasJoined succeeded: id={}, name={}",
                profile.id, profile.name
            );
        }
        other => {
            panic!("expected Ok(Some(profile)), got {other:?} — see Manual verification procedure")
        }
    }
}
