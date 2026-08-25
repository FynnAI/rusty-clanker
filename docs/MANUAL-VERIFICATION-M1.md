# M1 Manual Verification — Online-Mode Session Validation (Acceptance Criterion 3)

This is the one M1 acceptance step this project's own binding rules forbid automating
(`09-testing-quality.md`'s zero-human-test-loop principle governs *routine*
verification; a real Microsoft/Mojang account's login flow is a genuine one-time
human action, not a routine check). Perform it once per M1 completion attempt,
immediately after a `full`-mode `m1-acceptance` CI run (`.github/workflows/ci.yml`)
is green.

## Procedure

1. Start a `rusty-clanker-server` build **without** `--offline` (online-mode is the
   documented default, NET-D6), bound to a reachable address.
2. Either:
   - **(a)** Launch the real, unmodified vanilla Java Edition 26.2 client via the
     official Minecraft launcher, using a genuine purchased Microsoft account, and
     connect to the server; or
   - **(b)** Run `cargo run -p rc-paritybot --example manual_online_check -- <host> <port> <email>`
     (a small, interactive-only example this blueprint does not wire into any
     automated test) — this calls `azalea::Account::microsoft(email).await`, which
     opens a real interactive Microsoft device-code OAuth flow in your terminal.
3. Confirm the connection succeeds (spawns into the world, no `unverified_username`/
   `authservers_down` disconnect) — this is the direct, positive proof that
   `rusty-clanker-server`'s NET-D6 `hasJoined` call against Mojang's real session
   server succeeded for a genuine account.
4. Record the date, the account used (username only, never credentials), and the
   engine build/commit hash tested, in the M1 completion record wherever this
   project tracks milestone sign-off.

Never automate this procedure. Never store or transmit account credentials as part of
any script this project ships.
