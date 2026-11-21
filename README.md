# Little Big Redstone for Horizon

A Horizon port of **Little Big Redstone 1.9.13-1.21.1**
(`Swedz/Little-Big-Redstone`, MIT) — the microchip block, its I/O ports and
**all six logic gates**.

Built for Horizon's **mod API 0.2** (SDK `0a875da9`, mixins `e59da9094f0c`)
against Minecraft 26.3. Version 0.2.0 was the same slice republished as one flat
SDK package (0.1.0 was a retired `.hnmod` container, submission PR #902);
0.2.2 adds the other five gates.

## What it does

Upstream compacts redstone into a microchip block whose circuit board holds
logic components wired to I/O ports; each port is bound to one face of the block.
This slice implements the guide's own scenes:

- `little_big_redstone:red_microchip` — the block (six face properties, a block
  entity holding the circuit, redstone in and out on its faces);
- the **six gates**, each on the jar's own truth table and source rule:
  - **AND** / **NAND** — 2..10 inputs; AND passes the greatest input when every
    input is on, NAND emits `1` unless every input is on;
  - **OR** — 2..10 inputs, analog: the greatest input;
  - **XOR** — 2..10 inputs, analog parity: the greatest input when an odd number
    are on;
  - **NOR** — 2..10 inputs, boolean: `1` only when every input is off;
  - **NOT** — one input, boolean: `1` when the input is off.
- **I/O ports**: input ports compare the signal on their face with their
  strength (`<=`, `==`, `>=`) and pass it through; output ports emit their
  configured strength (or pass the signal when the strength is 0), weakly or
  strongly;
- the circuit is saved in the block entity's compound, so it survives a reload;
- `/lbr circuit <x> <y> <z> <gate> <input…> <output>` builds a gate
  (`and`, `or`, `xor`, `nand`, `nor`, `not`), `… output <face> <strength>
  <weak|strong>` retargets the output port, `… status` reports the circuit and
  its live signals, `… clear` empties it.

## Install

One file: `little_big_redstone.hmod`. It is the code **and** the content in one
package (`horizon-mod.toml`, `Cargo.toml`, `src/`, `assets/`, `data/`, the
generated `uses.json`), so the microchip block, its item, models, textures,
loot, recipes and tags arrive with it. Build it with `cargo horizon-mod build`;
the game must be built by `horizon-launcher native-build` against the same
pinned Horizon build (the launcher reads `uses.json` from the port folder).

## What it does not do yet

The other 15 microchip colours, the other 14 logic components (RS-NOR latch,
T flip-flop, pulse throttler, sequencer, selector, randomizer, comparator,
calculator, reader, battery, tag, debugger, …), the circuit-board GUI (replaced
by `/lbr`), the wire rendering, the redstone-face overlay, the microchip's
custom model loader and the logic-array/floppy-disk items. See `notes.md`.

## Licence

MIT. The original is Copyright (c) 2024 Swedz; see `LICENSE` and
`LICENSE-NOTICE.md`, which travel inside the package.
