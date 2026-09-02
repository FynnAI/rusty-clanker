//! Tier-1 block entities (M3-B06): chest, furnace, hopper — each a `bevy_ecs::Component` with
//! a tick behavior, a comparator-signal function, and a hand-written NBT codec. `BlockEntityHeader`
//! is the common `pos`-carrying header attached to every one of them; `BlockEntityWorldAccess`
//! is the Stage-7 ECS-agnostic core boundary (Context: mirrors `BlockWorldAccess`'s own shape).

pub mod chest;
pub mod container_signal_source;
pub mod furnace;
pub mod hopper;
pub mod save_records;

use bevy_ecs::prelude::Component;
use rc_core::{BlockPos, ChunkKey};

use crate::container::TierOneContainer;

/// Attached to every block-entity `Entity` (M2-B01's `BlockEntityIndex` members) alongside its
/// one type-specific component (Context: "common header").
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq)]
pub struct BlockEntityHeader {
    pub pos: BlockPos,
}

/// Discriminates which typed component a `BlockEntityWorldAccess` position resolves to —
/// position-keyed, never exposing a raw `bevy_ecs::Entity` to the ECS-agnostic core
/// algorithms (mirrors `BlockWorldAccess`'s own design, Context).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockEntityKind {
    Chest,
    Furnace,
    Hopper,
}

/// The Stage-7 ECS-agnostic core boundary (Context: "`BlockEntityWorldAccess` — the Stage-7
/// ECS-agnostic core boundary"). A production adapter (`stage7::ecs`) implements it over real
/// `Query`s; acceptance tests use a trivial `HashMap`-backed test double.
pub trait BlockEntityWorldAccess {
    /// Every chunk currently loaded in this region, ascending `(x, z)` order (Context's own
    /// reproducible, non-vanilla-order-dependent choice).
    fn region_chunks(&self) -> Vec<ChunkKey>;
    /// Block entities in `chunk`, in `BlockEntityIndex`'s own stored (load) order — the one
    /// ordering guarantee that *is* vanilla-observable (Context).
    fn block_entities_in_chunk(&self, chunk: ChunkKey) -> Vec<(BlockPos, BlockEntityKind)>;
    fn container_at_mut(&mut self, pos: BlockPos) -> Option<&mut dyn TierOneContainer>;
    fn get_hopper_mut(&mut self, pos: BlockPos) -> Option<&mut hopper::HopperBlockEntity>;
    fn get_furnace_mut(&mut self, pos: BlockPos) -> Option<&mut furnace::FurnaceBlockEntity>;
    fn get_chest_mut(&mut self, pos: BlockPos) -> Option<&mut chest::ChestBlockEntity>;
    /// Injected redstone-power query (Context: "Redstone lock" — not implemented by this
    /// blueprint's own production adapter; test doubles supply a fixed answer).
    fn is_locked_by_redstone(&self, pos: BlockPos) -> bool;
    /// M3.5-B06 (Context §3.2): the raw block-state `ENABLED` bit at `pos` -- a hopper's own
    /// `HopperBlockEntity::tick` gates its transfer attempt on this, in addition to (and
    /// distinct from) `is_locked_by_redstone` above. Default `true` (matches vanilla's own
    /// "always enabled" default) so every pre-existing implementor (`stage7/ecs.rs`,
    /// `crates/testing/gametest/src/replay.rs`, every `crates/mechanics/tests/*.rs` test
    /// double) compiles and behaves unchanged; only the real production adapter
    /// (`stage7::ecs::EcsBlockEntityWorld`) overrides it with a real block-state read.
    fn hopper_enabled(&self, _pos: BlockPos) -> bool {
        true
    }
    /// Applies a furnace lit-state block swap if `resolver` is present and resolves one
    /// (Context: "Lit-state block swap"). A no-op if `resolver` is `None`.
    fn swap_furnace_lit_state(
        &mut self,
        pos: BlockPos,
        now_lit: bool,
        resolver: Option<&dyn furnace::FurnaceLitStateResolver>,
    );
}
