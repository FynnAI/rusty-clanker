//! `M1 field-report` acceptance tests: `NbtTextComponent`'s collapse rule
//! (`docs/findings-for-planning.md` section B, "Text components on the wire collapse to a
//! bare string"). Vanilla's own component codec (`ComponentSerialization`, the
//! `tryCollapseToString` step of its NBT-encode path, ASSET-D18(f) reference,
//! `net.minecraft.network.chat.ComponentSerialization`) writes a component carrying no
//! style, no siblings, and no translate/keybind/score/selector/nbt content as a bare,
//! unnamed `TAG_String` holding the text directly; only a richer component becomes the
//! `TAG_Compound` `{"text": "..."}` wrapper. `EntityDataSerializers.OPTIONAL_COMPONENT`
//! uses the same codec (TEST-D56: expectations below cite this rule, not any run of the
//! implementation under test).

use bytes::BytesMut;
use rc_protocol::{NbtTextComponent, PacketDecodeError, WireRead, WireWrite};

const NBT_TAG_STRING: u8 = 0x08;
const NBT_TAG_COMPOUND: u8 = 0x0A;
const NBT_TAG_END: u8 = 0x00;

/// Hand-builds the pre-collapse-fix `{"text": "<text>"}` compound bytes: unnamed root
/// `TAG_Compound(0x0A)` -> `TAG_String(0x08)` field named `text` -> the value -> `TAG_End`.
/// This is vanilla's own wire shape for a component `tryCollapseToString` returns `null`
/// for (any style/sibling content); `NbtTextComponent` itself can only ever hold plain
/// text, so this fixture is hand-built here rather than produced through `write_wire` —
/// it exercises exactly the read-side "still accept the compound form" half of the rule.
fn compound_text_bytes(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(NBT_TAG_COMPOUND);
    out.push(NBT_TAG_STRING);
    let key = b"text";
    out.extend_from_slice(&(key.len() as u16).to_be_bytes());
    out.extend_from_slice(key);
    let value = text.as_bytes();
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value);
    out.push(NBT_TAG_END);
    out
}

#[test]
fn plain_text_encodes_as_bare_tag_string() {
    let mut buf = BytesMut::new();
    NbtTextComponent("hi".to_string()).write_wire(&mut buf);
    // Collapsed layout (rule above): type byte 0x08 (TAG_String), then the
    // modified-UTF-8-length-prefixed text — no name field (network NBT, unnamed root) and
    // no surrounding TAG_Compound. Every real call site's text is plain ASCII, so a raw
    // UTF-8 byte count is exact here (`NbtTextComponent`'s own doc comment).
    assert_eq!(buf.as_ref(), &[NBT_TAG_STRING, 0x00, 0x02, b'h', b'i']);
}

#[test]
fn plain_text_empty_string_encodes_as_bare_tag_string() {
    let mut buf = BytesMut::new();
    NbtTextComponent(String::new()).write_wire(&mut buf);
    assert_eq!(buf.as_ref(), &[NBT_TAG_STRING, 0x00, 0x00]);
}

#[test]
fn plain_text_round_trips_through_the_collapsed_form() {
    let value = NbtTextComponent("Server is restarting".to_string());
    let mut buf = BytesMut::new();
    value.write_wire(&mut buf);
    let mut bytes = buf.freeze();
    let decoded = NbtTextComponent::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn plain_text_round_trips_with_non_ascii() {
    let value = NbtTextComponent("héllo wörld 日本語".to_string());
    let mut buf = BytesMut::new();
    value.write_wire(&mut buf);
    let mut bytes = buf.freeze();
    let decoded = NbtTextComponent::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn read_wire_still_accepts_the_legacy_compound_form_for_plain_text() {
    // A pre-fix payload (or any peer that still wraps plain text in a compound) must
    // still decode correctly -- "decoding accepts both forms" per the rule above.
    let bytes = compound_text_bytes("hi");
    let mut bytes = bytes::Bytes::from(bytes);
    let decoded = NbtTextComponent::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, NbtTextComponent("hi".to_string()));
}

#[test]
fn read_wire_accepts_a_richer_components_compound_form() {
    // Vanilla only ever reaches the TAG_Compound shape for a component `tryCollapseToString`
    // rejects -- any style, sibling, or translate/keybind/score/selector/nbt content.
    // `NbtTextComponent` cannot itself construct such a component (it has no
    // style/sibling representation), so this fixture hand-builds the compound shape vanilla
    // would send for one: the "text" field plus one extra TAG_String field standing in for
    // a style attribute (e.g. "color") that a richer component would carry alongside it.
    // `NbtTextComponent`'s own minimal reader (not a general NBT codec, per its doc comment)
    // still extracts exactly the "text" field, ignoring the rest.
    let mut out = Vec::new();
    out.push(NBT_TAG_COMPOUND);
    out.push(NBT_TAG_STRING);
    out.extend_from_slice(&4u16.to_be_bytes());
    out.extend_from_slice(b"text");
    out.extend_from_slice(&2u16.to_be_bytes());
    out.extend_from_slice(b"hi");
    out.push(NBT_TAG_STRING);
    out.extend_from_slice(&5u16.to_be_bytes());
    out.extend_from_slice(b"color");
    out.extend_from_slice(&3u16.to_be_bytes());
    out.extend_from_slice(b"red");
    out.push(NBT_TAG_END);

    let mut bytes = bytes::Bytes::from(out);
    let decoded = NbtTextComponent::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, NbtTextComponent("hi".to_string()));
}

#[test]
fn read_wire_rejects_a_root_tag_that_is_neither_string_nor_compound() {
    let mut bytes = bytes::Bytes::from(vec![0x03u8, 0x00, 0x00, 0x00, 0x01]); // TAG_Int root
    let err = NbtTextComponent::read_wire(&mut bytes).unwrap_err();
    assert!(matches!(
        err,
        PacketDecodeError::MalformedNbtTextComponent(_)
    ));
}
