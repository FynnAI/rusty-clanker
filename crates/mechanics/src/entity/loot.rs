//! The general, `random_sequence`-capable loot-roll engine (MECH-D52 ff., Context §J) driving
//! a hand-authored, closed, interim table for M3's own tier-1 block set (Context §J's own
//! "sourcing stance, restated and reconciled" — the real `xtask`-generated/`rc-registries`-
//! homed pipeline stays a future blueprint's own scope).

use std::collections::HashMap;
use std::sync::OnceLock;

use rc_registries::generated_v776::registries::RegistryEntryId;
use rc_registries::generated_v776::registries::item;

use crate::entity::ItemStackRecord;
use crate::random::RcRandomSource;

pub trait LootRandom {
    fn next_int_bounded(&mut self, bound: i32) -> i32;
}

impl LootRandom for crate::random::XoroshiroRandom {
    fn next_int_bounded(&mut self, bound: i32) -> i32 {
        RcRandomSource::next_int_bounded(self, bound)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RollProvider {
    Constant(u32),
    Uniform { min: u32, max: u32 },
}

impl RollProvider {
    /// `rng-parity-notes.md` §5.3's own `uniform.get_int`: `lo=min, hi=max; if lo>=hi return lo
    /// (no draw); else lo + rng.next_int_bounded(hi-lo+1)`.
    pub fn resolve(self, rng: &mut dyn LootRandom) -> u32 {
        match self {
            RollProvider::Constant(n) => n,
            RollProvider::Uniform { min, max } => {
                if min >= max {
                    min
                } else {
                    min + rng.next_int_bounded((max - min + 1) as i32) as u32
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CountProvider {
    Constant(u8),
    Uniform { min: u8, max: u8 },
}

impl CountProvider {
    pub fn resolve(self, rng: &mut dyn LootRandom) -> u8 {
        match self {
            CountProvider::Constant(n) => n,
            CountProvider::Uniform { min, max } => {
                if min >= max {
                    min
                } else {
                    min + rng.next_int_bounded((max - min + 1) as i32) as u8
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootEntry {
    pub item_id: RegistryEntryId,
    pub base_weight: i32,
    pub quality: i32,
    pub count: CountProvider,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootPool {
    pub rolls: RollProvider,
    pub bonus_rolls: RollProvider,
    pub entries: Vec<LootEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootTable {
    pub sequence_id: &'static str,
    pub pools: Vec<LootPool>,
}

/// Context §J's own algorithm, restated field-precise. `luck` is always `0.0` at this
/// milestone's own scope (no luck source exists) — the parameter is real, not vestigial,
/// since `quality`-weighted entries and `bonus_rolls` both consume it structurally, ready for
/// a future luck-status-effect blueprint to supply a nonzero value with zero engine change.
pub fn roll_loot_table(
    table: &LootTable,
    rng: &mut dyn LootRandom,
    luck: f32,
) -> Vec<ItemStackRecord> {
    let mut results = Vec::new();

    for pool in &table.pools {
        // Both draws happen unconditionally, left-to-right, regardless of `luck`'s own value
        // -- `bonus_rolls.resolve` may consume a real RNG draw even when its own contribution
        // to `roll_count` ends up multiplied away by `luck == 0.0`.
        let base_rolls = pool.rolls.resolve(rng);
        let bonus_rolls = pool.bonus_rolls.resolve(rng);
        let roll_count = base_rolls + ((bonus_rolls as f64) * (luck as f64)).floor() as u32;

        for _ in 0..roll_count {
            let mut valid: Vec<(&LootEntry, i32)> = Vec::new();
            let mut total_weight: i32 = 0;
            for entry in &pool.entries {
                let weight = (((entry.base_weight as f64) + (entry.quality as f64) * (luck as f64))
                    .floor() as i32)
                    .max(0);
                if weight > 0 {
                    valid.push((entry, weight));
                    total_weight += weight;
                }
            }
            if valid.is_empty() || total_weight == 0 {
                continue;
            }

            let chosen: &LootEntry = if valid.len() == 1 {
                valid[0].0
            } else {
                // Exactly ONE draw -- the single-candidate shortcut above is what keeps this
                // branch from ever running when only one entry has positive weight.
                let mut index = rng.next_int_bounded(total_weight);
                let mut chosen_entry = valid[0].0;
                for (entry, weight) in &valid {
                    if index < *weight {
                        chosen_entry = entry;
                        break;
                    }
                    index -= weight;
                }
                chosen_entry
            };

            let count = chosen.count.resolve(rng);
            results.push(ItemStackRecord {
                item_id: chosen.item_id,
                count,
                components: None,
            });
        }
    }

    results
}

/// The closed set this blueprint's own tier-1 table covers (Context §J's own table).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Tier1DroppableBlock {
    Stone,
    Dirt,
    GrassBlock,
    RedstoneWire,
    RedstoneTorch,
    Repeater,
    Comparator,
    Piston,
    StickyPiston,
    Chest,
    Hopper,
    Furnace,
    BlastFurnace,
    Smoker,
}

fn single_unconditional_drop(sequence_id: &'static str, item_id: RegistryEntryId) -> LootTable {
    LootTable {
        sequence_id,
        pools: vec![LootPool {
            rolls: RollProvider::Constant(1),
            bonus_rolls: RollProvider::Constant(0),
            entries: vec![LootEntry {
                item_id,
                base_weight: 1,
                quality: 0,
                count: CountProvider::Constant(1),
            }],
        }],
    }
}

static TIER1_TABLES: OnceLock<HashMap<Tier1DroppableBlock, LootTable>> = OnceLock::new();

fn build_tier1_tables() -> HashMap<Tier1DroppableBlock, LootTable> {
    use Tier1DroppableBlock::*;
    HashMap::from([
        (
            Stone,
            single_unconditional_drop("minecraft:blocks/stone", item::COBBLESTONE),
        ),
        (
            Dirt,
            single_unconditional_drop("minecraft:blocks/dirt", item::DIRT),
        ),
        (
            GrassBlock,
            single_unconditional_drop("minecraft:blocks/grass_block", item::DIRT),
        ),
        (
            RedstoneWire,
            single_unconditional_drop("minecraft:blocks/redstone_wire", item::REDSTONE),
        ),
        (
            RedstoneTorch,
            single_unconditional_drop("minecraft:blocks/redstone_torch", item::REDSTONE_TORCH),
        ),
        (
            Repeater,
            single_unconditional_drop("minecraft:blocks/repeater", item::REPEATER),
        ),
        (
            Comparator,
            single_unconditional_drop("minecraft:blocks/comparator", item::COMPARATOR),
        ),
        (
            Piston,
            single_unconditional_drop("minecraft:blocks/piston", item::PISTON),
        ),
        (
            StickyPiston,
            single_unconditional_drop("minecraft:blocks/sticky_piston", item::STICKY_PISTON),
        ),
        (
            Chest,
            single_unconditional_drop("minecraft:blocks/chest", item::CHEST),
        ),
        (
            Hopper,
            single_unconditional_drop("minecraft:blocks/hopper", item::HOPPER),
        ),
        (
            Furnace,
            single_unconditional_drop("minecraft:blocks/furnace", item::FURNACE),
        ),
        (
            BlastFurnace,
            single_unconditional_drop("minecraft:blocks/blast_furnace", item::BLAST_FURNACE),
        ),
        (
            Smoker,
            single_unconditional_drop("minecraft:blocks/smoker", item::SMOKER),
        ),
    ])
}

/// Context §J's own closed, hand-authored table — one `LootTable` per tier-1 broken-block
/// case, keyed by `BlockStateId` range/value via the caller's own resolution
/// (`entity_drops.rs`, `rusty-clanker-server`), not by this function itself.
pub fn tier1_loot_table(block: Tier1DroppableBlock) -> &'static LootTable {
    TIER1_TABLES
        .get_or_init(build_tier1_tables)
        .get(&block)
        .expect("build_tier1_tables populates every Tier1DroppableBlock variant")
}

/// Context §K — per-region, lazily-populated `random_sequence` cache. `rc-rng`'s own
/// `create_random_sequence` stays a pure function; this is the stateful cache the concrete
/// mechanism behind `rng-parity-notes.md` §5.2's own "statefulness across invocations" rule
/// needs.
#[derive(Default)]
#[cfg_attr(feature = "server-systems", derive(bevy_ecs::prelude::Resource))]
pub struct RandomSequenceStore(HashMap<String, crate::random::XoroshiroRandom>);

impl RandomSequenceStore {
    /// Creates via `create_random_sequence` on first reference and returns the same,
    /// already-advanced instance on every subsequent call for the same id.
    pub fn get_or_create(
        &mut self,
        sequence_id: &str,
        world_seed: i64,
    ) -> &mut crate::random::XoroshiroRandom {
        self.0.entry(sequence_id.to_string()).or_insert_with(|| {
            crate::random::create_random_sequence_default(sequence_id, world_seed)
        })
    }
}
