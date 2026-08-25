# Registry & Tag Synchronization (Configuration Phase) — Debugging Deep Dive

## Provenance

Produced by directly reading the locally-decompiled, officially-distributed 26.2
server jar at `C:\Users\krank\mc-research\26.2\src` (per ASSET-D18(f); method and
legal posture as recorded in this corpus's `README.md` — same source binary,
same Vineflower decompile, never committed, never copied verbatim). Classes read
in full or in relevant part for this document: `RegistryDataLoader`,
`RegistrySynchronization`, `RegistryLoadTask`, `NetworkRegistryLoadTask`,
`ResourceManagerRegistryLoadTask`, `RegistryOps`, `RegistryCodecs`,
`HolderSetCodec`, `MappedRegistry`, `HolderSet`, `RegistryValidator`,
`TagNetworkSerialization`, `ClientboundUpdateTagsPacket`,
`ConfigurationProtocols`, `DimensionType`, `DimensionTypes` (data bootstrap),
`Enchantment`, `Enchantments` (data bootstrap, sampled), `SulfurCubeArchetype`,
`SulfurCubeArchetypes` (data bootstrap), `EnvironmentAttribute`,
`EnvironmentAttributeMap`, `EnvironmentAttributes`, `TimelineTags`,
`EnchantmentTags`, `DialogTags`, `BlockTags` (grep), `ItemTags` (grep),
`Dialogs` (data bootstrap), `DialogTagsProvider`. Two real 26.2 client crash
reports were read as primary evidence of observed (not decompiled) behavior:
`disconnect-2026-08-25_17.13.34-client.txt` and `-16.17.03-client.txt`.

**No client-side source exists in the decompiled tree** — only
`net.minecraft.client` is absent entirely (0 files under that package). Every
client-side claim below (`RegistryDataCollector`, `ClientConfigurationPacketListenerImpl`)
is therefore inferred from (a) the two crash reports' stack traces, which name
exact client method call chains, and (b) the *shared* code both client and
server load (`RegistryDataLoader`, `MappedRegistry`, `HolderSetCodec`, etc. —
all under `net.minecraft.resources`/`net.minecraft.core`/`net.minecraft.tags`,
compiled once and used by both sides). Claims resting only on inference from
shared code and the crash-report stack traces (not on directly-read client
source) are explicitly flagged **INFERRED** below. All descriptions,
explanations, and pseudocode are original wording; no method body or comment
is transcribed from the decompiled source. Short verbatim strings quoted below
(e.g. `"Unbound tags in registry "`) are runtime error-message literals that
already appear, unmodified, in the two crash-report files the user captured
from their own client — they are quoted for diagnostic traceability, not
copied as an end run around the no-verbatim rule for source code.

## 1. Purpose

This document exists to answer one debugging question precisely: **why does a
real, unmodified vanilla 26.2 client reject our server's Configuration-phase
registry sync**, and what is the minimal correct fix. It supersedes guessing
from the crash log alone by tracing the exact mechanism in vanilla's own
shared registry-loading code.

## 2. The observed failure (ground truth)

Two client crash reports, both `IllegalStateException: Failed to load
registries due to errors`, thrown from `RegistryDataLoader.createReportWithBriefInfo`,
reached via `RegistryDataCollector.collectGameRegistries` →
`ClientConfigurationPacketListenerImpl.handleConfigurationFinished`, i.e. the
client processes **all** buffered registry/tag data in one batch exactly when
it receives `ClientboundFinishConfigurationPacket`, not incrementally as each
`RegistryData` packet arrives (**INFERRED** from the stack trace; see §4.4).

The current-state report (`17.13.34`, server always sending `has_data=false`)
shows two distinct error shapes:

- **Per-entry parse failures** — `"Failed to parse <id> from pack vanilla"` —
  for every entry of exactly three registries: `minecraft:dimension_type` (4/4
  entries), `minecraft:enchantment` (43/43 entries), `minecraft:sulfur_cube_archetype`
  (12/12 entries). All other 26 registries load cleanly.
- **Registry-level freeze failures** — `"Unbound tags in registry
  ResourceKey[minecraft:root / <registry>]: [<tag>, ...]"` — for exactly three
  registries: `minecraft:dialog` (`pause_screen_additions`, `quick_actions`),
  `minecraft:enchantment` (`exclusive_set/armor`, `boots`, `bow`, `crossbow`,
  `damage`, `mining`, `riptide`), `minecraft:timeline` (`in_end`, `in_nether`,
  `in_overworld`).

An earlier report (`16.17.03`, server sending real inline NBT for
`dimension_type`) shows a different, narrower failure —
`"Failed to parse value {height:384,min_y:-64} for key minecraft:overworld
from server"` — confirming the *current* `has_data=false` approach is strictly
closer to correct (that inline-NBT approach hand-authored an incomplete
payload against an unverified schema and is not analyzed further here; see
`play::world::SYNCHRONIZED_REGISTRIES`'s own doc comment for why it was
abandoned).

Both error shapes trace to **one root cause**, proven below by reading the
shared decode/freeze pipeline: **the server never sends the Configuration-phase
tag-synchronization packet.**

## 3. Server-side registry-sync flow

### 3.1 Driving classes

- `net.minecraft.resources.RegistryDataLoader` — owns three static tables of
  `RegistryData<T>` (registry key + element `Codec<T>` + optional
  `RegistryValidator<T>`): `WORLDGEN_REGISTRIES` (full server-side datapack
  load, 41 registries, `DIRECT_CODEC` throughout), `DIMENSION_REGISTRIES` (just
  `level_stem`), and — the one this document is about —
  **`SYNCHRONIZED_REGISTRIES`** (29 registries, the exact set/order sent to
  clients). It also owns the two `load(...)` entry points: a
  `ResourceManager`-backed overload (server datapack loading) and a
  `Map<ResourceKey, NetworkedRegistryData>`-backed overload (network loading,
  used identically by client and by any process consuming network-shaped
  registry data).
- `net.minecraft.core.RegistrySynchronization` — server-side packer.
  `packRegistries`/`packRegistry` iterate `RegistryDataLoader.SYNCHRONIZED_REGISTRIES`
  and, per entry, decide `has_data` (§3.3). `PackedRegistryEntry(id, data:
  Optional<Tag>)` is the on-wire per-entry shape. `NETWORKABLE_REGISTRIES` is
  just the key-set of `SYNCHRONIZED_REGISTRIES`, used to filter which
  registries are even eligible to go over the wire (`isNetworkable`).
- `net.minecraft.tags.TagNetworkSerialization` — server-side tag packer,
  `serializeTagsToNetwork(LayeredRegistryAccess)`. Iterates
  `RegistrySynchronization.networkSafeRegistries`, which is the **union** of
  the 29 synchronized/dynamic registries *and* every registry in
  `RegistryLayer.STATIC` (block, item, entity_type, fluid, game_event, and
  every other built-in-Java-registered registry) — see §5.1, this union is the
  single most important fact in this document.
- `net.minecraft.network.protocol.configuration.ClientboundRegistryDataPacket` —
  wire packet carrying one registry's `PackedRegistryEntry` list per packet
  (registry id + entry list; `has_data` is per-entry, encoded as the presence
  of NBT payload bytes, matching our own `RegistryDataEntryOut`).
- `net.minecraft.network.protocol.common.ClientboundUpdateTagsPacket` — wire
  packet carrying **all** registries' tag payloads in one packet (§5). Declared
  in `network.protocol.common` (not `.configuration`) because the identical
  class is reused, unmodified, by the Play-phase tag-sync path too — this is
  the "packet shared verbatim by configuration and play" already catalogued in
  `02-network-protocol.md`'s package table.
- `net.minecraft.server.network.config.SynchronizeRegistriesTask` (named,
  package location confirmed by `02-network-protocol.md`'s existing package
  table; not independently re-read for this document) — the server-side
  Configuration-phase task that presumably invokes `packRegistries` +
  `serializeTagsToNetwork` and enqueues both packet kinds. Not opened directly
  for this pass; flagged **not independently verified** below.

### 3.2 The exact ordered `SYNCHRONIZED_REGISTRIES` list (29 registries)

Reproduced here as a **functional fact** (registry key + which codec variant —
`NETWORK_CODEC` vs `DIRECT_CODEC` — is used for network sync), not as
transcribed source. In `RegistryDataLoader.SYNCHRONIZED_REGISTRIES`'s own
declared order:

| # | Registry (`minecraft:` prefix implied) | Codec used for network sync |
|---|---|---|
| 1 | `worldgen/biome` | `Biome.NETWORK_CODEC` (lighter than the full worldgen `DIRECT_CODEC` — omits generation-only fields like density functions/carvers a client never needs) |
| 2 | `chat_type` | `DIRECT_CODEC` |
| 3 | `trim_pattern` | `DIRECT_CODEC` |
| 4 | `trim_material` | `DIRECT_CODEC` |
| 5 | `wolf_variant` | `NETWORK_CODEC`, validator: non-empty |
| 6 | `wolf_sound_variant` | `NETWORK_CODEC`, validator: non-empty |
| 7 | `pig_variant` | `NETWORK_CODEC`, validator: non-empty |
| 8 | `pig_sound_variant` | `NETWORK_CODEC`, validator: non-empty |
| 9 | `frog_variant` | `NETWORK_CODEC`, validator: non-empty |
| 10 | `cat_variant` | `NETWORK_CODEC`, validator: non-empty |
| 11 | `cat_sound_variant` | `NETWORK_CODEC`, validator: non-empty |
| 12 | `cow_sound_variant` | `DIRECT_CODEC`, validator: non-empty |
| 13 | `cow_variant` | `NETWORK_CODEC`, validator: non-empty |
| 14 | `chicken_sound_variant` | `DIRECT_CODEC`, validator: non-empty |
| 15 | `chicken_variant` | `NETWORK_CODEC`, validator: non-empty |
| 16 | `zombie_nautilus_variant` | `NETWORK_CODEC`, validator: non-empty |
| 17 | `painting_variant` | `DIRECT_CODEC`, validator: non-empty |
| 18 | `sulfur_cube_archetype` | `DIRECT_CODEC` (no `NETWORK_CODEC` variant exists) |
| 19 | `dimension_type` | `NETWORK_CODEC` |
| 20 | `damage_type` | `DIRECT_CODEC` |
| 21 | `banner_pattern` | `DIRECT_CODEC` |
| 22 | `enchantment` | `DIRECT_CODEC` (no `NETWORK_CODEC` variant exists) |
| 23 | `jukebox_song` | `DIRECT_CODEC` |
| 24 | `instrument` | `DIRECT_CODEC` |
| 25 | `test_environment` | `DIRECT_CODEC` |
| 26 | `test_instance` | `DIRECT_CODEC` |
| 27 | `dialog` | `DIRECT_CODEC` |
| 28 | `world_clock` | `DIRECT_CODEC` |
| 29 | `timeline` | `NETWORK_CODEC` |

Cross-checked against our own `SYNCHRONIZED_REGISTRIES` constant
(`crates/server/src/play/world.rs`): same 29 keys, same per-registry entry
lists, different declared order (ours groups `dimension_type`/`worldgen/biome`
first). **Order among these 29 `RegistryData` packets does not matter** — see
§4.4 for why (client batches everything before loading any of it) — so this is
a non-bug stylistic difference, not a fix-list item.

`enchantment_provider`, `recipe`, `configured_carver`, `configured_feature`,
`placed_feature`, `structure`, `structure_set`, `processor_list`,
`template_pool`, `noise_settings`, `noise`, `density_function`,
`world_preset`, `flat_level_generator_preset`, `trial_spawner_config`,
`multi_noise_biome_source_parameter_list`, `villager_trade`, `trade_set` all
appear in `WORLDGEN_REGISTRIES` (server datapack load) but are **absent** from
`SYNCHRONIZED_REGISTRIES` — never sent to the client at all, confirming they
are server-only generation-time registries with no client-visible wire
representation.

### 3.3 The `has_data` mechanism ("optional elements")

`RegistrySynchronization.packRegistry`, per element:

```
can_skip_contents = registry
    .registration_info(element.key)
    .known_pack_info()                      // which data pack this element came from
    .filter(|pack| client_known_packs.contains(pack))
    .is_some()

contents = if can_skip_contents { None } else { Some(encode(element)) }
```

In plain terms: the server tags every registered element with the identity of
the data pack it came from (`RegistrationInfo.knownPackInfo`). If the
*client's own* echoed `known_packs` list (from the Configuration `Select Known
Packs` exchange, our `KnownPacksServerbound`) already contains that pack, the
server omits the payload (`has_data=false`) — telling the client "you already
have this element's definition locally, from that pack; don't wait for bytes."
If the client didn't echo that pack (a modified pack, or the datapack was
added/changed at runtime), the server sends the real encoded contents
(`has_data=true`).

Our implementation always sends `has_data=false`, unconditionally, regardless
of the client's echoed `known_packs` — this is a **deliberate, documented
simplification** (see `configuration_flow.rs`'s own comment on the
`KnownPacksServerbound` gate), correct *only* because every entry name in
`SYNCHRONIZED_REGISTRIES` is a genuine stock-vanilla identifier the client's
built-in `minecraft:core@26.2` known pack already ships — so `has_data=false`
is always the right call *content-wise*, even though we never actually inspect
the client's echo to justify it. This part of the design is sound and is not
part of the bug.

## 4. Client-side path & why `has_data=false` parsing fails

### 4.1 `NetworkRegistryLoadTask` — the has_data=false branch

`RegistryDataLoader.load(entries, knownDataSource, contextRegistries,
registriesToLoad, executor)` is the network-loading overload; its
`LoaderFactory` constructs one `NetworkRegistryLoadTask<T>` per registry in
`registriesToLoad`. Per received `PackedRegistryEntry`:

```
if entry.data.is_some() {
    // has_data=true: decode the inline NBT the server sent, via RegistryOps<Tag>
    result = decode_from_network(codec, nbt_ops, entry.data)
    on_error: wrap as "Failed to parse value {nbt} for key {id} from server"
} else {
    // has_data=false: find the client's OWN bundled copy of this element
    // (packaged inside the client jar's built-in "vanilla" known pack) and
    // decode THAT, via RegistryOps<JsonElement> — never touches the network at all
    resource = known_data_source.find(registry_id, entry.id)   // e.g. data/minecraft/dimension_type/overworld.json
    result = decode_from_resource(codec, json_ops, resource)
    on_error: wrap as "Failed to parse {id} from pack {source_pack_id}"
}
```

This exactly matches both observed error message shapes in the two crash
reports (`"... from server"` vs `"... from pack vanilla"`) — confirming
`"pack vanilla"` in the current report means literally "the client's own
built-in copy," not anything the server transmitted.

**The decisive point:** decoding the built-in copy still goes through the
*same* `RegistryOps` "registry context" (`RegistryOps.RegistryInfoLookup`)
built for *this exact network-loading batch* — not some separate,
already-fully-resolved singleplayer-style context. Any `Holder`/`HolderSet`
field the built-in JSON contains is resolved against *this connection's*
in-progress registry state, which is exactly why a perfectly valid vanilla
built-in file can fail to parse over a network connection even though the
identical bytes parse fine in singleplayer (where every registry, static and
dynamic, is already fully loaded *and tag-bound* before any parsing starts).

### 4.2 Two different `HolderGetter` behaviors feed the same codec

`RegistryDataLoader.createContext` builds one `RegistryOps.RegistryInfoLookup`
map, populated in two passes, **second pass wins on key collision**:

1. `contextRegistries.forEach(... createInfoForContextRegistry(lookup))` — for
   each already-available registry (the client's static/built-in ones — item,
   block, entity_type, sound_event, attribute, etc. — plus, on other call
   sites, any previously-frozen layer). `createInfoForContextRegistry` wraps
   the registry's **own already-frozen `HolderLookup.RegistryLookup`** as both
   the `owner` and the `getter`.
2. `newRegistriesAndLoaders.forEach(... e.createRegistryInfo())` — for every
   registry in *this* load's own `registriesToLoad` (the 29 synchronized
   registries). `RegistryLoadTask.createRegistryInfo()` instead wraps a
   **`ConcurrentHolderGetter` over the registry's own in-progress
   `createRegistrationLookup()`**.

These two `HolderGetter` implementations behave completely differently for a
tag lookup (`HolderGetter<T>.get(TagKey<T>)`), and this difference is the
entire mechanism behind the bug:

- **Frozen/context registry's getter** (`MappedRegistry.get(TagKey)` on an
  already-frozen registry): `return this.allTags.get(id)` — `allTags` is fixed
  at that registry's own freeze time. If tag `id` was never bound before that
  freeze, this returns `Optional.empty()` — **permanently**, for the rest of
  this load.
- **Registration-phase getter** (`MappedRegistry.createRegistrationLookup()`'s
  own `get(TagKey)`): `return Optional.of(this.getOrCreateTagForRegistration(id))`
  — **always succeeds**, creating an *unbound* placeholder `HolderSet.Named` on
  first reference if one doesn't exist yet. Reading that placeholder's
  contents later (iterate/size/`contains`) throws
  `"Trying to access unbound tag '<key>' from registry <owner>"`, but merely
  *constructing* it during decode never throws.

`HolderSetCodec.decode`'s tag branch (`Either.left(TagKey)` path) calls
`registry.get(key)` on whichever `HolderGetter` `RegistryOps.getter(registryKey)`
returned, then:

```
lookup_tag(getter, key) =
    getter.get(key)
        .map(DataResult::success)
        .unwrap_or_else(|| DataResult::error("Missing tag: '<key>' in '<registry>'"))
```

So: **a tag reference into a registration-phase registry (one of the 29)
always decodes successfully** (creates a placeholder, deferred binding) — but
**a tag reference into an already-frozen context registry (item, block, ...)
fails outright at decode time with `"Missing tag"`** if that tag was never
bound before the context registry froze. This bifurcation is exactly why some
registries fail at decode (`"Failed to parse ... from pack vanilla"`) and
others only fail later at freeze (`"Unbound tags in registry ..."`) — they are
two different symptoms of the identical root cause (no tag data ever
delivered), striking at two different points in the pipeline depending on
which side of the frozen/registration-phase line the referenced tag's owning
registry falls on.

**Confidence: HIGH.** Every step above is read directly from shared code
(`RegistryOps`, `HolderSetCodec`, `MappedRegistry`, `RegistryLoadTask`,
`NetworkRegistryLoadTask`) compiled once and used by both client and server —
not inferred or guessed. The one assumption carried from Java semantics into
this description is that a real 26.2 multiplayer client freezes its static
registries (item, block, ...) **before** this network-registry-load batch
runs, and that freeze happens with **zero tags pre-bound** for a fresh
connection (i.e. static registry tags are not "baked in" from client startup
and reused across servers, but rebound per-connection from *this* server's own
Configuration-phase tag packet, exactly like `TagNetworkSerialization`'s own
inclusion of the static layer in its packed output already proves the
*server* side expects to do). This specific assumption about client-side
freeze *timing* is **INFERRED**, not read from client source — but it is the
only interpretation consistent with `TagNetworkSerialization.serializeTagsToNetwork`
including the static layer at all (§5.1) and with the mechanism above fully
reproducing every observed error, per-registry, exactly as reported (§4.3).

### 4.3 Per-registry breakdown of the three failing registries

All three failures are `RegistryCodecs.homogeneousList(registryKey)`
(`HolderSet<T>`) fields whose real vanilla data — confirmed by reading the
data-generator *bootstrap* classes that construct these registries'
entries — is populated via a **tag reference**, where the referenced tag lives
in a registry that is a **frozen context registry on the client** (never one
of the 29 synchronized ones):

- **`minecraft:dimension_type`** — `DimensionType`'s record has a required
  (non-optional) field `infiniburn: HolderSet<Block>`
  (`RegistryCodecs.homogeneousList(Registries.BLOCK).fieldOf("infiniburn")`).
  `DimensionTypes.bootstrap` (the data-generator source for the four built-in
  dimension types) constructs every one of the four entries via
  `blocks.getOrThrow(BlockTags.INFINIBURN_OVERWORLD)` /
  `..._NETHER` / `..._END` — i.e. `infiniburn` round-trips through the tag
  reference `#minecraft:infiniburn_overworld` / `#minecraft:infiniburn_nether`
  / `#minecraft:infiniburn_end` in the on-disk/bundled JSON. `Registries.BLOCK`
  is a static registry — frozen client-side before this load, with (absent an
  Update Tags packet) zero tags bound. **All four entries reference one of
  these three tags → all four fail.** (The same four entries also carry an
  *optional* `timelines: HolderSet<Timeline>` field referencing
  `TimelineTags.IN_OVERWORLD`/`IN_NETHER`/`IN_END` — `Registries.TIMELINE` is
  one of the 29 synchronized/registration-phase registries, so *this specific
  field* decodes fine per §4.2, but leaves an unbound placeholder behind in
  the `timeline` registry — this is the direct, confirmed cause of the
  separate `"Unbound tags in registry timeline"` freeze failure, entirely
  independent of why `dimension_type` itself fails to parse.)
- **`minecraft:enchantment`** — `Enchantment.EnchantmentDefinition`'s
  `supported_items` field is required
  (`RegistryCodecs.homogeneousList(Registries.ITEM).fieldOf("supported_items")`,
  `primary_items` is the same but optional). Every sampled entry in
  `Enchantments.java` (the bootstrap source; ~30+ call sites checked) supplies
  it via `items.getOrThrow(ItemTags.<X>_ENCHANTABLE)` — e.g.
  `ItemTags.ARMOR_ENCHANTABLE`, `MELEE_WEAPON_ENCHANTABLE`, `BOW_ENCHANTABLE`,
  etc. — a required-field, universally-tag-referenced pattern. `Registries.ITEM`
  is static/frozen client-side. **Every one of the 43 entries fails on this
  field alone** (`"Missing tag: 'enchantable/...' in 'item'"`, inferred error
  text — the outer wrapper message is the only part confirmed by the crash
  log). Separately, `Enchantment`'s own `exclusive_set` field
  (`RegistryCodecs.homogeneousList(Registries.ENCHANTMENT).optionalFieldOf("exclusive_set", ...)`)
  is a **self-referential** tag into `minecraft:enchantment` itself — one of
  the 29 registration-phase registries — so per §4.2 this specific field
  decodes fine, but (for the subset of entries that actually set it, e.g. via
  `.exclusiveWith(enchantments.getOrThrow(EnchantmentTags.ARMOR_EXCLUSIVE))`)
  leaves unbound placeholders in `enchantment`'s own tag table — the direct
  cause of `"Unbound tags in registry enchantment: [exclusive_set/armor, boots,
  bow, crossbow, damage, mining, riptide]"`, again independent of why
  individual enchantment entries fail to parse.
- **`minecraft:sulfur_cube_archetype`** — `SulfurCubeArchetype`'s `items`
  field is required
  (`RegistryCodecs.homogeneousList(Registries.ITEM).fieldOf("items")`).
  `SulfurCubeArchetypes.bootstrap` supplies it per-archetype via a dedicated
  `ItemTags.SULFUR_CUBE_ARCHETYPE_<NAME>` constant (confirmed for `REGULAR`,
  `BOUNCY` directly; the remaining 10 archetype registrations in the same file
  follow the identical `register(context, KEY, ItemTags.SULFUR_CUBE_ARCHETYPE_<NAME>,
  ...)` call shape). Same static-`ITEM`-tag mechanism as enchantment.
  **All 12 entries fail.** No self-referential tag exists on this registry, so
  (correctly) no `"Unbound tags in registry sulfur_cube_archetype"` freeze
  error is reported — only per-entry parse failures.
- **`minecraft:dialog`** (freeze-only failure, no per-entry parse errors) —
  the `custom_options` and `quick_actions` built-in dialog entries
  (`Dialogs.bootstrap`) are `DialogListDialog`s whose "which dialogs to list"
  field is populated via `dialogs.getOrThrow(DialogTags.PAUSE_SCREEN_ADDITIONS)`
  / `.QUICK_ACTIONS` respectively — a **self-referential** tag into
  `minecraft:dialog` itself, one of the 29 registration-phase registries. Per
  §4.2 this decodes without error (placeholder created), but the placeholder
  is never bound → `"Unbound tags in registry dialog: [pause_screen_additions,
  quick_actions]"`. `DialogTagsProvider` (the data-generator tag-content
  source) is confirmed to register both tag keys, but this pass could not
  confirm their vanilla *membership* (which dialog ids each tag actually
  lists) from the provider file alone — see §5.2/§7.

### 4.4 Registry ordering — confirmed non-issue

`RegistryDataLoader`'s network `load(...)` builds **one** `RegistryOps.RegistryInfoLookup`
from **all** `registriesToLoad` tasks up front (`createContext`), *then*
kicks off every task's `load()` roughly concurrently (`for i in 0..taskCount:
loadCompletions[i] = loadTasks[i].load(...)`), and only proceeds to freeze
anything after `CompletableFuture.allOf(loadCompletions)` resolves. This means
cross-registry `Holder`/`HolderSet` references between any two of the 29
synchronized registries resolve correctly **regardless of which order their
`RegistryData` packets arrived in**, because every registry's registration-lookup
getter already exists in the shared context map before any individual
registry's elements are decoded. The client crash stack trace independently
corroborates this at the transport level: `RegistryDataCollector` only calls
into this load path once, from `handleConfigurationFinished` — i.e. it
buffers every received `RegistryData`/tag packet through the whole
Configuration phase and only triggers the batch `load()` when
`ClientboundFinishConfigurationPacket` arrives (**INFERRED** from the stack
trace's method chain: `handleConfigurationFinished` →
`collectGameRegistries` → `loadNewElementsAndTags`).

**Practical conclusion: none of the 29 `RegistryData` packets, nor the
`ClientboundUpdateTagsPacket`, need any particular relative order.** They only
need to **all** arrive before `FinishConfiguration` is sent. Environment
attribute (`environment_attribute`) does **not** need to be added to
`SYNCHRONIZED_REGISTRIES` or sent in any particular position: `EnvironmentAttribute`
is registered via `BuiltInRegistries.ENVIRONMENT_ATTRIBUTE` (confirmed —
`EnvironmentAttribute.toString()` reads `BuiltInRegistries.ENVIRONMENT_ATTRIBUTE`
directly), and `EnvironmentAttributes.CODEC = BuiltInRegistries.ENVIRONMENT_ATTRIBUTE.byNameCodec()`
(confirmed by direct grep) — a plain compile-time-registered, by-name codec
wholly independent of `RegistryOps`/`HolderLookup`/network sync. It is never a
`RegistryDataLoader.RegistryData` entry in either `WORLDGEN_REGISTRIES` or
`SYNCHRONIZED_REGISTRIES`, confirmed by its total absence from both lists.
`DimensionType.attributes: EnvironmentAttributeMap` therefore carries **no**
network-registry-ordering dependency at all — it was a reasonable suspect
given its 26.2-era novelty, but is definitively **not** implicated in this
bug.

## 5. The tag transmission contract

### 5.1 Packet identity and scope

`net.minecraft.network.protocol.common.ClientboundUpdateTagsPacket` — declared
once, used unmodified in both the Configuration and Play packet tables
(`ConfigurationProtocols.CLIENTBOUND_TEMPLATE` and, by strong implication from
the shared-class placement, the Play equivalent). In
`ConfigurationProtocols.CLIENTBOUND_TEMPLATE`'s own declared `addPacket(...)`
order, counting 0-based from the first entry (`CookieRequest`), Update Tags is
the **14th** clientbound Configuration packet — sequential-index packet id
**`0x0D`**. This is independently corroborated by our own existing packet ids
in `crates/protocol/src/configuration.rs`, which already match this same
sequential-declaration-order scheme exactly at three other points:
`RegistryData` = `0x07` (8th declared clientbound packet — confirmed), 
`UpdateEnabledFeatures` = `0x0C` (13th — confirmed), `KnownPacksClientbound`
(`SelectKnownPacks`) = `0x0E` (15th — confirmed). **Confidence: HIGH** that
`0x0D` is the correct Configuration-state clientbound id for Update Tags,
derived by the same counting method already validated against our three
existing, working packet ids.

Server-side construction: `TagNetworkSerialization.serializeTagsToNetwork(registries)`
iterates `RegistrySynchronization.networkSafeRegistries(registries)`, which is
literally defined as:

```
networked_registries = the 29 SYNCHRONIZED_REGISTRIES, sourced from the WORLDGEN layer
static_registries    = every registry in the STATIC layer (item, block,
                        entity_type, fluid, game_event, potion, attribute,
                        sound_event, ... every BuiltInRegistries-style table)
network_safe_registries = networked_registries ++ static_registries
```

then, per registry, `serializeToNetwork` walks `registry.getTags()` and, per
tag, resolves every held element to its **numeric registry-internal id**
(`registry.getId(holder.value())`) — building `Map<Identifier tag_id,
IntList member_ids>`. **This is definitive, directly-read proof** (not
inference) that vanilla's Configuration-phase tag packet is not scoped to only
the 29 dynamic registries — it always includes every static registry's tags
too, in the same packet, and a real client expects to receive and bind them
there.

### 5.2 Exact wire structure

```
ClientboundUpdateTagsPacket {
    tags: Map<ResourceKey<Registry>, NetworkPayload>   // VarInt count, then entries
}
// per map entry: ResourceKey written as its Identifier (registry id, e.g. "minecraft:dimension_type")

NetworkPayload {
    tags: Map<Identifier, IntList>   // VarInt count, then entries
}
// per map entry: tag Identifier (e.g. "minecraft:in_overworld"),
//                then an IdList: VarInt count, then VarInt per numeric member id

// A registry with zero tags to send is simply omitted from the outer map
// entirely (`serializeTagsToNetwork` filters out any registry whose payload
// `.isEmpty()`) — it is NOT sent as an empty inner map.
```

**Critical, easy-to-miss wire detail:** tag membership is a list of **numeric
ids**, not identifiers. For one of the 29 synchronized registries, that
numeric id is the entry's position/index as established by *this same
connection's* own `RegistryData` packet for that registry (0-based, in the
order the entries were listed). For a static registry (item, block), the
numeric id is the client's own fixed, jar-baked ordering for that registry —
which our from-scratch server must reproduce exactly if it is to compute
correct member-id lists (our own `crates/registries/generated` already
contains a generated `block_states.rs`, which is a plausible existing source
for block ids; no equivalent item-id table was found in this pass — flagged in
§7).

### 5.3 Which tags must minimally be bound to pass this specific crash

Directly confirmed by name from §4.3's bootstrap-source reading (**HIGH
confidence on tag *names*** — every name below is read from a `TagKey`
constant or its direct call site, not guessed):

| Registry | Tag(s) that must be bound | Why (from §4.3) |
|---|---|---|
| `minecraft:block` (static) | `infiniburn_overworld`, `infiniburn_nether`, `infiniburn_end` | `dimension_type.infiniburn` |
| `minecraft:item` (static) | the full `enchantable/*` family: `armor`, `foot_armor`, `leg_armor`, `chest_armor`, `head_armor`, `melee_weapon`, `sweeping`, `fire_aspect`, `sharp_weapon`, `weapon`, `mining`, `mining_loot`, `fishing`, `trident`, `lunge`, `durability`, `bow`, `equippable`, `crossbow`, `vanishing`, `mace` (20 tags, confirmed by direct listing in `ItemTags.java`) | `enchantment.supported_items`/`primary_items` |
| `minecraft:item` (static) | `sulfur_cube_archetype_regular`, `_bouncy`, and presumably 10 more, one per archetype name (`slow_bouncy`, `slow_flat`, `fast_flat`, `light`, `fast_sliding`, `slow_sliding`, `high_resistance`, `sticky`, `explosive`, `hot` — **names inferred from the registry's own 12 entry ids, not individually confirmed by reading each bootstrap call site**) | `sulfur_cube_archetype.items` |
| `minecraft:timeline` (dynamic) | `in_overworld`, `in_nether`, `in_end` | `dimension_type.timelines` (self-consistent — cross-referenced from another registry) |
| `minecraft:enchantment` (dynamic) | `exclusive_set/armor`, `/boots`, `/bow`, `/crossbow`, `/damage`, `/mining`, `/riptide` | `enchantment.exclusive_set` (self-referential) |
| `minecraft:dialog` (dynamic) | `pause_screen_additions`, `quick_actions` | `dialog`'s own `DialogListDialog` entries (self-referential) |

**Exact tag *membership* (which specific items/blocks/entries each tag
contains) was not verified in this pass** for any of the above — only which
tag *names* must exist and be bound. See §5.4 and §7 for why this distinction
matters and what it means for the fix.

### 5.4 "Bound empty" is enough to fix *this* crash — but not enough for gameplay parity

`MappedRegistry.bindTags(pendingTags)` — `pendingTags.forEach((id, values) ->
getOrCreateTagForRegistration(id).bind(values))` — and `HolderSet.Named.isBound()`
— `contents != null` — mean the freeze-time check (`"Unbound tags in registry
..."`) and the decode-time `"Missing tag"` check are **both satisfied by an
empty member list**, as long as the tag id itself is present as a key in the
`NetworkPayload` sent for that registry. Concretely: sending
`minecraft:block → {"infiniburn_overworld": []}` (an empty `IntList`) is
enough to make `dimension_type`'s `infiniburn` field decode successfully —
the resulting `HolderSet<Block>` is simply empty.

This is the pragmatic core of the "minimal-correct recipe" in §6: **the crash
this document investigates can be fixed by sending every tag name in §5.3's
table, even with zero members each.** But an empty `infiniburn_overworld` tag
is a **silent gameplay-parity bug** the moment a player lights a fire near
netherrack/magma in the overworld (vanilla's real tag makes certain overworld
blocks burn forever; ours would make none). The same applies to every
`enchantable/*` tag (an empty tag means *no item is enchantable via table/anvil
predicate checks that consult it*) and to `sulfur_cube_archetype_*` tags
(affects which items a sulfur-cube variant can pick up/interact with, per
`SulfurCubeArchetype.items`' actual gameplay use — not traced in this
document). **This document's scope is the Configuration-phase crash only** —
real, vanilla-correct tag membership for every tag in §5.3 is a separate,
larger data-acquisition task (most naturally sourced via the project's own
data-generator pipeline per ASSET-D18(f), the same mechanism
`docs/research/mc-26.2/README.md` already documents this whole corpus was
partly built from) and is explicitly flagged as **out of this document's
scope** in §7.

## 6. Minimal-correct recipe

Ordered, implementation-ready specification for a from-scratch server to pass
`RegistryDataLoader`'s client-side checks and reach Play, expressed as
pseudocode over our own existing `run_configuration` shape:

```
// Steps 1-3 unchanged: brand, feature flags, known-pack negotiation.

// Step 4: registry-data sync — UNCHANGED, already correct.
//   Order among the 29 registries does not matter (§4.4).
//   has_data=false for every entry remains correct (§3.3).
for (registry_id, entries) in SYNCHRONIZED_REGISTRIES:
    send RegistryData { registry_id, entries: entries.map(|id| { id, data: None }) }

// Step 5 (currently SKIPPED — this is the fix): send Update Tags, id 0x0D,
// BEFORE FinishConfiguration. Must include, at minimum, every tag name in
// §5.3's table — real vanilla-correct membership where available (via a
// future data-generator pass), empty member lists as an interim/fallback
// (§5.4) wherever real membership isn't yet sourced. Registries with zero
// tags to send are simply omitted from the outer map.
send ClientboundUpdateTagsPacket {
    tags: {
        "minecraft:block":       { "infiniburn_overworld": [...ids], "infiniburn_nether": [...], "infiniburn_end": [...] },
        "minecraft:item":        { "enchantable/armor": [...], ... all 20 enchantable/* tags ..., "sulfur_cube_archetype_regular": [...], ... all 12 archetype tags ... },
        "minecraft:timeline":    { "in_overworld": [...ids into OUR OWN timeline registry entry list], "in_nether": [...], "in_end": [...] },
        "minecraft:enchantment": { "exclusive_set/armor": [...], "/boots": [...], "/bow": [...], "/crossbow": [...], "/damage": [...], "/mining": [...], "/riptide": [...] },
        "minecraft:dialog":      { "pause_screen_additions": [...], "quick_actions": [...] },
    }
}

// Step 6 unchanged: FinishConfiguration, terminal, then await AcknowledgeFinishConfiguration.
send FinishConfiguration {}
```

Per-registry notes:

- **`block`/`item` numeric ids**: must match the client's own fixed vanilla
  ordering for these static registries — not our own arbitrary choice (§5.2).
  Confidence: **the requirement is HIGH-confidence; the concrete id values
  need sourcing from our own registry-id tables** (`crates/registries/generated`),
  flagged incomplete for item ids in §7.
- **`timeline` numeric ids**: must match the index positions in *our own*
  `SYNCHRONIZED_REGISTRIES["minecraft:timeline"]` entry list (this one *is*
  entirely under our control, unlike the static registries above).
- Every other of the 29 registries may legitimately have **no** tags at all
  (e.g. `test_environment`) — omit them from the payload entirely rather than
  sending an empty inner map (matches vanilla's own `.filter(!isEmpty())`
  behavior, §5.1).
- This fix requires adding a new wire type to `rc_protocol` — no
  `UpdateTags`/`ClientboundUpdateTags`-shaped packet or `TagNetworkSerialization`-shaped
  data model exists in `crates/protocol/src` today (confirmed absent by
  search of `crates/protocol/src/configuration.rs`, the file that owns every
  other Configuration-state packet).

## 7. Cross-check against current implementation — ordered fix list

Files read: `crates/server/src/net/configuration_flow.rs`,
`crates/server/src/play/world.rs` (`SYNCHRONIZED_REGISTRIES`),
`crates/protocol/src/configuration.rs`.

1. **Missing wire packet.** `crates/protocol/src/configuration.rs` has no
   `ClientboundUpdateTags`/`UpdateTags` type, no `TagNetworkSerialization::NetworkPayload`-equivalent
   struct, and no id `0x0D` reservation. This must be added (nested `WireWrite`/`WireRead`
   for `Map<Identifier, Map<Identifier, Vec<VarInt>>>`, id `0x0D`,
   `ConnectionState::Configuration`, clientbound) before anything else in this
   fix list can be sent.
2. **Missing tag content source.** No tag-membership data exists anywhere in
   `crates/registries` today (`grep` for `tag` under `crates/registries/src`
   returned nothing). At minimum, the six tag *names* enumerated in §5.3 need
   concrete `(tag_id, [member_ids])` tables — even with empty member lists as
   an interim step per §5.4 — sourced and added to
   `crates/registries/generated` (or a new sibling module) alongside the
   existing `registries.rs`/`block_states.rs`.
3. **`configuration_flow.rs`'s `run_configuration`**: replace the comment "Step
   5 (Update Tags) is deliberately not sent (Constraints (e))" and its
   corresponding no-op with an actual send of the new Update Tags packet,
   positioned anywhere after Step 3 (known-pack gate) and before Step 6
   (`FinishConfiguration`) — order relative to the 29 `RegistryData` sends
   does not matter (§4.4), but it must precede `FinishConfiguration`.
4. **No other change is required** to fix this specific crash: `SYNCHRONIZED_REGISTRIES`'s
   registry set (29/29, matching vanilla exactly), entry lists, `has_data=false`
   policy, and packet order are all already correct per §3.2/§3.3/§4.4. The
   registry declaration order difference between our table and vanilla's own
   (§3.2) is cosmetic and does not need to change.

### Open questions / not independently verified in this pass

- Exact numeric-id source for `minecraft:item` (needed for §6's Update Tags
  payload) — `crates/registries/generated` has `block_states.rs` but no
  equivalent item-id table was found; needs confirming whether one exists
  elsewhere in the workspace or needs generating.
- Full vanilla membership (not just names) of all six tags in §5.3, and of the
  10 not-individually-confirmed `sulfur_cube_archetype_*` item tag names
  (§5.3's table) — needs a dedicated data-generator pass per ASSET-D18(f), out
  of scope here.
- Whether `net.minecraft.server.network.config.SynchronizeRegistriesTask`
  (server-side Configuration task orchestrator) does anything beyond calling
  `packRegistries`/`serializeTagsToNetwork` and enqueuing both packets — not
  opened directly in this pass; assumed straightforward from the two packer
  classes' own public API shape, but not confirmed.
- Whether `RegistryDataCollector`/`ClientConfigurationPacketListenerImpl`
  genuinely batch *all* Configuration-phase data before the first `load()`
  call, or whether some partial/incremental resolution also happens earlier —
  inferred solely from the crash reports' stack traces (§4.4), since no
  client-side source exists in this decompile to confirm directly.
- Whether a real vanilla server ever sends `ClientboundUpdateTagsPacket` more
  than once during one Configuration phase (e.g. split across multiple
  packets) — assumed single packet, matching `TagNetworkSerialization.serializeTagsToNetwork`'s
  single-map return shape, but not independently confirmed against a packet
  capture.
