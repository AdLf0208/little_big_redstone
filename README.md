# Little Big Redstone for Horizon

A Horizon port of **Little Big Redstone 1.9.13-1.21.1**
(`Swedz/Little-Big-Redstone`, MIT) — slice 1: **one microchip with an AND gate**.

Built for Horizon's **mod API 0.2** (SDK `0b14f8ac`, mixins `e59da9094f0c`) against
Minecraft 26.3. Version 0.2.0 is the same slice republished as one flat SDK
package: 0.1.0 was a retired `.hnmod` container (submission PR #902).

## What it does

Upstream compacts redstone into a microchip block whose circuit board holds
logic components wired to I/O ports; each port is bound to one face of the block.
This slice implements the guide's own AND scene:

- `little_big_redstone:red_microchip` — the block (six face properties, a block
  entity holding the circuit, redstone in and out on its faces);
- the **AND gate** with 2..10 inputs (the jar's truth table: `0,0 → 0`,
  `0,1 → 0`, `1,0 → 0`, `1,1 → 1`; when every input is on the output is the
  greatest input);
- **I/O ports**: input ports compare the signal on their face with their
  strength (`<=`, `==`, `>=`) and pass it through; output ports emit their
  configured strength (or pass the signal when the strength is 0), weakly or
  strongly;
- the circuit is saved in the block entity's compound, so it survives a reload;
- `/lbr circuit <x> <y> <z> and <input_a> <input_b> <output>` builds the scene,
  `… status` reports it and its live signals, `… clear` empties it.

## Install

One file: `little_big_redstone.hmod`. It is the code **and** the content in one
package (`horizon-mod.toml`, `Cargo.toml`, `src/`, `assets/`, `data/`), so the
microchip block, its item, models, textures, loot, recipes and tags arrive with
it. Build it with `cargo horizon-mod build`; the game must be built by
`horizon-launcher native-build` against the same pinned Horizon build.

## What it does not do yet

The other 15 microchip colours, the other 20 logic components, the circuit-board
GUI (replaced by `/lbr`), the wire rendering, the redstone-face overlay, the
microchip's custom model loader and the logic-array/floppy-disk items. See
`notes.md`.

## Licence

MIT. The original is Copyright (c) 2024 Swedz; see `LICENSE` and
`LICENSE-NOTICE.md`, which travel inside the package.
