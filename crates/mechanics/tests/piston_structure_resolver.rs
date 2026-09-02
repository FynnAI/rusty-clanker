//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit, see world_bounds_fan_out.rs) orientations=waived(signal/push direction varies, not the component's own four-horizontal facing) self=waived(no player/actor entity in this suite's own domain model) composition=yes nondefault-state=yes
//! M3-B05 — push/pull structure resolution acceptance tests (Context §C): `classify`'s own
//! tier-1 push/destroy/block table, `resolve_extend`'s 12-block cap and destroy-terminates-the-
//! walk rule, `resolve_retract`'s one-block sticky pull, and MECH-D14's cross-partition
//! obstruction.

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::BlockWorldAccess;
use rc_mechanics::RegionOwnership;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::piston::{
    ExtendAbort, PullPlan, PushClass, classify, resolve_extend, resolve_retract,
};
use rc_messaging::{Address, RegionId};

use support::FakeWorld;

/// `origin` shifted `n` steps along `dir` -- `Direction` itself only offers a single-step
/// `apply`; this file's own small helper for building a straight test-fixture line.
fn step(dir: Direction, origin: BlockPos, n: i32) -> BlockPos {
    let mut pos = origin;
    for _ in 0..n {
        pos = dir.apply(pos);
    }
    pos
}

// --- Context §C's own tier-1 literal table, restated here as this test file's own fixture ids
// (kept in sync by hand with `piston.rs`'s own private `classify` table — Constraints (b), the
// same placeholder-literal cross-reference convention this project already established). ---

const STONE: BlockStateId = BlockStateId(1);
const DIRT: BlockStateId = BlockStateId(10);
const GRASS_BLOCK: BlockStateId = BlockStateId(9);
const BEDROCK: BlockStateId = BlockStateId(85);
const REDSTONE_WIRE: BlockStateId = BlockStateId(5171);
const REDSTONE_TORCH: BlockStateId = BlockStateId(6885);
const REPEATER: BlockStateId = BlockStateId(7037);
const COMPARATOR: BlockStateId = BlockStateId(11264);
const PISTON_RETRACTED: BlockStateId = BlockStateId(2263);
const PISTON_EXTENDED: BlockStateId = BlockStateId(900_101);
/// Own-state writeback (M3 field-report fix): the real id `commit_extend` now writes for a
/// facing=South, regular (non-sticky) extended piston base (blocks.json's own
/// `minecraft:piston` entry, protocol 776: `facing=south,extended=true` = state 2259) -- must
/// classify as `Immovable` exactly like `PISTON_EXTENDED`'s own placeholder above.
const PISTON_EXTENDED_REAL: BlockStateId = BlockStateId(2259);
/// M3 field-report fix (Task 3): the real `minecraft:piston_head` id (`type=normal,
/// facing=east, short=false`, cited directly off
/// `datagen-output/26.2/generated/reports/blocks.json`, protocol 776) that closed `piston.rs`'s
/// own former `PISTON_HEAD_IDS` placeholder table -- any id in `classify`'s own now-real
/// `PISTON_HEAD_RANGE` classifies identically, this one is chosen only to match
/// `piston_head_id`'s own real writes for facing=East.
const PISTON_HEAD: BlockStateId = BlockStateId(2275);
const CHEST: BlockStateId = BlockStateId(3988);
const FURNACE: BlockStateId = BlockStateId(5328);
const HOPPER: BlockStateId = BlockStateId(11313);
/// M3 field-report fix: real vanilla `PistonBaseBlock.isPushable`'s own hardcoded-identity
/// exception list (checked ahead of, and independent of, the `getDestroySpeed == -1` rule
/// `BEDROCK` above already exercises) — all four have positive hardness (breakable), so only
/// this explicit identity check makes them `Immovable`, not the bedrock rule. Ids read directly
/// off `datagen-output/26.2/generated/reports/blocks.json`, protocol 776 (single-state blocks
/// except `RESPAWN_ANCHOR`, whose `charges` property never affects pushability — the default,
/// `charges=0`, state id is used here).
const OBSIDIAN: BlockStateId = BlockStateId(3369);
const CRYING_OBSIDIAN: BlockStateId = BlockStateId(21820);
const RESPAWN_ANCHOR: BlockStateId = BlockStateId(21821);
const REINFORCED_DEEPSLATE: BlockStateId = BlockStateId(32085);

fn local_ownership() -> RegionOwnership {
    RegionOwnership::always_local(Address::Region(RegionId(0)))
}

#[test]
fn classify_matches_tier1_table() {
    let mut world = FakeWorld::new();
    let normal_rows = [STONE, DIRT, GRASS_BLOCK, PISTON_RETRACTED];
    for (i, id) in normal_rows.into_iter().enumerate() {
        let pos = BlockPos::new(i as i32, 0, 0);
        world.set_block(pos, id);
        assert_eq!(
            classify(&world, pos, true),
            PushClass::Normal,
            "{id:?} must be Normal"
        );
    }

    let immovable_rows = [
        BEDROCK,
        OBSIDIAN,
        CRYING_OBSIDIAN,
        RESPAWN_ANCHOR,
        REINFORCED_DEEPSLATE,
        PISTON_EXTENDED,
        PISTON_EXTENDED_REAL,
        PISTON_HEAD,
        CHEST,
        FURNACE,
        HOPPER,
    ];
    for (i, id) in immovable_rows.into_iter().enumerate() {
        let pos = BlockPos::new(i as i32, 1, 0);
        world.set_block(pos, id);
        assert_eq!(
            classify(&world, pos, true),
            PushClass::Immovable,
            "{id:?} must be Immovable"
        );
    }

    let destroy_rows = [REDSTONE_WIRE, REDSTONE_TORCH, REPEATER, COMPARATOR];
    for (i, id) in destroy_rows.into_iter().enumerate() {
        let pos = BlockPos::new(i as i32, 2, 0);
        world.set_block(pos, id);
        assert_eq!(
            classify(&world, pos, true),
            PushClass::Destroy,
            "{id:?} must be Destroy"
        );
    }
}

#[test]
fn push_stops_at_the_first_destroy_class_block_nondefault_case() {
    let mut world = FakeWorld::new();
    let piston = BlockPos::new(0, 0, 0);
    let stones: Vec<BlockPos> = (1..=3).map(|i| step(Direction::East, piston, i)).collect();
    for &p in &stones {
        world.set_block(p, STONE);
    }
    let torch = step(Direction::East, piston, 4);
    world.set_block(torch, REDSTONE_TORCH);
    // Position 5 (one step past the torch) is deliberately left unset (air/unloaded) -- never
    // reached, since Destroy terminates the walk.

    let plan = resolve_extend(&world, &local_ownership(), piston, Direction::East)
        .expect("a Destroy-terminated push must succeed");
    assert_eq!(plan.to_push, stones);
    assert_eq!(plan.to_destroy, Some(torch));
    assert!(!plan.to_push.contains(&torch));
}

#[test]
fn push_refuses_entirely_on_an_immovable_block() {
    let mut world = FakeWorld::new();
    let piston = BlockPos::new(0, 0, 0);
    world.set_block(step(Direction::East, piston, 1), STONE);
    world.set_block(step(Direction::East, piston, 2), STONE);
    world.set_block(step(Direction::East, piston, 3), BEDROCK);

    let result = resolve_extend(&world, &local_ownership(), piston, Direction::East);
    assert_eq!(result, Err(ExtendAbort::Blocked));
}

#[test]
fn push_refuses_at_exactly_thirteen_blocks_composition_case() {
    let mut world = FakeWorld::new();
    let piston = BlockPos::new(0, 0, 0);
    for i in 1..=13 {
        world.set_block(step(Direction::East, piston, i), STONE);
    }
    let result = resolve_extend(&world, &local_ownership(), piston, Direction::East);
    assert_eq!(result, Err(ExtendAbort::TooManyBlocks));

    let mut world12 = FakeWorld::new();
    for i in 1..=12 {
        world12.set_block(step(Direction::East, piston, i), STONE);
    }
    let plan = resolve_extend(&world12, &local_ownership(), piston, Direction::East)
        .expect("exactly 12 pushable blocks then air must succeed");
    assert_eq!(plan.to_push.len(), 12);
}

#[test]
fn sticky_retract_pulls_the_one_directly_adjacent_normal_block() {
    let mut world = FakeWorld::new();
    let piston = BlockPos::new(0, 0, 0);
    let head = Direction::East.apply(piston);
    let candidate = Direction::East.apply(head);
    world.set_block(candidate, STONE);

    let plan = resolve_retract(&world, &local_ownership(), piston, Direction::East, true);
    assert_eq!(
        plan,
        PullPlan {
            pulled: Some(candidate)
        }
    );
}

#[test]
fn sticky_retract_does_not_pull_an_immovable_or_destroy_class_block() {
    let piston = BlockPos::new(0, 0, 0);
    let head = Direction::East.apply(piston);
    let candidate = Direction::East.apply(head);

    for id in [BEDROCK, REDSTONE_WIRE] {
        let mut world = FakeWorld::new();
        world.set_block(candidate, id);
        let plan = resolve_retract(&world, &local_ownership(), piston, Direction::East, true);
        assert_eq!(plan, PullPlan { pulled: None }, "{id:?} must not be pulled");
    }
}

/// A `BlockWorldAccess` wrapping `FakeWorld` that counts `get_block` calls at one specific
/// position — this file's own local instrumentation (not a `support::FakeWorld` change, which
/// stays untouched, shared unmodified with every other blueprint's own test file).
struct CountingWorld {
    inner: FakeWorld,
    watched: BlockPos,
    reads_at_watched: AtomicUsize,
}

impl BlockWorldAccess for CountingWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        if pos == self.watched {
            self.reads_at_watched.fetch_add(1, Ordering::SeqCst);
        }
        self.inner.get_block(pos)
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        self.inner.set_block(pos, state)
    }
    fn dimension(&self) -> DimensionId {
        self.inner.dimension()
    }
    fn owner_of(&self, chunk: ChunkKey) -> Address {
        self.inner.owner_of(chunk)
    }
    fn local_identity(&self) -> Address {
        self.inner.local_identity()
    }
}

#[test]
fn non_sticky_retract_never_pulls_regardless_of_what_is_in_front() {
    let piston = BlockPos::new(0, 0, 0);
    let head = Direction::East.apply(piston);
    let candidate = Direction::East.apply(head);

    let mut inner = FakeWorld::new();
    inner.set_block(candidate, STONE);
    let world = CountingWorld {
        inner,
        watched: candidate,
        reads_at_watched: AtomicUsize::new(0),
    };

    let plan = resolve_retract(&world, &local_ownership(), piston, Direction::East, false);
    assert_eq!(plan, PullPlan { pulled: None });
    assert_eq!(
        world.reads_at_watched.load(Ordering::SeqCst),
        0,
        "a non-sticky retract must never even read the candidate position"
    );
}

#[test]
fn non_local_neighbor_is_treated_as_blocked() {
    let piston = BlockPos::new(0, 0, 0);
    let first = step(Direction::East, piston, 1);
    let second = step(Direction::East, piston, 2);

    let mut world = FakeWorld::new();
    world.set_block(first, STONE);
    // `second` deliberately left unset -- ownership must be checked before any "is this loaded"
    // fallback (MECH-D14: a non-local position is Immovable regardless of any cached content).

    let local = Address::Region(RegionId(1));
    let remote = Address::Region(RegionId(2));
    let second_chunk = second.chunk_key(DimensionId::OVERWORLD);
    let ownership = RegionOwnership {
        local,
        resolve: Box::new(
            move |chunk: ChunkKey| if chunk == second_chunk { remote } else { local },
        ),
    };

    let result = resolve_extend(&world, &ownership, piston, Direction::East);
    assert_eq!(result, Err(ExtendAbort::Blocked));
}
