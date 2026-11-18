#!/usr/bin/env bash
# In-game check for the Little Big Redstone microchip slice.
#
# The rig (The End, floor at y=63): a microchip at (0,64,0) configured with
# `/lbr circuit 0 64 0 and north south east` — input A on the north face, input B
# on the south face, output on the east face, a redstone lamp at (1,64,0).
# Redstone blocks are the 15-strength sources; a line of redstone wire from a
# block 12 blocks south is the 5-strength source.
#
#   1. the jar's truth table, in game: dark/dark -> dark, A only -> dark,
#      B only -> dark, A+B -> lit (the three wrong combinations are the
#      controls for the one right one);
#   2. removing A with B still powered darkens it again (not a latch);
#   3. analog: A=15 with B=5 still lights it (an AND that demanded 15 on both
#      would stay dark), and the microchip's saved per-face power reads 15;
#   4. the face properties: the powered input faces and the emitting output
#      face are true, the others false;
#   5. the circuit and its state survive a full quit + relaunch of the same
#      world, with no reconfigure;
#   6. `/lbr ... status` reports the AND gate, the two input ports and the
#      output port; `/lbr ... clear` empties it and the lamp stays dark with
#      both inputs on;
#   7. every wrong command is refused with a message and changes nothing
#      (duplicate face, unknown face, no microchip there, unknown action);
#   8. counterfactual: stone in the microchip's cell, both inputs powered ->
#      lamp dark;
#   9. the content half: all 64 microchip states bind, the item is
#      `/give`-able, the block drops itself to a diamond pickaxe, the jar's own
#      shaped recipe crafts from the circuit board and the `c:` tags
#      (`tools/craft_check.py`), and a stick in the circuit board's place
#      crafts nothing.
#
# SELFTEST=1 keeps the rig but asserts the unpowered lamp is LIT, so the run
# must fail: the counterfactual proving the lamp checks can fail.
#
# Since 0.2.0 the port is one flat SDK `.hmod`: the code is compiled into the game
# by the launcher and `assets/`/`data/` are loaded from the same package, so the
# game under test is one the **launcher** built with this package:
#
#   GAME=<instance>/horizon-client tools/e2e.sh
#
# (0.1.0 ran a game built from source with `--dev-mod content/`; that path and the
# embedded-content bundle it needed are gone, see notes.md.)
#
# CONTROL_GAME=<an unmodded launcher game> runs the negative control first: the
# microchip id must be unknown there, so the run proves the ids come from this
# package and not from the harness.
#
# SELFTEST=1 keeps the rig but asserts the unpowered lamp is LIT, so the run
# must fail: the counterfactual proving the lamp checks can fail.
set -euo pipefail

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/little_big_redstone
# The control binary is shared at the campaign pin (same protocol). `GAME` has no
# default on purpose: a native `.hmod` is compiled into the game by the launcher,
# so there is no per-port build to fall back to.
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-}
CONTROL_GAME=${CONTROL_GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/little_big_redstone-e2e.sock}
SELFTEST=${SELFTEST:-0}
WORLD=LittleBigRedstoneE2E
CONTROL_WORLD=LittleBigRedstoneControl

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -n "$GAME" ] || {
    echo "GAME is required: a game the launcher built with little_big_redstone.hmod" >&2
    echo "  (horizon-launcher native-build . --instance <i> --trust-native-code)" >&2
    exit 1
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/little_big_redstone-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

# A server-side write in the End (the rig's dimension).
setblock() { ctl command "execute in minecraft:the_end run setblock $1" >/dev/null; }
# A native mod command is typed as player chat (`ctl command` bypasses
# `perform_command` at this pin).
say() { ctl chat "/$1" >/dev/null; }
messages() { ctl messages 40 | jq -r '.chat[]' 2>/dev/null || true; }

block_state() { ctl block "$1" "$2" "$3" | jq -r '.state // empty'; }
# A state string is `name[prop=value,...]`; the property list alone, comma
# wrapped, so one `prop=value` can be matched at any position.
block_props() { local state; state=$(block_state "$1" "$2" "$3"); state=${state#*[}; printf ',%s,' "${state%]}"; }
block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }

# A face property and a lamp are written by the level-tick evaluation, one tick
# after the input settles, and the client's block copy arrives with the server's
# update: give each assertion a few windows before calling it wrong (measured
# 2026-11-12: the analog phase's lamp read raced its own tick).
expect_lamp() { # x y z lit
    local state i
    for i in 1 2 3 4; do
        state=$(block_state "$1" "$2" "$3")
        case "$state" in
            *"lit=$4"*) echo "lamp $1,$2,$3: $state"; return 0 ;;
        esac
        ctl wait 4
    done
    fail "expected the lamp at $1,$2,$3 lit=$4, got '$state'"
}
# One `key=value` per argument: the pin may print a block state's properties in
# any order, so a combined substring would be brittle.
expect_chip() {
    local state pair i missing
    for i in 1 2 3 4; do
        state=$(block_props 0 64 0)
        missing=""
        for pair in "$@"; do
            case "$state" in
                *",$pair,"*) ;;
                *) missing=$pair ;;
            esac
        done
        if [ -z "$missing" ]; then
            echo "microchip: $state"
            return 0
        fi
        ctl wait 4
    done
    fail "expected the microchip state to contain $missing, got '$(block_state 0 64 0)'"
}
expect_message() { # substring
    local all
    all=$(messages)
    case "$all" in
        *"$1"*) echo "reply: $1" ;;
        *) fail "no chat reply containing '$1'; last lines: $(printf '%s' "$all" | tail -3 | tr '\n' '|')" ;;
    esac
}

# The microchip's saved per-face power (`MicrochipBlockEntity`'s compound, the
# port's `power` cache), printed as the int array.
chip_power() {
    ctl command "execute in minecraft:the_end run data get block 0 64 0 microchip.power" \
        | jq -r '.chat // [] | join(" ")'
}

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ -f "$SCRATCH/logs/latest.log" ]; then
        cp "$SCRATCH/logs/latest.log" "$PORT/target/last-game.log" 2>/dev/null || true
    fi
    rm -rf "$SCRATCH" 2>/dev/null || true
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD] [GAME]
    local world=${1:-$WORLD}
    local game=${2:-$GAME}
    rm -f "$SOCK"
    cd "$SCRATCH"
    # No --dev-mod: a native `.hmod` is compiled into the launcher's game and its
    # assets/data travel in the same package.
    "$CTL" --socket "$SOCK" launch --game "$game" --world "$world" --seed 20261112 \
        --creative --offscreen --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
}

# Back at the rig after a launch or a relaunch, without touching the blocks.
# The wait matters: `execute in ... run fill` before the End chunk is loaded
# answers ok and changes nothing, and a redstone wire placed on the missing
# floor is removed a tick later (measured 2026-11-12).
go_home() {
    ctl command "execute in minecraft:the_end run tp @s 2.5 64 3.5" >/dev/null
    ctl wait 20
}

# A fresh world's rig: a stone floor and clear air, built after the player is in
# the End and the chunk is loaded. The player stands south-east of the
# microchip (never inside it, and out of the wire line's way).
build_rig() {
    go_home
    ctl command "execute in minecraft:the_end run fill -6 63 -6 16 63 16 minecraft:stone" >/dev/null
    ctl command "execute in minecraft:the_end run fill -6 64 -6 16 70 16 minecraft:air" >/dev/null
    ctl wait 10
    floor=$(block_name 0 63 1)
    [ "$floor" = "minecraft:stone" ] || fail "the rig floor did not fill at 0,63,1 (read '$floor')"
    go_home
}

# The negative control (CONTROL_GAME): an unmodded launcher game, where the
# microchip id must be unknown — the run then proves the ids come from this
# package and not from the harness.
control_leg() {
    echo "== control: an unmodded launcher game knows no microchip =="
    launch "$CONTROL_WORLD" "$CONTROL_GAME"
    build_rig
    setblock "0 64 0 little_big_redstone:red_microchip" 2>/dev/null || true
    ctl wait 6
    local name
    name=$(block_name 0 64 0)
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the microchip id was known"
    ctl give little_big_redstone:red_microchip 1 >/dev/null 2>&1 || true
    ctl wait 4
    ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "little_big_redstone:red_microchip")] | length == 0' >/dev/null \
        || fail "the control game has a microchip item, so the package is installed there"
    echo "  without the package: no microchip item, the cell stayed $name"
    ctl quit >/dev/null 2>&1 || true
}

echo "== launch (The End, a launcher-built game carrying little_big_redstone.hmod) =="
if [ -n "$CONTROL_GAME" ]; then control_leg; fi
launch
build_rig

echo "== the microchip block binds and the command builds the guide's AND scene =="
setblock "0 64 0 little_big_redstone:red_microchip"
setblock "1 64 0 minecraft:redstone_lamp"
ctl wait 6
[ "$(block_name 0 64 0)" = "little_big_redstone:red_microchip" ] || fail "the microchip did not place"
say "lbr circuit 0 64 0 and north south east"
ctl wait 6
expect_message "AND gate at 0 64 0: inputs north and south, output east"
expect_chip east=false north=false south=false

echo "== truth table 0,0 -> 0 (unpowered is dark) =="
if [ "$SELFTEST" = 1 ]; then
    expect_lamp 1 64 0 true   # selftest: the unpowered lamp is asserted lit (must FAIL)
else
    expect_lamp 1 64 0 false
fi

echo "== truth table 0,1 -> 0 (B only is dark) =="
setblock "0 64 1 minecraft:redstone_block"
ctl wait 6
expect_lamp 1 64 0 false
expect_chip north=false south=true east=false

echo "== truth table 1,0 -> 0 (A only is dark) =="
setblock "0 64 1 minecraft:air"
setblock "0 64 -1 minecraft:redstone_block"
ctl wait 6
expect_lamp 1 64 0 false
expect_chip north=true south=false east=false

echo "== truth table 1,1 -> 1 (both inputs light the lamp) =="
setblock "0 64 1 minecraft:redstone_block"
ctl wait 6
expect_lamp 1 64 0 true
expect_chip east=true north=true south=true

echo "== removing A with B still on darkens it (not a latch) =="
setblock "0 64 -1 minecraft:air"
ctl wait 6
expect_lamp 1 64 0 false
expect_chip east=false north=false south=true

echo "== analog: A=15 with B=5 still lights it, and the saved power is 15 =="
setblock "0 64 1 minecraft:air"
setblock "0 64 -1 minecraft:redstone_block"
# A line of 11 wires from a block at z=12 decays to power 5 at the microchip's
# south face. Each placement is read back: a wire that did not stick would
# otherwise only show up as a wrong number at the end.
for z in $(seq 1 11); do
    setblock "0 64 $z minecraft:redstone_wire"
    ctl wait 1
    got=$(block_state 0 64 "$z")
    case "$got" in
        *redstone_wire*) ;;
        *) fail "the wire at 0,64,$z did not place (read '$got')" ;;
    esac
done
setblock "0 64 12 minecraft:redstone_block"
# The client's block copy arrives with the server's update; give it two windows
# before calling the read wrong.
wire=$(block_state 0 64 1)
case "$wire" in
    *"power=5"*) ;;
    *)
        ctl wait 20
        wire=$(block_state 0 64 1)
        ;;
esac
case "$wire" in
    *"power=5"*) echo "south wire: $wire" ;;
    *)
        echo "the wire line as read:" >&2
        for z in $(seq 1 12); do echo "  0,64,$z: $(block_state 0 64 "$z")" >&2; done
        fail "expected the wire at 0,64,1 to carry power=5, got '$wire'"
        ;;
esac
expect_lamp 1 64 0 true
power=$(chip_power)
case "$power" in
    *"15"*) echo "microchip power: $power" ;;
    *) fail "expected the microchip to emit 15 on its east face, got '$power'" ;;
esac

echo "== the circuit and its state survive a relaunch =="
ctl quit >/dev/null 2>&1 || true
sleep 2
launch
# A relaunch can leave the End's win screen up and the player at the overworld
# spawn; close it and come back to the rig.
ctl close >/dev/null 2>&1 || true
ctl wait 4
go_home
ctl wait 10
expect_lamp 1 64 0 true
say "lbr circuit 0 64 0 status"
ctl wait 6
expect_message "slot 2: AND gate with 2 inputs"

echo "== status reports the ports; clear empties the microchip =="
say "lbr circuit 0 64 0 status"
ctl wait 6
expect_message "I/O input on north"
expect_message "I/O output on east"
say "lbr circuit 0 64 0 clear"
ctl wait 6
expect_message "cleared the microchip at 0 64 0"
ctl wait 6
expect_lamp 1 64 0 false
expect_chip east=false north=false south=false
say "lbr circuit 0 64 0 status"
ctl wait 6
expect_message "has no circuit"

echo "== every wrong command is refused and changes nothing =="
say "lbr circuit 0 64 0 and north north east"
ctl wait 4
expect_message "north is named twice"
say "lbr circuit 0 64 0 and north south sideways"
ctl wait 4
expect_message "sideways is not a face"
# `flip` is not a registered literal, so since 0.2.0 the game's **dispatcher** refuses it
# before the hook runs (a typed command is parsed once by the game; there is no split line
# left to parse). The assertion is the state, not the game's own text: the circuit is still
# empty, so the refused verb built nothing.
say "lbr circuit 0 64 0 flip north south east"
ctl wait 4
echo "  unregistered verb refused before the hook; last reply: $(messages | tail -1)"
say "lbr circuit 0 64 0 status"
ctl wait 6
expect_message "has no circuit"
say "lbr circuit 0 64 5 status"
ctl wait 4
expect_message "no microchip at 0 64 5"
say "lbr circuit 0 64 0 and north south east"
ctl wait 6
expect_message "AND gate at 0 64 0"
ctl wait 6
expect_lamp 1 64 0 true

echo "== counterfactual: stone in the microchip's cell, both inputs on =="
# The analog line is removed first: at this pin a redstone wire strongly powers
# a conductor beside it (`RedstoneWireBlock.getDirectSignal` ignores the
# direction), so a stone block next to a wire would be lit through the wire and
# the control would prove nothing about the microchip.
for z in $(seq 1 12); do setblock "0 64 $z minecraft:air"; done
setblock "0 64 1 minecraft:redstone_block"
setblock "0 64 -1 minecraft:redstone_block"
ctl wait 8
expect_lamp 1 64 0 true
echo "with the microchip in place and both inputs powered: the lamp is lit"
setblock "0 64 0 minecraft:stone"
ctl wait 6
expect_lamp 1 64 0 false
echo "stone at 0,64,0 with both inputs powered: the lamp stays dark"
setblock "0 64 0 little_big_redstone:red_microchip"
ctl wait 4
say "lbr circuit 0 64 0 and north south east"
ctl wait 6
expect_lamp 1 64 0 true
echo "restored: the microchip lights it again"

echo "== the content half: 64 states, the item, the drop, the recipe =="
# The state loop writes every face combination, so nothing may rewrite the face
# properties while it runs: empty the circuit and take the inputs away first (an
# empty circuit is not evaluated, so the state stays what `setblock` wrote).
say "lbr circuit 0 64 0 clear"
setblock "0 64 1 minecraft:air"
setblock "0 64 -1 minecraft:air"
ctl wait 8
accepted=0
for down in false true; do
    for up in false true; do
        for north in false true; do
            for south in false true; do
                for west in false true; do
                    for east in false true; do
                        state="down=$down,east=$east,north=$north,south=$south,up=$up,west=$west"
                        setblock "0 64 0 little_big_redstone:red_microchip[$state]"
                        ctl wait 1
                        got=$(block_props 0 64 0)
                        ok=1
                        for pair in "down=$down" "east=$east" "north=$north" "south=$south" "up=$up" "west=$west"; do
                            case "$got" in
                                *",$pair,"*) ;;
                                *) ok=0 ;;
                            esac
                        done
                        [ "$ok" = 1 ] || fail "state $state did not bind, read back '$got'"
                        accepted=$((accepted + 1))
                    done
                done
            done
        done
    done
done
[ "$accepted" -eq 64 ] || fail "only $accepted of 64 microchip states bound"
echo "64 microchip states bound"
say "lbr circuit 0 64 0 and north south east"
ctl wait 6

ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl give little_big_redstone:red_microchip 1 >/dev/null || fail "cannot give the microchip"
ctl wait 4
ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "little_big_redstone:red_microchip")] | length >= 1' >/dev/null \
    || fail "the microchip item did not arrive"
echo "the microchip item is giveable"

# The block drops itself (upstream's `self` loot table with
# `requiresCorrectToolForDrops`): in survival, a diamond pickaxe picks it up.
ctl command "gamemode survival @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl command "give @s minecraft:diamond_pickaxe 1" >/dev/null
ctl wait 4
ctl look-at-block 0 64 0 >/dev/null || fail "cannot aim at the microchip"
ctl wait 2
reply=$(ctl break 0 64 0) || fail "cannot break the microchip: $reply"
ctl wait 4
held=$(ctl inventory | jq -r '.counts["little_big_redstone:red_microchip"] // 0')
ground=$(ctl entities 32 item | jq '[.entities[] | select(.kind == "item" and .item.item == "little_big_redstone:red_microchip") | .item.count] | add // 0')
[ $((held + ground)) -ge 1 ] || fail "the microchip dropped $((held + ground)), expected at least one"
echo "the broken microchip dropped $((held + ground)) of itself"
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 2

# The recipe: the jar's own shaped recipe over the circuit board and the `c:`
# tags, in a real crafting table (a stick in the board's place crafts nothing).
setblock "8 64 0 minecraft:crafting_table"
ctl command "execute in minecraft:the_end run tp @s 8.5 64 2.5" >/dev/null
ctl wait 5
python3 "$PORT/tools/craft_check.py" "$SOCK" "$CTL" 8 64 0 || fail "the microchip recipe check failed"
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 2

echo "PASS: truth table 0,0 / 0,1 / 1,0 dark and 1,1 lit, A removed with B on dark (not a latch), A=15 with B=5 lights it and the saved power is 15, face properties track the ports, the circuit and its state survive a relaunch, status reports the AND gate and its ports, clear empties it, every wrong command is refused and changes nothing, stone in the cell stays dark, 64 microchip states bind, the item is giveable, the block drops itself, and the jar's shaped recipe crafts from the circuit board and the c: tags while a stick crafts nothing"
