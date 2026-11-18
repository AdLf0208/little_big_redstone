# Little Big Redstone — a Horizon port

**Little Big Redstone** is a Minecraft mod. Compact your big redstone into a little block!. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Swedz/Little-Big-Redstone](https://github.com/Swedz/Little-Big-Redstone) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Little Big Redstone ([source](https://github.com/Swedz/Little-Big-Redstone)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `little_big_redstone.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **An input port now asks the game.** `LevelRef::signal(host, neighbour, face)` is the engine's own
- **`block_signal` answers from the registry's power cache**, not from the block entity. The hook is
- **The face properties are written one by one** (`host.set_block_property`), not as one resolved
- **`clear` replaces the block entity's data** (`block_entity_replace_data(empty)`): `merge_data`
- **Commands are typed.** The dispatcher parses `/lbr …` once and hands `run_mod_command` a literal
- **`<pos:block_pos>` instead of three `<x:integer>`s**, so relative coordinates (`~ ~1 ~`) work.
- **The client half of `level_tick_post` does nothing**: the hook is asked on both sides (the SDK

## Not yet ported

- The other 15 microchip colours; the 20 other logic components (NOT, NAND, NOR,
- The circuit-board GUI, wire routing/rendering, the redstone-face overlay, the
- The logic-array and floppy-disk items (a saved program travels between
- A microchip that is not ticking (its chunk unloaded) keeps its last outputs;
- `setblock` with a state the content declares but the block never wrote leaves
- No sounds, particles, comparator output, hopper interaction or loot for the
- **No in-game run in this environment.** The e2e needs a game the launcher built with this package
- **Grammar-level refusals are the dispatcher's** now (behaviour 5), so the `flip` phase of the e2e

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `little_big_redstone.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Little Big Redstone was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Swedz/Little-Big-Redstone

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run little_big_redstone` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/little_big_redstone/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
