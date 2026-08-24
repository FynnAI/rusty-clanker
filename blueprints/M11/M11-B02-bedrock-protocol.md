# M11-B02 — Bedrock Game Protocol (`rc-bedrock-protocol`)

| Field | Content |
|---|---|
| ID | M11-B02 |
| Milestone | M11 — Bedrock Cross-Play |
| Prerequisites | **M11-B01** (`rc-bedrock-raknet`) — read in full. This blueprint's encoded bytes are exactly the opaque application-payload stream `RaknetSession::send(channel, reliability, payload)` / `RaknetSession::recv() -> Option<Bytes>` moves (M11-B01 §L) — this blueprint's own output is what a future translator hands to that API, in practice predominantly `Reliability::ReliableOrdered` on `OrderChannel(0)` per M11-B01 §L's own note. **No Cargo dependency edge exists between the two crates** (CROSS-D5 rule 5 draws no such edge; restated in Context §A) — M11-B01 is a prerequisite for the shape of the seam this blueprint's own output must fit, not for compiled code this blueprint links against. **M11-B03** (`rc-bedrock-auth`) — read in full, especially its "Seam to the future packet-layer blueprint ('M11-B02')" section, which this blueprint now concretely fulfils: `LoginPacket`'s `chain`/`client_data_token` fields are the exact `Vec<String>`/`String` values M11-B03's `validate_chain`/`verify_client_data_token` consume; `ServerToClientHandshakePacket`'s `web_token` field carries the JWT M11-B03's `ServerEcdhKeyPair`/`generate_salt` build; the encryption seam (Context §I) is where M11-B03's `BedrockAeadEncryptor`/`BedrockAeadDecryptor` attach. **No Cargo dependency edge exists here either** — restated in Context §A. This blueprint also draws on the already-written **M11-B04** (`rc-bedrock-mappings`) for the shape of the id-correspondence problem its own packet fields carry raw ids for (Context §A), again with **zero Cargo dependency edge** exercised (M11-B04 is not formally listed as this blueprint's own prerequisite by the milestone's task assignment, but its already-shipped `BedrockBlockState`/`ids.rs` shapes are read here for the one fact that matters: the mapping crate never assigns a wire-level runtime id, leaving that entirely to this blueprint — Context §M). |
| Implements | CROSS-D1 (the packet layer is pure data/codec that never mutates game state, restated — the exact NET-D8 framing extended to a second protocol); CROSS-D2 (`rc-bedrock-protocol`'s own crate identity/responsibility, restated in full — Context §A); CROSS-D5 rule 5 (dependency ceiling: `rc-core` only in principle, `rc-bedrock-mappings` only for registry-shaped constants "where needed" — this blueprint exercises **neither** edge, Context §A, mirroring M11-B01's own precedent of a permitted-but-unexercised `rc-core` edge); CROSS-D6 (pinned Bedrock protocol 2168 — referenced as a version marker, never hardcoded into wire logic); CROSS-D9's hand-written-from-public-documentation discipline, extended a second time to this protocol family (Context §B); CROSS-D10 (`resource_packs` config surface — restated at the `ResourcePacksInfoPacket`/`ResourcePacksStackPacket` wire level, Context §L); CROSS-D11 (restated only at the seam this blueprint's own packet shapes expose to M11-B03 — Context §D/§I); CROSS-D15/D16/D17 (the translation-tier framework, applied here as this blueprint's own per-packet/per-field scope boundary — every section below states plainly which tier its coverage reaches); CROSS-D19/D20 (referenced, not implemented — this blueprint's packet fields carry the *raw wire ids* a future `rc-bedrock-translator` resolves against M11-B04's tables, never a resolved semantic type); ASSET-D18(e)/(f), CROSS-D27/D29 (source-provenance discipline, restated in Context §B). |
| Crates touched | `rc-bedrock-protocol` (`crates/bedrock-protocol/`) — new, full implementation, this blueprint's entire scope. Nothing else — no `rusty-clanker-server` wiring, no `xtask lint-deps` extension (both deferred to a future composition-root/translator blueprint, exactly as M11-B01/M11-B03 each deferred their own equivalents; restated in Constraints). |
| Estimated scope | **L, explicitly beyond the nominal single-blueprint size class.** Unlike Java's own protocol scope — split across four blueprints (`M1-B01` framing/trait-model, `M1-B02` status, `M1-B04` login/configuration, `M1-B05` play) — `M11`'s own coarser one-blueprint-per-crate allocation places `rc-bedrock-protocol`'s **entire** game-packet catalog, batch envelope through the full M11 tier list, in this one file, under one fixed ID assigned by the milestone's own task decomposition (unlike `M5-B12`/`M5-B13`'s own lettered-split precedent for an oversized single task, not available here since the assignment is a single fixed blueprint ID). This blueprint manages that size the way `00-blueprint-spec.md`'s sizing rule's own spirit demands even where its literal ~800-line ceiling cannot be honored: every large or nested packet is scoped field-by-field with a named, honest boundary (Context, throughout) rather than either silently under-covering the catalog or silently ballooning past what one implementer can review as one unit. |

## Goal & Done definition

Give `rc-bedrock-protocol` the complete Bedrock game-packet wire-codec layer CROSS-D2 assigns it: the `0xFE` batch envelope, compression negotiation and zlib framing, the sub-packet length/header packing (packet id + sender/target sub-client), the seam M11-B03's AES-256-GCM encryption session attaches at, this crate's own hand-rolled little-endian/VarInt "network NBT" variant, and — field-by-field, from live-fetched official/public documentation, confidence-flagged throughout — the M11 packet catalog spanning the login/handshake sequence (`RequestNetworkSettings`→`NetworkSettings`→`Login`→`ServerToClientHandshake`/`ClientToServerHandshake`→`PlayStatus`→`ResourcePacksInfo`/`ResourcePacksStack`/`ResourcePackClientResponse`→`StartGame`), the post-spawn catalog (`CreativeContent`, `BiomeDefinitionList`, `AvailableActorIdentifiers`), chunk delivery (`LevelChunk` + the `SubChunkRequest`/`SubChunk` on-demand system), block updates (`UpdateBlock`), movement (`MovePlayer`, `PlayerAuthInput`), the M11-tier inventory surface (`InventoryContent`, `InventorySlot`, `ItemStackRequest`), chat (`Text`), the player roster (`PlayerList`), and entity lifecycle/sync (`AddPlayer`, `AddActor`, `RemoveActor`, `MoveActorAbsolute`, `SetActorData`) at the mapped tier. Every type this crate exposes is **pure data plus encode/decode** — no socket, no `tokio`, no ECS, no ownership of *when* a packet is sent — mirroring `rc-protocol`'s own isolation (NET-D8: "the network layer performs no game-state mutation itself") a second time for a second protocol family. This crate's sole intended consumer is a future `rc-bedrock-translator` blueprint, which drives `RaknetSession::send`/`recv` (M11-B01), decrypts/decompresses using this crate's batch functions plus M11-B03's AEAD types, decodes/encodes concrete packets using this crate's types, and is the only place any of these bytes are given game-state meaning.

Done when:

- [ ] `cargo build -p rc-bedrock-protocol --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-bedrock-protocol`.
- [ ] Every primitive round-trip (`VarInt32`/`VarUint32`/`VarInt64`/`VarUint64` including known-answer zigzag vectors and max-byte-length rejection), the network-NBT round-trip suite, the batch-envelope round-trip (uncompressed and zlib-compressed), the sub-packet header bit-field round-trip (including boundary values: packet id `1023`, sender/target sub-client `3`), and every packet type's own round-trip test all pass — no fixture weakened.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint declares **zero** internal RC-crate Cargo dependencies (Context §A); no `SIM` crate (`rc-scheduler`, `rc-mechanics`) gains or loses reachability to or from `rc-bedrock-protocol`.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-bedrock-protocol` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### §A — Crate boundary and dependency graph: zero internal RC-crate dependency, exercised nowhere

CROSS-D2, restated exactly: `rc-bedrock-protocol` is "Bedrock's own wire codec (packet framing, its own little-endian/varint 'network NBT' variant, the JWT-chain types), pure data/codec, no sockets, mirroring `rc-protocol`'s shape." CROSS-D5 rule 5 fixes its dependency ceiling: "`rc-bedrock-protocol`... depend[s] only on `rc-core`... (plus `rc-bedrock-protocol`→`rc-bedrock-mappings` for registry-shaped constants where needed)." `12-workspace-structure.md`'s own ratified dependency graph draws exactly these two possible edges: `bproto --> core`, `bproto --> bmap`.

**This blueprint declares neither edge.** Exactly the judgment call M11-B01 already made and justified for its own permitted-but-unexercised `rc-core` edge ("nothing in this crate's own content needs a coordinate type, an entity-ID type, or a shared error convention beyond what this crate defines for itself"), restated here for both of this crate's permitted edges:

- **No `rc-core` edge.** Every coordinate/identity type this crate's packets need (`BlockPos`, `Vec2`, `Vec3`, `Uuid128`, a raw `u64` entity id) is a small, self-contained newtype this crate defines itself (Context §F) — none of them are `rc-core`'s own Java-shaped `ChunkKey`/`DimensionId`/`RcEntityId` types, which encode Java-specific conventions (Java dimension namespacing, Java's own entity-id space) this crate's wire types deliberately do not reuse, exactly as CROSS-D1 requires ("a Bedrock player is... indistinguishable from a Java player" at the simulation level, but the two protocols' own **wire** representations of position/identity remain genuinely distinct until a future translator reconciles them).
- **No `rc-bedrock-mappings` edge.** M11-B04's own runtime API (`ids::{BedrockBlockState, BedrockItemId, BedrockBiomeId, BedrockEntityId, BedrockSoundId, BedrockParticleId}`, `tables::MappingTables`) is a **semantic correspondence** layer — "this Java block state corresponds to this Bedrock block state" — consumed by a component that needs to *resolve* an id, which is `rc-bedrock-translator`'s job, one layer above this crate (CROSS-D5 rule 6 draws `rc-bedrock-translator → rc-bedrock-mappings`, never through this crate). Every id this crate's own packet fields carry is instead a **raw wire-primitive newtype this crate defines locally** — `BlockRuntimeId(pub u32)`, `ItemNetworkId(pub i32)`, `EntityTypeId(pub String)` (Bedrock entity types are namespaced strings on the wire, not small integers, per §S) — never a re-export of M11-B04's own `BedrockBlockState`/`BedrockItemId`. This keeps the crate boundary CROSS-D2 draws exact: this crate's job is "how do these bytes turn into a `StartGamePacket` value and back," never "what does `block_runtime_id: 4821` *mean*."

**No `rc-bedrock-protocol-macros` crate exists.** `12`'s ratified CROSS-D2 five-crate manifest has no macro-crate counterpart to `rc-protocol-macros`/`#[derive(RcPacket)]` (NET-D3) — contrast M1-B01's own `#[derive(RcPacket)]` container-attribute model, which this blueprint deliberately does **not** attempt to reproduce (there is nothing to depend on: `rc-protocol-macros` itself is off-limits by CROSS-D5, and inventing a new, unratified sixth crate is out of this blueprint's authority). Every packet type below is instead **hand-written**, with a manually-implemented `encode`/`decode` pair, exactly mirroring M11-B01's own already-established precedent for its RakNet-internal packets (`Frame::encode`/`decode`, `DatagramHeader::encode`/`decode` — no macro there either, for the identical structural reason). Context §E fixes the one shared trait (`BedrockPacket`) this blueprint introduces to keep that hand-written surface uniform across ~30 packet types without a macro.

**No `rc-nbt` edge either** (also absent from CROSS-D5's enumerated edges). This crate's own network-NBT variant (§K) is hand-rolled from scratch rather than routed through `rc-nbt`'s `simdnbt` wrapper, which is built around Java's own on-disk/network NBT conventions (NET-D5) that this blueprint could not confirm share an implementation-ready API surface for Bedrock's own network-little-endian-plus-zigzag-VarInt variant (§B) — hand-rolling a small, fully-specified, independently-tested codec is the safer, more auditable choice here, the same reasoning CROSS-D9 already applies to `rc-bedrock-raknet` choosing not to depend on a third-party RakNet crate.

**Net effect:** `rc-bedrock-protocol`'s `Cargo.toml` (Deliverables) names **zero** path dependencies on any other `rc-*` crate — the smallest possible dependency footprint CROSS-D5 permits, exercising none of its two allowed internal edges, exactly mirroring M11-B01's own resolution of the identical question.

### §B — Source confidence ledger

Every fact restated below was live-fetched **2026-08-24** (this blueprint's derivation session), per ASSET-D18(b)/(h)/CROSS-D27. Sources are exclusively the official `mojang.github.io/bedrock-protocol-docs` site (Mojang-published, EULA-gated exactly as CROSS-D27 already establishes for this source) and `wiki.bedrock.dev` (CROSS-D9's own named public-documentation source, extended a second time here from RakNet to the game-packet layer) — no GeyserMC/CloudburstMC/gophertunnel source code was opened at any point (CROSS-D29's firewall extension, restated); two summarizing fetches surfaced `gophertunnel`'s own `pkg.go.dev`-rendered doc comments purely as cross-corroborating **documented behavior**, per ASSET-D18(e)'s architecture-reading allowance, mirroring M11-B03's own identical, already-approved pattern.

**A structural caveat that applies to every HIGH row below, stated once rather than repeated per row:** `mojang.github.io/bedrock-protocol-docs/latest/` tracks the **current** protocol snapshot (confirmed, during this session's own fetches, to be **2192**, dated August 2026) — materially ahead of CROSS-D6's pinned **2168**. Every field *shape* restated here is treated the same way M11-B03 already treated its own version-skew gap for `LoginPacket` ("the packet *shape* is stable across that gap"): packet **field lists and types** for the packets this blueprint covers are not expected to have added/removed fields between 2168 and 2192 (none of the "Protocol Updates"/changelog entries this session's fetches surfaced named any of this blueprint's own packets), but this is an **assumption, not a confirmation** — CROSS-D7(b)'s own bump-review step ("reviewed against fresh packet captures") is exactly the mechanism that re-verifies every field list below against the actual pinned 2168 client at implementation time.

| Confidence | Fact | Source(s) |
|---|---|---|
| **HIGH** | `varint32`/`varint64` = ZigZag-encoded then base-128-with-continuation-bit; `varuint32`/`varuint64` = base-128-with-continuation-bit, **no** ZigZag; strings = raw UTF-8 bytes prefixed by their length as a `varuint32` | `mojang.github.io/bedrock-protocol-docs/latest/primitives/` (live fetch) |
| **HIGH** | `NetworkSettingsPacket` (id **143**) = `CompressionThreshold: uint16`, `CompressionAlgorithm: uint16` (`0`=ZLib, `1`=Snappy, `2`=None), `ClientThrottleEnabled: bool`, `ClientThrottleThreshold: uint8`, `ClientThrottleScalar: float` | `mojang.github.io/bedrock-protocol-docs/latest/packets/network-settings-packet/` (live fetch, id confirmed by a second, targeted fetch of the same page) |
| **HIGH** | `RequestNetworkSettingsPacket` (id **193**) = one field, `ClientNetworkVersion: int32` | Same site, `.../request-network-settings-packet/` (live fetch) |
| **HIGH** | Login sequence order: `RequestNetworkSettings`→`NetworkSettings`→`Login`→(`ServerToClientHandshake`→`ClientToServerHandshake`, only when `auth_mode = online`)→`ResourcePacksInfo`→(`ClientCacheStatus` optional)→`ResourcePackClientResponse`→`ResourcePacksStack`→`PlayStatus(LoginSuccess)`→`StartGame`→`PlayStatus(PlayerSpawn)` | `wiki.bedrock.dev/servers/bedrock` (live fetch) — matches `15-crossplay.md`'s own already-fixed Bedrock Connection Lifecycle diagram exactly |
| **HIGH** | Game-packet batch: a RakNet application payload prefixed with a single `0xFE` byte, carrying (once compression is negotiated) one compression-identifying byte, then the compressed/uncompressed sub-packet stream; `0x00`=Zlib, `0x01`=Snappy, `0xFF`/`0xFFFF`=no compression | `wiki.bedrock.dev/servers/bedrock` (live fetch) |
| **MEDIUM** | Whether that per-batch compression-id byte is present on **every** post-negotiation batch (redundantly re-stating the algorithm `NetworkSettingsPacket` already fixed once) or only during some transitional window — this session's fetch stated the byte's meaning but not conclusively its exact presence rule relative to the separate `NetworkSettingsPacket` negotiation | `wiki.bedrock.dev/servers/bedrock` (live fetch) — resolved by this blueprint's own stated policy, §G |
| **HIGH** | Sub-packet framing inside one decompressed batch: `GamepacketLength: varuint32` (whole sub-packet including its own header), then a `varuint32` **header** packing packet id (low 10 bits, `0..1023`) + sub-client sender id (next 2 bits, `0..3`) + sub-client target id (next 2 bits, `0..3`) | `wiki.bedrock.dev/servers/bedrock` (live fetch) |
| **HIGH** | `PlayStatusPacket` (id **2**) = `Status: int32` enum: `0`=LoginSuccess, `1`=LoginFailedClientOld, `2`=LoginFailedServerOld, `3`=PlayerSpawn, `4`=LoginFailedInvalidTenant, `5`/`6`=Edu/Vanilla edition mismatch, `7`=LoginFailedServerFullSubClient, `8`/`9`=Editor/Vanilla mismatch | `.../packets/play-status-packet/` (live fetch) |
| **HIGH** | `ResourcePacksInfoPacket` (id **6**) = `ResourcePackRequired: bool`, `HasAddonPacks: bool`, `HasScripts: bool`, `ForceDisableVibrantVisuals: bool`, `WorldTemplateIdAndVersion: PackIdVersion`, `ResourcePacks: array<PackInfoData>` (`PackInfoData` ⊇ pack id+version, size `uint64`, content key, subpack name, content identity, has-scripts, is-addon, is-ray-tracing-capable, CDN url — all string/bool fields) | `.../packets/resource-packs-info-packet/` (live fetch) |
| **HIGH** | `ResourcePackStackPacket` (id **7**) = `TexturePackRequired: bool`, `TexturePackList: array<PackInstanceId>` (pack id + version + subpack name), `BaseGameVersion: string`, `Experiments`, `IncludeEditorPacks: bool` | `.../packets/resource-pack-stack-packet/` (live fetch) |
| **HIGH** | `ResourcePackClientResponsePacket` (id **8**) = `Response: int8` enum `{0=Cancel, 1=Downloading, 2=DownloadingFinished, 3=ResourcePackStackFinished}`, plus a `DownloadingPacks: array<string>` present only when `Response = Downloading` | `.../packets/resource-pack-client-response-packet/` (live fetch) |
| **HIGH** | `StartGamePacket` (id **11**/`0x0B`) — 25 top-level fields in wire order (§M) | `.../packets/start-game-packet/` (live fetch) |
| **HIGH** | `LevelSettings` — 50 fields in wire order (§M) | `.../types/level-settings/` (live fetch) |
| **HIGH** | `GameRule` = `RuleName: string`, `RuleCanBeModified: bool`, `RuleValue: oneOf<null, bool, int32, float>` | `.../types/game-rule/` (live fetch) |
| **HIGH** | `SpawnSettings` = `SpawnBiomeType: int16` (`0`=Default, `1`=UserDefined), `UserDefinedBiomeName: string`, `Dimension: varint32` | `.../types/spawn-settings/` (live fetch) |
| **HIGH** | `Experiments` = `Toggles: array<{Name: string, Enabled: bool}>`, `ExperimentsEverToggled: bool` | `.../types/experiments/` (live fetch) |
| **MEDIUM** | `ServerBlockProperty` = `{name: string, <block-state definition>}` — the exact shape of the per-block definition beyond its name was not independently confirmed field-by-field this session | `.../packets/start-game-packet/` (live fetch, StartGame's own field-13 description) — this blueprint's own resolution treats the definition half as an opaque network-NBT compound (§M) |
| **HIGH** | `CreativeContentPacket` (id **145**) = `Groups: array<{CreativeCategory: uint8 enum, Name: string, GroupIcon: NetworkItemInstanceDescriptor}>`, `Entries: array<{CreativeNetId: varuint32, Item: NetworkItemInstanceDescriptor, GroupIndex: varuint32}>` | `.../packets/creative-content-packet/` (live fetch) |
| **MEDIUM** | `BiomeDefinitionListPacket` (id **122**) — a structured payload (a name→data map plus a separate biome-string-list), **not** confirmed as a single monolithic NBT compound this session, contrary to some older community write-ups | `.../packets/biome-definition-list-packet/` (live fetch) — resolved as opaque (§N) |
| **MEDIUM** | `AvailableActorIdentifiersPacket` (id **119**, the current name for the packet historically called `AvailableEntityIdentifiers` in older community docs) — payload wire type not confirmed beyond "an identifier list" this session | `.../packets/available-actor-identifiers-packet/` (live fetch) — resolved as opaque (§N) |
| **HIGH** | `LevelChunkPacket` (id **58**) = `ChunkPosition: {x: varint32, z: varint32}`, `Dimension: varint32`, `SubChunkCount: varuint32` (with the sentinel values `0xffffffff` = "client should request sub-chunks" and `0xfffffffe` = "known air, skip"), `ClientRequestSubChunkLimit: varint32` (optional), `CacheEnabled: bool`, `CacheBlobIds: array<uint64>` (when cache enabled), `Payload: <length-prefixed bytes>` | `.../packets/level-chunk-packet/` (live fetch); sentinel values corroborated by `deepwiki.com/Mojang/bedrock-protocol-docs/8.2-chunk-and-subchunk-system`'s own synthesis of the same upstream docs (independent re-derivation, same primary source) |
| **HIGH** | `SubChunkRequestPacket` (id **175**) = `Dimension: varint32`, `CenterPos: {x,y,z: int32}`, `Offsets: array<{x,y,z: int8}>` (relative to `CenterPos`) | `.../packets/sub-chunk-request-packet/` (live fetch) |
| **HIGH** | `SubChunkPacket` (id **174**) = `CacheEnabled: bool`, `Dimension: varint32`, `CenterPos: {x,y,z: int32}`, `Entries: array<{Offset: {x,y,z: int8}, Result: uint8 enum {1=Success, 2=LevelChunkDoesntExist, 3=WrongDimension, 4=PlayerDoesntExist, 5=IndexOutOfBounds, 6=SuccessAllAir}, Payload: optional<bytes>, HeightMapType+Data: optional, BlobId: optional<uint64>}>` | `.../packets/sub-chunk-packet/` (live fetch); enum-value integers 1–6 are this session's own consistent renumbering of the fetched list (the raw fetch text omitted explicit numbers for two entries — **flagged**: confirm exact `Result` integers against a fresh capture at implementation time, per CROSS-D7(b)) |
| **HIGH** | `UpdateBlockPacket` (id **21**) = `BlockPosition: BlockPos` (three `varint32`), `BlockRuntimeId: varuint32`, `Flags: varuint32`, `Layer: varuint32` | `.../packets/update-block-packet/` (live fetch, two targeted fetches agree) |
| **HIGH** | `PlayerAuthInputPacket` (id **144**) — 19 fields (§Q) | `.../packets/player-auth-input-packet/` (live fetch) |
| **HIGH** | `MovePlayerPacket` (id **19**) — 9 fields (§Q) | `.../packets/move-player-packet/` (live fetch) |
| **LOW/FLAGGED** | Whether `MovePlayerPacket` (client→server) remains meaningfully used once `PlayerAuthInputPacket` is active, and the exact server-authoritative-movement config surface (`AuthoritativeMovementMode` and any rewind window) | Not found in this session's fetches — this blueprint's own resolved stance (§Q) is a documented simplification, not a confirmed fact |
| **HIGH** | `TextPacket` (id **9**) = `Localize: bool`, a `Body` shaped by `MessageType` (`raw`/`tip`/`systemMessage`/`textObjectWhisper`/`textObject`/`textObjectAnnouncement` carry `Message: string` only; `chat`/`whisper`/`announcement` additionally carry `PlayerName: string`; `translate`/`popup`/`jukeboxPopup` carry `Message` + `Parameters: array<string>`), then `XUID: string`, `PlatformId: string`, `FilteredMessage: string` | `.../packets/text-packet/` (live fetch) |
| **HIGH** | `InventoryContentPacket` (id **49**) = `ContainerId: varuint32`, `Slots: array<NetworkItemStackDescriptor>`, `FullContainerName: {Name: uint8 enum, DynamicId: uint32}`, `StorageItem: NetworkItemStackDescriptor` | `.../packets/inventory-content-packet/` (live fetch) |
| **HIGH** | `InventorySlotPacket` (id **50**) = `ContainerId: uint8`, `Slot: varuint32`, `FullContainerName: optional`, `StorageItem: optional<NetworkItemStackDescriptor>`, `Item: NetworkItemStackDescriptor` | `.../packets/inventory-slot-packet/` (live fetch) |
| **MEDIUM** | `ItemStackRequestPacket` (id **147**) = `Requests: array<{ClientRequestId: varint32, Actions: array<ItemStackRequestAction>}>`; the 18-member action-type vocabulary (`Take`, `Place`, `Swap`, `Drop`, `Destroy`, `Consume`, `Create`, `ScreenLabTableCombine`, `ScreenBeaconPayment`, `ScreenHUDMineBlock`, `CraftRecipe`, `CraftRecipeAuto`, `CraftCreative`, `CraftRecipeOptional`, `CraftRepairAndDisenchant`, `CraftLoom`, `CraftNonImplemented`, `CraftResults`) is confirmed by name; **per-action field layout was not independently confirmed field-by-field this session** beyond "source/destination container slots plus operation-specific parameters" | `.../packets/item-stack-request-packet/` (live fetch) — resolved as a decomposed-common-actions-plus-opaque-catch-all tier (§R) |
| **MEDIUM** | `PlayerListPacket` (id **63**) — `PlayerListAddEntry`/`PlayerListRemoveEntry` variants; add-entry ⊇ UUID, unique entity id, name, XUID, platform online id, build platform enum, serialized skin data, persona pieces, account flags | Earlier session fetch of `.../packets/player-list-packet/` (live fetch, summarized rather than a literal field table — resolved as identity-fields-decomposed-plus-opaque-skin-blob, §S) |
| **HIGH** | `AddPlayerPacket` (id **12**) — 16 fields ⊇ UUID, name, runtime/unique entity id, platform chat id, position/velocity/rotation, held item, game type, entity data, permissions/abilities, device id, build platform, actor links | `.../packets/add-player-packet/` (live fetch, summarized field count and categories) |
| **HIGH** | `AddActorPacket` (id **13**) = unique id (`varint64`), runtime id (`varuint64`), actor type (`string`), position/velocity (`Vec3`), rotation (`Vec2`), Y-head/Y-body rotation (`float` each), attributes list, actor (metadata) data list, synced properties, actor links | `.../packets/add-actor-packet/` (live fetch) |
| **HIGH** | `RemoveActorPacket` (id **14**) = `TargetActorId: varint64` | `.../packets/remove-actor-packet/` (live fetch) |
| **HIGH** | `MoveActorAbsolutePacket` (id **18**) = `RuntimeId: varuint64`, `Header/Flags: uint8`, `Position: Vec3`, `RotationX/Y/YHead: uint8` (each a byte-packed angle, `0..255` mapping to `0..360°`) | `.../packets/move-actor-absolute-packet/` (live fetch) |
| **HIGH** | `SetActorDataPacket` (id **39**) = `TargetRuntimeId: varuint64`, `ActorData: array<{Id: varuint32, Value: <polymorphic: byte/short/int/float/string/compound/BlockPos/i64/Vec3>}>`, `SyncedProperties: {IntEntries, FloatEntries}`, `Tick: varuint64` | `.../packets/set-actor-data-packet/` (live fetch) |
| **HIGH** | `DisconnectPacket` (id **5**) = `Reason: varint32` enum (range `0..149`), optional `{Message: string, FilteredMessage: string}` | `.../packets/disconnect-packet/` (live fetch) |
| **HIGH** | `LoginPacket` (id **1**), `ServerToClientHandshakePacket` (id **3**), `ClientToServerHandshakePacket` (id **4**, zero fields) — restated verbatim from M11-B03's own already-verified Source confidence ledger, not re-fetched this session | M11-B03 Context, "Source confidence ledger" |
| **LOW/FLAGGED** | Whether the AES-256-GCM encryption seam (M11-B03) wraps the **whole post-compression batch payload** (this blueprint's own resolved policy, §I) or something narrower | Not found this session — inherits M11-B03's own already-flagged nonce-construction/key-derivation uncertainty; the *scope* of what gets encrypted is this blueprint's own necessary extension of that same gap, flagged identically |
| **LOW/FLAGGED (this blueprint's own placeholder)** | The exact hash function `StartGamePacket.BlockNetworkIdsAreHashes = true` implies for a hashed block-runtime-id — not found in any source this session consulted | This blueprint's own resolved, explicitly-placeholder choice (§M) |

### §C — The RakNet seam (M11-B01 §L, restated)

M11-B01 §L, restated exactly: once `RaknetListener::accept()` yields a `Connected` `RaknetSession`, it is "an ordered multi-channel reliable byte-message transport — `send(channel, reliability, payload)` / `async recv() -> Option<Bytes>`... 32 order channels... all eight reliability types... in practice, per Bedrock's own observed convention, predominantly `Reliability::ReliableOrdered` on `OrderChannel(0)`." This blueprint's own `batch::encode_batch`/`batch::decode_batch` (§G) produce and consume exactly the `Bytes` values that seam's `payload` parameter and `recv()` return type carry — this crate never calls `RaknetSession` itself (no Cargo edge, §A); a future `rc-bedrock-translator` is the one piece of code that holds both a `RaknetSession` and this crate's types side by side.

### §D — The auth/handshake seam (M11-B03, fulfilled concretely)

M11-B03's own "Seam to the future packet-layer blueprint ('M11-B02')" section described six steps a "future blueprint" would need; this blueprint is that future blueprint, and §L below defines the exact packet types those steps operate on:

1. `LoginPacket::chain: Vec<String>` and `LoginPacket::client_data_token: String` (§L) are extracted from the packet's own wire `ConnectionRequest` JSON envelope (§L) — the two values M11-B03's `validate_chain`/`verify_client_data_token` take directly.
2. `ServerToClientHandshakePacket::web_token: String` (§L) is the JWT M11-B03's step 6 builds (`ServerEcdhKeyPair::generate()`, `generate_salt()`, then a self-signed single-claim JWT this blueprint's own JWT-encoding helpers in `login.rs` — reusing the same base64url/`serde_json` shapes M11-B03 itself already established, **hand-rolled here too, no dependency on `rc-bedrock-auth`**, §A — produce from those raw bytes).
3. `ClientToServerHandshakePacket` (§L) has zero fields (M11-B03's own confirmed fact, restated) — its mere *arrival* is the signal a future translator uses to call M11-B03's `server_keys.diffie_hellman(...)`/install the AEAD pair (§I).

### §E — The hand-written `BedrockPacket` trait (no macro)

Every packet type below implements one small, uniform trait this blueprint defines (`packet.rs`, Deliverables):

```rust
pub trait BedrockPacket: Sized {
    /// The 10-bit packet id (§J) — a compile-time constant per concrete type.
    const ID: u16;
    fn encode(&self, out: &mut bytes::BytesMut);
    fn decode(buf: &mut bytes::Bytes) -> Result<Self, crate::error::PacketDecodeError>;
}
```

Field-level encoding within one `encode`/`decode` body is expressed against a small `WireWrite`/`WireRead` pair (`primitives.rs`, Deliverables) implemented for every primitive type in §F's table — this is this crate's own, hand-rolled analog of `rc-protocol`'s `WireWrite`/`WireRead` traits (NET-D3), never the same trait (no shared crate, §A), reused across every packet's hand-written body purely to avoid repeating "read a `varuint32`-length-prefixed UTF-8 string" inline thirty times. There is no macro deriving `encode`/`decode` from field attributes (§A) — every packet's body is written out, field by field, by the implementer, following §F's primitive table and each packet's own field table below, exactly the discipline M11-B01's `Frame::encode`/`decode` already established for this crate family.

### §F — Primitives

All types `pub` in `primitives.rs`. Endianness note, stated once and never repeated per type below: **once past RakNet's own framing (M11-B01 §C, which is itself entirely big-endian for its non-frame-header fields), the Bedrock *game*-packet layer is little-endian throughout** for every fixed-width field — a real, easily-confused asymmetry with RakNet's own big-endian convention one layer below, restated explicitly here exactly as M11-B01 §C flagged its own equivalent asymmetry. This is **MEDIUM confidence** (§B) — the general Bedrock-wire LE convention is well-established community knowledge cross-consistent with every field table this blueprint fetched, but no single fetch this session stated it as one universal rule in so many words.

| Type | Shape | Wire encoding |
|---|---|---|
| `VarInt32(pub i32)` | signed 32-bit | ZigZag then base-128/continuation-bit, 1–5 bytes (§B, HIGH) |
| `VarUint32(pub u32)` | unsigned 32-bit | base-128/continuation-bit, **no** ZigZag, 1–5 bytes |
| `VarInt64(pub i64)` | signed 64-bit | ZigZag then base-128/continuation-bit, 1–10 bytes |
| `VarUint64(pub u64)` | unsigned 64-bit | base-128/continuation-bit, **no** ZigZag, 1–10 bytes |
| `BedrockString(pub String)` | UTF-8 text | `VarUint32` byte-length prefix, then raw UTF-8 bytes (§B, HIGH) |
| `Vec2 { pub x: f32, pub y: f32 }` | 2D float | two `f32` LE |
| `Vec3 { pub x: f32, pub y: f32, pub z: f32 }` | 3D float | three `f32` LE |
| `BlockPos { pub x: i32, pub y: i32, pub z: i32 }` | block coordinate | three `VarInt32` (§B, HIGH, via `UpdateBlockPacket`'s own confirmed "`varint32` compression") |
| `Uuid128 { pub hi: u64, pub lo: u64 }` | 128-bit identity (`mce::UUID`) | two `u64` LE — **exact hi/lo-vs-Java-UUID-bit-ordering correspondence is MEDIUM confidence** (§B); this blueprint treats `Uuid128` as its own opaque 128-bit value, never assumed byte-identical to a `uuid::Uuid`'s own internal layout without an explicit, tested conversion (Deliverables, `primitives.rs::Uuid128::from_uuid`/`to_uuid`, flagged `// MEDIUM confidence bit-order — verify against a real client capture`) |
| `bool` | flag | one byte, `0`/`1` |
| `u8`/`i8` | byte | one byte, sign-extension only for `i8` |
| `u16`/`i16` | short | two bytes LE |
| `u32`/`i32` (fixed-width, non-Var) | fixed int | four bytes LE — used only where a field's own table entry states a fixed (non-`varint`) width, e.g. `NetworkSettingsPacket`'s `CompressionThreshold`/`CompressionAlgorithm` (§B, both `uint16`) |
| `u64`/`i64` (fixed-width) | fixed long | eight bytes LE |
| `f32` | float | four bytes LE, IEEE-754 |

`WireWrite`/`WireRead` are implemented for every row above plus `Vec<T: WireWrite/WireRead>` behind an explicit **caller-chosen length-prefix type per field** (this blueprint's own hand-written bodies choose `VarUint32`-prefixed for every array field this blueprint's fetched sources confirmed uses that convention — the dominant one — flagging the rare fixed-count-prefix case, e.g. `SubChunkRequestPacket`'s offset list, individually in that packet's own section below rather than assuming one universal array convention).

### §G — The `0xFE` batch envelope

Restated from §B's HIGH-confidence row: the byte stream this crate hands to/from the RakNet seam (§C) is, in order:

```
0xFE                                    // 1 byte, the game-packet-batch marker (HIGH)
[compression_id: u8]                    // present per §G's own policy below (MEDIUM)
<payload>                               // compressed or raw sub-packet stream, §H/§J
```

**This blueprint's own resolved policy for the flagged compression-id byte (§B):** present on **every** batch sent **after** `NetworkSettingsPacket` has been exchanged (i.e., every batch from `LoginPacket` onward), absent on the exactly two batches exchanged **before** negotiation (`RequestNetworkSettingsPacket`, `NetworkSettingsPacket` itself — necessarily uncompressed, since no algorithm has been agreed yet). The byte's value, once present, is always the algorithm `NetworkSettingsPacket` already fixed for the whole connection (§H) — it is redundant with that earlier negotiation by design, not an independent per-batch choice, matching the "always the best is Zlib in production" framing §B's source states. This resolution is flagged `// MEDIUM confidence policy — confirm against a fresh 26.44 capture, CROSS-D7(b)` at its one call site (`batch.rs`, Deliverables) rather than asserted as settled fact.

```rust
/// The batch envelope this crate hands to/from the RakNet seam (§C). `compression` is `None`
/// only for the two pre-`NetworkSettings` batches (§G's own resolved policy); `Some(_)` for
/// every batch after.
pub fn encode_batch(sub_packets: &[Bytes], compression: Option<CompressionAlgorithm>) -> Bytes;

/// Inverse of `encode_batch`. `Err` on a missing/malformed `0xFE` marker, an unrecognized
/// compression-id byte, or a compression-payload decode failure (§H) — never panics on
/// adversarial input (Acceptance tests, `fuzz_stub.rs`).
pub fn decode_batch(raw: &Bytes, compression: Option<CompressionAlgorithm>) -> Result<Vec<Bytes>, crate::error::BatchDecodeError>;
```

### §H — Compression

`NetworkSettingsPacket`'s own confirmed enum (§B, HIGH): `CompressionAlgorithm { Zlib = 0, Snappy = 1, None = 2 }`. **This blueprint implements `Zlib` fully**, reusing NET-D5's already-workspace-pinned `flate2` crate (`zlib-ng` feature) — the identical dependency `rc-protocol` itself already uses for Java's own compression (NET-D5), consumed here as a second, independent user of the same already-reviewed pin, **adding zero new workspace dependency** (an explicit, positive property of this blueprint worth stating plainly, in contrast to M11-B01's `rand` and M11-B03's `aes-gcm`/`sha2` each adding one). **`Snappy` is an explicit, named non-goal for this blueprint's own baseline scope**: §B's own source states "always use Zlib in production, since it is the best," Rusty Clanker is the party choosing the algorithm (`NetworkSettingsPacket` is server→client), and taking on a new `snap`-crate dependency purely to support an algorithm this blueprint's own server-side code will never select would be dead weight — mirroring M11-B01's own precedent of correctly encoding/decoding a reliability variant (`*WithAckReceipt`) it does not itself exercise. `CompressionAlgorithm::Snappy`'s `compress`/`decompress` methods (Deliverables) return `Err(CompressionError::SnappyNotImplemented)` unconditionally — the variant is representable (a real client's own choice must round-trip through this enum without a decode error) but not functional, an explicit, bounded exception named here rather than silently absent.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CompressionAlgorithm { Zlib = 0, Snappy = 1, None = 2 }

impl CompressionAlgorithm {
    /// `Err(CompressionError::SnappyNotImplemented)` for `Snappy` (§H's own named non-goal).
    pub fn compress(self, plaintext: &[u8]) -> Result<Bytes, crate::error::CompressionError>;
    pub fn decompress(self, data: &[u8]) -> Result<Bytes, crate::error::CompressionError>;
}

pub struct NetworkSettingsPacket {
    pub compression_threshold: u16,
    pub compression_algorithm: CompressionAlgorithm,
    pub client_throttle_enabled: bool,
    pub client_throttle_threshold: u8,
    pub client_throttle_scalar: f32,
}
impl BedrockPacket for NetworkSettingsPacket { const ID: u16 = 143; /* ... */ }

pub struct RequestNetworkSettingsPacket { pub client_network_version: i32 }
impl BedrockPacket for RequestNetworkSettingsPacket { const ID: u16 = 193; /* ... */ }
```

### §I — Encryption seam (M11-B03's AES-256-GCM)

**This blueprint's own resolved scope (§B, LOW/FLAGGED, inherited from M11-B03's own already-flagged encryption uncertainty):** once `ClientToServerHandshakePacket` has been received (§D step 3), the AEAD wraps the **entire post-compression batch payload** — i.e., `BedrockAeadEncryptor::seal` is called once per outbound batch on exactly the bytes `CompressionAlgorithm::compress` produced, and its ciphertext-plus-tag becomes the `<payload>` §G's envelope carries after the (still-present, per §G's own policy) compression-id byte; `BedrockAeadDecryptor::open` is called once per inbound batch on those same bytes before `CompressionAlgorithm::decompress` runs. This crate defines **no type of its own for this seam** — it is deliberately a call a future `rc-bedrock-translator` makes directly against M11-B03's already-published `BedrockAeadEncryptor`/`BedrockAeadDecryptor` (no Cargo edge from this crate, §A; this crate's own `encode_batch`/`decode_batch` operate purely on already-plaintext bytes, agnostic to whether encryption ran before/after they were called). This scope decision — "whole batch, not per-sub-packet" — is this blueprint's own necessary resolution of a gap `15-crossplay.md` left implicit, flagged for reconciliation into that document's next revision exactly as M11-B03 already flagged its own comparable additions (`mojang_root_key_override`, the offline-UUID-derivation extension).

### §J — Sub-packet framing inside one decompressed (and, once active, decrypted) batch

§B's HIGH-confidence row, restated as the exact bit layout:

```
GamepacketLength: VarUint32              // byte length of everything below, this one sub-packet
PacketHeader:     VarUint32              // packed: (target_sub_client << 12) | (sender_sub_client << 10) | packet_id
    packet_id:          bits 0..10   (0..1023)
    sender_sub_client:  bits 10..12  (0..3)
    target_sub_client:  bits 12..14  (0..3)
Body: [u8; GamepacketLength - <PacketHeader's own encoded byte length>]
```

`sender_sub_client`/`target_sub_client` are RakNet's own split-screen sub-client addressing (multiple local players sharing one connection, e.g. couch co-op) — this blueprint's own M11 baseline always writes `0` for both (Rusty Clanker's own server side never itself hosts a split-screen client) and accepts any value `0..=3` on decode without interpreting it further (a future translator's own concern if split-screen support is ever added, out of this blueprint's scope, named explicitly rather than silently dropped).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubClientId(pub u8); // 0..=3, enforced by PacketHeader::decode

#[derive(Debug, Clone, Copy)]
pub struct PacketHeader { pub packet_id: u16, pub sender: SubClientId, pub target: SubClientId }
impl PacketHeader {
    pub fn encode(&self, out: &mut bytes::BytesMut);
    /// `Err` if `packet_id > 1023` (a 10-bit-field overflow — a programmer error on encode, a
    /// malformed-input error on decode).
    pub fn decode(buf: &mut Bytes) -> Result<Self, crate::error::PacketDecodeError>;
}

/// Packs one sub-packet's `GamepacketLength`-prefixed, header-prefixed wire form.
pub fn pack_sub_packet(header: PacketHeader, body: &[u8]) -> Bytes;
/// Unpacks one sub-packet from the front of `buf`, advancing it past this sub-packet's own
/// bytes. Never panics on adversarial input (Acceptance tests).
pub fn unpack_sub_packet(buf: &mut Bytes) -> Result<(PacketHeader, Bytes), crate::error::PacketDecodeError>;
```

### §K — Network NBT (hand-rolled, no `simdnbt`/`rc-nbt` dependency)

Restated from §B's MEDIUM-HIGH cross-referenced community-documentation row (wiki.vg's own NBT page, cross-checked against the generic community consensus this session's searches surfaced): Bedrock's network NBT variant differs from both Java's NBT and Bedrock's own on-disk little-endian NBT in exactly these ways —

- `TAG_Short`, `TAG_Float`, `TAG_Double` — their **raw** values are little-endian (no VarInt for these three scalar leaf types).
- `TAG_Int` values, and every **length prefix** (`TAG_List`'s element count, `TAG_Byte_Array`/`TAG_Int_Array`/`TAG_Long_Array`'s element counts) — encoded as `VarInt32` (this crate's own §F type — ZigZag, base-128).
- Strings (`TAG_String`'s payload, and every named tag's own name field) — `VarUint32`-length-prefixed UTF-8, the **same** convention §F's `BedrockString` already uses at the outer packet layer (one shared string convention across both layers, worth stating explicitly since it means `nbt.rs` reuses `primitives::BedrockString`'s own encode/decode rather than a second, parallel implementation).
- `TAG_Byte`, `TAG_Long` — one raw byte / eight raw LE bytes respectively, unaffected by the VarInt convention (only the three types named above and every length prefix use VarInt; a bare scalar leaf's own single value uses its natural fixed width, matching the same asymmetry §C already established for RakNet's own frame-header-vs-everything-else split, restated here for a third field-family distinction within this one format).

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkNbtValue {
    Byte(i8), Short(i16), Int(i32), Long(i64), Float(f32), Double(f64),
    ByteArray(Vec<i8>), String(String),
    List(Vec<NetworkNbtValue>),                    // homogeneous; empty list has no element-type tag ambiguity issue for this crate's own use, §M/§N
    Compound(Vec<(String, NetworkNbtValue)>),       // insertion-order preserved — never re-sorted
    IntArray(Vec<i32>), LongArray(Vec<i64>),
}

/// Encodes one root, named compound (name = "" for every packet use in this blueprint — Bedrock's
/// network NBT root tag is always an unnamed/empty-named compound, per every fetched source's own
/// example payloads).
pub fn encode_network_nbt_root(value: &NetworkNbtValue, out: &mut bytes::BytesMut);
pub fn decode_network_nbt_root(buf: &mut Bytes) -> Result<NetworkNbtValue, crate::error::NbtDecodeError>;
```

**The remainder of this Context section (§L–§T) defines the login/handshake/play packet catalog itself**, continuing the same `### §<letter>` structure §A–§K already established — restated here as plain prose rather than a second `##` heading, since `00-blueprint-spec.md`'s own mandatory structure fixes `## Context (self-contained)` as one section, not two.

### §L — Login, handshake, resource packs

```rust
pub struct LoginPacket {
    pub client_protocol_version: i32,   // fixed i32 BE per §B's own official-doc field description — the one
                                          // field this blueprint's own §F LE convention does NOT apply to,
                                          // flagged explicitly: `RequestNetworkSettingsPacket.client_network_version`
                                          // and this field are both stated as plain `int32` with no LE/VarInt
                                          // qualifier by the official source, and this blueprint treats an
                                          // unqualified `int32` in that source's own vocabulary as BE, mirroring
                                          // how RakNet's own multi-byte non-frame fields are BE (M11-B01 §C) —
                                          // MEDIUM confidence, confirm against a fresh capture (§B)
    /// The raw JSON `ConnectionRequest` string's own `chain` array, extracted but not re-parsed by
    /// this crate — handed verbatim to `rc_bedrock_auth::validate_chain` (§D step 1, no Cargo edge).
    pub chain: Vec<String>,
    /// The same envelope's separate client-data (skin/device) JWT string.
    pub client_data_token: String,
}
impl BedrockPacket for LoginPacket { const ID: u16 = 1; /* ... */ }

pub struct ServerToClientHandshakePacket { pub web_token: String }
impl BedrockPacket for ServerToClientHandshakePacket { const ID: u16 = 3; /* ... */ }

/// Zero fields (§B, HIGH) — `encode` writes nothing beyond the sub-packet header; `decode`
/// consumes nothing and always succeeds once the header itself parsed.
pub struct ClientToServerHandshakePacket;
impl BedrockPacket for ClientToServerHandshakePacket { const ID: u16 = 4; /* ... */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PlayStatus {
    LoginSuccess = 0, LoginFailedClientOld = 1, LoginFailedServerOld = 2, PlayerSpawn = 3,
    LoginFailedInvalidTenant = 4, LoginFailedEditionMismatchEduToVanilla = 5,
    LoginFailedEditionMismatchVanillaToEdu = 6, LoginFailedServerFullSubClient = 7,
    LoginFailedEditorMismatchEditorToVanilla = 8, LoginFailedEditorMismatchVanillaToEditor = 9,
}
pub struct PlayStatusPacket { pub status: PlayStatus }
impl BedrockPacket for PlayStatusPacket { const ID: u16 = 2; /* ... */ }

pub struct PackIdVersion { pub id: Uuid128, pub version: String /* SemVer string, per §B */ }
pub struct PackInfoData {
    pub id_version: PackIdVersion,
    pub size: u64,
    pub content_key: String,
    pub subpack_name: String,
    pub content_identity: String,
    pub has_scripts: bool,
    pub is_addon: bool,
    pub is_ray_tracing_capable: bool,
    pub cdn_url: String,
}
pub struct ResourcePacksInfoPacket {
    pub resource_pack_required: bool,
    pub has_addon_packs: bool,
    pub has_scripts: bool,
    pub force_disable_vibrant_visuals: bool,
    pub world_template_id_and_version: PackIdVersion,
    /// CROSS-D10's `resource_packs` config — this blueprint's own consumer (a future
    /// translator/composition-root blueprint) populates this array from that config's paths;
    /// this crate itself never reads config, only carries the already-resolved list.
    pub resource_packs: Vec<PackInfoData>,
}
impl BedrockPacket for ResourcePacksInfoPacket { const ID: u16 = 6; /* ... */ }

pub struct PackInstanceId { pub id_version: PackIdVersion, pub subpack_name: String }
pub struct ResourcePacksStackPacket {
    pub texture_pack_required: bool,
    pub texture_pack_list: Vec<PackInstanceId>,
    pub base_game_version: String,
    pub experiments: Experiments, // §M
    pub include_editor_packs: bool,
}
impl BedrockPacket for ResourcePacksStackPacket { const ID: u16 = 7; /* ... */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum ResourcePackClientResponseStatus {
    Cancel = 0, Downloading = 1, DownloadingFinished = 2, ResourcePackStackFinished = 3,
}
pub struct ResourcePackClientResponsePacket {
    pub response: ResourcePackClientResponseStatus,
    /// Present (non-empty) only when `response == Downloading` (§B) — this crate does not
    /// enforce that invariant on construction, only on decode (an inbound packet with entries
    /// under a non-`Downloading` status is accepted as-is, never rejected — the translator's own
    /// concern, not a wire-shape violation).
    pub downloading_packs: Vec<String>,
}
impl BedrockPacket for ResourcePackClientResponsePacket { const ID: u16 = 8; /* ... */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum DisconnectReasonCode { /* the 0..149 enum's own values — this blueprint's own baseline
                                    decomposes only the handful this crate's own error paths emit
                                    (`Unknown = 0` fallback plus whatever the implementation step
                                    below enumerates from the fetched page); every other value in
                                    range round-trips via `DisconnectReasonCode::Other(i32)` */
    Unknown = 0,
    Other(i32),
}
pub struct DisconnectPacket {
    pub reason: DisconnectReasonCode,
    pub message: Option<(String, String)>, // (message, filtered_message), present iff not `None` on the wire (§B)
}
impl BedrockPacket for DisconnectPacket { const ID: u16 = 5; /* ... */ }
```

### §M — `StartGamePacket`

The single largest packet this blueprint covers — every field below is restated directly from §B's two HIGH-confidence fetches (`StartGamePacket`'s own 25 top-level fields, `LevelSettings`' own 50), in wire order, with no field silently dropped.

```rust
pub struct StartGamePacket {
    pub entity_id_self: VarInt64,           // ActorUniqueID
    pub runtime_entity_id: VarUint64,        // ActorRuntimeID
    pub game_type: VarInt32,                  // GameType enum, same small vocabulary as PlayStatus's own
                                                // spawn-adjacent context — Survival/Creative/Adventure/Spectator
    pub position: Vec3,
    pub rotation: Vec2,
    pub settings: LevelSettings,               // below
    pub level_id: String,
    pub level_name: String,
    pub template_content_identity: String,
    pub is_trial: bool,
    pub movement_authority: VarInt32,           // SyncedPlayerMovementSettings's own mode field — this
                                                  // blueprint's own resolved movement-authority stance is
                                                  // fixed in §Q, this field carries whichever numeric value
                                                  // that stance resolves to; the second, `rewind_history_size:
                                                  // VarInt32` field §B's own summary bundled into this same
                                                  // "Movement Settings" entry is carried as
                                                  // `movement_rewind_history_size` below
    pub movement_rewind_history_size: VarInt32,
    pub server_authoritative_block_breaking: bool,   // this blueprint's own reasonable inference from the
                                                        // "Movement Settings (varint32 + boolean)" summary —
                                                        // MEDIUM confidence, flagged for confirmation
    pub level_current_time: u64,
    pub enchantment_seed: VarInt32,
    pub block_properties: Vec<ServerBlockProperty>,     // below, §B MEDIUM
    pub multiplayer_correlation_id: String,
    pub enable_item_stack_net_manager: bool,
    pub server_version: String,
    /// §B: "Player Property Data — unknown" — this blueprint's own honest resolution: an
    /// opaque, pre-serialized network-NBT compound (§K), never decomposed field-by-field, exactly
    /// the same pattern §N applies to `BiomeDefinitionListPacket`/`AvailableActorIdentifiersPacket`.
    pub player_property_data: NetworkNbtValue,
    pub server_block_type_registry_checksum: u64,
    pub world_template_id: Uuid128,
    pub server_enabled_client_side_generation: bool,
    /// The field this whole blueprint's own "block-palette hashing — verify" question resolves
    /// against. **This blueprint's own resolved decision: always `true`.** Rusty Clanker sends
    /// self-describing, order-independent hashed block runtime ids rather than committing to a
    /// positionally-agreed enumerated palette the client must reproduce byte-for-byte — this
    /// removes the need to ship/maintain a second "canonical palette order" artifact alongside
    /// M11-B04's own `BedrockBlockState{name, states, version}` correspondence tables, which
    /// themselves carry no ordinal position (M11-B04 Context §7's own runtime shape stores
    /// Java→Bedrock as a flat array indexed by *Java* `BlockStateId`, never a Bedrock-side
    /// ordinal). **The hash function itself is this blueprint's own placeholder, LOW/FLAGGED**
    /// (§B, §O) — `hash_block_state` (§O) implements FNV-1a-64 (truncated to the low 32 bits)
    /// over the network-NBT encoding (§K) of `{name, states}`, chosen for being simple, pure,
    /// and auditable, **not** because it was confirmed to match a real Bedrock client's own
    /// internal hash — CROSS-D25's manual-verification pass (a real, pinned-version client) is
    /// the only mechanism that can confirm or refute this choice; until then this field's own
    /// `true` value is internally consistent (Rusty Clanker's own client-facing ids always match
    /// what `hash_block_state` computes) but not proven wire-compatible.
    pub block_network_ids_are_hashes: bool,
    pub server_auth_sound_enabled: bool,        // "NetworkPermissions" per §B's own summary
    /// §B: "Server Configuration Join Info — optional server_config (complex nested)" — this
    /// blueprint's own honest resolution: represented as `Option<NetworkNbtValue>` (opaque),
    /// never decomposed, for the identical reason `player_property_data` is opaque above.
    pub server_configuration_join_info: Option<NetworkNbtValue>,
    pub server_telemetry_data: ServerTelemetryData, // below, 4 string fields per §B
}

pub struct ServerBlockProperty {
    pub name: String,
    /// §B MEDIUM confidence on the exact per-property shape beyond the name — resolved as an
    /// opaque network-NBT compound describing the block's custom-content state definition,
    /// mirroring the same opaque-blob pattern used elsewhere in this packet.
    pub definition: NetworkNbtValue,
}

pub struct ServerTelemetryData {
    pub telemetry_id: String, pub telemetry_a: String, pub telemetry_b: String, pub telemetry_c: String,
}

pub struct LevelSettings {
    pub seed: u64,
    pub spawn_settings: SpawnSettings,
    pub generator_type: VarInt32,
    pub game_type: VarInt32,
    pub is_hardcore: bool,
    pub game_difficulty: VarInt32,
    pub default_spawn_block_position: BlockPos,
    pub achievements_disabled: bool,
    pub editor_world_type: VarInt32,
    pub is_created_in_editor: bool,
    pub is_exported_from_editor: bool,
    pub day_cycle_stop_time: VarInt32,
    pub education_edition_offer: VarUint32,
    pub education_features_enabled: bool,
    pub education_product_id: String,
    pub rain_level: f32,
    pub lightning_level: f32,
    pub has_confirmed_platform_locked_content: bool,
    pub multiplayer_game_intent: bool,
    pub lan_broadcast_intent: bool,
    pub xbox_live_broadcast_setting: VarInt32,
    pub platform_broadcast_setting: VarInt32,
    pub commands_enabled: bool,
    pub texture_packs_required: bool,
    pub rule_data: Vec<GameRule>,           // "GameRulesChangedPacketData" — array wrapper, this
                                              // blueprint's own resolved prefix convention:
                                              // VarUint32-count-prefixed, matching every other
                                              // confirmed array field's own convention (§F)
    pub experiments: Experiments,
    pub has_bonus_chest_enabled: bool,
    pub start_with_map_enabled: bool,
    pub player_permissions: i8,
    pub server_chunk_tick_range: i32,
    pub has_locked_behavior_pack: bool,
    pub has_locked_resource_pack: bool,
    pub is_from_locked_template: bool,
    pub use_msa_gamertags_only: bool,
    pub is_from_world_template: bool,
    pub is_world_template_option_locked: bool,
    pub only_spawn_v1_villagers: bool,
    pub persona_disabled: bool,
    pub custom_skins_disabled: bool,
    pub emote_chat_muted: bool,
    pub base_game_version: String,
    pub limited_world_width: i32,
    pub limited_world_depth: i32,
    pub nether_type: bool,
    /// §B: `EduSharedUriResource` — this blueprint's own honest resolution: two strings
    /// (`button_name`, `link_uri`), the smallest shape consistent with "shared URI resource" and
    /// every other Edu-prefixed field in this struct being a plain string/bool — **flagged
    /// MEDIUM, the exact field count/names were not independently confirmed this session**.
    pub edu_shared_uri_resource: EduSharedUriResource,
    pub override_force_experimental_gameplay: Option<bool>,
    pub chat_restriction_level: u8,
    pub disable_player_interactions: bool,
    pub server_editor_connection_policy: VarInt32,
    pub allow_anonymous_block_drops_in_editor_worlds: bool,
}

pub struct EduSharedUriResource { pub button_name: String, pub link_uri: String }

pub struct SpawnSettings {
    pub spawn_biome_type: SpawnBiomeType,
    pub user_defined_biome_name: String,
    pub dimension: VarInt32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum SpawnBiomeType { Default = 0, UserDefined = 1 }

#[derive(Debug, Clone)]
pub enum GameRuleValue { Null, Bool(bool), Int(i32), Float(f32) }
pub struct GameRule { pub name: String, pub can_be_modified: bool, pub value: GameRuleValue }

pub struct ExperimentToggle { pub name: String, pub enabled: bool }
pub struct Experiments { pub toggles: Vec<ExperimentToggle>, pub ever_toggled: bool }

impl BedrockPacket for StartGamePacket { const ID: u16 = 11; /* ... */ }
```

### §N — `CreativeContentPacket`, `BiomeDefinitionListPacket`, `AvailableActorIdentifiersPacket`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CreativeCategory { Construction = 0, Nature = 1, Equipment = 2, Items = 3, ItemCommandOnly = 4 }

pub struct NetworkItemInstanceDescriptor {
    pub network_id: VarInt32,        // 0 = empty/air slot, per the near-universal Bedrock convention
                                       // this blueprint's own §R shares (restated once here, applied there)
    pub count: u16,
    pub metadata: VarUint32,
    pub block_runtime_id: VarInt32,
    /// Opaque per-instance NBT tail (enchantments, custom name, damage, can-place-on/can-destroy
    /// lists) — §M's own "M11 tier: type-level identity only" stance (mirroring M11-B04 Context
    /// §5's own item-table scoping) applies identically here: this crate carries the bytes,
    /// never decomposes per-instance component/NBT content.
    pub instance_tail: Bytes,
}
pub struct CreativeGroupInfo { pub category: CreativeCategory, pub name: String, pub group_icon: NetworkItemInstanceDescriptor }
pub struct CreativeItemEntry { pub creative_net_id: VarUint32, pub item: NetworkItemInstanceDescriptor, pub group_index: VarUint32 }
pub struct CreativeContentPacket { pub groups: Vec<CreativeGroupInfo>, pub entries: Vec<CreativeItemEntry> }
impl BedrockPacket for CreativeContentPacket { const ID: u16 = 145; /* ... */ }

/// §B MEDIUM confidence on the exact internal structure (not confirmed as monolithic NBT this
/// session) — this blueprint's own resolved, honest scope: **carried opaquely**. A future
/// `rc-bedrock-translator`/mappings-consumer blueprint is the one place that knows how to build
/// this payload from `rc-bedrock-mappings`' own biome table; this crate only frames/lengths it.
pub struct BiomeDefinitionListPacket { pub payload: Bytes }
impl BedrockPacket for BiomeDefinitionListPacket { const ID: u16 = 122; /* ... */ }

/// Same opaque-payload treatment as `BiomeDefinitionListPacket`, same reasoning (§B MEDIUM).
pub struct AvailableActorIdentifiersPacket { pub payload: Bytes }
impl BedrockPacket for AvailableActorIdentifiersPacket { const ID: u16 = 119; /* ... */ }
```

### §O — Chunk delivery: `LevelChunkPacket`, `SubChunkRequestPacket`, `SubChunkPacket`

**This blueprint's own resolved decision, closing a gap `15-crossplay.md` left open** (the task framing that produced this blueprint expected `15` to have already picked a chunk-delivery mode; it does not — CROSS-D1 through CROSS-D30 never mention `LevelChunk`/`SubChunk` by name): Rusty Clanker's M11 baseline uses the **on-demand `SubChunkRequest`/`SubChunk` system** (§B, HIGH — introduced 1.18.10, the only delivery model any source this session fetched described as currently live), never the legacy single-packet-per-column full delivery some historical third-party servers used. `LevelChunkPacket` is sent first, as a **skeleton**: `sub_chunk_count = 0xffffffff` (the documented "client should request sub-chunks" sentinel, §B) and an empty `payload`/`cache_blob_ids`; the client responds with `SubChunkRequestPacket`s the server answers with `SubChunkPacket`s, per §B's own field tables. This resolution is flagged for reconciliation into `15`'s next revision, exactly the same pattern M11-B03 already established for its own comparable gap-filling additions.

```rust
pub struct ChunkPos { pub x: VarInt32, pub z: VarInt32 }

pub struct LevelChunkPacket {
    pub position: ChunkPos,
    pub dimension: VarInt32,
    /// This blueprint's own baseline (§O's own resolved decision) always writes
    /// `SUB_CHUNK_COUNT_REQUEST_MODE = 0xffff_ffff` here — the two other sentinel/legal values
    /// (`0xffff_fffe` = "known air," or a genuine positive count for the legacy full-delivery
    /// mode) are represented and decodable but never constructed by this blueprint's own
    /// baseline encode path.
    pub sub_chunk_count: u32,
    pub client_request_sub_chunk_limit: Option<VarInt32>,
    pub cache_enabled: bool,
    pub cache_blob_ids: Vec<u64>,          // present iff `cache_enabled`
    pub payload: Bytes,                     // empty for the skeleton-mode baseline (§O)
}
pub const SUB_CHUNK_COUNT_REQUEST_MODE: u32 = 0xffff_ffff;
pub const SUB_CHUNK_COUNT_KNOWN_AIR: u32 = 0xffff_fffe;
impl BedrockPacket for LevelChunkPacket { const ID: u16 = 58; /* ... */ }

pub struct SubChunkOffset { pub x: i8, pub y: i8, pub z: i8 }
pub struct CenterPos { pub x: i32, pub y: i32, pub z: i32 }

pub struct SubChunkRequestPacket {
    pub dimension: VarInt32,
    pub center: CenterPos,
    /// §B's own confirmed shape for this one array is a fixed per-entry 3-byte record with no
    /// separate stated length-prefix convention beyond the generic array rule (§F) — this
    /// blueprint applies §F's default (`VarUint32`-count-prefixed) here too, flagged `// confirm
    /// against a fresh capture` since this is the one array field this session's fetch did not
    /// explicitly restate the prefix type for.
    pub offsets: Vec<SubChunkOffset>,
}
impl BedrockPacket for SubChunkRequestPacket { const ID: u16 = 175; /* ... */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SubChunkResult {
    Success = 1, LevelChunkDoesntExist = 2, WrongDimension = 3, PlayerDoesntExist = 4,
    IndexOutOfBounds = 5, SuccessAllAir = 6,
}
/// §B: two of the six enum entries' exact integers were reconstructed rather than literally
/// quoted from the fetch — flagged `// confirm exact Result integers against a fresh capture,
/// CROSS-D7(b)` at this enum's own definition site.
pub struct SubChunkEntry {
    pub offset: SubChunkOffset,
    pub result: SubChunkResult,
    pub payload: Option<Bytes>,
    pub height_map: Option<HeightMapData>,
    pub blob_id: Option<u64>,
}
/// §O restates M11-B04 Context §7's own runtime-shape precedent for "16×16, `[z][x]` order, `i8`
/// status per cell" — this blueprint's own `HeightMapData` mirrors that shape at the wire level.
pub struct HeightMapData { pub status: HeightMapStatus, pub values: Option<[[i8; 16]; 16]> }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HeightMapStatus { NoData = 0, HasData = 1, AllTooHigh = 2, AllTooLow = 3 }

pub struct SubChunkPacket {
    pub cache_enabled: bool,
    pub dimension: VarInt32,
    pub center: CenterPos,
    pub entries: Vec<SubChunkEntry>,
}
impl BedrockPacket for SubChunkPacket { const ID: u16 = 174; /* ... */ }

/// §M's own placeholder hash function, defined here since it operates on block states, the same
/// domain `chunk.rs` and `block.rs` both serialize. **LOW/FLAGGED, this blueprint's own
/// placeholder** (§B/§M) — FNV-1a-64, truncated to the low 32 bits, over
/// `encode_network_nbt_root` applied to a synthetic `{name, states}` compound built from the
/// fields named. Deterministic and pure (same input always produces the same output, a required
/// property regardless of whether this exact algorithm ever matches a real client), but **not**
/// claimed wire-compatible until CROSS-D25's manual-verification pass confirms or replaces it.
pub fn hash_block_state(name: &str, states: &[(&str, crate::ids_local::BlockPropertyValue)]) -> BlockRuntimeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockRuntimeId(pub u32);
```

### §P — `UpdateBlockPacket`

```rust
pub struct UpdateBlockPacket {
    pub position: BlockPos,
    pub block_runtime_id: BlockRuntimeId, // VarUint32 on the wire, §B HIGH
    pub flags: VarUint32,
    pub layer: VarUint32,
}
impl BedrockPacket for UpdateBlockPacket { const ID: u16 = 21; /* ... */ }
```

### §Q — Movement: `MovePlayerPacket`, `PlayerAuthInputPacket`

**This blueprint's own resolved movement-authority stance** (§B, LOW/FLAGGED — no source this session confirmed the current default, and `15-crossplay.md` does not pick one either, the same kind of gap §O closed for chunk delivery): `PlayerAuthInputPacket` (client→server) is the **sole authoritative per-tick input path** for M11's baseline — every field below is decoded and handed to a future translator as the tick's movement/interaction input. `MovePlayerPacket` is retained, decodable in both directions, for exactly two roles: (a) a **server→client authoritative-correction** message (position mode `Teleport`, per its own `PositionMode` enum) when a future translator's own reconciliation logic needs to snap a client back into agreement with authoritative state, and (b) round-trip fidelity for any legacy client-sent `MovePlayerPacket` this crate's decode path may still see — never treated as the primary input source. This resolves CROSS-D16(f)'s own already-fixed tier-2 stance ("client-side prediction/reconciliation differences... produce bounded, latency-correlated rubber-banding... never desync of authoritative position") at the wire-type level: the *packets* that carry that reconciliation are these two, restated here as this blueprint's own concrete answer to "which one is authoritative," flagged for reconciliation into `15`'s next revision exactly as §O's chunk-delivery resolution already is.

```rust
bitflags_like_newtype! { // this crate's own small bitset newtype (Deliverables, primitives.rs) —
                          // not the external `bitflags` crate (not in Deliverables' Cargo.toml,
                          // §Constraints (b) — a hand-rolled `u32` newtype with named bit
                          // accessors is enough for this one field and avoids one more new
                          // external dependency)
pub struct PlayerAuthInputFlags(pub u32); // Ascend/Descend/Jumping/Sneaking/Sprinting/etc. (§B) —
                                            // this blueprint's own baseline names the handful this
                                            // corpus's own M3-B02 movement/collision work already
                                            // has Java-side analogs for (Jump/Sneak/Sprint/mount
                                            // input) as named accessor methods; every other bit
                                            // round-trips through the raw `u32` untouched.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum InputMode { Undefined = 0, Mouse = 1, Touch = 2, GamePad = 3 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PlayMode { Normal = 0, Teaser = 1, Screen = 2, ExitLevel = 3 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum InteractionModel { Touch = 0, Crosshair = 1, Classic = 2 }

pub struct PlayerAuthInputPacket {
    pub rotation: Vec2,
    pub position: Vec3,
    pub move_vector: Vec2,
    pub head_yaw: f32,
    pub input_data: PlayerAuthInputFlags,     // §B lists this as `array<int32>`; this blueprint's
                                                 // own resolution packs it as one bitset newtype
                                                 // rather than a literal `Vec<i32>` of one-flag-
                                                 // per-element, since every source's own enum
                                                 // vocabulary (Ascend/Descend/Jumping/...) is a
                                                 // small closed set consistent with bitset
                                                 // semantics — flagged MEDIUM, confirm the exact
                                                 // wire shape (bitset vs. literal small-int array)
                                                 // against a fresh capture
    pub input_mode: InputMode,
    pub play_mode: PlayMode,
    pub interaction_model: InteractionModel,
    pub interact_rotation: Vec2,
    pub client_tick: VarUint64,
    pub pos_delta: Vec3,
    /// §B: "Item Use Transaction (optional) — Legacy inventory transaction data." This
    /// blueprint's own resolved scope: carried opaquely (`Bytes`), never decomposed — legacy
    /// inventory-transaction data is explicitly superseded by `ItemStackRequestPacket` (§R) for
    /// M11's own baseline tier, so this crate preserves the bytes for round-trip fidelity without
    /// claiming to interpret them.
    pub item_use_transaction: Option<Bytes>,
    pub item_stack_request: Option<ItemStackRequest>, // §R
    pub player_block_actions: Vec<PlayerBlockAction>,
    pub vehicle_rotation: Vec2,
    pub client_predicted_vehicle: VarUint64, // ActorUniqueID, 0 = none
    pub analog_move_vector: Vec2,
    pub camera_orientation: Vec3,
    pub raw_move_vector: Vec2,
}
pub struct PlayerBlockAction { pub action: VarInt32, pub position: BlockPos, pub face: VarInt32 }
impl BedrockPacket for PlayerAuthInputPacket { const ID: u16 = 144; /* ... */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PositionMode { Normal = 0, Reset = 1, Teleport = 2, Rotation = 3 } // §Q's own resolved
                                                                              // naming for the
                                                                              // fetched "Position
                                                                              // Mode — uint8 (enum)"
                                                                              // field; exact
                                                                              // integer-to-name
                                                                              // mapping beyond
                                                                              // `Teleport`'s own
                                                                              // load-bearing role
                                                                              // (§Q) is MEDIUM
pub struct MovePlayerTeleportData { pub cause: VarInt32, pub source_entity_type: VarInt32 }
pub struct MovePlayerPacket {
    pub runtime_id: VarUint64,
    pub position: Vec3,
    pub rotation: Vec2,
    pub head_yaw: f32,
    pub mode: PositionMode,
    pub on_ground: bool,
    pub riding_runtime_id: VarUint64, // 0 = none
    pub teleport: Option<MovePlayerTeleportData>,
    pub tick: VarUint64,
}
impl BedrockPacket for MovePlayerPacket { const ID: u16 = 19; /* ... */ }
```

### §R — Inventory tier: `InventoryContentPacket`, `InventorySlotPacket`, `ItemStackRequestPacket`

**This blueprint's own honest tier boundary**, restating CROSS-D15's "basic survival/creative inventory manipulation excluding CROSS-D16's offhand carve-out" as Tier-1 and CROSS-D17(b)'s "Java-side custom UI/GUI screens beyond Bedrock's native form primitives" as Tier-3: `InventoryContentPacket`/`InventorySlotPacket` are decomposed fully (§B HIGH — they carry no action semantics, only slot state, so there is nothing tiered about them). `ItemStackRequestPacket`'s 18-member action vocabulary (§B MEDIUM) is decomposed **only for the five actions a plain survival/creative slot manipulation actually uses** — `Take`, `Place`, `Swap`, `Drop`, `Destroy` — with every other action type (every `Screen*`/`Craft*` variant, which correspond to Tier-3-adjacent custom-UI/crafting-table interactions this blueprint's own scope does not reach) preserved through one **opaque, byte-round-tripping catch-all variant**, never silently dropped:

```rust
pub struct NetworkItemStackDescriptor {
    pub network_id: VarInt32,       // 0 = empty slot (§N's own restated convention)
    pub count: u16,
    pub metadata: VarUint32,
    pub block_runtime_id: BlockRuntimeId,
    pub instance_tail: Bytes,        // §N's own opaque-NBT-tail convention, restated
}

pub struct StackRequestSlotInfo { pub container_id: u8, pub slot: u8, pub stack_network_id: VarInt32 }

#[derive(Debug, Clone)]
pub enum ItemStackRequestAction {
    Take { count: u8, source: StackRequestSlotInfo, destination: StackRequestSlotInfo },
    Place { count: u8, source: StackRequestSlotInfo, destination: StackRequestSlotInfo },
    Swap { source: StackRequestSlotInfo, destination: StackRequestSlotInfo },
    Drop { count: u8, source: StackRequestSlotInfo, randomly: bool },
    Destroy { count: u8, source: StackRequestSlotInfo },
    /// Every action type this blueprint's own M11 baseline does not decompose (§R) — `kind` is
    /// the raw wire discriminant byte, `raw` is that action's own remaining, un-interpreted
    /// bytes. `encode` writes `kind` then `raw` verbatim; `decode` — for a `kind` value outside
    /// the five decomposed variants above — reads the **entire remainder of this one action's
    /// own encoded region** into `raw` only where this blueprint's own implementation step can
    /// determine that region's exact length from context (Constraints, this is this blueprint's
    /// one honestly-named implementation risk: several `Craft*` actions' own field layouts were
    /// not independently confirmed this session, so a mis-sized `Other` read could misalign the
    /// *next* action's own decode — flagged `// HIGH-RISK: confirm every unmodeled action's exact
    /// field layout against a fresh capture before this variant is trusted with adjacent-field
    /// safety, CROSS-D7(b)` at this variant's own decode arm).
    Other { kind: u8, raw: Bytes },
}
pub struct ItemStackRequest { pub client_request_id: VarInt32, pub actions: Vec<ItemStackRequestAction> }
pub struct ItemStackRequestPacket { pub requests: Vec<ItemStackRequest> }
impl BedrockPacket for ItemStackRequestPacket { const ID: u16 = 147; /* ... */ }

pub struct FullContainerName { pub name: u8, pub dynamic_id: u32 }
pub struct InventoryContentPacket {
    pub container_id: VarUint32,
    pub slots: Vec<NetworkItemStackDescriptor>,
    pub full_container_name: FullContainerName,
    pub storage_item: NetworkItemStackDescriptor,
}
impl BedrockPacket for InventoryContentPacket { const ID: u16 = 49; /* ... */ }

pub struct InventorySlotPacket {
    pub container_id: u8,
    pub slot: VarUint32,
    pub full_container_name: Option<FullContainerName>,
    pub storage_item: Option<NetworkItemStackDescriptor>,
    pub item: NetworkItemStackDescriptor,
}
impl BedrockPacket for InventorySlotPacket { const ID: u16 = 50; /* ... */ }
```

### §S — Chat, player list, entity lifecycle/sync

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextType {
    Raw, Tip, System, WhisperTextObject, TextObject, AnnouncementTextObject,
    Chat, Whisper, Announcement,
    Translate, Popup, JukeboxPopup,
}
#[derive(Debug, Clone)]
pub enum TextBody {
    MessageOnly { message: String },
    AuthorAndMessage { player_name: String, message: String },
    MessageAndParams { message: String, parameters: Vec<String> },
}
pub struct TextPacket {
    pub localize: bool,
    pub text_type: TextType,
    pub body: TextBody,           // shape determined by `text_type`, per §B's own three-way split
    pub xuid: String,
    pub platform_id: String,
    pub filtered_message: String,
}
impl BedrockPacket for TextPacket { const ID: u16 = 9; /* ... */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BuildPlatform {
    Unknown = 0, Google = 1, IOS = 2, OSX = 3, Amazon = 4, Win32 = 5, Dedicated = 6,
    TvOS = 7, Sony = 8, Nintendo = 9, Xbox = 10, WindowsPhone = 11, Linux = 12,
}
/// §B MEDIUM: identity fields decomposed; the skin/persona payload — §B's own "serialized skin
/// data (image dimensions, animated textures, geometry, cape information)... persona
/// customization pieces (29 types)" — is carried opaquely (`skin_and_persona: Bytes`), the
/// identical "type-level identity, not full instance decomposition" boundary §N/§R already draw,
/// applied here to skin data for the same reason: full skin-geometry decomposition is
/// `rc-bedrock-translator`'s own future scope, not this codec crate's.
pub struct PlayerListAddEntry {
    pub uuid: Uuid128,
    pub actor_unique_id: VarInt64,
    pub name: String,
    pub xuid: String,
    pub platform_online_id: String,
    pub build_platform: BuildPlatform,
    pub skin_and_persona: Bytes,
    pub is_teacher: bool,
    pub is_host: bool,
}
pub enum PlayerListEntry { Add(PlayerListAddEntry), Remove { uuid: Uuid128 } }
/// One `PlayerListPacket` carries either an all-`Add` or an all-`Remove` array on the real wire
/// (§B's own "action identifier" framing) — this blueprint's own `PlayerListPacket` still models
/// a mixed `Vec<PlayerListEntry>` for encode-side flexibility, but `decode` enforces the
/// real-wire invariant (a single leading action byte applies to the whole array, never per-entry)
/// by reading one `PlayerListAction` discriminant then one homogeneous array, never a
/// per-element tag — `PlayerListEntry`'s own two-variant shape is this blueprint's own in-memory
/// convenience, not a literal per-element wire tag.
pub struct PlayerListPacket { pub entries: Vec<PlayerListEntry> }
impl BedrockPacket for PlayerListPacket { const ID: u16 = 63; /* ... */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum GameType { Survival = 0, Creative = 1, Adventure = 2, SurvivalSpectator = 3, CreativeSpectator = 4, Spectator = 6 }
/// §B's own "EntityData field supports multiple payload types" restated as this crate's shared
/// entity-metadata value type, reused by `AddPlayerPacket`, `AddActorPacket`, and
/// `SetActorDataPacket` alike (§B's own field tables name the identical payload-type set for all
/// three, so one shared enum serves all of them rather than three near-identical copies —
/// mirroring M11-B04 Context §Context-4's own "one generic struct serves all three" reasoning).
#[derive(Debug, Clone)]
pub enum ActorDataValue {
    Byte(i8), Short(i16), Int(VarInt32), Float(f32), String(String),
    Compound(NetworkNbtValue), Pos(BlockPos), Long(VarInt64), Vec3(Vec3),
}
pub struct ActorDataEntry { pub id: VarUint32, pub value: ActorDataValue }

pub struct AddPlayerPacket {
    pub uuid: Uuid128,
    pub name: String,
    pub runtime_id: VarUint64,
    pub unique_id: VarInt64,
    pub platform_chat_id: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Vec2,
    pub head_yaw: f32,
    pub held_item: NetworkItemStackDescriptor,
    pub game_type: GameType,
    pub actor_data: Vec<ActorDataEntry>,
    /// §B's own "Permissions & Abilities" bundle — this blueprint's own resolved shape: the
    /// smallest set consistent with every other Bedrock ability-flag list this session's own
    /// research corpus (M11-B03's own honest-scoping precedent) would expect: a permission-level
    /// enum plus a raw ability-flags bitset, flagged `// MEDIUM — exact PlayerPermissionLevel
    /// enum values and AbilityLayer field count not independently confirmed this session`.
    pub permission_level: VarInt32,
    pub ability_flags: u32,
    pub device_id: String,
    pub build_platform: BuildPlatform,
}
impl BedrockPacket for AddPlayerPacket { const ID: u16 = 12; /* ... */ }

pub struct SyncedAttribute { pub name: String, pub min: f32, pub max: f32, pub current: f32, pub default: f32 }
pub struct ActorLink { pub rider_unique_id: VarInt64, pub ridden_unique_id: VarInt64, pub link_type: u8, pub immediate: bool, pub cause_rider_dismount: bool }
pub struct AddActorPacket {
    pub unique_id: VarInt64,
    pub runtime_id: VarUint64,
    /// §S's own local newtype (§A) — never `rc-bedrock-mappings::BedrockEntityId`.
    pub actor_type: EntityTypeId,
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Vec2,
    pub head_yaw: f32,
    pub body_yaw: f32,
    pub attributes: Vec<SyncedAttribute>,
    pub actor_data: Vec<ActorDataEntry>,
    pub synced_properties: PropertySyncData,
    pub links: Vec<ActorLink>,
}
pub struct PropertySyncData {
    pub int_entries: Vec<(VarUint32, VarInt32)>,
    pub float_entries: Vec<(VarUint32, f32)>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityTypeId(pub String); // namespaced identifier, e.g. "minecraft:zombie" (§S)
impl BedrockPacket for AddActorPacket { const ID: u16 = 13; /* ... */ }

pub struct RemoveActorPacket { pub target_unique_id: VarInt64 }
impl BedrockPacket for RemoveActorPacket { const ID: u16 = 14; /* ... */ }

pub struct MoveActorAbsolutePacket {
    pub runtime_id: VarUint64,
    pub flags: u8,
    pub position: Vec3,
    /// §B's own confirmed byte-packed angle encoding (`0..255` mapping to a full `0..360°`
    /// rotation) — a real, distinct convention from `Vec2`'s own `f32` rotation fields used
    /// elsewhere in this same packet family (`AddActorPacket`, `PlayerAuthInputPacket`), worth
    /// flagging explicitly since conflating the two would silently corrupt every decoded angle.
    pub rotation_x: u8, pub rotation_y: u8, pub rotation_y_head: u8,
}
impl BedrockPacket for MoveActorAbsolutePacket { const ID: u16 = 18; /* ... */ }

pub struct SetActorDataPacket {
    pub target_runtime_id: VarUint64,
    pub actor_data: Vec<ActorDataEntry>,
    pub synced_properties: PropertySyncData,
    pub tick: VarUint64,
}
impl BedrockPacket for SetActorDataPacket { const ID: u16 = 39; /* ... */ }
```

### §T — Serialization/codegen stance, restated

No derive macro exists or is introduced by this blueprint (§A/§E) — every `encode`/`decode` body above is hand-written by the implementer against the `WireWrite`/`WireRead` primitive table (§F) and this section's own field-by-field tables, mirroring this crate-family's own already-established convention (M11-B01's `Frame`/`DatagramHeader`) rather than NET-D3's `#[derive(RcPacket)]` approach, which has no ratified counterpart crate here (§A). This is a deliberate, structural consequence of CROSS-D2's own five-crate manifest, not an oversight.

## Deliverables

### `crates/bedrock-protocol/Cargo.toml` (new)

```toml
[package]
name = "rc-bedrock-protocol"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
bytes     = { workspace = true }
thiserror = { workspace = true }
tracing   = { workspace = true }
flate2    = { workspace = true }   # already NET-D5-pinned (zlib-ng feature) — §H, zero new
                                     # workspace dependency added by this blueprint

[dev-dependencies]
proptest = { workspace = true }
```

(No `rc-*` path dependency — §A. No new `[workspace.dependencies]` entry — §H's own explicitly-noted positive property, in contrast to every prior M11 blueprint each adding at least one new external pin.)

### `crates/bedrock-protocol/src/lib.rs`

```rust
//! `rc-bedrock-protocol` — Bedrock's own game-packet wire codec (CROSS-D2): the `0xFE` batch
//! envelope, compression, this crate's own hand-rolled network-NBT variant, and the M11 packet
//! catalog (login/handshake through the mapped-tier play surface). Server-only,
//! `crossplay`-Cargo-feature-gated at the `rusty-clanker-server` consumer level (WS-D5(e)) — this
//! crate itself carries no feature gate of its own. Pure data/codec: no sockets, no `tokio`, no
//! ECS, mirrors `rc-protocol`'s own isolation (NET-D8). Depends on zero other RC crates (§A).

pub mod error;
pub mod primitives;
pub mod nbt;
pub mod packet;
pub mod batch;
pub mod handshake;
pub mod login;
pub mod resourcepacks;
pub mod startgame;
pub mod catalog;
pub mod chunk;
pub mod block;
pub mod movement;
pub mod inventory;
pub mod chat;
pub mod entity;

pub use packet::{BedrockPacket, PacketHeader, SubClientId};
pub use batch::{CompressionAlgorithm, decode_batch, encode_batch};
pub use primitives::{BedrockString, BlockPos, Uuid128, Vec2, Vec3, VarInt32, VarInt64, VarUint32, VarUint64};
pub use nbt::{NetworkNbtValue, decode_network_nbt_root, encode_network_nbt_root};
```

### Module contents

Every type and function signature named in Context §E–§S above is this blueprint's own complete public API surface, organized into source files exactly as Context's own section headers imply their module home:

| Module | Types/functions (from Context) |
|---|---|
| `error.rs` | `PacketDecodeError`, `BatchDecodeError`, `CompressionError`, `NbtDecodeError` — each a `thiserror::Error` enum; every fallible function above returns one of these, never panics on adversarial input (Acceptance tests, `fuzz_stub.rs`) |
| `primitives.rs` | §F's full table, `WireWrite`/`WireRead` traits + impls, `PlayerAuthInputFlags` (§Q's hand-rolled bitset newtype) |
| `nbt.rs` | §K: `NetworkNbtValue`, `encode_network_nbt_root`, `decode_network_nbt_root` |
| `packet.rs` | §E/§J: `BedrockPacket`, `PacketHeader`, `SubClientId`, `pack_sub_packet`, `unpack_sub_packet` |
| `batch.rs` | §G/§H: `encode_batch`, `decode_batch`, `CompressionAlgorithm` |
| `handshake.rs` | §H: `NetworkSettingsPacket`, `RequestNetworkSettingsPacket` |
| `login.rs` | §L: `LoginPacket`, `ServerToClientHandshakePacket`, `ClientToServerHandshakePacket`, `PlayStatus`, `PlayStatusPacket`, `DisconnectPacket`, `DisconnectReasonCode` |
| `resourcepacks.rs` | §L: `PackIdVersion`, `PackInfoData`, `ResourcePacksInfoPacket`, `PackInstanceId`, `ResourcePacksStackPacket`, `ResourcePackClientResponseStatus`, `ResourcePackClientResponsePacket` |
| `startgame.rs` | §M: `StartGamePacket`, `LevelSettings`, `ServerBlockProperty`, `ServerTelemetryData`, `EduSharedUriResource`, `SpawnSettings`, `SpawnBiomeType`, `GameRuleValue`, `GameRule`, `ExperimentToggle`, `Experiments` |
| `catalog.rs` | §N: `CreativeCategory`, `NetworkItemInstanceDescriptor`, `CreativeGroupInfo`, `CreativeItemEntry`, `CreativeContentPacket`, `BiomeDefinitionListPacket`, `AvailableActorIdentifiersPacket` |
| `chunk.rs` | §O: `ChunkPos`, `LevelChunkPacket`, `SUB_CHUNK_COUNT_REQUEST_MODE`, `SUB_CHUNK_COUNT_KNOWN_AIR`, `SubChunkOffset`, `CenterPos`, `SubChunkRequestPacket`, `SubChunkResult`, `SubChunkEntry`, `HeightMapData`, `HeightMapStatus`, `SubChunkPacket`, `hash_block_state`, `BlockRuntimeId` |
| `block.rs` | §P: `UpdateBlockPacket` |
| `movement.rs` | §Q: `InputMode`, `PlayMode`, `InteractionModel`, `PlayerAuthInputPacket`, `PlayerBlockAction`, `PositionMode`, `MovePlayerTeleportData`, `MovePlayerPacket` |
| `inventory.rs` | §R: `NetworkItemStackDescriptor`, `StackRequestSlotInfo`, `ItemStackRequestAction`, `ItemStackRequest`, `ItemStackRequestPacket`, `FullContainerName`, `InventoryContentPacket`, `InventorySlotPacket` |
| `chat.rs` | §S: `TextType`, `TextBody`, `TextPacket` |
| `entity.rs` | §S: `BuildPlatform`, `PlayerListAddEntry`, `PlayerListEntry`, `PlayerListPacket`, `GameType`, `ActorDataValue`, `ActorDataEntry`, `AddPlayerPacket`, `SyncedAttribute`, `ActorLink`, `AddActorPacket`, `PropertySyncData`, `EntityTypeId`, `RemoveActorPacket`, `MoveActorAbsolutePacket`, `SetActorDataPacket` |

Every packet struct's `impl BedrockPacket` block is `todo!()`-stubbed in the test changeset (Acceptance tests' own changeset boundary) and filled in during the implementation changeset (Implementation steps), field by field, per each section's own table above — no field is optional to implement; a struct missing a field relative to its own Context table is itself a defect in the implementation changeset, not a simplification.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46):** the test changeset is every file listed below, plus every `src/*.rs` file from Deliverables with executable bodies (every `encode`/`decode`/`compress`/`decompress`/`hash_block_state` body) replaced by `todo!()` — every struct field, enum variant, derive, and public signature stays exactly as Deliverables/Context fix it. The implementation changeset fills in real bodies only; it must not touch a test file, must not add/remove/weaken a test case.

### `crates/bedrock-protocol/tests/support/mod.rs` (test-only, not a deliverable)

`fn hex(s: &str) -> Bytes` (a small hand-decode helper for literal byte fixtures written as hex strings in test source, improving fixture readability); every fixture byte array in every test file below is constructed directly from this blueprint's own Context field tables (own-authored, per ASSET-D18/D19 — never extracted from a captured trace).

### `crates/bedrock-protocol/tests/primitives.rs`

1. `varint32_known_answer_vectors` — table-driven: `(0, [0x00])`, `(-1, [0x01])` (ZigZag of `-1` is `1`), `(1, [0x02])`, `(-2, [0x03])`, `(300, [0xAC, 0x04])` (ZigZag `600 = 0b10_0101_1000` → `[0xd8, 0x04]`, computed and asserted by the test itself against the stated algorithm rather than hand-transcribed here, avoiding a transcription error in the blueprint itself) — `VarInt32::encode`/`decode` round-trip and match every listed byte sequence exactly.
2. `varuint32_no_zigzag` — `VarUint32(300).encode()` produces a **different** byte sequence than `VarInt32(300).encode()` would for the same numeric magnitude (proving ZigZag is genuinely applied only to the signed variants, §F/§B) — assert this by direct byte-sequence inequality, not by re-deriving the formula.
3. `varint64_and_varuint64_max_byte_length` — the largest representable `i64::MIN`/`u64::MAX` each round-trip within `VarInt64`'s 10-byte / `VarUint64`'s 10-byte cap; a hand-built 11-continuation-byte sequence is rejected (`Err`, never panics).
4. `bedrock_string_roundtrip_and_length_prefix` — `BedrockString("Hi".into()).encode()` produces exactly `[0x02, b'H', b'i']` (§B's own quoted example, restated as a literal assertion); an empty string round-trips to `[0x00]`.
5. `vec2_vec3_block_pos_uuid128_roundtrip` — one fixed, hand-chosen value per type, encode then decode, exact field equality (including negative `BlockPos` coordinates, exercising `VarInt32`'s own ZigZag path a second time in context).
6. `fixed_width_endianness_is_little_endian` — `1u16.encode()` (a raw, non-Var fixed-width field per §F) produces `[0x01, 0x00]`, never `[0x00, 0x01]` — a direct, load-bearing assertion of §F's own flagged LE convention.

### `crates/bedrock-protocol/tests/nbt.rs`

1. `network_nbt_roundtrip_every_tag_kind` — one hand-built `NetworkNbtValue::Compound` fixture containing at least one of every variant (`Byte`, `Short`, `Int`, `Long`, `Float`, `Double`, `ByteArray`, `String`, `List`, `Compound` (nested), `IntArray`, `LongArray`), `encode_network_nbt_root` then `decode_network_nbt_root`, assert exact equality including insertion order.
2. `network_nbt_int_and_list_lengths_use_zigzag_varint` — a fixed `Int(-1)` value's encoded bytes match `VarInt32(-1)`'s own known-answer encoding from `primitives.rs` test 1 (cross-file consistency, proving `nbt.rs` genuinely reuses the same VarInt algorithm rather than a silently-diverged second implementation) — same check repeated for a `List`'s own element-count prefix.
3. `network_nbt_short_float_double_are_fixed_le_never_var` — a `Short(-1)`'s encoded byte length is exactly 2 (never a 1-byte VarInt encoding, proving the "these three types are NOT VarInt" rule, §K, is actually honored).
4. `fuzz_network_nbt_decode_never_panics` (`proptest!`) — an arbitrary `Vec<u8>` of length `0..=512` fed to `decode_network_nbt_root`, wrapped in `catch_unwind`, asserted to return (never panic) regardless of the `Result` variant.

### `crates/bedrock-protocol/tests/batch_and_framing.rs`

1. `batch_roundtrip_uncompressed` — a fixed list of 3 hand-built sub-packet byte blobs, `encode_batch(&blobs, None)` → `decode_batch` → exact equality; assert the raw bytes' first byte is `0xFE` and no second byte is present (§G's own pre-negotiation policy).
2. `batch_roundtrip_zlib` — same 3 blobs, `encode_batch(&blobs, Some(CompressionAlgorithm::Zlib))`; assert byte 2 (after `0xFE`) equals `0x00` (§B's own confirmed Zlib id), the payload round-trips through `decode_batch`, and the compressed form is byte-length-smaller than the uncompressed form for a sufficiently repetitive fixture (a real compression sanity check, not just a round-trip one).
3. `batch_snappy_selection_is_representable_but_returns_not_implemented` — `CompressionAlgorithm::Snappy.compress(b"x")` → `Err(CompressionError::SnappyNotImplemented)`, never a panic, never silently falling back to Zlib (§H's own named non-goal, proven not to silently misbehave).
4. `sub_packet_header_bit_field_boundaries` — `PacketHeader { packet_id: 1023, sender: SubClientId(3), target: SubClientId(3) }` round-trips exactly (the maximum representable value in every one of the three bit-fields simultaneously); `PacketHeader { packet_id: 1024, .. }` — `encode` (a programmer error) is asserted via `#[should_panic]` or a debug-assert, per the implementer's own choice, `PacketHeader::decode` of a hand-built header value that *would* decode to `packet_id > 1023` is impossible by construction (10 bits caps it), so this direction needs no separate test.
5. `pack_unpack_sub_packet_roundtrip` — `pack_sub_packet` then `unpack_sub_packet` on 3 distinct `(PacketHeader, body)` pairs of varying body length (including a body long enough to require a multi-byte `GamepacketLength` VarUint32), exact equality.
6. `fuzz_batch_and_sub_packet_decode_never_panics` (`proptest!`, two cases) — arbitrary `Vec<u8>` fed to `decode_batch` and separately to `unpack_sub_packet`, both under `catch_unwind`, both asserted to never panic.

### `crates/bedrock-protocol/tests/login_and_handshake.rs`

1. `request_network_settings_and_network_settings_roundtrip` — one fixed value each, round-trip.
2. `login_packet_chain_and_client_data_roundtrip` — a `LoginPacket` with a 3-element `chain` (arbitrary placeholder strings — this test does not itself validate JWT content, only that the string vector round-trips byte-for-byte) and a `client_data_token` string; round-trip.
3. `client_to_server_handshake_zero_fields_roundtrips` — `ClientToServerHandshakePacket.encode()` produces zero bytes beyond the sub-packet header; `decode` on an empty body succeeds.
4. `play_status_every_enum_value_roundtrips` — all 10 named `PlayStatus` variants, table-driven.
5. `disconnect_packet_with_and_without_message_roundtrip` — two cases, `message: None` and `message: Some((..))`.
6. `resource_packs_info_stack_client_response_roundtrip` — one fixed value each, including `ResourcePackClientResponsePacket` with `response: Downloading` and a non-empty `downloading_packs` list, and a second case with `response: Cancel` and an empty list.

### `crates/bedrock-protocol/tests/startgame.rs`

1. `start_game_packet_full_roundtrip` — one large, own-authored fixture populating **every** field named in Context §M (all 25 top-level `StartGamePacket` fields, all 50 `LevelSettings` fields, plus `GameRule`/`SpawnSettings`/`Experiments`/`ServerBlockProperty` nested values) with distinct, recognizable values; encode then decode; assert full structural equality field-by-field (a single top-level `assert_eq!` on the whole struct, `#[derive(PartialEq)]`, is sufficient and preferred over 75 individual field assertions — but the fixture itself must set every field to a non-default value so a forgotten field would be caught).
2. `block_network_ids_are_hashes_is_always_true_in_this_blueprints_baseline` — a trivial but load-bearing assertion pinning the resolved decision (§M) so a future accidental edit does not silently flip it: this blueprint's own composition helper `StartGamePacket::baseline_for(...)` (Deliverables addition, implementer's own convenience constructor, not individually specified field-by-field here) sets `block_network_ids_are_hashes: true`.
3. `hash_block_state_is_deterministic_and_distinguishes_distinct_states` — `hash_block_state("minecraft:stone", &[])` called twice produces the identical `BlockRuntimeId`; a different `name` or a different `states` slice produces a different `BlockRuntimeId` for at least 20 own-chosen distinct `(name, states)` fixtures (a basic collision sanity check, not a cryptographic guarantee — proptest is not required here since FNV-1a's own collision behavior over a small fixed fixture set is deterministic and cheap to check exhaustively).
4. `fuzz_start_game_decode_never_panics` (`proptest!`) — arbitrary bytes fed to `StartGamePacket::decode`, `catch_unwind`, never panics.

### `crates/bedrock-protocol/tests/catalog.rs`

`creative_content_roundtrip` — one group, two entries, round-trip. `biome_definition_list_and_available_actor_identifiers_are_opaque_passthrough` — a fixed `Bytes` payload round-trips byte-for-byte through both packet types' own `encode`/`decode` with zero interpretation (proving the opaque-blob design, §N, actually preserves bytes exactly rather than silently truncating/reformatting).

### `crates/bedrock-protocol/tests/chunk_and_block.rs`

1. `level_chunk_skeleton_mode_roundtrip` — a `LevelChunkPacket` with `sub_chunk_count: SUB_CHUNK_COUNT_REQUEST_MODE`, empty `payload`/`cache_blob_ids`; round-trip; separately, `sub_chunk_count: SUB_CHUNK_COUNT_KNOWN_AIR` and a genuine positive count (e.g. `24`) with non-empty `payload` each round-trip too (proving the type does not silently assume only the baseline's own chosen mode).
2. `sub_chunk_request_and_sub_chunk_response_roundtrip` — a `SubChunkRequestPacket` with 5 own-chosen `SubChunkOffset` values (including negative `i8` offsets); a matching `SubChunkPacket` with one `Success`, one `LevelChunkDoesntExist` (no `payload`/`blob_id`), and one `SuccessAllAir` entry, each round-tripping independently.
3. `height_map_data_all_four_statuses_roundtrip` — `NoData`/`HasData` (with a fixed `[[i8;16];16]`)/`AllTooHigh`/`AllTooLow`, table-driven.
4. `update_block_packet_roundtrip` — a fixed `BlockPos` (including at least one negative coordinate), `BlockRuntimeId`, `flags`, `layer`; round-trip.

### `crates/bedrock-protocol/tests/movement.rs`

1. `player_auth_input_full_roundtrip` — one fixture populating all 19 fields (§Q), including a non-empty `player_block_actions` list, `item_use_transaction: Some(..)`, and `item_stack_request: Some(..)` with at least one decomposed action and one `Other` catch-all action.
2. `player_auth_input_flags_named_accessors` — table-driven over the handful of named `PlayerAuthInputFlags` accessor methods the implementer adds (Jump/Sneak/Sprint at minimum, per §Q's own baseline note): setting one named bit and re-reading it via its own accessor round-trips; setting an *unnamed* bit (a raw `u32` value with only a high, unnamed bit set) still round-trips through the raw `u32` representation untouched.
3. `move_player_packet_teleport_and_non_teleport_roundtrip` — one case with `mode: PositionMode::Teleport` and `teleport: Some(..)`, one with a non-`Teleport` mode and `teleport: None`.
4. `fuzz_player_auth_input_and_move_player_decode_never_panics` (`proptest!`, two cases).

### `crates/bedrock-protocol/tests/inventory.rs`

1. `inventory_content_and_slot_roundtrip` — one fixture each, including at least one `NetworkItemStackDescriptor` with `network_id: 0` (empty slot, §R's own restated convention) and one non-empty slot with a non-empty `instance_tail`.
2. `item_stack_request_decomposed_actions_roundtrip` — one `ItemStackRequest` containing one instance each of `Take`/`Place`/`Swap`/`Drop`/`Destroy`, round-trip, exact field equality per action.
3. `item_stack_request_opaque_catchall_preserves_unknown_action_bytes_exactly` — a hand-built `Other { kind: 200, raw: Bytes::from_static(&[0xDE, 0xAD, 0xBE, 0xEF]) }` (an arbitrary, deliberately-unrecognized `kind` value) round-trips with `raw` byte-for-byte identical — the one test directly proving §R's own "never silently dropped" claim for the undecomposed action tier.
4. `fuzz_item_stack_request_decode_never_panics` (`proptest!`).

### `crates/bedrock-protocol/tests/chat_and_entity.rs`

1. `text_packet_every_type_variant_roundtrips` — table-driven over all 9 `TextType` variants named in §S, each with its own correct `TextBody` shape (`MessageOnly` for `Raw`/`Tip`/`System`/the three `*TextObject*` variants; `AuthorAndMessage` for `Chat`/`Whisper`/`Announcement`; `MessageAndParams` for `Translate`/`Popup`/`JukeboxPopup`) — 9 cases, one per `TextType` value.
2. `player_list_add_and_remove_roundtrip` — one `PlayerListPacket` with two `Add` entries (distinct `Uuid128`s, non-empty `skin_and_persona` blob), one with a single `Remove` entry.
3. `add_player_add_actor_remove_actor_move_actor_absolute_set_actor_data_roundtrip` — one fixture per packet type, each populating every field named in §S (`AddActorPacket`'s own `attributes`/`actor_data`/`synced_properties`/`links` all non-empty; `MoveActorAbsolutePacket`'s three angle-byte fields distinguishable from a `Vec2`-based rotation to guard against the §S-flagged conflation risk — assert `rotation_x`/`rotation_y`/`rotation_y_head` are each read back as the literal `u8` written, not reinterpreted as a float).
4. `fuzz_text_and_entity_decode_never_panics` (`proptest!`, covering `TextPacket`, `AddActorPacket`, `SetActorDataPacket` decode paths).

### `crates/bedrock-protocol/tests/encryption_seam_plumbing.rs`

Proves §I's own "whole post-compression batch payload" scope decision at the *plumbing* level, **without** a Cargo dependency on `rc-bedrock-auth` (§A — a test-local fake AEAD stand-in is used instead, keeping this crate's own test changeset self-contained and buildable with zero knowledge of M11-B03's real crypto):

```rust
// test-local only, not a deliverable — a length-preserving-plus-4-byte-tag XOR "cipher" purely to
// prove ordering/scope, never claimed to resemble real AES-GCM.
struct FakeAead { key: u8 }
impl FakeAead {
    fn seal(&self, plaintext: &[u8]) -> Vec<u8> { /* XOR every byte with `key`, append a fixed 4-byte marker tag */ }
    fn open(&self, ciphertext: &[u8]) -> Result<Vec<u8>, &'static str> { /* verify + strip the marker tag, XOR back */ }
}
```

`encryption_wraps_whole_compressed_batch_payload` — build a 3-sub-packet batch, `compress`, `FakeAead::seal` the compressed bytes, prepend `0xFE` + the compression-id byte (§G) to the **sealed** bytes (proving the intended pipeline order: compress, then encrypt, then frame — §I's own stated scope), then reverse the whole pipeline (`FakeAead::open`, `decompress`, `decode_batch`'s own sub-packet unpacking) and assert the original 3 sub-packets come back exactly.

## Implementation steps

1. **`crates/bedrock-protocol/Cargo.toml`, `src/lib.rs`.** Exactly as Deliverables. Observable: `cargo build -p rc-bedrock-protocol` compiles with every module a stub.
2. **`error.rs`, `primitives.rs`.** Implement every §F type's `WireWrite`/`WireRead`, the `VarInt32`/`VarUint32`/`VarInt64`/`VarUint64` ZigZag-vs-not encode/decode per §B's own confirmed algorithm, `BedrockString`, `PlayerAuthInputFlags`'s hand-rolled bit accessors. Observable: `primitives.rs` test file passes in full.
3. **`nbt.rs`.** Implement `encode_network_nbt_root`/`decode_network_nbt_root` per §K's own field-family rules, reusing `primitives::BedrockString`'s own string codec directly (no second implementation). Observable: `nbt.rs` test file passes in full.
4. **`packet.rs`.** Implement `PacketHeader::encode`/`decode` (the 10/2/2-bit packing, §J), `pack_sub_packet`/`unpack_sub_packet`. Observable: `batch_and_framing.rs`'s `sub_packet_header_bit_field_boundaries`/`pack_unpack_sub_packet_roundtrip` cases pass.
5. **`batch.rs`.** Implement `CompressionAlgorithm::compress`/`decompress` (Zlib via `flate2`, Snappy returning `Err`), `encode_batch`/`decode_batch` per §G's own resolved policy. Observable: `batch_and_framing.rs` passes in full.
6. **`handshake.rs`, `login.rs`, `resourcepacks.rs`.** Implement every packet's `encode`/`decode` per §L's field tables. Observable: `login_and_handshake.rs` passes in full.
7. **`startgame.rs`.** Implement `StartGamePacket`/`LevelSettings`/every nested type's `encode`/`decode` per §M's own full field-by-field table, in the exact wire order given — this is the single largest implementation step in this blueprint; work top-to-bottom through §M's own struct definitions field by field, never reordering. Observable: `startgame.rs` test file passes in full.
8. **`catalog.rs`.** Implement `CreativeContentPacket` fully; `BiomeDefinitionListPacket`/`AvailableActorIdentifiersPacket` as the opaque `{ payload: Bytes }` passthrough (§N). Observable: `catalog.rs` passes.
9. **`chunk.rs`, `block.rs`.** Implement `LevelChunkPacket`/`SubChunkRequestPacket`/`SubChunkPacket`/`UpdateBlockPacket` per §O/§P; implement `hash_block_state` as FNV-1a-64 (truncated to `u32`) over `encode_network_nbt_root` applied to a `{name, states}` compound built from the function's own arguments — see §M/§O's own LOW/FLAGGED note, implement it as specified even though it is a placeholder, never leave it `unimplemented!()`. Observable: `chunk_and_block.rs` passes.
10. **`movement.rs`.** Implement `PlayerAuthInputPacket`/`MovePlayerPacket` per §Q. Observable: `movement.rs` passes.
11. **`inventory.rs`.** Implement `InventoryContentPacket`/`InventorySlotPacket` fully; `ItemStackRequestPacket`'s five decomposed actions plus the `Other` catch-all per §R — for the catch-all's decode arm, since this blueprint's own research could not confirm every undecomposed action's exact field layout (§B MEDIUM, §R's own flagged risk), the implementer's own resolution must be: read the action's `kind` byte, then — for the five known kinds, decode per their own struct; for any other kind, read a `VarUint32`-length-prefixed byte blob as `raw` (this blueprint's own chosen, explicit wire convention for the catch-all specifically, **not** confirmed to match how a real client actually delimits these action types on the wire — flagged here as this blueprint's own necessary, named design choice, since without *some* explicit length convention the catch-all cannot safely skip past an unknown action to the next one; a future revision, informed by a real capture, may need to replace this convention with the real one once confirmed). Observable: `inventory.rs` passes.
12. **`chat.rs`, `entity.rs`.** Implement `TextPacket` per §S's three-way body shape; `PlayerListPacket`/`AddPlayerPacket`/`AddActorPacket`/`RemoveActorPacket`/`MoveActorAbsolutePacket`/`SetActorDataPacket` per §S. Observable: `chat_and_entity.rs` passes.
13. **`tests/encryption_seam_plumbing.rs`'s `FakeAead`.** Observable: `encryption_seam_plumbing.rs` passes.
14. **Doctests.** `cargo test --doc -p rc-bedrock-protocol` passes.
15. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
16. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45/D46).** Every file under `crates/bedrock-protocol/tests/` is committed first, alongside `todo!()`-stubbed `src/*.rs` files carrying every already-fixed field/derive/signature from Deliverables. The implementation changeset (Implementation steps) fills in real bodies only — it must not edit a test file, must not add/remove/weaken a test case, in particular the `item_stack_request_opaque_catchall_preserves_unknown_action_bytes_exactly` and `block_network_ids_are_hashes_is_always_true_in_this_blueprints_baseline` cases, both of which pin this blueprint's own explicit design decisions against silent drift.

(b) **No new external dependencies beyond `bytes`/`thiserror`/`tracing`/`flate2` (all already `[workspace.dependencies]`-pinned) plus `proptest` (dev-only, already pinned).** This blueprint adds **zero** new `[workspace.dependencies]` entries (§H) — do not add `snap` (§H's own named non-goal), `bitflags` (§Q's own hand-rolled newtype suffices for this one field), `simdnbt` or `rc-nbt` (§A/§K — this crate's network-NBT codec is hand-rolled), or any other crate not named here.

(c) **No Mojang or third-party reimplementation code (ASSET-D18/D19/D30/CROSS-D27/D29).** Every field layout, packet id, and algorithm in this blueprint is restated in this project's own words from the public/official sources cited in Context §B, live-fetched 2026-08-24 — `mojang.github.io/bedrock-protocol-docs` (EULA-gated per CROSS-D27, consulted, never redistributed) and `wiki.bedrock.dev` — never from GeyserMC/CloudburstMC/gophertunnel source code (CROSS-D29's firewall applies regardless of their permissive licenses; the two `pkg.go.dev`-rendered doc-comment fetches this session made are documented-behavior-only consultations per ASSET-D18(e), not source-code reads), never from any decompiled or leaked Minecraft source.

(d) **Own-authored test fixtures only.** Every byte fixture in every test file is constructed directly from this blueprint's own Context field tables — never extracted from a real packet capture of a Mojang client, never sourced from any third-party project's own test fixtures.

(e) **Dependency-graph discipline (§A).** `rc-bedrock-protocol` must never gain a dependency on `rc-core`, `rc-bedrock-mappings`, `rc-nbt`, `rc-protocol`, `rc-bedrock-raknet`, `rc-bedrock-auth`, `rc-scheduler`, or `rc-mechanics` as part of this blueprint's own changeset — both of CROSS-D5's permitted edges (`rc-core`, `rc-bedrock-mappings`) stay unexercised (§A), exactly as reasoned there; a future blueprint may add either edge if it introduces a genuine need this one does not have.

(f) **Scope boundary — this blueprint does not implement `rc-bedrock-translator`'s own job.** No ECS-ingress event production, no RakNet session wiring (`RaknetListener`/`RaknetSession` calls), no `rusty-clanker-server` composition-root code, no resolution of any raw wire id (`BlockRuntimeId`, `EntityTypeId`, `ItemNetworkId`-shaped fields) against `rc-bedrock-mappings`' own semantic tables, and no `xtask lint-deps` extension naming `rc-bedrock-protocol` explicitly — all of these are a future translator/composition-root blueprint's own scope, named explicitly rather than left implicit, mirroring M11-B01/M11-B03's own identical scope-boundary discipline.

(g) **This blueprint's own resolved gap-filling decisions are flagged, not silently asserted as settled fact.** Three genuine gaps `15-crossplay.md` left open are resolved here for the first time: (i) `StartGamePacket.block_network_ids_are_hashes = true`, with a placeholder, unconfirmed hash function (§M); (ii) on-demand `SubChunkRequest`/`SubChunk` chunk delivery as M11's baseline mode, never legacy full-column delivery (§O); (iii) `PlayerAuthInputPacket` as the sole authoritative movement-input path, `MovePlayerPacket` restricted to server-authoritative correction (§Q). All three are flagged for reconciliation into `15-crossplay.md`'s next revision (Open items, below), mirroring M11-B03's own already-established pattern for its comparable additions (`mojang_root_key_override`, the offline-UUID-derivation extension) — do not present any of the three as an already-ratified `CROSS-D` decision in code comments or documentation this blueprint's implementation changeset produces.

(h) **No `unsafe` code.** Every type and function in this blueprint's deliverables is implementable in 100% safe Rust.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-bedrock-protocol --all-features
cargo nextest run -p rc-bedrock-protocol
cargo test --doc -p rc-bedrock-protocol
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-bedrock-protocol` runs every case named in Acceptance tests — `primitives.rs` (6), `nbt.rs` (4, one a `proptest!`), `batch_and_framing.rs` (6, two `proptest!`), `login_and_handshake.rs` (6), `startgame.rs` (4, one `proptest!`), `catalog.rs` (2), `chunk_and_block.rs` (4), `movement.rs` (4, one `proptest!` covering two cases), `inventory.rs` (4, one `proptest!`), `chat_and_entity.rs` (4, one `proptest!` covering three decode paths), `encryption_seam_plumbing.rs` (1) — all pass, zero flakiness. CI (`.github/workflows/ci.yml`, unmodified by this blueprint) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Open items for a future M11 blueprint

**Interface notes, carried here rather than as a separate section (`00-blueprint-spec.md`'s own 8-section structure has no dedicated slot for them, and folding them into Constraints/Context would scatter what a future author needs as one lookup):**

- **Provides to a future `rc-bedrock-translator` blueprint:** the complete M11 packet catalog (§L–§S) plus the batch/compression/framing infrastructure (§G–§J) as the sole public API surface that crate is expected to consume — `BedrockPacket::encode`/`decode` per concrete type, `encode_batch`/`decode_batch`, `pack_sub_packet`/`unpack_sub_packet`. This blueprint's own raw wire-id newtypes (`BlockRuntimeId`, `EntityTypeId`, `ItemNetworkId`-shaped fields throughout) are exactly what that future crate resolves against `rc-bedrock-mappings`' own semantic tables (M11-B04) — this blueprint never performs that resolution itself (§A/Constraints (f)).
- **Provides to a future composition-root/M11-B01-wiring blueprint:** confirmation that this crate's own output is deployment-topology-agnostic (never assumes monolithic vs. cluster placement, CROSS-D3) — the same neutrality M11-B01's own RakNet layer already established, extended one layer up.
- **Needs from `15-crossplay.md`:** reconciliation of this blueprint's own three gap-filling decisions (Constraints (g)) — `BlockNetworkIdsAreHashes = true`, the on-demand sub-chunk-request chunk-delivery mode, and `PlayerAuthInputPacket`-primary movement authority — into that document's own CROSS-D decision register, exactly the same request M11-B03 already made for its own comparable additions.
- **Needs from a future `rc-bedrock-auth`-integration pass:** independent confirmation (an `ASSET-D30` firewall pass or an `ASSET-D18(c)` packet capture, per M11-B03's own Constraints (c)) of this blueprint's own §I scope decision (whole-batch AEAD wrapping) against a real Bedrock client — this blueprint's own `encryption_seam_plumbing.rs` test proves internal plumbing correctness only, never wire-compatibility.
- **Needs from a future mapping/hash-verification pass:** confirmation or replacement of `hash_block_state`'s own placeholder FNV-1a-64 algorithm (§M/§O) against a real pinned-version (26.44) Bedrock client, per CROSS-D25's manual-verification carve-out — until then, `block_network_ids_are_hashes: true` is internally consistent within this crate's own round-trip tests but not proven wire-compatible.

**Open items:**

- The exact `ItemStackRequestAction::Other` catch-all's own length-delimiting convention (Implementation step 11) is this blueprint's own necessary invention, not a confirmed real-wire fact — a future revision, informed by a real capture of every `Craft*`/`Screen*` action, should replace it with the actual per-action field layouts (decomposing them individually, as the five baseline actions already are) rather than leaving the catch-all as this blueprint's own permanent design.
- `LevelSettings.edu_shared_uri_resource`'s exact field shape, `AddPlayerPacket`'s permission/ability field layout, and `SubChunkResult`'s exact two under-confirmed enum integers (§B) are each individually flagged MEDIUM — each is a one-line constant/struct-shape fix at implementation time once CROSS-D7(b)'s own fresh-capture review confirms them, never a design change.
- Whether a future revision should replace this blueprint's own hand-rolled network-NBT codec (§K) with a `simdnbt`-based one, if `simdnbt`'s own Bedrock-network-mode API is ever independently confirmed to exist and match this crate's own requirements exactly (§A) — left open, not assumed either way.
