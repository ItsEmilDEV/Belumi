#!/usr/bin/env python3
"""Generate 26.2 -> 26.3 registry id translation tables for Belumi.

Input: the `registries.json` data-generator reports of the official servers
  java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports --output out
Entries are matched by resource-location name, never by position.

Block states come from `blocks.json` (matched by block name + properties).

Usage: gen_26_2_ids.py REPORTS_26_2_DIR REPORTS_26_3_DIR OUTPUT.rs
"""
import json
import sys

# (rust const name, registry key, also emit the 26.3 -> 26.2 inverse)
REGISTRIES = [
    ("ENTITY_TYPE", "minecraft:entity_type", False),
    ("BLOCK", "minecraft:block", False),
    ("ITEM", "minecraft:item", True),
    ("DATA_COMPONENT_TYPE", "minecraft:data_component_type", True),
    ("PARTICLE_TYPE", "minecraft:particle_type", False),
    ("SOUND_EVENT", "minecraft:sound_event", False),
    ("CUSTOM_STAT", "minecraft:custom_stat", False),
    ("COMMAND_ARGUMENT_TYPE", "minecraft:command_argument_type", False),
]


def load(directory):
    with open(f"{directory}/registries.json") as f:
        return json.load(f)


def registry_ids(reg, key):
    return {n: e["protocol_id"] for n, e in reg[key]["entries"].items()}


def block_state_ids(directory):
    with open(f"{directory}/blocks.json") as f:
        blocks = json.load(f)
    return {
        (name, json.dumps(st.get("properties"), sort_keys=True)): st["id"]
        for name, block in blocks.items()
        for st in block["states"]
    }


def table(old_ids, new_ids, label):
    out = [None] * (max(old_ids.values()) + 1)
    for name, old_id in old_ids.items():
        out[old_id] = new_ids.get(name, -1)
    assert None not in out, f"{label}: ids are not dense"
    return out


def emit(lines, const, values, doc):
    lines.append(f"pub static {const}: [i32; {len(values)}] = [")
    for i in range(0, len(values), 16):
        lines.append("    " + ", ".join(str(v) for v in values[i : i + 16]) + ",")
    lines.append("];")
    shifted = sum(1 for i, v in enumerate(values) if v != i)
    print(f"{const}: {len(values)} ids, {shifted} shifted, {values.count(-1)} unmapped")


def main():
    old_dir, new_dir, output = sys.argv[1:4]
    old, new = load(old_dir), load(new_dir)
    lines = []
    for const, key, inverse in REGISTRIES:
        o, n = registry_ids(old, key), registry_ids(new, key)
        emit(lines, f"{const}_FROM_26_2", table(o, n, key), f"26.2 -> 26.3 `{key}` ids.")
        if inverse:
            emit(lines, f"{const}_TO_26_2", table(n, o, key), f"26.3 -> 26.2 `{key}` ids.")
    o, n = block_state_ids(old_dir), block_state_ids(new_dir)
    emit(lines, "BLOCK_STATE_FROM_26_2", table({k: v for k, v in o.items()}, n, "block states"), "26.2 -> 26.3 block state ids.")
    with open(output, "w") as f:
        f.write("\n".join(lines) + "\n")


main()
