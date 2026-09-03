//! M4-B07 — `LightProperties`/`LightPropertiesRegistry`/`shape_occludes` acceptance
//! tests (Context §3).

use rc_chunk_storage::BlockStateId;
use rc_mechanics::direction::Direction;
use rc_mechanics::light::{LightProperties, LightPropertiesRegistry, shape_occludes};

#[test]
fn unregistered_state_resolves_to_air() {
    let registry = LightPropertiesRegistry::new();
    assert_eq!(registry.resolve(BlockStateId(999)), LightProperties::AIR);
}

#[test]
fn register_range_and_resolve() {
    let mut registry = LightPropertiesRegistry::new();
    let props = LightProperties {
        opacity: 15,
        block_emission: 0,
        occludes_face: [true; 6],
    };
    registry.register_range(BlockStateId(10), BlockStateId(20), props);

    assert_eq!(registry.resolve(BlockStateId(15)), props);
    // Exclusive upper bound, no underflow into the neighboring id.
    assert_eq!(registry.resolve(BlockStateId(9)), LightProperties::AIR);
    assert_eq!(registry.resolve(BlockStateId(20)), LightProperties::AIR);
}

#[test]
#[should_panic]
fn register_range_panics_on_overlap() {
    let mut registry = LightPropertiesRegistry::new();
    registry.register_range(BlockStateId(10), BlockStateId(20), LightProperties::AIR);
    registry.register_range(BlockStateId(15), BlockStateId(25), LightProperties::AIR);
}

#[test]
fn shape_occludes_either_side_sufficient() {
    let full = LightProperties {
        occludes_face: [true; 6],
        ..LightProperties::AIR
    };
    let plain = LightProperties::AIR;

    assert!(shape_occludes(full, plain, Direction::West));
    assert!(shape_occludes(plain, full, Direction::West));
    assert!(!shape_occludes(plain, plain, Direction::West));
}

#[test]
fn get_opacity_floors_at_one() {
    let zero_opacity = LightProperties {
        opacity: 0,
        ..LightProperties::AIR
    };
    assert_eq!(zero_opacity.get_opacity(), 1);

    let five_opacity = LightProperties {
        opacity: 5,
        ..LightProperties::AIR
    };
    assert_eq!(five_opacity.get_opacity(), 5);
}
