//! M3-B05 — `Direction::vanilla_ordinal` acceptance test (Context §E), pure.

use rc_mechanics::direction::Direction;

#[test]
fn vanilla_ordinal_matches_m2b07s_table() {
    assert_eq!(Direction::Down.vanilla_ordinal(), 0);
    assert_eq!(Direction::Up.vanilla_ordinal(), 1);
    assert_eq!(Direction::North.vanilla_ordinal(), 2);
    assert_eq!(Direction::South.vanilla_ordinal(), 3);
    assert_eq!(Direction::West.vanilla_ordinal(), 4);
    assert_eq!(Direction::East.vanilla_ordinal(), 5);
}
