# Items, Data Components, Recipes & Loot — Vanilla 26.2 Research

## 1. Purpose

This subsystem defines what an item *is* (`Item` behaviour class + a per-stack `DataComponentMap`), how individual `ItemStack`s carry mutable per-instance state as a sparse patch over item-type defaults, and the three data-driven economies built on top of items: **crafting** (recipes, matched inside container menus), **loot** (weighted random item generation from world events), and **trading** (villager/wandering-trader offers). It also owns the generic container/menu framework (`AbstractContainerMenu`) that every block/entity inventory screen (chest, furnace, anvil, enchanting table, brewing stand, villager trade UI, …) is built from, including the click-handling state machine and the server↔client slot synchronization protocol.

## 2. Where it lives

| Package | Responsibility | Representative classes | File count |
|---|---|---|---|
| `net.minecraft.world.item` | `Item` behaviour class, `ItemStack` value type, concrete item behaviours (100+ `*Item` subclasses), `Items` registry, rarity/tooltip/use-animation enums | `Item`, `ItemStack`, `ItemStackTemplate`, `Items`, `Rarity`, `ItemUseAnimation`, `BundleItem`, `BowItem`, `CrossbowItem` | 100 |
| `net.minecraft.world.item.component` | Concrete data-component payload types (non-enum ones) | `Tool`, `Consumable`, `Equippable`, `BundleContents`, `ItemContainerContents`, `CustomData`, `ItemAttributeModifiers`, `TooltipDisplay` | 46 |
| `net.minecraft.world.item.consume_effects` | Effects fired when a `Consumable` finishes consuming | `ApplyStatusEffectsConsumeEffect`, `TeleportRandomlyConsumeEffect` | 7 |
| `net.minecraft.world.item.context` | Use-context value objects passed into `Item.useOn` | `UseOnContext`, `BlockPlaceContext` | 4 |
| `net.minecraft.world.item.crafting` | Recipe types, `RecipeManager`, ingredient matching, crafting-input model | `RecipeManager`, `Ingredient`, `ShapedRecipe`, `ShapelessRecipe`, `CraftingInput`, `PlacementInfo`, `RecipeMap`, `RecipeCache` | 52 |
| `net.minecraft.world.item.crafting.display` | Client-facing recipe *display* model (decoupled from the actual matching logic since 1.21.2) | `RecipeDisplay`, `SlotDisplay`, `ShapedCraftingRecipeDisplay` | 14 |
| `net.minecraft.world.item.enchantment` | `Enchantment` data type, effect-component registry, per-item enchantment map | `Enchantment`, `EnchantmentEffectComponents`, `EnchantmentHelper`, `ItemEnchantments`, `EnchantmentInstance` | 14 |
| `net.minecraft.world.item.enchantment.effects` | Concrete enchantment-effect payload types (the "verbs" an enchantment can invoke) | `AddValue`, `DamageEntity`, `ExplodeEffect`, `ApplyMobEffect`, `SummonEntityEffect` | 26 |
| `net.minecraft.world.item.enchantment.providers` | Data-driven enchantment *application* (used by loot functions / mob equipment gen) | `EnchantmentProvider`, `EnchantmentsByCost`, `SingleEnchantment` | 7 |
| `net.minecraft.world.item.equipment` (+ `.trim`) | Armor material/type/asset model, trim system | `ArmorMaterial`, `ArmorType`, `Equippable`, `TrimMaterial`, `TrimPattern` | 15 |
| `net.minecraft.world.item.slot` | Composable "slot source" views over containers (used by recipe display / crafter targeting) | `SlotSource`, `SlotCollection`, `RangeSlotSource` | 12 |
| `net.minecraft.world.item.trading` | Data-driven villager trades | `TradeSet`, `VillagerTrade`, `MerchantOffer`, `MerchantOffers`, `ItemCost`, `TradeCost`, `Merchant` | 11 |
| `net.minecraft.world.item.alchemy` | Potions and the (hardcoded, non-data-driven) brewing recipe table | `Potion`, `Potions`, `PotionContents`, `PotionBrewing` | 6 |
| `net.minecraft.core.component` | The data-component engine itself: type registry, map, patch, lookup | `DataComponentType`, `DataComponents`, `DataComponentMap`, `DataComponentPatch`, `PatchedDataComponentMap`, `TypedDataComponent` | 12 |
| `net.minecraft.core.component.predicates` | Per-component *predicate* codecs (used by advancements, loot conditions, `MatchTool`) | `DataComponentPredicate`, `DataComponentPredicates`, `EnchantmentsPredicate` | 18 |
| `net.minecraft.world.level.storage.loot` | Loot table root, pool, context, validation | `LootTable`, `LootPool`, `LootContext`, `LootParams`, `ValidationContext` | 15 |
| `net.minecraft.world.level.storage.loot.entries` | Loot pool entry types (what an entry *contributes*) | `LootItem`, `TagEntry`, `AlternativesEntry`, `SequentialEntry`, `NestedLootTable`, `EntryGroup`, `SlotLoot` | 16 |
| `net.minecraft.world.level.storage.loot.functions` | Item-stack post-processing functions | `SetItemCountFunction`, `EnchantRandomlyFunction`, `SetComponentsFunction`, `CopyComponentsFunction` | 49 |
| `net.minecraft.world.level.storage.loot.predicates` | Loot conditions (gate whether an entry/pool/function applies) | `LootItemRandomChanceCondition`, `MatchTool`, `LocationCheck`, `EntityHasScoreCondition` | 25 |
| `net.minecraft.world.level.storage.loot.providers.number` | Pluggable numeric value sources used throughout loot/enchant/trade JSON | `ConstantValue`, `UniformGenerator`, `BinomialDistributionGenerator`, `EnchantmentLevelProvider` | 11 |
| `net.minecraft.world.level.storage.loot.providers.nbt` / `.score` | NBT-value and scoreboard-value providers for loot functions/conditions | `ContextNbtProvider`, `StorageNbtProvider`, `ScoreboardNameProviders` | 10 |
| `net.minecraft.world.level.storage.loot.parameters` | The fixed set of loot-context parameter *keys* and the named *parameter sets* that group them | `LootContextParams`, `LootContextParamSets` | 3 |
| `net.minecraft.world.inventory` | Generic menu/container framework: `AbstractContainerMenu`, every concrete menu, slot model, sync protocol | `AbstractContainerMenu`, `Slot`, `ContainerInput`, `ClickAction`, `RemoteSlot`, `AnvilMenu`, `MerchantMenu`, `CrafterMenu` | 61 |
| `net.minecraft.world.inventory.tooltip` | Menu-side tooltip payloads that need client rendering (e.g. bundle contents preview) | `TooltipComponent`, `BundleTooltip` | 3 |
| `net.minecraft.world.entity.npc.villager` | Villager entity, profession→trade-set wiring, trade generation trigger | `Villager`, `AbstractVillager`, `VillagerProfession`, `VillagerData` | 7 |
| `net.minecraft.stats` | Server + client recipe-book bookkeeping (known/highlighted recipes, settings) | `RecipeBook`, `ServerRecipeBook`, `RecipeBookSettings` | 10 |

## 3. How it works

### 3.1 Item vs. ItemStack vs. data components

`Item` (`net.minecraft.world.item.Item`) is a **stateless singleton behaviour object**, one instance per registered item type (e.g. one `Item` instance for `minecraft:diamond_sword`), created at bootstrap and never mutated. It exposes overridable hooks (`useOn`, `use`, `finishUsingItem`, `hurtEnemy`, `mineBlock`, `interactLivingEntity`, `inventoryTick`, `getUseAnimation`, `getUseDuration`, `appendHoverText`, `overrideStackedOnOther`/`overrideOtherStackedOnMe`, …) that concrete subclasses (`SwordItem`-equivalents dissolved into data + `Item` base, `BowItem`, `BundleItem`, `SpawnEggItem`, etc.) override for behaviour that cannot be expressed declaratively. Each `Item` owns a **default `DataComponentMap`** (`Item.components()`, sourced from its `Holder.Reference`), built at construction time from an `Item.Properties` builder — this is the item's "prototype" component set (durability, stack size, tool rules, attribute modifiers, food, equippable slot, …).

`ItemStack` (`net.minecraft.world.item.ItemStack`) is the **per-instance value**: an `Item` holder reference, an integer `count` (clamped 1–99, `ABSOLUTE_MAX_STACK_SIZE = 99`; `DEFAULT_MAX_STACK_SIZE = 64`), and a `PatchedDataComponentMap`. `ItemStack.EMPTY` is a sentinel singleton (`item == null`); `isEmpty()` is true for the sentinel, for stacks whose item resolves to `Items.AIR`, or for `count <= 0`. `ItemStack` deliberately does **not** implement structural `equals`/`hashCode` in the usual sense — equality is via explicit static helpers (`matches`, `isSameItem`, `isSameItemSameComponents`, `matchesIgnoringComponents`) because two different `ItemStack` Java objects with identical item+count+components must compare equal for stacking/matching purposes without being reference-equal.

### 3.2 The data-component system

**`DataComponentType<T>`** (`net.minecraft.core.component`) is an opaque, registry-backed marker for a strongly-typed value slot. Each type optionally carries a **persistent `Codec<T>`** (NBT/JSON (de)serialization; absent ⇒ "transient", disk-invisible, e.g. `CREATIVE_SLOT_LOCK`) and a **`StreamCodec`** for network sync (absent types fall back to wrapping the persistent codec). Builders can mark a type `.cacheEncoding()` (results are memoized in a shared `EncoderCache` of capacity 512, since e.g. `CUSTOM_NAME`/`LORE` re-encode identically very often) and `.ignoreSwapAnimation()` (mutating `DAMAGE` alone must not retrigger the held-item swap animation client-side).

**`DataComponents`** (`net.minecraft.core.component.DataComponents`) is the closed registry of all vanilla component types — **90 types** registered via a private `register(id, builder)` helper, one static field per component. Enumerated in full:

| Component | Payload type | Purpose |
|---|---|---|
| `custom_data` | `CustomData` (NBT) | Freeform NBT bag (mod/plugin data, legacy `Tag` fallback) |
| `max_stack_size` | `Integer` (1–99) | Stack cap |
| `max_damage` | `Integer` | Durability ceiling |
| `damage` | `Integer` | Current damage (0 = pristine); `.ignoreSwapAnimation()` |
| `unbreakable` | `Unit` | Disables durability loss / durability bar |
| `use_effects` | `UseEffects` | Interact-vibration + hurt-on-use flags for spears etc. |
| `custom_name` | `Component` | Player-set rename (anvil) |
| `minimum_attack_charge` | `Float` (0–1) | Spear/weapon charge gate |
| `damage_type` | `Holder<DamageType>` | Weapon's damage type override (e.g. spear) |
| `item_name` | `Component` | Base (non-custom) display name |
| `item_model` | `Identifier` | Client model id override |
| `lore` | `ItemLore` | Tooltip lore lines |
| `rarity` | `Rarity` | COMMON/UNCOMMON/RARE/EPIC — colors name, bumped by enchant |
| `enchantments` | `ItemEnchantments` | Applied enchantments (map Holder→level) |
| `can_place_on` | `AdventureModePredicate` | Adventure-mode placement whitelist |
| `can_break` | `AdventureModePredicate` | Adventure-mode mining whitelist |
| `attribute_modifiers` | `ItemAttributeModifiers` | Equip-slot-scoped attribute modifiers |
| `custom_model_data` | `CustomModelData` | Resource-pack model variant selector |
| `tooltip_display` | `TooltipDisplay` | Per-component tooltip visibility toggles |
| `repair_cost` | `Integer` | Anvil "prior work" XP surcharge |
| `creative_slot_lock` | `Unit` | Transient; marks creative-inventory locked slots |
| `enchantment_glint_override` | `Boolean` | Force foil on/off regardless of enchant state |
| `intangible_projectile` | `Unit` | Marks projectile-only pseudo items |
| `food` | `FoodProperties` | Nutrition/saturation/always-edible |
| `consumable` | `Consumable` | Eating/drinking duration, animation, sound, effects |
| `use_remainder` | `UseRemainder` | Item stack left behind after use (e.g. bottle) |
| `use_cooldown` | `UseCooldown` | Cooldown group + seconds after use |
| `damage_resistant` | `DamageResistant` | Damage-type tag the item survives (e.g. fire-resistant) |
| `tool` | `Tool` | Per-block-tag mining speed rules, damage-per-block, drop rules |
| `weapon` | `Weapon` | Melee item-damage-per-attack |
| `attack_range` | `AttackRange` | Spear reach/charge distances |
| `enchantable` | `Enchantable` | Enchantability value |
| `equippable` | `Equippable` | Armor/equip slot, sounds, allowed entities, swap rules |
| `repairable` | `Repairable` | Valid repair-material item/tag |
| `glider` | `Unit` | Elytra-style gliding flag |
| `tooltip_style` | `Identifier` | Alternate tooltip frame texture |
| `death_protection` | `DeathProtection` | Totem-of-undying effect list |
| `blocks_attacks` | `BlocksAttacks` | Shield-style blocking parameters |
| `piercing_weapon` | `PiercingWeapon` | Spear piercing-hit sounds/behaviour |
| `kinetic_weapon` | `KineticWeapon` | Spear charge/dismount/knockback timing |
| `swing_animation` | `SwingAnimation` | Attack swing animation kind + duration |
| `additional_trade_cost` | `Integer` | Transient; villager-offer surcharge accumulator |
| `stored_enchantments` | `ItemEnchantments` | Enchanted-book payload (not "active") |
| `dye` | `DyeColor` | Bundle-selector / firework dye base |
| `dyed_color` | `DyedItemColor` | Leather-armor dye tint |
| `map_color` | `MapItemColor` | Explorer-map tint |
| `map_id` | `MapId` | Which `MapItemSavedData` this map references |
| `map_decorations` | `MapDecorations` | Map icon overlays |
| `map_post_processing` | `MapPostProcessing` | Lock/scale actions applied via cartography table |
| `charged_projectiles` | `ChargedProjectiles` | Crossbow-loaded projectile stack(s) |
| `bundle_contents` | `BundleContents` | Bundle's nested item list + fractional weight |
| `potion_contents` | `PotionContents` | Potion id + custom effects + color |
| `potion_duration_scale` | `Float` | Lingering-potion effect duration multiplier |
| `suspicious_stew_effects` | `SuspiciousStewEffects` | Stew's granted mob effects |
| `writable_book_content` | `WritableBookContent` | Unsigned book pages |
| `written_book_content` | `WrittenBookContent` | Signed book pages/title/author/generation |
| `trim` | `ArmorTrim` | Armor-trim pattern+material |
| `debug_stick_state` | `DebugStickState` | Per-block cycling state for the debug stick |
| `entity_data` | `TypedEntityData<EntityType<?>>` | Spawn-egg's stored entity NBT |
| `bucket_entity_data` | `CustomData` | Captured mob-bucket entity NBT |
| `block_entity_data` | `TypedEntityData<BlockEntityType<?>>` | BlockItem's stored block-entity NBT |
| `instrument` | `InstrumentComponent` | Goat-horn instrument selection |
| `provides_trim_material` | `Holder<TrimMaterial>` | Ingot-style "usable as trim material" marker |
| `ominous_bottle_amplifier` | `OminousBottleAmplifier` | Bad-omen bottle amplifier level |
| `jukebox_playable` | `JukeboxPlayable` | Disc→song binding |
| `provides_banner_patterns` | `HolderSet<BannerPattern>` | Pattern-item's unlocked patterns |
| `recipes` | `List<ResourceKey<Recipe<?>>>` | Knowledge-book unlock payload |
| `lodestone_tracker` | `LodestoneTracker` | Compass lodestone target + tracked flag |
| `firework_explosion` | `FireworkExplosion` | Single-star explosion shape/color/effects |
| `fireworks` | `Fireworks` | Rocket's flight duration + star list |
| `profile` | `ResolvableProfile` | Player-head GameProfile (name/UUID/textures) |
| `note_block_sound` | `Identifier` | Player-head custom note-block sound |
| `banner_patterns` | `BannerPatternLayers` | Banner's applied pattern layers |
| `base_color` | `DyeColor` | Shield/banner base color |
| `pot_decorations` | `PotDecorations` | Decorated-pot's 4 sherd sides |
| `container` | `ItemContainerContents` | Shulker-box-item's nested inventory snapshot |
| `block_state` | `BlockItemStateProperties` | Partial blockstate to apply on placement |
| `bees` | `Bees` | Bee-nest/hive item's captured occupants |
| `sulfur_cube_content` | `SulfurCubeContent` | (26.2-era) sulfur cube payload |
| `lock` | `LockCode` | Container-item's lock key string |
| `container_loot` | `SeededContainerLoot` | Deferred loot table + seed for a placed container |
| `break_sound` | `Holder<SoundEvent>` | Item-entity break sound override |
| `villager/variant`, `wolf/variant`, `wolf/sound_variant`, `wolf/collar`, `fox/variant`, `salmon/size`, `parrot/variant`, `tropical_fish/pattern`, `tropical_fish/base_color`, `tropical_fish/pattern_color`, `mooshroom/variant`, `rabbit/variant`, `pig/variant`, `pig/sound_variant`, `cow/variant`, `cow/sound_variant`, `chicken/variant`, `chicken/sound_variant`, `zombie_nautilus/variant`, `frog/variant`, `horse/variant`, `painting/variant`, `llama/variant`, `axolotl/variant`, `cat/variant`, `cat/sound_variant`, `cat/collar`, `sheep/color`, `shulker/color` | various `Holder<X>`/enum | Spawn-egg-adjacent per-mob visual/sound variant storage (28 components) used when spawning from data or copying mob appearance onto items |

`COMMON_ITEM_COMPONENTS` is a shared base `DataComponentMap` (max-stack-size 64, empty lore/enchantments, repair-cost 0, default use-effects/attribute-modifiers/rarity/break-sound/tooltip-display/swing-animation) that every `Item.Properties` starts from before per-item overrides are layered on.

**Storage model — patch over prototype.** An `ItemStack` never stores a full component map. `PatchedDataComponentMap` (`net.minecraft.core.component`) wraps an immutable **prototype** map (`Item.components()`) plus a small **patch**: a `Reference2ObjectMap<DataComponentType<?>, Optional<?>>` where `Optional.of(v)` means "override to v" and `Optional.empty()` means "remove the prototype's default". `get(type)` checks the patch first, falls back to the prototype. `set`/`remove` compare against the prototype's default and **collapse the patch entry away entirely if the new value equals the default** — so setting a value back to its prototype default shrinks the patch back to empty (`isPatchSanitized` performs the same collapsing check when constructing from a serialized `DataComponentPatch`). The map is **copy-on-write**: `copy()`/`asPatch()` mark `copyOnWrite = true` and share the underlying `Reference2ObjectMap` until the next mutation, which then clones it (`ensureMapOwnership`). This makes `ItemStack.copy()` cheap when the copy is never mutated (the overwhelmingly common case for passing stacks around).

**`DataComponentPatch`** is the serializable, standalone form of that patch (used for NBT persistence and network transfer of "just the differences"). Its `CODEC` serializes as a map keyed by a `PatchKey` string: a bare `"minecraft:custom_name"` key with a value means "set", a `"!minecraft:custom_name"` key (no value, `Codec.EMPTY`) means "remove"; transient component types are rejected by the codec (`isTransient()` ⇒ persistent-codec validation fails). Two network stream codecs exist: the ordinary `STREAM_CODEC` (writes positive-count/negative-count varints then interleaved type+value/type-only entries) and a `DELIMITED_STREAM_CODEC` used for **untrusted** input, which length-prefixes each component's payload (`registryFriendlyLengthPrefixed(Integer.MAX_VALUE)`) so a malformed value can be skipped/rejected without desyncing the rest of the buffer — used when accepting stack data typed into anvil/creative UI etc. straight from a client.

**Stack equality/stacking rules.** `isSameItemSameComponents(a, b)` — item identity **and** `Objects.equals(a.components, b.components)` (delegates to `PatchedDataComponentMap.equals`, which compares prototype + patch map) — is the canonical "can these merge" check used everywhere (slot insert, crafting-input grouping, bundle insert, `moveItemStackTo`). `isStackable()` additionally requires `maxStackSize() > 1` and (`!isDamageableItem() || !isDamaged()`) — a partially-damaged tool never stacks even if its max stack size were >1. `matchesIgnoringComponents(a, b, predicate)` walks every component key and allows mismatches only where `predicate.test(type)` is true — used for "close enough" comparisons (e.g. server verifying a client-reported stack while ignoring components the client isn't trusted to compute, like durability-derived ones).

### 3.3 Item usage flow (server-side)

`Item.use(level, player, hand)` is the generic entry point invoked from the "use item" packet handler. Vanilla's default dispatch (before any subclass override) checks, in order: (1) a `CONSUMABLE` component present ⇒ `Consumable.startConsuming` (returns `FAIL` if `canConsume` rejects, e.g. player not hungry and food isn't `canAlwaysEat`; otherwise either starts a multi-tick "use item" state via `LivingEntity.startUsingItem` or, if `consumeTicks() <= 0`, consumes instantly); else (2) a swappable `EQUIPPABLE` ⇒ swap into its slot; else (3) `BLOCKS_ATTACKS` present ⇒ start using (shield-block state); else (4) `KINETIC_WEAPON` present (spears) ⇒ start using + play charge sound; else `PASS`.

While a multi-tick use is active, `ItemStack.onUseTick` fires every tick: for `CONSUMABLE` items this drives periodic eat/drink particle+sound emission (`Consumable.shouldEmitParticlesAndSounds`: starts once `ticksElapsed > consumeTicks * 0.21875` i.e. roughly the last ~78% of the duration, then fires every 4th tick — constants `CONSUME_EFFECTS_START_FRACTION = 0.21875F`, `CONSUME_EFFECTS_INTERVAL = 4`); for `KINETIC_WEAPON` items it drives the spear's periodic damage-entities sweep instead.

On release/finish, `finishUsingItem` calls `Consumable.onConsume`: emits eat/drink particles+sound, awards `Stats.ITEM_USED`, fires the `CONSUME_ITEM` criterion trigger, invokes every `ConsumableListener` component present on the stack (`FoodProperties` itself is one — it applies nutrition/saturation via `Player.getFoodData().eat(...)`), applies each `ConsumeEffect` in `onConsumeEffects` (apply/clear status effects, teleport-randomly for chorus fruit, play-sound), emits an EAT/DRINK `GameEvent`, then shrinks the stack by 1 (skipped for creative/infinite-materials).

`applyAfterUseComponentSideEffects` runs after *any* use (instant or duration-based) and is where `USE_REMAINDER` (e.g. milk bucket → empty bucket, honey bottle → glass bottle — replaces the used stack with a template, respecting infinite-materials mode via `handleExtraItemsCreatedOnUse`) and `USE_COOLDOWN` (applies the item's cooldown group to `ItemCooldowns`) are consumed from the **pre-use snapshot** of the stack, not the post-use one — this matters because components like `USE_REMAINDER` might otherwise have already been stripped by the use itself.

**Cooldowns** (`ItemCooldowns` / `ServerItemCooldowns`) are keyed by an `Identifier` "cooldown group" (usually the item id but can be shared across items, e.g. all ender pearls), not per-`ItemStack`. `ServerItemCooldowns` pushes a `ClientboundCooldownPacket` (duration in ticks, 0 = clear) whenever a cooldown starts or ends so the client can render the cooldown swipe overlay.

**Block/entity interaction** goes through `Item.useOn(UseOnContext)` (block right-click) and `Item.interactLivingEntity` / `ItemStack.interactLivingEntity` (entity right-click — the latter first checks an `EQUIPPABLE` with `equipOnInteract()` true, e.g. saddles/carpets, letting equip short-circuit before falling into the item's own interaction). Adventure-mode restricts block interaction: `ItemStack.useOn` refuses to proceed (returns `PASS`) if the player lacks `mayBuild` and the block isn't whitelisted by the stack's `CAN_PLACE_ON` `AdventureModePredicate`; the symmetric `CAN_BREAK` predicate gates mining via `canDestroyBlock`.

### 3.4 The generic menu/container framework

`AbstractContainerMenu` is the base class every inventory screen extends (chest, furnace family, anvil family, brewing stand, enchanting table, villager trade, crafting table, player inventory, …). It owns: the flat `slots` list (each block/entity container is mapped into a contiguous slice of this list — e.g. player inventory + hotbar always occupy the *last* N slots of every menu via `addStandardInventorySlots`), a single **carried item** (`ItemStack` the player is dragging with the cursor), a set of `DataSlot`s (small server-authoritative integers like furnace burn time, anvil cost, enchant costs — synced individually), and the click state machine.

**Slot indices are stable per-menu-instance contract**, not derived at runtime: constants like `AnvilMenu.INPUT_SLOT=0`, `ADDITIONAL_SLOT=1`, `RESULT_SLOT=2`, or `StonecutterMenu`'s `INPUT_SLOT=0`/`RESULT_SLOT=1`/inventory `2..29`/hotbar `29..38`, are hardcoded per menu type and must match exactly between client and server since click packets reference slots purely by index.

**Click dispatch — `ContainerInput` (server name for the historic "ClickType")** has exactly 7 members, each id-stable for network encoding:

| `ContainerInput` | id | Semantics |
|---|---|---|
| `PICKUP` | 0 | Left/right click a slot (button 0/1 selects `ClickAction.PRIMARY`/`SECONDARY`) |
| `QUICK_MOVE` | 1 | Shift-click: repeatedly calls `quickMoveStack` until it stops moving anything or the clicked slot's item changes identity |
| `SWAP` | 2 | Number-key (0–8) or offhand-key (button 40) hotbar swap |
| `CLONE` | 3 | Middle-click in creative: clones a full stack into the cursor |
| `THROW` | 4 | Q / Ctrl+Q: drop 1 (button 0) or the whole stack (button 1) |
| `QUICK_CRAFT` | 5 | Click-drag distribution across multiple slots (painting) |
| `PICKUP_ALL` | 6 | Double-click: vacuum up every compatible stack in the menu into the cursor |

`ClickAction` is just `PRIMARY`/`SECONDARY` (left/right). The `PICKUP` branch's core logic: empty carried + non-empty slot ⇒ pick up (whole stack on primary, `ceil(count/2)` on secondary); non-empty carried + empty slot ⇒ place (whole stack on primary, 1 on secondary, via `Slot.safeInsert`); both non-empty, same item+components ⇒ merge (whole carried on primary, 1 on secondary) if the slot accepts, else pull remaining capacity from the slot into carried; both non-empty, different item ⇒ full swap only if the carried stack's count fits the slot's max size. Before any of this, `overrideStackedOnOther`/`overrideOtherStackedOnMe` give an `Item` a chance to hijack the click entirely (bundles use this to implement "click bundle with item in hand ⇒ insert", "click item with bundle in hand ⇒ extract").

**Quick-craft (drag) state machine** — three `QUICKCRAFT_TYPE_*` distribution modes (`CHARITABLE=0`: split the carried stack evenly across dragged slots, `Mth.floor(count/slots)` each; `GREEDY=1`: 1 item per dragged slot; `CLONE=2`, creative-only: full max-stack per slot without depleting the source) driven by a 3-phase header sequence (`QUICKCRAFT_HEADER_START=0` → `CONTINUE=1` (repeated per dragged slot) → `END=2`, encoded together with the type into a single `buttonNum` via `getQuickcraftMask`/`getQuickcraftType`/`getQuickcraftHeader` bit-packing: `header = mask & 3`, `type = (mask >> 2) & 3`). A single-slot drag degenerates into an ordinary `PICKUP` click (`doClick(slot, quickcraftType, PICKUP, player)`).

**Container synchronization protocol.** Every slot has a **local** last-known value (`lastSlots`, used to fire `ContainerListener.slotChanged` for menu-internal listeners) and a **remote** shadow (`RemoteSlot`, used to decide whether the server needs to push a network update). `RemoteSlot.Synchronized` does not necessarily hold the actual `ItemStack` — it can hold just a **hash** (`HashedStack`, produced by a `HashedPatchMap.HashGenerator`) received from the client's own prediction, and considers itself "matching" if either the full stack matches or the client's claimed hash matches the local stack; on a hash match it locally caches a copy so later comparisons are cheap. This hash-based shadow is the 1.21.2+ mechanism that avoids re-sending full `ItemStack` payloads for client-predicted slot changes (crafting-table shift-click results, etc.) while still letting the server assert authority by falling back to a full push (`ClientboundContainerSetSlotPacket`) whenever prediction and reality diverge. `broadcastChanges()` (called every tick a menu is open) is the single place that walks all slots + carried + data slots and decides, per-item, whether to notify local `ContainerListener`s and/or push to the `ContainerSynchronizer` (the player's connection).

`AbstractContainerMenu.incrementStateId()` maintains a 15-bit (`& 32767`) rolling state id attached to every full-state push/click acknowledgement — the client echoes it back so the server can detect and reject stale/out-of-order click packets.

### 3.5 Crafting: recipe model and matching

**Recipe type hierarchy.** `Recipe<T extends RecipeInput>` is the root interface (`matches`, `assemble`, `getSerializer`, `getType`, `placementInfo()`, `display()`, `group()`, `isSpecial()`). Seven `RecipeType`s are registered: `CRAFTING`, `SMELTING`, `BLASTING`, `SMOKING`, `CAMPFIRE_COOKING`, `STONECUTTING`, `SMITHING`. `RecipeSerializer` values (30 total, registered in `RecipeSerializers`) are the JSON/network codec pairing for each **concrete** recipe class, distinct from `RecipeType` (many serializers — e.g. `crafting_shaped`, `crafting_shapeless`, `crafting_special_*` — all share `RecipeType.CRAFTING`):

`crafting_shaped` (`ShapedRecipe`), `crafting_shapeless` (`ShapelessRecipe`), `crafting_dye` (`DyeRecipe`), `crafting_imbue` (`ImbueRecipe`), `crafting_transmute` (`TransmuteRecipe`), `crafting_decorated_pot` (`DecoratedPotRecipe`), `crafting_special_bookcloning`, `crafting_special_mapextending`, `crafting_special_firework_rocket`, `crafting_special_firework_star`, `crafting_special_firework_star_fade`, `crafting_special_bannerduplicate`, `crafting_special_shielddecoration`, `crafting_special_repairitem` (all `CustomRecipe` subtypes — hand-coded matching logic, no JSON ingredient list, `isSpecial() = true`), `smelting`, `blasting`, `smoking`, `campfire_cooking` (all `AbstractCookingRecipe` subtypes), `stonecutting` (`StonecutterRecipe`), `smithing_transform` (`SmithingTransformRecipe`), `smithing_trim` (`SmithingTrimRecipe`).

**`Ingredient`** wraps a `HolderSet<Item>` (either a direct list or a tag reference) and is a `Predicate<ItemStack>` via `input.is(values)`. It is intentionally *not* count-aware — quantity lives on the outer recipe structure. Constructing one from an empty `HolderSet` throws immediately (`Ingredients can't be empty`), and air is explicitly forbidden as a member.

**Shaped matching.** `ShapedRecipePattern` parses a `key`→symbol map and up to 3×3 `pattern` rows into a flat `List<Optional<Ingredient>>`, first `shrink()`-ing the pattern to its tightest bounding box (trims fully-blank leading/trailing rows and leading/trailing columns) — so a `" X \nXXX\n   "` shrinks to a 3×2 pattern before matching. It precomputes `symmetrical` (`Util.isSymmetrical`) once at construction. `matches(CraftingInput)` first rejects on `ingredientCount` mismatch (cheap early-out), then requires exact width/height match, then tries the un-flipped placement and — only if the pattern isn't horizontally symmetrical — the horizontally-mirrored placement too (`this.width - x - 1` index remap). `CraftingInput` itself is pre-trimmed the same way via `CraftingInput.ofPositioned` (finds the tight bounding box of non-empty slots) before being handed to any recipe's `matches`, so a shaped recipe never has to account for an offset grid — the same physical crafting grid position is remapped for every candidate recipe.

**Shapeless matching.** `ShapelessRecipe.matches` fast-paths the single-ingredient/single-slot case, otherwise defers to `CraftingInput.stackedContents().canCraft(this, null)` — a **multiset bipartite-matching** check (`StackedContents`/`StackedItemContents`) that verifies each of the input's item-count buckets can be assigned to a distinct ingredient slot, independent of grid position. Ordinary crafting-table recipe lookup (`RecipeManager.getRecipeFor`) is a **linear scan** of `RecipeMap.byType(type)` filtered by `.matches()` — there is no ingredient-indexed lookup structure; the per-menu `RecipeCache` (fixed-size **direct-mapped, most-recently-used-first** cache — capacity 10 in `CrafterMenu`, keyed by a copy of the grid's items-at-count-1 plus width/height) exists specifically to avoid re-scanning the whole recipe list on every identical-input tick.

**Cooking recipes** (`AbstractCookingRecipe`, shared by furnace/blast furnace/smoker/campfire) are single-ingredient `SingleItemRecipe`s carrying `experience` (float XP per craft) and `cookingTime` (ticks; default differs per serializer — furnace smelting defaults to 200 ticks, blasting/smoking halve it to 100, campfire defaults to 600 — passed as `defaultCookingTime` into `cookingMapCodec`).

**Stonecutting.** `StonecutterMenu` does not auto-select a recipe: placing an input item populates `RecipeManager.stonecutterRecipes()` (a precomputed `SelectableRecipe.SingleInputSet<StonecutterRecipe>`, built once in `RecipeManager.finalizeRecipeLoading` from every enabled `StonecutterRecipe`) filtered by that input, and the player must click a numbered button (`clickMenuButton`) to choose among the multiple matches before a result appears in the output slot.

**Smithing** (`SmithingMenu`) has three fixed input slots — template (0), base (1), addition (2) — each gated by a `RecipePropertySet` (`SMITHING_TEMPLATE`/`SMITHING_BASE`/`SMITHING_ADDITION`), which `RecipeManager` builds once by scanning every registered `SmithingRecipe` and unioning their respective ingredient sets (`RecipeManager.IngredientCollector`) — this is how the smithing-table UI can grey out slots for items that *no* smithing recipe accepts without re-testing every recipe on every click. `SmithingTransformRecipe` fully replaces the base item (netherite upgrade); `SmithingTrimRecipe` (`SimpleSmithingRecipe`) instead layers an `ArmorTrim` component onto a copy of the base item.

**Recipe book.** `ServerRecipeBook` (per player, persisted under NBT tag `recipeBook`) tracks two `Set<ResourceKey<Recipe<?>>>`: `known` (unlocked, ever) and `highlight` (unlocked-but-not-yet-seen, drives the client's "NEW" badge). Unlocking sends a `ClientboundRecipeBookAddPacket` per newly-known recipe (resolved to its `RecipeDisplayEntry`, since 1.21.2 recipes and their client-facing *display* entries are decoupled — one recipe can own multiple displays, and `RecipeManager.unpackRecipeInfo` flattens every enabled recipe's `display()` list into a single server-wide indexed table at reload time, referenced by `RecipeDisplayId` rather than the recipe's own resource key).

### 3.6 Enchantments as data

**Structure.** `Enchantment` is a record: `description` (translatable `Component`), `definition` (`EnchantmentDefinition` — supported/primary item `HolderSet`s, `weight` 1–1024, `maxLevel` 1–255, `minCost`/`maxCost` linear `Cost(base, perLevelAboveFirst)` formulas, `anvilCost`, applicable `EquipmentSlotGroup`s), `exclusiveSet` (a `HolderSet<Enchantment>` of mutually-incompatible enchants), and `effects` — a **`DataComponentMap`** keyed by a *separate* registry, `EnchantmentEffectComponents` (own `DataComponentType<?>` universe, distinct from item `DataComponents`). This means an enchantment's behaviour is itself expressed through the same generic component-map mechanism used for items, just against a different type registry.

**31 effect-component types** are registered, mostly `List<ConditionalEffect<X>>` (an effect payload plus an optional gating `LootItemCondition`, evaluated against a purpose-built `LootContext`) or `List<TargetedConditionalEffect<X>>` (adds an `enchanted`/`affected` `EnchantmentTarget` pair — attacker/victim/damaging-entity routing): `damage_protection`, `damage_immunity`, `damage`, `smash_damage_per_fallen_block`, `knockback`, `armor_effectiveness`, `post_attack`, `post_piercing_attack`, `hit_block`, `item_damage`, `equipment_drops`, `location_changed`, `tick`, `ammo_use`, `projectile_piercing`, `projectile_spawned`, `projectile_spread`, `projectile_count`, `trident_return_acceleration`, `fishing_time_reduction`, `fishing_luck_bonus`, `block_experience`, `mob_experience`, `repair_with_xp`, `attributes` (flat `List<EnchantmentAttributeEffect>`, no conditions), `crossbow_charge_time`, `crossbow_charging_sounds`, `trident_sound`, `prevent_equipment_drop` (`Unit`), `prevent_armor_change` (`Unit`), `trident_spin_attack_strength`. Each list is validated at load time against the specific `ContextKeySet` (loot-context param set) its evaluation site guarantees will be populated (e.g. `damage`-family effects validate against `ENCHANTED_DAMAGE`, `tick` against `ENCHANTED_ENTITY`) — this is a static cross-check that a data pack cannot author an effect condition that references a parameter never available at the point it fires.

**Effect payload kinds** (26 classes under `.effects`): value transforms (`AddValue`, `MultiplyValue`, `SetValue`, `ScaleExponentially`, `RemoveBinomial`) implementing `EnchantmentValueEffect` — these are the "mini-DSL" used by `damage`, `damage_protection`, `knockback`, etc. to describe "level → numeric delta" curves declaratively; entity/world actions (`DamageEntity`, `Ignite`, `ExplodeEffect`, `ApplyMobEffect`, `ApplyEntityImpulse`, `ApplyExhaustion`, `ChangeItemDamage`, `SpawnParticlesEffect`, `PlaySoundEffect`, `SummonEntityEffect`, `ReplaceBlock`, `ReplaceDisk`, `SetBlockProperties`, `RunFunction`) implementing `EnchantmentEntityEffect`/`EnchantmentLocationBasedEffect`; `DamageImmunity` (marker-style, used by `damage_immunity`); `AllOf` (composes multiple entity effects into one).

**Per-item storage.** `ItemEnchantments` (component `ENCHANTMENTS` for active effects, `STORED_ENCHANTMENTS` for enchanted-book payloads that don't apply until transferred) is an `Object2IntOpenHashMap<Holder<Enchantment>>` levels 1–255, with a `Mutable` builder variant (`upgrade` merges via `Integer::max`, letting anvil combination logic simply call `upgrade` per source enchantment without special-casing "already present at equal/lower level").

**Enchanting-table algorithm** (`EnchantmentMenu` + `EnchantmentHelper`): bookshelf count is capped at 15 (`bookcases = min(bookcases, 15)`); a per-slot base roll is `random.nextInt(8) + 1 + (bookcases >> 1) + random.nextInt(bookcases + 1)`, then divided per slot index — slot 0 (top): `max(roll/3, 1)`; slot 1 (middle): `roll*2/3 + 1`; slot 2 (bottom): `max(roll, bookcases*2)` — and any slot whose resulting cost is below `slotIndex + 1` is zeroed out (so slot 2 always needs cost ≥ 3). Selecting the actual enchantment(s) for a chosen slot (`EnchantmentHelper.selectEnchantment`) perturbs the displayed cost further — `cost += 1 + rand(enchantability/4+1) + rand(enchantability/4+1)`, then `cost = clamp(round(cost * (1 + (rand+rand-1)*0.15)), 1, MAX)` — filters candidate enchantments to those whose `[minCost,maxCost]` window (at their highest eligible level) contains the perturbed cost, picks one via weighted random (`Enchantment.weight`), and then, with probability `random.nextInt(50) <= cost` (repeated, halving `cost` each iteration), keeps adding further **compatible** enchantments (`Enchantment.areCompatible` — mutual `exclusiveSet` check) from the remaining candidate pool. A plain `Book` gets exactly one enchantment (the extras are re-rolled away) and is transmuted into `ENCHANTED_BOOK` at apply time.

**Anvil algorithm** (`AnvilMenu.createResult`): only proceeds if `EnchantmentHelper.canStoreEnchantments(input)`. Two exclusive branches on the second-slot item: (a) *repair material* (`input.isValidRepairItem(addition)`) — repairs `min(damage, maxDamage/4)` per material item consumed, `price += 1` per item consumed (`repairItemCountCost` tracks how many were actually used, since surplus material items are left untouched); (b) *combine* (matching item or an enchanted book) — if damageable and not a book, computes a durability-boosted merge (`remaining = (input's remaining durability) + (addition's remaining durability) + 12% of max damage`) then merges every enchantment from the addition via "same level ⇒ +1, else max(level)" with a per-enchantment anvil fee (`enchantment.getAnvilCost() * level`, halved-minimum-1 when the source is a book) — **incompatible enchantments cost an extra flat +1 XP level penalty each and are dropped**, and stacking >1 count in the input slot forces the price to a flat 40 (soft-blocks batch-enchant abuse). Renaming (`itemName` differs from current hover name) adds a flat `COST_RENAME = 1`. The **final work cost never appears if it exceeds 40 levels** and the player isn't `hasInfiniteMaterials()` — the classic "too expensive" 39-cap: `onlyRenaming` mode explicitly clamps displayed cost to 39 rather than blocking a pure-rename. Every successful anvil operation also **increases the item's `REPAIR_COST` "prior work" penalty**: `calculateIncreasedRepairCost(base) = base*2 + 1` (so 0→1→3→7→15…), taking the max of input/addition's existing penalty as the new base before doubling.

### 3.7 Brewing

`PotionBrewing` (`net.minecraft.world.item.alchemy`) is **not data-pack driven** — the entire recipe table is a hardcoded sequence of `builder.addMix(...)`/`addStartMix(...)`/`addContainerRecipe(...)` calls in `addVanillaMixes`, gated only by `FeatureFlagSet` (so a disabled experimental feature can still remove entries). Three sub-tables exist: allowed **container** ingredients (bottle types: `POTION`/`SPLASH_POTION`/`LINGERING_POTION`), **container-mix** recipes (upgrade the bottle type itself: water-bottle+gunpowder→splash, splash+dragon's-breath→lingering), and **potion-mix** recipes (`Potion` → ingredient → `Potion`, the actual effect-brewing table, including the two-step `addStartMix` pattern that implicitly also registers "Water + X → Mundane" alongside "Awkward + X → named potion"). `BREWING_TIME_SECONDS = 20` (i.e. 400 ticks total) is the nominal brew duration; `BrewingStandBlockEntity.FUEL_USES = 20` — one blaze powder services 20 brewing operations, tracked via a `DATA_FUEL_USES` container-data slot alongside `DATA_BREW_TIME`.

### 3.8 Loot tables engine

**Model.** `LootTable` = a param-set tag (`ContextKeySet`, determines which context values must be supplied — e.g. block-drop tables require `BLOCK_STATE`/`ORIGIN`/`TOOL`), an optional `random_sequence` id (see below), a list of `LootPool`s, and post-processing `functions`. Each `LootPool` = a list of `LootPoolEntryContainer` entries, pool-level `conditions`, pool-level `functions`, and two `NumberProvider`s — `rolls` and `bonus_rolls` (bonus scaled by `context.getLuck()` and floored). Roll evaluation: for each roll, gather every entry whose `getWeight(luck)` is `> 0` after `entry.expand()` resolves composite entries (alternatives/sequence/group) down to concrete singleton entries, sum weights, then either take the sole survivor directly or pick one via a cumulative-weight random draw (`random.nextInt(totalWeight)` then subtract each candidate's weight until negative).

**Entry types (9, registered in `LootPoolEntries`):** `empty` (`EmptyLootItem`, contributes nothing but can still hold conditions/weight for shaping pool probability), `item` (`LootItem`, a fixed `Item`), `loot_table` (`NestedLootTable`, recurses into another loot table — `LootContext.pushVisitedElement`/`popVisitedElement` guards against infinite self-reference, logging a warning and aborting rather than stack-overflowing), `dynamic` (`DynamicLoot`, block-entity-supplied contents, e.g. banner/shulker-box "drop with my current contents"), `tag` (`TagEntry`, expands to every item in an item tag, optionally `expand`-ing to one random member vs. all members), `slots` (`SlotLoot`, references items currently occupying named container slots — vault/trial-chamber machinery), `alternatives` (`AlternativesEntry`, tries children in order, first whose conditions pass wins, rest are skipped), `sequence` (`SequentialEntry`, runs children in order until one fails its conditions, then stops), `group` (`EntryGroup`, runs every child unconditionally, still subject to each child's own conditions).

**Condition types (19, `LootItemConditions`):** `inverted`, `any_of`, `all_of` (composite boolean logic), `random_chance` (flat probability), `random_chance_with_enchanted_bonus` (probability boosted per level of a chosen enchantment on the tool), `entity_properties` / `entity_scores` (predicate against `THIS_ENTITY`), `killed_by_player`, `block_state_property`, `match_tool` (`ItemPredicate`-style match against `TOOL`), `table_bonus` (fortune/looting-style per-level probability table), `survives_explosion` (`1/explosionRadius` chance, only meaningful when `EXPLOSION_RADIUS` param present), `damage_source_properties`, `location_check`, `weather_check`, `reference` (indirection to a shared, separately-registered condition), `time_check`, `value_check` (generic numeric range test against a `NumberProvider`), `enchantment_active_check` (queries whether a given enchantment's location-based effect is currently active on the acting entity), `environment_attribute_check`.

**Function types (49, `LootItemFunctions`)** — the largest sub-registry; every one is a `LootItemConditionalFunction` (carries its own condition list, short-circuits if unmet) applying a transform to the in-flight `ItemStack`. Grouped by concern: quantity (`set_count`, `apply_bonus`, `limit_count`, `enchanted_count_increase`), enchanting (`enchant_with_levels`, `enchant_randomly`, `set_enchantments`), item identity/data (`set_item`, `furnace_smelt`, `set_damage`, `set_custom_data`, `set_components`, `set_custom_model_data`, `toggle_tooltips`), naming/lore (`set_name`, `copy_name`, `set_lore`), containers (`set_contents`, `modify_contents`, `set_loot_table` — attaches a deferred+seeded loot table to a placed container rather than filling it immediately), copying (`copy_custom_data`, `copy_state`, `copy_components`), combat-context items (`set_attributes`, `explosion_decay` — thins stack counts by explosion radius), cosmetic (`set_banner_pattern`, `set_potion`, `set_random_dyes`, `set_random_potion`, `set_instrument`, `set_stew_effect`, `set_book_cover`, `set_written_book_pages`, `set_writable_book_pages`, `set_ominous_bottle_amplifier`, `fill_player_head`), control flow (`filtered` — re-gates by a `ItemPredicate`, `reference`, `sequence`), exploration (`exploration_map`), and terminal (`discard` — unconditionally drops the item from the pool's output).

**Number providers (8, `NumberProviders`):** `constant`, `uniform` (inclusive float range), `binomial` (n trials at probability p — used for e.g. bonus-roll style distributions), `score` (scoreboard-objective value), `storage` (command-storage NBT value), `sum` (adds N nested providers), `enchantment_level` (reads the `ENCHANTMENT_LEVEL` context param, used inside enchantment-effect JSON), `environment_attribute` (reads a world/biome-derived attribute).

**Loot context param sets (`LootContextParamSets`, 24 registered).** Each is a named bundle of *required* + *optional* `LootContextParams` that a given trigger site guarantees. Beyond the familiar `chest`/`entity`/`block`/`fishing`/`gift`/`archaeology`/`vault`, six sets exist purely for **enchantment effect validation**: `enchanted_damage`, `enchanted_item`, `enchanted_location`, `enchanted_entity`, `hit_block`, plus `villager_trade` (required: `ORIGIN`, `THIS_ENTITY`, `ADDITIONAL_COST_COMPONENT_ALLOWED`) for trade-offer condition/function evaluation, and the catch-all `generic`/`ALL_PARAMS` (every param, used as the default when a table doesn't declare a `type`).

**Determinism — `random_sequence`.** A loot table (and independently a `TradeSet`) may declare a `random_sequence` id. `LootContext` resolves this to a **named, world-persistent `RandomSequence`** (seeded from the world seed + the sequence id, stored server-side) rather than using a fresh/ambient RNG — this is what makes repeated openings of loot chests sharing the same `random_sequence` id (e.g. every dungeon chest of a kind) produce a *coordinated*, seed-reproducible stream rather than independent randomness, matching vanilla's per-structure loot determinism guarantees.

### 3.9 Villager trading (fully data-driven in 26.2)

The old hardcoded `VillagerTrades.ItemListing[]`-per-profession-per-level Java tables are gone from the *authoring* path (Java-side `VillagerTrades`/`TradeRebalanceVillagerTrades` classes still exist as **bootstrap/datagen sources** that got exported once into the datapack, not as runtime logic). At runtime, trade generation is entirely registry-driven:

- **`TradeSet`** (registry `minecraft:trade_set`, one entry per profession+level, e.g. `trade_set/armorer/level_1`) = a `HolderSet<VillagerTrade>` (`trades`, typically a tag reference like `#minecraft:armorer/level_1`), a `NumberProvider` `amount` (how many trades to roll), `allow_duplicates` (bool), and an optional `random_sequence`.
- **`VillagerTrade`** (registry `minecraft:villager_trade`) = `wants` (a `TradeCost`), optional `additional_wants` (second-cost slot, e.g. emerald+item trades), `gives` (an `ItemStackTemplate`), `max_uses`/`reputation_discount`/`xp` (each a `NumberProvider`, defaulting to constants 4/0/1), an optional `merchant_predicate` (`LootItemCondition`, evaluated against `VILLAGER_TRADE` param set — e.g. biome-gated trades), a list of `given_item_modifiers` (`LootItemFunction`s applied to the `gives` template — this is how a trade can attach random enchantments to its output, or read/consume the transient `ADDITIONAL_TRADE_COST` component to bump price), and an optional `double_trade_price_enchantments` tag (if the generated item carries any enchantment from this set, the computed price doubles).
- **`TradeCost`/`ItemCost`**: `TradeCost` is the JSON-facing form (item + `NumberProvider` count + `DataComponentExactPredicate`); `toItemCost(lootContext, additionalCost)` resolves it to a concrete `ItemCost` (clamped `[0, item.maxStackSize]`), folding in the modifier-derived `additionalCost`.

**Generation flow** (`AbstractVillager.addOffersFromTradeSet` → `Villager.updateTrades`, triggered on profession/level change): build a `LootContext` for `VILLAGER_TRADE` (params `ORIGIN`, `THIS_ENTITY` = the villager, `ADDITIONAL_COST_COMPONENT_ALLOWED`), resolved against the `TradeSet`'s `random_sequence` if present. Roll `tradeSet.calculateNumberOfTrades(context)` times, each time picking a uniformly random member of the trade pool (`lootContext.getRandom().nextInt(poolSize)`) and calling `VillagerTrade.getOffer(lootContext)`; a `null` result (merchant predicate failed, or the computed `ItemCost` count fell to `< 1`) either removes that candidate from the pool (`allow_duplicates = false`) or is simply retried, so the exact composition — but not the count — of villager stock is randomized per instance.

`MerchantOffer` (the runtime, per-villager-instance trade — distinct from the template `VillagerTrade`) tracks live `uses`/`maxUses`, a `demand` counter (`updateDemand()`: `demand += uses - (maxUses - uses)`, i.e. demand rises the more a trade is used relative to its remaining headroom) and a `specialPriceDiff` (hero-of-the-village / other discounts). Effective price (`getModifiedCostCount`) is `clamp(basePrice + max(0, floor(basePrice * demand * priceMultiplier)) + specialPriceDiff, 1, maxStackSize)` — demand only ever inflates price (the `max(0, …)` floors any negative demand contribution), while `specialPriceDiff` can push it either way.

## 4. Key types

| Class (package) | Role | Notable details |
|---|---|---|
| `Item` (`.item`) | Stateless per-type behaviour singleton | `Item(Properties)`; `components()`; `useOn`/`use`/`finishUsingItem`/`hurtEnemy`/`mineBlock` overridable hooks; `DEFAULT_MAX_STACK_SIZE=64`, `ABSOLUTE_MAX_STACK_SIZE=99` |
| `ItemStack` (`.item`) | Per-instance value: item + count + component patch | `EMPTY` sentinel; `matches`/`isSameItemSameComponents`/`matchesIgnoringComponents` static equality helpers; `hurtAndBreak` durability path routes through `EnchantmentHelper.processDurabilityChange` |
| `DataComponentType<T>` (`.core.component`) | Opaque typed slot marker | Optional persistent `Codec<T>` + `StreamCodec`; `.cacheEncoding()`, `.ignoreSwapAnimation()` |
| `DataComponents` (`.core.component`) | Registry of all 90 vanilla component types | `COMMON_ITEM_COMPONENTS` shared base map |
| `PatchedDataComponentMap` (`.core.component`) | `ItemStack`'s live storage: prototype + copy-on-write patch | `set`/`remove` collapse patch entries back to nothing when value == prototype default |
| `DataComponentPatch` (`.core.component`) | Serializable diff form of a patch | `PatchKey` string encoding (`"!id"` = removed); `DELIMITED_STREAM_CODEC` for untrusted input |
| `AbstractContainerMenu` (`.inventory`) | Base class for every inventory screen | `doClick` state machine; `broadcastChanges`; `incrementStateId` (15-bit rolling id) |
| `Slot` (`.inventory`) | One inventory position inside a menu, bound to a `Container` + local index | `mayPlace`/`mayPickup`/`safeInsert`/`safeTake` |
| `RemoteSlot.Synchronized` (`.inventory`) | Server-side shadow of what the client believes a slot holds | Can hold a `HashedStack` hash instead of a full copy |
| `RecipeManager` (`.item.crafting`) | Loads/holds all recipes; builds derived indices at reload | `finalizeRecipeLoading` builds `RecipePropertySet`s + stonecutter index + display table |
| `Ingredient` (`.item.crafting`) | Item-or-tag predicate, not count-aware | Rejects empty/air `HolderSet` at construction |
| `ShapedRecipePattern` (`.item.crafting`) | Parsed+shrunk shaped-crafting grid | `matches` tries mirrored placement unless `symmetrical` |
| `CraftingInput` (`.item.crafting`) | Pre-trimmed snapshot of a crafting grid + `StackedItemContents` | `ofPositioned` returns the trim offset too |
| `RecipeCache` (`.item.crafting`) | Fixed-size MRU-front cache of last-matched recipe per grid state | Capacity 10 in `CrafterMenu` |
| `Enchantment` (`.item.enchantment`) | Data-driven enchantment definition + effect map | `effects` is a `DataComponentMap` over `EnchantmentEffectComponents` |
| `EnchantmentHelper` (`.item.enchantment`) | Static algorithm home: cost rolls, selection, per-hook dispatch | `getEnchantmentCost`, `selectEnchantment`, many `modifyX`/`processX` hooks called from combat/mining/fishing code |
| `ItemEnchantments` (`.item.enchantment`) | Per-item enchantment level map | `Mutable.upgrade` merges via `max` |
| `LootTable` / `LootPool` (`.level.storage.loot`) | Root loot definition / weighted-roll group | `getRandomItemsRaw` guards recursion via visited-entry stack |
| `LootContext` (`.level.storage.loot`) | Per-evaluation state: resolved params + RNG + visited-entry guard | RNG sourced from a persistent named `RandomSequence` when `random_sequence` is set |
| `TradeSet` / `VillagerTrade` (`.item.trading`) | Data-driven trade-pool template / single trade template | `VillagerTrade.getOffer` resolves to a live `MerchantOffer` |
| `MerchantOffer` (`.item.trading`) | Live, stateful trade instance held by a merchant | `demand`/`specialPriceDiff` drive dynamic pricing |
| `PotionBrewing` (`.item.alchemy`) | Hardcoded brewing recipe table (not data-pack driven) | `Builder.addStartMix` implicitly also registers the Water→Mundane branch |

## 5. Constants & magic values

| Constant | Value | Source class |
|---|---|---|
| Default / absolute max stack size | 64 / 99 | `Item` |
| Max tooltip durability bar segments | 13 | `Item.MAX_BAR_WIDTH` |
| "Approximately infinite" use duration | 72000 ticks (1 hour) | `Item` |
| Anvil rename max length | 50 chars | `AnvilMenu.MAX_NAME_LENGTH` |
| Anvil "too expensive" cap | 40 levels | `AnvilMenu` |
| Anvil renaming cost | 1 level | `AnvilMenu.COST_RENAME` |
| Anvil incompatible-enchant penalty | +1 level each | `AnvilMenu.COST_INCOMPATIBLE_PENALTY` |
| Anvil repair-cost growth | `base*2+1` | `AnvilMenu.calculateIncreasedRepairCost` |
| Anvil durability-merge armor bonus | 12% of max damage | `AnvilMenu.createResult` |
| Anvil batch-stack enchant price | flat 40 | `AnvilMenu.createResult` |
| Enchanting table max bookshelves counted | 15 | `EnchantmentHelper.getEnchantmentCost` |
| Enchanting table per-slot cost divisor pattern | slot0: `/3`; slot1: `*2/3+1`; slot2: `max(x, bookcases*2)` | `EnchantmentHelper.getEnchantmentCost` |
| Enchant-selection continue chance | `rand(50) <= cost` | `EnchantmentHelper.selectEnchantment` |
| Enchantment max level (hard ceiling) | 255 | `Enchantment.MAX_LEVEL` |
| Enchantment weight range | 1–1024 | `EnchantmentDefinition` codec |
| Brewing nominal duration | 20 s (400 ticks) | `PotionBrewing.BREWING_TIME_SECONDS` |
| Blaze powder brewing fuel uses | 20 | `BrewingStandBlockEntity.FUEL_USES` |
| Default consumable eat/drink duration | 1.6 s (32 ticks) | `Consumable.DEFAULT_CONSUME_SECONDS` |
| Consume particle/sound start fraction | 0.21875 (7/32) of duration | `Consumable.CONSUME_EFFECTS_START_FRACTION` |
| Consume particle/sound interval | every 4 ticks | `Consumable.CONSUME_EFFECTS_INTERVAL` |
| Shaped recipe max grid size | 3×3 | `ShapedRecipePattern.MAX_SIZE` |
| Shapeless recipe ingredient count | 1–9 | `ShapelessRecipe.MAP_CODEC` (`Ingredient.CODEC.listOf(1,9)`) |
| Crafting-table grid | 3×3 (`CRAFT_SLOT_COUNT=9`) | `CraftingMenu` |
| Player inventory crafting grid | 2×2 | `InventoryMenu.CRAFTING_GRID_WIDTH/HEIGHT` |
| Crafter recipe cache size | 10 entries | `CrafterBlock.RECIPE_CACHE` |
| Default cooking time (furnace) | 200 ticks | `SmeltingRecipe` default via `cookingMapCodec` |
| Default cooking time (blast furnace / smoker) | 100 ticks | `BlastingRecipe`/`SmokingRecipe` default |
| Default cooking time (campfire) | 600 ticks | `CampfireCookingRecipe` default |
| Bundle total weight capacity | 1 (a `Fraction`, i.e. "100% full") | `BundleContents` |
| Bundle-inside-bundle extra weight tax | 1/16 | `BundleContents.BUNDLE_IN_BUNDLE_WEIGHT` |
| Item container contents max slots | 256 | `ItemContainerContents.MAX_SIZE` |
| Menu container-state id wraparound | 15-bit (`& 32767`) | `AbstractContainerMenu.incrementStateId` |
| Encoder cache size (component re-encoding memo) | 512 | `DataComponents.ENCODER_CACHE` |
| Component-patch decode entry cap | 65536 | `DataComponentPatch` stream decode |

## 6. Cross-subsystem interfaces

**Consumes from:**
- **World/chunk/persistence** — `ItemStack` NBT (de)serialization (`ItemStack.CODEC`), block-entity storage of contained items (chests, shulker boxes via `container`/`container_loot` components), lock codes.
- **Entities** — `LivingEntity`/`Player` state for consuming (food data, hunger), equipment slots (`EquipmentSlot`/`EquipmentSlotGroup`) that both `attribute_modifiers` and `equippable` target; `Attribute`/`AttributeModifier` types for item- and enchantment-granted modifiers.
- **World generation / structures** — structure loot tables reference `BuiltInLootTables` ids; `random_sequence` ties loot RNG to world-seed-derived persistent sequences owned by the level.
- **Registries / data-driven loading** — every recipe/loot-table/enchantment/trade-set/villager-trade is a `Registry`/`HolderLookup` entry loaded via the standard datapack reload pipeline (`SimpleJsonResourceReloadListener`); `FeatureFlagSet` gates experimental items/enchantments/recipes.
- **Advancements** — `CriteriaTriggers` (`RECIPE_UNLOCKED`, `CONSUME_ITEM`, `ENCHANTED_ITEM`, `ITEM_DURABILITY_CHANGED`) fire from this subsystem's mutation points.

**Provides to:**
- **Networking/protocol** — every menu-related clientbound/serverbound packet (`ClientboundContainerSetSlotPacket`, `ClientboundContainerSetContentPacket`, `ServerboundContainerClickPacket`/`ContainerInput`, `ClientboundRecipeBookAddPacket`/`RemovePacket`/`SettingsPacket`, `ClientboundCooldownPacket`, `ClientboundSetHeldSlotPacket`) is shaped directly by this subsystem's data model (`DataComponentPatch` stream codecs, `HashedStack`, `ItemStack.STREAM_CODEC`).
- **Combat/mechanics** — `EnchantmentHelper` hook methods are the integration surface mining/combat/projectile code calls into (damage modification, knockback, durability, XP), so `05-game-mechanics.md` (combat, mining) depends on this domain rather than duplicating enchantment math.
- **Villager AI/mechanics** — the profession→`TradeSet` wiring and `Merchant` interface are consumed by villager AI/behavior code (restocking, reputation) which lives in the entity/mechanics domain.
- **Client rendering (Phase 2)** — `RecipeDisplay`/`SlotDisplay` (crafting book UI), `TooltipComponent` (bundle preview), tooltip component ordering all define exactly what a client must render — this is the authoritative contract for the future Rust client's recipe-book and tooltip UI.

## 7. Data-generator cross-reference

| Path (under `datagen/generated`) | Contents |
|---|---|
| `reports/minecraft/components/item/*.json` | One file per item id: the item's **fully-resolved default `DataComponentMap`** as JSON (e.g. `diamond_sword.json` shows `attribute_modifiers`, `tool` rules incl. `cobweb`/`#sword_instantly_mines`/`#sword_efficient` speed entries, `max_damage=1561`, `enchantable.value=10`, `repairable.items=#diamond_tool_materials`). This is the ground truth for every `Item.Properties` builder call in-game and should drive an automatically generated Rust default-component table rather than hand-transcription. |
| `data/minecraft/recipe/*.json` | 1585 files. All seven `RecipeType`s' JSON forms; shaped recipes show the `key`/`pattern`/`result` shape used by §3.5; cooking recipes show `experience`/`group`/`ingredient`. |
| `data/minecraft/loot_table/{blocks,entities,chests,gameplay,archaeology,equipment,shearing,spawners,pots,dispensers,harvest,carve,brush,charged_creeper}/**` | Every vanilla loot table, organized by trigger category — directly instantiable as `LootTable.DIRECT_CODEC` fixtures for differential testing. |
| `data/minecraft/enchantment/*.json` | Every vanilla enchantment's full definition — `description`, `min_cost`/`max_cost` linear formulas, `weight`, `max_level`, `slots`, `supported_items`/`primary_items` tag refs, and the `effects` map keyed by `EnchantmentEffectComponents` (e.g. `sharpness.json`'s `minecraft:damage` effect is `{type: add, value: {type: linear, base: 1.0, per_level_above_first: 0.5}}`) — this is the canonical source for reimplementing every enchantment's numeric curve without needing to read Java. |
| `data/minecraft/trade_set/<profession>/level_<n>.json` | Per profession+level `TradeSet` (`amount`, `random_sequence`, `trades` tag reference). |
| `data/minecraft/villager_trade/<profession>/<level>/*.json` | Individual `VillagerTrade` definitions (`wants`, `gives`, `max_uses`, `reputation_discount`, etc.) — ground truth for every villager offer without touching the (now-vestigial) hardcoded Java tables. |
| `data/minecraft/tags/item/*.json` | Item tags referenced pervasively by ingredients, enchantment `supported_items`, repair-material lists. |
| `reports/registries.json` | Canonical id lists for every registry mentioned above (`data_component_type`, `enchantment_effect_component_type`, `loot_pool_entry_type`, `loot_function_type`, `loot_condition_type`, `loot_number_provider_type`, `recipe_type`, `recipe_serializer`, `trade_set`, `villager_trade`, `menu` (`MenuType`)). |
| `reports/packets.json` | Wire shapes for every container/recipe-book/cooldown packet named in §6 — needed to implement the sync protocol bit-for-bit. |

## 8. Notes for Rusty Clanker

- **The component system is the single hardest piece to get bit-identical.** `PatchedDataComponentMap`'s "collapse to prototype default" behaviour on `set`/`remove` is not just an optimization — it changes what gets serialized (an explicit-but-equal-to-default value never round-trips as present in the patch) and therefore what `DataComponentPatch` equality and hashing see. An ECS-component-store reimplementation must replicate this collapsing exactly, including for compound values (`Optional`, records) where Java's `equals` is structural — Rust's `PartialEq` derive on the equivalent structs must match field-for-field.
- **`ItemStack` equality has no single canonical definition** — there are four (`matches`, `isSameItem`, `isSameItemSameComponents`, `matchesIgnoringComponents`) used in different call sites for different purposes (slot stacking vs. crafting-input dedup vs. server-trust verification). Picking one Rust `PartialEq` impl and using it everywhere will silently diverge from vanilla in at least one of: shift-click stacking, bundle insertion, or anvil/creative desync detection.
- **Recipe matching is unindexed by design** (linear scan + a tiny MRU cache), and vanilla accepts that cost because recipe counts per `RecipeType` are small and the cache absorbs the hot path (holding the same items in a crafting grid across many ticks). A from-scratch engine under load (e.g. many `CrafterMenu`s from redstone auto-crafters) should still consider a real ingredient-indexed lookup rather than reproducing the linear scan verbatim, since ARCH's fully-multithreaded design may run many more concurrent crafting evaluations than vanilla's single-threaded server ever did — but the **observable behaviour** (which recipe wins on ambiguous multi-match input — first insertion order in the sorted-by-`Identifier` `RecipeMap`) must be preserved for parity.
- **Loot/enchant/trade randomness determinism hinges on `random_sequence`.** Any table/enchantment-effect/trade-set that references one must draw from the same seed-derived persistent sequence state as vanilla, not from an ambient thread-local RNG — otherwise repeated structure loot (dungeon chests, buried treasure, trial-chamber vaults) will not match seed-for-seed with vanilla, breaking the project's "bit-identical by default" parity mandate (ARCH-D/PLAN's testing goals). This persistent sequence state is server/world-owned, not item-subsystem-owned, and needs a clear ownership boundary with `03-world-chunks-persistence.md`.
- **Enchantment effects reuse the *loot* condition/number-provider machinery wholesale** (`ConditionalEffect` wraps a `LootItemCondition`; value curves are `EnchantmentValueEffect` implementors structurally identical in spirit to `NumberProvider`). A Rust implementation should model "evaluate an expression tree against a typed context" as one shared engine serving loot conditions/functions, enchantment effects, and (per `VillagerTrade.merchant_predicate`) trades — building three parallel copies would triple the parity-testing surface for what is genuinely one mechanism with three context-key-set flavors.
- **Container sync is intentionally hash-optimistic**, trusting client-predicted slot states (`HashedStack`) until proven wrong. A cluster-mode reimplementation (13-cluster-architecture.md) doing seamless region handoff must ensure the *new* owning node reconstructs an equivalent `RemoteSlot` shadow state for every open menu before resuming sync, or the first post-handoff broadcast will look like a spurious full-resync to the client (harmless, but a determinism/latency smell worth avoiding).
- **Anvil/enchanting-table cost formulas are exact integer/float pipelines with a specific evaluation order** (e.g. the anvil's `namingCost == price` check to detect "pure rename" happens *after* all enchant-cost accumulation, using integer equality on the accumulated `price`) — these are easy to get "close" but not bit-identical to in a naive re-derivation; treat §3.6's formulas as normative pseudocode to port directly rather than re-deriving from wiki descriptions.
- **`Consumable`/`ConsumableListener` is an extension-point pattern worth keeping**: `FoodProperties` itself is just one `ConsumableListener` implementor discovered via `stack.getAllOfType(ConsumableListener.class)` — i.e. *any* component type on the stack can hook into "on consume" without `Consumable` needing to know about it ahead of time. This is a clean precedent for how Rusty Clanker's modding API (06-modding-api.md) could let mod-defined components hook vanilla lifecycle events without a central registry of hook types.
- **Villager trading's Java `VillagerTrades`/`TradeRebalanceVillagerTrades` classes are legacy datagen sources, not runtime logic** — do not port their Java structure; port the *generated* `trade_set`/`villager_trade` JSON directly, since that is what 26.2 actually executes.
- **Brewing is the one major crafting-adjacent system that is *not* data-pack-driven** in vanilla 26.2 (hardcoded `PotionBrewing.addVanillaMixes`). Rusty Clanker should decide deliberately whether to keep this as a hardcoded Rust table (parity-simplest, matches vanilla's own architecture) or promote it to a data-driven form for moddability — either is legitimate, but it should be a stated `MECH-D`/`MOD-D` decision rather than an oversight, since every *other* recipe-shaped system in this domain is JSON-driven and an implementer might assume brewing is too.
- **`DataComponentPatch`'s two stream-codec variants (`STREAM_CODEC` vs. `DELIMITED_STREAM_CODEC`) encode a trust boundary directly into the wire format** — the delimited (length-prefixed-per-component) form exists specifically so a hostile/buggy client's malformed component payload can be skipped without desyncing the whole packet. NET-D (protocol/networking doc) should note that any packet accepting component data *from* a client must use the delimited codec, and any Rust decoder must reject-and-resync at the component boundary the same way, not abort the whole connection on the first bad component.
