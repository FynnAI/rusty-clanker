//! PLAN-D12 NET-hardening changeset 1: `ClientboundCommandsPacket` -- vanilla's real
//! command dispatch graph for a fresh, non-operator player. Hand-implemented `RcPacket`
//! (the node-graph shape -- a flat entry list of self-referential-by-index nodes, each
//! carrying an optional type-dependent argument payload -- is not a shape
//! `#[derive(RcPacket)]` supports, mirroring `attribute_packets.rs`'s own established
//! precedent for this class of packet).
//!
//! The tree below is a byte-for-byte structural decode of the real oracle's own 265-
//! byte `commands` body observed at every join in the frozen protocol-diff capture
//! (`session/spawn`, `session/disconnect_reconnect`'s own reconnect, and every other
//! session step) -- not a guess from reading `Commands.java`'s registration call sites
//! (which register far more commands than a NO-PERMISSION player's own filtered tree
//! ever includes; `ClientboundCommandsPacket`'s own `NodeInspector` -- consulted by
//! `Commands.sendCommands`, ASSET-D18(f) reference -- omits every node this connection's
//! own permission level cannot run, which for an un-opped player is exactly this 26-node
//! set: `/me`, `/help`, `/list`, `/msg` (aliased `/tell`/`/w`), `/random`, `/teammsg`
//! (aliased `/tm`), `/trigger`). Argument-type ids are `rc_registries::generated_v776::registries::
//! command_argument_type`'s own already-generated, independently-cross-checked
//! constants (that module's own doc comment: id-for-id identical to this decode).
//! Argument payload shapes (`serializeToNetwork`) are decompiled-source-verified
//! per type: `brigadier:string` writes one more VarInt (`StringArgumentType.StringType`
//! ordinal), `entity` writes one more byte (`EntityArgument.Info`'s own `FLAG_SINGLE`/
//! `FLAG_PLAYERS_ONLY` bitset), `brigadier:integer` writes one more byte (`hasMin`/
//! `hasMax` flags, no further bytes when both are false) -- `message`/`objective`/
//! `int_range` carry no extra payload at all (`SingletonArgumentInfo.contextFree`, no
//! template fields).

use bytes::BufMut;
use rc_protocol::{Bytes, BytesMut, PacketDecodeError, RcPacket, VarInt};
use rc_registries::generated_v776::registries::command_argument_type as arg_type;

const TYPE_LITERAL: u8 = 1;
const TYPE_ARGUMENT: u8 = 2;
const FLAG_EXECUTABLE: u8 = 0x04;
const FLAG_REDIRECT: u8 = 0x08;
const FLAG_CUSTOM_SUGGESTIONS: u8 = 0x10;

enum Node {
    Root {
        children: &'static [i32],
    },
    Literal {
        name: &'static str,
        children: &'static [i32],
        executable: bool,
        redirect: Option<i32>,
    },
    /// `payload` is the already-serialized argument-type-specific tail (everything
    /// after `argument_type_id`) -- empty for a type with no template fields.
    Argument {
        name: &'static str,
        arg_type_id: i32,
        payload: &'static [u8],
        suggestions: Option<&'static str>,
        children: &'static [i32],
        executable: bool,
    },
}

/// `StringArgumentType.StringType.GREEDY_PHRASE`'s own ordinal (real oracle bytes,
/// this module's own doc comment).
const STRING_TYPE_GREEDY_PHRASE: &[u8] = &[0x02];
/// `EntityArgument.Info.FLAG_PLAYERS_ONLY`, `FLAG_SINGLE` unset -- `/msg`'s own real
/// multi-target-players-only shape.
const ENTITY_FLAGS_PLAYERS_ONLY: &[u8] = &[0x02];
/// `IntegerArgumentInfo`'s own flags byte with neither `hasMin` nor `hasMax` set --
/// no further bytes follow.
const INTEGER_FLAGS_UNBOUNDED: &[u8] = &[0x00];
const NO_PAYLOAD: &[u8] = &[];

const NODES: &[Node] = &[
    // 0: root
    Node::Root {
        children: &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    },
    // 1: me
    Node::Literal {
        name: "me",
        children: &[11],
        executable: false,
        redirect: None,
    },
    // 2: help
    Node::Literal {
        name: "help",
        children: &[12],
        executable: true,
        redirect: None,
    },
    // 3: list
    Node::Literal {
        name: "list",
        children: &[13],
        executable: true,
        redirect: None,
    },
    // 4: msg
    Node::Literal {
        name: "msg",
        children: &[14],
        executable: false,
        redirect: None,
    },
    // 5: tell (redirects to msg)
    Node::Literal {
        name: "tell",
        children: &[],
        executable: false,
        redirect: Some(4),
    },
    // 6: w (redirects to msg)
    Node::Literal {
        name: "w",
        children: &[],
        executable: false,
        redirect: Some(4),
    },
    // 7: random
    Node::Literal {
        name: "random",
        children: &[15, 16],
        executable: false,
        redirect: None,
    },
    // 8: teammsg
    Node::Literal {
        name: "teammsg",
        children: &[17],
        executable: false,
        redirect: None,
    },
    // 9: tm (redirects to teammsg)
    Node::Literal {
        name: "tm",
        children: &[],
        executable: false,
        redirect: Some(8),
    },
    // 10: trigger
    Node::Literal {
        name: "trigger",
        children: &[18],
        executable: false,
        redirect: None,
    },
    // 11: me action:message (executable)
    Node::Argument {
        name: "action",
        arg_type_id: arg_type::MESSAGE.0 as i32,
        payload: NO_PAYLOAD,
        suggestions: None,
        children: &[],
        executable: true,
    },
    // 12: help command:string(greedy) (executable)
    Node::Argument {
        name: "command",
        arg_type_id: arg_type::STRING.0 as i32,
        payload: STRING_TYPE_GREEDY_PHRASE,
        suggestions: None,
        children: &[],
        executable: true,
    },
    // 13: list uuids (executable)
    Node::Literal {
        name: "uuids",
        children: &[],
        executable: true,
        redirect: None,
    },
    // 14: msg targets:entity(players only)
    Node::Argument {
        name: "targets",
        arg_type_id: arg_type::ENTITY.0 as i32,
        payload: ENTITY_FLAGS_PLAYERS_ONLY,
        suggestions: None,
        children: &[19],
        executable: false,
    },
    // 15: random value
    Node::Literal {
        name: "value",
        children: &[20],
        executable: false,
        redirect: None,
    },
    // 16: random roll
    Node::Literal {
        name: "roll",
        children: &[21],
        executable: false,
        redirect: None,
    },
    // 17: teammsg message:message (executable)
    Node::Argument {
        name: "message",
        arg_type_id: arg_type::MESSAGE.0 as i32,
        payload: NO_PAYLOAD,
        suggestions: None,
        children: &[],
        executable: true,
    },
    // 18: trigger objective:objective (custom suggestions, executable)
    Node::Argument {
        name: "objective",
        arg_type_id: arg_type::OBJECTIVE.0 as i32,
        payload: NO_PAYLOAD,
        suggestions: Some("minecraft:ask_server"),
        children: &[22, 23],
        executable: true,
    },
    // 19: msg ... message:message (executable)
    Node::Argument {
        name: "message",
        arg_type_id: arg_type::MESSAGE.0 as i32,
        payload: NO_PAYLOAD,
        suggestions: None,
        children: &[],
        executable: true,
    },
    // 20: random value ... range:int_range (executable)
    Node::Argument {
        name: "range",
        arg_type_id: arg_type::INT_RANGE.0 as i32,
        payload: NO_PAYLOAD,
        suggestions: None,
        children: &[],
        executable: true,
    },
    // 21: random roll ... range:int_range (executable)
    Node::Argument {
        name: "range",
        arg_type_id: arg_type::INT_RANGE.0 as i32,
        payload: NO_PAYLOAD,
        suggestions: None,
        children: &[],
        executable: true,
    },
    // 22: trigger <objective> add
    Node::Literal {
        name: "add",
        children: &[24],
        executable: false,
        redirect: None,
    },
    // 23: trigger <objective> set
    Node::Literal {
        name: "set",
        children: &[25],
        executable: false,
        redirect: None,
    },
    // 24: trigger <objective> add value:integer (executable)
    Node::Argument {
        name: "value",
        arg_type_id: arg_type::INTEGER.0 as i32,
        payload: INTEGER_FLAGS_UNBOUNDED,
        suggestions: None,
        children: &[],
        executable: true,
    },
    // 25: trigger <objective> set value:integer (executable)
    Node::Argument {
        name: "value",
        arg_type_id: arg_type::INTEGER.0 as i32,
        payload: INTEGER_FLAGS_UNBOUNDED,
        suggestions: None,
        children: &[],
        executable: true,
    },
];

const ROOT_INDEX: i32 = 0;

fn write_prefixed_str(buf: &mut BytesMut, s: &str) {
    VarInt::new(s.len() as i32).encode(buf);
    buf.put_slice(s.as_bytes());
}

fn encode_node(node: &Node, buf: &mut BytesMut) {
    match node {
        Node::Root { children } => {
            buf.put_u8(0);
            VarInt::new(children.len() as i32).encode(buf);
            for c in *children {
                VarInt::new(*c).encode(buf);
            }
        }
        Node::Literal {
            name,
            children,
            executable,
            redirect,
        } => {
            let mut flags = TYPE_LITERAL;
            if *executable {
                flags |= FLAG_EXECUTABLE;
            }
            if redirect.is_some() {
                flags |= FLAG_REDIRECT;
            }
            buf.put_u8(flags);
            VarInt::new(children.len() as i32).encode(buf);
            for c in *children {
                VarInt::new(*c).encode(buf);
            }
            if let Some(r) = redirect {
                VarInt::new(*r).encode(buf);
            }
            write_prefixed_str(buf, name);
        }
        Node::Argument {
            name,
            arg_type_id,
            payload,
            suggestions,
            children,
            executable,
        } => {
            let mut flags = TYPE_ARGUMENT;
            if *executable {
                flags |= FLAG_EXECUTABLE;
            }
            if suggestions.is_some() {
                flags |= FLAG_CUSTOM_SUGGESTIONS;
            }
            buf.put_u8(flags);
            VarInt::new(children.len() as i32).encode(buf);
            for c in *children {
                VarInt::new(*c).encode(buf);
            }
            write_prefixed_str(buf, name);
            VarInt::new(*arg_type_id).encode(buf);
            buf.put_slice(payload);
            if let Some(s) = suggestions {
                write_prefixed_str(buf, s);
            }
        }
    }
}

pub struct Commands;

impl RcPacket for Commands {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x10;

    fn encode_body(&self, buf: &mut BytesMut) {
        VarInt::new(NODES.len() as i32).encode(buf);
        for node in NODES {
            encode_node(node, buf);
        }
        VarInt::new(ROOT_INDEX).encode(buf);
    }

    fn decode_body(_buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        Err(PacketDecodeError::UnexpectedEof)
    }
}
