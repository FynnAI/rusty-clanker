# M10-B04 — Chat, Combat & Build-Loop Completion

| Field | Content |
|---|---|
| ID | M10-B04 |
| Milestone | M10 — Client Feature Parity: Entities, UI, Isomorphic Mods |
| Prerequisites | M9-B03 (client authentication & connection — this blueprint additively extends `rusty-clanker-client`'s already-shipped `connection::play`'s dispatch loop, `world::{ClientWorld, PlayerState}`, and consumes `rc-msa-auth`'s `AuthSession`/`McAccessToken` exactly as committed to obtain the bearer token a new chat-session fetch needs). M10-B02 (UI framework, text & HUD — this blueprint is the "sibling M10 blueprint owning Play-phase packet decode" that M10-B02 §Interfaces names by name and waits on: it populates `hud::state::HudState`'s public fields/setters from real packet data, drains `gui::chat_screen::ChatScreen::pending_submission`, and consumes `text::component::TextComponent`/`gui::widget::{Screen, HudOverlay, ScreenAction}`/`container` types exactly as committed — no signature from that blueprint is touched). Consulted, not build prerequisites (no new Cargo edge, shape-consistency only, the same distinction M9-B05/M9-B06/M10-B01 already draw for their own consulted-context lists): M10-B01 (entity rendering — this blueprint calls the already-public `rc_render::entity::animation::AnimationState::{trigger_hurt, trigger_attack_swing}` and `crates/client/src/world/entities.rs`'s already-public `ClientEntityStore::apply_animation`/`.get`/`.get_mut`/`.iter` exactly as committed, and additively extends `ClientEntityStore` with one new public method, §Context 9; reuses `crates/render/src/entity/catalog.rs`'s already-public `hitbox_dimensions` for its own client-side entity-reach raycast; never modifies any M10-B01 file's already-committed signature). M9-B06 (camera & movement prediction — this blueprint's block-targeting raycast reuses `crates/client/src/player/shape_source.rs`'s already-public `ClientBlockShapeSource` unmodified, and this blueprint's mouse-click input handling is installed the same way M10-B02's `UiInputRouter` already routes gameplay input, alongside — never inside — M9-B06's own `InputAdapter`/`PredictionSimulation`). M4-B05 (combat & damage — this blueprint restates, client-side, exactly the packet ids/fields and cooldown-charge formula that blueprint fixed server-side: `Interact`, `Set Health`, `Entity Event`, `Combat Death`; §Context 9 also identifies and resolves a real inconsistency between that blueprint's already-shipped server behavior and M10-B01's own assumption about how hurt/death are signaled). M3-B03 (breaking & placing — this blueprint restates, client-side, the dig-timing formula, the `DestroyState` packet lifecycle, and the exact packet ids/fields that blueprint fixed server-side: `Player Action`, `Use Item On`, `Set Block Destroy Stage`, `Level Event`, plus M9-B03's already-shipped `BlockUpdateIn`/`AcknowledgeBlockChangeIn` decode types, reused unmodified). |
| Implements | NET-D3 (hand-written packet types — nineteen new client-side packet structs, restated field-for-field from the server-side originals, the same "duplicated, not shared" pattern M9-B00-index.md's own Cross-blueprint consistency notes already flags and accepts for every Play-state packet); NET-D5 (chat is NBT-encoded on the wire — this blueprint is the first to actually need a wire-form text-component decode, closing NET-D5's own flagged Open Question with a bounded, hand-rolled reader, §Context 3); MECH-D40/D43–D46 (attack-cooldown charge curve and critical-hit gate — restated for a client-local, HUD-only cooldown indicator, never gameplay-authoritative); MECH-D61 (survival dig-timing formula — restated for a client-local, cosmetic-only destroy-progress overlay); MECH-D62/D63 (reach/angle validation — restated for client-side entity-target picking, informational only, never a substitute for the server's own authoritative check); CLIENT-D1 (Tier A/Tier B classification applied concretely to every element this blueprint adds, §Context throughout); CLIENT-D3 (render-pass order — this blueprint's `BlockBreakPass` occupies the fixed "Opaque/Cutout Terrain" neighborhood M9-B04 already established, §Context 6); CLIENT-D19/CLIENT-D26/D29 (this blueprint is the first real production caller of M10-B01's `AnimationState::trigger_hurt`/`trigger_attack_swing`); CLIENT-D23 (GUI framework — `DeathScreen`/`TabListOverlay` are new `Screen`/`HudOverlay` implementations over M10-B02's already-committed widget system, no new UI technology); ASSET-D7/D8 (identity chain — this blueprint's chat-session-key fetch is a new, sibling Mojang API call alongside ASSET-D7's profile fetch and ASSET-D8's serverId-hash join call, reusing the identical bearer-token/custody discipline; a real, cited gap in `08-assets-auth-legal.md`'s own decision register, which names neither a chat-session endpoint nor a decision ID for it — restated here rather than silently invented, §Context 3); TEST-D45/D46 (test-first changeset boundary, protected paths — restated, binding); TEST-D53 (three-tier GPU-testing rule — restated in full, §Context 13, the identical rule M9-B01/M10-B01/M10-B02 already establish and this blueprint inherits rather than re-derives). |
| Crates touched | `rc-msa-auth` (`crates/msa-auth/`) — new `src/chat_session.rs` (pure decode of the chat-session-certificate response); additive `pub mod chat_session;` in `lib.rs`. `rc-render` (`crates/render/`) — new `block_break/` module tree (three files) plus one additive `pub mod block_break;` in `lib.rs`; new `hud/tab_list.rs` plus one additive `pub mod tab_list;` in `hud/mod.rs`; additive `pub tab_list: Option<...>` field on M10-B02's already-committed `hud::state::HudState` (every existing field/method unchanged); new `gui/death_screen.rs` plus one additive `pub mod death_screen;` in `gui/mod.rs`. `rusty-clanker-client` (`crates/client/`) — new `chat/` module tree (three files); new `connection/{text_component_nbt,chat_packets,combat_packets,build_packets,playerlist_packets,lifecycle_packets}.rs`; new `world/{destroy_state,tab_list}.rs`; new `player/{combat,targeting}.rs`; additive re-exports in `connection/mod.rs`; additive fields on M9-B03's already-committed `world::store::ClientWorld` (`ClientWorld` gains four new fields — no existing field/method changed); body-only additive extension of `connection/play.rs`'s steady-state dispatch (new match arms, `run_play`'s signature unchanged, the identical discipline M9-B06 Constraint (b) and M10-B01 §Deliverables already establish for this same file); body-only additive extension of M10-B02's already-committed `ui_input.rs`/`app.rs` (mouse-button routing, window-focus pause, one new `KeyBindings` field); `Cargo.toml` gains two already-workspace-pinned-or-newly-pinned lines (`rsa`, already pinned NET-D6; `sha2`, newly pinned by this blueprint, §Constraints). |
| Estimated scope | L — exceeds the ~800-line Context guideline, flagged explicitly per `blueprints/M10/M10-B01-entity-rendering.md`'s and `blueprints/M10/M10-B02-ui-hud.md`'s own identical precedent for a coherent, non-splittable task: chat signing, combat input, the build loop, death/respawn, sleep, the tab list, and window-focus/pause are the entire remaining "close the M10 play loop" surface criterion 1 needs, and every one of them shares the identical `connection/play.rs` dispatch-loop extension point — splitting any one into its own blueprint would leave that extension point owned by two uncoordinated blueprints racing on the same file, the exact hazard `M9-B00-index.md`'s own Cross-blueprint consistency notes already warns against. |

## Goal & Done definition

Close the M10 milestone's client-side play loop: real signed chat (profile-key acquisition, the message-signing chain, acknowledgement tracking, and clientbound system/player/disguised message decode feeding M10-B02's `ChatLog`); combat input (attack-to-`Interact`-packet, a client-predicted attack-cooldown indicator, local-player damage tilt and remote-entity hurt-flash/death triggers wired into M10-B01's animation system, and `Set Health`-driven HUD updates); the build loop (crosshair block targeting, `Player Action`/`Use Item On` send with sequence tracking, a client-predicted destroy-progress overlay with its own crack-texture render pass, mirroring M3-B03's dig-timing formula); death and respawn screens; the client-side half of sleep/bed stance; a tab-list (player-info) overlay; and window-focus/pause behavior. This blueprint is, by design, the first blueprint to actually drive `connection/play.rs`'s steady-state dispatch loop beyond movement/chunk/entity packets — every packet family named above is a new match arm in that same loop, added once, coherently, rather than by several blueprints racing on the same file.

Done when:

- [ ] `cargo build -p rc-render -p rusty-clanker-client -p rc-msa-auth --all-features` succeeds with zero warnings.
- [ ] Every Tier-1 acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-render -p rusty-clanker-client -p rc-msa-auth`, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), with **zero** test constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` or a real `winit::event_loop::EventLoop`/`Window` (§Context 13's Tier-1 boundary — identical to every prior M9/M10 render/client blueprint's own rule).
- [ ] Every pre-existing M9-B0x/M10-B01/M10-B02 test still passes unmodified — mechanically verified by re-running those suites without touching them.
- [ ] `cargo run -p xtask -- lint-deps` exits 0 — this blueprint's two dependency edges (`rsa`, `sha2`, both on `rusty-clanker-client`) touch no `SIM`/`NETRENDER` boundary rule (client-only crate).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-render -p rusty-clanker-client -p rc-msa-auth` exits 0.
- [ ] `docs/MANUAL-VERIFICATION-M10-B04.md` exists with the content Deliverables specifies.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025`, on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. Scope boundary — what this blueprint does NOT do

- **Inventory/container content sync (`Container Set Content`/`Set Slot`/`Set Cursor Item`), `Set Experience`, `Boss Event`, and scoreboard packets are NOT decoded by this blueprint**, even though M10-B02 §Interfaces lists them alongside the ones this blueprint does own — this project's own task assignment for this blueprint names chat/combat/build/death/sleep/tab-list/window-focus explicitly and does not name inventory content, XP, boss bars, or the scoreboard. These remain open for a future sibling blueprint (this document's own best guess at its eventual id is "M10-B03" or "M10-B06," since neither is written or numbered as of this blueprint's own derivation — restated as a real, named gap, not silently absorbed). **This has one concrete, load-bearing consequence this blueprint restates honestly rather than working around: the client has no way to know the contents of the player's held hotbar slot at M10's own scope** (only which slot is *selected*, via `Set Held Item`, which this blueprint does decode). Every build-loop and combat formula below that would normally depend on the held item (tool material for dig speed, weapon for damage) is therefore bounded to a fixed, flagged default rather than a real per-item lookup, §Context 5/7.
- **`Update Attributes` is NOT decoded** — the client's attack-cooldown indicator (§Context 7) uses a fixed default `AttackSpeed = 4.0` (vanilla's own bare-fist/no-modifier default) rather than a real, server-authoritative attribute value, for the identical held-item-content reason above.
- **Server-side respawn is NOT implemented anywhere in the merged codebase** — M4-B05 §Context "Player death" states this explicitly ("the actual respawn packet round-trip... is explicitly out of scope... a future player-lifecycle blueprint clears `is_dead` and completes the cycle"). This blueprint's own respawn half (§Context 8) is real, complete, and independently tested against a fake server, but is **not exercisable end-to-end against a real Rusty Clanker server** until that future blueprint lands — restated as an honest, actionable gap, never silently assumed closed.
- **Server-side bed/sleep interaction is NOT implemented anywhere in the merged codebase** — M3-B03's own `Use Item On` handler interprets every right-click-on-block as a placement attempt (Context, "Packets"); no merged blueprint special-cases a bed target. This blueprint's own client-side sleep stance (§Context 10) is real and independently tested, but — like respawn — cannot be exercised end-to-end against a real server until a future MECH blueprint adds bed interaction. Restated honestly, not silently assumed.
- **No client-side prediction of block-place or block-break WORLD STATE is implemented** — only the destroy-progress **overlay** (a cosmetic timing aid, §Context 5) is predicted; the actual block appearing/disappearing is always driven by the server's own `Block Update` packet (already decoded by M9-B03, reused unmodified). §Context 5 explains why (the held-item-content gap above makes a correct placement prediction impossible at M10's own scope, and a *wrong* prediction is worse than none).
- **Commands (`/`-prefixed chat) are decoded and sent as ordinary unsigned-argument chat text, never specially parsed or auto-completed** — command argument signing (`ArgumentSignatures`, research doc §3.13) is out of scope; a `/`-prefixed message is sent through the same signed `Chat Message` packet as ordinary chat (vanilla routes it differently server-side, to `Signed Chat Command`, but this blueprint does not implement client-side command-argument extraction — flagged, Open Questions).
- **The MOD-D18 mod-facing bridge is untouched** — nothing in this blueprint is reachable from a mod; M10-B05's own scope, unchanged.
- **Shield blocking, waypoints/locator bar, dialogs (07's 3.14), and full data-component tooltips** are out of scope, matching M4-B05's/M10-B02's own already-stated boundaries — nothing here adds a placeholder for any of them.

### 2. Reused seams, restated concretely

`connection/play.rs`'s steady-state dispatch loop (M9-B03 §11, extended body-only by M9-B06/M10-B01) is a `loop { tokio::select! { raw = conn.recv_raw() => { match raw.id { ... } }, intent = outbound.recv() => { ... } } }` shape — every packet family below adds one new `match` arm to the first branch (decode + apply), and the chat/combat/build outbound sends are triggered from new, additively-installed shared state the second branch's existing per-tick heartbeat already drains once per tick (mirroring M9-B06's own `SharedMotion`-via-`Arc<Mutex<_>>` pattern exactly, §Context 4). `ClientWorld` (M9-B03, extended additively by M10-B01) gains four new fields this blueprint owns: `chat: crate::chat::ChatSessionHandle`, `tab_list: crate::world::tab_list::TabListStore`, `destroy: crate::world::destroy_state::ClientDestroyState`, `combat: crate::player::combat::LocalCombatState` — every one `Default`-constructible, so `ClientWorld::new()`'s body gains four field initializers and nothing else about that already-committed type changes.

`hud::state::HudState` (M10-B02) is written to directly from packet handlers via its already-public fields/setters (`health`, `absorption`, `food`, `saturation`, `selected_slot`, `attack_cooldown`, `set_action_bar`, ...) — this blueprint is the first real producer for the fields M10-B02 built and hand-fed with fixture data. **`hud.hotbar`'s per-slot item CONTENTS are not written by this blueprint** (§Context 1's held-item-content gap) — only `selected_slot` (which slot is active) changes, via `Set Held Item` (§Context 5), so every slot in `hud.hotbar` stays `None` at this blueprint's own scope, rendering as nine empty slots with a correctly-tracked selection outline.

### 3. Chat — signed messages at protocol 776

**Chat-session key acquisition — a real, cited gap in `08-assets-auth-legal.md`'s own decision register.** `08` names ASSET-D7 (profile fetch) and ASSET-D8 (serverId-hash join call) but no decision anywhere in that document's register names the chat-session-certificate endpoint signed chat needs. Vanilla's real client calls `POST https://api.minecraftservices.com/player/certificates` with the same Minecraft-scoped bearer token (`AuthSession.access_token`, M9-B03, reused unmodified — the identical token `skin_fetch.rs`'s HTTP calls already use, M10-B01 §Context 9) and an empty body, receiving a JSON response `{ keyPair: { publicKey: "<PEM>", privateKey: "<PEM>" }, publicKeySignature: "<base64>", publicKeySignatureV2: "<base64>", expiresAt: "<ISO-8601>", refreshedAfter: "<ISO-8601>" }` — **moderate confidence on this exact shape**, sourced from this project's own established public-knowledge restatement discipline (the identical sourcing category M10-B01 §Context 9 already uses for the profile-skins/capes endpoint), flagged for reconciliation against a real captured response during `docs/MANUAL-VERIFICATION-M10-B04.md`'s own pass. `publicKeySignature`/`publicKeySignatureV2` is Mojang's own RSA signature over the public key (plus, for V2, the owning player's UUID and expiry) — proof, verifiable by any server holding Mojang's own well-known signing key, that Mojang vouches for this session key; this blueprint decodes and stores it (to forward to the server, §below) but never itself verifies it (verification is the server's job, per the research doc's own `SignedMessageChain.Decoder.unpack`, §3.13, "requires... a non-expired `ProfilePublicKey`... calls `verify`").

```rust
// crates/msa-auth/src/chat_session.rs (new)

#[derive(Debug, Clone)]
pub struct ChatSessionKeyPair {
    /// PKCS#8 PEM, as returned by the endpoint — decoded to an `rsa::RsaPrivateKey` by the
    /// caller (`rc-msa-auth` never links `rsa` itself, keeping this crate's own dependency
    /// footprint identity-chain-only, per its own already-established boundary, M9-B03
    /// §Context 1's "a new client-only crate... implementing the full... chain" framing —
    /// cryptographic *use* of the key, as opposed to its *custody*, belongs to the caller).
    pub private_key_pem: String,
    pub public_key_pem: String,
    /// Raw signature bytes Mojang attached to `public_key_pem` — forwarded to the server
    /// verbatim inside the `Player Session` packet (§below), never re-derived.
    pub public_key_signature: Vec<u8>,
    pub expires_at: std::time::SystemTime,
}

#[derive(Debug, thiserror::Error)]
pub enum ChatSessionError {
    #[error("missing or malformed field {0:?} in chat-session response")]
    Malformed(&'static str),
    #[error("expiresAt/refreshedAfter is not a valid ISO-8601 timestamp: {0}")]
    BadTimestamp(String),
}

/// Pure decode over the endpoint's own already-parsed JSON body — this function performs no
/// HTTP call itself, mirroring `skin.rs::resolve_skin_property`'s identical "pure decode,
/// caller supplies bytes" split (M10-B01 §Context 9). `public_key_signature` is decoded from
/// its base64 wire form into raw bytes here (the one piece of decode work this function does
/// beyond straight JSON-field extraction).
pub fn parse_chat_session_response(json: &serde_json::Value) -> Result<ChatSessionKeyPair, ChatSessionError>;
```

**Fetch (`rusty-clanker-client`, new `chat/session.rs` — network I/O stays client-side, mirroring `skin_fetch.rs`'s identical boundary):**

```rust
// crates/client/src/chat/session.rs (new)

#[derive(Debug, thiserror::Error)]
pub enum ChatSessionFetchError {
    #[error("network/transport error fetching chat session: {0}")]
    Transport(String),
    #[error("unexpected HTTP status {0}")]
    UnexpectedStatus(u16),
    #[error(transparent)]
    Decode(#[from] rc_msa_auth::chat_session::ChatSessionError),
}

/// One HTTP POST (`reqwest`, already a `rusty-clanker-client` dependency since M10-B01) with
/// `Authorization: Bearer <access_token>` and an empty JSON body, then
/// `rc_msa_auth::chat_session::parse_chat_session_response`. Called once per session, at
/// Play-entry time (Implementation steps), never re-fetched mid-session (the returned key
/// pair's own `expires_at` — server-side signed-chat session lifetime is measured in days,
/// well beyond one play session — is checked only at the next connection attempt, not
/// mid-session, mirroring `AuthSession`'s own "checked at resume time" cadence, M9-B03).
pub async fn fetch_chat_session(access_token: &rc_msa_auth::session::McAccessToken) -> Result<rc_msa_auth::chat_session::ChatSessionKeyPair, ChatSessionFetchError>;
```

**The `Player Session` packet (serverbound, registers the session key with the server the client is joining) — restated from a live fetch of `minecraft.wiki/w/Java_Edition_protocol/Packets` performed while deriving this blueprint (2026-08-24), cross-checked for internal consistency against every id this blueprint's own corpus already fixes independently (every id below that overlaps a prior blueprint's own restated value — `Interact` 0x1A, `Player Action` 0x29, `Use Item On` 0x42, `Set Block Destroy Stage` 0x05, `Entity Event` 0x22, `Set Health` 0x68 — matches that prior blueprint's own value exactly, which this blueprint treats as corroborating evidence for the rest of this table rather than a coincidence). **Moderate confidence on every numeric id below**, the identical caveat class every packet table in this corpus already carries, needing the same one-line reconciliation against a real `reports/packets.json` capture before being treated as final:

| Packet | Bound | ID | Fields (wire order) |
|---|---|---|---|
| `Player Session` | server | `0x0A` | `session_id: u128` (UUID, 16 raw bytes), `expires_at: i64` (epoch millis), `public_key: Vec<u8>` (VarInt-length-prefixed DER, re-encoded from the PEM `ChatSessionKeyPair::public_key_pem` carries — this blueprint's own `rsa`-crate re-encode step, §Deliverables), `key_signature: Vec<u8>` (VarInt-length-prefixed, `public_key_signature` verbatim) |
| `Chat Message` | server | `0x09` | `message: String` (VarInt-length-prefixed UTF-8, max 256 chars), `timestamp: i64` (epoch millis), `salt: i64` (cryptographically random, client-generated per message), `signature: Option<[u8; 256]>` (present flag + 256 raw bytes — `None` only for an unsigned/offline-mode session, §below), `message_count: i32` (VarInt — this session's own count of messages the client has seen acknowledged so far, the `LastSeenMessages.Update`'s own `offset` term restated, §below), `acknowledged: [u8; 3]` (a fixed 20-bit `BitSet`, packed 3 bytes, `LastSeenMessagesValidator`'s own 20-entry acknowledgement window, research doc §3.13/table) |
| `Signed Chat Command` | server | `0x08` | Not sent by this blueprint (§Context 1, "Commands") — restated here only so the id space is complete and a future blueprint that does implement command-argument signing does not need to re-derive this table. |
| `Player Chat Message` | client | `0x41` | `sender: u128` (UUID), `index: i32` (VarInt, `SignedMessageLink.index`), `signature: Option<[u8; 256]>`, `body: String` (VarInt-length-prefixed, the raw, unfiltered content — signed), `timestamp: i64`, `salt: i64`, `previous_messages: Vec<(i32, [u8; 256])>` (VarInt-count-prefixed array of `(message_id, signature)` — the sender's own `LastSeenMessages` snapshot at send time, decoded and discarded by this blueprint, §below), `unsigned_content: Option<TextComponentNbt>` (present only when server-side filtering/decoration replaced the display text — §Context "text_component_nbt.rs"), `filter_type: i32` (VarInt enum, `0`=PassThrough/`1`=FullyFiltered/`2`=PartiallyFiltered — this blueprint never applies `2`'s per-byte mask, treating it identically to `0`, a bounded, flagged simplification since no filter mask consumer exists), `chat_type: i32` (VarInt, registry id into `chat_type`, resolved against a small, hand-authored fallback table, §below, since no live registry-sync exists for this id yet), `sender_name: TextComponentNbt`, `target_name: Option<TextComponentNbt>` |
| `System Chat Message` | client | `0x79` | `content: TextComponentNbt`, `overlay: bool` (`true` = render as an action-bar-style overlay line rather than the scrolling chat log — this blueprint routes `overlay: true` through `HudState::set_action_bar` instead of `ChatLog::push`, §Deliverables) |
| `Disguised Chat Message` | client | `0x20` | `message: TextComponentNbt`, `chat_type: i32` (VarInt), `sender_name: TextComponentNbt`, `target_name: Option<TextComponentNbt>` — used for `/say`/`/me`/disguised-sender messages; this blueprint renders it through the identical decoration path as `Player Chat Message` minus the signature-chain fields (there are none — a disguised message is never signed, matching research doc §3.13's own "system messages are simply unsigned... `sender = NIL_UUID`" framing) |

**Chat-type decoration — a bounded, hand-authored fallback table**, since no registry-sync packet decode exists client-side for `chat_type` (WORLD-D2-adjacent registry data arrives via the `Configuration`-phase registry-data packets M9-B03 already decodes into `ClientRegistryTable`, but that type's own public surface — per M9-B03's own Deliverables — was never asked to expose `chat_type` entries specifically; extending it is a one-field addition a future blueprint may make, not attempted here): this blueprint hand-restates the seven built-in `ChatType` decorations research doc §3.13 already names (`CHAT`, `SAY_COMMAND`, `MSG_COMMAND_INCOMING`/`_OUTGOING`, `TEAM_MSG_COMMAND_INCOMING`/`_OUTGOING`, `EMOTE_COMMAND`) as a fixed Rust match, keyed by the small integer set vanilla's own default datapack assigns them (moderate confidence, flagged) — a message whose `chat_type` id falls outside this fixed set renders with a generic `"<sender>: <content>"` decoration rather than failing, the same "unknown id is tolerated, never fatal" policy this corpus applies everywhere (M9-B03 §Context throughout).

```rust
// crates/client/src/chat/mod.rs (new)
pub mod session;
pub mod signing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatTypeDecoration { Chat, SayCommand, MsgIncoming, MsgOutgoing, TeamMsgIncoming, TeamMsgOutgoing, Emote, Generic }
/// Bounded lookup, §above. Never errors — an unrecognized id resolves to `Generic`.
pub fn resolve_chat_type(registry_id: i32) -> ChatTypeDecoration;
/// Composes the final rendered line from a decoded chat-family packet's parts, per
/// `decoration`'s own template shape (research doc §3.13's own "translation-key template plus
/// which parameter slots... get substituted" restated as a fixed Rust match rather than a
/// data-driven translation-key lookup — this blueprint's own bounded simplification, since no
/// translation-key/lang-file interpreter exists anywhere in this corpus yet, flagged, Open
/// Questions).
pub fn decorate(decoration: ChatTypeDecoration, sender: &rc_render::text::component::TextComponent, target: Option<&rc_render::text::component::TextComponent>, content: &rc_render::text::component::TextComponent) -> rc_render::text::component::TextComponent;
```

**A bounded, hand-rolled network-NBT reader — `text_component_nbt.rs`, a real, necessary interim measure.** `NET-D5` names `simdnbt` as the project's eventual pinned NBT crate, but M9-B03 §Context 12 already found, while deriving that blueprint, that `rc-nbt` (the wrapper crate `12-workspace-structure.md` reserves for it) "is still M0-B01's empty-shell scaffold" — no merged blueprint has actually wired a real, working NBT decoder into this project as of this blueprint's own derivation, and M9-B03 itself chose to store `LevelChunkWithLight`'s own heightmap NBT as opaque, unparsed bytes rather than build a second one. This blueprint cannot make the identical choice — chat is unusable without decoding its own text-component payload — so it hand-rolls a **minimal** network-NBT reader, scoped to exactly the tag kinds a text component's own NBT-compound shape can contain (`TAG_Compound`(10), `TAG_String`(8), `TAG_Byte`(1), `TAG_List`(9), `TAG_End`(0)) rather than a general-purpose NBT library, mirroring M1-B05's own identical precedent of hand-rolling exactly the NBT *writer* it needed for the same reason (M9-B03 §Context 12 cites this exact precedent by name). A future blueprint that lands a real `rc-nbt` should replace this file's reader, mirroring M1-B05's own hand-rolled writer's identical eventual fate — flagged, Open Questions.

```rust
// crates/client/src/connection/text_component_nbt.rs (new)

#[derive(Debug, thiserror::Error)]
pub enum NbtTextComponentError {
    #[error("unexpected NBT tag id {0} at a text-component field position")]
    UnexpectedTag(u8),
    #[error("truncated NBT buffer")]
    Truncated,
    #[error("unrecognized style color name {0:?}")]
    UnknownColor(String),
}

/// A thin, opaque wrapper marking "this byte range is one network-NBT-encoded text
/// component" — the exact type `Player Chat Message`'s `unsigned_content`/`sender_name`/
/// `target_name` fields and `System Chat Message`/`Disguised Chat Message`'s own content
/// fields carry, per the packet table above. Exists so packet-struct definitions (Deliverables)
/// can name a concrete field type without eagerly decoding at parse time (mirroring
/// `rc-protocol`'s own general "typed field, lazy where it can be" convention).
pub struct TextComponentNbt(pub bytes::Bytes);

/// Reads ONE unnamed, root-level NBT-Compound-or-String tag (the network form vanilla's own
/// wire protocol uses for a text component — moderate confidence: this blueprint's own
/// best-effort restatement that the NBT compound's field NAMES (`text`/`translate`/`with`/
/// `color`/`bold`/`italic`/`underlined`/`strikethrough`/`obfuscated`/`font`/`insertion`/
/// `clickEvent`/`hoverEvent`/`extra`) are unchanged from the long-documented JSON form's own
/// key names, since Mojang's own SNBT text-component format is confirmed (minecraft.wiki,
/// ASSET-D18(b)) to use identical key strings to the JSON form, only in NBT tag types instead
/// of JSON value types — `bold`/`italic`/`underlined`/`strikethrough`/`obfuscated` decode as
/// `TAG_Byte` (`0`/`1`), `color` as `TAG_String` (a named color or `"#RRGGBB"` hex), everything
/// else exactly as `rc_render::text::component::parse_json` already interprets those same key
/// names against a JSON `Value` — this function's own body constructs the identical
/// intermediate `serde_json::Value` shape from the NBT bytes and delegates to that
/// already-committed function, so a decode-format correction here never touches M10-B02's own
/// `component.rs`) into `rc_render::text::component::TextComponent`. Bare-string root (a lone
/// `TAG_String` with no compound wrapper — vanilla's own shorthand for `Content::Text` with no
/// styling) is accepted directly, `TextComponent::plain(s)`. **Reconciliation step, flagged**:
/// capture one real chat packet from a live vanilla-adjacent session (or the pinned version's
/// own decompiled jar's `TextComponent.Serializer`/`ComponentSerialization` class, ASSET-D18(f))
/// before treating this function's field-name mapping as final.
pub fn decode_text_component_nbt(bytes: &TextComponentNbt) -> Result<rc_render::text::component::TextComponent, NbtTextComponentError>;
```

**The signing chain — restated exactly from `docs/research/mc-26.2/11-player-gameplay.md` §3.13, with this blueprint's own concrete byte-assembly candidate.** Chain identity is `(sender_uuid, session_uuid, index)`; `index` increments by one per message this client sends in this session (starting `0`), never reset except by a fresh `Player Session` re-registration. `SignedMessageBody(content, timestamp, salt, lastSeen)` is exactly what gets signed — this blueprint's own candidate byte assembly, **moderate confidence, self-consistency-tested only (§below explains why a byte-exact known-answer vector is not attempted)**:

```
bytes = version_marker(1u8, value 1)
      ++ sender_uuid (16 bytes, big-endian)
      ++ session_uuid (16 bytes, big-endian)
      ++ index (i32, big-endian)
      ++ salt (i64, big-endian)
      ++ timestamp_epoch_seconds (i64, big-endian)
      ++ content_len (i32, big-endian) ++ content (UTF-8 bytes)
      ++ last_seen_count (i32, big-endian)
      ++ for each last-seen entry, in order: has_signature(1u8) ++ [signature (256 bytes), if has_signature]
signature = RSA-PKCS#1v1.5-SHA256(private_key, SHA256(bytes))   // Java's SHA256withRSA
```

```rust
// crates/client/src/chat/signing.rs (new)

#[derive(Debug, Clone, Copy)]
pub struct SignedMessageLink { pub sender: u128, pub session: u128, pub index: i32 }

#[derive(Debug, Clone)]
pub struct LastSeenEntry { pub message_id: i32, pub signature: Option<[u8; 256]> }

pub const LAST_SEEN_WINDOW: usize = 20; // research doc §3.13's own `LastSeenMessagesValidator` window size, restated

/// A bounded ring buffer of the last `LAST_SEEN_WINDOW` messages this client has displayed —
/// owned by `ClientWorld.chat` (§Context 2). Feeds both the outgoing `Chat Message` packet's
/// `acknowledged` bitset (§table above) and the byte assembly's own `last_seen` term.
#[derive(Debug, Clone, Default)]
pub struct LastSeenTracker { /* entries: VecDeque<LastSeenEntry>, capacity-bounded */ }
impl LastSeenTracker {
    pub fn new() -> Self;
    /// Called once per accepted `Player Chat Message`/`Disguised Chat Message` this client
    /// receives (never for `System Chat Message`, which carries no signature to track).
    pub fn record_seen(&mut self, id: i32, signature: Option<[u8; 256]>);
    /// The current `(message_count, acknowledged_bitset)` pair this client's NEXT outgoing
    /// `Chat Message` packet must carry.
    pub fn build_acknowledgement(&self) -> (i32, [u8; 3]);
}

#[derive(Debug, thiserror::Error)]
pub enum SigningError {
    #[error("private key is malformed or not a valid PKCS#8 PEM: {0}")]
    BadKey(String),
    #[error(transparent)]
    Rsa(#[from] rsa::Error),
}

#[derive(Debug, Clone)]
pub struct SignedChatMessage { pub timestamp_millis: i64, pub salt: i64, pub signature: [u8; 256] }

/// Owns the parsed `RsaPrivateKey` (decoded once from `ChatSessionKeyPair::private_key_pem` at
/// construction) plus the running `index` counter (§above). One instance per session, owned by
/// `ClientWorld.chat`.
pub struct MessageSigner { /* key: rsa::RsaPrivateKey, sender: u128, session: u128, next_index: i32 */ }
impl MessageSigner {
    pub fn new(key_pair: &rc_msa_auth::chat_session::ChatSessionKeyPair, sender_uuid: u128, session_uuid: u128) -> Result<Self, SigningError>;
    /// Generates a fresh, cryptographically random `salt` (`rand::random::<i64>()` — `rand` is
    /// already an indirect workspace dependency via `rc-mechanics`' own `RcRandom`-adjacent
    /// tooling, but this blueprint uses the OS-backed `rand::rngs::OsRng` path specifically,
    /// never `RcRandom` — chat-message salt is explicitly NOT part of MECH-D5's
    /// seed-determinism contract, the identical "ambient, non-deterministic, non-parity-
    /// relevant randomness" category M4-B05 §Context "Ambient combat RNG" already establishes
    /// for its own unrelated RNG need), assembles the byte sequence above, signs it, and
    /// increments `next_index`. Returns the exact fields the `Chat Message` packet needs.
    pub fn sign(&mut self, content: &str, last_seen: &LastSeenTracker) -> Result<SignedChatMessage, SigningError>;
    /// The current `SignedMessageLink` (post-increment `index` from the most recent `sign`
    /// call, or `index: 0` if `sign` has never been called) — informational, for tests/logging.
    pub fn link(&self) -> SignedMessageLink;
}
```

**Insecure/offline fallback** (research doc §3.13, "`SignedMessageChain.Decoder.unsigned`"): if `fetch_chat_session` fails (network error, or the connecting account has no valid profile key — never expected for a real Microsoft-account session, but a real, handled failure mode) `ClientWorld.chat` holds `ChatSessionHandle::Unsigned` instead of a `MessageSigner` — outgoing `Chat Message` packets are still sent, with `signature: None` and `salt: 0` (vanilla's own documented unsigned-fallback shape) — the server-side acceptance of this depends on that server's own `enforce-secure-profile` setting (research doc line 137), entirely outside this blueprint's control; this blueprint's own job is only to send the correct, honestly-unsigned shape, never to fabricate a signature it cannot produce.

```rust
#[derive(Debug)]
pub enum ChatSessionHandle { Signed(Box<crate::chat::signing::MessageSigner>), Unsigned }
impl Default for ChatSessionHandle { fn default() -> Self { ChatSessionHandle::Unsigned } }
```

**No chat preview.** Research doc §3.13's own closing line is restated as this blueprint's own binding constraint: "there is no server-side preview/decoration round-trip before send" — `gui::chat_screen::ChatScreen` (M10-B02, already committed) composes locally and submits once; this blueprint's own network-layer job is exactly "drain `pending_submission`, sign, send" with no intermediate round trip of any kind.

### 4. Combat — attack input, cooldown, hurt/death, health

**Attack input → packets.** A left-click while `CaptureMode::Gameplay` (M10-B02 §Context 16) is held sends, unconditionally, one serverbound "swing" signal (§below) and, additionally, one `Interact{interaction_type: Attack}` packet **iff** this blueprint's own client-side entity-target raycast (§below) currently resolves a target within reach — mirroring vanilla's own real behavior (an empty-air left-click still swings the arm, visible to other players once a future server blueprint broadcasts it, §Context 9, but sends no `Interact` at all).

```rust
// crates/client/src/connection/combat_packets.rs (new)

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x1A)]
pub struct InteractOut {
    #[rc(varint)] pub entity_id: i32,
    #[rc(varint)] pub interaction_type: i32, // this blueprint only ever sends 1 (Attack)
    #[rc(varint)] pub hand: i32,             // 0 (main hand) — this blueprint never attacks off-hand
    pub sneaking: bool,
}
/// Restated verbatim from M4-B05's own packet table (Context, "Packets") — client encode
/// direction, this blueprint's own first real construction of this packet.
pub const INTERACT_ATTACK: i32 = 1;

/// The serverbound "swing arm" signal — restated, flagged forward, exactly per M10-B01
/// §Context 2's own identical framing: **no merged server blueprint sends the matching
/// clientbound `Entity Animation` broadcast to other viewers yet** (M4-B05's own scope was
/// damage math, not swing-visibility broadcast; this blueprint's own send call therefore has
/// no server-side counterpart to exercise against a real Rusty Clanker server until a future
/// blueprint adds one — restated honestly, not silently assumed working). Sent on EVERY
/// left-click, whether or not `Interact` also fires this same click.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x36)]
pub struct SwingArmOut { #[rc(varint)] pub hand: i32 } // moderate confidence on id — new packet, no prior blueprint fixes it

/// Restated verbatim from M4-B05's own packet table. `entity_id` is a plain `i32` (**not**
/// VarInt — the real, cited asymmetry M4-B05 §Context "Packets" already flags and this
/// blueprint restates unchanged).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x22)]
pub struct EntityEventIn { pub entity_id: i32, pub event_id: u8 }
pub const ENTITY_EVENT_HURT: u8 = 2;
pub const ENTITY_EVENT_DEATH: u8 = 3;
```

**Client-side entity-target raycast — restated independently from `rc-mechanics`' own server-side twin (M4-B05, `raycast_entity_reach`/`entity_dimensions`), since `rc-mechanics` is not a crate this blueprint may depend on (CLIENT-D25's shared-crate list names only `rc-protocol`/`rc-physics`/`rc-registries`/`rc-mechanics`'s `client-predict` feature — plain entity-target raycasting is not part of that feature's own scope).** Informational only — this raycast decides which `entity_id` (if any) the client's own `Interact` packet names; the server independently re-validates reach/angle per MECH-D62 unmodified, and a client-side false-positive (predicting a hit the server rejects) simply produces an `Interact` packet the server silently no-ops, per M4-B05's own already-established "reject is ack-only, no packet response" behavior — never a client-visible error, matching CLIENT-D28's own broader "client is not authoritative" framing extended here.

```rust
// crates/client/src/player/combat.rs (new)

pub const ENTITY_INTERACTION_RANGE: f64 = 3.0; // restated verbatim, M4-B05 §Context "Reach and angle validation"

/// Slab-method ray/AABB test against each `TrackedEntity`'s own hitbox (built from M10-B01's
/// already-public `rc_render::entity::catalog::hitbox_dimensions`, centered on the entity's
/// OWN interpolated render position, `InterpolationBuffer::sample_at` — never its last raw
/// network sample, so the target the player visually sees is exactly what this function tests
/// against). Returns the closest hit's `network_id`, or `None`. Excludes the local player's own
/// entity id trivially (the local player is never itself a `TrackedEntity`, M10-B01 §Context 10).
pub fn pick_entity_target(
    origin: glam::DVec3,
    direction: glam::Vec3,
    entities: &crate::world::entities::ClientEntityStore,
    current_tick: u64,
    partial_ticks: f32,
) -> Option<i32>;

/// Client-local mirror of M4-B05's own attack-cooldown charge curve (Context, "Attack-cooldown
/// charge curve") — HUD-indicator use only, never gameplay-authoritative (§Context 1's held-item
/// gap: `attack_speed` is always `DEFAULT_ATTACK_SPEED`, since `Update Attributes` is not
/// decoded). `ticker` increments once per client tick, resets to `0` on every local attack
/// input (§below) — mirrors the server's own `attack_strength_ticker` reset-on-`on_attack()`
/// rule, M4-B05 §Context "Attack-cooldown charge curve," applied client-side for the sole
/// purpose of driving `HudState.attack_cooldown` a full tick sooner than a server round-trip
/// could.
pub const DEFAULT_ATTACK_SPEED: f64 = 4.0;
/// The crosshair-indicator variant uses `offset = 0.0` (not damage-scaling's own `0.5`) — a
/// real, documented distinction between vanilla's `getAttackStrengthScale(0.0F)` (HUD) and
/// `getAttackStrengthScale(0.5F)` (damage math), moderate confidence, restated here since M4-B05
/// only ever needed the `0.5` variant.
pub fn attack_cooldown_indicator(ticker: u32, attack_speed: f64) -> f32;

#[derive(Debug, Clone, Default)]
pub struct LocalCombatState { pub attack_ticker: u32, pub damage_tilt: crate::player::combat::DamageTiltState }
impl LocalCombatState {
    /// Advances `attack_ticker` by one (saturating — never wraps) — called once per client tick.
    pub fn advance_tick(&mut self);
    /// Resets `attack_ticker` to `0` — called on every local left-click, whether or not it
    /// produced an `Interact` packet (matches the server's own identical reset-on-every-swing
    /// rule, not reset-only-on-hit).
    pub fn on_local_attack(&mut self);
}

pub const DAMAGE_TILT_TICKS: u8 = 10; // reuses M10-B01's `HURT_FLASH_TICKS` value verbatim, cited
#[derive(Debug, Clone, Copy, Default)]
pub struct DamageTiltState { pub ticks_remaining: u8 }
impl DamageTiltState {
    pub fn trigger(&mut self); // idempotent re-trigger: resets to DAMAGE_TILT_TICKS, mirrors M10-B01's own trigger_hurt rule
    pub fn advance_tick(&mut self);
    /// `0.0..=1.0`, linear decay — the local player's own screen-space damage-flash/tilt
    /// intensity a future render-integration blueprint consumes (§Interfaces — this blueprint
    /// declares the state and its trigger conditions, never a shader or camera-shake effect
    /// itself, the identical "declare the seam, a not-yet-written composition-root blueprint
    /// wires it" pattern M9-B04/M9-B05/M9-B06/M10-B01 all already establish for their own
    /// GPU-facing facades).
    pub fn intensity(&self) -> f32;
}
```

**Health updates and the local damage-tilt trigger — `Set Health` (client, `0x68`), restated verbatim from M4-B05's own packet table.**

```rust
// crates/client/src/connection/lifecycle_packets.rs (new)
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x68)]
pub struct SetHealthIn { pub health: f32, #[rc(varint)] pub food: i32, pub saturation: f32 }
```

Handler (§Deliverables `connection/play.rs`): writes `hud.health = pkt.health; hud.food = pkt.food.clamp(0,20) as u8; hud.saturation = pkt.saturation;` into `HudState` (M10-B02's already-public fields) and, **iff** `pkt.health < previous_health` (the previous packet's own value, tracked in `LocalCombatState`, a new field this blueprint adds to that same struct — not `ClientWorld` directly, keeping every combat-adjacent local-only field in one place), calls `combat.damage_tilt.trigger()` — the sole, always-correct signal for the local player's own damage tilt, since `Set Health` is, per M4-B05's own text, always sent to "the entity's own owning player," unlike `Set Entity Data`/`Entity Event`, which are ambiguous about self-delivery (this blueprint does not rely on `EntityEventIn` ever naming the local player's own `entity_id`, even though it may in practice — §Context 9's own reconciliation note covers the ambiguous case defensively, not as this trigger's primary path).

**Remote-entity hurt/death — the first real production caller of M10-B01's `AnimationState`, and a real reconciliation, §Context 9 below.**

### 5. The build loop — targeting, place/break, destroy-progress overlay

**Selected hotbar slot — `Set Held Item`, restated per §Context 3's own live fetch (moderate confidence).** The one piece of hotbar state this blueprint DOES track (§Context 1: slot contents remain unknown, but which slot is active is real, small, and directly gates every `Use Item On`/`Player Action` this blueprint sends — vanilla always places/breaks with whatever the currently-selected slot holds).

```rust
// crates/client/src/connection/lifecycle_packets.rs (new, continued)
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x69)]
pub struct SetHeldItemIn { pub slot: u8 } // 0..=8
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x35)]
pub struct SetHeldItemOut { pub slot: u16 } // scroll-wheel/number-key change; this blueprint sends this on the same input M10-B02's own `KeyBindings` numeric-slot-select bindings would drive, once that sibling blueprint's own input wiring exists — until then, sent only in response to a scroll-wheel delta this blueprint's own `GameplayMouseRouter` reads directly (a small, additive extension of that same type, not a new one)
```

Handler: `SetHeldItemIn` writes `hud.selected_slot = pkt.slot` (M10-B02's already-public field) — the sole `HudState` write this packet performs, since slot contents stay unknown (§Context 1).

**Crosshair block targeting — reuses M9-B06's already-public `ClientBlockShapeSource` and M3-B03's own `rc_physics::cast_ray` (a `rc-physics`, i.e. shared-crate, function — no restatement needed, unlike the entity raycast above).**

```rust
// crates/client/src/player/targeting.rs (new)

pub const BLOCK_INTERACTION_RANGE: f64 = 4.5; // Player.DEFAULT_BLOCK_INTERACTION_RANGE, research doc §4 "Key types" table, restated

/// The six-value vanilla face/direction ordinal (`Down=0, Up=1, North=2, South=3, West=4,
/// East=5`) `Player Action`'s/`Use Item On`'s own `direction` VarInt field carries — restated
/// client-side, independently, since the project's own `Direction` type (`rc_mechanics`,
/// M2-B07/M3-B03's own `Face::from_ordinal`) lives in a crate this blueprint may not depend on
/// (CLIENT-D25's shared-crate list does not name `rc-mechanics` outside its own narrow
/// `client-predict` feature, which does not cover this enum) — the identical "restate the small
/// shared shape across a forbidden crate boundary" pattern M9-B03/M9-B06/M10-B01 already use
/// for their own ~20 duplicated packet structs, applied here to one six-value enum instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFace { Down, Up, North, South, West, East }
impl BlockFace {
    pub fn to_ordinal(self) -> i32;
    pub fn from_ordinal(v: i32) -> Option<BlockFace>;
    /// The unit normal this face points along — used to derive the placement position
    /// (`target.pos + face.normal()`) and the cursor-relative orientation math a future
    /// blueprint's real placement-rotation logic would need (not computed by this blueprint,
    /// §Context 1 — placement always uses the held item's own default orientation, since no
    /// real held-item content exists client-side to orient in the first place).
    pub fn normal(self) -> rc_core::BlockPos;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockTarget { pub pos: rc_core::BlockPos, pub face: BlockFace, pub hit_point: glam::DVec3 }

/// `origin`/`direction` from the player's own eye position/look vector (the identical
/// construction `M9-B06`'s own `PlayerController` already derives for its camera, restated here
/// independently rather than reaching into that blueprint's own private fields — this function
/// takes the two vectors directly, a plain, dependency-free signature). Wraps
/// `rc_physics::cast_ray` (M3-B03, unmodified) against a `crate::player::shape_source::
/// ClientBlockShapeSource` built from the caller's own `&ClientWorld` reference (M9-B06,
/// unmodified) — the exact same shape-source type M9-B06's own movement collision already uses,
/// reused here for a second, independent purpose (raycasting instead of sweep-collision),
/// exactly the kind of "one shared shape source, two consumers" reuse `rc-physics`'s own
/// `BlockShapeSource` trait exists to enable.
pub fn pick_block_target(
    origin: glam::DVec3,
    direction: glam::Vec3,
    world: &crate::world::ClientWorld,
) -> Option<BlockTarget>;
```

**Place/break packets — restated verbatim from M3-B03's own packet table (Context, "Packet layout — corrected and new"):**

```rust
// crates/client/src/connection/build_packets.rs (new)

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x29)]
pub struct PlayerActionOut { #[rc(varint)] pub status: i32, pub location: i64, #[rc(varint)] pub direction: i32, #[rc(varint)] pub sequence: i32 }
pub const ACTION_START_DESTROY: i32 = 0;
pub const ACTION_ABORT_DESTROY: i32 = 1;
pub const ACTION_STOP_DESTROY: i32 = 2;

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x42)]
pub struct UseItemOnOut {
    #[rc(varint)] pub hand: i32, pub location: i64, #[rc(varint)] pub direction: i32,
    pub cursor_x: f32, pub cursor_y: f32, pub cursor_z: f32, pub inside_block: bool,
    #[rc(varint)] pub sequence: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x05)]
pub struct SetBlockDestroyStageIn { #[rc(varint)] pub entity_id: i32, pub location: i64, pub destroy_stage: i8 }

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x2E)]
pub struct LevelEventIn { pub event_id: i32, pub location: i64, pub data: i32 } // event_id plain Int, restated
pub const LEVEL_EVENT_BLOCK_BREAK: i32 = 2001; // restated verbatim, M3-B03

/// A monotonic per-connection counter — `Player Action`/`Use Item On` both consume one value
/// per send (never shared/interleaved incorrectly — a single counter serves both packet kinds,
/// matching vanilla's own single `BlockStatePredictionHandler` sequence space).
#[derive(Debug, Default)]
pub struct BlockActionSequencer { next: i32 }
impl BlockActionSequencer {
    pub fn new() -> Self; // starts at 1 — 0 is reserved/never sent, matching vanilla's own convention
    pub fn next_sequence(&mut self) -> i32;
}
```

`Acknowledge Block Change` (`AcknowledgeBlockChangeIn`, M9-B03, already decoded — that blueprint's own Constraint (f) states it is only "tolerantly logged," never acted on). This blueprint upgrades that handling **additively** (body-only, `AcknowledgeBlockChangeIn`'s own struct shape unchanged): its `sequence` field is compared against `ClientDestroyState`'s own `last_sent_sequence` purely as a liveness/ordering check (a mismatch is logged at `warn`, never a panic or a state rollback) — restated honestly as bounded: **no real corrective rollback exists**, because §Context 1 already establishes this blueprint predicts no world state to roll back (only the cosmetic overlay, below) — a future blueprint that adds real held-item-aware placement prediction is what would give this field's arrival a genuine corrective role, mirroring vanilla's own real `BlockStatePredictionHandler`.

**Destroy-progress overlay — a client-local, cosmetic prediction, mirroring M3-B03's own dig-timing formula exactly, bounded to a bare-hand assumption (§Context 1).**

```rust
// crates/client/src/world/destroy_state.rs (new)

/// Restated verbatim from M3-B03's own tier-1 block table (Context, "Tier-1 block table") —
/// the ONLY blocks this blueprint's own overlay times correctly; any other block falls back to
/// `FALLBACK_HARDNESS` (§below). `rc-registries` exposes no hardness data client-side as of this
/// blueprint's own derivation (confirmed absent, `M9-B05`'s own additive-codegen precedent was
/// checked and does not cover it) — restating this small, already-server-hand-authored table a
/// second time, rather than plumbing a new registry-codegen field, is this blueprint's own
/// bounded, cited choice (identical in kind to every other "restate across the crate boundary"
/// case this corpus already accepts, e.g. M9-B03/M9-B06's ~20 duplicated packet structs).
pub fn block_hardness(block_state_id: u32) -> Option<f32>; // keyed by the same tier-1 block-state ids M3-B03 itself resolves; None for anything outside that table
pub const FALLBACK_HARDNESS: f32 = 1.5; // Stone's own value — this blueprint's own reasonable, flagged default for every other block

/// `speed_for_local_prediction()` is ALWAYS the bare-hand case (`ToolMaterial::None`,
/// multiplier `1`, never "effective," divisor always `100.0` unless `hardness == 0.0`) — §Context
/// 1's held-item-content gap makes anything else impossible to compute correctly; this is a
/// deliberate UNDER-estimate in the common case (a real tool digs faster than this overlay
/// predicts), self-correcting the instant the server's own `Block Update` actually removes the
/// block (§below) — restated as a real, bounded, Tier-B (cosmetic-only) simplification, never a
/// gameplay-authoritative timer.
pub fn ticks_to_break_predicted(hardness: f32) -> u32; // `ceil(hardness * 100.0)` per M3-B03's own general-case formula with speed=1, divisor=100 (no-correct-tool branch)

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ClientDestroyState {
    pub target: Option<rc_core::BlockPos>,
    pub start_tick: u64,
    pub current_stage: i8, // -1 = no overlay
    pub last_sent_sequence: i32,
}
impl ClientDestroyState {
    /// Called when the held mouse button targets a NEW block (or the same block for the first
    /// time this hold) — resets `start_tick`/`current_stage`, does not itself send a packet
    /// (the caller sends `PlayerActionOut{status: ACTION_START_DESTROY, ..}` separately, this
    /// method only tracks the resulting local timing state).
    pub fn begin(&mut self, pos: rc_core::BlockPos, current_tick: u64, sequence: i32);
    /// Called once per client tick while a destroy is active — recomputes `current_stage` from
    /// `ticks_to_break_predicted` and the elapsed tick count, per the identical
    /// `floor(progress * 10.0).clamp(0, 9)` formula M3-B03 §Context "Dig packet lifecycle"
    /// already fixes server-side (restated, not re-derived).
    pub fn advance_tick(&mut self, block_state_id: u32, current_tick: u64);
    /// Clears the overlay — called on mouse release (before the block finishes, sends
    /// `ACTION_ABORT_DESTROY`), on crosshair leaving the target block, or (unconditionally, no
    /// packet needed) once a `Block Update`/`Level Event{LEVEL_EVENT_BLOCK_BREAK}` for the SAME
    /// position arrives — the server's own real completion signal always wins over this
    /// blueprint's own local prediction, matching §Context 1's "never a substitute for the
    /// server's own authoritative outcome" framing.
    pub fn clear(&mut self);
}
```

**The block-crack render seam — `rc-render`'s new `block_break/` module, closing the "block-crack rendering seam to M9-B05" item this blueprint's task explicitly names.** M9-B05 itself builds no destroy-overlay content of any kind (confirmed absent from that blueprint's own text) — this is real, new, from-scratch content, not an extension of an existing M9-B05 file.

```rust
// crates/render/src/block_break/mod.rs (new)
pub mod overlay;
pub mod texture;
pub mod pass;

// crates/render/src/block_break/overlay.rs (new) — pure, Tier-1-testable
/// Reuses M9-B04's own terrain `Vertex` format verbatim (§header, "packed 16-byte vertex") —
/// the overlay draws as an ordinary, slightly-inflated unit cube at the targeted block's
/// section-local position, textured via the destroy-stage layer instead of a terrain material
/// layer, so it needs no new vertex format of its own.
pub fn destroy_overlay_mesh(local_pos: (u8, u8, u8), stage: u8, texture_layer: u16) -> (Vec<crate::vertex::Vertex>, Vec<u32>);
pub const OVERLAY_INFLATE: f32 = 1.0 / 256.0; // avoids z-fighting against the real block face beneath it, moderate confidence, flagged

// crates/render/src/block_break/texture.rs (new)
/// The ten `textures/block/destroy_stage_0.png`..`destroy_stage_9.png` local-install textures
/// (vanilla-standard naming, ASSET-D18(b)-class public knowledge — these are ordinary local
/// `.minecraft` assets, loaded via `rc_assets::store::AssetStore::load_texture` at runtime,
/// never bundled, identical custody stance to every other texture this corpus loads) packed
/// into one small `texture_2d_array<f32>`, mirroring M10-B01's own `EntityTextureBuilder`
/// pad-into-canvas pattern exactly.
pub struct DestroyStageTextureData { /* 10-layer rgba8 pixel data, tile size */ }
pub struct DestroyStageTextureBuilder;
impl DestroyStageTextureBuilder {
    /// Pure CPU-side build, Tier-1-testable.
    pub fn build(store: &mut rc_assets::store::AssetStore) -> Result<DestroyStageTextureData, rc_assets::store::LoadError>;
    /// Real-GPU — untested in Tier 1 (§Context 13).
    pub fn upload(data: &DestroyStageTextureData, device: &wgpu::Device, queue: &wgpu::Queue) -> DestroyStageTextureArray;
}
pub struct DestroyStageTextureArray { /* opaque GPU handle */ }

// crates/render/src/block_break/pass.rs (new)
/// Occupies CLIENT-D3's fixed pass order in the same neighborhood as M9-B04's own Cutout
/// Terrain pass (drawn immediately after it, sharing the same depth attachment, `LoadOp::Load`
/// on both color and depth — never clearing) — restated per M9-B04 §Context 5's own "fixed,
/// directly-coded sequence, no general DAG executor yet" simplification, identical framing to
/// M10-B01's own `EntityPass` placement. **Not wired into `rusty-clanker-client`'s `Shell` by
/// this blueprint** — the same still-open composition-root gap M9-B04/M9-B05/M9-B06/M10-B01
/// each already flag identically (§Interfaces).
pub struct BlockBreakPass { /* pipeline, camera bind group, per-frame overlay vertex buffer */ }
impl BlockBreakPass {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, texture: &DestroyStageTextureArray) -> Self;
    pub fn update_camera(&mut self, queue: &wgpu::Queue, camera: &rc_render::camera::Camera);
    /// `overlay: Option<(BlockPos, u8)>` — `None` draws nothing (no destroy active this frame).
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, color_target: &wgpu::TextureView, depth_target: &wgpu::TextureView, origin: rc_render::camera::RenderOrigin, overlay: Option<(rc_core::BlockPos, u8)>) -> Result<(), rc_render::renderer::RenderError>;
}
```

### 6. Sending loop — how input becomes the packets above

`crates/client/src/ui_input.rs` (M10-B02, body-only additive extension — `UiInputRouter`'s own public surface unchanged): under `CaptureMode::Gameplay`, a `WindowEvent::MouseInput{button: Left, state: Pressed}` now, in addition to whatever M10-B02 already does (nothing, at that blueprint's own scope — confirmed, M10-B02 names no mouse-click gameplay handling anywhere in its own text), calls into a new, small per-`Shell` owned type this blueprint adds:

```rust
// crates/client/src/player/combat.rs (new, continued)

/// The gameplay mouse-input dispatcher — owns nothing GPU/network-shaped itself, only decides
/// WHAT to queue into `ClientWorld`'s shared state for the network task's own per-tick drain
/// (§Context 2) to pick up, mirroring `SharedMotion`'s identical "written by Shell's tick loop,
/// read by the network task" cross-thread shape (M9-B06 §Context 1).
pub struct GameplayMouseRouter { /* left_held: bool, right_held: bool */ }
impl GameplayMouseRouter {
    pub fn new() -> Self;
    /// `Left` press: queues a `PendingSwing` (always) and, if `pick_entity_target` resolves,
    /// a `PendingInteractAttack{target}`; also begins/continues `ClientDestroyState` tracking
    /// if `pick_block_target` resolves and no entity target took priority (vanilla's own real
    /// tie-break: an entity target under the crosshair is attacked, never mined-through).
    /// `Left` release: if a destroy is active, queues `PendingPlayerAction{ACTION_STOP_DESTROY}`.
    /// `Right` press: queues `PendingUseItemOn{target}` if `pick_block_target` resolves.
    pub fn on_mouse_button(&mut self, button: winit::event::MouseButton, pressed: bool, world: &crate::world::ClientWorld, look: (glam::DVec3, glam::Vec3));
    /// Called once per client tick — re-evaluates an in-progress destroy (crosshair moved off
    /// target -> queues `ACTION_ABORT_DESTROY`) and drains queued actions into `ClientWorld`'s
    /// shared `PendingActions` for the network task to send (§below).
    pub fn advance_tick(&mut self, world: &mut crate::world::ClientWorld, look: (glam::DVec3, glam::Vec3));
}

/// The cross-thread queue `connection/play.rs`'s outbound drain (§Context 2) consumes once per
/// tick heartbeat — a plain `Vec`-backed queue behind `ClientWorld`'s own existing mutex (no
/// second `Arc<Mutex<_>>`, unlike `SharedMotion` — this state has no render-thread reader, only
/// a tick-thread writer and a network-thread drainer, so `ClientWorld`'s own already-shared
/// handle is sufficient).
#[derive(Debug, Clone, Default)]
pub struct PendingActions {
    pub swings: u32,                         // count, not bool — multiple swings can queue between two network heartbeats under a fast click
    pub interact_attacks: Vec<i32>,           // target network_ids
    pub player_actions: Vec<(i32, rc_core::BlockPos, crate::player::targeting::BlockFace)>, // (status, pos, face)
    pub use_item_on: Vec<(rc_core::BlockPos, crate::player::targeting::BlockFace, glam::Vec3, bool)>, // (pos, face, cursor, inside_block)
}
```

`connection/play.rs`'s outbound drain (§Context 2's already-described `tokio::select!` branch, extended body-only): on every `OutboundIntent` heartbeat, locks `ClientWorld`, takes (`std::mem::take`) `world.combat.pending` (a new field this blueprint adds alongside `LocalCombatState`'s own fields — grouping both under the same lock acquisition M9-B06's own movement drain already performs), and for each queued item constructs and sends the matching packet from §Context 4/5 above, assigning a fresh `sequence` from `world.destroy.sequencer` (a new field, `BlockActionSequencer`) for every `PlayerActionOut`/`UseItemOnOut` — the identical "heartbeat drains shared state" shape M9-B06 §Context "connection/play.rs" already establishes, extended to a second kind of shared state (discrete action queues, not continuous motion) rather than duplicating the loop.

### 7. Death & respawn

**`Combat Death` (client) — restated verbatim from M4-B05's own `Player Combat Kill` packet, renamed to this blueprint's own live-fetch-confirmed current name (§Context 3's own fetch, cross-checked: `0x44` matches M4-B05's `Player Combat Kill` id exactly, corroborating both as the same packet under two names across this corpus's own research vintage — restated once, under the current name, per NET-D2's "single pinned version" discipline).**

```rust
// crates/client/src/connection/lifecycle_packets.rs (new, continued)
#[derive(RcPacket, Debug, Clone)]
#[packet(state = "play", bound = "client", id = 0x44)]
pub struct CombatDeathIn { #[rc(varint)] pub player_id: i32, pub message: crate::connection::text_component_nbt::TextComponentNbt }

/// `Client Command` — restated per §Context 3's own live fetch.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x0C)]
pub struct ClientCommandOut { #[rc(varint)] pub action_id: i32 }
pub const CLIENT_COMMAND_RESPAWN: i32 = 0;

#[derive(RcPacket, Debug, Clone)]
#[packet(state = "play", bound = "client", id = 0x52)]
pub struct RespawnIn {
    pub dimension_type: String, pub dimension_name: String, pub seed: i64,
    pub game_mode: u8, pub previous_game_mode: i8, pub is_debug: bool, pub is_flat: bool,
    pub copy_metadata: bool, pub has_death_location: bool,
    // [if has_death_location: death_dimension: String, death_position: i64]
}
```

**Handler.** `CombatDeathIn` (only ever meaningful when `pkt.player_id == world.player.entity_id`, M9-B03's already-decoded `PlayerState.entity_id` — a `player_id` naming any other id is logged and dropped, since this blueprint tracks no other player's own death state beyond what `EntityEventIn{event_id: 3}` already drives through M10-B01's `ClientEntityStore`, §Context 9): opens `rc_render::gui::death_screen::DeathScreen::new(decode_text_component_nbt(&pkt.message)?)` via `UiInputRouter::open_screen` (M10-B02, already-public) and sets a new `world.combat.is_dead = true` flag (movement/interaction input is suppressed while set — a small, additive guard this blueprint adds to `GameplayMouseRouter`/the existing movement-intent path, mirroring M4-B05's own server-side identical `is_dead`-gates-further-input framing, restated client-side). The death screen's own "Respawn" button (`DeathScreen::take_action() == DeathScreenAction::Respawn`) sends `ClientCommandOut{action_id: CLIENT_COMMAND_RESPAWN}` and closes the screen; `RespawnIn`, on arrival, clears `world.combat.is_dead`, resets `world.entities` (M10-B01's `ClientEntityStore::new()` — every previously-tracked entity is stale post-respawn, a fresh dimension may have different ones) and `world.chunks` (a fresh dimension needs fresh chunk data, matching vanilla's own real "respawn implies a full re-send" behavior), and updates `world.player.dimension_name`/`.is_flat` from the packet's own fields (`PlayerState`, M9-B03, already carries both — additive body-only writes, no new field).

```rust
// crates/render/src/gui/death_screen.rs (new)
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DeathScreenAction { None, Respawn, TitleScreen }
pub struct DeathScreen { /* message: crate::text::component::TextComponent, last_action: DeathScreenAction */ }
impl DeathScreen {
    pub fn new(message: crate::text::component::TextComponent) -> Self;
    pub fn take_action(&mut self) -> DeathScreenAction; // drains, resets to None
}
impl super::widget::Screen for DeathScreen {
    fn layout(&mut self, viewport_px: (u32, u32), gui_scale: u32) -> super::widget::Widget; // "You Died!" title (research doc §3.16's own death-message text as body) + Respawn/Title-Screen buttons
    fn on_ui_event(&mut self, event: &super::widget::UiEvent) -> super::widget::ScreenResponse;
    fn can_close_with_escape(&self) -> bool { false } // vanilla's own real behavior — Escape does not dismiss the death screen
}
```

**The honest gap, restated once more, concretely:** since no merged server blueprint sends `RespawnIn` (§Context 1), this blueprint's own `DeathScreen`→`ClientCommandOut`→(awaited)`RespawnIn` round trip is exercised in this blueprint's own acceptance tests only against a **fake** server harness (§Acceptance tests) that plays the server's own missing half for the test's sake — never against a real `rusty-clanker-server` subprocess, which would currently hang after `ClientCommandOut` is sent. `docs/MANUAL-VERIFICATION-M10-B04.md` states this limitation explicitly rather than attempting to demonstrate it end-to-end.

### 8. Sleep / bed stance — 07's own tier, checked and resolved here

`07-client-architecture.md` names no sleep-specific decision anywhere in its own Decisions table or Open Questions — the task's own "(check)" instruction is answered here: **07 does not tier sleep at all; this blueprint resolves the tier itself**, per CLIENT-D1's own general framework. The "X/Y players sleeping" / "Skipping night" announcement (research doc §3.12, `announceSleepStatus`) is an ordinary `System Chat Message` (§Context 3's already-built pipeline handles it with zero new code) — **Tier A**, since a player reads it to decide whether to keep waiting. The sleeping camera position/pose itself is **Tier B** (cosmetic): M10-B01's already-shipped `Pose::Sleeping` metadata ordinal (§B01 Context 2's index-6 table) already drives a REMOTE sleeping entity's render pose with zero new code this blueprint needs to add; the LOCAL player's own sleeping-camera lock is new, small, client-local state:

```rust
// crates/client/src/player/combat.rs is NOT the right home (sleep is not combat) —
// crates/client/src/player/sleep.rs (new)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)] pub struct SleepState { pub is_sleeping: bool }
impl SleepState {
    /// Set on receiving a `Set Entity Data` update to the LOCAL player's own `Pose` metadata
    /// index (index 6, M10-B01 §Context 2 table) reading `Sleeping` — the server's own
    /// authoritative signal, never a client-predicted guess (a right-click-on-bed does not
    /// itself set this; only the server's own metadata echo does, matching vanilla's own
    /// server-authoritative sleep-state model).
    pub fn set_sleeping(&mut self, sleeping: bool);
}
```

`PlayerController`'s own camera (M9-B06, consulted-only, no signature touched) is not modified by this blueprint — a future rendering-integration blueprint that actually wires camera-to-bed repositioning consumes `SleepState::is_sleeping` as one more input, mirroring every other "declare the seam, a not-yet-written composition-root blueprint wires it" case in this document (§Interfaces). **Leaving a bed early**: a bound key (this blueprint adds no new `KeyBindings` field for it — vanilla overloads the existing sneak/jump keys, restated: while `SleepState::is_sleeping`, a sneak or jump key-press sends `PlayerCommandOut{action_id: 2}` instead of its ordinary movement meaning, a small, additive branch in the already-existing input-mapping call site) — restated as a `Player Command` (Entity Action) packet, action id `2` (`LeaveBed`, §Context 3's own live-fetch table):

```rust
// crates/client/src/connection/build_packets.rs (new, continued) — or its own small file; placed here since it shares the `Player Action`-adjacent packet-authoring pass
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x2A)]
pub struct PlayerCommandOut { #[rc(varint)] pub entity_id: i32, #[rc(varint)] pub action_id: i32, #[rc(varint)] pub jump_boost: i32 }
pub const PLAYER_COMMAND_LEAVE_BED: i32 = 2;
```

**The honest gap, restated once more:** identical in kind to respawn (§Context 7) — no merged server blueprint interprets a bed target inside `Use Item On`, so `SleepState` never transitions to `true` against a real server today. This blueprint's own decode/state-machine is real and independently tested; the trigger condition simply never fires in production until a future MECH blueprint adds it.

### 9. Reconciliation with M10-B01 — a real inconsistency between two already-merged blueprints, resolved here

**The finding.** M10-B01 §Context 2 states, about its own `Entity Animation` (`0x03`) packet: *"this packet's server-side send call does not exist in any merged blueprint yet either... until a future M4-adjacent blueprint actually sends `Entity Animation` on a real attack, this blueprint's decode path exists but is never exercised against a real server."* This is **correct** for the swing-visibility half (§Context 4's own `SwingArmOut`/its still-missing server-side broadcast counterpart carries the identical honest flag). It is **incorrect** for the hurt/death-animation half: M4-B05 (merged before M10-B01, in this project's own commit history) already, for real, broadcasts `Entity Event{event_id: 2}` (generic hurt) on every successful hit and `Entity Event{event_id: 3}` (death) on every death, to every tracking viewer (M4-B05 §Context "Damage pipeline"/"Death"). M10-B01's own `ClientEntityStore::apply_animation` was built to consume `Entity Animation`'s own `animation_id` `1` ("TakeDamage") for exactly this purpose — a packet, and an `animation_id` value, that no merged server blueprint has ever sent or will send for this purpose, since M4-B05 already covers it through a differently-shaped, already-real packet M10-B01's own derivation appears not to have cross-checked against.

**The resolution, this blueprint's own.** `EntityEventIn` (§Context 4, this blueprint's new packet struct — a real, independent restatement of M4-B05's `Entity Event`, distinct from M10-B01's already-committed `Entity Animation` struct in `entity_packets.rs`, which is left completely unmodified) is decoded by this blueprint and routed through one new, additive public method on M10-B01's already-committed `ClientEntityStore`:

```rust
// crates/client/src/world/entities.rs (M10-B01, additive extension — every existing method/field unchanged)
impl ClientEntityStore {
    /// New in this blueprint. `event_id == ENTITY_EVENT_HURT` (2) calls the target's
    /// `AnimationState::trigger_hurt` (M10-B01, already public); `== ENTITY_EVENT_DEATH` (3)
    /// calls `AnimationState::trigger_death` (M10-B01, already public) — this blueprint's own
    /// first real production caller of both, closing the gap M10-B01 itself could not close at
    /// its own derivation time (M4-B05's already-real behavior). Unknown `event_id` values are
    /// silently ignored, matching every other "unknown wire value is tolerated" rule this
    /// corpus already establishes.
    pub fn apply_entity_event(&mut self, network_id: i32, event_id: u8);
}
```

M10-B01's own `Entity Animation`/`apply_animation` path (`animation_id` `0`/`3` → `trigger_attack_swing`) is left exactly as committed — it remains correctly wired for whichever future blueprint eventually adds the still-missing swing-broadcast server call (§Context 4's own `SwingArmOut` note); its own `animation_id == 1` branch simply stays permanently unreached in production, a small, now-explicitly-documented dead branch rather than a silently-wrong one. **This section is this blueprint's own required disclosure, not a defect report against M10-B01** — the identical "restate the conflict, resolve it in the later-derived blueprint, never silently patch an earlier one" discipline `M9-B00-index.md`'s own Cross-blueprint consistency notes already establishes as this corpus's binding convention.

### 10. Tab list (player info)

`Player Info Update`/`Player Info Remove` — restated per §Context 3's own live fetch:

```rust
// crates/client/src/connection/playerlist_packets.rs (new)
#[derive(Debug, Clone)]
pub struct PlayerInfoEntry {
    pub uuid: u128, pub name: Option<String>, pub game_mode: Option<i32>,
    pub listed: Option<bool>, pub latency_ms: Option<i32>,
    pub display_name: Option<crate::connection::text_component_nbt::TextComponentNbt>,
}
/// Hand-implemented `RcPacket` — **not** `#[derive(RcPacket)]` — since the `actions` bitset
/// gates which per-entry fields are present, a conditional shape the derive macro's attribute
/// grammar cannot express, mirroring M4-B05's own identical `Interact`/`Update Attributes`
/// precedent (Context "Packets") and M10-B01's own `decode_metadata_entries` precedent for the
/// same class of "derive shape doesn't fit" packet.
#[packet(state = "play", bound = "client", id = 0x46)]
pub struct PlayerInfoUpdateIn { pub actions: u8, pub entries: Vec<PlayerInfoEntry> } // bit 0 AddPlayer, bit 1 InitializeChat (decoded-and-discarded — no chat-session cross-player verification at M10), bit 2 UpdateGameMode, bit 5 UpdateListed, bit 6 UpdateLatency, bit 7 UpdateDisplayName (moderate confidence on exact bit positions, flagged)

#[derive(RcPacket, Debug, Clone)]
#[packet(state = "play", bound = "client", id = 0x45)]
pub struct PlayerInfoRemoveIn { #[rc(prefixed_array = "VarInt")] pub uuids: Vec<u128> }
```

```rust
// crates/client/src/world/tab_list.rs (new)
#[derive(Debug, Clone, Default)]
pub struct TabListStore { entries: std::collections::HashMap<u128, TabListEntryState> }
#[derive(Debug, Clone)] struct TabListEntryState { name: String, game_mode: i32, listed: bool, latency_ms: i32, display_name: Option<rc_render::text::component::TextComponent> }
impl TabListStore {
    pub fn new() -> Self;
    pub fn apply_update(&mut self, pkt: &crate::connection::playerlist_packets::PlayerInfoUpdateIn);
    pub fn apply_remove(&mut self, pkt: &crate::connection::playerlist_packets::PlayerInfoRemoveIn);
    /// The per-frame, plain-data snapshot `HudState.tab_list` (§below) carries — built fresh
    /// each frame the overlay is visible, never held across frames (mirrors `EntityRenderState`'s
    /// own "plain data, not a trait object, built fresh" shape, M10-B01 §Context 12).
    pub fn snapshot(&self) -> rc_render::hud::tab_list::TabListSnapshot;
}
```

```rust
// crates/render/src/hud/tab_list.rs (new)
#[derive(Debug, Clone)]
pub struct TabListEntry { pub name: String, pub game_mode: i32, pub latency_ms: i32, pub display_name: Option<crate::text::component::TextComponent> }
#[derive(Debug, Clone, Default)]
pub struct TabListSnapshot { pub entries: Vec<TabListEntry> } // listed-only, sorted by name — the caller's own `snapshot()` filters/sorts, this type is already display-ready

/// Shown only while a bound key (`KeyBindings::show_tab_list`, §Deliverables, default `Tab`) is
/// HELD — not a toggle screen, matching vanilla's own real press-and-hold behavior. Reads
/// `hud.tab_list` (the new, additive `HudState` field, §below) rather than taking a second
/// parameter, so this type satisfies `HudOverlay`'s own already-fixed one-parameter signature
/// (M10-B02) without needing that trait's signature touched.
pub struct TabListOverlay;
impl crate::gui::widget::HudOverlay for TabListOverlay {
    fn layout(&self, hud: &crate::hud::state::HudState, viewport_px: (u32, u32), gui_scale: u32) -> crate::gui::widget::Widget; // returns Widget::Group(vec![]) when hud.tab_list is None (key not held) — the overlay's own visibility gate lives in the DATA, not a second trait method
}
```

`hud::state::HudState` (M10-B02, additive field, every existing field/method unchanged): `pub tab_list: Option<crate::hud::tab_list::TabListSnapshot>` — written every tick by `Shell`'s own tick-content extension (§Deliverables `app.rs`, body-only): `hud.tab_list = if tab_key_held { Some(world.tab_list.snapshot()) } else { None }`.

### 11. Window focus / pause behavior

`Shell::handle_window_event` (M9-B01, already extended body-only by M10-B02 for UI-capture routing — this blueprint's own further body-only extension, `handle_window_event`'s own signature unchanged): gains one new branch, checked before M10-B02's own `CaptureMode`-dispatch branch (a focus-loss event is meaningful regardless of current capture mode): `WindowEvent::Focused(false)` — if `self.ui.active_screen().is_none()` (no screen already open — never override an already-open screen, e.g. the death screen) **and** the current session is singleplayer (a new, small `Shell` field this blueprint adds, `singleplayer: bool`, set once at construction from the same signal `CLIENT-D27`'s embedded-server path already knows, restated per research doc §3.14's own `Dialog.pause`-adjacent framing already cited by M10-B02 §Context 14 for `PauseScreen::wants_pause`) — calls `self.ui.open_screen(Box::new(rc_render::gui::pause_settings::PauseScreen::new(true)))` (M10-B02, already-public, unmodified). `WindowEvent::Focused(true)` (focus regained) is a deliberate **no-op** — matching vanilla's own real behavior of never auto-resuming; the player must explicitly click "Back to Game." Multiplayer sessions (`singleplayer == false`) never auto-pause on focus loss, matching `PauseScreen::wants_pause`'s own already-committed `false`-in-multiplayer rule (M10-B02 §Context 14) — this blueprint's own focus-loss trigger and that already-shipped method now agree by construction, restated as a real, checked consistency rather than an assumed one.

### 12. Player Info tab list & death screen text — the `HudState` additive field, restated once

For clarity, this blueprint's total additive footprint on M10-B02's already-committed `hud::state::HudState` is exactly **one** new field (`tab_list`, §Context 10) — every combat/health/chat write this blueprint performs (§Context 3/4/5) targets fields M10-B02 already declared (`health`, `food`, `saturation`, `attack_cooldown`, `selected_slot`, `set_action_bar`) and needed no new field of its own.

### 13. Testing strategy — TEST-D53's three tiers, restated

Identical binding resolution to every prior M9/M10 render/client blueprint: **zero** Tier-1-gated test in this blueprint's own suite constructs a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` or a real `winit::event_loop::EventLoop`/`Window`, or a real `TcpStream`/live network call (chat-session fetch, entity-target picking, dig-timing math, signing, packet decode — all pure or fake-server-driven). **Tier 1** (PR-blocking): everything in this blueprint's own Acceptance tests below. **Tier 2** (nightly, lavapipe/WARP, per M10-B01's own already-established job): `crates/render/tests/gpu_smoke/block_break_render.rs` — renders one destroy-stage overlay to an offscreen target, asserts pixel presence (not exact match), registered into the identical nightly job M10-B01 already provisions, this blueprint's own first content to actually populate it. **Tier 3** (manual): `docs/MANUAL-VERIFICATION-M10-B04.md`.

## Deliverables

### `crates/msa-auth/src/chat_session.rs` (new), `crates/msa-auth/src/lib.rs` (additive)

Exactly per §Context 3. `lib.rs` gains `pub mod chat_session;`.

### `crates/render/src/lib.rs` (additive — one new line)
```rust
pub mod block_break;
```

### `crates/render/src/block_break/{mod,overlay,texture,pass}.rs` (new)

Exactly per §Context 5.

### `crates/render/src/hud/mod.rs` (additive — one new line), `crates/render/src/hud/tab_list.rs` (new)

Exactly per §Context 10. `mod.rs` gains `pub mod tab_list;`.

### `crates/render/src/hud/state.rs` (additive — one new field, every existing field/method unchanged)
```rust
pub struct HudState {
    // ...every existing field unchanged...
    pub tab_list: Option<crate::hud::tab_list::TabListSnapshot>,
}
```

### `crates/render/src/gui/mod.rs` (additive — one new line), `crates/render/src/gui/death_screen.rs` (new)

Exactly per §Context 7. `mod.rs` gains `pub mod death_screen;`.

### `crates/client/src/chat/{mod,session,signing}.rs` (new)

Exactly per §Context 3.

### `crates/client/src/connection/text_component_nbt.rs` (new)

Exactly per §Context 3.

### `crates/client/src/connection/{chat_packets,combat_packets,build_packets,playerlist_packets,lifecycle_packets}.rs` (new)

Exactly per §Context 3/4/5/7/10. `chat_packets.rs` holds `PlayerSessionOut`, `ChatMessageOut`, `PlayerChatMessageIn`, `SystemChatMessageIn`, `DisguisedChatMessageIn` (the four packet structs named in §Context 3's table, plus `chat/mod.rs`'s `ChatTypeDecoration`/`resolve_chat_type`/`decorate` re-exported from there).

### `crates/client/src/world/{destroy_state,tab_list}.rs` (new)

Exactly per §Context 5/10.

### `crates/client/src/player/{combat,targeting,sleep}.rs` (new)

Exactly per §Context 4/5/6/8.

### `crates/client/src/connection/mod.rs` (additive re-exports, every existing line unchanged)
```rust
pub use chat_packets::{ChatMessageOut, DisguisedChatMessageIn, PlayerChatMessageIn, PlayerSessionOut, SystemChatMessageIn};
pub use combat_packets::{EntityEventIn, InteractOut, SwingArmOut, INTERACT_ATTACK, ENTITY_EVENT_HURT, ENTITY_EVENT_DEATH};
pub use build_packets::{BlockActionSequencer, LevelEventIn, PlayerActionOut, PlayerCommandOut, SetBlockDestroyStageIn, UseItemOnOut, ACTION_ABORT_DESTROY, ACTION_START_DESTROY, ACTION_STOP_DESTROY, PLAYER_COMMAND_LEAVE_BED};
pub use playerlist_packets::{PlayerInfoEntry, PlayerInfoRemoveIn, PlayerInfoUpdateIn};
pub use lifecycle_packets::{ClientCommandOut, CombatDeathIn, RespawnIn, SetHealthIn, SetHeldItemIn, SetHeldItemOut, CLIENT_COMMAND_RESPAWN};
pub use text_component_nbt::{decode_text_component_nbt, TextComponentNbt};
```

### `crates/client/src/world/mod.rs` (additive — four new fields on `ClientWorld`, every existing field/method unchanged)
```rust
pub struct ClientWorld {
    // ...every existing field unchanged (player, entities per M10-B01)...
    pub chat: crate::chat::ChatSessionHandle,
    pub chat_seen: crate::chat::signing::LastSeenTracker,
    pub tab_list: crate::world::tab_list::TabListStore,
    pub destroy: crate::world::destroy_state::ClientDestroyState,
    pub combat: crate::player::combat::LocalCombatState,
}
```
(`ClientWorld::new()`'s body gains five field initializers, all `Default`/`::new()`-constructed — no other change.)

### `crates/client/src/connection/play.rs` (modify — body-only extension, `run_play`'s signature unchanged)

Per §Context 2/6/7: the steady-state dispatch `match` gains new arms for every clientbound id named in §Context 3/4/5/7/10, each decoding via the new packet modules above and calling into `ClientWorld`'s new fields or M10-B02's `HudState`; the outbound-drain branch gains the `PendingActions`-consuming logic of §Context 6.

### `crates/client/src/ui_input.rs`, `crates/client/src/app.rs` (modify — body-only extensions per §Context 6/11, `Shell`/`UiInputRouter`'s own public surfaces unchanged except one new `Shell` field, `singleplayer: bool`, set once in `Shell::new`'s body from the caller-supplied connection kind)

### `crates/client/src/input.rs` (modify — one additive `KeyBindings` field, every existing field/derive unchanged)
```rust
pub struct KeyBindings {
    // ...every existing field unchanged (incl. M10-B02's open_inventory/open_chat/open_command)...
    #[serde(default = "default_show_tab_list")]
    pub show_tab_list: winit::keyboard::KeyCode, // default KeyCode::Tab
}
```

### `crates/client/Cargo.toml` (additive — two lines, one already-pinned, one newly-pinned by this blueprint)
```toml
[dependencies]
# ...every existing line unchanged...
rsa  = { workspace = true }   # already pinned, NET-D6 — this crate's first real client-side use
sha2 = { workspace = true }   # NEW workspace pin this blueprint introduces — SHA-256 for chat-message signing (SHA256withRSA), distinct from NET-D6's SHA-1 server-hash use
```

### `docs/planning/12-workspace-structure.md`'s pin table — flagged forward, not edited by this blueprint

Per TEST-D46/this corpus's own governance rule (`00-blueprint-spec.md` §Governance: "where a blueprint and a planning document conflict, the planning document wins and the blueprint must be corrected" — here the direction is the reverse, a genuinely NEW pin this blueprint introduces that `12`'s own committed pin table does not yet list): `sha2 = "0.10.9"` (RustCrypto family, matching this project's own established `aes`/`cfb8`/`rsa`/`sha1` affinity) should be added to `12`'s `[workspace.dependencies]` table by whoever merges this blueprint's implementation changeset — restated here as the exact edit that document's own next revision needs, per this corpus's established "cite the gap, name the exact edit" precedent (M9-B03 §Context 1, M10-B01 §Interfaces).

### `docs/MANUAL-VERIFICATION-M10-B04.md` (implementer creates; content this blueprint specifies)

A short, reproducible reference-host procedure: authenticate a real Microsoft account (reusing M9-B03's own session); confirm `fetch_chat_session` returns a real key pair against the real endpoint; send a real signed chat message against a real vanilla or Rusty Clanker server and confirm it displays correctly on another real client; confirm the attack-cooldown crosshair indicator fills over roughly the expected 5-ish-tick window on a fresh session; confirm attacking a real spawned mob (via a debug/test server) produces a visible hurt flash on that mob and, at 0 HP, a death animation; confirm the local player's own screen tilts/flashes when damaged; confirm holding left-click on a block shows an incrementally-filling crack overlay that clears the instant the block actually breaks; confirm right-click places a block at the targeted face; confirm the death screen appears on death with the correct message and that clicking "Respawn" against the FAKE-server test harness (§Context 7 — not a real server, since server-side respawn does not exist yet) completes the round trip; confirm holding Tab shows a player-name list; confirm alt-tabbing away from a singleplayer session opens the pause menu, and does not in multiplayer.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** `crates/render/tests/{block_break_overlay,block_break_texture,tab_list_widget,death_screen}.rs`, `crates/client/tests/{chat_signing,chat_session_decode,text_component_nbt,chat_packet_decode,combat_targeting,attack_cooldown,destroy_state,build_packet_decode,playerlist_decode,lifecycle_flow,focus_pause}.rs`, `crates/msa-auth/tests/chat_session_response.rs`, plus every new file from Deliverables with every function body `todo!()`-stubbed (structs/enums fully defined) are committed first. The implementation changeset fills bodies and extends the Cargo.toml/`ClientWorld`/`HudState`/`KeyBindings`/`play.rs`/`ui_input.rs`/`app.rs`/`entities.rs` bodies named in Deliverables — it must not modify any file under any of the three `tests/` directories above, and must not touch any pre-existing M9-B0x/M10-B01/M10-B02 test file.

- `crates/msa-auth/tests/chat_session_response.rs`: `parses_a_well_formed_response` — a hand-authored fixture JSON matching §Context 3's own shape, `parse_chat_session_response` returns `Ok(ChatSessionKeyPair{..})` with every field matching. `missing_key_pair_field_is_an_error` — fixture with `keyPair` omitted, `Err(ChatSessionError::Malformed("keyPair"))`. `malformed_public_key_signature_base64_is_an_error` — a non-base64 string in that field, `Err`.
- `crates/client/tests/chat_signing.rs` (**the task's own required "signing-chain conformance against a fake server" — self-consistency vectors, flagged per §Context 3's own honesty note, since no byte-exact Mojang known-answer vector exists in this project's research corpus**): `sign_then_verify_round_trips` — a `MessageSigner` built from a freshly-generated (test-only, `rsa::RsaPrivateKey::new`) key pair, `sign("hello", &LastSeenTracker::new())`, then independently reconstruct the same byte assembly and verify the returned signature against the matching public key — `Ok`. `index_increments_per_sign_call` — three `sign` calls, assert `link().index` reads `0, 1, 2` in `link()` calls taken between each. `salt_is_never_repeated_across_calls` — 100 `sign` calls, assert every returned `salt` is distinct (a probabilistic, not absolute, proof — documented as such in the test's own comment). `tampered_content_fails_verification` — sign `"hello"`, verify the signature against the byte assembly for `"hellp"` instead — verification fails. `unsigned_handle_never_produces_a_signature` — `ChatSessionHandle::Unsigned`'s own send-path (a small helper this test exercises directly) always yields `signature: None, salt: 0`.
- `crates/client/tests/chat_session_decode.rs`: `last_seen_tracker_window_is_bounded` — push `LAST_SEEN_WINDOW + 5` entries via `record_seen`, assert `build_acknowledgement().0` (the count) reflects only the bounded window, never overflowing the fixed 3-byte bitset. `acknowledgement_bitset_marks_every_tracked_entry` — push 5 entries, assert the returned `[u8;3]`'s low 5 bits are all set and the rest clear.
- `crates/client/tests/text_component_nbt.rs`: `decodes_plain_text` — a hand-encoded NBT buffer for `{text: "hi"}`, `decode_text_component_nbt` returns `TextComponent::plain("hi")`. `decodes_color_and_bold` — `{text: "hi", color: "red", bold: 1u8}`, assert `style.color == Some(TextColor::Named(NamedColor::Red))` and `style.bold == Some(true)`. `decodes_nested_extra` — a component with one `extra` sibling, assert `extra.len() == 1` and its own content round-trips. `bare_string_root_is_accepted` — a lone `TAG_String` root (no compound wrapper), decodes to `TextComponent::plain(..)` unchanged. `truncated_buffer_is_a_decode_error_not_a_panic` — a buffer cut mid-tag, `Err(NbtTextComponentError::Truncated)`.
- `crates/client/tests/chat_packet_decode.rs`: golden byte-vectors for `PlayerChatMessageIn`/`SystemChatMessageIn`/`DisguisedChatMessageIn`/`ChatMessageOut`/`PlayerSessionOut` (hand-encoded fixture bytes, decoded/encoded, asserted field-by-field), mirroring `play_packets.rs`'s own established convention exactly (M9-B03). `resolve_chat_type_falls_back_to_generic_for_unknown_id` — id `999`, `ChatTypeDecoration::Generic`.
- `crates/client/tests/combat_targeting.rs`: `picks_the_closest_of_two_entities_in_line` — two fixture `TrackedEntity`s on the same ray at different distances, `pick_entity_target` returns the nearer's `network_id`. `entity_beyond_reach_is_not_picked` — a single entity at `4.0` blocks (beyond `ENTITY_INTERACTION_RANGE = 3.0`), returns `None`. `entity_off_ray_is_not_picked` — an entity well off-axis, `None`.
- `crates/client/tests/attack_cooldown.rs`: `cooldown_reaches_full_at_delay_ticks` — `attack_cooldown_indicator(5, 4.0)` (delay = `20.0/4.0 = 5`), returns `1.0`. `cooldown_is_zero_at_ticker_zero` — `attack_cooldown_indicator(0, 4.0) == 0.0`. `advance_tick_increments_ticker` — `LocalCombatState::default()`, three `advance_tick` calls, `attack_ticker == 3`. `on_local_attack_resets_ticker` — advance to `5`, `on_local_attack()`, assert `attack_ticker == 0`. `damage_tilt_decays_linearly_and_matches_b01_duration` — `trigger()`, then `advance_tick` `DAMAGE_TILT_TICKS` times one at a time, asserting `intensity()` is `1.0` immediately after trigger, monotonically decreasing, and `0.0` at the final tick — the identical shape M10-B01's own `hurt_ticks_remaining` golden test already establishes, reused here for this blueprint's own local-only twin.
- `crates/client/tests/destroy_state.rs` (**the task's own required "destroy-progress stage timing goldens — mirror M3-B03's formula"**): `stone_bare_hand_takes_150_ticks` — `ticks_to_break_predicted(1.5) == 150` (hand-computed: `ceil(1.5 * 100.0)`). `stage_reaches_9_just_before_completion` — a `ClientDestroyState` begun at tick `0` against Stone, `advance_tick` called at tick `149`, assert `current_stage == 9` (`floor(149/150 * 10) = 9`). `stage_sequence_is_monotonically_nondecreasing` — advance across every tick from `0` to `150`, assert `current_stage` never decreases between consecutive ticks. `unknown_block_uses_fallback_hardness` — a block-state id outside the tier-1 table, `block_hardness` returns `None` and the caller's own fallback path uses `FALLBACK_HARDNESS`, asserted via `ticks_to_break_predicted(FALLBACK_HARDNESS) == 150` (Stone and the fallback share the same value by construction, a documented coincidence, not a bug). `clear_resets_state` — begin, advance, `clear()`, assert `target == None, current_stage == -1`.
- `crates/client/tests/build_packet_decode.rs`: golden byte-vectors for `PlayerActionOut`/`UseItemOnOut`/`SetBlockDestroyStageIn`/`LevelEventIn`/`PlayerCommandOut`, mirroring M9-B03's own established convention. `sequencer_starts_at_one_and_increments` — `BlockActionSequencer::new()`, three `next_sequence()` calls return `1, 2, 3`.
- `crates/client/tests/playerlist_decode.rs`: `add_player_action_populates_name_and_uuid` — a fixture `PlayerInfoUpdateIn` with only the AddPlayer bit set, `TabListStore::apply_update` then `snapshot()` shows one entry with the fixture's name. `remove_drops_entry` — add then `apply_remove`, `snapshot().entries.is_empty()`. `update_latency_only_updates_the_named_field` — apply an AddPlayer update, then an UpdateLatency-only update for the same uuid, assert `name` is unchanged and `latency_ms` reflects the second update.
- `crates/client/tests/build_packet_decode.rs` (continued): `set_held_item_in_updates_selected_slot` — fake-fed `SetHeldItemIn{slot: 4}` routed through the same handler `lifecycle_flow.rs` exercises end-to-end, assert `hud.selected_slot == 4`. `set_held_item_out_round_trips` — golden byte-vector for `SetHeldItemOut{slot: 3}`.
- `crates/client/tests/lifecycle_flow.rs` (**exercised against a fake, in-test TCP/duplex server harness, mirroring M9-B03's own established fake-server test convention exactly — the task's own required "death/respawn state machine" test**): `combat_death_opens_death_screen_and_suppresses_input` — fake server sends `CombatDeathIn{player_id: <world's own entity_id>, message: <encoded "You died">}`; assert the client's own `UiInputRouter::active_screen()` is `Some` (a `DeathScreen`) and `world.combat.is_dead == true`. `respawn_button_sends_client_command_and_awaits_respawn` — continuing the above, simulate `DeathScreen::take_action() -> Respawn`; assert the fake server reads back `ClientCommandOut{action_id: CLIENT_COMMAND_RESPAWN}` within a bounded wait; fake server then sends `RespawnIn{..}`; assert `world.combat.is_dead == false`, `world.entities` is empty (freshly reset), and `UiInputRouter::active_screen()` is `None` (death screen closed). `set_health_decrease_triggers_damage_tilt` — fake server sends `SetHealthIn{health: 20.0, ..}` then `SetHealthIn{health: 14.0, ..}`; assert `world.combat.damage_tilt.intensity() > 0.0` after the second packet, `== 0.0` after the first (no prior value to compare against, no trigger). `set_health_increase_does_not_trigger_damage_tilt` — the reverse delta, assert `intensity() == 0.0` throughout. `entity_event_hurt_triggers_remote_animation` — fake server first sends a `Spawn Entity` for a zombie (reusing M10-B01's own already-established fixture shape), then `EntityEventIn{entity_id: <that zombie>, event_id: ENTITY_EVENT_HURT}`; assert, via `ClientEntityStore::get`, the zombie's `anim.hurt_ticks_remaining == HURT_FLASH_TICKS` (M10-B01's own constant, reused) — the task's own required proof that §Context 9's reconciliation is real, not merely documented.
- `crates/client/tests/focus_pause.rs`: `focus_lost_opens_pause_in_singleplayer` — a `Shell` constructed with `singleplayer: true` and no screen open, `handle_window_event(&WindowEvent::Focused(false))`; assert `ui_router_mut().active_screen().is_some()`. `focus_lost_is_a_noop_in_multiplayer` — identical setup with `singleplayer: false`; assert `active_screen().is_none()` after the same event. `focus_lost_does_not_override_an_already_open_screen` — open a (non-pause) screen first, then fire the event; assert the ORIGINAL screen is still active (`PauseScreen` is never pushed on top of it). `focus_regained_is_always_a_noop` — open the pause menu via focus loss, then `handle_window_event(&WindowEvent::Focused(true))`; assert the pause menu is still open (not auto-closed).
- `crates/render/tests/block_break_overlay.rs`: `destroy_overlay_mesh_produces_24_vertices_36_indices` — one unit cube, matching M9-B04's own established "6 faces × 4 verts, × 2 tris" shape. `higher_stage_selects_higher_texture_layer` — `destroy_overlay_mesh((0,0,0), 9, 9)` vs. `((0,0,0), 0, 0)`, assert every emitted vertex's material/layer field differs accordingly (exact field per `Vertex`'s own packed layout, M9-B04).
- `crates/render/tests/block_break_texture.rs`: `builds_ten_distinct_layers_from_fixture_textures` — ten small, distinct-colored fixture `DecodedTexture`s (never real Mojang assets — hand-authored, solid-color 16×16 fixtures, matching this corpus's own established "fixture, never real content" test-asset convention throughout), `DestroyStageTextureBuilder::build` (fed via a test-only in-memory `AssetStore` seeded with those fixtures) returns data with 10 distinct layers.
- `crates/render/tests/tab_list_widget.rs`: `empty_snapshot_produces_empty_group` — `TabListOverlay::layout` against a `HudState` with `tab_list: None`, returns `Widget::Group(vec![])`. `populated_snapshot_produces_one_text_widget_per_entry` — three fixture entries, assert the returned tree contains exactly three `Widget::Text` nodes with matching names.
- `crates/render/tests/death_screen.rs`: `layout_includes_the_death_message` — `DeathScreen::new(TextComponent::plain("you died"))`, assert the laid-out `Widget` tree contains a `Widget::Text` whose component matches. `respawn_button_click_sets_action` — simulate the appropriate `UiEvent::MouseButton` at the Respawn button's own laid-out rect, `on_ui_event` then `take_action() == DeathScreenAction::Respawn`. `escape_does_not_close` — `can_close_with_escape() == false`.

## Implementation steps

1. **`rc-msa-auth`'s `chat_session.rs`.** Implement `ChatSessionKeyPair`/`parse_chat_session_response`. Observable: `chat_session_response.rs` passes; every pre-existing `rc-msa-auth` test still passes unmodified.
2. **`rc-render`'s `block_break/overlay.rs` + `texture.rs` (pure halves).** Observable: `block_break_overlay.rs`/`block_break_texture.rs` pass.
3. **`rc-render`'s `hud/tab_list.rs` + `HudState`'s additive field.** Observable: `tab_list_widget.rs` passes; every pre-existing M10-B02 test still passes.
4. **`rc-render`'s `gui/death_screen.rs`.** Observable: `death_screen.rs` passes.
5. **`crates/client`'s `connection/text_component_nbt.rs`.** Observable: `text_component_nbt.rs` passes.
6. **`crates/client`'s `chat/signing.rs` + `chat/session.rs`.** Observable: `chat_signing.rs`/`chat_session_decode.rs` pass; `chat/session.rs`'s real HTTP call is exercised only manually (real network I/O), not by a Tier-1 test.
7. **`crates/client`'s `connection/{chat_packets,combat_packets,build_packets,playerlist_packets,lifecycle_packets}.rs`.** Observable: `chat_packet_decode.rs`/`build_packet_decode.rs`/`playerlist_decode.rs` pass.
8. **`crates/client`'s `player/{combat,targeting,sleep}.rs`.** Observable: `combat_targeting.rs`/`attack_cooldown.rs` pass.
9. **`crates/client`'s `world/{destroy_state,tab_list}.rs`.** Observable: `destroy_state.rs` passes.
10. **`crates/client`'s `world/mod.rs` extension (`ClientWorld`'s five new fields).** Observable: compiles against every module above; every pre-existing `crates/client` test still passes.
11. **`crates/client/src/world/entities.rs`'s additive `apply_entity_event` (§Context 9).** Observable: compiles against M10-B01's already-committed `ClientEntityStore`; every pre-existing M10-B01 test still passes unmodified.
12. **`crates/client`'s `connection/play.rs` extension.** Add every new dispatch arm + the `PendingActions`-drain outbound extension. Observable: compiles; `lifecycle_flow.rs` passes; every pre-existing `play_flow.rs`/`teleport_reconciliation.rs`/`movement_cadence.rs`-class test still passes unmodified.
13. **`crates/client`'s `ui_input.rs`/`app.rs`/`input.rs` extensions.** Add mouse-click routing, `Focused` handling, the `show_tab_list` binding. Observable: `focus_pause.rs` passes; every pre-existing M10-B02 `ui_input`-adjacent test still passes.
14. **Real-GPU glue (`block_break/texture.rs`'s `upload`, `block_break/pass.rs`).** Not exercised by Tier 1. Write and register the Tier-2 `gpu_smoke/block_break_render.rs` test into M10-B01's already-provisioned nightly job. Observable: `cargo build -p rc-render --all-features` succeeds.
15. **`docs/MANUAL-VERIFICATION-M10-B04.md`.** Write per Deliverables; execute and record the pass.
16. **Full build + full local Tier-1 test pass.** `cargo build -p rc-render -p rusty-clanker-client -p rc-msa-auth --all-features`, `cargo nextest run -p rc-render -p rusty-clanker-client -p rc-msa-auth`, confirming zero warnings, every new test green, and every pre-existing M9-B0x/M10-B01/M10-B02 test still green.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45).** Every test file named in Acceptance tests is committed first, against `todo!()`-stubbed bodies matching Deliverables' exact signatures. The implementation changeset must not edit any file under any of the three `tests/` directories named above, and must not weaken, delete, or `#[ignore]` any named test case (TEST-D46/D49).

(b) **Every pre-existing M9-B0x/M10-B01/M10-B02 test file and public signature is a protected surface.** No file under `crates/render/tests/`, `crates/client/tests/`, or `crates/msa-auth/tests/` that a prior blueprint already committed is touched. No public signature already committed by any prior blueprint (`ClientEntityStore`'s existing methods, `HudState`'s existing fields/methods, `Screen`/`HudOverlay`'s trait signatures, `ClientWorld`'s existing fields, `Shell`/`UiInputRouter`'s existing methods, `KeyBindings`'s existing fields, `AuthSession`, `McAccessToken`, `play_packets.rs`'s existing structs) is modified — every extension is additive-only (a new field, a new method, a new match arm), the identical discipline every prior M9/M10 blueprint already binds itself to for these same files.

(c) **No new external dependencies beyond `rsa` (already pinned, NET-D6) and `sha2` (newly pinned by this blueprint, §Deliverables, flagged forward to `12`'s own pin table) on `rusty-clanker-client`.** No general-purpose NBT crate (`text_component_nbt.rs` is a bounded, hand-rolled reader, §Context 3 — not `simdnbt`, not `rc-nbt`). No second HTTP client (`reqwest`, already present since M10-B01). No `image`-crate-adjacent second PNG decoder for destroy-stage textures (`rc_assets::texture::decode_png`, reused via `AssetStore::load_texture`, unmodified).

(d) **No Mojang or third-party reimplementation code.** Every packet id/field-layout table in this blueprint is either restated verbatim from an already-merged prior blueprint's own already-flagged table (M4-B05, M3-B03, M9-B03) or sourced from a live fetch of minecraft.wiki's own public protocol documentation performed while deriving this blueprint (§Context 3/4/5/7/10, each moderate-confidence-flagged) — never the pinned version's decompiled jar beyond what ASSET-D18(f)/ASSET-D28 already confirm safe project-wide, and this blueprint consults neither for anything. The NBT text-component field-name mapping (§Context 3) is this blueprint's own best-effort restatement of the same publicly-documented key names M10-B02's own JSON `TextComponent` parser already uses (ASSET-D18(b)), applied to NBT tag types instead of JSON value types — never independently re-derived from any Mojang source.

(e) **The Tier-1 headless boundary (§Context 13) is binding.** No test under `crates/render/tests/` (outside the `gpu_smoke/` subdirectory) or `crates/client/tests/` constructs a real `wgpu`/`winit` object or a real `TcpStream`/live HTTP connection.

(f) **No scope creep into named-deferred seams.** Do not implement inventory/container content sync, `Update Attributes`, `Set Experience`, boss bars, the scoreboard, command-argument signing, shield blocking, dialogs, waypoints, or client-side world-state prediction for block place/break — every one is a named, deliberate deferral (§Context 1), and adding a placeholder implementation of any of them "to look more complete" would misrepresent this blueprint's own seams as filled when they are not.

(g) **Zero `unsafe` code.** Every deliverable in this blueprint is ordinary safe Rust, matching M10-B01's/M10-B02's own identical constraint.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-render -p rusty-clanker-client -p rc-msa-auth --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo nextest run -p rc-render -p rusty-clanker-client -p rc-msa-auth
cargo test --doc -p rc-render -p rusty-clanker-client -p rc-msa-auth
```

Expected: every command exits 0, with zero test in the default `nextest` run constructing a real `wgpu`/`winit` object or a real network connection (§Context 13, Constraint e), and every pre-existing M9-B0x/M10-B01/M10-B02 test still passing unmodified. This is the authoritative Tier-1 done-signal (TEST-D50) — CI green on both `ubuntu-24.04` and `windows-2025`.

**Tier 2 (nightly cron, extends M10-B01's already-provisioned job, no new CI job created):**
```
cargo nextest run -p rc-render --features gpu-smoke -- gpu_smoke::block_break_render
```

`docs/MANUAL-VERIFICATION-M10-B04.md`'s real-account/real-server pass is executed and recorded manually (TEST-D53 Tier 3).

## Interfaces

**Needs from a not-yet-written composition-root/integration blueprint (the same gap M9-B04/M9-B05/M9-B06/M10-B01/M10-B02 §Interfaces each already name identically):** wiring `BlockBreakPass` into `Shell`'s render sequence alongside `TerrainRenderer`/`EntityPass`/`ViewmodelRenderer`/`GuiRenderer`; consuming `DamageTiltState::intensity()` in an actual camera-shake/screen-flash effect; driving `GameplayMouseRouter::advance_tick`/`ClientDestroyState::advance_tick` once per client tick from the real tick loop.

**Needs from a future sibling blueprint (inventory/container content — not yet named or numbered):** decoding `Container Set Content`/`Set Slot`/`Set Cursor Item` into `HudState.hotbar`'s real per-slot contents — the single gap this blueprint's own §Context 1/5 names as blocking both correct build-loop item selection and a real (rather than bare-hand-bounded) destroy-progress prediction; decoding `Update Attributes` — the gap blocking a real (rather than fixed-default) attack-cooldown indicator.

**Needs from a future M4-adjacent server blueprint:** the still-missing `Entity Animation` swing-broadcast send call (§Context 4/9); real server-side bed/sleep interaction inside `Use Item On` (§Context 8); real server-side respawn (`RespawnIn`'s own send call, and `ClientCommandOut`'s own receive handling) — M4-B05's own already-stated deferral, restated and left exactly as open as that blueprint left it.

**Needs from a future `02-protocol-networking.md` revision:** should fold this blueprint's own `text_component_nbt.rs` field-name mapping into a real, general `rc-nbt`-backed codec, replacing this blueprint's own bounded, hand-rolled reader — mirroring M1-B05's own hand-rolled NBT writer's identical eventual fate, cited by M9-B03 §Context 12.

**Provides to `08-assets-auth-legal.md`:** the concrete chat-session-certificate endpoint/response shape (§Context 3) that document's own decision register does not yet name — flagged for that document's own next revision to fold in as a formal decision ID alongside ASSET-D7/D8.

**Provides to a future M10-B05 (mod API client wiring):** nothing new — this blueprint adds no mod-facing extension point, consistent with §Context 1's own stated boundary.

## Open Questions

- Every packet id restated from this blueprint's own live fetch (§Context 3/4/5/7/10) — `Player Session`(0x0A)/`Chat Message`(0x09)/`Player Chat Message`(0x41)/`System Chat Message`(0x79)/`Disguised Chat Message`(0x20)/`Player Info Update`(0x46)/`Player Info Remove`(0x45)/`Respawn`(0x52)/`Client Command`(0x0C)/`Player Command`(0x2A)/`Combat Death`(0x44)/`SwingArmOut`(0x36) — carries the identical moderate-confidence, pending-`reports/packets.json`-reconciliation status every packet table in this corpus already carries; every id that overlapped an already-independently-fixed prior blueprint's own value matched exactly, corroborating but not proving the rest.
- The signing-chain byte assembly (§Context 3) is this blueprint's own best-effort candidate, not a byte-exact known-answer vector — flagged for reconciliation against a real packet capture; a mismatch changes only `signing.rs`'s internal byte-assembly order, never `MessageSigner`'s own public signature.
- The NBT text-component field-name mapping (§Context 3) assumes SNBT's key names are unchanged from the JSON form's own key names — a reasonable, but not independently re-verified, assumption; flagged for the same reconciliation pass.
- `Player Info Update`'s exact `actions` bitset bit positions (§Context 10) are this blueprint's own best-effort restatement — a single-function (`PlayerInfoUpdateIn`'s own decode body) correction point if wrong.
- The client-side destroy-progress overlay's bare-hand-only bound (§Context 5) is a real, accepted Tier-B (cosmetic) limitation until a future inventory blueprint closes the held-item-content gap — not attempted here, since a wrong-tool-aware guess would be worse than an honestly-bounded one.
- Whether commands should route through `Signed Chat Command`'s own per-argument signing (§Context 1, "Commands") rather than this blueprint's own "send as ordinary chat text" simplification is left open, pending a future blueprint that actually implements client-side command parsing/autocomplete.
- `ClientDestroyState`/`GameplayMouseRouter`'s own cross-thread placement (owned by `ClientWorld`'s existing mutex, §Context 6, rather than a second `Arc<Mutex<_>>` mirroring `SharedMotion`) should be revisited if a future blueprint's own profiling shows lock contention between the tick thread's writes and the network thread's per-heartbeat drain — not a correctness concern at this blueprint's own scope, since both sides already serialize through `ClientWorld`'s single lock.
