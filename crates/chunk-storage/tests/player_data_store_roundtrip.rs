//! M2-B06 Acceptance tests: the `PlayerDataStore` trait, exercised against both a
//! test-local in-memory fake and the real filesystem implementation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::{
    FilesystemPlayerDataStore, LoadedPlayerRecord, PlayerDataStore, PlayerPersistenceError,
    load_player, save_player,
};
use rc_core::DimensionId;

/// Test-local in-memory fake, mirroring the `FakeBackend`/`MockTransport` convention
/// already established elsewhere in this workspace for test-local trait fakes.
/// `Arc`-wrapped specifically so `fake.clone()` shares the same underlying map (a
/// shallow clone) — used by `crates/server/tests/play_persistence_store.rs` to
/// simulate a server restart by dropping and rebuilding a session store over the same
/// underlying data.
#[derive(Clone, Default)]
pub struct FakeStore {
    entries: Arc<Mutex<HashMap<uuid::Uuid, Vec<u8>>>>,
}

impl PlayerDataStore for FakeStore {
    fn read_player_data(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<Option<Vec<u8>>, PlayerPersistenceError> {
        Ok(self.entries.lock().unwrap().get(&uuid).cloned())
    }

    fn write_player_data(
        &self,
        uuid: uuid::Uuid,
        payload: &[u8],
    ) -> Result<(), PlayerPersistenceError> {
        self.entries.lock().unwrap().insert(uuid, payload.to_vec());
        Ok(())
    }
}

#[test]
fn load_player_returns_none_for_an_unknown_uuid() {
    let fake = FakeStore::default();
    let result = load_player(&fake, uuid::Uuid::new_v4()).unwrap();
    assert!(result.is_none());
}

#[test]
fn save_then_load_round_trips_through_the_fake_store() {
    let fake = FakeStore::default();
    let uuid = uuid::Uuid::new_v4();
    let record = LoadedPlayerRecord::fresh_default(DimensionId::OVERWORLD, [1.0, 2.0, 3.0]);

    save_player(&fake, uuid, &record).unwrap();
    let loaded = load_player(&fake, uuid).unwrap().unwrap();

    assert_eq!(loaded.data, record.data);
}

fn unique_temp_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("rc-player-data-store-{}", uuid::Uuid::new_v4()))
}

#[test]
fn filesystem_store_creates_the_players_data_directory_and_round_trips() {
    let root = unique_temp_dir();
    let store = FilesystemPlayerDataStore::new(root.clone());
    let uuid = uuid::Uuid::new_v4();
    let record = LoadedPlayerRecord::fresh_default(DimensionId::THE_END, [-4.0, 80.0, 12.5]);

    save_player(&store, uuid, &record).unwrap();
    let loaded = load_player(&store, uuid).unwrap().unwrap();

    assert_eq!(loaded.data, record.data);

    let expected_path = root
        .join("players")
        .join("data")
        .join(format!("{uuid}.dat"));
    assert_eq!(store.player_data_path(uuid), expected_path);
    assert!(expected_path.is_file());
}
