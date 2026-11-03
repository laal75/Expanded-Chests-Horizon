"""Writes the Horizon Chests content mod (data, assets and original textures) into argv[1]."""
import json
import os
import random
import struct
import sys
import zlib

ROOT = sys.argv[1]
ID = "horizon_chests"
# The one place the version is set (tools/version.py keeps the rest in step).
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "VERSION"), encoding="utf-8") as f:
    VERSION = f.read().strip()


def write(rel, obj):
    path = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        if isinstance(obj, str):
            f.write(obj)
        else:
            json.dump(obj, f, indent=2)
            f.write("\n")


def png(rel, pixels):
    """pixels: 16 rows of 16 (r, g, b, a)."""
    raw = b"".join(b"\x00" + bytes(c for px in row for c in px) for row in pixels)
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    body = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 16, 16, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    path = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(body)


def shade(c, k):
    return tuple(max(0, min(255, int(v * k))) for v in c[:3]) + (255,)


# ---- tiers ----------------------------------------------------------------------------------
# name: (title, metal colour, columns, rows, blast resistance, recipe pattern, key). Each tier is
# crafted around the one before it; native/api/src/lib.rs has the same columns and rows.
TIERS = {
    "iron_chest": ("Iron Chest", (196, 198, 202), 9, 6, 6.0,
                   ["III", "ICI", "III"], {"I": "minecraft:iron_ingot", "C": "minecraft:chest"}),
    "gold_chest": ("Gold Chest", (240, 200, 60), 12, 6, 6.0,
                   ["GGG", "GCG", "GGG"], {"G": "minecraft:gold_ingot", "C": f"{ID}:iron_chest"}),
    "diamond_chest": ("Diamond Chest", (92, 222, 214), 15, 6, 6.0,
                      ["LDL", "DCD", "LDL"],
                      {"L": "minecraft:glass", "D": "minecraft:diamond", "C": f"{ID}:gold_chest"}),
    "netherite_chest": ("Netherite Chest", (78, 70, 74), 18, 6, 1200.0,
                        ["ONO", "OCO", "OOO"],
                        {"O": "minecraft:obsidian", "N": "minecraft:netherite_ingot", "C": f"{ID}:diamond_chest"}),
}
WOOD = (150, 108, 62)


# ---- textures -------------------------------------------------------------------------------
def body(metal, seed):
    """A wooden box in a metal frame, with the lid's seam a third of the way down."""
    rng = random.Random(seed)
    px = []
    for y in range(16):
        row = []
        for x in range(16):
            if x in (0, 15) or y in (0, 15):
                c = shade(metal, 0.62)
            elif x in (1, 14) or y in (1, 14):
                c = shade(metal, 1.0 + rng.random() * 0.08)
            elif y == 5:
                c = shade(metal, 0.75)
            else:
                # Plank grain runs across; every fourth row is a gap between boards.
                k = 0.55 if (y - 2) % 4 == 3 else 0.9 + rng.random() * 0.15
                c = shade(WOOD, k)
            row.append(c)
        px.append(row)
    return px


def front(metal, seed):
    px = body(metal, seed)
    # The latch, over the seam.
    for y in range(4, 9):
        for x in range(6, 10):
            edge = x in (6, 9) or y in (4, 8)
            px[y][x] = shade(metal, 0.6 if edge else 1.25)
    px[6][7] = px[6][8] = shade(metal, 0.4)
    return px


def top(metal, seed):
    rng = random.Random(seed)
    px = []
    for y in range(16):
        row = []
        for x in range(16):
            if x in (0, 15) or y in (0, 15):
                c = shade(metal, 0.62)
            elif x in (1, 14) or y in (1, 14) or x in (7, 8):
                c = shade(metal, 1.0 + rng.random() * 0.08)
            else:
                k = 0.55 if (x - 2) % 4 == 3 else 0.9 + rng.random() * 0.15
                c = shade(WOOD, k)
            row.append(c)
        px.append(row)
    return px


for i, (name, (_, metal, *_)) in enumerate(TIERS.items()):
    png(f"assets/{ID}/textures/block/{name}_front.png", front(metal, 10 + i))
    png(f"assets/{ID}/textures/block/{name}_side.png", body(metal, 20 + i))
    png(f"assets/{ID}/textures/block/{name}_top.png", top(metal, 30 + i))

# ---- manifest -------------------------------------------------------------------------------
write("horizon-mod.toml", f'''id = "{ID}"
name = "Horizon Chests"
version = "{VERSION}"
description = "Bigger chests in four tiers. Content half; the code is horizon_chests_core."
authors = []
api = "0.2"
side = "both"
capabilities = ["registries"]
''')

# ---- blocks ---------------------------------------------------------------------------------
# The furnace template gives a `facing` toward the player who placed it; `lit` is never set.
FACINGS = {"north": 0, "east": 90, "south": 180, "west": 270}
lang = {}
for name, (title, _, cols, rows, blast, _, _) in TIERS.items():
    write(f"data/{ID}/horizon/block/{name}.json", {
        "template": "minecraft:furnace",
        "properties": {"hardness": 3.0, "explosion_resistance": blast, "requires_tool": False,
                       "sounds": "minecraft:iron_block"},
    })
    write(f"data/{ID}/horizon/menu/{name}.json", {"slots": cols * rows})
    write(f"assets/{ID}/models/block/{name}.json", {
        "parent": "minecraft:block/orientable",
        "textures": {
            "front": f"{ID}:block/{name}_front",
            "side": f"{ID}:block/{name}_side",
            "top": f"{ID}:block/{name}_top",
        },
    })
    variants = {}
    for facing, rot in FACINGS.items():
        for lit in ("false", "true"):
            v = {"model": f"{ID}:block/{name}"}
            if rot:
                v["y"] = rot
            variants[f"facing={facing},lit={lit}"] = v
    write(f"assets/{ID}/blockstates/{name}.json", {"variants": variants})
    write(f"assets/{ID}/items/{name}.json", {"model": {"type": "minecraft:model", "model": f"{ID}:block/{name}"}})
    write(f"data/{ID}/loot_table/blocks/{name}.json", {
        "type": "minecraft:block",
        "pools": [{"condition": {"type": "minecraft:survives_explosion"},
                   "entries": [{"type": "minecraft:item", "name": f"{ID}:{name}"}], "rolls": 1}],
        "random_sequence": f"{ID}:blocks/{name}",
    })
    lang[f"block.{ID}.{name}"] = title
    lang[f"container.{ID}.{name}"] = title
write("data/minecraft/tags/block/mineable/pickaxe.json", {"values": [f"{ID}:{n}" for n in TIERS]})
# The common chests tag, which other mods read to treat these as chests: Horizon Machines'
# item tubes connect to what is in it.
write("data/c/tags/block/chests.json", {"values": [f"{ID}:{n}" for n in TIERS]})

# ---- the storage barrel ---------------------------------------------------------------------
# One item, kept as a count: 32 stacks, doubled by each Storage Upgrade up to 1024 with five.
# native/api/src/barrel.rs has the slots: 0 takes in, 1 gives out, 2 holds the upgrades.
BARREL = "storage_barrel"
OAK = (164, 124, 74)
STEEL = (150, 154, 160)


def barrel_side(seed):
    rng = random.Random(seed)
    px = []
    for y in range(16):
        row = []
        for x in range(16):
            if y in (2, 13):
                c = shade(STEEL, 0.9 + rng.random() * 0.1)
            elif x in (0, 15) or y in (0, 15):
                c = shade(OAK, 0.6)
            else:
                # Staves run up and down.
                c = shade(OAK, 0.6 if x % 5 == 0 else 0.88 + rng.random() * 0.14)
            row.append(c)
        px.append(row)
    return px


def barrel_front(seed):
    rng = random.Random(seed)
    px = []
    for y in range(16):
        row = []
        for x in range(16):
            if x in (0, 15) or y in (0, 15):
                c = shade(STEEL, 0.75)
            elif x in (1, 14) or y in (1, 14):
                c = shade(OAK, 0.6)
            elif 3 <= x <= 12 and 3 <= y <= 12:
                # The label frame the item sits behind.
                edge = x in (3, 12) or y in (3, 12)
                c = shade(STEEL, 1.1) if edge else shade((48, 36, 26), 1.0 + rng.random() * 0.1)
            else:
                c = shade(OAK, 0.85 + rng.random() * 0.12)
            row.append(c)
        px.append(row)
    return px


def barrel_top(seed):
    rng = random.Random(seed)
    px = []
    for y in range(16):
        row = []
        for x in range(16):
            d = max(abs(x - 7.5), abs(y - 7.5))
            if d > 6.5:
                c = shade(STEEL, 0.8)
            else:
                c = shade(OAK, 0.6 if y % 4 == 0 else 0.86 + rng.random() * 0.14)
            row.append(c)
        px.append(row)
    return px



def upgrade_icon():
    px = [[(0, 0, 0, 0)] * 16 for _ in range(16)]
    for y in range(2, 14):
        for x in range(2, 14):
            edge = x in (2, 13) or y in (2, 13)
            px[y][x] = shade(STEEL, 0.6) if edge else shade(OAK, 0.95)
    # An arrow pointing up: more room.
    green = (90, 200, 90, 255)
    for y in range(7, 12):
        px[y][7] = px[y][8] = green
    for i in range(3):
        for x in range(6 - i, 10 + i):
            px[4 + i][x] = green
    return px


png(f"assets/{ID}/textures/block/{BARREL}_front.png", barrel_front(50))
png(f"assets/{ID}/textures/block/{BARREL}_side.png", barrel_side(51))
png(f"assets/{ID}/textures/block/{BARREL}_top.png", barrel_top(52))
png(f"assets/{ID}/textures/item/storage_upgrade.png", upgrade_icon())

write(f"data/{ID}/horizon/block/{BARREL}.json", {
    "template": "minecraft:furnace",
    "properties": {"hardness": 2.5, "explosion_resistance": 2.5, "requires_tool": False,
                   "sounds": "minecraft:barrel"},
})
write(f"data/{ID}/horizon/menu/{BARREL}.json", {"slots": 3, "data_slots": 4, "results": [1]})
write(f"assets/{ID}/models/block/{BARREL}.json", {
    "parent": "minecraft:block/orientable",
    "textures": {"front": f"{ID}:block/{BARREL}_front", "side": f"{ID}:block/{BARREL}_side",
                 "top": f"{ID}:block/{BARREL}_top"},
})
variants = {}
for facing, rot in FACINGS.items():
    for lit in ("false", "true"):
        v = {"model": f"{ID}:block/{BARREL}"}
        if rot:
            v["y"] = rot
        variants[f"facing={facing},lit={lit}"] = v
write(f"assets/{ID}/blockstates/{BARREL}.json", {"variants": variants})
write(f"assets/{ID}/items/{BARREL}.json", {"model": {"type": "minecraft:model", "model": f"{ID}:block/{BARREL}"}})
# The code puts what the barrel held on the dropped barrel.
write(f"data/{ID}/loot_table/blocks/{BARREL}.json", {
    "type": "minecraft:block",
    "pools": [{"entries": [{"type": "minecraft:item", "name": f"{ID}:{BARREL}"}], "rolls": 1}],
    "random_sequence": f"{ID}:blocks/{BARREL}",
})
lang[f"block.{ID}.{BARREL}"] = "Storage Barrel"
lang[f"container.{ID}.{BARREL}"] = "Storage Barrel"
write("data/minecraft/tags/block/mineable/axe.json", {"values": [f"{ID}:{BARREL}"]})
write("data/c/tags/block/barrels.json", {"values": [f"{ID}:{BARREL}"]})

write(f"data/{ID}/horizon/item/storage_upgrade.json", {"template": "minecraft:sugar"})
write(f"assets/{ID}/models/item/storage_upgrade.json",
      {"parent": "minecraft:item/generated", "textures": {"layer0": f"{ID}:item/storage_upgrade"}})
write(f"assets/{ID}/items/storage_upgrade.json",
      {"model": {"type": "minecraft:model", "model": f"{ID}:item/storage_upgrade"}})
lang[f"item.{ID}.storage_upgrade"] = "Storage Upgrade"


# ---- crafting and the recipe book -----------------------------------------------------------
def recipe(name, pattern, key, unlock):
    write(f"data/{ID}/recipe/{name}.json", {
        "type": "minecraft:crafting_shaped", "category": "misc", "pattern": pattern, "key": key,
        "result": {"id": f"{ID}:{name}"},
    })
    # Unlocks once the player holds `unlock`.
    criteria = {"has_the_recipe": {"trigger": "minecraft:recipe_unlocked",
                                   "conditions": {"recipe": f"{ID}:{name}"}},
                "has_item": {"trigger": "minecraft:inventory_changed",
                             "conditions": {"items": [{"items": unlock}]}}}
    write(f"data/{ID}/advancement/recipes/misc/{name}.json", {
        "parent": "minecraft:recipes/root",
        "criteria": criteria,
        "requirements": [list(criteria)],
        "rewards": {"recipes": [f"{ID}:{name}"]},
    })


for name, (_, _, _, _, _, pattern, key) in TIERS.items():
    recipe(name, pattern, key, key["C"])
recipe(BARREL, ["PIP", "IBI", "PIP"],
       {"P": "#minecraft:planks", "I": "minecraft:iron_ingot", "B": "minecraft:barrel"}, "minecraft:barrel")
recipe("storage_upgrade", ["IGI", "GCG", "IGI"],
       {"I": "minecraft:iron_ingot", "G": "minecraft:gold_ingot", "C": "minecraft:chest"}, f"{ID}:{BARREL}")

lang[f"itemGroup.{ID}.chests"] = "Horizon Chests"
write(f"assets/{ID}/lang/en_us.json", lang)
write(f"data/{ID}/horizon/creative_tab/chests.json", {
    "title": "Horizon Chests",
    "icon": f"{ID}:diamond_chest",
    "items": [f"{ID}:{n}" for n in TIERS] + [f"{ID}:{BARREL}", f"{ID}:storage_upgrade"],
})
