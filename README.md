# Rusty Clanker

A from-scratch **Rust reimplementation of the Minecraft: Java Edition server** (Phase 1) and a native client (Phase 2), wire-compatible with the vanilla protocol (pinned target: Java Edition 26.2, protocol 776). Vanilla gameplay parity — bit-identical by default — on a fully multithreaded, cluster-capable architecture, with an isomorphic modding API and config-activated Bedrock cross-play.

## Status

**Blueprint derivation phase.** This repository currently contains no engine code — it contains the complete planning corpus (`docs/planning/`, 16 documents, ~300 binding decisions) and self-contained implementation blueprints (`blueprints/`, milestones M0–M9) from which implementation will proceed. See `docs/planning/00-overview.md` for the entry point and `blueprints/00-blueprint-spec.md` for the blueprint format.

## Architecture pillars

- No single-threaded tick loop: dynamic worker pools batch quiet world regions; hot regions scale out. ECS domain parallelization lets multiple threads work the same chunk simultaneously without lock contention — while redstone stays strictly sequential per region for bit-exact vanilla parity.
- Interface-first message passing behind a `Transport` trait: the same engine binary runs monolithic (single container, in-memory channels) or as a cluster (spatial partitioning, QUIC, proxy with seamless zero-disconnect sector handoff).
- Seed-identical world generation via an interpreter over vanilla's own worldgen data.
- Isomorphic mods: one compiled mod artifact carries shared/server/client parts.

## License

Rusty Clanker is licensed under the **GNU Affero General Public License, version 3.0 only** (see [LICENSE](LICENSE), SPDX: `AGPL-3.0-only`).

**Contributions:** all contributions require a Contributor License Agreement (CLA) granting the maintainer the right to relicense the contribution. This keeps the project's licensing options open long-term; your contribution always also remains available under the AGPL-3.0 terms it was published under.

## Legal

- Rusty Clanker is **not** an official Minecraft product or service and is **not** approved by, associated with, or endorsed by Mojang or Microsoft. "Minecraft" is a trademark of Mojang Synergies AB.
- This repository and all release artifacts ship **no Mojang-authored content** — no game assets, no textures, sounds, models, or data files. The Phase 2 client reads assets at runtime exclusively from the user's own legally owned local `.minecraft` installation, and online play requires a legitimate, purchased game account. We distribute an engine, not the game.
