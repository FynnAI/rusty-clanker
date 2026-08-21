# Combat & Damage Formulas — Vanilla 26.2 Deep Research

## 1. Purpose

Combat is the subsystem where "close but not exact" is most visible to a player: half a heart off, one tick of invulnerability wrong, a missing armor-toughness term, or a knockback vector computed in the wrong order all produce a result a veteran player will notice within seconds. Unlike worldgen or lighting, where a bug can hide for a while, damage numbers and knockback vectors are checked against community-known formulas constantly (DPS calculators, PvP guides, speedrun routes). This document traces the **entire** server-authoritative damage pipeline — from the moment `LivingEntity.hurtServer` is called to the moment health is actually decremented — in exact evaluation order, with every constant, every float/double cast point, and every RNG call identified from the decompiled 26.2 source. It also covers the attacker-side assembly of melee/projectile/fall damage, difficulty scaling, natural regeneration, and the Instant Health/Harming formulas, since all of these feed values into the same pipeline.

This is the single most cross-cutting domain in the game: it touches `world/entity` (health/attributes), `world/damagesource` (types, combat rules), `world/item/enchantment` (data-driven EPF), `world/item` (weapon/shield components), `world/food` (regen), and `world/DifficultyInstance` (scaling). Every one of those must independently reproduce the exact float sequence below or damage-per-hit will drift.

## 2. Where it lives

| Package / class | Responsibility |
|---|---|
| `net.minecraft.world.entity.LivingEntity` | `hurtServer` (the entry point), `actuallyHurt` (armor/magic absorb + absorption + health decrement, mob variant), invulnerability bookkeeping (`invulnerableTime`, `lastHurt`, `hurtTime`/`hurtDuration`), `knockback(...)`, `causeFallDamage`/`calculateFallDamage`, `applyItemBlocking` (shield), `getDamageAfterArmorAbsorb`/`getDamageAfterMagicAbsorb`, `getKnockback` |
| `net.minecraft.world.entity.player.Player` | `attack(Entity)` (melee assembly: charge scale, crit, sweep, knockback), `stabAttack` (piercing/spear secondary attack), `actuallyHurt` (player-specific override — armor→magic→absorption→exhaustion/combat-tracker/health), `hurtServer` override (difficulty scaling of *incoming* damage), `getAttackStrengthScale`/`getCurrentItemAttackStrengthDelay`, `causeFoodExhaustion` |
| `net.minecraft.server.level.ServerPlayer` | `getEnchantedDamage` (wires `Player.attack`'s magic-boost calc to `EnchantmentHelper.modifyDamage`), `indicateDamage` (hurt-direction tilt) |
| `net.minecraft.world.entity.Mob` | `doHurtTarget` — the generic mob melee path (deterministic, no crit/sweep/charge) |
| `net.minecraft.world.damagesource.DamageSource` / `DamageSources` | Per-hit envelope (`type`, `causingEntity`, `directEntity`, `damageSourcePosition`); `scalesWithDifficulty()`, `getFoodExhaustion()` |
| `net.minecraft.world.damagesource.DamageType` (+ `DamageScaling`, `DamageEffects`, `DeathMessageType`) | Data-driven registry entries (`data/minecraft/damage_type/*.json`): message id, scaling mode, exhaustion cost |
| `net.minecraft.world.damagesource.CombatRules` | The two pure math functions: `getDamageAfterAbsorb` (armor+toughness) and `getDamageAfterMagicAbsorb` (enchantment/resistance-style flat EPF reduction) |
| `net.minecraft.world.damagesource.CombatTracker` / `CombatEntry` | Rolling hit log used to compose death messages (not damage math itself) |
| `net.minecraft.world.item.component.BlocksAttacks` | Shield/blocking-item data component: angle-gated damage reduction, item-damage function, block-delay, axe-disable interaction |
| `net.minecraft.world.item.component.Weapon` | `item_damage_per_attack`, `disable_blocking_for_seconds` (what makes axes disable shields) |
| `net.minecraft.world.item.component.PiercingWeapon` / `KineticWeapon` | New 26.x multi-target "stab" weapons (spear-family) |
| `net.minecraft.world.item.MaceItem` | Smash-attack fall-damage-to-melee-damage conversion + area knockback |
| `net.minecraft.world.item.enchantment.EnchantmentHelper` | Static dispatch surface: `modifyDamage`, `modifyArmorEffectiveness`, `modifyKnockback`, `getDamageProtection`, `doPostAttackEffects`, `processProjectileSpread`/`Count` |
| `net.minecraft.world.item.enchantment.Enchantment` | Per-enchantment effect evaluation (`modifyDamageProtection`, `modifyDamage`, `modifyKnockback`, `modifyArmorEffectivness`) against `LootContext`-based conditional effects |
| `net.minecraft.world.item.enchantment.effects.*` (`AddValue`, `MultiplyValue`, …) | The actual per-enchantment value-transform primitives (`EnchantmentValueEffect.process`) |
| `net.minecraft.world.item.enchantment.LevelBasedValue` (`Linear`, `Fraction`, `Clamped`, `LevelsSquared`, `Exponent`) | The level→float curve types enchantment JSON `"value"` fields select |
| `net.minecraft.world.entity.projectile.arrow.AbstractArrow` | `onHitEntity` (velocity-scaled damage, crit-arrow RNG, Power enchant), `doKnockback` (Punch enchant) |
| `net.minecraft.world.item.BowItem` / `ProjectileWeaponItem` | Charge→power curve, crit-arrow trigger condition, shot spread RNG |
| `net.minecraft.world.entity.projectile.Projectile` | `shootFromRotation`/`getMovementToShoot` — inaccuracy RNG (`random.triangle`) |
| `net.minecraft.world.food.FoodData` / `FoodConstants` | Natural regeneration tick logic, all exhaustion-cost constants |
| `net.minecraft.world.effect.HealOrHarmMobEffect` | Instant Health / Instant Damage math (including undead inversion) |
| `net.minecraft.world.DifficultyInstance` / `Difficulty` | `getEffectiveDifficulty()` (local-difficulty scalar used by mob spawning/behavior, not player damage scaling directly) |
| `net.minecraft.world.entity.ai.attributes.Attributes` | All the numeric attribute defaults this document depends on (armor, toughness, knockback, gravity, safe-fall-distance, fall-damage-multiplier, sweeping-damage-ratio, attack-speed, …) |

## 3. The mechanics

### 3.1 Top-level pipeline (call order)

```
LivingEntity.hurtServer(level, source, damage)
├─ isInvulnerableTo(level, source)?           → false, no-op
├─ isDeadOrDying()?                           → false, no-op
├─ fire-resistance vs IS_FIRE source?         → false, no-op
├─ wake from sleep if sleeping
├─ noActionTime = 0; damage = max(damage, 0)
├─ damageBlocked = applyItemBlocking(...)     (§3.3, shield)
│  damage -= damageBlocked
├─ if source.is(IS_FREEZING) && victim.is(FREEZE_HURTS_EXTRA_TYPES): damage *= 5.0F
├─ if source.is(DAMAGES_HELMET) && helmet non-empty:
│     hurtHelmet(source, damage)              (durability, on the PRE-reduction damage)
│     damage *= 0.75F
├─ NaN/Infinite → damage = Float.MAX_VALUE
├─ invulnerability top-up gate                 (§3.2)
│     → actuallyHurt(level, source, <full or delta damage>)   (§3.4–3.7)
├─ resolveMobResponsibleForDamage / resolvePlayerResponsibleForDamage
├─ if tookFullDamage:
│     onBlocked() sound  XOR  broadcastDamageEvent()
│     markHurt() (unless NO_IMPACT and fully blocked)
│     dealDefaultKnockback(source, damage, blocked)   (§3.10, impulse #1)
├─ death check → checkTotemDeathProtection → die()   (or) hurt sounds
├─ record lastDamageSource/lastDamageStamp; onMobHurt() per active effect
└─ stats/criteria triggers (DAMAGE_BLOCKED_BY_SHIELD, ENTITY_HURT_PLAYER, PLAYER_HURT_ENTITY)
```

`ServerPlayer`/`Player` intercepts this at two points: `Player.hurtServer` wraps the call and applies **incoming-damage difficulty scaling** (§3.8) before delegating to `super.hurtServer`; `Player.actuallyHurt` fully replaces `LivingEntity.actuallyHurt` for the armor/magic/absorption/health step (§3.7 documents the divergence from the mob path).

All damage values in this pipeline are **`float`**. `CombatRules` and armor/toughness math stay in `float` throughout; only knockback vectors and a few movement-adjacent calculations use `double`.

### 3.2 Invulnerability window & the 10-tick top-up rule

Fields: `invulnerableTime` (int, ticks remaining), `lastHurt` (float, the damage value of the most recent hit), `hurtTime`/`hurtDuration` (red-flash animation timer, always set to 10 on a full hit, unrelated to invulnerability logic).

```
if invulnerableTime > 10 and !source.is(BYPASSES_COOLDOWN):
    if damage <= lastHurt:
        return false                          # hit fully absorbed, no-op
    actuallyHurt(level, source, damage - lastHurt)   # only the DELTA is dealt
    lastHurt = damage
    # invulnerableTime is NOT reset here — it keeps counting down
else:
    lastHurt = damage
    invulnerableTime = 20
    actuallyHurt(level, source, damage)        # full damage dealt
    hurtDuration = 10; hurtTime = 10
```

The condition is `invulnerableTime > 10`, **not** `> 0`. Since a fresh hit sets `invulnerableTime = 20` and it decrements once per tick (`LivingEntity.tick()`, except `ServerPlayer` decrements its own copy inside its own `tick()`), the **top-up branch is only active for the first 10 of those 20 ticks** (ticks 20→11 after the hit). Once `invulnerableTime` has decayed to 10 or below, the *next* hit is treated as **entirely fresh**: `lastHurt` and `invulnerableTime` are both reset from scratch and the *full* damage is dealt again, restarting the whole 20-tick cycle. The practical effect matches vanilla's well-known "0.5s i-frame": a second hit within 10 ticks either does nothing (weaker/equal) or does only the excess above the first hit's damage; a hit landing 11–20 ticks after the previous one is a full-price fresh hit. `BYPASSES_COOLDOWN` (`DamageTypeTags.BYPASSES_COOLDOWN`) is currently an **empty tag** in vanilla 26.2 data — no shipped damage type bypasses the window; it exists purely as a datapack/mod extension point.

`invulnerableTime > 10.0F` is compared as a float even though the field is an `int` — no precision hazard since it's a whole-number comparison, but note the literal is `10.0F` in source.

### 3.3 Shield blocking (`BlocksAttacks` data component)

A living entity is "blocking with" an item (`getItemBlockingWith()`) only while actively using it (`isUsingItem()`), the used item carries `minecraft:blocks_attacks`, **and** enough ticks have elapsed since the item was raised: `elapsedTicks = useDuration - useItemRemaining >= blockDelayTicks()` where `blockDelayTicks() = round(blockDelaySeconds * 20)`. Shield's `block_delay_seconds = 0.25` → **5 ticks** of raise-up delay before blocking is active.

`applyItemBlocking(level, source, damage)`:
1. `damage <= 0` → return 0 (nothing to block).
2. No blocking item → return 0.
3. If `bypassedBy` tag set contains the source's damage type → return 0 (shield's default: `#minecraft:bypasses_shield`, which includes `#bypasses_armor`, cactus, campfire, dry-out, falling anvil, falling stalactite, hot floor, sulfur-cube-hot, in-fire, lava, lightning bolt, sweet berry bush).
4. If the direct entity is a piercing arrow (`getPierceLevel() > 0`) → return 0 (piercing arrows shield-ignore).
5. Compute the horizontal angle between the blocker's view direction and the vector to the damage source's position (both flattened to the XZ plane and normalized before the dot product; `angle = acos(dot)`, radians). **If the source has no fixed position** (`source.getSourcePosition() == null`), `angle` is hard-set to `π` (180°) — i.e. positionless damage sources can never be blocked by a directional shield.
6. For each `DamageReduction(horizontalBlockingAngle, type, base, factor)` in `damageReductions` (shield's default is a single entry `(90.0, none, base=0.0, factor=1.0)`):
   - `angle > radians(horizontalBlockingAngle)` → contributes 0.
   - else if `type` present and the source's damage type isn't in it → contributes 0.
   - else contributes `clamp(base + factor * dealtDamage, 0, dealtDamage)`.
   - Contributions from all entries are **summed**, then the total is clamped to `[0, dealtDamage]`.
7. Shield's single default entry with `factor=1.0` means: **full block** (100% of damage) whenever the attack arrives within 90° of the blocker's forward view — an all-or-nothing gate, not a partial reduction curve.
8. `hurtBlockingItem` then applies durability: `itemDamage = itemDamage.apply(blockedAmount)` where the default `ItemDamageFunction(threshold=1.0, base=0.0, factor=1.0)` computes `apply(d) = d < threshold ? 0 : floor(base + factor*d)`; **shield overrides this to `threshold=3.0, base=1.0, factor=1.0`**, so a blocked hit under 3 damage costs 0 durability, otherwise `floor(1 + blockedDamage)`.
9. If the blocked amount is > 0, the source isn't a projectile, and the direct entity is a `LivingEntity`, `blockUsingItem` fires → `attacker.blockedByItem(defender, source, damage)`, which knocks the *attacker* back: `attacker.knockback(0.5, attackerX - defenderX, attackerZ - defenderZ, source, damage)` (note: pushes the attacker away from the blocker, magnitude 0.5, **unaffected by any enchantment**).

**Axe-disables-shield**: `LivingEntity.getSecondsToDisableBlocking()` (default implementation) reads the *attacker's* active weapon's `minecraft:weapon` component: returns `weapon.disableBlockingForSeconds()` only if that weapon is also the currently-active-use item, else 0. Axes carry `disable_blocking_for_seconds = 5.0`; swords/tridents ship `weapon: {}` (defaults: `item_damage_per_attack=1`, `disable_blocking_for_seconds=0.0`). This value flows into `Player.blockUsingItem` → `BlocksAttacks.disable(level, defender, secondsToDisableBlocking, blockingItem)`: `cooldownTicks = round(secondsToDisableBlocking * disableCooldownScale * 20)` (shield's `disable_cooldown_scale` defaults to 1.0) → **100 ticks** (5s) shield-item cooldown via `player.getCooldowns().addCooldown(...)`, plus `user.stopUsingItem()` (forces the shield down immediately) and the disable sound. Only fires when the blocked hit actually landed with `damageBlocked > 0` (via `blockUsingItem`, called from `applyItemBlocking` step 9's sibling branch) — the Warden overrides `getSecondsToDisableBlocking()` with its own (larger) value for its melee attack.

### 3.4 Armor absorption (`CombatRules.getDamageAfterAbsorb`)

Only runs if the source doesn't carry `DamageTypeTags.BYPASSES_ARMOR` (tag contents: on-fire, in-wall, cramming, drown, fly-into-wall, generic, wither, dragon-breath, starve, fall, ender-pearl, freeze, stalagmite, magic, indirect-magic, out-of-world, generic-kill, sonic-boom, outside-border — i.e. most "environmental"/status damage types bypass armor entirely, and notably **fall damage bypasses armor** but not enchantment protection).

Durability is applied to armor **before** the mitigation math, using the **pre-absorb** damage value: `hurtArmor(source, damage)` → `doHurtEquipment`: for each affected slot, `durabilityDamage = max(1, floor(damage/4))` (int) applied via `hurtAndBreak` if the piece is damageable and `canBeHurtBy(source)`.

Then, all in `float`:
```
toughness      = 2.0 + armorToughness / 4.0                       # BASE_ARMOR_TOUGHNESS = 2.0
realArmor      = clamp(totalArmor - damage/toughness, totalArmor*0.2, 20.0)   # MIN_ARMOR_RATIO=0.2, MAX_ARMOR=20.0
armorFraction  = realArmor / 25.0                                  # ARMOR_PROTECTION_DIVIDER = 25.0
if source carries a weapon item (and level is a ServerLevel):
    armorFraction = clamp(EnchantmentHelper.modifyArmorEffectiveness(level, weapon, victim, source, armorFraction), 0.0, 1.0)
damage = damage * (1.0 - armorFraction)
```
`totalArmor = Mth.floor(getAttributeValue(Attributes.ARMOR))` — the ARMOR attribute is floored to an int **before** being handed to the formula, even though the formula itself is float. The `totalArmor*0.2` floor means armor mitigation can never be reduced below 20% of its nominal value purely by stacking damage, no matter how large the hit. `modifyArmorEffectiveness` is the hook Breach (mace-only enchant, `-0.15`/level, additive, uncapped low end but clamped `[0,1]` after) uses to punch through armor.

### 3.5 Resistance effect (first stage of "magic absorb")

Inside `getDamageAfterMagicAbsorb` (§3.6), *before* the enchantment-protection step, and only if the source doesn't carry `BYPASSES_EFFECTS`:
```
if hasEffect(RESISTANCE) and !source.is(BYPASSES_RESISTANCE):
    absorbValue = (amplifier + 1) * 5          # int; amplifier 0 = Resistance I
    absorb      = 25 - absorbValue             # int
    v           = damage * absorb              # float * int → float
    damage      = max(v / 25.0, 0.0)           # float
```
This is exactly **20% reduction per effect level** (amplifier+1), floor-clamped at 0 — Resistance V (amplifier 4) zeroes `absorb` and thus all damage from non-bypassing sources; amplifier ≥5 would go negative but `max(...,0)` prevents any damage *increase*. Stat bookkeeping: the resisted delta is awarded to `DAMAGE_RESISTED` (victim) or `DAMAGE_DEALT_RESISTED` (attacker, if the source's causing entity is a player) via `Math.round(delta * 10)`.

### 3.6 Enchantment "magic" protection (data-driven EPF)

`getDamageAfterMagicAbsorb(damageSource, damage)`:
1. `damageSource.is(BYPASSES_EFFECTS)` → return unchanged.
2. Apply Resistance (§3.5).
3. `damage <= 0` → return 0.
4. `damageSource.is(BYPASSES_ENCHANTMENTS)` → return unchanged.
5. Otherwise, on a `ServerLevel`: `enchantmentArmor = EnchantmentHelper.getDamageProtection(level, victim, source)`; if `> 0`, `damage = CombatRules.getDamageAfterMagicAbsorb(damage, enchantmentArmor)`.

`getDamageProtection` walks **every equipped item in every `EquipmentSlot`** on the victim (`runIterationOnEquipment`), and for each enchantment on each item, evaluates that enchantment's `minecraft:damage_protection` effect list (a list of `ConditionalEffect<EnchantmentValueEffect>`, each gated by a `LootItemCondition` — typically `damage_source_properties` checking damage-type tags). Every matching effect calls `EnchantmentValueEffect.process(level, random, currentValue)` against one **shared `MutableFloat` accumulator** — i.e. contributions from *every armor piece and every enchantment on it* are summed **additively**, uncapped at this stage. The standard effect shape (`AddValue`) does `result += LevelBasedValue.calculate(enchantLevel)`, and the vanilla protection-family enchants all use `type: "minecraft:linear"`: `calculate(level) = base + perLevelAboveFirst * (level - 1)`.

`CombatRules.getDamageAfterMagicAbsorb(damage, totalMagicArmor)`:
```
realArmor = clamp(totalMagicArmor, 0.0, 20.0)
return damage * (1.0 - realArmor / 25.0)
```
Same 20-cap / 25-divisor shape as physical armor, but **no toughness term and no per-weapon armor-effectiveness hook** — this is a separate, simpler formula from §3.4. Because the EPF sum is uncapped *before* this clamp, stacking e.g. 4 pieces of Protection IV (EPF 4 each → sum 16) still lands under the cap, but mixing Protection IV + a situational enchant (Blast/Fire/Projectile Protection IV, EPF 8 each) on the *matching* damage type can exceed 20 and saturate at the 80%-reduction ceiling.

Per-type EPF values (all `linear(base, perLevelAboveFirst)`, `max_level` in parentheses), read directly from `data/minecraft/enchantment/*.json`:

| Enchantment | Applies when (damage-type tag) | EPF at level 1 / per extra level | Max level → max EPF |
|---|---|---|---|
| Protection | not `bypasses_invulnerability` (i.e. almost everything) | 1 / +1 | 4 → 4 |
| Fire Protection | `is_fire` | 2 / +2 | 4 → 8 |
| Blast Protection | `is_explosion` | 2 / +2 | 4 → 8 |
| Projectile Protection | `is_projectile` | 2 / +2 | 4 → 8 |
| Feather Falling | `is_fall` | 3 / +3 | 4 → 12 |

Protection, Fire/Blast/Projectile Protection are mutually exclusive with each other via `exclusive_set: #minecraft:exclusive_set/armor` (cannot be combined **on the same item**, but different pieces of armor can each carry a different member of the set, and all of them stack additively with each other and with Feather Falling — Feather Falling is *not* in that exclusive set). Feather Falling's fall-damage reduction is **not** a special-cased multiplier — it is exactly this same generic EPF pipeline, gated on the `is_fall` tag, which is why fall damage's `BYPASSES_ARMOR` exemption (§3.4) does not also exempt it from enchantment protection.

Fire/Blast Protection additionally carry a `minecraft:attributes` effect (`burning_time` × `(1 - 0.15*level)` for Fire Protection; `explosion_knockback_resistance += 0.15*level` for Blast Protection) — these are attribute-modifier side effects independent of the damage-reduction math above.

### 3.7 Absorption hearts, exhaustion, and the health decrement — and the player/mob divergence

`LivingEntity.actuallyHurt` (used by **mobs**, i.e. anything that doesn't override it) after armor+magic absorb (§3.4–3.6) have already reduced `dmg`:
```
originalDamage = dmg
dmg = max(dmg - absorptionAmount, 0.0)
setAbsorptionAmount(absorptionAmount - (originalDamage - dmg))      # subtract exactly the absorbed portion
absorbed = originalDamage - dmg
if absorbed > 0 and source.getEntity() is ServerPlayer: award DAMAGE_DEALT_ABSORBED stat
if dmg != 0:
    combatTracker.recordDamage(source, dmg)
    setHealth(getHealth() - dmg)
    setAbsorptionAmount(absorptionAmount - dmg)        # ← SECOND subtraction, mob path only
    gameEvent(ENTITY_DAMAGE)
```

`Player.actuallyHurt` (used by **players**, fully overrides the above rather than calling `super`):
```
dmg = getDamageAfterArmorAbsorb(source, dmg)
dmg = getDamageAfterMagicAbsorb(source, dmg)
originalDamage = dmg
dmg = max(dmg - absorptionAmount, 0.0)
setAbsorptionAmount(absorptionAmount - (originalDamage - dmg))      # only this ONE subtraction
absorbed = originalDamage - dmg
if absorbed > 0: award DAMAGE_ABSORBED stat (self, not DEALT_ABSORBED)
if dmg != 0:
    causeFoodExhaustion(source.getFoodExhaustion())
    combatTracker.recordDamage(source, dmg)
    setHealth(getHealth() - dmg)
    if dmg finite: award DAMAGE_TAKEN stat
    gameEvent(ENTITY_DAMAGE)
```

**The mob path decrements `absorptionAmount` a second time**, by the full post-absorption health-damage amount, on top of the first (absorbed-portion) decrement — a mob's Absorption pool therefore drains *faster than the damage it actually blocked* (it loses `blockedAmount + healthDamageAmount` total per hit, not just `blockedAmount`). The player path has no such second subtraction: a player's Absorption pool drains by exactly the amount it blocked, and never more. This is a genuine, source-verified asymmetry between the two code paths — reproduce it exactly as written, not as "probably a mistake that should be unified."

**Final consolidated order for a player taking damage**, end to end:
1. Shield block (subtracts a flat, angle-gated amount) — §3.3
2. Freezing 5× / helmet 0.75× multipliers on the remainder — §3.1
3. Invulnerability top-up gate selects full-damage vs. delta-only — §3.2
4. Armor absorption (toughness formula + weapon armor-effectiveness enchant) — §3.4
5. Resistance effect (20%/level, floor 0) — §3.5
6. Enchantment protection EPF (summed, capped 0–20, `/25`) — §3.6
7. Absorption hearts consume the remainder first — §3.7
8. Exhaustion cost + `CombatTracker` entry + `setHealth` — §3.7

### 3.8 Difficulty scaling of incoming player damage

`Player.hurtServer` (override, wraps `super`) runs **before** any of §3.1's item-blocking/freeze/helmet logic (it's the outermost layer): if `source.scalesWithDifficulty()` (see §3.7 registry semantics below) is true:
```
PEACEFUL: damage = 0.0
EASY:     damage = min(damage/2.0 + 1.0, damage)
HARD:     damage = damage * 3.0 / 2.0
# NORMAL: unchanged
```
then, if the scaled damage is exactly `0.0F`, the call short-circuits to `false` (no hit at all — no invulnerability consumption, no animation) rather than proceeding through `super.hurtServer` with a zero. `scalesWithDifficulty()` (`DamageSource`) is:
```
NEVER                         → false
WHEN_CAUSED_BY_LIVING_NON_PLAYER → causingEntity is a LivingEntity and not a Player
ALWAYS                        → true
```
Most damage types (mob attacks, projectiles, fire, cactus, …) are `WHEN_CAUSED_BY_LIVING_NON_PLAYER` — i.e. difficulty scaling **only applies when a non-player mob is the cause**, so mob-vs-player combat scales with difficulty but player-vs-player and most environmental damage does not, except the small set of `ALWAYS`-scaling types (`explosion`, `player_explosion`, `sonic_boom`, `bad_respawn_point`).

### 3.9 Player melee attack assembly (`Player.attack`)

Exact order (all `float` unless noted):
```
baseDamage = isAutoSpinAttack() ? autoSpinAttackDmg : getAttributeValue(ATTACK_DAMAGE)     # raw, attribute already includes weapon's ADD_VALUE/ADD_MULTIPLIED_* modifiers
weapon      = getWeaponItem()
source      = weapon.getDamageSource(this)
chargeScale = getAttackStrengthScale(0.5)                                                  # §3.9a
magicBoost  = chargeScale * (getEnchantedDamage(target, baseDamage, source) - baseDamage)   # computed against UNSCALED baseDamage
baseDamage *= baseDamageScaleFactor()                                                      # §3.9a, charge curve applied HERE
onAttack()                                                                                  # resets attackStrengthTicker to 0
if not deflected (see §3.14 for projectile deflection):
  if baseDamage > 0 or magicBoost > 0:
    fullStrength   = chargeScale > 0.9
    knockbackAttack = isSprinting() and fullStrength                 # plays KNOCKBACK sound, +0.5 knockback later
    baseDamage += weapon.getItem().getAttackDamageBonus(target, baseDamage, source)   # Mace smash bonus lives here (§3.11); 0 for ordinary weapons
    critical = fullStrength and canCriticalAttack(target)            # §3.9b
    if critical: baseDamage *= 1.5
    totalDamage = baseDamage + magicBoost
    sweep = isSweepAttack(fullStrength, critical, knockbackAttack)   # §3.9c
    wasHurt = target.hurtOrSimulate(source, totalDamage)
    if wasHurt:
       causeExtraKnockback(target, getKnockback(target, source) + (knockbackAttack ? 0.5 : 0.0), ..., true)   # §3.10, impulse #2
       if sweep: doSweepAttack(target, baseDamage, source, chargeScale)                                       # §3.9d — note: uses baseDamage AFTER bonus+crit, NOT totalDamage
       attackVisualEffects(...)   # crit()/magicCrit() client hooks
       setLastHurtMob(target)
       itemAttackInteraction(...)  # weapon.hurtEnemy + doPostAttackEffects (Thorns, Fire Aspect, etc.)
       damageStatsAndHearts(...)   # DAMAGE_DEALT stat = round(actualHealthDrop * 10)
       causeFoodExhaustion(0.1)    # EXHAUSTION_ATTACK
```
Two subtleties worth flagging explicitly: (1) `magicBoost` is computed from the **pre-charge-curve** `baseDamage`, then itself scaled by the *same* `chargeScale` fraction — so Sharpness-style bonus damage scales with attack-cooldown charge exactly like the base hit does, but via an independently-scaled delta rather than being folded into the charge multiplication. (2) The weapon's `getAttackDamageBonus` (Mace fall bonus) is added **after** the charge-curve multiply but **before** the crit ×1.5 — a critical Mace smash multiplies the *combined* base+fall-bonus damage by 1.5, not just the base attribute portion.

#### 3.9a Attack-cooldown charge curve

```
attackStrengthDelay(ticks) = 1.0 / getAttributeValue(ATTACK_SPEED) * 20.0        # ATTACK_SPEED default 4.0 → 5-tick delay
chargeScale(a)  = clamp((attackStrengthTicker + a) / attackStrengthDelay, 0.0, 1.0)
baseDamageScaleFactor() = 0.2 + chargeScale(0.5)^2 * 0.8
```
`attackStrengthTicker` increments by 1 every player tick and resets to 0 on `onAttack()` (fired at the start of every `attack()` call, i.e. every left-click swing that reaches damage assembly, regardless of whether it deals damage). The `0.5` offset in `getAttackStrengthScale(0.5F)` accounts for the fractional tick position at which the attack packet is processed relative to the last tick boundary.

#### 3.9b Critical hit condition

`canCriticalAttack(target)` requires **all** of: `fallDistance > 0.0`, not on ground, not on a climbable, not in water, not mobility-restricted, not a passenger, target is a `LivingEntity`, and **not sprinting**. Combined with the outer `fullStrength` (charge > 0.9) gate. On success: `baseDamage *= 1.5F` and the client-visible `crit()` hook fires (spawns crit particles) plus the `PLAYER_ATTACK_CRIT` sound. There is **no RNG involved** in the player crit condition — it is a fully deterministic function of movement/state.

#### 3.9c Sweep-attack condition

`isSweepAttack(fullStrength, critical, knockbackAttack)`: requires `fullStrength && !critical && !knockbackAttack && onGround()`, then computes `approxSpeedSq = getKnownMovement().horizontalDistanceSqr()` (the player's **last client-reported** movement, not raw server velocity — an anti-cheat-aware value) and requires `approxSpeedSq < (getSpeed() * 2.5)^2`, and finally requires the main-hand item to carry `ItemTags.SWORDS`. All four gates are deterministic; no RNG.

#### 3.9d Sweep damage to nearby entities

```
ratioAttr = getAttributeValue(SWEEPING_DAMAGE_RATIO)          # 0.0 unless Sweeping Edge is worn
sweepBase = 1.0 + ratioAttr * baseDamage                      # baseDamage = post-charge, post-bonus, post-crit main-hit value
for each LivingEntity within entity.getBoundingBox().inflate(1.0, 0.25, 1.0) where distanceToSqr(attacker) < 9.0,
        excluding self, the primary target, allies, and marker armor stands:
    perTargetDamage = getEnchantedDamage(nearby, sweepBase, source) * chargeScale
    if nearby.hurtServer(level, source, perTargetDamage):
        nearby.knockback(0.4, sin(yaw), -cos(yaw), source, perTargetDamage)   # attacker-yaw-directed, NOT source-relative
        EnchantmentHelper.doPostAttackEffects(level, nearby, source)
```
`getEnchantedDamage` on the sweep path re-invokes the **same** generic `EnchantmentHelper.modifyDamage` used for the primary target — Sharpness/Smite/Bane bonuses are computed **independently per swept target**, not derived as a fraction of the main hit's bonus. Sweeping Edge itself is implemented purely as an `ADD_VALUE` attribute modifier on `SWEEPING_DAMAGE_RATIO` with value `numerator/denominator = level/(level+1)` (`Fraction` of two `Linear` curves: numerator `base=1, perLevel=1`; denominator `base=2, perLevel=1`) — levels 1/2/3 → ratios 1/2, 2/3, 3/4.

### 3.10 Knockback — the two-stack impulse and vector math

Melee knockback is applied in **two separate calls** to `LivingEntity.knockback`, chronologically:

**Impulse #1 — generic hurt knockback**, fired unconditionally for *any* successful `hurtServer` call (not just melee — projectiles, explosions with a position, etc.) from inside `dealDefaultKnockback` (§3.1 step): direction is `(sourcePosition - victimPosition)` in the XZ plane if the source has a fixed position, or (for a `Projectile` direct entity) `-calculateHorizontalHurtKnockbackDirection(...)`; magnitude is a flat **0.4**.

**Impulse #2 — attacker-directed extra knockback**, fired only from `Player.attack`/`Mob.doHurtTarget` **after** the hit already landed (i.e. strictly after impulse #1 has already mutated the target's velocity): direction is `(sin(attackerYaw), -cos(attackerYaw))` — the **attacker's facing**, not the relative position vector; magnitude is `getKnockback(target, source) + (knockbackAttack ? 0.5 : 0.0)`.

`getKnockback(target, source)`:
```
attr = getAttributeValue(ATTACK_KNOCKBACK)          # 0.0 default; no vanilla weapon sets this by default
result = EnchantmentHelper.modifyKnockback(level, weapon, target, source, attr) / 2.0
```
Knockback/Punch enchant effect: `ADD_VALUE`, `+1.0` per level (`linear(base=1, perLevel=1)`), levels 1–2 for Knockback (melee, no requirement), levels 1–2 for Punch (bow, gated to `direct_attacker` being an arrow entity type). The **halving by 2.0** happens after the enchant addition, inside `getKnockback` itself.

`LivingEntity.knockback(power, xd, zd, source, damage, comesFromEffect)`:
```
power *= (1.0 - getAttributeValue(KNOCKBACK_RESISTANCE))
if power <= 0: return
while xd*xd + zd*zd < 1e-5:                          # degenerate direction guard
    xd = (random.nextDouble() - random.nextDouble()) * 0.01
    zd = (random.nextDouble() - random.nextDouble()) * 0.01
dir = normalize(xd, 0, zd) * power
newVelocity.x = oldVelocity.x / 2.0 - dir.x
newVelocity.z = oldVelocity.z / 2.0 - dir.z
newVelocity.y = onGround() ? min(0.4, oldVelocity.y/2.0 + power) : oldVelocity.y
```
Because each call **halves the entity's existing velocity before subtracting the new impulse**, calling `knockback` twice in the same hit (impulses #1 then #2) is *not* equivalent to a single combined call — the first impulse's contribution is itself halved again by the second call. Reimplementations must call the two impulses in the documented order with the documented (different!) direction vectors to match vanilla bit-for-bit, not merge them into one call. The degenerate-direction RNG fallback consumes the **victim's own** `RandomSource` (2 `nextDouble()` calls per loop iteration; loop is only entered when `xd`/`zd` are both extremely close to zero, e.g. a source exactly on top of the victim), and is entered independently for each of the two impulses if their respective direction vectors happen to be degenerate.

### 3.11 Mace smash attacks

`canSmashAttack(attacker) = attacker.fallDistance > 1.5 && !attacker.isFallFlying()`. When true, `MaceItem.getAttackDamageBonus` (called from step "`baseDamage += weapon.getAttackDamageBonus(...)`" in §3.9) converts accumulated fall distance into bonus melee damage:
```
d = attacker.fallDistance     # double
if d <= 3.0:  bonus = 4.0 * d
elif d <= 8.0: bonus = 12.0 + 2.0 * (d - 3.0)
else:          bonus = 22.0 + (d - 8.0)
bonus += EnchantmentHelper.modifyFallBasedDamage(level, weapon, target, source, 0.0) * d     # Density enchant: +0.5/level per fallen block
return (float) bonus
```
Density is a flat `smash_damage_per_fallen_block` EPF-style additive value (`linear(base=0.5, perLevel=0.5)`), multiplied by `fallDistance` here rather than added once. On landing (`hurtEnemy`, called from `itemAttackInteraction`), a smash attack also: zeroes the attacker's Y-velocity to a small positive nudge, sets `ignoreFallDamageFromCurrentImpulse` (so the attacker doesn't *also* take fall damage from the same fall that just became melee damage), and — if the target is on the ground — triggers a **radius-3.5-block area knockback** against every non-allied, non-passenger `LivingEntity` in range: `knockbackPower = (3.5 - distance) * 0.7 * (fallDistance > 5.0 ? 2 : 1) * (1 - target.KNOCKBACK_RESISTANCE)`, pushed away from the impact point with a flat `0.7` upward component, independent of the main knockback formula in §3.10. `postHurtEnemy` resets `fallDistance` to 0 after a successful smash so the same fall can't be "cashed in" twice.

### 3.12 Piercing / "stab" weapons (spear-family, `PiercingWeapon` component)

A new 26.x melee mechanic distinct from `Player.attack`: `PiercingWeapon.attack(attacker, hand)` raycasts along `getAttackRangeWith(weapon)` and calls `attacker.stabAttack(hand, hitEntity, rawAttackDamageAttributeValue, dealsDamage=true, dealsKnockback, dismounts)` **once per entity hit along the ray** (multi-target, unlike the single-target `Player.attack`). `Player.stabAttack`:
```
magicBoost = getEnchantedDamage(target, baseDamage, source) - baseDamage
if not already mid-use-item with this same slot's weapon:
    magicBoost *= chargeScale(0.5)
    baseDamage *= baseDamageScaleFactor()          # same curve as §3.9a
totalDamage = dealsDamage ? baseDamage + magicBoost : 0.0
wasHurt = dealsDamage && target.hurtOrSimulate(source, totalDamage)
if dealsKnockback:
    causeExtraKnockback(target, 0.4, ..., comesFromEffect=false)              # flat impulse, independent of ATTACK_KNOCKBACK
    causeExtraKnockback(target, getKnockback(target, source), ..., true)      # same enchant-aware impulse as §3.10
```
No crit, no sweep — this path never sets `criticalAttack`/`sweepAttack` flags. `PiercingWeapon.canHitEntity` explicitly excludes already-dead targets, non-projectile-hittable entities, and (mirroring `cannotAttack`) player-vs-player friendly-fire rules.

### 3.13 Mob melee attacks (`Mob.doHurtTarget`)

The generic mob path is **fully deterministic** — no crit, no sweep, no charge curve, no RNG:
```
dmg = getAttributeValue(ATTACK_DAMAGE)
weapon = getWeaponItem()
source = weapon.getDamageSource(this)
dmg = EnchantmentHelper.modifyDamage(level, weapon, target, source, dmg)
dmg += weapon.getItem().getAttackDamageBonus(target, dmg, source)
wasHurt = target.hurtServer(level, source, dmg)
if wasHurt:
    causeExtraKnockback(target, getKnockback(target, source), ..., true)     # impulse #2 only — same formula as §3.10
    if target is LivingEntity: weapon.hurtEnemy(target, this)
    EnchantmentHelper.doPostAttackEffects(level, target, source)
```
Any apparent "variance" in observed mob damage (e.g. a Zombie sometimes hitting harder) comes entirely from `ATTACK_DAMAGE` attribute *ranges* rolled once at spawn/equip time (difficulty-scaled follow-up gear, random enchantments on spawn, etc.), never from a per-swing RNG roll in this method. Individual mob subclasses may override `doHurtTarget` with bespoke behavior (e.g. bosses); this document covers only the shared generic path.

### 3.14 Projectile (arrow) damage

**Charge → velocity** (`BowItem`):
```
timeHeld = useDuration - remainingTime
p = timeHeld / 20.0
power = clamp01( (p*p + p*2.0) / 3.0 )          # saturates to 1.0 at timeHeld >= 20 ticks
isCrit = (power == 1.0F)                         # exact float equality — only a full draw crits
shoot(..., power * 3.0, uncertainty=1.0, isCrit)
```
`Projectile.getMovementToShoot(xd, yd, zd, pow, uncertainty)`: normalizes `(xd,yd,zd)`, then adds **three independent `random.triangle(0.0, 0.0172275 * uncertainty)` samples** (one per axis, consuming the projectile entity's own `RandomSource`, in x/y/z order), then scales the whole vector by `pow`. This is the shot-spread/inaccuracy RNG — 3 calls per shot at `uncertainty=1.0` for a bow (crossbows and dispenser-fired arrows use different `uncertainty` constants but the same triangle-distribution mechanism and axis order).

**On-hit damage** (`AbstractArrow.onHitEntity`):
```
impactSpeed = deltaMovement.length()                       # float, blocks/tick at the moment of impact
arrowDamage = baseDamage                                    # double; default 2.0, or set via setBaseDamageFromMob for mob-fired arrows
if weaponItem present and on ServerLevel:
    arrowDamage = EnchantmentHelper.modifyDamage(level, weaponItem, target, source, (float)arrowDamage)   # Power enchant added HERE, pre-velocity-scale
damage (int) = ceil(clamp(impactSpeed * arrowDamage, 0.0, 2147483647.0))
if pierceLevel > 0: pierced-entity bookkeeping (discard once pierceLevel+1 entities hit)
if isCritArrow():
    dmgIncrease = random.nextInt(damage/2 + 2)              # int division; consumes the ARROW's own RandomSource, 1 call
    damage = min(dmgIncrease + damage, Integer.MAX_VALUE)
entity.hurtOrSimulate(source, damage)                       # → hurtServer pipeline (§3.1) from here
```
Power enchant: `linear(base=1.0, perLevel=0.5)`, gated on the *direct attacker* being an arrow-family entity, added additively to the 2.0 base **before** the velocity multiply — so Power's effective damage contribution scales with arrow speed exactly like the base damage does, not as a flat post-multiply bonus. Crit-arrow RNG (`random.nextInt(damage/2 + 2)`) is a **single call**, only made when `isCritArrow()` is true (set only at full-draw bow release, or explicitly for skeleton/other mob shots that opt in), and draws from the **arrow entity's** RNG stream — not the shooter's, not a shared/global stream.

**Knockback** (`AbstractArrow.doKnockback`, separate from melee's two-impulse model — arrows only ever apply one impulse, and only from the Punch enchant, never a flat base): `knockback = EnchantmentHelper.modifyKnockback(level, firedFromWeapon, target, source, 0.0)`; if `> 0`: `push = normalize(velocity.x, 0, velocity.z) * (knockback * 0.6 * (1 - target.KNOCKBACK_RESISTANCE))`, applied via `target.push(...)` (a raw velocity add, not the halving `knockback()` method used elsewhere) plus a flat `+0.1` Y.

### 3.15 Fall damage

```
calculateFallPower(fallDistance) = fallDistance + 1e-6 - getAttributeValue(SAFE_FALL_DISTANCE)     # double; default SAFE_FALL_DISTANCE = 3.0
calculateFallDamage(fallDistance, damageModifier):
    if entity.is(FALL_DAMAGE_IMMUNE): return 0
    base = calculateFallPower(fallDistance)
    return floor(base * damageModifier * getAttributeValue(FALL_DAMAGE_MULTIPLIER))                # default FALL_DAMAGE_MULTIPLIER = 1.0
```
`damageModifier` is supplied by the **landed-on block**, via `Block.fallOn(level, state, pos, entity, fallDistance)` → `entity.causeFallDamage(fallDistance, damageModifier, damageSources().fall())`. Default `Block.fallOn` uses `damageModifier = 1.0`. Overrides observed in source:

| Block | Override behavior |
|---|---|
| Bed | Halves `fallDistance` itself (`fallDistance * 0.5`) before delegating to the default `1.0`-modifier path — i.e. the *distance* is halved, not the modifier |
| Hay Bale | `damageModifier = 0.2` |
| Slime Block | `damageModifier = 0.0` **unless** the entity is sneaking (`isSuppressingBounce()`), in which case fall damage is dealt normally (sneaking suppresses the bounce, restoring damage) |
| Powder Snow | Doesn't call `causeFallDamage` at all — only plays a landing sound if `fallDistance >= 4.0` — landing in powder snow is fully fall-damage-immune regardless of distance |

`LivingEntity.causeFallDamage` (override) additionally clamps the effective fall distance against `currentImpulseImpactPos` when the entity is in a "current-impulse grace" state (set by Mace smash attacks and similar mechanics, §3.11) — this is what prevents a Mace smash from *also* triggering a separate fall-damage hit for the same fall. The final `dmg = calculateFallDamage(effectiveFallDistance, damageModifier)`; if `> 0`, fall/landing sounds play and `hurt(damageSource, dmg)` is called (re-entering the full §3.1 pipeline — meaning fall damage, like any other damage, still passes through Feather Falling's EPF reduction even though it bypasses physical armor, per §3.4/§3.6).

**Jump Boost** has no direct formula term anywhere in this pipeline — it only increases the player's initial upward jump velocity (a movement-layer effect), which indirectly produces a larger accumulated `fallDistance` if the player subsequently falls back down without landing on anything in between. There is no vanilla mechanic that reduces fall damage *because* Jump Boost is active.

### 3.16 Local difficulty scalar (`DifficultyInstance.getEffectiveDifficulty`)

Not used for player-damage scaling directly (that's §3.8's simpler 3-branch switch) — this is the continuous 0.0–~6.75 scalar vanilla uses for spawn-rate/loot/behavior scaling (e.g. zombie reinforcement chance, phantom spawn thresholds). Included here because the assignment's domain explicitly calls it out and because mob-attack-adjacent behaviors reference it.

```
Difficulty ids: PEACEFUL=0, EASY=1, NORMAL=2, HARD=3

calculateDifficulty(base, totalGameTime, localGameTime, moonBrightness):
    if base == PEACEFUL: return 0.0
    isHard = base == HARD
    scale = 0.75
    globalScale = clamp((totalGameTime - 72000.0) / 1440000.0, 0.0, 1.0) * 0.25
    scale += globalScale
    localScale = clamp(localGameTime / 3600000.0, 0.0, 1.0) * (isHard ? 1.0 : 0.75)
    localScale += clamp(moonBrightness * 0.25, 0.0, globalScale)
    if base == EASY: localScale *= 0.5
    scale += localScale
    return base.getId() * scale                    # float; base.getId() ∈ {1,2,3} for non-peaceful
```
`localGameTime` = the chunk's `inhabitedTime` counter (accumulated ticks any player has spent near that chunk — resets per-chunk, not global); `moonBrightness` comes from `ServerLevel.getMoonBrightness(pos)` (dimension-dependent moon-phase curve; non-overworld dimensions typically return 0). `getSpecialMultiplier()` (a secondary derived value some mob behaviors read) is `0` below effective-difficulty 2.0, `1` above 4.0, and linearly interpolated `(effectiveDifficulty - 2.0)/2.0` between.

### 3.17 `damage_type` registry — scaling & exhaustion (data-driven)

Every `DamageType` record is `(msgId, DamageScaling, exhaustion: float, DamageEffects, DeathMessageType)`. `DamageScaling` is the 3-value enum used by §3.8 (`NEVER`/`WHEN_CAUSED_BY_LIVING_NON_PLAYER`/`ALWAYS`). `exhaustion` is consumed via `DamageSource.getFoodExhaustion() = type.exhaustion()`, applied in `Player.actuallyHurt` **only when net health damage after all reductions is nonzero** (§3.7) — i.e. a fully-absorbed or fully-blocked hit costs no exhaustion. Full table of all 51 vanilla damage types' `exhaustion`/`scaling` (from `data/minecraft/damage_type/*.json`):

| Damage type | exhaustion | scaling |
|---|---|---|
| arrow, cactus, campfire, dry_out, falling_anvil, falling_block, falling_stalactite, fireball, fireworks, hot_floor, in_fire, lava, lightning_bolt, mace_smash, mob_attack, mob_attack_no_aggro, mob_projectile, player_attack, spear, spit, sting, sulfur_cube_hot, sweet_berry_bush, thorns, thrown, trident, unattributed_fireball, wind_charge, wither_skull | 0.1 | `when_caused_by_living_non_player` |
| bad_respawn_point | 0.1 | `always` |
| explosion, player_explosion | 0.1 | `always` |
| cramming, dragon_breath, drown, fall, freeze, generic, generic_kill, fly_into_wall, in_wall, indirect_magic, magic, on_fire, out_of_world, outside_border, stalagmite, starve, wither | 0.0 | `when_caused_by_living_non_player` |
| sonic_boom | 0.0 | `always` |
| ender_pearl | 0.0 | `when_caused_by_living_non_player` |

(Full per-file breakdown available by regenerating from `data/minecraft/damage_type/*.json`; the table above groups by identical `(exhaustion, scaling)` pairs — every vanilla melee/mob/projectile hit costs the victim **0.1 exhaustion**, matching `FoodConstants.EXHAUSTION_ATTACK`; nearly all pure-environmental/status types cost **0.0**.)

Selected damage-type tags relevant to the pipeline above (all from `data/minecraft/tags/damage_type/`):
- `bypasses_armor`: on_fire, in_wall, cramming, drown, fly_into_wall, generic, wither, dragon_breath, starve, **fall**, ender_pearl, freeze, stalagmite, magic, indirect_magic, out_of_world, generic_kill, sonic_boom, outside_border.
- `bypasses_enchantments`: sonic_boom only.
- `bypasses_resistance` / `bypasses_invulnerability`: out_of_world, generic_kill only (both).
- `bypasses_cooldown`: **empty** — no vanilla type ships in this tag.
- `is_projectile`: arrow, trident, mob_projectile, unattributed_fireball, fireball, wither_skull, thrown, wind_charge.
- `is_fall`: fall, ender_pearl, stalagmite.
- `is_fire`: in_fire, campfire, on_fire, lava, hot_floor, sulfur_cube_hot, unattributed_fireball, fireball.
- `is_explosion`: fireworks, explosion, player_explosion, bad_respawn_point.
- `damages_helmet`: falling_anvil, falling_block, falling_stalactite (drives §3.1's 0.75× helmet reduction).
- `no_knockback`: a large set including explosion/fire/fall/magic/drown/etc. — these never trigger `dealDefaultKnockback` (§3.10 impulse #1).

### 3.18 Natural regeneration & exhaustion (`FoodData`)

`FoodData.tick(player)`, once per player-tick:
```
if exhaustionLevel > 4.0:                              # EXHAUSTION_DROP = 4.0
    exhaustionLevel -= 4.0
    if saturationLevel > 0: saturationLevel = max(saturationLevel - 1.0, 0.0)
    elif difficulty != PEACEFUL: foodLevel = max(foodLevel - 1, 0)

if NATURAL_HEALTH_REGENERATION gamerule:
    if saturationLevel > 0 and player.isHurt() and foodLevel >= 20:
        tickTimer++
        if tickTimer >= 10:                             # HEALTH_TICK_COUNT_SATURATED
            spend = min(saturationLevel, 6.0)            # EXHAUSTION_HEAL = 6.0 is also the max per-tick spend cap
            player.heal(spend / 6.0)
            addExhaustion(spend)
            tickTimer = 0
    elif foodLevel >= 18 and player.isHurt():             # HEAL_LEVEL = 18
        tickTimer++
        if tickTimer >= 80:                              # HEALTH_TICK_COUNT
            player.heal(1.0)
            addExhaustion(6.0)
            tickTimer = 0
    elif foodLevel <= 0:                                  # STARVE_LEVEL = 0
        tickTimer++
        if tickTimer >= 80:
            if health > 10 or difficulty == HARD or (health > 1 and difficulty == NORMAL):
                player.hurtServer(level, damageSources().starve(), 1.0)
            tickTimer = 0
    else: tickTimer = 0
```
Note the **fast-regen branch's heal amount is fractional**: `spend/6.0` where `spend = min(saturation, 6.0)` — a full 6-saturation tick heals exactly 1.0 (half a heart... actually one full heart, since `heal()` takes health points directly and 1.0 = half a heart is wrong: health is tracked in half-heart units of 1.0 per half-heart in some docs, but here `heal(1.0F)` and `MAX_HEALTH` default 20.0 confirms **health points, 2 per heart** — so `heal(spend/6.0)` with `spend=6.0` heals exactly 1 full health point (half a heart) per 10-tick pulse when saturation is available, versus the slow branch's flat 1 health point per 80-tick pulse). Starvation only actually deals damage above certain health/difficulty thresholds — it can bring a player down to 1 HP on Normal or 10 HP-triggered-only-above on Easy-equivalent logic (Easy has no starve-damage branch reachable since the `if` chain's damage-dealing arm requires `difficulty == HARD` or health thresholds that don't gate out Easy — re-read: Easy difficulty *is* included in the "no starve damage" implicit case only via the health-threshold checks, i.e. Easy players can still starve down to 1 HP by the `health > 1` check not being difficulty-gated the same way... the exact gate is: damage fires if `health > 10 OR difficulty==HARD OR (health > 1 AND difficulty==NORMAL)`; on Easy, none of these three conditions can be satisfied once health drops to ≤10 (first condition fails) and difficulty isn't HARD or NORMAL, so **Easy-difficulty starvation damage stops entirely once health reaches 10**, Normal stops at 1, Hard never stops (can starve to death).

Exhaustion cost constants (`FoodConstants`, applied at their respective call sites — walking/sprinting/swimming exhaustion is driven by distance moved per tick in the movement code, not shown here; jump/attack/mine/damage-taken are flat per-event costs):

| Action | Constant | Value |
|---|---|---|
| Attacking (any successful `Player.attack` hit) | `EXHAUSTION_ATTACK` | 0.1 |
| Taking damage | per-damage-type `exhaustion` field (§3.17) | 0.0 or 0.1 (typ.) |
| Jumping | `EXHAUSTION_JUMP` | 0.05 |
| Sprint-jumping | `EXHAUSTION_SPRINT_JUMP` | 0.2 |
| Mining a block | `EXHAUSTION_MINE` | 0.005 |
| Sprinting (per meter) | `EXHAUSTION_SPRINT` | 0.1 |
| Swimming (per meter) | `EXHAUSTION_SWIM` | 0.01 |
| Walking (per meter) | `EXHAUSTION_WALK` | 0.0 |
| Crouch-walking (per meter) | `EXHAUSTION_CROUCH` | 0.0 |
| Regen (fast, saturated) | `EXHAUSTION_HEAL`-equivalent | up to 6.0 per pulse (= spent saturation) |
| Regen (slow, food-only) | `EXHAUSTION_HEAL` | 6.0 per pulse |

`causeFoodExhaustion` is a **no-op for invulnerable players** (`abilities.invulnerable`, e.g. Creative/Spectator) and a client-side no-op (only mutates `foodData` server-side). `addExhaustion` clamps the accumulator at a ceiling of **40.0**.

### 3.19 Instant Health / Instant Damage (potion math)

`HealOrHarmMobEffect` backs both `minecraft:instant_health` and `minecraft:instant_damage`, distinguished only by an `isHarm` flag; both check `isHarm == mob.isInvertedHealAndHarm()` (`isInvertedHealAndHarm()` = `EntityTypeTags.INVERTED_HEALING_AND_HARM`, i.e. undead mobs) to decide whether to heal or harm on **this specific target** — meaning Instant Health harms undead and Instant Damage heals them, computed per-application, not as a separate potion.

**Per-tick application** (used for the lingering-cloud "ticks" path, `applyEffectTick`, always amplitude-integer, no fractional scale):
```
if isHarm == invertedTarget: mob.heal(max(4 << amplification, 0))
else:                        mob.hurtServer(level, damageSources().magic(), 6 << amplification)
```
**Instantaneous application** (drinking a potion or splash/lingering-cloud direct application, `applyInstantaneousEffect`, with a `scale` factor from splash-radius dilution or similar):
```
if isHarm == invertedTarget: amount = (int)(scale * (4 << amplification) + 0.5); mob.heal(amount)
else:
    amount = (int)(scale * (6 << amplification) + 0.5)
    source = damageSources().magic()  if no causing entity  else  damageSources().indirectMagic(source, owner)
    mob.hurtServer(level, source, amount)
```
Both `4 << amplification` and `6 << amplification` are integer left-shifts — i.e. **doubling per potion level**, not a linear per-level add: level 1 (amplification 0) → 4/6, level 2 (amplification 1) → 8/12, level 3 → 16/24, etc. The `scale * value + 0.5` rounding is a manual round-half-up on the double product before truncating to `int`. Both damage/heal paths use the `magic`/`indirect_magic` damage type (exhaustion 0.0, `bypasses_armor` — see §3.4/§3.17 — so Instant Damage always ignores physical armor but is still subject to Resistance and enchantment-protection EPF via §3.5/§3.6, since `magic`/`indirect_magic` do **not** carry `bypasses_effects` or `bypasses_enchantments`).

## 4. Constants table (consolidated)

| Constant | Value | Source |
|---|---|---|
| `MAX_ARMOR` | 20.0 | `CombatRules` |
| `ARMOR_PROTECTION_DIVIDER` | 25.0 | `CombatRules` |
| `BASE_ARMOR_TOUGHNESS` | 2.0 | `CombatRules` (`toughness = 2.0 + armorToughness/4.0`) |
| `MIN_ARMOR_RATIO` | 0.2 | `CombatRules` (floor = `totalArmor * 0.2`) |
| Invulnerability window | 20 ticks set, top-up-only above 10 ticks remaining | `LivingEntity.hurtServer` |
| Hurt-flash duration | 10 ticks (`hurtDuration`/`hurtTime`) | `LivingEntity.hurtServer` |
| Freezing extra-damage multiplier | ×5.0 | `LivingEntity.hurtServer` |
| Damaged-helmet reduction | ×0.75 | `LivingEntity.hurtServer` |
| Shield default block angle | 90° | `shield.json` (`BlocksAttacks.DamageReduction` default) |
| Shield block delay | 0.25s = 5 ticks | `shield.json` |
| Shield item-damage threshold/base/factor | 3.0 / 1.0 / 1.0 | `shield.json` |
| Axe shield-disable duration | 5.0s = 100 ticks | `weapon` component on axes; `disable_cooldown_scale` default 1.0 |
| Resistance reduction per level | 20% (`(amp+1)*5` out of 25) | `LivingEntity.getDamageAfterMagicAbsorb` |
| Protection EPF/level | 1.0 | `protection.json` |
| Fire/Blast/Projectile Protection EPF/level | 2.0 | respective jsons |
| Feather Falling EPF/level | 3.0 | `feather_falling.json` |
| Default `dealDefaultKnockback` magnitude | 0.4 | `LivingEntity.dealDefaultKnockback` |
| Knockback/Punch enchant bonus/level | +1.0, then `/2.0` in `getKnockback` | `knockback.json`/`punch.json`, `LivingEntity.getKnockback` |
| Sprint-attack knockback bonus | +0.5 | `Player.attack` |
| Block-with-item knockback (attacker pushed) | 0.5, uses `LivingEntity.knockback` | `LivingEntity.blockedByItem` |
| Arrow knockback scale | ×0.6, applied via raw `push`, not `knockback()` | `AbstractArrow.doKnockback` |
| Bow charge curve | `(t/20)²·(1)+ (t/20)·2` all `/3`, clamp 1.0; velocity = `power*3.0` | `BowItem.getPowerForTime` |
| Bow inaccuracy triangle half-width | `0.0172275 * uncertainty` per axis | `Projectile.getMovementToShoot` |
| Crit-arrow bonus | `random.nextInt(damage/2 + 2)` added | `AbstractArrow.onHitEntity` |
| Power enchant/level | +1.0 base, +0.5/level, pre-velocity-multiply | `power.json` |
| Sweeping Edge ratio | `level/(level+1)` | `sweeping_edge.json` |
| Sharpness/level | +1.0 base, +0.5/level | `sharpness.json` |
| Smite / Bane of Arthropods /level | +2.5 base, +2.5/level, conditional on target type | respective jsons |
| Breach armor-effectiveness/level | −0.15 base, −0.15/level | `breach.json` |
| Density smash bonus/level (×fall distance) | +0.5 base, +0.5/level | `density.json` |
| Attack cooldown charge curve | `0.2 + charge²·0.8` | `Player.baseDamageScaleFactor` |
| Attack strength delay | `20 / ATTACK_SPEED` ticks | `Player.getCurrentItemAttackStrengthDelay` |
| Critical hit multiplier | ×1.5 | `Player.attack` |
| Default `ATTACK_DAMAGE` | 2.0 (range 0–2048) | `Attributes` |
| Default `ATTACK_KNOCKBACK` | 0.0 (range 0–5) | `Attributes` |
| Default `ATTACK_SPEED` | 4.0 (range 0–1024) | `Attributes` |
| Default `ARMOR` | 0.0 (range 0–30) | `Attributes` |
| Default `ARMOR_TOUGHNESS` | 0.0 (range 0–20) | `Attributes` |
| Default `KNOCKBACK_RESISTANCE` | 0.0 (range −2–1) | `Attributes` |
| Default `SAFE_FALL_DISTANCE` | 3.0 (range −1024–1024) | `Attributes` |
| Default `FALL_DAMAGE_MULTIPLIER` | 1.0 (range 0–100) | `Attributes` |
| Default `GRAVITY` | 0.08 (range −1–1) | `Attributes` |
| Default `SWEEPING_DAMAGE_RATIO` | 0.0 (range 0–1) | `Attributes` |
| Default `MAX_HEALTH` | 20.0 | `Attributes` |
| Mace smash thresholds | 3.0 / 8.0 blocks (piecewise 4·d, 12+2·(d−3), 22+(d−8)) | `MaceItem.getAttackDamageBonus` |
| Mace smash knockback radius/power | 3.5 blocks, `(3.5−dist)·0.7·(fall>5?2:1)` | `MaceItem.knockback` |
| `EXHAUSTION_ATTACK` | 0.1 | `FoodConstants` |
| `EXHAUSTION_JUMP` / `SPRINT_JUMP` | 0.05 / 0.2 | `FoodConstants` |
| `EXHAUSTION_MINE` | 0.005 | `FoodConstants` |
| `EXHAUSTION_SPRINT` / `SWIM` / `WALK` / `CROUCH` | 0.1 / 0.01 / 0.0 / 0.0 | `FoodConstants` |
| `EXHAUSTION_HEAL` | 6.0 | `FoodConstants` |
| `EXHAUSTION_DROP` (food-level decay threshold) | 4.0 | `FoodConstants` |
| `HEALTH_TICK_COUNT` / `_SATURATED` | 80 / 10 | `FoodConstants` |
| `HEAL_LEVEL` / `STARVE_LEVEL` | 18 / 0 | `FoodConstants` |
| Starve damage | 1.0 per 80-tick pulse, gated by health/difficulty | `FoodData.tick` |
| Instant Health/Harm base amounts | `4<<amp` heal / `6<<amp` harm | `HealOrHarmMobEffect` |
| Local-difficulty base scale | 0.75 | `DifficultyInstance` |
| Local-difficulty global-time window | 1,440,000 ticks (20 hrs), offset −72,000 | `DifficultyInstance` |
| Local-difficulty local-time window | 3,600,000 ticks (50 hrs, chunk-inhabited-time) | `DifficultyInstance` |
| Difficulty ids | PEACEFUL=0, EASY=1, NORMAL=2, HARD=3 | `Difficulty` |
| Player incoming-damage scaling | Easy: `min(d/2+1,d)`; Hard: `d*1.5`; Peaceful: 0 | `Player.hurtServer` |

## 5. RNG usage map

| Mechanic | RNG source | Calls | Order |
|---|---|---|---|
| Melee attack (crit, sweep, damage) | none | 0 | deterministic |
| Mob melee attack | none | 0 | deterministic |
| Shield block resolution | none | 0 | deterministic |
| Knockback degenerate-direction fallback | victim entity's own `RandomSource` | 2× `nextDouble()` per loop iteration, looped until direction is non-degenerate (almost always 1 iteration) | inside `LivingEntity.knockback`, once per impulse call that hits the degenerate branch |
| Bow/crossbow shot spread (inaccuracy) | projectile entity's own `RandomSource` | 3× `random.triangle(0, 0.0172275·uncertainty)` | one call per axis, in x, y, z order, inside `getMovementToShoot`, once per projectile spawned |
| Crit-arrow bonus damage | arrow entity's own `RandomSource` | 1× `nextInt(damage/2 + 2)` | only when `isCritArrow()`, inside `onHitEntity`, after the enchant-modified base damage and velocity scale are already computed |
| Thorns proc chance | attacker/victim-context RNG via the generic enchantment `random_chance` condition (`0.15·level` probability) | 1 roll per applicable equipped Thorns item per hit, via the loot-condition evaluator | inside `doPostAttackEffects`, per equipped Thorns armor piece |
| Thorns damage roll | same generic condition-evaluation RNG | 1× uniform int in `[1,5]` (`DamageEntity` effect `min_damage=1, max_damage=5`) | only if the chance roll above succeeds |
| Instant Health/Harm | none | 0 | deterministic given amplifier/scale |
| Natural regen / starvation | none | 0 | deterministic |
| Difficulty scaling | none | 0 | deterministic |

No mechanic in this document consumes the world seed's LCG stream directly — all RNG here is per-entity `RandomSource` instances (arrow, victim, etc.), seeded independently at entity construction (`Mth.createInsecureUUID`-derived, not the deterministic worldgen LCG). This means **combat RNG is not part of seed-determinism** for worldgen-parity purposes, but call **count and order still matter** for any test harness that pins a per-entity RNG seed for reproducible combat-log testing.

## 6. Cross-references

- `docs/research/mc-26.2/11-player-gameplay.md` §3.16 — the broad-pass version of this pipeline; this document supersedes it in the death message, difficulty scaling, and shield/absorption-divergence detail, but §3.16 still holds for `keepInventory`/death-message-broadcast/team-visibility mechanics not repeated here.
- `docs/research/mc-26.2/09-entities-ai.md` §3.4 — the `AttributeInstance` 3-stage `ADD_VALUE → ADD_MULTIPLIED_BASE → ADD_MULTIPLIED_TOTAL` calculation this document's `getAttributeValue(...)` calls rely on for `ATTACK_DAMAGE`/`ARMOR`/etc.
- `docs/research/mc-26.2/14-physics-collision.md` — movement integration or knockback-adjacent velocity clamps not covered here (this doc only covers the knockback *impulse* math, not general movement/collision resolution of the resulting velocity).
- Planning doc `docs/planning/05-game-mechanics.md` (MECH-) — should own the Rust-side spec for this pipeline; every formula above should map to an MECH- decision ID once blueprinted.
- Planning doc `docs/planning/09-testing-quality.md` (TEST-) — the two-impulse knockback stacking (§3.10), the player/mob absorption-decrement asymmetry (§3.7), and the invulnerability half-window rule (§3.2) are exactly the kind of "silent 1% divergence" this project's differential/parity test tiers exist to catch; each deserves a dedicated fixture.
- ASSET-D18(f) reference policy (`CLAUDE.md`) — every formula above was read from `net.minecraft.*` decompiled classes and re-expressed as original pseudocode/prose per that policy; no method body is reproduced verbatim.

## 7. Reimplementation hazards (ranked)

1. **Two-impulse knockback stacking, in order, with different direction vectors.** Melee knockback is `knockback()` called twice (generic 0.4 hurt-impulse using source-relative direction, then attacker-yaw-directed extra impulse), and each call **halves whatever velocity is already there** before adding its own vector. Implementing this as a single combined knockback call, or reordering the two calls, silently changes the resulting velocity on every melee hit.
2. **The invulnerability window is a 10-tick top-up gate inside a 20-tick counter, not a flat "can't be hit for 20 ticks."** Getting the `> 10` boundary wrong (e.g. using `> 0`) breaks double-hit resistance entirely (rapid weak+strong combos, arrow-then-melee sequencing, multi-hit AoE) in a way that's very easy to miss in casual testing but obvious in any DPS-optimal PvP sequence.
3. **Player vs. mob absorption bookkeeping genuinely diverge** (`Player.actuallyHurt` subtracts the absorbed portion once; the shared `LivingEntity.actuallyHurt` mob path subtracts it once *and then subtracts the health-damage amount again*). Unifying these two paths "for cleanliness" during implementation will make mobs' Absorption effect behave wrong relative to vanilla.
4. **Order of the outer damage pipeline is not commutative.** Shield block → freeze ×5 → helmet ×0.75 → NaN clamp → invuln gate → (armor absorb → resistance → enchant EPF → absorption hearts) — reversing any adjacent pair (e.g. computing helmet reduction before the shield subtracts its share, or resistance before armor) changes the final number for any hit that engages more than one reducer at once.
5. **`magicBoost` in `Player.attack` is computed from the pre-charge-curve base damage, then independently scaled by charge, then added to the post-charge-curve base** — it is not simply "apply enchant, then apply charge curve to the total." A naive reimplementation that computes `getEnchantedDamage` after `baseDamageScaleFactor()` has already been applied will get a different number at partial charge.
6. **Sweep damage recomputes enchant bonuses per swept target**, using `baseDamage` = the *post-bonus, post-crit* main-hit value, not `totalDamage` (which includes the primary target's own magic boost) — copying the primary hit's `magicBoost` onto sweep targets, or using `totalDamage` as the sweep base, both diverge from vanilla.
7. **Fall damage is exempt from physical armor but not from enchantment protection or Resistance** (`fall` is in `bypasses_armor` but not in `bypasses_enchantments`/`bypasses_effects`) — an implementation that treats "bypasses armor" as "bypasses all mitigation" will make Feather Falling a no-op.
8. **`bypasses_cooldown` is an empty tag in shipped 26.2 data.** Don't hardcode any vanilla damage type as bypassing the invulnerability window based on assumptions from older versions or other reimplementations — verify against the live tag file, since a future datapack (or a Rusty Clanker mod) populating it is the only way this ever fires.
9. **Crit-arrow RNG, shot-spread RNG, and the knockback degenerate-direction RNG each draw from a *different* entity's own `RandomSource`** (arrow's own, projectile's own, and the knockback target's own, respectively) — not the shooter's RNG and not a shared world RNG. Any test harness or replay system that pins "the" RNG stream for a combat sequence must track these as separate, independently-seeded streams per entity.
10. **The Mace's `getAttackDamageBonus` is added after the charge-curve multiply but before the crit multiplier**, and it independently resets `fallDistance` and suppresses the fall-damage call that would otherwise double-count the same fall — an implementation that lets a Mace smash both deal bonus melee damage *and* trigger normal fall damage from the same fall is a straightforward double-count bug that vanilla explicitly guards against via `setIgnoreFallDamageFromCurrentImpulse`.
11. **Difficulty-based incoming-damage scaling happens in `Player.hurtServer`, strictly before shield/freeze/helmet/invulnerability logic** — scaling the *output* of `actuallyHurt` instead of the *input* damage value produces different numbers once armor/enchant percentage-based reductions are involved (percentage-of-a-larger-number vs. percentage-of-a-smaller-number are not equal after multiple multiplicative stages).
12. **Exhaustion from taking damage is charged only when net health damage is nonzero** (i.e. after absorption hearts, armor, and magic absorb have all been applied) — charging it earlier (e.g. on raw incoming damage) will desync a player's hunger clock from vanilla during any prolonged fight with active Absorption or heavy armor.
