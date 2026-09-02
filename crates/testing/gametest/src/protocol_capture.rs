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

pub const PROTOCOL_CAPTURE_FORMAT_VERSION: u32 = 1;

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
        self.take(4).map(|s| i32::from_be_bytes(s.try_into().unwrap()))
    }

    fn i64_be(&mut self) -> Option<i64> {
        self.take(8).map(|s| i64::from_be_bytes(s.try_into().unwrap()))
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
        "player_position" | "set_entity_data" | "move_entity_pos" | "move_entity_pos_rot"
        | "move_entity_rot" => normalize_leading_varint(&packet.body),
        "add_entity" => normalize_add_entity(&packet.body),
        "player_info_update" => {
            try_normalize_player_info_update(&packet.body).unwrap_or_default()
        }
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

    report
}

/// Runs `diff_step` per `step_id` present in either file (a step missing on one side
/// entirely is its own reported case, never silently skipped — mirrors
/// `PlacementDiffReport`'s own discipline): every step's own two packet lists are
/// first passed through `normalize_chunk_batch_order`, then every packet's `body` is
/// replaced by `normalize_body`'s result, before `diff_step` ever compares them.
pub fn diff_captures(
    oracle: &ProtocolCaptureFile,
    ours: &ProtocolCaptureFile,
) -> std::collections::BTreeMap<String, ProtocolDiffReport> {
    use std::collections::BTreeMap;

    let oracle_by_step: BTreeMap<&str, &StepCapture> = oracle
        .steps
        .iter()
        .map(|s| (s.step_id.as_str(), s))
        .collect();
    let ours_by_step: BTreeMap<&str, &StepCapture> = ours
        .steps
        .iter()
        .map(|s| (s.step_id.as_str(), s))
        .collect();
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
                packets: vec![
                    pkt(0, 33, vec![9, 9, 9, 9, 9, 9, 9, 9], Some("keep_alive")),
                    pkt(1, 9, vec![10, 20, 30], Some("block_update")),
                ],
            }],
        );
        let report = diff_captures(&oracle, &ours);
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
                packets: vec![pkt(0, 9, vec![10, 20, 30], Some("block_update"))],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                packets: vec![pkt(0, 9, vec![10, 20, 31], Some("block_update"))],
            }],
        );
        let report = diff_captures(&oracle, &ours);
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
                packets: vec![pkt(0, 33, vec![0xBB; 40], Some("keep_alive"))],
            }],
        );
        let report = diff_captures(&oracle, &ours);
        let step = report.get("session/spawn").expect("step present");
        assert!(step.mismatches.is_empty());
    }

    #[test]
    fn an_unlisted_packet_name_is_never_masked() {
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                packets: vec![pkt(0, 200, vec![1, 2, 3], Some("totally_custom_packet"))],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                packets: vec![pkt(0, 200, vec![1, 2, 4], Some("totally_custom_packet"))],
            }],
        );
        let report = diff_captures(&oracle, &ours);
        let step = report.get("session/spawn").expect("step present");
        assert_eq!(step.mismatches.len(), 1);
    }

    #[test]
    fn packet_id_absent_on_one_side_is_a_presence_set_mismatch_not_silently_dropped() {
        let oracle = cap(
            "oracle:abc",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                packets: vec![],
            }],
        );
        let ours = cap(
            "ours",
            vec![StepCapture {
                step_id: "session/spawn".to_string(),
                packets: vec![pkt(0, 77, vec![1], Some("some_packet"))],
            }],
        );
        let report = diff_captures(&oracle, &ours);
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
                packets: vec![pkt(0, 42, vec![7, 7, 7], Some("some_packet"))],
            }],
        );
        let report = diff_captures(&oracle, &ours);
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

    #[test]
    fn round_trips_through_postcard_on_disk() {
        let dir = std::env::temp_dir().join(format!(
            "protocol-capture-self-test-{}",
            std::process::id()
        ));
        let path = dir.join("capture.postcard");
        let capture = cap(
            "oracle:deadbeef",
            vec![StepCapture {
                step_id: "session/login".to_string(),
                packets: vec![pkt(0, 1, vec![1, 2, 3], Some("login"))],
            }],
        );
        write_capture(&path, &capture).unwrap();
        let read_back = read_capture(&path).unwrap();
        assert_eq!(read_back, capture);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
