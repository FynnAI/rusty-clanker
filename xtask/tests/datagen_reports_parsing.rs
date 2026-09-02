use xtask::datagen::reports::{
    BlockReport, BlockStateReport, OrderedProperties, OrderedValueList, RegistriesReport,
    find_default_state_id,
};

#[test]
fn parses_registries_report_minimal() {
    let json = r#"
    {
      "minecraft:block": {
        "default": "minecraft:air",
        "entries": {
          "minecraft:air": { "protocol_id": 0 },
          "minecraft:stone": { "protocol_id": 1 }
        }
      },
      "minecraft:item": {
        "entries": { "minecraft:air": { "protocol_id": 0 } }
      }
    }
    "#;
    let report: RegistriesReport = serde_json::from_str(json).unwrap();
    assert_eq!(
        report["minecraft:block"].entries["minecraft:air"].protocol_id,
        0
    );
    assert_eq!(
        report["minecraft:block"].entries["minecraft:stone"].protocol_id,
        1
    );
}

#[test]
fn parses_blocks_report_and_finds_default_state() {
    let json = r#"
    {
      "minecraft:air": {
        "definition": { "type": "minecraft:air" },
        "properties": {},
        "states": [ { "id": 0, "default": true } ]
      },
      "minecraft:oak_door": {
        "definition": { "type": "minecraft:door", "block_set_type": "oak" },
        "properties": {
          "facing": ["east", "north", "south", "west"],
          "half": ["lower", "upper"],
          "hinge": ["left", "right"],
          "open": ["false", "true"],
          "powered": ["false", "true"]
        },
        "states": [
          { "id": 5655, "properties": { "facing": "east", "half": "lower", "hinge": "left", "open": "false", "powered": "false" } },
          { "id": 5680, "properties": { "facing": "north" }, "default": true },
          { "id": 5718, "properties": { "facing": "west", "half": "upper", "hinge": "right", "open": "true", "powered": "true" } }
        ]
      }
    }
    "#;
    let report: xtask::datagen::reports::BlocksReport = serde_json::from_str(json).unwrap();
    assert_eq!(find_default_state_id(&report["minecraft:air"]), Some(0));
    assert_eq!(
        find_default_state_id(&report["minecraft:oak_door"]),
        Some(5680)
    );
}

#[test]
fn find_default_state_id_returns_none_when_unflagged() {
    let block = BlockReport {
        states: vec![
            BlockStateReport {
                id: 0,
                default: false,
                properties: OrderedProperties::default(),
            },
            BlockStateReport {
                id: 1,
                default: false,
                properties: OrderedProperties::default(),
            },
        ],
        properties: OrderedValueList::default(),
    };
    assert_eq!(find_default_state_id(&block), None);
}
