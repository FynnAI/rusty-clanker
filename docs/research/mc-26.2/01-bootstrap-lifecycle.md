# Server Bootstrap & Lifecycle (vanilla 26.2)

## 1. Purpose

This subsystem owns everything between process start and process exit for the Minecraft Java Edition dedicated server: command-line parsing, static engine bootstrap (registry/behavior wiring), world/registry-data loading, construction of the `MinecraftServer`/`DedicatedServer` object graph, the fixed-cadence server tick loop with its sprint/freeze/step machinery, the cooperative main-thread task-execution model (`BlockableEventLoop`), all background thread pools, graceful shutdown, the watchdog that force-kills a hung server, and the profiling/metrics infrastructure (Tracy zones, JFR, debug sample charts) that instruments the tick loop. It is the substrate every other subsystem's per-tick work ultimately runs on top of.

## 2. Where it lives

| Package | Responsibility | Representative classes | Files |
|---|---|---|---|
| `net.minecraft.server` | Process entry, bootstrap, the `MinecraftServer` base class, tick-rate manager, world loading glue | `Main`, `Bootstrap`, `MinecraftServer`, `ServerTickRateManager`, `TickTask`, `WorldLoader`, `WorldStem`, `RegistryLayer`, `Services`, `Eula`, `ConsoleInput` | 27 |
| `net.minecraft.server.dedicated` | Concrete headless server: `server.properties`, startup sequence, watchdog | `DedicatedServer`, `DedicatedServerProperties`, `DedicatedServerSettings`, `ServerWatchdog`, `Settings` | 7 |
| `net.minecraft.server.gui` | Optional Swing GUI shown when not run with `--nogui` | `MinecraftServerGui` | 4 |
| `net.minecraft.server.rcon.thread` / `net.minecraft.server.rcon` | RCON remote-console and legacy GS4 query listeners, each its own accept-loop thread | `GenericThread`, `RconThread`, `QueryThreadGs4`, `RconClient` | 4 + 5 |
| `net.minecraft.server.jsonrpc` | Optional JSON-RPC management server (new in recent versions) with its own Netty event-loop group | `ManagementServer`, `JsonRpc` | 13 |
| `net.minecraft.server.notifications` | Realms/telemetry-style activity notifications hook (server start/stop/save events) | `NotificationManager`, `ServerActivityMonitor` | 5 |
| `net.minecraft.server.network` | TCP/Netty accept + per-connection pipeline, native-transport selection | `ServerConnectionListener`, `EventLoopGroupHolder`, `ServerLoginPacketListenerImpl` | 22 |
| `net.minecraft.server.level.progress` | Level-load progress reporting used while chunks around spawn are primed | `LevelLoadListener`, `LoggingLevelLoadListener`, `ChunkLoadStatusView` | 5 |
| `net.minecraft.util.thread` | The generic cooperative-scheduling primitives: blocking event loops, consecutive/priority executors, strict queues | `BlockableEventLoop`, `ReentrantBlockableEventLoop`, `TaskScheduler`, `AbstractConsecutiveExecutor`, `ConsecutiveExecutor`, `PriorityConsecutiveExecutor`, `StrictQueue` | 9 |
| `net.minecraft.util.profiling` | In-tick profiler abstraction, Tracy zone bridge, single-tick dump-on-stall profiler | `Profiler`, `ProfilerFiller`, `ActiveProfiler`, `InactiveProfiler`, `TracyZoneFiller`, `SingleTickProfiler`, `ProfileResults` | 15 |
| `net.minecraft.util.profiling.jfr` | Java Flight Recorder integration: server/chunk/packet/GC events, `.jfr` capture and summary parsing | `JvmProfiler`, `JfrProfiler`, `Environment`, `SummaryReporter` | 6 (+ `event`, `stats`, `parse`, `serialize` subpackages) |
| `net.minecraft.util.debugchart` | Rolling sample buffers exposed to the in-game debug charts and remote debug subscribers | `SampleLogger`, `LocalSampleLogger`, `RemoteSampleLogger`, `TpsDebugDimensions`, `RemoteDebugSampleType` | 8 |
| `net.minecraft.util.monitoring.jmx` | Optional JMX MBean exposing server statistics | `MinecraftServerStatistics` | 2 |
| `net.minecraft.world.clock` | Data-driven per-dimension "world clock" and named "timeline" time markers (new in 26.2) that replace ad-hoc `dayTime % 24000` checks | `ServerClockManager`, `ClockManager`, `WorldClock`, `WorldClocks`, `ClockState`, `ClockNetworkState`, `ClockTimeMarker`, `ClockTimeMarkers`, `PackedClockStates` | 10 |
| `net.minecraft.network` (root) | Cross-thread inbound-packet handoff to the server thread | `PacketProcessor` | (shared with NET domain) |

## 3. How it works

### 3.1 Process entry and argument parsing (`Main.main`)

`Main.main` is the JVM entry point. In order:

1. `SharedConstants.tryDetectVersion()` resolves the build's `WorldVersion` (protocol/data version), required before anything below can run.
2. A `joptsimple.OptionParser` declares the CLI surface: `--nogui`, `--initSettings`, `--demo`, `--bonusChest`, `--forceUpgrade`, `--eraseCache`, `--recreateRegionFiles`, `--safeMode`, `--help`, `--universe <dir>` (default `.`), `--world <name>`, `--port <int>` (default `-1`, meaning "use server.properties"), `--serverId <string>`, `--jfrProfile`, `--pidFile <path>`.
3. If `--pidFile` is given, the current process PID is written to that file immediately (`ProcessHandle.current().pid()`).
4. `CrashReport.preload()` warms up crash-report machinery before anything can crash.
5. If `--jfrProfile` is set, `JvmProfiler.INSTANCE.start(Environment.SERVER)` begins a JFR recording before the server object even exists.
6. `Bootstrap.bootStrap()` then `Bootstrap.validate()` run static engine initialization (see 3.2).
7. `Util.startTimerHackThread()` starts a daemon thread that just sleeps for `Integer.MAX_VALUE` milliseconds — its only purpose is to force the JVM's OS-level timer resolution to a finer grain on platforms (historically Windows) where an idle JVM otherwise coarsens `Thread.sleep`/`LockSupport.parkNanos` granularity to ~15 ms, which would ruin tick-timing precision.
8. `server.properties` is loaded via `DedicatedServerSettings`, force-saved back to disk (fills in any missing keys), and `RegionFileVersion.configure(...)` applies the configured region-file compression.
9. `eula.txt` is loaded via `Eula`; if `--initSettings` was passed, the process exits right after writing `server.properties`/`eula.txt`. If EULA is not agreed, the process logs and exits without starting anything.
10. `Services.create(...)` builds the Yggdrasil session-service / profile-repository / username↔UUID cache bundle (`Proxy.NO_PROXY` for the dedicated server).
11. A `NotificationManager` and an optional `ManagementServer` (JSON-RPC) are created.
12. The level directory is resolved (`LevelStorageSource`), existing world data is loaded and upgraded through the data fixers if present (`DataFixers.getFileFixer()`), or `null` if this is a fresh world.
13. `WorldLoader.load(...)` is driven synchronously to completion via `Util.blockUntilDone` (see 3.3) to produce a `WorldStem` (loaded registries + resources + level data).
14. If `--forceUpgrade`/`--recreateRegionFiles` was passed, a `WorldUpgrader` runs a blocking chunk-by-chunk conversion loop with progress logging before the server starts.
15. `MinecraftServer.spin(...)` (3.4) constructs the `DedicatedServer` and starts its dedicated thread.
16. A JVM shutdown hook (`Runtime.getRuntime().addShutdownHook`) is registered whose thread calls `dedicatedServer.halt(true)` — this is what makes Ctrl-C / `SIGTERM` trigger an orderly shutdown.

### 3.2 Static bootstrap (`Bootstrap.bootStrap`)

`Bootstrap.bootStrap()` is idempotent (guarded by a `volatile boolean isBootstrapped`) and runs a fixed, order-dependent sequence of static initializers that wire up behavior tables the registries alone don't capture:

1. Asserts `BuiltInRegistries.REGISTRY` (the registry-of-registries) is non-empty — registries must already be classloaded/populated by the time this runs.
2. `FireBlock.bootStrap()` — builds the fire-spread flammability tables.
3. `ComposterBlock.bootStrap()` — builds the item→compost-chance table.
4. Sanity-check that `EntityType.getKey(EntityTypes.PLAYER)` resolves (entity registry sanity check).
5. `EntitySelectorOptions.bootStrap()` — registers `@e[...]` selector option parsers.
6. `DispenseItemBehavior.bootStrap()` — registers per-item dispenser behaviors.
7. `CauldronInteractions.bootStrap()` — registers cauldron interaction tables.
8. `BuiltInRegistries.bootStrap()` — freezes/finalizes the built-in (non-datapack) registries.
9. `CreativeModeTabs.validate()` — sanity-checks creative-tab contents.
10. `wrapStreams()` — replaces `System.out`/`System.err` with `LoggedPrintStream`/`DebugLoggedPrintStream` wrappers that funnel raw `print`/`println` calls through SLF4J instead of the real console, so third-party library `System.out.println` calls end up in the log.

`bootstrapDuration` (an `AtomicLong`, nanoseconds→ms) records how long this took. `Bootstrap.validate()` (called separately, right after `bootStrap()`) additionally runs `Commands.validate()` and translation-completeness checks (`getMissingTranslations`) but only when `SharedConstants.IS_RUNNING_IN_IDE` is true, plus an always-on `DefaultAttributes.validate()`. Any code that touches registries/behaviors before `bootStrap()` has run should call `Bootstrap.checkBootstrapCalled(...)`, which throws `IllegalArgumentException` naming the caller.

### 3.3 World/registry loading pipeline (`WorldLoader`, `WorldStem`, `RegistryLayer`)

This is a `CompletableFuture` pipeline, not a synchronous call, deliberately built so most of it can run on the background executor while a caller blocks the calling thread only for I/O it must wait on:

1. `RegistryLayer.createRegistryAccess()` builds a 4-layer `LayeredRegistryAccess<RegistryLayer>` — `STATIC` (built-ins, pre-populated from `BuiltInRegistries.REGISTRY`), `WORLDGEN`, `DIMENSIONS`, `RELOADABLE` — layered so datapack content can override/extend built-ins without racing them.
2. `WorldLoader.PackConfig.createResourceManager()` runs `MinecraftServer.configurePackRepository(...)` to decide the active pack selection (respecting `--safeMode`, stored `DataPackConfig`, and per-pack requested feature flags — packs whose required `FeatureFlagSet` isn't a subset of the allowed set are silently disabled with a log line), then opens all selected packs into a `MultiPackResourceManager`.
3. Static-registry tags are loaded first (`TagLoader.loadTagsForExistingRegistries`), producing `PendingTags` used to patch worldgen-registry holders once those are decoded.
4. `RegistryDataLoader.load(...)` runs twice, chained: once for `WORLDGEN_REGISTRIES` (biomes, configured features, etc.), then again for `DIMENSION_REGISTRIES` (the `LevelStem` entries, which reference worldgen registries) — each stage's output becomes lookup context for the next.
5. The caller-supplied `WorldDataSupplier` (in `Main`, this either loads existing level data via `LevelStorageSource.getLevelDataAndDimensions` or synthesizes a new world via `createNewWorldData`) runs once dimension registries are available, producing the final `WorldGenSettings`/`LevelSettings`.
6. `ReloadableServerResources.loadResources(...)` builds everything that depends on the fully-assembled registries (recipes, advancements, loot tables, functions, tags second pass) and updates registry-tag-derived component defaults.
7. The `ResultFactory` (in `Main`, `WorldStem::new`) packages resource manager + `ReloadableServerResources` + registries + level data into an immutable `WorldStem` record.

`Util.blockUntilDone` is the bridge used to run this async pipeline from a synchronous call site: it hands the pipeline a `BlockingQueue`-backed `Executor` as its "main thread executor", then loop-polls that queue (100 ms timeout) running whatever main-thread-affine step was scheduled, until the future completes. This lets `Main` stay single-threaded conceptually while `WorldLoader` genuinely uses `Util.backgroundExecutor()` for the CPU-bound registry decode work.

### 3.4 Server construction and `MinecraftServer.spin`

`MinecraftServer.spin(Function<Thread, S> factory)` is the standard way any embedder (dedicated server, integrated/singleplayer server) creates and starts the server:

1. Creates the dedicated `Thread`, named exactly `"Server thread"` (`MinecraftServer.SERVER_THREAD_NAME`), whose `Runnable` is `serverReference.get().runServer()` — note the thread is created *before* the server object exists, closing over an `AtomicReference` that gets filled in afterward, because the server's own constructor needs the `Thread` object (for `ReentrantBlockableEventLoop`'s "am I on my own thread" checks).
2. Sets an uncaught-exception handler that logs via SLF4J.
3. If the machine has more than 4 available processors, raises the thread's priority to `8` (`Thread.NORM_PRIORITY + 3`) — a mild scheduling hint on multi-core boxes.
4. Invokes `factory.apply(thread)` — for `DedicatedServer` this runs the full constructor chain described in 3.5 — stores the result in the `AtomicReference`, then calls `thread.start()`.

The `MinecraftServer` constructor (still running on whatever thread called `spin`, *not* the new server thread) does the heavyweight but non-blocking setup: validates the Overworld dimension exists in the loaded registries, opens `SavedDataStorage`, builds `ServerConnectionListener`, `ServerTickRateManager`, player-data storage, `RandomSequences`/`WeatherData`/`GameRules` (all backed by `SavedData` entries), `ServerFunctionManager`, `StructureTemplateManager`, `PotionBrewing.bootstrap(...)`, `FuelValues.vanillaBurnTimes(...)`, a Tracy `DiscontinuousFrame` named `"Server Tick"`, the `PacketProcessor` (bound to the server thread), and `ServerClockManager` (`this.clockManager.init(this)` — see 3.10).

### 3.5 `DedicatedServer.initServer()` — the real startup sequence

This runs *on the server thread* as the first thing `runServer()` does. Order matters:

1. Spawns the **console handler thread** (`"Server console handler"`, daemon) that blocks on `System.in` line reads and forwards each line into a synchronized `List<ConsoleInput>` queue for the server thread to drain.
2. Logs the version string and warns if `Runtime.getRuntime().maxMemory() < 512 MiB`.
3. Loads `server.properties`, sets local IP / online-mode / proxy-prevention (skipped, defaulting to `127.0.0.1` and no-auth-check, for singleplayer-embedded use).
4. Sets the default `GameType` from properties.
5. Resolves the bind port (CLI `--port` overrides `server-port`).
6. `initializeKeyPair()` generates an RSA key pair (`Crypt.generateKeyPair()`) used for encrypted login and (when online-mode) profile-key/session validation.
7. `getConnection().startTcpServerListener(...)` binds the Netty listener. **Failure here aborts startup** (`initServer` returns `false`) — this is the single most common dedicated-server startup failure (port already in use).
8. If offline-mode, logs the "server is running in offline/insecure mode" warning block.
9. `convertOldUsers()` runs legacy ban-list/op-list/whitelist/player-file migration with up to 2 retries and a 5-second backoff each (`CONVERSION_RETRIES = 2`, `CONVERSION_RETRY_DELAY_MS = 5000`), then persists the name→UUID cache if anything changed.
10. Builds the `DedicatedPlayerList` and the tick-time `RemoteSampleLogger`.
11. Resolves offline-mode UUIDs for the username cache, then calls `loadLevel()` (3.6), which is the long pole of startup (chunk generation/loading around spawn).
12. Logs `"Done ({time}s)! For help, type \"help\""`.
13. Conditionally starts the **GS4 query thread** (`enable-query`) and the **RCON thread** (`enable-rcon`) — both extend `GenericThread` (3.9).
14. If `getMaxTickLength() > 0` (i.e. `max-tick-time` property, default 60000 ms), spawns the **watchdog daemon thread** (3.12).
15. If `enable-jmx-monitoring`, registers the JMX MBean.
16. Forces a full synchronous save (`saveEverything(false, true, true)`).
17. Notifies `NotificationManager.serverStarted()`.

`showGui()` (called from `Main` right after `spin(...)` returns, i.e. concurrently with the above running on the server thread) opens the optional Swing GUI (`MinecraftServerGui`) unless `--nogui`/headless.

### 3.6 Level loading and initial-chunk priming

`MinecraftServer.loadLevel()` calls, in order: `createLevels()` (constructs a `ServerLevel` per `LevelStem` in the dimension registry, Overworld first, so derived level data for the Nether/End can reference it; on a brand-new world it also computes the initial spawn position via a spiral scan of up to 121 chunks around the biome-sampled spawn point, described further in the world-gen domain), `forceDifficulty()`, then `prepareLevels()`. `prepareLevels()` reactivates any previously-deactivated chunk tickets (`TicketStorage`) and then repeatedly calls `waitUntilNextTick()` in a tight loop with `nextTickTimeNanos` set only `PREPARE_LEVELS_DEFAULT_DELAY_NANOS` (10 microseconds!) in the future — i.e. it pumps the chunk-loading executor as fast as possible while polling `ChunkLoadCounter.pendingChunks()` — until every level's tracked ticket set has finished loading. This is the "Preparing spawn area" phase.

### 3.7 The tick loop (`runServer`)

After `initServer()` succeeds, `nextTickTimeNanos` is set to "now", a status icon is loaded, and the server enters its `while (this.running)` loop — this loop *is* the server thread's entire remaining lifetime. Per iteration:

1. **Decide this tick's budget.** If not paused and the tick-rate manager both `isSprinting()` and `checkShouldSprintThisTick()` returns true, `thisTickNanos = 0` (a "sprint" tick — see 3.8 — runs back-to-back with no artificial delay) and `nextTickTimeNanos` is reset to "now". Otherwise `thisTickNanos = tickRateManager.nanosecondsPerTick()` (normally `1_000_000_000 / 20 = 50_000_000` ns) and an **overload check** runs: if the server is more than `OVERLOADED_THRESHOLD_NANOS` (`1_000_000_000` ns, i.e. one full second's worth of ticks) *plus* `20 × thisTickNanos` behind schedule, and it's been at least `OVERLOADED_WARNING_INTERVAL_NANOS` (10 s) `+ 100 × thisTickNanos` since the last warning, it logs `"Can't keep up! Is the server overloaded? Running Nms or M ticks behind"` and **fast-forwards** `nextTickTimeNanos` by whole `thisTickNanos` multiples to catch up — this is vanilla's "catch-up" behavior: it does *not* run the missed ticks, it skips wall-clock time forward and accepts the missed ticks are simply gone.
2. `nextTickTimeNanos += thisTickNanos` — schedules the deadline for *this* tick's end (not next tick's start — this is the actual "when should I stop waiting" mark used by `waitUntilNextTick`).
3. A `Profiler.Scope` is opened around `createProfiler()` (3.13) for the whole tick.
4. `processPacketsAndTick(sprinting)` runs: first `packetProcessor.processQueuedPackets()` drains every packet queued by Netty I/O threads since the last tick and handles it synchronously on the server thread, then `tickServer(haveTime)` runs the actual game tick (3.7.1).
5. After the tick body, `mayHaveDelayedTasks = true` and `delayedTasksMaxNextTickTimeNanos = max(now + thisTickNanos, nextTickTimeNanos)` are set, then `waitUntilNextTick()` blocks (3.8's `managedBlock`) — this is where **all main-thread tasks scheduled via `execute()`/`submit()` from other threads actually run**, interleaved with idle-waiting, until the deadline passes.
6. If this was a sprint tick, `tickRateManager.endTickWork()` accumulates elapsed sprint time for the eventual sprint-completion report.
7. Tick-time sampling (`logFullTickTime`) and metrics-recorder tick-end bookkeeping run in a `finally`.
8. `isReady = true` is set (this flag gates when the server starts accepting status/login) and `JvmProfiler.INSTANCE.onServerTick(smoothedTickTimeMillis)` reports to JFR.

Any uncaught `Throwable` escaping the loop body is converted to a `CrashReport` (unwrapping any `ReportedException` chain first), a system report is filled in, the report is saved to `<serverDirectory>/crash-reports/crash-<timestamp>-server.txt`, and `onServerCrash(report)` runs — then the `finally` block always runs `stopServer()` (graceful shutdown) followed by `onServerExit()`, **even on crash**: vanilla always attempts a clean save/close on the way out.

#### 3.7.1 `tickServer` — one game tick

1. If `pauseWhenEmptySeconds() > 0` (dedicated-server-only property, default 60s) and the player count is 0 and the server isn't sprinting, an idle-tick counter increments; once it reaches the threshold (`seconds × 20`), the server logs "pausing", force-autosaves once, then **only** ticks the network connection (`tickConnection()`) and returns early — the whole world stops simulating while nobody is connected.
2. `tickCount++`.
3. `tickRateManager.tick()` (updates `runGameElements`/decrements frozen-step counter — 3.8).
4. `tickChildren(haveTime)` — the actual world simulation (3.7.2).
5. Every `STATUS_EXPIRE_TIME_NANOS` (5 s), the cached `ServerStatus` (used for the multiplayer server-list ping) is rebuilt.
6. `ticksUntilAutosave` decrements; at/below zero, `autoSave()` runs (`saveEverything(true, false, false)` — silent, non-flushing, non-forced) and recomputes the next interval via `computeNextAutosaveInterval()`.
7. Tick-time tallying updates a 100-slot ring buffer (`tickTimesNanos[tickCount % 100]`, `aggregatedTickTimesNanos` running sum) and an EMA (`smoothedTickTimeMillis = smoothed*0.8 + instantaneous*0.2`, `AVERAGE_TICK_TIME_SMOOTHING = 0.8F`) used for `/tps`-style reporting.

`AUTOSAVE_INTERVAL = 6000` ticks (5 minutes at 20 TPS) is the default `ticksUntilAutosave` seed. `computeNextAutosaveInterval()` targets a **wall-clock** 300 seconds regardless of current tick rate — while sprinting it estimates ticks/second from the recent average tick time; otherwise it uses the configured tick rate — and clamps to a floor of `MIMINUM_AUTOSAVE_TICKS = 100` ticks so a very slow tick rate can't push autosave out indefinitely.

#### 3.7.2 `tickChildren` — subsystem tick order

Executed in this exact order every tick (each step profiled under its own name):

1. `player.connection.suspendFlushing()` for every player — batches outgoing packets for the whole tick instead of flushing per-write.
2. `commandFunctions` — `ServerFunctionManager.tick()`.
3. `clocks` — `ServerClockManager.tick()`, **only if `tickRateManager.runsNormally()`** (i.e. not frozen unless single-stepping — see 3.8/3.10).
4. Every 20 ticks (`tickCount % 20 == 0`): `timeSync` force-broadcasts the current game time to all clients (`forceGameTimeSynchronization`), independent of any change — a periodic clock-drift correction.
5. `levels` — `updateEffectiveRespawnData()` then, **for each loaded `ServerLevel` in insertion order (Overworld, then dimension-registry iteration order)**, `level.tick(haveTime)` runs (entity/block/chunk simulation — owned by the world-chunks and game-mechanics domains). A `Throwable` here is wrapped into a `ReportedException` with level details attached and re-thrown, which is what ultimately produces the crash-report path in 3.7.
6. `connection` — `ServerConnectionListener.tick()` (accepts pending connections, expires stale ones); `DedicatedServer` overrides this to additionally drain queued console commands afterward.
7. `players` — `PlayerList.tick()`.
8. `debugSubscribers` — `ServerDebugSubscribers.tick()`.
9. `gameTests`, only if `runsNormally()` — `GameTestTicker.SINGLETON.tick()`.
10. `server gui refresh` — every `Runnable` registered via `addTickable()` runs (the Swing GUI's player-list refresh hooks into this).
11. `send chunks` — for every player, `connection.chunkSender.sendNextChunks(player)` then `connection.resumeFlushing()` (undoes step 1 — this is also where the whole tick's batched packets actually go out on the wire).
12. `serverActivityMonitor.tick()`.

`DedicatedServer.tickServer` additionally ticks the optional JSON-RPC management server after calling `super.tickServer(...)`.

### 3.8 `TickRateManager` / `ServerTickRateManager` — sprint, freeze, step

`TickRateManager` (base, in `net.minecraft.world`) holds the raw state: `tickrate` (float, clamped `≥ MIN_TICKRATE = 1.0`), the derived `nanosecondsPerTick = 1e9 / tickrate`, `runGameElements` (whether the world simulates this tick), `isFrozen`, and `frozenTicksToRun` (remaining single-step count while frozen). Its own `tick()` sets `runGameElements = !isFrozen || frozenTicksToRun > 0` and decrements `frozenTicksToRun` when stepping — this is the mechanism behind the `/tick freeze` and `/tick step <n>` commands: freezing stops world simulation but the *server* keeps ticking (connections, autosave, etc. still run — only `tickChildren`'s `runsNormally()`-gated sections stop).

`ServerTickRateManager` (server-side subclass) adds the **sprint** feature (`/tick sprint <ticks>`): `requestGameToSprint(n)` sets `remainingSprintTicks = scheduledCurrentSprintTicks = n`, remembers the pre-sprint frozen state, and force-unfreezes. While `isSprinting()` (`scheduledCurrentSprintTicks > 0`) is true, each pass through `runServer`'s scheduling step calls `checkShouldSprintThisTick()`, which — as long as `runGameElements` is true — decrements `remainingSprintTicks` and returns true, causing that tick to run with **zero artificial delay** (`thisTickNanos = 0`) back-to-back with the previous one, as fast as the hardware allows; `endTickWork()` (called after each sprint tick's `waitUntilNextTick` returns) accumulates real elapsed nanoseconds into `sprintTimeSpend`. When the sprint's tick budget is exhausted, `finishTickSprint()` computes the achieved ticks/second and average ms/tick, reports it via the server's command source (`commands.tick.sprint.report`), restores the pre-sprint frozen state, and calls `server.onTickRateChanged()`. `setTickRate(...)` (the `/tick rate` command) and `setFrozen(...)` both push a `ClientboundTickingStatePacket` to all players; step requests additionally push `ClientboundTickingStepPacket`. A newly-joining player is synced via `updateJoiningPlayer(...)`, which sends both packets immediately.

### 3.9 `BlockableEventLoop<R>` / `ReentrantBlockableEventLoop<R>` — the cooperative task model

This pair (`net.minecraft.util.thread`) is the generic mechanism `MinecraftServer` (via `TickTask`) and many other single-threaded owners in the codebase (chunk executors, client render thread, etc.) build on. `BlockableEventLoop<R extends Runnable>` implements `java.util.concurrent.Executor` plus `TaskScheduler<R>`:

- A `ConcurrentLinkedQueue<R>` (`pendingRunnables`) holds tasks scheduled from *other* threads.
- `execute(Runnable)` wraps the runnable via the subclass's `wrapRunnable` (for `MinecraftServer` this produces a `TickTask` stamped with the current `tickCount`) and either enqueues it (`schedule`, which also `LockSupport.unpark`s the owning thread) if `scheduleExecutables()` says the caller is on a different thread, or runs it **immediately, inline**, if the caller is already on the owning thread.
- `submit(Runnable/Supplier)` wraps the same dispatch in a `CompletableFuture`; `executeBlocking(Runnable)` is the synchronous variant — schedules-and-joins if off-thread, runs inline if on-thread.
- `pollTask()` dequeues and runs at most one task, gated by the subclass's `shouldRun(R)` predicate *unless* `shouldRunAllTasks()` (true whenever `managedBlock` is currently reentered, tracked via a `blockingCount` counter) forces it through regardless.
- `managedBlock(BooleanSupplier condition)` is the actual blocking primitive: it increments `blockingCount`, then loops `while (!condition.get()) { if (!pollTask()) waitForTasks(); }`, decrementing on exit. This is what lets the server thread "block" waiting for e.g. an async chunk load to finish while *still* draining its own task queue instead of truly parking.
- `waitForTasks()` (default) does `Thread.yield()` then `LockSupport.parkNanos(100_000)` (0.1 ms) — a short poll-sleep, not a true blocking wait, because a task can be scheduled at any moment from any thread via `unpark`.
- `doRunTask(R)` wraps execution in a Tracy zone (`"Task"`) and swallows/logs non-fatal exceptions (rethrowing only `OutOfMemoryError`/`StackOverflowError`, including ones nested inside a `ReportedException`, via `isNonRecoverable`).
- A static `delayedCrash` (`Supplier<CrashReport>`) lets a **worker thread** (see `Util.onThreadException`) mark a crash that gets re-thrown on the *owning* thread the next time it calls `pollTask()` — this is how a background-executor exception (e.g. a chunk-generation OOM) reliably kills the main server tick loop instead of silently vanishing into a worker-thread stack trace.

`ReentrantBlockableEventLoop<R>` adds exactly one thing: a `reentrantCount` incremented around every `doRunTask`, and `scheduleExecutables()` returns true (i.e. "treat me as off-thread, queue it") whenever `reentrantCount != 0` **even if actually called from the owning thread**. This exists so that a task *running inside* `pollTask()` that itself calls `execute()`/`submit()` gets queued for a later iteration rather than recursing synchronously — `MinecraftServer` extends this class directly.

`MinecraftServer` layers tick-awareness on top: `TickTask` (record-like: `getTick()` + `Runnable`) is the queued unit; `shouldRun(task)` allows a task through once `task.getTick() + MAX_TICK_LATENCY(3) < tickCount` (a task that's been waiting 3+ ticks is forced through regardless of `haveTime()`) or once `haveTime()` is true. `haveTime()` returns true if the calling code is itself inside a reentrant task (`runningTask()`) or if "now" is still before whichever deadline currently applies — `delayedTasksMaxNextTickTimeNanos` while `mayHaveDelayedTasks` is set (i.e. right after a tick, giving queued tasks a little extra room), else the plain `nextTickTimeNanos`. `pollTaskInternal()` layers chunk-source task polling on top of the base queue: after the base `BlockableEventLoop.pollTask()` finds nothing, and only if sprinting, `shouldRunAllTasks()`, or `haveTime()` is true, it round-robins every loaded level's `ChunkSource.pollTask()` — this is the hook point where chunk generation/loading work interleaves with main-thread ticking (owned in depth by the world/chunks domain). `waitForTasks()` is overridden to park for the *actual* remaining time until `nextTickTimeNanos` (falling back to the default 100 µs poll only when not specifically waiting for next tick), and separately tracks `idleTimeNanos` for the tick-time debug chart when logging is enabled.

Beyond `BlockableEventLoop` itself, the same package supplies `AbstractConsecutiveExecutor`/`ConsecutiveExecutor`/`PriorityConsecutiveExecutor`: a lighter-weight pattern (own status state machine `SLEEPING → RUNNING → CLOSED`, a `StrictQueue` — either a plain FIFO or a `FixedPriorityQueue` with `N` parallel queues drained lowest-index-first) that self-reschedules onto a supplied backing `Executor` (typically `Util.backgroundExecutor()`) exactly once per non-empty state transition rather than being polled by an owning thread's tick loop — used by chunk/entity-persistence pipelines rather than by `MinecraftServer` itself, but it is the sibling mechanism worth knowing when reimplementing this package as a whole.

### 3.10 World clock & timeline system (new data-driven day/night machinery)

26.2 replaces the historically hard-coded `dayTime % 24000` day/night logic with a generic, datapack-defined clock system, owned end-to-end by `net.minecraft.world.clock` but wired into the server lifecycle exactly like any other `SavedData`:

- **`WorldClock`** is a near-empty registry element (a `Holder<WorldClock>` is just an identity via `Registries.WORLD_CLOCK`); vanilla registers exactly two, `minecraft:overworld` and `minecraft:the_end` (`WorldClocks.bootstrap`). A clock has no behavior of its own — it is purely an addressable "which timeline am I" key.
- **`ServerClockManager`** (a `SavedData`, type key `minecraft:world_clocks`, registered in `MinecraftServer`'s constructor and initialized via `init(server)`) holds one `ClockInstance` per registered `WorldClock`: `totalTicks` (long), `partialTick` (float, for non-integer `rate`), `rate` (float, default 1.0), `paused` (bool). `init()` also asks the `Registries.TIMELINE` registry (the `minecraft:timeline` datapack folder) to register every clock's **time markers** into the matching instance.
- **`tick()`** runs from `tickChildren`'s `"clocks"` step, gated on the `ADVANCE_TIME` game rule (not merely on `runsNormally()`/freeze state — a separate, player-facing toggle): each instance accumulates `partialTick += rate`, folds whole ticks into `totalTicks`.
- **Time markers** (`ClockTimeMarker`, keyed under `ClockTimeMarkers` — `day`, `noon`, `night`, `midnight`, `wake_up_from_sleep`, `roll_village_siege`) are named points on a clock's cycle, defined per-timeline in the datapack (e.g. `minecraft:timeline/day.json` binds to `clock: "minecraft:overworld"`, `period_ticks: 24000`, and gives each marker a tick offset — `noon` at 6000, `night` at 13000, `midnight` at 18000, `day` at 1000). Game code that used to hard-code "is it past 13000" now calls `moveToTimeMarker(clock, NIGHT)`/`isAtTimeMarker(...)` — the debug-world spawn setup (`setupDebugLevel`) uses exactly this to jump the overworld clock to `NOON`.
- Modifying a clock (`setTotalTicks`, `moveToTimeMarker`, `addTicks`, `setPaused`, `setRate`) always: applies the change, broadcasts a `ClientboundSetTimePacket` carrying only that clock's `ClockNetworkState` delta to every player, marks the `SavedData` dirty, and invalidates every loaded level's `EnvironmentAttributes` tick cache — because (out of scope for this document, owned by game-mechanics/world) a timeline JSON can also drive arbitrary **keyframe tracks** (`tracks` in the same JSON: sky color, fog color, moon/sun/star angle, "monsters burn", "bees stay in hive", etc.) that other systems sample by ticks-into-cycle. `forceGameTimeSynchronization()` (every 20 ticks, 3.7.2 step 4) independently re-broadcasts current time as a periodic drift-correction, decoupled from clock changes.
- Persistence: `ServerClockManager` (de)serializes through `PackedClockStates` (`Map<Holder<WorldClock>, ClockState>`), so clock position/rate/pause survive a save/reload like any other `SavedData`.

### 3.11 Shutdown (`stopServer` / `halt`)

`halt(boolean wait)` is the only externally-safe way to stop the server: it flips `running = false` (read by the tick loop's `while` condition, so the *next* time control returns to the loop head it exits) and, if `wait`, blocks the caller on `serverThread.join()`. It is called from the JVM shutdown hook (`Main`'s `"Server Shutdown Thread"`, `wait=true`) and from the RCON/console `/stop` command path.

Once the tick loop's `while` exits, its `finally` runs `stopServer()` unconditionally (even after a crash — see 3.7): `packetProcessor.close()` (rejects further inbound scheduling), any in-progress metrics recording is cancelled, `getConnection().stop()` closes the Netty listener, all players are saved and force-removed (`playerList.saveAll()` / `removeAll()`), every level's `noSave` flag is cleared (undoing any programmatic "don't save" state), then a loop repeatedly nudges chunk-source ticket deactivation and ticks each level's `ChunkSource` (`tick(() -> true, false)`) via `waitUntilNextTick()` until `chunkMap.hasWork()` is false everywhere — i.e. shutdown **actively drains pending chunk I/O** rather than abandoning it. A final `saveAllChunks(false, true, false)` (flush=true) forces everything to disk, every `ServerLevel.close()` runs, `savedDataStorage.close()` and the resource manager close, and the level-storage lock file is released. `DedicatedServer.stopServer()` additionally emits a `serverShuttingDown()` notification first and calls `Util.shutdownExecutors()` last (awaits up to 3 s each on `Util.backgroundExecutor()` and `Util.ioPool()`). `onServerExit()` (run after `stopServer()`, also always) is where `DedicatedServer` closes the text-filter client, the Swing GUI, RCON/GS4 threads, and the JSON-RPC server.

### 3.12 Watchdog (`ServerWatchdog`)

Started only when `max-tick-time > 0` (default 60000 ms). Runs its own daemon thread loop: while the server is running, it reads `server.getNextTickTime()` (the same `nextTickTimeNanos` the tick loop itself maintains) and compares against "now". If the gap exceeds `maxTickTimeNanos` (`max-tick-time` property × 1e6), it treats the server as **hung** (not merely slow): logs a fatal-marked error, builds a full `CrashReport` (including a thread dump of every JVM thread, sorted daemon-then-state-then-name, with the *server thread's* stack trace attached to a synthetic `Error("Watchdog (...)")`), attaches per-level watchdog stats and the random-tick-speed game rule, prints and saves the report, then calls its `exit()`: schedules a `Runtime.getRuntime().halt(1)` 10 seconds out (`MAX_SHUTDOWN_TIME`) as a hard-kill safety net, then immediately calls `System.exit(1)` (which itself may block on shutdown hooks — hence the safety-net timer forcibly halting the JVM if that also hangs). Between checks it sleeps for exactly the remaining time until the next check would be due (`(nextTickTime + maxTickTimeNanos - now) / 1e6` ms), so it does not busy-poll.

### 3.13 Profiling infrastructure

Three independent, cooperating layers instrument the tick loop:

- **`ProfilerFiller` / `Profiler` (push/pop zone tree).** `Profiler.get()` returns the current thread's active filler (a `ThreadLocal`), defaulting to a Tracy-backed filler (`TracyZoneFiller`, active whenever `TracyClient.isAvailable()` — i.e. the native jtracy library loaded) or an `InactiveProfiler` no-op otherwise. `Profiler.use(filler)` (used once per tick around the whole `runServer` body) *combines* the caller's filler with the thread's default one via `ProfilerFiller.combine`, so a Tracy zone and, e.g., a `SingleTickProfiler` snapshot can both receive every `push`/`pop` call transparently. `push(name)`/`pop()`/`popPush(name)` delimit nested named zones (`"tick"` → `"scheduledPacketProcessing"`, `"tick"` → `"levels"` → per-level → `"tick"`, etc., matching the call structure in 3.7/3.7.2); `incrementCounter` feeds a named running counter (rendered as a Tracy "plot").
- **jtracy / Tracy integration.** `com.mojang.jtracy` (a native-backed binding, not part of the decompiled Java sources — treat its surface as the constants below) exposes `TracyClient.beginZone(name[, function, file, line])` → `Zone` (closed to end the zone), `TracyClient.createPlot(name)` for counters, `TracyClient.createDiscontinuousFrame(name)` (`MinecraftServer` creates one named `"Server Tick"` and calls `.start()`/`.end()` around `processPacketsAndTick` each tick — this is what makes each server tick appear as a distinct frame in the Tracy profiler UI), and `TracyClient.setThreadName(name, colorSeed)` (called by every custom thread factory in `Util`/`EventLoopGroupHolder` so worker threads show up named in Tracy). `TracyZoneFiller` additionally walks the Java call stack (`StackWalker`, 5-frame limit) to attach source function/file/line to each zone, but only when `SharedConstants.IS_RUNNING_IN_IDE` — in production builds zones carry names only, to avoid stack-walking overhead every tick.
- **JFR (`net.minecraft.util.profiling.jfr`).** `JvmProfiler.INSTANCE` resolves to a real `JfrProfiler` only if the `jdk.jfr` module is present *and* `FlightRecorder.isAvailable()`; otherwise a `NoOpProfiler`. Started either via `--jfrProfile` at process start or via `JvmProfiler.INSTANCE.start(Environment.from(server))` around initial level loading (gated by the `DEBUG_JFR_PROFILING_ENABLE_LEVEL_LOADING` debug flag). Custom JFR event types exist for server tick time, chunk generation, structure generation, region-file I/O, packets sent/received, and client FPS — each with a matching `*Stat`/`TimedStat` value class under `jfr.stats` for offline summarization (`SummaryReporter`, `JfrStatsParser`) into the `.txt`/JSON reports written alongside a `.jfr` capture.
- **`SingleTickProfiler` — automatic slow-tick dump.** Only active when `SharedConstants.DEBUG_MONITOR_TICK_TIMES` is set. Wraps every tick in its own `ActiveProfiler` (independent of the Tracy filler, combined via `ProfilerFiller.combine`) and, if the tick's total duration meets or exceeds `SharedConstants.MAXIMUM_TICK_TIME_NANOS` (300 ms), writes a full profiler-results dump to `debug/tick-results-<timestamp>.txt`.
- **Metrics recording (`ActiveMetricsRecorder`/`InactiveMetricsRecorder`).** A separate, opt-in ("Save Server Profile" / `/debug start` style) sampling system: `startRecordingMetrics(onStopped, onFinished)` arms `willStartRecordingMetrics`; the *next* `createProfiler()` call (start of the next tick) lazily constructs an `ActiveMetricsRecorder` (using `ServerMetricsSamplersProvider`, `Util.timeSource()`, `Util.ioPool()`, and a `MetricsPersister("server")`) and every subsequent tick calls `startTick()`/`endTick()` on it until stopped/cancelled; on completion it schedules `saveDebugReport(...)` back onto the server thread via `executeBlocking`.
- **Debug sample charts (`net.minecraft.util.debugchart`).** `TpsDebugDimensions` (`FULL_TICK`, `TICK_SERVER_METHOD`, `SCHEDULED_TASKS`, `IDLE`) are the four channels sampled every tick into a `SampleLogger` — `DedicatedServer` uses a `RemoteSampleLogger` that only actually records (`isTickTimeLoggingEnabled()`) when `ServerDebugSubscribers.hasAnySubscriberFor(DEDICATED_SERVER_TICK_TIME)` is true, i.e. only while some connected client has that debug overlay open, to avoid the overhead otherwise. `startMeasuringTaskExecutionTime`/`finishMeasuringTaskExecutionTime` around `waitUntilNextTick` split its duration into "scheduled tasks" vs. "idle" for this chart.

## 4. Key types

| Class (package) | Role | Notable details |
|---|---|---|
| `Main` (`server`) | Process entry point | `public static void main(String[] args)`; owns CLI option definitions and the pre-server world-loading sequence |
| `Bootstrap` (`server`) | Static engine bootstrap | `bootStrap()` idempotent via `volatile boolean isBootstrapped`; `bootstrapDuration: AtomicLong` |
| `MinecraftServer` (`server`) | Abstract server core; the tick-loop owner | `extends ReentrantBlockableEventLoop<TickTask> implements CommandSource, ServerInfo, ChunkIOErrorReporter`; abstract `initServer()`; `static <S extends MinecraftServer> S spin(Function<Thread,S> factory)` |
| `DedicatedServer` (`server.dedicated`) | Concrete headless server | `extends MinecraftServer implements ServerInterface`; owns console/RCON/query/watchdog thread lifecycles |
| `ServerTickRateManager` (`server`) | Sprint/freeze runtime state, server-side | `extends TickRateManager`; `requestGameToSprint(int)`, `checkShouldSprintThisTick()`, `stepGameIfPaused(int)` |
| `TickRateManager` (`world`) | Base tick-rate/freeze state | `tickrate: float` (≥1.0), `nanosecondsPerTick: long`, `frozenTicksToRun: int` |
| `TickTask` (`server`) | Queued main-thread unit of work | `implements Runnable`; `int tick` (tick stamp at schedule time) + wrapped `Runnable` |
| `WorldLoader` (`server`) | Async registry/resource load pipeline | `static <D,R> CompletableFuture<R> load(InitConfig, WorldDataSupplier<D>, ResultFactory<D,R>, Executor background, Executor mainThread)` |
| `WorldStem` (`server`) | Immutable bundle of loaded resources+registries+level data | `record ... implements AutoCloseable` |
| `RegistryLayer` (`server`) | The 4-layer registry-access enum | `STATIC, WORLDGEN, DIMENSIONS, RELOADABLE` |
| `Services` (`server`) | Auth/session/profile-cache bundle | `record`; `static Services create(YggdrasilAuthenticationService, File nameCacheDir)` |
| `BlockableEventLoop<R>` (`util.thread`) | Generic single-owner-thread cooperative executor | `implements Executor, TaskScheduler<R>`; `managedBlock(BooleanSupplier)`, `pollTask()`, `BLOCK_TIME_NANOS = 100_000` |
| `ReentrantBlockableEventLoop<R>` (`util.thread`) | Adds re-entrancy-safe scheduling | `scheduleExecutables()` returns true whenever `reentrantCount != 0` |
| `AbstractConsecutiveExecutor<T>` (`util.thread`) | Self-rescheduling task queue on a backing `Executor` | Status state machine `SLEEPING/RUNNING/CLOSED`; `hasWork()`, `runAll()` |
| `StrictQueue` (`util.thread`) | Pluggable queue strategy for consecutive executors | `FixedPriorityQueue` (N parallel queues, drained low-index-first) and `QueueStrictQueue` (plain FIFO) |
| `PacketProcessor` (`network`) | Cross-thread inbound-packet handoff | `ConcurrentLinkedQueue`; `scheduleIfPossible(listener, packet)` from I/O threads, `processQueuedPackets()` on server thread |
| `ServerWatchdog` (`server.dedicated`) | Hang detector / hard-kill | `implements Runnable`; `MAX_SHUTDOWN_TIME = 10_000` ms |
| `GenericThread` (`server.rcon.thread`) | Base for RCON/GS4 listener threads | Owns a nullable `Thread thread`; `start()`/`stop()` with up-to-5-second join-and-warn loop |
| `EventLoopGroupHolder` (`server.network`) | Lazily-created, cached Netty `EventLoopGroup` per transport kind | Enum-like singletons `NIO`/`EPOLL`/`KQUEUE`/`LOCAL`; thread name pattern `"Netty {type} IO #%d"` |
| `Profiler` / `ProfilerFiller` (`util.profiling`) | Push/pop profiling zone tree, thread-local active filler | `Profiler.use(filler): Scope`; default filler is Tracy-backed or `InactiveProfiler` |
| `SingleTickProfiler` (`util.profiling`) | Automatic dump-on-slow-tick profiler | Active only if `DEBUG_MONITOR_TICK_TIMES`; threshold `SharedConstants.MAXIMUM_TICK_TIME_NANOS` |
| `JvmProfiler` (`util.profiling.jfr`) | JFR facade | `INSTANCE` resolves to `JfrProfiler` or `NoOpProfiler` depending on JVM module/feature availability |
| `ServerClockManager` (`world.clock`) | Per-dimension clock state, `SavedData`-backed | `TYPE` key `minecraft:world_clocks`; `tick()` gated on `GameRules.ADVANCE_TIME` |
| `WorldClock` / `ClockTimeMarker` (`world.clock`) | Registry element identifying a clock / a named point on its cycle | `Registries.WORLD_CLOCK`, `Registries.TIMELINE` (datapack-defined) |
| `Util` (`util`) | Thread pools, time source, misc helpers | See §5 for exact executor construction |

## 5. Constants & magic values

| Constant | Value | Source class |
|---|---|---|
| Tick rate (default) | `20` ticks/second, `50` ms/tick | `SharedConstants.TICKS_PER_SECOND` / `MILLIS_PER_TICK` |
| Nanoseconds per tick (default) | `1_000_000_000 / 20 = 50_000_000` | `TickRateManager.nanosecondsPerTick` (derived) |
| Minimum tick rate | `1.0` (float, via `/tick rate`) | `TickRateManager.MIN_TICKRATE` |
| Server thread name | `"Server thread"` | `MinecraftServer.SERVER_THREAD_NAME` |
| Server-thread priority bump | `8`, applied only if `availableProcessors() > 4` | `MinecraftServer.spin` |
| Overloaded-tick threshold | `1_000_000_000` ns (1 s) plus `20×` one tick | `MinecraftServer.OVERLOADED_THRESHOLD_NANOS` / `OVERLOADED_TICKS_THRESHOLD` |
| Overload-warning repeat interval | `10_000_000_000` ns (10 s) plus `100×` one tick | `MinecraftServer.OVERLOADED_WARNING_INTERVAL_NANOS` / `_TICKS_WARNING_INTERVAL` |
| Status (server-list ping) cache lifetime | `5_000_000_000` ns (5 s) | `MinecraftServer.STATUS_EXPIRE_TIME_NANOS` |
| `prepareLevels()` poll delay | `10_000` ns (10 µs) | `MinecraftServer.PREPARE_LEVELS_DEFAULT_DELAY_NANOS` |
| Max status player-list sample | `12` | `MinecraftServer.MAX_STATUS_PLAYER_SAMPLE` |
| Autosave interval (ticks) | `6000` (5 min @ 20 TPS) | `MinecraftServer.AUTOSAVE_INTERVAL` |
| Autosave floor (ticks) | `100` | `MinecraftServer.MIMINUM_AUTOSAVE_TICKS` |
| Autosave wall-clock target | `300` s | `MinecraftServer.computeNextAutosaveInterval` (local constant) |
| Forced-task latency | task allowed through after `3` ticks even without spare time | `MinecraftServer.MAX_TICK_LATENCY` |
| Tick-time smoothing factor | `0.8` (EMA weight on prior value) | `MinecraftServer.AVERAGE_TICK_TIME_SMOOTHING` |
| Tick-time ring buffer size | `100` samples | `MinecraftServer.TICK_STATS_SPAN` / `tickTimesNanos` array length |
| Absolute max world size | `29_999_984` blocks | `MinecraftServer.ABSOLUTE_MAX_WORLD_SIZE` |
| Spawn position search radius | `5` chunks (11×11 spiral, capped `±5`) | `MinecraftServer.SPAWN_POSITION_SEARCH_RADIUS` |
| Server-activity-monitor interval | `30` s between notifications | `MinecraftServer.SERVER_ACTIVITY_MONITOR_SECONDS_BETWEEN_NOTIFICATIONS` |
| `BlockableEventLoop` idle-poll interval | `100_000` ns (0.1 ms) | `BlockableEventLoop.BLOCK_TIME_NANOS` / `waitForTasks` |
| Max single tick before "slow tick" dump | `300` ms | `SharedConstants.MAXIMUM_TICK_TIME_NANOS` |
| Default `max-tick-time` (watchdog threshold) | `60_000` ms (1 minute) | `DedicatedServerProperties.maxTickTime` |
| Default `pause-when-empty-seconds` | `60` s | `DedicatedServerProperties.pauseWhenEmptySeconds` |
| Watchdog hard-kill safety timer | `10_000` ms after `System.exit` | `ServerWatchdog.MAX_SHUTDOWN_TIME` |
| Old-user-conversion retries | `2`, with `5000` ms backoff | `DedicatedServer.CONVERSION_RETRIES` / `CONVERSION_RETRY_DELAY_MS` |
| Background executor thread cap | `clamp(cpuCount − 1, 1, 255)`, overridable via `-Dmax.bg.threads` | `Util.maxAllowedExecutorThreads` / `getMaxThreads` |
| Main background executor name | pool `"Main"`, threads `"Worker-Main-N"` | `Util.BACKGROUND_EXECUTOR` (`makeExecutor`) |
| I/O pool | cached pool, threads `"IO-Worker-N"`, non-daemon | `Util.IO_POOL` |
| Non-critical/download pool | cached pool, threads `"Download-N"`, daemon | `Util.DOWNLOAD_POOL` |
| Executor shutdown grace | `3` s each, background + I/O pools | `Util.shutdownExecutors` |
| RCON/GS4 thread stop wait | up to `5` × 1 s joins before forced interrupt | `GenericThread.MAX_STOP_WAIT` |
| Netty IO thread name pattern | `"Netty {NIO\|Epoll\|Kqueue\|Local} IO #%d"`, daemon | `EventLoopGroupHolder.createThreadFactory` |
| Timer-hack thread sleep | `Integer.MAX_VALUE` ms | `Util.startTimerHackThread` |
| Overworld day length | `24000` ticks (per `timeline/day.json`, `period_ticks`) | data-driven, `world_clock`/`timeline` datapack files |
| Default random tick speed | `3` | `SharedConstants.DEFAULT_RANDOM_TICK_SPEED` |
| Max chained neighbor updates (default) | `1_000_000` | `SharedConstants.MAX_CHAINED_NEIGHBOR_UPDATES` / `DedicatedServerProperties` |

## 6. Cross-subsystem interfaces

**Consumes from:**
- **World/chunks (03):** `ServerLevel`, `ChunkSource`/`ChunkMap` — `tickChildren` drives `level.tick(haveTime)`; `pollTaskInternal` drains chunk-source tasks each main-thread poll; shutdown drains `chunkMap.hasWork()`.
- **Networking (02):** `ServerConnectionListener` (bind/accept/tick), `PacketProcessor` (inbound packet queue drained once per tick before the game tick body), `EventLoopGroupHolder` (Netty transport selection).
- **Game mechanics (05):** `PlayerList`, `GameRules` (`ADVANCE_TIME` gates clock ticking; several rule changes trigger immediate packet broadcasts via `onGameRuleChanged`), `CustomBossEvents`, `ServerFunctionManager`, `GameTestTicker`.
- **Modding (06):** feature-flag-gated pack selection (`configurePackRepository`) and reload (`reloadResources`) both run through this domain's scheduling (`managedBlock` when called from the server thread).
- **World generation (04):** initial spawn-position search inside `createLevels`/`setInitialSpawn` calls into chunk generator/biome-sampler APIs owned there.

**Provides to every other subsystem:**
- The `MinecraftServer` instance itself as the single source of truth for "am I on the server thread" (`isSameThread()`), "how much tick budget is left" (`haveTime()`), and "schedule this for the main thread" (`execute`/`submit`/`executeBlocking`, inherited from `BlockableEventLoop`) — essentially every other domain's cross-thread-safety story routes through this.
- `ProfilerFiller`/`Profiler.get()` — the ambient per-thread profiling handle every ticking subsystem is expected to `push`/`pop` around its own work.
- `ServerTickRateManager`/`TickRateManager` — the authoritative "is the world currently simulating" signal (`runsNormally()`, `isEntityFrozen(entity)`) consulted by entity AI, redstone, and mob spawning.
- `ServerClockManager` — the authoritative current-time-per-dimension source, replacing direct `level.getDayTime()` reads for anything that needs named time-of-day semantics.
- `Util.backgroundExecutor()` / `Util.ioPool()` — the two general-purpose thread pools essentially every other domain's async work (chunk gen, region I/O, network compression, structure template loading, etc.) is expected to run on, rather than spawning ad hoc threads.

## 7. Data-generator cross-reference

| File(s) | Contents relevant to this domain |
|---|---|
| `data/minecraft/world_clock/overworld.json`, `.../the_end.json` | Registers the two vanilla `WorldClock` instances (currently empty bodies — the clock element itself carries no data, only an identity) |
| `data/minecraft/timeline/day.json` | The Overworld's day/night timeline: `clock: "minecraft:overworld"`, `period_ticks: 24000`, the six `ClockTimeMarkers` with their tick offsets (`day`@1000, `noon`@6000, `night`@13000, `midnight`@18000, plus `roll_village_siege`@18000 and `wake_up_from_sleep`@0), and the full set of environment-attribute keyframe `tracks` (sky/fog/cloud color, sun/moon/star angle & brightness, "monsters burn", "bees stay in hive", etc. — consumed outside this domain) |
| `data/minecraft/timeline/early_game.json`, `moon.json`, `villager_schedule.json` | Additional timelines layering more keyframe tracks / markers onto the same or other clocks (not deep-dived here — see game-mechanics domain for track semantics) |
| `reports/registries.json` | Does **not** list `minecraft:world_clock`/`minecraft:timeline`/`minecraft:clock_time_marker` — these are datapack (world-defined) registries, not built-in ones, so they only appear as folders under `data/minecraft/...`, not in the built-in registry report |
| `reports/packets.json` | Cross-reference for `ClientboundSetTimePacket`, `ClientboundTickingStatePacket`, `ClientboundTickingStepPacket` — the three packets this domain emits (owned in full by the networking domain) |

No `blocks.json`/`registries.json` (built-in) entries are specific to bootstrap/lifecycle itself — this domain is almost entirely code-driven rather than data-driven, aside from the world-clock/timeline datapack registries above.

## 8. Notes for Rusty Clanker

- **The tick loop's "catch-up" behavior is a determinism hazard to name explicitly, not hide.** Vanilla does not run missed ticks when overloaded — it silently fast-forwards `nextTickTimeNanos` and accepts the skipped simulation. A from-scratch engine that instead tries to "catch up honestly" by running the backlog of ticks will diverge from vanilla's observed behavior under load (and can create a runaway spiral, since each backlog tick takes just as long as the one that caused the backlog). ARCH design should decide up front whether Rusty Clanker mimics vanilla's tick-skipping or deliberately documents a bounded exception (per the project's parity-exception policy) if it chooses to genuinely simulate backlog ticks.
- **`haveTime()`'s two-tier deadline (`nextTickTimeNanos` vs. `delayedTasksMaxNextTickTimeNanos`) is subtle and load-bearing.** It exists specifically so that main-thread tasks scheduled *during* a tick get a little extra runway right after that tick (rather than being immediately starved by an already-expired deadline), without changing the deadline used for tick-overload detection. A naive single-deadline reimplementation will produce measurably different task-latency characteristics (and, in edge cases, different tick-to-tick scheduling of chunk work) even if the raw tick cadence matches.
- **The `MAX_TICK_LATENCY = 3`-tick force-run rule is an implicit fairness guarantee** ("a scheduled task is never delayed more than 3 ticks even under a starved/overloaded main thread") that nothing else in the codebase documents explicitly — it is easy to omit when reimplementing `shouldRun` and get subtly different task-starvation behavior under load.
- **Reentrant task scheduling (`ReentrantBlockableEventLoop`) is what prevents unbounded recursion when a task itself calls `execute()`.** A straightforward "if on my thread, run synchronously, else queue" without the reentrancy counter is a correctness trap the moment any tick-loop code path re-enters `execute()` from inside an already-running task (this happens routinely — e.g. a chunk-load callback scheduling a follow-up task). Rusty Clanker's ECS-scheduler equivalent needs the same "am I nested" tracking if it exposes a similar main-thread-affinity API to mods.
- **The watchdog reads the *same* `nextTickTimeNanos` field the tick loop itself advances — it is not an independent timer.** This coupling means the watchdog's threshold is measured against "how far behind the intended schedule are we", not "how long has the current tick actually been running" — a hung single tick and a server merely running consistently slow both eventually trip it the same way. A cluster-mode reimplementation with per-region tick loops needs to decide whether each region gets its own watchdog thread/threshold or whether a single supervisor watches all of them.
- **Shutdown actively drains chunk I/O to completion before flushing final saves** (the `while (... hasWork()) { ...; waitUntilNextTick(); }` loop in `stopServer`) rather than simply cancelling in-flight work — this ordering (drain, *then* force-flush-save) matters for save-file consistency and is easy to get wrong by parallelizing shutdown steps that vanilla deliberately serializes.
- **`ReentrantBlockableEventLoop`'s crash-propagation mechanism (`delayedCrash` static + `BlockableEventLoop.relayDelayCrash`) is the only path by which a background-thread exception reliably kills the server tick loop** rather than being logged and ignored. Any reimplementation of the background-executor error handling needs an equivalent "poison the main loop" signal, or background-thread failures (e.g. corrupted chunk generation) will silently continue running an inconsistent world.
- **The tick-rate/sprint/freeze machinery changes what "one tick" means to *every* other subsystem**, not just to this one: `TickRateManager.isEntityFrozen(entity)` (frozen unless it's a player or carries a player passenger) is consulted directly by entity AI, and `runsNormally()` gates whole sections of `tickChildren` (clocks, game tests) as well as redstone/scheduled-tick processing elsewhere. Any subsystem that ticks independently of the central `tickChildren` call (e.g. a hypothetical parallel worker) must consult the same freeze/sprint state or vanilla-parity `/tick freeze`+`/tick step` semantics will silently break for that subsystem.
- **The world-clock/timeline system is a genuine architectural break from older versions' hard-coded day/night handling** and is worth treating as first-class data (own registries, own `SavedData`) in Rusty Clanker's world-persistence design from the start, rather than bolting a "current time" integer onto `ServerLevel` and retrofitting named markers later — vanilla itself just made exactly that migration in this version.
- **Profiling/threading names are part of observable behavior for anyone attaching a JFR/Tracy/`jstack` capture to compare against vanilla for parity debugging.** Reusing vanilla's exact thread names (`"Server thread"`, `"Server Watchdog"`, `"IO-Worker-N"`, `"Netty {type} IO #N"`, etc.) and profiler zone names (`"tick"` → `"levels"` → `"tick"`, etc.) costs little and makes side-by-side diagnostic comparisons with a real vanilla server dramatically easier during Rusty Clanker's parity-testing phase (TEST domain).
- **`Util.blockUntilDone`'s "borrow the calling thread as a task executor" pattern** (used to synchronously drive the otherwise-async `WorldLoader` pipeline from `Main`) is a reusable idiom worth keeping for Rusty Clanker's own startup sequencing, rather than inventing a second bespoke "block until async thing finishes" mechanism alongside the tick loop's `managedBlock`.
