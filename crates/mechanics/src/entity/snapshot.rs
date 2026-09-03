//! `EntitySnapshot` — the real, versioned component-serialization scheme replacing
//! M0-B02's opaque-bytes placeholder (Context: "`EntitySnapshot`... versioned
//! component-serialization scheme"). `rc_messaging::EntitySnapshot`'s own
//! `component_data: Vec<u8>` field stays exactly the opaque placeholder it already is —
//! this module's own job is entirely on the producing/consuming side.

use thiserror::Error;

use crate::entity::kinds::EntityPayload;
use crate::entity::{BaseEntity, EntityKind, LivingEntity};

pub const ENTITY_SNAPSHOT_FORMAT_VERSION: u16 = 1;

/// One component's identity inside a snapshot (Context: "`EntitySnapshot` — the
/// real, versioned component-serialization scheme").
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ComponentKind {
    Base,
    Living,
    Item,
    Zombie,
    Villager,
    Cow,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ComponentBlob {
    pub kind: ComponentKind,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SnapshotPayload {
    pub format_version: u16,
    pub entity_kind: EntityKind,
    pub components: Vec<ComponentBlob>,
}

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error(
        "entity snapshot format version {found} is not supported (this build supports exactly {supported})"
    )]
    UnsupportedFormatVersion { found: u16, supported: u16 },
    #[error("postcard decode failed: {0}")]
    Decode(String),
}

/// Builds the exact `Vec<u8>` a caller hands to `rc_messaging::EntitySnapshot.component_data`
/// (via `Box::new`, per that type's own already-fixed shape — this function does not
/// itself touch `rc-messaging`). `base`/`living`/`payload` are the already-assembled
/// component values a future transfer system reads out of its own region's `World`.
pub fn serialize_entity_snapshot(
    kind: EntityKind,
    base: &BaseEntity,
    living: Option<&LivingEntity>,
    payload: &EntityPayload,
) -> Vec<u8> {
    let mut components = Vec::with_capacity(3);
    components.push(ComponentBlob {
        kind: ComponentKind::Base,
        bytes: postcard::to_allocvec(base).expect("BaseEntity is always postcard-serializable"),
    });
    if let Some(living) = living {
        components.push(ComponentBlob {
            kind: ComponentKind::Living,
            bytes: postcard::to_allocvec(living)
                .expect("LivingEntity is always postcard-serializable"),
        });
    }
    let (payload_kind, payload_bytes) = match payload {
        EntityPayload::Item(item) => (
            ComponentKind::Item,
            postcard::to_allocvec(item).expect("ItemBundle is always postcard-serializable"),
        ),
        EntityPayload::Zombie(zombie) => (
            ComponentKind::Zombie,
            postcard::to_allocvec(zombie).expect("ZombieBundle is always postcard-serializable"),
        ),
        EntityPayload::Villager(villager) => (
            ComponentKind::Villager,
            postcard::to_allocvec(villager)
                .expect("VillagerBundle is always postcard-serializable"),
        ),
        EntityPayload::Cow(cow) => (
            ComponentKind::Cow,
            postcard::to_allocvec(cow).expect("CowBundle is always postcard-serializable"),
        ),
    };
    components.push(ComponentBlob {
        kind: payload_kind,
        bytes: payload_bytes,
    });

    let snapshot = SnapshotPayload {
        format_version: ENTITY_SNAPSHOT_FORMAT_VERSION,
        entity_kind: kind,
        components,
    };
    postcard::to_allocvec(&snapshot).expect("SnapshotPayload is always postcard-serializable")
}

/// Inverse. `Ok` only for `format_version == ENTITY_SNAPSHOT_FORMAT_VERSION` — never a
/// silent best-effort decode of an unrecognized version (mirrors WORLD-D16's own
/// "exact match or reject" `DataVersion` policy).
pub fn deserialize_entity_snapshot(bytes: &[u8]) -> Result<SnapshotPayload, SnapshotError> {
    let payload: SnapshotPayload =
        postcard::from_bytes(bytes).map_err(|e| SnapshotError::Decode(e.to_string()))?;
    if payload.format_version != ENTITY_SNAPSHOT_FORMAT_VERSION {
        return Err(SnapshotError::UnsupportedFormatVersion {
            found: payload.format_version,
            supported: ENTITY_SNAPSHOT_FORMAT_VERSION,
        });
    }
    Ok(payload)
}
