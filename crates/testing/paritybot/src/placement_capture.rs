//! `xtask placement-diff`'s own live capture orchestration (governance changeset, "M3
//! field-report harness: a placement differential harness") — the azalea-backed
//! counterpart to `rc_gametest::placement_spec`'s pure geometry. Mirrors
//! `corpus_capture.rs`'s own architectural role exactly (that module's own doc comment
//! has the full citation for why a *live* bot connection lives here rather than in
//! `rc-gametest`), but for a structurally different job: `corpus_capture` drives the
//! real oracle's own console (`send_console_command`, `/setblock`/`/tp`/`tick step`)
//! and only ever *observes* through the bot; this module drives every placement,
//! hotbar selection, break, and movement *through the bot itself*, since the whole
//! point of this harness is to exercise the real client -> server wire path
//! (`UseItemOn`/`SetCreativeModeSlot`/`SetCarriedItem`) neither the redstone corpus nor
//! any console command ever touches. Runs unmodified against either a real vanilla
//! oracle or our own real `rusty-clanker-server` — both are already-listening
//! `host:port` Minecraft servers by the time `run_capture` is called; which kind of
//! process is behind that socket, and how it got started, is `placement_diff_runner`'s
//! own job, not this module's.
//!
//! No tick-freeze/barrier dance (contrast `corpus_capture.rs`'s own extensive one): a
//! placement scenario asks "what block state resulted", not "what state holds at this
//! *exact* tick boundary" — real, unfrozen ticks are exactly what a real client
//! experiences, and a generous fixed settle wait after each action
//! (`ACTION_SETTLE_TICKS`, mirroring `restart_persistence.rs`'s own established idiom)
//! is sufficient to observe the settled result.

use std::time::Duration;

use azalea::BlockPos as AzBlockPos;
use azalea::Client;
use azalea::core::position::Vec3 as AzVec3;
use azalea::pathfinder::PathfinderClientExt;
use azalea::pathfinder::goals::BlockPosGoal;
use azalea::protocol::packets::game::s_interact::InteractionHand;
use azalea::protocol::packets::game::s_player_action::{
    Action as PlayerActionKind, ServerboundPlayerAction,
};
use azalea::protocol::packets::game::s_use_item_on::{BlockHit, ServerboundUseItemOn};
use azalea::protocol::packets::game::{ServerboundSetCarriedItem, ServerboundSetCreativeModeSlot};
use azalea::registry::builtin::ItemKind;
use azalea_inventory::ItemStack;
use rc_gametest::placement_spec::{
    self, ApproachDirection, BlockKind, BotPitch, ClickedFace, Direction6, InteractionScenario,
    PlacementScenario,
};
use rc_gametest::{CellObservation, PlacementCaptureFile, ScenarioCapture};

use crate::packet_capture::{BlockSnapshotView, PacketCaptureError, connect_and_observe};

#[derive(Debug, thiserror::Error)]
pub enum PlacementCaptureError {
    #[error("bot connect failed: {0}")]
    BotConnect(#[from] PacketCaptureError),
    #[error("azalea error: {0}")]
    Azalea(String),
    #[error("timed out walking to {0:?}")]
    WalkTimeout((i32, i32, i32)),
    #[error(
        "floor-height discovery timed out — no non-air block ever observed below the bot's own spawn column within {0:?}"
    )]
    FloorDiscoveryTimeout(Duration),
}

/// Bot usernames — `[a-zA-Z0-9_]`, well under 16 characters (Context's own hard
/// constraint, `corpus_capture.rs`'s `CORPUS_BOT_NAME` doc comment has the full
/// "silently over-ran vanilla's own limit" field report this mirrors).
const MAIN_BOT_NAME: &str = "rc_place_bot";
const CHEST_BOT_A: &str = "rc_chest_bot_a";
const CHEST_BOT_B: &str = "rc_chest_bot_b";

const LOGIN_TIMEOUT: Duration = Duration::from_secs(30);
/// Governance fix (found live, `InteractionScenario::ChestRejoinVisibility`'s own
/// dedicated bot connections): `load_scenario.rs`'s own `STEP_TIMEOUT` (15s) is sized
/// for a short hop between two *nearby* points a long-lived, already-positioned bot
/// takes — every single-step scenario's own walk fits that shape (`SLOT_SPACING`/
/// `SLOT_ROW_SPACING` are both small). `capture_chest_rejoin_visibility`'s own two
/// bot connections are the one exception: each spawns fresh at the world's own spawn
/// point and must reach `interaction_slot_origin`'s own row in a single hop, which a
/// live run measured at ~150 blocks — comfortably clears 15s only under generous
/// real-world pathfinding/tick conditions this harness can't assume. 90s absorbs a
/// real single long walk with real margin without meaningfully weakening "the bot is
/// actually stuck" as a genuine failure signal.
const WALK_TIMEOUT: Duration = Duration::from_secs(90);
/// How many client-side ticks to wait after an action (hotbar select, place, break,
/// aim) before trusting the resulting observation — mirrors `restart_persistence.rs`'s
/// own `ACTION_SETTLE_TICKS`/`AIM_SETTLE_TICKS` idiom (that module's own doc comment
/// has the full "queued, not applied synchronously" rationale this restates).
const ACTION_SETTLE_TICKS: usize = 6;
/// A longer settle for the two-step interaction scenarios' own final observation,
/// generous enough to comfortably clear the task's own "capture torch cell after 2
/// ticks" budget with real margin — this harness's own job is to catch a wrong
/// *result*, never to pin an exact tick count (`corpus_capture.rs`'s own tick-freeze
/// barrier stays that job, for the redstone corpus specifically).
const INTERACTION_SETTLE_TICKS: usize = 10;
const AIM_SETTLE_TICKS: usize = 3;
/// How far past the bot's own look direction to place a synthetic aim target
/// (`aim_bot`'s own doc comment) — comfortably past every reach distance this
/// harness ever validates against, so the resulting yaw/pitch lands deep inside
/// `nearest_horizontal_direction4`/`nearest_direction6`'s own dominant-axis buckets
/// regardless of the bot's own real stance position.
const AIM_TARGET_DISTANCE: f64 = 100.0;

/// `kind`'s own real `minecraft:item` registry entry, for `ServerboundSetCreativeModeSlot
/// ` (mirrors `crates/server/src/play/mining.rs::placeable_kind_for_item_id`'s own
/// reverse mapping — same 13-entry closed set (`Lever` added, M3.5-B03 follow-up
/// deliverable 6), restated here against azalea's own independently-generated
/// `ItemKind`, both pinned to the identical protocol-776 registry snapshot,
/// cross-checked live in this harness's own implementation report).
pub(crate) fn item_kind_for(kind: BlockKind) -> ItemKind {
    match kind {
        BlockKind::Stone => ItemKind::Stone,
        BlockKind::RedstoneWire => ItemKind::Redstone,
        BlockKind::RedstoneTorch => ItemKind::RedstoneTorch,
        BlockKind::Repeater => ItemKind::Repeater,
        BlockKind::Comparator => ItemKind::Comparator,
        BlockKind::Piston => ItemKind::Piston,
        BlockKind::StickyPiston => ItemKind::StickyPiston,
        BlockKind::Chest => ItemKind::Chest,
        BlockKind::Furnace => ItemKind::Furnace,
        BlockKind::BlastFurnace => ItemKind::BlastFurnace,
        BlockKind::Smoker => ItemKind::Smoker,
        BlockKind::Hopper => ItemKind::Hopper,
        BlockKind::Lever => ItemKind::Lever,
    }
}

pub(crate) fn to_az_direction(direction: Direction6) -> azalea::core::direction::Direction {
    use azalea::core::direction::Direction as AzDirection;
    match direction {
        Direction6::Down => AzDirection::Down,
        Direction6::Up => AzDirection::Up,
        Direction6::North => AzDirection::North,
        Direction6::South => AzDirection::South,
        Direction6::West => AzDirection::West,
        Direction6::East => AzDirection::East,
    }
}

/// Monotonically increasing per-connection sequence counter — every `UseItemOn`/
/// `PlayerAction` this module ever sends carries its own next value (Context: the
/// vanilla wire protocol's own opaque, client-chosen "interaction sequence" echoed
/// back verbatim by `AcknowledgeBlockChange`, never independently validated —
/// `play_creative_hotbar_held_item.rs`'s own established convention).
///
/// `pub(crate)` (M3.5-B05 addition): `block_entity_persistence`'s own placement leg
/// reuses this exact hotbar-select/aim/`UseItemOn` machinery rather than duplicating
/// it a second time. `pub` (wider still, M3.5-B03): `redstone_wire_capture::
/// capture_contraption_over_wire`'s own `pub` signature (§4.4) threads a `&mut
/// SeqCounter` through its own public parameter list, which requires this type (and
/// its sole field, to construct one) to be at least as visible as that function
/// itself.
pub struct SeqCounter(pub i32);
impl SeqCounter {
    pub(crate) fn next(&mut self) -> i32 {
        self.0 += 1;
        self.0
    }
}

impl SeqCounter {
    pub(crate) fn new() -> Self {
        Self(0)
    }
}

/// Selects `kind`'s own item into hotbar slot 0 and carries it — the exact two real
/// packets `play_creative_hotbar_held_item.rs` decodes
/// (`SetCreativeModeSlot`/`SetCarriedItem`), sent here through azalea's own
/// `Client::write_packet` instead of a raw loopback socket. Slot `36` is
/// `InventoryMenu.USE_ROW_SLOT_START` (that test's own doc comment has the full
/// citation) — the full-container index for hotbar slot 0, which `SetCarriedItem`
/// then addresses directly as bare index `0`.
pub(crate) async fn select_item(client: &Client, kind: BlockKind) {
    client.write_packet(ServerboundSetCreativeModeSlot {
        slot_num: 36,
        item_stack: ItemStack::new(item_kind_for(kind), 1),
    });
    client.write_packet(ServerboundSetCarriedItem { slot: 0 });
    client.wait_ticks(ACTION_SETTLE_TICKS).await;
}

/// Points the bot's own look direction at a synthetic target far along the ray
/// `(yaw_degrees, pitch_degrees)` describes from the bot's own *current* position —
/// `crates/server/src/play/mining.rs::look_vector`'s own formula, restated
/// independently (this crate's own black-box posture, `placement_spec.rs`'s module doc
/// comment). A large `AIM_TARGET_DISTANCE` keeps the resulting real yaw/pitch deep
/// inside production's own dominant-axis classification buckets regardless of the
/// bot's own exact stance, `ApproachDirection::yaw_degrees`/`BotPitch::pitch_degrees`'s
/// own doc comments have the full margin argument. Settles for `AIM_SETTLE_TICKS`
/// (`restart_persistence.rs`'s own established `look_at_click` idiom) so the resulting
/// rotation packet has actually reached the server before the caller's own next action
/// depends on it.
pub(crate) async fn aim_bot(
    client: &Client,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> Result<(), PlacementCaptureError> {
    let pos = client
        .position()
        .map_err(|err| PlacementCaptureError::Azalea(err.to_string()))?;
    let yaw_rad = (yaw_degrees as f64).to_radians();
    let pitch_rad = (pitch_degrees as f64).to_radians();
    let dx = -yaw_rad.sin() * pitch_rad.cos();
    let dy = -pitch_rad.sin();
    let dz = yaw_rad.cos() * pitch_rad.cos();
    let target = AzVec3 {
        x: pos.x + dx * AIM_TARGET_DISTANCE,
        y: pos.y + dy * AIM_TARGET_DISTANCE,
        z: pos.z + dz * AIM_TARGET_DISTANCE,
    };
    client.look_at(target);
    client.wait_ticks(AIM_SETTLE_TICKS).await;
    Ok(())
}

/// Sends one real `UseItemOn` with an explicit, hand-fabricated `BlockHit` (task
/// Context: "`UseItemOn` with explicit block hit (position, face, cursor)") — never
/// azalea's own automatic raycast-derived hit result
/// (`azalea_client::interact::StartUseItemQueued`'s own `force_block` fallback
/// resolves to an arbitrary `Direction::Up`/center-cursor guess whenever the bot isn't
/// *exactly* looking at the forced block, this harness's own implementation report has
/// the full citation for why that path is unsuitable here) — so every scenario's own
/// declared `clicked_face`/cursor is realized byte-for-byte regardless of the bot's
/// own real aim precision. `clicked` and the resulting placement both still require
/// the bot's own *real* look direction/position to satisfy production's own
/// yaw/pitch-driven orientation rule and reach check respectively (`aim_bot`/
/// `walk_to` are always called before this).
pub(crate) fn send_use_item_on(
    client: &Client,
    seq: &mut SeqCounter,
    clicked: (i32, i32, i32),
    face: Direction6,
    cursor: (f32, f32, f32),
) {
    let block_pos = AzBlockPos::new(clicked.0, clicked.1, clicked.2);
    let location = AzVec3 {
        x: block_pos.x as f64 + cursor.0 as f64,
        y: block_pos.y as f64 + cursor.1 as f64,
        z: block_pos.z as f64 + cursor.2 as f64,
    };
    client.write_packet(ServerboundUseItemOn {
        hand: InteractionHand::MainHand,
        block_hit: BlockHit {
            block_pos,
            direction: to_az_direction(face),
            location,
            inside: false,
            world_border: false,
        },
        seq: seq.next() as u32,
    });
}

/// Places `kind`'s own item at `clicked`'s `face`, after aiming the bot along
/// `yaw_degrees`/`pitch_degrees` — the one full placement step every scenario, single-
/// or multi-step, ultimately bottoms out at.
pub(crate) async fn place(
    client: &Client,
    seq: &mut SeqCounter,
    kind: BlockKind,
    clicked: (i32, i32, i32),
    face: Direction6,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> Result<(), PlacementCaptureError> {
    select_item(client, kind).await;
    aim_bot(client, yaw_degrees, pitch_degrees).await?;
    send_use_item_on(
        client,
        seq,
        clicked,
        face,
        placement_spec::face_cursor(face),
    );
    client.wait_ticks(ACTION_SETTLE_TICKS).await;
    Ok(())
}

/// Instantly breaks (creative-mode `StartDestroyBlock` alone, `crates/server/src/play/
/// world.rs`'s own `instabuild` branch — restated independently, real vanilla creative
/// mode has the identical single-click-instant-break rule) the block at `pos`.
/// `direction` is not read by production's own break path at all (only placement
/// orientation reads a face) — `Up`, arbitrarily but consistently.
pub(crate) async fn break_block(client: &Client, seq: &mut SeqCounter, pos: (i32, i32, i32)) {
    client.write_packet(ServerboundPlayerAction {
        action: PlayerActionKind::StartDestroyBlock,
        pos: AzBlockPos::new(pos.0, pos.1, pos.2),
        direction: azalea::core::direction::Direction::Up,
        seq: seq.next() as u32,
    });
    client.wait_ticks(ACTION_SETTLE_TICKS).await;
}

/// Walks the bot to `target` (a real block position, e.g. one cell above a scenario's
/// own floor-relative stance) via azalea's own pathfinder — `load_scenario.rs`'s own
/// established `goto(BlockPosGoal(..))` idiom, timeout-wrapped identically
/// (`STEP_TIMEOUT`'s own doc comment there). Real walking, not a raw position packet:
/// works identically against a real vanilla oracle (which validates movement) and our
/// own server (which currently trusts any position — `crates/server/src/play/world.rs`
/// 's own `PlayerMarker::position` doc comment — but this module never relies on that
/// asymmetry, so a future movement-validation addition to our own server can never
/// silently break this harness).
pub(crate) async fn walk_to(
    client: &Client,
    target: (i32, i32, i32),
) -> Result<(), PlacementCaptureError> {
    let goal = BlockPosGoal(AzBlockPos::new(target.0, target.1, target.2));
    tokio::time::timeout(WALK_TIMEOUT, client.goto(goal))
        .await
        .map_err(|_| PlacementCaptureError::WalkTimeout(target))?;
    Ok(())
}

/// Discovers this session's own natural superflat floor height (governance addition:
/// the real vanilla oracle's own default `level-type=flat` preset and this project's
/// own `SuperflatFiller` layer table are never assumed to agree — verified live per
/// this harness's own implementation report — so every scenario's own absolute world Y
/// is resolved once, per side, against whatever floor height that *specific* session's
/// own server actually reports, rather than a hardcoded constant): scans downward from
/// a generous ceiling at the bot's own current column for the first non-air state,
/// polling until the column's own chunk has actually loaded into azalea's world model
/// or `timeout` elapses. A genuinely flat world (both sides' own launch configuration,
/// `placement_diff_runner`'s own doc comment) reports the identical height at every
/// column, so this one measurement is valid for every scenario's own slot.
pub(crate) async fn discover_floor_y(
    client: &Client,
    view: &BlockSnapshotView,
    timeout: Duration,
) -> Result<i32, PlacementCaptureError> {
    const CEILING: i32 = 24;
    const FLOOR_SCAN_BOTTOM: i32 = -64;
    let pos = client
        .position()
        .map_err(|err| PlacementCaptureError::Azalea(err.to_string()))?;
    let x = pos.x.floor() as i32;
    let z = pos.z.floor() as i32;

    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        for y in (FLOOR_SCAN_BOTTOM..=CEILING).rev() {
            if let Some(state_id) = view.state_id_at((x, y, z))
                && state_id != 0
            {
                return Ok(y);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(PlacementCaptureError::FloorDiscoveryTimeout(timeout));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// One relative-to-`floor_y` position resolved to an absolute world position — `y == 0`
/// is the floor's own top solid surface (`discover_floor_y`'s own doc comment), matching
/// `placement_spec`'s own documented convention for every slot origin.
pub(crate) fn absolute(floor_y: i32, relative: (i32, i32, i32)) -> (i32, i32, i32) {
    (relative.0, floor_y + relative.1, relative.2)
}

/// Snapshots `cells` (relative to `origin`) — `state_id_at`, plus `has_block_entity_at`
/// for the one scenario that reads it (`InteractionScenario::ChestRejoinVisibility`).
/// A cell whose chunk never loaded reports `state_id: 0` (`AIR_STATE_ID`,
/// `corpus_capture.rs`'s own established convention) rather than failing the whole
/// capture — an unreachable/never-loaded cell is itself a meaningful (if suspicious)
/// observation for `diff_captures` to compare, never silently dropped.
fn snapshot_cells(
    view: &BlockSnapshotView,
    floor_y: i32,
    origin: (i32, i32, i32),
    cells: &[(i32, i32, i32)],
) -> Vec<CellObservation> {
    cells
        .iter()
        .map(|&rel| {
            let world = absolute(
                floor_y,
                (origin.0 + rel.0, origin.1 + rel.1, origin.2 + rel.2),
            );
            CellObservation {
                pos: rel,
                state_id: view.state_id_at(world).unwrap_or(0),
                has_block_entity: view.has_block_entity_at(world),
            }
        })
        .collect()
}

fn stance_for(floor_y: i32, origin: (i32, i32, i32)) -> (i32, i32, i32) {
    let (dx, dy, dz) = placement_spec::STANCE_OFFSET;
    absolute(floor_y, (origin.0 + dx, origin.1 + dy, origin.2 + dz))
}

/// Captures one single-step `PlacementScenario`: walk to its own stance, place a rig
/// block first if `ClickedFace` needs one, place the probe, snapshot every cell
/// `placement_spec::scenario_cells` declares.
async fn capture_single_step(
    client: &Client,
    view: &BlockSnapshotView,
    seq: &mut SeqCounter,
    floor_y: i32,
    scenario: &PlacementScenario,
) -> Result<ScenarioCapture, PlacementCaptureError> {
    let origin = placement_spec::slot_origin(scenario.kind, scenario.slot_index);
    let geometry = placement_spec::face_geometry(
        scenario.face,
        placement_spec::wall_face_for(scenario.kind, scenario.approach),
    );
    let stance = stance_for(floor_y, origin);
    walk_to(client, stance).await?;

    let yaw = scenario.approach.yaw_degrees();
    let pitch = scenario.pitch.pitch_degrees();

    // Rig construction (Context: only `SideOfWall`/`BottomOfCeiling` need one at all —
    // `placement_spec::face_geometry`'s own doc comment). The two faces need
    // structurally different construction sequences (`face_geometry`'s own doc
    // comment on `BottomOfCeiling` has the full "temp stepping stone, then break it"
    // account), so each is handled by its own dedicated arm rather than a shared "place
    // at `geometry.rig`" step that would silently double-place `SideOfWall`'s own
    // single-cell rig if written to also cover `BottomOfCeiling`'s multi-step one (a
    // real bug an earlier draft of this function had — recorded here so it is never
    // reintroduced).
    match scenario.face {
        ClickedFace::TopOfFloor => {
            // No rig — the natural floor itself is the support
            // (`face_geometry(TopOfFloor).rig == None`).
        }
        ClickedFace::SideOfWall => {
            // The rig is always a plain `Stone` block, orientation-independent
            // (`resolve_orientation`'s own `Stone | RedstoneWire => Orientation::None`
            // row) — its own approach/pitch never matters, so it is always placed
            // level, facing the scenario's own baseline direction, to keep rig
            // placement itself outside this harness's own diffed signal entirely.
            // Clicking the floor's own `Up` face lands it exactly at
            // `geometry.rig` (`(0, 1, 0)`, `face_geometry`'s own doc comment).
            place(
                client,
                seq,
                BlockKind::Stone,
                absolute(floor_y, origin),
                Direction6::Up,
                ApproachDirection::North.yaw_degrees(),
                BotPitch::Level.pitch_degrees(),
            )
            .await?;
        }
        ClickedFace::BottomOfCeiling => {
            // `placement_spec::face_geometry`'s own `BottomOfCeiling` doc comment:
            // build a temporary stepping stone at the target cell, place the real
            // ceiling rig on top of it, then break the stepping stone — the rig (not
            // gravity-affected) stays floating, freeing the target cell for the probe.
            let temp = absolute(floor_y, (origin.0, origin.1 + 1, origin.2));
            place(
                client,
                seq,
                BlockKind::Stone,
                absolute(floor_y, origin),
                Direction6::Up,
                ApproachDirection::North.yaw_degrees(),
                BotPitch::Level.pitch_degrees(),
            )
            .await?;
            place(
                client,
                seq,
                BlockKind::Stone,
                temp,
                Direction6::Up,
                ApproachDirection::North.yaw_degrees(),
                BotPitch::Level.pitch_degrees(),
            )
            .await?;
            break_block(client, seq, temp).await;
        }
    }

    let clicked_world = absolute(
        floor_y,
        (
            origin.0 + geometry.clicked.0,
            origin.1 + geometry.clicked.1,
            origin.2 + geometry.clicked.2,
        ),
    );
    place(
        client,
        seq,
        scenario.kind,
        clicked_world,
        geometry.clicked_face,
        yaw,
        pitch,
    )
    .await?;

    let cells = placement_spec::scenario_cells(scenario);
    Ok(ScenarioCapture {
        scenario_id: scenario.id.clone(),
        cells: snapshot_cells(view, floor_y, origin, &cells),
    })
}

/// (a) Place wire, then place a second wire adjacent — capture both states.
async fn capture_wire_wire_connection(
    client: &Client,
    view: &BlockSnapshotView,
    seq: &mut SeqCounter,
    floor_y: i32,
    slot_index: usize,
) -> Result<ScenarioCapture, PlacementCaptureError> {
    let origin = placement_spec::interaction_slot_origin(slot_index);
    let stance = stance_for(floor_y, origin);
    walk_to(client, stance).await?;

    // Floor for both wire cells.
    for rel in [(0, 0, 0), (1, 0, 0)] {
        place(
            client,
            seq,
            BlockKind::Stone,
            absolute(
                floor_y,
                (origin.0 + rel.0, origin.1 + rel.1, origin.2 + rel.2),
            ),
            Direction6::Up,
            ApproachDirection::North.yaw_degrees(),
            BotPitch::Level.pitch_degrees(),
        )
        .await?;
    }

    // Wire A at (0, 1, 0), clicking the floor's Up face at (0, 0, 0).
    place(
        client,
        seq,
        BlockKind::RedstoneWire,
        absolute(floor_y, (origin.0, origin.1, origin.2)),
        Direction6::Up,
        ApproachDirection::North.yaw_degrees(),
        BotPitch::Level.pitch_degrees(),
    )
    .await?;

    // Wire B adjacent at (1, 1, 0).
    place(
        client,
        seq,
        BlockKind::RedstoneWire,
        absolute(floor_y, (origin.0 + 1, origin.1, origin.2)),
        Direction6::Up,
        ApproachDirection::North.yaw_degrees(),
        BotPitch::Level.pitch_degrees(),
    )
    .await?;

    client.wait_ticks(INTERACTION_SETTLE_TICKS).await;

    let cells = [(0, 1, 0), (1, 1, 0)];
    Ok(ScenarioCapture {
        scenario_id: InteractionScenario::WireWireConnection.id().to_string(),
        cells: snapshot_cells(view, floor_y, origin, &cells),
    })
}

/// (b) Place torch on a stone support, then break the support — capture the torch
/// cell after it settles.
async fn capture_torch_pop_on_support_break(
    client: &Client,
    view: &BlockSnapshotView,
    seq: &mut SeqCounter,
    floor_y: i32,
    slot_index: usize,
) -> Result<ScenarioCapture, PlacementCaptureError> {
    let origin = placement_spec::interaction_slot_origin(slot_index);
    let stance = stance_for(floor_y, origin);
    walk_to(client, stance).await?;

    // Floor, then a support column at (0, 1, 0), then a torch on top of it at (0, 2, 0).
    place(
        client,
        seq,
        BlockKind::Stone,
        absolute(floor_y, origin),
        Direction6::Up,
        ApproachDirection::North.yaw_degrees(),
        BotPitch::Level.pitch_degrees(),
    )
    .await?;
    let support = absolute(floor_y, (origin.0, origin.1 + 1, origin.2));
    place(
        client,
        seq,
        BlockKind::RedstoneTorch,
        support,
        Direction6::Up,
        ApproachDirection::North.yaw_degrees(),
        BotPitch::Level.pitch_degrees(),
    )
    .await?;

    // Break the support out from under the torch.
    break_block(client, seq, support).await;
    client.wait_ticks(INTERACTION_SETTLE_TICKS).await;

    let cells = [(0, 1, 0), (0, 2, 0)];
    Ok(ScenarioCapture {
        scenario_id: InteractionScenario::TorchPopOnSupportBreak.id().to_string(),
        cells: snapshot_cells(view, floor_y, origin, &cells),
    })
}

/// (c) Place torch, then place wire adjacent — capture the wire's power state.
async fn capture_wire_power_from_adjacent_torch(
    client: &Client,
    view: &BlockSnapshotView,
    seq: &mut SeqCounter,
    floor_y: i32,
    slot_index: usize,
) -> Result<ScenarioCapture, PlacementCaptureError> {
    let origin = placement_spec::interaction_slot_origin(slot_index);
    let stance = stance_for(floor_y, origin);
    walk_to(client, stance).await?;

    for rel in [(0, 0, 0), (1, 0, 0)] {
        place(
            client,
            seq,
            BlockKind::Stone,
            absolute(
                floor_y,
                (origin.0 + rel.0, origin.1 + rel.1, origin.2 + rel.2),
            ),
            Direction6::Up,
            ApproachDirection::North.yaw_degrees(),
            BotPitch::Level.pitch_degrees(),
        )
        .await?;
    }

    // Torch standing on its own floor cell at (0, 1, 0).
    place(
        client,
        seq,
        BlockKind::RedstoneTorch,
        absolute(floor_y, origin),
        Direction6::Up,
        ApproachDirection::North.yaw_degrees(),
        BotPitch::Level.pitch_degrees(),
    )
    .await?;

    // Wire adjacent at (1, 1, 0), clicking the second floor cell's Up face.
    place(
        client,
        seq,
        BlockKind::RedstoneWire,
        absolute(floor_y, (origin.0 + 1, origin.1, origin.2)),
        Direction6::Up,
        ApproachDirection::North.yaw_degrees(),
        BotPitch::Level.pitch_degrees(),
    )
    .await?;

    client.wait_ticks(INTERACTION_SETTLE_TICKS).await;

    let cells = [(0, 1, 0), (1, 1, 0)];
    Ok(ScenarioCapture {
        scenario_id: InteractionScenario::WirePowerFromAdjacentTorch
            .id()
            .to_string(),
        cells: snapshot_cells(view, floor_y, origin, &cells),
    })
}

/// (d) Place chest, disconnect the bot, reconnect, capture whether the chunk's own
/// block-entity list carries the chest. Uses two short-lived, dedicated bot
/// connections (`CHEST_BOT_A`/`CHEST_BOT_B`) rather than the caller's own long-lived
/// main-loop bot — disconnecting *that* one would end every remaining scenario's own
/// capture, and reconnecting under the identical account name back-to-back risks a
/// "still logging off" race against the server's own disconnect handling neither side
/// needs to solve for this harness to do its job.
async fn capture_chest_rejoin_visibility(
    host: &str,
    port: u16,
    slot_index: usize,
) -> Result<ScenarioCapture, PlacementCaptureError> {
    let origin = placement_spec::interaction_slot_origin(slot_index);

    let (view_a, observer_a) = connect_and_observe(host, port, CHEST_BOT_A, LOGIN_TIMEOUT).await?;
    let client_a = view_a
        .client()
        .expect("connect_and_observe only returns after Event::Spawn");
    let floor_y = discover_floor_y(&client_a, &view_a, LOGIN_TIMEOUT).await?;
    let mut seq = SeqCounter(0);

    let stance = stance_for(floor_y, origin);
    walk_to(&client_a, stance).await?;
    place(
        &client_a,
        &mut seq,
        BlockKind::Stone,
        absolute(floor_y, origin),
        Direction6::Up,
        ApproachDirection::North.yaw_degrees(),
        BotPitch::Level.pitch_degrees(),
    )
    .await?;
    place(
        &client_a,
        &mut seq,
        BlockKind::Chest,
        absolute(floor_y, origin),
        Direction6::Up,
        ApproachDirection::North.yaw_degrees(),
        BotPitch::Level.pitch_degrees(),
    )
    .await?;
    client_a.wait_ticks(INTERACTION_SETTLE_TICKS).await;

    client_a.disconnect();
    drop(observer_a);
    // A clean disconnect is not instantaneous server-side — a brief real-time grace
    // before the reconnect avoids racing this same slot's own player-session teardown
    // (mirrors this crate's own established "give the server a moment" idiom used
    // wherever a session boundary is crossed, e.g. `restart_persistence.rs`'s own
    // server-restart wait).
    tokio::time::sleep(Duration::from_millis(500)).await;

    let (view_b, observer_b) = connect_and_observe(host, port, CHEST_BOT_B, LOGIN_TIMEOUT).await?;
    let client_b = view_b
        .client()
        .expect("connect_and_observe only returns after Event::Spawn");
    walk_to(&client_b, stance).await?;
    client_b.wait_ticks(INTERACTION_SETTLE_TICKS).await;

    let cells = snapshot_cells(&view_b, floor_y, origin, &[(0, 1, 0)]);
    client_b.disconnect();
    drop(observer_b);

    Ok(ScenarioCapture {
        scenario_id: InteractionScenario::ChestRejoinVisibility.id().to_string(),
        cells,
    })
}

/// Full capture run against one already-listening `host:port` Minecraft server —
/// `placement_diff_runner`'s own bin target is the only caller, once per side, after
/// launching that side's own real server process (module doc comment). `only` filters
/// to a single scenario id, mirroring every other verb's own `--only` convention
/// (`fetch_corpus.rs`/`parity_check.rs`).
pub async fn run_capture(
    host: &str,
    port: u16,
    scenarios: &[PlacementScenario],
    interactions: &[InteractionScenario],
    only: Option<&str>,
    source_label: String,
) -> Result<PlacementCaptureFile, PlacementCaptureError> {
    let (view, _observer) = connect_and_observe(host, port, MAIN_BOT_NAME, LOGIN_TIMEOUT).await?;
    let client = view
        .client()
        .expect("connect_and_observe only returns after Event::Spawn");
    let floor_y = discover_floor_y(&client, &view, LOGIN_TIMEOUT).await?;
    let mut seq = SeqCounter(0);

    let mut out = PlacementCaptureFile {
        format_version: rc_gametest::CAPTURE_FORMAT_VERSION,
        source_label,
        scenarios: Vec::new(),
    };

    // Governance fix (resilience): one scenario's own transient failure (a pathfinder
    // timeout, an unresolved floor height on a slow-to-load chunk) no longer aborts
    // this whole run via a bare `?` — every remaining scenario still gets its own
    // chance, and the failed one simply never gets a `ScenarioCapture` entry, which
    // `diff_captures`'s own `missing_in_oracle`/`missing_in_ours` reporting already
    // surfaces as its own loud, dedicated case rather than the entire run silently
    // producing zero results because scenario 50 of 90 hit a one-off timeout.
    for scenario in scenarios {
        if only.is_some_and(|only| only != scenario.id) {
            continue;
        }
        match capture_single_step(&client, &view, &mut seq, floor_y, scenario).await {
            Ok(capture) => out.scenarios.push(capture),
            Err(err) => {
                eprintln!("placement_capture: scenario {} failed: {err}", scenario.id);
            }
        }
    }

    for (index, interaction) in interactions.iter().enumerate() {
        if only.is_some_and(|only| only != interaction.id()) {
            continue;
        }
        let outcome = match interaction {
            InteractionScenario::WireWireConnection => {
                capture_wire_wire_connection(&client, &view, &mut seq, floor_y, index).await
            }
            InteractionScenario::TorchPopOnSupportBreak => {
                capture_torch_pop_on_support_break(&client, &view, &mut seq, floor_y, index).await
            }
            InteractionScenario::WirePowerFromAdjacentTorch => {
                capture_wire_power_from_adjacent_torch(&client, &view, &mut seq, floor_y, index)
                    .await
            }
            InteractionScenario::ChestRejoinVisibility => {
                // Never shares the main bot's own connection (this function's own doc
                // comment on `capture_chest_rejoin_visibility`) — drives its own pair
                // of short-lived connections against the same `host`/`port` instead.
                capture_chest_rejoin_visibility(host, port, index).await
            }
        };
        match outcome {
            Ok(capture) => out.scenarios.push(capture),
            Err(err) => {
                eprintln!(
                    "placement_capture: scenario {} failed: {err}",
                    interaction.id()
                );
            }
        }
    }

    client.disconnect();

    Ok(out)
}
