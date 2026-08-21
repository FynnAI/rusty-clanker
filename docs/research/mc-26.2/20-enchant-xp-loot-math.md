# Enchanting, XP & Loot Probability Math — Vanilla 26.2 Deep Research

## 1. Purpose

This is the one domain in the whole engine where "close" is worthless: every mechanic here is a probability distribution, and a distribution that is off by one RNG call, one integer-truncation point, or one iteration order silently desyncs from vanilla while looking completely correct in casual play. Enchanting-table offers, anvil costs, grindstone refunds, XP-level thresholds, and every loot roll in the game are all pure functions of (game state, RNG stream) — there is no rendering, no timing, no player skill involved, which means **bit-identical parity here is fully achievable and therefore fully expected**. Getting a formula "roughly right" (e.g. anvil cost off by the tax/price split, or `apply_bonus`'s `ore_drops` formula without the double-zero clamp) produces items, XP totals, and drop rates that diverge from vanilla in ways players who know the game will notice immediately (speedrunners, anvil-fee calculators, drop-rate documentation all encode these exact constants). This document exists so a Rust implementer never has to guess a formula, a cast point, or an RNG call count — every claim below is read from the decompiled 26.2 source or the generated vanilla datapack, not from memory of older versions or from wiki paraphrase.

## 2. Where it lives

| Package / file | Responsibility |
|---|---|
| `net.minecraft.world.item.enchantment.EnchantmentHelper` | Static home for the enchanting-table cost/selection algorithm, and every per-hook enchantment-effect dispatcher (`processMobExperience`, `modifyDurabilityToRepairFromXp`, `getFishingLuckBonus`, …) |
| `net.minecraft.world.item.enchantment.Enchantment` | Data-driven enchantment record: `EnchantmentDefinition` (weight, max level, `Cost` linear formulas, anvil cost, slots), `exclusiveSet`, `effects` (a `DataComponentMap` over the separate `EnchantmentEffectComponents` registry) |
| `net.minecraft.world.item.enchantment.LevelBasedValue` | The level→float curve DSL (`linear`, `clamped`, `fraction`, `levels_squared`, `exponent`, `lookup`) used inside enchantment-effect JSON |
| `net.minecraft.world.item.enchantment.effects.*` | The effect "verbs": `AddValue`, `MultiplyValue`, `SetValue`, `ScaleExponentially`, `RemoveBinomial` implement `EnchantmentValueEffect.process(level, random, input) -> float` |
| `net.minecraft.world.item.enchantment.ItemEnchantments` | Per-item enchantment level map — backed by fastutil `Object2IntOpenHashMap<Holder<Enchantment>>` (see §7 hazard #1) |
| `net.minecraft.world.inventory.EnchantmentMenu` | Bookshelf counting, the three displayed-cost rolls, the per-slot clue draw, and commit-time enchant application |
| `net.minecraft.world.level.block.EnchantingTableBlock` | `BOOKSHELF_OFFSETS` geometry + `isValidBookShelf` tag checks |
| `net.minecraft.world.inventory.AnvilMenu` | `createResult()` — the entire anvil cost/merge pipeline |
| `net.minecraft.world.inventory.GrindstoneMenu` | Disenchant merge + XP-refund roll |
| `net.minecraft.world.entity.player.Player` | XP level curve (`getXpNeededForNextLevel`), `giveExperiencePoints`/`giveExperienceLevels`, per-player enchanting seed, death XP reward |
| `net.minecraft.world.entity.ExperienceOrb` | Orb value tiering/splitting/merging, mending-repair consumption |
| `net.minecraft.world.level.storage.loot.LootPool` / `LootTable` / `LootContext` | Roll/bonus-roll evaluation, weighted entry selection, RNG-source resolution (`random_sequence` vs. ambient) |
| `net.minecraft.world.level.storage.loot.entries.*` | `LootPoolSingletonContainer` (weight+quality/luck), `AlternativesEntry`/`SequentialEntry`/`EntryGroup` (composite short-circuit logic) |
| `net.minecraft.world.level.storage.loot.functions.ApplyBonusCount` | `binomial_with_bonus_count` / `ore_drops` / `uniform_bonus_count` formulas |
| `net.minecraft.world.level.storage.loot.providers.number.*` | `UniformGenerator`, `BinomialDistributionGenerator`, `ConstantValue`, `Sum`, `EnchantmentLevelProvider` |
| `net.minecraft.world.level.storage.loot.predicates.*` | `LootItemRandomChanceCondition`, `..WithEnchantedBonusCondition`, `BonusLevelTableCondition`, `ExplosionCondition`, `AllOfCondition` (short-circuit AND) |
| `net.minecraft.world.RandomSequence` / `RandomSequences` | World-persistent, seed-derived named RNG streams (Xoroshiro128++-backed) |
| `net.minecraft.world.entity.projectile.FishingHook` | Bobber state machine, wait-time formulas, open-water rule, catch resolution |
| `net.minecraft.util.RandomSource` / `Mth` | `nextInt(min,max)`, `nextFloat(min,max)`, `triangle(mean,spread)`, `floor` — the numeric primitives every formula above is built from |

Broad-cartography cross-references: `10-items-recipes-loot.md` §3.6/§3.8 already summarizes the enchanting-table and loot-table shape at a "where and roughly how" level — this document supersedes its formulas with verified exact math and adds everything §3.6/§3.8 did not cover (bookshelf tag geometry, exact RNG call counts/order, XP curve, orb mechanics, fishing wait-time, `apply_bonus` math, luck formula). `11-player-gameplay.md` line 285–286 already states the XP curve and death-reward formula; confirmed byte-for-byte against source here (§3.5).

## 3. The mechanics

### 3.1 RNG sources — which stream feeds which mechanic

This underlies every RNG call count below, so it comes first. 26.2 has two `RandomSource` algorithms (full LCG/Xoroshiro internals are owned by `05-worldgen.md` §RNG — not repeated here): a legacy 48-bit LCG (`LegacyRandomSource`) and a 128-bit `Xoroshiro128PlusPlus`. What matters for this domain is **which instance** backs each call site, because that determines whether a roll is world-seed-reproducible at all:

| Call site | `RandomSource` instance | Seeded from | Reproducible from world seed? |
|---|---|---|---|
| `Entity.random` (every entity: mobs, `ExperienceOrb`, `FishingHook`'s own jitter) | `RandomSource.create()` → `LegacyRandomSource` | `RandomSupport.generateUniqueSeed()` = `(SEED_UNIQUIFIER.updateAndGet(s -> s * 1181783497276652981L)) XOR System.nanoTime()` — a per-instance, per-JVM-run value | **No.** Ephemeral, unique per entity instance, tied to wall-clock nanotime |
| `Level.getRandom()` (ambient level RNG — anvil damage-chance roll, grindstone none, `LootContext` fallback when no `random_sequence`) | Same `RandomSource.create()` pattern, one instance per `Level` | Same `generateUniqueSeed()` | **No** |
| `EnchantmentMenu.random` (table cost rolls + clue draw) | `RandomSource.create()`, but immediately **reseeded** via `setSeed(enchantmentSeed [+ slot])` before every use | The player's persisted `enchantmentSeed` int (NBT `XpSeed`) | Reproducible **given the player's current `enchantmentSeed`**, but that seed itself is rerolled via the ephemeral `player.random.nextInt()` after every successful enchant — so it is not derivable from the world seed, only self-consistent within one un-consumed roll |
| `LootContext` when the table/effect declares `random_sequence: "<id>"` | `server.getRandomSequence(id)` → `RandomSequences.get` → `RandomSequence` → `XoroshiroRandomSource` | `(worldSeed XOR salt) XOR seedFromHashOf(id)` (see §3.6.4) | **Yes.** This is the only loot RNG that is genuinely deterministic from the world seed |
| `LootContext` when no `random_sequence` is set (most mob-drop and many block-drop tables) | Falls back to `level.getRandom()` | Ephemeral | **No** — interleaves with every other ambient use of the level's RNG |

**Consequence for Rusty Clanker:** "seed-identical parity" for loot is a real, testable contract only for `random_sequence`-tagged tables (structure/dungeon/buried-treasure/vault/fishing-junk-style chests, and the fishing loot table itself — see §3.7) and for worldgen. Ordinary mob combat drops, most block drops, anvil damage rolls, and grindstone XP rolls are **not** seed-reproducible in vanilla itself; call-count/order still matters for interleaving behavior, but there is no fixed "correct" numeric outcome to test against beyond replaying the exact same ambient-RNG stream.

### 3.2 Enchanting table — bookshelf geometry

`EnchantingTableBlock.BOOKSHELF_OFFSETS` is generated once, statically, as all integer offsets `(x, y, z)` with `x, z ∈ [-2, 2]`, `y ∈ {0, 1}`, filtered to `|x| == 2 || |z| == 2` (the outer ring of the 5×5 square). That ring has `25 − 9 = 16` cells per y-layer; with two y-layers the candidate list has **32 offsets**, not 15 — the historical "15 bookshelves" figure is a *cap on the cost formula*, not the count of checkable positions (see below).

For each offset, `EnchantingTableBlock.isValidBookShelf(level, tablePos, offset)` requires **both**:
1. The block at `tablePos + offset` matches tag `minecraft:enchantment_power_provider` — in the vanilla datapack this tag contains exactly `minecraft:bookshelf` (no chiseled bookshelf; the geometry/tag split is what makes this moddable).
2. The block at `tablePos + (offset.x / 2, offset.y, offset.z / 2)` — the "sight line" cell halfway between table and shelf — matches tag `minecraft:enchantment_power_transmitter`, which in vanilla is `#minecraft:replaceable` (air, tall grass, water, etc., generalized from the historical "must be air" rule).

Integer division here is **Java truncating division toward zero**, not floor: for `offset.x ∈ {-2,-1,0,1,2}`, `offset.x / 2` yields `{-1, 0, 0, 0, 1}` — `-1/2` is `0`, not `-1`. `offset.y` is used unhalved (already 0 or 1).

`EnchantmentMenu.slotsChanged` counts `bookcases` as the number of offsets (out of the 32) that pass both checks, then **clamps only for the cost formula**: `bookcases = min(bookcases, 15)`. Building a valid double-ring wall of bookshelves can legitimately produce a raw count above 15; the clamp makes counts of 15 and, say, 22 cost-identical.

### 3.3 Enchanting table — the three displayed costs

One `RandomSource` (`EnchantmentMenu.random`) is **reseeded once** with the player's `enchantmentSeed` and then used **sequentially across all three slots** — the three cost rolls are not independent reseeds:

```
random.setSeed(enchantmentSeed)
for slot in 0..3:
    roll = random.nextInt(8) + 1 + (bookcases >> 1) + random.nextInt(bookcases + 1)   // 2 RNG calls
    cost[slot] = match slot:
        0 -> max(roll / 3, 1)
        1 -> roll * 2 / 3 + 1
        2 -> max(roll, bookcases * 2)
    if cost[slot] < slot + 1: cost[slot] = 0      // done by EnchantmentMenu, not EnchantmentHelper
```
Six RNG calls total for the three displayed costs, always in slot order 0→1→2. All arithmetic is `int`; `roll/3` and `roll*2/3` truncate toward zero (roll is always ≥ `nextInt(8)+1 ≥ 1`, i.e. non-negative, so truncation == floor here). `bookcases >> 1` is an arithmetic right shift (== `bookcases/2` for non-negative `bookcases`).

### 3.4 Enchanting table — resolving a slot's actual enchantment(s)

This runs twice per slot with the **same deterministic reseed** — once for the UI "clue" glyphs (cosmetic, purely for the tooltip icons/levels shown before spending), once again on click to actually apply. Both calls reseed with `enchantmentSeed + slotIndex` (a *different* seed per slot, unrelated to the cost-roll seeding in §3.3), so the two evaluations produce **identical results** as long as `enchantmentSeed` hasn't changed between them:

```
random.setSeed(enchantmentSeed + slot)
list = selectEnchantment(random, item, cost[slot], tag_stream(minecraft:in_enchanting_table))
if item is a plain Book and list.size() > 1:
    list.remove(random.nextInt(list.size()))     // collapses a multi-enchant roll to exactly one for books
```
`minecraft:in_enchanting_table` is defined as `#minecraft:non_treasure` — i.e. every enchantment **except** the treasure set (`mending`, `frost_walker`, `soul_speed`, `swift_sneak`, `wind_burst`, `binding_curse`, `vanishing_curse`), which can only be obtained from loot/fishing/trading or anvil-combined from a book.

`selectEnchantment(random, item, cost, source)` — the core selection algorithm:

```
enchantable = item[ENCHANTABLE]           // Enchantable(int value); no component -> return []
cost += 1 + random.nextInt(enchantable/4 + 1) + random.nextInt(enchantable/4 + 1)   // 2 calls, int division truncates
span  = (random.nextFloat() + random.nextFloat() - 1.0) * 0.15                       // 2 calls, float arithmetic, range [-0.15, 0.15]
cost  = clamp(round(cost + cost * span), 1, i32::MAX)          // Math.round(float): floor(x + 0.5), see §3.11
candidates = getAvailableEnchantmentResults(cost, item, source)     // no RNG — see below
results = []
if candidates not empty:
    pick_and_push(candidates)                                   // WeightedRandom: 1 nextInt(totalWeight) call
    while random.nextInt(50) <= cost:                            // 1 call per iteration, condition checked BEFORE body
        if results not empty:
            candidates.retain(|c| Enchantment::areCompatible(results.last, c))
        if candidates empty: break
        pick_and_push(candidates)                                // 1 call
        cost /= 2                                                 // int division, truncates toward zero
return results
```
`getAvailableEnchantmentResults` (no randomness): for every enchantment in `source` whose `isPrimaryItem(item)` is true (or the item is a plain Book, which is primary for everything), scan its level from `maxLevel` **down to** `minLevel = 1`, and take the **first** (i.e. highest) level whose `[minCost(level), maxCost(level)]` window contains the perturbed `cost`; that `(enchantment, level)` pair becomes one candidate. An enchantment with no level satisfying the window contributes nothing.

Weighted pick (`WeightedRandom.getRandomItem`, `net.minecraft.util.random.WeightedRandom`): sum all candidates' `Enchantment.weight` (1–1024 per JSON schema), draw `index = random.nextInt(totalWeight)`, then linear-scan the candidate list subtracting each one's weight from `index` until it goes negative — that candidate wins. **One RNG call per pick, regardless of candidate-list size.**

`Enchantment.areCompatible(a, b)`: `a != b && !a.exclusiveSet.contains(b) && !b.exclusiveSet.contains(a)` — checked **both directions**, since `exclusive_set` tags are not guaranteed symmetric in the data (though the vanilla set is).

Applying the result: on click, `EnchantmentMenu.getEnchantmentList` is recomputed fresh (not reused from the cosmetic clue pass) and **every** `EnchantmentInstance` in the returned list is applied via `item.enchant(...)` — not just one. A book is transmuted to `ENCHANTED_BOOK` before enchants are written. Currency (lapis) is consumed at a flat `slotIndex + 1` count regardless of how many enchantments landed. After a successful application, `player.experienceLevel -= enchantmentCost` (clamped to 0, which also zeroes `experienceProgress`/`totalExperience` if it would go negative — should never trigger given the pre-check, but is present) and `enchantmentSeed = player.random.nextInt()` is rerolled from the player's own ephemeral RNG (§3.1), **not** from the table's RNG.

### 3.5 Enchantment incompatibility (`exclusive_set` tags, full vanilla content)

| Tag | Members |
|---|---|
| `#minecraft:exclusive_set/armor` | `protection`, `blast_protection`, `fire_protection`, `projectile_protection` |
| `#minecraft:exclusive_set/damage` | `sharpness`, `smite`, `bane_of_arthropods`, `impaling`, `density`, `breach` |
| `#minecraft:exclusive_set/mining` | `fortune`, `silk_touch` |
| `#minecraft:exclusive_set/bow` | `infinity`, `mending` |
| `#minecraft:exclusive_set/crossbow` | `multishot`, `piercing` |
| `#minecraft:exclusive_set/riptide` | `loyalty`, `channeling` |
| `#minecraft:exclusive_set/boots` | `frost_walker`, `depth_strider` |

Each enchantment's `exclusive_set` field references one of these tags (or none). Compatibility is checked pairwise per §3.4; there is no global "one enchantment per category" rule beyond these explicit sets.

### 3.6 Anvil (`AnvilMenu.createResult`)

Exact walkthrough, in the order the source performs it (order matters — several branches mutate shared state that later branches read):

1. **Guard:** proceeds only if the input slot is non-empty and `EnchantmentHelper.canStoreEnchantments(input)` (i.e. the item has an `ENCHANTMENTS` or, for books, `STORED_ENCHANTMENTS` component slot at all).
2. **Prior-work tax:** `tax: i64 = input[REPAIR_COST].unwrap_or(0) + addition[REPAIR_COST].unwrap_or(0)` — the **sum**, not the max, of both items' accumulated repair-cost penalties. `price: i32 = 0` starts separately and is what the branches below accumulate into; `tax` and `price` are combined only at the very end.
3. **Branch A — repair material** (`input.isValidRepairItem(addition)`, e.g. iron ingots on an iron pickaxe): loop while `repairAmount = min(currentDamage, maxDamage/4) > 0` and consumed-count `< addition.count`: apply the repair, `price += 1` per material item actually consumed, recompute `repairAmount` from the new damage. If the very first `repairAmount` is already `≤ 0` (item not damaged enough to need repair), the whole operation aborts (`result = EMPTY`, `cost = 0`) rather than falling through. `repairItemCountCost` (how many material items were actually spent — may be less than the stack, letting a player over-place materials) is stored for `onTake` to shrink the addition slot by exactly that many.
4. **Branch B — combine** (matching item, or an enchanted/regular book as the addition): only taken if not a repair-material match.
   - **Durability merge**, only if `result.isDamageableItem() && !usingBook`: `remaining = (input.maxDamage − input.damage) + (addition.maxDamage − addition.damage) + maxDamage*12/100` (int arithmetic, 12% bonus of max damage, multiply-then-divide truncation), `resultDamage = max(0, maxDamage − remaining)`. Applied — and `price += 2` — **only if** `resultDamage < result.damage` (i.e. only if it's an actual improvement).
   - **Enchantment merge**, iterating `addition`'s `ItemEnchantments.entrySet()` (hazard: fastutil hash order, §7 #1) — for each `(enchantment, additionLevel)`:
     - `current = target's existing level for this enchantment (0 if absent)`
     - `newLevel = additionLevel + 1` **if** `current == additionLevel`, **else** `max(additionLevel, current)` — the classic "same level bumps by one, otherwise take the higher" rule.
     - `compatible = enchantment.canEnchant(input)` (does the enchantment's `supported_items` accept the input's item type at all) — forced `true` regardless if `player.hasInfiniteMaterials()` or `input.is(ENCHANTED_BOOK)`.
     - For every *other* enchantment already accumulated on the target (from the input's original set plus any earlier iterations of this same loop that already succeeded): if it's **not** `Enchantment::areCompatible` with the candidate, `compatible = false` **and `price += 1` — once per conflicting existing enchantment**, not once per candidate.
     - If compatible: clamp `newLevel` to the enchantment's `maxLevel`, write it into the target map, `fee = enchantment.anvilCost` (halved, floor, minimum 1, if the source is a book: `max(1, fee/2)`), `price += fee * newLevel`. If `input.count > 1`, `price` is force-set to a flat `40` (redundantly re-set on every qualifying entry — idempotent, but shows the "no batch-enchanting stacked items" rule is enforced per-entry, not once).
     - If **every** addition enchantment failed compatibility and **none** succeeded, the whole operation aborts (`result = EMPTY`, `cost = 0`).
5. **Renaming**, evaluated after combine/repair: if the player typed a non-blank name different from the input's current hover name → `namingCost = 1`, `price += 1`, apply `CUSTOM_NAME`. Else, if the name box is blank/unset **but the input currently has a custom name** → `namingCost = 1`, `price += 1`, **remove** `CUSTOM_NAME` (clearing an existing rename also costs 1 level).
6. **Finalize price:** `finalPrice = price <= 0 ? 0 : clamp(tax + price, 0, i32::MAX)`. If `price <= 0`, `result` is discarded to `EMPTY` regardless of any durability/enchant work computed above (this only happens if nothing chargeable occurred, i.e. same item with no compatible enchants and no rename).
7. **Pure-rename cap:** `onlyRenaming = (namingCost == price && namingCost > 0)` — true only when the **entire** price is the rename fee (no repair/enchant contribution). If so and `cost >= 40`, clamp the **displayed** cost to `39` — renaming is never blocked by the 40-level cap, only combine/repair operations are.
8. **Too-expensive block:** if `cost >= 40` and the player lacks infinite materials, `result = EMPTY` (item stays visible as too-expensive in some UIs conceptually, but nothing is placed in the output slot; `mayPickup` also independently re-checks `cost > 0 && experienceLevel >= cost`).
9. **Repair-cost propagation**, only if `result` survived: `baseCost = max(input[REPAIR_COST], addition[REPAIR_COST])` (max, not sum — distinct from the `tax` calculation in step 2). **If the operation was anything other than pure-renaming** (`namingCost != price || namingCost == 0`): `baseCost = calculateIncreasedRepairCost(baseCost) = baseCost*2 + 1` (so successive real anvil uses escalate `0→1→3→7→15→31→…`, i.e. `2^n − 1` after `n` uses starting from 0). **A pure rename does not apply this growth** — renaming an item never inflates its future anvil costs, a real and easy-to-miss vanilla quirk.
10. On pickup (`onTake`): deduct `cost` levels (`giveExperienceLevels(-cost)`) unless infinite materials; consume `repairItemCountCost` material items (repair branch) or the whole addition stack (any other successful branch, unless `onlyRenaming`); **and independently**, with the block still an anvil and the player non-infinite-materials, `if (random.nextFloat() < 0.12F)`: damage the anvil block one stage (`AnvilBlock.damage`, may destroy it entirely on the last stage) — this uses `Level.getRandom()`, the ambient (non-reproducible) RNG, and is evaluated **after** the item/cost logic, on pickup, not on result-preview.

### 3.7 Grindstone (`GrindstoneMenu`)

- **Single-item mode** (only one slot filled): if the item has no enchantments at all, result is empty (no-op); otherwise strip every non-curse enchantment (§3.5 curse tag = `binding_curse`, `vanishing_curse`) and recompute `REPAIR_COST` **from scratch** as `2^(remaining curse count) − 1` (i.e. `calculateIncreasedRepairCost` applied that many times starting from `0`) — this is a full reset, unrelated to the item's prior repair-cost history, unlike the anvil's incremental growth.
- **Two-item mode** (both slots filled, both count ≤ 1, same item type): durability merge is `remaining = (input remaining) + (addition remaining) + maxDamage*5/100` (**5%** bonus, vs. the anvil's 12%) against `durability = max(input.maxDamage, addition.maxDamage)`; non-damageable matching items instead stack into `count = 2` if `maxStackSize ≥ 2` and the two stacks are otherwise identical. Enchantments merge via `upgrade` (max-of-levels) for every entry in the *additional* item's enchantments, **except** a curse already present on the target at any level is left untouched (`if (!curse || target has none of it yet) upgrade`) — then the same non-curse strip + repair-cost-reset as the single-item branch applies to the merged result.
- **XP refund**, computed on take (`ResultContainer` slot's `onTake`), independent of the merge branch taken: `amount = Σ over both input items, Σ over each non-curse enchantment: enchantment.getMinCost(level)` (the enchantment's **min-cost linear formula** evaluated at its current level on the item — note this uses `minCost`, not `maxCost`, and both items contribute independently even in single-item mode where the second item is empty). If `amount > 0`: `half = ceil(amount / 2.0)` (double-precision division then ceiling), refund `= half + random.nextInt(half)` — **one** RNG call, using `Level.getRandom()` (ambient, non-reproducible). If `amount == 0`, no orb spawns at all.

### 3.8 XP level curve and point accounting

`Player.getXpNeededForNextLevel()` — the points required to go from the *current* level to the next, piecewise on the *current* level:

| Level range | Formula |
|---|---|
| `< 15` | `7 + level * 2` |
| `15 ≤ level < 30` | `37 + (level − 15) * 5` |
| `≥ 30` | `112 + (level − 30) * 9` |

(Levels 0→1 costs 7, 1→2 costs 9, …, 15→16 costs 37, …, 30→31 costs 112, growing by 2/5/9 per level in the three bands respectively.)

`giveExperiencePoints(n)` (all in `float` for `experienceProgress`, `int` for level/total):
```
score += n
experienceProgress += n as f32 / xpNeededForNextLevel(currentLevel) as f32
totalExperience = clamp(totalExperience + n, 0, i32::MAX)
while experienceProgress < 0.0:                 // only reachable with negative n
    remaining = experienceProgress * xpNeededForNextLevel(currentLevel)
    giveExperienceLevels(-1)                     // mutates currentLevel first
    experienceProgress = if old_level > 0 { 1.0 + remaining / xpNeededForNextLevel(currentLevel) } else { 0.0 }
while experienceProgress >= 1.0:
    experienceProgress = (experienceProgress - 1.0) * xpNeededForNextLevel(currentLevel)   // pre-levelup divisor
    giveExperienceLevels(1)
    experienceProgress = experienceProgress / xpNeededForNextLevel(currentLevel)            // post-levelup divisor
```
Note the loop deliberately re-reads `xpNeededForNextLevel` **after** `giveExperienceLevels` mutates the level in both the down- and up-conversion, so a multi-level gain correctly re-derives the divisor for each newly-entered level rather than using one stale divisor for the whole jump.

`giveExperienceLevels(amount)`: `experienceLevel = saturating_add(experienceLevel, amount)`, clamped to `≥ 0` (with progress/total reset to 0 on underflow). If `amount > 0`, the new level is a multiple of 5, and at least 100 ticks have passed since the last level-up sound, plays `PLAYER_LEVELUP` at volume `min(level/30.0, 1.0) * 0.75`.

`onEnchantmentPerformed` (enchanting-table spend path) calls `experienceLevel -= cost` **directly** — it does not go through `giveExperiencePoints`/`giveExperienceLevels`, so no level-up sound fires for enchant-cost deduction, and the clamp-to-zero on underflow is duplicated inline.

Death XP reward (`Player.getBaseExperienceReward`): `!KEEP_INVENTORY-gamerule && !spectator ? min(experienceLevel * 7, 100) : 0`, additionally passed through `EnchantmentHelper.processMobExperience` (the killer's `MOB_EXPERIENCE` equipment effects — no vanilla enchantment populates this by default, but the hook is live for data packs/mods). Players are `isAlwaysExperienceDropper() = true`, so this always fires on death subject only to `!wasExperienceConsumed()`.

### 3.9 Experience orbs — value tiers, splitting, merging, mending

**Value tier table** (`ExperienceOrb.getExperienceValue`, also mirrored 1:1 by `getIcon` for the sprite breakpoints) — the largest tier `≤` the requested amount is picked:

| Requested ≥ | Orb value |
|---|---|
| 2477 | 2477 |
| 1237 | 1237 |
| 617 | 617 |
| 307 | 307 |
| 149 | 149 |
| 73 | 73 |
| 37 | 37 |
| 17 | 17 |
| 7 | 7 |
| 3 | 3 |
| else | 1 |

**Splitting** (`ExperienceOrb.awardWithDirection`): while `amount > 0`, take `newCount = getExperienceValue(amount)`, subtract it from `amount`, and either merge it into an existing orb or spawn a fresh entity — repeat. A request of, say, 100 becomes `73 + 17 + 7 + 3` as four orbs (or fewer if merges succeed), not a single 100-value orb.

**Merge-on-spawn** (`tryMergeToExisting`, called before every new orb entity is actually created): pick a random "merge group" `id = level.getRandom().nextInt(40)` (constant `ORB_GROUPS_PER_AREA = 40`), search a 1×1×1 box centered on the spawn point for an existing, non-removed orb with the **same value** and `(orb.entityId − id) % 40 == 0`; if found, increment its `count` and reset its `age` instead of spawning — **one RNG call per split chunk**, and the group id is redrawn fresh each time (it is not the new orb's own id, since no entity exists yet).

**Passive merge** (`scanForMerges`, every 20 ticks per orb): merges any other orb within `0.5` blocks whose `(entityId − thisEntityId) % 40 == 0` and same value, taking `age = min(both ages)` — no RNG involved, this is the entity-id-derived "merge lane" grouping that makes orbs from the same original drop event coalesce visually over time.

**Mending consumption** (`ExperienceOrb.playerTouch` → `repairPlayerItems`, evaluated **before** any XP is added to the player's level bar):
```
selected = EnchantmentHelper.getRandomItemWith(REPAIR_WITH_XP, player, |stack| stack.isDamaged())   // uniform pick, 1 RNG call, only among currently-damaged equipped items with an active mending-family effect
if selected.is_none(): return amount unchanged            // -> becomes real player XP
toRepair = EnchantmentHelper.modifyDurabilityToRepairFromXp(level, item, amount)   // runs REPAIR_WITH_XP effect chain; vanilla mending = MultiplyValue(factor = 2.0), i.e. toRepair = amount * 2, floored via the float->int MutableFloat conversion
repair = min(toRepair, item.damageValue)
item.damageValue -= repair
if repair > 0:
    remaining = amount - repair * amount / toRepair        // int arithmetic, multiply-then-divide, truncating
    if remaining > 0: return repairPlayerItems(player, remaining)   // recurse: try another random eligible item with the leftover
return 0
```
Only the value returned by the **top-level** call becomes `player.giveExperiencePoints(remaining)`; every fully-absorbed orb value contributes zero to the level bar. `getRandomItemWith` scans equipment slots (`EquipmentSlot.VALUES` order) collecting every `(item, enchantment)` pair whose enchantment has an active `REPAIR_WITH_XP` effect and matches the predicate, then does a single uniform `random.nextInt(candidates.len())` pick — so with two mending items equipped, which one gets first crack at an orb is a coin flip **per orb touch**, not "always the first slot".

### 3.10 XP sources — exact formulas and RNG call counts

| Source | Formula | RNG calls | RandomSource |
|---|---|---|---|
| Mob kill, base | `Mob.xpReward` field (per-mob constant, e.g. zombie 5) if set, else the below equipment-bonus loop; `0` for mobs with `xpReward == 0` | 0 (base) | — |
| Mob kill, equipment bonus | For each `EquipmentSlot` in `[MAINHAND, OFFHAND, FEET, LEGS, CHEST, HEAD, BODY, SADDLE]` where `slot.canIncreaseExperience()` (all except `SADDLE`): if the slot holds an item **and** `dropChances.byEquipment(slot) ≤ 1.0` (excludes force-100%-drop items), `result += 1 + random.nextInt(3)` (1–3 bonus XP) | 1 per qualifying equipped slot, up to 7 | mob's own `Entity.random` (ephemeral) |
| Mob kill, MOB_EXPERIENCE effect | `EnchantmentHelper.processMobExperience(level, killer, victim, baseAmount)` — applies killer equipment's `MOB_EXPERIENCE` value-effect chain (empty by default in vanilla) | effect-dependent | killer's `Entity.random` |
| Player death | `min(experienceLevel*7, 100)` if not keepInventory/spectator, then through the same `MOB_EXPERIENCE` hook | 0 (base) | — |
| Breeding | Flat `random.nextInt(7) + 1` (1–7) per successful breed, gated by `MOB_DROPS` gamerule | 1 | breeding parent's `Entity.random` |
| Furnace-family "pop XP" (on GUI close / recipe-book bulk take) | Per accumulated `(recipe, craftedCount)` pair: `raw = craftedCount * recipe.experience` (float); `xpReward = floor(raw)`; `frac = frac(raw)`; `if frac != 0.0 && random.nextFloat() < frac: xpReward += 1` | 0 or 1 per recipe batch (only rolled if `frac != 0`) | `level.getRandom()` (ambient) |
| Thrown XP bottle, on impact | `3 + random.nextInt(5) + random.nextInt(5)` (range 3–11, triangular-ish, sum of two independent U[0,4] draws) | 2 | bottle entity's own `Entity.random` |
| Grindstone refund | See §3.7 | 1 (if `amount > 0`) | `level.getRandom()` (ambient) |
| Fishing catch | `random.nextInt(6) + 1` (1–6) per non-junk/non-treasure fish item retrieved, spawned alongside each caught item | 1 per item | the `FishingHook`'s own `Entity.random` |

All XP-orb spawn calls route through `ExperienceOrb.award`/`awardWithDirection` (§3.9), so the raw amount computed above is what gets tier-split into one or more orb entities, not necessarily a single orb.

### 3.11 Numeric primitives used throughout

| Primitive | Definition |
|---|---|
| `Mth.nextInt(random, min, max)` | `min >= max ? min : random.nextInt(max - min + 1) + min` — inclusive both ends, 1 call (0 if degenerate) |
| `Mth.nextFloat(random, min, max)` | `min >= max ? min : random.nextFloat() * (max - min) + min` — 1 call |
| `RandomSource.triangle(mean, spread)` (float or double overload) | `mean + spread * (next() - next())` — **2 calls**, sum of two independent uniforms minus each other (triangular distribution centered on `mean`) |
| `Mth.floor(float\|double v)` | `(int) Math.floor(v)` — true mathematical floor (rounds toward −∞), **not** truncation-toward-zero |
| `Math.round(float v)` (used raw in `selectEnchantment`'s cost perturbation) | Java semantics: `floor(v + 0.5f)` as `int` (round-half-up toward +∞ for the `.5` case), distinct from `Mth.floor` |
| `NumberProvider.getInt(context)` default | `Math.round(this.getFloat(context))` — every loot number provider that doesn't override `getInt` goes through this float→round path, so a `uniform` provider's `getInt()` is **not** the same as calling `Mth.nextInt` directly (it floats first, then rounds, rather than drawing an integer range directly) |

### 3.12 Loot tables — roll evaluation (`LootPool.addRandomItems`)

```
if !compositeCondition.test(context): return          // pool-level conditions, short-circuit AND, see §3.14
rolls = rollsProvider.getInt(context) + Mth.floor(bonusRollsProvider.getFloat(context) * context.getLuck())
for _ in 0..rolls:
    addRandomItem(...)
```
Note the asymmetry: `rolls` uses `getInt` (round-based), `bonusRolls` uses `getFloat` then is explicitly `Mth.floor`'d after multiplying by luck — a *different* rounding rule than the `getInt` default despite both ultimately producing an integer roll count.

`addRandomItem` (one single roll → one entry's output):
```
for each top-level entry container in the pool:
    entry.expand(context, |leaf| {                    // resolves alternatives/sequence/group/tag down to concrete leaves
        w = leaf.getWeight(context.getLuck())          // see §3.13
        if w > 0: validEntries.push(leaf); totalWeight += w
    })
if totalWeight == 0 || validEntries.is_empty(): return    // silently contributes nothing
if validEntries.len() == 1: validEntries[0].createItemStack(...)   // single survivor skips the RNG draw entirely
else:
    index = random.nextInt(totalWeight)                 // 1 call
    for entry in validEntries: index -= entry.getWeight(luck); if index < 0: entry.createItemStack(...); return
```
So a pool with only one entry surviving its conditions consumes **zero** RNG calls for the pick (only whatever its own functions/conditions consume) — this is a common source of off-by-one RNG-count bugs in reimplementations that always call `nextInt` unconditionally.

### 3.13 The luck formula (`LootPoolSingletonContainer.EntryBase.getWeight`)

```
getWeight(luck: f32) -> i32 = max(Mth.floor(weight as f32 + quality as f32 * luck), 0)
```
`weight` and `quality` are entry-level `int`s (JSON `weight`/`quality`, defaults 1/0). `luck` comes from `LootContext.getLuck()` = `LootParams.luck`, set per call site via `.withLuck(...)` — typically `player.getAttributeValue(Attributes.LUCK)` (`Attributes.LUCK`: range `[-1024, 1024]`, default `0.0`, driven by the Luck/Bad Luck potion effects and any equipment/attribute-modifier sources), sometimes summed with a mechanic-local bonus (fishing hook: `hook.luck + owner.getLuck()`, where `hook.luck` is the rod's `LUCK_OF_THE_SEA` enchantment level captured at cast time, §3.15).

This same formula governs the outer `fishing.json` pool's three category weights (junk/treasure/fish — quality `−2`/`+2`/`−1` respectively, §3.15) exactly as it governs any ordinary loot pool entry — it is one universal mechanism, not fishing-specific.

### 3.14 Composite entries and conditions — exact short-circuit order

- **`AlternativesEntry`** (JSON `alternatives`): tries children **in declared order**, first whose own conditions pass "wins" (that child's `expand` returns `true` and no further children are tried); if none pass, contributes nothing. This is how ore-drop tables implement "silk touch → drop the block itself, else → drop the smelted-down item with `apply_bonus`" (§3.16) as a strict first-match, not a weighted choice.
- **`SequentialEntry`** (JSON `sequence`): runs children in order, **stops at the first whose conditions fail** (that failure short-circuits the rest); all children up to that point have already contributed their output.
- **`EntryGroup`** (JSON `group`): runs every child unconditionally (each still gated by its own conditions individually) — no short-circuiting between children.
- **`Util.allOf(conditions)`** (pool-level `conditions`, function-level `conditions`, `all_of` condition type): plain left-to-right short-circuit AND over the list — a `random_chance`-style condition later in the list is **not evaluated** (consumes no RNG call) if an earlier condition in the same list already failed. This makes condition **order** part of the observable RNG-call-count contract for any table mixing a stateful/random condition with a cheap deterministic one.
- **`LootItemConditionalFunction.apply`**: `compositePredicates.test(context) ? run(...) : itemStack` — a function's own `conditions` are evaluated **before** `run()`, so a failed condition on e.g. `apply_bonus` consumes zero RNG calls from that function (§3.16's formulas literally do not execute).

### 3.15 Fishing — loot table, wait-time state machine, open water

**Top-level table** (`minecraft:fishing`, `random_sequence: "minecraft:gameplay/fishing"` — genuinely seed-reproducible per §3.1): one pool, `rolls: 1`, three `loot_table` entries selected by the §3.13 weight+quality/luck formula:

| Entry | weight | quality | Gate |
|---|---|---|---|
| `gameplay/fishing/junk` | 10 | −2 | none |
| `gameplay/fishing/treasure` | 5 | +2 | `entity_properties` on `this` (the hook): `type_specific/fishing_hook.in_open_water == true` |
| `gameplay/fishing/fish` | 85 | −1 | none |

**Casting:** `lureSpeed = (int)(EnchantmentHelper.getFishingTimeReduction(level, rod, player) * 20.0F)` — the `LURE` enchantment's `fishing_time_reduction` effect (`add, linear(base=5, per_level_above_first=5)` → 5/10/15 at levels 1/2/3) is a **seconds** value, multiplied by 20 to get ticks (Lure III → 300 ticks off the wait). `luck = EnchantmentHelper.getFishingLuckBonus(level, rod, player)` — `LUCK_OF_THE_SEA`'s `fishing_luck_bonus` effect (`add, linear(base=1, per_level_above_first=1)` → 1/2/3), clamped `≥ 0`; stored on the hook entity as `this.luck` and reused later as the loot-context luck contribution.

**Wait-time state machine**, driven once per tick while bobbing in water (`catchingFish`), all rolls on the hook's own `Entity.random` unless noted:
1. **Fishing-speed modifier**: `speed = 1`; `if random.nextFloat() < 0.25 && isRainingAt(above): speed += 1`; `if random.nextFloat() < 0.5 && !canSeeSky(above): speed -= 1` — **2 unconditional RNG calls every tick while bobbing**, regardless of outcome.
2. **Phase `timeUntilLured`** (initial wait, no bite yet): starts as `Mth.nextInt(random, 100, 600) - lureSpeed` (1 RNG call, then a flat subtraction — Lure reduces the *ceiling* of the wait roll, not a percentage). Each tick, `timeUntilLured -= speed`; a "tease" splash chance escalates as the timer runs down (`teaseChance = 0.15`, `+0.05` per remaining tick under 20, `+0.02` per remaining tick between 20–40, `+0.01` per remaining tick between 40–60 — evaluated once via the matching band, not summed across bands) with `random.nextFloat() < teaseChance` gating a cosmetic splash particle (2 more RNG calls for its position when it fires). When the timer expires: `fishAngle = Mth.nextFloat(random, 0, 360)`, `timeUntilHooked = Mth.nextInt(random, 20, 80)`.
3. **Phase `timeUntilHooked`** (fish approaching, biting imminent): `timeUntilHooked -= speed` each tick; while positive, `fishAngle += random.triangle(0.0, 9.188)` (2 calls) drives cosmetic bubble-trail particles. On expiry: `nibble = Mth.nextInt(random, 20, 40)`, `DATA_BITING = true` (visual bob).
4. **Phase `nibble`** (bite window — the player must reel in now): decrements by 1 each tick regardless of `speed`; on reaching 0, both timers reset to 0 and biting clears — **if the player does not reel in during this window, the fish is lost and the whole cycle restarts from `timeUntilLured`.**

**`retrieve()`** (player reels in): if `nibble > 0` (a fish is actively biting), builds a `LootParams` for `LootContextParamSets.FISHING` with `TOOL = rod`, `THIS_ENTITY = hook`, `withLuck(hook.luck + owner.getLuck())`, resolves it against the persistent `random_sequence`, and calls `LootTable.getRandomItems` (full §3.12 machinery). For each resulting `ItemStack`: spawns an `ItemEntity` tossed toward the player, **and** spawns one `ExperienceOrb` worth `random.nextInt(6) + 1` (1–6, hook's own RNG, one orb per item, not per catch) alongside it, and awards `Stats.FISH_CAUGHT` if the item is tagged `#minecraft:fishes`.

**Open-water rule** (`calculateOpenWater`, gates the treasure pool entry): scans four horizontal 5×5 layers at `y ∈ {−1, 0, 1, 2}` relative to the hook. Each layer is classified `ABOVE_WATER` (every cell is air or a lily pad), `INSIDE_WATER` (every cell is a source-block water fluid with an empty collision shape), or `INVALID` (mixed, or any non-air/non-water solid present) — a layer with **any** disagreement between cells collapses straight to `INVALID`. Scanning bottom-to-top, the sequence of layer classes must never go `INVALID` outright, never transition `ABOVE_WATER → …` as the very first layer without having started `INVALID`-free, and never transition `INSIDE_WATER` layer directly beneath an `ABOVE_WATER` layer (i.e. air must not sit directly on top of the water column near the hook) — any violation sets `openWater = false` for that check. `openWater` only re-arms to `true` once `nibble ≤ 0 && timeUntilHooked ≤ 0` (freshly waiting), otherwise it stays latched from the last check while `outOfWaterTime < 10`.

### 3.16 `apply_bonus` — the three fortune/looting formulas

JSON shape (from `coal_ore.json`): `{"function": "minecraft:apply_bonus", "enchantment": "minecraft:fortune", "formula": "minecraft:ore_drops"}`. `ApplyBonusCount.run` reads `level = EnchantmentHelper.getItemEnchantmentLevel(enchantment, tool)` from the `LootContextParams.TOOL` param (returns 0 if the tool has none of the referenced enchantment or no tool is present) and dispatches to the formula:

**`ore_drops`** (used by every vanilla ore block):
```
if level == 0: return count            // no RNG call at all
bonus = random.nextInt(level + 2) - 1   // 1 call; raw range [-1, level]
if bonus < 0: bonus = 0                 // clamps the -1 outcome
return count * (bonus + 1)
```
Note the asymmetric distribution this produces: raw draws of both `-1` and `0` (each probability `1/(level+2)`) collapse to `bonus = 0`, so the "no bonus" outcome has **double** the probability of any single positive bonus value `1..level` — this is the well-known Fortune "no bonus is twice as likely as any other single outcome" quirk, confirmed directly in source.

**`uniform_bonus_count`** (used by e.g. Looting on many mob-drop tables, `bonusMultiplier` typically the enchantment's own per-level scaling written into the JSON):
```
return count + random.nextInt(bonusMultiplier * level + 1)   // 1 call, ALWAYS — even at level 0, nextInt(1) == 0 but still consumes a draw
```

**`binomial_with_bonus_count`** (used by e.g. Fortune on crops/seeds-style drops):
```
for i in 0..(level + extraRounds):
    if random.nextFloat() < probability: count += 1
return count
```
RNG calls = exactly `level + extraRounds`, one `nextFloat()` per trial — **zero** calls if `level + extraRounds == 0`.

All three read the enchantment level via `LootContextParams.TOOL`, meaning **Fortune/Silk Touch have no entry at all in the `EnchantmentEffectComponents` system** — checked directly against `fortune.json`/`silk_touch.json`: fortune has no `effects` block whatsoever; silk touch's only effect is `block_experience: {type: set, value: 0.0}` (this is why silk-touch-mined ores give zero XP — nothing else). The actual "drop raw block instead" behavior for silk touch lives entirely in loot-table JSON via `alternatives` + a `match_tool` condition checking `enchantments: [{enchantments: "minecraft:silk_touch", levels: {min: 1}}]` on the tool (§3.14's `AlternativesEntry`, first-match order: silk-touch branch listed first).

### 3.17 Other loot conditions with RNG (exact formulas)

- **`random_chance`**: `random.nextFloat() < chance.getFloat(context)` — `chance` is a full `NumberProvider`, not necessarily a constant.
- **`random_chance_with_enchanted_bonus`**: reads the enchantment level off `LootContextParams.ATTACKING_ENTITY` (0 if absent or not a `LivingEntity`); `chance = level > 0 ? enchantedChance.calculate(level) : unenchantedChance` (a `LevelBasedValue`, typically `Linear`); then `random.nextFloat() < chance`. This is the mechanism behind Looting-boosted rare drops (e.g. rare mob-head/item chances) — distinct from the `equipment_drops` effect-component path used for ordinary item-count Looting bonuses.
- **`table_bonus`** (`BonusLevelTableCondition`): a flat `List<Float>` of chances indexed by enchant level, clamped to the list's last entry for levels beyond its length: `chance = values[min(level, values.len()-1)]`; then `random.nextFloat() < chance`.
- **`survives_explosion`**: `1/explosionRadius` chance (`random.nextFloat() <= probability`, note `<=` here vs. `<` everywhere else), only applied when `LootContextParams.EXPLOSION_RADIUS` is present in the context — silently always-true (no RNG call) otherwise.

### 3.18 The enchantment-effect value DSL (`EnchantmentValueEffect`)

Every `minX_cost`-adjacent numeric knob that isn't Fortune/Silk-Touch-style loot-table math instead flows through this five-verb DSL, each wrapping a `LevelBasedValue` curve (§3.19) and a `RandomSource` it may or may not consume:

| Effect | `process(level, random, input) -> f32` | RNG |
|---|---|---|
| `add` (`AddValue`) | `input + curve.calculate(level)` | none |
| `multiply` (`MultiplyValue`) | `input * curve.calculate(level)` | none |
| `set` (`SetValue`) | `curve.calculate(level)` (ignores `input` entirely) | none |
| `scale_exponentially` (`ScaleExponentially`) | `input * base.calculate(level).powf(exponent.calculate(level))` (computed in `f64` via `Math.pow`, cast back to `f32`) | none |
| `remove_binomial` (`RemoveBinomial`) | see below | **conditional**, 0–1 or 0–n calls |

`remove_binomial(level, random, n)` — an exact binomial-count remover with a normal-approximation fast path:
```
p = chance_curve.calculate(level)
if n > 128.0 AND n*p >= 20.0 AND n*(1-p) >= 20.0:
    mu = floor(n * p)                                  // f64
    sigma = sqrt(n * p * (1.0 - p))                    // f64
    drop = round(mu + random.nextGaussian() * sigma)    // 1 RNG call (Gaussian)
    drop = clamp(drop, 0, n as i32)
else:
    drop = 0
    for _ in 0..(n as i32):
        if random.nextFloat() < p: drop += 1            // up to n RNG calls (Bernoulli loop)
return n - drop
```
This is a De Moivre–Laplace normal approximation to the exact binomial, switched in only for large `n` with both `np` and `n(1-p)` comfortably away from the tails — **the RNG call count for this single effect varies by orders of magnitude (1 vs. up to `n`) depending purely on the runtime value of `n`**, which is exactly the kind of branch a naive port silently gets wrong by always taking one path.

### 3.19 `LevelBasedValue` curve variants (exact `calculate(level) -> f32`)

| Variant | Formula |
|---|---|
| `constant(v)` | `v` |
| `linear(base, per_level_above_first)` | `base + per_level_above_first * (level - 1)` |
| `clamped(value, min, max)` | `clamp(value.calculate(level), min, max)` |
| `fraction(numerator, denominator)` | `denominator.calculate(level) == 0.0 ? 0.0 : numerator.calculate(level) / denominator.calculate(level)` |
| `levels_squared(added)` | `level² + added` (`level` cast to float before squaring) |
| `exponent(base, power)` | `base.calculate(level).powf(power.calculate(level))` (`f64` `Math.pow`, cast to `f32`) |
| `lookup(values, fallback)` | `level <= values.len() ? values[level - 1] : fallback.calculate(level)` |

Distinct from `Enchantment.Cost(base, per_level_above_first)` (`i32`, `calculate(level) = base + per_level_above_first*(level-1)`) — same shape, different type (int vs. float) and different registry (used only for `min_cost`/`max_cost`/anvil-adjacent, never inside `effects`).

### 3.20 Vault and trial-spawner rewards (brief — reuse, no new math)

`VaultBlockEntity` builds an ordinary `LootParams` for `LootContextParamSets.VAULT` (`ORIGIN`, `THIS_ENTITY = player`, `TOOL = inserted key item`, `.withLuck(player.getLuck())`) against the vault's configured `lootTable` and calls the standard `LootTable.getRandomItems` — everything in §3.12–§3.14 applies verbatim; there is no vault-specific probability math beyond the ordinary weight+quality/luck formula and whatever functions/conditions the specific reward table's JSON declares. The cosmetic "cycling display item" preview additionally picks one uniformly-random item from the rolled result list (`Util.getRandom`, 1 call) purely for the client-facing spinning icon — it does not affect the actual reward.

Trial-spawner rewards select **one** loot table id from a `WeightedList<ResourceKey<LootTable>>` (`TrialSpawnerConfig.lootTablesToEject`, a generic weighted-list utility distinct from `LootPool` but using the same cumulative-weight draw shape as `WeightedRandom`, §3.4), then roll that selected table through the same standard machinery. No additional formula is introduced.

## 4. Constants table (consolidated)

| Constant | Value | Source |
|---|---|---|
| Enchanting table bookshelf candidate offsets | 32 (16-cell ring × 2 y-layers) | `EnchantingTableBlock.BOOKSHELF_OFFSETS` |
| Bookshelf count cap (cost formula only) | `min(bookcases, 15)` | `EnchantmentHelper.getEnchantmentCost` |
| Enchantment power provider tag (vanilla content) | `minecraft:bookshelf` only | `tags/block/enchantment_power_provider.json` |
| Enchantment power transmitter tag (vanilla content) | `#minecraft:replaceable` | `tags/block/enchantment_power_transmitter.json` |
| Table per-slot base roll | `nextInt(8)+1+(bookcases>>1)+nextInt(bookcases+1)` | `EnchantmentHelper.getEnchantmentCost` |
| Table per-slot divisor | slot0 `/3` min1; slot1 `*2/3+1`; slot2 `max(x,bookcases*2)` | same |
| Slot zero-out threshold | `cost < slotIndex+1 → 0` | `EnchantmentMenu.slotsChanged` |
| `selectEnchantment` cost bump | `+1 + nextInt(ench/4+1) + nextInt(ench/4+1)` | `EnchantmentHelper.selectEnchantment` |
| `selectEnchantment` triangular span | `(nextFloat()+nextFloat()-1)*0.15`, clamp round to `[1, MAX]` | same |
| Extra-enchant continue chance | `nextInt(50) <= cost`, `cost /= 2` per iteration | same |
| `in_enchanting_table` = | `#non_treasure` (all minus mending/frost_walker/soul_speed/swift_sneak/wind_burst/2 curses) | `tags/enchantment/in_enchanting_table.json`, `treasure.json` |
| Enchantment weight range | 1–1024 | `EnchantmentDefinition` codec |
| Enchantment max level (hard ceiling) | 255 | `Enchantment.MAX_LEVEL` |
| Anvil rename cost | flat 1 level | `AnvilMenu.COST_RENAME` |
| Anvil incompatible penalty | +1 level **per conflicting existing enchant** | `AnvilMenu.createResult` |
| Anvil durability-merge bonus | `maxDamage*12/100`, `price += 2` if applied | `AnvilMenu.createResult` |
| Anvil batch-stack price | flat 40 | same |
| Anvil too-expensive cap | 40 (blocks non-rename ops); pure rename capped display at 39 | same |
| Anvil repair-cost growth | `base*2+1` (skipped for pure renames) | `AnvilMenu.calculateIncreasedRepairCost` |
| Anvil block-damage chance on pickup | `nextFloat() < 0.12` | `AnvilMenu.onTake` |
| Grindstone durability-merge bonus | `maxDamage*5/100` | `GrindstoneMenu.mergeItems` |
| Grindstone repair-cost reset | `2^(curses remaining) − 1` (full reset, not incremental) | `GrindstoneMenu.removeNonCursesFrom` |
| Grindstone XP refund | `half=ceil(amount/2.0)`, `half + nextInt(half)` | `GrindstoneMenu` result slot |
| XP curve `<15` | `7 + level*2` | `Player.getXpNeededForNextLevel` |
| XP curve `15..30` | `37 + (level-15)*5` | same |
| XP curve `≥30` | `112 + (level-30)*9` | same |
| Death XP reward | `min(experienceLevel*7, 100)`, 0 if keepInventory/spectator | `Player.getBaseExperienceReward` |
| Orb value tiers | 1,3,7,17,37,73,149,307,617,1237,2477 | `ExperienceOrb.getExperienceValue` |
| Orb merge group count | 40 (`ORB_GROUPS_PER_AREA`) | `ExperienceOrb` |
| Orb merge distance (passive) | 0.5 blocks, every 20 ticks | `ExperienceOrb` |
| Orb lifetime | 6000 ticks | `ExperienceOrb.LIFETIME` |
| Mending ratio | `×2.0` durability per XP point | `mending.json` effect |
| Breeding XP | `nextInt(7)+1` (1–7) | `Animal.java` |
| Furnace pop-XP rounding | `floor(raw)`, `+1` w.p. `frac(raw)` | `RecipeBookMenu`/`Recipe` XP pop path |
| XP bottle range | `3 + nextInt(5) + nextInt(5)` (3–11) | `ThrownExperienceBottle` |
| Fishing catch orb | `nextInt(6)+1` (1–6) per item | `FishingHook.retrieve` |
| Fishing table weights | junk 10/−2, treasure 5/+2, fish 85/−1 | `fishing.json` |
| Lure tick reduction | `fishing_time_reduction × 20` ticks off `[100,600]` roll | `FishingRodItem`, `lure.json` |
| Luck of the Sea bonus | `+1` per level, added to `owner.getLuck()` | `luck_of_the_sea.json` |
| `timeUntilHooked` roll | `nextInt(20,80)` | `FishingHook.catchingFish` |
| `nibble` (bite window) roll | `nextInt(20,40)` | same |
| Weight+quality/luck formula | `max(floor(weight + quality*luck), 0)` | `LootPoolSingletonContainer.EntryBase.getWeight` |
| `ore_drops` | `level==0 ? count : count*(clamp(nextInt(level+2)-1,0,∞)+1)` | `ApplyBonusCount.OreDrops` |
| `uniform_bonus_count` | `count + nextInt(bonusMultiplier*level+1)` | `ApplyBonusCount.UniformBonusCount` |
| `binomial_with_bonus_count` | `level+extraRounds` Bernoulli(p) trials | `ApplyBonusCount.BinomialWithBonusCount` |
| `random_sequence` seed derivation | `(worldSeed XOR salt) XOR seedFromHashOf(id)`, Xoroshiro128++ | `RandomSequence`/`RandomSequences` |
| Ambient RNG seed | `SEED_UNIQUIFIER * 1181783497276652981L XOR nanoTime()` | `RandomSupport.generateUniqueSeed` |
| `LUCK` attribute | default 0.0, range [−1024, 1024] | `Attributes.LUCK` |

## 5. RNG usage map

| Mechanic | Source | Calls | Order-sensitive? |
|---|---|---|---|
| Table 3 displayed costs | table's own `RandomSource`, seeded once from `enchantmentSeed` | 6 (2×3 slots, sequential, not reset between slots) | Yes — slot 0 must be evaluated before slot 1 before slot 2 on the *same* stream |
| Table slot resolve (clue **and** commit) | reseeded per call from `enchantmentSeed + slot` | 2 (cost bump) + 2 (span) + 1 (first pick) + 1 per extra-enchant iteration + 1 more if a Book collapses multi-pick | Yes — the whole sequence per §3.4; identical seed ⇒ identical outcome across the clue/commit re-evaluations |
| Anvil pickup block-damage | `level.getRandom()` (ambient) | 0 or 1, only if not infinite-materials | No formula depends on order, but happens strictly after cost/XP deduction |
| Grindstone refund | `level.getRandom()` (ambient) | 0 or 1 | — |
| Breeding | parent `Entity.random` | 1 | — |
| XP bottle | bottle `Entity.random` | 2, both consumed unconditionally | Order fixed but both draws are symmetric (sum) |
| Fishing (per bobbing tick) | hook `Entity.random` (mostly), `syncronizedRandom` (visual jitter only, reseeded every tick from `uuidBits XOR gameTime`) | 2 unconditional (speed modifier) + phase-dependent (0–5+) | Yes — phase order (`lured→hooked→nibble`) is a strict state machine, and the speed-modifier draws happen every tick regardless of phase |
| Fishing catch loot roll | `random_sequence("minecraft:gameplay/fishing")` (seed-reproducible) | 1 top-level pool pick + whatever the resolved sub-table needs | Yes, deterministic given world seed + player luck |
| Ordinary block/mob loot table (no `random_sequence`) | `level.getRandom()` (ambient) | rolls × (pick + function/condition RNG) | Order-sensitive for interleaving, not reproducible in isolation |
| `apply_bonus` (`ore_drops`) | context random (whichever the table resolved) | 0 if level==0, else 1 | — |
| `apply_bonus` (`uniform_bonus_count`) | same | always 1, even at level 0 | Easy miscount: level 0 still draws |
| `apply_bonus` (`binomial_with_bonus_count`) | same | exactly `level+extraRounds` | Zero if that sum is 0 |
| `remove_binomial` effect | effect's context random | 1 (Gaussian) or up to `n` (Bernoulli loop) | Branch chosen by `n`,`p` at runtime — not a fixed count |
| Loot pool entry pick | context random | 0 if 0 or 1 survivors, else 1 | Survivor-count-dependent — a common miscount source |
| Mending repair | mending-eligible-item pick: `player`'s ambient random (via `EnchantmentHelper.getRandomItemWith` → `Util.getRandomSafe`) | 1 per recursion level (one per orb-touch, possibly recursing) | Which item gets first crack is random per touch |

## 6. Cross-references

- `docs/research/mc-26.2/10-items-recipes-loot.md` §3.6/§3.8/§7 — broad shape of enchantments-as-data and the loot engine; this document's §3.2–§3.6, §3.12–§3.19 are the verified-exact successor to those summarized formulas. §7 of doc 10 already flags loot/enchant/trade determinism hinging on `random_sequence` — §3.1/§3.12 here is the full mechanical backing for that claim.
- `docs/research/mc-26.2/11-player-gameplay.md` line 285–286 — XP curve and death-reward formula, confirmed byte-identical against source in §3.8 here.
- `docs/research/mc-26.2/05-worldgen.md` §RNG — owns the two `RandomSource` algorithm internals (LCG vs. Xoroshiro128++, seed-upgrade constants); §3.1 here only states *which instance* backs each loot/enchant call site, not the algorithms themselves.
- `docs/planning/05-game-mechanics.md` (MECH-) — should own the enchantment-effect dispatch order during combat (which `EnchantmentHelper.modifyX` hooks fire in what sequence relative to base damage calc) as a combat-pipeline decision; this document supplies the individual effect-verb math (§3.18) that combat math composes.
- `docs/planning/09-testing-quality.md` (TEST-) — the `random_sequence`-tagged mechanics in §3.1/§3.12/§3.15 are the natural golden-fixture surface for loot/fishing differential tests (seed-reproducible); ordinary ambient-RNG mechanics (§3.6 anvil, §3.7 grindstone, most mob drops) are not fixture-able the same way and should be tested for *distribution* shape and RNG call-count parity instead, not exact sequences.
- `docs/planning/12-workspace-structure.md` (WS-) — the `ItemEnchantments` fastutil hash-order hazard (§7 #1) affects whichever crate owns the ECS item-component representation; worth flagging there as a design input, not just a testing footnote.

## 7. Reimplementation hazards, ranked

1. **`ItemEnchantments`'s iteration order is JVM identity-hashcode order, not insertion order, and vanilla itself does not guarantee it.** `Object2IntOpenHashMap<Holder<Enchantment>>` is a fastutil open-addressing hash table; `Holder.Reference` overrides neither `equals` nor `hashCode`, so bucket placement is driven by `System.identityHashCode`-derived values that are stable only within one JVM process, not reproducible across runs or portable to Rust at all. This directly affects: (a) `AnvilMenu.createResult`'s per-entry enchantment-merge loop — when a single anvil operation adds two *mutually incompatible* enchantments to a target that has neither yet (e.g. a multi-enchant book from a command), which one "wins" the compatibility check depends on this hash order; (b) every `EnchantmentHelper.runIterationOnItem`/`runIterationOnEquipment` dispatch (damage, knockback, durability, all per-hit effect processing) when an item somehow carries multiple enchantments touching the same effect slot. **There is no "correct" order to reverse-engineer here** — pick a stable, documented Rust ordering (e.g. enchantment registration order, or sorted by resource id) and record it as a deliberate `MECH-D` decision rather than chasing unreproducible Java internals.
2. **RNG call counts are conditional on runtime values in more places than they look**, and every skipped call shifts every subsequent draw for the rest of the tick: `uniform_bonus_count` always draws even at level 0; `ore_drops`/`binomial_with_bonus_count` draw zero times at level 0; a loot pool with exactly one surviving entry skips its pick draw entirely; `remove_binomial` draws either 1 (Gaussian) or up to `n` (Bernoulli loop) depending on a three-way threshold on `n` and `p`; a `LootItemConditionalFunction`'s `run()` (and all its RNG) is skipped entirely if any of its own `conditions` fail, and `AllOf`/`Util.allOf` short-circuits left-to-right. A naive port that "always calls the RNG helper" for these will desync call order even when it gets individual-formula math right.
3. **Two independent per-player/per-table reseed points inside the enchanting table**, easy to collapse into one: the three *displayed costs* share one seeded stream across all three slots sequentially (§3.3), while *resolving a slot's actual enchantment* reseeds independently per slot from `enchantmentSeed + slotIndex` (§3.4) — and that per-slot resolve runs **twice** (once for the cosmetic clue, once again identically on commit) plus a **third**, non-reseeded draw immediately after for the clue-only "which icon to show" pick that must not be confused with the real applied result. Mixing up which of these three draws is "the real one" silently changes what gets applied vs. merely displayed.
4. **Two different RNG algorithms and two different reproducibility guarantees coexist under one `RandomSource` interface**, and the domain in this document straddles both: enchanting-table/anvil/grindstone/most-mob-loot/XP-orb-scatter all use the ephemeral, `nanoTime()`-seeded ambient RNG (never world-seed-reproducible, by design), while `random_sequence`-tagged loot tables (fishing, structure chests, vaults) use the persistent, Xoroshiro128++-backed, world-seed-derived sequence store. A blanket "make everything replayable from the world seed" implementation would be **more deterministic than vanilla itself** for the ambient-RNG mechanics — which sounds harmless but breaks the "bit-identical by default" contract in the other direction (a Rusty Clanker server would produce different, but *more* reproducible, outcomes than a real vanilla server given the same seed and inputs, which is itself a parity deviation worth an explicit `ARCH-D`/`TEST-D` exception if intentional).
5. **Float/double/int boundary crossings are numerous and each has a specific rounding rule that differs from its neighbors**: `selectEnchantment`'s cost perturbation uses `Math.round(float)` (floor-of-`x+0.5`, round-half-up); the loot luck formula and `bonus_rolls` scaling use `Mth.floor` (true floor, rounds toward −∞, **not** the same as `Math.round`); `NumberProvider.getInt`'s default is `Math.round(getFloat(...))`, meaning a `uniform` provider's integer path is a round of a uniform float draw, not a direct integer-range draw; the anvil's durability-bonus and grindstone's differ only in a `12%` vs `5%` integer-truncating multiply-then-divide. Porting any of these with the wrong one of {truncate-toward-zero, floor, round-half-up} produces off-by-one results that only show up at specific unlucky input values, not in casual testing.
6. **Anvil's `tax` (sum of both items' prior `REPAIR_COST`) and the final repair-cost write-back (`max` of both items' `REPAIR_COST`, then conditionally `*2+1`) use different combining operators on the same two source values**, and the growth step is explicitly **skipped** for pure-rename operations while the `tax` contribution to displayed cost is **not** skipped for them (a pure rename on a heavily-repaired item still shows a nonzero cost from `tax`, just capped at 39, and does not itself add further prior-work penalty). A reimplementation that unifies these into one "repair cost" concept will get renaming-cost economics wrong in exactly the way that's easy to miss in a one-anvil-use test.
7. **`RemoveBinomial`'s Gaussian-approximation branch consumes `nextGaussian()`, a call vanilla's other RNG-consuming loot/enchant math never uses** (everywhere else in this domain draws exclusively `nextInt`/`nextFloat`/`triangle`). A `RandomSource` reimplementation that doesn't faithfully port the Box–Muller-or-equivalent Gaussian algorithm bit-for-bit (owned by the worldgen RNG doc, but consumed here) will diverge silently for any large-`n` `remove_binomial` effect, and the branch threshold itself (`n>128 && n*p≥20 && n*(1-p)≥20`) must be replicated exactly or the two code paths (which consume wildly different RNG call counts) get selected differently between vanilla and Rusty Clanker even when the final numeric distributions are statistically similar.
