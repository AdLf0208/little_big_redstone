//! Little Big Redstone for Horizon — the red microchip and its circuit.
//!
//! Ported from Little Big Redstone 1.9.13-1.21.1 (`Swedz/Little-Big-Redstone`, MIT). [`runtime`] is
//! the pure truth — faces, ports, components, wires, the AND gate, the command grammar — and is
//! byte-identical to the retired `.hnmod` port's file. [`support`] is the glue: the block entity,
//! the per-face redstone answers, the level-tick evaluation and `/lbr`. This file is the one
//! `impl Hooks`.
//!
//! # Where the retired container's hooks went
//!
//! The 2026-11-17 mod-API merge (#16941, launcher ≥ 0.2.70) retired `.hnmod`: the plan, the
//! injection points and the per-hook helper files are gone. A hook is a typed method here, and a
//! moment that belongs to one block is a **Part** attached to it.
//!
//! | `.hnmod` hook (API) | here |
//! |---|---|
//! | the `content_root` injection (AE2 pattern) | gone: one flat `.hmod` carries `assets/` and `data/` itself |
//! | `block_entity` (15) | the block entity **type** declared in [`Mod::register`] plus `BlockEntityPart::block_entity` on `support::Microchip` |
//! | `block_entity_ticks` (15) | `BlockEntityPart::block_entity_ticks` |
//! | `block_entity_tick` (15) | `BlockEntityPart::block_entity_tick` |
//! | `level_tick_post` (23) | [`Mod::level_tick_post`] |
//! | `block_signal` (38) | `BlockPart::block_signal` |
//! | `is_signal_source` (38) | `BlockPart::is_signal_source` |
//! | `register_commands` (8) | [`Mod::register_commands`] |
//! | `perform_command` (8) | [`Mod::run_mod_command`] (typed arguments, not a raw line) |
//! | `world_lifecycle` (v1) | [`Mod::world_lifecycle`] |
//! | the `[[module]]` pair (`runtime`, `support`) | `pub mod runtime;` and `pub mod support;` — ordinary modules |
//!
//! The rules themselves are all in [`runtime`]; this file and `support` only read the game, call the
//! runtime and write the result back.

pub mod runtime;
pub mod support;

use mod_api::{
    self, Flow, Hooks, HostSide, LevelTickPostEvent, RegisterCommandsEvent, Registrar,
    RegistrationContext, RunModCommandEvent, ServerHost, WorldLifecycleEvent,
};

/// The mod keeps no state of its own: the registry lives in `support` (one per session, shared by
/// every microchip in every dimension).
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the microchip's block entity type and attach the Part that answers for it.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        support::declare(r);
    }

    /// `/lbr help|circuit …` (upstream edits a circuit in the microchip's GUI).
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        support::declare_commands(event.commands);
    }

    /// Run one of the commands above and take it.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        support::run_command(host, event)
    }

    /// End of a level's tick: evaluate every microchip whose block entity ticked, write its saved
    /// compound, its face properties and the neighbour notifications.
    fn level_tick_post(&self, host: HostSide<'_>, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        support::level_tick_post(host, event.level);
    }

    /// A world opened or closed: the registered microchips belong to the world that is going away.
    fn world_lifecycle(&self, _host: &mut dyn ServerHost, event: &mut WorldLifecycleEvent<'_>) {
        support::world_lifecycle(event);
    }
}
