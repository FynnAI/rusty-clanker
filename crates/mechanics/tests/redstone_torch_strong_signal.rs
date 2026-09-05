//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=yes self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain, see redstone_wire.rs/redstone_repeater.rs) nondefault-state=yes
//! M3 field-report test-authoring (finding 4, `docs/planning/05-game-mechanics.md` MECH-D13's
//! lever sentence names this same per-face sturdiness family; the strong-signal axis itself is
//! a `RedstoneTorchBlock`/`RedstoneWallTorchBlock` fact): vanilla's `getDirectSignal` is
//! hard-coded to fire only straight up (the receiver sits directly ABOVE the torch), for BOTH
//! the floor and wall variants — `RedstoneWallTorchBlock` never overrides it.
//! `TorchBehavior::direct_signal_toward` used to derive the strong-signal axis from the
//! attachment (`input_direction().opposite()`), which happens to equal `Up` for a floor torch
//! (`input_direction() == Down`, `.opposite() == Up`) but is wrong for a wall torch — it fires
//! sideways, toward the torch's own `facing`, instead of up. This project's own `towards` runs
//! source -> receiver (`signal.rs`'s `direct_signal_to` calls `direct_signal_toward(npos,
//! d.opposite())`), so the fix is attachment-independent: `self.lit(pos) && towards ==
//! Direction::Up`.

mod support;

use std::sync::Arc;

use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    RedstoneSignalSource, SignalSourceRegistry, TorchAttachment, TorchBehavior,
};

use support::FakeWorld;

fn torch_with(attachment: TorchAttachment) -> Arc<TorchBehavior> {
    let torch = Arc::new(TorchBehavior::new(attachment));
    torch.bind_registry(Arc::new(SignalSourceRegistry::new()));
    torch
}

#[test]
fn wall_torch_direct_signal_fires_only_straight_up_orientation_case() {
    let world = FakeWorld::new();
    let torch = torch_with(TorchAttachment::Wall(Direction::East));
    let pos = BlockPos::new(0, 0, 0);
    assert_eq!(
        torch.direct_signal_toward(&world, pos, Direction::Up),
        15,
        "wall torch's own direct/strong signal must fire straight up regardless of its own \
         facing (finding 4 — vanilla's getDirectSignal is hard-coded to the receiver directly \
         above the torch)"
    );
    assert_eq!(
        torch.direct_signal_toward(&world, pos, Direction::East),
        0,
        "wall torch must NOT fire its direct signal sideways, toward its own facing"
    );
    assert_eq!(torch.direct_signal_toward(&world, pos, Direction::West), 0);
    assert_eq!(torch.direct_signal_toward(&world, pos, Direction::Down), 0);
}

#[test]
fn floor_torch_direct_signal_fires_only_straight_up_nondefault_case() {
    let world = FakeWorld::new();
    let torch = torch_with(TorchAttachment::Floor);
    let pos = BlockPos::new(0, 0, 0);
    assert_eq!(
        torch.direct_signal_toward(&world, pos, Direction::Up),
        15,
        "floor torch's own direct/strong signal fires straight up — already correct before \
         this fix, kept green as a regression lock"
    );
    assert_eq!(torch.direct_signal_toward(&world, pos, Direction::North), 0);
    assert_eq!(torch.direct_signal_toward(&world, pos, Direction::Down), 0);
}
