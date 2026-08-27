//! M3-B07 — `BlockSnapshotView`'s own initial-state contract, independent of any
//! azalea/network behavior (blueprint Acceptance tests, `packet_capture_types.rs`).

use rc_paritybot::packet_capture::BlockSnapshotView;

#[test]
fn block_snapshot_view_defaults_to_none() {
    let view = BlockSnapshotView::new();

    assert_eq!(view.state_id_at((0, 0, 0)), None);
    assert_eq!(view.state_id_at((100, -64, -100)), None);
    assert_eq!(view.analog_at((0, 0, 0)), None);
    assert_eq!(view.analog_at((100, -64, -100)), None);
}
