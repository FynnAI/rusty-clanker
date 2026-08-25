//! M1-B05 acceptance test: `HardcodedWorld`'s `PlayerSessionSink` impl -- the one file in
//! this blueprint that constructs M1-B04's `PlayerSession` and proves `accept` actually
//! reaches `enter_play`.

use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one};
use rusty_clanker_server::net::{
    ConnectionConfig, PlayerSession, PlayerSessionSink, spawn_connection,
};
use rusty_clanker_server::play::HardcodedWorld;
use rusty_clanker_server::play::packets::LoginPlay;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) = tokio::join!(listener.accept(), TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    (server, connect_result.unwrap())
}

#[tokio::test]
async fn hardcoded_world_accepts_player_session_and_reaches_play() {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let (server, mut client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());

        let profile = rusty_clanker_server::net::ResolvedProfile {
            id: uuid::Uuid::from_u128(1),
            name: "tester".to_string(),
            properties: Vec::new(),
        };
        let entity_ids = rc_core::RcEntityIdAllocator::default();
        let session = PlayerSession {
            profile,
            entity_id: entity_ids.alloc(),
            connection: handle.clone(),
            inbound,
        };

        let world = HardcodedWorld::new();
        world.accept(session);

        let mut accumulator = bytes::BytesMut::new();
        let (id, body) = loop {
            if let Some(payload) =
                rc_protocol::try_decode_frame(&mut accumulator, CompressionState::Disabled).unwrap()
            {
                let mut b = payload;
                let id = VarInt::decode(&mut b).unwrap().get();
                break (id, b);
            }
            let mut chunk = [0u8; 4096];
            let n = client.read(&mut chunk).await.unwrap();
            assert!(n > 0, "peer closed before a full frame arrived");
            accumulator.extend_from_slice(&chunk[..n]);
        };

        assert_eq!(id, LoginPlay::ID);
        let login_play = decode_one::<LoginPlay>(body).unwrap();
        assert_eq!(login_play.game_mode, 1);
    })
    .await
    .unwrap();
}
