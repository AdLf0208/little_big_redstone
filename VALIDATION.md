# Validation — Little Big Redstone

## 0.2.2 (pin `0a875da`, 2026-11-21) — current

The gate family slice: OR, XOR, NAND, NOR and NOT beside the 0.2.1 AND gate, the gate word and the
output-port retarget on `/lbr`, and their in-game truth tables. SDK Nightly
**`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** (2026-11-20), mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, game 26.3. Logs:
`ports/_shared/evidence/little_big_redstone/sdk-*.log` and
`ports/_shared/evidence/little_big_redstone-e2e/e2e-*.log`; the red→green record, the coverage
table and the traps are in `notes.md` → *Slice 2*.

| Step | Result |
|---|---|
| red first (five gate rules stubbed) | **8 failed / 42 passed**, `not yet implemented: the or gate's output rule` |
| `cargo test --offline` | **50 passed**, 0 failed |
| `cargo clippy --all-targets -- -D warnings` | exit 0, **0 warnings** |
| `cargo fmt --check` | clean |
| `rm -f Cargo.lock && cargo horizon-mod build` (twice) | **57,072 B**, sha256 `481bec309448c22afc34196d1d32c87828545437d6577c7d78787f0cdb4c8cb8`, `cmp`-identical |
| `cargo horizon-mod check` | `little_big_redstone 0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, **exit 0** |
| `fits --build 0a875da` | `0a875da  made for`, exit 0 |
| `fits --build 0b14f8a` (the different build) | `0b14f8a  fits`, exit 0 |
| `unzip -l` | **37 entries**, no `Cargo.lock`, `uses.json` inside, `LICENSE` + `LICENSE-NOTICE.md` inside |
| tree vs package | `horizon-mod.toml`, `Cargo.toml`, `src/{lib,runtime,support}.rs` and `uses.json` byte-identical |
| `verify-sdk-package.py little_big_redstone` | **1/1** |
| in-game | `PASS` + `EXIT:0`; `SELFTEST=1` `FAIL` + `EXIT:1`; `SELFTEST=gates` `FAIL` + `EXIT:1`; control (`GAME=$CONTROL_GAME`) `FAIL` + `EXIT:1` |
| `check-e2e-logs.py little_big_redstone-e2e --strict` | exit 0, **6/6 ok** |

What the 0.2.2 in-game leg adds to 0.2.1's (full list in `notes.md` → *What the in-game leg proves*):
a redstone wire at the output face and a pass-through output port, so NOT reads `power=1` (and the
saved `microchip.power[5]` = 1), OR reads 5 for inputs 3 and 5, XOR reads 0/15/15/0 and, with four
inputs, 0/0/15/0/15 by parity, NAND and NOR read exactly 1, `status` names each gate, and every wrong
gate shape is refused without touching the circuit.

## 0.2.0 (SDK format, 2026-11-18) — the AND slice on `0b14f8a`

SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`**, mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, game 26.3, one flat
`little_big_redstone.hmod` (code + content). Every step ran in a
`ports/tools/build-slot.sh` lease with `BUILD_LOCAL=1` and `CARGO_INCREMENTAL=0`,
directly in the port — an SDK package has no staging step. Logs:
`ports/_shared/evidence/little_big_redstone/sdk-*.log`; the full table, the file
map, the hook table and the new traps are in `notes.md`
→ [`## Migration to the SDK format`](notes.md#migration-to-the-sdk-format-020-2026-11-18).

| Step | Result |
|---|---|
| `src/runtime.rs` first | **19 passed** before any glue existed (byte-identical, `6c5cc833…fea6`) |
| `cargo test --offline` | **30 passed** (19 runtime + 11 glue), 0 failed |
| `cargo clippy --all-targets -- -D warnings` | exit 0, **0 warnings** |
| `cargo fmt --check` | clean |
| `rm -f Cargo.lock && cargo horizon-mod build` | `target/horizon-mod/little_big_redstone.hmod` |
| `cargo horizon-mod check little_big_redstone.hmod` | `little_big_redstone 0.2.0 (native-mod, format native-1/0b14f8a…/e59da90…, for 26.3)`, **48,725 B**, sha256 `16b6ff8a…f4de`, **exit 0** |
| `cargo horizon-mod fits … --build nightly` | `0b14f8a  made for`, exit 0 |
| second build | same 48,725 B, same sha256 (reproducible) |
| `unzip -l` | **37 entries**, no `Cargo.lock`, `LICENSE` + `LICENSE-NOTICE.md` inside |
| `unzip -p … src/runtime.rs \| sha256sum` | `6c5cc833…fea6`, the retired file byte for byte |
| content carry | 28 files, 0 diffs against the retired copy |
| `uses.json` | 304 entries (12 hook names / 19 event fields, the `block` and `block_entity` parts attached, `capabilities = ["registries"]`) |
| in-game e2e | **not run**: `horizon-launcher` is not installed here, so no 0.2.0 game carrying the package exists. `tools/e2e.sh` is updated for it (`GAME=` required, `CONTROL_GAME=` optional, `SELFTEST=1`) |

## 0.1.0 (retired `.hnmod` format, 2026-11-12 — historical record)

Everything below this heading describes the retired container (`.hnmod`,
`mixins/`, `content/`, `tools/horizon.py`). It is kept as the behaviour's
evidence: the runtime and every readout string are unchanged in 0.2.0. The
retired tree itself is at
`ports/_shared/evidence/little_big_redstone/retired-hnmod-0.1.0/`.

Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` + horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57`, every step through
`ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`. Prepared 26.3 resources
at `/home/adlf/Desktop/horizon/.local/horizon-client/26.3`. Started 2026-11-12.

## Ladder

| Step | Command | Result |
|---|---|---|
| `cargo fmt --check` | in the port | clean |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | in the port | exit 0, 0 warnings |
| `cargo test --locked --offline` | in the port | **19 passed** (the pure runtime) |
| `tools/horizon.py stage` | pin checkout + pin mixins | exit 0, staged |
| `tools/horizon.py check` | `-p horizon-client --bin horizon-client --no-default-features` | `cargo_exit: 0`, **0 warnings** |
| `tools/horizon.py test` | `--lib game::little_big_redstone` | `cargo_exit: 0`, **25 passed** (19 runtime + 6 glue) |
| `tools/horizon.py build` | `--profile fast` | `cargo_exit: 0`, 0 warnings, binary 110,393,592 B |
| `tools/package.sh` | content + bundle | `.hmod` 15,753 B (29 entries), `.hnmod` 37,464 B (17 entries) |
| `cargo horizon-mod check <hmod>` | the pin's own tool | `no code: content only, nothing to run` |
| `cargo horizon-mod check <hnmod> --kind native-mod` | the pin's own tool | exit 0, `native-1/1d63e0a…/5a64af93…` |
| `tools/e2e.sh` | in-game, The End, `--offscreen` | see below |

## Artifacts

| File | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `content/little_big_redstone.hmod` | 15,753 B | `0e34bed5119b4eda4704cd1cdccc01253c87311bdbcc98cd42d41517767cdc2f` | 29 |
| `little_big_redstone.hnmod` | 37,464 B | `2a396c11dcef5681f018e9dd3aa86044c1af37fd6d909d3b66b88988d0fd841d` | 17 |

The `.hnmod` carries `mixins/little_big_redstone/data/little_big_redstone-content.hmod`
byte-identical to `content/little_big_redstone.hmod` (the AE2 pattern), plus
`LICENSE` and `LICENSE-NOTICE.md`. Rebuilding the content from `content/`
reproduces the same 15,753 B / `0e34bed5…` archive, so the delivered bytes are
the built bytes (the e2e ran on exactly these).

## In-game evidence

`ports/_shared/evidence/little_big_redstone/` — one build lease, three legs:

| Log | Leg | Verdict |
|---|---|---|
| `e2e-default.log` | `--dev-mod content/` | `PASS` + `EXIT:0` |
| `e2e-embedded.log` | no `--dev-mod` (the bundle's embedded content), preceded by the unusable-`.horizon-cache` control | `PASS` + `EXIT:0`; the control knew no microchip id and the embedded file is byte-identical to `content/little_big_redstone.hmod` |
| `e2e-selftest.log` | `SELFTEST=1` (the unpowered lamp asserted lit) | `FAIL: expected the lamp at 1,64,0 lit=true, got '…lit=false'` + `EXIT:1` |

`python3 ports/tools/check-e2e-logs.py little_big_redstone --strict`:

```
ok       little_big_redstone    e2e-default.log            PASS + EXIT:0
ok       little_big_redstone    e2e-embedded.log           PASS + EXIT:0
ok       little_big_redstone    e2e-selftest.log           FAIL + EXIT:1
logs=3  ok=3  missing-exit=0  bad-exit=0
```

What the default leg asserts (all in the log, each with the `ctl` reply):

1. the microchip places and `/lbr circuit 0 64 0 and north south east` answers
   `AND gate at 0 64 0: inputs north and south, output east`;
2. the jar's truth table: `0,0 → dark`, `0,1 → dark`, `1,0 → dark`, `1,1 → lit`,
   with the face properties tracking the ports at each step;
3. removing A with B still powered darkens it (not a latch);
4. **analog**: A = 15 (redstone block) with B = 5 (a wire line decaying from a
   block 12 blocks south, read back `power=5`) lights the lamp, and the saved
   `microchip.power` reads 15 on the east face;
5. **a relaunch**: quit, relaunch the same world, come back to the rig — the lamp
   is lit again with no reconfigure, and `/lbr … status` still lists the AND gate;
6. `/lbr … status` names the input ports and the output port; `/lbr … clear`
   empties the circuit and the lamp stays dark with both inputs on;
7. every wrong command is refused with its message and changes nothing
   (duplicate face, unknown face, unknown action, no microchip there);
8. **counterfactual**: with the microchip in place and both inputs on the lamp is
   lit; `stone` in its cell and the lamp stays dark; restored, it lights again;
9. the content: **64 microchip states** bind, the item is `/give`-able, a broken
   microchip drops itself to a diamond pickaxe, the jar's shaped recipe crafts
   one `red_microchip` from the circuit board and the `c:` tags (a stick in the
   board's place crafts nothing), and the circuit-board recipe crafts from
   polished deepslate/diamond/redstone/quartz (a stick crafts nothing).

## Gates

| Gate | Result |
|---|---|
| `python3 ports/tools/check-e2e-logs.py little_big_redstone --strict` | exit 0 (3/3 ok) |
| `python3 ports/tools/check-skills.py --notes ports/little_big_redstone` | PASS |
| `python3 ports/tools/upstream_ready.py little_big_redstone` | exit 1: **`status is pending, not ported`** — the ledger row is the orchestrator's to change (this job's brief forbids editing it) |
| `python3 ports/tools/upstream_ready.py little_big_redstone --include-blocked` | exit 0, `READY` — licence, artifact, `Skills:` line, e2e log and pin all pass |
