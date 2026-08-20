# Services & Misc Server Surface — MC 26.2

## 1. Purpose

This domain covers everything on the dedicated server that is neither the core ECS/tick pipeline (doc 01), the wire protocol (doc 02), nor the roadmap (doc 11): session authentication against Mojang's services, the `server.properties` configuration surface, the new JSON-RPC server-management API, the Swing console/RCON/GS4 remote-control surfaces, world border, weather/day-night timing, raids, ambient "event" spawners, the permission model, the in-game GameTest framework, the debug-subscription system, crash reporting, and the datapack-driven feature-flag system. It is the collection point for load-bearing infrastructure that a reimplementation must reproduce even though none of it is "gameplay" in the strict sense.

## 2. Where it lives

Package → responsibility, with representative classes and file counts (`.java` files, direct children only unless noted). Packages marked **(leftover)** exist under `net/minecraft/server` but are substantively owned by another planning document; they are listed here only so the cartography is complete, per the assignment's "map leftovers explicitly" instruction.

| Package | Responsibility | Representative classes | Files |
|---|---|---|---|
| `net.minecraft.server` (root) | Server bootstrap, service wiring, top-level `MinecraftServer` | `MinecraftServer`, `Main`, `Services`, `Bootstrap`, `Eula` | — |
| `net.minecraft.server.dedicated` | `server.properties` schema, dedicated-server entry point, watchdog | `DedicatedServer`, `DedicatedServerProperties`, `Settings`, `ServerWatchdog`, `DedicatedServerSettings` | 7 |
| `net.minecraft.server.jsonrpc` | JSON-RPC 2.0 / OpenRPC server-management transport core | `ManagementServer`, `Connection`, `JsonRpc`, `IncomingRpcMethods`, `OutgoingRpcMethods`, `JsonRpcNotificationService` | 13 |
| `net.minecraft.server.jsonrpc.api` | Wire-schema description types used to self-generate the OpenRPC document | `MethodInfo`, `ParamInfo`, `ResultInfo`, `Schema`, `SchemaComponent`, `PlayerDto` | 8 |
| `net.minecraft.server.jsonrpc.dataprovider` | Datagen hook that dumps the live schema to `json-rpc-api-schema.json` | `JsonRpcApiSchema` | 2 |
| `net.minecraft.server.jsonrpc.internalapi` | Thin service façade wrapping `PlayerList`/`StoredUserList`/`GameRules` for RPC handlers | `MinecraftApi`, `MinecraftPlayerListService(Impl)`, `MinecraftBanListService(Impl)`, `MinecraftAllowListService(Impl)`, `MinecraftOperatorListService(Impl)`, `MinecraftGameRuleService(Impl)`, `MinecraftServerSettingsService(Impl)`, `MinecraftServerStateService(Impl)`, `MinecraftExecutorService(Impl)` | 17 |
| `net.minecraft.server.jsonrpc.methods` | Per-domain RPC method bodies + DTOs/codecs, registered into `IncomingRpcMethods`/`OutgoingRpcMethods` | `PlayerService`, `BanlistService`, `IpBanlistService`, `AllowlistService`, `OperatorService`, `GameRulesService`, `ServerSettingsService`, `ServerStateService`, `DiscoveryService` | 16 |
| `net.minecraft.server.jsonrpc.security` | Bearer-token auth handler, TLS keystore loading, secret generation | `AuthenticationHandler`, `SecurityConfig`, `JsonRpcSslContextProvider` | 3 |
| `net.minecraft.server.jsonrpc.websocket` | Netty codec glue between raw WebSocket frames and Gson `JsonElement` | `WebSocketToJsonCodec`, `JsonToWebSocketEncoder` | 2 |
| `net.minecraft.server.gui` | Swing operator console (log view + chat/command box + stats/player list) | `MinecraftServerGui`, `StatsComponent`, `PlayerListComponent` | 4 |
| `net.minecraft.server.rcon` + `.rcon.thread` **(leftover → 02-NET)** | RCON protocol and GS4 query/stats listener | `RconThread`, `RconClient`, `QueryThreadGs4`, `RconConsoleSource` | 8 |
| `net.minecraft.server.chase` | Undocumented raw-TCP camera-position sync protocol (`/chase lead`/`/chase follow`) | `ChaseServer`, `ChaseClient` | 3 |
| `net.minecraft.server.notifications` | Event bus that fans server-state changes out to registered `NotificationService`s (JSON-RPC is one consumer) | `NotificationManager`, `NotificationService`, `EmptyNotificationService`, `ServerActivityMonitor` | 5 |
| `net.minecraft.server.players` | Login profile cache/resolution, ban/allow/op file-backed lists, legacy-list conversion | `PlayerList`, `ProfileResolver`, `CachedUserNameToIdResolver`, `NameAndId`, `StoredUserList`, `UserBanList`, `IpBanList`, `UserWhiteList`, `ServerOpList`, `OldUsersConverter` | 19 |
| `net.minecraft.server.permissions` | Data-driven permission model backing op levels and function permissions | `Permission`, `Permissions`, `PermissionLevel`, `PermissionSet`, `LevelBasedPermissionSet`, `PermissionCheck` | 12 |
| `net.minecraft.server.waypoints` **(leftover → 05-MECH)** | Server-side locator-bar waypoint fan-out, gated by `GameRules.LOCATOR_BAR` | `ServerWaypointManager` | 2 |
| `net.minecraft.server.bossevents` **(leftover → 05-MECH)** | `/bossbar` command backing store | `CustomBossEvent`, `CustomBossEvents` | 3 |
| `net.minecraft.server.advancements` **(leftover → 05-MECH)** | Advancement-tab visibility rules | `AdvancementVisibilityEvaluator` | 2 |
| `net.minecraft.server.dialog` + `.action`/`.body`/`.input` **(leftover → 05-MECH / 07-CLIENT)** | Server-authored custom-GUI "dialog" screen definitions (`/debugconfig dialog`, dialog item components) | `Dialog`, `Dialogs`, `ButtonListDialog`, `ConfirmationDialog`, `DialogAction`, `Input` | 33 |
| `net.minecraft.server.packs` + `.linkfs`/`.metadata`/`.repository`/`.resources` **(leftover → 01-ARCH / 03-WORLD)** | Server-side pack resource loading/repository (datapacks, vanilla pack) | `PackRepository`, `VanillaPackResources`, `FilePackResources`, `DownloadQueue` | 15+ |
| `net.minecraft.server.commands` **(leftover → 05-MECH, except items below)** | Brigadier command implementations | `ChaseCommand`, `RaidCommand`, `DebugConfigCommand`, `FetchProfileCommand`, `WhitelistCommand`, `BanPlayerCommands`, `BanIpCommands`, `OpCommand`, `DeOpCommands`, `GameRuleCommand`, `DifficultyCommand`, `StopCommand`, `SaveAllCommand`/`SaveOnCommand`/`SaveOffCommand` | 96 |
| `net.minecraft.server.level` **(leftover → 01-ARCH)** | `ServerLevel` tick pipeline (owns weather/border/raid tick calls) | `ServerLevel`, `ServerChunkCache` | 37 |
| `net.minecraft.server.network` **(leftover → 02-NET)** | Packet listeners incl. the login handshake described in §3.1 | `ServerLoginPacketListenerImpl`, `ServerGamePacketListenerImpl` | 22 |
| `net.minecraft.world.level.border` | World border shape, lerp, damage parameters | `WorldBorder`, `BorderChangeListener`, `BorderStatus` | 4 |
| `net.minecraft.world.entity.raid` | Raid state machine and per-server raid registry | `Raid`, `Raids`, `Raider` | 3 |
| `net.minecraft.world.effect` (relevant slice) | Bad-Omen-successor effect that triggers a raid | `RaidOmenMobEffect` | 1 of many |
| `net.minecraft.world.clock` | Data-driven "world clock" abstraction (replaces the old hardcoded day counter) | `WorldClock`, `WorldClocks`, `ClockManager`, `ServerClockManager`, `ClockTimeMarker(s)` | 6 |
| `net.minecraft.world.timeline` | Datapack-defined keyframe tracks driving `EnvironmentAttribute`s (sky color, mob-burn window, etc.) off a `WorldClock` | `Timeline`, `Timelines` | 2 |
| `net.minecraft.world.flag` | Feature-flag bitset system gating experimental content | `FeatureFlag`, `FeatureFlags`, `FeatureFlagSet`, `FeatureFlagRegistry` | 5 |
| `net.minecraft.world.level.levelgen` (relevant slice) | Ambient hostile spawners | `PatrolSpawner`, `PhantomSpawner` | 2 of many |
| `net.minecraft.world.entity.npc` (relevant slice) | Ambient passive/neutral spawners | `CatSpawner`, `WanderingTraderSpawner` | 2 of many |
| `net.minecraft.world.entity.ai.village` (relevant slice) | Legacy zombie-siege event spawner | `VillageSiege` | 1 of many |
| `net.minecraft.gametest` + `.framework` | In-game automated test framework | `GameTestServer`, `GameTestRunner`, `GameTestTicker`, `GameTestInfo`, `GameTestInstance`, `TestEnvironmentDefinition`, `GameTestHelper` | 46 |
| `net.minecraft.util.debug` | Client-subscription-driven debug telemetry (successor to the old static `DebugPackets` broadcaster) | `DebugSubscription(s)`, `ServerDebugSubscribers`, `LevelDebugSynchronizers`, `TrackingDebugSynchronizer` | 18 |
| `net.minecraft` (root, relevant slice) | Crash report model | `CrashReport`, `CrashReportCategory`, `SystemReport`, `ReportType` | 4 of many |

## 3. How it works

### 3.1 Session authentication & profile resolution

`Services` (`net.minecraft.server.Services`) is constructed once at boot from a `YggdrasilAuthenticationService` and bundles: a `MinecraftSessionService` (authlib, used only for `hasJoinedServer`/`fetchProfile` — compiled dependency, not decompiled), a `ServicesKeySet` (public keys for player-signed content), a `GameProfileRepository` (name→profile lookup), a `UserNameToIdResolver` (`CachedUserNameToIdResolver`, backed by `usercache.json`), and a `ProfileResolver` (`ProfileResolver.Cached`, an in-memory Guava cache in front of the session service).

Login handshake (`ServerLoginPacketListenerImpl`, states `HELLO → KEY → AUTHENTICATING → VERIFYING → (WAITING_FOR_DUPE_DISCONNECT) → PROTOCOL_SWITCHING → ACCEPTED`):
1. `handleHello` — validates the requested username, short-circuits to the fixed singleplayer profile if it matches, otherwise sends `ClientboundHelloPacket` (server RSA public key + random 4-byte challenge) if `usesAuthentication()` is true and the connection isn't an in-process (singleplayer) memory connection; offline connections synthesize an offline UUID via `UUIDUtil.createOfflineProfile`.
2. `handleKey` — decrypts the client's AES secret key with the server private key, verifies the echoed challenge, derives the legacy `serverId` digest (`Crypt.digestData` over an **empty string** server-id, the server's public key, and the shared secret — the empty-string server-id is intentional, a leftover of Mojang's session-server protocol), switches the connection to encrypted mode, then spawns a dedicated `User Authenticator #N` thread that calls `sessionService.hasJoinedServer(name, digest, address)`.
3. On a successful `ProfileResult`, `serverActivityMonitor.reportLoginActivity()` fires (feeds the JSON-RPC `server/activity` notification, §3.3) and the profile moves to `VERIFYING`. `AuthenticationUnavailableException` and a `null` result both fall back to an offline profile only in singleplayer; otherwise the client is disconnected with `multiplayer.disconnect.unverified_username` / `...authservers_down`.
4. `verifyLoginAndFinishConnectionSetup` (main thread, driven from `tick()` while in `VERIFYING`) runs `PlayerList.canPlayerLogin` — ban list, IP ban list, whitelist, and player-count-limit checks, in that order — then guards against a stale duplicate connection for the same UUID (`disconnectAllPlayersWithProfile`, waits for the dupe's disconnect via `WAITING_FOR_DUPE_DISCONNECT` before proceeding) and rejects if `getIntendedProfileId()` (set on transfer) doesn't match.
5. `finishLoginAndWaitForClient` sends `ClientboundLoginFinishedPacket` and switches the connection to the configuration protocol; `handleLoginAcknowledgement` hands off to `ServerConfigurationPacketListenerImpl`.

`ProfileResolver.Cached` maintains two Guava `LoadingCache`s (by name, by UUID; `expireAfterAccess(10 min)`, `maximumSize(256)`), used by `/fetchprofile` and by profile-key/skull resolution paths — not the login path itself.

Chat-signing / profile public keys: `Services.profileKeySignatureValidator()` builds a `SignatureValidator` from `servicesKeySet.keys(ServicesKeyType.PROFILE_KEY)` (any configured key may verify — `Collection.stream().anyMatch`), used in `ServerGamePacketListenerImpl` to validate an incoming `RemoteChatSession`'s public key signature; `DedicatedServer.enforceSecureChat()` (`enforce-secure-profile && online-mode && services.canValidateProfileKeys()`) gates whether unsigned/expired-key chat sessions are rejected outright. `SignatureValidator` itself (`net.minecraft.util`) is a generic `(payload, signature) -> boolean` functional interface with a `NO_VALIDATION` no-op and two factories: from a raw `PublicKey`+algorithm, or from a `ServicesKeySet`/`ServicesKeyType` pair.

`OldUsersConverter` migrates the pre-UUID plain-text lists (`banned-players.txt`, `banned-ips.txt`, `ops.txt`, `white-list.txt`) to the modern JSON format on first boot, resolving usernames to UUIDs through the same `GameProfileRepository`.

### 3.2 `server.properties` — full option inventory

Backed by `DedicatedServerProperties extends Settings<DedicatedServerProperties>`. Every field is a `Settings<...>.MutableValue<T>` (hot-reloadable via `/reload`-adjacent mechanisms and exposed for live mutation by the JSON-RPC `serversettings/*` methods, §3.3) or a plain `final` value (read once at boot, requires a restart). Table below lists **every** property; "Mutable" = can change without a server restart.

| Key | Default | Mutable | Effect |
|---|---|---|---|
| `online-mode` | `true` | no | Enables Mojang session verification (`hasJoinedServer`) during login |
| `prevent-proxy-connections` | `false` | no | Passes the player's public IP (not the proxy's) to `hasJoinedServer` to defeat proxy-based multi-accounting |
| `server-ip` | `""` | no | Bind address for the game socket (empty = all interfaces) |
| `allow-flight` | `false` | yes | Server-side flight-hack kick threshold toggle |
| `motd` | `"A Minecraft Server"` | yes | Status-ping description |
| `enable-code-of-conduct` | `false` | no | Shows an in-game code-of-conduct acknowledgement prompt |
| `bug-report-link` | `""` | no | URL surfaced in crash/disconnect UI |
| `force-gamemode` | `false` | yes | Forces reconnecting players back to the configured `gamemode` |
| `enforce-whitelist` | `false` | yes | Immediately kicks currently-connected players who are not whitelisted when the whitelist is (re)enabled |
| `difficulty` | `easy` | yes | World difficulty (`peaceful`/`easy`/`normal`/`hard`, also accepts legacy numeric id) |
| `gamemode` | `survival` | yes | Default game mode for new/force-synced players |
| `level-name` | `"world"` | no | Save directory name |
| `server-port` | `25565` | no | Game protocol TCP port |
| `management-server-enabled` | `false` | no | Enables the JSON-RPC management server (§3.3) |
| `management-server-host` | `"localhost"` | no | Bind host for the management server |
| `management-server-port` | `0` | no | Bind port (`0` = OS-assigned ephemeral port) |
| `management-server-secret` | random 40-char alnum | no | Bearer token clients must present; regenerated and persisted to the properties file if absent/invalid |
| `management-server-tls-enabled` | `true` | no | Requires a PKCS12 keystore (`management-server-tls-keystore[-password]`) and serves over HTTPS/WSS |
| `management-server-tls-keystore` | `""` | no | Path to the PKCS12 keystore file |
| `management-server-tls-keystore-password` | `""` | no | Keystore password (overridden by env var `MINECRAFT_MANAGEMENT_TLS_KEYSTORE_PASSWORD` or system property `management.tls.keystore.password`) |
| `management-server-allowed-origins` | `""` | no | Comma-separated `Origin` allow-list for the WebSocket-subprotocol auth path only |
| `announce-player-achievements` | *(unset)* | no | Deprecated/legacy boolean, read via `getLegacyBoolean`, no longer has a first-class effect |
| `enable-query` | `false` | no | Enables the GS4 query/stats UDP listener |
| `query.port` | `25565` | no | GS4 query port |
| `enable-rcon` | `false` | no | Enables the RCON TCP listener |
| `rcon.port` | `25575` | no | RCON port |
| `rcon.password` | `""` | no | RCON auth password |
| `hardcore` | `false` | no | Locks difficulty to Hard and converts player death into a spectator ban-from-world |
| `use-native-transport` | `true` | no | Enables Netty epoll/io_uring native transport where available |
| `spawn-protection` | `16` | yes | Radius (blocks) around world spawn where non-ops cannot build (0 disables) |
| `op-permission-level` | `4` (`OWNER`) | yes | Permission level granted by `/op`, drives `LevelBasedPermissionSet` |
| `function-permission-level` | `2` (`GAMEMASTER`) | no | Permission level `/function`-invoked command sources run at |
| `max-tick-time` | `60000` (ms) | no | Watchdog threshold (`ServerWatchdog`) — a single tick exceeding this forcibly crashes the server |
| `max-chained-neighbor-updates` | `1000000` | no | Cap on chained block-update propagation before it is truncated |
| `rate-limit` | `0` | no | Max incoming packets/sec per connection before disconnect (`0` = unlimited) |
| `command-spam-threshold-seconds` | `10` | no | Command execution rate-limit window |
| `chat-spam-threshold-seconds` | `10` | no | Chat rate-limit window |
| `view-distance` | `10` | yes | Chunk send radius |
| `simulation-distance` | `10` | yes | Chunk tick radius |
| `max-players` | `20` | yes | Player cap (ops may still exceed via `canBypassPlayerLimit`) |
| `network-compression-threshold` | `256` | no | Packet-size threshold above which zlib compression kicks in |
| `broadcast-rcon-to-ops` | `true` | no | Mirrors RCON command output to online ops |
| `broadcast-console-to-ops` | `true` | no | Mirrors console command output to online ops |
| `max-world-size` | `29999984` | no | Clamped to `[1, 29999984]`; hard world-border ceiling |
| `sync-chunk-writes` | `true` | no | Forces synchronous chunk I/O flushes |
| `region-file-compression` | `"deflate"` | no | Anvil region-file compression codec |
| `enable-jmx-monitoring` | `false` | no | Exposes tick-time MBeans over JMX |
| `enable-status` | `true` | yes | Whether the status ping (server-list entry) responds at all |
| `hide-online-players` | `false` | yes | Omits the player sample list from the status response |
| `entity-broadcast-range-percentage` | `100` | yes | Clamped to `[10, 1000]`; scales per-entity-type view distance |
| `text-filtering-config` | `""` | no | External chat-filter service config path |
| `text-filtering-version` | `0` | no | Filter config schema version |
| `resource-pack-id` | `""` | no | UUID for the server resource pack; auto-derived from the URL if empty |
| `resource-pack` | `""` | no | Server resource pack URL |
| `resource-pack-sha1` | `""` | no | SHA-1 of the pack (must match `^[a-fA-F0-9]{40}$`) |
| `resource-pack-hash` | *(unset)* | no | Deprecated alias for `resource-pack-sha1` |
| `require-resource-pack` | `false` | no | Disconnects clients that decline the server pack |
| `resource-pack-prompt` | `""` | no | JSON text component shown alongside the pack prompt |
| `initial-enabled-packs` | vanilla default enabled set | no | Comma-separated datapack ids enabled on first world creation |
| `initial-disabled-packs` | vanilla default disabled set | no | Comma-separated datapack ids disabled on first world creation |
| `player-idle-timeout` | `0` | yes | Minutes of inactivity before auto-kick (`0` disables) |
| `status-heartbeat-interval` | `0` | yes | Seconds between JSON-RPC `server/status` heartbeat notifications (`0` disables, §3.3) |
| `white-list` | `false` | yes | Enables whitelist enforcement |
| `enforce-secure-profile` | `true` | no | Requires a valid Mojang-signed profile public key for chat signing |
| `log-ips` | `true` | no | Includes player IPs in server logs |
| `pause-when-empty-seconds` | `60` | yes | Seconds of zero-players before the world stops ticking |
| `level-seed` | `""` | no | World seed (parsed as long or hashed string; random if empty) |
| `generate-structures` | `true` | no | Master structure-generation toggle |
| `generator-settings` | `"{}"` | no | JSON generator settings (used for the `flat` preset) |
| `level-type` | `"minecraft:normal"` | no | `WorldPreset` id; accepts legacy names `default`/`largebiomes` |
| `accepts-transfers` | `false` | yes | Whether the server accepts inbound player-transfer packets |

### 3.3 JSON-RPC server management API

A Netty pipeline (`ManagementServer.start`) serves HTTP + WebSocket on `management-server-host:management-server-port`: `HttpServerCodec → HttpObjectAggregator(65536) → AuthenticationHandler → WebSocketServerProtocolHandler("/") → WebSocketFrameAggregator(65536) → WebSocketToJsonCodec → JsonToWebSocketEncoder → Connection`. TLS is layered in front of `HttpServerCodec` when `management-server-tls-enabled` (the default) is true, loaded from a PKCS12 keystore via `JsonRpcSslContextProvider`.

**Auth** (`AuthenticationHandler`, a `@Sharable` duplex handler intercepting `HttpRequest`): accepts either an `Authorization: Bearer <secret>` header (any origin) or the secret smuggled as a `Sec-WebSocket-Protocol: minecraft-v1,<secret>` value (only from an `Origin` present in `management-server-allowed-origins`); the secret is compared with `MessageDigest.isEqual` (constant-time). A per-channel `authenticated` attribute gates every subsequent inbound message; failure sends `401` JSON and closes the channel. On the WebSocket-subprotocol path the server echoes `Sec-WebSocket-Protocol: minecraft-v1` back so the upgrade succeeds.

**Message model**: standard JSON-RPC 2.0. `Connection.handleJsonObject` classifies each object as a request (`method` present), a response (`result` present, `id` numeric and known), or an error, and supports batched arrays. Standard error codes (`JsonRPCErrors`): `-32700` parse, `-32600` invalid request, `-32601` method not found, `-32602` invalid params, `-32603` internal.

**Method registry**: every incoming method is a `BuiltInRegistries.INCOMING_RPC_METHOD` entry keyed by an `Identifier` (e.g. `minecraft:players/kick`), registered via a fluent builder in `IncomingRpcMethods.bootstrap` (`.description(...)`, `.param(name, schema)`, `.response(name, schema)`, `.allowPreServerInit()`, `.notOnMainThread()`, `.undiscoverable()`). Each `IncomingRpcMethod.Attributes` controls whether the handler runs on the main server thread (`minecraftApi.submit(...).join()`, the default — most handlers touch `PlayerList`/`GameRules`) or off-thread, and whether it's callable before the dedicated server has finished starting. **85 methods** are registered as of 26.2, covering allowlist/bans/ip_bans/operators (list+set+add+remove+clear each), player list/kick, server status/save/stop/system_message, 20 `serversettings/*` get+set pairs mirroring the mutable `server.properties` fields (difficulty, allow-flight, motd, spawn-protection, gamemode, view/simulation-distance, max-players, player-idle-timeout, status-heartbeat-interval, force-gamemode, accept-transfers, operator permission level, hide-online-players, enable-status ("status_replies"), entity-broadcast-range, pause-when-empty-seconds), gamerules get+update, and one meta-method `rpc.discover` that returns a live-generated OpenRPC 1.3.2 document (info, methods, component schemas) — the exact document datagen dumps to `json-rpc-api-schema.json` (§7).

**Notifications** (server → client, no response expected) are declared the same way in `OutgoingRpcMethods` (e.g. `server/started`, `server/stopping`, `server/saving`, `server/saved`, `server/activity` — rate-limited to one per 30s by `ServerActivityMonitor` — `players/joined`, `players/left`, `operators/added`/`removed`, `allowlist/added`/`removed`, `ip_bans/added`/`removed`, `bans/added`/`removed`, `gamerules/updated`, and `server/status`, the periodic heartbeat gated by `status-heartbeat-interval` seconds and the only notification with `allowPreServerInit()` so it can report "not yet started" status). `NotificationManager` is a single event bus (`playerJoined`, `serverStarted`, `onGameRuleChanged<T>`, …) that fans out to every registered `NotificationService`; `JsonRpcNotificationService` (constructed in `JsonRpc.create`) is one such subscriber and, on each event, calls `Connection.sendNotification` on every currently-open management connection (`ManagementServer.forEachConnection`). `StoredUserList` subclasses (bans/allowlist/oplist) also hold a `NotificationService` reference so file-backed list mutations (even from `/ban`, `/whitelist` commands, not just RPC) surface as notifications.

**RPC request lifecycle for outgoing (server-initiated) requests**: `Connection.sendRequest` assigns an incrementing transaction id, stores a `PendingRpcRequest` with a 5000 ms deadline, and `Connection.tick()` (driven from `ManagementServer.tick()`, itself driven once per server tick from `DedicatedServer.tickServer` — see leftover note in §2) sweeps expired entries and fails their `CompletableFuture` with `ReadTimeoutException`.

`SecurityConfig.generateSecretKey()` produces a 40-char `[A-Za-z0-9]` token (`isValid` enforces the same shape); `JsonRpc.create` throws `IllegalStateException` at boot if `management-server-secret` is set but doesn't match, forcing an explicit fix rather than silently generating a new one.

### 3.4 Console, GUI and remote-control surfaces

`MinecraftServerGui` is a plain Swing `JFrame` (854×480) built from a chat/log panel (`JTextArea`, monospaced, fed by `LogQueues.getNextLogEvent("ServerGuiConsole")` on a dedicated "Server log monitor" daemon thread) plus a west-side info panel (`StatsComponent` — CPU/memory/tick charts — and `PlayerListComponent`). The chat text field forwards typed lines straight to `server.handleConsoleInput(text, server.createCommandSourceStack())`, i.e. it is just an alternate console, not a distinct permission surface. Closing the window calls `server.halt(true)`.

RCON (`net.minecraft.server.rcon` + `.rcon.thread`) and the GS4 query/stats listener (`QueryThreadGs4`) are protocol-level surfaces and are treated as owned by the NET/protocol document; they are noted here only for completeness of the `net/minecraft/server` package map.

`ChaseServer`/`ChaseClient` (`net.minecraft.server.chase`, driven by `/chase lead|follow|stop`) implement a bespoke, **unauthenticated**, line-based TCP protocol (`t <dim> <x> <y> <z> <yaw> <pitch>\n`) that broadcasts the position/rotation of `playerList.getPlayers().get(0)` (the first player in iteration order — not a chosen "leader") to any connected client every 100 ms; a follower server applies the received transform to move its own camera/player in lock-step. This exists for external camera rigs (marketing captures) and has no bearing on gameplay determinism, but it is a raw socket with no auth token bound to an op-only command — worth flagging as a security-relevant surface if reimplemented.

### 3.5 World border

`WorldBorder` (`SavedData`, one instance per level via `SavedDataType`) stores center (`centerX`/`centerZ`), `damagePerBlock` (default `0.2`), `safeZone` (default `5.0`, blocks outside the border edge before damage starts), `warningTime` (default `15`, seconds), `warningBlocks` (default `5`), an `absoluteMaxSize` (default `29999984`, mirrors `max-world-size`), and a pluggable `BorderExtent` strategy: `StaticBorderExtent` (fixed size, precomputed AABB/`VoxelShape`) or `MovingBorderExtent` (linear lerp between `from`/`to` over a tick count, computed from elapsed `gameTime` so it survives save/reload without drift — `calculateSize()` is `progress = (duration - remaining)/duration`, clamped at `1.0`). `tick()` (called once per level tick, §"cross-subsystem interfaces") simply asks the current extent to `update()`, which for a moving extent decrements `lerpProgress` and swaps itself for a `StaticBorderExtent` once the lerp completes. Collision uses `Shapes.join(Shapes.INFINITY, box(...), ONLY_FIRST)` — an infinite solid with the in-bounds region subtracted out, floor/ceil-aligned to block boundaries. `MAX_SIZE = 5.999997E7`, `MAX_CENTER_COORDINATE = 2.9999984E7`.

### 3.6 Weather cycle

Two independent 0–1 floats, `rainLevel`/`thunderLevel`, ramp by ±0.01 per tick toward boolean targets (`isRaining`/`isThundering`) stored in a per-server `WeatherData` saved-data object (shared, not per-level — see cross-interfaces). `ServerLevel.advanceWeatherCycle()` (called from `tick()` when `TickRateManager.runsNormally()`) runs a small RNG state machine per tick when `canHaveWeather()`:
- If `clearWeatherTime > 0` (set by `/weather clear <duration>`), it decrements and forces both rain and thunder off, skipping the random schedule entirely.
- Otherwise `thunderTime`/`rainTime` count down independently; at zero the corresponding boolean flips and a new duration is sampled: `THUNDER_DURATION = UniformInt(3600, 15600)` ticks (3–13 min) when turning thunder *on*, `THUNDER_DELAY = UniformInt(12000, 180000)` (10 min–2.5 h) when turning it *off*; `RAIN_DURATION = UniformInt(12000, 24000)` (10–20 min) and `RAIN_DELAY = UniformInt(12000, 180000)` symmetrically.
- Level-change broadcasts (`ClientboundGameEventPacket` RAIN_LEVEL_CHANGE/THUNDER_LEVEL_CHANGE) fire only when the smoothed float actually changes; a raining↔clear transition additionally rebroadcasts both levels to the whole player list (not just the current dimension) so joining/other-dimension clients stay in sync.

Lightning: `tickThunder(chunk)` (per loaded chunk, per tick) rolls `random.nextInt(100000) == 0` only while `isRaining() && isThundering()`; on a hit it finds a strike target via `findLightningTargetAround`, and — gated by `GameRules.SPAWN_MOBS` and a `difficulty.getEffectiveDifficulty() * 0.01` chance, only if the block below isn't tagged `lightning_rods` — may spawn a trapped `SkeletonHorse` in addition to the visible `LightningBolt` (a "skeleton horse trap").

Sleep-based skip: when `sleepStatus.areEnoughSleeping/areEnoughDeepSleeping` (percentage from `GameRules.PLAYERS_SLEEPING_PERCENTAGE`) pass, the level wakes all sleepers, advances the default clock to the `WAKE_UP_FROM_SLEEP` time marker (§3.7, gated by `GameRules.ADVANCE_TIME`), and — gated by `GameRules.ADVANCE_WEATHER` — resets the weather cycle to clear if it was raining.

### 3.7 World clocks & timelines

26.2 replaces the historically hardcoded per-dimension day counter with a fully data-driven abstraction, exposed through three **unstable (`stable: false`) datapack registries**: `world_clock`, `timeline`, and the code-only `clock_time_marker`.

- **`WorldClock`** (`net.minecraft.world.clock`) is just a named registry key (`WorldClocks.OVERWORLD`, `WorldClocks.THE_END` are the two vanilla instances) — an *identity*, not a schedule.
- **`ServerClockManager`** (`SavedData`, world-level singleton via `server.clockManager()`) owns one mutable `ClockInstance` per registered `WorldClock`: `totalTicks` (monotonic, never resets across days — unlike the old `dayTime % 24000`), a fractional `partialTick` accumulator, a `rate` multiplier (default `1.0`), and a `paused` flag. `tick()` runs only when `GameRules.ADVANCE_TIME` is set; each instance advances `partialTick += rate`, folds whole ticks into `totalTicks`. Mutating a clock (`setTotalTicks`, `addTicks`, `setRate`, `setPaused`, `moveToTimeMarker`) immediately broadcasts a `ClientboundSetTimePacket` diff to all players and invalidates every level's `EnvironmentAttributeSystem` tick cache.
- **`Timeline`** (`net.minecraft.world.timeline`, datapack JSON under `data/minecraft/timeline/*.json`) binds to exactly one `WorldClock`, optionally declares a `period_ticks` (e.g. the overworld day is `24000`), and carries two independent payloads: named **time markers** (`ClockTimeMarker`: an offset within the period, e.g. `minecraft:day` at tick 1000, `minecraft:noon` at 6000, `minecraft:night` at 13000, `minecraft:midnight` at 18000, plus the non-visual `wake_up_from_sleep` and `roll_village_siege` markers) and **attribute tracks** (`AttributeTrack`, keyframed values of an `EnvironmentAttribute` — sky color, fog color, sky-light factor, star brightness, whether fireflies/bees/creakings/eyeblossoms are active, monster-burn window, cat wake-gift chance, turtle-egg hatch chance — sampled and combined via a `modifier` such as `multiply`/`maximum`/`or`/`override`, with optional easing incl. cubic-bezier). Multiple timelines can layer onto the same clock (e.g. `day`, `moon`, `villager_schedule`, `early_game` all target `minecraft:overworld`); `Timeline.validateRegistry` rejects two timelines defining the same time marker for the same clock.
- Consumers ask `ClockManager.getTotalTicks(clock)` or, for marker-relative logic, `ServerClockManager.isAtTimeMarker`/`moveToTimeMarker`/`commandTimeMarkersForClock`. `VillageSiege` (§3.9) is a concrete example: it no longer checks a raw `dayTime` modulo, it checks `clockManager().isAtTimeMarker(defaultClock, ClockTimeMarkers.ROLL_VILLAGE_SIEGE)`.
- The GameTest framework has a first-class `TestEnvironmentDefinition.ClockTime` hook that pins a clock to a specific tick for the test's duration and restores it afterward (§3.11).

### 3.8 Raids

`Raids` (`SavedData`, one per level, `raidMap: Int2ObjectMap<Raid>`) ticks every active raid once per level tick and removes stopped ones. A raid is created/extended by `createOrExtendRaid(player, pos)`, invoked from `RaidOmenMobEffect.applyEffectTick` (fires on the *last* tick of the `Raid Omen` effect via `shouldApplyEffectTickThisTick(remaining == 1)`), gated by `GameRules.RAIDS` and `EnvironmentAttributes.CAN_START_RAID` at the position. The raid center is the centroid of nearby occupied village POIs within 64 blocks (falling back to the trigger position if none).

`Raid` state machine (`RaidStatus`: `ONGOING → VICTORY|LOSS → STOPPED`, plus a direct `stop()` from any state):
- `absorbRaidOmen(player)` adds `effect.getAmplifier() + 1` to `raidOmenLevel`, clamped to `[0, 5]` (`DEFAULT_MAX_RAID_OMEN_LEVEL = 5`); the omen level scales both wave count (`getNumGroups`: peaceful 0, easy 3, normal 5, hard 7 — difficulty-based, not omen-based) and post-victory enchant odds (`getEnchantOdds`: 0/0/10%/25%/50%/75% for omen level 0–5).
- Per-tick: deactivates if the chunk at center isn't loaded; stops outright on `Difficulty.PEACEFUL`; relocates center toward a nearby village section if the current center stopped being a village, losing the raid (if any wave already spawned) or stopping it (if not) when no village can be found; times out after `RAID_TIMEOUT_TICKS = 48000` (40 min real-time); when all current-wave raiders are dead and more waves remain, waits `raidCooldownTicks = 300` ticks (15 s) before spawning the next wave, searching for a valid spawn position every 5 ticks of the cooldown.
- Wave composition (`RaiderType.spawnsPerWaveBeforeBonus[8]`, indexed by wave 1–7 plus a synthetic bonus-wave slot) is a fixed per-difficulty/per-wave table baked into the enum, e.g. `PILLAGER = {0,4,3,3,4,4,4,2}`, `VINDICATOR = {0,0,2,0,1,4,2,5}`, `RAVAGER = {0,0,0,1,0,1,0,2}`, `EVOKER = {0,0,0,0,0,1,1,2}`, `WITCH = {0,0,0,0,3,0,0,1}`; `getPotentialBonusSpawns` layers difficulty-dependent extra rolls on top (e.g. Hard pillagers/vindicators always get `+[0,2]` bonus spawns).
- Victory requires the raid to have started, no more waves, zero raiders alive, sustained for `POST_RAID_TICKS = 40` ticks; grants `HERO_OF_THE_VILLAGE_DURATION = 48000` ticks (40 min) of Hero of the Village at amplifier `raidOmenLevel - 1` to every tracked hero. A post-raid celebration/defeat bossbar lingers for `MAX_CELEBRATION_TICKS = 600` ticks (30 s) before the raid object is finally discarded.
- `VALID_RAID_RADIUS_SQR = 9216` (96²) / `RAID_REMOVAL_THRESHOLD_SQR = 12544` (112²) bound how far a player can be from the raid center and still count as participating vs. cause raider despawn (`Raids.canJoinRaid` also requires `getNoActionTime() <= 2400`, i.e. 2 min without acting on the target).

### 3.9 Ambient/event custom spawners

All five are `CustomSpawner`s registered **only for the Overworld level** (`MinecraftServer.createLevels`, `overworldCustomSpawners`); non-overworld dimensions get an empty spawner list, so none of the mechanics below can occur in the Nether/End regardless of game rules.

| Spawner | Gate (`GameRules`) | Cadence | Notes |
|---|---|---|---|
| `PatrolSpawner` | `SPAWN_PATROLS` | every `12000 + rand(1200)` ticks (~10 min ± 1 min), only if daytime (`isBrightOutside`), then 1-in-5 roll, then requires a random online non-spectator player not within 2 chunks of a village | Group size = `ceil(effectiveDifficulty) + 1` pillagers spawned 24–48 blocks from the player; first spawned is the patrol leader (`setPatrolLeader` + `findPatrolTarget`) |
| `PhantomSpawner` | `SPAWN_PHANTOMS` | every `(60 + rand(60)) * 20` ticks (~1–2 min) | Requires sky-darken ≥ 5 or a no-sky-light dimension, per-player: needs `time_since_rest` stat roll `random.nextInt(clamp(value,1,MAX)) >= 72000` (statistically the stat must exceed several days of no-sleep before phantoms become likely), spawns above the player, group size `1 + rand(difficulty.id + 1)` |
| `CatSpawner` | none (no game rule) | fixed `1200`-tick interval | Spawns near a random player; in a village (≥4 occupied `home` POIs within 48, capped at 5 nearby cats) or a hut structure (tag `cats_spawn_in`, requires zero existing cats within 16, spawned as persistence-required) |
| `VillageSiege` | none directly (uses clock/attribute system) | rolled once per Overworld day at the `ROLL_VILLAGE_SIEGE` time marker (§3.7), 1-in-10 chance | On success, picks a village-adjacent player, spawns 20 zombies in bursts of 1 every 2 ticks around a ring 32 blocks from the player; biome tag `without_zombie_sieges` opts a biome out |
| `WanderingTraderSpawner` | `SPAWN_WANDERING_TRADERS` | tick-delay `1200`, real spawn delay counted down from `WanderingTraderData` (persisted `SavedData`), default `24000` ticks (20 min) | Spawn chance starts at `25%`, `+25` per elapsed cycle up to `75%` cap, resets to `25%` on a successful spawn; picks a random online player, prefers a `meeting` POI within 48 blocks, spawns 2 leashed trader llamas, sets 48000-tick despawn delay |

### 3.10 Permission model

`Permission` (sealed-ish via subtypes `Permission.HasCommandLevel` and `Permission.Atom`) is the fine-grained capability unit; `PermissionSet` (`hasPermission(Permission)`, `union`) is the thing a command source actually holds. `LevelBasedPermissionSet` (used for ops: `ALL`/`MODERATOR`/`GAMEMASTER`/`ADMIN`/`OWNER`, backing `PermissionLevel` ids 0–4) implements `hasPermission` by comparing `PermissionLevel.isEqualOrHigherThan` for `HasCommandLevel` permissions, and special-cases `Permissions.COMMANDS_ENTITY_SELECTORS` to require at least `GAMEMASTERS`. Named atoms in `Permissions` cover entity-selector usage and four chat capabilities (`send_messages`, `send_commands`, `receive_player_messages`, `receive_system_messages`) — the latter group exists so a permission provider (e.g. a mod, or a future non-level-based grant) can restrict chat independently of command level. `op-permission-level` and `function-permission-level` in `server.properties` both deserialize straight into a `LevelBasedPermissionSet` via `PermissionLevel.byId`.

### 3.11 GameTest framework

A registry-driven automated in-game test system, distinct from unit tests — tests place a structure, run for a bounded number of ticks, and assert conditions via a `GameTestHelper` fluent API.

- **`GameTestInstance`** (registry `TEST_INSTANCE_TYPE`, two built-in kinds: `block_based` and `function`) is a datapack-defined **`test_instance`** entry (registry `test_instance`, `stable: false`) carrying a `TestData<Holder<TestEnvironmentDefinition<?>>>`: which structure NBT to load, which `TestEnvironmentDefinition` ("batch") to apply, `maxTicks`/`setupTicks`, `required`/`manualOnly`, `maxAttempts`/`requiredSuccesses`, `skyAccess`, `rotation`, `padding`.
- **`TestEnvironmentDefinition`** (registry `test_environment`, `stable: false`, dispatch types registered in its own `bootstrap`) composes setup/teardown pairs: `all_of` (sequential composite, tears down in reverse), `clock_time` (pins a `WorldClock` via `ServerClockManager.setTotalTicks`, §3.7), `difficulty`, `function` (runs a `CommandFunction` with gamemaster permission and suppressed output), `game_rules` (snapshot+restore a `GameRuleMap`), `timeline_attributes` (layers extra `Timeline`s onto the level's `EnvironmentAttributeSystem` for the test's duration), `weather` (forces `clear`/`rain`/`thunder` via `setWeatherParameters`, restoring the prior state after).
- **Execution**: `GameTestTicker.SINGLETON` is driven once per server tick from `MinecraftServer.tickServer` (leftover, §2); it ticks every registered `GameTestInfo` and removes finished ones. A `GameTestRunner` groups tests into `GameTestBatch`es (`GameTestBatchFactory`), spawns each test's structure via a `StructureSpawner` (`StructureGridSpawner` lays tests out in an 8-per-row grid — `DEFAULT_TESTS_PER_ROW = 8`, 5-block column gaps, 6-block row gaps — optionally clearing the area between batches), and reports pass/fail through `GameTestListener`/`GameTestBatchListener` to a `TestReporter` (`LogTestReporter`, `JUnitLikeTestReporter`, or a `GlobalTestReporter` fan-out).
- `GameTestServer` (`net.minecraft.gametest.Main`) is a standalone entry point that boots a headless server purely to run a batch of tests and exit — the mechanism CI-style test suites are expected to use.

### 3.12 Debug subscription system

Replaces the old fire-and-forget static broadcaster with an explicit client-opt-in model. Clients send `ServerboundDebugSubscriptionRequestPacket` naming a `DebugSubscription<T>` (registry `DEBUG_SUBSCRIPTION`, e.g. `bees`, `brains`, `breezes`, `goal_selectors`, `entity_paths`, `bee_hives`, `pois`, `raids`, `structures`, `game_event_listeners`, `village_sections`, plus temporary/expiring ones: `entity_block_intersections` (100-tick TTL), `redstone_wire_orientations` (200 ticks), `neighbor_updates` (200 ticks), `game_events` (60 ticks), and a valueless `dedicated_server_tick_time`). `ServerDebugSubscribers.tick()` (once per server tick) rebuilds a `subscription → [ServerPlayer]` map from each connected player's live `debugSubscriptions()` set; access is gated by `hasRequiredPermissions` (must be op, or the singleplayer owner running from an IDE build). `LevelDebugSynchronizers`/`TrackingDebugSynchronizer` then push `ClientboundDebug*Packet`s only to that tick's actual subscribers instead of broadcasting unconditionally — e.g. `ServerLevel.tick()` only sets a neighbor-update debug listener at all when `hasAnySubscriberFor(NEIGHBOR_UPDATES)` is true, so the feature is zero-cost when nobody is watching.

### 3.13 Crash reports & watchdog

`CrashReport` (title + `Throwable` + ordered `List<CrashReportCategory>` + a `SystemReport`) builds a human-readable dump: an auto-derived "-- Head --" stack trace from the first-added category's `fillInStackTrace`, then each category's key/value details, then the system report. `addCategory` walks the real exception's stack trace to align each category with the code path that triggered it, disabling further stack-trace correlation (`trackingStackTrace = false`) if the arithmetic goes out of bounds — a defensive fallback rather than a hard failure. `saveToFile` writes to `<world>/crash-reports/crash-<timestamp>-<label>.txt` and is idempotent (refuses a second save target).

`ServerWatchdog` (`Runnable`, one dedicated thread) polls `server.getNextTickTime()` against `Util.getNanos()`; once a single tick has run longer than `max-tick-time` (default 60000 ms) it treats the server as hung: dumps a full thread listing (sorted daemon-then-state-then-name) into a synthetic `Error("Watchdog (...)")` crash report (`ServerWatchdog.createWatchdogCrashReport`), attaches performance stats (random-tick-speed rule value, per-level watchdog stats), prints and saves it, then force-exits via `System.exit(1)` racing a 10-second `Runtime.halt(1)` fallback timer in case shutdown itself hangs.

### 3.14 Feature flags

`FeatureFlagSet` is a `long`-backed bitset (`FeatureFlag.mask = 1L << bit`) scoped to a `FeatureFlagUniverse` ("main"). 26.2 defines exactly **four** flags, all built with `builder.createVanilla(...)` (meaning: shipped, non-experimental, always in `VANILLA_SET`): `vanilla`, `trade_rebalance`, `redstone_experiments`, `minecart_improvements`. `DEFAULT_FLAGS = VANILLA_SET` (i.e. only `vanilla` is on by default; the other three are opt-in per-world via `WorldDataConfiguration`/datapack). `FeatureFlags.isExperimental(set)` is simply "not a subset of `VANILLA_SET`". Elements/blocks/items/recipes gated behind an unset flag are excluded from registries at world-load time — this is a data-availability gate, not a runtime permission check.

### 3.15 Ban/allow/op lists

`StoredUserList<K, V extends StoredUserEntry<K>>` is the shared abstract base (Gson-backed JSON file, in-memory `Map<String, V>` keyed by a string form of the key, pretty-printed on save) for `UserBanList` (`banned-players.json`), `IpBanList` (`banned-ips.json`), `ServerOpList` (`ops.json`), `UserWhiteList` (`whitelist.json`). Each holds a `NotificationService` reference so every mutation — from a command *or* from JSON-RPC — surfaces through the same `NotificationManager` fan-out described in §3.3. Entry types (`UserBanListEntry`, `IpBanListEntry`, `ServerOpListEntry`, `UserWhiteListEntry`) share a `created`/`source`/`expires`/`reason` shape via `BanListEntry`/`StoredUserEntry`; op entries additionally carry a `PermissionLevel` and a bypass-player-limit flag (the JSON-RPC `operator` schema, §7, mirrors this exactly).

## 4. Key types

| Class (package) | Role | Notable details |
|---|---|---|
| `Services` (`server`) | Bundles all Mojang-service handles at boot | `record(sessionService, servicesKeySet, profileRepository, nameToIdCache, profileResolver)`; `profileKeySignatureValidator()`, `canValidateProfileKeys()` |
| `ServerLoginPacketListenerImpl` (`server.network`) | Login handshake state machine | `State{HELLO,KEY,AUTHENTICATING,NEGOTIATING,VERIFYING,WAITING_FOR_DUPE_DISCONNECT,PROTOCOL_SWITCHING,ACCEPTED}`; spawns one `User Authenticator #N` thread per login |
| `ProfileResolver.Cached` (`server.players`) | Name/UUID→`GameProfile` cache | Two Guava `LoadingCache`s, 10 min access-expiry, 256-entry cap each |
| `CachedUserNameToIdResolver` (`server.players`) | `usercache.json` persistence | 1000-entry MRU cap on save, 1-month entry expiry |
| `DedicatedServerProperties` (`server.dedicated`) | `server.properties` schema | ~55 fields, see full table §3.2; `WorldDimensionData` nested record resolves `level-type`/`generator-settings` into a `WorldDimensions` |
| `Settings<T>` (`server.dedicated`) | Generic typed-properties base | `MutableValue<T>` inner class for hot-reloadable fields; `get`/`getMutable`/`getLegacyBoolean`/`getLegacyString` |
| `ManagementServer` (`server.jsonrpc`) | Netty bootstrap + connection registry | `startWithTls`/`startWithoutTls`; `scheduleHeartbeat`; `tick()` fans to every `Connection` |
| `Connection` (`server.jsonrpc`) | Per-socket JSON-RPC state machine | `pendingRequests: Int2ObjectMap<PendingRpcRequest<?>>`; `dispatchIncomingRequest` looks up `BuiltInRegistries.INCOMING_RPC_METHOD` by `Identifier` |
| `AuthenticationHandler` (`server.jsonrpc.security`) | Bearer/subprotocol auth gate | Constant-time compare (`MessageDigest.isEqual`); origin allow-list only applies to the WS-subprotocol path |
| `IncomingRpcMethod<Params,Result>` / `OutgoingRpcMethod<Params,Result>` (`server.jsonrpc`) | RPC method descriptor | Fluent builder; `Attributes{discoverable, allowPreServerInit, runOnMainThread}` |
| `NotificationManager` (`server.notifications`) | Central event bus | Implements `NotificationService` itself and fans every call out to `registerService`d listeners |
| `MinecraftServerGui` (`server.gui`) | Swing console | `showFrameFor(server)`; log pump thread reads `LogQueues.getNextLogEvent` |
| `ChaseServer`/`ChaseClient` (`server.chase`) | Ad-hoc camera-sync protocol | Unauthenticated line protocol `t <dim> <x> <y> <z> <yaw> <pitch>`; 100 ms broadcast interval |
| `WorldBorder` (`world.level.border`) | Border shape + lerp | `BorderExtent` strategy interface; `Settings` record is the persisted snapshot |
| `Raid` (`world.entity.raid`) | Raid state machine | `RaidStatus{ONGOING,VICTORY,LOSS,STOPPED}`; `RaiderType` enum embeds the wave-composition table |
| `Raids` (`world.entity.raid`) | Per-level raid registry | `SavedData`; `createOrExtendRaid`, `getNearbyRaid`, `canJoinRaid` |
| `ServerClockManager` (`world.clock`) | Mutable clock state per `WorldClock` | `SavedData`; `ClockInstance{totalTicks,partialTick,rate,paused}`; broadcasts `ClientboundSetTimePacket` on any mutation |
| `Timeline` (`world.timeline`) | Datapack keyframe schedule bound to a clock | `Builder`; `AttributeTrack` per `EnvironmentAttribute`; validates marker ranges against `period_ticks` |
| `FeatureFlagSet`/`FeatureFlags` (`world.flag`) | Experimental-content bitset | 4 vanilla flags; `isExperimental = !isSubsetOf(VANILLA_SET)` |
| `GameTestInstance` (`gametest.framework`) | Registry-defined single test | Abstract; `block_based`/`function` concrete kinds; carries a `TestData<Holder<TestEnvironmentDefinition<?>>>` |
| `TestEnvironmentDefinition<T>` (`gametest.framework`) | Composable setup/teardown for a test batch | `AllOf`, `ClockTime`, `Functions`, `SetDifficulty`, `SetGameRules`, `Timelines`, `Weather` |
| `GameTestTicker` (`gametest.framework`) | Global test-tick driver | Singleton; `IDLE/RUNNING/HALTING` state guards re-entrant `clear()` |
| `DebugSubscription<T>` (`util.debug`) | Registry entry for one debug data channel | Optional `StreamCodec<T>` (null = valueless ping); optional `expireAfterTicks` |
| `ServerDebugSubscribers` (`util.debug`) | Per-tick subscriber roster | Rebuilt every tick from each player's live subscription set |
| `CrashReport`/`CrashReportCategory` (`net.minecraft`) | Structured crash dump | `addCategory` correlates each category to a stack-trace slice of the root exception |
| `ServerWatchdog` (`server.dedicated`) | Hung-tick killer | Separate thread; force-exits with a 10 s `Runtime.halt` fallback |
| `Permission`/`PermissionSet`/`LevelBasedPermissionSet` (`server.permissions`) | Fine-grained + level-based authorization | `HasCommandLevel` vs. named `Atom`s; `union` picks the stricter (numerically lower) level |

## 5. Constants & magic values

| Value | Meaning | Source |
|---|---|---|
| `MAX_TICKS_BEFORE_LOGIN = 600` | Login handshake timeout (ticks, ~30 s) | `ServerLoginPacketListenerImpl` |
| RPC secret length `40`, charset `[A-Za-z0-9]` | `management-server-secret` shape | `SecurityConfig` |
| `65536` | Max HTTP/WebSocket aggregated frame size for the management server | `ManagementServer` |
| RPC request timeout `5000` ms | Deadline for a server-initiated RPC awaiting a client reply | `Connection.sendRequest` |
| `server/activity` rate limit `30` s | Hardcoded, not configurable | `MinecraftServer` (`new ServerActivityMonitor(nm, 30)`) |
| `MAX_SIZE = 5.999997E7` | World border absolute max size (blocks) | `WorldBorder` |
| `MAX_CENTER_COORDINATE = 2.9999984E7` | World border center coordinate bound | `WorldBorder` |
| `damagePerBlock` default `0.2`, `safeZone` default `5.0` | Border damage model defaults | `WorldBorder` |
| `RAIN_DELAY = [12000, 180000]`, `RAIN_DURATION = [12000, 24000]` | Weather cycle rain timing (ticks) | `ServerLevel` |
| `THUNDER_DELAY = [12000, 180000]`, `THUNDER_DURATION = [3600, 15600]` | Weather cycle thunder timing (ticks) | `ServerLevel` |
| Weather smoothing rate `±0.01`/tick | `rainLevel`/`thunderLevel` ramp | `ServerLevel.advanceWeatherCycle` |
| Lightning roll `1/100000` per chunk per tick while raining+thundering | Strike probability | `ServerLevel.tickThunder` |
| `RAID_TIMEOUT_TICKS = 48000` | Raid hard timeout (40 min) | `Raid` |
| `DEFAULT_MAX_RAID_OMEN_LEVEL = 5` | Raid Omen cap | `Raid` |
| Raid cooldown between waves `300` ticks (15 s) | | `Raid` |
| `POST_RAID_TICK_LIMIT = 40`, `MAX_CELEBRATION_TICKS = 600` | Victory confirmation / bossbar linger | `Raid` |
| `HERO_OF_THE_VILLAGE_DURATION = 48000` | Hero effect duration (ticks) | `Raid` |
| `VALID_RAID_RADIUS_SQR = 9216`, `RAID_REMOVAL_THRESHOLD_SQR = 12544` | 96²/112² participation & despawn radii | `Raid` |
| Raid wave counts by difficulty `{0,3,5,7}` | Peaceful/Easy/Normal/Hard | `Raid.getNumGroups` |
| `PatrolSpawner` cadence `12000 + rand(1200)` | ~10 min ± 1 min | `PatrolSpawner` |
| `PhantomSpawner` cadence `(60 + rand(60)) * 20` | ~1–2 min | `PhantomSpawner` |
| `CatSpawner` cadence `1200` ticks | Fixed | `CatSpawner` |
| `WanderingTraderSpawner` default spawn-delay `24000` ticks, chance `25%→75%` step `+25%` | 20 min cycle | `WanderingTraderSpawner` |
| `GameTestRunner.DEFAULT_TESTS_PER_ROW = 8` | GameTest structure grid width | `GameTestRunner` |
| `StructureGridSpawner` spacing `5` (columns) / `6` (rows) | Block gap between test structures | `StructureGridSpawner` |
| Debug-subscription TTLs: `entity_block_intersections=100`, `redstone_wire_orientations=200`, `neighbor_updates=200`, `game_events=60` (ticks) | Auto-expiry for transient debug channels | `DebugSubscriptions` |
| `max-tick-time` default `60000` ms | Watchdog crash threshold | `DedicatedServerProperties` / `ServerWatchdog` |
| Watchdog shutdown fallback `10000` ms | Forced `Runtime.halt` if graceful `System.exit` stalls | `ServerWatchdog` |
| Feature flag count `4` (`vanilla`, `trade_rebalance`, `redstone_experiments`, `minecart_improvements`) | Current 26.2 flag set | `FeatureFlags` |
| Overworld day period `24000` ticks | `period_ticks` in `world_clock/overworld.json` | datagen data |

## 6. Cross-subsystem interfaces

**Consumes:**
- **Tick pipeline (01-ARCH)** — `WorldBorder.tick()`, `advanceWeatherCycle()`, `Raids.tick()`, and `ServerClockManager.tick()` are all invoked once per level tick from `ServerLevel.tick()`; `GameTestTicker.SINGLETON.tick()` and `ManagementServer.tick()` are invoked once per **server** tick from `MinecraftServer`/`DedicatedServer`. A reimplementation must preserve this ordering (border → weather → sleep/clock → time → block/fluid ticks → raids → chunk source → block events) since raids and weather both read `getGameRules()` state that other systems in the same tick may have just changed.
- **Protocol/connection layer (02-NET)** — the entire login handshake (§3.1) lives in the packet-listener state machine; RCON/GS4 share the same "misc server surface" package tree but are protocol concerns owned there.
- **World/persistence (03-WORLD)** — `WorldBorder`, `Raids`, `ServerClockManager`, `WanderingTraderData` are all `SavedData` and go through the same Anvil-adjacent save-data storage as chunks/NBT.
- **Game mechanics (05-MECH)** — `GameRules` gate almost every mechanism here (`RAIDS`, `SPAWN_PATROLS`, `SPAWN_PHANTOMS`, `SPAWN_WANDERING_TRADERS`, `ADVANCE_TIME`, `ADVANCE_WEATHER`, `PLAYERS_SLEEPING_PERCENTAGE`, `LOCATOR_BAR`, `MAX_SNOW_ACCUMULATION_HEIGHT`, `SPAWN_MOBS`); `EnvironmentAttributes` (`CAN_START_RAID`, `CAN_PILLAGER_PATROL_SPAWN`) is consumed from world.attribute, itself fed by `Timeline` tracks.
- **Modding API (06-MOD)** — `FeatureFlagSet` gates which registry content a mod-contributed datapack element requires; `NotificationService` is the natural extension point for a mod wanting server-lifecycle hooks without depending on JSON-RPC.

**Provides:**
- **To external tooling** — the JSON-RPC API is the officially sanctioned programmatic control surface (start/stop/save, player/ban/allowlist/op management, live game-rule and server-setting mutation, event notifications); a reimplementation targeting parity with third-party server-management dashboards should treat the 85-method surface in §3.3/§7 as a compatibility contract independent of internal architecture.
- **To operators** — `server.properties` (§3.2) is the primary configuration contract; the Swing GUI and RCON are alternate command-entry points funneling into the same `handleConsoleInput`/`CommandSourceStack` path used by in-game commands.
- **To the client** — `ClientboundSetTimePacket` (clock sync), `ClientboundGameEventPacket` (rain/thunder level changes), `ClientboundDebug*Packet`s (opt-in debug channels), and the raid bossbar (`ServerBossEvent`) are all server→client pushes originating from this domain.

## 7. Data-generator cross-reference

| File | Contents |
|---|---|
| `reports/json-rpc-api-schema.json` | The exact OpenRPC 1.3.2 document `rpc.discover` generates live: `info{title, version:"3.0.0"}`, all 85 `methods` (name, params, result, each with a JSON-schema `$ref`), and `components.schemas` — 14 named component schemas: `difficulty` (enum `peaceful/easy/normal/hard`), `game_type` (enum `survival/creative/adventure/spectator`), `player` (`{id, name}`), `operator` (`{player, permissionLevel, bypassesPlayerLimit}`), `user_ban`/`incoming_ip_ban`/`ip_ban` (`{player|ip, reason, source, expires}`), `kick_player` (`{player, message}`), `message` (`{literal | translatable + translatableParams}`), `system_message` (`{message, overlay, receivingPlayers}`), `server_state` (`{started, players[], version{name,protocol}}`), `typed_game_rule`/`untyped_game_rule` (`{key, value}`, typed variant adds an enum `type: integer|boolean`), `version` (`{name, protocol}`). This is the ground truth for a Rust client/server-management implementation's wire types. |
| `reports/registries.json` | Confirms which registries here are static/code-defined (feature flags are **not** listed — they're a plain Java enum-like structure, not a registry) |
| `reports/datapack.json` | Marks `test_environment`, `test_instance`, `timeline`, `world_clock` as `stable: false` (experimental/unstable datapack registries), and `permission_check_type`/`permission_type` as data-driven-tag-only (`elements: false`) |
| `data/minecraft/world_clock/overworld.json`, `.../the_end.json` | Concrete `WorldClock` registrations (both are empty objects — the type carries no fields, only identity) |
| `data/minecraft/timeline/day.json`, `moon.json`, `early_game.json`, `villager_schedule.json` | Concrete `Timeline` datapack entries; `day.json` is the full vanilla day/night keyframe schedule (sky/fog/cloud color, sun/moon/star angles, sky-light factor, monster-burn window, bee/firefly/creaking/eyeblossom activity windows, cat wake-gift chance, turtle-egg hatch chance) bound to `minecraft:overworld` with `period_ticks: 24000` and all six `ClockTimeMarker`s |
| `data/minecraft/test_environment/default.json`, `data/minecraft/test_instance/always_pass.json` | Minimal worked examples of the GameTest environment/instance datapack schema |
| `reports/blocks.json`, `reports/registries.json` | Not specific to this domain but every feature-flag-gated block/item ultimately traces back to one of the 4 flags in `FeatureFlags` |

## 8. Notes for Rusty Clanker

- **The JSON-RPC API is a genuine wire-compatibility target, separate from the game protocol.** It is versioned (`openrpc: "1.3.2"`, `info.version: "3.0.0"`), self-describing (`rpc.discover`), and intentionally decoupled from internal server structure via DTOs (`PlayerDto`, `KickDto`, etc.) — a Rust implementation should mirror the 85-method/14-schema surface exactly (method names, param/result field names, JSON-RPC 2.0 error codes) rather than re-deriving a "management API" from scratch, since third-party dashboards will be built against the vanilla schema.
- **`StoredUserList` mutation and JSON-RPC notification are already unified in vanilla** — `/ban`, `/whitelist add`, and `minecraft:bans/add` all funnel through the same `NotificationService`. Any Rust implementation that special-cases "RPC-originated" changes vs. "command-originated" changes would silently diverge from vanilla observable behavior (a dashboard watching `bans/added` must fire regardless of origin).
- **The login handshake's `serverId` is hardcoded to the empty string**, not derived from server identity — this is a legacy Mojang session-server API quirk (the old `serverId`/`server.hash` mechanism was replaced but the digest call signature wasn't), and must be reproduced exactly (empty string, not e.g. a server UUID) or `hasJoinedServer` calls will fail signature/lookup on Mojang's side.
- **`WorldClock`/`Timeline` is a significant architectural bet that changes what "day/night" *is*.** It is no longer a single global `dayTime` long; it's a per-clock monotonic tick counter plus a layered, datapack-authored keyframe interpolation system feeding an `EnvironmentAttribute` graph (owned by MECH/05, but the clock *substrate* lives here). Determinism hazard: `partialTick` fractional accumulation with a non-1.0 `rate` means tick-exact clock state depends on accumulated rounding, not just `totalTicks / rate` — a Rust port must replicate the same integer-floor-and-carry accumulation, not a naive multiply, to stay bit-identical across arbitrary rate changes. Because this system is marked `stable: false` in the datapack report, its shape may still move before final release — treat the *mechanism* (registry-driven clock + timeline layering) as the stable design decision, but don't hard-freeze the exact JSON schema.
- **All five ambient/event spawners are Overworld-only by construction**, not by an explicit dimension check inside each spawner — the gate is entirely "which spawner list was passed to this `ServerLevel`'s constructor" in `MinecraftServer.createLevels`. A Rust ECS reimplementation should make this an explicit per-dimension spawner-set configuration, not something spawners self-check, to avoid the failure mode where a custom dimension accidentally inherits the Overworld list (or a modded dimension needs a subset).
- **`VillageSiege` migrating from a raw daytime-modulo check to `ClockTimeMarkers.ROLL_VILLAGE_SIEGE`** is a concrete, load-bearing example of the clock/timeline system replacing old-style hardcoded time math throughout the codebase — expect more such migrations in 05-MECH's day/night-dependent mechanics (mob burning, crop growth windows, etc.) and plan the environment-attribute substrate accordingly rather than re-hardcoding tick thresholds per-mechanic.
- **The management server's origin allow-list only protects the WebSocket-subprotocol auth path.** `AuthenticationHandler.performSecurityChecks` checks `management-server-allowed-origins` only when the secret arrives via `Sec-WebSocket-Protocol`; a client presenting the same secret via `Authorization: Bearer` is admitted regardless of its `Origin` header. `management-server-allowed-origins` is therefore not a complete access-control statement by itself — document this precisely if reimplementing so operators don't misconfigure it as a firewall.
- **`ChaseServer`/`ChaseClient` has zero authentication** — reachable by anyone who can hit the bound TCP port once an op has run `/chase lead`. Low real-world risk (position/rotation broadcast only, opt-in, rarely used) but worth flagging explicitly as a deliberate vanilla-parity exception if Rusty Clanker chooses to add auth here — any deviation from silent bit-identical parity must be documented per the project's binding principles.
- **Redstone/scheduled-tick sequencing is untouched by anything in this document** — world border, weather, raids, and clocks are level-tick-scoped and single-threaded per region already in vanilla (they run inline in `ServerLevel.tick()`), which lines up naturally with ARCH-D's "redstone and scheduled block ticking always sequential, single-worker per region" constraint; no special-casing needed here beyond preserving the exact within-tick ordering listed in §6.
- **Crash reports and the watchdog are not gameplay-observable but are operationally load-bearing**: `max-tick-time` directly determines whether a legitimately slow tick (e.g. huge redstone contraption, chunk generation burst) gets killed as "hung." A Rust reimplementation with a different scheduler (work-stealing pool per ARCH-D) needs an equivalent single well-defined "tick deadline" signal for the watchdog to observe — the vanilla model watches wall-clock time since the *scheduled* next-tick time, not CPU time, so it will also fire on GC pauses or OS scheduling stalls, which is probably the correct behavior to preserve.
- **Feature flags are a closed, small, code-enumerated set (4 flags)**, not an open datapack registry — mods/datapacks cannot invent new flags in vanilla. If Rusty Clanker's modding API (06-MOD) wants mod-defined feature gating, it must be a distinct mechanism, not an extension of `FeatureFlagRegistry`, to stay bit-identical with vanilla's closed set when running unmodded.

