//! MECH-D82/MECH-D83 (M3 field-report wave 3, Stream B3): one requested clientbound `sound`
//! packet, queued by a `BlockBehavior::on_use` handler (`UseUpdateContext::request_sound`) and
//! drained by the direct-action call site that dispatched it (`crates/server/src/play/world.rs`'s
//! own `BlockActionKind::Place` handling) -- never a per-tick Stage-4 background resource, since
//! every real producer (the comparator's own mode-cycle click) is itself a direct, synchronous
//! player-action response, not something Stage 4's own ordinary tick-driven systems ever emit.

use rc_core::BlockPos;
use rc_registries::generated_v776::registries::RegistryEntryId;

/// Vanilla's own `net.minecraft.sounds.SoundSource` enum ordinal (ASSET-D18(f) reference,
/// verified): `MASTER=0, MUSIC=1, RECORDS=2, WEATHER=3, BLOCKS=4, HOSTILE=5, NEUTRAL=6,
/// PLAYERS=7, AMBIENT=8, VOICE=9, UI=10`. Only `Blocks` has a real producer in this blueprint
/// (the comparator's own click); every other variant is named for completeness against the
/// full real enum, not dispatched anywhere yet.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SoundSource {
    Master,
    Music,
    Records,
    Weather,
    Blocks,
    Hostile,
    Neutral,
    Players,
    Ambient,
    Voice,
    Ui,
}

impl SoundSource {
    /// `FriendlyByteBuf.writeEnum`'s own wire encoding for this enum (a plain `VarInt` of the
    /// ordinal, ASSET-D18(f) reference verified) -- `packets.rs`'s own `Sound` packet encoder
    /// reads this directly.
    pub const fn vanilla_ordinal(self) -> u8 {
        match self {
            SoundSource::Master => 0,
            SoundSource::Music => 1,
            SoundSource::Records => 2,
            SoundSource::Weather => 3,
            SoundSource::Blocks => 4,
            SoundSource::Hostile => 5,
            SoundSource::Neutral => 6,
            SoundSource::Players => 7,
            SoundSource::Ambient => 8,
            SoundSource::Voice => 9,
            SoundSource::Ui => 10,
        }
    }
}

/// One requested clientbound `sound` packet (Context, MECH-D82/B3). `seed` is deliberately
/// NOT a field here -- vanilla draws it from the level's own random source at broadcast time
/// (`ServerLevel.playSeededSound`'s own caller), a concern the draining call site owns, not the
/// requesting behavior (`crates/server/src/play/world.rs`'s own doc comment on its draining
/// call site has the concrete seed-sourcing decision this blueprint shipped).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SoundRequest {
    pub pos: BlockPos,
    /// The wire `minecraft:sound_event` registry id (e.g. `rc_registries::generated_v776::
    /// registries::sound_event::BLOCK_COMPARATOR_CLICK`) -- never a raw string, matching the
    /// `Sound` packet's own registry-id-holder wire form (`packets.rs`'s own doc comment).
    pub sound: RegistryEntryId,
    pub source: SoundSource,
    pub volume: f32,
    pub pitch: f32,
    /// `true` iff the acting connection must NOT receive this sound (vanilla's own
    /// `level.playSound(player, ..)` "except" argument -- the comparator's own click excludes
    /// the clicking player, who already predicted the sound client-side).
    pub except_actor: bool,
}
