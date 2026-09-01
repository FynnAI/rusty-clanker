//! The placement-differential capture format plus `diff_captures` — the azalea-free,
//! pure counterpart to `trace.rs`'s own `RedstoneTrace`/`diff_traces`, restated for
//! `xtask placement-diff` (`placement_spec.rs`'s own module doc comment has the full
//! rationale for why this is a new format rather than a reuse of `RedstoneTrace`: a
//! placement scenario has no tick dimension at all, and each scenario's own cell set
//! varies by `ClickedFace`/interaction kind rather than sharing one fixed
//! `bounds_min`/`bounds_max` volume).

use std::path::Path;

pub const CAPTURE_FORMAT_VERSION: u32 = 1;

/// One observed cell, relative to its own scenario's slot origin
/// (`placement_spec::slot_origin`/`interaction_slot_origin`) — never an absolute
/// world position, so a capture file stays comparable across two runs that assigned
/// scenarios to different concrete world coordinates.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellObservation {
    pub pos: (i32, i32, i32),
    pub state_id: u32,
    /// `true` iff a block-entity packet or chunk block-entity-list entry was ever
    /// observed at this cell this session (`InteractionScenario::ChestRejoinVisibility`'s
    /// own reason to exist — every other scenario always reads `false` here and never
    /// inspects the field).
    pub has_block_entity: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ScenarioCapture {
    pub scenario_id: String,
    pub cells: Vec<CellObservation>,
}

/// One full capture run's own output — either the real vanilla oracle's or our own
/// real server's, `source_label` distinguishes which (`"oracle:<jar sha1>"` or
/// `"ours"`, `placement_capture`'s own doc comment has the exact strings each side
/// writes).
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PlacementCaptureFile {
    pub format_version: u32,
    pub source_label: String,
    pub scenarios: Vec<ScenarioCapture>,
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureReadError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("postcard decode error reading {path}: {source}")]
    Decode {
        path: String,
        source: postcard::Error,
    },
}

pub fn write_capture(path: &Path, capture: &PlacementCaptureFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = postcard::to_allocvec(capture)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(path, bytes)
}

pub fn read_capture(path: &Path) -> Result<PlacementCaptureFile, CaptureReadError> {
    let bytes = std::fs::read(path).map_err(|source| CaptureReadError::Io {
        path: path.display().to_string(),
        source,
    })?;
    postcard::from_bytes(&bytes).map_err(|source| CaptureReadError::Decode {
        path: path.display().to_string(),
        source,
    })
}

/// One bit-exact divergence between the oracle's and our own server's observation of
/// the same scenario's same relative cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellMismatch {
    pub scenario_id: String,
    pub pos: (i32, i32, i32),
    pub oracle_state_id: u32,
    pub ours_state_id: u32,
    pub oracle_has_block_entity: bool,
    pub ours_has_block_entity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlacementDiffReport {
    pub mismatches: Vec<CellMismatch>,
    /// Scenario ids the oracle capture never produced a `ScenarioCapture` for at all
    /// (a scenario this run's own `--only` filter excluded on the oracle side, or a
    /// scenario whose oracle capture step itself failed) — surfaced separately from
    /// `mismatches` since there is no oracle cell to diff against at all, never
    /// silently skipped.
    pub missing_in_oracle: Vec<String>,
    pub missing_in_ours: Vec<String>,
}

/// Structural, bit-exact diff (mirrors `trace::diff_traces`'s own discipline): every
/// scenario id present in *both* captures is compared cell-by-cell, matched by
/// relative `pos` (never by list index — a scenario's own cell list order is an
/// implementation detail of whichever capture leg produced it, `diff_captures`'s own
/// test coverage exercises a deliberately reordered second capture to pin this).
/// Never panics on a structural mismatch (a cell present on one side but not the
/// other, for the same scenario) — folded into `mismatches` as a `u32::MAX`
/// sentinel-free reporting: a cell missing from one side is instead reported as its
/// own `CellMismatch` with that side's `state_id` left at the value `0`
/// (`minecraft:air`'s own state id, `corpus_capture.rs`'s established `AIR_STATE_ID`
/// convention) and `has_block_entity: false`, since "this cell was never observed" and
/// "this cell was observed as air" are operationally the same signal for this
/// harness's own purpose (a capture leg that silently never touched a cell is exactly
/// as suspicious as one that reports it empty).
pub fn diff_captures(
    oracle: &PlacementCaptureFile,
    ours: &PlacementCaptureFile,
) -> PlacementDiffReport {
    use std::collections::BTreeMap;

    let mut report = PlacementDiffReport::default();

    let oracle_by_id: BTreeMap<&str, &ScenarioCapture> = oracle
        .scenarios
        .iter()
        .map(|s| (s.scenario_id.as_str(), s))
        .collect();
    let ours_by_id: BTreeMap<&str, &ScenarioCapture> = ours
        .scenarios
        .iter()
        .map(|s| (s.scenario_id.as_str(), s))
        .collect();

    for id in ours_by_id.keys() {
        if !oracle_by_id.contains_key(id) {
            report.missing_in_oracle.push((*id).to_string());
        }
    }
    for id in oracle_by_id.keys() {
        if !ours_by_id.contains_key(id) {
            report.missing_in_ours.push((*id).to_string());
        }
    }

    for (id, oracle_scenario) in &oracle_by_id {
        let Some(ours_scenario) = ours_by_id.get(id) else {
            continue;
        };

        let oracle_cells: BTreeMap<(i32, i32, i32), CellObservation> =
            oracle_scenario.cells.iter().map(|c| (c.pos, *c)).collect();
        let ours_cells: BTreeMap<(i32, i32, i32), CellObservation> =
            ours_scenario.cells.iter().map(|c| (c.pos, *c)).collect();

        let mut all_positions: Vec<(i32, i32, i32)> = oracle_cells
            .keys()
            .chain(ours_cells.keys())
            .copied()
            .collect();
        all_positions.sort_unstable();
        all_positions.dedup();

        for pos in all_positions {
            let oracle_cell = oracle_cells.get(&pos);
            let ours_cell = ours_cells.get(&pos);
            let oracle_state = oracle_cell.map(|c| c.state_id).unwrap_or(0);
            let ours_state = ours_cell.map(|c| c.state_id).unwrap_or(0);
            let oracle_be = oracle_cell.is_some_and(|c| c.has_block_entity);
            let ours_be = ours_cell.is_some_and(|c| c.has_block_entity);
            if oracle_state != ours_state || oracle_be != ours_be {
                report.mismatches.push(CellMismatch {
                    scenario_id: (*id).to_string(),
                    pos,
                    oracle_state_id: oracle_state,
                    ours_state_id: ours_state,
                    oracle_has_block_entity: oracle_be,
                    ours_has_block_entity: ours_be,
                });
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(source: &str, scenarios: Vec<ScenarioCapture>) -> PlacementCaptureFile {
        PlacementCaptureFile {
            format_version: CAPTURE_FORMAT_VERSION,
            source_label: source.to_string(),
            scenarios,
        }
    }

    fn cell(pos: (i32, i32, i32), state_id: u32) -> CellObservation {
        CellObservation {
            pos,
            state_id,
            has_block_entity: false,
        }
    }

    #[test]
    fn identical_captures_diff_clean() {
        let a = cap(
            "oracle:abc",
            vec![ScenarioCapture {
                scenario_id: "stone/dir_north/face_top_of_floor/pitch_level".to_string(),
                cells: vec![cell((0, 1, 0), 1)],
            }],
        );
        let b = cap("ours", a.scenarios.clone());
        let report = diff_captures(&a, &b);
        assert!(report.mismatches.is_empty());
        assert!(report.missing_in_oracle.is_empty());
        assert!(report.missing_in_ours.is_empty());
    }

    #[test]
    fn differing_state_id_is_a_mismatch() {
        let a = cap(
            "oracle:abc",
            vec![ScenarioCapture {
                scenario_id: "hopper/dir_north/face_top_of_floor/pitch_level".to_string(),
                cells: vec![cell((0, 1, 0), 42)],
            }],
        );
        let b = cap(
            "ours",
            vec![ScenarioCapture {
                scenario_id: "hopper/dir_north/face_top_of_floor/pitch_level".to_string(),
                cells: vec![cell((0, 1, 0), 999)],
            }],
        );
        let report = diff_captures(&a, &b);
        assert_eq!(report.mismatches.len(), 1);
        assert_eq!(report.mismatches[0].oracle_state_id, 42);
        assert_eq!(report.mismatches[0].ours_state_id, 999);
    }

    #[test]
    fn cell_order_never_matters() {
        let scenario_id = "redstone_wire/dir_north/face_top_of_floor/pitch_level".to_string();
        let a = cap(
            "oracle:abc",
            vec![ScenarioCapture {
                scenario_id: scenario_id.clone(),
                cells: vec![cell((0, 1, 0), 1), cell((1, 1, 0), 2)],
            }],
        );
        let b = cap(
            "ours",
            vec![ScenarioCapture {
                scenario_id,
                // Deliberately reversed order.
                cells: vec![cell((1, 1, 0), 2), cell((0, 1, 0), 1)],
            }],
        );
        let report = diff_captures(&a, &b);
        assert!(report.mismatches.is_empty());
    }

    #[test]
    fn scenario_missing_on_one_side_is_reported_not_silently_skipped() {
        let a = cap(
            "oracle:abc",
            vec![ScenarioCapture {
                scenario_id: "stone/dir_north/face_top_of_floor/pitch_level".to_string(),
                cells: vec![cell((0, 1, 0), 1)],
            }],
        );
        let b = cap("ours", vec![]);
        let report = diff_captures(&a, &b);
        assert!(report.mismatches.is_empty());
        assert_eq!(
            report.missing_in_ours,
            vec!["stone/dir_north/face_top_of_floor/pitch_level".to_string()]
        );
        assert!(report.missing_in_oracle.is_empty());
    }

    #[test]
    fn block_entity_presence_disagreement_is_a_mismatch() {
        let scenario_id = "interaction/chest_rejoin_visibility".to_string();
        let a = cap(
            "oracle:abc",
            vec![ScenarioCapture {
                scenario_id: scenario_id.clone(),
                cells: vec![CellObservation {
                    pos: (0, 1, 0),
                    state_id: 5,
                    has_block_entity: true,
                }],
            }],
        );
        let b = cap(
            "ours",
            vec![ScenarioCapture {
                scenario_id,
                cells: vec![CellObservation {
                    pos: (0, 1, 0),
                    state_id: 5,
                    has_block_entity: false,
                }],
            }],
        );
        let report = diff_captures(&a, &b);
        assert_eq!(report.mismatches.len(), 1);
        assert!(report.mismatches[0].oracle_has_block_entity);
        assert!(!report.mismatches[0].ours_has_block_entity);
    }

    #[test]
    fn round_trips_through_postcard_on_disk() {
        let dir =
            std::env::temp_dir().join(format!("placement-trace-self-test-{}", std::process::id()));
        let path = dir.join("capture.postcard");
        let capture = cap(
            "oracle:deadbeef",
            vec![ScenarioCapture {
                scenario_id: "piston/dir_east/face_top_of_floor/pitch_looking_down".to_string(),
                cells: vec![cell((0, 1, 0), 7), cell((0, 2, 0), 8)],
            }],
        );
        write_capture(&path, &capture).unwrap();
        let read_back = read_capture(&path).unwrap();
        assert_eq!(read_back, capture);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
