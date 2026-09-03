//! An in-process, scripted "fake server" test double (Context, "Fake-server protocol
//! cheat sheet"). Every clientbound packet this module can send is hand-encoded with
//! `rc_protocol`'s frame/varint/wire primitives directly — the same toolset `probe.rs`
//! uses — never `rc_protocol`'s own `RcPacket` catalog types, matching this crate's own
//! "no packet-catalog machinery" stance.
//!
//! Field shapes for the Login/Play packets below were verified empirically against
//! the pinned `azalea` rev's own source (`azalea-protocol/src/packets/{game/c_login.rs,
//! common.rs}`, `azalea-buf`'s per-type `AzBuf` impls) — the strongest correctness
//! guarantee available short of a real `rusty-clanker-server` (Constraints (d) sanctions
//! reading azalea's own source this way: it is a *client* library, not a server
//! reimplementation, so ASSET-D30's firewall does not apply). `SendLoginSuccess`'s
//! shape matches `rc_protocol::login::{LoginSuccess, LoginProfile, LoginProfileProperty}`
//! (M1-B04). `SendPlayLogin`'s shape is `ClientboundLogin`'s real field list --
//! `player_id`/`hardcore`/`levels`/`max_players`/`chunk_radius`/`simulation_distance`/
//! `reduced_debug_info`/`show_death_screen`/`do_limited_crafting`/`common`
//! (`CommonPlayerSpawnInfo`, inline, no sub-framing)/`online_mode`/`enforces_secure_chat`
//! -- confirming the cheat sheet's own inclusion of `online_mode` was correct.
//! **Discovered discrepancy, out of this blueprint's scope to fix:** the already-shipped
//! `rusty_clanker_server::play::packets::LoginPlay` (M1-B05) omits `online_mode`
//! entirely -- a real gap against azalea's own confirmed wire shape, flagged in this
//! blueprint's own report as an open problem for M1-B05, not corrected here (M1-B05's
//! own files are outside this blueprint's assigned scope).

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rc_protocol::{Bytes, BytesMut, CompressionState, VarInt, WireRead, WireWrite};

/// One scripted step. Every self-test in this blueprint (its own and
/// `rc-paritybot`'s) builds a `Vec<ScriptStep>` and hands it to `spawn`. Steps that
/// `Expect*` a client packet read and validate only what this blueprint's fake server
/// needs to proceed (e.g. `ExpectLoginStart` reads and discards the name/UUID rather
/// than asserting a specific value) — the fake server is a permissive stand-in for a
/// real server's request side, strict only where a self-test specifically wants a
/// negative case (`SendStatusResponse`'s own free-form `json` field already covers
/// every negative case this blueprint's own acceptance tests need).
#[derive(Debug, Clone)]
pub enum ScriptStep {
    ExpectHandshake, // reads Handshake, discards fields, does not
    // validate `Intention` (both Status- and
    // Login-flow scripts start with this step)
    ExpectStatusRequest,
    SendStatusResponse {
        json: String,
    }, // caller controls the exact JSON, including
    // deliberately malformed/incomplete bodies
    // for negative self-tests
    ExpectPingRequest,
    SendPongEcho,
    ExpectLoginStart,
    SendLoginSuccess {
        username: String,
    },
    ExpectLoginAcknowledged,
    ExpectClientInformation, // configuration phase's first client packet;
    // read and discarded
    SendKnownPacksEmpty,
    ExpectKnownPacksResponse,
    SendFinishConfiguration,
    ExpectAcknowledgeFinishConfiguration,
    SendPlayLogin, // the full ClientboundLogin per the
    // cheat sheet, fixed placeholder field
    // values baked into this step's own
    // implementation — no per-call
    // parameterization needed by any self-test
    // in this blueprint
    RunIdleFor {
        duration: Duration,
        keepalive_interval: Duration,
    }, // sends real
    // `Keep Alive` packets on `keepalive_interval`
    // for `duration`, ignoring the client's own
    // keep-alive replies (azalea answers them
    // automatically; this fake server does not
    // need to read them to prove connection
    // survival — only that it is *still connected*
    // at the end)
    CloseAbruptly, // drops the TCP connection with no Disconnect
                   // packet — the harness-side failure this
                   // blueprint's negative self-tests assert on
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeServerOutcome {
    ScriptCompleted,
    UnexpectedClientClose { at_step: usize },
    IoError { at_step: usize, message: String },
}

/// Binds an ephemeral loopback port, spawns a background OS thread that accepts
/// exactly one connection and executes `script` step by step (blocking `std::net`
/// I/O throughout — no tokio dependency in this crate), and returns the bound address
/// plus a `JoinHandle` the caller joins after its own client-side interaction
/// completes. `CloseAbruptly` and reaching the script's end both terminate the
/// thread; any `Expect*` step reading a mismatched or absent packet where the
/// connection has already closed reports `UnexpectedClientClose` naming the step
/// index, not a panic.
pub fn spawn(script: Vec<ScriptStep>) -> (SocketAddr, JoinHandle<FakeServerOutcome>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral loopback port");
    let addr = listener
        .local_addr()
        .expect("bound listener has a local addr");

    let handle = std::thread::spawn(move || {
        let (stream, _peer) = match listener.accept() {
            Ok(pair) => pair,
            Err(err) => {
                return FakeServerOutcome::IoError {
                    at_step: 0,
                    message: format!("accept failed: {err}"),
                };
            }
        };
        let mut conn = Conn {
            stream,
            accumulator: BytesMut::new(),
        };
        let mut last_ping_payload: i64 = 0;

        for (index, step) in script.iter().enumerate() {
            match run_step(&mut conn, step, &mut last_ping_payload) {
                Ok(StepResult::Continue) => {}
                Ok(StepResult::Stop) => return FakeServerOutcome::ScriptCompleted,
                Err(StepError::UnexpectedClose) => {
                    return FakeServerOutcome::UnexpectedClientClose { at_step: index };
                }
                Err(StepError::Io(message)) => {
                    return FakeServerOutcome::IoError {
                        at_step: index,
                        message,
                    };
                }
            }
        }
        // Natural end of script (no `CloseAbruptly` step): deliberately keep the
        // socket open rather than let it close when `conn` drops here -- a self-test
        // simulating a server that accepts a connection and then genuinely never
        // responds again (rather than a clean/graceful close) depends on this; only
        // an explicit `CloseAbruptly` step actually closes the connection.
        std::mem::forget(conn);
        FakeServerOutcome::ScriptCompleted
    });

    (addr, handle)
}

enum StepResult {
    Continue,
    Stop,
}

enum StepError {
    UnexpectedClose,
    Io(String),
}

struct Conn {
    stream: TcpStream,
    accumulator: BytesMut,
}

impl Conn {
    /// Blocking read of exactly one full frame's decompressed payload (id-VarInt-plus-
    /// fields bytes), buffering as many reads as needed off the connection.
    fn read_one_frame(&mut self) -> Result<Bytes, StepError> {
        loop {
            match rc_protocol::try_decode_frame(&mut self.accumulator, CompressionState::Disabled) {
                Ok(Some(payload)) => return Ok(payload),
                Ok(None) => {}
                Err(err) => return Err(StepError::Io(err.to_string())),
            }
            let mut chunk = [0u8; 4096];
            match self.stream.read(&mut chunk) {
                Ok(0) => return Err(StepError::UnexpectedClose),
                Ok(n) => self.accumulator.extend_from_slice(&chunk[..n]),
                Err(err) => return Err(StepError::Io(err.to_string())),
            }
        }
    }

    fn write_frame(&mut self, payload: &[u8]) -> Result<(), StepError> {
        let mut framed = BytesMut::new();
        rc_protocol::encode_frame(payload, CompressionState::Disabled, &mut framed)
            .map_err(|err| StepError::Io(err.to_string()))?;
        self.stream
            .write_all(&framed)
            .map_err(|err| StepError::Io(err.to_string()))
    }
}

fn run_step(
    conn: &mut Conn,
    step: &ScriptStep,
    last_ping_payload: &mut i64,
) -> Result<StepResult, StepError> {
    match step {
        ScriptStep::ExpectHandshake
        | ScriptStep::ExpectStatusRequest
        | ScriptStep::ExpectLoginStart
        | ScriptStep::ExpectLoginAcknowledged
        | ScriptStep::ExpectClientInformation
        | ScriptStep::ExpectKnownPacksResponse
        | ScriptStep::ExpectAcknowledgeFinishConfiguration => {
            conn.read_one_frame()?;
            Ok(StepResult::Continue)
        }

        ScriptStep::ExpectPingRequest => {
            let mut body = conn.read_one_frame()?;
            // Best-effort: id then i64 payload. A malformed/short body from a
            // misbehaving peer is tolerated by simply keeping the prior payload
            // (0 on the very first ping) rather than failing the whole script --
            // this step's own job is capturing the echo value, not validating the
            // client.
            if let Ok(_id) = VarInt::read_wire(&mut body)
                && let Ok(payload) = i64::read_wire(&mut body)
            {
                *last_ping_payload = payload;
            }
            Ok(StepResult::Continue)
        }

        ScriptStep::SendStatusResponse { json } => {
            let mut body = BytesMut::new();
            VarInt::new(0x00).encode(&mut body);
            json.write_wire(&mut body);
            conn.write_frame(&body)?;
            Ok(StepResult::Continue)
        }

        ScriptStep::SendPongEcho => {
            let mut body = BytesMut::new();
            VarInt::new(0x01).encode(&mut body);
            body.extend_from_slice(&last_ping_payload.to_be_bytes());
            conn.write_frame(&body)?;
            Ok(StepResult::Continue)
        }

        ScriptStep::SendLoginSuccess { username } => {
            let mut body = BytesMut::new();
            VarInt::new(0x02).encode(&mut body);
            body.extend_from_slice(&[0u8; 16]); // uuid, offline-mode placeholder
            username.write_wire(&mut body);
            VarInt::new(0).encode(&mut body); // properties: empty prefixed array
            body.extend_from_slice(&[0u8; 16]); // session_id
            conn.write_frame(&body)?;
            Ok(StepResult::Continue)
        }

        ScriptStep::SendKnownPacksEmpty => {
            let mut body = BytesMut::new();
            VarInt::new(0x0E).encode(&mut body);
            VarInt::new(0).encode(&mut body); // known packs: empty prefixed array
            conn.write_frame(&body)?;
            Ok(StepResult::Continue)
        }

        ScriptStep::SendFinishConfiguration => {
            // Discovered, necessary extension beyond this blueprint's own cheat sheet
            // (matching `SendPlayLogin`'s own precedent, above): a real client's
            // `CommonPlayerSpawnInfo.dimension_type` resolves against whatever
            // `minecraft:dimension_type` registry entries Configuration actually
            // advertised — an empty registry (this step's own first implementation
            // attempt) leaves `DimensionKind { id: 0 }` unresolved, which this
            // blueprint originally assumed was harmless (logged, not fatal) but which
            // in fact starves the client's own "is my spawn chunk loaded" bookkeeping
            // of a resolved world/dimension identity, so `Event::Spawn` never fires.
            // Restates `rc_protocol::configuration::RegistryData`'s own already-
            // reconciled shape (M1-B04, id `0x07`) as the outer envelope, but --
            // unlike M1-B04's own "every entry always has_data=false" scope
            // boundary, which this step's own first implementation attempt copied
            // -- sends this one entry with real inline NBT. Verified empirically
            // (constructing azalea-protocol's own `ClientboundRegistryData` and
            // dumping its `AzBuf::azalea_write` output, Constraints (d)):
            // `DimensionKind`'s own `ResolvableDataRegistry::DeserializesTo` is the
            // strongly-typed `DimensionKindElement` (`height: u32`, `min_y: i32`,
            // ...), never a bare presence flag, so a `has_data=false` entry can
            // never resolve -- and an unresolved dimension_type makes the client's
            // own login-packet handler return before ever registering a `World`
            // object at all, which is *why* `Event::Spawn` never fired (no `World`
            // means `update_in_loaded_chunk` can never find the entity's own loaded
            // chunk, no matter how correct the chunk-loading packets above are).
            // The NBT payload itself is written in the same unnamed/network style
            // already used for heightmaps (root `TAG_Compound`, no name, terminated
            // by `TAG_End`) -- confirmed as this azalea rev's own expected shape by
            // the same empirical dump.
            let mut registry_data = BytesMut::new();
            VarInt::new(0x07).encode(&mut registry_data);
            "minecraft:dimension_type"
                .to_string()
                .write_wire(&mut registry_data); // registry_id
            VarInt::new(1).encode(&mut registry_data); // entries: one
            "minecraft:overworld"
                .to_string()
                .write_wire(&mut registry_data); // entry_id
            true.write_wire(&mut registry_data); // has_data (Option<NbtCompound> == Some)
            encode_dimension_type_nbt(&mut registry_data);
            conn.write_frame(&registry_data)?;

            let mut body = BytesMut::new();
            VarInt::new(0x03).encode(&mut body);
            conn.write_frame(&body)?;
            Ok(StepResult::Continue)
        }

        ScriptStep::SendPlayLogin => {
            // Discovered, necessary extension beyond this blueprint's own cheat
            // sheet (Context, above): a real client's `Event::Spawn` (verified
            // against azalea's own current source, `azalea/src/events.rs`'s
            // `spawn_listener`) fires only once the entity gains an `InLoadedChunk`
            // marker -- which requires the server to actually send the player's own
            // spawn chunk, not merely the `ClientboundLogin` packet. This step
            // therefore sends the full M1-B05-shaped Play-entry sequence (Login,
            // default spawn position, position sync, the "start waiting for chunks"
            // game event, chunk-cache center, one all-air spawn chunk bracketed by
            // a chunk batch) rather than `ClientboundLogin` alone -- still exactly
            // one `ScriptStep`, per this blueprint's own Deliverables shape, just a
            // richer implementation of it.
            for payload in encode_play_entry_sequence() {
                conn.write_frame(&payload)?;
            }
            Ok(StepResult::Continue)
        }

        ScriptStep::RunIdleFor {
            duration,
            keepalive_interval,
        } => {
            let deadline = Instant::now() + *duration;
            let mut next_id: i64 = 0;
            loop {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                let sleep_for = (*keepalive_interval).min(deadline - now);
                std::thread::sleep(sleep_for);
                if Instant::now() >= deadline {
                    break;
                }
                next_id += 1;
                let mut body = BytesMut::new();
                VarInt::new(0x2C).encode(&mut body); // Play Keep Alive (clientbound)
                body.extend_from_slice(&next_id.to_be_bytes());
                conn.write_frame(&body)?;
            }
            Ok(StepResult::Continue)
        }

        ScriptStep::CloseAbruptly => {
            let _ = conn.stream.shutdown(std::net::Shutdown::Both);
            Ok(StepResult::Stop)
        }
    }
}

/// `SendPlayLogin`'s exact byte output — Play "Login" (clientbound id `0x31`, M1-B05's
/// own already-reconciled id), field-for-field matching
/// `rusty_clanker_server::play::packets::LoginPlay`'s wire shape (module doc comment),
/// plus the minimal follow-on Play-entry packets a real client needs to actually reach
/// `Event::Spawn` (this match arm's own doc comment) — one all-air spawn chunk at
/// `(0, 0)` (matching `SPAWN_POSITION`'s `(0, -60, 0)`, M1-B05's own placeholder spawn
/// point), bracketed by a chunk batch, preceded by the default-spawn-position/position-
/// sync/game-event packets a real client's own chunk-loading bookkeeping expects first.
fn encode_play_entry_sequence() -> Vec<BytesMut> {
    let mut login_play = BytesMut::new();
    VarInt::new(0x31).encode(&mut login_play);

    1i32.write_wire(&mut login_play); // entity_id (player_id) -- MinecraftEntityId's own
    // AzBuf impl (verified against azalea-core's own source) is a plain, non-VarInt i32.
    false.write_wire(&mut login_play); // hardcore
    let dimension_names = vec!["minecraft:overworld".to_string()];
    rc_protocol::write_prefixed_vec(&dimension_names, &mut login_play); // levels
    VarInt::new(20).encode(&mut login_play); // max_players
    VarInt::new(2).encode(&mut login_play); // chunk_radius
    VarInt::new(2).encode(&mut login_play); // simulation_distance
    false.write_wire(&mut login_play); // reduced_debug_info
    true.write_wire(&mut login_play); // show_death_screen
    false.write_wire(&mut login_play); // do_limited_crafting
    // `common: CommonPlayerSpawnInfo` -- inline, no sub-framing (verified against
    // azalea-protocol's own source, `packets/common.rs`).
    VarInt::new(0).encode(&mut login_play); // dimension_type (DimensionKind's own AzBuf
    // impl, verified against azalea-registry's own `data_registry!` macro expansion, is
    // a plain `#[var] id: u32` -- a straight VarInt registry index, not a Holder scheme)
    "minecraft:overworld"
        .to_string()
        .write_wire(&mut login_play); // dimension
    0i64.write_wire(&mut login_play); // seed
    0u8.write_wire(&mut login_play); // game_type (Survival) -- GameMode::azalea_write
    // emits a single raw byte; VarInt(0) and a raw 0x00 byte are identical for value 0
    (-1i8).write_wire(&mut login_play); // previous_game_type (OptionalGameType:
    // GameMode::to_optional_id -> raw i8, -1 == "no previous", verified against
    // azalea-core's own source)
    false.write_wire(&mut login_play); // is_debug
    true.write_wire(&mut login_play); // is_flat
    false.write_wire(&mut login_play); // last_death_location (Option<T>, presence-bool
    // encoding, verified against azalea-buf's own source; no trailing fields when false)
    VarInt::new(0).encode(&mut login_play); // portal_cooldown
    VarInt::new(64).encode(&mut login_play); // sea_level
    // Back in the outer packet:
    true.write_wire(&mut login_play); // online_mode -- confirmed present via
    // azalea-protocol's own current source (`ClientboundLogin`'s real field list has
    // this between `common` and `enforces_secure_chat`); a forced correction of this
    // module's own first implementation attempt, which had dropped it
    false.write_wire(&mut login_play); // enforces_secure_chat

    // `ClientboundSetDefaultSpawnPosition { global_pos: GlobalPos { dimension:
    // Identifier, pos: BlockPos }, yaw: f32, pitch: f32 }` -- verified against
    // azalea-protocol/azalea-core's own current source; a forced correction of this
    // module's own first implementation attempt, which had wrongly assumed a bare
    // packed-position-plus-angle-byte shape with no dimension identifier.
    let mut set_default_spawn_position = BytesMut::new();
    VarInt::new(0x61).encode(&mut set_default_spawn_position);
    "minecraft:overworld"
        .to_string()
        .write_wire(&mut set_default_spawn_position); // global_pos.dimension
    SPAWN_POSITION_PACKED.write_wire(&mut set_default_spawn_position); // global_pos.pos
    0.0f32.write_wire(&mut set_default_spawn_position); // yaw
    0.0f32.write_wire(&mut set_default_spawn_position); // pitch

    // `ClientboundPlayerPosition { #[var] id: u32, change: PositionMoveRotation { pos:
    // Vec3, delta: Vec3, look_direction: LookDirection { y_rot: f32, x_rot: f32 } },
    // relative: RelativeMovements }` -- verified against azalea-protocol's own current
    // source (`packets/game/c_player_position.rs`, `common/movements.rs`);
    // `RelativeMovements`'s own `AzBuf` impl reads a raw 4-byte `u32` bitset (`0` ==
    // every axis absolute), not a single flag byte. A forced correction of this
    // module's own first implementation attempt, which had wrongly assumed a flat
    // x/y/z/yaw/pitch/flags/teleport_id shape with no velocity ("delta") fields.
    let mut synchronize_player_position = BytesMut::new();
    VarInt::new(0x48).encode(&mut synchronize_player_position);
    VarInt::new(1).encode(&mut synchronize_player_position); // id (teleport id)
    0.0f64.write_wire(&mut synchronize_player_position); // change.pos.x
    (-60.0f64).write_wire(&mut synchronize_player_position); // change.pos.y
    0.0f64.write_wire(&mut synchronize_player_position); // change.pos.z
    0.0f64.write_wire(&mut synchronize_player_position); // change.delta.x
    0.0f64.write_wire(&mut synchronize_player_position); // change.delta.y
    0.0f64.write_wire(&mut synchronize_player_position); // change.delta.z
    0.0f32.write_wire(&mut synchronize_player_position); // change.look_direction.y_rot (yaw)
    0.0f32.write_wire(&mut synchronize_player_position); // change.look_direction.x_rot (pitch)
    0i32.write_wire(&mut synchronize_player_position); // relative -- all-absolute (every
    // bit 0; `rc_protocol::WireWrite` has no `u32` impl, `i32`'s identical 4-byte
    // big-endian bit pattern for `0` is used instead)

    // GameEvent 13 == "Start waiting for level chunks" (M1-B05's own already-
    // reconciled id/value), the signal real client implementations use to know the
    // server intends to stream chunks next.
    let mut game_event = BytesMut::new();
    VarInt::new(0x26).encode(&mut game_event);
    13u8.write_wire(&mut game_event); // event
    0.0f32.write_wire(&mut game_event); // value

    let mut set_chunk_cache_center = BytesMut::new();
    // 0x5E, not the cheat sheet's 0x58 (which this protocol's own clientbound Play
    // packet table assigns to `set_border_center` instead) -- every clientbound Play
    // packet id in this function was independently recomputed as this exact azalea
    // rev's own declaration-order index (`declare_state_packets!`'s own id-assignment
    // rule, verified against its macro source) rather than trusted from the cheat
    // sheet; only this one and `set_chunk_cache_center` disagreed with the sheet.
    VarInt::new(0x5E).encode(&mut set_chunk_cache_center);
    VarInt::new(0).encode(&mut set_chunk_cache_center); // chunk_x
    VarInt::new(0).encode(&mut set_chunk_cache_center); // chunk_z

    let mut chunk_batch_start = BytesMut::new();
    VarInt::new(0x0C).encode(&mut chunk_batch_start);

    let mut level_chunk = BytesMut::new();
    VarInt::new(0x2D).encode(&mut level_chunk);
    0i32.write_wire(&mut level_chunk); // chunk_x
    0i32.write_wire(&mut level_chunk); // chunk_z
    VarInt::new(0).encode(&mut level_chunk); // heightmaps: empty `Vec<(HeightmapKind,
    // Box<[u64]>)>` (verified against azalea-protocol's own source,
    // `ClientboundLevelChunkPacketData` -- not a raw NBT compound, a forced correction
    // of this module's own first implementation attempt) -- a real client recomputes
    // heightmaps from the chunk's own block data when none are supplied
    rc_protocol::write_prefixed_vec(&build_air_chunk_data(), &mut level_chunk); // data
    VarInt::new(0).encode(&mut level_chunk); // block_entities: empty
    VarInt::new(0).encode(&mut level_chunk); // sky_light_mask: empty
    VarInt::new(0).encode(&mut level_chunk); // block_light_mask: empty
    VarInt::new(0).encode(&mut level_chunk); // empty_sky_light_mask: empty
    VarInt::new(0).encode(&mut level_chunk); // empty_block_light_mask: empty
    VarInt::new(0).encode(&mut level_chunk); // sky_light_arrays: empty
    VarInt::new(0).encode(&mut level_chunk); // block_light_arrays: empty

    let mut chunk_batch_finished = BytesMut::new();
    VarInt::new(0x0B).encode(&mut chunk_batch_finished);
    VarInt::new(1).encode(&mut chunk_batch_finished); // batch_size

    vec![
        login_play,
        set_default_spawn_position,
        synchronize_player_position,
        game_event,
        set_chunk_cache_center,
        chunk_batch_start,
        level_chunk,
        chunk_batch_finished,
    ]
}

/// `(0, -60, 0)` packed into a "Position" wire value (26-bit X, 26-bit Z, 12-bit Y,
/// two's complement — matches `rusty_clanker_server::play::packets::pack_position`'s
/// own documented layout), written as one plain big-endian `i64`.
const SPAWN_POSITION_PACKED: i64 = (-60i64) & 0xFFF;

/// The minimal network-NBT (unnamed root, no name field, `TAG_End`-terminated)
/// `DimensionKindElement` compound a real client needs to resolve `dimension_type`
/// (`height`/`min_y` as `TAG_Int`, matching M1-B05's own already-reconciled world
/// height, `WORLD_MIN_Y = -64` / `SECTION_COUNT * 16 = 384`) — every other field of
/// the real struct is `Option`-typed or absorbed by its own `#[simdnbt(flatten)]`
/// catch-all, so omitting them is not a further approximation, it is what a minimal,
/// legal instance of this exact type already looks like.
fn encode_dimension_type_nbt(buf: &mut BytesMut) {
    0x0Au8.write_wire(buf); // root TAG_Compound, unnamed (network NBT).

    0x03u8.write_wire(buf); // TAG_Int
    6u16.write_wire(buf);
    buf.extend_from_slice(b"height");
    384i32.write_wire(buf);

    0x03u8.write_wire(buf); // TAG_Int
    5u16.write_wire(buf);
    buf.extend_from_slice(b"min_y");
    (-64i32).write_wire(buf);

    0x00u8.write_wire(buf); // TAG_End
}

/// One all-air, 24-section chunk `data` blob (`WORLD-D2`'s "SingleValue" 0-bit
/// paletted-container path for both the block and biome containers, `block_count = 0`
/// per section) — sufficient for a real client to accept the chunk and mark it loaded;
/// this fake server does not need real terrain content for any self-test in this
/// blueprint. 24 sections matches M1-B05's own already-reconciled world height
/// (`WORLD_MIN_Y = -64`, `SECTION_COUNT = 24`).
fn build_air_chunk_data() -> Vec<u8> {
    const AIR_BLOCK_STATE_ID: i32 = 0; // rc-registries' own generated `block_states::AIR`
    const AIR_BIOME_ID: i32 = 0;
    const SECTION_COUNT: usize = 24;

    let mut section = BytesMut::new();
    0i16.write_wire(&mut section); // block_count
    // Block paletted container, SingleValue path.
    0u8.write_wire(&mut section); // bits_per_entry
    VarInt::new(AIR_BLOCK_STATE_ID).encode(&mut section);
    VarInt::new(0).encode(&mut section); // data_array_length
    // Biome paletted container, SingleValue path.
    0u8.write_wire(&mut section);
    VarInt::new(AIR_BIOME_ID).encode(&mut section);
    VarInt::new(0).encode(&mut section);
    let section_bytes = section.to_vec();

    let mut data = Vec::with_capacity(section_bytes.len() * SECTION_COUNT);
    for _ in 0..SECTION_COUNT {
        data.extend_from_slice(&section_bytes);
    }
    data
}
