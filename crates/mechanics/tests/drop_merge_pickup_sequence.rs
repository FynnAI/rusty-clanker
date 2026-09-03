//! M4-B02 acceptance tests: `pickup.rs`'s pure merge-eligibility helpers (Context §L) — no
//! `bevy_ecs::World` involved.

use rc_mechanics::entity::ItemStackRecord;
use rc_mechanics::entity::pickup::{MAX_STACK_SIZE, stacks_can_combine, stacks_mergeable};
use rc_registries::generated_v776::registries::item;

fn stack(
    item_id: rc_registries::generated_v776::registries::RegistryEntryId,
    count: u8,
) -> ItemStackRecord {
    ItemStackRecord {
        item_id,
        count,
        components: None,
    }
}

#[test]
fn identical_stacks_within_merge_radius_are_mergeable() {
    let a = stack(item::COBBLESTONE, 1);
    let b = stack(item::COBBLESTONE, 2);
    assert!(stacks_mergeable(&a, &b));
}

#[test]
fn different_item_ids_are_never_mergeable() {
    let a = stack(item::COBBLESTONE, 1);
    let b = stack(item::DIRT, 1);
    assert!(!stacks_mergeable(&a, &b));
}

#[test]
fn merge_respects_max_stack_size() {
    let a = stack(item::COBBLESTONE, 40);
    let b = stack(item::COBBLESTONE, 30);
    assert_eq!(a.count as u32 + b.count as u32, 70);
    assert!(70 > MAX_STACK_SIZE as u32);
    assert!(
        stacks_mergeable(&a, &b),
        "same item/components alone would say mergeable"
    );
    assert!(
        !stacks_can_combine(&a, &b),
        "combined count exceeds MAX_STACK_SIZE"
    );
}
