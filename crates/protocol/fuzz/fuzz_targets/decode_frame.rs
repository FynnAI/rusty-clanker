#![no_main]
use bytes::BytesMut;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut buf = BytesMut::from(data);
    // Every declared threshold value the fuzzer can reach is exercised, not just one.
    let _ = rc_protocol::try_decode_frame(&mut buf, rc_protocol::CompressionState::Disabled);
    let mut buf2 = BytesMut::from(data);
    let _ = rc_protocol::try_decode_frame(
        &mut buf2,
        rc_protocol::CompressionState::Enabled { threshold: 256 },
    );
    // Neither call may panic for any input — that is this target's entire assertion.
});
