//! TEST-D59: the `xtask protocol-diff` known-divergence register — parsing,
//! validation, and the pure per-step resolution pass that classifies every
//! `protocol_capture::diff_captures` mismatch against it. The register file itself
//! (`crates/testing/gametest/corpus/protocol-diff/known-divergences.ron`) is
//! planning-owned content (TEST-D46 protected path, governance changesets only) —
//! this module owns only the mechanism that reads and applies it, never its content.
//!
//! Resolution design note (deviation from TEST-D59's own literal "dropped before
//! comparison" wording for `Timer` entries, recorded in
//! `docs/findings-for-planning.md`): rather than removing a `Timer`-registered packet
//! type from each side's own captured packet list *before* `diff_step` ever runs
//! (which would make a genuinely observed divergence invisible, including for
//! reporting), this module classifies `Timer` entries at exactly the same stage as
//! `Missing`/`Body` entries — after `diff_step` has already produced its
//! `ProtocolDiffReport`, `resolve_step` reclassifies each of its
//! `missing_in_oracle`/`missing_in_ours`/`mismatches` entries as `known` whenever a
//! matching register entry covers it (`Timer` covering both a presence-only and a
//! count/body divergence, per TEST-D59's own "presence and count are ignored in both
//! directions" text), or leaves it in the still-failing set otherwise. The observable
//! effect is identical (a `Timer`-covered divergence never fails a step), but this
//! shape lets a genuinely observed `Timer` divergence still surface as `known
//! (timer-driven)` in the case detail — TEST-D59's own "reports a registered
//! divergence as a pass case" text — rather than silently vanishing pre-comparison.

use std::path::Path;

use crate::protocol_capture::{PacketTypeDiff, ProtocolDiffReport};

/// TEST-D59's own three-way class taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
pub enum DivergenceClass {
    /// A packet type one side never sends yet — a `missing_in_oracle`/
    /// `missing_in_ours` divergence.
    Missing,
    /// A field-level body difference — a `mismatches` divergence.
    Body,
    /// Presence/count of this packet type within a step window is decided by a
    /// wall-clock or tick timer on each server independently — ignored, both
    /// directions, permanently (module doc comment's own resolution-design note).
    Timer,
}

/// One committed register row: `(steps, packet, class, closes_with, expires)`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct KnownDivergence {
    /// An exact step/contraption id, or a glob of the form `"<prefix ending in
    /// '/'>*"` (`matches_step`'s own doc comment has the exact matching rule).
    pub steps: String,
    /// A namespaced clientbound packet resource name, e.g. `"minecraft:set_time"`
    /// (`protocol_packet_catalog`'s own tables are this field's validation source).
    pub packet: String,
    pub class: DivergenceClass,
    /// Required (`Some`) for `Missing`/`Body`, forbidden (`None`) for `Timer` —
    /// `load_register`'s own validation enforces this.
    pub closes_with: Option<String>,
    /// Required (`Some`) for `Missing`/`Body`, forbidden (`None`) for `Timer`.
    pub expires: Option<String>,
}

impl KnownDivergence {
    /// TEST-D59 register v2: `"<step id or glob such as session/place/*, or the bare
    /// wildcard *>"` — the bare literal `"*"` matches every step id (every
    /// scripted-session step **and** every `redstone/<category>/<name>` contraption
    /// id — the register's own `Timer` entries need exactly this, since a timer-
    /// driven packet like `keep_alive` diverges on both kinds of step and
    /// `"session/*"` never matches a contraption id at all); `"<prefix ending in
    /// '/'>*"` is a prefix match against everything up to and including that `/`;
    /// anything else is an exact match against `step_id`. A `steps` value ending in
    /// `*` with no immediately preceding `/` and not equal to the bare `"*"` (not a
    /// glob this schema defines at all) is compared as a plain exact string instead
    /// of ever being treated as a wildcard — `step_id` can never itself contain a
    /// literal `*`, so this can only ever under-match, never accidentally match
    /// everything.
    pub fn matches_step(&self, step_id: &str) -> bool {
        if self.steps == "*" {
            return true;
        }
        if let Some(prefix) = self.steps.strip_suffix('*')
            && prefix.ends_with('/')
        {
            return step_id.starts_with(prefix);
        }
        self.steps == step_id
    }

    /// `packet` compared against `resolved_name` (the already-resolved, unnamespaced
    /// name `protocol_capture::CapturedPacket::packet_name`/`ProtocolDiffReport::
    /// packet_names` carries — azalea's own `ProtocolPacket::name()` never includes
    /// the `"minecraft:"` prefix) with the register's own namespaced `packet` field
    /// stripped of that same prefix before comparing.
    pub fn matches_packet(&self, resolved_name: &str) -> bool {
        self.packet
            .strip_prefix("minecraft:")
            .unwrap_or(&self.packet)
            == resolved_name
    }
}

/// TEST-D59: "packet name must resolve in the packets mapping the normalizer uses
/// (state-aware...)". State is decided by `steps`'s own *exact* text: the two literal
/// step ids `"session/login"`/`"session/configuration"` select their own single
/// table; `"session/disconnect_reconnect"`/`"session/observe_chunk"` (M3.5-B03
/// follow-up, `docs/findings-for-planning.md`) can each legitimately carry packets
/// from ALL THREE protocol states within the one capture (`protocol_session::
/// resolve_multi_phase`'s own doc comment: a real reconnect crosses Login->
/// Configuration->Play, and the pinned oracle's own mid-play `start_configuration`
/// resync crosses back again) — validated against the union of all three catalogs
/// rather than any single one, since a register entry naming a genuine
/// Login/Configuration-state packet for either of these two steps is exactly as
/// valid as one naming a Play-state packet; every other exact id, every glob
/// (`"session/place/*"`, `"session/*"`, ...), and every redstone-corpus contraption id
/// resolves against the play-state table alone — the only table a step whose own
/// identity isn't pinned to one specific handshake state (and isn't one of the two
/// mixed-state steps above) could ever be safely validated against.
fn packet_name_known(steps: &str, packet: &str) -> bool {
    use crate::protocol_packet_catalog::{
        CONFIGURATION_CLIENTBOUND_PACKET_NAMES, LOGIN_CLIENTBOUND_PACKET_NAMES,
        PLAY_CLIENTBOUND_PACKET_NAMES,
    };
    match steps {
        "session/login" => LOGIN_CLIENTBOUND_PACKET_NAMES.contains(&packet),
        "session/configuration" => CONFIGURATION_CLIENTBOUND_PACKET_NAMES.contains(&packet),
        "session/disconnect_reconnect" | "session/observe_chunk" => {
            LOGIN_CLIENTBOUND_PACKET_NAMES.contains(&packet)
                || CONFIGURATION_CLIENTBOUND_PACKET_NAMES.contains(&packet)
                || PLAY_CLIENTBOUND_PACKET_NAMES.contains(&packet)
        }
        _ => PLAY_CLIENTBOUND_PACKET_NAMES.contains(&packet),
    }
}

/// Reads and validates `path` as the TEST-D59 register: `Missing`/`Body` entries
/// require both `closes_with` and `expires`; `Timer` entries require neither; every
/// entry's `packet` must resolve in the state-appropriate `protocol_packet_catalog`
/// table; no two entries may share the same `(steps, packet, class)` triple. Any
/// violation is `Err` naming the offending entry — never a partial/best-effort
/// register.
///
/// Register v2 deviation from v1's `(steps, packet)`-only key (recorded in
/// `docs/findings-for-planning.md`): `Missing` covers only a presence-set divergence
/// and `Body` covers only a body/count divergence (`resolve_step`'s own
/// `PRESENCE_CLASSES`/`BODY_CLASSES` split) — a packet type can genuinely need both
/// kinds of coverage across the different concrete step ids one glob matches (e.g.
/// `chunk_batch_start` is missing entirely on some `session/*`-matching steps but
/// present-with-a-differing-body on others), so one `Missing` entry and one `Body`
/// entry for the identical `(steps, packet)` pair are two independent, non-redundant
/// rows, not a duplicate — only a literal repeat of the same `(steps, packet, class)`
/// triple is ever a copy/paste mistake this check exists to catch.
pub fn load_register(path: &Path) -> Result<Vec<KnownDivergence>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let entries: Vec<KnownDivergence> = ron::from_str(&text)
        .map_err(|err| format!("failed to parse {} as RON: {err}", path.display()))?;

    let mut seen: std::collections::BTreeSet<(String, String, DivergenceClass)> =
        std::collections::BTreeSet::new();
    for entry in &entries {
        match entry.class {
            DivergenceClass::Timer => {
                if entry.closes_with.is_some() || entry.expires.is_some() {
                    return Err(format!(
                        "{}: Timer entry (steps={:?}, packet={:?}) must carry neither \
                         closes_with nor expires",
                        path.display(),
                        entry.steps,
                        entry.packet
                    ));
                }
            }
            DivergenceClass::Missing | DivergenceClass::Body => {
                if entry.closes_with.is_none() || entry.expires.is_none() {
                    return Err(format!(
                        "{}: {:?} entry (steps={:?}, packet={:?}) requires both \
                         closes_with and expires",
                        path.display(),
                        entry.class,
                        entry.steps,
                        entry.packet
                    ));
                }
            }
        }

        if !packet_name_known(&entry.steps, &entry.packet) {
            return Err(format!(
                "{}: unknown packet {:?} for steps {:?} — not found in the state-\
                 appropriate clientbound packet catalog (protocol_packet_catalog.rs)",
                path.display(),
                entry.packet,
                entry.steps
            ));
        }

        if !seen.insert((entry.steps.clone(), entry.packet.clone(), entry.class)) {
            return Err(format!(
                "{}: duplicate register entry for (steps={:?}, packet={:?}, class={:?})",
                path.display(),
                entry.steps,
                entry.packet,
                entry.class
            ));
        }
    }

    Ok(entries)
}

/// Which of a step's own `ProtocolDiffReport` collections a `KnownEntryMatch` came
/// from — report-only, lets a caller format e.g. "present only in ours" differently
/// from "body/count mismatch" without re-deriving it from `matched.class`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchKind {
    MissingInOracle,
    MissingInOurs,
    Mismatch,
}

/// One `ProtocolDiffReport` entry the register covers.
pub struct KnownEntryMatch<'a> {
    pub packet_id: i32,
    pub packet_name: Option<String>,
    pub kind: MismatchKind,
    pub matched: &'a KnownDivergence,
}

/// One step's own resolution verdict — `resolve_step`'s return value.
pub struct StepVerdict<'a> {
    pub known: Vec<KnownEntryMatch<'a>>,
    pub unregistered_missing_in_oracle: Vec<i32>,
    pub unregistered_missing_in_ours: Vec<i32>,
    pub unregistered_mismatches: Vec<&'a PacketTypeDiff>,
}

impl StepVerdict<'_> {
    /// A step passes when nothing unregistered remains (TEST-D59's own wording,
    /// verbatim) — a step with only `known` entries (or none at all) still passes.
    pub fn passes(&self) -> bool {
        self.unregistered_missing_in_oracle.is_empty()
            && self.unregistered_missing_in_ours.is_empty()
            && self.unregistered_mismatches.is_empty()
    }
}

fn find_match<'a>(
    register: &'a [KnownDivergence],
    step_id: &str,
    packet_name: Option<&str>,
    allowed: &[DivergenceClass],
) -> Option<&'a KnownDivergence> {
    let name = packet_name?;
    register
        .iter()
        .find(|e| allowed.contains(&e.class) && e.matches_step(step_id) && e.matches_packet(name))
}

/// TEST-D59's own per-step diff resolution (Deliverable 2): classifies every entry of
/// `report`'s own `missing_in_oracle`/`missing_in_ours`/`mismatches` against
/// `register`, by packet name (`report.packet_names`, populated by `diff_step` for
/// every packet id it ever saw on either side — an id absent from that map can never
/// match any register entry and always stays unregistered, TEST-D59's own "an
/// unresolvable id can never match an entry" clause). `Missing`/`Timer` entries cover
/// presence-only divergences; `Body`/`Timer` entries cover `mismatches`.
pub fn resolve_step<'a>(
    step_id: &str,
    report: &'a ProtocolDiffReport,
    register: &'a [KnownDivergence],
) -> StepVerdict<'a> {
    let mut verdict = StepVerdict {
        known: Vec::new(),
        unregistered_missing_in_oracle: Vec::new(),
        unregistered_missing_in_ours: Vec::new(),
        unregistered_mismatches: Vec::new(),
    };

    const PRESENCE_CLASSES: &[DivergenceClass] =
        &[DivergenceClass::Missing, DivergenceClass::Timer];
    const BODY_CLASSES: &[DivergenceClass] = &[DivergenceClass::Body, DivergenceClass::Timer];

    for &id in &report.missing_in_oracle {
        let name = report.packet_names.get(&id).cloned();
        match find_match(register, step_id, name.as_deref(), PRESENCE_CLASSES) {
            Some(entry) => verdict.known.push(KnownEntryMatch {
                packet_id: id,
                packet_name: name,
                kind: MismatchKind::MissingInOracle,
                matched: entry,
            }),
            None => verdict.unregistered_missing_in_oracle.push(id),
        }
    }
    for &id in &report.missing_in_ours {
        let name = report.packet_names.get(&id).cloned();
        match find_match(register, step_id, name.as_deref(), PRESENCE_CLASSES) {
            Some(entry) => verdict.known.push(KnownEntryMatch {
                packet_id: id,
                packet_name: name,
                kind: MismatchKind::MissingInOurs,
                matched: entry,
            }),
            None => verdict.unregistered_missing_in_ours.push(id),
        }
    }
    for diff in &report.mismatches {
        match find_match(register, step_id, diff.packet_name.as_deref(), BODY_CLASSES) {
            Some(entry) => verdict.known.push(KnownEntryMatch {
                packet_id: diff.packet_id,
                packet_name: diff.packet_name.clone(),
                kind: MismatchKind::Mismatch,
                matched: entry,
            }),
            None => verdict.unregistered_mismatches.push(diff),
        }
    }

    verdict
}

/// TEST-D59's own expiry check: every `Missing`/`Body` entry (`Timer` entries carry
/// no `expires` at all and never appear here) whose own `expires` milestone
/// `is_milestone_complete` reports `true` for — "an expired entry is a regression in
/// disguise". Deliberately takes the completeness check as an injected closure rather
/// than reading `blueprints/` itself, so this stays a pure fold over `register` the
/// same way the rest of this module is — resolving `blueprints/<milestone>/*-
/// COMPLETION-REPORT.md` against a real checkout is `xtask`'s own job (it already
/// resolves `repo_root`), not this azalea-free crate's.
pub fn expired_entries(
    register: &[KnownDivergence],
    is_milestone_complete: impl Fn(&str) -> bool,
) -> Vec<&KnownDivergence> {
    register
        .iter()
        .filter(|e| e.class != DivergenceClass::Timer)
        .filter(|e| e.expires.as_deref().is_some_and(&is_milestone_complete))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol_capture::{CapturedPacket, PROTOCOL_CAPTURE_FORMAT_VERSION};
    use crate::protocol_capture::{
        ContraptionBounds, ProtocolCaptureFile, StepCapture, diff_captures,
    };

    fn write_temp_ron(name: &str, contents: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rc-gametest-known-divergences-{name}-{}-{}",
            std::process::id(),
            name.len()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("known-divergences.ron");
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn a_well_formed_register_loads_cleanly() {
        let path = write_temp_ron(
            "valid",
            r#"[
                (steps: "session/*", packet: "minecraft:keep_alive", class: Timer, closes_with: None, expires: None),
                (steps: "session/spawn", packet: "minecraft:change_difficulty", class: Missing, closes_with: Some("NET hardening"), expires: Some("M5")),
                (steps: "session/login", packet: "minecraft:login_finished", class: Body, closes_with: Some("M4-B01"), expires: Some("M4")),
            ]"#,
        );
        let entries = load_register(&path).expect("valid register loads");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].class, DivergenceClass::Timer);
    }

    #[test]
    fn a_timer_entry_with_an_expiry_is_rejected() {
        let path = write_temp_ron(
            "timer-with-expiry",
            r#"[
                (steps: "session/*", packet: "minecraft:keep_alive", class: Timer, closes_with: None, expires: Some("M4")),
            ]"#,
        );
        let err = load_register(&path).expect_err("Timer entry with expiry must be rejected");
        assert!(err.contains("neither"), "unexpected error: {err}");
    }

    #[test]
    fn a_missing_entry_without_closes_with_is_rejected() {
        let path = write_temp_ron(
            "missing-without-closes-with",
            r#"[
                (steps: "session/spawn", packet: "minecraft:commands", class: Missing, closes_with: None, expires: Some("M5")),
            ]"#,
        );
        let err =
            load_register(&path).expect_err("Missing entry without closes_with must be rejected");
        assert!(err.contains("requires both"), "unexpected error: {err}");
    }

    #[test]
    fn a_body_entry_without_expires_is_rejected() {
        let path = write_temp_ron(
            "body-without-expires",
            r#"[
                (steps: "session/login", packet: "minecraft:login_finished", class: Body, closes_with: Some("M4-B01"), expires: None),
            ]"#,
        );
        let err = load_register(&path).expect_err("Body entry without expires must be rejected");
        assert!(err.contains("requires both"), "unexpected error: {err}");
    }

    #[test]
    fn an_unknown_packet_name_is_a_parse_error() {
        let path = write_temp_ron(
            "unknown-packet",
            r#"[
                (steps: "session/spawn", packet: "minecraft:totally_made_up_packet", class: Missing, closes_with: Some("M4-B01"), expires: Some("M4")),
            ]"#,
        );
        let err = load_register(&path).expect_err("unknown packet name must be rejected");
        assert!(err.contains("unknown packet"), "unexpected error: {err}");
    }

    #[test]
    fn wrong_state_table_makes_a_real_packet_name_unresolvable() {
        // `minecraft:login_finished` is a real *login*-state clientbound packet, but
        // this entry claims it for `session/spawn` (play state) — must fail exactly
        // like a typo, since `session/spawn` never resolves against the login table.
        let path = write_temp_ron(
            "wrong-state-table",
            r#"[
                (steps: "session/spawn", packet: "minecraft:login_finished", class: Missing, closes_with: Some("M4-B01"), expires: Some("M4")),
            ]"#,
        );
        let err = load_register(&path).expect_err("wrong-state packet name must be rejected");
        assert!(err.contains("unknown packet"), "unexpected error: {err}");
    }

    #[test]
    fn a_duplicate_steps_packet_pair_is_rejected() {
        let path = write_temp_ron(
            "duplicate",
            r#"[
                (steps: "session/*", packet: "minecraft:keep_alive", class: Timer, closes_with: None, expires: None),
                (steps: "session/*", packet: "minecraft:keep_alive", class: Timer, closes_with: None, expires: None),
            ]"#,
        );
        let err =
            load_register(&path).expect_err("duplicate (steps, packet) pair must be rejected");
        assert!(err.contains("duplicate"), "unexpected error: {err}");
    }

    fn missing_entry(steps: &str, packet: &str) -> KnownDivergence {
        KnownDivergence {
            steps: steps.to_string(),
            packet: packet.to_string(),
            class: DivergenceClass::Missing,
            closes_with: Some("M4-B01".to_string()),
            expires: Some("M4".to_string()),
        }
    }

    #[test]
    fn glob_matches_exact_and_prefixed_step_ids_but_not_others() {
        let entry = missing_entry("session/place/*", "minecraft:add_entity");
        assert!(entry.matches_step("session/place/stone"));
        assert!(entry.matches_step("session/place/hopper"));
        assert!(!entry.matches_step("session/break/stone"));
        assert!(!entry.matches_step("session/place"));

        let exact = missing_entry("session/spawn", "minecraft:commands");
        assert!(exact.matches_step("session/spawn"));
        assert!(!exact.matches_step("session/spawn2"));
        assert!(!exact.matches_step("session/place/stone"));
    }

    #[test]
    fn packet_matching_strips_the_minecraft_namespace() {
        let entry = missing_entry("session/spawn", "minecraft:commands");
        assert!(entry.matches_packet("commands"));
        assert!(!entry.matches_packet("minecraft:commands"));
        assert!(!entry.matches_packet("command_suggestions"));
    }

    fn pkt(index: u32, packet_id: i32, body: Vec<u8>, name: Option<&str>) -> CapturedPacket {
        CapturedPacket {
            index,
            packet_id,
            body,
            packet_name: name.map(str::to_string),
        }
    }

    fn one_step_report(
        oracle: Vec<CapturedPacket>,
        ours: Vec<CapturedPacket>,
    ) -> ProtocolDiffReport {
        let oracle_file = ProtocolCaptureFile {
            format_version: PROTOCOL_CAPTURE_FORMAT_VERSION,
            source_label: "oracle:abc".to_string(),
            steps: vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: oracle,
            }],
        };
        let ours_file = ProtocolCaptureFile {
            format_version: PROTOCOL_CAPTURE_FORMAT_VERSION,
            source_label: "ours".to_string(),
            steps: vec![StepCapture {
                step_id: "session/spawn".to_string(),
                observe_from: 0,
                packets: ours,
            }],
        };
        diff_captures(&oracle_file, &ours_file, &ContraptionBounds::new())
            .remove("session/spawn")
            .expect("session/spawn present")
    }

    #[test]
    fn a_missing_entry_turns_an_oracle_only_type_into_known() {
        let report = one_step_report(vec![pkt(0, 55, vec![1, 2, 3], Some("commands"))], vec![]);
        assert_eq!(report.missing_in_ours, vec![55]);

        let register = vec![missing_entry("session/spawn", "minecraft:commands")];
        let verdict = resolve_step("session/spawn", &report, &register);

        assert!(verdict.passes());
        assert_eq!(verdict.known.len(), 1);
        assert_eq!(verdict.known[0].kind, MismatchKind::MissingInOurs);
        assert!(verdict.unregistered_missing_in_ours.is_empty());
    }

    #[test]
    fn a_timer_entry_suppresses_a_count_mismatch_regardless_of_direction() {
        // Oracle sends `keep_alive` twice, ours sends it once — a pure count
        // mismatch (`mismatches`, not a presence-set entry, since both sides sent
        // it at least once). `keep_alive`'s own `NORMALIZATION_RULES` row masks the
        // whole body, so the two bodies are already both `vec![]` after
        // normalization -- only the excess count of `1` distinguishes them.
        let report = one_step_report(
            vec![
                pkt(0, 33, vec![1; 8], Some("keep_alive")),
                pkt(1, 33, vec![2; 8], Some("keep_alive")),
            ],
            vec![pkt(0, 33, vec![3; 8], Some("keep_alive"))],
        );
        assert_eq!(report.mismatches.len(), 1);

        let register = vec![KnownDivergence {
            steps: "session/*".to_string(),
            packet: "minecraft:keep_alive".to_string(),
            class: DivergenceClass::Timer,
            closes_with: None,
            expires: None,
        }];
        let verdict = resolve_step("session/spawn", &report, &register);

        assert!(verdict.passes());
        assert_eq!(verdict.known.len(), 1);
        assert_eq!(verdict.known[0].kind, MismatchKind::Mismatch);
    }

    #[test]
    fn an_unregistered_type_still_fails() {
        let report = one_step_report(
            vec![pkt(0, 9, vec![1, 2, 3], Some("block_update"))],
            vec![pkt(0, 9, vec![1, 2, 4], Some("block_update"))],
        );
        let verdict = resolve_step("session/spawn", &report, &[]);
        assert!(!verdict.passes());
        assert_eq!(verdict.unregistered_mismatches.len(), 1);
        assert!(verdict.known.is_empty());
    }

    #[test]
    fn an_id_with_no_resolvable_name_never_matches_any_entry() {
        let report = one_step_report(vec![pkt(0, 200, vec![1], None)], vec![]);
        let register = vec![KnownDivergence {
            steps: "session/*".to_string(),
            packet: "minecraft:commands".to_string(),
            class: DivergenceClass::Timer,
            closes_with: None,
            expires: None,
        }];
        let verdict = resolve_step("session/spawn", &report, &register);
        assert!(!verdict.passes());
        assert_eq!(verdict.unregistered_missing_in_ours, vec![200]);
    }

    #[test]
    fn expired_entries_flags_only_missing_and_body_whose_milestone_is_complete() {
        let register = vec![
            missing_entry("session/spawn", "minecraft:commands"),
            KnownDivergence {
                steps: "session/login".to_string(),
                packet: "minecraft:login_finished".to_string(),
                class: DivergenceClass::Body,
                closes_with: Some("M4-B01".to_string()),
                expires: Some("M5".to_string()),
            },
            KnownDivergence {
                steps: "session/*".to_string(),
                packet: "minecraft:keep_alive".to_string(),
                class: DivergenceClass::Timer,
                closes_with: None,
                expires: None,
            },
        ];
        let expired = expired_entries(&register, |milestone| milestone == "M4");
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].packet, "minecraft:commands");
    }
}
