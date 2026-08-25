#![no_main]
use bytes::Bytes;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut b = Bytes::copy_from_slice(data);
    let _ = rc_protocol::VarInt::decode(&mut b);
    let mut b2 = Bytes::copy_from_slice(data);
    let _ = rc_protocol::VarLong::decode(&mut b2);
});
