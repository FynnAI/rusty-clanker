//! M4-B02 acceptance tests: `roll_loot_table`'s single-candidate shortcut, weighted-selection
//! draw, count-provider draw, and `random_sequence`-backed determinism/statefulness (Context
//! §J/§K).

use std::cell::RefCell;

use rc_mechanics::entity::loot::{
    CountProvider, LootEntry, LootPool, LootRandom, LootTable, RandomSequenceStore, RollProvider,
    Tier1DroppableBlock, roll_loot_table, tier1_loot_table,
};
use rc_mechanics::random::{RcRandomSource, XoroshiroRandom};
use rc_registries::generated_v776::registries::item;

struct PanicsOnDraw;
impl LootRandom for PanicsOnDraw {
    fn next_int_bounded(&mut self, bound: i32) -> i32 {
        panic!(
            "next_int_bounded({bound}) called — the single-candidate shortcut should have skipped this draw entirely"
        );
    }
}

struct CountingRandom<'a> {
    inner: &'a mut XoroshiroRandom,
    calls: RefCell<u32>,
}
impl<'a> LootRandom for CountingRandom<'a> {
    fn next_int_bounded(&mut self, bound: i32) -> i32 {
        *self.calls.borrow_mut() += 1;
        RcRandomSource::next_int_bounded(self.inner, bound)
    }
}

fn synthetic_weighted_pool_table() -> LootTable {
    LootTable {
        sequence_id: "test:synthetic_weighted_pool",
        pools: vec![LootPool {
            rolls: RollProvider::Constant(1),
            bonus_rolls: RollProvider::Constant(0),
            entries: vec![
                LootEntry {
                    item_id: item::DIRT,
                    base_weight: 1,
                    quality: 0,
                    count: CountProvider::Constant(1),
                },
                LootEntry {
                    item_id: item::COBBLESTONE,
                    base_weight: 3,
                    quality: 0,
                    count: CountProvider::Constant(1),
                },
            ],
        }],
    }
}

#[test]
fn single_entry_table_never_draws_rng() {
    let table = tier1_loot_table(Tier1DroppableBlock::Stone);
    let mut rng = PanicsOnDraw;
    let results = roll_loot_table(table, &mut rng, 0.0);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].item_id, item::COBBLESTONE);
    assert_eq!(results[0].count, 1);
}

#[test]
fn synthetic_two_entry_weighted_pool_consumes_exactly_one_draw() {
    let table = synthetic_weighted_pool_table();

    // Total weight = 1 + 3 = 4 -- compute the expected chosen entry from a fresh, identically-
    // seeded oracle, then check the real roll (through a counting wrapper) draws exactly once
    // and lands on the same entry.
    let mut oracle = XoroshiroRandom::new(7);
    let draw = RcRandomSource::next_int_bounded(&mut oracle, 4);
    let expected_item = if draw < 1 {
        item::DIRT
    } else {
        item::COBBLESTONE
    };

    let mut source = XoroshiroRandom::new(7);
    let mut counting = CountingRandom {
        inner: &mut source,
        calls: RefCell::new(0),
    };
    let results = roll_loot_table(&table, &mut counting, 0.0);
    assert_eq!(
        *counting.calls.borrow(),
        1,
        "exactly one weighted-selection draw"
    );
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].item_id, expected_item);
}

fn synthetic_uniform_count_table() -> LootTable {
    LootTable {
        sequence_id: "test:synthetic_uniform_count",
        pools: vec![LootPool {
            rolls: RollProvider::Constant(2),
            bonus_rolls: RollProvider::Constant(0),
            entries: vec![LootEntry {
                item_id: item::DIRT,
                base_weight: 1,
                quality: 0,
                count: CountProvider::Uniform { min: 1, max: 4 },
            }],
        }],
    }
}

#[test]
fn synthetic_uniform_count_provider_consumes_one_draw_per_roll() {
    let table = synthetic_uniform_count_table();

    let mut oracle = XoroshiroRandom::new(55);
    let expected_counts: Vec<i32> = (0..2)
        .map(|_| RcRandomSource::next_int_bounded(&mut oracle, 4) + 1)
        .collect();

    let mut source = XoroshiroRandom::new(55);
    let mut counting = CountingRandom {
        inner: &mut source,
        calls: RefCell::new(0),
    };
    let results = roll_loot_table(&table, &mut counting, 0.0);
    assert_eq!(
        *counting.calls.borrow(),
        2,
        "one count draw per roll, the single-entry shortcut still applies to selection"
    );
    assert_eq!(results.len(), 2);
    for (got, want) in results.iter().zip(expected_counts) {
        assert_eq!(got.count as i32, want);
        assert!((1..=4).contains(&got.count));
    }
}

#[test]
fn same_seed_same_sequence_id_reproduces_bit_identical_drops() {
    let table = synthetic_weighted_pool_table();

    let mut store_a = RandomSequenceStore::default();
    let rng_a = store_a.get_or_create(table.sequence_id, 777);
    let results_a = roll_loot_table(&table, rng_a, 0.0);

    let mut store_b = RandomSequenceStore::default();
    let rng_b = store_b.get_or_create(table.sequence_id, 777);
    let results_b = roll_loot_table(&table, rng_b, 0.0);

    assert_eq!(results_a.len(), results_b.len());
    for (a, b) in results_a.iter().zip(results_b.iter()) {
        assert_eq!(a.item_id, b.item_id);
        assert_eq!(a.count, b.count);
    }
}

#[test]
fn reconciling_two_breaks_of_the_same_block_type_shares_one_continuing_sequence() {
    let table = synthetic_weighted_pool_table();

    let mut store = RandomSequenceStore::default();
    {
        let rng1 = store.get_or_create(table.sequence_id, 42);
        let _first_roll = roll_loot_table(&table, rng1, 0.0);
    }
    // A second roll through the SAME store/id -- proving the mechanism is actually exercised
    // for two successive breaks of the same block type (Context §K's own "statefulness
    // across invocations" rule).
    {
        let rng2 = store.get_or_create(table.sequence_id, 42);
        let _second_roll = roll_loot_table(&table, rng2, 0.0);
    }

    // A high-entropy 64-bit draw from the now-twice-advanced stream, compared against a
    // completely fresh stream's own very first such draw -- unlike the loot table's own tiny
    // 4-outcome selection draw (whose small output space made a coincidental match here
    // observed in practice), a `next_long()` collision has probability on the order of
    // `2^-64`, making this comparison a reliable proof of continuation vs. reset.
    let continuing_next = store.get_or_create(table.sequence_id, 42).next_long();

    let mut fresh_store = RandomSequenceStore::default();
    let fresh_next = fresh_store.get_or_create(table.sequence_id, 42).next_long();

    assert_ne!(
        continuing_next, fresh_next,
        "the continuing stream must diverge from a fresh store's own first draw"
    );
}
