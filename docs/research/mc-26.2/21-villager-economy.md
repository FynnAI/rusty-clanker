# Villager Economy, Gossip & Trade Math (26.2)

## 1. Purpose

Villager trading, reputation and golem/zombie interactions are one of the few subsystems where the *observable* player experience is a deterministic function of a large pile of small integer/float formulas, several independent random-number streams, and exact call ordering. Get the demand formula's operand order wrong, use `f64` where vanilla uses `f32`, or call the RNG once instead of twice, and prices, offer rolls, gossip decay, or golem-spawn timing silently diverge from a real server while still "looking right" in casual play — the worst kind of parity bug, because it only surfaces as reports of "my villager prices feel off" or "seed X doesn't match" months later.

26.2 additionally moved the entire trade-content catalog (which items, which quantities, which discounts) out of hardcoded Java and into two registries (`trade_set`, `villager_trade`) resolved through a generic loot-context/`NumberProvider` machinery shared with loot tables and enchanting. The **math and RNG plumbing that resolves that data into a live offer is now common code that must be bit-exact**, independent of the JSON content itself (which Rusty Clanker will port as data, per `ASSET-D18`/`ASSET-D19`, not reproduce from the Java generator classes — see §7 and cross-references). This document is the exact specification of that plumbing, plus every other villager-economy mechanic that is formula- or RNG-shaped: gossip decay/transfer, restock scheduling, XP/leveling, breeding, wandering-trader spawn odds, iron-golem spawn conditions, and the Hero-of-the-Village discount chain.

## 2. Where it lives

| Package / class | Role |
|---|---|
| `net.minecraft.world.item.trading.MerchantOffer` | Live, stateful single trade instance: cost items, uses, demand, special-price diff, price multiplier, XP. Owns the price-clamp math. |
| `net.minecraft.world.item.trading.MerchantOffers` | `ArrayList<MerchantOffer>`; offer lookup (`getRecipeFor`) by satisfied-cost match, honoring a client "selection hint" slot. |
| `net.minecraft.world.item.trading.ItemCost` / `TradeCost` | `ItemCost` = resolved runtime cost (item + fixed count + component predicate). `TradeCost` = the JSON-facing template (item + `NumberProvider` count + predicate); `TradeCost.toItemCost(lootContext, additionalCost)` resolves one into the other, clamped to `[0, item.maxStackSize]`. |
| `net.minecraft.world.item.trading.VillagerTrade` | One JSON-defined trade template (`wants`, `additional_wants`, `gives`, `max_uses`, `reputation_discount`, `xp`, `merchant_predicate`, `given_item_modifiers`, `double_trade_price_enchantments`). `getOffer(LootContext)` resolves a template to a live `MerchantOffer` or `null`. |
| `net.minecraft.world.item.trading.TradeSet` | Registry `minecraft:trade_set`: a `HolderSet<VillagerTrade>` pool (`trades`, usually a tag), a `NumberProvider amount` (how many offers to roll), `allow_duplicates`, an optional `random_sequence` id. |
| `net.minecraft.world.item.trading.TradeSets` | Bootstrap wiring: registers one `TradeSet` per profession × level (and 3 wandering-trader buckets), with default `amount = 2.0` unless overridden. |
| `net.minecraft.world.item.trading.Merchant` | Interface implemented by `AbstractVillager`, `ClientSideMerchant`; owns `notifyTrade`/`notifyTradeUpdated`/`canRestock` defaults. |
| `net.minecraft.world.item.trading.VillagerTrades` / `TradeRebalanceVillagerTrades` | **Datagen-only** Java sources that were exported once into the shipped `data/minecraft/trade_set|villager_trade/**` JSON (`VillagerTrades::bootstrap` is wired into `VanillaRegistries`; `TradeRebalanceVillagerTrades::bootstrap` into a separate, non-default `TradeRebalanceRegistries` set). **Not executed at runtime** — confirmed by grep: no call site invokes either `bootstrap` method outside the two `*Registries` builder classes, and the two Java sources' hardcoded numbers *disagree* with the actual shipped JSON in places (e.g. `TradeRebalanceVillagerTrades`'s armorer-boots trade uses `reputationDiscount = 0.05F`; the shipped `villager_trade/armorer/1/emerald_iron_boots.json` uses `0.2`). Ground truth is the JSON under `datagen/generated/data/minecraft/{trade_set,villager_trade}/**`, exactly as broad doc 10 §3.9/§5 already flags. |
| `net.minecraft.world.entity.npc.villager.{Villager,AbstractVillager,VillagerData,VillagerDataHolder,VillagerProfession,VillagerType}` | Entity-side state machine: offer generation trigger, restock scheduling, XP/leveling, special-price application, gossip storage, breeding, POI/profession acquisition. |
| `net.minecraft.world.entity.npc.wanderingtrader.{WanderingTrader,WanderingTraderSpawner}` | Wandering trader entity + world-tick spawner (spawn-chance ladder, position search, llama escort). |
| `net.minecraft.world.entity.ai.gossip.{GossipContainer,GossipType}` | Per-villager reputation ledger: 5 weighted, capped, decaying categories per (target UUID) entry; weighted transfer between villagers. |
| `net.minecraft.world.entity.ai.village.ReputationEventType` | The 5 (4 live + 1 dead) reputation-event tags dispatched via `ServerLevel.onReputationEvent`. |
| `net.minecraft.world.entity.monster.zombie.ZombieVillager` | Cure timer, conversion-speed roll, `ZOMBIE_VILLAGER_CURED` firing. |
| `net.minecraft.world.entity.ai.behavior.{AcquirePoi,AssignProfessionFromJobSite,ResetProfession,YieldJobSite,WorkAtPoi,TradeWithVillager,VillagerMakeLove,VillagerPanicTrigger,GiveGiftToHero}` | Brain behaviors driving job-site claim, profession assignment/loss, restock trigger, inter-villager gossip babble, breeding, panic-triggered golem spawning, Hero-of-the-Village gifting. |
| `net.minecraft.world.entity.ai.sensing.GolemSensor` | Marks `GOLEM_DETECTED_RECENTLY` memory (599-tick TTL) when an iron golem is in the `NEAREST_LIVING_ENTITIES` memory. |
| `net.minecraft.world.entity.raid.Raid` | Grants `HERO_OF_THE_VILLAGE` on raid victory; `raidOmenLevel` → amplifier. |
| `net.minecraft.world.RandomSequences` / `RandomSequence` | World-persisted, identifier-keyed **Xoroshiro** RNG streams — the actual source consumed for trade-offer rolling (see §5). |
| `net.minecraft.world.inventory.{MerchantMenu,MerchantContainer,MerchantResultSlot}` | Trade-commit call order: cost consumption → `notifyTrade` → stat/criteria → UI refresh. |

## 3. The mechanics

### 3.1 Profession → level → `TradeSet` wiring

`VillagerProfession` is a record carrying `Int2ObjectMap<ResourceKey<TradeSet>> tradeSetsByLevel`, one entry per level 1–5 (e.g. `armorer` → `armorer/level_1` … `armorer/level_5`). `VillagerProfession.getTrades(int level)` is a direct map lookup, `null` if absent (e.g. `NONE`, `NITWIT` never populate the map — nitwits and unemployed villagers have no offers at all).

`Villager.updateTrades(ServerLevel)`:
```
tradeKey = profession.getTrades(villagerData.level())
if tradeKey != null:
    addOffersFromTradeSet(level, this.offers, tradeKey)
```
called from `AbstractVillager.getOffers()` lazily on first access (offer list starts `null`) and again from `Villager.increaseMerchantCareer()` on every level-up (appends the new level's offers to the *existing* `MerchantOffers` list — old-level offers are never removed, they just stop restocking new uses once max level content grows the list). `WanderingTrader.updateTrades` instead calls `addOffersFromTradeSet` three times unconditionally against three fixed keys: `WANDERING_TRADER_BUYING`, then `WANDERING_TRADER_UNCOMMON`, then `WANDERING_TRADER_COMMON` (order matters only for offer-list index/UI order, not for RNG independence — each call gets its own `LootContext`).

**Default trade count per `TradeSet`:** `amount = ConstantValue.exactly(2.0F)` for every profession level 1–5, with two shipped exceptions: `librarian/level_5` = `3.0F`, `wandering_trader/common` = `5.0F`. `wandering_trader/buying` and `wandering_trader/uncommon` use the same 2.0F default. Confirmed against the actual `trade_set/**/*.json`:

| `TradeSet` | `amount` | `allow_duplicates` |
|---|---|---|
| every `<profession>/level_<1..5>` except librarian level 5 | 2 | false |
| `librarian/level_5` | 3 | false |
| `wandering_trader/buying` | 2 | false |
| `wandering_trader/uncommon` | 2 | false |
| `wandering_trader/common` | 5 | false |

`allow_duplicates` is `false` for every shipped `TradeSet` (no JSON overrides it and `TradeSets.register` never passes `true`), so offer resolution always uses the **without-duplicates** algorithm below.

### 3.2 Offer-roll algorithm (`AbstractVillager.addOffersFromItemListingsWithoutDuplicates`)

Given a `HolderSet<VillagerTrade>` pool of size *N* and a target count *k* (`= TradeSet.calculateNumberOfTrades(lootContext)`, i.e. `amount.getInt(lootContext)`):

```
leftover = pool.toMutableList()          // size N, order = registry/tag iteration order
found = 0
while found < k and leftover.notEmpty():
    i = lootContext.random.nextInt(leftover.size())   // one RNG call
    trade = leftover.remove(i)                         // swap-free ArrayList.remove(index)
    offer = trade.getOffer(lootContext)                 // may be null (predicate/enchant/cost failure)
    if offer != null:
        merchantOffers.add(offer)
        found += 1
    // if offer == null, the candidate is discarded WITHOUT retry — it does not go back in the pool
```

Notes for bit-parity:
- Exactly one `nextInt(leftover.size())` call per loop iteration, and the loop iterates once per *candidate examined*, not once per *offer accepted* — a rejected candidate (predicate false, or the enchant/cost pipeline nulled it) still consumes one RNG draw and shrinks the pool by one, so the number of RNG draws for a given roll is `min(N, iterations-until-k-accepted-or-pool-exhausted)`, not simply `k`.
- If the pool is exhausted before *k* offers are found, the villager simply ends up with fewer offers than `amount` requested — this is normal for small pools (e.g. `wandering_trader/buying` has only 6 candidate trades in the tag).
- `Lists.newArrayList(potentialOffers)` preserves the `HolderSet`'s iteration order, which is registry-tag order — this is the deterministic starting order the RNG indexes into; reimplementations must replicate the same pool ordering (from the same tag file) or offer selection will not match even with an identical RNG stream.
- The `allow_duplicates = true` path (`addOffersFromItemListings`, unused by any shipped data as of 26.2 but part of the public data format) instead re-rolls the *same, non-shrinking* list every iteration and only removes a candidate from the pool when `getOffer` returns `null` (a permanently-invalid candidate) — implementers must keep both algorithms since a datapack can still set `allow_duplicates: true`.

### 3.3 `VillagerTrade.getOffer(LootContext)` — exact resolution order

1. If `merchant_predicate` is present and evaluates `false` against the loot context → return `null` immediately (candidate rejected, no further steps run).
2. Instantiate the result stack from `gives` (an `ItemStackTemplate`).
3. Apply `given_item_modifiers` (loot-item functions, e.g. `enchant_randomly`, `enchant_with_levels`, `filtered`/`discard`) **in list order**; if any modifier empties the stack → return `null`.
4. Pop the `ADDITIONAL_TRADE_COST` data component off the *result* stack (if present) into a local `additionalCost` int (removing it from the item the player receives — it is a transient computation value, not a real component of the delivered item; confirmed as `additional_trade_cost` in broad doc 10's data-component table).
5. If `double_trade_price_enchantments` is present and the result's `STORED_ENCHANTMENTS` intersects that enchantment `HolderSet` → `additionalCost *= 2`.
6. `itemCost = wants.toItemCost(lootContext, additionalCost)` → `count = clamp(wants.count.getInt(lootContext) + additionalCost, 0, item.maxStackSize)`. If `itemCost.count() < 1` → return `null` (this is how a librarian enchanted-book trade whose base `wants.count` is `0` and whose rolled enchant-cost component happens to resolve to `0` silently fails to produce an offer for that pool candidate).
7. If `additional_wants` is present, resolve it the same way but with `additionalCost` fixed at `0` (the enchant-cost surcharge never applies to the second cost slot); if its count is `< 1` → return `null`.
8. Construct the `MerchantOffer`:
   `new MerchantOffer(itemCost, additionalItemCost, result, max(maxUses.getInt(ctx), 1), max(xp.getInt(ctx), 0), max(reputationDiscount.getFloat(ctx), 0.0F))`
   — i.e. `maxUses` floors at 1, `xp` floors at 0, and the JSON `reputation_discount` value becomes `MerchantOffer.priceMultiplier` directly (floored at `0.0F`, never negative).

### 3.4 Price math — `MerchantOffer.getModifiedCostCount`

This is the function that turns a base cost into the number actually charged, and it runs **every time the UI needs to display or validate a cost** (not cached):

```
basePrice   = cost.count()                                          // int, from ItemCost
demandDiff  = max(0, Mth.floor(basePrice * this.demand * this.priceMultiplier))
result      = clamp(basePrice + demandDiff + this.specialPriceDiff, 1, cost.itemStack().maxStackSize())
```

**Type discipline (parity-critical):** `basePrice` is `int`, `this.demand` is `int`, `this.priceMultiplier` is `float`. Java evaluates `basePrice * this.demand * this.priceMultiplier` left-to-right: `int * int → int`, then `int * float → float`. The multiplication therefore happens **in `f32`**, not `f64` — a Rust port that promotes to `f64` before multiplying will occasionally floor to a different integer than vanilla at demand/multiplier combinations where the `f32` rounding differs from `f64` rounding before the floor. `Mth.floor(float v)` itself is `(int) Math.floor((double) v)` — the float→double widening on entry to `Math.floor` is lossless, so the hazard is entirely in *where* the value is truncated to `f32` precision (immediately after the multiply chain), not in the floor call.

`demandDiff` is clamped to `≥ 0` — a negative raw product (impossible here since `demand` is never forced negative by any code path, but `priceMultiplier` is also never negative post-clamp) is defensive, not reachable in practice, but must still be replicated as written.

### 3.5 Demand update — `MerchantOffer.updateDemand`

```
demand = demand + uses - (maxUses - uses)
       = demand + 2*uses - maxUses
```
Pure `int` arithmetic, no floor/cast. Called once per offer from `Villager.updateDemand()`, which is called from `Villager.restock()` (every real restock) and from `Villager.catchUpDemand()` (see §3.7) — **never** from the trade-commit path itself. `demand` is *not* reset by `resetUses()`; it persists and accumulates across every restock for the lifetime of the offer (saved in `MerchantOffer.CODEC`'s `"demand"` field), so a heavily-traded offer's demand only ever grows (uses stays near maxUses → `2*uses - maxUses` stays positive each restock) unless the item goes untouched for a full restock cycle (uses = 0 → `demand += -maxUses`, i.e. demand *decreases* by a full `maxUses` for a completely unused offer at restock time — this is the game's only demand-decay mechanism, there is no time-based decay).

### 3.6 Reputation discount

`Villager.getPlayerReputation(Player)` = `gossips.getReputation(player.uuid, _ -> true)` = **signed sum** over every gossip entry this villager holds about that player, across all 5 `GossipType`s, of `value * type.weight` (weights carry sign — see §3.8 table). This is a single villager's private opinion; there is no village-wide aggregate reputation value anywhere in the code.

`Villager.updateSpecialPrices(Player)`, called once at the start of every trading session (`mobInteract` → `startTrading` → `updateSpecialPrices`, **not** re-evaluated mid-session):

```
reputation = getPlayerReputation(player)
if reputation != 0:
    for offer in offers:
        offer.addToSpecialPriceDiff( -Mth.floor(reputation * offer.priceMultiplier) )   // f32 multiply, per §3.4's discipline

if player.hasEffect(HERO_OF_THE_VILLAGE):
    amplifier = player.getEffect(HERO_OF_THE_VILLAGE).amplifier
    for offer in offers:
        modifier      = 0.3 + 0.0625 * amplifier          // f64 (double literal), NOT f32
        costReduction = (int) Math.floor(modifier * offer.baseCostA.count)   // f64 floor
        offer.addToSpecialPriceDiff( -max(costReduction, 1) )
```

Both loops call `addToSpecialPriceDiff` (`specialPriceDiff += add`), so **the two discounts stack additively** within one session — a high-reputation Hero of the Village gets both reductions summed into one `specialPriceDiff`. Note the deliberate mixed numeric precision: the reputation-discount multiply is `f32` (via `priceMultiplier: float`), the Hero-of-the-Village multiply is `f64` (via the literal `0.3`/`0.0625` and `Math.floor`, not `Mth.floor`) — a Rust port must keep these two paths in their respective float widths, not unify them into one type.

`specialPriceDiff` resets to `0` for every offer only on `stopTrading()` (`resetSpecialPrices`, called when the trading player is cleared, e.g. menu closed) — so within a single open trading-menu session the diff never re-accumulates from a second `updateSpecialPrices` call (that function only ever runs once per session, at open).

Wandering traders **never** call `updateSpecialPrices` — `WanderingTrader.mobInteract` opens the trade menu directly, with no reputation or Hero-of-the-Village step. Wandering-trader prices are demand-only.

### 3.7 Trade-commit order (`MerchantResultSlot.onTake`)

Exact sequence when a player clicks the result slot to complete a trade:

1. `checkTakeAchievements(carried)` → `carried.onCraftedBy(player, removeCount)` (crafting-stat bookkeeping, not economy-relevant).
2. `offer = slots.getActiveOffer()` — the offer object already resolved by `MerchantContainer.updateSellItem()` when the payment slots last changed (see below); `null` short-circuits the rest.
3. `offer.take(buyA, buyB) || offer.take(buyB, buyA)` — tries cost slot 0 as the primary cost first, then falls back to treating slot 1 as primary (handles the UI letting the player place items in either payment slot). `take()` re-validates `satisfiedBy` and, on success, `buyA.shrink(getCostA().count)` and (if a second cost exists) `buyB.shrink(getCostB().count)` — **the actual displayed cost at time of shrink**, i.e. demand/reputation/Hero-of-the-Village-adjusted, not the JSON base price.
4. Only if step 3 succeeded: `merchant.notifyTrade(offer)` → `AbstractVillager.notifyTrade`:
   a. `offer.increaseUses()` (`uses++`, **not** `updateDemand` — demand only updates at restock, §3.5).
   b. `ambientSoundTime` reset (forces the next ambient-sound check to fire immediately).
   c. `rewardTradeXp(offer)` (§3.8).
   d. If the trading player is a `ServerPlayer`, fire `CriteriaTriggers.TRADE`.
5. Only if step 3 succeeded: `player.awardStat(TRADED_WITH_VILLAGER)`, then `slots.setItem(0, buyA); slots.setItem(1, buyB)` — writing the now-shrunk stacks back re-triggers `MerchantContainer.updateSellItem()` (recomputes the *next* possible offer/result preview for the still-open menu, and plays the trade-updated sound).
6. **Unconditionally** (outside the step-3 success check): `merchant.overrideXp(merchant.villagerXp + offer.xp)`. For real villagers/wandering traders this is a **no-op** — `AbstractVillager.overrideXp` is an empty method body; it exists only for `ClientSideMerchant` to keep the client-predicted XP bar in sync. The actual server-side XP grant happens exclusively through `rewardTradeXp` in step 4c. A reimplementation that wires `overrideXp` into real XP accounting on the server would double-grant XP.

`MerchantOffer.satisfiedBy`/`take` always compare against `getModifiedCostCount` (the live, demand+special-price-adjusted count), never the raw JSON `count` — offer validity and cost-shrink amount are computed fresh from current `demand`/`specialPriceDiff` at click time, not cached from menu-open time.

### 3.8 XP and leveling (`VillagerData`, `Villager.rewardTradeXp`)

`VillagerData.NEXT_LEVEL_XP_THRESHOLDS = [0, 10, 70, 150, 250]` (index = current level 0..4; only indices 1–4 are meaningful since level 0 doesn't exist — villager levels run 1–5). `getMaxXpPerLevel(level)` returns `THRESHOLDS[level]` if `canLevelUp(level)` (`1 ≤ level < 5`), else `0`.

| Current level | XP needed to reach next level |
|---|---|
| 1 → 2 | 10 |
| 2 → 3 | 70 |
| 3 → 4 | 150 |
| 4 → 5 | 250 |
| 5 | — (max level, `canLevelUp` false) |

`Villager.rewardTradeXp(offer)`:
```
popXp = 3 + random.nextInt(4)                 // uniform 3..6, one RNG call, entity's own (non-deterministic) random
villagerXp += offer.xp                         // accumulates career XP (separate from the popped orb amount!)
lastTradedPlayer = tradingPlayer               // queues the TRADE reputation event for next customServerAiStep
if shouldIncreaseLevel():                      // canLevelUp(level) && villagerXp >= getMaxXpPerLevel(level)
    updateMerchantTimer = 40                    // ticks of delay before the level-up actually applies
    increaseProfessionLevelOnUpdate = true
    popXp += 5                                  // level-up bonus folded into the SAME orb, range becomes 8..11
if offer.shouldRewardExp():                     // rewardExp field, true unless a datapack sets false
    spawn ExperienceOrb(popXp) at villager position + (0, 0.5, 0)
```
The level-up itself is deferred: `customServerAiStep` counts `updateMerchantTimer` down each tick while **not currently trading**; on reaching 0, if `increaseProfessionLevelOnUpdate`, calls `increaseMerchantCareer(level)` (`villagerData.level += 1`, then `updateTrades(level)` appends the new level's rolled offers) and grants `MobEffects.REGENERATION` for 200 ticks at amplifier 0 (visible "levelling up" glow + healing). This means a villager that keeps trading continuously (timer only ticks down while `!isTrading()`) can accumulate XP past a level threshold for an arbitrary number of trades before the level-up visually/mechanically lands — the 40-tick delay only starts counting once the player stops interacting.

`lastTradedPlayer` → in the next `customServerAiStep`, fires `ReputationEventType.TRADE` against that player (once per tick this field is non-null, i.e. once per completed trade, cleared immediately after firing) and broadcasts entity event `14` (happy-villager particles).

### 3.9 Restock mechanics

Trigger site: `WorkAtPoi.start()`, itself gated to run at most roughly every 300 ticks and only 50% of attempts (`checkExtraStartConditions`: `level.getGameTime() - lastCheck < 300` → skip; else one `random.nextInt(2) != 0` roll → skip half the time; on success, `lastCheck` resets and the villager must additionally be within `1.73` blocks of its claimed job-site POI). When it fires: `if body.shouldRestock(level): body.restock()`.

`Villager.shouldRestock(ServerLevel)`:
```
isNewDay = gameTime > lastRestockGameTime + 12000          // half a vanilla day, ticks
currentDay = registryAccess.get(Timelines.OVERWORLD_DAY)
                .map(t -> t.value().getPeriodCount(level.clockManager()))
                .orElse(0)
isNewDay |= (lastRestockCheckDay > 0 && currentDay > lastRestockCheckDay)
lastRestockCheckDay = currentDay
if isNewDay:
    lastRestockGameTime = gameTime
    resetNumberOfRestocks()        // catchUpDemand() then numberOfRestocksToday = 0
return allowedToRestock() && needsToRestock()
```
`allowedToRestock()`: `numberOfRestocksToday == 0 || (numberOfRestocksToday < 2 && gameTime > lastRestockGameTime + 2400)` — **at most 2 restocks per day-window**, and the 2nd must be at least 2400 ticks (2 minutes) after the 1st. `needsToRestock()`: any offer has `uses > 0` (i.e. has been used at least once since its last reset).

`Villager.restock()`: `updateDemand()` for every offer (§3.5), then `resetUses()` for every offer (`uses = 0`), resend offers to any currently-open trading player, `lastRestockGameTime = gameTime`, `numberOfRestocksToday++`.

`catchUpDemand()` (called from `resetNumberOfRestocks`, i.e. once per detected new day): computes `missedUpdates = 2 - numberOfRestocksToday` (how many of the day's up-to-2 restocks the villager *didn't* get around to, e.g. because it never worked at its job site that day); if `> 0`, force `resetUses()` on every offer once, then call `updateDemand()` `missedUpdates` times (so demand still drops by the full `-maxUses` per missed restock even though the villager wasn't actually present to "restock" — this keeps demand decay from a workstation-avoiding villager consistent with one that dutifully restocked and had nothing bought).

**Day-boundary note:** 26.2 resolves "is it a new day" through the new `Timelines.OVERWORLD_DAY` / `ClockManager` calendar system (an `EnvironmentAttribute`-era generalization of vanilla's old flat daylight cycle) in addition to the raw 12000-tick gameTime check, rather than the old versions' plain `dayTime / 24000` comparison. The calendar system itself is a separate cross-cutting subsystem (out of scope here); the parity-relevant fact for villager economy is only that `shouldRestock` ORs *two* independent day-boundary signals together, so a reimplementation must reproduce both triggers, not just the gameTime one.

### 3.10 Gossip container internals

`GossipType` — `(id, weight, max, decayPerDay, decayPerTransfer)`, verified directly from source:

| Type | weight | max (cap) | decay / day (24000 ticks) | decay / transfer |
|---|---:|---:|---:|---:|
| `MAJOR_NEGATIVE` | −5 | 100 | 10 | 10 |
| `MINOR_NEGATIVE` | −1 | 200 | 20 | 20 |
| `MINOR_POSITIVE` | 1 | 25 | 1 | 5 |
| `MAJOR_POSITIVE` | 5 | 20 | 0 | 20 |
| `TRADING` | 1 | 25 | 2 | 20 |

Plus three flat constants used by callers (not by `GossipContainer` itself): `REPUTATION_CHANGE_PER_EVENT = 25`, `REPUTATION_CHANGE_PER_EVERLASTING_MEMORY = 20`, `REPUTATION_CHANGE_PER_TRADE = 2` (these are exactly the amounts `Villager.onReputationEventFrom` adds — see §3.11 — the constant names are a bit of documentation-as-code that doesn't get referenced by field name at the call sites, which use literal `25`/`20`/`2`).

Storage: `Map<UUID target, Object2IntMap<GossipType> entries>` per villager (`GossipContainer.gossips`), i.e. one raw integer *value* per (target player, type) pair; the signed weighted contribution to reputation is always `value * type.weight`, computed on demand, never stored.

**Add** (`GossipContainer.add(target, type, delta)`):
```
newValue = merge(old, delta) where merge(old, delta):
    sum = old + delta
    return sum > type.max ? max(type.max, old) : sum      // note: NOT simply min(sum, max)
then clamp: if value > type.max: value = type.max
            if value < 2: entry removed entirely            // DISCARD_THRESHOLD
```
The `merge` function's `max(type.max, old)` branch means an entry that is *already above* its cap (only reachable via a prior direct `max` value, since normal `add` never overshoots — see clamp step) is left at its current (over-cap) value rather than being pulled down to `max`, when the naive sum would have exceeded the cap; the immediately-following `makeSureValueIsntTooLowOrTooHigh` clamp then pulls any actually-over-cap value back down to `max` anyway, so the net externally observable effect is a plain `min(sum, max)` clamp with a floor-discard at `< 2` — but a reimplementation replicating the two-step internal logic verbatim (rather than collapsing to the simpler equivalent) is safer for any subtle edge case around simultaneous multi-source merges.

**Decay** (`GossipContainer.decay()` → each entry): `newValue = value - type.decayPerDay`; if `< 2`, entry removed; else stored. Driven by `Villager.maybeDecayGossip()`, called every tick from `Villager.tick()`: first call after spawn/load sets `lastGossipDecayTime = gameTime` (no decay yet); thereafter decays once every `24000` ticks (`GOSSIP_DECAY_INTERVAL`, one full vanilla day) measured from `lastGossipTime`'s own tracking field, saved to NBT (`LastGossipDecay`) so decay cadence survives reload/relog rather than resetting.

**Weighted transfer between two villagers** (`GossipContainer.transferFrom(source, random, maxCount)`, called as `this.gossips.transferFrom(target.gossips, this.random, 10)` from `Villager.gossip`):
1. Flatten the *source*'s gossip map to a list of `(target, type, value)` entries.
2. Build a cumulative weighted range array: `ranges[i] = (Σ_{j≤i} |entries[j].value * entries[j].weight|) - 1` (weighted by **absolute** value, so a villager is exactly as likely to pass on a strong grudge as a strong commendation).
3. Up to `maxCount` times (10, from the hardcoded call-site argument — the `MAX_GOSSIP_TOPICS = 10` field exists but is never referenced by name, it just happens to equal the literal passed in): draw `random.nextInt(rangesEnd)`, binary-search the `ranges` array for the entry whose cumulative range contains the draw, add it to an **identity-hash `Set`** (so re-drawing the exact same entry object is a no-op, not a duplicate — meaning fewer than `maxCount` *distinct* entries are typically selected once the pool of entries is small relative to 10 draws).
4. For each selected entry: `decayedValue = value - type.decayPerTransfer`; if `≥ 2`, merge into the *receiver*'s ledger via `mergeValuesForTransfer = max(oldValue, newValue)` (transfer merge is a **max**, not additive — receiving the same gossip twice does not stack it, it only refreshes it upward if the incoming copy is fresher/stronger).

Gossip babble happens inside `TradeWithVillager` (a Brain behavior that, despite its name, is the villager-to-villager "meet and socialize" interaction at the meeting point/bell — not player trading), which calls `body.gossip(level, target, timestamp)` every tick two villagers are within `5.0` (squared distance) of each other and locked in an `INTERACTION_TARGET` pairing. `Villager.gossip`:
```
if (timestamp not within [lastGossipTime, lastGossipTime+1200)) and (timestamp not within [target.lastGossipTime, target.lastGossipTime+1200)):
    this.gossips.transferFrom(target.gossips, this.random, 10)
    this.lastGossipTime = target.lastGossipTime = timestamp
    this.spawnGolemIfNeeded(level, timestamp, 5)
```
i.e. gossip is **one-directional per call** (this villager pulls from target, never the reverse in the same call — but since both villagers run their own Brain tick and both are typically paired as each other's `INTERACTION_TARGET`, a full bidirectional exchange happens over two ticks, one from each side), gated by a shared `1200`-tick (`GOSSIP_COOLDOWN`, 60 s) cooldown tracked independently per villager (not per pair) — a villager that just gossiped with anyone cannot gossip again with a *different* villager for another 60 s either.

### 3.11 Reputation-event generation (`Villager.onReputationEventFrom`)

Dispatch is a trivial passthrough: `ServerLevel.onReputationEvent(type, source, target) → target.onReputationEventFrom(type, source)`. Villager's handler:

| `ReputationEventType` | Gossip added (about `source`) | Fired from |
|---|---|---|
| `TRADE` | `TRADING +2` | `Villager.customServerAiStep`, once per completed trade (via `lastTradedPlayer`, §3.8) |
| `VILLAGER_HURT` | `MINOR_NEGATIVE +25` | `Villager.setLastHurtByMob`, whenever a non-null attacker damages this villager |
| `VILLAGER_KILLED` | `MAJOR_NEGATIVE +25` | `Villager.die` → `tellWitnessesThatIWasMurdered`, fired **once per living, `ReputationEventHandler`-implementing witness** in this villager's `NEAREST_VISIBLE_LIVING_ENTITIES` memory at time of death (typically other nearby villagers) — so killing one villager in view of *N* others adds `+25` to *each* of those *N* villagers' opinion of the killer independently, not a single global entry |
| `ZOMBIE_VILLAGER_CURED` | `MAJOR_POSITIVE +20` **and** `MINOR_POSITIVE +25` (both added) | `ZombieVillager.finishConversion`, fired **once**, only against the newly-converted villager itself (not witnesses), only if a `conversionStarter` UUID resolving to a live `ServerPlayer` exists |
| `GOLEM_KILLED` | *(none — `Villager.onReputationEventFrom` has no branch for it)* | **Never fired** anywhere in 26.2 — declared in `ReputationEventType` but dead: no call site invokes `onReputationEvent(GOLEM_KILLED, …)`. A reimplementation should not spend effort wiring this event; it is vestigial. |

**Per-cure "stacking":** because `MAJOR_POSITIVE`'s cap is exactly `20` and a single cure event adds `20` in one call, one cure already saturates that villager's `MAJOR_POSITIVE` entry about that player — a second cure of a *different* zombie villager cannot push that specific entry any higher (still capped 20), but it *does* create/raise a `MAJOR_POSITIVE` entry on a **different villager** (the newly-cured one), so overall village opinion of a serial zombie-curer scales with the number of distinct villagers cured, not with repeated cures of the same relationship. `MINOR_POSITIVE`'s cap of `25` behaves identically (one cure's `+25` saturates it immediately).

### 3.12 Iron golem spawn conditions

Two independent trigger paths, both funneling into `Villager.spawnGolemIfNeeded(level, timestamp, villagersNeededToAgree)`:

| Trigger | Cadence | `villagersNeededToAgree` |
|---|---|---|
| `VillagerPanicTrigger.tick` (Brain behavior active while `PANIC` activity is active, i.e. hurt or a hostile is in memory) | every 100 ticks (`timestamp % 100 == 0`) while panicking | 3 |
| `Villager.gossip` (successful babble exchange, §3.10) | once per successful gossip call (≤ every 1200 ticks per villager) | 5 |

`spawnGolemIfNeeded`:
```
if not wantsToSpawnGolem(timestamp): return
searchBox = this.boundingBox.inflate(10, 10, 10)               // 20×20×20 AABB centered on this villager
nearby = level.getEntitiesOfClass(Villager, searchBox)
agreeing = nearby.filter(v -> v.wantsToSpawnGolem(timestamp)).limit(5).toList()
if agreeing.size() >= villagersNeededToAgree:
    result = SpawnUtil.trySpawnMob(IRON_GOLEM, MOB_SUMMONED, level, this.blockPosition(),
                                     spawnAttempts=10, spawnRangeXZ=8, spawnRangeY=6,
                                     strategy=LEGACY_IRON_GOLEM, checkCollisions=false)
    if result present:
        nearby.forEach(GolemSensor::golemDetected)   // ALL nearby villagers (not just the agreeing ones) get the cooldown
```
`wantsToSpawnGolem(timestamp)`: `golemSpawnConditionsMet(gameTime) && !brain.hasMemoryValue(GOLEM_DETECTED_RECENTLY)`. `golemSpawnConditionsMet`: the villager's `LAST_SLEPT` memory must be present **and** `gameTime - lastSlept < 24000` — i.e. **a villager only wants a golem if it has slept (successfully used a bed) within the last vanilla day**; a villager that never sleeps (no bed, or persistently interrupted) never votes for a golem, regardless of panic or gossip activity. `GOLEM_DETECTED_RECENTLY` is a 599-tick-TTL memory set whenever `GolemSensor` (a 200-tick-cadence sensor) sees an iron golem in `NEAREST_LIVING_ENTITIES`, *or* whenever a spawn attempt in this method succeeds (applied to the entire `nearby` list, agreeing or not) — this is the sole spawn-rate limiter, since there is no explicit "golems per village" cap anywhere in this code path.

`SpawnUtil.trySpawnMob`'s position search, per attempt (up to 10, one full attempt = 2 RNG draws minimum): `dx = randomBetweenInclusive(random, -8, 8)`, `dz = randomBetweenInclusive(random, -8, 8)` (`level.getRandom()`, the level's own RNG, not the villager's), search position starts 6 blocks above the villager and the placement strategy (`LEGACY_IRON_GOLEM`) walks down/adjusts to a valid ground position; `checkCollisions = false` means no entity-bounding-box pre-check is done before entity creation (the mob's own `checkSpawnObstruction` after construction is the real gate). First successful attempt wins; failure after 10 attempts silently does nothing (no retry until the next trigger tick).

### 3.13 Zombie-villager cure

`ZombieVillager.mobInteract`: requires the item in hand to be `GOLDEN_APPLE` and the zombie to currently have the `WEAKNESS` effect (from being hit by a splash/lingering weakness potion, or the `Golden Apple + Weakness` classic cure combo); on success, consumes 1 apple and calls `startConverting(player.uuid, random.nextInt(2401) + 3600)` — **conversion time is uniform in `[3600, 6000]` ticks** (3–5 minutes), one RNG call. `startConverting` also removes `WEAKNESS`, grants `STRENGTH` for the same duration at amplifier `min(difficulty.id - 1, 0)`. `Difficulty` ids are `PEACEFUL=0, EASY=1, NORMAL=2, HARD=3`, so this expression is `0` (Strength I) on Easy/Normal/Hard and `-1` on Peaceful — the code never gates conversion on difficulty, so a zombie villager cured while the world is set to Peaceful genuinely receives a **negative-amplifier** `STRENGTH` instance (vanilla's `MobEffectInstance`/attribute-modifier layer tolerates negative amplifiers as a weakening effect rather than rejecting them); a reimplementation that assumes amplifier is always `≥ 0` for this grant will mishandle the Peaceful case.

Per-tick progress (`ZombieVillager.getConversionProgress`, called once per tick while converting, subtracted from the remaining timer):
```
amount = 1
if random.nextFloat() < 0.01:                              // 1% chance per tick to even check for accelerant blocks
    specialBlocksCount = 0
    for each block in an 8×8×8 box centered on the zombie (early-exits once specialBlocksCount == 14):
        if block is IRON_BARS or a BedBlock:
            if random.nextFloat() < 0.3:
                amount += 1
            specialBlocksCount += 1
return amount
```
So conversion normally decrements the timer by exactly `1` per tick; roughly 1% of ticks additionally scan for up to 14 iron-bars/bed blocks in an 8×8×8 volume, each contributing an independent 30%-chance `+1` to that tick's decrement (so a well-built "curing cage" with many bars/beds nearby can occasionally decrement the timer by well over `1` on a lucky check-tick, accelerating the cure — this reproduces vanilla's classic "surround with iron bars and a bed" cure-speedup advice). RNG calls per tick: 1 guaranteed (`nextFloat` gate), plus up to 14 more (`nextFloat` per qualifying block) only on the ~1%-chance ticks.

On completion (`finishConversion`): converts to `Villager` preserving equipment (except pieces enchanted with `PREVENT_ARMOR_CHANGE`, which drop instead), villager data, gossip container, trade offers (copied), and XP; adds `NAUSEA` for 200 ticks (the classic post-cure disorientation); if `conversionStarter` resolves to a live `ServerPlayer`, fires `CriteriaTriggers.CURED_ZOMBIE_VILLAGER` and `onReputationEvent(ZOMBIE_VILLAGER_CURED, player, villager)` (§3.11).

Freshly-spawned zombie villagers (`initializeZombieVillagerData`) get a **uniformly random profession** from the entire `VillagerProfession` registry (`BuiltInRegistries.VILLAGER_PROFESSION.getRandom(random)`, one RNG call) rather than always `NONE` — so a cured zombie villager can immediately show up with pre-rolled trade offers for a random profession/level-1 the moment it finishes converting, if `updateTrades` runs before a player reassigns it.

### 3.14 Profession / workstation acquisition (POI claim flow)

1. **`AcquirePoi`** (generic Brain behavior parameterized per profession's `acquirableJobSite` predicate): rate-limited to roughly one scan attempt per `20 + random.nextInt(20)` ticks (`20–39`, re-rolled after every attempt) via a `nextScheduledStart` closure captured per behavior instance; on a scan tick, queries `PoiManager.findAllClosestFirstWithType` for up to 5 unclaimed (`HAS_SPACE`) matching POIs within 48 blocks, filtered by a per-POI **jittered linear retry backoff** cache (`JitteredLinearRetry`: on each failed pathfind to a given POI, `currentDelay = min(currentDelay + random.nextInt(40) + 40, 400)`, next retry no sooner than `currentDelay` ticks later, entry expires after `400` ticks of no attempts) so a villager doesn't hammer-repath to an unreachable POI every scan tick. On a successful path, `PoiManager.take(...)` claims exactly 1 slot at the target POI and sets the `POTENTIAL_JOB_SITE` memory.
2. **`AssignProfessionFromJobSite`**: once the villager is within 2 blocks of its `POTENTIAL_JOB_SITE` (or was spawned as part of a structure and has `assignProfessionWhenSpawned` set — see below), promotes the memory to `JOB_SITE` and, **only if current profession is `NONE`**, looks up the first registered `VillagerProfession` whose `heldJobSite` predicate matches the claimed POI's type and assigns it, then `refreshBrain` (rebuilds the Brain's activity packages for the new profession — necessary since `VillagerGoalPackages.getWorkPackage` is keyed by profession). A villager that already has a profession keeps it even if it opportunistically claims a mismatched POI type (this path only *assigns*, never re-assigns).
3. **`ResetProfession`** ("firing"): only fires (reverts to `NONE`) if the villager currently has *no* `JOB_SITE` memory, its profession is neither `NONE` nor `NITWIT`, **and** `villagerXp == 0 && level ≤ 1` — i.e. a villager that has already earned any trade XP or reached level 2+ can never be automatically fired for losing its workstation; only a completely untouched level-1/0-XP villager reverts to unemployed.
4. **`YieldJobSite`**: an unemployed, non-baby villager holding an unclaimed `POTENTIAL_JOB_SITE` will hand it off to the first eligible nearby villager (in `NEAREST_LIVING_ENTITIES` order) that (a) has no `POTENTIAL_JOB_SITE` of its own, and (b) either already holds that exact `JOB_SITE`'s position, or has no `JOB_SITE` at all and can already path to it, **and whose current profession's `heldJobSite` predicate already matches** the POI type (i.e. this only redirects a job site toward a villager of the *same profession* that already lost/never had it — it is not a general recruitment mechanism for unemployed villagers of arbitrary profession).
5. Structure-spawned villagers (`EntitySpawnReason.STRUCTURE`, e.g. village-generation-placed villagers) set `assignProfessionWhenSpawned = true` in `finalizeSpawn`, which lets `AssignProfessionFromJobSite` claim/assign immediately without waiting for the 2-block proximity check — this is why village-generated villagers appear to have professions assigned instantly on chunk load rather than walking to a workstation first.

### 3.15 Breeding economics

`Villager.FOOD_POINTS = {BREAD: 4, POTATO: 1, CARROT: 1, BEETROOT: 1}` (no other food item counts, notably no golden carrot/apple). `BREEDING_FOOD_THRESHOLD = 12`.

`canBreed()`: `foodLevel + countFoodPointsInInventory() ≥ 12 && !isSleeping() && age == 0` (must be a non-baby, non-sleeping villager whose *combined* stored + carried food value clears the threshold — food doesn't need to already be digested, just present).

`VillagerMakeLove` behavior, once two adult, breeding-eligible villagers are paired via `BREED_TARGET`:
1. `start`: locks gaze, broadcasts entity event `18` (love particles) for both, schedules `birthTimestamp = timestamp + 275 + random.nextInt(50)` (**275–324 ticks**, ~13.75–16.2 s courting period, one RNG call at pairing time).
2. `tick`, while courting and within 5 (squared distance): if `timestamp < birthTimestamp`, a `random.nextInt(35) == 0` roll (~2.86% per tick) triggers a heart-particle broadcast (event `12`) for both — purely cosmetic, no economic effect, but a real per-tick RNG draw while courting.
3. On reaching `birthTimestamp`: **both parents eat and digest food unconditionally** (`eatAndDigestFood()` = `eatUntilFull()` then `foodLevel -= 12`) **before** checking whether a vacant bed exists. `eatUntilFull()` walks inventory slots in fixed order consuming whole `FOOD_POINTS`-matching stacks until `foodLevel ≥ 12` — deterministic, no RNG.
4. Only then does the behavior try to reserve a vacant `HOME`-type POI bed within 48 blocks (reachable via pathfinding); **if none is available, the birth is aborted (event `13`, "no" particles) but the food has already been consumed on both parents** — a village that runs out of empty beds burns food on every failed courting cycle with no offspring produced. This is a genuine, source-confirmed hazard (§7), not an assumption.
5. If a bed is found: child type is rolled via a **single** `random.nextDouble()` draw: `< 0.5` → biome-appropriate `VillagerType` for the *breeding location*, `< 0.75` (i.e. `[0.5, 0.75)`) → the initiating parent's own type, else (`[0.75, 1.0)`) → the partner's type. Parents' `age` is set to `6000` (adult, resets any residual growth state), the child's `age` is set to `-24000` (standard 20-minute baby-growth duration, identical convention to animal breeding), and the child inherits `VillagerProfession.NONE` (villager children are always born unemployed regardless of parents' professions) with `villagerDataFinalized = true` (type is locked in immediately, skipping the biome-on-spawn finalize path).
6. The reserved bed's `GlobalPos` becomes the child's `HOME` memory directly (no separate acquisition step needed for its first bed).

### 3.16 Wandering trader

**Spawn scheduling** (`WanderingTraderSpawner`, ticked every server tick but internally rate-limited): outer tick-delay of `1200` ticks (1 minute) between scheduling checks; when it fires, decrements a persisted (`SavedData`) `spawnDelay` counter by `1200`; once `spawnDelay ≤ 0`:
```
spawnDelay = 24000                                    // reset to a full day for the next cycle
chanceToSpawn = data.spawnChance()                    // persisted, starts at 25
newSpawnChance = clamp(chanceToSpawn + 25, 25, 75)     // ladder step, saved immediately regardless of outcome
data.spawnChance = newSpawnChance
if random.nextInt(100) <= chanceToSpawn:               // uses the OLD chance, not the just-incremented one
    if spawn(level) succeeds:
        data.spawnChance = 25                          // success resets the ladder back to the floor
```
So the spawn-chance ladder is **25 → 50 → 75 → 75 → 75 …** (clamped at 75, `MAX_SPAWN_CHANCE`), climbing by `25` (`SPAWN_CHANCE_INCREASE`) every failed daily roll and resetting to `25` (`MIN_SPAWN_CHANCE`) only on an actual successful spawn — a roll that "wins" the `≤ chanceToSpawn` check but then fails the *placement* search (`spawn(level)` returns `false`) does **not** reset the ladder, only a placed trader does.

`spawn(level)`: picks a uniformly random online player (`level.getRandomPlayer()`); a `random.nextInt(10) != 0` check (**10% pass rate**, `SPAWN_ONE_IN_X_CHANCE`) gates the whole attempt further — even after winning the daily percentage roll, a wandering trader spawn attempt only actually proceeds 1/10 of the time it's invoked (this second gate resets every time `spawn` is called, independent of the outer daily chance). Reference position is the nearest unoccupied `MEETING` POI within 48 blocks of the player, falling back to the player's own position; up to 10 position-search attempts (`NUMBER_OF_SPAWN_ATTEMPTS`) each picking a uniformly random offset within ±48 blocks XZ and the terrain height at that column, validated against the entity's spawn-placement rules and a manual 1×3×1 (`betweenClosed(pos, pos.offset(1,2,1))`) empty-collision check. On success: despawn timer set to `48000` ticks (2 days), wander target and 16-block home restriction set to the reference position, and **2 attempted llama escorts** (each an independent position search within 4 blocks, leashed to the trader if a valid spot is found — 0, 1, or 2 llamas may actually appear).

**Trade set:** `WanderingTrader.updateTrades` = `WANDERING_TRADER_BUYING` (2 offers) + `WANDERING_TRADER_UNCOMMON` (2 offers) + `WANDERING_TRADER_COMMON` (5 offers) = **up to 9 total offers**, each category independently rolled without duplicates from its own tag pool (buying pool has only 6 candidate trades total, so both of its 2 rolled offers are effectively "2 of these 6 items are buyable this spawn"). `WanderingTrader` never overrides `canRestock()` (`Merchant` interface default `false`), so **wandering-trader offers never restock** — once every offer's `maxUses` is exhausted, that offer is gone until the trader despawns and a fresh one spawns. Wandering traders also never apply reputation or Hero-of-the-Village discounts (§3.6).

**Despawn:** `WanderingTrader.aiStep` → `maybeDespawn`: while `despawnDelay > 0` and not currently trading, decrement every tick; on reaching exactly `0`, `discard()` (silent removal, no death event).

### 3.17 Hero of the Village (raid interaction)

Granted on `Raid` victory (`postRaidTicks` reaches 40 with no waves remaining and 0 raiders alive), to every living, non-spectator entity in the raid's `heroesOfTheVillage` UUID set (populated via `Raid.addHeroOfTheVillage`, called whenever a player lands the killing blow on a raider — cross-referenced, not re-derived here, from the raid-mechanics domain):
```
addEffect(HERO_OF_THE_VILLAGE, duration=48000, amplifier = raidOmenLevel - 1)
```
`raidOmenLevel` ranges `0..5` (`getMaxRaidOmenLevel() = 5`, accumulated via `absorbRaidOmen` from the `RAID_OMEN` effect amplifier at raid-trigger time), so the granted Hero-of-the-Village amplifier ranges **0..4** (Hero I at the minimum raid-omen level that can actually trigger a raid, up through Hero V-equivalent scaling at the maximum). Duration is a flat `48000` ticks (40 minutes) regardless of amplifier. This amplifier is exactly the value consumed by §3.6's `0.3 + 0.0625 * amplifier` discount formula and by `GiveGiftToHero`'s hero-detection (`player.hasEffect(HERO_OF_THE_VILLAGE)`, amplifier-independent — any Hero level triggers gift-giving) and periodic gift-throw timing (`600 + random.nextInt(6001)` ticks, **600–6600**, between gift throws, one RNG call per gift-throw cycle completion).

## 4. Constants table (consolidated)

| Constant | Value | Source |
|---|---:|---|
| `MerchantOffer` demand update | `demand += 2*uses - maxUses` | `MerchantOffer.updateDemand` |
| `MerchantOffer` cost formula | `clamp(base + max(0, floor(base*demand*mult)) + specialDiff, 1, maxStack)` | `MerchantOffer.getModifiedCostCount` |
| Common-trade `reputation_discount` | `0.05` | shipped `villager_trade/**` JSON (resource/food trades) |
| Premium-trade `reputation_discount` | `0.2` | shipped `villager_trade/**` JSON (gear/enchanted/emerald-cost trades) |
| Hero of the Village discount modifier | `0.3 + 0.0625 * amplifier` (f64) | `Villager.updateSpecialPrices` |
| Hero of the Village min discount per offer | `max(costReduction, 1)` | same |
| Hero of the Village duration | `48000` ticks | `Raid.HERO_OF_THE_VILLAGE_DURATION` |
| Hero of the Village amplifier | `raidOmenLevel - 1`, range `0..4` | `Raid` (victory grant) |
| Default `TradeSet.amount` | `2` | `TradeSets.register` |
| `librarian/level_5` amount | `3` | `TradeSets` |
| `wandering_trader/common` amount | `5` | `TradeSets` |
| `wandering_trader/{buying,uncommon}` amount | `2` each | `TradeSets` |
| `MAX_GOSSIP_TOPICS` / transfer draw count | `10` | `Villager` |
| `GOSSIP_COOLDOWN` | `1200` ticks | `Villager` |
| `GOSSIP_DECAY_INTERVAL` | `24000` ticks | `Villager` |
| `GossipType` weight/max/decayDay/decayTransfer | see §3.10 table | `GossipType` |
| `DISCARD_THRESHOLD` (min viable gossip value) | `2` | `GossipContainer` |
| `REPUTATION_CHANGE_PER_EVENT` | `25` | `GossipType` (documents the hurt/killed magnitude) |
| `REPUTATION_CHANGE_PER_EVERLASTING_MEMORY` | `20` | `GossipType` (documents cure major-positive magnitude) |
| `REPUTATION_CHANGE_PER_TRADE` | `2` | `GossipType` |
| Villager XP thresholds (level→next) | `[10, 70, 150, 250]` for levels 1-4 | `VillagerData.NEXT_LEVEL_XP_THRESHOLDS` |
| Trade-XP popped-orb amount | `3 + rand(4)` (+5 on level-up) | `Villager.rewardTradeXp` |
| Level-up delay | `40` ticks | `Villager.updateMerchantTimer` |
| Level-up Regeneration grant | `200` ticks, amplifier 0 | `Villager.customServerAiStep` |
| Max restocks/day | `2` | `Villager.allowedToRestock` |
| Min gap between restocks | `2400` ticks | same |
| Restock-check attempt cadence | `≥300` ticks, 50% roll | `WorkAtPoi` |
| `AcquirePoi` scan interval | `20 + rand(20)` ticks | `AcquirePoi` |
| `AcquirePoi` scan range | `48` blocks | `AcquirePoi.SCAN_RANGE` |
| `AcquirePoi` retry backoff | `+rand(40)+40`/failure, cap `400` | `AcquirePoi.JitteredLinearRetry` |
| `ResetProfession` firing condition | `xp==0 && level≤1 && no JOB_SITE` | `ResetProfession` |
| Breeding food threshold | `12` points | `Villager.BREEDING_FOOD_THRESHOLD` |
| `FOOD_POINTS` | bread 4, potato/carrot/beetroot 1 | `Villager.FOOD_POINTS` |
| Courting duration | `275 + rand(50)` ticks | `VillagerMakeLove` |
| Courting heart-particle roll | `1/35` per tick | same |
| Baby growth duration | `24000` ticks (`age = -24000`) | `VillagerMakeLove.breed` |
| Child type roll thresholds | `<0.5` biome, `<0.75` parent A, else parent B | `Villager.getBreedOffspring` |
| Zombie-villager cure time | `3600 + rand(2401)` ticks (3600–6000) | `ZombieVillager.mobInteract` |
| Cure accelerant scan chance | `1%`/tick, box `8×8×8`, cap 14 blocks | `ZombieVillager.getConversionProgress` |
| Cure accelerant per-block bonus | `30%` chance, `+1` tick-equivalent | same |
| Post-cure Nausea | `200` ticks | `ZombieVillager.finishConversion` |
| Golem panic-trigger cadence / threshold | every `100` ticks, `≥3` agreeing | `VillagerPanicTrigger` |
| Golem gossip-trigger threshold | `≥5` agreeing | `Villager.gossip` |
| Golem search AABB | `±10` blocks (20×20×20) | `Villager.spawnGolemIfNeeded` |
| Golem sleep-recency gate | `<24000` ticks since `LAST_SLEPT` | `Villager.golemSpawnConditionsMet` |
| Golem detected-recently TTL | `599` ticks | `GolemSensor` |
| Golem sensor scan cadence | `200` ticks | `GolemSensor.GOLEM_SCAN_RATE` |
| Golem placement search | `10` attempts, `±8` XZ, `+6` Y start | `Villager.spawnGolemIfNeeded` → `SpawnUtil.trySpawnMob` |
| Wandering trader spawn-chance ladder | `25→50→75` (+25/day, cap 75), reset to 25 on success | `WanderingTraderSpawner` |
| Wandering trader daily-schedule delay | `24000` ticks | `WanderingTraderSpawner.DEFAULT_SPAWN_DELAY` |
| Wandering trader placement gate | `1/10` pass rate | `WanderingTraderSpawner.SPAWN_ONE_IN_X_CHANCE` |
| Wandering trader position search | `10` attempts, `±48` XZ | `WanderingTraderSpawner` |
| Wandering trader despawn | `48000` ticks | `WanderingTraderSpawner.spawn` |
| Wandering trader home restriction | `16` blocks | same |
| Wandering trader gift/llama count | up to `2` llamas | same |
| Gift-to-hero interval | `600 + rand(6001)` ticks (600–6600) | `GiveGiftToHero` |
| Gift throwing distance | `5` blocks | same |
| Legacy LCG (entity `random`, `level.random`) multiplier | `25214903917` (`0x5DEECE66D`) | `LegacyRandomSource`, 48-bit modulus |
| Trade-roll RNG source | `XoroshiroRandomSource`, world-seed+salt+identifier-hash-derived, **persisted per-identifier** | `RandomSequences`/`RandomSequence` |

## 5. RNG usage map

Two structurally different random sources are load-bearing here, and conflating them is a top-tier parity hazard (see §7):

**(A) Entity-local `RandomSource` (`this.random` on `Villager`/`ZombieVillager`/etc., and `level.getRandom()`)** — a `LegacyRandomSource` (classic 48-bit LCG, multiplier `0x5DEECE66D`), seeded via `RandomSource.create(RandomSupport.generateUniqueSeed())` at entity construction — **not** derived from the world seed, and **not persisted** to NBT (a reloaded entity gets a freshly, non-deterministically seeded instance). Consumed by:
- `Villager.rewardTradeXp`: 1 call (`nextInt(4)`) per completed trade.
- `Villager.gossip` → `transferFrom`: up to 10 calls (`nextInt(rangesEnd)`) per successful babble exchange.
- `VillagerMakeLove`: 1 call (`nextInt(50)`) at courting start; 1 call (`nextInt(35)`) per tick while courting; 1 call (`nextDouble()`) at birth for type inheritance.
- `ZombieVillager`: 1 call (`nextInt(2401)`) on starting conversion; per tick while converting, 1 call (`nextFloat`), plus up to 14 more (`nextFloat`) on the ~1% of ticks that scan for accelerant blocks; `initializeZombieVillagerData`: 1 call (`getRandom` on the profession registry) at spawn.
- `Villager.customServerAiStep`: 1 call (`nextInt(100)`) per tick, unconditionally, for the raid-particle broadcast check (not economy-relevant but shares the same stream and consumes a draw every tick a villager is alive and not `NoAI`).
- `AcquirePoi`: 1 call (`nextInt(20)`) per scan-interval reschedule; `JitteredLinearRetry`: 1 call (`nextInt(40)`) per retry-backoff recompute.
- `WorkAtPoi`: 1 call (`nextInt(2)`) per ≥300-tick restock-check attempt.
- `GiveGiftToHero`: 1 call (`nextInt(6001)`) per gift-throw cycle completion.
- `WanderingTraderSpawner` (its own dedicated `RandomSource.create()` instance, same LCG family, held by the spawner object across ticks — **not** per-trader): 1 call (`nextInt(100)`) per daily schedule fire, 1 call (`nextInt(10)`) per `spawn()` invocation, up to 10× 2 calls (`nextInt(radius*2)` for x and z) per position search, up to 3× that again for the 2 llama-escort searches.
- `Villager.spawnGolemIfNeeded` → `SpawnUtil.trySpawnMob`: `level.getRandom()`, up to 10× 2 calls (`nextInt` via `randomBetweenInclusive`) per golem-spawn attempt.

**(B) World-persisted `RandomSequences` (Xoroshiro128, one stream per `Identifier`)** — used **exclusively** for rolling which `VillagerTrade`s populate a `TradeSet` (`AbstractVillager.addOffersFromTradeSet` → `LootContext.Builder.create(tradeSet.randomSequence())` → `MinecraftServer.getRandomSequence(id)` → `RandomSequences.get(id, worldSeed)`). Seed derivation: `seed128 = upgradeSeedTo128bitUnmixed(worldSeed XOR salt).xor(seedFromHashOf(identifier.toString())).mixed()`, where `salt` defaults to a per-world random `int` set once at world creation (`RandomSequences.setSeedDefaults`) and `identifier` is the `TradeSet`'s own `random_sequence` field (e.g. `"minecraft:trade_set/armorer/level_1"`). Critically, **this stream is not per-villager and not per-roll** — it is a single, world-global, save-persisted (`SavedData`, dirty-marked on every draw) counter keyed only by the identifier string. Every armorer anywhere in the world that ever rolls its level-1 trades (initial spawn *or* every subsequent level-up-triggered re-roll for its *next* level, which uses a *different* identifier) draws from the **same continuing stream**, in whatever order those rolls happen to occur during actual gameplay (chunk-load order, player-triggered level-ups, etc.) — this makes trade-content RNG order inherently dependent on real-time gameplay sequencing, not just the world seed, and is **not naively reproducible by seeding a fresh RNG per villager**.

## 6. Cross-references

- Broad doc `09-entities-ai.md` §3.7 (villager Brain/schedule overview, `Activity` package table) and its constants line (already verified consistent with this document's §3.10/§3.11) — this document supersedes that line's precision for gossip/reputation math.
- Broad doc `10-items-recipes-loot.md` §3.9 and its Interfaces/hazards sections — already correctly identifies the Java `VillagerTrades`/`TradeRebalanceVillagerTrades` classes as vestigial datagen sources; this document's §2 confirms that with an exact contradiction example (armorer boots `0.05` vs. shipped `0.2`) and traces the actual bootstrap wiring (`VanillaRegistries` vs. `TradeRebalanceRegistries`).
- `MECH-D31` (`05-game-mechanics.md`): villagers are explicitly a **Brain** (memory/sensor/activity) mob, not Goal/GoalSelector — every behavior cited here (`TradeWithVillager`, `VillagerMakeLove`, `VillagerPanicTrigger`, `AcquirePoi`, `WorkAtPoi`, `GiveGiftToHero`, `AssignProfessionFromJobSite`, `ResetProfession`, `YieldJobSite`) is a `Behavior<Villager>`/`BehaviorControl<Villager>` running inside that system.
- `MECH-D1`/`MECH-D2` (tick pipeline): villager Brain tick, restock, gossip decay, and demand math all run inside per-entity Stage 6a/6b (entity ticking), not a separate scheduled-tick phase — no special tick-pipeline placement needed beyond normal entity AI ticking.
- `ASSET-D18`/`ASSET-D19` (reference-source policy): the exact price/JSON numbers cited here (`0.05`, `0.2`, item counts, `max_uses`) are **facts** (constants/structures, explicitly permitted to state per the reference-source policy), not reproduced algorithm bodies; the pipeline pseudocode in §3.2–§3.9 is written in this document's own words per that same policy.
- Domain 13/14 (cluster architecture / performance engineering): `RandomSequences`' world-global, save-persisted, dirty-marking design (§5B) is itself a piece of shared mutable state that a cluster-mode partitioned world must route through the message substrate (whichever region "wins" the race to roll a given `TradeSet` identifier first must be the one whose result other regions observe) — flagged here for `13-cluster-architecture.md` to account for; not resolved in this document.
- Raid/Hero-of-the-Village amplifier math (§3.17) references `Raid.raidOmenLevel`/`getNumGroups`/`getEnchantOdds`, which belong to the raid-mechanics domain proper and are not re-derived here beyond the exact grant formula this domain consumes.

## 7. Reimplementation hazards (ranked)

1. **Two RNG sources, easy to conflate.** Trade-*content* rolling (§5B, Xoroshiro, world-persisted, identifier-keyed, shared across every villager) is architecturally nothing like every other villager RNG use (§5A, classic LCG, per-entity, non-deterministic, non-persisted). Seeding a single per-villager RNG stream for "everything villager-related" — the natural first instinct — will desync trade content from real vanilla servers immediately, since real trade rolls depend on world-save-lifetime draw order across *all* villagers sharing an identifier, not on any one villager's spawn.
2. **`f32` vs `f64` precision in price math.** §3.4's demand-price multiply and §3.6's reputation-discount multiply are `f32` (via `MerchantOffer.priceMultiplier: float`); §3.6's Hero-of-the-Village multiply is `f64` (double literals). A uniform "use f64 everywhere" port will occasionally floor to a different integer price than vanilla at specific demand/reputation values. Both widths must be preserved exactly where vanilla uses them.
3. **Demand persists forever, with no time decay.** `demand` only changes on `restock()`/`catchUpDemand()` calls (§3.5) and is saved to NBT — an implementation that resets demand on chunk unload/reload, or applies any decay-over-time, will diverge from a long-lived, heavily-traded villager's actual prices.
4. **`RandomSequences` seeding requires the world-creation-time `salt`, not just the seed.** The salt is itself a persisted `SavedData` value (`RandomSequences.salt`), set once per world (typically to a random int at creation) — reconstructing trade rolls from "world seed + identifier" alone, without replicating the salt's storage/generation, will not match.
5. **Offer-roll RNG draw count is candidate-examined, not offers-accepted.** §3.2's without-duplicates loop consumes one RNG draw per pool candidate it looks at, including rejected ones (failed `merchant_predicate`, or a `getOffer` that nulled out) — undercounting draws for rejected candidates desyncs every subsequent draw in that `LootContext`, silently corrupting the rest of that villager's offer list.
6. **`overrideXp` is a server no-op; real XP flows only through `rewardTradeXp`.** §3.7 step 6 — porting `overrideXp` as if it were the authoritative XP-grant call (its name strongly suggests it) will double-grant trade XP server-side.
7. **Breeding consumes food even on a failed birth.** §3.15 step 3–4 — food is eaten and digested by both parents *before* the vacant-bed check; a village with no free beds will still burn through food indefinitely on repeated failed courting cycles. Skipping food consumption when no bed is available (the "obviously correct" design) is not what vanilla does.
8. **Golem-detected cooldown applies to the whole nearby list, not just the villagers who "voted."** §3.12 — `GolemSensor::golemDetected` is applied to every villager in the search AABB on a successful spawn, including ones that didn't meet the sleep-recency gate; only replicating it for the `agreeing` subset undercounts who's on cooldown next time.
9. **Restock's "new day" check is a double condition (raw gameTime *or* calendar period), and restock scheduling itself is doubly probabilistic** (§3.9: `WorkAtPoi`'s ≥300-tick-and-50%-roll gate on top of the `shouldRestock`/`allowedToRestock` state machine) — an implementation that restocks deterministically the instant conditions are met, without reproducing the outer sampling gate, will restock measurably more often/earlier than vanilla.
10. **`GOLEM_KILLED` is dead code; don't build reputation plumbing for it.** Confirmed zero call sites in 26.2 — implementing it "for completeness" risks inventing behavior vanilla doesn't have.
11. **Gossip merge-on-add vs merge-on-transfer use different reducers** (`min(sum, max)`-equivalent clamp for direct `add`, vs. `max(old, new)` for `transferFrom`) — using the same reducer for both (an easy copy-paste mistake given how similar the two code paths look) changes long-run gossip values whenever a villager both directly witnesses events *and* receives gossip about the same player from others.
