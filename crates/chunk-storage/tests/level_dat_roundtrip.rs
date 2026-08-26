//! M2-B06 Acceptance tests: `LevelDat`'s minimal schema round trip, unknown top-level
//! `Data` field preservation, and the GZip byte-shape contract `AnvilDiskBackend::
//! write_level_dat`/`read_level_dat` (M2-B03) expect.

use rc_chunk_storage::LevelDat;
use rc_nbt::{borrow, owned};

#[test]
fn fresh_default_round_trips_through_gzip_bytes() {
    let original =
        LevelDat::fresh_default("New World", 1_700_000_000_000, (0, -59, 0, 0.0), "26.2");

    let bytes = original.to_gzip_bytes().unwrap();
    let decoded = LevelDat::from_gzip_bytes(&bytes).unwrap();

    assert_eq!(decoded.data_version, 4903);
    assert_eq!(decoded.level_name, original.level_name);
    assert_eq!(decoded.time, original.time);
    assert_eq!(decoded.last_played, original.last_played);
    assert_eq!(decoded.spawn_x, original.spawn_x);
    assert_eq!(decoded.spawn_y, original.spawn_y);
    assert_eq!(decoded.spawn_z, original.spawn_z);
    assert_eq!(decoded.spawn_angle, original.spawn_angle);
    assert_eq!(decoded.version_name, original.version_name);
    assert_eq!(decoded.version_snapshot, original.version_snapshot);
    assert_eq!(decoded.version_series, original.version_series);
}

#[test]
fn unknown_top_level_data_fields_survive_round_trip() {
    let original =
        LevelDat::fresh_default("New World", 1_700_000_000_000, (0, -59, 0, 0.0), "26.2");
    let mut data = original.to_data_compound();

    let mut game_rules = owned::NbtCompound::new();
    game_rules.insert("doDaylightCycle", "true");
    data.insert("GameRules", owned::NbtTag::Compound(game_rules.clone()));

    let bytes = rc_nbt::write_owned(&owned::BaseNbt::new("", data));
    let nbt = rc_nbt::read_borrowed(&bytes).unwrap();
    let base = match nbt {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => panic!("expected Nbt::Some"),
    };
    let compound = base.as_compound();

    let decoded = LevelDat::from_data_compound(&compound).unwrap();
    let resaved = decoded.to_data_compound();

    assert_eq!(
        resaved.get("GameRules"),
        Some(&owned::NbtTag::Compound(game_rules))
    );
}

#[test]
fn gzip_bytes_are_actually_gzip_compressed() {
    let level = LevelDat::fresh_default("New World", 0, (0, -59, 0, 0.0), "26.2");
    let bytes = level.to_gzip_bytes().unwrap();

    assert!(bytes.len() >= 2);
    assert_eq!(&bytes[..2], &[0x1F, 0x8B]);
}
