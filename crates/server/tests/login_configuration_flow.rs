//! M1-B04 acceptance tests: the scripted full Login -> Configuration -> Play byte-sequence
//! exchange against a fake client, reject/timeout paths. Every test uses
//! `ServerLoginConfig{ online_mode: false, .. }`, exercising the offline branch exclusively
//! (Context: the online branch needs a real Mojang session-server round trip, covered only
//! by the blueprint's own manual verification pass).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use rc_protocol::{
    AcknowledgeFinishConfiguration, CompressionState, ConfigurationKeepAliveServerbound,
    ConfigurationPluginMessage, ConnectionState, FinishConfiguration, Identifier, KnownPack,
    KnownPacksClientbound, KnownPacksServerbound, LoginAcknowledged, LoginDisconnect, LoginStart,
    LoginSuccess, RcPacket, RegistryData, SetCompression, UpdateEnabledFeatures, VarInt,
    decode_one, encode_payload,
};
use rusty_clanker_server::net::{
    ConfigurationError, ConnectionConfig, DriveError, LoginError, PlayerSession, PlayerSessionSink,
    ServerConfigurationConfig, ServerLoginConfig, drive_connection, run_login_with_watchdog,
    spawn_connection,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

/// A small, synthetic 2-registry `worldgen_registries` fixture — deliberately not the real
/// generated table (Context/Deliverables' decoupling of `run_configuration` from the
/// manually-codegen'd `crates/registries/generated/v776/registry_entries.rs`).
const TEST_WORLDGEN_REGISTRIES: &[(&str, &[&str])] = &[
    ("minecraft:dimension_type", &["minecraft:overworld"]),
    (
        "minecraft:worldgen/biome",
        &["minecraft:plains", "minecraft:desert"],
    ),
];

fn default_known_pack() -> KnownPack {
    KnownPack {
        namespace: "minecraft".to_string(),
        id: "core".to_string(),
        version: "26.2".to_string(),
    }
}

#[derive(Default)]
struct TestSink {
    sessions: Mutex<Vec<PlayerSession>>,
}
impl PlayerSessionSink for TestSink {
    fn accept(&self, session: PlayerSession) {
        self.sessions.lock().unwrap().push(session);
    }
}

/// Same proven shape as M1-B01/M1-B02's own `connected_pair()` precedent.
async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) = tokio::join!(listener.accept(), TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    (server, connect_result.unwrap())
}

/// Tracks the fake client's own view of the connection's compression state — switched from
/// `Disabled` to `Enabled{256}` by the test itself, immediately after decoding `SetCompression`
/// (which is always sent uncompressed), exactly mirroring what a real client's own codec does.
struct ClientCodec {
    accumulator: BytesMut,
    compression: CompressionState,
}
impl ClientCodec {
    fn new() -> Self {
        Self {
            accumulator: BytesMut::new(),
            compression: CompressionState::Disabled,
        }
    }

    async fn send<P: RcPacket>(&self, socket: &mut TcpStream, packet: &P) {
        self.send_raw(socket, encode_payload(packet)).await;
    }

    async fn send_raw(&self, socket: &mut TcpStream, payload: Bytes) {
        let mut framed = BytesMut::new();
        rc_protocol::encode_frame(&payload, self.compression, &mut framed).unwrap();
        socket.write_all(&framed).await.unwrap();
    }

    async fn recv(&mut self, socket: &mut TcpStream) -> (i32, Bytes) {
        loop {
            if let Some(payload) =
                rc_protocol::try_decode_frame(&mut self.accumulator, self.compression).unwrap()
            {
                let mut body = payload;
                let id = VarInt::decode(&mut body).unwrap().get();
                return (id, body);
            }
            let mut chunk = [0u8; 4096];
            let n = socket.read(&mut chunk).await.unwrap();
            assert!(n > 0, "peer closed before a full frame arrived");
            self.accumulator.extend_from_slice(&chunk[..n]);
        }
    }
}

/// Spawns a loopback connection pair, sets both state slots to `Login` (this blueprint's own
/// starting assumption — a sibling Handshake blueprint normally does this), and drives
/// `drive_connection` on the server half against `TEST_WORLDGEN_REGISTRIES` and an always-
/// offline `ServerLoginConfig`.
async fn spawn_full_drive(
    sink: Arc<TestSink>,
) -> (TcpStream, tokio::task::JoinHandle<Result<(), DriveError>>) {
    let (server, client) = connected_pair().await;
    let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
    handle.set_inbound_state(ConnectionState::Login);
    handle.set_outbound_state(ConnectionState::Login);

    let key_pair = Arc::new(rc_auth::ServerKeyPair::generate().unwrap());
    let sessions = Arc::new(rc_auth::MojangSessionService::new(
        rc_auth::SessionServiceConfig::default(),
    ));
    let entity_ids = Arc::new(rc_core::RcEntityIdAllocator::default());
    let login_config = ServerLoginConfig {
        online_mode: false,
        compression_threshold: 256,
        client_ip: None,
    };
    let configuration_config = ServerConfigurationConfig::default();

    let task = tokio::spawn(drive_connection(
        inbound,
        handle,
        key_pair,
        sessions,
        entity_ids,
        login_config,
        configuration_config,
        TEST_WORLDGEN_REGISTRIES,
        sink,
    ));
    (client, task)
}

/// Drives one fake client through a fully successful Login sequence, leaving `codec` set to
/// `Enabled{256}` and positioned right after sending `LoginAcknowledged`.
async fn drive_login(codec: &mut ClientCodec, client: &mut TcpStream, name: &str) {
    codec
        .send(
            client,
            &LoginStart {
                name: name.to_string(),
                player_uuid: Uuid::nil(),
            },
        )
        .await;

    let (id, body) = codec.recv(client).await;
    assert_eq!(id, SetCompression::ID);
    let set_compression = decode_one::<SetCompression>(body).unwrap();
    assert_eq!(set_compression.threshold, 256);
    codec.compression = CompressionState::Enabled { threshold: 256 };

    let (id, body) = codec.recv(client).await;
    assert_eq!(id, LoginSuccess::ID);
    let login_success = decode_one::<LoginSuccess>(body).unwrap();
    assert_eq!(login_success.profile.name, name);

    codec.send(client, &LoginAcknowledged {}).await;
}

/// Drives one fake client through the Configuration setup packets (brand, feature flags,
/// known-packs offer) and replies with the matching known-pack — leaving the connection
/// positioned right before the server's registry-data sync.
async fn drive_configuration_setup(codec: &mut ClientCodec, client: &mut TcpStream) {
    let (id, body) = codec.recv(client).await;
    assert_eq!(id, ConfigurationPluginMessage::ID);
    let brand = decode_one::<ConfigurationPluginMessage>(body).unwrap();
    assert_eq!(brand.channel, Identifier::new("minecraft:brand"));

    let (id, body) = codec.recv(client).await;
    assert_eq!(id, UpdateEnabledFeatures::ID);
    let features = decode_one::<UpdateEnabledFeatures>(body).unwrap();
    assert_eq!(
        features.features,
        vec![Identifier::new("minecraft:vanilla")]
    );

    let (id, body) = codec.recv(client).await;
    assert_eq!(id, KnownPacksClientbound::ID);
    let known_packs = decode_one::<KnownPacksClientbound>(body).unwrap();
    assert_eq!(known_packs.known_packs, vec![default_known_pack()]);

    codec
        .send(
            client,
            &KnownPacksServerbound {
                known_packs: vec![default_known_pack()],
            },
        )
        .await;
}

#[tokio::test]
async fn full_login_configuration_play_handoff_offline_mode() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let sink = Arc::new(TestSink::default());
        let (mut client, task) = spawn_full_drive(sink.clone()).await;
        let mut codec = ClientCodec::new();

        drive_login(&mut codec, &mut client, "TestPlayer").await;
        drive_configuration_setup(&mut codec, &mut client).await;

        for (registry_id, entries) in TEST_WORLDGEN_REGISTRIES {
            let (id, body) = codec.recv(&mut client).await;
            assert_eq!(id, RegistryData::ID);
            let registry_data = decode_one::<RegistryData>(body).unwrap();
            assert_eq!(registry_data.registry_id, Identifier::new(*registry_id));
            assert_eq!(registry_data.entries.len(), entries.len());
        }

        let (id, body) = codec.recv(&mut client).await;
        assert_eq!(id, FinishConfiguration::ID);
        decode_one::<FinishConfiguration>(body).unwrap();

        codec
            .send(&mut client, &AcknowledgeFinishConfiguration {})
            .await;

        let result = task.await.unwrap();
        assert!(
            result.is_ok(),
            "drive_connection should succeed: {result:?}"
        );

        let sessions = sink.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].profile.name, "TestPlayer");
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn login_rejects_invalid_username() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let sink = Arc::new(TestSink::default());
        let (mut client, task) = spawn_full_drive(sink).await;
        let mut codec = ClientCodec::new();

        codec
            .send(
                &mut client,
                &LoginStart {
                    // Space and `!`, both outside `[a-zA-Z0-9_]`.
                    name: "bad name!".to_string(),
                    player_uuid: Uuid::nil(),
                },
            )
            .await;

        let (id, body) = codec.recv(&mut client).await;
        assert_eq!(id, LoginDisconnect::ID);
        decode_one::<LoginDisconnect>(body).unwrap();

        let mut buf = [0u8; 16];
        let n = client.read(&mut buf).await.unwrap();
        assert_eq!(n, 0, "socket should close after an invalid username");

        let result = task.await.unwrap();
        match result {
            Err(DriveError::Login(LoginError::InvalidName(name))) => {
                assert_eq!(name, "bad name!");
            }
            other => {
                panic!("expected DriveError::Login(LoginError::InvalidName(_)), got {other:?}")
            }
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn login_watchdog_times_out() {
    let short_watchdog = Duration::from_millis(300);
    tokio::time::timeout(short_watchdog * 10, async {
        let (server, mut client) = connected_pair().await;
        let (mut inbound, handle) = spawn_connection(server, ConnectionConfig::default());
        handle.set_inbound_state(ConnectionState::Login);
        handle.set_outbound_state(ConnectionState::Login);

        let key_pair = rc_auth::ServerKeyPair::generate().unwrap();
        let sessions = rc_auth::MojangSessionService::new(rc_auth::SessionServiceConfig::default());
        let login_config = ServerLoginConfig {
            online_mode: false,
            compression_threshold: 256,
            client_ip: None,
        };

        let watchdog_task = tokio::spawn(async move {
            run_login_with_watchdog(
                &mut inbound,
                &handle,
                &key_pair,
                &sessions,
                &login_config,
                short_watchdog,
            )
            .await
        });

        let mut codec = ClientCodec::new();
        codec
            .send(
                &mut client,
                &LoginStart {
                    name: "TestPlayer".to_string(),
                    player_uuid: Uuid::nil(),
                },
            )
            .await;
        let (id, _) = codec.recv(&mut client).await;
        assert_eq!(id, SetCompression::ID);
        codec.compression = CompressionState::Enabled { threshold: 256 };
        let (id, _) = codec.recv(&mut client).await;
        assert_eq!(id, LoginSuccess::ID);

        // Deliberately never send LoginAcknowledged — the watchdog must fire.
        let result = watchdog_task.await.unwrap();
        match result {
            Err(LoginError::Timeout(duration)) => assert_eq!(duration, short_watchdog),
            other => panic!("expected Err(LoginError::Timeout(_)), got {other:?}"),
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn configuration_rejects_known_pack_mismatch() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let sink = Arc::new(TestSink::default());
        let (mut client, task) = spawn_full_drive(sink).await;
        let mut codec = ClientCodec::new();

        drive_login(&mut codec, &mut client, "TestPlayer").await;

        // Drain the three Configuration setup packets without replying to them yet.
        let (id, _) = codec.recv(&mut client).await;
        assert_eq!(id, ConfigurationPluginMessage::ID);
        let (id, _) = codec.recv(&mut client).await;
        assert_eq!(id, UpdateEnabledFeatures::ID);
        let (id, _) = codec.recv(&mut client).await;
        assert_eq!(id, KnownPacksClientbound::ID);

        codec
            .send(
                &mut client,
                &KnownPacksServerbound {
                    known_packs: vec![KnownPack {
                        namespace: "minecraft".to_string(),
                        id: "core".to_string(),
                        version: "1.0".to_string(),
                    }],
                },
            )
            .await;

        let result = task.await.unwrap();
        assert!(
            matches!(
                result,
                Err(DriveError::Configuration(
                    ConfigurationError::KnownPackMismatch
                ))
            ),
            "expected KnownPackMismatch, got {result:?}"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn configuration_ignores_unsolicited_plugin_message_and_keep_alive_reply() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let sink = Arc::new(TestSink::default());
        let (mut client, task) = spawn_full_drive(sink).await;
        let mut codec = ClientCodec::new();

        drive_login(&mut codec, &mut client, "TestPlayer").await;

        let (id, _) = codec.recv(&mut client).await;
        assert_eq!(id, ConfigurationPluginMessage::ID);
        let (id, _) = codec.recv(&mut client).await;
        assert_eq!(id, UpdateEnabledFeatures::ID);
        let (id, _) = codec.recv(&mut client).await;
        assert_eq!(id, KnownPacksClientbound::ID);

        // Extra, unsolicited Serverbound Plugin Message (id 0x02) — not a defined Rust type
        // in this blueprint's own catalog (Constraints (e)), hand-encoded.
        let mut plugin_message_payload = BytesMut::new();
        VarInt::new(0x02).encode(&mut plugin_message_payload);
        plugin_message_payload.extend_from_slice(b"whatever-channel-data");
        codec
            .send_raw(&mut client, plugin_message_payload.freeze())
            .await;

        // Extra, unsolicited Keep Alive reply with a bogus id — no challenge is pending yet
        // (the server has not sent its own first keep-alive challenge), so this must be
        // silently dropped, not treated as a mismatch (Context).
        codec
            .send(
                &mut client,
                &ConfigurationKeepAliveServerbound {
                    keep_alive_id: 999_999,
                },
            )
            .await;

        codec
            .send(
                &mut client,
                &KnownPacksServerbound {
                    known_packs: vec![default_known_pack()],
                },
            )
            .await;

        for _ in TEST_WORLDGEN_REGISTRIES {
            let (id, body) = codec.recv(&mut client).await;
            assert_eq!(id, RegistryData::ID);
            decode_one::<RegistryData>(body).unwrap();
        }

        let (id, body) = codec.recv(&mut client).await;
        assert_eq!(id, FinishConfiguration::ID);
        decode_one::<FinishConfiguration>(body).unwrap();

        codec
            .send(&mut client, &AcknowledgeFinishConfiguration {})
            .await;

        let result = task.await.unwrap();
        assert!(
            result.is_ok(),
            "unsolicited plugin message / keep-alive reply must not break the sequence: {result:?}"
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn player_session_carries_inbound_receiver_still_configuration_state() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let sink = Arc::new(TestSink::default());
        let (mut client, task) = spawn_full_drive(sink.clone()).await;
        let mut codec = ClientCodec::new();

        drive_login(&mut codec, &mut client, "TestPlayer").await;
        drive_configuration_setup(&mut codec, &mut client).await;

        for _ in TEST_WORLDGEN_REGISTRIES {
            let (id, body) = codec.recv(&mut client).await;
            assert_eq!(id, RegistryData::ID);
            decode_one::<RegistryData>(body).unwrap();
        }

        let (id, body) = codec.recv(&mut client).await;
        assert_eq!(id, FinishConfiguration::ID);
        decode_one::<FinishConfiguration>(body).unwrap();

        codec.send(&mut client, &AcknowledgeFinishConfiguration {}).await;

        let result = task.await.unwrap();
        assert!(result.is_ok());

        let sessions = sink.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            sessions[0].connection.inbound_state(),
            ConnectionState::Configuration,
            "inbound must stay Configuration until a later blueprint's player-spawn setup advances it"
        );
        assert_eq!(
            sessions[0].connection.outbound_state(),
            ConnectionState::Play,
            "outbound flips to Play only once AcknowledgeFinishConfiguration has arrived"
        );
    })
    .await
    .unwrap();
}
