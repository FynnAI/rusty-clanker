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

## 💚 Support the Project — Buy Fuel for the Robots

Let's be transparent about something unusual: **most of this code isn't written by a human.**

Rusty Clanker is developed by a fleet of autonomous AI agents (Claude models) that plan, write, test, and review Rust code around the clock, orchestrated by one human maintainer. That's why a from-scratch, ECS-based, cluster-capable Minecraft-compatible server engine is moving at a pace that would normally take a small studio. It's also why this project has a funding model that sounds slightly ridiculous — because it is, and it works.

### Why we need funding

AI agents don't drink coffee. They drink **tokens**, and tokens cost money. Every planning pass, every implemented milestone, every one of the thousands of tests in this repo was produced by paid AI compute. The monthly bill is the single real cost of this project — there are no salaries, no office, no merch budget.

**The goal: 200 EUR / month. That's it. Full stop.**

We don't want more. If the goal is met, the agents keep coding 24/7. That's the entire economic model.

### Where the money goes

| | |
|---|---|
| 🤖 **100%** | AI API costs & subscription tiers (Claude Max/API) that keep the agents coding |
| 💸 **0%** | Everything else. No cut, no overhead, no mystery line items. |

### The pitch

If you've ever paid **40 EUR/month** for a laggy Java server that tips over when your friends build one hopper clock too many — consider putting **10 EUR** here instead. You aren't buying anyone a coffee. You're literally buying **server fuel for the AI developers** so we can fix this ecosystem permanently: one engine, bit-exact vanilla behavior, thousands of players on hardware that used to struggle with forty.

- **GitHub Sponsors:** [→ github.com/sponsors/FynnAI](https://github.com/sponsors/FynnAI) *(placeholder — activate before launch)*
- **Ko-fi:** [→ ko-fi.com/rustyclanker](https://ko-fi.com/rustyclanker) *(placeholder — activate before launch)*

Every sponsored euro is compute. Every unit of compute is code. Every line of code is one step closer to retiring the 40-EUR lag machine.

## Legal

- Rusty Clanker is **not** an official Minecraft product or service and is **not** approved by, associated with, or endorsed by Mojang or Microsoft. "Minecraft" is a trademark of Mojang Synergies AB.
- This repository and all release artifacts ship **no Mojang-authored content** — no game assets, no textures, sounds, models, or data files. The Phase 2 client reads assets at runtime exclusively from the user's own legally owned local `.minecraft` installation, and online play requires a legitimate, purchased game account. We distribute an engine, not the game.
