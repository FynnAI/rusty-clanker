//! M2-B06 Acceptance tests: `PlayerSessionStore`'s own join/disconnect/restart
//! round-trip, fully real, with no `HardcodedWorld`, no TCP, and no dependency on
//! `play::world`/`play::connection`'s own (still-settling) shape.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::{
    FilesystemPlayerDataStore, InventorySlotEntry, ItemStackRecord, PlayerDataStore,
    PlayerPersistenceError,
};
use rc_core::DimensionId;
use rusty_clanker_server::play::PlayerSessionStore;

/// Test-local in-memory fake, mirroring `rc-chunk-storage`'s own
/// `player_data_store_roundtrip.rs::FakeStore`. `Arc`-wrapped so `fake.clone()`
/// shares the same underlying map — simulating "the same on-disk directory survives
/// a process restart" without any real filesystem I/O.
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

fn unique_temp_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("rc-play-persistence-store-{}", uuid::Uuid::new_v4()))
}

#[test]
fn join_disconnect_restart_round_trips_position_and_inventory() {
    let uuid = uuid::Uuid::new_v4();
    let fake = FakeStore::default();

    let store1 = PlayerSessionStore::new(Arc::new(fake.clone()));
    store1
        .load_or_create(uuid, DimensionId::OVERWORLD, [0.0, -59.0, 0.0])
        .unwrap();
    store1.with_record_mut(uuid, |r| {
        r.data.inventory.push(InventorySlotEntry {
            slot: 0,
            item: ItemStackRecord {
                id: "minecraft:diamond".into(),
                count: 5,
                components: None,
            },
        });
        r.data.health = 7.5;
        r.data.pos = [42.0, 70.0, -13.0];
    });
    store1.save_and_remove(uuid).unwrap();

    // Simulate a server restart: drop store1, build a fresh PlayerSessionStore over
    // the same underlying data.
    drop(store1);
    let store2 = PlayerSessionStore::new(Arc::new(fake));
    store2
        .load_or_create(uuid, DimensionId::OVERWORLD, [0.0, -59.0, 0.0])
        .unwrap();

    let data = store2.with_record_mut(uuid, |r| r.data.clone()).unwrap();
    assert_eq!(data.pos, [42.0, 70.0, -13.0]);
    assert_eq!(data.health, 7.5);
    assert_eq!(
        data.inventory,
        vec![InventorySlotEntry {
            slot: 0,
            item: ItemStackRecord {
                id: "minecraft:diamond".into(),
                count: 5,
                components: None,
            },
        }]
    );
}

#[test]
fn same_scenario_against_a_real_filesystem_store() {
    let uuid = uuid::Uuid::new_v4();
    let root = unique_temp_dir();
    let real_store = FilesystemPlayerDataStore::new(root);

    let store1 = PlayerSessionStore::new(Arc::new(real_store.clone()));
    store1
        .load_or_create(uuid, DimensionId::OVERWORLD, [0.0, -59.0, 0.0])
        .unwrap();
    store1.with_record_mut(uuid, |r| {
        r.data.inventory.push(InventorySlotEntry {
            slot: 0,
            item: ItemStackRecord {
                id: "minecraft:diamond".into(),
                count: 5,
                components: None,
            },
        });
        r.data.health = 7.5;
        r.data.pos = [42.0, 70.0, -13.0];
    });
    store1.save_and_remove(uuid).unwrap();

    drop(store1);
    let store2 = PlayerSessionStore::new(Arc::new(real_store));
    store2
        .load_or_create(uuid, DimensionId::OVERWORLD, [0.0, -59.0, 0.0])
        .unwrap();

    let data = store2.with_record_mut(uuid, |r| r.data.clone()).unwrap();
    assert_eq!(data.pos, [42.0, 70.0, -13.0]);
    assert_eq!(data.health, 7.5);
    assert_eq!(
        data.inventory,
        vec![InventorySlotEntry {
            slot: 0,
            item: ItemStackRecord {
                id: "minecraft:diamond".into(),
                count: 5,
                components: None,
            },
        }]
    );
}
