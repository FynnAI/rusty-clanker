//! `rusty-clanker-server` — server composition-root binary and embeddable library target.
//! M1-B01 scaffold: the ARCH-D21 Tokio connection layer (`net`) exists and is independently
//! testable; the full `pub fn run_embedded(...)` composition root (binding this to a real
//! TCP listener, `rc-scheduler`'s tick loop, and a packet catalog) is a later blueprint's
//! scope, not implemented here.

pub mod net;
