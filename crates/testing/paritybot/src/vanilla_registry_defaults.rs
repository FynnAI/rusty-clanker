//! A tiny, purpose-built man-in-the-middle relay that gives azalea (this crate's own
//! vanilla-client stand-in, `idle_stability.rs`'s own module doc comment) exactly one thing
//! a real vanilla client has that azalea does not: a built-in default for the
//! `minecraft:dimension_type`/`minecraft:overworld` synchronized-registry entry.
//!
//! Scope, deliberately narrow: `rusty-clanker-server`'s own Configuration-phase registry
//! sync (M1 registry-sync fix) now sends every entry of every synchronized registry with
//! `has_data=false` -- the correct 1.20.5+ semantics, where the client is expected to
//! already have its own built-in definition for a stock vanilla name. A real vanilla client
//! does ship such built-in defaults (loaded from its own local "generated" data at startup);
//! azalea's own `RegistryHolder` (the pinned rev's `azalea-core/src/registry_holder/mod.rs`)
//! does not -- confirmed directly in its source: `RegistryType::append_nbt` treats a
//! `has_data=false` entry (`None`) as `map.shift_remove(&key)`, and `RegistryHolder::default()`
//! starts with every map empty, with no compiled-in vanilla fallback data anywhere in the
//! crate. `CommonPlayerSpawnInfo::dimension_type` (`azalea-protocol/src/packets/common.rs`)
//! therefore fails to resolve `minecraft:overworld`, which makes azalea's own `login()`
//! packet handler (`azalea-client/src/plugins/packet/game/mod.rs`) return before inserting
//! `MinecraftEntityId`, so neither `Event::Login` nor `Event::Spawn` ever fires -- an azalea
//! limitation, not a real-client one, and not something a well-behaved production server
//! should work around by sending non-vanilla payloads.
//!
//! This module closes exactly that one gap, entirely on the test-oracle side: it sits
//! between azalea and the real server, forwards every byte unchanged in both directions,
//! except that it rewrites the one `minecraft:dimension_type` registry entry azalea's own
//! code path actually resolves (`minecraft:overworld`) from `has_data=false` back to
//! `has_data=true` with a minimal, azalea-sufficient inline payload -- standing in for the
//! built-in default a real client would have supplied from its own local data instead of
//! trusting the wire. No other registry, and no other `dimension_type` entry
//! (`overworld_caves`/`the_nether`/`the_end`, never resolved by this scenario), is touched.
//! Zero server or protocol code changes; this is test-oracle infrastructure only.

use bytes::{BufMut, Bytes, BytesMut};
use rc_protocol::{
    CompressionState, FinishConfiguration, Identifier, LoginSuccess, RcPacket, RegistryData,
    SetCompression, VarInt, decode_one, encode_frame, encode_payload, try_decode_frame,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// A running relay instance. Dropping this does not stop the relay (it has no shutdown API,
/// matching `HardcodedWorld::new`'s own "no shutdown API in this blueprint's scope"
/// precedent) -- fine for a short-lived scenario-runner subprocess whose whole process exits
/// once `run_idle_stability_scenario` returns.
pub struct RelayHandle {
    pub local_addr: std::net::SocketAddr,
}

/// Binds a relay on `127.0.0.1:0` and returns immediately with its bound address; every
/// accepted client connection is relayed to `upstream_host:upstream_port` on its own spawned
/// task (never just one -- `azalea::ClientBuilder::start`'s own infinite-retry behavior,
/// `idle_stability.rs`'s module doc comment, means more than one connection attempt is a
/// real possibility this relay must keep serving correctly).
pub async fn spawn(upstream_host: String, upstream_port: u16) -> std::io::Result<RelayHandle> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;

    tokio::spawn(async move {
        loop {
            let (client_stream, _) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(err) => {
                    eprintln!("vanilla_registry_defaults relay: accept failed: {err}");
                    return;
                }
            };
            let upstream_host = upstream_host.clone();
            tokio::spawn(async move {
                if let Err(err) =
                    relay_one_connection(client_stream, &upstream_host, upstream_port).await
                {
                    eprintln!("vanilla_registry_defaults relay: connection ended: {err}");
                }
            });
        }
    });

    Ok(RelayHandle { local_addr })
}

async fn relay_one_connection(
    client_stream: TcpStream,
    upstream_host: &str,
    upstream_port: u16,
) -> std::io::Result<()> {
    let upstream_stream = TcpStream::connect((upstream_host, upstream_port)).await?;
    let (client_read, client_write) = client_stream.into_split();
    let (upstream_read, upstream_write) = upstream_stream.into_split();

    // Client -> server: never rewritten, so a plain byte-for-byte pump suffices -- no
    // framing/compression decode needed on this side at all.
    let client_to_upstream = async move {
        let mut client_read = client_read;
        let mut upstream_write = upstream_write;
        let _ = tokio::io::copy(&mut client_read, &mut upstream_write).await;
    };

    // Server -> client: the one direction that ever needs a byte rewritten, so this side
    // has to speak real frames.
    let upstream_to_client = pump_and_rewrite(upstream_read, client_write);

    tokio::join!(client_to_upstream, upstream_to_client);
    Ok(())
}

/// Which `has_data=false`-vs-`has_data=true` packet-id meaning is currently in effect on the
/// server -> client stream -- tracked purely by observing that same stream's own contents
/// (`LoginSuccess` is always the last thing this project's own `net::login_flow` sends in
/// Login state; `FinishConfiguration` is always the last thing `net::configuration_flow`
/// sends in Configuration state), never by decoding the client -> server direction, which
/// this relay never inspects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Login,
    Configuration,
    Play,
}

async fn pump_and_rewrite(
    mut upstream_read: tokio::net::tcp::OwnedReadHalf,
    mut client_write: tokio::net::tcp::OwnedWriteHalf,
) {
    let mut accumulator = BytesMut::new();
    let mut read_chunk = [0u8; 8192];
    let mut compression = CompressionState::Disabled;
    let mut phase = Phase::Login;

    loop {
        let n = match upstream_read.read(&mut read_chunk).await {
            Ok(0) => return, // upstream closed cleanly
            Ok(n) => n,
            Err(err) => {
                eprintln!("vanilla_registry_defaults relay: upstream read failed: {err}");
                return;
            }
        };
        accumulator.extend_from_slice(&read_chunk[..n]);

        loop {
            let decode_compression = compression;
            let payload = match try_decode_frame(&mut accumulator, decode_compression) {
                Ok(Some(payload)) => payload,
                Ok(None) => break, // need more bytes for a complete frame
                Err(err) => {
                    eprintln!("vanilla_registry_defaults relay: frame decode failed: {err}");
                    return;
                }
            };

            let (rewritten, next_phase, next_compression) =
                process_one_packet(&payload, phase, compression);
            phase = next_phase;

            let mut framed = BytesMut::new();
            // The compression state a frame is *encoded* under is always the state that was
            // active when it arrived, never the state a packet inside it just negotiated
            // (`SetCompression` itself is always sent uncompressed, mirroring `net::
            // login_flow`'s own "Set Compression, before Login Success -- always sent
            // uncompressed" ordering) -- `next_compression` only takes effect starting with
            // the *next* frame.
            if let Err(err) = encode_frame(&rewritten, decode_compression, &mut framed) {
                eprintln!("vanilla_registry_defaults relay: frame re-encode failed: {err}");
                return;
            }
            compression = next_compression;

            if let Err(err) = client_write.write_all(&framed).await {
                eprintln!("vanilla_registry_defaults relay: client write failed: {err}");
                return;
            }
        }
    }
}

/// Decides, for one already-frame-decoded server -> client `payload` (packet-id `VarInt`
/// plus body, exactly `try_decode_frame`'s own return shape), what to actually forward, plus
/// the `Phase`/`CompressionState` that should apply to whatever frame comes *after* this one.
/// Any decode hiccup on a packet this function does inspect falls back to forwarding
/// `payload` completely unchanged -- never worse than this relay simply not existing.
fn process_one_packet(
    payload: &Bytes,
    phase: Phase,
    compression: CompressionState,
) -> (Bytes, Phase, CompressionState) {
    let mut probe = payload.clone();
    let Ok(id) = VarInt::decode(&mut probe).map(|v| v.get()) else {
        return (payload.clone(), phase, compression);
    };

    match phase {
        Phase::Login if id == SetCompression::ID => {
            let next_compression = decode_one::<SetCompression>(probe)
                .map(|pkt| CompressionState::Enabled {
                    threshold: pkt.threshold as u32,
                })
                .unwrap_or(compression);
            (payload.clone(), phase, next_compression)
        }
        Phase::Login if id == LoginSuccess::ID => {
            (payload.clone(), Phase::Configuration, compression)
        }
        Phase::Configuration if id == RegistryData::ID => {
            let rewritten = rewrite_dimension_type_entry(probe).unwrap_or_else(|| payload.clone());
            (rewritten, phase, compression)
        }
        Phase::Configuration if id == FinishConfiguration::ID => {
            (payload.clone(), Phase::Play, compression)
        }
        _ => (payload.clone(), phase, compression),
    }
}

const DIMENSION_TYPE_REGISTRY: &str = "minecraft:dimension_type";
const FALLBACK_ENTRY: &str = "minecraft:overworld";

/// `Some(rewritten_payload)` iff `body` decodes as a `RegistryData` for
/// `minecraft:dimension_type` and its `minecraft:overworld` entry was `has_data=false` --
/// rewritten to carry `overworld_fallback_nbt()` instead. `None` (including any decode
/// failure) leaves the caller to forward the original bytes unchanged.
fn rewrite_dimension_type_entry(body: Bytes) -> Option<Bytes> {
    let mut registry = decode_one::<RegistryData>(body).ok()?;
    if registry.registry_id != Identifier::new(DIMENSION_TYPE_REGISTRY) {
        return None;
    }
    let target = Identifier::new(FALLBACK_ENTRY);
    let mut changed = false;
    for entry in &mut registry.entries {
        if entry.entry_id == target && entry.data.is_none() {
            entry.data = Some(overworld_fallback_nbt());
            changed = true;
        }
    }
    if !changed {
        return None;
    }
    Some(encode_payload(&registry))
}

/// The minimal network-NBT (unnamed root `TAG_Compound`, `TAG_End`-terminated)
/// `DimensionKindElement` compound azalea's own pinned-rev struct
/// (`azalea-core/src/registry_holder/dimension_type.rs`, the default non-`strict_registry`
/// build) needs to resolve `dimension_type` at all: `height: u32`, `min_y: i32` are its only
/// non-`Option` fields (`ultrawarm` is `Option`, everything else lands in its own
/// `#[simdnbt(flatten)] _extra: HashMap<String, NbtTag>` catch-all) -- this is azalea's own
/// documented minimum, not a further approximation of it. `384`/`-64` are real vanilla's own
/// well-established, stable `minecraft:overworld` `height`/`min_y` values -- unaffected by
/// this pinned version's broader `dimension_type` codec rework (`environment_attribute`,
/// `play::world::SYNCHRONIZED_REGISTRIES`'s own doc comment has the full writeup); this
/// project's own `crates/server/src/play/chunk.rs` (`SECTION_COUNT * 16`, `WORLD_MIN_Y`) and
/// `crates/testing/test-harness/src/fake_server.rs`'s own `encode_dimension_type_nbt`
/// independently already carry these same two numbers. Not shared as a common constant
/// across those three crates -- deliberately duplicated, since this crate stays free of a
/// dependency edge on either `rusty-clanker-server` or `rc-test-harness`.
fn overworld_fallback_nbt() -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(0x0A); // root TAG_Compound, unnamed (network NBT).

    buf.put_u8(0x03); // TAG_Int
    buf.put_u16(6);
    buf.put_slice(b"height");
    buf.put_i32(384);

    buf.put_u8(0x03); // TAG_Int
    buf.put_u16(5);
    buf.put_slice(b"min_y");
    buf.put_i32(-64);

    buf.put_u8(0x00); // TAG_End
    buf.to_vec()
}
