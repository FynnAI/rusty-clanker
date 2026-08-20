# Top-Level Source Cartography — Minecraft: Java Edition 26.2 Server

## 1. Purpose

This document maps the entire decompiled vanilla server jar (26.2, protocol 776) at the package level: every top-level and second-level Java package under `net.minecraft` and `com.mojang`, plus the outer bundler jar and the jar-internal resource layout (`data/`, `assets/`). It is the navigation index for all other `docs/research/mc-26.2/*` documents — each of which goes deep on one domain — and separately documents the process-startup machinery (bundler → `Main` → `Bootstrap` → registry freeze → world load) that no single domain doc owns. No feature mechanism is explained in depth here; that is the job of the sibling documents referenced in every table row.

## 2. Where it lives — master package map

Counts are `.java` file counts (Vineflower output, includes `package-info.java` where present) from `C:\Users\krank\mc-research\26.2\src`. "Root" rows are files directly in a package, not its subpackages. Total: **4,849 Java files** across `net/minecraft` (4,839) and `com/mojang` (10).

### `com.mojang` (10 files)

| Package | Files | Responsibility | Representative classes | Depth doc |
|---|---|---|---|---|
| `com.mojang.math` | 10 | Matrix/quaternion/rotation math shared by physics and (client) rendering; no other `com.mojang` code is bundled in the server jar — Brigadier, DataFixerUpper, Authlib, and Mojang logging ship as separate library jars (see §3.1) and are not part of this decompiled tree. | `Transformation`, `Axis`, `OctahedralGroup`, `GivensParameters`, `MatrixUtil` | 14-physics-collision |

### `net.minecraft` root (18 files)

Process-level primitives with no natural subsystem home; several are the direct subject of this document.

| Class | Role |
|---|---|
| `SharedConstants` | Central constant pool: protocol version, world version, tick timings, string/name length limits, all `DEBUG_*` feature toggles. See §3.2. |
| `DetectedVersion`, `WorldVersion` | Reads `/version.json` from the jar classpath into a `WorldVersion` record; `SharedConstants.CURRENT_VERSION` is set from it once at startup. See §3.4. |
| `CrashReport`, `CrashReportCategory`, `CrashReportDetail`, `ReportedException`, `SystemReport` | Structured crash-report generation (`crash-reports/crash-*.txt`), attaches OS/JVM/mod-list/thread-dump sections. |
| `ChatFormatting` | The 22 vanilla color/format codes (`§`-code enum) shared by chat, commands, and logging. |
| `ExitCodes` | Named process exit codes used by `DedicatedServer`/`Main` on fatal shutdown paths. |
| `DefaultUncaughtExceptionHandler(WithName)` | Installed on server/network/worker threads; routes uncaught exceptions into `CrashReport`. |
| `TracingExecutor` | `ExecutorService` wrapper that tags tasks for the JFR/tracy profiler (see `util.profiling`). |
| `IdentifierException`, `Optionull`, `CharPredicate`, `SuppressForbidden`, `ReportType` | Small shared utilities/annotations (forbidden-API suppression marker, nullable-friendly `Optional` helpers). |

### `net.minecraft.server` and subpackages (420 files)

| Package | Files | Responsibility | Representative classes | Depth doc |
|---|---|---|---|---|
| `server` (root) | 27 | Server process core: main tick loop owner, registry layering, datapack/world-loading pipeline, server-side function & scoreboard managers. See §3.3–3.5. | `MinecraftServer`, `WorldLoader`, `WorldStem`, `RegistryLayer`, `Main`, `Bootstrap`, `ReloadableServerResources`, `ReloadableServerRegistries`, `ServerFunctionManager`, `ServerScoreboard`, `Services`, `TickTask` | 01-bootstrap-lifecycle |
| `server.level` | 42 | Per-dimension server state: chunk lifecycle/ticket management, chunk loading task graph, server-side entity/player wrappers, threaded lighting driver. | `ServerLevel`, `ChunkMap`, `ChunkHolder`, `DistanceManager`, `ChunkTaskDispatcher`, `WorldGenRegion`, `ServerPlayer`, `ServerEntity`, `ThreadedLevelLightEngine` | 03-world-chunks (chunk mgmt); 11-player-gameplay (`ServerPlayer*`) |
| `server.network` | 28 | Netty pipeline wiring on the server side, connection listener, per-phase packet listener implementations, chat/text filtering. | `ServerConnectionListener`, `ServerGamePacketListenerImpl`, `ServerConfigurationPacketListenerImpl`, `ServerCommonPacketListenerImpl`, `PlayerChunkSender`, `LegacyQueryHandler` | 02-protocol-networking |
| `server.players` | 19 | Player roster, ban/op/whitelist persistence, name↔UUID resolution cache. | `PlayerList`, `UserBanList`, `IpBanList`, `ServerOpList`, `ProfileResolver`, `SleepStatus` | 11-player-gameplay |
| `server.commands` | 102 | One class per built-in `/command` (`Commands` registration lives in `net.minecraft.commands`). | `TeleportCommand`, `GamemodeCommand`, `FillCommand`, `ExecuteCommand`, … | 13-commands-data-driven |
| `server.packs` | 55 | Resource/data-pack discovery, layering, and `ResourceManager` (server only loads data packs; resource-pack code here serves the "generate client resources" datagen path). | `PackRepository`, `ServerPacksSource`, `VanillaPackResources`, `ResourceManager` (in `server.packs.resources`) | 01-bootstrap-lifecycle |
| `server.dedicated` | 7 | `server.properties` schema and the dedicated-server entry object. | `DedicatedServer`, `DedicatedServerProperties`, `DedicatedServerSettings` | 01-bootstrap-lifecycle |
| `server.dialog` | 35 | Data-driven server-defined UI dialogs (buttons/inputs sent to client, new since 1.21.6-era). | `Dialog`, `DialogTypes`, `action.*`, `body.*`, `input.*` | 11-player-gameplay |
| `server.jsonrpc` | 65 | The out-of-process JSON-RPC management API (start/stop/monitor the server, distinct from RCON). | `JsonRpc`, `ManagementServer`, `IncomingRpcMethods`, `OutgoingRpcMethods` | 15-services-misc |
| `server.rcon` | 9 | Legacy Source-RCON protocol implementation. | `RconServer`, `RconClient` | 15-services-misc |
| `server.gui` | 4 | The optional Swing server console GUI (`--nogui` disables it). | `MinecraftServerGui` | 15-services-misc |
| `server.notifications` | 5 | OS-level desktop notifications (e.g. player joined while server window unfocused). | `NotificationManager` | 15-services-misc |
| `server.permissions` | 12 | Data-driven permission-predicate registry (`Permission`, `PermissionCheck` types) backing the JSON-RPC/management surface. | `Permission`, `PermissionCheck`, `PermissionTypes` | 13-commands-data-driven |
| `server.bossevents` | 3 | `/bossbar` command backing store. | `CustomBossEvent`, `CustomBossEvents` | 11-player-gameplay |
| `server.advancements` | 2 | Server-side advancement command display helpers. | `AdvancementVisibilityRules` | 11-player-gameplay |
| `server.waypoints` | 2 | Server half of the locator-bar waypoint system. | `ServerWaypointManager` | 11-player-gameplay |
| `server.chase` | 3 | `/chase` debug spectator-follow command. | `ChaseCommand`, `ChaseClient` | 15-services-misc |

### `net.minecraft.network` and subpackages (411 files)

| Package | Files | Responsibility | Representative classes | Depth doc |
|---|---|---|---|---|
| `network` (root) | 42 | Netty channel handlers: framing (`Varint21FrameDecoder`), compression, encryption cipher, packet (de)serialization dispatch, connection object. | `Connection`, `FriendlyByteBuf`, `RegistryFriendlyByteBuf`, `PacketDecoder`/`PacketEncoder`, `CompressionDecoder`/`Encoder`, `CipherDecoder`/`Encoder`, `VarInt`, `VarLong` | 02-protocol-networking |
| `network.protocol` (root) | 13 | Cross-phase infrastructure: protocol-phase enum, packet-type registration. | `ConnectionProtocol`, `ProtocolInfo` | 02-protocol-networking |
| `network.protocol.handshake` | 6 | First-contact packet (declares intent: status/login/transfer). | `ClientIntentionPacket` | 02-protocol-networking |
| `network.protocol.status` | 8 | Server list ping. | `ClientboundStatusResponsePacket` | 02-protocol-networking |
| `network.protocol.login` | 19 | Auth handshake, encryption request/response, compression negotiation. | `ServerboundHelloPacket`, `ClientboundGameProfilePacket`, `ClientboundLoginCompressionPacket` | 02-protocol-networking |
| `network.protocol.configuration` | 14 | Post-login, pre-play config phase (registry sync, resource pack push, feature flags). | `ClientboundRegistryDataPacket`, `ClientboundUpdateEnabledFeaturesPacket` | 02-protocol-networking |
| `network.protocol.game` | 194 | The Play-phase packet set — by far the largest single packet family (entities, blocks, chunks, inventory, chat, player state). | `ClientboundAddEntityPacket`, `ClientboundLevelChunkWithLightPacket`, `ServerboundPlayerActionPacket`, `ClientboundSetEntityDataPacket`, … | 02-protocol-networking |
| `network.protocol.common` | 27 | Packets valid across multiple phases (keep-alive, plugin/custom-payload, resource-pack, cookies wrapper types). | `ClientboundKeepAlivePacket`, `ClientboundCustomPayloadPacket` | 02-protocol-networking |
| `network.protocol.cookie` | 6 | Client-side "cookie" storage packets (opaque server-set client state, used across reconnects/transfers). | `ClientboundStoreCookiePacket` | 02-protocol-networking |
| `network.protocol.ping` | 6 | Play-phase ping/pong latency packets (distinct from status ping). | `ClientboundPingPacket` | 02-protocol-networking |
| `network.chat` | 63 | Chat `Component` tree (text/translatable/scoreboard/selector content), signed-chat message tracking, click/hover events, chat-type decoration. | `Component`, `ComponentContents`, `ClickEvent`, `HoverEvent`, `LastSeenMessagesTracker`, `FilterMask` | 02-protocol-networking |
| `network.codec` | 7 | Generic `StreamCodec`-style (de)serialization combinators used to build packet field codecs. | (codec builder/combinator classes) | 02-protocol-networking |
| `network.syncher` | 6 | Entity data-tracker (`SynchedEntityData`) — the mechanism entities use to replicate mutable fields to clients. | `SynchedEntityData`, `EntityDataSerializer` | 09-entities-ai |

### `net.minecraft.world` and subpackages (2,585 files — the largest root)

| Package | Files | Responsibility | Representative classes | Depth doc |
|---|---|---|---|---|
| `world.level` (root) | 53 | The `Level`/world-access interface stack (read/write/simulate capability splits), explosion logic, spawning, color resolvers, chunk position math. | `Level`, `LevelAccessor`, `LevelReader`, `BlockGetter`, `CollisionGetter`, `ChunkPos`, `Explosion`, `NaturalSpawner`, `GameType` | 03-world-chunks |
| `world.level.chunk` | 54 | Chunk data structures and generation-facing interfaces (not chunk *lifecycle*, which is `server.level`). | `ChunkAccess`, `ChunkGenerator`, `ChunkSource`, `GlobalPalette`, `EmptyLevelChunk` | 03-world-chunks |
| `world.level.chunk.storage` *(nested, counted under `chunk`)* | — | Region-file (`.mca`) reading/writing, `RegionFileVersion` (compression algorithm selection). | `RegionFile`, `RegionFileStorage`, `RegionFileVersion` | 04-persistence |
| `world.level.storage` | 152 (incl. `loot`) | World save-folder layout, level-data (NBT `level.dat`) model, player-data files, command-storage NBT. | `LevelStorageSource`, `PrimaryLevelData`, `LevelData`, `DataVersion`, `PlayerDataStorage`, `CommandStorage` | 04-persistence |
| `world.level.storage.loot` | 129 | Loot table tree: pools, entries, functions, conditions, number providers. | `LootTable`, `LootPool`, `LootItemFunction`, `LootContext` | 10-items-recipes |
| `world.level.levelgen` (root) | 43 | Noise-based terrain core: density functions, aquifers, noise router, world-gen settings. | `DensityFunction`, `DensityFunctions`, `Aquifer`, `NoiseRouterData`, `WorldOptions`, `WorldGenSettings` | 05-worldgen-parity |
| `world.level.levelgen.feature` | 177 | Placed decoration ("features"): trees, ores, vegetation, structures-as-features glue. | `Feature`, `TreeFeature`, `OreFeature`, `ConfiguredFeature` | 05-worldgen-parity |
| `world.level.levelgen.structure` | 129 | Structure generation: piece assembly, jigsaw system, structure placement rules. | `Structure`, `StructurePiece`, `JigsawStructure`, `StructureTemplate` (template class itself lives under `chunk`/`levelgen`, see 06) | 06-structures |
| `world.level.levelgen.placement` | 23 | Where/how many times a feature is attempted per chunk (decorator chain). | `PlacementModifier`, `RarityFilter`, `HeightRangePlacement` | 05-worldgen-parity |
| `world.level.levelgen.carver` | 11 | Cave/canyon carving (subtractive terrain shaping, pre-surface). | `CaveCarver`, `CanyonWorldCarver` | 05-worldgen-parity |
| `world.level.levelgen.synth` | 8 | Low-level noise primitives (Perlin/Simplex octaves, blended noise). | `PerlinNoise`, `SimplexNoise`, `BlendedNoise` | 05-worldgen-parity |
| `world.level.levelgen.heightproviders`, `blockpredicates`, `blending`, `flat`, `material`, `presets` | 9, 19, 3, 5, 2, 3 | Supporting DSLs for the above: height distributions, block-match predicates for carving/placement, old/new-terrain blending at chunk borders, superflat presets, world-preset registry defaults. | `TrapezoidHeight`, `MatchingBlockPredicate`, `Blender`, `FlatLevelGeneratorSettings`, `WorldPresets` | 05-worldgen-parity |
| `world.level.biome` | 19 | Biome definition, climate-parameter lookup (multi-noise), biome→spawn/generation settings. | `Biome`, `Climate`, `MultiNoiseBiomeSourceParameterList`, `BiomeSource`, `MobSpawnSettings` | 05-worldgen-parity |
| `world.level.block` (root) | 326 | One class per distinct block *behavior* (not one per block — variants share a class via `BlockState` properties). | `AbstractFurnaceBlock`, `RedStoneWireBlock`, `PistonBaseBlock`, `DoorBlock`, … | 07-blocks |
| `world.level.block.state` | 45 | `BlockState`/property system, shape caching, state predicates. | `BlockState`, `BlockBehaviour`, `StateDefinition`, `VoxelShape` refs | 07-blocks |
| `world.level.block.entity` | 86 | Block entities ("tile entities") — chests, furnaces, signs, spawners, etc. | `BlockEntity`, `ChestBlockEntity`, `BeaconBlockEntity` | 07-blocks |
| `world.level.block.piston` | 7 | Piston push/pull structure-move resolution. | `PistonStructureResolver` | 07-blocks / 14-physics-collision |
| `world.level.block.grower`, `.sounds` | 2, 2 | Sapling→tree growth strategy objects; per-block sound-type helpers. | `TreeGrower` | 07-blocks |
| `world.level.redstone` | 10 | Redstone wire propagation and neighbor-update ordering/batching. | `NeighborUpdater`, `DefaultRedstoneWireEvaluator`, `ExperimentalRedstoneWireEvaluator`, `Orientation` | 08-redstone-ticking |
| `world.level.gameevent` | 15 | The "game event" / vibration system (sculk sensors, allay listening, etc.) — position-based event broadcast with occlusion. | `GameEvent`, `GameEventDispatcher`, `PositionSource`, `EuclideanGameEventListenerRegistry` | 08-redstone-ticking / 09-entities-ai |
| `world.level.lighting` | 15 | Block-light and sky-light propagation engines (single-threaded BFS-style flood fill per section). | `LightEngine`, `BlockLightEngine`, `SkyLightEngine`, `DynamicGraphMinFixedPoint`, `DataLayerStorageMap` | 12-lighting |
| `world.level.pathfinder` | 15 | A*-style mob pathfinding: node evaluators per locomotion type, path-type costing. | `PathFinder`, `WalkNodeEvaluator`, `FlyNodeEvaluator`, `SwimNodeEvaluator`, `AmphibiousNodeEvaluator`, `BinaryHeap` | 09-entities-ai |
| `world.level.dimension` | 8 | Dimension type definitions (overworld/nether/end physics constants) and dimension↔generator binding. | `DimensionType`, `LevelStem`, `BuiltinDimensionTypes` | 03-world-chunks |
| `world.level.border` | 4 | World border shape, damage, and interpolation over time. | `WorldBorder` | 03-world-chunks |
| `world.level.material` | 12 | Fluid state model (`FluidState`) and material/`MapColor` classification. | `Fluid`, `FluidState`, `MapColor` | 07-blocks |
| `world.level.portal` | 4 | Nether/End portal search and teleport-frame logic. | `PortalShape`, `TeleportTransition` | 09-entities-ai / 03-world-chunks |
| `world.level.gamerules` | 7 | `/gamerule` typed registry (boolean/int rule types, default values). | `GameRules`, `GameRule` | 13-commands-data-driven |
| `world.level.saveddata` | 14 | Per-world persistent auxiliary data (map item data, raid state) via `SavedData` framework. | `SavedData`, `MapItemSavedData` | 04-persistence |
| `world.level.timers` | 6 | Scheduled one-shot callback system backing `/schedule`. | `TimerQueue`, `TimerCallback` | 13-commands-data-driven |
| `world.level.entity` | 19 | Level-scoped entity container/section-indexing (distinct from the entity classes themselves). | `LevelEntityGetter`, `EntityTickList`, `PersistentEntitySectionManager` | 09-entities-ai |
| `world.level.validation` | 5 | Path/filename sanitization for world-related file I/O (defends against path traversal in NBT-supplied names). | `PathAllowList` | 04-persistence |
| `world.entity` (root) | 75 | The `Entity` base class hierarchy root, movement/physics-integration entry points, entity type registry glue. | `Entity`, `EntityType`, `EntityTypes`, `LivingEntity`, `Mob` | 09-entities-ai |
| `world.entity.ai` | 277 | Brain/goal AI: goal selector, memory modules, sensors, behaviors, village/POI-aware activities. | `Brain`, `ActivityData` (root, only 2 files — most AI content is in nested subpackages `goal`, `behavior`, `memory`, `sensing`, `village`, `attributes`, `navigation`, `control`, counted within this 277) | 09-entities-ai |
| `world.entity.animal`, `.monster`, `.boss`, `.npc`, `.ambient` | 130, 84, 24, 15, 3 | Concrete mob implementations by category. | `Cow`, `Zombie`, `EnderDragon`, `Villager`, `Bat` | 09-entities-ai |
| `world.entity.player` | 14 | Shared (server+notional client) player entity base, inventory-holder contract. | `Player`, `Inventory`, `Abilities` | 11-player-gameplay |
| `world.entity.projectile` | 37 | Arrows, thrown items, fireballs — flight/impact behavior. | `AbstractArrow`, `ThrownTrident`, `Fireball` | 09-entities-ai |
| `world.entity.vehicle` | 24 | Boats, minecarts, and their variants. | `Boat`, `AbstractMinecart` | 09-entities-ai |
| `world.entity.decoration` | 12 | Non-mob "furniture" entities (item frames, paintings, armor stands, text/interaction entities). | `ArmorStand`, `ItemFrame`, `Painting` | 09-entities-ai |
| `world.entity.item` | 4 | Dropped-item and primed-TNT entities. | `ItemEntity`, `PrimedTnt` | 09-entities-ai / 10-items-recipes |
| `world.entity.raid`, `.schedule`, `.variant` | 4, 2, 11 | Raid event state; villager schedule/activity time tables; per-species cosmetic variant registries (wolf/cat/cow/etc. — backs `data/minecraft/*_variant`). | `Raid`, `Schedule`, `WolfVariant` | 09-entities-ai |
| `world.item` (root) | 100 | `Item` base class, one class per item with unique behavior, item-stack utility. | `Item`, `ItemStack`, `BlockItem`, `BowItem` | 10-items-recipes |
| `world.item.crafting` | 66 | Recipe types (`RecipeType`), recipe-book categories, ingredient matching. | `CraftingRecipe`, `AbstractCookingRecipe`, `Ingredient`, `RecipeManager` | 10-items-recipes |
| `world.item.enchantment` | 47 | Enchantment definitions, effect components, enchanting-table cost tables. | `Enchantment`, `EnchantmentHelper` | 10-items-recipes |
| `world.item.component` | 46 | Data-component payload types (the post-1.20.5 item-data model: `DataComponentType<T>` value classes). | `ItemLore`, `FoodProperties` (component form), `Consumable` | 10-items-recipes |
| `world.item.equipment` | 15 | Armor/tool material and equipment-slot definitions. | `ArmorMaterial`, `Equippable` | 10-items-recipes |
| `world.item.trading` | 11 | Villager trade offer model. | `MerchantOffer`, `MerchantOfferList` | 09-entities-ai / 10-items-recipes |
| `world.item.slot`, `.context`, `.consume_effects`, `.alchemy` | 12, 4, 7, 6 | Equipment-slot group definitions; item-use context objects; on-consume effect payloads (food/potion secondary effects); potion base/mix registry. | `EquipmentSlotGroup`, `UseOnContext`, `PotionContents` | 10-items-recipes |
| `world.inventory` | 64 | Container menus (crafting UI server-side state machines), slot logic, click-type resolution. | `AbstractContainerMenu`, `Slot`, `CraftingMenu`, `ClickType` | 11-player-gameplay |
| `world.phys` | 28 | Vector/AABB/ray-cast primitives used by movement, collision, and targeting. | `Vec3`, `AABB`, `BlockHitResult`, `HitResult` | 14-physics-collision |
| `world.attribute` | 28 | Numeric mob attribute system (max health, movement speed, attack damage) and modifier stacking. | `Attribute`, `AttributeInstance`, `AttributeModifier` | 09-entities-ai |
| `world.damagesource` | 12 | Damage-type/source model and death-message selection. | `DamageSource`, `DamageType`, `CombatTracker` | 09-entities-ai |
| `world.effect` | 20 | Mob (potion) effects. | `MobEffect`, `MobEffectInstance` | 09-entities-ai |
| `world.food` | 5 | Hunger/saturation model. | `FoodData` | 11-player-gameplay |
| `world.scores` | 16 | Scoreboard objectives, teams, per-entity score storage. | `Scoreboard`, `Objective`, `PlayerTeam`, `Score` | 11-player-gameplay |
| `world.ticks` | 14 | Scheduled block/fluid tick queues (the data structure `LevelTicks` that redstone and fluid flow schedule into). | `LevelTicks`, `LevelChunkTicks`, `ScheduledTick`, `TickPriority` | 08-redstone-ticking |
| `world.clock` | 10 | Named world-clock/calendar definitions (`data/minecraft/world_clock`) driving day-length/season-like time display. | `WorldClock` | 15-services-misc |
| `world.timeline` | 5 | Data-driven camera/event timelines (`data/minecraft/timeline`), likely cutscene/intro sequencing. | `Timeline` | 11-player-gameplay |
| `world.waypoints` | 9 | Shared (server+protocol) waypoint model backing the locator bar. | `Waypoint`, `WaypointTransmitter` | 11-player-gameplay |
| `world.flag` | 7 | Feature-flag registry gating experimental content per data-pack version. | `FeatureFlags`, `FeatureFlagSet` | 01-bootstrap-lifecycle |

### `net.minecraft.core` and subpackages (110 files)

| Package | Files | Responsibility | Representative classes | Depth doc |
|---|---|---|---|---|
| `core` (root) | 40 | Registry framework (the generic `Registry<T>`/`Holder<T>` machinery, not the concrete registries), block/section position math, `NonNullList`. | `Registry`, `MappedRegistry`, `Holder`, `HolderSet`, `RegistryAccess`, `LayeredRegistryAccess`, `BlockPos`, `SectionPos`, `Direction` | 01-bootstrap-lifecycle |
| `core.registries` | 4 | The concrete list of ~100 built-in registry keys and their bootstrap wiring. | `BuiltInRegistries`, `Registries` | 01-bootstrap-lifecycle |
| `core.component` | 30 | Data-component *type* registry infrastructure (the `DataComponentType<T>` registry itself; payload classes live in `world.item.component`). | `DataComponentType`, `DataComponents`, `DataComponentMap` | 10-items-recipes |
| `core.particles` | 21 | Particle type registry and per-particle option payloads. | `ParticleType`, `ParticleTypes`, `ParticleOptions` | 15-services-misc |
| `core.dispenser` | 14 | Dispenser-block item-dispatch behavior registry. | `DispenseItemBehavior` | 07-blocks |
| `core.cauldron` | 3 | Cauldron per-item/per-block interaction registry. | `CauldronInteraction` | 07-blocks |

### Remaining top-level packages

| Package | Files | Responsibility | Representative classes | Depth doc |
|---|---|---|---|---|
| `network` family | (see above table) | — | — | 02-protocol-networking |
| `world` family | (see above table) | — | — | (split, see above) |
| `server` family | (see above table) | — | — | (split, see above) |
| `core` family | (see above table) | — | — | (split, see above) |
| `advancements` (+`predicates`, `+triggers`) | 116 | Advancement tree model, unlock-condition predicate DSL (item/entity/location/damage predicates), trigger listener types. | `Advancement`, `AdvancementTree`, `EntitySubPredicates`, `CriteriaTriggers` | 11-player-gameplay |
| `commands` (+`arguments`, `+execution`, `+functions`, `+synchronization`) | 122 | Brigadier integration layer: argument-type parsers, `.mcfunction` execution engine (macros, `execute` chains), argument-type network sync for command-tree packets. | `Commands`, `CommandSourceStack`, `ExecutionCommandSource`, `CacheableFunction`, `ArgumentTypeInfos` | 13-commands-data-driven |
| `data` (+`advancements`, `+info`, `+loot`, `+metadata`, `+recipes`, `+registries`, `+structures`, `+tags`, `+worldgen`) | 163 | **Not shipped at runtime.** The `DataGenerator` source that *produces* `datagen/generated/**` (§7) when run as a separate Gradle task against this same codebase — the authoritative in-repo description of every built-in loot table, recipe, tag, and worldgen preset, expressed as Java builder code rather than static JSON. | `DataGenerator`, `VanillaPackProvider`, worldgen provider classes | 05, 06, 10 (per data kind) |
| `gametest` (+`framework`) | 47 | In-game structural test framework (`/test` command family): places a test structure, runs assertions tick-by-tick. | `GameTestHelper`, `GameTestRunner`, `TestFunction` | 09-testing-quality |
| `gizmos` | 15 | Server-authored debug-draw primitives (points/lines/boxes/arrows/text) with a style/color API, presumably serialized to a debug client channel for visualization — analogous in *purpose* (not implementation) to a game-engine gizmo/debug-draw API. | `Gizmo`, `Gizmos`, `GizmoCollector`, `CuboidGizmo`, `ArrowGizmo` | 15-services-misc |
| `locale` | 3 | Server-side translation-key lookup (`Language`), used for server log messages and command feedback that must resolve `translatable` components without a client. | `Language` | 15-services-misc |
| `nbt` (+`visitors`) | 43 | NBT tag tree, binary reader/writer, streaming visitors (used for selective/partial reads without full deserialization). | `CompoundTag`, `ListTag`, `NbtIo`, `StreamTagVisitor` | 04-persistence |
| `recipebook` | 3 | Server-side "place recipe" helper (auto-fills a crafting grid from a recipe-book click). | `ServerPlaceRecipe`, `PlaceRecipeHelper` | 10-items-recipes |
| `references` | 5 | Flat static-constant tables of item/block identifiers used for cross-references that must not go through the full registry (bootstrap-order-sensitive lookups). | `BlockIds`, `ItemIds`, `BlockItemIds` | 07-blocks / 10-items-recipes |
| `resources` | 15 | Identifier (`namespace:path`) type, `ResourceKey<T>`, and the registry *data loader* that turns datapack JSON into registry entries via `Codec`. | `Identifier`, `ResourceKey`, `RegistryDataLoader`, `RegistryOps` | 01-bootstrap-lifecycle |
| `sounds` | 6 | Sound-event registry (server only needs event identifiers, not audio data — actual `.ogg` assets are client-only and never shipped server-side). | `SoundEvent`, `SoundEvents` | 15-services-misc |
| `stats` | 10 | Player statistics registry and per-player stat storage/increment API. | `Stats`, `StatType`, `ServerStatsCounter` | 11-player-gameplay |
| `tags` | 29 | Every built-in `TagKey<T>` constant table (one class per taggable registry) plus the tag-file loader and network sync codec. | `BlockTags`, `ItemTags`, `EntityTypeTags`, `TagLoader`, `TagNetworkSerialization` | 13-commands-data-driven |
| `util` (root) | 91 | Grab-bag of foundational helpers with no other home: math (`Mth`), `Util` (thread pools, time, CSV/JSON helpers), `GsonHelper`. | `Util`, `Mth`, `GsonHelper` | 15-services-misc |
| `util.datafix` | 396 | DataFixerUpper *schema definitions* — one schema-diff class roughly per historical data version, chained to upgrade old saves/NBT to the current `WORLD_VERSION`. By far the largest subpackage outside `world`. | `DataFixers`, `DataFixTypes` | 04-persistence |
| `util.filefix` | 57 | File-level (as opposed to in-tag) migrations: renaming/moving save-folder files across versions. | (per-version file-fix classes) | 04-persistence |
| `util.worldupdate` | 6 | The `--forceUpgrade` world-upgrader driver (walks all region files through the data fixer chain). | `WorldUpgrader`, `UpgradeProgress` | 04-persistence |
| `util.thread` | 9 | Task-queue/executor abstractions used to build the server's worker pools. | `BlockableEventLoop`, `ReentrantBlockableEventLoop` | 01-bootstrap-lifecycle |
| `util.profiling` | 70 | Tick-phase profiler (`ProfilerFiller`), JFR event types, debug-sample recording consumed by `/debug` and F3. | `ProfilerFiller`, `Profiler`, JFR event classes | 15-services-misc |
| `util.parsing` | 29 | Generic reader-combinator parsing helpers shared by command/tag/text parsers. | `SuggestionsSupplier`, reader/combinator types | 13-commands-data-driven |
| `util.valueproviders` | 18 | Data-driven numeric distributions (`IntProvider`, `FloatProvider` — uniform, clamped-normal, biased-to-bottom, etc.) used throughout loot, worldgen, and item components. | `IntProvider`, `UniformInt`, `ClampedNormalFloat` | 05-worldgen-parity / 10-items-recipes |
| `util.random` | 4 | Weighted-list random-selection helpers built on top of `RandomSource`. | `WeightedList` (name approximate) | 05-worldgen-parity |
| `util.debug`, `.debugchart`, `.eventlog`, `.monitoring` | 19, 8, 4, 2 | `/debug` subscription channel definitions; F3 performance-chart sample buffers; structured event logging (chat, admin actions) to disk; light system/JVM monitoring hooks. | `DebugSubscriptions`, `SampleLogger`, `EventLog` | 15-services-misc |

## 3. How it works

### 3.1 The outer bundler jar

The distributed server artifact (`server-bundler.jar` here, e.g. `minecraft_server.26.2.jar` upstream) is **not** the game jar itself — it is a thin launcher (`net.minecraft.bundler.Main`, ~11 KB compiled, present only in the outer jar and not decompiled into this source tree) that:

1. Reads `META-INF/main-class` (contains the literal string `net.minecraft.server.Main`) and `META-INF/classpath-joined` (a `;`-joined ordered list of library-jar relative paths, ~148 entries) from its own jar.
2. Reads `META-INF/versions.list` (one line: `<sha256>\t26.2\t26.2/server-26.2.jar`) to locate the real game jar at `META-INF/versions/26.2/server-26.2.jar` (24.9 MB, embedded inside the bundler jar).
3. Extracts both the versioned game jar and every entry under `META-INF/libraries/**` (third-party dependency jars, verified by size/hash against `META-INF/libraries.list`) into a per-version cache directory outside the working directory, so repeated launches skip re-extraction.
4. Builds a `URLClassLoader` from the extracted jars and reflectively invokes `net.minecraft.server.Main.main(String[])` with the original process arguments.

This is why the decompiled `src/` tree contains only `net.minecraft.*` and `com.mojang.math` — every other `com.mojang.*` package (Brigadier, DataFixerUpper, Authlib, Mojang `logging`, `jtracy`) plus all third-party libraries (Netty 4.2.15, Guava 33.6, Gson 2.14, SLF4J, JOML, Azure/MSAL4J for account services, OSHI for system info, LZ4) live in separately-versioned jars under `META-INF/libraries/` and were never part of this decompilation target. `server-26.2.jar` itself (the "real" jar) is what was decompiled into `src/`; its own `META-INF/` carries `MOJANGCS.RSA`/`MOJANGCS.SF` (Mojang's jar signature) alongside the standard manifest.

### 3.2 `SharedConstants` — the version/protocol/tick constant pool

`SharedConstants` (`net.minecraft.SharedConstants`) is a static-only class, loaded once, holding every cross-cutting numeric/boolean constant vanilla needs before a `Level` or registry exists. Key groups:

- **Version identity** (all `@Deprecated` in favor of reading `WorldVersion`, but still the compile-time source of truth): `WORLD_VERSION = 4903`, `SERIES = "main"`, `RELEASE_NETWORK_PROTOCOL_VERSION = 776`, `SNAPSHOT_NETWORK_PROTOCOL_VERSION = 322`, `RESOURCE_PACK_FORMAT_MAJOR/MINOR = 88/0`, `DATA_PACK_FORMAT_MAJOR/MINOR = 107/1`. `getProtocolVersion()` hard-returns `776` regardless of the deprecated field (the live call path).
- **Tick timing**: `TICKS_PER_SECOND = 20`, `MILLIS_PER_TICK = 50`, `TICKS_PER_MINUTE = 1200`, `TICKS_PER_GAME_DAY = 24000`, `MAXIMUM_TICK_TIME_NANOS = 300 ms` (the watchdog threshold before a tick is logged as overlong).
- **Random tick math**: `DEFAULT_RANDOM_TICK_SPEED = 3`, plus pre-computed derived constants (`AVERAGE_GAME_TICKS_PER_RANDOM_TICK_PER_BLOCK`, `AVERAGE_RANDOM_TICKS_PER_BLOCK_PER_MINUTE/GAME_DAY`) used by debug/statistics display, not by the tick loop itself.
- **Hard limits**: `MAX_CHAT_LENGTH = 256`, `MAX_USER_INPUT_COMMAND_LENGTH = 32500`, `MAX_FUNCTION_COMMAND_LENGTH = 2000000`, `MAX_PLAYER_NAME_LENGTH = 16`, `MAX_CHAINED_NEIGHBOR_UPDATES = 1000000`, `MAX_RENDER_DISTANCE = 32`, `MAX_CLOUD_DISTANCE = 128`.
- **Misc**: `DEFAULT_MINECRAFT_PORT = 25565`, `WORLD_RESOLUTION = 16` (blocks per chunk axis), `MAXIMUM_BLOCK_EXPLOSION_RESISTANCE = 3_600_000.0F`, `ILLEGAL_FILE_CHARACTERS` (the 15-character save-folder-name blacklist).
- **~90 `DEBUG_*` boolean flags**, each backed by a `MC_DEBUG_<NAME>` system property, read once and gated behind a single master `DEBUG_ENABLED` (`MC_DEBUG_ENABLED`) switch — this is the mechanism behind every `/debug`-adjacent dev toggle (pathfinding visualization, structure debug rendering, aquifer/carver/feature disable switches for isolating worldgen layers, etc.).
- A static initializer block sets Netty's `ResourceLeakDetector` level to `DISABLED` and wires Brigadier's `CommandSyntaxException` stack-trace/message-builder hooks to Minecraft's own `BrigadierExceptions` — i.e. `SharedConstants` classload time is also where a third-party library gets vanilla-specific behavior injected, a subtle static-init-order dependency.

### 3.3 `DetectedVersion` / `WorldVersion` machinery

`WorldVersion` is a small interface (`dataVersion()`, `id()`, `name()`, `protocolVersion()`, `packVersion(PackType)`, `buildTime()`, `stable()`) with one implementation, the record `WorldVersion.Simple`. Two ways to obtain one:

- `DetectedVersion.createBuiltIn(id, name[, stable])` — synthesizes a `Simple` from the compile-time `SharedConstants` values plus `DataVersion(4903, "main")`; used when no `version.json` is present (e.g. IDE/dev runs). `DetectedVersion.BUILT_IN` is a static instance created eagerly with a random UUID as `id`.
- `DetectedVersion.tryDetectVersion()` — loads `/version.json` as a classpath resource (present at the jar root, see §3.5) and parses it into a `Simple` via `createFromJson`. Falls back to `BUILT_IN` with a warning if the resource is missing; throws `IllegalStateException` if present but malformed.

`Main.main()` calls `SharedConstants.tryDetectVersion()` as its **first statement**, before argument parsing. `SharedConstants.setVersion(WorldVersion)` (called internally by `tryDetectVersion`) is idempotent-once: a second call with a *different* instance throws `IllegalStateException("Cannot override the current game version!")`, guarding against a version being redetected mid-process. After this point `SharedConstants.getCurrentVersion()` is the canonical version object threaded through save-data (`DataVersion` tag), the `WorldStem`, and protocol handshake responses; calling it before `tryDetectVersion()`/`setVersion()` throws `IllegalStateException("Game version not set")`.

`DataVersion` (in `world.level.storage`) is the pairing of the integer world-version number with the `series_id` string ("main" vs. potential future series), and is the exact value written to the `DataVersion` NBT tag (`SharedConstants.DATA_VERSION_TAG`) in every saved level/player/region file — the anchor the data-fixer chain (§`util.datafix`) upgrades *from*.

### 3.4 Startup order: `Main` → `Bootstrap` → registry freeze → world load

`net.minecraft.server.Main.main(String[])` is the real entry point (invoked by the bundler, see §3.1). Its ordered sequence:

1. `SharedConstants.tryDetectVersion()`.
2. Parse CLI options (JOptSimple) — notable flags: `--nogui`, `--initSettings`, `--demo`, `--bonusChest`, `--forceUpgrade`, `--eraseCache`, `--recreateRegionFiles`, `--safeMode` (loads world with vanilla datapack only, ignoring any installed datapacks), `--universe <dir>`, `--world <name>`, `--port`, `--serverId`, `--jfrProfile`, `--pidFile <path>`.
3. `CrashReport.preload()` — warms crash-report class initialization before anything can fail.
4. `Bootstrap.bootStrap()` (blocking, synchronous, guarded by a `volatile boolean isBootstrapped` so it only runs once):
   a. Assert `BuiltInRegistries.REGISTRY` (the registry-of-registries) is non-empty — this registry is populated purely as a side effect of `BuiltInRegistries`' static field initializers running when the class is first touched, *before* `Bootstrap.bootStrap()` is even called; each `registerSimple`/`registerDefaulted`/… call both creates a `WritableRegistry` and stores a `RegistryBootstrap<T>` loader `Supplier` in an internal `LOADERS` map, without yet invoking it.
   b. Call four narrow legacy `bootStrap()` hooks that must run in this exact order relative to registry population: `FireBlock.bootStrap()`, `ComposterBlock.bootStrap()` (both populate static per-block-id lookup tables that are not themselves registries), then assert `EntityType.getKey(EntityTypes.PLAYER) != null` as a sanity check that entity types loaded, then `EntitySelectorOptions.bootStrap()`, `DispenseItemBehavior.bootStrap()`, `CauldronInteractions.bootStrap()`.
   c. `BuiltInRegistries.bootStrap()`: runs every deferred `LOADERS` supplier (this is where the ~100 built-in registries actually get their entries — blocks, items, entity types, sound events, particle types, etc. — each populated by a `<Registry>Bootstrap` class's static registration calls), then **freezes** every registry (`Registry.freeze()` — after this, registration calls throw), binding any tag reference not yet resolved to an empty tag, then validates no registry ended up empty (logged/paused-in-IDE, not fatal in production).
   d. `CreativeModeTabs.validate()` — checks creative-tab item lists are internally consistent.
   e. `wrapStreams()` — replaces `System.out`/`System.err` with logging-forwarding `PrintStream`s (`LoggedPrintStream`, or `DebugLoggedPrintStream` if debug logging is enabled) so raw `System.out.println` calls anywhere in the codebase still reach the structured logger.
   f. Records `bootstrapDuration` (nanosecond-precision, exposed for startup-time diagnostics).
5. `Bootstrap.validate()` — only in IDE runs (`SharedConstants.IS_RUNNING_IN_IDE`): checks every registry entry has a translation key (`Bootstrap.getMissingTranslations`) and validates command registration (`Commands.validate()`) and default attribute maps (`DefaultAttributes.validate()`).
6. `Util.startTimerHackThread()` — a workaround thread for a JVM timer-resolution quirk on some platforms.
7. Load/create `server.properties` (`DedicatedServerSettings`, force-saved so a fresh file always has every key), configure region-file compression (`RegionFileVersion.configure(...)`), check `eula.txt` (hard-stops with a log message, not an exception, if not agreed).
8. Build `Services` (auth service + session cache, `YggdrasilAuthenticationService`), `NotificationManager`, and the JSON-RPC `ManagementServer`.
9. Open the level storage (`LevelStorageSource.createDefault(universePath).validateAndCreateAccess(levelName)`), read and data-fix any existing `level.dat` (`access.getUnfixedDataTagWithFallback()` → `DataFixers.getFileFixer().fix(...)`), and reject incompatible/older-than-1.6.4 saves outright.
10. Build the datapack `PackRepository`, then **asynchronously** load `WorldStem` via `WorldLoader.load(...)` (registry layering happens here — see §3.5), blocked on synchronously via `Util.blockUntilDone`.
11. If `--forceUpgrade`/`--recreateRegionFiles`: run `WorldUpgrader` synchronously over the whole save before proceeding.
12. `MinecraftServer.spin(...)` constructs the `DedicatedServer` on a **new dedicated server thread** (not the launcher thread) and starts its own tick loop; the launcher thread then only installs a JVM shutdown hook (`dedicatedServer.halt(true)`) and returns — `Main.main` does not block on the server thread.

### 3.5 Registry layering (`RegistryLayer`) and `WorldStem`

Vanilla resolves the tension between "built-in registries are static, loaded once per JVM" and "datapack-defined registries (worldgen, dimensions, loot, recipes, tags…) are per-world and reloadable" with a four-layer stack, `LayeredRegistryAccess<RegistryLayer>`:

| Layer (enum order) | Populated from | Reload cadence |
|---|---|---|
| `STATIC` | `BuiltInRegistries.REGISTRY` (§3.4 step 4c) — wrapped once into a frozen `RegistryAccess.Frozen` at class-init time via `RegistryLayer.STATIC_ACCESS` | Once per JVM process |
| `WORLDGEN` | Datapack-defined worldgen registries (configured features, structures, density functions, noise settings, biome parameter lists, …) | Once per world (baked into save data) |
| `DIMENSIONS` | Dimension definitions (`LevelStem` per dimension) — depends on `WORLDGEN` having resolved | Once per world |
| `RELOADABLE` | Everything datapacks can hot-swap without a world restart: loot tables, recipes, advancements, tags, gamerule-adjacent data, dialogs | On `/reload` and datapack changes |

`RegistryLayer.createRegistryAccess()` builds the stack and immediately replaces the `STATIC` slot; the other three start empty and are filled in by `WorldLoader.load(...)`, whose result is packaged into the `WorldStem` record: `resourceManager` (closeable, owns pack file handles), `dataPackResources` (`ReloadableServerResources` — the compiled `RELOADABLE`-layer objects like `RecipeManager`, `LootDataManager`, `ServerAdvancementManager`), `registries` (the full `LayeredRegistryAccess<RegistryLayer>`), and `worldDataAndGenSettings` (the level's `LevelData` + `WorldGenSettings`, either loaded from an existing `level.dat` or freshly constructed for a new world — `Main.createNewWorldData` branches on demo-mode vs. normal `server.properties`-driven settings). `WorldStem` is `AutoCloseable`; closing it releases the pack `ResourceManager`. This four-layer split is the mechanism a `/reload` command exploits: it only needs to rebuild the `RELOADABLE` layer (plus whatever of `WORLDGEN`/`DIMENSIONS` a datapack change touches — full world restart is required for those, `RELOADABLE`-only changes are hot-swappable).

### 3.6 Jar-internal resource layout

The decompiled `src/` root mirrors the actual `server-26.2.jar` contents byte-for-byte in structure (it was extracted from that jar):

```
server-26.2.jar
├── net/, com/                → compiled classes (this decompilation target)
├── version.json              → read by DetectedVersion.tryDetectVersion() as classpath resource "/version.json"
├── data/minecraft/           → the vanilla datapack, embedded so a server always has a baseline datapack
│   ├── advancement/  (1688 files)   recipe/ (1585)   loot_table/ (1355)   structure/ (1212, NBT)
│   ├── worldgen/ (963)              tags/ (794)      villager_trade/ (388)   datapacks/ (106 — bundled optional datapacks, e.g. bundle/vanilla feature-flag packs)
│   └── … damage_type/, enchantment/, dimension_type/, trim_pattern/, banner_pattern/, jukebox_song/, painting_variant/, trial_spawner/, world_clock/, timeline/, dialog/, etc.
├── assets/minecraft/lang/    → en_us.json + deprecated.json ONLY — no textures/models/sounds/blockstates ship server-side (client-only assets are never bundled here; the server only needs translation strings for server-generated feedback/log text, confirming the "server never carries renderable Mojang assets" legal boundary)
└── META-INF/                → MANIFEST.MF, MOJANGCS.RSA + MOJANGCS.SF (Mojang's own jar-signing files)
```

`data/minecraft/**` here is the *runtime-embedded copy* of the same content produced by the datagen tool (`net.minecraft.data`, §2) and reported in `datagen/generated/data/minecraft/**` — i.e. two views of one authored dataset: one is what ships inside the jar (loaded by `VanillaPackResources` at the lowest priority under any installed datapacks), the other is what the `runData` Gradle task dumps to disk for tooling/datagen consumption. `assets/minecraft/lang/en_us.json` is the only asset directory present, because the dedicated server jar strips everything render-related at build time — this is a hard confirmation of ASSET-domain's "bring your own assets" boundary: even Mojang's own server binary does not carry textures, models, sounds, or blockstate JSON.

## 4. Key types

| Class (package) | Role | Notable details |
|---|---|---|
| `SharedConstants` (`net.minecraft`) | Constant pool | `getProtocolVersion()` hardcodes `776`; ~90 `DEBUG_*` flags gated by one master switch; static init wires Brigadier exception hooks |
| `WorldVersion` / `WorldVersion.Simple` (`net.minecraft`) | Version identity value object | Interface + one record impl; `packVersion(PackType)` switches resource vs. data pack format |
| `DetectedVersion` (`net.minecraft`) | Version detection | `tryDetectVersion()` reads `/version.json`; `BUILT_IN` fallback uses a random UUID as id |
| `DataVersion` (`net.minecraft.world.level.storage`) | `(int worldVersion, String seriesId)` pair | Written to every save file's `DataVersion` NBT tag |
| `Bootstrap` (`net.minecraft.server`) | Idempotent one-time init | `volatile boolean isBootstrapped` guard; `checkBootstrapCalled()` used defensively elsewhere in the codebase to assert ordering |
| `BuiltInRegistries` (`net.minecraft.core.registries`) | The ~100 built-in registries | `internalRegister` defers population via a `LOADERS` supplier map; `freeze()` iterates every registry and calls `Registry.freeze()` |
| `RegistryLayer` (`net.minecraft.server`) | 4-layer registry stack enum | `STATIC` layer's `RegistryAccess.Frozen` built once at enum class-init |
| `LayeredRegistryAccess<T>` (`net.minecraft.core`) | Generic layered-registry container | Type parameter is the layer enum (`RegistryLayer` for the server) |
| `WorldStem` (`net.minecraft.server`) | Loaded-world bundle record | `AutoCloseable`; bundles resource manager + reloadable resources + registries + level data |
| `WorldLoader` (`net.minecraft.server`) | Async datapack/registry loader | Produces a `WorldStem`; has `InitConfig`/`PackConfig`/`DataLoadContext`/`DataLoadOutput` nested types |
| `Main` (`net.minecraft.server`) | Dedicated-server entry point | JOptSimple CLI parsing; spins `DedicatedServer` on a new thread via `MinecraftServer.spin` |
| `MinecraftServer` (`net.minecraft.server`) | Server tick-loop owner (base class; `DedicatedServer` extends it) | Depth: 01-bootstrap-lifecycle |
| `Identifier` (`net.minecraft.resources`) | `namespace:path` resource name type | Used as the key type for essentially every registry and asset/data reference in the engine |
| `ResourceKey<T>` (`net.minecraft.resources`) | Typed `(registryKey, Identifier)` pair | Distinguishes "a block state's identity" from "a biome's identity" even if both used the same string |

## 5. Constants & magic values

| Constant | Value | Source class |
|---|---|---|
| `RELEASE_NETWORK_PROTOCOL_VERSION` / `getProtocolVersion()` | 776 | `SharedConstants` |
| `SNAPSHOT_NETWORK_PROTOCOL_VERSION` | 322 | `SharedConstants` |
| `WORLD_VERSION` / `DataVersion` | 4903 | `SharedConstants`, `version.json` |
| `SERIES` / `series_id` | `"main"` | `SharedConstants`, `version.json` |
| Resource pack format | major 88, minor 0 | `SharedConstants`, `version.json` |
| Data pack format | major 107, minor 1 | `SharedConstants`, `version.json` |
| `TICKS_PER_SECOND` | 20 | `SharedConstants` |
| `MILLIS_PER_TICK` | 50 | `SharedConstants` |
| `TICKS_PER_MINUTE` | 1200 | `SharedConstants` |
| `TICKS_PER_GAME_DAY` | 24000 | `SharedConstants` |
| `MAXIMUM_TICK_TIME_NANOS` | 300,000,000 (300 ms) | `SharedConstants` |
| `DEFAULT_RANDOM_TICK_SPEED` | 3 | `SharedConstants` |
| `DEFAULT_MINECRAFT_PORT` | 25565 | `SharedConstants` |
| `WORLD_RESOLUTION` | 16 (blocks/chunk axis) | `SharedConstants` |
| `MAX_CHAT_LENGTH` | 256 | `SharedConstants` |
| `MAX_USER_INPUT_COMMAND_LENGTH` | 32,500 | `SharedConstants` |
| `MAX_FUNCTION_COMMAND_LENGTH` | 2,000,000 | `SharedConstants` |
| `MAX_PLAYER_NAME_LENGTH` | 16 | `SharedConstants` |
| `MAX_CHAINED_NEIGHBOR_UPDATES` | 1,000,000 | `SharedConstants` |
| `MAX_RENDER_DISTANCE` | 32 | `SharedConstants` |
| `MAX_CLOUD_DISTANCE` | 128 | `SharedConstants` |
| `MAXIMUM_BLOCK_EXPLOSION_RESISTANCE` | 3,600,000.0 | `SharedConstants` |
| `WORLD_ICON_SIZE` | 64 | `SharedConstants` |
| `ILLEGAL_FILE_CHARACTERS` | 15 chars: `/ \n \r \t \0 \f \` ? * \ < > \| " :` | `SharedConstants` |
| `SNBT_NAG_VERSION` | 4882 | `SharedConstants` |
| `java_version` (required JVM) | 25 (`java-runtime-epsilon`) | `version.json` |
| `RPC_MANAGEMENT_SERVER_API_VERSION` | `"3.0.0"` | `SharedConstants` |
| `Bundler-Format` | 1.0 | `server-bundler.jar!META-INF/MANIFEST.MF` |
| Third-party library count bundled by launcher | 148 jars | `server-bundler.jar!META-INF/libraries.list` |
| Inner versioned game jar size | 24,952,681 bytes | `server-bundler.jar!META-INF/versions/26.2/server-26.2.jar` |

## 6. Cross-subsystem interfaces

- **Every domain** consumes `Identifier`/`ResourceKey` (`net.minecraft.resources`) and the `Registry`/`Holder` framework (`net.minecraft.core` + `core.registries`) as their common naming and lookup substrate — this is the one package pair nothing in the engine can avoid depending on.
- **Every domain** is gated, directly or transitively, by `Bootstrap.bootStrap()` having completed; several classes call `Bootstrap.checkBootstrapCalled(...)` defensively to fail loudly if touched too early (a hazard worth replicating deliberately rather than implicitly in Rust — see §8).
- **`server` (root)** hands the `LayeredRegistryAccess<RegistryLayer>` and `ReloadableServerResources` from `WorldStem` down into world/dimension construction (03), command execution context (13), and loot/recipe managers (10) — i.e. this package is the sole producer of "the fully assembled, world-specific rule set" that every gameplay subsystem reads from.
- **`network`** is purely a transport/serialization layer: it depends on `world.level`/`world.entity`/`world.item` state to know *what* to serialize (packet payload types mirror gameplay types almost 1:1) but no gameplay package depends back on `network` except to enqueue outbound packets — the dependency arrow is gameplay → network, never the reverse.
- **`util.datafix`** is consumed exclusively by `world.level.storage`/`nbt` at load time (04) and has no runtime relevance once a save is confirmed to be at the current `WORLD_VERSION` — a pure migration-time subsystem.
- **`core.registries`** (`BuiltInRegistries`) is populated by bootstrap calls scattered across nearly every other package (`FireBlock.bootStrap()` in 07-blocks, `EntitySelectorOptions.bootStrap()` in 13-commands, `DispenseItemBehavior.bootStrap()`/`CauldronInteractions.bootStrap()` in 07-blocks) — registry population is not self-contained within `core`, it is a distributed side effect triggered in a specific order by `Bootstrap`.

## 7. Data-generator cross-reference

| File/dir under `datagen/generated` | Produced by (`net.minecraft.data.*`) | Contains |
|---|---|---|
| `reports/registries.json` | `data.registries` | Every registry, every entry's `Identifier`, and its `protocol_id` — the network-wire integer ID for that entry within that registry's current data-pack load. Essential for 02-protocol-networking's VarInt-ID packet fields. |
| `reports/blocks.json` | `data.info` (block-state dumper) | Every block, its full `BlockState` property cross-product, and each state's numeric state ID. Feeds 07-blocks. |
| `reports/packets.json` | `data.info` (packet dumper) | Every packet's field layout per protocol phase. Primary source for 02-protocol-networking. |
| `reports/commands.json` | `data.info` (command-tree dumper) | The full Brigadier command tree as sent to clients (argument types, literals, redirects). Feeds 13-commands-data-driven. |
| `reports/datapack.json` | `data.info` | Datapack/registry metadata summary. |
| `reports/json-rpc-api-schema.json` | `data.info` | Schema for the `server.jsonrpc` management API (§2 table). |
| `reports/biome_parameters/minecraft/**` | `data.worldgen` | Per-biome-source multi-noise parameter dumps. Feeds 05-worldgen-parity. |
| `data/minecraft/**` | `data.advancements`, `.loot`, `.recipes`, `.structures`, `.tags`, `.worldgen` | The full generated vanilla datapack (same content as the jar-embedded copy, §3.6) — the ground truth for 05, 06, 10, and 13's data-driven content, and the primary source (alongside decompiled behavior classes) for parity fixtures in 09-testing-quality. |

## 8. Notes for Rusty Clanker

- **Static-init-order hazards are load-bearing in vanilla and must be made explicit, not implicit, in Rust.** `BuiltInRegistries`'s ~100 registries populate as a side effect of *class loading* (JVM static field init), then `Bootstrap.bootStrap()` runs a handful of *additional* hand-ordered bootstrap calls (`FireBlock` → `ComposterBlock` → entity-type sanity check → `EntitySelectorOptions` → `DispenseItemBehavior` → `CauldronInteractions` → `BuiltInRegistries.bootStrap()` itself) before freezing. A Rust reimplementation has no equivalent to "JVM class-load side effects," so this entire two-phase, partially-ordered initialization sequence must become one explicit, single-pass, deterministically-ordered Rust startup function — get the order subtly wrong (e.g. a lookup table populated after something that reads it) and the failure mode in vanilla is a silent empty-table bug, not a crash, per `validate()`'s "log and pause in IDE, otherwise continue" behavior on empty registries.
- **The "protocol_version" number is not derived from anything else — it is an opaque authored integer (776) bumped by hand per release.** ARCH/NET decisions must treat it as a literal pin, never computed from `WORLD_VERSION` or the pack-format numbers (all four numbers — protocol, world version, resource-pack format, data-pack format — are independently authored and only coincidentally similar in magnitude).
- **The four-layer registry stack (`RegistryLayer.STATIC/WORLDGEN/DIMENSIONS/RELOADABLE`) is the actual mechanism behind "hot-reload some things, not others."** This is a strong signal for how Rusty Clanker should structure its own registry/ECS-resource lifecycle: a clean separation between "process-lifetime, compiled-in" data, "world-lifetime, baked at world creation" data, and "reload-command-lifetime" data — collapsing this into one flat registry (the naive approach) would make `/reload` semantics impossible to replicate faithfully.
- **`SharedConstants.setVersion` throwing on a second differing call, and `getCurrentVersion` throwing before any call, are both deliberate fail-fast guards against a class of bug (accidental re-detection or premature use) that a Rust `OnceLock<WorldVersion>` (or equivalent) can enforce at compile-adjacent time more strongly than Java's runtime check — worth doing as a `OnceCell` with a panicking double-set rather than silently ignoring a second `set_version` call, to preserve the same "this is a programming error, not a runtime condition" signal.
- **`--safeMode` (vanilla datapack only) and the `RELOADABLE`-layer hot-reload both imply that "the effective rule set" is a runtime-composed value, never a compile-time constant**, even though Rusty Clanker's *built-in* vanilla data is code-generated at build time from a fetched data-generator dump (per `CLAUDE.md`'s legal/build rules). The engine must still assemble the built-in data + installed datapacks into one layered structure at world-load time exactly like vanilla, not bake "the" ruleset once — this is required for both `/reload` and multi-datapack support to be observationally equivalent to vanilla, and matters for CLUSTER mode too (every node/region owner must compose registries identically).
- **The server jar embeds a full copy of the vanilla datapack (`data/minecraft/**`, thousands of files) purely as the lowest-priority pack layer.** Rusty Clanker's equivalent (compiled-in vanilla defaults, per WS/ASSET decisions) should be checked for behavioral parity against exactly this reference tree — it is the actual bytes the real server ships and layers datapacks on top of, distinct from (though textually identical to) the `datagen/generated/data/minecraft/**` report copy this project is licensed to consult.
- **The dedicated server jar carries zero renderable assets** (`assets/minecraft/lang/` only) — hard confirmation that ASSET-D-series "bring your own assets" decisions have no server-side exception to worry about; even Mojang's own server ships no textures/sounds/models, so there is no precedent-based pressure to bundle any client asset with the Rusty Clanker server binary.
- **`util.datafix` (396 files) + `util.filefix` (57 files) is the single largest package by a wide margin outside `world`,** and it exists purely to upgrade old save data across versions. Since Rusty Clanker pins a single MC version with no multi-version compatibility layer (NET-D1), this entire subsystem has **no equivalent obligation** except in one narrow sense: importing a save created by vanilla 26.2 (or earlier, via vanilla's own upgrade path first) must still land on parity with vanilla's post-fix NBT shape — worth scoping precisely in 04-world-chunks-persistence so it's clear Rusty Clanker is not expected to reimplement historical schema migrations, only to read the *current*-version shape correctly.
- **The bundler's classpath/library-extraction mechanism (`net.minecraft.bundler`) is Java-specific packaging plumbing with no Rust analog worth preserving** (Cargo/the WS-series crate graph replaces it outright) — noted here only so it is not mistaken for an architecturally significant component when read out of context in future blueprints.
- **`gizmos` (net.minecraft.gizmos) is a server-authored debug-draw primitive API new enough that its wire/consumption path was not traced in this pass** — worth a follow-up look (likely folds into 02-protocol-networking or a debug-channel note in 15-services-misc) since a debug-draw facility is independently useful for Rusty Clanker's own development tooling regardless of vanilla parity.
