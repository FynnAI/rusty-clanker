//! M2-B06 Acceptance tests: `PlayerSessionStore::save_all`'s periodic-sweep semantics
//! and `with_record_mut`'s safe no-op on a disconnected UUID.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::{PlayerDataStore, PlayerPersistenceError};
use rc_core::DimensionId;
use rusty_clanker_server::play::PlayerSessionStore;

#[derive(Clone, Default)]
struct FakeStore {
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
fn save_all_persists_every_currently_connected_player() {
    let fake = FakeStore::default();
    let store = PlayerSessionStore::new(Arc::new(fake.clone()));

    let uuid_a = uuid::Uuid::new_v4();
    let uuid_b = uuid::Uuid::new_v4();
    store
        .load_or_create(uuid_a, DimensionId::OVERWORLD, [0.0, 0.0, 0.0])
        .unwrap();
    store
        .load_or_create(uuid_b, DimensionId::OVERWORLD, [0.0, 0.0, 0.0])
        .unwrap();

    store.with_record_mut(uuid_a, |r| r.data.health = 11.0);
    store.with_record_mut(uuid_b, |r| r.data.health = 3.25);

    // Neither player disconnects — `save_all` must still persist both.
    store.save_all();

    let store2 = PlayerSessionStore::new(Arc::new(fake));
    store2
        .load_or_create(uuid_a, DimensionId::OVERWORLD, [0.0, 0.0, 0.0])
        .unwrap();
    store2
        .load_or_create(uuid_b, DimensionId::OVERWORLD, [0.0, 0.0, 0.0])
        .unwrap();

    let health_a = store2.with_record_mut(uuid_a, |r| r.data.health).unwrap();
    let health_b = store2.with_record_mut(uuid_b, |r| r.data.health).unwrap();
    assert_eq!(health_a, 11.0);
    assert_eq!(health_b, 3.25);
}

#[test]
fn with_record_mut_returns_none_for_a_disconnected_uuid() {
    let fake = FakeStore::default();
    let store = PlayerSessionStore::new(Arc::new(fake));

    let result = store.with_record_mut(uuid::Uuid::new_v4(), |r| r.data.health);
    assert!(result.is_none());
}
