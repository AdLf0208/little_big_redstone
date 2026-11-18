# Little Big Redstone — port notes

Ported from **Little Big Redstone 1.9.13-1.21.1**
(`Swedz/Little-Big-Redstone`, **MIT**, read from the jar's
`META-INF/neoforge.mods.toml`) for Minecraft Java 1.21.1 NeoForge. Upstream
behaviour was read from the MIT source at commit
`f08bd39c1ce26f236fc1daded565209298c6d91a` (2026-10-22) and from the guide pages
the jar ships; this is a clean-room Rust rewrite, not a translation. What was
read and every number extracted: [`FINDINGS.md`](FINDINGS.md).

**Class + tier:** `content` · **deep** (`ports/ledger.csv`, mirrored from
`ports/_shared/source-map/atm10-sources.csv`). Approach per the pin's
`docs/modpacks.md` ("Tiers (the API a port needs)", "Code: one agent per mod"):
a **deep** shape — a block with a block entity whose state is a small circuit
that reads redstone on its faces and drives them — not a data carry and not a
plain logic block. **Licence rank 0** (MIT, permissive): ported now because it is
a rank-0 core content mod and no unported dependency stands in the way
(`tesseract_api` is a library the rewrite does not call). The campaign keeps
every port local-only until the maintainer approves publishing
(`ARCHITECTURE.md` §8).

## Slice 1 — one microchip, one AND gate

The smallest complete slice that is honest for the tier. The artefact decides the
shape: the microchip is the **block**, and a logic component is an **item placed
on its circuit board** (`guides/…/logic/introduction.md`), so the block's
redstone faces are its I/O ports' faces.

| in | what |
|---|---|
| `little_big_redstone:red_microchip` | the block: six face properties, a block entity holding the circuit, redstone in and out |
| the AND gate | 2..10 inputs, `0,0→0 / 0,1→0 / 1,0→0 / 1,1→1`, and the greatest input when all are on |
| the I/O port | input: compare with `<=`/`==`/`>=` and pass the signal; output: emit the configured strength (or pass it at 0), weak or strong |
| `/lbr circuit <x> <y> <z> and <a> <b> <out>` | the configuration surface (upstream edits the board in a GUI) |
| `/lbr circuit <x> <y> <z> status` / `clear` | report the circuit and its live signals / empty it |
| `little_big_redstone:redstone_circuit_board` | the microchip recipe's own ingredient, so the recipe can match |
| the two shaped recipes + the `c:` tags they name | the jar's own data, with 26.3's missing common tags declared |

**Why this and not more:** the tier's *deep* feature is the circuit layer —
per-component state and redstone propagation — and one gate with real ports,
real wires, a saved circuit and a configuration surface exercises all of it. The
other 15 colours, the other 20 components and the board GUI are content and UI
volume, not new behaviour (gaps below).

## How it works

- **State.** The circuit lives in the block entity's `microchip` compound
  (`components`, `wires`, the per-face `power` cache, `strong`), so it survives a
  save and a reload. Component field names follow upstream's codec where the
  slice has the concept; the component `type` is `io`/`and_gate` and the format
  carries `version: 1`.
- **Tick.** The block entity's tick registers the position (a write inside it is
  reverted at this pin); `level_tick_post` reads the circuit, reads each face's
  incoming signal, evaluates the components in dependency order, writes the
  compound when a component output changed, writes the six face properties when
  they changed, and notifies the neighbours when a face's power changed
  (`MicrochipBlockEntity.tick` / `RedstoneAwareness.postTick`).
- **Redstone.** `block_signal` answers the saved per-face power (weak, or the
  same when the port is strong and a direct signal is asked for);
  `is_signal_source` is true so redstone wire connects. The input read rebuilds
  `getSignal(neighbour, face)` from the neighbour's public state — our own
  microchips and comparators by state, a redstone wire by its `power`, everything
  else from `StateFacts.weak` (see the deviation below).
- **Configuration.** `/lbr` is the port's own surface; it is registered for every
  player and answers through `native_feedback`. Every wrong combination is
  refused with a message naming the token (duplicate face, unknown face, bad
  coordinate, unknown action, no microchip at the position).

## Hooks

| Hook | Helper | What it does |
|---|---|---|
| `block_entity` (v1, API 15) | `block_entity.rs` | Gives `little_big_redstone:red_microchip` a block entity. |
| `block_entity_ticks` (v1, API 15) | `ticks.rs` | Says the microchip ticks on the server. |
| `block_entity_tick` (v1, API 15) | `tick.rs` | Registers the position for this tick's evaluation (no writes). |
| `level_tick_post` (v1, API 23) | `tick_post.rs` | Evaluates every registered microchip, writes the compound and the faces, notifies neighbours. |
| `block_signal` (v1, API 38) | `signal.rs` | Answers the stored per-face power, weak or direct. |
| `is_signal_source` (v1, API 38) | `source.rs` | A microchip is a signal source. |
| `register_commands` (v1, API 8) | `register.rs` | Declares the `/lbr` usages. |
| `perform_command` (v1, API 8) | `perform.rs` | Parses and runs a `/lbr` line from player chat. |
| `world_lifecycle` (v1, API 44) | `world.rs` | Drops the registered positions when a world opens or closes. |
| `content_root` (target, AE2 pattern) | `embedded_content.rs` | Writes the embedded `.hmod` to `.horizon-cache/` and appends it to the dev list. |

## Verification (0.1.0, retired `.hnmod` format — historical)

The numbers below are the retired container's, kept as the record of the slice's
behaviour. The 0.2.0 SDK ladder is in
[`## Migration to the SDK format`](#migration-to-the-sdk-format-020-2026-11-18).

Horizon `1d63e0a` + horizon-mixins `5a64af93`, every step through
`ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`. Full table, artifact
hashes and the e2e transcript in [`VALIDATION.md`](VALIDATION.md).

| Step | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings |
| `cargo test --locked --offline` | **19 passed** (the pure runtime) |
| `horizon.py stage` / `check` / `test` / `build` | exit 0; `cargo_exit 0`; **0 warnings**; embedded **25 passed** (19 runtime + 6 glue) |
| `tools/package.sh` | `.hmod` **15,753 B** `0e34bed5…cc2f` (29), `.hnmod` **37,464 B** `2a396c11…841d` (17) |
| `horizon-mod check` (both files) | `.hmod`: "no code: content only"; `.hnmod`: exit 0, `native-1/1d63e0a…/5a64af93…` |
| `tools/e2e.sh` (one lease, three legs) | default `PASS`+`EXIT:0`; embedded `PASS`+`EXIT:0` with the unusable-cache control; `SELFTEST=1` `FAIL`+`EXIT:1` |
| `check-e2e-logs.py --strict` | exit 0, 3/3 ok |
| `upstream_ready.py` | exit 1 on `status is pending` only; `--include-blocked` → READY |

The default leg is fail-capable three ways: the selftest leg asserts the
unpowered lamp lit (and fails), the stone counterfactual would stay lit if the
output were not the microchip's, and `1,1 → lit` would fail if the gate were an
OR (the `0,1` / `1,0` phases are the controls for it).

## Deviations (documented, not defects)

1. **The circuit-board GUI is a command.** Upstream places components and draws
   wires in the microchip's menu; the pin's menus are item containers, and the
   slice's job is the behaviour, so `/lbr circuit … and` builds the guide's own
   scene and `… status` reports it. What a user notices: the circuit is set up
   with a command instead of by dragging components.
2. **Input reads are rebuilt, not asked.** The pin has no per-direction
   redstone read for a mod (`native_neighbor_signal` is the strongest of all six
   faces), so `signal_toward` reconstructs `getSignal(neighbour, face)` from the
   neighbour's public state. Another mod's `block_signal` answer is therefore
   invisible to an input port, and indirect (strong) power conducted through a
   solid cube is not counted (the same deviation `morered` records).
3. **Wire precedence is deterministic.** Upstream takes "the first wire whose
   source is above zero" from a hash-backed set (`Microchip.java` `tickLogic`);
   the port sorts wires by output slot and port, so a circuit with two wires into
   one port always resolves the same way.
4. **Cycle order.** Upstream orders components with a modified BFS
   (`LogicTraversal.java`); the port uses a Kahn topological order with slot
   tie-breaks and appends cycle members in slot order. Identical for the acyclic
   circuits the slice builds; a feedback loop can evaluate in a different order.
5. **The block model is vanilla.** Upstream's `models/block/red_microchip.json`
   names a custom loader (`little_big_redstone:microchip`) that draws the
   redstone faces and the board; the pin's content format cannot bake a custom
   loader, so the port uses `minecraft:block/cube_bottom_top` with upstream's own
   three animated textures. The signal-face overlay is not drawn.
6. **The template is glass.** Upstream's microchip does not conduct redstone
   (`isRedstoneConductor(Blocks::never)`); `minecraft:glass` is the pin's
   full-cube non-conducting template, so a source beside the microchip does not
   conduct through it to a lamp behind. Glass does not occlude, so neighbours'
   hidden faces are not culled (visual only).
7. **One extra cache field.** The saved compound carries a `power` array and a
   `strong` array: upstream keeps the emitted power in its in-memory redstone
   awareness and gates `getSignal` on a face property, while the port's
   `block_signal` is a hook that cannot evaluate, so the last emitted power is
   cached beside the circuit. Behaviourally the same (a face emits only while it
   is live).
8. **A wire's strong power is directionless at this pin.** `vanilla_direct_signal`
   answers a redstone wire's own signal for every direction
   (`horizon-world/src/level/redstone.rs`), while vanilla's
   `RedstoneWireBlock.getDirectSignal` answers only for `DOWN`. An input port
   therefore sees more strong power than vanilla would give when a redstone wire
   touches it (the weak answer, which is what an input port reads, is unaffected).
   The e2e's stone counterfactual keeps the wire away from the stone for this
   reason.
9. **The content model has no per-state variants.** The jar's blockstate maps
   every state to one model (its custom loader draws the faces); the port
   declares all **64** states against one `cube_bottom_top` model.

## Upstream defects fixed

None claimed. Nothing in the artefact was internally inconsistent in the slice's
area: the guide's truth table, the gate's `processInputs`, the I/O port's
comparison, the port defaults and the recipe chain all agree with their data
files. The nondeterministic wire precedence (deviation 3) is recorded as a
deviation rather than a defect because it is not provably wrong on any input the
slice builds, and no upstream report is warranted for it.

## Harness findings (measured 2026-11-12, this pin)

1. **`execute in <dimension> run fill` before the dimension's chunk is loaded
   answers `ok` and changes nothing.** The rig's floor was silently missing, and
   a redstone wire placed on it was removed a tick later — the failure looked
   like "the wire did not place". `build_rig` now teleports into the End, waits
   20 ticks, fills, and *reads the floor back* before going on.
2. **A redstone wire strongly powers a conductor beside it at this pin.**
   `vanilla_direct_signal` returns the wire's signal whatever the direction
   (`horizon-world/src/level/redstone.rs`), while vanilla's
   `RedstoneWireBlock.getDirectSignal` answers only for `DOWN`. So a stone block
   next to a wire is powered *through* the wire: the counterfactual has to remove
   the analog line before replacing the microchip with stone, or the control
   proves nothing about the microchip. (This also means a port that reads
   `StateFacts.strong` sees more strong power than vanilla would give.)
3. **The face properties and the lamp follow the evaluation by one tick.** The
   assertions retry a few windows before failing; the first version read the
   lamp in the same tick the wire's power settled and saw it dark.
4. **A live circuit rewrites the face properties every tick**, so the 64-state
   content check runs with the circuit cleared and the inputs removed; otherwise
   the microchip's own evaluation overwrites the state `setblock` just wrote.

## Gaps

- The other 15 microchip colours; the 20 other logic components (NOT, NAND, NOR,
  XOR, OR, latch, pulse throttler, sequencer, selector, randomizer, comparator,
  calculator, reader, battery, tag, debugger); the AND gate's 3..10-input
  configurations are modelled and tested but the command builds the 2-input one.
- The circuit-board GUI, wire routing/rendering, the redstone-face overlay, the
  microchip's custom model loader, the six-way rotation tool.
- The logic-array and floppy-disk items (a saved program travels between
  microchips), the sticky notes and the note-board entity, the dye/copy/seal/
  clear-config recipes, the guide book, the EMI/JEI pages, the redstone-bit item.
- A microchip that is not ticking (its chunk unloaded) keeps its last outputs;
  the face properties and the saved power are only refreshed when it ticks.
- `setblock` with a state the content declares but the block never wrote leaves
  the face properties as given until the first evaluation (one tick).
- No sounds, particles, comparator output, hopper interaction or loot for the
  circuit's components (upstream drops them from the block entity).

## Migration to the SDK format (0.2.0, 2026-11-18)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70; PR #17992 cleans up the old files) retired the
`.hnmod` container: the registry closed every `.hnmod` submission on 2026-11-18 with one reason, and
an `.hnmod` no longer loads. This port's submission, **PR #902** (`little_big_redstone` 0.1.0), was
closed by that purge. **0.2.0** is the same slice republished as one flat SDK `.hmod`; the changelog
text is at the end of this section.

Pins: SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (game 26.3, `package_format` 6,
`content_api` "0.2"), mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, `cargo-horizon-mod`
from `horizon-rust/horizon@main`. The pinned SDK is cached in-tree at `../_shared/sdk/0b14f8a/` and
was copied into `.horizon/sdk/` (gitignored) so every build here ran offline.

### Where the retired tree is

Moved — never copied into the package root — **before** the first `cargo horizon-mod build`, to
`../_shared/evidence/little_big_redstone/retired-hnmod-0.1.0/`:

| 0.1.0 path | Why it had to go |
|---|---|
| `mixins/little_big_redstone/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, 10 helper `.rs`) | the plan/injection container is retired; `runtime.rs` moved to `src/`, the glue to `src/support.rs` + `src/lib.rs` |
| `little_big_redstone.hnmod` (37,464 B, 17 entries) | unloadable on launcher ≥ 0.2.70; the flat `.hmod` replaces it |
| `content/` (`horizon-mod.toml`, `little_big_redstone.hmod` 15,753 B, `target/`) | its `assets/` and `data/` are the package root now; its manifest merged into the root `horizon-mod.toml` |
| `compatibility.json` | `[native]` in the manifest is the pin |
| `rust-toolchain.toml` | the pinned SDK decides the toolchain (`sdk.json`: rust 1.99.1) |
| `tools/horizon.py`, `tools/package.py`, `tools/package.sh` | staging and `.hnmod` packaging belong to the retired format |
| `Cargo.lock` | deleted, never packed (it changes the artifact and resolves newer versions than the SDK's own lock) |

**Kept:** `tools/e2e.sh` (updated: `GAME=` from the launcher, optional `CONTROL_GAME=`, no
`--dev-mod`), `tools/craft_check.py`, `notes.md`, `FINDINGS.md`, `VALIDATION.md`, `LICENSE`,
`LICENSE-NOTICE.md`, `README.md` (rewritten for the flat package).

### File map (0.2.0)

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and identity: `id = "little_big_redstone"`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` pinned to `0b14f8a`/`e59da90` |
| `Cargo.toml` | one crate, `edition = "2024"`; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks`: `register`, `register_commands`, `run_mod_command`, `level_tick_post`, `world_lifecycle` — each one line into `support` |
| `src/support.rs` | all the game glue: the Part (block + block-entity methods), the tick registry and power cache, the evaluation, the command surface, the circuit codec. 11 unit tests (codec, directions, typed-argument table) |
| `src/runtime.rs` | the pure truth (faces, ports, components, wires, the AND gate, the command grammar), **byte-identical**, 19 tests |
| `assets/`, `data/` | the carried content, moved from `content/` — 28 files, **every one byte-identical** to the retired copy (size + sha256 compared) |
| `LICENSE`, `LICENSE-NOTICE.md` | MIT text and provenance, **inside** the package |
| `tools/e2e.sh`, `tools/craft_check.py` | the in-game scenario and the crafting check |
| `little_big_redstone.hmod` | the deliverable (48,725 B, sha256 `16b6ff8a…f4de`, 37 entries) |

Manifest mapping: `[mod] id`/`version` → `id`/`version` (0.1.0 → **0.2.0**, a resubmission is a new
version); `[mod] priority = 100` → gone (ordering is `#[listener(priority = …)]`); `[compatibility]
horizon_revision` → `[native] horizon`; the bundle's mixins revision → `[native] mixins` (**moved**:
`5a64af93…` → `e59da909…`); the game version → `[native] game = "26.3"`; `[[module]] runtime` →
`pub mod runtime;`; `[[module]] state` → `pub mod support;`. `license`, `homepage` and `issues` are
not manifest fields (`build` strips them; the licence is in `Cargo.toml` and in the package).

### Helper → hook table (every injection has a home)

| 0.1.0 injection (hook, API) | helper | 0.2.0 |
|---|---|---|
| `embedded-content` (`content_root`, AE2 HEAD, cancellable) | `embedded_content.rs` | **gone**: the flat `.hmod` carries `assets/` + `data/` itself |
| `block-entity` (15) | `block_entity.rs` | the block entity **type** declared in `Hooks::register` (`r.block_entity_type("red_microchip").blocks(&["red_microchip"])`) + `BlockEntityPart::block_entity` (returns the empty starting compound) |
| `block-entity-ticks` (15) | `ticks.rs` | `BlockEntityPart::block_entity_ticks` → `event.ticks = true` |
| `block-entity-tick` (15) | `tick.rs` | `BlockEntityPart::block_entity_tick` → registers the position, refreshes the power cache, answers `Unchanged` |
| `level-tick-post` (23) | `tick_post.rs` | `Hooks::level_tick_post` (server half only; the level comes in the event) |
| `block-signal` (38) | `signal.rs` | `BlockPart::block_signal` (attached with `Registrar::block_part`) |
| `is-signal-source` (38) | `source.rs` | `BlockPart::is_signal_source` |
| `register-commands` (8) | `register.rs` | `Hooks::register_commands` |
| `perform-command` (8) | `perform.rs` | `Hooks::run_mod_command` (typed arguments; `Flow::Cancel` claims it) |
| `world-lifecycle` (v1) | `world.rs` | `Hooks::world_lifecycle` |
| `[[module]] runtime` | `runtime.rs` | `src/runtime.rs` — byte-identical |
| `[[module]] state` | `support.rs` | `src/support.rs` |

No hook in the retired plan is left without a home, and no gap was added by the mapping itself.

### Behaviour that changed with the format (deliberate, and why)

1. **An input port now asks the game.** `LevelRef::signal(host, neighbour, face)` is the engine's own
   `getSignal(…)` read, so deviation 2 of 0.1.0 ("input reads are rebuilt, not asked") is **closed**:
   another mod's `block_signal` is visible to an input port, and a conducting block contributes its
   `getDirectSignalTo` answer exactly as vanilla counts it. The direction is the one the engine's own
   diodes use: `signal(pos_of_neighbour, direction_from_this_chip)`.
2. **`block_signal` answers from the registry's power cache**, not from the block entity. The hook is
   asked from inside the level's update and the SDK says to answer from your own state; a
   `&dyn LevelHost` cannot hand out a block-entity handle, and `BlockEntityRef` cannot be built by a
   mod. The cache is filled from the saved compound in `block_entity_tick` (a reloaded chip answers
   its saved power from its first tick) and from the evaluation, and is cleared on a world close and
   when a different level starts ticking. Window: between a level switch and the first microchip tick
   in the new level, a read at a position that also holds a chip in the other dimension could answer
   the other level's power for one tick — the evaluation then rewrites the faces and notifies, so it
   self-corrects.
3. **The face properties are written one by one** (`host.set_block_property`), not as one resolved
   state, because `LevelRef::set_block` drops the block entity at this pin. Each write is a no-op when
   the value already matches, so a steady circuit writes nothing.
4. **`clear` replaces the block entity's data** (`block_entity_replace_data(empty)`): `merge_data`
   can merge a compound but cannot delete a key.
5. **Commands are typed.** The dispatcher parses `/lbr …` once and hands `run_mod_command` a literal
   path plus resolved arguments, so `runtime::parse_command` is no longer on the glue's path (it stays
   public and tested — `runtime.rs` must not change). Every reply the port produces is unchanged,
   because the refusals are the runtime's own `CommandError` values ("north is named twice",
   "sideways is not a face", "no microchip at 0 64 5", "wrong number of arguments"). What *is* the
   dispatcher's now: a **grammar-level** mistake (`/lbr circuit 0 64 0 flip …`, or a missing
   `<pos>`) is refused by the game before any hook runs, with the game's own message, where 0.1.0's
   split-line hook answered with the port's. `tools/e2e.sh` asserts the state after such a line (the
   circuit is still empty) instead of the port's text.
6. **`<pos:block_pos>` instead of three `<x:integer>`s**, so relative coordinates (`~ ~1 ~`) work.
   Absolute numbers parse and read out exactly as before.
7. **The client half of `level_tick_post` does nothing**: the hook is asked on both sides (the SDK
   gives it a `HostSide`), and only the server half has an authoritative level.

### The ladder (measured 2026-11-18)

Every command ran inside a `ports/tools/build-slot.sh` lease with `BUILD_LOCAL=1` (the far box has no
copy of this port) and `CARGO_INCREMENTAL=0`, directly in the port: there is no staging step and no
staged tree for an SDK package.

| Step | Command | Result |
|---|---|---|
| runtime tests, **before any glue** | `cargo test --offline` (crate root = `pub mod runtime;`) | **19 passed**, 0 failed — `evidence/little_big_redstone/sdk-test-runtime.log` |
| glue red first | `cargo test` with the codec stubbed (`todo!()`) | **5 failed** (the codec tests), 25 passed — `sdk-test-red.log` |
| tests | `cargo test --offline` | **30 passed** (19 `runtime::`, 11 `support::`), 0 failed — `sdk-test.log` |
| lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings — `sdk-clippy.log` |
| format | `cargo fmt --check` (**not** `--all`: that checks the fetched SDK) | exit 0, clean — `sdk-fmt.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `SDK 0b14f8a … in ./.horizon/sdk`, `Built ./target/horizon-mod/little_big_redstone.hmod` — `sdk-build.log` |
| registry check | `cargo horizon-mod check little_big_redstone.hmod` | `little_big_redstone 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)` / `48725 bytes, SHA-256 16b6ff8a…f4de, unsigned`, **EXIT 0** — `sdk-check.log` |
| machine form | `… --json` | `{"ok":true,…,"kind":"native-mod","sha256":"16b6ff8a…","size":48725,"version":"0.2.0"}` — `sdk-check-json.log` |
| fits the pinned build | `cargo horizon-mod fits little_big_redstone.hmod --build nightly` | `Little Big Redstone:` / `0b14f8a  made for`, **EXIT 0** — `sdk-fits.log` |
| same package twice | `rm -f Cargo.lock && cargo horizon-mod build` again | 48,725 B and `16b6ff8a…f4de` both times — `sdk-build-second.log`, `sdk-reproducible.txt` |
| `runtime.rs` untouched | `unzip -p little_big_redstone.hmod src/runtime.rs \| sha256sum` | `6c5cc833…fea6` = `src/runtime.rs` = the retired file, byte for byte — `sdk-runtime-sha256.txt` |
| entries | `unzip -l little_big_redstone.hmod` | **37**: 5 root files (`horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`), `assets/…` 19, `data/…` 14, `src/{lib,runtime,support}.rs`, `uses.json`; **no `Cargo.lock`** — `sdk-entries.log` |
| content carry | size + sha256 of all 28 carried files vs `retired-hnmod-0.1.0/content/` | **28 unchanged, 0 diffs** — `sdk-content-carry.log` |
| what it uses | `unzip -p little_big_redstone.hmod uses.json` | **304 entries**: 31 hook entries (12 hook names — the 9 this port answers plus `blocks`, `command` and `items` from the macro's defaults — and their 19 event fields), 29 methods, 19 traits, 145 types, 73 handles, 3 macros (`horizon_mod`, `horizon_part`, `Wire`), the `block` and `block_entity` part kinds this port attaches (`entity` also appears, from the SDK's own machinery), `capability:registries` — `sdk-uses.json` |

### Traps new to this port (beyond the migration brief's)

1. **A `block_signal` Part cannot read the block entity.** `BlockPart::block_signal` is handed a
   `&dyn LevelHost`, which has `block_entity_data(BlockEntityRef)` but no way to make a handle, and
   `BlockEntityRef`'s fields are `pub(crate)`. The emitted power therefore lives in the port's own
   registry (trap 2 above). A port that answers from a block entity in that hook has no route.
2. **`block_entity_tick`'s reply defaults to `Changed`.** The generated dispatch sets
   `*event.reply = Changed` before each listener and keeps `Changed` unless the listener writes
   `Unchanged`, so a registering tick that leaves it alone dirties every microchip's chunk every
   tick. Set `BlockEntityTickReply::Unchanged` explicitly.
3. **`LevelRef::signal`'s direction is the reader's, not the emitter's.** The engine's own diodes
   call `signal(pos + facing, facing)` to read their input, and `block_signal` receives the emitter's
   face (`direction.opposite()`); reading it the other way round silently inverts every input port.
4. **`LevelRef::set_block` would eat the circuit.** Syncing six face properties with one whole-state
   write drops the block entity (and with it the saved circuit); `set_block_property` keeps it.
5. **`SetBlockProperty` is on `ServerHost` and acts on the level in place**, not on a `LevelRef`; a
   parked level cannot be written through it, so `level_tick_post` gates on `level.in_place(host)`.
6. **`levels()` is on `ServerHost`, not `Host`.** A block-entity tick is not told its level, and the
   helper that finds it (`host.levels().list_levels().find(|l| l.in_place(host))`) needs the server
   host in its signature — `&dyn Host` does not compile.
7. **`BlockSignalEvent.pos` is a `BlockPos`, not `[i32; 3]`** (the retired binding was an array).
8. **`DimensionKey::id(host)` is the way from a command's dimension to a `LevelRef`**
   (`host.level(&name)`): `dimension_key_id` is a handle *backend* method, not a host one.
9. **An unregistered verb never reaches the hook.** A literal the usage does not name is refused by
   the dispatcher, so the port cannot answer "flip is not an action" any more (see behaviour 5).
10. **`JavaString` is still not exported.** Reads go through a local `field(compound, name)` that
    compares `key.to_string_lossy()`, writes through `compound.insert("key".into(), …)`.

### Gaps after the migration

1. **No in-game run in this environment.** The e2e needs a game the launcher built with this package
   (`horizon-launcher native-build .`), and `horizon-launcher` is not installed here — the same trap
   every 0.2.0 migration hit. `tools/e2e.sh` is updated for that run (`GAME=` required,
   `CONTROL_GAME=` optional negative control, `SELFTEST=1` kept) and **no 0.2.0 in-game leg is
   claimed**; the 0.1.0 runs in `../_shared/evidence/little_big_redstone/` (`e2e-default.log`,
   `e2e-embedded.log`, `e2e-selftest.log`) remain the behaviour's evidence for a `runtime.rs` that
   did not change and a glue layer that kept every readout string.
2. **Grammar-level refusals are the dispatcher's** now (behaviour 5), so the `flip` phase of the e2e
   asserts the state instead of the port's message.
3. **The power cache's one-tick window on a level switch** (behaviour 2).
4. **`runtime::parse_command` is unused by the glue.** It stays public and tested because
   `runtime.rs` must stay byte-identical; the typed path reproduces its decisions.
5. The 0.1.0 slice gaps above are unchanged: 15 colours, 20 components, the board GUI, wire
   rendering, the face overlay, the custom model loader, the logic-array items.

### Changelog (0.2.0 — supersedes #902)

> **0.2.0 — republished as an SDK `.hmod`.** Horizon's mod-API merge (launcher ≥ 0.2.70) retired
> `.hnmod` and closed **#902** (`little_big_redstone` 0.1.0); this version supersedes it. One flat
> `little_big_redstone.hmod` now carries the code *and* the content (the microchip block, its item,
> models, textures, loot, both recipes, the `c:` tags and the block/item tags):
> `src/lib.rs` is the one `impl mod_api::Hooks`, `src/support.rs` the glue (one Part for the block and
> its block entity, the tick registry, the evaluation, the command surface), and `src/runtime.rs` is
> byte-identical to 0.1.0, so the circuit, the AND gate and the I/O ports behave exactly as before.
> Built against SDK `0b14f8a` / mixins `e59da90` for 26.3. Changed by the format: the inputs ask the
> engine's own per-direction `getSignal` (the 0.1.0 rebuilt read is gone), `block_signal` answers from
> the port's power cache instead of the block entity, the face properties are written one by one so
> the block entity survives, `clear` replaces the block entity's data, and `/lbr` is a typed command
> (the grammar is the dispatcher's, the replies are the same). Same feature set and the same gaps
> (15 colours, 20 components, board GUI, wire rendering, face overlay, custom model loader,
> logic-array items). No 0.2.0 in-game run: the launcher is not installed here.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port
