//! M3.5-B03: the protocol-differential capture format, the data-driven normalization
//! table, the cross-packet chunk-batch reordering pass, and the byte-level per-
//! packet-type multiset diff — the azalea-free, pure half of TEST-D54's harness
//! (`xtask protocol-diff`'s own module doc comment has the full architecture; this
//! module is the "capture format plus diff" counterpart `placement_trace.rs`
//! establishes for the M3 placement harness, restated here for a structurally
//! different unit: a raw, pre-typed-decode `(index, packet_id, body)` triple rather
//! than a resolved block-state observation).
//!
//! Every masked field this module's `NORMALIZATION_RULES` table names is resolved
//! against the ASSET-D18(f) reference by the TEST-D57 research pass
//! (`blueprints/M3.5/M3.5-B03-CLAIMS.md`) — masking is **structural**, never a fixed
//! byte offset (only `KeepAlive`'s single field has one at all): every listed packet
//! type is decoded field-by-field with a small, purpose-built decoder (`rc-gametest`
//! never depends on `rc-protocol` or any azalea packet struct — none of the packets
//! this table names have an existing typed decoder in this workspace at all, since
//! `rc-protocol` defines no play-state packet catalog and `rc-gametest` must never
//! link `azalea`, TEST-D8's own boundary), re-encoded with the masked field(s)
//! replaced by a canonical value, and the diff compares the re-encoded bytes. A
//! packet whose own real shape this module's decoder cannot confidently model to the
//! end (an unexpected `Optional`/nested-payload branch it does not have byte-level
//! knowledge of) degrades to masking that ONE packet instance's whole body — the
//! same "safe direction to fail" `packet_name: None` resolution already establishes
//! for an unresolved name (§3.7) — rather than guessing a byte layout it cannot
//! verify and risking a silent misalignment. Every such simplification actually
//! exercised by this module (`SetTime`'s `clockUpdates` map value shape;
//! `PlayerInfoUpdate`'s `INITIALIZE_CHAT`/`UPDATE_DISPLAY_NAME` payload shape when
//! present) is recorded in `docs/findings-for-planning.md`.

use std::path::Path;

/// Bumped from `1` in M3.5-B03's own follow-up governance changeset (deliverable 1,
/// `docs/findings-for-planning.md`): `StepCapture` gained `observe_from`. Postcard is
/// not self-describing (no field tags), so a struct-shape change is not itself
/// safely detectable by attempting the full decode — `read_capture` therefore probes
/// this leading field first and rejects anything that does not match with a clear,
/// dedicated error (`CaptureReadError::UnsupportedVersion`) before ever attempting
/// the full decode.
pub const PROTOCOL_CAPTURE_FORMAT_VERSION: u32 = 2;

/// One raw clientbound packet, captured pre-any-typed-decode (§3.7).
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CapturedPacket {
    pub index: u32,
    pub packet_id: i32,
    pub body: Vec<u8>,
    /// Best-effort, display-only — see this module's own doc comment. Never read by
    /// `diff_step` (grouping and pass/fail are always keyed on `packet_id`, §3.9),
    /// only by `normalize_body` to select which `NORMALIZATION_RULES` row applies.
    pub packet_name: Option<String>,
}

/// One scripted-session step's or one redstone-corpus contraption's own captured
/// clientbound stream. `step_id` matches `SESSION_STEPS`/a contraption's own `spec.id`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StepCapture {
    pub step_id: String,
    /// M3.5-B03 follow-up (deliverable 1): the packet index (within this step's own
    /// `packets`, matching each `CapturedPacket::index`) at which this step's own
    /// setup — pre-clear, real placement, setblock prelude, tick barrier — ends and
    /// the observed window begins. Meaningful only for a contraption step (`step_id`
    /// starting with `"redstone/"`, `is_contraption_step`'s own doc comment); every
    /// scripted-session step carries `0` here (§3.5's own scripted actions ARE the
    /// step, from its very first packet — there is no separate setup phase to skip),
    /// which is also `apply_observation_window`'s own no-op value for a step it
    /// leaves untouched. Set by `protocol_diff_runner`/`redstone_wire_capture` at the
    /// moment each contraption's own scripted actions begin (`apply_actions`'s own
    /// call site), identically on both sides — never read from wall-clock time or
    /// any other side-specific signal, so the same conceptual moment is captured
    /// regardless of how many setup packets each side's own server happened to emit.
    pub observe_from: u32,
    pub packets: Vec<CapturedPacket>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProtocolCaptureFile {
    pub format_version: u32,
    /// `"oracle:<jar sha1>"` or `"ours"` — mirrors `PlacementCaptureFile::source_label`.
    pub source_label: String,
    pub steps: Vec<StepCapture>,
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureReadError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("postcard decode error reading {path}: {source}")]
    Decode {
        path: String,
        source: postcard::Error,
    },
    /// M3.5-B03 follow-up (deliverable 1/6): `path`'s own leading `format_version`
    /// field does not match `PROTOCOL_CAPTURE_FORMAT_VERSION` — this capture predates
    /// (or postdates) a `StepCapture` shape this build understands, and must be
    /// re-captured rather than decoded, since postcard's own lack of field tags means
    /// attempting the full decode against a mismatched shape risks silent
    /// misalignment rather than a clean parse failure (this module's own doc comment
    /// has the full rationale).
    #[error(
        "{path}: capture format version {found} is not supported by this build \
         (expected {expected}) — this capture predates the observation-window field \
         and must be re-captured, never decoded as-is"
    )]
    UnsupportedVersion {
        path: String,
        found: u32,
        expected: u32,
    },
}

/// The leading-field-only probe `read_capture` decodes first (module doc comment's
/// own rationale) — postcard reads a struct's fields strictly in declaration order
/// with no look-ahead, so deserializing this single-field prefix of the real
/// `ProtocolCaptureFile` shape correctly consumes only `format_version`'s own bytes
/// regardless of what (if anything) follows, on any capture format version old or
/// new.
#[derive(serde::Deserialize)]
struct FormatVersionProbe {
    format_version: u32,
}

pub fn write_capture(path: &Path, capture: &ProtocolCaptureFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = postcard::to_allocvec(capture)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(path, bytes)
}

pub fn read_capture(path: &Path) -> Result<ProtocolCaptureFile, CaptureReadError> {
    let bytes = std::fs::read(path).map_err(|source| CaptureReadError::Io {
        path: path.display().to_string(),
        source,
    })?;

    let probe: FormatVersionProbe = postcard::take_from_bytes(&bytes)
        .map(|(probe, _rest)| probe)
        .map_err(|source| CaptureReadError::Decode {
            path: path.display().to_string(),
            source,
        })?;
    if probe.format_version != PROTOCOL_CAPTURE_FORMAT_VERSION {
        return Err(CaptureReadError::UnsupportedVersion {
            path: path.display().to_string(),
            found: probe.format_version,
            expected: PROTOCOL_CAPTURE_FORMAT_VERSION,
        });
    }

    postcard::from_bytes(&bytes).map_err(|source| CaptureReadError::Decode {
        path: path.display().to_string(),
        source,
    })
}

/// One packet type's declarative normalization rule (§3.8).
#[derive(Debug, Clone)]
pub struct FieldMask {
    /// Report-only label, e.g. `"teleport_id"`.
    pub field: &'static str,
    /// Byte range within `body` this field occupies — resolved against the ASSET-D18(f)
    /// reference per TEST-D57 (§3.8), never asserted by this blueprint. Every masked
    /// field but `SetTime`'s own `game_time` sits behind a variable-width `VarInt`,
    /// nested structure, or conditional presence bit (TEST-D57 pass, CLAIMS row 1) —
    /// its own per-instance offset is resolved dynamically by `normalize_body`'s
    /// structural decoder, never read from this field. Carried as `0..0` (a
    /// documentation-only sentinel) for every such row; populated with the real,
    /// genuinely fixed range only where TEST-D57 confirmed one exists (`SetTime`'s
    /// `game_time`, `0..8`).
    pub range: std::ops::Range<usize>,
}

pub struct PacketNormalizationRule {
    pub packet_name: &'static str,
    pub masks: &'static [FieldMask],
    /// `true` replaces the whole body with an empty sentinel (e.g. `KeepAlive`) —
    /// mutually exclusive with a non-empty `masks`.
    pub mask_whole_body: bool,
}

/// The static table backing §3.8. TEST-D57-verified per
/// `blueprints/M3.5/M3.5-B03-CLAIMS.md`; a packet name not listed here is compared
/// byte-for-byte, unmasked (§3.8's own explicit default, `normalize_body`'s own
/// fallback arm) — an unlisted differing packet type is a failure, never silently
/// tolerated. Adding a new masked field is a governance changeset (TEST-D46), never
/// something an implementation changeset may do on its own initiative.
pub const NORMALIZATION_RULES: &[PacketNormalizationRule] = &[
    // `KeepAlive` (clientbound): sole field `id: long`, offset 0, width 8, big-endian,
    // whole body — the one row in this table with a genuinely fixed, whole-packet
    // offset+width (CLAIMS row 1).
    PacketNormalizationRule {
        packet_name: "keep_alive",
        masks: &[],
        mask_whole_body: true,
    },
    // `Login`/join-game: the masked value is `CommonPlayerSpawnInfo::seed` (a real
    // vanilla field literally named `seed`, functionally a seed hash —
    // `BiomeManager.obfuscateSeed`, CLAIMS row 1) — reached only after `playerId: i32`
    // (4B) + `hardcore: bool` (1B) + `levels: Set<ResourceKey>` (VarInt-counted,
    // variable-length strings) + `dimensionType: Holder<DimensionType>` (a
    // variable-width registry-reference VarInt) + `dimension: Identifier` (a
    // variable-length string) — `seed`'s own byte offset is data-dependent, never a
    // fixed constant; its width (8B, plain `i64`) is fixed once reached.
    PacketNormalizationRule {
        packet_name: "login",
        masks: &[FieldMask {
            field: "seed",
            range: 0..0,
        }],
        mask_whole_body: false,
    },
    // `PlayerPosition` (clientbound teleport): `id` (teleport-id) is the packet's
    // first field, offset 0, VarInt (variable width, CLAIMS row 1).
    PacketNormalizationRule {
        packet_name: "player_position",
        masks: &[FieldMask {
            field: "teleport_id",
            range: 0..0,
        }],
        mask_whole_body: false,
    },
    // `AddEntity`: `id` first field, offset 0, VarInt; `uuid` (16B fixed width)
    // follows immediately after `id`, at an offset that varies with `id`'s own
    // VarInt-encoded length.
    PacketNormalizationRule {
        packet_name: "add_entity",
        masks: &[
            FieldMask {
                field: "id",
                range: 0..0,
            },
            FieldMask {
                field: "uuid",
                range: 0..0,
            },
        ],
        mask_whole_body: false,
    },
    // `SetEntityData`: `id` is the sole leading field, offset 0, VarInt — the
    // remainder is a self-terminating (0xFF-marker) metadata list, correctly left
    // unmasked.
    PacketNormalizationRule {
        packet_name: "set_entity_data",
        masks: &[FieldMask {
            field: "id",
            range: 0..0,
        }],
        mask_whole_body: false,
    },
    // `MoveEntity*` family: three distinct wire packet ids, `entityId` the first
    // field in all three, offset 0, VarInt.
    PacketNormalizationRule {
        packet_name: "move_entity_pos",
        masks: &[FieldMask {
            field: "entity_id",
            range: 0..0,
        }],
        mask_whole_body: false,
    },
    PacketNormalizationRule {
        packet_name: "move_entity_pos_rot",
        masks: &[FieldMask {
            field: "entity_id",
            range: 0..0,
        }],
        mask_whole_body: false,
    },
    PacketNormalizationRule {
        packet_name: "move_entity_rot",
        masks: &[FieldMask {
            field: "entity_id",
            range: 0..0,
        }],
        mask_whole_body: false,
    },
    // `RemoveEntities`: the entire body *is* the entity-id list (VarInt count +
    // VarInt ids) — masking "the entity-id field(s)" here means masking the whole
    // body, since no other field exists (CLAIMS row 1's own correction).
    PacketNormalizationRule {
        packet_name: "remove_entities",
        masks: &[],
        mask_whole_body: true,
    },
    // `PlayerInfoUpdate`: the latency sub-field lives inside a per-player `Entry`,
    // present only if the packet's own `actions: EnumSet<Action>` bitmask (a single
    // byte, 8 actions) includes `UPDATE_LATENCY` — masking this field requires a
    // structural, `actions`-driven decode, never a byte range.
    PacketNormalizationRule {
        packet_name: "player_info_update",
        masks: &[FieldMask {
            field: "latency",
            range: 0..0,
        }],
        mask_whole_body: false,
    },
    // `LoginFinished`: `record ClientboundLoginFinishedPacket(GameProfile gameProfile,
    // UUID sessionId)` (ASSET-D18(f) reference) — `gameProfile` (id: UUID 16B + name:
    // VarInt-length string + properties: VarInt-counted list of {name, value, optional
    // signature} strings) is deterministic (offline-mode UUID is a pure hash of the
    // fixed bot account name, TEST-D57) and stays unmasked; `sessionId` (16B, the same
    // fixed-width UUID codec) is a fresh random value picked per login and immediately
    // follows the profile at a data-dependent offset (the properties list is
    // variable-length, even though this harness's own bot always presents zero of
    // them) — masked wholesale (M3.5-B03 governance fix: replaces the register entry
    // this session UUID divergence used to need on every step that logs in).
    PacketNormalizationRule {
        packet_name: "login_finished",
        masks: &[FieldMask {
            field: "session_id",
            range: 0..0,
        }],
        mask_whole_body: false,
    },
    // `CustomPayload` (clientbound plugin message): `channel: Identifier` (VarInt-
    // length string) followed by the channel-specific payload. This harness only ever
    // observes the `minecraft:brand` channel, whose own payload
    // (`record BrandPayload(String brand)`, ASSET-D18(f) reference) is a single
    // VarInt-length string that is the *entire* remainder of the packet — masked
    // wholesale, since our own server name will never equal vanilla's own literal
    // `"vanilla"` value and nothing else in the packet differs (M3.5-B03 governance
    // fix: replaces the register entry this divergence used to need). A channel other
    // than `minecraft:brand` is left completely unmasked — this harness never
    // sends/observes any other channel, and a genuine difference there would be a
    // real divergence, never masked blind.
    PacketNormalizationRule {
        packet_name: "custom_payload",
        masks: &[FieldMask {
            field: "brand",
            range: 0..0,
        }],
        mask_whole_body: false,
    },
    // `SetTime`: 26.2's real shape is `(gameTime: i64, clockUpdates: Map<Holder
    // <WorldClock>, ClockNetworkState>)` — no separate day-time field exists any
    // more. `gameTime` is fixed offset 0 / width 8 (CLAIMS row 1's own genuinely
    // static case, alongside `KeepAlive`); every value in `clockUpdates` is masked
    // too (the map's own *keys* — which clocks exist — stay unmasked, per §3.8).
    PacketNormalizationRule {
        packet_name: "set_time",
        masks: &[
            FieldMask {
                field: "game_time",
                range: 0..8,
            },
            FieldMask {
                field: "clock_updates_values",
                range: 8..8,
            },
        ],
        mask_whole_body: false,
    },
];

/// A tiny, panic-free byte cursor this module's own structural decoders share — every
/// read is checked (`Option`-returning); nothing here ever indexes out of bounds or
/// asserts, so a genuinely malformed or unmodeled packet body degrades to `None`
/// (this module's own "safe direction to fail") rather than a panic.
struct Cur<'a> {
    body: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    fn new(body: &'a [u8]) -> Self {
        Self { body, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.body.len().saturating_sub(self.pos) < n {
            return None;
        }
        let out = &self.body[self.pos..self.pos + n];
        self.pos += n;
        Some(out)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|s| s[0])
    }

    fn bool(&mut self) -> Option<bool> {
        self.u8().map(|v| v != 0)
    }

    fn i32_be(&mut self) -> Option<i32> {
        self.take(4)
            .map(|s| i32::from_be_bytes(s.try_into().unwrap()))
    }

    fn i64_be(&mut self) -> Option<i64> {
        self.take(8)
            .map(|s| i64::from_be_bytes(s.try_into().unwrap()))
    }

    /// Minecraft's own VarInt: 7 data bits per byte, MSB continuation flag, capped at
    /// 5 bytes for an `i32` (identical algorithm to `rc_protocol::varint::VarInt::
    /// decode` — restated locally rather than taken as a dependency, since this
    /// crate's own `Cargo.toml` doc comment already forbids every dependency this
    /// module does not strictly need, and this one primitive is small and closed
    /// enough not to be worth a whole new dependency edge for).
    fn varint(&mut self) -> Option<i32> {
        let mut result: i32 = 0;
        for i in 0..5 {
            let byte = self.u8()?;
            result |= ((byte & 0x7F) as i32) << (7 * i);
            if byte & 0x80 == 0 {
                return Some(result);
            }
        }
        None
    }

    /// A VarInt-length-prefixed UTF-8 string field — only its byte span is consumed
    /// (content is never inspected), matching every caller's own "skip past, never
    /// mask" need for this field type.
    fn string(&mut self) -> Option<()> {
        let len = self.varint()?;
        if len < 0 {
            return None;
        }
        self.take(len as usize)?;
        Some(())
    }
}

fn canonical_varint_zero() -> u8 {
    0
}

/// `try_normalize_login`'s `Some`-path structural walk (this module's own doc
/// comment) — `None` on any unmodeled shape (the `dimensionType` holder's rare
/// direct/inline branch, `holder == 0`, is never exercised by a real vanilla or our
/// own server response and is the one case this decoder declines to guess).
fn try_normalize_login(body: &[u8]) -> Option<Vec<u8>> {
    let mut cur = Cur::new(body);
    cur.i32_be()?; // playerId
    cur.bool()?; // hardcore
    let levels = cur.varint()?;
    if levels < 0 {
        return None;
    }
    for _ in 0..levels {
        cur.string()?;
    }
    let holder = cur.varint()?; // dimensionType: Holder<DimensionType>
    if holder == 0 {
        return None; // inline-value branch, not modeled — safe bailout.
    }
    cur.string()?; // dimension: Identifier
    let seed_start = cur.pos;
    cur.i64_be()?; // seed — presence-checked, then masked below.
    let seed_end = cur.pos;

    let mut out = Vec::with_capacity(body.len());
    out.extend_from_slice(&body[..seed_start]);
    out.extend_from_slice(&0i64.to_be_bytes());
    out.extend_from_slice(&body[seed_end..]);
    Some(out)
}

/// Masks a single leading VarInt field to canonical `0` (one byte, `0x00`), copying
/// every following byte unchanged — the shared shape `player_position`'s
/// `teleport_id`, `set_entity_data`'s `id`, and the `move_entity_*` family's
/// `entity_id` all share.
fn normalize_leading_varint(body: &[u8]) -> Vec<u8> {
    let mut cur = Cur::new(body);
    if cur.varint().is_none() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(body.len());
    out.push(canonical_varint_zero());
    out.extend_from_slice(&body[cur.pos..]);
    out
}

/// `AddEntity`: masks the leading `id` VarInt plus the 16-byte `uuid` immediately
/// following it.
fn normalize_add_entity(body: &[u8]) -> Vec<u8> {
    let mut cur = Cur::new(body);
    if cur.varint().is_none() {
        return Vec::new();
    }
    if cur.take(16).is_none() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(body.len());
    out.push(canonical_varint_zero());
    out.extend(std::iter::repeat_n(0u8, 16));
    out.extend_from_slice(&body[cur.pos..]);
    out
}

/// `PlayerInfoUpdate`'s `actions` bitmask, per CLAIMS row 1's own confirmed enum
/// declaration order (a single-byte `EnumSet`, ≤ 8 actions).
const ACTION_ADD_PLAYER: u8 = 1 << 0;
const ACTION_INITIALIZE_CHAT: u8 = 1 << 1;
const ACTION_UPDATE_GAME_MODE: u8 = 1 << 2;
const ACTION_UPDATE_LISTED: u8 = 1 << 3;
const ACTION_UPDATE_LATENCY: u8 = 1 << 4;
const ACTION_UPDATE_DISPLAY_NAME: u8 = 1 << 5;
const ACTION_UPDATE_LIST_ORDER: u8 = 1 << 6;
const ACTION_UPDATE_HAT: u8 = 1 << 7;

/// Structural, `actions`-driven walk over every player `Entry` in the packet, masking
/// only `UPDATE_LATENCY`'s own VarInt wherever the bitmask says it is present.
/// `INITIALIZE_CHAT`'s signed-chat-session payload and `UPDATE_DISPLAY_NAME`'s
/// component payload are only ever reached in their common, empty (`Optional` absent)
/// form here — a genuinely present session/display-name payload's own nested shape is
/// not modeled (recorded in `docs/findings-for-planning.md`) and bails this one
/// instance out to `None` (whole-body-masked by the caller) rather than guessing.
fn try_normalize_player_info_update(body: &[u8]) -> Option<Vec<u8>> {
    let mut cur = Cur::new(body);
    let actions = cur.u8()?;
    let count = cur.varint()?;
    if count < 0 {
        return None;
    }

    let mut out = Vec::with_capacity(body.len());
    let mut last_copied = 0usize;

    for _ in 0..count {
        cur.take(16)?; // uuid, unmasked

        if actions & ACTION_ADD_PLAYER != 0 {
            cur.string()?; // name
            let props = cur.varint()?;
            if props < 0 {
                return None;
            }
            for _ in 0..props {
                cur.string()?; // name
                cur.string()?; // value
                if cur.bool()? {
                    cur.string()?; // signature
                }
            }
        }
        if actions & ACTION_INITIALIZE_CHAT != 0 && cur.bool()? {
            return None; // signed session payload present — not modeled.
        }
        if actions & ACTION_UPDATE_GAME_MODE != 0 {
            cur.varint()?;
        }
        if actions & ACTION_UPDATE_LISTED != 0 {
            cur.bool()?;
        }
        if actions & ACTION_UPDATE_LATENCY != 0 {
            let start = cur.pos;
            cur.varint()?;
            let end = cur.pos;
            out.extend_from_slice(&body[last_copied..start]);
            out.push(canonical_varint_zero());
            last_copied = end;
        }
        if actions & ACTION_UPDATE_DISPLAY_NAME != 0 && cur.bool()? {
            return None; // component payload present — not modeled.
        }
        if actions & ACTION_UPDATE_LIST_ORDER != 0 {
            cur.varint()?;
        }
        if actions & ACTION_UPDATE_HAT != 0 {
            cur.bool()?;
        }
    }

    out.extend_from_slice(&body[last_copied..]);
    Some(out)
}

/// `LoginFinished`'s own structural walk (NORMALIZATION_RULES's own doc comment has
/// the full field citation): `gameProfile.id` (16B UUID) and `.name` (VarInt-length
/// string) are copied unmasked; `.properties` (VarInt-counted, each `{name, value,
/// optional signature}`, all VarInt-length strings, the optional signature gated by a
/// leading bool) is walked past unmasked too (this harness's own bot always presents
/// zero properties, but the decoder does not assume that); the fixed-width 16B
/// `sessionId` immediately following is masked to canonical zero bytes.
fn try_normalize_login_finished(body: &[u8]) -> Option<Vec<u8>> {
    let mut cur = Cur::new(body);
    cur.take(16)?; // gameProfile.id (UUID), unmasked — deterministic in offline mode.
    cur.string()?; // gameProfile.name
    let props = cur.varint()?;
    if props < 0 {
        return None;
    }
    for _ in 0..props {
        cur.string()?; // property name
        cur.string()?; // property value
        if cur.bool()? {
            cur.string()?; // optional signature
        }
    }
    let session_id_start = cur.pos;
    cur.take(16)?; // sessionId (UUID) — masked below.
    let session_id_end = cur.pos;

    let mut out = Vec::with_capacity(body.len());
    out.extend_from_slice(&body[..session_id_start]);
    out.extend(std::iter::repeat_n(0u8, 16));
    out.extend_from_slice(&body[session_id_end..]);
    Some(out)
}

/// `minecraft:brand`'s own wire form, `channel: Identifier` (VarInt-length-prefixed
/// UTF-8 string): the VarInt length byte `0x0f` (15) followed by the 15 ASCII bytes
/// of `"minecraft:brand"` — matched verbatim against `CustomPayload`'s own leading
/// bytes rather than decoded field-by-field, since this harness never needs the
/// channel string for anything but this one equality check.
const BRAND_CHANNEL_WIRE: &[u8] = b"\x0fminecraft:brand";

/// `CustomPayload`'s own structural walk (NORMALIZATION_RULES's own doc comment has
/// the full field citation): the leading `channel` field is copied unmasked; when it
/// is exactly `minecraft:brand`, the entire remainder (`BrandPayload`'s own single
/// `brand` string, which is the whole rest of the packet) is dropped from the
/// compared bytes — the same "drop the whole variable-length field" precedent
/// `normalize_set_time`'s own `clockUpdates` map already establishes. Any other
/// channel is returned unchanged (`Some(body.to_vec())`), never masked — this
/// harness never sends/observes one, so a genuine difference there must still show up
/// as a real divergence.
fn try_normalize_custom_payload(body: &[u8]) -> Option<Vec<u8>> {
    let mut cur = Cur::new(body);
    let channel_start = cur.pos;
    cur.string()?; // channel: Identifier
    let channel_end = cur.pos;

    if &body[channel_start..channel_end] != BRAND_CHANNEL_WIRE {
        return Some(body.to_vec());
    }
    Some(body[..channel_end].to_vec())
}

/// `SetTime`: `gameTime` (fixed offset 0, width 8) is masked precisely; the entire
/// `clockUpdates` map that follows is masked wholesale (dropped from the compared
/// bytes) rather than only its *values* — `ClockNetworkState`'s own wire width is not
/// modeled here (recorded in `docs/findings-for-planning.md`), so this decoder cannot
/// safely locate where one entry's value ends and the next key begins; masking the
/// whole map is the safe direction to fail in (never a byte-alignment guess), at the
/// cost of also hiding the map's own *keys* from comparison — a documented, bounded
/// simplification, narrower than §3.8's literal "keys stay unmasked" text.
fn normalize_set_time(body: &[u8]) -> Vec<u8> {
    if body.len() < 8 {
        return Vec::new();
    }
    vec![0u8; 8]
}

/// Applies `NORMALIZATION_RULES` (matched by `packet.packet_name`; an unresolved or
/// unlisted name returns `body` unchanged — §3.8's explicit default). Every
/// structural decoder above is infallible in its public shape (always returns a
/// `Vec<u8>`, never panics) — a body this decoder cannot confidently walk to its own
/// end degrades to an empty canonical sentinel (this module's own doc comment has the
/// full "safe direction to fail" rationale), never a byte-for-byte passthrough of a
/// field this table claims to mask.
pub fn normalize_body(packet: &CapturedPacket) -> Vec<u8> {
    let Some(name) = packet.packet_name.as_deref() else {
        return packet.body.clone();
    };
    let Some(rule) = NORMALIZATION_RULES.iter().find(|r| r.packet_name == name) else {
        return packet.body.clone();
    };
    if rule.mask_whole_body {
        return Vec::new();
    }
    match name {
        "login" => try_normalize_login(&packet.body).unwrap_or_default(),
        "player_position"
        | "set_entity_data"
        | "move_entity_pos"
        | "move_entity_pos_rot"
        | "move_entity_rot" => normalize_leading_varint(&packet.body),
        "add_entity" => normalize_add_entity(&packet.body),
        "player_info_update" => try_normalize_player_info_update(&packet.body).unwrap_or_default(),
        "login_finished" => try_normalize_login_finished(&packet.body).unwrap_or_default(),
        "custom_payload" => try_normalize_custom_payload(&packet.body).unwrap_or_default(),
        "set_time" => normalize_set_time(&packet.body),
        _ => packet.body.clone(),
    }
}

/// The chunk position `LevelChunkWithLight` bodies carry as their own first two
/// fields: `x: i32` (offset 0), `z: i32` (offset 4), both fixed-width, big-endian,
/// non-VarInt (TEST-D57 pass, CLAIMS row 2 — CONFIRMED).
fn level_chunk_xz(packet: &CapturedPacket) -> Option<(i32, i32)> {
    if packet.packet_name.as_deref() != Some("level_chunk_with_light") {
        return None;
    }
    let mut cur = Cur::new(&packet.body);
    let x = cur.i32_be()?;
    let z = cur.i32_be()?;
    Some((x, z))
}

/// The §3.8 cross-packet chunk-batch reordering pass, applied in place: within each
/// maximal contiguous run of `LevelChunkWithLight`-named packets immediately
/// following a `ChunkBatchStart` and immediately followed by a `ChunkBatchFinished`,
/// the run is re-sorted by its own chunk `(x, z)` position — never reordered across a
/// batch boundary or past any other packet type. A `LevelChunkWithLight` packet
/// appearing outside any such bracket (no preceding `ChunkBatchStart` / no following
/// `ChunkBatchFinished` immediately bounding its own run) is left exactly where it
/// was.
pub fn normalize_chunk_batch_order(packets: &mut [CapturedPacket]) {
    let mut i = 0;
    while i < packets.len() {
        if packets[i].packet_name.as_deref() != Some("chunk_batch_start") {
            i += 1;
            continue;
        }
        let run_start = i + 1;
        let mut run_end = run_start;
        while run_end < packets.len()
            && packets[run_end].packet_name.as_deref() == Some("level_chunk_with_light")
        {
            run_end += 1;
        }
        if run_end < packets.len()
            && packets[run_end].packet_name.as_deref() == Some("chunk_batch_finished")
        {
            packets[run_start..run_end]
                .sort_by_key(|p| level_chunk_xz(p).unwrap_or((i32::MAX, i32::MAX)));
        }
        i = run_end + 1;
    }
}

/// `true` iff `step_id` is a redstone-corpus contraption id (`redstone/<category>/
/// <name>`, `rc_gametest::spec::ContraptionSpec::id`'s own doc comment) rather than a
/// `SESSION_STEPS` scripted-session step (every one of which is namespaced
/// `session/...`) — `apply_observation_window`'s own gate: session steps keep
/// today's full comparison unconditionally (§ deliverable 1).
pub fn is_contraption_step(step_id: &str) -> bool {
    step_id.starts_with("redstone/")
}

/// A `value` is an unsigned `bits`-wide field already isolated by the caller (masked,
/// no stray high bits). Returns its two's-complement signed interpretation — restated
/// locally from `crates/server/src/play/packets.rs::sign_extend` (`rc-gametest` never
/// depends on `crates/server`, this module's own doc comment).
fn sign_extend(value: i64, bits: u32) -> i64 {
    let sign_bit = 1i64 << (bits - 1);
    if value >= sign_bit {
        value - (sign_bit << 1)
    } else {
        value
    }
}

/// The single-block position long `block_update`/`block_event` both carry (§
/// deliverable 1's own spec: x 26 bits, z 26 bits, y 12 bits) — `crates/server/src/
/// play/packets.rs::unpack_position`'s own packing, TEST-D57 CONFIRMED there,
/// restated here.
fn unpack_block_position(packed: i64) -> (i32, i32, i32) {
    let raw_x = (packed >> 38) & 0x3FF_FFFF;
    let raw_z = (packed >> 12) & 0x3FF_FFFF;
    let raw_y = packed & 0xFFF;
    (
        sign_extend(raw_x, 26) as i32,
        sign_extend(raw_y, 12) as i32,
        sign_extend(raw_z, 26) as i32,
    )
}

/// § deliverable 1's own shared coordinate shapes — `clippy::type_complexity`'s own
/// threshold flags the bare nested-tuple form once it appears in enough signatures
/// (`apply_observation_window`'s own `bbox` parameter, first), so every function
/// below that used to spell out `(i32, i32, i32)`/`((i32, i32, i32), (i32, i32, i32))`
/// inline now names these instead — purely a readability alias, never a new runtime
/// shape (`BlockBounds` is still exactly the `(min, max)` pair `ContraptionBounds`'s
/// own doc comment already describes).
type BlockPos = (i32, i32, i32);
type BlockBounds = (BlockPos, BlockPos);

/// `section_blocks_update`'s own leading `section_pos` field, decoded to the
/// section's own absolute block-coordinate range (inclusive, 16 blocks per axis) —
/// `crates/server/src/play/packets.rs::pack_section_position`'s own packing (22-bit
/// `chunk_x`, 20-bit `section_y`, 22-bit `chunk_z`), TEST-D57 CONFIRMED there,
/// restated here (this module's own doc comment: `rc-gametest` never depends on
/// `crates/server`). Only the section's own address is needed for the deliverable-1
/// bounding-box filter (a whole-packet keep/drop decision, module doc comment on
/// `block_change_in_bounds`) — the per-block `states` entries are never decoded here.
fn section_block_range(packed: i64) -> BlockBounds {
    let raw_x = (packed >> 42) & 0x3F_FFFF;
    let raw_y = packed & 0xF_FFFF;
    let raw_z = (packed >> 20) & 0x3F_FFFF;
    let section_x = sign_extend(raw_x, 22) as i32;
    let section_y = sign_extend(raw_y, 20) as i32;
    let section_z = sign_extend(raw_z, 22) as i32;
    let min = (section_x * 16, section_y * 16, section_z * 16);
    let max = (min.0 + 15, min.1 + 15, min.2 + 15);
    (min, max)
}

fn pos_in_bounds(pos: BlockPos, min: BlockPos, max: BlockPos) -> bool {
    pos.0 >= min.0
        && pos.0 <= max.0
        && pos.1 >= min.1
        && pos.1 <= max.1
        && pos.2 >= min.2
        && pos.2 <= max.2
}

fn ranges_overlap(a: BlockBounds, b: BlockBounds) -> bool {
    let (a_min, a_max) = a;
    let (b_min, b_max) = b;
    a_min.0 <= b_max.0
        && a_max.0 >= b_min.0
        && a_min.1 <= b_max.1
        && a_max.1 >= b_min.1
        && a_min.2 <= b_max.2
        && a_max.2 >= b_min.2
}

/// § deliverable 1: `true` keeps `packet` in the comparison, `false` drops it.
/// `block_update`/`block_event` each carry exactly one absolute block position
/// (`unpack_block_position`) — kept iff that position falls inside `[min, max]`.
/// `section_blocks_update` carries one *section* covering up to 16³ blocks
/// (`section_block_range`) — kept iff that section's own range overlaps `[min, max]`
/// at all (a whole-packet decision, never a per-entry re-encode: every real
/// contraption's own committed footprint is far smaller than one 16-block section,
/// per `WIRE_SLOT_A`'s own measured range in `redstone_wire_capture.rs`, so a section
/// overlapping the bounding box at all is itself already strong evidence the batch is
/// about this contraption, not a coincidentally-adjacent one — `xtask`'s own 64-block
/// `world_origin_for` spacing/`WIRE_SLOT_A`/`WIRE_SLOT_B`'s own margins keep every
/// other contraption's own footprint out of the same section entirely). Any other
/// packet name (including one this module cannot resolve, `packet_name: None`) is
/// never position-filtered at all — `apply_observation_window`'s own caller only
/// invokes this for the three block-change names; a body this function's own decoder
/// cannot walk to its own end (a truncated or otherwise malformed body) is kept
/// unconditionally, the same "safe direction to fail" every other decoder in this
/// module establishes — never silently dropped on a decode failure that might be
/// hiding a genuine divergence.
fn block_change_in_bounds(packet: &CapturedPacket, min: BlockPos, max: BlockPos) -> bool {
    match packet.packet_name.as_deref() {
        Some("block_update") | Some("block_event") => {
            let mut cur = Cur::new(&packet.body);
            match cur.i64_be() {
                Some(packed) => pos_in_bounds(unpack_block_position(packed), min, max),
                None => true,
            }
        }
        Some("section_blocks_update") => {
            let mut cur = Cur::new(&packet.body);
            match cur.i64_be() {
                Some(packed) => ranges_overlap(section_block_range(packed), (min, max)),
                None => true,
            }
        }
        _ => true,
    }
}

/// § deliverable 1: the contraption observation window, applied in place —
/// `is_contraption_step(step_id)` gates the whole function (every scripted-session
/// step returns immediately, unmodified: "session steps keep today's full
/// comparison"). For a contraption step: first drops every packet whose own `index`
/// is `< observe_from` (the setup phase — pre-clear, real placement, setblock
/// prelude, tick barrier — this step's own capture recorded before its scripted
/// actions began, §`StepCapture::observe_from`'s own doc comment); then, only when
/// `bbox` is `Some` (the caller could not resolve this step id back to a committed
/// `ContraptionSpec` — an unknown/renamed contraption id — leaves the position filter
/// off entirely rather than guessing, this module's own safe-default posture),
/// applies `block_change_in_bounds` to every remaining `block_update`/`block_event`/
/// `section_blocks_update` packet. `bbox` is the caller's own already-expanded box
/// (`rc_gametest::spec::bounding_box(spec)` widened by one block on every axis, per
/// the blueprint's own "expanded by one block" text) — this module never resolves a
/// step id back to a spec file itself (`xtask` alone reads the corpus RON files).
pub fn apply_observation_window(
    step_id: &str,
    observe_from: u32,
    packets: &mut Vec<CapturedPacket>,
    bbox: Option<BlockBounds>,
) {
    if !is_contraption_step(step_id) {
        return;
    }
    packets.retain(|p| p.index >= observe_from);
    if let Some((min, max)) = bbox {
        packets.retain(|p| block_change_in_bounds(p, min, max));
    }
}

pub struct PacketTypeDiff {
    pub packet_id: i32,
    /// The first `Some` name any captured instance of this `packet_id` carried, on
    /// either side — report-only.
    pub packet_name: Option<String>,
    pub oracle_only_bodies: Vec<(Vec<u8>, usize)>, // (normalized body, excess count)
    pub ours_only_bodies: Vec<(Vec<u8>, usize)>,
}

#[derive(Default)]
pub struct ProtocolDiffReport {
    pub mismatches: Vec<PacketTypeDiff>,
    pub missing_in_oracle: Vec<i32>, // packet_id present only in ours
    pub missing_in_ours: Vec<i32>,   // packet_id present only in oracle
    /// The first `Some` `packet_name` observed for each `packet_id` this step's own
    /// two packet lists carried, on either side — covers every id in `mismatches`
    /// *and* every id in `missing_in_oracle`/`missing_in_ours` (unlike
    /// `PacketTypeDiff::packet_name`, which only names a mismatched id). TEST-D59's
    /// `known_divergences::resolve_step` resolves every mismatch's own packet name
    /// through this map — an id absent from it (every packet on both sides that
    /// carried no `packet_name` at capture time) can never match any register entry.
    pub packet_names: std::collections::BTreeMap<i32, String>,
}

/// §3.9, for one step's two packet lists (chunk-batch reordering + normalization
/// already applied by the caller — kept as two separable steps for the acceptance
/// tests' own benefit, mirroring `diff_captures`'s pure-function shape). Grouping and
/// pass/fail are always keyed on the raw `packet_id`, never the best-effort
/// `packet_name`.
pub fn diff_step(oracle: &[CapturedPacket], ours: &[CapturedPacket]) -> ProtocolDiffReport {
    use std::collections::BTreeMap;

    let mut report = ProtocolDiffReport::default();
    let mut names: BTreeMap<i32, String> = BTreeMap::new();

    let mut oracle_groups: BTreeMap<i32, BTreeMap<Vec<u8>, usize>> = BTreeMap::new();
    for p in oracle {
        if let Some(n) = &p.packet_name {
            names.entry(p.packet_id).or_insert_with(|| n.clone());
        }
        *oracle_groups
            .entry(p.packet_id)
            .or_default()
            .entry(p.body.clone())
            .or_insert(0) += 1;
    }
    let mut ours_groups: BTreeMap<i32, BTreeMap<Vec<u8>, usize>> = BTreeMap::new();
    for p in ours {
        if let Some(n) = &p.packet_name {
            names.entry(p.packet_id).or_insert_with(|| n.clone());
        }
        *ours_groups
            .entry(p.packet_id)
            .or_default()
            .entry(p.body.clone())
            .or_insert(0) += 1;
    }

    let all_ids: std::collections::BTreeSet<i32> = oracle_groups
        .keys()
        .chain(ours_groups.keys())
        .copied()
        .collect();

    for id in all_ids {
        match (oracle_groups.get(&id), ours_groups.get(&id)) {
            (Some(_), None) => report.missing_in_ours.push(id),
            (None, Some(_)) => report.missing_in_oracle.push(id),
            (None, None) => unreachable!("id came from the union of both key sets"),
            (Some(oracle_bodies), Some(ours_bodies)) => {
                if oracle_bodies == ours_bodies {
                    continue;
                }
                let all_bodies: std::collections::BTreeSet<&Vec<u8>> =
                    oracle_bodies.keys().chain(ours_bodies.keys()).collect();
                let mut oracle_only = Vec::new();
                let mut ours_only = Vec::new();
                for body in all_bodies {
                    let oracle_count = oracle_bodies.get(body).copied().unwrap_or(0);
                    let ours_count = ours_bodies.get(body).copied().unwrap_or(0);
                    if oracle_count > ours_count {
                        oracle_only.push((body.clone(), oracle_count - ours_count));
                    } else if ours_count > oracle_count {
                        ours_only.push((body.clone(), ours_count - oracle_count));
                    }
                }
                report.mismatches.push(PacketTypeDiff {
                    packet_id: id,
                    packet_name: names.get(&id).cloned(),
                    oracle_only_bodies: oracle_only,
                    ours_only_bodies: ours_only,
                });
            }
        }
    }

    report.packet_names = names;
    report
}

/// Runs `diff_step` per `step_id` present in either file (a step missing on one side
/// entirely is its own reported case, never silently skipped — mirrors
/// `PlacementDiffReport`'s own discipline): every step's own two packet lists are
/// first passed through `normalize_chunk_batch_order`, then every packet's `body` is
/// replaced by `normalize_body`'s result, before `diff_step` ever compares them.
/// `step_id -> ((min_x,min_y,min_z),(max_x,max_y,max_z))`, already expanded by one
/// block on every axis — `diff_captures`'s own bounding-box source for § deliverable
/// 1's `apply_observation_window` (only `xtask`, which alone reads the corpus RON
/// files via `rc_gametest::spec::load_spec`/`bounding_box`, ever populates this; a
/// step id absent from the map (every scripted-session step, and a contraption id
/// this map's own builder could not resolve) gets `apply_observation_window`'s own
/// `bbox: None` — the observe-from truncation still applies, the position filter
/// does not).
pub type ContraptionBounds = std::collections::BTreeMap<String, BlockBounds>;

/// Runs `diff_step` per `step_id` present in either file (a step missing on one side
/// entirely is its own reported case, never silently skipped — mirrors
/// `PlacementDiffReport`'s own discipline): every step's own two packet lists are
/// first passed through `apply_observation_window` (§ deliverable 1 — a no-op for
/// every scripted-session step), then `normalize_chunk_batch_order`, then every
/// packet's `body` is replaced by `normalize_body`'s result, before `diff_step` ever
/// compares them.
pub fn diff_captures(
    oracle: &ProtocolCaptureFile,
    ours: &ProtocolCaptureFile,
    contraption_bounds: &ContraptionBounds,
) -> std::collections::BTreeMap<String, ProtocolDiffReport> {
    use std::collections::BTreeMap;

    let oracle_by_step: BTreeMap<&str, &StepCapture> = oracle
        .steps
        .iter()
        .map(|s| (s.step_id.as_str(), s))
        .collect();
    let ours_by_step: BTreeMap<&str, &StepCapture> =
        ours.steps.iter().map(|s| (s.step_id.as_str(), s)).collect();
    let all_step_ids: std::collections::BTreeSet<&str> = oracle_by_step
        .keys()
        .chain(ours_by_step.keys())
        .copied()
        .collect();

    let mut result = BTreeMap::new();
    for step_id in all_step_ids {
        let mut oracle_packets: Vec<CapturedPacket> = oracle_by_step
            .get(step_id)
            .map(|s| s.packets.clone())
            .unwrap_or_default();
        let mut ours_packets: Vec<CapturedPacket> = ours_by_step
            .get(step_id)
            .map(|s| s.packets.clone())
            .unwrap_or_default();
        let oracle_observe_from = oracle_by_step.get(step_id).map_or(0, |s| s.observe_from);
        let ours_observe_from = ours_by_step.get(step_id).map_or(0, |s| s.observe_from);
        let bbox = contraption_bounds.get(step_id).copied();

        apply_observation_window(step_id, oracle_observe_from, &mut oracle_packets, bbox);
        apply_observation_window(step_id, ours_observe_from, &mut ours_packets, bbox);

        normalize_chunk_batch_order(&mut oracle_packets);
        normalize_chunk_batch_order(&mut ours_packets);
        for p in &mut oracle_packets {
            let normalized = normalize_body(p);
            p.body = normalized;
        }
        for p in &mut ours_packets {
            let normalized = normalize_body(p);
            p.body = normalized;
        }

        let report = diff_step(&oracle_packets, &ours_packets);
        result.insert(step_id.to_string(), report);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(source: &str, steps: Vec<StepCapture>) -> ProtocolCaptureFile {
        ProtocolCaptureFile {
            format_version: PROTOCOL_CAPTURE_FORMAT_VERSION,
            source_label: source.to_string(),
            steps,
        }
    }

    fn pkt(index: u32, packet_id: i32, body: Vec<u8>, name: Option<&str>) -> CapturedPacket {
        CapturedPacket {
            index,
            packet_id,
            body,
            packet_name: name.map(str::to_string),
        }
    }

    #[test]
    fn identical_captures_diff_clean() {
        // A `keep_alive`-named packet with differing raw challenge bytes (masked to
        // clean), plus a `block_update`-named packet with identical bytes on both
        // sides (unlisted, therefore compared unmasked but equal regardless).
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![
                    pkt(0, 33, vec![1, 2, 3, 4, 5, 6, 7, 8], Some("keep_alive")),
                    pkt(1, 9, vec![10, 20, 30], Some("block_update")),
                ],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![
                    pkt(0, 33, vec![9, 9, 9, 9, 9, 9, 9, 9], Some("keep_alive")),
                    pkt(1, 9, vec![10, 20, 30], Some("block_update")),
                ],
            }],
        );
        let report = diff_captures(&oracle, &ours, &ContraptionBounds::new());
        let step = report.get("session/spawn").expect("step present");
        assert!(step.mismatches.is_empty());
        assert!(step.missing_in_oracle.is_empty());
        assert!(step.missing_in_ours.is_empty());
    }

    #[test]
    fn an_unmasked_field_difference_is_reported() {
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![pkt(0, 9, vec![10, 20, 30], Some("block_update"))],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![pkt(0, 9, vec![10, 20, 31], Some("block_update"))],
            }],
        );
        let report = diff_captures(&oracle, &ours, &ContraptionBounds::new());
        let step = report.get("session/spawn").expect("step present");
        assert_eq!(step.mismatches.len(), 1);
        let diff = &step.mismatches[0];
        assert_eq!(diff.packet_id, 9);
        assert_eq!(diff.oracle_only_bodies, vec![(vec![10, 20, 30], 1)]);
        assert_eq!(diff.ours_only_bodies, vec![(vec![10, 20, 31], 1)]);
    }

    #[test]
    fn keep_alive_masks_the_whole_body_not_just_a_prefix() {
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![pkt(
                    0,
                    33,
                    vec![0xAA; 32], // every byte differs from `ours`, well past 8B.
                    Some("keep_alive"),
                )],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![pkt(0, 33, vec![0xBB; 40], Some("keep_alive"))],
            }],
        );
        let report = diff_captures(&oracle, &ours, &ContraptionBounds::new());
        let step = report.get("session/spawn").expect("step present");
        assert!(step.mismatches.is_empty());
    }

    #[test]
    fn an_unlisted_packet_name_is_never_masked() {
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![pkt(0, 200, vec![1, 2, 3], Some("totally_custom_packet"))],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![pkt(0, 200, vec![1, 2, 4], Some("totally_custom_packet"))],
            }],
        );
        let report = diff_captures(&oracle, &ours, &ContraptionBounds::new());
        let step = report.get("session/spawn").expect("step present");
        assert_eq!(step.mismatches.len(), 1);
    }

    #[test]
    fn packet_id_absent_on_one_side_is_a_presence_set_mismatch_not_silently_dropped() {
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![pkt(0, 77, vec![1], Some("some_packet"))],
            }],
        );
        let report = diff_captures(&oracle, &ours, &ContraptionBounds::new());
        let step = report.get("session/spawn").expect("step present");
        assert_eq!(step.missing_in_oracle, vec![77]);
        assert!(step.missing_in_ours.is_empty());
        assert!(step.mismatches.is_empty());
    }

    #[test]
    fn a_repeated_normalized_body_with_differing_counts_is_a_mismatch() {
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![
                    pkt(0, 42, vec![7, 7, 7], Some("some_packet")),
                    pkt(1, 42, vec![7, 7, 7], Some("some_packet")),
                ],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: vec![pkt(0, 42, vec![7, 7, 7], Some("some_packet"))],
            }],
        );
        let report = diff_captures(&oracle, &ours, &ContraptionBounds::new());
        let step = report.get("session/spawn").expect("step present");
        assert_eq!(step.mismatches.len(), 1);
        let diff = &step.mismatches[0];
        assert_eq!(diff.oracle_only_bodies, vec![(vec![7, 7, 7], 1)]);
        assert!(diff.ours_only_bodies.is_empty());
    }

    fn chunk_body(x: i32, z: i32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&x.to_be_bytes());
        out.extend_from_slice(&z.to_be_bytes());
        out.extend_from_slice(b"payload");
        out
    }

    #[test]
    fn normalize_chunk_batch_order_sorts_within_one_batch_only() {
        let mut packets = vec![
            pkt(0, 1, vec![], Some("chunk_batch_start")),
            pkt(1, 2, chunk_body(5, 5), Some("level_chunk_with_light")),
            pkt(2, 2, chunk_body(1, 1), Some("level_chunk_with_light")),
            pkt(3, 3, vec![], Some("chunk_batch_finished")),
            // A second, independent batch.
            pkt(4, 1, vec![], Some("chunk_batch_start")),
            pkt(5, 2, chunk_body(9, 9), Some("level_chunk_with_light")),
            pkt(6, 2, chunk_body(0, 0), Some("level_chunk_with_light")),
            pkt(7, 3, vec![], Some("chunk_batch_finished")),
            // A `level_chunk_with_light` packet outside any bracket at all — must
            // never move.
            pkt(8, 2, chunk_body(3, 3), Some("level_chunk_with_light")),
        ];
        normalize_chunk_batch_order(&mut packets);

        assert_eq!(packets[1].body, chunk_body(1, 1));
        assert_eq!(packets[2].body, chunk_body(5, 5));
        assert_eq!(packets[5].body, chunk_body(0, 0));
        assert_eq!(packets[6].body, chunk_body(9, 9));
        // The unbracketed packet at the tail is untouched.
        assert_eq!(packets[8].body, chunk_body(3, 3));
    }

    /// One `login_finished` body: 16B uuid + VarInt-length name + VarInt `0`
    /// properties + 16B session id — matches `try_normalize_login_finished`'s own
    /// doc comment field order exactly.
    fn login_finished_body(uuid: u8, name: &str, session_id: u8) -> Vec<u8> {
        let mut out = vec![uuid; 16];
        out.push(name.len() as u8); // name is short enough for a 1-byte VarInt.
        out.extend_from_slice(name.as_bytes());
        out.push(0); // zero properties
        out.extend(std::iter::repeat_n(session_id, 16));
        out
    }

    #[test]
    fn login_finished_masks_only_the_trailing_session_id() {
        let masked = try_normalize_login_finished(&login_finished_body(0xAB, "bot", 0xCD))
            .expect("well-formed body normalizes");
        let mut expected = vec![0xABu8; 16];
        expected.push(3);
        expected.extend_from_slice(b"bot");
        expected.push(0);
        expected.extend(std::iter::repeat_n(0u8, 16));
        assert_eq!(masked, expected);
    }

    #[test]
    fn login_finished_session_id_difference_normalizes_clean() {
        // Same profile (uuid, name), different session id — the exact M3.5-B03
        // divergence this normalizer exists to close.
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/login".to_string(),
                observe_from: 0,
                packets: vec![pkt(
                    0,
                    2,
                    login_finished_body(0xAB, "bot", 0xCD),
                    Some("login_finished"),
                )],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/login".to_string(),
                observe_from: 0,
                packets: vec![pkt(
                    0,
                    2,
                    login_finished_body(0xAB, "bot", 0xEF),
                    Some("login_finished"),
                )],
            }],
        );
        let report = diff_captures(&oracle, &ours, &ContraptionBounds::new());
        let step = report.get("session/login").expect("step present");
        assert!(step.mismatches.is_empty());
    }

    #[test]
    fn login_finished_username_difference_still_reported() {
        // A genuinely differing (unmasked) field must still surface.
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/login".to_string(),
                observe_from: 0,
                packets: vec![pkt(
                    0,
                    2,
                    login_finished_body(0xAB, "bot", 0xCD),
                    Some("login_finished"),
                )],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/login".to_string(),
                observe_from: 0,
                packets: vec![pkt(
                    0,
                    2,
                    login_finished_body(0xAB, "not-bot", 0xCD),
                    Some("login_finished"),
                )],
            }],
        );
        let report = diff_captures(&oracle, &ours, &ContraptionBounds::new());
        let step = report.get("session/login").expect("step present");
        assert_eq!(step.mismatches.len(), 1);
    }

    fn brand_payload_body(brand: &str) -> Vec<u8> {
        let mut out = vec![0x0f]; // "minecraft:brand" is 15 bytes, a 1-byte VarInt.
        out.extend_from_slice(b"minecraft:brand");
        out.push(brand.len() as u8);
        out.extend_from_slice(brand.as_bytes());
        out
    }

    #[test]
    fn custom_payload_masks_only_the_brand_channels_payload() {
        let masked = try_normalize_custom_payload(&brand_payload_body("vanilla"))
            .expect("well-formed body normalizes");
        let mut expected = vec![0x0f];
        expected.extend_from_slice(b"minecraft:brand");
        assert_eq!(masked, expected);
    }

    #[test]
    fn custom_payload_brand_difference_normalizes_clean() {
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/configuration".to_string(),
                observe_from: 0,
                packets: vec![pkt(
                    0,
                    1,
                    brand_payload_body("vanilla"),
                    Some("custom_payload"),
                )],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/configuration".to_string(),
                observe_from: 0,
                packets: vec![pkt(
                    0,
                    1,
                    brand_payload_body("rusty-clanker"),
                    Some("custom_payload"),
                )],
            }],
        );
        let report = diff_captures(&oracle, &ours, &ContraptionBounds::new());
        let step = report.get("session/configuration").expect("step present");
        assert!(step.mismatches.is_empty());
    }

    #[test]
    fn custom_payload_non_brand_channel_is_never_masked() {
        // A channel other than `minecraft:brand` must still be compared unmasked —
        // the normalizer never blind-masks a channel this harness has never seen.
        let mut other_channel = vec![0x0d]; // "minecraft:test" is 14 bytes.
        other_channel.extend_from_slice(b"minecraft:test");
        other_channel.extend_from_slice(&[1, 2, 3]);
        let mut other_channel_differing = other_channel.clone();
        *other_channel_differing.last_mut().unwrap() = 4;

        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/configuration".to_string(),
                observe_from: 0,
                packets: vec![pkt(0, 1, other_channel, Some("custom_payload"))],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/configuration".to_string(),
                observe_from: 0,
                packets: vec![pkt(0, 1, other_channel_differing, Some("custom_payload"))],
            }],
        );
        let report = diff_captures(&oracle, &ours, &ContraptionBounds::new());
        let step = report.get("session/configuration").expect("step present");
        assert_eq!(step.mismatches.len(), 1);
    }

    #[test]
    fn round_trips_through_postcard_on_disk() {
        let dir =
            std::env::temp_dir().join(format!("protocol-capture-self-test-{}", std::process::id()));
        let path = dir.join("capture.postcard");
        let capture = cap(
            "oracle:deadbeef",
            vec![StepCapture {
                step_id: "session/login".to_string(),
                observe_from: 0,
                packets: vec![pkt(0, 1, vec![1, 2, 3], Some("login"))],
            }],
        );
        write_capture(&path, &capture).unwrap();
        let read_back = read_capture(&path).unwrap();
        assert_eq!(read_back, capture);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
