# FINDINGS — Little Big Redstone (`little_big_redstone`)

Written before any port code, from the jar's own inventory and data files and
from the mod's MIT source read for behaviour. Nothing here is a translation of
upstream code: the port is a clean-room Rust rewrite of what the mod does.

## The artefact

| | |
|---|---|
| Jar | `ports/_shared/licence-cache/little-big-redstone-1.9.13-1.21.1.jar` |
| Entries | **1,255** |
| `mods.toml` | `modId = "little_big_redstone"`, `version = "1.9.13-1.21.1"`, **`license = "MIT"`**, author Swedz, `displayURL = https://github.com/Swedz/Little-Big-Redstone` |
| Required dependency | `tesseract_api` `[1.12.16,1.13)` (a library; a rewrite does not call it) |
| Optional | `emi` |
| Source | `Swedz/Little-Big-Redstone`, clone at `f08bd39c1ce26f236fc1daded565209298c6d91a` (2026-10-22), `LICENSE` = MIT (Copyright (c) 2024 Swedz) |
| Blockstates | 17 files: the **16 coloured microchips** and nothing else — 16 blocks, one state each in the jar's own blockstate (the six face booleans are drawn by a custom model loader, not by variants) |
| Items | 87: 16 microchips, 16 logic arrays, 16 floppy disks, 18 sticky notes (16 colours + base + sealant), the 21 logic components, the circuit board, redstone bits, tags |
| Recipes | 88: 16 microchip colours, 17 logic arrays, 17 floppy disks, 17 sticky notes, 19 logic components, plus the circuit board, the redstone bit and the three special recipes |

### The shape the artefact actually has

The **microchip is the block**; the logic components (`and_gate`, `or_gate`,
`io`, …) are **items placed on the microchip's circuit board** in a GUI
(`guides/little_big_redstone/guide/logic/introduction.md`). The microchip's
redstone faces are the faces of its I/O ports, so "a logic gate whose inputs come
from the blocks it faces and whose output drives the block behind it" is a
microchip with an input port on two faces and an output port on a third.

The guide pages ship inside the jar as markdown
(`assets/little_big_redstone/guides/little_big_redstone/guide/…`), which is how
the numbers below were read without touching code.

## Behaviour read from the upstream source (analysis only)

| Rule | Where |
|---|---|
| The microchip block has six boolean properties, `up/down/north/south/east/west`, one per face | `MicrochipBlock.java:33-38`, `:47-54` |
| It is a signal source and answers `getSignal` from its redstone awareness; `getDirectSignal` only where the port is strong | `MicrochipBlock.java:126-152` |
| It does **not** conduct redstone: `isRedstoneConductor(Blocks::never)` | `MicrochipBlock.java:59-61` |
| Block properties: map colour STONE, `destroyTime(4f)`, `requiresCorrectToolForDrops`, `mineable/pickaxe`, `self` loot | `LBRBlocks.java:127-140` |
| The block entity ticks every server tick and saves a `microchip` compound | `MicrochipBlockEntity.java:196-236`, `:285-333` |
| A tick: `removed()`, `preTick()`, `tickLogic()`, then — only when dirty — `postTick()`, `setChanged()` and packets | `MicrochipBlockEntity.java:196-236` |
| Components tick in a traversal order built by a modified BFS over the wires | `Microchip.java:249-278`, `LogicTraversal.java:40-75` |
| A component input port takes the **first wire whose source output is above zero**, else 0 | `Microchip.java:249-278` |
| Wires carry a signal with no loss (an output port's value is read directly) | `Microchip.java:249-278`, `Wire.java` |
| **AND gate**: 0 if any input is 0, else the **greatest** input | `ANDGate.java:46-62` |
| The AND gate takes **2..10** inputs (clamped), default 2, one output | `MultiLogicGateConfig.java:34-70` |
| **I/O port, input mode**: output = `comparison.test(signal, strength) ? signal : 0` | `LogicIO.java:73-82` |
| **I/O port, output mode**: output = its wired input; the face emits `strength` when set, the passed signal when `strength == 0`, nothing when off | `LogicIO.java:83-98` |
| Port default: input, `north`, strength 1, `>=`, weak | `LogicIOConfig.java:40-46` |
| Comparisons are `<=`, `==`, `>=` | `LogicComparisonMode.java:12-16` |
| An input and an output port on the **same face** are both inert (`checkValid`) | `LogicIOConfig.java:88-100`, `guides/…/logic/io.md` |
| One port per face: `load()` registers the first and skips the rest | `RedstoneAwareness.java:104-127` |
| Output power per face is the **strongest** port on it; the six face booleans are `inputPower > 0 || outputPower > 0` | `RedstoneAwareness.java:186-232` |
| A neighbour change reads `getSignal(neighbour, direction)` for input faces only | `RedstoneAwareness.java:150-170` |
| A component's last output is saved in its own NBT (`output`) | `LogicGate.java:26-32`, `LogicIO.java:29-40` |
| The circuit is saved as `{size, sticky_notes, components, wires}` | `Microchip.java:42-58` |

The guide's own numbers, quoted by the jar:

- AND truth table (`logic/and_gate.md`): `0,0 → 0`, `0,1 → 0`, `1,0 → 0`, `1,1 → 1`.
- "AND Gates will yield an output signal equal to the greatest input signal."
- "AND gates can be configured to have anywhere between 2 and 10 inputs."
- I/O: "When the port is in input mode, the redstone signal inputted must meet
  or exceed the signal in order for the port to provide an output of ON. When
  the port is in output mode … the redstone signal provided will be equal to the
  signal strength set."
- "Each direction may only either act as an input or output, not both. … neither
  port will work and a warning indicator will be displayed."

## What this slice carries

- `little_big_redstone:red_microchip` — the block, its item, the three animated
  face textures, a `cube_bottom_top` model, the 26.3 loot table, the
  `mineable/pickaxe` and `little_big_redstone:microchips` tags, the lang keys.
- `little_big_redstone:redstone_circuit_board` — the microchip recipe's own
  ingredient, with its texture, model and recipe.
- The two recipes (`microchip/red.json`, `redstone_circuit_board.json`) and the
  `c:` tags they name (`c:dyes/red`, `c:dusts/redstone`, `c:ingots/iron`,
  `c:storage_blocks/iron`, `c:gems/quartz`, `c:gems/diamond`), which 26.3 does
  not ship.

## Not carried (gaps)

The other 15 microchip colours; the logic-array and floppy-disk items; the
sticky notes and the note-board entity; the other 20 logic components
(NOT/NAND/NOR/XOR/OR, latch, pulse throttler, sequencer, selector, randomizer,
comparator, calculator, reader, battery, tag, debugger); the circuit-board GUI,
the wires' visual routing, the redstone-face overlay rendering, the
microchip's custom model loader, the dye/copy/seal/clear-config recipes, the
wrench/rotation tool, and the guide book.
