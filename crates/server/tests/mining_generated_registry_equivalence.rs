//! test-matrix: boundaries=waived(pure id/geometry arithmetic against a local FakeWorld, never drives across the real world Y-limit, see world_bounds_fan_out.rs) orientations=yes self=waived(no player/actor entity in this suite's own domain model) composition=yes nondefault-state=yes
//! M3.5-B02 (WS-D15) test-authoring changeset: pins `crates/server/src/play/mining.rs`'s own
//! chest/hopper/tier1-oriented-placement id arithmetic against `rc-registries`'
//! M3.5-B01-generated per-block-state-property registry (`rc_registries::
//! block_state_properties`), through the exact `pub` functions this file's own contract keeps
//! signature-stable (`chest_state_id`, `tier1_oriented_state_table`,
//! `hopper_facing_from_raw_state`). Every assertion here is true today, before the
//! Implementation changeset retires any production arithmetic (both sides of every
//! equivalence below are values already present in the repository) -- proving the retirement
//! ahead of time.
//!
//! `mining.rs`'s own private `decode_chest_state`'s `waterlogged=true` defect (§3.7 of
//! `blueprints/M3.5/M3.5-B02-retire-hand-authored-id-tables.md`) is pinned by a direct,
//! inline `#[cfg(test)]` unit test inside `mining.rs` itself instead (`decode_chest_state` is
//! a private fn, unreachable from this external integration-test crate) -- see
//! `chest_decode_covers_waterlogged_ids` there; it starts red and turns green only once Step 8
//! retires `decode_chest_state` against the generated registry.

use std::collections::HashMap;

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, Direction, NeighborUpdateEngine,
    RegionOwnership, ScheduledTickQueue,
};
use rc_messaging::{Address, RegionId, RegionMessage};
use rc_registries::block_state_properties::{properties, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::BlockStateId as GeneratedBlockStateId;
use rusty_clanker_server::play::{
    ChestType, Face, HeldItemStub, Orientation, PlaceOutcome, PlaceableBlockKind, apply_placement,
    chest_state_id, hopper_facing_from_raw_state, tier1_oriented_state_table,
};

fn dir_str(d: Direction) -> &'static str {
    match d {
        Direction::North => "north",
        Direction::South => "south",
        Direction::East => "east",
        Direction::West => "west",
        Direction::Up => "up",
        Direction::Down => "down",
    }
}

fn parse_direction(s: &str) -> Direction {
    match s {
        "north" => Direction::North,
        "south" => Direction::South,
        "east" => Direction::East,
        "west" => Direction::West,
        "up" => Direction::Up,
        "down" => Direction::Down,
        other => panic!("parse_direction: unrecognized direction string {other:?}"),
    }
}

const HORIZONTAL4: [Direction; 4] = [
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
];
const FULL6: [Direction; 6] = [
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
    Direction::Up,
    Direction::Down,
];

#[test]
fn chest_facing_and_type_state_id_matches_generated_state_id() {
    for dir in HORIZONTAL4 {
        for (chest_type, type_str) in [
            (ChestType::Single, "single"),
            (ChestType::Left, "left"),
            (ChestType::Right, "right"),
        ] {
            let expected = state_id(
                block_id::CHEST,
                &[("facing", dir_str(dir)), ("type", type_str)],
            )
            .expect("every legal chest property combination resolves")
            .0;
            let actual = chest_state_id(dir, chest_type);
            assert_eq!(actual, expected, "dir={dir:?} chest_type={chest_type:?}");
        }
    }
}

/// As `crates/server/tests/mining_placement_obstruction.rs`'s own `FakeWorld` -- a plain
/// `HashMap`-backed `BlockWorldAccess`, single fixed local region.
struct FakeWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
}

impl FakeWorld {
    fn new() -> Self {
        FakeWorld {
            blocks: HashMap::new(),
        }
    }
}

impl BlockWorldAccess for FakeWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.blocks.get(&pos).copied()
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let changed = self.blocks.get(&pos) != Some(&state);
        self.blocks.insert(pos, state);
        changed
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        Address::Region(RegionId(0))
    }
    fn local_identity(&self) -> Address {
        Address::Region(RegionId(0))
    }
}

/// Places a chest at `target` (`inside_block: true` skips face-offset resolution, `yaw`
/// selects the base FACING per this project's own established `nearest_horizontal_direction4`
/// convention -- `0.0` -> North, mirroring `mining_placement_orientation.rs`'s own worked
/// examples) through the exact same `mining::apply_placement` a real `Use Item On` packet
/// drives -- the only production call site that reaches the private `decode_chest_state`
/// (via its own `chest_neighbor_at` closure), so this is what indirectly exercises it.
fn place_chest(world: &mut FakeWorld, target: BlockPos, yaw_degrees: f32) -> PlaceOutcome {
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
    let behaviors = BlockBehaviorRegistry::new();
    apply_placement(
        world,
        &mut engine,
        &mut scheduled,
        &mut events,
        &mut outbound,
        &mut changed,
        &ownership,
        &behaviors,
        0,
        target,
        Face::Up,
        true,
        (0.5, 0.5, 0.5),
        HeldItemStub::Block(PlaceableBlockKind::Chest),
        yaw_degrees,
        0.0,
        &[],
        false,
    )
}

/// Indirect coverage of the private `decode_chest_state` through its only production call
/// site (`apply_placement`'s own `chest_neighbor_at` closure) -- a representative sample (one
/// facing, both directions of the clockwise/counter-clockwise merge check), not an exhaustive
/// sweep of all 12 `(facing, chest_type)` combinations (mirrors this project's own established
/// "representative sample" convention, e.g. `redstone_generated_registry_equivalence.rs`'s own
/// wire test). A `chest_type`'s exact `Left`/`Right` identity is not independently observable
/// through `chest_neighbor_at`'s own `ChestNeighbor { facing, is_single }` shape (both
/// non-Single types collapse to `is_single: false`) -- what decode_chest_state's own three-way
/// `type` decode DOES observably affect is whether a later placement merges at all, which this
/// chain of three placements exercises: chest 1 (Single) legitimately merges with chest 2,
/// proving decode recovers `Single` correctly; chest 1 (now Right, after the merge) correctly
/// refuses a third merge attempt, proving decode also recovers a non-Single type correctly.
#[test]
fn chest_merge_chain_round_trip_through_production_decode_path() {
    let mut world = FakeWorld::new();

    // Chest 1: Single, facing North (yaw 0.0 -> base FACING North, this project's own
    // established convention).
    let pos0 = BlockPos::new(0, 64, 0);
    let outcome0 = place_chest(&mut world, pos0, 0.0);
    let expected_single = chest_state_id(Direction::North, ChestType::Single);
    match outcome0 {
        PlaceOutcome::Applied { new_state, .. } => assert_eq!(new_state, expected_single),
        other => panic!("expected chest 1 to place successfully, got {other:?}"),
    }

    // Chest 2: placed West of chest 1, same yaw (base FACING North too). North's own clockwise
    // neighbor direction is East, so chest 2's own `chest_neighbor_at(East)` reads chest 1's
    // position -- a real, decoded (via `decode_chest_state`) Single/North chest -- and merges.
    let pos1 = Direction::West.apply(pos0);
    let outcome1 = place_chest(&mut world, pos1, 0.0);
    let expected_left = chest_state_id(Direction::North, ChestType::Left);
    match outcome1 {
        PlaceOutcome::Applied { new_state, .. } => assert_eq!(new_state, expected_left),
        other => panic!("expected chest 2 to merge as Left, got {other:?}"),
    }
    // Chest 1's own id updated to Right as the merge's neighbor-writeback side effect.
    let expected_right = chest_state_id(Direction::North, ChestType::Right);
    assert_eq!(
        world.get_block(pos0),
        Some(BlockStateId(expected_right)),
        "chest 1 must have flipped to Right after the merge"
    );

    // Chest 3: placed East of chest 1 (unoccupied), same yaw. North's own clockwise neighbor
    // direction (East) reads nothing; the counter-clockwise direction (West) reads chest 1 --
    // now Right, `is_single: false` -- so `decode_chest_state` correctly telling
    // `chest_neighbor_at` that chest 1 is no longer Single is what prevents a spurious second
    // merge here.
    let pos2 = Direction::East.apply(pos0);
    let outcome2 = place_chest(&mut world, pos2, 0.0);
    match outcome2 {
        PlaceOutcome::Applied { new_state, .. } => {
            assert_eq!(
                new_state, expected_single,
                "chest 3 must resolve to a plain Single -- chest 1 is no longer merge-eligible"
            );
        }
        other => panic!("expected chest 3 to place successfully (as Single), got {other:?}"),
    }
    // Chest 1's own id must be unchanged by chest 3's placement -- no second merge fired.
    assert_eq!(world.get_block(pos0), Some(BlockStateId(expected_right)));
}

#[test]
fn tier1_oriented_entries_nondefault_orientations_match_generated_ids() {
    let table = tier1_oriented_state_table();

    // No-orientation kinds: a pure single-value lookup, no property variation.
    assert_eq!(
        table.lookup(PlaceableBlockKind::Stone, Orientation::None),
        rc_registries::generated_v776::block_states::default_state::STONE.0
    );
    assert_eq!(
        table.lookup(PlaceableBlockKind::RedstoneWire, Orientation::None),
        rc_registries::generated_v776::block_states::default_state::REDSTONE_WIRE.0
    );
    assert_eq!(
        table.lookup(PlaceableBlockKind::RedstoneTorch, Orientation::None),
        rc_registries::generated_v776::block_states::default_state::REDSTONE_TORCH.0
    );

    for dir in HORIZONTAL4 {
        let facing = dir_str(dir);

        type Case = (
            PlaceableBlockKind,
            Orientation,
            rc_registries::generated_v776::block_state_properties::BlockId,
            &'static [(&'static str, &'static str)],
        );
        let cases: [Case; 9] = [
            (
                PlaceableBlockKind::RedstoneTorch,
                Orientation::Horizontal(dir),
                block_id::REDSTONE_WALL_TORCH,
                &[],
            ),
            (
                PlaceableBlockKind::Repeater,
                Orientation::Horizontal(dir),
                block_id::REPEATER,
                &[],
            ),
            (
                PlaceableBlockKind::Comparator,
                Orientation::Horizontal(dir),
                block_id::COMPARATOR,
                &[],
            ),
            (
                PlaceableBlockKind::Furnace,
                Orientation::Horizontal(dir),
                block_id::FURNACE,
                &[],
            ),
            (
                PlaceableBlockKind::BlastFurnace,
                Orientation::Horizontal(dir),
                block_id::BLAST_FURNACE,
                &[],
            ),
            (
                PlaceableBlockKind::Smoker,
                Orientation::Horizontal(dir),
                block_id::SMOKER,
                &[],
            ),
            (
                PlaceableBlockKind::Hopper,
                Orientation::Horizontal(dir),
                block_id::HOPPER,
                &[],
            ),
            (
                PlaceableBlockKind::Chest,
                Orientation::Chest(dir, ChestType::Left),
                block_id::CHEST,
                &[("type", "left")],
            ),
            (
                PlaceableBlockKind::Chest,
                Orientation::Chest(dir, ChestType::Right),
                block_id::CHEST,
                &[("type", "right")],
            ),
        ];

        for (kind, orientation, block, extra_props) in cases {
            let mut props: Vec<(&str, &str)> = vec![("facing", facing)];
            props.extend_from_slice(extra_props);
            let expected = state_id(block, &props)
                .unwrap_or_else(|| panic!("{block:?} with {props:?} must resolve"))
                .0;
            let actual = table.lookup(kind, orientation);
            assert_eq!(
                actual, expected,
                "kind={kind:?} orientation={orientation:?}"
            );
        }

        // Chest's own placement-time default type (Single).
        let expected_single = state_id(block_id::CHEST, &[("facing", facing)])
            .expect("chest facing-only resolves to the Single default")
            .0;
        assert_eq!(
            table.lookup(
                PlaceableBlockKind::Chest,
                Orientation::Chest(dir, ChestType::Single)
            ),
            expected_single
        );
    }

    // Hopper's own clamped-Down orientation.
    let expected_hopper_down = state_id(block_id::HOPPER, &[("facing", "down")])
        .expect("hopper facing=down resolves to its own placement-time default")
        .0;
    assert_eq!(
        table.lookup(
            PlaceableBlockKind::Hopper,
            Orientation::Full(Direction::Down)
        ),
        expected_hopper_down
    );

    // Piston/sticky_piston: every FULL6 facing, always extended=false at placement.
    for dir in FULL6 {
        let facing = dir_str(dir);
        let expected_piston = state_id(
            block_id::PISTON,
            &[("extended", "false"), ("facing", facing)],
        )
        .expect("piston resolves")
        .0;
        assert_eq!(
            table.lookup(PlaceableBlockKind::Piston, Orientation::Full(dir)),
            expected_piston
        );
        let expected_sticky = state_id(
            block_id::STICKY_PISTON,
            &[("extended", "false"), ("facing", facing)],
        )
        .expect("sticky_piston resolves")
        .0;
        assert_eq!(
            table.lookup(PlaceableBlockKind::StickyPiston, Orientation::Full(dir)),
            expected_sticky
        );
    }
}

#[test]
fn hopper_facing_from_raw_state_matches_generated_properties() {
    for dir in [
        Direction::Down,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ] {
        for enabled in ["true", "false"] {
            let raw = state_id(
                block_id::HOPPER,
                &[("facing", dir_str(dir)), ("enabled", enabled)],
            )
            .expect("every legal hopper property combination resolves")
            .0;
            let actual = hopper_facing_from_raw_state(raw);

            let props = properties(GeneratedBlockStateId(raw));
            let (_, facing_str) = props
                .iter()
                .find(|(name, _)| *name == "facing")
                .expect("hopper's own generated properties include facing");
            let expected = parse_direction(facing_str);

            assert_eq!(actual, expected, "raw={raw} enabled={enabled}");
        }
    }
}
