//! `rc-test-harness` — dev/test-only (TEST-D1): subprocess orchestration for a
//! `rusty-clanker-server` under test (`process`), the raw-TCP Server-List-Ping status
//! probe (`probe`, also exposed as the `status_probe` binary), and the in-process
//! scripted "fake server" test double (`fake_server`) both this crate's own tests and
//! `rc-paritybot`'s tests drive a real protocol client against. World-state
//! hashing/diffing and the synchronous test-mode tick driver (TEST-D1's other named
//! responsibilities for this crate) are reserved, unimplemented, for the milestone
//! that first needs real comparable world content (M2+) — not part of this blueprint.

pub mod chunk_soak;
pub mod fake_server;
pub mod fixtures;
pub mod probe;
pub mod process;
pub mod save_cadence;
pub mod tick_cadence;
