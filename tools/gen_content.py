"""Writes the Expanded Chests content mod (data, assets and original textures) into argv[1]."""
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
    "obsidian_chest": ("Obsidian Chest", (52, 38, 76), 18, 6, 1200.0,
                       ["OOO", "OCO", "OOO"],
                       {"O": "minecraft:obsidian", "C": f"{ID}:diamond_chest"}),
}
LEGACY = set()
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
name = "Expanded Chests"
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
write("data/c/tags/block/chests.json", {"values": [f"{ID}:{n}" for n in TIERS] + [f"{ID}:large_chest"]})

# ---- the storage barrel ---------------------------------------------------------------------
# One item, kept as a count: 32 stacks, doubled by each Storage Upgrade up to 32768 with ten.
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
write("data/minecraft/tags/block/mineable/axe.json",
      {"values": [f"{ID}:{BARREL}", f"{ID}:compacting_drawer", f"{ID}:large_chest", f"{ID}:large_barrel"]})
write("data/c/tags/block/barrels.json", {"values": [f"{ID}:{BARREL}", f"{ID}:large_barrel"]})

# ---- the compacting drawer --------------------------------------------------------------------
# One material in all its forms (native/api/src/drawer.rs): 0 takes in, 1 to 3 give out, 4 upgrades.
DRAWER = "compacting_drawer"


def drawer_front(seed):
    px = barrel_side(seed)
    for left in (2, 6, 10):
        for y in range(5, 11):
            for x in range(left, left + 4):
                px[y][x] = shade(STEEL, 0.55 if x in (left, left + 3) or y in (5, 10) else 0.95)
    return px


png(f"assets/{ID}/textures/block/{DRAWER}_front.png", drawer_front(53))
write(f"data/{ID}/horizon/block/{DRAWER}.json", {
    "template": "minecraft:furnace",
    "properties": {"hardness": 2.5, "explosion_resistance": 2.5, "requires_tool": False,
                   "sounds": "minecraft:barrel"},
})
write(f"data/{ID}/horizon/menu/{DRAWER}.json", {"slots": 5, "data_slots": 4, "results": [1, 2, 3]})
write(f"assets/{ID}/models/block/{DRAWER}.json", {
    "parent": "minecraft:block/orientable",
    "textures": {"front": f"{ID}:block/{DRAWER}_front", "side": f"{ID}:block/{BARREL}_side",
                 "top": f"{ID}:block/{BARREL}_top"},
})
variants = {}
for facing, rot in FACINGS.items():
    for lit in ("false", "true"):
        v = {"model": f"{ID}:block/{DRAWER}"}
        if rot:
            v["y"] = rot
        variants[f"facing={facing},lit={lit}"] = v
write(f"assets/{ID}/blockstates/{DRAWER}.json", {"variants": variants})
write(f"assets/{ID}/items/{DRAWER}.json", {"model": {"type": "minecraft:model", "model": f"{ID}:block/{DRAWER}"}})
write(f"data/{ID}/loot_table/blocks/{DRAWER}.json", {
    "type": "minecraft:block",
    "pools": [{"entries": [{"type": "minecraft:item", "name": f"{ID}:{DRAWER}"}], "rolls": 1}],
    "random_sequence": f"{ID}:blocks/{DRAWER}",
})
lang[f"block.{ID}.{DRAWER}"] = "Compacting Drawer"
lang[f"container.{ID}.{DRAWER}"] = "Compacting Drawer"

write(f"data/{ID}/horizon/item/storage_upgrade.json", {"template": "minecraft:sugar"})
write(f"assets/{ID}/models/item/storage_upgrade.json",
      {"parent": "minecraft:item/generated", "textures": {"layer0": f"{ID}:item/storage_upgrade"}})
write(f"assets/{ID}/items/storage_upgrade.json",
      {"model": {"type": "minecraft:model", "model": f"{ID}:item/storage_upgrade"}})
lang[f"item.{ID}.storage_upgrade"] = "Storage Upgrade"


# ---- the Large Chest and Large Barrel ports -------------------------------------------------
# A hollow box built in the world (native/api/src/large.rs): planks or logs, the same metal block
# on all eight corners, and up to one port on each side. Slots 0-8 take in; the rest are view slots
# (a stack of each kind the box holds): 247 on a chest port (a menu holds 256 slots at most), 9 on a barrel port.
LARGE = {"large_chest": ("Large Chest Port", 9 + 247), "large_barrel": ("Large Barrel Port", 9 + 9)}


def large_chest_port(seed):
    rng = random.Random(seed)
    px = []
    for y in range(16):
        row = []
        for x in range(16):
            d = max(abs(x - 7.5), abs(y - 7.5))
            if x in (0, 15) or y in (0, 15):
                c = shade(STEEL, 0.7)
            elif d >= 5.5:
                k = 0.55 if (y - 1) % 4 == 3 else 0.9 + rng.random() * 0.15
                c = shade(WOOD, k)
            elif d >= 4.5:
                c = shade(STEEL, 1.1)
            else:
                # The opening the items go in by, darker toward its middle.
                c = shade((40, 30, 22), 0.8 + d * 0.08)
            row.append(c)
        px.append(row)
    # A little latch at the bottom of the ring, as the chests have.
    for y in range(11, 14):
        for x in range(7, 9):
            px[y][x] = shade(STEEL, 1.3 if y == 12 else 0.6)
    return px


def large_barrel_port(seed):
    px = barrel_front(seed)
    # A funnel ring inside the label frame.
    for y in range(16):
        for x in range(16):
            d = max(abs(x - 7.5), abs(y - 7.5))
            if 2.5 <= d < 3.5:
                px[y][x] = shade(STEEL, 1.15)
            elif d < 2.5:
                px[y][x] = shade((30, 22, 16), 0.9 + d * 0.1)
    return px


png(f"assets/{ID}/textures/block/large_chest.png", large_chest_port(60))
png(f"assets/{ID}/textures/block/large_barrel.png", large_barrel_port(61))
for name, (title, slots) in LARGE.items():
    write(f"data/{ID}/horizon/block/{name}.json", {
        "template": "minecraft:furnace",
        "properties": {"hardness": 2.5, "explosion_resistance": 6.0, "requires_tool": False,
                       "sounds": "minecraft:oak_planks"},
    })
    write(f"data/{ID}/horizon/menu/{name}.json",
          {"slots": slots, "data_slots": 16, "results": list(range(9, slots))})
    write(f"assets/{ID}/models/block/{name}.json",
          {"parent": "minecraft:block/cube_all", "textures": {"all": f"{ID}:block/{name}"}})
    write(f"assets/{ID}/blockstates/{name}.json", {"variants": {
        f"facing={f},lit={l}": {"model": f"{ID}:block/{name}"} for f in FACINGS for l in ("false", "true")}})
    write(f"assets/{ID}/items/{name}.json", {"model": {"type": "minecraft:model", "model": f"{ID}:block/{name}"}})
    write(f"data/{ID}/loot_table/blocks/{name}.json", {
        "type": "minecraft:block",
        "pools": [{"entries": [{"type": "minecraft:item", "name": f"{ID}:{name}"}], "rolls": 1}],
        "random_sequence": f"{ID}:blocks/{name}",
    })
    lang[f"block.{ID}.{name}"] = title
    lang[f"container.{ID}.{name}"] = title.replace(" Port", "")

# ---- chest upgrades ---------------------------------------------------------------------------
# Sneak and right-click a chest with one to make it the next tier where it stands, keeping what
# it holds (native/upgrade_chest.rs). Each is planks, or the tier before's metal, ringed by the
# next tier's.
def chest_upgrade_icon(metal):
    """A little chest in the tier's metal on the end of a diagonal stick."""
    px = [[(0, 0, 0, 0)] * 16 for _ in range(16)]
    stick = (120, 84, 46)
    for i in range(9):
        x, y = 1 + i, 14 - i
        px[y][x] = shade(stick, 1.0)
        if x + 1 < 16:
            px[y][x + 1] = shade(stick, 0.7)
    # The chest: wood in a metal frame, lid seam and latch.
    for y in range(2, 10):
        for x in range(7, 15):
            edge = x in (7, 14) or y in (2, 9)
            px[y][x] = shade(metal, 0.85) if edge else shade(WOOD, 0.95 if y % 2 else 0.85)
    for x in range(8, 14):
        px[5][x] = shade(metal, 0.7)
    px[5][10] = px[5][11] = px[6][10] = px[6][11] = shade(metal, 1.15)
    return px


CHEST_UPGRADES = {
    "iron_chest_upgrade": ("Iron Chest Upgrade", "iron_chest",
                           ["III", "IPI", "III"], {"I": "minecraft:iron_ingot", "P": "#minecraft:planks"}),
    "gold_chest_upgrade": ("Gold Chest Upgrade", "gold_chest",
                           ["GGG", "GIG", "GGG"], {"G": "minecraft:gold_ingot", "I": "minecraft:iron_ingot"}),
    "diamond_chest_upgrade": ("Diamond Chest Upgrade", "diamond_chest",
                              ["LDL", "DGD", "LDL"],
                              {"L": "minecraft:glass", "D": "minecraft:diamond", "G": "minecraft:gold_ingot"}),
    "obsidian_chest_upgrade": ("Obsidian Chest Upgrade", "obsidian_chest",
                               ["OOO", "ODO", "OOO"], {"O": "minecraft:obsidian", "D": "minecraft:diamond"}),
}
for name, (title, tier, _, _) in CHEST_UPGRADES.items():
    png(f"assets/{ID}/textures/item/{name}.png", chest_upgrade_icon(TIERS[tier][1]))
    write(f"data/{ID}/horizon/item/{name}.json", {"template": "minecraft:sugar"})
    write(f"assets/{ID}/models/item/{name}.json",
          {"parent": "minecraft:item/generated", "textures": {"layer0": f"{ID}:item/{name}"}})
    write(f"assets/{ID}/items/{name}.json", {"model": {"type": "minecraft:model", "model": f"{ID}:item/{name}"}})
    lang[f"item.{ID}.{name}"] = title


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
    if name in LEGACY:
        continue
    recipe(name, pattern, key, key["C"])
recipe(BARREL, ["PIP", "IBI", "PIP"],
       {"P": "#minecraft:planks", "I": "minecraft:iron_ingot", "B": "minecraft:barrel"}, "minecraft:barrel")
recipe("large_chest", ["PHP", "PCP", "PPP"],
       {"P": "#minecraft:planks", "H": "minecraft:hopper", "C": "minecraft:chest"}, "minecraft:hopper")
recipe("large_barrel", ["PHP", "PBP", "PPP"],
       {"P": "#minecraft:planks", "H": "minecraft:hopper", "B": f"{ID}:{BARREL}"}, f"{ID}:{BARREL}")
recipe(DRAWER, ["PBP"], {"P": "minecraft:piston", "B": f"{ID}:{BARREL}"}, f"{ID}:{BARREL}")
recipe("storage_upgrade", ["IGI", "GCG", "IGI"],
       {"I": "minecraft:iron_ingot", "G": "minecraft:gold_ingot", "C": "minecraft:chest"}, f"{ID}:{BARREL}")
for name, (_, _, pattern, key) in CHEST_UPGRADES.items():
    recipe(name, pattern, key, next(v for v in key.values() if not v.startswith("#")))

lang[f"itemGroup.{ID}.chests"] = "Expanded Chests"
write(f"assets/{ID}/lang/en_us.json", lang)
write(f"data/{ID}/horizon/creative_tab/chests.json", {
    "title": "Expanded Chests",
    "icon": f"{ID}:diamond_chest",
    "items": [f"{ID}:{n}" for n in TIERS if n not in LEGACY] + [f"{ID}:{BARREL}", f"{ID}:{DRAWER}", f"{ID}:storage_upgrade", f"{ID}:large_chest", f"{ID}:large_barrel"]
             + [f"{ID}:{n}" for n in CHEST_UPGRADES],
})
