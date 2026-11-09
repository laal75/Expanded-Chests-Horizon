# Expanded Chests

Bigger chests and storage barrels for [Horizon](https://github.com/horizon-rust/horizon), the Rust
reimplementation of Minecraft: Java Edition. Everything in it is original: names, numbers, code and
textures.

## What it adds

### Chests

| Chest | Slots | Crafted from |
|---|---|---|
| Iron Chest | 54 (9 × 6) | A chest ringed with 8 iron ingots |
| Gold Chest | 72 (12 × 6) | An Iron Chest ringed with 8 gold ingots |
| Diamond Chest | 90 (15 × 6) | A Gold Chest with 4 diamonds and 4 glass |
| Obsidian Chest | 108 (18 × 6) | A Diamond Chest ringed with 8 obsidian; blast-proof |

Every chest is six rows tall, like a double chest, so its screen fits a small window; bigger tiers
grow wider. Hoppers reach every slot from any side, shift-clicks move stacks as a chest's do, and
a broken chest keeps what's in it, as a shulker box does.

**Chest Upgrades:** sneak and right-click a chest holding the next tier's Chest Upgrade, and it
becomes that tier where it stands, keeping everything inside. The upgrade is used up.

| Chest Upgrade | Turns | Crafted from |
|---|---|---|
| Iron Chest Upgrade | a single chest into an Iron Chest | 8 iron ingots around planks |
| Gold Chest Upgrade | an Iron Chest into a Gold Chest | 8 gold ingots around an iron ingot |
| Diamond Chest Upgrade | a Gold Chest into a Diamond Chest | 4 diamonds and 4 glass around a gold ingot |
| Obsidian Chest Upgrade | a Diamond Chest into an Obsidian Chest | 8 obsidian around a diamond |

### Compacting Drawer

Holds one material and offers it in all its forms at once: put in nuggets, ingots or blocks and take
any of them. Holds 32 stacks of the largest form, doubled by each **Storage Upgrade**. Made from a
Storage Barrel between two pistons. Works with iron, gold, copper, netherite, diamond, emerald, lapis,
redstone, coal, raw metals, wheat, slime, bone meal, dried kelp, quartz, glowstone, amethyst, clay, snow
and honeycomb.

### Storage Barrel

Holds one kind of item: 32 stacks, doubled by each **Storage Upgrade**, up to 32,768 stacks with
ten. Hoppers and tubes fill and empty it like a chest. Break it and it drops as a barrel that still
holds its items and its Storage Upgrades; place it again to get them back.

| Item | Crafted from |
|---|---|
| Storage Barrel | A barrel, 4 iron ingots and 4 planks |
| Storage Upgrade | A chest, 4 iron ingots and 4 gold ingots |

Every recipe unlocks in the recipe book the first time you hold one of its ingredients.

## Works with other mods

- The chests are in the common `c:chests` block tag, and the storage barrel in `c:barrels`, so mods
  that read those tags treat them as chests and barrels.
- [Machine Tech](https://github.com/laal75/machines-tech-horizon)' item tubes connect to them, and
  fill each storage barrel with the item it already holds before using any chest.
- For other native mods: `horizon_chests_core::barrel_item(dimension, pos)` returns the item a
  storage barrel holds. Declare an optional dependency on `horizon_chests_core` to call it.

## Install

### Easy install

The ready-made packs are in the [`dist`](dist) folder, so there is nothing to clone:

| File | What it is | Where it goes in the game folder |
|---|---|---|
| [`horizon_chests-0.7.4.hmod`](dist/horizon_chests-0.7.4.hmod) | The content: blocks, items, recipes, textures | `horizon-mods/` |
| [`horizon_chests_core-0.7.4.hnmod`](dist/horizon_chests_core-0.7.4.hnmod) | The code | `horizon-native-mods/` |

1. Download both files. The game folder of the main Horizon instance is `%APPDATA%\horizon` on
   Windows; `horizon-launcher instance list` shows your instances.
2. Copy each file into the folder the table names.
3. Turn both halves on in the game folder's `horizon-mods.txt`:

   ```text
   mod.horizon_chests:true
   mod.horizon_chests_core:true
   ```

4. Build the code into the instance. This also rebuilds your other installed native mods, so they
   stay in the game. The first build downloads a Rust toolchain and takes 20 to 30 minutes; later
   ones take a few minutes.

   ```sh
   horizon-launcher native-build --instance <instance id> --trust-native-code --allow-version-mismatch
   ```

   `--trust-native-code` confirms that you trust the code: native mods run with your user's rights.
5. Play the instance: `horizon-launcher play --instance <instance id>`.

### From source

Expanded Chests is a Horizon **native mod**: the Horizon launcher compiles it into the game for you.
It has two halves, and you need both:

- the content (`content/`, or `horizon_chests.hmod`): blocks, items, recipes and textures
- the code (`native/`, or `horizon_chests_core.hnmod`): the chests' and barrel's slots and screens

You need the [Horizon launcher](https://github.com/horizon-rust/horizon). The launcher downloads its
own Rust toolchain the first time it builds a native mod, which takes 20 to 30 minutes; later builds
take a few minutes.

1. Download or clone this repository.
2. Build the code into an instance (a new one is safest while you try it):

   ```sh
   horizon-launcher instance create "Expanded Chests"
   horizon-launcher native-build native --instance <instance id> --trust-native-code
   ```

   `horizon-launcher instance list` shows the instance id. `--trust-native-code` confirms that you
   trust the code: native mods run with your user's rights, so read it first if you like.
3. Copy the `content/` folder into the instance's game folder as `horizon-mods/horizon_chests/`
   (or put `horizon_chests-<version>.hmod` from `python tools/pack.py` there).
4. Turn both halves on in the instance's `horizon-mods.txt`:

   ```text
   mod.horizon_chests:true
   mod.horizon_chests_core:true
   ```

5. Play the instance: `horizon-launcher play --instance <instance id>`.

To build it together with other native mods such as Machine Tech, put their `native` folders (or
links to them) side by side in one folder and run `native-build` on that folder.

### Horizon version

The code builds on Horizon commit `fdc87ae` (the 2026-11-08 Nightly), set in
`native/horizon-mixin.toml` and `native/api/Cargo.toml`, the
same as Machine Tech, so the two build into one game. The launcher fetches that version when it
builds.

## Development

- `content/` is **generated**: edit `tools/gen_content.py`, then run
  `python tools/gen_content.py content`. The script writes the textures too.
- `native/api/src/` is the code: `lib.rs` (the chests: block entities, slots and screens),
  `barrel.rs` (the storage barrel) and `keep.rs` (keeping contents when broken). The chest grids
  are in both the code and the generator.
- `python tools/pack.py` writes `dist/horizon_chests-<version>.hmod` (the content, for an
  instance's `horizon-mods/`) and `dist/horizon_chests_core-<version>.hnmod` (the code, for
  `horizon-native-mods/` or a GitHub release).

### Versions and releases

`VERSION` holds the version (semver); [CHANGELOG.md](CHANGELOG.md) says what each one changed.
Note changes under its **Unreleased** heading as you go. To release:

1. `python tools/version.py 0.3.0`: sets the version in `VERSION`, both manifests and
   `Cargo.toml`, regenerates the content, and dates the Unreleased notes as 0.3.0.
2. Commit, then `git tag v0.3.0` and push with `git push --follow-tags`.
3. `python tools/pack.py`, and attach the two files in `dist/` to a GitHub release for the tag.

`python tools/version.py` alone checks that every file agrees; `pack.py` refuses when they don't.

## License

MIT; see [LICENSE](LICENSE).
