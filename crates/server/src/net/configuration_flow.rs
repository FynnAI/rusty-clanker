//! Drives one connection's `ConnectionState::Configuration` (NET-D3/NET-D4/NET-D9/NET-D10),
//! from brand/feature-flags/known-packs negotiation through registry-data sync to the
//! terminal `FinishConfiguration`/`AcknowledgeFinishConfiguration` exchange — M1-B04
//! blueprint Context, "Configuration sequence, exact order" / "Keep-alive during
//! Configuration."

use std::time::Duration;

use bytes::BytesMut;
use rc_protocol::{
    AcknowledgeFinishConfiguration, ClientInformation, ConfigurationKeepAliveClientbound,
    ConfigurationKeepAliveServerbound, ConfigurationPluginMessage, ConnectionState,
    FinishConfiguration, Identifier, KnownPack, KnownPacksClientbound, KnownPacksServerbound,
    RawPacket, RcPacket, RegistryData, RegistryDataEntryOut, UpdateEnabledFeatures, VarInt,
    WireWrite, decode_one, encode_payload,
};
use tokio::sync::mpsc;
use tokio::time::Interval;

use crate::net::{ConnectionHandle, SendError};

pub const KEEP_ALIVE_INTERVAL: Duration = Duration::from_millis(15_000);

#[derive(Debug, Clone)]
pub struct ServerConfigurationConfig {
    pub server_brand: String,
    pub known_pack: KnownPack,
    pub feature_flags: Vec<Identifier>,
}
impl Default for ServerConfigurationConfig {
    fn default() -> Self {
        Self {
            server_brand: "rusty-clanker".to_string(),
            known_pack: KnownPack {
                namespace: "minecraft".to_string(),
                id: "core".to_string(),
                version: "26.2".to_string(),
            },
            feature_flags: vec![Identifier::new("minecraft:vanilla")],
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error("connection closed by peer during configuration")]
    Closed,
    #[error("keep-alive timed out")]
    KeepAliveTimeout,
    #[error("unsolicited or mismatched keep-alive reply")]
    KeepAliveMismatch,
    #[error(transparent)]
    Decode(#[from] rc_protocol::PacketDecodeError),
    #[error(transparent)]
    Send(#[from] SendError),
}

/// Which gating serverbound packet `drive_until_gate` is currently waiting on — the other
/// gating id, if it arrives instead, is out-of-order and silently ignored (Context).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigGate {
    KnownPacks,
    FinishAck,
}

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// `BrandPayload`'s own wire shape is itself one `String` — `data` here is exactly that
/// string's own VarInt-length-prefixed UTF-8 bytes, not raw text.
fn encode_brand_payload(brand: &str) -> Vec<u8> {
    let mut buf = BytesMut::new();
    brand.to_string().write_wire(&mut buf);
    buf.to_vec()
}

/// The one registry a real client cannot resolve on its own (M1 integration fix — Context,
/// completion-report diagnosis: a real client logs `Couldn't resolve dimension_type
/// DimensionKind { id: 0 }`, then desyncs and fails to parse every subsequent packet, and
/// `Event::Spawn` never fires, because a real client's `DimensionKind` resolves to the
/// strongly-typed `DimensionKindElement`, which a `has_data=false` entry can never produce).
/// M1-B04's own original "every entry always has_data=false" default (Context, "why every
/// entry is sent with has_data=false") still holds for every other registry this server
/// ever advertises -- this is the one bounded, documented exception.
const DIMENSION_TYPE_REGISTRY: &str = "minecraft:dimension_type";

/// The minimal network-NBT (unnamed root, no name field, `TAG_End`-terminated)
/// `DimensionKindElement` compound a real client needs to resolve `dimension_type`
/// (`height`/`min_y` as `TAG_Int`) -- every other field of the real struct is `Option`-typed
/// or absorbed by its own catch-all, so omitting them is not a further approximation, it is
/// what a minimal, legal instance of this exact type already looks like. Byte-for-byte the
/// same shape as `crates/testing/test-harness/src/fake_server.rs`'s own already-proven
/// `encode_dimension_type_nbt` (M1-B06's own empirical dump of azalea-protocol's real
/// `ClientboundRegistryData`/`DimensionKindElement` decode expectations, Constraints (d)) --
/// this is that same reference shape, reused rather than reinvented, now wired into the real
/// production `run_configuration` path instead of only the test double. `height`/`min_y`
/// values come from `crate::play::chunk`'s own already-committed world-height constants
/// (`SECTION_COUNT * 16`, `WORLD_MIN_Y`), not restated as independent magic numbers, so this
/// payload can never silently drift out of sync with the chunk content the same connection
/// sends moments later.
fn encode_dimension_type_nbt() -> Vec<u8> {
    let height = (crate::play::chunk::SECTION_COUNT as i32) * 16;
    let min_y = crate::play::chunk::WORLD_MIN_Y;

    let mut buf = BytesMut::new();
    0x0Au8.write_wire(&mut buf); // root TAG_Compound, unnamed (network NBT).

    0x03u8.write_wire(&mut buf); // TAG_Int
    6u16.write_wire(&mut buf);
    buf.extend_from_slice(b"height");
    height.write_wire(&mut buf);

    0x03u8.write_wire(&mut buf); // TAG_Int
    5u16.write_wire(&mut buf);
    buf.extend_from_slice(b"min_y");
    min_y.write_wire(&mut buf);

    0x00u8.write_wire(&mut buf); // TAG_End
    buf.to_vec()
}

/// Best-effort: sends a Configuration-phase Disconnect (ignoring any send failure — the
/// connection may already be unusable) then unconditionally closes the connection.
/// Configuration-phase disconnects reuse Login's own `Disconnect` byte shape (`reason:
/// NbtTextComponent`, network-NBT — M1 integration fix, `wire::NbtTextComponent`'s own doc
/// comment) hand-encoded under Configuration's own `Disconnect` id (`0x02`) — Context's
/// packet-catalog table; no separate derived Rust type exists for this.
async fn disconnect(handle: &ConnectionHandle, reason: &str) {
    let mut payload = BytesMut::new();
    VarInt::new(0x02).encode(&mut payload);
    rc_protocol::NbtTextComponent(reason.to_string()).write_wire(&mut payload);
    let _ = handle.try_send_payload(payload.freeze());
    // Same enqueue/close race `net::login_flow::disconnect` and `net::status::serve_status`
    // (M1-B02) already document and work around identically — yield once so the writer task
    // drains and writes the already-enqueued Disconnect before the close signal exists.
    tokio::task::yield_now().await;
    handle.close();
}

/// Reads and dispatches inbound packets (plus the periodic keep-alive concern) until either
/// `gate`'s own awaited packet arrives (`Ok(())`) or a fatal condition occurs (`Err`).
/// Restates Context's "keep dispatching every received packet by id ... until the
/// specifically-awaited id arrives."
async fn drive_until_gate(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
    interval: &mut Interval,
    keep_alive_pending: &mut Option<i64>,
    gate: ConfigGate,
) -> Result<(), ConfigurationError> {
    loop {
        tokio::select! {
            maybe_raw = inbound.recv() => {
                let Some(raw) = maybe_raw else {
                    return Err(ConfigurationError::Closed);
                };

                if raw.id == ConfigurationKeepAliveServerbound::ID {
                    if let Some(pending) = *keep_alive_pending {
                        let reply = match decode_one::<ConfigurationKeepAliveServerbound>(raw.body)
                        {
                            Ok(reply) => reply,
                            Err(err) => {
                                disconnect(handle, "malformed packet").await;
                                return Err(ConfigurationError::Decode(err));
                            }
                        };
                        if reply.keep_alive_id == pending {
                            *keep_alive_pending = None;
                        } else {
                            disconnect(handle, "Timed out").await;
                            return Err(ConfigurationError::KeepAliveMismatch);
                        }
                    }
                    // Else: no challenge pending — an unsolicited reply, silently dropped
                    // (falls under the general "not one of the gating ids" rule, Context).
                } else if raw.id == ClientInformation::ID {
                    // Recorded for later gameplay-system use, out of this blueprint's own
                    // scope — decoded and dropped; a malformed body here is non-gating.
                    let _ = decode_one::<ClientInformation>(raw.body);
                } else if raw.id == KnownPacksServerbound::ID && gate == ConfigGate::KnownPacks {
                    // M1 integration fix: this used to require the client's echoed
                    // `known_packs` to exactly equal the one offered pack, disconnecting on
                    // any other response (M1-B04's own "defensive design" against an
                    // unverifiable assumption, its Context section's own words). Driving a
                    // real client (azalea) against this exact gate showed the assumption was
                    // wrong: a real, fresh client always echoes an *empty* list (it has
                    // nothing cached locally yet), which this check treated as a fatal
                    // mismatch -- disconnecting every real client during Configuration. The
                    // response's actual real-vanilla purpose is purely informational (which
                    // already-cached entries the server may then omit full data for); this
                    // server decides which registry entries carry real inline NBT purely by
                    // `registry_id` (`DIMENSION_TYPE_REGISTRY`, below -- a later M1 integration
                    // fix on top of this one), never by inspecting the client's echoed
                    // `known_packs` response, so the response is decoded (a malformed body is
                    // still a fatal protocol violation) but never gates on its *content* -- any
                    // echoed list, including an empty one, is accepted and the sequence
                    // proceeds.
                    if let Err(err) = decode_one::<KnownPacksServerbound>(raw.body) {
                        disconnect(handle, "malformed packet").await;
                        return Err(ConfigurationError::Decode(err));
                    }
                    return Ok(());
                } else if raw.id == AcknowledgeFinishConfiguration::ID && gate == ConfigGate::FinishAck {
                    if let Err(err) = decode_one::<AcknowledgeFinishConfiguration>(raw.body) {
                        disconnect(handle, "malformed packet").await;
                        return Err(ConfigurationError::Decode(err));
                    }
                    return Ok(());
                }
                // Else: either an out-of-order gating id, or one of the explicitly
                // unimplemented packets (Plugin Message, Cookie Response, Pong, Resource
                // Pack Response, Custom Click Action, Accept Code of Conduct) — silently
                // dropped, never a disconnect (Context / Constraints (e)).
            }
            _ = interval.tick() => {
                if keep_alive_pending.is_some() {
                    disconnect(handle, "Timed out").await;
                    return Err(ConfigurationError::KeepAliveTimeout);
                }
                let id = now_millis();
                handle.try_send_payload(encode_payload(&ConfigurationKeepAliveClientbound {
                    keep_alive_id: id,
                }))?;
                *keep_alive_pending = Some(id);
            }
        }
    }
}

/// Drives one connection's Configuration state, per Context's numbered sequence.
/// `worldgen_registries` decouples this function from the only-manually-generated real
/// content — a later blueprint's production call site supplies the real table once it
/// wires `crates/registries/generated/v776/registry_entries.rs` into `rc-registries`
/// itself; this blueprint's own tests pass a small synthetic fixture instead.
pub async fn run_configuration(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
    config: &ServerConfigurationConfig,
    worldgen_registries: &'static [(&'static str, &'static [&'static str])],
) -> Result<(), ConfigurationError> {
    // Steps 1-2: brand + feature flags, sent immediately.
    handle.try_send_payload(encode_payload(&ConfigurationPluginMessage {
        channel: Identifier::new("minecraft:brand"),
        data: encode_brand_payload(&config.server_brand),
    }))?;
    handle.try_send_payload(encode_payload(&UpdateEnabledFeatures {
        features: config.feature_flags.clone(),
    }))?;

    let mut interval = tokio::time::interval(KEEP_ALIVE_INTERVAL);
    // The first tick of a freshly-created `tokio::time::interval` fires immediately —
    // consumed here so the first *real* keep-alive challenge fires KEEP_ALIVE_INTERVAL
    // after Configuration begins, not instantly.
    interval.tick().await;
    let mut keep_alive_pending: Option<i64> = None;

    // Step 3: known-pack negotiation.
    handle.try_send_payload(encode_payload(&KnownPacksClientbound {
        known_packs: vec![config.known_pack.clone()],
    }))?;
    drive_until_gate(
        inbound,
        handle,
        &mut interval,
        &mut keep_alive_pending,
        ConfigGate::KnownPacks,
    )
    .await?;

    // Step 4: registry-data sync. Every entry is `has_data=false` (Context) *except*
    // `minecraft:dimension_type`'s own entries, which carry real inline NBT (M1 integration
    // fix, `encode_dimension_type_nbt`'s own doc comment).
    for (registry_id, entries) in worldgen_registries {
        let entries_out = entries
            .iter()
            .map(|entry_id| RegistryDataEntryOut {
                entry_id: Identifier::new(*entry_id),
                data: if *registry_id == DIMENSION_TYPE_REGISTRY {
                    Some(encode_dimension_type_nbt())
                } else {
                    None
                },
            })
            .collect();
        handle.try_send_payload(encode_payload(&RegistryData {
            registry_id: Identifier::new(*registry_id),
            entries: entries_out,
        }))?;
    }

    // Step 5 (Update Tags) is deliberately not sent (Constraints (e)).

    // Step 6: Finish Configuration, terminal.
    handle.try_send_payload(encode_payload(&FinishConfiguration {}))?;
    drive_until_gate(
        inbound,
        handle,
        &mut interval,
        &mut keep_alive_pending,
        ConfigGate::FinishAck,
    )
    .await?;

    // Outbound flips to Play only once the ack has actually arrived — inbound stays
    // Configuration (Context's asymmetric state-slot table; a later blueprint's
    // player-spawn setup advances it).
    handle.set_outbound_state(ConnectionState::Play);
    Ok(())
}
