//! M4-B04 acceptance tests: MECH-D35's `MobCensusReport` wire type and the
//! `RegionMessage` size-bound regression guard.

use rc_messaging::{Address, Message, MobCensusReport, RegionId, RegionMessage};

#[test]
fn mob_census_report_roundtrips_through_serde() {
    let original = Message {
        from: RegionId(1),
        to: Address::Region(RegionId(2)),
        tick_stamp: 42,
        seq: 7,
        payload: RegionMessage::MobCensusReport(MobCensusReport {
            region: RegionId(1),
            counts: [70, 10, 15, 5, 5, 5, 20],
        }),
    };

    let bytes = postcard::to_allocvec(&original).unwrap();
    let decoded: Message<RegionMessage> = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, original);
}

/// Re-asserts M0-B02's own `size_of::<RegionMessage>() <= 128` with `MobCensusReport`
/// now a variant (regression guard, not a new assertion target).
#[test]
fn region_message_size_bound_still_holds() {
    assert!(std::mem::size_of::<RegionMessage>() <= 128);
}
