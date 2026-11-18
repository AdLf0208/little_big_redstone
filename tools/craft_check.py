#!/usr/bin/env python3
"""Craft the Little Big Redstone microchip in a real crafting table.

Called by `tools/e2e.sh` with the open game's control socket. Uses the harness's
crafting grid (`ctl screen`: slots 100-108 row-major, result 200), gives the
ingredients and asserts the jar's own shaped recipes:

1. `data/little_big_redstone/recipe/microchip/red.json`
   `iIi / RCR / iDi` with `i` = `c:ingots/iron`, `I` = `c:storage_blocks/iron`,
   `R` = `c:dusts/redstone`, `C` = the circuit board, `D` = `c:dyes/red` -> one
   `little_big_redstone:red_microchip`, and the stack reaches the inventory;
2. the same grid with a stick where the circuit board belongs -> an empty
   result (the control: a recipe that matched anything would craft here);
3. `data/little_big_redstone/recipe/redstone_circuit_board.json`
   `DdD / RQR / DdD` with `D` = polished deepslate, `d` = `c:gems/diamond`,
   `R` = `c:dusts/redstone`, `Q` = `c:gems/quartz` -> one
   `little_big_redstone:redstone_circuit_board`; a stick in the quartz's place
   -> an empty result.

The `c:` tags are the port's own declarations (26.3 ships none), so a missing
or misnamed tag makes case 1 fail: the recipe would name an empty ingredient.

The table is replaced before every case, so a failed or untaken result cannot
leak into the next grid. Exits 1 with the observed result on the first
mismatch.
"""
import json
import subprocess
import sys

GRID = {
    "top_left": 100,
    "top": 101,
    "top_right": 102,
    "left": 103,
    "centre": 104,
    "right": 105,
    "bottom_left": 106,
    "bottom": 107,
    "bottom_right": 108,
}
RESULT = 200


def main():
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]

    def run(*args):
        out = subprocess.run([ctl, "--socket", sock, *args], text=True, capture_output=True)
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(ticks=4):
        run("wait", str(ticks))

    def slots():
        screen = run("screen")
        if "slots" not in screen:
            raise SystemExit(f"no menu is open: {screen}")
        return screen["slots"]

    def find(item):
        for slot in slots():
            if slot.get("item") == item:
                return slot["slot"]
        raise SystemExit(f"missing ingredient {item} in the inventory")

    def result():
        """The result slot's stack, or None: an empty slot has `item: null`."""
        stack = [s for s in slots() if s["slot"] == RESULT]
        if not stack or not stack[0].get("item"):
            return None
        return stack[0]

    def setup(items):
        try:
            run("close")
        except SystemExit:
            pass
        wait(5)
        # Replace the table so its block entity, and therefore the crafting
        # grid, is fresh for this case.
        run("setblock", x, y, z, "minecraft:air")
        run("setblock", x, y, z, "minecraft:crafting_table")
        wait(2)
        # A client `tp` can be swallowed while the inventory screen is closing;
        # a server `/tp` moves the player even then.
        run("command", f"tp @s {float(x) - 0.5} {y} {float(z) + 0.5}")
        wait(3)
        run("command", "clear @s")
        for item, count in items:
            run("give", item, str(count))
        wait(3)
        run("interact", x, y, z)
        wait(5)

    def place(item, targets):
        """Right-click one item into each target slot, then put the cursor back."""
        slot = find(item)
        run("click-slot", str(slot))
        wait(1)
        for target in targets:
            run("click-slot", str(target), "right")
            wait(1)
        run("click-slot", str(slot))
        wait(2)

    failures = []

    def craft(label, items, placements, want_item, want_count):
        setup(items)
        for item, targets in placements:
            place(item, targets)
        got = result()
        print(f"{label} ->", json.dumps(got))
        if not got or got.get("item") != want_item or got.get("count") != want_count:
            failures.append(f"{label}: expected {want_count} {want_item}, got {got}")
            return
        # Take the result and check it reaches the inventory.
        run("click-slot", str(RESULT))
        wait(5)
        run("click-slot", "9")
        wait(3)
        made = run("inventory")["counts"].get(want_item, 0)
        if made < want_count:
            failures.append(f"{label}: the crafted stack reached the inventory as {made}, not {want_count}")

    def empty_control(label, items, placements):
        setup(items)
        for item, targets in placements:
            place(item, targets)
        got = result()
        print(f"{label} ->", json.dumps(got))
        if got is not None:
            failures.append(f"{label}: the control crafted {got['item']}")

    # 1. The microchip: iIi / RCR / iDi.
    microchip = [
        ("minecraft:iron_ingot", 4),
        ("minecraft:iron_block", 1),
        ("minecraft:redstone", 2),
        ("little_big_redstone:redstone_circuit_board", 1),
        ("minecraft:red_dye", 1),
    ]
    craft(
        "iIi/RCR/iDi with the circuit board",
        microchip,
        [
            (
                "minecraft:iron_ingot",
                [
                    str(GRID["top_left"]),
                    str(GRID["top_right"]),
                    str(GRID["bottom_left"]),
                    str(GRID["bottom_right"]),
                ],
            ),
            ("minecraft:iron_block", [str(GRID["top"])]),
            ("minecraft:redstone", [str(GRID["left"]), str(GRID["right"])]),
            ("little_big_redstone:redstone_circuit_board", [str(GRID["centre"])]),
            ("minecraft:red_dye", [str(GRID["bottom"])]),
        ],
        "little_big_redstone:red_microchip",
        1,
    )

    # 2. The control: a stick in the circuit board's place.
    craft_control = [
        ("minecraft:iron_ingot", 4),
        ("minecraft:iron_block", 1),
        ("minecraft:redstone", 2),
        ("minecraft:stick", 1),
        ("minecraft:red_dye", 1),
    ]
    empty_control(
        "iIi/RsR/iDi with a stick",
        craft_control,
        [
            (
                "minecraft:iron_ingot",
                [
                    str(GRID["top_left"]),
                    str(GRID["top_right"]),
                    str(GRID["bottom_left"]),
                    str(GRID["bottom_right"]),
                ],
            ),
            ("minecraft:iron_block", [str(GRID["top"])]),
            ("minecraft:redstone", [str(GRID["left"]), str(GRID["right"])]),
            ("minecraft:stick", [str(GRID["centre"])]),
            ("minecraft:red_dye", [str(GRID["bottom"])]),
        ],
    )

    # 3. The circuit board: DdD / RQR / DdD.
    craft(
        "DdD/RQR/DdD",
        [
            ("minecraft:polished_deepslate", 4),
            ("minecraft:diamond", 2),
            ("minecraft:redstone", 2),
            ("minecraft:quartz", 1),
        ],
        [
            (
                "minecraft:polished_deepslate",
                [
                    str(GRID["top_left"]),
                    str(GRID["top_right"]),
                    str(GRID["bottom_left"]),
                    str(GRID["bottom_right"]),
                ],
            ),
            ("minecraft:diamond", [str(GRID["top"]), str(GRID["bottom"])]),
            ("minecraft:redstone", [str(GRID["left"]), str(GRID["right"])]),
            ("minecraft:quartz", [str(GRID["centre"])]),
        ],
        "little_big_redstone:redstone_circuit_board",
        1,
    )

    # 4. Its control: a stick where the quartz belongs.
    empty_control(
        "DdD/RsR/DdD with a stick",
        [
            ("minecraft:polished_deepslate", 4),
            ("minecraft:diamond", 2),
            ("minecraft:redstone", 2),
            ("minecraft:stick", 1),
        ],
        [
            (
                "minecraft:polished_deepslate",
                [
                    str(GRID["top_left"]),
                    str(GRID["top_right"]),
                    str(GRID["bottom_left"]),
                    str(GRID["bottom_right"]),
                ],
            ),
            ("minecraft:diamond", [str(GRID["top"]), str(GRID["bottom"])]),
            ("minecraft:redstone", [str(GRID["left"]), str(GRID["right"])]),
            ("minecraft:stick", [str(GRID["centre"])]),
        ],
    )

    try:
        run("close")
    except SystemExit:
        pass

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    print("recipe: the microchip and the circuit board craft from the jar's own shaped recipes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
