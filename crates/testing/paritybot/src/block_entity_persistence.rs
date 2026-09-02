//! M3.5-B05's own restart-round-trip scenario, live-bot legs only
//! (`AC_block_entities_survive_restart`, blueprint Section 4 steps 1 and 3). The
//! disk-content seeding/verification legs (steps 2 and 4 — "xtask process, no server
//! running") live in `xtask::m3_5_be_report` directly, against `rc-chunk-storage`/
//! `rc-mechanics`, never here (this crate's own binary must never link into `xtask.exe`
//! — the module doc comment on every prior `xtask *_report` verb has the full
//! "azalea's own upstream nightly-toolchain requirement" rationale, unchanged here).
//!
//! No container-open protocol path exists in this codebase at all (`chest.rs`'s own
//! doc comment: "the full container-menu system are explicitly out of scope") — a real
//! client therefore has no wire-protocol way to put items *into* a chest/furnace/hopper
//! it did not already spawn with contents. This module's own job is narrower and
//! honest about that gap: place each of the three tier-1 block-entity kinds (empty,
//! exactly as a real client's `UseItemOn` would leave them) and prove they are really
//! there — `xtask::m3_5_be_report`'s own disk-seeding leg is what establishes non-empty
//! contents as a starting condition, working directly against this blueprint's own
//! production `BlockEntityCodec`/`ChunkNbtCodec` code path.
//!
//! Follows `restart_persistence.rs`'s own established connect/recenter/disconnect
//! shape and `placement_capture.rs`'s own hotbar-select + `UseItemOn` machinery
//! (reused directly via `pub(crate)`, not duplicated) — no precise aim geometry is
//! needed for either (`restart_persistence.rs`'s own governance-note addendum: the
//! real server-side reach check is a direction-independent box-distance predicate).

use std::time::Duration;

use azalea::prelude::*;
use rc_gametest::placement_spec::{BlockKind, Direction6};
use rc_registries::generated_v776::block_states::default_state::{CHEST, FURNACE, HOPPER};

use crate::packet_capture::{BlockSnapshotView, PacketCaptureError, connect_and_observe};
use crate::placement_capture::{SeqCounter, select_item, send_use_item_on};

pub const BOT_USERNAME: &str = "rc_m35_be_bot";

/// The three fixed test positions this scenario places/observes — one per tier-1
/// block-entity kind, all inside chunk (0,0) near `HardcodedWorld`'s own spawn,
/// each clicked on the floor cell directly below it with face `Up` (mirrors
/// `restart_persistence.rs`'s own established target-set/click convention).
pub const CHEST_POS: (i32, i32, i32) = (2, -59, 0);
pub const FURNACE_POS: (i32, i32, i32) = (3, -59, 0);
pub const HOPPER_POS: (i32, i32, i32) = (2, -59, 1);

const AIM_SETTLE_TICKS: usize = 3;
const ACTION_SETTLE_TICKS: usize = 6;
/// How long a fresh join's own initial chunk batch (and its embedded block-entity
/// list, `LevelChunkWithLight.block_entities`) needs to settle into azalea's world
/// model before `observe_presence` trusts what it reads — mirrors
/// `restart_persistence.rs`'s own `SETTLE_GRACE` idiom, in ticks rather than wall time
/// since this leg already waits on `Event::Spawn` via `connect_and_observe`.
const OBSERVE_SETTLE_TICKS: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum BlockEntityPlacementError {
    #[error("bot connect failed: {0}")]
    Connect(#[from] PacketCaptureError),
    #[error("azalea error: {0}")]
    Azalea(String),
    #[error(
        "placing {kind} at {pos:?} did not take effect: expected raw block-state id {expected}, observed {observed:?}"
    )]
    PlacementRejected {
        kind: &'static str,
        pos: (i32, i32, i32),
        expected: u32,
        observed: Option<u32>,
    },
}

/// Recenters the bot within its own spawn block — an integer `BlockPos` corner cast
/// straight to `f64` (a brand-new player's own join position) leaves the farthest of
/// this scenario's own three targets at the edge of reach; recentering opens a
/// comfortable reachable window for all three (mirrors `restart_persistence.rs`'s own
/// identically-purposed, independently-verified helper).
fn recenter_in_spawn_block(client: &Client) -> Result<(), BlockEntityPlacementError> {
    client
        .query_self::<&mut azalea::entity::Position, _>(|mut pos| {
            pos.x = pos.x.floor() + 0.5;
            pos.z = pos.z.floor() + 0.5;
        })
        .map_err(|err| BlockEntityPlacementError::Azalea(err.to_string()))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn place_and_verify(
    client: &Client,
    view: &BlockSnapshotView,
    seq: &mut SeqCounter,
    kind: BlockKind,
    kind_label: &'static str,
    pos: (i32, i32, i32),
    expected_state: u32,
) -> Result<(), BlockEntityPlacementError> {
    let below = (pos.0, pos.1 - 1, pos.2);
    select_item(client, kind).await;
    send_use_item_on(client, seq, below, Direction6::Up, (0.5, 1.0, 0.5));
    client.wait_ticks(ACTION_SETTLE_TICKS).await;

    let observed = view.state_id_at(pos);
    if observed != Some(expected_state) {
        return Err(BlockEntityPlacementError::PlacementRejected {
            kind: kind_label,
            pos,
            expected: expected_state,
            observed,
        });
    }
    Ok(())
}

/// Spawn #1 (blueprint Section 4, step 1): places `minecraft:chest`/`furnace`/`hopper`
/// at the three fixed positions above, verifying each one's own block state lands
/// before moving to the next (`place_and_verify`'s own doc comment — the same
/// "close that hole" discipline `restart_persistence.rs::verify_effect` established),
/// then performs a clean disconnect.
pub async fn apply_placements(
    host: &str,
    port: u16,
    login_timeout: Duration,
) -> Result<(), BlockEntityPlacementError> {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(apply_placements_inner(host, port, login_timeout))
        .await
}

async fn apply_placements_inner(
    host: &str,
    port: u16,
    login_timeout: Duration,
) -> Result<(), BlockEntityPlacementError> {
    let (view, _observer) = connect_and_observe(host, port, BOT_USERNAME, login_timeout).await?;
    let client = view
        .client()
        .expect("connect_and_observe only returns after Event::Spawn");
    recenter_in_spawn_block(&client)?;
    client.wait_ticks(AIM_SETTLE_TICKS).await;

    let mut seq = SeqCounter::new();
    place_and_verify(
        &client,
        &view,
        &mut seq,
        BlockKind::Chest,
        "chest",
        CHEST_POS,
        CHEST.0,
    )
    .await?;
    place_and_verify(
        &client,
        &view,
        &mut seq,
        BlockKind::Furnace,
        "furnace",
        FURNACE_POS,
        FURNACE.0,
    )
    .await?;
    place_and_verify(
        &client,
        &view,
        &mut seq,
        BlockKind::Hopper,
        "hopper",
        HOPPER_POS,
        HOPPER.0,
    )
    .await?;

    client.disconnect();
    Ok(())
}

/// One position's own observed state after `observe_presence` — `state_id` mirrors
/// ordinary chunk content (the block itself, already proven to survive a restart
/// since M2); `has_block_entity` is the weak, `encode_block_entities`-derives-from-
/// raw-block-state-id-alone check the blueprint's own Context names (real proof that
/// the ECS round trip worked lives in `xtask::m3_5_be_report`'s own disk-comparison
/// leg, step 4, not here).
#[derive(Debug, Clone, Copy)]
pub struct ObservedPosition {
    pub pos: (i32, i32, i32),
    pub state_id: Option<u32>,
    pub has_block_entity: bool,
}

/// Spawn #2 (blueprint Section 4, step 3): reads the three positions' own block state
/// and whether each one's own chunk-packet block-entity list carries an entry there.
pub async fn observe_presence(
    host: &str,
    port: u16,
    login_timeout: Duration,
) -> Result<Vec<ObservedPosition>, BlockEntityPlacementError> {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(observe_presence_inner(host, port, login_timeout))
        .await
}

async fn observe_presence_inner(
    host: &str,
    port: u16,
    login_timeout: Duration,
) -> Result<Vec<ObservedPosition>, BlockEntityPlacementError> {
    let (view, _observer) = connect_and_observe(host, port, BOT_USERNAME, login_timeout).await?;
    let client = view
        .client()
        .expect("connect_and_observe only returns after Event::Spawn");
    client.wait_ticks(OBSERVE_SETTLE_TICKS).await;

    let results = [CHEST_POS, FURNACE_POS, HOPPER_POS]
        .into_iter()
        .map(|pos| ObservedPosition {
            pos,
            state_id: view.state_id_at(pos),
            has_block_entity: view.has_block_entity_at(pos),
        })
        .collect();

    client.disconnect();
    Ok(results)
}
