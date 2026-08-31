//! M3-B07 — the replay driver's own single-region invariant, `tier1_registry`'s
//! empty-registry contract, and `snapshot_volume`'s full-coverage/canonical-order
//! guarantee (blueprint Acceptance tests, `replay_isolation.rs`). Synthetic /
//! committed-fixture data only — no oracle, no network.

use std::path::PathBuf;
use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_gametest::replay::{replay_contraption, tier1_registry};
use rc_gametest::spec::{Category, ContraptionSpec, PlacedBlock, load_spec};

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/redstone")
}

/// A spec with no blocks at all — sufficient for the two `tier1_registry` tests below, which
/// only probe `BlockBehaviorRegistry::resolve` and never dispatch, so no repeater/comparator/
/// piston placement seeding is ever needed.
fn empty_spec() -> ContraptionSpec {
    ContraptionSpec {
        id: "test/synthetic/empty".to_string(),
        category: Category::UpdateOrderProbe,
        description: "synthetic".to_string(),
        quirk: "synthetic — no blocks, used only to probe tier1_registry's own resolve() contract"
            .to_string(),
        max_ticks: 0,
        blocks: vec![],
        actions: vec![],
    }
}

#[test]
fn replay_contraption_never_produces_a_nonempty_outbound() {
    let path = corpus_dir().join("torch_inverter_basic.ron");
    let spec = load_spec(&path).expect("load torch_inverter_basic.ron");
    let (registry, container_signals) = tier1_registry(&spec);

    // A single always_local region can never route a message cross-region
    // (Deliverables, `replay_contraption`'s own doc comment) — `replay_contraption`
    // asserts this internally; this test's job is proving that assertion is never
    // tripped for a legitimate single-region contraption. Reaching this line at all
    // is the assertion.
    let _trace = replay_contraption(&spec, &registry, &container_signals, None);
}

#[test]
fn tier1_registry_resolves_unregistered_ids_to_the_shared_noop_default() {
    let (registry, _container_signals) = tier1_registry(&empty_spec());

    // Governance fix (M3 field-report): `tier1_registry` now wires the real M3-B04/M3-B05
    // tier-1 registrations (`tier1_registry`'s own doc comment has the full citation), so this
    // contract narrows from "every id" to "every id outside a registered tier-1 range" — still
    // proven by pointer identity across widely different unregistered ids, never merely "some
    // handler that happens to do nothing". `0`/`12_345`/`u32::MAX` are all deliberately outside
    // every tier-1 range this module registers (wire/torch/repeater/comparator/piston).
    let low = registry.resolve(BlockStateId(0));
    let mid = registry.resolve(BlockStateId(12_345));
    let high = registry.resolve(BlockStateId(u32::MAX));

    assert!(Arc::ptr_eq(low, mid));
    assert!(Arc::ptr_eq(low, high));
}

#[test]
fn tier1_registry_resolves_registered_redstone_ids_to_real_behaviors() {
    let (registry, _container_signals) = tier1_registry(&empty_spec());
    let noop = registry.resolve(BlockStateId(0));

    // One representative id per registered range (the same literals
    // `crates/mechanics/tests/piston_structure_resolver.rs` and
    // `crates/physics/src/shapes.rs` already use for these same blocks) — each must resolve to
    // something other than the shared `NoOpBehavior` default now that `tier1_registry` wires
    // the real components.
    for id in [
        4591,  // redstone_wire
        6885,  // redstone_torch (floor)
        6891,  // redstone_wall_torch
        7037,  // repeater
        11264, // comparator
        2258,  // piston
        2236,  // sticky_piston
    ] {
        let resolved = registry.resolve(BlockStateId(id));
        assert!(
            !Arc::ptr_eq(resolved, noop),
            "state id {id} should resolve to a real tier-1 behavior, not the NoOp default"
        );
    }
}

/// `snapshot_volume` (Deliverables) is a private helper of `replay.rs` — this test
/// exercises it indirectly through `replay_contraption`'s own public API (its only
/// caller), by inspecting the tick-0 snapshot of a synthetic contraption whose three
/// placed blocks deliberately do *not* fill their own bounding box, so most of the
/// asserted positions are unset (must read as air, `state_id: 0`) rather than
/// omitted.
#[test]
fn snapshot_volume_covers_the_full_bounding_box_in_canonical_order() {
    let spec = ContraptionSpec {
        id: "test/synthetic/snapshot_volume_coverage".to_string(),
        category: Category::UpdateOrderProbe,
        description: "synthetic".to_string(),
        quirk: "synthetic — exercises snapshot_volume's own full-coverage/canonical-order contract"
            .to_string(),
        max_ticks: 1,
        blocks: vec![
            PlacedBlock {
                pos: (0, 0, 0),
                vanilla_state: "minecraft:stone".to_string(),
                state_id: 1,
                has_analog_state: false,
            },
            PlacedBlock {
                pos: (2, 0, 0),
                vanilla_state: "minecraft:stone".to_string(),
                state_id: 2,
                has_analog_state: false,
            },
            PlacedBlock {
                pos: (0, 0, 2),
                vanilla_state: "minecraft:stone".to_string(),
                state_id: 3,
                has_analog_state: false,
            },
        ],
        actions: vec![],
    };

    let (registry, container_signals) = tier1_registry(&spec);
    let trace = replay_contraption(&spec, &registry, &container_signals, None);
    let tick0 = &trace.ticks[0];

    // (y, z, x) ascending, y fixed at 0 for this spec: z outer, x inner.
    let expected: Vec<((i32, i32, i32), u32)> = vec![
        ((0, 0, 0), 1),
        ((1, 0, 0), 0),
        ((2, 0, 0), 2),
        ((0, 0, 1), 0),
        ((1, 0, 1), 0),
        ((2, 0, 1), 0),
        ((0, 0, 2), 3),
        ((1, 0, 2), 0),
        ((2, 0, 2), 0),
    ];

    let actual: Vec<((i32, i32, i32), u32)> =
        tick0.blocks.iter().map(|b| (b.pos, b.state_id)).collect();
    assert_eq!(actual, expected);
}
