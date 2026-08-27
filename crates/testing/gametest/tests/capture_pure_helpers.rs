//! M3-B07 — `check_state_id_consistency`, the one pure function of the capture
//! pipeline (blueprint Acceptance tests, `capture_pure_helpers.rs`). No real oracle,
//! no network, no locally installed Java, required to go green.

use rc_gametest::capture::check_state_id_consistency;
use rc_gametest::spec::PlacedBlock;

fn sample_block(state_id: u32) -> PlacedBlock {
    PlacedBlock {
        pos: (0, 1, 0),
        vanilla_state: "minecraft:redstone_torch[lit=true]".to_string(),
        state_id,
        has_analog_state: false,
    }
}

#[test]
fn check_state_id_consistency_passes_on_match() {
    let declared = sample_block(100);
    assert_eq!(check_state_id_consistency(&declared, 100), Ok(()));
}

#[test]
fn check_state_id_consistency_flags_mismatch() {
    let declared = sample_block(100);
    let result = check_state_id_consistency(&declared, 101);
    assert_eq!(result, Err((100, 101)));
}
